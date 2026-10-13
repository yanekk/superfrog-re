//! Templates 139 (marker 13) and 138 (marker 14): a crawler going to and fro between a pair of
//! markers 13 and 14 on its row, paired like the pacer (only one spawns). Out of the spud's reach
//! (`$7d00`); contact hurts.

use crate::enemies::common::{
    TOUCH_SPUD_DAMAGE, partner_left_free, partner_right_free, row_base, touch_spud_damage,
};
use crate::enemies::w4::pacer::turn_at;
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// Its looks: moving left (h0+$32474) and right (h0+$324b0).
const LEFT: u32 = h0(0x32474);
const RIGHT: u32 = h0(0x324b0);
const UPDATE: u32 = h0(0x26eee);

/// Template 139: starts at the left end.
pub static CRAWLER_LEFT: ObjectKind = ObjectKind {
    name: "crawler (left end)",
    routines: &[
        (h0(0x26ec0), left_init),
        (UPDATE, crawler_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Template 138: starts at the right end, `$20` left of its marker.
pub static CRAWLER_RIGHT: ObjectKind = ObjectKind {
    name: "crawler (right end)",
    routines: &[
        (h0(0x26e8c), right_init),
        (UPDATE, crawler_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 13 (h0+$186e4): 5 down its cell, while the marker 14 to its right is not live.
pub fn spawn_139(c: &mut LevelCtx, cell: u32) {
    if partner_right_free(&c.bus, cell, 0xe) {
        obj_spawn_cell(c, template(139), cell, 0, 5);
    }
}

/// Marker 14 (h0+$18702): 5 down its cell, while the marker 13 to its left is not live.
pub fn spawn_138(c: &mut LevelCtx, cell: u32) {
    if partner_left_free(&c.bus, cell, 0xd) {
        obj_spawn_cell(c, template(138), cell, 0, 5);
    }
}

/// Both inits: its look; `+$64` the start of its row.
fn start(c: &mut LevelCtx, slot: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x64, 0);
    let base = row_base(b, slot, 0);
    b.set_l(slot + 0x64, base);
}

/// h0+$26ec0.
fn left_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot);
}

/// h0+$26e8c: then `$20` left.
fn right_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot);
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_sub(0x20));
}

/// h0+$26eee: x moves by `+$0c`; it turns at a 13 or a 14.
fn crawler_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    turn_at(c, slot, [0xd, 0xe], LEFT, RIGHT);
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
