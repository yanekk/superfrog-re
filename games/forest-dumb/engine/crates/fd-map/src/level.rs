//! A world's level as the page draws it, from bytes handed in (DESIGN §2.1, §2.2). Pure (DESIGN
//! §3.1): no file, clock or process here; `fd-conform/tests/purity.rs` scans this file.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use fd_assets::{LEVEL_ROWS, Level, loaded_hunk};

/// The six worlds, one level each (docs/LEVEL-INDEX.md).
pub const WORLDS: [u8; 6] = [1, 2, 3, 4, 5, 6];

const IDS: [&str; 6] = ["W1-1", "W2-1", "W3-1", "W4-1", "W5-1", "W6-1"];

/// The program's hunks (docs/07-executable-map.md); `loaded_hunk` wants a base for each.
const PROGRAM_HUNKS: usize = 5;
/// The level copper list's colour block in hunk 1 that `level_init` copies over `h1+$304` (docs/06,
/// "The palette"; `fd-core` copies the same block): 32 pairs of register and 12-bit colour.
const PALETTE_AT: usize = 0x384;
const COLOUR00: u16 = 0x180;
/// The coins (`$20`-`$27`): `h0+$7a9e` counts them over the whole object map and `items_left` starts
/// at that count less [`SPARE_ITEMS`] (docs/10, "Items"; map-page T00).
const COINS: std::ops::RangeInclusive<u8> = 0x20..=0x27;
const SPARE_ITEMS: usize = 50;
/// The exit's cells (docs/10).
const EXIT: u8 = 0xff;

/// The level's id: `W2-1` for world 2.
pub fn level_id(world: u8) -> String {
    format!("W{world}-1")
}

/// [`level_id`] borrowed for the page's `LevelDump`; `W?-1` for a world past the six.
pub(crate) fn level_id_str(world: u8) -> &'static str {
    IDS.get((world as usize).wrapping_sub(1))
        .copied()
        .unwrap_or("W?-1")
}

/// One non-zero cell of the object map: its value, its page layer and its name ([`object_name`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapObject {
    pub c: u16,
    pub r: u16,
    pub value: u8,
    pub cat: &'static str,
    pub name: String,
}

/// What the page draws of one world, read from its files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapLevel {
    pub world: u8,
    /// The used columns: the collision map's (240, W6-1 320); the tile maps' filler past them is
    /// never drawn.
    pub cols: u16,
    pub rows: u16,
    /// The level palette, 8 bits a channel.
    pub palette: Vec<[u8; 3]>,
    /// The tile map over the used columns, row by row (the page's order; the file is column by
    /// column).
    pub body: Vec<u16>,
    /// Each tile `body` uses, 16×16 colour numbers row by row.
    pub tiles: Vec<(u16, [u8; 256])>,
    /// Every non-zero object cell over the used columns, column by column.
    pub objects: Vec<MapObject>,
    /// The `$ff` cells, column by column.
    pub exit: Vec<(u16, u16)>,
    /// `items_left` as `level_init` leaves it, decimal: the items the exit needs.
    pub items_needed: u16,
}

/// World `world`'s level from its files (`level`, as `fd_assets::read_level` gives them) and the
/// main program's bytes (`program`, for the palette). An error names the file at fault.
pub fn map_level(world: u8, level: &Level, program: &[u8]) -> Result<MapLevel, String> {
    if !WORLDS.contains(&world) {
        return Err(format!("world {world}: there are worlds 1 to 6"));
    }
    let objects_file = format!("_MapaBobow{world}");
    let cols = level.collision.columns;
    if level.tiles.columns < cols || level.objects.columns < cols {
        return Err(format!(
            "W{world}-1: the tile map ({}) or object map ({}) is narrower than its {cols} used columns",
            level.tiles.columns, level.objects.columns
        ));
    }
    let body: Vec<u16> = (0..LEVEL_ROWS)
        .flat_map(|r| (0..cols).map(move |c| level.tile(c, r)))
        .collect();
    let used: std::collections::BTreeSet<u16> = body.iter().copied().collect();
    let tiles = used
        .into_iter()
        .map(|t| {
            let px = std::array::from_fn(|i| level.tile_pixel(t, (i % 16) as u8, (i / 16) as u8));
            (t, px)
        })
        .collect();
    let mut objects = Vec::new();
    let mut exit = Vec::new();
    for c in 0..cols {
        for r in 0..LEVEL_ROWS {
            let value = level.object(c, r);
            if value == 0 {
                continue;
            }
            if value == EXIT {
                exit.push((c, r));
            }
            let (cat, name) = object_name(value);
            objects.push(MapObject {
                c,
                r,
                value,
                cat,
                name,
            });
        }
    }
    if exit.is_empty() {
        return Err(format!(
            "{objects_file}: no exit (`$ff`) cell in the used columns"
        ));
    }
    Ok(MapLevel {
        world,
        cols,
        rows: LEVEL_ROWS,
        palette: palette(program)?,
        body,
        tiles,
        objects,
        exit,
        items_needed: items_needed(&objects_file, &level.objects.cells)?,
    })
}

/// The items the exit needs, from the object map's bytes (`file` names it in an error): every
/// `load_list_world_N` counts the coins over all 38 400 bytes, unused columns included, and
/// subtracts 50 (map-page T00, docs/10). The original keeps it as three BCD digits, so a count the
/// game could not hold is an error rather than a number the original would never show.
pub fn items_needed(file: &str, objects: &[u8]) -> Result<u16, String> {
    let coins = objects.iter().filter(|v| COINS.contains(v)).count();
    match coins.checked_sub(SPARE_ITEMS) {
        Some(n) if n <= 999 => Ok(n as u16),
        _ => Err(format!(
            "{file}: {coins} coins, and items needed (coins - {SPARE_ITEMS}) must be 0 to 999"
        )),
    }
}

/// The level palette from the main program's hunk 1, each register checked.
pub fn palette(program: &[u8]) -> Result<Vec<[u8; 3]>, String> {
    const FILE: &str = "FOREST_DUMB_FOREVER";
    let h1 = loaded_hunk(program, FILE, 1, &[0; PROGRAM_HUNKS]).map_err(|e| e.to_string())?;
    let word = |at: usize| h1.get(at..at + 2).map(|w| u16::from_be_bytes([w[0], w[1]]));
    (0..32u16)
        .map(|i| {
            let at = PALETTE_AT + 4 * i as usize;
            match (word(at), word(at + 2)) {
                (Some(reg), Some(c)) if reg == COLOUR00 + 2 * i && c <= 0xfff => {
                    let n = |s: u16| ((c >> s) & 0xf) as u8 * 17;
                    Ok([n(8), n(4), n(0)])
                }
                (reg, c) => Err(format!(
                    "{FILE}: hunk 1 +${at:x}: {reg:x?}={c:x?} is not colour {i}'s copper move"
                )),
            }
        })
        .collect()
}

/// Each value's group from the tables `tables` (name, text), in their order: a value two tables
/// group differently keeps the first, and `fd-conform/tests/object_tables.rs` fails on it (T01).
pub fn groups_of(tables: &[(&str, &str)]) -> BTreeMap<u8, String> {
    let mut out = BTreeMap::new();
    for (_, text) in tables {
        for block in text.split("[[value]]").skip(1) {
            let (mut value, mut group) = (None, None);
            for line in block.lines() {
                let line = line.split('#').next().unwrap_or("").trim();
                let Some((k, v)) = line.split_once('=') else {
                    continue;
                };
                let v = v.trim();
                match k.trim() {
                    "value" => {
                        value = match v.strip_prefix("0x") {
                            Some(hex) => u8::from_str_radix(hex, 16).ok(),
                            None => v.parse().ok(),
                        }
                    }
                    "group" => group = Some(v.trim_matches('"').to_string()),
                    _ => {}
                }
            }
            if let (Some(v), Some(g)) = (value, group) {
                out.entry(v).or_insert(g);
            }
        }
    }
    out
}

/// Every `re/objects-*.toml`'s groups, merged once.
pub fn groups() -> &'static BTreeMap<u8, String> {
    static GROUPS: OnceLock<BTreeMap<u8, String>> = OnceLock::new();
    GROUPS.get_or_init(|| groups_of(crate::TABLES))
}

/// An object value's page layer and name from its group in every `re/objects-*.toml` (the page
/// draws `coin`, `enemy`, `pickup` and `hazard` in their own shapes), or `other` and its number for
/// a value no table groups: never a guessed name (DESIGN §1).
pub fn object_name(value: u8) -> (&'static str, String) {
    name_in(groups(), value)
}

pub(crate) fn name_in(groups: &BTreeMap<u8, String>, value: u8) -> (&'static str, String) {
    let group = groups.get(&value).map(String::as_str);
    let cat = match group {
        // The page's summary counts the objects named `coin`; docs/10: `$20`..`$27` are coins.
        Some("item") => return ("coin", "coin".into()),
        Some("creature") => "enemy",
        Some("bonus") => "pickup",
        Some("water") => "hazard",
        Some("spring") => "spring",
        Some("vanishing-block") => "block",
        Some("flame") => "flame",
        // The page labels an object named `level exit` EXIT.
        Some("exit") => return ("exit", "level exit".into()),
        _ => "other",
    };
    let name = match group {
        Some(g) => format!("{g} ${value:02x}"),
        None => format!("object ${value:02x}"),
    };
    (cat, name)
}
