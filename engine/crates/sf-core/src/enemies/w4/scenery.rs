//! Templates 32 (marker 10) and 27 (marker 11): scenery that only animates where it was placed.

use crate::enemies::common::NOTHING;
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    bob_anim, bob_follow, nothing, obj_despawn_if_offscreen, obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind};

/// Their update (h0+$24ef6).
const STAND: u32 = h0(0x24ef6);

/// Template 32 (also W5's marker 15).
pub static SCENERY_A: ObjectKind = ObjectKind {
    name: "scenery (W4 marker 10)",
    routines: &[(h0(0x24eca), init_32), (STAND, stand), (NOTHING, nothing)],
};

/// Template 27.
pub static SCENERY_B: ObjectKind = ObjectKind {
    name: "scenery (W4 marker 11)",
    routines: &[(h0(0x24ee0), init_27), (STAND, stand), (NOTHING, nothing)],
};

/// Marker 10 (h0+$185b6): 4 up its cell.
pub fn spawn_32(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(32), cell, 0, 0xfffc);
}

/// Marker 11 (h0+$185c8): `$c` down its cell.
pub fn spawn_27(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(27), cell, 0, 0xc);
}

/// h0+$24eca: its look (h0+$322c4).
fn init_32(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x322c4));
}

/// h0+$24ee0: its look (h0+$322e0).
fn init_27(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x322e0));
}

/// h0+$24ef6: its bob where it is; gone once off the screen.
pub fn stand(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
