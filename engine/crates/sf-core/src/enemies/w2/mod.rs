//! World 2's object types (T43): 17 templates, none shared with world 1 or 3; world 5 reuses the
//! swinging ball (63, 64) and the darts (53, 54). Each marker's spawner is the stub in the
//! `$100` row of `obj_flag_jumptab` (h0+$178ec); most are `obj_spawn_cell` with a pixel offset.

pub mod backdrop;
pub mod bouncer;
pub mod crumbling_ledge;
pub mod dart;
pub mod flyer;
pub mod hopper;
pub mod mace;
pub mod scenery;
pub mod spikes;
pub mod swooper;
pub mod walker;

use crate::items::template;
use crate::level::objects::obj_spawn_cell;
use crate::level::{Dispatch, LevelCtx, register};

/// World 2's row of `obj_flag_jumptab` (`obj_world_offset` `$100`).
pub const WORLD: u8 = 1;

/// Every world 2 kind and spawner, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 38, &flyer::FLYER);
    register(d, 75, &bouncer::BOUNCER);
    register(d, 74, &bouncer::BOUNCER_LEFT);
    register(d, 52, &backdrop::BACKDROP);
    register(d, 40, &hopper::HOPPER);
    register(d, 39, &walker::WALKER);
    register(d, 24, &scenery::SCENERY_A);
    register(d, 25, &scenery::SCENERY_B);
    register(d, 26, &scenery::SCENERY_C);
    register(d, 71, &spikes::SPIKES);
    register(d, 72, &spikes::SPIKES_B);
    register(d, 63, &mace::MACE);
    register(d, 64, &mace::CHAIN);
    register(d, 53, &dart::DART);
    register(d, 54, &dart::DART_B);
    register(d, 37, &swooper::SWOOPER);
    register(d, 149, &crumbling_ledge::LEDGE);
    let cell_spawns: [(u8, MarkerFn); 14] = [
        (1, |c, a| spawn(c, 38, a, 0)),
        (2, bouncer::spawn_at_left_end),
        (3, bouncer::spawn_at_right_end),
        (10, |c, a| spawn(c, 52, a, 0xc)),
        (11, |c, a| spawn(c, 40, a, 0xc)),
        (12, |c, a| spawn(c, 39, a, 9)),
        (13, |c, a| spawn(c, 24, a, 0xc)),
        (14, |c, a| spawn(c, 25, a, 0xc)),
        (15, |c, a| spawn(c, 26, a, 0xc)),
        (16, |c, a| spawn(c, 71, a, 0xc)),
        (17, mace::spawn),
        (18, |c, a| spawn(c, 53, a, 0x1c)),
        (19, |c, a| spawn(c, 54, a, 0x1c)),
        (20, |c, a| spawn(c, 72, a, 0xc)),
    ];
    for (m, f) in cell_spawns {
        d.register_marker(WORLD, m, f);
    }
    d.register_marker(WORLD, 21, |c, a| spawn(c, 37, a, 0xfff4));
    d.register_marker(WORLD, 36, |c, a| spawn(c, 149, a, 0x1c));
    // Marker 34's stub is an `rts` (h0+$18330).
    d.register_marker(WORLD, 34, |_, _| {});
}

type MarkerFn = fn(&mut LevelCtx, u32);

/// The marker of map cell `cell` (its low six bits).
pub fn marker(b: &crate::mem::Bus, cell: u32) -> u16 {
    b.w(cell) & 0x3f
}

/// A marker stub's `obj_spawn_cell`: template `t` at `cell`, `dy` pixels down it.
fn spawn(c: &mut LevelCtx, t: u32, cell: u32, dy: u16) {
    obj_spawn_cell(c, template(t), cell, 0, dy);
}
