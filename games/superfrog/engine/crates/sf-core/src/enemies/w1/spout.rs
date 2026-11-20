//! Template 33 (marker 10): `w1_spout`, scenery drawn into the background (its look is
//! negative) that every `$32` passes throws a shot out of the cell right of its own, in turn
//! up-left ([`SHOT_LEFT`], template 14), up ([`SHOT_UP`], 15) and up-right ([`SHOT_RIGHT`], 16),
//! each with its own animation. The shots fly in an arc and are freed on a solid cell; they hurt
//! on touch and nothing kills them.

use crate::enemies::common::{follow_and_despawn, marker_right_of, solid};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_free, obj_spawn_cell_alt};
use crate::level::{LevelCtx, ObjectKind};

/// `w1_spout_turn` (h0+$25356): which shot the next throw is, 0..2.
pub const TURN: u32 = h0(0x25356);

pub static SPOUT: ObjectKind = ObjectKind {
    name: "w1 spout",
    routines: &[(h0(0x2533a), init), (h0(0x25358), update)],
};

pub static SHOT_LEFT: ObjectKind = ObjectKind {
    name: "w1 spout shot left",
    routines: &[(h0(0x24a58), shot_left_init), (h0(0x24aac), shot_update)],
};

pub static SHOT_UP: ObjectKind = ObjectKind {
    name: "w1 spout shot up",
    routines: &[(h0(0x24a74), shot_up_init), (h0(0x24aac), shot_update)],
};

pub static SHOT_RIGHT: ObjectKind = ObjectKind {
    name: "w1 spout shot right",
    routines: &[(h0(0x24a90), shot_right_init), (h0(0x24aac), shot_update)],
};

/// `w1_spout_init` (h0+$2533a): its animation (h0+$31218, then h0+$31200 when it repeats); the
/// first throw in 10 passes.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, h0(0x31218));
    b.set_l(bob + 0x42, h0(0x31200));
    b.set_w(slot + 0x5c, 0xa);
}

/// `w1_spout_update` (h0+$25358): when its count (`+$5c`) runs out it starts again from `$32`
/// (an RNG number is drawn and dropped on the way), takes the next turn and throws: the turn's
/// animation, and its shot `dx`, `dy` pixels into the cell right of its own.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let t = c.bus.w(slot + 0x5c).wrapping_sub(1);
    c.bus.set_w(slot + 0x5c, t);
    if (t as i16) < 0 {
        c.rng_range(0x32);
        let b = &mut c.bus;
        b.set_w(slot + 0x5c, 0x32);
        let mut turn = b.w(TURN).wrapping_add(1);
        if (turn.wrapping_sub(3) as i16) >= 0 {
            turn = 0;
        }
        b.set_w(TURN, turn);
        // The turn is never outside 0..2 in play; the original then throws with its caller's
        // registers, which nothing here models.
        let (anim, dx, dy, shot) = match turn {
            0 => (h0(0x31200), 0xfffeu16, 0xau16, 14),
            1 => (h0(0x31224), 0xc, 0, 15),
            _ => (h0(0x31248), 0x18, 0xa, 16),
        };
        bob_anim(b, slot, anim);
        let cell = b.l(slot + 0x40).wrapping_add(2);
        obj_spawn_cell_alt(c, template(shot), cell, dx.wrapping_sub(0x10), dy.wrapping_add(0xc));
    }
    follow_and_despawn(c, slot);
}

/// h0+$24a58: up-left (h0+$3126c), 7 pixels a pass up to start.
fn shot_left_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x3126c));
    c.bus.set_w(slot + 0xe, 0xfff9);
}

/// h0+$24a74: straight up (h0+$31278), 12 pixels a pass up to start.
fn shot_up_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31278));
    c.bus.set_w(slot + 0xe, 0xfff4);
}

/// h0+$24a90: up-right (h0+$31284), 7 pixels a pass up to start.
fn shot_right_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31284));
    c.bus.set_w(slot + 0xe, 0xfff9);
}

/// h0+$24aac: gravity, then x and y move by the speeds; freed once the cell right of the one 8
/// pixels above it is solid.
fn shot_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    bob_follow(b, slot);
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa).wrapping_sub(8));
    if solid(marker_right_of(b, x, y)) {
        return obj_free(c, slot, 0);
    }
    crate::level::objects::obj_despawn_if_offscreen(c, slot, 0);
}
