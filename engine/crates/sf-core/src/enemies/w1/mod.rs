//! World 1 (Magic Woods; T42): the twelve types its markers spawn (`obj_flag_jumptab`'s `$000`
//! row: markers 1–3 and 10–18) and the four their parents throw or drop. Each marker's stub is
//! `obj_spawn_cell` with the type's template and a pixel offset into the cell; the patrollers'
//! (16, 17) look along the row for their other end first.

pub mod crawler;
pub mod dropper;
pub mod hover;
pub mod patroller;
pub mod runner;
pub mod sitter;
pub mod spiker;
pub mod spout;
pub mod walker;
pub mod watcher;

use crate::items::template;
use crate::level::objects::{OBJ_SLOTS, obj_spawn_cell};
use crate::level::{Dispatch, LevelCtx, register};

/// World 1's row of `obj_flag_jumptab` (`obj_world_offset` `$000`).
const W1: u8 = 0;

/// Every world 1 kind and spawner, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 8, &hover::HOVER);
    register(d, 12, &sitter::SITTER);
    register(d, 13, &walker::WALKER);
    register(d, 14, &spout::SHOT_LEFT);
    register(d, 15, &spout::SHOT_UP);
    register(d, 16, &spout::SHOT_RIGHT);
    register(d, 17, &dropper::DROP);
    register(d, 19, &runner::RUNNER_RIGHT);
    register(d, 20, &runner::RUNNER_LEFT);
    register(d, 21, &spiker::SPIKER);
    register(d, 22, &crawler::CRAWLER);
    register(d, 23, &dropper::DROPPER);
    register(d, 33, &spout::SPOUT);
    register(d, 76, &watcher::WATCHER);
    register(d, 77, &patroller::PATROLLER_LEFT);
    register(d, 78, &patroller::PATROLLER_RIGHT);
    for (marker, spawn) in [
        (1, spawn_8 as fn(&mut LevelCtx, u32)),
        (2, spawn_12),
        (3, spawn_13),
        (10, spawn_33),
        (11, spawn_23),
        (12, spawn_22),
        (13, spawn_20),
        (14, spawn_19),
        (15, spawn_21),
        (16, spawn_78),
        (17, spawn_77),
        (18, spawn_76),
    ] {
        d.register_marker(W1, marker, spawn);
    }
}

/// Marker 1 (h0+$18040).
fn spawn_8(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(8), cell, 0, 0);
}

/// Marker 2 (h0+$1804e).
fn spawn_12(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(12), cell, 0, 4);
}

/// Marker 3 (h0+$1805e).
fn spawn_13(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(13), cell, 0, 4);
}

/// Marker 10 (h0+$1806e).
fn spawn_33(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(33), cell, 0, 0xc);
}

/// Marker 11 (h0+$1807e).
fn spawn_23(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(23), cell, 0, 0xc);
}

/// Marker 12 (h0+$1808e).
fn spawn_22(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(22), cell, 0, 7);
}

/// Marker 13 (h0+$1809e).
fn spawn_20(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(20), cell, 0, 0xc);
}

/// Marker 14 (h0+$180ae).
fn spawn_19(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(19), cell, 0, 0xc);
}

/// Marker 15 (h0+$180be).
fn spawn_21(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(21), cell, 0, 9);
}

/// Marker 16 (h0+$180ce): a patroller's left end; it spawns once marker 17 is found to its right.
fn spawn_78(c: &mut LevelCtx, cell: u32) {
    if other_end_free(c, cell, 0x11, true) {
        obj_spawn_cell(c, template(78), cell, 0, 0x16);
    }
}

/// Marker 17 (h0+$180ec): a patroller's right end; it spawns once marker 16 is found to its left.
fn spawn_77(c: &mut LevelCtx, cell: u32) {
    if other_end_free(c, cell, 0x10, false) {
        obj_spawn_cell(c, template(77), cell, 0, 0x16);
    }
}

/// Marker 18 (h0+$1810a).
fn spawn_76(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(76), cell, 0, 0xc);
}

/// h0+$1b350 (`right`) and h0+$1b396: whether, within `$98` cells along the row from `cell`, a
/// cell is marked `marker` and no live slot was spawned from that cell (its patroller is not
/// already out).
fn other_end_free(c: &LevelCtx, cell: u32, marker: u16, right: bool) -> bool {
    let b = &c.bus;
    let Some(end) = (1..=0x98u32)
        .map(|k| if right { cell.wrapping_add(2 * k) } else { cell.wrapping_sub(2 * k) })
        .find(|&a| b.w(a) & 0x3f == marker)
    else {
        return false;
    };
    !(0..20).any(|k| b.l(OBJ_SLOTS + k * 0x7c + 0x40) == end)
}
