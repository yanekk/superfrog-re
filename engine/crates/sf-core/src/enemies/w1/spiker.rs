//! Template 21 (marker 15): `w1_spiker`, a walker turning at cells marked `$25` that hurts on
//! any touch: not stompable, and the spud passes over it (`touch_spud_damage`).

use crate::enemies::common::{
    TOUCH_SPUD_DAMAGE, follow_and_despawn, row_base, touch_spud_damage, walk_and_turn,
};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind};

/// Its animations, left (h0+$31434) and right (h0+$31458).
const LEFT: u32 = h0(0x31434);
const RIGHT: u32 = h0(0x31458);

pub static SPIKER: ObjectKind = ObjectKind {
    name: "w1 spiker",
    routines: &[
        (h0(0x248b2), init),
        (h0(0x248e2), update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// `w1_spiker_init` (h0+$248b2).
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
}

/// `w1_spiker_update` (h0+$248e2).
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    walk_and_turn(&mut c.bus, slot, LEFT, RIGHT);
    follow_and_despawn(c, slot);
}
