//! Template 12 (marker 2): `w1_sitter`, which stands where it spawned. A stomp flattens it
//! (`touch_spud_damage`) and it becomes a reward; the spud passes over it.

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, follow_and_despawn, touch_spud_damage};
use crate::enemies::w1::walker;
use crate::layout::h0;
use crate::level::{LevelCtx, ObjectKind};

pub static SITTER: ObjectKind = ObjectKind {
    name: "w1 sitter",
    routines: &[
        (h0(0x2493a), walker::init),
        (h0(0x249c2), update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// `w1_sitter_update` (h0+$249c2): the bob follows; freed once the camera has left it.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    follow_and_despawn(c, slot);
}
