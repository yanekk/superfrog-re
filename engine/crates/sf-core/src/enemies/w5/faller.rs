//! Templates 28 (marker 10) and 30 (marker 12): a block that hangs until the frog comes within
//! `$40` pixels of it across (30 also as soon as it is more than `$20` below the camera's top:
//! in view, in practice), shakes for three passes, then falls until the cell under it is solid and
//! lies there, harmless (passive). It hurts while it hangs and falls; nothing kills it
//! (`touch_kill_1hit` on `$7d00` hit points).

use crate::enemies::common::{
    TOUCH_KILL_1HIT, follow_and_despawn, map_cell, solid, touch_kill_1hit,
};
use crate::frog::{FROG, f};
use crate::items::template;
use crate::layout::h0;
use crate::level::camera::CAMERA_Y;
use crate::level::objects::{bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// 28's steps: hanging (h0+$24f1c), shaking (h0+$24f58), falling (h0+$24f88), down (h0+$24ff8).
const WAIT_28: u32 = h0(0x24f1c);
const SHAKE_28: u32 = h0(0x24f58);
const FALL_28: u32 = h0(0x24f88);
const DOWN_28: u32 = h0(0x24ff8);
/// 30's (h0+$2515c, h0+$251a8, h0+$251d8, h0+$25248).
const WAIT_30: u32 = h0(0x2515c);
const SHAKE_30: u32 = h0(0x251a8);
const FALL_30: u32 = h0(0x251d8);
const DOWN_30: u32 = h0(0x25248);

/// Template 28.
pub static FALLER: ObjectKind = ObjectKind {
    name: "w5 faller (marker 10)",
    routines: &[
        (h0(0x24f06), init_28),
        (WAIT_28, wait_28),
        (SHAKE_28, shake_28),
        (FALL_28, fall_28),
        (DOWN_28, down),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Template 30.
pub static FALLER_B: ObjectKind = ObjectKind {
    name: "w5 faller (marker 12)",
    routines: &[
        (h0(0x25146), init_30),
        (WAIT_30, wait_30),
        (SHAKE_30, shake_30),
        (FALL_30, fall_30),
        (DOWN_30, down),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Marker 10 (h0+$188d0): `$a` down its cell.
pub fn spawn_28(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(28), cell, 0, 0xa);
}

/// Marker 12 (h0+$188f4): `$14` up its cell.
pub fn spawn_30(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(30), cell, 0, 0xffec);
}

/// h0+$24f06: its look (h0+$32778).
fn init_28(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32778));
}

/// h0+$25146: its look (h0+$32794).
fn init_30(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32794));
}

/// Whether the frog is within `$40` pixels across: frog x − `$40` before its x and frog x + `$40`
/// not (as the original's word compares' signs have it).
fn frog_near(c: &LevelCtx, slot: u32) -> bool {
    let b = &c.bus;
    let (fx, x) = (b.w(FROG + f::X), b.w(slot + 8));
    !n_clear(fx.wrapping_sub(0x40).wrapping_sub(x))
        && n_clear(fx.wrapping_add(0x40).wrapping_sub(x))
}

/// Shaking for 3 passes (`+$60` 2) next.
fn start_shaking(c: &mut LevelCtx, slot: u32, shake: u32) {
    let b = &mut c.bus;
    b.set_l(slot + 4, shake);
    b.set_w(slot + 0x5c, 0);
    b.set_w(slot + 0x60, 2);
}

/// h0+$24f1c: hangs until the frog is near.
fn wait_28(c: &mut LevelCtx, slot: u32, _d7: u16) {
    if frog_near(c, slot) {
        start_shaking(c, slot, SHAKE_28);
    }
    follow_and_despawn(c, slot);
}

/// h0+$2515c: hangs until it is below the camera's top + `$20` (the word compare's sign) or the
/// frog is near.
fn wait_30(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let low = c.bus.w(CAMERA_Y + 2).wrapping_add(0x20);
    if !n_clear(low.wrapping_sub(c.bus.w(slot + 0xa))) || frog_near(c, slot) {
        start_shaking(c, slot, SHAKE_30);
    }
    follow_and_despawn(c, slot);
}

/// h0+$24f58 and h0+$251a8: a pixel left and right in turn; once `+$60` goes below 0, `fall`.
fn shake(c: &mut LevelCtx, slot: u32, fall: u32) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if !n_clear(t) {
        b.set_l(slot + 4, fall);
        b.set_w(slot + 0x5c, 0);
    }
    b.set_w(slot + 8, b.w(slot + 8).wrapping_sub(1));
    let flip = !b.w(slot + 0x5c);
    b.set_w(slot + 0x5c, flip);
    if flip != 0 {
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(2));
    }
    follow_and_despawn(c, slot);
}

fn shake_28(c: &mut LevelCtx, slot: u32, _d7: u16) {
    shake(c, slot, FALL_28);
}

fn shake_30(c: &mut LevelCtx, slot: u32, _d7: u16) {
    shake(c, slot, FALL_30);
}

/// The fall's end: y moves by `+$0e`; once the cell below the one under x, y is solid, its bob's
/// step timer is cleared and it lies `dy` from the top of its cell, `down` from then on.
fn land(c: &mut LevelCtx, slot: u32, down: u32, dy: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa));
    let at = map_cell(b, (x >> 4).wrapping_mul(2), (y >> 4).wrapping_mul(4));
    if solid(b.w(at.wrapping_add(0x260)) & 0x3f) {
        let bob = b.l(slot + 0x50);
        b.set_w(bob + 0x1a, 0);
        b.set_l(slot + 4, down);
        b.set_w(slot + 0xa, (b.w(slot + 0xa) & 0xfff0).wrapping_add(dy));
    }
    follow_and_despawn(c, slot);
}

/// h0+$24f88: falls a pixel a pass faster every other pass, to `$e`.
fn fall_28(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let flip = !b.w(slot + 0x5c);
    b.set_w(slot + 0x5c, flip);
    if flip != 0 && b.w(slot + 0xe) != 0xe {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    land(c, slot, DOWN_28, 0);
}

/// h0+$251d8: falls a pixel a pass faster every pass, to `$e`, and lies 3 pixels higher.
fn fall_30(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if b.w(slot + 0xe) != 0xe {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    land(c, slot, DOWN_30, 0xfffd);
}

/// h0+$24ff8 and h0+$25248: lies harmless.
fn down(c: &mut LevelCtx, slot: u32, _d7: u16) {
    c.bus.set_w(slot + 0x30, 1);
    follow_and_despawn(c, slot);
}
