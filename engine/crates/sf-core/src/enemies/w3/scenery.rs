//! World 3's scenery: objects that only animate where they spawned, touch nothing and hurt
//! nothing (passive). Template 87 (marker 5): `w3_pad`, the bounce pad's picture (world 3 draws
//! its pads as objects; the pad the frog presses does not show, `items::fixtures`). Templates 34,
//! 89 and 90 (markers 17, 14, 15; the last two in the cell below the marker): `w3_scenery_17`,
//! `_14`, `_15`. Template 98 (marker 24): `w3_flow`, which starts its animation in step with a
//! neighbour's.

use crate::enemies::common::follow_and_despawn;
use crate::layout::h0;
use crate::level::objects::{OBJ_SLOTS, bob_anim, nothing};
use crate::level::{LevelCtx, ObjectKind};

/// `w3_scenery_update` (h0+$27894) and `w3_scenery_17_update` (h0+$249e8): the bob follows; freed
/// once the camera has left it.
const UPDATE: u32 = h0(0x27894);

pub static SCENERY_17: ObjectKind = ObjectKind {
    name: "w3 scenery 17",
    routines: &[
        (h0(0x249d2), scenery_17_init),
        (h0(0x249e8), update),
        (h0(0x7d4), nothing),
    ],
};

pub static PAD: ObjectKind = ObjectKind {
    name: "w3 pad",
    routines: &[
        (h0(0x2787e), pad_init),
        (UPDATE, update),
        (h0(0x7d4), nothing),
    ],
};

pub static SCENERY_14: ObjectKind = ObjectKind {
    name: "w3 scenery 14",
    routines: &[
        (h0(0x27868), scenery_14_init),
        (UPDATE, update),
        (h0(0x7d4), nothing),
    ],
};

pub static SCENERY_15: ObjectKind = ObjectKind {
    name: "w3 scenery 15",
    routines: &[
        (h0(0x27852), scenery_15_init),
        (UPDATE, update),
        (h0(0x7d4), nothing),
    ],
};

pub static FLOW: ObjectKind = ObjectKind {
    name: "w3 flow",
    routines: &[
        (h0(0x2872e), flow_init),
        (h0(0x287a8), update),
        (h0(0x7d4), nothing),
    ],
};

/// The flow's animation (h0+$31eb0) and the end of its steps (h0+$31ef4).
pub const FLOW_ANIM: u32 = h0(0x31eb0);
pub const FLOW_END: u32 = h0(0x31ef4);

/// `w3_scenery_17_init` (h0+$249d2).
fn scenery_17_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x321ac));
}

/// `w3_pad_init` (h0+$2787e).
fn pad_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31ca4));
}

/// `w3_scenery_14_init` (h0+$27868).
fn scenery_14_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31c6c));
}

/// `w3_scenery_15_init` (h0+$27852).
fn scenery_15_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31c88));
}

/// h0+$27894, h0+$249e8, h0+$287a8.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    follow_and_despawn(c, slot);
}

/// `w3_flow_init` (h0+$2872e): its animation (h0+$31eb0) restarts from the step a neighbour's
/// bob is on: a slot spawned from the cell two to the left, or else two to the right, if that
/// cell is marked 24 (the neighbour's `+$28`, or its current step when that is 0). A step
/// outside the flow's own steps, or no such neighbour, starts it from the first.
fn flow_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x42, FLOW_ANIM);
    let cell = b.l(slot + 0x40);
    let mut step = FLOW_ANIM;
    let side = [cell.wrapping_sub(4), cell.wrapping_add(4)]
        .into_iter()
        .find(|&a| b.w(a) & 0x3f == 0x18);
    if let Some(side) = side
        && let Some(other) = (0..20u32)
            .map(|k| OBJ_SLOTS + k * 0x7c)
            .find(|&s| b.l(s + 0x40) == side)
    {
        let ob = b.l(other + 0x50);
        step = match b.l(ob + 0x28) {
            0 => b.l(ob + 0x16),
            a => a,
        };
    }
    if (step.wrapping_sub(FLOW_ANIM) as i32) < 0 || (step.wrapping_sub(FLOW_END) as i32) >= 0 {
        step = FLOW_ANIM;
    }
    b.set_l(bob + 0x28, step);
}
