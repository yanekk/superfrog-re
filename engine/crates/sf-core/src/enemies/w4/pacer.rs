//! Templates 137 (marker 2) and 136 (marker 3): a pacer bobbing along a wave between a pair of
//! markers 2 and 3 on its row, turning round at either. Only one of a pair spawns: each end's
//! stub looks for the other and spawns only while that has no live slot. One hit kills it; the
//! spud cannot (`$7d00`).

use crate::enemies::common::{
    TOUCH_KILL_1HIT, obj_path_y, partner_left_free, partner_right_free, row_base, touch_kill_1hit,
};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// Its bob (h0+$26730): y offsets from its spawn height, a word a pass, ending with `$ffff`.
pub const WAVE: u32 = h0(0x26730);
/// Its looks: moving left (h0+$32264) and right (h0+$32270).
const LEFT: u32 = h0(0x32264);
const RIGHT: u32 = h0(0x32270);
const UPDATE: u32 = h0(0x26e0c);

/// Template 137: starts at the left end.
pub static PACER_LEFT: ObjectKind = ObjectKind {
    name: "pacer (left end)",
    routines: &[
        (h0(0x26dc8), left_init),
        (UPDATE, pacer_update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 136: starts at the right end, `$20` left of its marker.
pub static PACER_RIGHT: ObjectKind = ObjectKind {
    name: "pacer (right end)",
    routines: &[
        (h0(0x26d7e), right_init),
        (UPDATE, pacer_update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Marker 2 (h0+$1857a): `$a` down its cell, while the marker 3 to its right is not live.
pub fn spawn_137(c: &mut LevelCtx, cell: u32) {
    if partner_right_free(&c.bus, cell, 3) {
        obj_spawn_cell(c, template(137), cell, 0, 0xa);
    }
}

/// Marker 3 (h0+$18598): `$a` down its cell, while the marker 2 to its left is not live.
pub fn spawn_136(c: &mut LevelCtx, cell: u32) {
    if partner_left_free(&c.bus, cell, 2) {
        obj_spawn_cell(c, template(136), cell, 0, 0xa);
    }
}

/// Both inits: its look; its spawn point at `+$68`; the wave; `+$64` the start of its row.
fn start(c: &mut LevelCtx, slot: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x68, b.l(slot + 8));
    b.set_l(slot + 0x58, WAVE);
    b.set_l(slot + 0x54, WAVE);
    b.set_l(slot + 0x64, 0);
    let base = row_base(b, slot, 0);
    b.set_l(slot + 0x64, base);
}

/// h0+$26dc8.
fn left_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot);
}

/// h0+$26d7e: then `$20` left.
fn right_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot);
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_sub(0x20));
}

/// Turn round when the cell two columns on from its x in `+$64`'s row holds `a` or `b`: `+$0c`
/// negated, the look `left`; moving right, the look `right` and two steps taken.
pub fn turn_at(c: &mut LevelCtx, slot: u32, markers: [u16; 2], left: u32, right: u32) {
    let b = &mut c.bus;
    let col = (b.w(slot + 8) >> 4).wrapping_mul(2) as u32;
    let cell = col.wrapping_add(b.l(slot + 0x64)).wrapping_add(4);
    if !markers.contains(&(b.w(cell) & 0x3f)) {
        return;
    }
    let vx = b.w(slot + 0xc).wrapping_neg();
    b.set_w(slot + 0xc, vx);
    bob_anim(b, slot, left);
    if vx & 0x8000 == 0 {
        bob_anim(b, slot, right);
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx.wrapping_add(vx)));
    }
}

/// h0+$26e0c: y on the wave from its spawn height; x moves by `+$0c`; it turns at a 2 or a 3.
fn pacer_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let y0 = b.w(slot + 0x6a);
    obj_path_y(b, slot, y0);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    turn_at(c, slot, [2, 3], LEFT, RIGHT);
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
