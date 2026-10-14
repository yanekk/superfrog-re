//! Template 99 (marker 23): `w3_drip`, a drop that waits hidden, then falls from its spawn point
//! until it reaches a cell marked `$3d` or above, and waits again a random 8–12 passes. It hurts
//! on any touch; not stompable, and the spud passes over it (`touch_spud_damage`).

use crate::enemies::common::{
    TOUCH_SPUD_DAMAGE, follow_and_despawn, marker_right_of, touch_spud_damage,
};
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Where a waiting drop is put (x and y).
const HIDDEN: u16 = 0x6d60;

pub static DRIP: ObjectKind = ObjectKind {
    name: "w3 drip",
    routines: &[
        (h0(0x287b8), init),
        (h0(0x287e0), update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// `w3_drip_init` (h0+$287b8): hidden, falling in 6 passes.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, HIDDEN);
    b.set_w(slot + 0xa, HIDDEN);
    b.set_w(slot + 0x64, 6);
    bob_anim(b, slot, h0(0x31ef4));
}

/// `w3_drip_update` (h0+$287e0): `+$64` counts down; at 0 the drop starts 4 right of its spawn
/// point at 2 pixels a pass, and below 0 it falls, a pixel a pass faster up to 6. Once the cell
/// right of the one under it is marked `$3d` or above it is hidden again. Only a falling drop is
/// despawned.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x64).wrapping_sub(1);
    b.set_w(slot + 0x64, t);
    if n_clear(t.wrapping_sub(1)) {
        return bob_follow(b, slot);
    }
    if n_clear(t) {
        b.set_l(slot + 8, b.l(slot + 0x44));
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(6).wrapping_sub(2));
        b.set_w(slot + 0xe, 2);
    }
    if !n_clear(b.w(slot + 0xe).wrapping_sub(6)) {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa));
    if n_clear(marker_right_of(b, x, y).wrapping_sub(0x3d)) {
        let wait = c.rng_range(4).wrapping_add(8);
        let b = &mut c.bus;
        b.set_w(slot + 0x64, wait);
        b.set_w(slot + 8, HIDDEN);
        b.set_w(slot + 0xa, HIDDEN);
        return bob_follow(b, slot);
    }
    follow_and_despawn(c, slot);
}
