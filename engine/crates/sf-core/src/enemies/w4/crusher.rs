//! Templates 141 (marker 15) and 142 (marker 16): a crusher that waits harmless for `$15`
//! passes, strikes `$30` pixels in three passes (141 down, 142 up), then creeps back two pixels a
//! pass for `$18` passes, over and over. Out of the spud's reach (`$7d00`).

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, touch_spud_damage};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// 141's steps: waiting (h0+$26f80), striking (h0+$26faa), going back (h0+$26fd8).
const DOWN_WAIT: u32 = h0(0x26f80);
const DOWN_STRIKE: u32 = h0(0x26faa);
const DOWN_BACK: u32 = h0(0x26fd8);
/// 142's (h0+$27006, h0+$27030, h0+$2705e).
const UP_WAIT: u32 = h0(0x27006);
const UP_STRIKE: u32 = h0(0x27030);
const UP_BACK: u32 = h0(0x2705e);
const INIT: u32 = h0(0x26F64);

/// Template 141: strikes down.
pub static CRUSHER_DOWN: ObjectKind = ObjectKind {
    name: "crusher (down)",
    routines: &[
        (INIT, crusher_init),
        (DOWN_WAIT, down_wait),
        (DOWN_STRIKE, down_strike),
        (DOWN_BACK, down_back),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Template 142: strikes up.
pub static CRUSHER_UP: ObjectKind = ObjectKind {
    name: "crusher (up)",
    routines: &[
        (INIT, crusher_init),
        (UP_WAIT, up_wait),
        (UP_STRIKE, up_strike),
        (UP_BACK, up_back),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 15 (h0+$18720): 2 left and `$c` down its cell.
pub fn spawn_141(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(141), cell, 0xfffe, 0xc);
}

/// Marker 16 (h0+$18732): 2 left and 4 up its cell.
pub fn spawn_142(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(142), cell, 0xfffe, 0xfffc);
}

/// h0+$26f64: its look (h0+$324ec); `$14` passes' wait.
fn crusher_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x324ec));
    c.bus.set_w(slot + 0x5c, 0x14);
}

/// One step of the cycle: once `+$5c` runs out, `next` with `count`; this pass it is harmless or
/// not (`passive`) and moves `dy`.
fn step(c: &mut LevelCtx, slot: u32, next: u32, count: u16, passive: u16, dy: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if !n_clear(t) {
        b.set_l(slot + 4, next);
        b.set_w(slot + 0x5c, count);
    }
    b.set_w(slot + 0x30, passive);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(dy));
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

fn down_wait(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, DOWN_STRIKE, 2, 1, 0);
}

fn down_strike(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, DOWN_BACK, 0x17, 0, 0x10);
}

fn down_back(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, DOWN_WAIT, 0x14, 0, 0xfffe);
}

fn up_wait(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, UP_STRIKE, 2, 1, 0);
}

fn up_strike(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, UP_BACK, 0x17, 0, 0xfff0);
}

fn up_back(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, UP_WAIT, 0x14, 0, 2);
}
