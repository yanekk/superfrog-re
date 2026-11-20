//! World 3 (T44): the eighteen types its markers spawn (`obj_flag_jumptab`'s `$200` row: markers
//! 5, 13–21, 23–28 and 36, and the swinging ball's chain links). Each marker's stub is
//! `obj_spawn_cell` with the type's template and a pixel offset into the cell; some spawn into a
//! neighbouring cell, and the swinging ball's (27) spawns its five links below it.

pub mod bouncer;
pub mod crumble;
pub mod dart;
pub mod drip;
pub mod hopper;
pub mod jumper;
pub mod scenery;
pub mod shuffler;
pub mod swing;
pub mod walkers;

use crate::items::template;
use crate::level::objects::obj_spawn_cell;
use crate::level::{Dispatch, LevelCtx, register};

/// World 3's row of `obj_flag_jumptab` (`obj_world_offset` `$200`).
const W3: u8 = 2;

/// Every world 3 kind and spawner, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 34, &scenery::SCENERY_17);
    register(d, 55, &dart::DART_LEFT);
    register(d, 56, &dart::DART_RIGHT);
    register(d, 65, &swing::BALL);
    register(d, 66, &swing::LINK);
    register(d, 87, &scenery::PAD);
    register(d, 88, &bouncer::BOUNCER);
    register(d, 89, &scenery::SCENERY_14);
    register(d, 90, &scenery::SCENERY_15);
    register(d, 91, &walkers::STOMPABLE);
    register(d, 92, &shuffler::SHUFFLER);
    register(d, 94, &hopper::HOPPER);
    register(d, 95, &jumper::JUMPER);
    register(d, 96, &walkers::SPIKER);
    register(d, 97, &walkers::SPIKER_STAYING);
    register(d, 98, &scenery::FLOW);
    register(d, 99, &drip::DRIP);
    register(d, 150, &crumble::CRUMBLE);
    for (marker, spawn) in [
        (5, spawn_87 as fn(&mut LevelCtx, u32)),
        (13, spawn_97),
        (14, spawn_89),
        (15, spawn_90),
        (16, spawn_88),
        (17, spawn_34),
        (18, spawn_55),
        (19, spawn_56),
        (20, spawn_92),
        (21, spawn_94),
        (23, spawn_99),
        (24, spawn_98),
        (25, spawn_96),
        (26, spawn_91),
        (27, swing::spawn),
        (28, spawn_95),
        (36, spawn_150),
    ] {
        d.register_marker(W3, marker, spawn);
    }
}

/// Marker 5 (h0+$18344): a pad's picture in the cell up and left of the marker, `$c` down it,
/// from the pad's right cell only (unless the cell right of the marker is marked 5 too).
fn spawn_87(c: &mut LevelCtx, cell: u32) {
    if c.bus.w(cell + 2) & 0x3f == 5 {
        return;
    }
    obj_spawn_cell(c, template(87), cell.wrapping_sub(0x262), 0, 0xc);
}

/// Marker 13 (h0+$184e2).
fn spawn_97(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(97), cell, 0, 4);
}

/// Marker 14 (h0+$1837c): in the cell below.
fn spawn_89(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(89), cell.wrapping_add(0x260), 0, 0xc);
}

/// Marker 15 (h0+$18392): in the cell below.
fn spawn_90(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(90), cell.wrapping_add(0x260), 0, 0xc);
}

/// Marker 16 (h0+$1836a): 10 pixels left of the cell and `$24` above it.
fn spawn_88(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(88), cell, 0xfff6, 0xffdc);
}

/// Marker 17 (h0+$18526).
fn spawn_34(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(34), cell, 0, 0xc);
}

/// Marker 18 (h0+$18548): 12 pixels left of the cell.
fn spawn_55(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(55), cell, 0xfff4, 0x11);
}

/// Marker 19 (h0+$1855a).
fn spawn_56(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(56), cell, 8, 0x11);
}

/// Marker 20 (h0+$184a2).
fn spawn_92(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(92), cell, 0, 0xa);
}

/// Marker 21 (h0+$184b2).
fn spawn_94(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(94), cell, 0, 0xa);
}

/// Marker 23 (h0+$18516).
fn spawn_99(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(99), cell, 0, 0xc);
}

/// Marker 24 (h0+$18502).
fn spawn_98(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(98), cell, 0, 0xc);
}

/// Marker 25 (h0+$184d2).
fn spawn_96(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(96), cell, 0, 4);
}

/// Marker 26 (h0+$184c2).
fn spawn_91(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(91), cell, 0, 0xa);
}

/// Marker 28 (h0+$184f2).
fn spawn_95(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(95), cell, 0, 4);
}

/// Marker 36 (h0+$18536).
fn spawn_150(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(150), cell, 0, 0x1c);
}
