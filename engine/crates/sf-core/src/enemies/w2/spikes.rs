//! Templates 71 (marker 16) and 72 (marker 20): a hazard that rises and falls in a cycle,
//! passive: touching it hurts the frog only on the frames it is up. Each starts its cycle in step
//! with a live neighbour of its kind two cells left or right, so a row of them moves as a wave.

use crate::enemies::w2::marker;
use crate::layout::h0;
use crate::level::objects::OBJ_SLOTS;
use crate::level::{LevelCtx, ObjectKind};

const STILL: u32 = h0(0x26692);
const TOUCH: u32 = h0(0x237c0);
/// The frames on which it hurts (h0+$2dbec … $2dd3c, `$30` apart).
const UP_FRAMES: [u32; 8] = [
    h0(0x2dbec),
    h0(0x2dc1c),
    h0(0x2dc4c),
    h0(0x2dc7c),
    h0(0x2dcac),
    h0(0x2dcdc),
    h0(0x2dd0c),
    h0(0x2dd3c),
];

pub static SPIKES: ObjectKind = ObjectKind {
    name: "spikes (marker 16)",
    routines: &[
        (h0(0x2652c), |c, s, _| {
            spikes_init(c, s, 0x10, h0(0x31708), true)
        }),
        (STILL, crate::enemies::w2::backdrop::still),
        (TOUCH, touch_spikes),
    ],
};

pub static SPIKES_B: ObjectKind = ObjectKind {
    name: "spikes (marker 20)",
    routines: &[
        (h0(0x265e0), |c, s, _| {
            spikes_init(c, s, 0x14, h0(0x317e0), false)
        }),
        (STILL, crate::enemies::w2::backdrop::still),
        (TOUCH, touch_spikes),
    ],
};

/// h0+$2652c (marker 16, its cycle `anim`) / h0+$265e0 (marker 20): the cycle starts where a live
/// neighbour's is: with marker `m` two cells left (then right) holding a live slot, its bob's
/// step (`+$28` while still to start, else `+$16`) less its start (`+$42`) gives the offset. The
/// marker 16 hazard is `$20` bytes ahead of a left neighbour and `$10` behind a right one;
/// marker 20's the other way round. Ahead past the cycle's end (a negative long) wraps back
/// `$98`; behind before its start wraps from its end.
fn spikes_init(c: &mut LevelCtx, slot: u32, m: u16, anim: u32, left_ahead: bool) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x42, anim);
    let mut start = anim;
    let cell = b.l(slot + 0x40);
    for (at, ahead) in [
        (cell.wrapping_sub(4), left_ahead),
        (cell.wrapping_add(4), !left_ahead),
    ] {
        if marker(b, at) != m {
            if at == cell.wrapping_sub(4) {
                continue;
            }
            break;
        }
        let Some(other) = (0..20)
            .map(|k| OBJ_SLOTS + k * 0x7c)
            .find(|&s| b.l(s + 0x40) == at)
        else {
            continue;
        };
        let ob = b.l(other + 0x50);
        // With no `+$42` the original subtracts whatever `d1` held; every live bob has one.
        let from = b.l(ob + 0x42);
        let step = match b.l(ob + 0x28) {
            0 => b.l(ob + 0x16),
            s => s,
        };
        let d = step.wrapping_sub(from);
        if ahead {
            start = anim.wrapping_add(d.wrapping_add(0x20));
            if (b.l(start) as i32) < 0 {
                start = start.wrapping_sub(0x98);
            }
        } else {
            let d = d.wrapping_sub(0x10);
            start = if (d as i32) >= 0 {
                anim.wrapping_add(d)
            } else {
                anim.wrapping_add(0x98).wrapping_sub(d.wrapping_neg())
            };
        }
        break;
    }
    b.set_l(bob + 0x28, start);
}

/// h0+$237c0: the frog is hurt if the bob shows one of the up frames.
fn touch_spikes(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &c.bus;
    let frame = b.l(b.l(b.l(slot + 0x50) + 0x16));
    if UP_FRAMES.contains(&frame) {
        crate::level::death::frog_hurt(c, slot);
    }
}
