//! Template 37 (marker 21): flies left and right in turn (`$40` passes each way) on a wave about
//! its spawn height, standing still on the wave's low part; now and then it stops to show its
//! other animation (a wider hitbox), then flies on. It hurts on contact and nothing kills it; it
//! is never freed but to make room.

use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `swooper_wave` (h0+$257c4): y offsets a word apart, ending at `$80`.
pub const WAVE: u32 = h0(0x257c4);
const FLY: u32 = h0(0x31b7c);
const PAUSE: u32 = h0(0x31bd0);
const FLYING: u32 = h0(0x257e4);
const PAUSED: u32 = h0(0x25874);
const HITBOXES: u32 = h0(0x2356e);
const PAUSE_HITBOXES: u32 = h0(0x23578);

pub static SWOOPER: ObjectKind = ObjectKind {
    name: "swooper",
    routines: &[
        (h0(0x25786), swooper_init),
        (FLYING, flying),
        (PAUSED, paused),
    ],
};

/// h0+$25786: `+$5c` its spawn height, the wave from its start, `$40` passes before it may
/// pause (`+$60`), `vx` −2 for `$40` passes (`+$6c`).
fn swooper_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, FLY);
    b.set_w(slot + 0x5c, b.w(slot + 0xa));
    b.set_l(slot + 0x64, WAVE);
    b.set_l(slot + 0x68, WAVE);
    b.set_w(slot + 0x60, 0x40);
    b.set_w(slot + 0xc, 0xfffe);
    b.set_w(slot + 0x6c, 0x40);
}

/// h0+$257e4: the wave's next height; once `+$60` is spent and its animation is at its start it
/// pauses (keeping its turn count in `+$70`); every `$40` passes it turns; it moves only where
/// the wave's next offset is below 9.
fn flying(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let mut at = b.l(slot + 0x64);
    if b.w(at) == 0x80 {
        at = b.l(slot + 0x68);
        b.set_l(slot + 0x64, at);
    }
    b.set_l(slot + 0x64, b.l(slot + 0x64).wrapping_add(2));
    b.set_w(slot + 0xa, b.w(slot + 0x5c).wrapping_add(b.w(at)));
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if !n_clear(t) {
        let bob = b.l(slot + 0x50);
        if b.l(bob + 0x16) == b.l(bob + 8) {
            b.set_l(bob + 0x28, PAUSE);
            b.set_l(bob + 0x42, FLY);
            b.set_l(slot + 4, PAUSED);
            b.set_l(slot + 0x14, PAUSE_HITBOXES);
            b.set_w(slot + 0x70, b.w(slot + 0x6c));
            b.set_w(slot + 0x60, 2);
        }
    }
    let turn = b.w(slot + 0x6c).wrapping_sub(1);
    b.set_w(slot + 0x6c, turn);
    if !n_clear(turn) {
        b.set_w(slot + 0x6c, 0x40);
        b.set_w(slot + 0xc, b.w(slot + 0xc).wrapping_neg());
    }
    let next = b.w(b.l(slot + 0x64));
    let dx = if n_clear(next.wrapping_sub(9)) {
        0
    } else {
        b.w(slot + 0xc)
    };
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(dx));
    bob_follow(b, slot);
}

/// h0+$25874: once the pause animation is back at its start it flies on, its wave one step in,
/// `$30`–`$50` passes before it may pause again (`rng_range($20)`), its turn count restored.
fn paused(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let bob = c.bus.l(slot + 0x50);
    if c.bus.l(bob + 0x16) == c.bus.l(bob + 8) {
        let b = &mut c.bus;
        bob_anim(b, slot, FLY);
        b.set_l(slot + 0x64, WAVE + 2);
        b.set_l(slot + 0x68, WAVE);
        b.set_l(slot + 4, FLYING);
        b.set_l(slot + 0x14, HITBOXES);
        let t = c.rng_range(0x20).wrapping_add(0x30);
        let b = &mut c.bus;
        b.set_w(slot + 0x60, t);
        b.set_w(slot + 0x6c, b.w(slot + 0x70));
    }
    bob_follow(&mut c.bus, slot);
}
