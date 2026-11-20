//! Template 92 (marker 20): `w3_shuffler`, which walks its row only on the steps of its animation
//! whose word `+6` is 0, turning at cells marked `$25` (looking from x + 3). A stomp flattens it
//! (`touch_spud_damage` on `$1f` hit points) into the death animation of the way it faces, and it
//! becomes a reward.

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, follow_and_despawn, row_base, touch_spud_damage};
use crate::enemies::w3::walkers::at_turn;
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its animations, left (h0+$31f5c) and right (h0+$31fe8), and the death animations each turn
/// sets (h0+$32080, h0+$32074).
const LEFT: u32 = h0(0x31f5c);
const RIGHT: u32 = h0(0x31fe8);

pub static SHUFFLER: ObjectKind = ObjectKind {
    name: "w3 shuffler",
    routines: &[
        (h0(0x278a4), init),
        (h0(0x278d8), update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// `w3_shuffler_init` (h0+$278a4).
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
    b.set_w(slot + 0x5c, 0);
}

/// `w3_shuffler_update` (h0+$278d8): x moves by `+$0c` when its bob's step's word `+6` is 0; at a
/// turn the speed turns and x moves by the new speed at once.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    let vx = if b.w(b.l(bob + 0x16) + 6) == 0 {
        b.w(slot + 0xc)
    } else {
        0
    };
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
    if at_turn(b, slot) {
        let vx = b.w(slot + 0xc).wrapping_neg();
        b.set_w(slot + 0xc, vx);
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
        let (anim, death) = if n_clear(vx) {
            (RIGHT, h0(0x32074))
        } else {
            (LEFT, h0(0x32080))
        };
        b.set_l(slot + 0x18, death);
        bob_anim(b, slot, anim);
    }
    follow_and_despawn(c, slot);
}
