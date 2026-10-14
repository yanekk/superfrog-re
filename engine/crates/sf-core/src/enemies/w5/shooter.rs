//! Templates 84 (marker 20) and 83 (marker 21): a pacer between a pair of markers 20 and 21, 3
//! pixels a pass, that shoots: `$a` passes after it turns left it stops for 9 and fires one shot
//! (template 85) from its middle. 84 starts at its cell, 83 `$20` left of its. It hurts on any
//! touch and nothing kills it (`touch_kill_1hit` on `$7d00` hit points). Template 85, the shot,
//! flies left and falls until it meets a solid cell.

use crate::enemies::common::{
    TOUCH_KILL_1HIT, follow_and_despawn, map_cell, partner_left_free, partner_right_free, solid,
    touch_kill_1hit,
};
use crate::enemies::w5::pacer::{pace, pacer_init};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_free, obj_spawn_at_xy_nodedup, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its animations, left (h0+$328f4), right (h0+$32920) and firing (h0+$3294c).
const LEFT: u32 = h0(0x328f4);
const RIGHT: u32 = h0(0x32920);
const FIRING: u32 = h0(0x3294c);
/// Its steps: walking (h0+$26c08), stopped to fire (h0+$26cb6).
const WALK: u32 = h0(0x26c08);
const FIRE: u32 = h0(0x26cb6);
/// The countdown that never runs out in practice.
const NEVER: u16 = 0x7d00;

/// Template 83: from the right end (marker 21).
pub static SHOOTER_LEFT: ObjectKind = ObjectKind {
    name: "w5 shooter from the right end",
    routines: &[
        (h0(0x26b8e), init_83),
        (WALK, walk),
        (FIRE, fire),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 84: from the left end (marker 20).
pub static SHOOTER_RIGHT: ObjectKind = ObjectKind {
    name: "w5 shooter from the left end",
    routines: &[
        (h0(0x26bce), init_84),
        (WALK, walk),
        (FIRE, fire),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 85: the shot.
pub static SHOT: ObjectKind = ObjectKind {
    name: "w5 shooter's shot",
    routines: &[
        (h0(0x26cfa), shot_init),
        (h0(0x26d14), shot_update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Marker 20 (h0+$18882): once marker 21 is found free to its right, `$1e` down its cell.
pub fn spawn_84(c: &mut LevelCtx, cell: u32) {
    if partner_right_free(&c.bus, cell, 0x15) {
        obj_spawn_cell(c, template(84), cell, 0, 0x1e);
    }
}

/// Marker 21 (h0+$188a0): once marker 20 is found free to its left.
pub fn spawn_83(c: &mut LevelCtx, cell: u32) {
    if partner_left_free(&c.bus, cell, 0x14) {
        obj_spawn_cell(c, template(83), cell, 0, 0x1e);
    }
}

/// h0+$26b8e: `$20` left of its cell, not about to fire.
fn init_83(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pacer_init(c, slot, LEFT);
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_sub(0x20));
    b.set_w(slot + 0x60, NEVER);
}

/// h0+$26bce: firing `$a` passes on (`+$60`), facing right.
fn init_84(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pacer_init(c, slot, RIGHT);
    c.bus.set_w(slot + 0x60, 0xa);
}

/// h0+$26c08: when `+$60` runs out it shows its firing frame (its `+$42` the left walk) and stops
/// for 9 passes ([`fire`]); it paces on this pass all the same, and a turn sets the count again:
/// `$a` going left, never going right.
fn walk(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if t == 0 {
        let bob = b.l(slot + 0x50);
        b.set_l(bob + 0x28, FIRING);
        b.set_l(bob + 0x42, LEFT);
        b.set_l(slot + 4, FIRE);
        b.set_w(slot + 0x60, 9);
    }
    if pace(c, slot, [0x14, 0x15], LEFT, RIGHT) {
        let vx = c.bus.w(slot + 0xc);
        c.bus
            .set_w(slot + 0x60, if n_clear(vx) { NEVER } else { 0xa });
    }
    follow_and_despawn(c, slot);
}

/// h0+$26cb6: stands; when `+$60` runs out it walks again and shoots from 8 right, 8 down.
fn fire(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if t == 0 {
        b.set_l(slot + 4, WALK);
        b.set_w(slot + 0x60, NEVER);
        let (x, y) = (
            b.w(slot + 8).wrapping_add(8),
            b.w(slot + 0xa).wrapping_add(8),
        );
        obj_spawn_at_xy_nodedup(c, template(85), x, y);
    }
    follow_and_despawn(c, slot);
}

/// h0+$26cfa: its look (h0+$32998).
fn shot_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32998));
    c.bus.set_w(slot + 0x5c, 0);
}

/// h0+$26d14: x moves by `+$0c`; every other pass it falls a pixel a pass faster, to `$c`. It is
/// freed in a solid cell (the one under x, y − 8).
fn shot_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    let flip = !b.w(slot + 0x5c);
    b.set_w(slot + 0x5c, flip);
    if flip != 0 && b.w(slot + 0xe) != 0xc {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa).wrapping_sub(8));
    let at = map_cell(b, (x >> 4).wrapping_mul(2), (y >> 4).wrapping_mul(4));
    if solid(b.w(at) & 0x3f) {
        obj_free(c, slot, 0);
        return;
    }
    follow_and_despawn(c, slot);
}
