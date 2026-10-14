//! Template 65 (marker 27): `w3_swing`, a ball swinging on a path round a point above it, and
//! template 66: the five links of its chain, which the marker's stub spawns in the cells below
//! the ball's and which keep themselves evenly spaced between the ball and the chain's top. The
//! ball hurts on any touch (`touch_hurt`); neither the spud nor a stomp harms it. It is freed once
//! fewer than five links are out, a link once its ball is freed.
//!
//! The update of both is shared with world 2's (templates 63, 64) and later worlds' swings.

use crate::enemies::common::{TOUCH_HURT, touch_hurt};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    OBJ_SLOTS, bob_anim, bob_follow, nothing, obj_free, obj_path_xy, obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `chain_link_count` (h0+$262ba, LAYOUT): the index the next link spawned takes, 1..5 (a long
/// inside the code).
pub const LINK_INDEX: u32 = h0(0x262ba);
/// `swing_path` (h0+$26398): pixel offsets (x, y words) of one swing there and back, a −1 long
/// after the last.
pub const PATH: u32 = h0(0x26398);
/// `swing_update` (h0+$2619c) and `swing_link_update` (h0+$262be).
pub const UPDATE: u32 = h0(0x2619c);
pub const LINK_UPDATE: u32 = h0(0x262be);

/// The links' animations in every world (h0+$31560, $31c54, $32bb8, $3227c), which a ball counts.
const LINK_ANIMS: [u32; 4] = [h0(0x31560), h0(0x31c54), h0(0x32bb8), h0(0x3227c)];
/// The balls' animations in every world (h0+$32288, $3156c, $31c60, $32bc4), which a link looks for.
const BALL_ANIMS: [u32; 4] = [h0(0x32288), h0(0x3156c), h0(0x31c60), h0(0x32bc4)];

pub static BALL: ObjectKind = ObjectKind {
    name: "w3 swing",
    routines: &[
        (h0(0x2607c), init),
        (UPDATE, update),
        (TOUCH_HURT, touch_hurt),
    ],
};

pub static LINK: ObjectKind = ObjectKind {
    name: "w3 swing link",
    routines: &[
        (h0(0x26260), link_init),
        (LINK_UPDATE, link_update),
        (h0(0x7d4), nothing),
    ],
};

/// Marker 27 (h0+$183a8): the ball, `$c` down its cell, then a link in each of the five cells
/// below, numbered 1..5 ([`LINK_INDEX`], left at 5), each told its ball (`+$64`). A cell that
/// already has a live slot gets that slot told; with no room, the long at address `$64` is (the
/// original's).
pub fn spawn(c: &mut LevelCtx, cell: u32) {
    let r = obj_spawn_cell(c, template(65), cell, 0, 0xc);
    if r.d0 != 0 {
        return;
    }
    let ball = r.a0;
    c.bus.set_l(LINK_INDEX, 1);
    let mut at = cell;
    for k in 0..5 {
        at = at.wrapping_add(0x260);
        let r = obj_spawn_cell(c, template(66), at, 0, 0);
        if r.d0 == 0 {
            c.bus.set_l(r.a0.wrapping_add(0x64), ball);
        }
        if k < 4 {
            c.bus.set_l(LINK_INDEX, c.bus.l(LINK_INDEX).wrapping_add(1));
        }
    }
}

/// `w3_swing_init` (h0+$2607c): the path round `+$64` (where it spawned); `+$6c` the ball's
/// end of the chain, `$40` below; `+$70` the chain's top, 2 left of and 4 below its spawn point.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x31c60));
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x58, PATH);
    b.set_l(slot + 0x54, PATH);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(0x40));
    b.set_l(slot + 0x70, b.l(slot + 0x44));
    b.set_w(slot + 0x72, b.w(slot + 0x72).wrapping_add(4));
    b.set_w(slot + 0x70, b.w(slot + 0x70).wrapping_sub(2));
}

/// `swing_update` (h0+$2619c): the path's next offset from `$50` left of `+$64`; the chain's end
/// (`+$6c`) 8 right of and 4 below the ball. Unless five or more of the 20 slots' bobs (live or
/// not) show or are to start a link's animation, the ball is freed.
pub fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (x, y) = (b.w(slot + 0x64).wrapping_sub(0x50), b.w(slot + 0x66));
    obj_path_xy(b, slot, x, y);
    bob_follow(b, slot);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(4));
    b.set_w(slot + 0x6c, b.w(slot + 0x6c).wrapping_add(8));
    let links = (0..20u32)
        .map(|k| b.l(OBJ_SLOTS + k * 0x7c + 0x50))
        .filter(|&bob| {
            LINK_ANIMS.contains(&b.l(bob.wrapping_add(8)))
                || LINK_ANIMS.contains(&b.l(bob.wrapping_add(0x42)))
        })
        .count() as u16;
    if !n_clear(links.wrapping_sub(5)) {
        obj_free(c, slot, 0);
    }
}

/// `w3_swing_link_init` (h0+$26260): its index along the chain (`+$68`, a long).
fn link_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x31c54));
    b.set_l(slot + 0x68, b.l(LINK_INDEX));
}

/// `swing_link_update` (h0+$262be): freed once its ball (`+$64`) is not live or its bob neither
/// shows nor is to start a ball's animation; else it sits `index` sixths of the way from the
/// chain's top to its end, each axis.
pub fn link_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let ball = b.l(slot + 0x64);
    if b.l(ball.wrapping_add(0x40)) == 0 {
        return obj_free(c, slot, 0);
    }
    let bob = b.l(ball.wrapping_add(0x50));
    if !BALL_ANIMS.contains(&b.l(bob.wrapping_add(8)))
        && !BALL_ANIMS.contains(&b.l(bob.wrapping_add(0x42)))
    {
        return obj_free(c, slot, 0);
    }
    let index = b.w(slot + 0x6a);
    for (axis, top, end) in [(8, 0x70, 0x6c), (0xa, 0x72, 0x6e)] {
        let top = b.w(ball.wrapping_add(top));
        let d = top.wrapping_sub(b.w(ball.wrapping_add(end)));
        let along = sixths(if n_clear(d) { d } else { d.wrapping_neg() }, index);
        let along = if n_clear(d) {
            along.wrapping_neg()
        } else {
            along
        };
        b.set_w(slot + axis, along.wrapping_add(top));
    }
    bob_follow(b, slot);
}

/// `mulu.w`, then `divu.w #6`: `d * index / 6`; a quotient over `$ffff` overflows and leaves the
/// product's low word.
fn sixths(d: u16, index: u16) -> u16 {
    let p = d as u32 * index as u32;
    if p / 6 > 0xffff {
        p as u16
    } else {
        (p / 6) as u16
    }
}
