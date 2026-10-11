//! Amiga hunk executables (`0x3F3`): the game's code and the tables in it (docs/08).
//!
//! Hunks are kept as the file has them: contents unrelocated, so a long that the loader relocates
//! reads as an offset into its target hunk, which is how `re/symbols.toml` names addresses
//! (`hN+$off`). Hunks the header declares but the file does not carry (`d1_c03`, `d3_c04`: hunks 2
//! and 3) are kept as absent, sized from the header, because the game's loader still gives them
//! room (docs/08 "Memory map").

use crate::error::{AssetError, Reader, Why};
use crate::gamedata::ChunkId;

const HUNK_HEADER: u32 = 0x3f3;
const HUNK_CODE: u32 = 0x3e9;
const HUNK_DATA: u32 = 0x3ea;
const HUNK_BSS: u32 = 0x3eb;
const HUNK_RELOC32: u32 = 0x3ec;
const HUNK_SYMBOL: u32 = 0x3f0;
const HUNK_DEBUG: u32 = 0x3f1;
const HUNK_END: u32 = 0x3f2;

/// Which executable (docs/08 "The five executables", and the two the game carries packed inside).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExeId {
    /// `d1_c01`: loads and starts the game.
    Bootstrap,
    /// `d1_c03`: the game.
    Game,
    /// `d3_c01`: an unused copy of the bootstrap.
    BootstrapCopy,
    /// `d3_c02`: disk 3's boot, which starts the story intro.
    Disk3Boot,
    /// `d3_c04`: the story intro.
    Intro,
    /// Packed in `d1_c03` at h0+$35fee (`ovl_insert_disk`): the insert-disk prompt.
    InsertDiskOverlay,
    /// Packed in `d1_c03` at h0+$36a04 (`ovl_accessing_disk`): the Accessing-disk overlay.
    AccessingDiskOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkKind {
    Code,
    Data,
    Bss,
}

/// One HUNK_RELOC32 group: longs at `offsets` in this hunk get hunk `target`'s base added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reloc {
    pub target: u16,
    pub offsets: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub kind: HunkKind,
    /// The header's memory flags: bit 0 chip, bit 1 fast (bits 30 and 31 of the size long).
    pub mem: u8,
    /// Bytes the loader gives the hunk, from the header.
    pub size: u32,
    /// Its contents (empty for BSS and for an absent hunk). May be shorter than `size`.
    pub data: Vec<u8>,
    pub relocs: Vec<Reloc>,
    /// False for a hunk the header declares and the file does not carry (kept as BSS).
    pub present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExeImage {
    pub id: ExeId,
    /// The disk chunk it is, or for a packed one, the chunk that carries it.
    pub chunk: ChunkId,
    pub hunks: Vec<Hunk>,
}

impl ExeImage {
    /// `len` bytes of hunk `hunk` from offset `at`, as in the file (unrelocated).
    pub fn bytes(&self, hunk: usize, at: u32, len: usize, what: &str) -> Result<&[u8], AssetError> {
        let data = self.hunks.get(hunk).map(|h| &h.data[..]).unwrap_or(&[]);
        Reader::new(data, self.chunk, what).bytes(at as usize, len)
    }

    pub(crate) fn reader(&self, hunk: usize, what: &str) -> Reader<'_> {
        let data = self.hunks.get(hunk).map(|h| &h.data[..]).unwrap_or(&[]);
        Reader::new(data, self.chunk, what)
    }
}

/// Parse a hunk executable. Never panics; anything unexpected is an [`AssetError`].
pub fn parse_exe(id: ExeId, chunk: ChunkId, bytes: &[u8]) -> Result<ExeImage, AssetError> {
    let what = format!("{id:?} executable");
    let r = Reader::new(bytes, chunk, &what);
    let bad = |m: String| r.err(Why::Malformed(m));
    if r.u32(0)? != HUNK_HEADER {
        return Err(bad("no HUNK_HEADER".into()));
    }
    let mut at = 4;
    // Resident library names: a list of (length in longs, name), ending with 0.
    loop {
        let n = r.u32(at)? as usize;
        at += 4;
        if n == 0 {
            break;
        }
        at = n
            .checked_mul(4)
            .and_then(|b| b.checked_add(at))
            .ok_or_else(|| bad("library name too long".into()))?;
    }
    let (count, first, last) = (r.u32(at)?, r.u32(at + 4)?, r.u32(at + 8)?);
    at += 12;
    if first != 0 || last < first || last - first + 1 != count || count > 64 {
        return Err(bad(format!("hunk table {count} hunks, {first}..{last}")));
    }
    let mut hunks = Vec::new();
    for _ in 0..count {
        let s = r.u32(at)?;
        at += 4;
        let mem = (s >> 30) as u8;
        if mem == 3 {
            at += 4; // an extra long of memory attributes
        }
        hunks.push(Hunk {
            kind: HunkKind::Bss,
            mem,
            size: (s & 0x3fff_ffff) * 4,
            data: Vec::new(),
            relocs: Vec::new(),
            present: false,
        });
    }
    let mut index = 0;
    while at < bytes.len() {
        let t = r.u32(at)? & 0x3fff_ffff;
        at += 4;
        let hunk = hunks
            .get_mut(index)
            .ok_or_else(|| bad(format!("block {t:#x} past the last declared hunk")))?;
        match t {
            HUNK_CODE | HUNK_DATA => {
                let n = r.u32(at)? as usize * 4;
                hunk.data = r.bytes(at + 4, n)?.to_vec();
                hunk.kind = if t == HUNK_CODE {
                    HunkKind::Code
                } else {
                    HunkKind::Data
                };
                hunk.present = true;
                at += 4 + n;
            }
            HUNK_BSS => {
                hunk.kind = HunkKind::Bss;
                hunk.present = true;
                at += 4;
            }
            HUNK_RELOC32 => loop {
                let n = r.u32(at)? as usize;
                at += 4;
                if n == 0 {
                    break;
                }
                let target = r.u32(at)?;
                if target >= count {
                    return Err(bad(format!("relocation to hunk {target}")));
                }
                let offsets = (0..n)
                    .map(|k| r.u32(at + 4 + 4 * k))
                    .collect::<Result<Vec<_>, _>>()?;
                hunk.relocs.push(Reloc {
                    target: target as u16,
                    offsets,
                });
                at += 4 + 4 * n;
            },
            HUNK_SYMBOL => loop {
                let n = r.u32(at)? as usize;
                at += 4;
                if n == 0 {
                    break;
                }
                at += 4 * n + 4;
            },
            HUNK_DEBUG => at += 4 + 4 * r.u32(at)? as usize,
            HUNK_END => index += 1,
            _ => return Err(bad(format!("unknown hunk block {t:#x} at byte {}", at - 4))),
        }
    }
    Ok(ExeImage { id, chunk, hunks })
}
