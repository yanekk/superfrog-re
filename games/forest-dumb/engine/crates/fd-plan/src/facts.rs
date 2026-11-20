//! W1-1 as the planner sees it, from the level's files: the map's size, the exit's cells and the
//! items. Values from docs/05-file-formats.md and docs/10-object-system.md (T00, T21).

use amiga_plan::{Cell, Facts};
use fd_assets::{LEVEL_ROWS, Level};

/// The level's used columns: the collision map's (docs/05: 240, past which only filler rock).
pub const COLS: u16 = 240;
pub const ROWS: u16 = LEVEL_ROWS;

/// The exit's object value (docs/10: sets `level_exit_touched`).
pub const EXIT: u8 = 0xff;

/// Items (`$20` to `$23`, coins): taking one counts `items_left` down (docs/10).
pub fn is_item(v: u8) -> bool {
    (0x20..=0x23).contains(&v)
}

/// The mechanics (docs/10, T25): springs (`$10` to `$13`), water (`$28`, `$2a`), vanishing blocks (`$30`
/// to `$33`) and the flame (`$34`). None is a switch, so none makes a gate; play changes a vanishing
/// block's `collision_map`, not the level's file the planner's `open` reads.
pub fn is_mechanic(v: u8) -> bool {
    matches!(v, 0x10..=0x13 | 0x28 | 0x2a | 0x30..=0x34)
}

/// The cell's collision shape is solid on every pixel.
pub fn solid(level: &Level, c: u16, r: u16) -> bool {
    if c >= level.collision.columns || r >= ROWS {
        return true;
    }
    let shape = level.shape(c, r) as usize;
    level
        .shapes
        .get(shape)
        .is_none_or(|s| s.iter().all(|&w| w == 0xffff))
}

/// The level's facts: its size, the exit's cells and the items' cells, column by column. W1-1 has no
/// switches, so no gates: its mechanics ([`is_mechanic`]) are not switches.
pub fn facts(level: &Level) -> Facts {
    let cols = COLS.min(level.objects.columns);
    let cells = |want: fn(u8) -> bool| -> Vec<Cell> {
        (0..cols)
            .flat_map(|c| (0..ROWS).map(move |r| (c, r)))
            .filter(|&(c, r)| want(level.object(c, r)))
            .collect()
    };
    Facts {
        cols,
        rows: ROWS,
        exits: cells(|v| v == EXIT),
        collectibles: cells(is_item),
        gates: Vec::new(),
    }
}
