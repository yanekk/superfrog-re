//! Templates 63 (marker 17) and 64: a ball swinging on a chain of five links, hanging from its
//! marker's cell; the ball hurts (`touch_hurt`), the links only show. Each link sits on the line
//! from the chain's anchor to the ball, at its own fraction of it (`+$68`: 1..5 sixths).

use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    OBJ_SLOTS, bob_anim, bob_follow, obj_free, obj_path_xy, obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `mace_swing` (h0+$26398): the ball's positions (x, y longs) about its spawn point, ending with
/// a −1 long.
pub const SWING: u32 = h0(0x26398);
/// `mace_link_count` (h0+$262ba): the fraction the next link spawned takes.
pub const LINK_COUNT: u32 = h0(0x262ba);
const BALL: u32 = h0(0x3156c);
/// The four worlds' ball and link looks: a link stays only while its ball shows one of these,
/// a ball only while five links show one of theirs.
const BALLS: [u32; 4] = [h0(0x32288), h0(0x3156c), h0(0x31c60), h0(0x32bc4)];
const LINK: u32 = h0(0x31560);
/// The link looks' animations (`+8`), then their starts (`+$42`), in the order the ball tests them.
const LINK_ANIMS: [u32; 4] = [h0(0x31560), h0(0x31c54), h0(0x32bb8), h0(0x3227c)];
const LINK_STARTS: [u32; 4] = [h0(0x31560), h0(0x31c54), h0(0x3227c), h0(0x32bb8)];

pub static MACE: ObjectKind = ObjectKind {
    name: "mace",
    routines: &[(h0(0x26154), mace_init), (h0(0x2619c), mace_update)],
};

pub static CHAIN: ObjectKind = ObjectKind {
    name: "mace chain link",
    routines: &[(h0(0x26242), link_init), (h0(0x262be), link_update)],
};

/// Marker 17 (h0+$181a6): the ball at the cell (12 pixels down), then a link in each of the five
/// cells below it, each told its ball (`+$64`) and its fraction. A link that finds no slot is
/// told nothing: the original writes its ball to `$64` (the level 1 interrupt's vector), which
/// the engine does not have.
pub fn spawn(c: &mut LevelCtx, cell: u32) {
    let ball = obj_spawn_cell(c, template(63), cell, 0, 0xc);
    if ball.d0 as u16 != 0 {
        return;
    }
    c.bus.set_l(LINK_COUNT, 1);
    let mut at = cell;
    for k in 0..5 {
        at = at.wrapping_add(0x260);
        let link = obj_spawn_cell(c, template(64), at, 0, 0);
        if link.d0 as u16 == 0 && link.a0 != 0 {
            c.bus.set_l(link.a0 + 0x64, ball.a0);
        }
        // (Not after the last.)
        if k < 4 {
            c.bus.set_l(LINK_COUNT, c.bus.l(LINK_COUNT).wrapping_add(1));
        }
    }
}

/// h0+$26154: `+$64` its spawn point; `+$6c` where the chain meets it, `+$70` the anchor (its
/// spawn point 2 left, 4 down).
fn mace_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, BALL);
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x58, SWING);
    b.set_l(slot + 0x54, SWING);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(0x40));
    b.set_l(slot + 0x70, b.l(slot + 0x44));
    b.set_w(slot + 0x72, b.w(slot + 0x72).wrapping_add(4));
    b.set_w(slot + 0x70, b.w(slot + 0x70).wrapping_sub(2));
}

/// h0+$2619c: the next position of the swing (from `$50` left of its spawn point), the chain's
/// end 8 right and 4 down of it; freed when fewer than five links are about.
fn mace_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (dx, dy) = (b.w(slot + 0x64).wrapping_sub(0x50), b.w(slot + 0x66));
    obj_path_xy(b, slot, dx, dy);
    bob_follow(b, slot);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(4));
    b.set_w(slot + 0x6c, b.w(slot + 0x6c).wrapping_add(8));
    let links = (0..20)
        .map(|k| b.l(OBJ_SLOTS + k * 0x7c + 0x50))
        .filter(|&bob| LINK_ANIMS.contains(&b.l(bob + 8)) || LINK_STARTS.contains(&b.l(bob + 0x42)))
        .count();
    if links < 5 {
        obj_free(c, slot, 0);
    }
}

/// h0+$26242: `+$68` its fraction.
fn link_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LINK);
    b.set_l(slot + 0x68, b.l(LINK_COUNT));
}

/// h0+$262be: freed once its ball (`+$64`) is gone or shows no ball; else at its fraction of the
/// line from the anchor to the ball.
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
    let k = b.l(slot + 0x68) as u16;
    for (at, from, to) in [(8, 0x70, 0x6c), (0xa, 0x72, 0x6e)] {
        let anchor = b.w(ball.wrapping_add(from));
        let span = anchor.wrapping_sub(b.w(ball.wrapping_add(to)));
        let v = fraction(span, k).wrapping_add(anchor);
        b.set_w(slot + at, v);
    }
    // Nothing frees a link but its ball.
    bob_follow(b, slot);
}

/// `span · k / 6` towards the ball: `mulu`, `divu #6` (the product kept on overflow), negated
/// back for a span that was not negative.
fn fraction(span: u16, k: u16) -> u16 {
    let neg = !n_clear(span);
    let mag = if neg { span.wrapping_neg() } else { span };
    let p = mag as u32 * k as u32;
    let q = if p / 6 > 0xffff {
        p as u16
    } else {
        (p / 6) as u16
    };
    if neg { q } else { q.wrapping_neg() }
}
