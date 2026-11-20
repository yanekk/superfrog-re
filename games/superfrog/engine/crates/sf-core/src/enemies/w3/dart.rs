//! Templates 55 (marker 18, flying left) and 56 (marker 19, flying right): `w3_dart`, which waits
//! hidden, then flies from where it spawned until the cell ahead of it is solid, and waits again.
//! It hurts the frog while it flies; waiting, its touch still hurts (`touch_hurt`, passive) until
//! it has hit a wall once. Neither the spud nor a stomp harms it.
//!
//! The update and flight are shared with world 2's darts (templates 53 and 54) and later worlds'.

use crate::enemies::common::{TOUCH_HURT, marker_right_of, solid, touch_hurt};
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, nothing};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `dart_wait` (h0+$25fa8) and `dart_fly` (h0+$25f42).
pub const WAIT: u32 = h0(0x25fa8);
pub const FLY: u32 = h0(0x25f42);
/// Where a hidden dart's bob is put (its y).
const HIDDEN_Y: u16 = 0x2ee0;

pub static DART_LEFT: ObjectKind = ObjectKind {
    name: "w3 dart left",
    routines: &[
        (h0(0x25e62), init_left),
        (WAIT, wait),
        (FLY, fly),
        (TOUCH_HURT, touch_hurt),
        (h0(0x7d4), nothing),
    ],
};

pub static DART_RIGHT: ObjectKind = ObjectKind {
    name: "w3 dart right",
    routines: &[
        (h0(0x25e7e), init_right),
        (WAIT, wait),
        (FLY, fly),
        (TOUCH_HURT, touch_hurt),
        (h0(0x7d4), nothing),
    ],
};

/// `w3_dart_left_init` (h0+$25e62): waits `$20` passes first.
fn init_left(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31f44));
    c.bus.set_w(slot + 0x64, 0x20);
}

/// `w3_dart_right_init` (h0+$25e7e).
fn init_right(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31f50));
    c.bus.set_w(slot + 0x64, 0x20);
}

/// `dart_wait` (h0+$25fa8): passive, its bob hidden, counting `+$64` down; when the count is
/// over it waits `$60` passes next time, and flies from its spawn point, hurting on contact.
pub fn wait(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 1);
    let t = b.w(slot + 0x64).wrapping_sub(1);
    b.set_w(slot + 0x64, t);
    let bob = b.l(slot + 0x50);
    if n_clear(t) {
        b.set_w(bob.wrapping_sub(2), HIDDEN_Y);
        return;
    }
    b.set_w(slot + 0x64, 0x60);
    b.set_w(bob.wrapping_sub(2), HIDDEN_Y);
    b.set_l(slot + 4, FLY);
    b.set_l(slot + 8, b.l(slot + 0x44));
    b.set_l(slot + 0x10, TOUCH_HURT);
}

/// `dart_fly` (h0+$25f42): it hurts on contact; x moves by `+$0c`, and once the cell right of
/// the one at twice its speed ahead, 8 pixels up, is solid it waits again, touching nothing.
/// `+$64` counts down on (unread).
pub fn fly(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 0);
    let vx = b.w(slot + 0xc);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
    bob_follow(b, slot);
    b.set_w(slot + 0x64, b.w(slot + 0x64).wrapping_sub(1));
    let x = b.w(slot + 8).wrapping_add(vx.wrapping_add(vx));
    let y = b.w(slot + 0xa).wrapping_sub(8);
    if solid(marker_right_of(b, x, y)) {
        b.set_l(slot + 4, WAIT);
        b.set_l(slot + 0x10, h0(0x7d4));
    }
}
