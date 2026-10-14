//! Templates 53 (marker 18) and 54 (marker 19): a dart that waits hidden, flies out from its spawn
//! point until it meets a solid cell, and hides again; it hurts (`touch_hurt`) while flying, and
//! on contact with its spawn point while it waits there passive.

use crate::enemies::common::{cell_at, marker_at};
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow};
use crate::level::{LevelCtx, ObjectKind, n_clear};

const FLYING: u32 = h0(0x25f42);
const HIDDEN: u32 = h0(0x25fa8);
/// The y a hidden dart's bob is parked at.
const PARKED_Y: u16 = 0x2ee0;

pub static DART: ObjectKind = ObjectKind {
    name: "dart (marker 18)",
    routines: &[
        (h0(0x25e2a), |c, s, _| dart_init(c, s, h0(0x316f0))),
        (FLYING, flying),
        (HIDDEN, hidden),
    ],
};

pub static DART_B: ObjectKind = ObjectKind {
    name: "dart (marker 19)",
    routines: &[
        (h0(0x25e46), |c, s, _| dart_init(c, s, h0(0x316fc))),
        (FLYING, flying),
        (HIDDEN, hidden),
    ],
};

/// h0+$25e2a / h0+$25e46: `$20` passes before it flies.
fn dart_init(c: &mut LevelCtx, slot: u32, look: u32) {
    bob_anim(&mut c.bus, slot, look);
    c.bus.set_w(slot + 0x64, 0x20);
}

/// h0+$25fa8: passive, its bob parked out of sight; when its wait (`+$64`) runs out it flies from
/// its spawn point (`+$44`), hurting, for `$60`.
fn hidden(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 1);
    let t = b.w(slot + 0x64).wrapping_sub(1);
    b.set_w(slot + 0x64, t);
    let bob = b.l(slot + 0x50);
    b.set_w(bob.wrapping_sub(2), PARKED_Y);
    if !n_clear(t) {
        b.set_w(slot + 0x64, 0x60);
        b.set_l(slot + 4, FLYING);
        b.set_l(slot + 8, b.l(slot + 0x44));
        b.set_l(slot + 0x10, crate::enemies::common::TOUCH_HURT);
    }
}

/// h0+$25f42: `vx` along; once the cell two steps ahead of it, 8 pixels up, is solid (marker
/// `$3a` or above) it hides, harmless, for whatever is left of its `+$64`.
fn flying(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 0);
    let vx = b.w(slot + 0xc);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
    bob_follow(b, slot);
    b.set_w(slot + 0x64, b.w(slot + 0x64).wrapping_sub(1));
    let x = b.w(slot + 8).wrapping_add(vx.wrapping_mul(2));
    let at = cell_at(b, x, b.w(slot + 0xa).wrapping_sub(8)).wrapping_add(2);
    if n_clear(marker_at(b, at).wrapping_sub(0x3a)) {
        b.set_l(slot + 4, HIDDEN);
        b.set_l(slot + 0x10, crate::enemies::common::NOTHING);
    }
}
