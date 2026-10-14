//! Template 69 (marker 17): a spiked ball swinging along a path below its marker, and template
//! 70: the five links of its chain, each set between the ball and the chain's anchor in sixths.
//! Only the inits are W4's: the updates (h0+$2619c, h0+$262be) are shared with templates 63–68,
//! W6's mace among them (T40 ports the same two; one copy should stay when both have merged).

use crate::enemies::common::NOTHING;
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    OBJ_SLOTS, bob_anim, bob_follow, nothing, obj_free, obj_path_xy, obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind};

/// `chain_link_count` (h0+$262ba): a long inside the code; the marker's stub counts the links it
/// spawns in it (1..5), and each link's init keeps its number.
pub const CHAIN_LINK_COUNT: u32 = h0(0x262ba);
/// The swing (h0+$26398): positions (x, y words) ending with a −1 long.
pub const SWING: u32 = h0(0x26398);
/// The balls' update (h0+$2619c) and the links' (h0+$262be).
pub const BALL_UPDATE: u32 = h0(0x2619c);
pub const LINK_UPDATE: u32 = h0(0x262be);

/// The balls' looks (templates 63, 65, 67, 69): a link's ball is gone unless its bob shows one.
const BALLS: [u32; 4] = [h0(0x32288), h0(0x3156c), h0(0x31c60), h0(0x32bc4)];
/// The links' looks (templates 64, 66, 68, 70): a ball with fewer than five is gone.
const LINKS: [u32; 4] = [h0(0x31560), h0(0x31c54), h0(0x32bb8), h0(0x3227c)];

/// Template 69: the ball.
pub static MACE: ObjectKind = ObjectKind {
    name: "mace (W4)",
    routines: &[(h0(0x2610c), mace_init), (BALL_UPDATE, mace_update)],
};

/// Template 70: a link of its chain.
pub static MACE_LINK: ObjectKind = ObjectKind {
    name: "mace chain link (W4)",
    routines: &[
        (h0(0x2629c), link_init),
        (LINK_UPDATE, link_update),
        (NOTHING, nothing),
    ],
};

/// Marker 17 (h0+$185ea): the ball, `$c` down its cell; unless that leaves `d0` set, five links
/// in the cells below, numbered 1..5, each told its ball (`+$64`), when its spawn leaves `d0`
/// clear.
pub fn spawn_69(c: &mut LevelCtx, cell: u32) {
    let r = obj_spawn_cell(c, template(69), cell, 0, 0xc);
    if r.d0 as u16 != 0 {
        return;
    }
    let ball = r.a0;
    c.bus.set_l(CHAIN_LINK_COUNT, 1);
    let mut at = cell;
    for k in 0..5 {
        if k > 0 {
            let b = &mut c.bus;
            b.set_l(CHAIN_LINK_COUNT, b.l(CHAIN_LINK_COUNT).wrapping_add(1));
        }
        at = at.wrapping_add(0x260);
        let r = obj_spawn_cell(c, template(70), at, 0, 0);
        // With no slot the original writes the ball to $64 (a null slot's `+$64`), an interrupt
        // vector the engine does not keep: left out (no level fills every slot here).
        if r.d0 as u16 == 0 && r.a0 != 0 {
            c.bus.set_l(r.a0 + 0x64, ball);
        }
    }
}

/// h0+$2610c: its look (h0+$32288); the swing's origin (`+$64`, its spawn position) and the
/// swing; the chain's ends: `+$6c` the ball's (`$40` lower) and `+$70` the anchor (its spawn
/// point, 2 left and 4 down).
fn mace_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x32288));
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x58, SWING);
    b.set_l(slot + 0x54, SWING);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(0x40));
    b.set_l(slot + 0x70, b.l(slot + 0x44));
    b.set_w(slot + 0x72, b.w(slot + 0x72).wrapping_add(4));
    b.set_w(slot + 0x70, b.w(slot + 0x70).wrapping_sub(2));
}

/// h0+$2619c: the next swing position from the origin `$50` left; the chain's end on the ball
/// (8 right, 4 down); gone once fewer than five links are live (any slot whose bob shows a link,
/// free slots' null bobs read too).
fn mace_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let dx = b.w(slot + 0x64).wrapping_sub(0x50);
    let dy = b.w(slot + 0x66);
    obj_path_xy(b, slot, dx, dy);
    bob_follow(b, slot);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(4));
    b.set_w(slot + 0x6c, b.w(slot + 0x6c).wrapping_add(8));
    let links = (0..20)
        .map(|k| b.l(OBJ_SLOTS + k * 0x7c + 0x50))
        .filter(|&bob| {
            LINKS.contains(&b.l(bob.wrapping_add(8)))
                || LINKS.contains(&b.l(bob.wrapping_add(0x42)))
        })
        .count();
    if links < 5 {
        obj_free(c, slot, 0);
    }
}

/// h0+$2629c: its look (h0+$3227c) and its number.
fn link_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x3227c));
    b.set_l(slot + 0x68, b.l(CHAIN_LINK_COUNT));
}

/// One coordinate of a link: `n`/6 of the way from the anchor `anchor` to the ball's end `end`
/// (`mulu.w`, `divu.w #6`: an overflowing quotient leaves the product).
fn sixths(anchor: u16, end: u16, n: u32) -> u16 {
    let d = anchor.wrapping_sub(end);
    let (mag, up) = if d & 0x8000 != 0 {
        (d.wrapping_neg(), true)
    } else {
        (d, false)
    };
    let p = mag as u32 * (n & 0xffff);
    let q = if p / 6 > 0xffff {
        p as u16
    } else {
        (p / 6) as u16
    };
    let q = if up { q } else { q.wrapping_neg() };
    q.wrapping_add(anchor)
}

/// h0+$262be: gone with its ball (a ball slot freed, or its bob not a ball's); else placed its
/// number of sixths from the anchor to the ball.
fn link_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let ball = b.l(slot + 0x64);
    if b.l(ball.wrapping_add(0x40)) == 0 {
        return obj_free(c, slot, 0);
    }
    let bob = b.l(ball.wrapping_add(0x50));
    if !BALLS.contains(&b.l(bob.wrapping_add(8))) && !BALLS.contains(&b.l(bob.wrapping_add(0x42))) {
        return obj_free(c, slot, 0);
    }
    let n = b.l(slot + 0x68);
    let x = sixths(
        b.w(ball.wrapping_add(0x70)),
        b.w(ball.wrapping_add(0x6c)),
        n,
    );
    b.set_w(slot + 8, x);
    let y = sixths(
        b.w(ball.wrapping_add(0x72)),
        b.w(ball.wrapping_add(0x6e)),
        n,
    );
    b.set_w(slot + 0xa, y);
    bob_follow(b, slot);
}
