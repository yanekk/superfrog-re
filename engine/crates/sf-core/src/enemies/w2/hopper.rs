//! Template 40 (marker 11): sits, and now and then (one time in five, once its sitting frame
//! shows) jumps, alternately left and right; stompable while sitting and on its left jumps. The
//! spud or a stomp stuns it (`touch_spud_damage`) and a second stomp makes it a reward.

use crate::enemies::common::{cell_at, follow_and_despawn};
use crate::enemies::w2::marker;
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

const SIT: u32 = h0(0x31614);
/// The sitting animation's step where it may jump.
const SIT_READY: u32 = h0(0x3162c);
const LAND: u32 = h0(0x31624);
const JUMP: u32 = h0(0x31678);
const SITTING: u32 = h0(0x25d30);
const JUMPING: u32 = h0(0x25d9a);

pub static HOPPER: ObjectKind = ObjectKind {
    name: "hopper",
    routines: &[
        (h0(0x25d10), hopper_init),
        (SITTING, sitting),
        (JUMPING, jumping),
    ],
};

/// h0+$25d10: sitting, two passes to wait (`+$5c`), its next jump left (`+$60` 0).
fn hopper_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, SIT);
    b.set_w(slot + 0x5c, 2);
    b.set_w(slot + 0x60, 0);
}

/// h0+$25d30: on the ready step, once its wait is over, `rng_range($28)` of `$20` or more jumps
/// (`vy` −12, `vx` −1, or +1 every other jump, which is not stompable).
fn sitting(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let bob = c.bus.l(slot + 0x50);
    if c.bus.l(bob + 0x16) == SIT_READY {
        let wait = c.bus.w(slot + 0x5c).wrapping_sub(1);
        c.bus.set_w(slot + 0x5c, wait);
        if !n_clear(wait) && n_clear(c.rng_range(0x28).wrapping_sub(0x20)) {
            let b = &mut c.bus;
            b.set_l(bob + 0x28, JUMP);
            b.set_w(slot + 0xe, 0xfff4);
            b.set_l(slot + 4, JUMPING);
            b.set_w(slot + 0xc, 0xffff);
            let side = !b.w(slot + 0x60);
            b.set_w(slot + 0x60, side);
            if side != 0 {
                b.set_w(slot + 0xc, 1);
                b.set_w(slot + 0x2e, 0);
            }
        }
    }
    follow_and_despawn(c, slot);
}

/// h0+$25d9a: flies (`vy` one more each pass) until the cell below its foot (10 pixels left of
/// its x) is solid (marker `$3a` or above): it lands on that row's top, 4 pixels up, sitting
/// again, stompable, two passes to wait.
fn jumping(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    let below = cell_at(b, b.w(slot + 8).wrapping_sub(0xa), b.w(slot + 0xa)).wrapping_add(0x260);
    if n_clear(marker(b, below).wrapping_sub(0x3a)) {
        b.set_l(slot + 4, SITTING);
        let bob = b.l(slot + 0x50);
        b.set_l(bob + 0x28, LAND);
        b.set_l(bob + 0x42, SIT);
        b.set_w(slot + 0xa, (b.w(slot + 0xa) & 0xfff0).wrapping_add(0xfffc));
        b.set_w(slot + 0xe, 0);
        b.set_w(slot + 0x5c, 2);
        b.set_w(slot + 0x2e, 1);
    }
    follow_and_despawn(c, slot);
}
