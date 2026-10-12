//! The asset cache: [`GameData`] as one versioned, self-checking file, so the disks are imported
//! once (DESIGN §2.2, §3.5). The shell reads and writes the file; this only turns it into bytes
//! and back.
//!
//! Layout, all integers big-endian:
//!
//! ```text
//! magic        8 bytes   "SFRE-GDC"
//! format       u32       CACHE_FORMAT
//! build        u32 len + UTF-8   the engine build that wrote it (recorded, not checked)
//! body length  u64
//! body hash    32 bytes  BLAKE3 of the body
//! body                   GameData, field by field in declaration order
//! ```
//!
//! The body has no padding, no map with an unordered iteration and no float, so the same
//! `GameData` always gives the same bytes. A change to any type in `GameData` changes the body:
//! bump [`CACHE_FORMAT`] with it, so an old cache is re-imported instead of misread.

use std::collections::BTreeMap;
use std::fmt;

use sf_disk::DiskSlot;

use crate::anim::{Anim, AnimFrame, AnimId};
use crate::blake3::blake3;
use crate::exe::{ExeId, ExeImage, Hunk, HunkKind, Reloc};
use crate::font::{Font, FontId};
use crate::gamedata::{ChunkId, GameData, LevelData, LevelId, World};
use crate::palette::Palette;
use crate::picture::{Bitmap, Picture, PictureId};
use crate::sprites::{SpriteBank, SpriteId};
use crate::t7mp::{T7mp, Tag};
use crate::tables::{FileEntry, HiscoreEntry, PasswordEntry, Tables};
use crate::tiles::{TileMasks, Tileset, WorldGfx};

pub const CACHE_FORMAT: u32 = 3;

const MAGIC: [u8; 8] = *b"SFRE-GDC";

/// Why a cache file cannot be used. `Display` says it in words for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheError {
    /// Not a cache file at all.
    BadMagic,
    /// A cache written in another format (by another version of the engine).
    OtherFormat(u32),
    /// The body does not match its hash, or does not decode.
    Corrupt,
    /// The file ends before its header or body does.
    Truncated,
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::BadMagic => write!(f, "the saved game data is not a file this game wrote"),
            CacheError::OtherFormat(_) => {
                write!(
                    f,
                    "the saved game data was written by another version of this game"
                )
            }
            CacheError::Corrupt => write!(f, "the saved game data is damaged"),
            CacheError::Truncated => write!(f, "the saved game data is incomplete"),
        }
    }
}

impl std::error::Error for CacheError {}

/// `data` as a cache file, recording `engine_build` as the build that wrote it.
pub fn cache_encode(data: &GameData, engine_build: &str) -> Vec<u8> {
    let mut body = Vec::new();
    data.put(&mut body);
    let mut out = Vec::with_capacity(body.len() + 64);
    out.extend_from_slice(&MAGIC);
    CACHE_FORMAT.put(&mut out);
    engine_build.to_string().put(&mut out);
    (body.len() as u64).put(&mut out);
    out.extend_from_slice(&blake3(&body));
    out.extend_from_slice(&body);
    out
}

/// The `GameData` a cache file holds, if the file is whole, of this format and undamaged.
pub fn cache_decode(bytes: &[u8]) -> Result<GameData, CacheError> {
    if bytes.len() < MAGIC.len() {
        // A prefix of the magic is a cache cut short; anything else is not a cache.
        return Err(if MAGIC.starts_with(bytes) && !bytes.is_empty() {
            CacheError::Truncated
        } else {
            CacheError::BadMagic
        });
    }
    if bytes[..MAGIC.len()] != MAGIC {
        return Err(CacheError::BadMagic);
    }
    let mut r = Dec {
        data: bytes,
        at: MAGIC.len(),
        short: CacheError::Truncated,
    };
    let format = u32::get(&mut r)?;
    if format != CACHE_FORMAT {
        return Err(CacheError::OtherFormat(format));
    }
    let _build = String::get(&mut r)?;
    let len = u64::get(&mut r)?;
    let hash = r.take(32)?;
    let rest = &bytes[r.at..];
    if (rest.len() as u64) < len {
        return Err(CacheError::Truncated);
    }
    if rest.len() as u64 > len {
        return Err(CacheError::Corrupt);
    }
    if blake3(rest)[..] != *hash {
        return Err(CacheError::Corrupt);
    }
    // The hash held, so a body that still does not decode was written wrong: corrupt, not short.
    let mut r = Dec {
        data: rest,
        at: 0,
        short: CacheError::Corrupt,
    };
    let data = GameData::get(&mut r)?;
    if r.at != rest.len() {
        return Err(CacheError::Corrupt);
    }
    Ok(data)
}

struct Dec<'a> {
    data: &'a [u8],
    at: usize,
    /// What running off the end means here.
    short: CacheError,
}

impl<'a> Dec<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], CacheError> {
        let end = self.at.checked_add(n).ok_or(self.short)?;
        let b = self.data.get(self.at..end).ok_or(self.short)?;
        self.at = end;
        Ok(b)
    }

    /// A length: the count of what follows, each at least one byte, so never more than remain.
    fn len(&mut self) -> Result<usize, CacheError> {
        let n = u32::get(self)? as usize;
        if n > self.data.len() - self.at {
            return Err(self.short);
        }
        Ok(n)
    }
}

/// A value the cache body holds.
trait Codec: Sized {
    fn put(&self, out: &mut Vec<u8>);
    fn get(r: &mut Dec) -> Result<Self, CacheError>;
}

macro_rules! int {
    ($($t:ty),*) => {$(
        impl Codec for $t {
            fn put(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_be_bytes());
            }
            fn get(r: &mut Dec) -> Result<Self, CacheError> {
                Ok(<$t>::from_be_bytes(r.take(size_of::<$t>())?.try_into().unwrap()))
            }
        }
    )*};
}
int!(u8, u16, u32, u64);

impl Codec for bool {
    fn put(&self, out: &mut Vec<u8>) {
        (*self as u8).put(out);
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        match u8::get(r)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(CacheError::Corrupt),
        }
    }
}

impl<T: Codec> Codec for Vec<T> {
    fn put(&self, out: &mut Vec<u8>) {
        (self.len() as u32).put(out);
        for x in self {
            x.put(out);
        }
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        let n = r.len()?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(T::get(r)?);
        }
        Ok(v)
    }
}

impl Codec for String {
    fn put(&self, out: &mut Vec<u8>) {
        self.as_bytes().to_vec().put(out);
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        String::from_utf8(Vec::get(r)?).map_err(|_| CacheError::Corrupt)
    }
}

impl<T: Codec, const N: usize> Codec for [T; N] {
    fn put(&self, out: &mut Vec<u8>) {
        for x in self {
            x.put(out);
        }
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        let mut v = Vec::with_capacity(N);
        for _ in 0..N {
            v.push(T::get(r)?);
        }
        Ok(v.try_into().ok().unwrap())
    }
}

impl<T: Codec> Codec for Option<T> {
    fn put(&self, out: &mut Vec<u8>) {
        self.is_some().put(out);
        if let Some(x) = self {
            x.put(out);
        }
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        Ok(if bool::get(r)? {
            Some(T::get(r)?)
        } else {
            None
        })
    }
}

impl<A: Codec, B: Codec> Codec for (A, B) {
    fn put(&self, out: &mut Vec<u8>) {
        self.0.put(out);
        self.1.put(out);
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        Ok((A::get(r)?, B::get(r)?))
    }
}

/// In key order, so the bytes do not depend on how the map was built.
impl<K: Codec + Ord, V: Codec> Codec for BTreeMap<K, V> {
    fn put(&self, out: &mut Vec<u8>) {
        (self.len() as u32).put(out);
        for (k, v) in self {
            k.put(out);
            v.put(out);
        }
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        let n = r.len()?;
        let mut m = BTreeMap::new();
        for _ in 0..n {
            let k = K::get(r)?;
            // Keys come strictly ascending; anything else was not written by `put`.
            if m.last_key_value().is_some_and(|(last, _)| *last >= k) {
                return Err(CacheError::Corrupt);
            }
            m.insert(k, V::get(r)?);
        }
        Ok(m)
    }
}

/// A struct as its fields in order. Destructuring makes a field added to the type a compile
/// error here until the codec (and [`CACHE_FORMAT`]) follow.
macro_rules! record {
    ($t:ident { $($f:ident),* $(,)? }) => {
        impl Codec for $t {
            fn put(&self, out: &mut Vec<u8>) {
                let $t { $($f),* } = self;
                $($f.put(out);)*
            }
            fn get(r: &mut Dec) -> Result<Self, CacheError> {
                Ok($t { $($f: Codec::get(r)?),* })
            }
        }
    };
}

/// A field-less enum as its position in `[variants]`.
macro_rules! unit_enum {
    ($t:ident [$($v:path),* $(,)?]) => {
        impl Codec for $t {
            fn put(&self, out: &mut Vec<u8>) {
                let all = [$($v),*];
                (all.iter().position(|x| x == self).unwrap() as u8).put(out);
            }
            fn get(r: &mut Dec) -> Result<Self, CacheError> {
                let all = [$($v),*];
                all.get(u8::get(r)? as usize).copied().ok_or(CacheError::Corrupt)
            }
        }
    };
}

unit_enum!(DiskSlot[DiskSlot::D1, DiskSlot::D2, DiskSlot::D3, DiskSlot::D4]);
unit_enum!(World[World::W1, World::W2, World::W3, World::W4, World::W5, World::W6, World::Bonus, World::Final]);
unit_enum!(AnimId[AnimId::End1, AnimId::End2]);
unit_enum!(FontId[FontId::Hall]);
unit_enum!(HunkKind[HunkKind::Code, HunkKind::Data, HunkKind::Bss]);
unit_enum!(ExeId[
    ExeId::Bootstrap,
    ExeId::Game,
    ExeId::BootstrapCopy,
    ExeId::Disk3Boot,
    ExeId::Intro,
    ExeId::InsertDiskOverlay,
    ExeId::AccessingDiskOverlay,
]);

impl Codec for PictureId {
    fn put(&self, out: &mut Vec<u8>) {
        let (tag, world) = match self {
            PictureId::Title => (0u8, None),
            PictureId::HallOfTalent => (1, None),
            PictureId::Awards1 => (2, None),
            PictureId::Awards2 => (3, None),
            PictureId::Loading(w) => (4, Some(*w)),
            PictureId::EndOfWorld(w) => (5, Some(*w)),
            PictureId::SlotMachine => (6, None),
            PictureId::SlotMachineMask => (7, None),
            PictureId::ProgressMap => (8, None),
        };
        tag.put(out);
        if let Some(w) = world {
            w.put(out);
        }
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        Ok(match u8::get(r)? {
            0 => PictureId::Title,
            1 => PictureId::HallOfTalent,
            2 => PictureId::Awards1,
            3 => PictureId::Awards2,
            4 => PictureId::Loading(World::get(r)?),
            5 => PictureId::EndOfWorld(World::get(r)?),
            6 => PictureId::SlotMachine,
            7 => PictureId::SlotMachineMask,
            8 => PictureId::ProgressMap,
            _ => return Err(CacheError::Corrupt),
        })
    }
}

impl Codec for SpriteId {
    fn put(&self, out: &mut Vec<u8>) {
        match self {
            SpriteId::Hero => 0u8.put(out),
            SpriteId::Bobs(w) => {
                1u8.put(out);
                w.put(out);
            }
            SpriteId::SlotReels => 2u8.put(out),
        }
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        Ok(match u8::get(r)? {
            0 => SpriteId::Hero,
            1 => SpriteId::Bobs(World::get(r)?),
            2 => SpriteId::SlotReels,
            _ => return Err(CacheError::Corrupt),
        })
    }
}

impl Codec for Tag {
    fn put(&self, out: &mut Vec<u8>) {
        self.0.put(out);
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        Ok(Tag(Codec::get(r)?))
    }
}

impl Codec for Palette {
    fn put(&self, out: &mut Vec<u8>) {
        self.0.put(out);
    }
    fn get(r: &mut Dec) -> Result<Self, CacheError> {
        Ok(Palette(Codec::get(r)?))
    }
}

record!(ChunkId {
    disk,
    index,
    sector
});
record!(LevelId { world, level, alt });
record!(T7mp {
    width,
    height,
    body,
    tags
});
record!(LevelData {
    id,
    chunk,
    t7mp,
    extra
});
record!(Tileset { tiles });
record!(TileMasks { masks });
record!(Bitmap {
    width,
    height,
    depth,
    pixels
});
record!(SpriteBank { id, sheet });
record!(WorldGfx {
    world,
    tiles,
    masks,
    bobs,
    palette
});
record!(Font {
    id,
    charset,
    glyphs
});
record!(Picture {
    bitmap,
    palette,
    extra
});
record!(AnimFrame { bitmap, reltime });
record!(Anim {
    id,
    palette,
    chunks,
    frames
});
record!(FileEntry {
    start_sector,
    sectors,
    disk
});
record!(PasswordEntry { digits, handler });
record!(HiscoreEntry { score, rest });
record!(Tables {
    file_table,
    disk_ids,
    passwords,
    start_resume,
    hiscores,
    obj_marker_stubs,
    obj_templates,
});
record!(Reloc { target, offsets });
record!(Hunk {
    kind,
    mem,
    size,
    data,
    relocs,
    present
});
record!(ExeImage { id, chunk, hunks });
record!(GameData {
    levels,
    worlds,
    sprites,
    fonts,
    pictures,
    anims,
    tables,
    exes,
    raw,
});
