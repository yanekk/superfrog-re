//! Template 13 (marker 3): `w1_walker`, which walks along its row, turning at cells marked `$25`.
//! A stomp flattens it (`touch_spud_damage`) and it becomes a reward; the spud passes over it.

use crate::enemies::common::{
    TOUCH_SPUD_DAMAGE, follow_and_despawn, row_base, touch_spud_damage, walk_and_turn,
};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind};

/// Its animations, walking left (h0+$31080) and right (h0+$310d4).
const LEFT: u32 = h0(0x31080);
const RIGHT: u32 = h0(0x310d4);

pub static WALKER: ObjectKind = ObjectKind {
    name: "w1 walker",
    routines: &[
        (h0(0x2493a), init),
        (h0(0x2496a), update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// `w1_walker_init` (h0+$2493a, the sitter's too): walking left; its row's base in `+$64`.
pub fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
}

/// `w1_walker_update` (h0+$2496a).
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    walk_and_turn(&mut c.bus, slot, LEFT, RIGHT);
    follow_and_despawn(c, slot);
}
