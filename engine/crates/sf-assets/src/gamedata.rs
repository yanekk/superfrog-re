//! [`GameData`]: everything the game reads from its disks, and the catalogue of which chunk holds
//! what (docs/08 "Files, disks and level composition", docs/10).

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use sf_disk::{Adf, Chunk, DiskSet, DiskSlot, carve, decompress};

use crate::anim::{Anim, AnimId, decode_anim};
use crate::error::{AssetError, Reader, Why};
use crate::exe::{ExeId, ExeImage, parse_exe};
use crate::font::Font;
use crate::palette::Palette;
use crate::picture::{Picture, PictureId};
use crate::sprites::{SpriteBank, SpriteId};
use crate::t7mp::{T7mp, t7mp_decode};
use crate::tables::Tables;
use crate::tiles::{TileMasks, Tileset, WorldGfx};

/// A chunk by disk and 1-based position, as in docs/05 (`d1_c03`), and the sector it starts at
/// (chunks are sector-aligned), which is how the game's `file_table` finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkId {
    pub disk: DiskSlot,
    pub index: u16,
    pub sector: u16,
}

impl ChunkId {
    pub const fn new(disk: DiskSlot, index: u16, sector: u16) -> ChunkId {
        ChunkId {
            disk,
            index,
            sector,
        }
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.index {
            // Not among the chunks handed in: only where it should be is known.
            0 => write!(f, "d{} sector {}", self.disk.number(), self.sector),
            i => write!(f, "d{}_c{i:02}", self.disk.number()),
        }
    }
}

/// A world's files are named `Ln…` with `n` = 1..6, `B` (the bonus level) or `W` (the final level).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum World {
    W1,
    W2,
    W3,
    W4,
    W5,
    W6,
    Bonus,
    Final,
}

impl World {
    pub const ALL: [World; 8] = [
        World::W1,
        World::W2,
        World::W3,
        World::W4,
        World::W5,
        World::W6,
        World::Bonus,
        World::Final,
    ];

    /// The character in the world's file names: `'1'`..`'6'`, `'B'`, `'W'`.
    pub fn code(self) -> char {
        match self {
            World::W1 => '1',
            World::W2 => '2',
            World::W3 => '3',
            World::W4 => '4',
            World::W5 => '5',
            World::W6 => '6',
            World::Bonus => 'B',
            World::Final => 'W',
        }
    }

    fn file(self, kind: &str) -> String {
        format!("L{}{kind}", self.code())
    }
}

/// A level, addressed as docs/LEVEL-INDEX.md does (`W1-1`, `WB-1`, `WW-1`). `alt` marks W6's
/// second set of maps (d4_c14–c17, file-table entries 6–9), which `level_load` (h0+$a144) takes
/// instead of d4_c02–c05 when no drive past df0 is connected and df0 does not hold disk 1: what a
/// one-drive A500 plays. They differ in a few cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LevelId {
    pub world: World,
    pub level: u8,
    pub alt: bool,
}

impl fmt::Display for LevelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "W{}-{}", self.world.code(), self.level)?;
        if self.alt {
            write!(f, "alt")?;
        }
        Ok(())
    }
}

/// The ids the user types (`W1-1`): worlds 1–6, levels 1–4, the first map set; `W6-2alt`..`W6-4alt`
/// W6's second map set; `WB-1` the bonus level, `WW-1` the final level. Every id `Display` prints.
impl FromStr for LevelId {
    type Err = String;

    fn from_str(s: &str) -> Result<LevelId, String> {
        let one = |world| LevelId {
            world,
            level: 1,
            alt: false,
        };
        match s {
            "WB-1" => return Ok(one(World::Bonus)),
            "WW-1" => return Ok(one(World::Final)),
            _ => {}
        }
        if let Some(plain) = s.strip_prefix("W6-").and_then(|l| l.strip_suffix("alt")) {
            return match plain {
                "2" | "3" | "4" => Ok(LevelId {
                    world: World::W6,
                    level: plain.as_bytes()[0] - b'0',
                    alt: true,
                }),
                _ => Err(format!("`{s}`: only W6-2..W6-4 have a second map")),
            };
        }
        let bad = || format!("`{s}` is not a level id like W1-1 (worlds 1-6, levels 1-4)");
        let (w, l) = s
            .strip_prefix('W')
            .and_then(|r| r.split_once('-'))
            .ok_or_else(bad)?;
        // Digits only: `parse` would also take a leading `+`.
        let num = |d: &str| match d.as_bytes() {
            [c @ b'0'..=b'9'] => Ok(c - b'0'),
            _ => Err(bad()),
        };
        let (world, level) = (num(w)?, num(l)?);
        if !(1..=6).contains(&world) || !(1..=4).contains(&level) {
            return Err(bad());
        }
        Ok(LevelId {
            world: World::ALL[world as usize - 1],
            level,
            alt: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelData {
    pub id: LevelId,
    pub chunk: ChunkId,
    pub t7mp: T7mp,
    /// Other per-level files. None exist: object placement is in the T7MP BODY (docs/09), so this
    /// stays empty unless a later task finds one.
    pub extra: BTreeMap<String, Vec<u8>>,
}

/// The complete decoded content of the four disks. `Default` is the empty one, for code whose
/// tests need no content yet (`Game::new` before the mechanics read it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameData {
    pub levels: BTreeMap<LevelId, LevelData>,
    /// In [`World::ALL`] order.
    pub worlds: Vec<WorldGfx>,
    /// HERO, then FRUC. The worlds' bob sheets are in [`WorldGfx::bobs`].
    pub sprites: Vec<SpriteBank>,
    pub fonts: Vec<Font>,
    pub pictures: BTreeMap<PictureId, Picture>,
    pub anims: BTreeMap<AnimId, Anim>,
    pub tables: Tables,
    pub exes: Vec<ExeImage>,
    /// Content kept as its decompressed bytes, by catalogue name: the sound files (T11's), the
    /// story intro's pictures and sound (the intro is a separate program; whether the engine plays
    /// it is T20's call), the Accessing-disk picture and the hidden info page (docs/10).
    pub raw: BTreeMap<String, Vec<u8>>,
}

/// The name `load_file` is given for each `file_table` entry (d1_c03 h0+$cbfe), in entry order,
/// without its `SDDISK1:` prefix: the game's own strings. Where each entry is, disk and sector,
/// is the table's, read from the game at decode time. Several names occur twice: copies on disk
/// 2 for a one-drive game, and W6's two sets of maps.
const FILE_NAMES: [&str; 95] = [
    "END1", "END2", "LWBM", "LWBO", "LWMS", "LWMA1", "L6MA1", "L6MA2", "L6MA3", "L6MA4", "HERO",
    "MUSC", "MPIC", "HPIC", "AWA1", "AWA2", "L1FX", "FRUT", "FRUC", "FRUM", "LFFX", "LFMU", "L1MU",
    "L1LP", "L1BM", "L1BO", "L1MS", "L1MA1", "L1MA2", "L1MA3", "L1MA4", "L1ET", "L2MU", "L2LP",
    "L2BM", "L2BO", "L2MS", "L2MA1", "L2MA2", "L2MA3", "L2MA4", "L2ET", "L6MA1", "L6MA2", "L6MA3",
    "L6MA4", "INFT", "L1FX", "FRUT", "FRUC", "FRUM", "LFFX", "LFMU", "L3MU", "L3LP", "L3BM",
    "L3BO", "L3MS", "L3MA1", "L3MA2", "L3MA3", "L3MA4", "L3ET", "L4MU", "L4LP", "L4BM", "L4BO",
    "L4MS", "L4MA1", "L4MA2", "L4MA3", "L4MA4", "L4ET", "L5MU", "L5LP", "L5BM", "L5BO", "L5MS",
    "L5MA1", "L5MA2", "L5MA3", "L5MA4", "L5ET", "LBFX", "LBMU", "LBLP", "LBBM", "LBBO", "LBMA",
    "L6ET", "L6MU", "L6LP", "L6BM", "L6BO", "L6MS",
];

/// The chunks no `file_table` entry names, by disk and position: the executables and what only
/// they load (docs/08). Names are ours, lower case so they never collide with the game's.
const BOOTSTRAP: (DiskSlot, u16) = (DiskSlot::D1, 1);
const ACCESSING_PICTURE: (DiskSlot, u16) = (DiskSlot::D1, 2);
const GAME: (DiskSlot, u16) = (DiskSlot::D1, 3);
const BOOTSTRAP_COPY: (DiskSlot, u16) = (DiskSlot::D3, 1);
const DISK3_BOOT: (DiskSlot, u16) = (DiskSlot::D3, 2);
const INTRO_SOUND: (DiskSlot, u16) = (DiskSlot::D3, 3);
const INTRO: (DiskSlot, u16) = (DiskSlot::D3, 4);
const INTRO_GFX_1: (DiskSlot, u16) = (DiskSlot::D3, 5);
const INTRO_GFX_2: (DiskSlot, u16) = (DiskSlot::D3, 6);
const INTRO_GFX_3: (DiskSlot, u16) = (DiskSlot::D3, 7);
const INTRO_GFX_4: (DiskSlot, u16) = (DiskSlot::D3, 8);

const OTHER_FILES: [(&str, (DiskSlot, u16)); 11] = [
    ("bootstrap", BOOTSTRAP),
    ("accessing-picture", ACCESSING_PICTURE),
    ("game", GAME),
    ("bootstrap-copy", BOOTSTRAP_COPY),
    ("disk3-boot", DISK3_BOOT),
    ("intro-sound", INTRO_SOUND),
    ("intro", INTRO),
    ("intro-gfx-1", INTRO_GFX_1),
    ("intro-gfx-2", INTRO_GFX_2),
    ("intro-gfx-3", INTRO_GFX_3),
    ("intro-gfx-4", INTRO_GFX_4),
];

/// The disk a `file_table` disk index means: the one whose ID (its last four bytes, docs/08 "Disk
/// swaps") `disk_id_table` gives for it. Disks 1–4 carry `2222`..`5555`.
fn disk_with_id(id: [u8; 4]) -> Option<DiskSlot> {
    match &id {
        b"2222" => Some(DiskSlot::D1),
        b"3333" => Some(DiskSlot::D2),
        b"4444" => Some(DiskSlot::D3),
        b"5555" => Some(DiskSlot::D4),
        _ => None,
    }
}

/// Every chunk with the name it is known by: first the game's `file_table` entries in order (each
/// found by the disk and start sector the table gives), then the chunks only the executables load.
pub fn catalogue(
    chunks: &[(ChunkId, Vec<u8>)],
) -> Result<Vec<(&'static str, ChunkId)>, AssetError> {
    let find = |disk: DiskSlot, index: u16| {
        chunks
            .iter()
            .map(|c| c.0)
            .find(|c| c.disk == disk && c.index == index)
    };
    let (gd, gi) = GAME;
    let game_id = find(gd, gi)
        .ok_or_else(|| AssetError::new(ChunkId::new(gd, gi, 0), "game", Why::Missing))?;
    let game = chunks
        .iter()
        .find(|c| c.0 == game_id)
        .map(|c| &c.1[..])
        .unwrap_or(&[]);
    let tables = Tables::read(&parse_exe(ExeId::Game, game_id, game)?)?;
    let mut out = Vec::new();
    for (k, (name, e)) in FILE_NAMES.iter().zip(&tables.file_table).enumerate() {
        let entry = || format!("file_table entry {k} ({name})");
        let disk = tables
            .disk_ids
            .get(e.disk as usize)
            .and_then(|id| disk_with_id(*id))
            .ok_or_else(|| {
                AssetError::new(
                    game_id,
                    entry(),
                    Why::Malformed(format!("disk index {}", e.disk)),
                )
            })?;
        // A chunk not handed in is missing; it is named by its disk and sector.
        let id = chunks
            .iter()
            .map(|c| c.0)
            .find(|c| c.disk == disk && c.sector == e.start_sector)
            .ok_or_else(|| {
                AssetError::new(ChunkId::new(disk, 0, e.start_sector), *name, Why::Missing)
            })?;
        out.push((*name, id));
    }
    for (name, (disk, index)) in OTHER_FILES {
        let id = find(disk, index)
            .ok_or_else(|| AssetError::new(ChunkId::new(disk, index, 0), name, Why::Missing))?;
        out.push((name, id));
    }
    Ok(out)
}

/// Decompress one carved chunk. Three chunks (`d1_c16`, `d2_c39`, `d4_c16`) keep their trailer
/// in the sector after the carve, which the game's `file_table` reads too (docs/03, docs/08): when
/// the carve alone does not decompress, it is retried with the next sector of the disk appended.
pub fn unpack_chunk(adf: &Adf, chunk: &Chunk) -> Result<Vec<u8>, AssetError> {
    let id = ChunkId::new(chunk.disk, chunk.index, (chunk.offset / 512) as u16);
    match decompress(&chunk.raw) {
        Ok(d) => Ok(d),
        Err(first) => {
            let start = chunk.offset as usize;
            let end = (start + chunk.raw.len() + 512).min(adf.bytes.len());
            decompress(&adf.bytes[start..end])
                .map_err(|_| AssetError::new(id, "chunk", Why::Atn(first)))
        }
    }
}

/// Every chunk of the four disks, decompressed, in disk order: what [`decode`] takes.
pub fn unpack(set: &DiskSet) -> Result<Vec<(ChunkId, Vec<u8>)>, AssetError> {
    let mut out = Vec::new();
    for s in DiskSlot::ALL {
        let adf = set.disk(s);
        for c in carve(adf) {
            out.push((
                ChunkId::new(c.disk, c.index, (c.offset / 512) as u16),
                unpack_chunk(adf, &c)?,
            ));
        }
    }
    Ok(out)
}

struct Chunks<'a> {
    by_id: BTreeMap<ChunkId, &'a [u8]>,
    catalogue: Vec<(&'static str, ChunkId)>,
}

impl<'a> Chunks<'a> {
    /// The `nth` chunk the catalogue calls `name`.
    fn named(&self, name: &str, nth: usize) -> Result<(ChunkId, Reader<'a>), AssetError> {
        let id = self
            .catalogue
            .iter()
            .filter(|(n, _)| *n == name)
            .nth(nth)
            .map(|(_, id)| *id)
            .expect("a name from the catalogue");
        let data = self
            .by_id
            .get(&id)
            .ok_or_else(|| AssetError::new(id, name, Why::Missing))?;
        Ok((id, Reader::new(data, id, "")))
    }
}

/// A reader with its error naming `what`.
fn named<'a>(r: &Reader<'a>, what: &str) -> Reader<'a> {
    Reader::new(r.data, r.chunk, what)
}

/// Build [`GameData`] from the decompressed chunks ([`unpack`]'s output). Never panics: a missing,
/// truncated or corrupt chunk is an [`AssetError`] naming it.
pub fn decode(chunks: &[(ChunkId, Vec<u8>)]) -> Result<GameData, AssetError> {
    let c = Chunks {
        by_id: chunks.iter().map(|(id, d)| (*id, &d[..])).collect(),
        catalogue: catalogue(chunks)?,
    };

    let mut exes = Vec::new();
    for (name, id) in [
        ("bootstrap", ExeId::Bootstrap),
        ("game", ExeId::Game),
        ("bootstrap-copy", ExeId::BootstrapCopy),
        ("disk3-boot", ExeId::Disk3Boot),
        ("intro", ExeId::Intro),
    ] {
        let (chunk, r) = c.named(name, 0)?;
        exes.push(parse_exe(id, chunk, r.data)?);
    }
    let game = exes[1].clone();
    for (at, id) in [
        (0x35fee, ExeId::InsertDiskOverlay),
        (0x36a04, ExeId::AccessingDiskOverlay),
    ] {
        let packed = packed(&game, at, &format!("{id:?}"))?;
        exes.push(parse_exe(id, game.chunk, &packed)?);
    }
    let tables = Tables::read(&game)?;
    let fonts = vec![Font::hall(&game)?];

    let mut pictures = BTreeMap::new();
    let pic = |name: &str, h: u16, depth: u8, palette: bool| -> Result<Picture, AssetError> {
        let (_, r) = c.named(name, 0)?;
        let what = format!("{name} picture");
        Picture::read(
            &Reader::new(r.data, r.chunk, &what),
            320,
            h,
            depth,
            palette,
            0,
        )
    };
    pictures.insert(PictureId::Title, pic("MPIC", 256, 6, false)?);
    pictures.insert(PictureId::HallOfTalent, pic("HPIC", 288, 5, true)?);
    pictures.insert(PictureId::Awards1, pic("AWA1", 256, 5, true)?);
    pictures.insert(PictureId::Awards2, pic("AWA2", 256, 5, true)?);
    pictures.insert(PictureId::SlotMachine, pic("FRUT", 256, 6, false)?);
    pictures.insert(PictureId::SlotMachineMask, pic("FRUM", 256, 1, false)?);
    for w in &World::ALL[..6] {
        pictures.insert(PictureId::EndOfWorld(*w), pic(&w.file("ET"), 256, 5, true)?);
    }
    for w in &World::ALL[..7] {
        let name = w.file("LP");
        let (_, r) = c.named(&name, 0)?;
        let what = format!("{name} picture");
        let r = Reader::new(r.data, r.chunk, &what);
        // After the palette: 672 bytes (W1, W3, W6) or 1 680 (W2, W4, W5) of loading-screen
        // graphics the format does not explain yet; none for the bonus level.
        let extra = match w {
            World::W1 | World::W3 | World::W6 => 672,
            World::W2 | World::W4 | World::W5 => 1_680,
            _ => 0,
        };
        pictures.insert(
            PictureId::Loading(*w),
            Picture::read(&r, 320, 352, 5, true, extra)?,
        );
    }
    let map = packed(&game, 0x37a0a, "progress map")?;
    pictures.insert(
        PictureId::ProgressMap,
        Picture::read(
            &Reader::new(&map, game.chunk, "progress map picture"),
            320,
            256,
            5,
            true,
            0,
        )?,
    );

    let sprite = |name: &str, id: SpriteId| -> Result<SpriteBank, AssetError> {
        let (_, r) = c.named(name, 0)?;
        let what = format!("{name} sheet");
        SpriteBank::read(&Reader::new(r.data, r.chunk, &what), id)
    };
    let sprites = vec![
        sprite("HERO", SpriteId::Hero)?,
        sprite("FRUC", SpriteId::SlotReels)?,
    ];

    let mut worlds = Vec::new();
    for w in World::ALL {
        let bm = w.file("BM");
        let (_, r) = c.named(&bm, 0)?;
        let what = format!("{bm} tiles");
        let tiles = Tileset::read(&named(&r, &what), if w == World::Bonus { 640 } else { 840 })?;
        let masks = if w == World::Bonus {
            None
        } else {
            let ms = w.file("MS");
            let (_, r) = c.named(&ms, 0)?;
            let what = format!("{ms} masks");
            Some(TileMasks::read(&named(&r, &what))?)
        };
        let lp = if w == World::Final { World::W6 } else { w };
        let palette = pictures[&PictureId::Loading(lp)]
            .palette
            .expect("loading pictures carry a palette");
        worlds.push(WorldGfx {
            world: w,
            tiles,
            masks,
            bobs: sprite(&w.file("BO"), SpriteId::Bobs(w))?,
            palette,
        });
    }

    let mut levels = BTreeMap::new();
    let mut level = |name: String, nth: usize, id: LevelId| -> Result<(), AssetError> {
        let (chunk, r) = c.named(&name, nth)?;
        let t7mp = t7mp_decode(r.data, chunk, &format!("{name} map"))?;
        levels.insert(
            id,
            LevelData {
                id,
                chunk,
                t7mp,
                extra: BTreeMap::new(),
            },
        );
        Ok(())
    };
    for w in &World::ALL[..6] {
        for l in 1..=4u8 {
            // W6's first L6MA1..4 in the file table are the second-drive copies.
            let nth = if *w == World::W6 { 1 } else { 0 };
            level(
                w.file(&format!("MA{l}")),
                nth,
                LevelId {
                    world: *w,
                    level: l,
                    alt: false,
                },
            )?;
            if *w == World::W6 {
                level(
                    w.file(&format!("MA{l}")),
                    0,
                    LevelId {
                        world: *w,
                        level: l,
                        alt: true,
                    },
                )?;
            }
        }
    }
    level(
        "LBMA".into(),
        0,
        LevelId {
            world: World::Bonus,
            level: 1,
            alt: false,
        },
    )?;
    level(
        "LWMA1".into(),
        0,
        LevelId {
            world: World::Final,
            level: 1,
            alt: false,
        },
    )?;

    let mut anims = BTreeMap::new();
    for (name, id) in [("END1", AnimId::End1), ("END2", AnimId::End2)] {
        let (chunk, r) = c.named(name, 0)?;
        anims.insert(id, decode_anim(id, chunk, r.data)?);
    }

    let mut raw = BTreeMap::new();
    let mut keep = |name: &str| -> Result<(), AssetError> {
        let (_, r) = c.named(name, 0)?;
        raw.insert(name.to_string(), r.data.to_vec());
        Ok(())
    };
    for name in [
        "MUSC",
        "L1FX",
        "LFFX",
        "LFMU",
        "LBFX",
        "INFT",
        "accessing-picture",
        "intro-sound",
        "intro-gfx-1",
        "intro-gfx-2",
        "intro-gfx-3",
        "intro-gfx-4",
    ] {
        keep(name)?;
    }
    for w in &World::ALL[..7] {
        keep(&w.file("MU"))?;
    }
    // The front end (T20) loads these as files into memory, not as decoded pictures.
    for name in ["MPIC", "HPIC", "AWA1", "AWA2"] {
        keep(name)?;
    }
    // Disk 2's copy of `L1FX`: the effects of worlds 3 to 6 (docs/11).
    let (_, r) = c.named("L1FX", 1)?;
    raw.insert("L1FX-2".to_string(), r.data.to_vec());
    // Pictures packed in the game's hunk 0 that the title and the options screen unpack.
    for at in [0x3464c, 0x34e92, 0x36a04] {
        raw.insert(format!("h0@{at:x}"), packed(&game, at, "packed picture")?);
    }

    Ok(GameData {
        levels,
        worlds,
        sprites,
        fonts,
        pictures,
        anims,
        tables,
        exes,
        raw,
    })
}

/// An `ATN!` chunk packed inside the game's hunk 0 at `at`, decompressed.
fn packed(game: &ExeImage, at: u32, what: &str) -> Result<Vec<u8>, AssetError> {
    let data = game.hunks.first().map(|h| &h.data[..]).unwrap_or(&[]);
    let from = data.get(at as usize..).unwrap_or(&[]);
    decompress(from).map_err(|e| AssetError::new(game.chunk, what, Why::Atn(e)))
}

impl GameData {
    pub fn world(&self, w: World) -> &WorldGfx {
        &self.worlds[World::ALL.iter().position(|x| *x == w).unwrap()]
    }

    pub fn exe(&self, id: ExeId) -> &ExeImage {
        self.exes
            .iter()
            .find(|e| e.id == id)
            .expect("decode builds every executable")
    }

    /// The palette a level is drawn with.
    pub fn level_palette(&self, id: LevelId) -> Palette {
        self.world(id.world).palette
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ids_round_trip_and_bad_ones_are_refused() {
        let id: LevelId = "W3-2".parse().unwrap();
        assert_eq!((id.world, id.level, id.alt), (World::W3, 2, false));
        assert_eq!(id.to_string(), "W3-2");
        assert_eq!("W6-4".parse::<LevelId>().unwrap().world, World::W6);
        let bonus: LevelId = "WB-1".parse().unwrap();
        assert_eq!(
            (bonus.world, bonus.to_string()),
            (World::Bonus, "WB-1".to_string())
        );
        for id in ["W6-2alt", "W6-4alt", "WW-1"] {
            assert_eq!(id.parse::<LevelId>().unwrap().to_string(), id);
        }
        assert!("W6-3alt".parse::<LevelId>().unwrap().alt);
        assert_eq!("WW-1".parse::<LevelId>().unwrap().world, World::Final);
        for bad in [
            "W0-1", "W7-1", "W1-5", "1-1", "W1", "W1-x", "", "W+1-1", "W1-+2", "W6-1alt",
            "W6-5alt", "W5-2alt", "WW-2", "WB-2",
        ] {
            assert!(bad.parse::<LevelId>().is_err(), "{bad}");
        }
    }
}
