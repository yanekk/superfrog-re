//! The shots W6's enemies fire (templates 107–109): they fly straight (107, 109) or fall in an
//! arc (108) until they meet solid ground. 107's and 108's init is shared; each kind's own is in
//! its parent's file (109's in `cannon`).

use crate::enemies::common::{TOUCH_KILL_1HIT, map_cell, solid, touch_kill_1hit};
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_free};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// The straight shots' update (h0+$27e34) and the falling shot's (h0+$27e8e).
pub const SHOT_UPDATE: u32 = h0(0x27e34);
pub const FALLING_SHOT_UPDATE: u32 = h0(0x27e8e);
const SHOT_INIT: u32 = h0(0x27e00);

/// Template 107: the flier's shot.
pub static SHOT: ObjectKind = ObjectKind {
    name: "shot",
    routines: &[
        (SHOT_INIT, shot_init),
        (SHOT_UPDATE, shot_update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 108: the thrower's shot, which falls.
pub static FALLING_SHOT: ObjectKind = ObjectKind {
    name: "falling shot",
    routines: &[
        (SHOT_INIT, shot_init),
        (FALLING_SHOT_UPDATE, falling_shot_update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// h0+$27e00: its look (h0+$32b00).
fn shot_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32b00));
    c.bus.set_w(slot + 0x5c, 0);
}

/// h0+$27e34: gone on solid ground (the cell a row down from its position `$a` up); else it
/// moves by `+$0c`, `+$0e`.
pub fn shot_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    fly(c, slot);
}

/// h0+$27e8e: every other pass (`+$5c` flipped) it falls faster, up to 8; then as
/// [`shot_update`].
pub fn falling_shot_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = !b.w(slot + 0x5c);
    b.set_w(slot + 0x5c, t);
    if t != 0 && !n_clear(b.w(slot + 0xe).wrapping_sub(8)) {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    fly(c, slot);
}

/// h0+$27e34 / h0+$27ea0 on.
fn fly(c: &mut LevelCtx, slot: u32) {
    let b = &mut c.bus;
    let x = b.w(slot + 8);
    let y = b.w(slot + 0xa).wrapping_sub(0xa);
    let cell = map_cell(b, (x >> 4).wrapping_mul(2), (y >> 4).wrapping_mul(4)).wrapping_add(0x262);
    if solid(b, cell) {
        return obj_free(c, slot, 0);
    }
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
