//! Template 102 (marker 13): it paces its ground (turning at the turning marker), and after 8
//! passes walking left it may (the RNG's 1..=5 of 0..=5) stop and attack: while its attack
//! animation plays, a frog standing up to `$80` pixels left of it and level with it is hurt.
//! A stomp stuns it (`touch_spud_damage`, 1 hit point); then it can be taken as a reward.

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, row_base, touch_spud_damage};
use crate::enemies::w6::walker::at_turn;
use crate::frog::{FROG, f};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

const PACE: u32 = h0(0x28488);
const ATTACK: u32 = h0(0x28532);

/// Template 102.
pub static CHARGER: ObjectKind = ObjectKind {
    name: "charger",
    routines: &[
        (h0(0x28458), charger_init),
        (PACE, charger_pace),
        (ATTACK, charger_attack),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 13 (h0+$1893a): a charger, 7 down its cell.
pub fn spawn_102(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(102), cell, 0, 7);
}

/// h0+$28458: its look (h0+$32a38); `+$64` the start of the row below its cell.
fn charger_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x32a38));
    let base = row_base(b, slot, 0x260);
    b.set_l(slot + 0x64, base);
}

/// h0+$28488: `+$60` counts the passes it walks left (cleared walking right); at the 8th the
/// RNG may start the attack (h0+$32a90, not stompable). It moves `+$0c` and turns (h0+$32a38
/// left, h0+$32a64 right).
fn charger_pace(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if n_clear(b.w(slot + 0xc)) {
        b.set_w(slot + 0x60, 0);
    } else {
        let n = b.w(slot + 0x60).wrapping_add(1);
        b.set_w(slot + 0x60, n);
        if n == 8 && n_clear(c.rng_range(5).wrapping_sub(1)) {
            let b = &mut c.bus;
            b.set_w(slot + 0x2e, 0);
            b.set_l(slot + 4, ATTACK);
            let bob = b.l(slot + 0x50);
            b.set_l(bob + 0x28, h0(0x32a90));
            b.set_w(slot + 0x5c, 0);
        }
    }
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    if at_turn(c, slot) {
        let b = &mut c.bus;
        let vx = b.w(slot + 0xc).wrapping_neg();
        b.set_w(slot + 0xc, vx);
        let look = if vx & 0x8000 != 0 {
            h0(0x32a38)
        } else {
            h0(0x32a64)
        };
        bob_anim(b, slot, look);
    }
    crate::level::objects::bob_follow(&mut c.bus, slot);
}

/// h0+$28532: a frog on the ground, left of it by less than `$80`, its feet (y + `$39`) at or
/// below the charger's top and less than `$20` lower, is hurt. Once the animation is over
/// (`+$1a` at `$7530`) it paces again, stompable.
fn charger_attack(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &c.bus;
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa));
    let fx = b.w(FROG + f::X);
    let feet = b.w(FROG + f::Y).wrapping_add(0x39);
    if b.w(FROG + f::ON_GROUND) != 0
        && !n_clear(fx.wrapping_sub(x))
        && !n_clear(x.wrapping_sub(fx).wrapping_sub(0x80))
        && n_clear(feet.wrapping_sub(y))
        && !n_clear(feet.wrapping_sub(0x20).wrapping_sub(y))
    {
        crate::level::death::frog_hurt(c, slot);
    }
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    if n_clear(b.w(bob + 0x1a).wrapping_sub(0x7530)) {
        b.set_w(slot + 0x2e, 1);
        b.set_w(bob + 0x1a, 0);
        b.set_l(slot + 4, PACE);
    }
    b.set_l(bob.wrapping_sub(4), b.l(slot + 8));
}
