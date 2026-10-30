//! Template 94 (marker 21): `w3_hopper`, which walks its row bobbing on a short path, turning at
//! cells marked `$25`; going right, 8 passes after it last went left, it leaps now and then (three
//! times in four). It hurts on any touch; a hit or stomp cannot kill it (`touch_kill_1hit` on
//! `$7d00` hit points).

use crate::enemies::common::{TOUCH_KILL_1HIT, follow_and_despawn, row_base, touch_kill_1hit};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its bob (h0+$285f0): y offsets, ending at `$80`.
pub const PATH: u32 = h0(0x285f0);
/// `w3_hopper_update` (h0+$2861e) and `w3_hopper_leap` (h0+$286ee).
const UPDATE: u32 = h0(0x2861e);
const LEAP: u32 = h0(0x286ee);
/// Its animations, walking left (h0+$3208c) and right (h0+$320b0), leaping (h0+$320d4).
const LEFT: u32 = h0(0x3208c);
const RIGHT: u32 = h0(0x320b0);

pub static HOPPER: ObjectKind = ObjectKind {
    name: "w3 hopper",
    routines: &[
        (h0(0x285a2), init),
        (UPDATE, update),
        (LEAP, leap),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// `w3_hopper_init` (h0+$285a2): its row's base (`+$64`), its path from the start, its ground y
/// (`+$70`).
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
    b.set_l(slot + 0x68, PATH);
    b.set_l(slot + 0x6c, PATH);
    b.set_w(slot + 0x70, b.w(slot + 0xa));
    b.set_w(slot + 0x60, 0);
    b.set_w(slot + 0x5c, 0);
}

/// `w3_hopper_update` (h0+$2861e): y is its ground's plus the path's next offset. Going right
/// it counts passes (`+$60`; going left clears it), and at the 8th leaps unless the RNG says 0
/// of 0..3: up at 8 pixels a pass (it still moves this pass). x moves by `+$0c`, turning at a
/// cell marked `$25` ahead of x + 3.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let mut at = b.l(slot + 0x68);
    if b.w(at) == 0x80 {
        at = b.l(slot + 0x6c);
        b.set_l(slot + 0x68, at);
    }
    b.set_l(slot + 0x68, b.l(slot + 0x68).wrapping_add(2));
    b.set_w(slot + 0xa, b.w(slot + 0x70).wrapping_add(b.w(at)));
    if !n_clear(b.w(slot + 0xc)) {
        b.set_w(slot + 0x60, 0);
    } else {
        let n = b.w(slot + 0x60).wrapping_add(1);
        b.set_w(slot + 0x60, n);
        if n == 8 && n_clear(c.rng_range(3).wrapping_sub(1)) {
            let b = &mut c.bus;
            b.set_l(slot + 4, LEAP);
            let bob = b.l(slot + 0x50);
            b.set_l(bob + 0x28, h0(0x320d4));
            b.set_w(slot + 0xe, 0xfff8);
            b.set_w(slot + 0x5c, 0);
        }
    }
    super::walkers::walk(&mut c.bus, slot, LEFT, RIGHT);
    follow_and_despawn(c, slot);
}

/// `w3_hopper_leap` (h0+$286ee): once falling at 8 it walks again from the next pass; every third
/// pass it drifts a pixel right and falls a pixel a pass faster; it moves.
fn leap(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if n_clear(b.w(slot + 0xe).wrapping_sub(8)) {
        b.set_l(slot + 4, UPDATE);
    }
    let n = b.w(slot + 0x5c).wrapping_add(1);
    b.set_w(slot + 0x5c, n);
    if n_clear(n.wrapping_sub(3)) {
        b.set_w(slot + 0x5c, 0);
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(1));
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    follow_and_despawn(c, slot);
}
