//! Level facts every planner needs (DESIGN §2.2, §2.3, §3.2): standable cells, fixed hazards, coins,
//! switches and their gates, exits, the coins the exit still needs, and where the frog is. One
//! tested source in place of the copies in `plan_exit`, `plan_exit_w6`, `explore` and `plan_bonus`.
//!
//! Read from the live map in the game's memory (`level_map`, h1+$218e0), so a taken coin, a pressed
//! switch or an opened gate reads as it is now, and from the level's file for what never changes
//! (hazards, exits).

use sf_assets::LevelData;
use sf_core::frog::{FROG, f};
use sf_core::layout::{h0, h1};
use sf_core::mem::Rom;
use sf_core::{Game, LevelId, State};

/// A map cell, `(col, row)`, on the 300×84 map, top-left origin.
pub type Cell = (u16, u16);

pub const COLS: u16 = 300;
pub const ROWS: u16 = 84;

/// The map's row 0 in memory (`level_map` past the row above it) and a row's length in bytes (304
/// cells): a cell's marker is its low six bits.
const ROW0: u32 = h1(0x218e0 + 0x4c0);
const ROW_BYTES: u32 = 0x260;

/// Markers (docs/09, `level/collision.rs`): `$3a` and up are ground and walls; 6 is the only tile
/// `frog_kill` acts on; 8 and 9 coin objects; 48–51 switches; 54 the exit; 55 a tile pickup.
const GROUND: u16 = 0x3a;
const DEADLY: u16 = 6;
const EXIT: u16 = 54;
const TILE_PICKUP: u16 = 55;
const SWITCHES: std::ops::RangeInclusive<u16> = 48..=51;

/// `coins_left_for_exit` (h0+$832): coins still needed before the exit lock goes.
const COINS_LEFT: u32 = h0(0x832);
/// `marker_cells` (h0+$162e4): per switch marker 48–51, the address of its gate's cell, 0 if it
/// opens nothing (`switch`, h0+$f6be, then does nothing: W6-4's 51).
const MARKER_CELLS: u32 = h0(0x162e4);
/// `obj_world_offset` (h0+$9fa): `$100` per world.
const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
/// h0+$f382: per world, its tile pickup table of (tile, handler) longs, ending in a negative long;
/// `coin` (h0+$ede0) is the coin's handler.
const PICKUP_TABLES: u32 = 0xf382;
const COIN_HANDLER: u32 = h0(0xede0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CoinKind {
    /// A coin object, by its marker (8 or 9). Its marker stays in the map until it is taken, also
    /// once spawned into a slot, so the cell is where to go for it.
    Object(u8),
    /// A tile coin: a marker-55 cell whose tile the world's pickup table gives to `coin`. Taking
    /// one counts in `frog+$7e` only, never in `coins_left_for_exit` (`pickups::coin`, rust-engine
    /// FINDINGS T122), so only `Object` coins meet `coins_needed`.
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Coin {
    pub cell: Cell,
    pub kind: CoinKind,
}

/// A switch not yet pressed: a vertical pair of cells with marker 48–51, and the gate it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Switch {
    /// Top cell first.
    pub cells: [Cell; 2],
    pub gate: Cell,
    pub marker: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub level: LevelId,
    /// Open cells (live marker below `$3a`, not deadly) standing on ground (`$3a` and up below).
    /// Moving platforms are objects, not cells: the map finds landings on them by stepping.
    pub standable: Vec<Cell>,
    /// Marker 6 cells (spikes), from the file: they never move (DESIGN §2.2, D14).
    pub hazards: Vec<Cell>,
    pub coins: Vec<Coin>,
    pub switches: Vec<Switch>,
    pub exits: Vec<Cell>,
    pub coins_needed: u16,
}

/// Where `cell`'s word lives in the game's memory.
pub(crate) fn cell_addr((c, r): Cell) -> u32 {
    ROW0 + r as u32 * ROW_BYTES + 2 * c as u32
}

/// The live map word at `cell`: tile index above, marker in the low six bits.
pub(crate) fn cell_word(st: &State, cell: Cell) -> u16 {
    st.w(cell_addr(cell))
}

/// The live marker at `(c, r)`.
pub(crate) fn live(st: &State, c: u16, r: u16) -> u16 {
    cell_word(st, (c, r)) & 0x3f
}

/// The cell a map address names, if it is on the map.
fn cell_of(addr: u32) -> Option<Cell> {
    let off = addr.checked_sub(ROW0)?;
    let (r, c) = (off / ROW_BYTES, off % ROW_BYTES / 2);
    (r < ROWS as u32 && c < COLS as u32).then_some((c as u16, r as u16))
}

/// The world's coin tiles: its pickup table's tiles whose handler is `coin`.
fn coin_tiles(st: &State, rom: &Rom) -> Vec<u16> {
    let l = |a: u32| -> Option<u32> {
        let o = a as usize;
        Some(u32::from_be_bytes(rom.h0.get(o..o + 4)?.try_into().ok()?))
    };
    let world = st.l(OBJ_WORLD_OFFSET) >> 8;
    let Some(mut at) = l(PICKUP_TABLES + 4 * world).and_then(|a| a.checked_sub(h0(0))) else {
        return Vec::new();
    };
    let mut v = Vec::new();
    while let Some(tile) = l(at).filter(|&t| (t as i32) >= 0) {
        if l(at + 4) == Some(COIN_HANDLER) {
            v.push(tile as u16);
        }
        at += 8;
    }
    v
}

/// The level's facts as `game` has them now. `level` is the level being played (its file gives the
/// fixed hazards and exits); `rom` gives the world's pickup table (`DisplayData::rom`).
pub fn facts(game: &Game, level: &LevelData, rom: &Rom) -> Facts {
    let st = game.state();
    let t = &level.t7mp;
    let file = |c: u16, r: u16| t.body[r as usize * t.width as usize + c as usize] >> 10;
    let cells = || (0..ROWS).flat_map(|r| (0..COLS).map(move |c| (c, r)));
    let tiles = coin_tiles(st, rom);

    let standable = cells()
        .filter(|&(c, r)| {
            let m = live(st, c, r);
            m < GROUND && m != DEADLY && r + 1 < ROWS && live(st, c, r + 1) >= GROUND
        })
        .collect();
    let hazards = cells().filter(|&(c, r)| file(c, r) == DEADLY).collect();
    let exits = cells().filter(|&(c, r)| file(c, r) == EXIT).collect();
    let coins = cells()
        .filter_map(|(c, r)| {
            let cell = st.w(ROW0 + r as u32 * ROW_BYTES + 2 * c as u32);
            let kind = match cell & 0x3f {
                m @ (8 | 9) => CoinKind::Object(m as u8),
                TILE_PICKUP if tiles.contains(&(cell >> 6)) => CoinKind::Tile,
                _ => return None,
            };
            Some(Coin { cell: (c, r), kind })
        })
        .collect();
    // A switch is the top of a vertical pair; one whose `marker_cells` entry is 0 opens nothing
    // and is left out (the game ignores it too).
    let switches = cells()
        .filter_map(|(c, r)| {
            let m = live(st, c, r);
            if !SWITCHES.contains(&m) || r + 1 >= ROWS || live(st, c, r + 1) != m {
                return None;
            }
            if r > 0 && live(st, c, r - 1) == m {
                return None;
            }
            let gate = cell_of(st.l(MARKER_CELLS + 4 * (m as u32 - 48)))?;
            Some(Switch {
                cells: [(c, r), (c, r + 1)],
                gate,
                marker: m as u8,
            })
        })
        .collect();
    Facts {
        level: level.id,
        standable,
        hazards,
        coins,
        switches,
        exits,
        coins_needed: st.w(COINS_LEFT),
    }
}

/// The frog's cell: its middle, a row up (`((X+$17)/16, (Y−$10)/16)`, the old planners' formula),
/// clamped to the map.
pub fn frog_cell(st: &State) -> Cell {
    let w = |o| st.w(FROG + o) as i16 as i32;
    let c = ((w(f::X) + 0x17).div_euclid(16)).clamp(0, COLS as i32 - 1);
    let r = ((w(f::Y) - 0x10).div_euclid(16)).clamp(0, ROWS as i32 - 1);
    (c as u16, r as u16)
}

/// On ground or a platform top, at any speed (DESIGN §2.2: a move ends landed, never padded to a
/// stop).
pub fn landed(st: &State) -> bool {
    (st.w(FROG + f::ON_GROUND) != 0 || st.w(FROG + f::ON_PLATFORM) != 0) && st.w(FROG + f::AIR) == 0
}

/// Landed and not moving sideways: where a standing-start move (§2.5's waits) begins.
pub fn standing(st: &State) -> bool {
    landed(st) && st.w(FROG + f::VX) == 0
}
