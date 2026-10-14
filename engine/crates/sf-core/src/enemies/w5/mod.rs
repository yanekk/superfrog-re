//! World 5 (`obj_world_offset` `$400`, T39): every type its markers spawn, walked from
//! `obj_flag_jumptab`'s W5 row (docs/09). Markers 2, 3, 10–16, 20–22 spawn its own templates 80,
//! 79, 28, 31, 30, 82, 81, 32, 140, 84, 83 and 29 (84 and 83 shoot template 85). The rest are
//! other worlds' stubs and types: markers 17–19 world 2's mace (63, with its chain, 64) and darts
//! (53, 54), marker 24 world 3's flow (98); template 32 is world 4's scenery.

pub mod crusher;
pub mod drip;
pub mod faller;
pub mod pacer;
pub mod shooter;
pub mod watcher;

use crate::enemies::w2;
use crate::items::template;
use crate::level::objects::obj_spawn_cell;
use crate::level::{Dispatch, LevelCtx, register};

/// W5's row of `obj_flag_jumptab`.
pub const W5: u8 = 4;

/// Every W5 kind and spawner, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 28, &faller::FALLER);
    register(d, 29, &watcher::WATCHER);
    register(d, 30, &faller::FALLER_B);
    register(d, 31, &drip::DRIP);
    register(d, 79, &pacer::PACER_A_LEFT);
    register(d, 80, &pacer::PACER_A_RIGHT);
    register(d, 81, &pacer::PACER_B_LEFT);
    register(d, 82, &pacer::PACER_B_RIGHT);
    register(d, 83, &shooter::SHOOTER_LEFT);
    register(d, 84, &shooter::SHOOTER_RIGHT);
    register(d, 85, &shooter::SHOT);
    register(d, 140, &crusher::CRUSHER);
    d.register_marker(W5, 2, pacer::spawn_80);
    d.register_marker(W5, 3, pacer::spawn_79);
    d.register_marker(W5, 10, faller::spawn_28);
    d.register_marker(W5, 11, drip::spawn_31);
    d.register_marker(W5, 12, faller::spawn_30);
    d.register_marker(W5, 13, pacer::spawn_82);
    d.register_marker(W5, 14, pacer::spawn_81);
    d.register_marker(W5, 15, spawn_32);
    d.register_marker(W5, 16, crusher::spawn_140);
    d.register_marker(W5, 17, w2::mace::spawn);
    d.register_marker(W5, 18, spawn_53);
    d.register_marker(W5, 19, spawn_54);
    d.register_marker(W5, 20, shooter::spawn_84);
    d.register_marker(W5, 21, shooter::spawn_83);
    d.register_marker(W5, 22, watcher::spawn_29);
    d.register_marker(W5, 24, spawn_98);
}

/// Marker 15 (h0+$18906): world 4's scenery (template 32), `$c` down its cell.
fn spawn_32(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(32), cell, 0, 0xc);
}

/// Marker 24 (h0+$18502, world 3's stub): world 3's flow (template 98), `$c` down its cell.
fn spawn_98(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(98), cell, 0, 0xc);
}

/// Marker 18 (h0+$1830c, world 2's stub): world 2's dart (template 53), `$1c` down its cell.
fn spawn_53(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(53), cell, 0, 0x1c);
}

/// Marker 19 (h0+$1831e, world 2's stub): world 2's dart (template 54), `$1c` down its cell.
fn spawn_54(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(54), cell, 0, 0x1c);
}
