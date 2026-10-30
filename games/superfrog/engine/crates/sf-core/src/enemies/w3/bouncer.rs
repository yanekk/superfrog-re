//! Template 88 (marker 16): `w3_bouncer`, which falls and bounces up again off solid ground
//! three cells under it, forever, where it spawned. It hurts on any touch; a hit or stomp cannot
//! kill it (`touch_kill_1hit` on `$7d00` hit points). It is never despawned (evictable).

use crate::enemies::common::{TOUCH_KILL_1HIT, marker_at, solid, touch_kill_1hit};
use crate::layout::h0;
use crate::level::objects::bob_follow;
use crate::level::{LevelCtx, ObjectKind};

/// Its animation once bouncing (h0+$31c38); the first pass draws h0+$31c48's start.
const BOUNCE: u32 = h0(0x31c38);

pub static BOUNCER: ObjectKind = ObjectKind {
    name: "w3 bouncer",
    routines: &[
        (h0(0x25c8a), init),
        (h0(0x25caa), update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// `w3_bouncer_init` (h0+$25c8a): rising at 16 pixels a pass.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, h0(0x31c48));
    b.set_l(bob + 0x42, BOUNCE);
    b.set_w(slot + 0x64, 0);
    b.set_w(slot + 0xe, 0xfff0);
}

/// `w3_bouncer_update` (h0+$25caa): once the cell three below the one under (x + 16, y) is solid
/// its speed turns (the animation restarts), else it falls a pixel a pass faster; it moves.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let x = b.w(slot + 8).wrapping_add(0x10);
    let y = b.w(slot + 0xa);
    let cols = ((x >> 4).wrapping_add(x >> 4)) as u32;
    let row4 = (y >> 4).wrapping_mul(4);
    if solid(marker_at(b, cols, row4, 0x720)) {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_neg());
        let bob = b.l(slot + 0x50);
        b.set_l(bob + 0x28, BOUNCE);
        b.set_l(bob + 0x42, BOUNCE);
    } else {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    bob_follow(b, slot);
}
