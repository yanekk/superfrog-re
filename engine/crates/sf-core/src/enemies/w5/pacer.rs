//! Templates 80 (marker 2) and 79 (marker 3), 82 (marker 13) and 81 (marker 14): pacers that walk
//! between a pair of markers on their row, 8 pixels a pass (79/80, between 2 and 3) or 3 (81/82,
//! between 13 and 14), as world 1's patroller does. The left end's starts at its cell, the right
//! end's `$20` left of its. It hurts on any touch and nothing kills it (`touch_kill_1hit` on
//! `$7d00` hit points). Only one spawns per pair: a marker's stub looks for its partner and spawns
//! nothing while that one is live.

use crate::enemies::common::{
    TOUCH_KILL_1HIT, follow_and_despawn, partner_left_free, partner_right_free, row_base,
    touch_kill_1hit,
};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// 79/80's animations, left (h0+$32834) and right (h0+$32868), and update.
const A_LEFT: u32 = h0(0x32834);
const A_RIGHT: u32 = h0(0x32868);
const A_UPDATE: u32 = h0(0x26874);
/// 81/82's (h0+$3289c, h0+$328c8; h0+$26b14).
const B_LEFT: u32 = h0(0x3289c);
const B_RIGHT: u32 = h0(0x328c8);
const B_UPDATE: u32 = h0(0x26b14);

/// Template 79: from the right end (marker 3).
pub static PACER_A_LEFT: ObjectKind = ObjectKind {
    name: "w5 pacer (markers 2/3) from the right end",
    routines: &[
        (h0(0x26806), init_79),
        (A_UPDATE, update_a),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 80: from the left end (marker 2).
pub static PACER_A_RIGHT: ObjectKind = ObjectKind {
    name: "w5 pacer (markers 2/3) from the left end",
    routines: &[
        (h0(0x26840), init_80),
        (A_UPDATE, update_a),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 81: from the right end (marker 14).
pub static PACER_B_LEFT: ObjectKind = ObjectKind {
    name: "w5 pacer (markers 13/14) from the right end",
    routines: &[
        (h0(0x26aa6), init_81),
        (B_UPDATE, update_b),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 82: from the left end (marker 13).
pub static PACER_B_RIGHT: ObjectKind = ObjectKind {
    name: "w5 pacer (markers 13/14) from the left end",
    routines: &[
        (h0(0x26ae0), init_82),
        (B_UPDATE, update_b),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Marker 2 (h0+$1880a): once marker 3 is found free to its right, `$1a` down its cell.
pub fn spawn_80(c: &mut LevelCtx, cell: u32) {
    if partner_right_free(&c.bus, cell, 3) {
        obj_spawn_cell(c, template(80), cell, 0, 0x1a);
    }
}

/// Marker 3 (h0+$18828): once marker 2 is found free to its left.
pub fn spawn_79(c: &mut LevelCtx, cell: u32) {
    if partner_left_free(&c.bus, cell, 2) {
        obj_spawn_cell(c, template(79), cell, 0, 0x1a);
    }
}

/// Marker 13 (h0+$18846): once marker 14 is found free to its right, `$1e` down its cell.
pub fn spawn_82(c: &mut LevelCtx, cell: u32) {
    if partner_right_free(&c.bus, cell, 0xe) {
        obj_spawn_cell(c, template(82), cell, 0, 0x1e);
    }
}

/// Marker 14 (h0+$18864): once marker 13 is found free to its left.
pub fn spawn_81(c: &mut LevelCtx, cell: u32) {
    if partner_left_free(&c.bus, cell, 0xd) {
        obj_spawn_cell(c, template(81), cell, 0, 0x1e);
    }
}

/// The inits' common part: `anim`, its spawn position in `+$68`, its row's base in `+$64`.
pub(crate) fn pacer_init(c: &mut LevelCtx, slot: u32, anim: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, anim);
    b.set_l(slot + 0x68, b.l(slot + 8));
    b.set_l(slot + 0x64, 0);
    b.set_l(slot + 0x64, row_base(b, slot, 0));
}

/// h0+$26806: going left, `$20` left of its cell.
fn init_79(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pacer_init(c, slot, A_LEFT);
    c.bus.set_w(slot + 8, c.bus.w(slot + 8).wrapping_sub(0x20));
}

/// h0+$26840.
fn init_80(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pacer_init(c, slot, A_RIGHT);
}

/// h0+$26aa6: `$20` left of its cell.
fn init_81(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pacer_init(c, slot, B_LEFT);
    c.bus.set_w(slot + 8, c.bus.w(slot + 8).wrapping_sub(0x20));
}

/// h0+$26ae0.
fn init_82(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pacer_init(c, slot, B_RIGHT);
}

/// The pacers' step: x moves by `+$0c`; at a cell marked `ends` two cells on from the one under
/// x − 16 it turns, and going right steps on twice its speed. Whether it turned.
pub(crate) fn pace(c: &mut LevelCtx, slot: u32, ends: [u16; 2], left: u32, right: u32) -> bool {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    let x = b.w(slot + 8).wrapping_sub(0x10);
    let at = ((x >> 4).wrapping_add(x >> 4) as u32)
        .wrapping_add(b.l(slot + 0x64))
        .wrapping_add(4);
    if !ends.contains(&(b.w(at) & 0x3f)) {
        return false;
    }
    let vx = b.w(slot + 0xc).wrapping_neg();
    b.set_w(slot + 0xc, vx);
    bob_anim(b, slot, left);
    if n_clear(vx) {
        bob_anim(b, slot, right);
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx.wrapping_add(vx)));
    }
    true
}

/// h0+$26874: between markers 2 and 3.
fn update_a(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pace(c, slot, [2, 3], A_LEFT, A_RIGHT);
    follow_and_despawn(c, slot);
}

/// h0+$26b14: between markers 13 and 14.
fn update_b(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pace(c, slot, [0xd, 0xe], B_LEFT, B_RIGHT);
    follow_and_despawn(c, slot);
}
