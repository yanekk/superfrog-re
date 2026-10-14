//! World 3's walkers, which walk their row turning at cells marked `$25` (looking from x + 3).
//! Template 91 (marker 26): `w3_stompable`: a stomp flattens it (`touch_spud_damage` on `$1f` hit
//! points) and it becomes a reward. Templates 96 (marker 25) and 97 (marker 13): `w3_spiker`,
//! which hurts on any touch: not stompable, and the spud passes over it; 97's is never despawned
//! (evictable).

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, follow_and_despawn, row_base, touch_spud_damage};
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow};
use crate::level::{LevelCtx, ObjectKind, n_clear};
use crate::mem::Bus;

/// The stompable's animations, left (h0+$31d0c) and right (h0+$31cc8); the spiker's (h0+$31d94,
/// h0+$31d50).
const STOMPABLE_LEFT: u32 = h0(0x31d0c);
const STOMPABLE_RIGHT: u32 = h0(0x31cc8);
const SPIKER_LEFT: u32 = h0(0x31d94);
const SPIKER_RIGHT: u32 = h0(0x31d50);

pub static STOMPABLE: ObjectKind = ObjectKind {
    name: "w3 stompable",
    routines: &[
        (h0(0x27c90), stompable_init),
        (h0(0x27cc0), stompable_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

pub static SPIKER: ObjectKind = ObjectKind {
    name: "w3 spiker",
    routines: &[
        (h0(0x27d1c), spiker_init),
        (h0(0x27d4c), spiker_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

pub static SPIKER_STAYING: ObjectKind = ObjectKind {
    name: "w3 spiker (staying)",
    routines: &[
        (h0(0x27d1c), spiker_init),
        (h0(0x27da8), spiker_staying_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Whether the cell on the walker's row ahead of x + 3 is marked `$25` (`+$64` its row's base, a
/// row down: the cell read is `$25e` before).
pub fn at_turn(b: &Bus, slot: u32) -> bool {
    let x = b.w(slot + 8).wrapping_add(3);
    let at = ((x >> 4).wrapping_add(x >> 4) as u32).wrapping_add(b.l(slot + 0x64));
    b.w(at.wrapping_sub(0x25e)) & 0x3f == 0x25
}

/// x moves by `+$0c`; at a turn it turns, taking `left` going left and `right` going right.
pub(super) fn walk(b: &mut Bus, slot: u32, left: u32, right: u32) {
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    if at_turn(b, slot) {
        let vx = b.w(slot + 0xc).wrapping_neg();
        b.set_w(slot + 0xc, vx);
        bob_anim(b, slot, if n_clear(vx) { right } else { left });
    }
}

/// `w3_stompable_init` (h0+$27c90).
fn stompable_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, STOMPABLE_LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
}

/// `w3_stompable_update` (h0+$27cc0).
fn stompable_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    walk(&mut c.bus, slot, STOMPABLE_LEFT, STOMPABLE_RIGHT);
    follow_and_despawn(c, slot);
}

/// `w3_spiker_init` (h0+$27d1c).
fn spiker_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, SPIKER_LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
}

/// `w3_spiker_update` (h0+$27d4c).
fn spiker_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    walk(&mut c.bus, slot, SPIKER_LEFT, SPIKER_RIGHT);
    follow_and_despawn(c, slot);
}

/// `w3_spiker_staying_update` (h0+$27da8): never despawned.
fn spiker_staying_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    walk(b, slot, SPIKER_LEFT, SPIKER_RIGHT);
    bob_follow(b, slot);
}
