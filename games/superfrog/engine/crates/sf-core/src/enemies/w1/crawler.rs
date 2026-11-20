//! Template 22 (marker 12): `w1_crawler`, a walker turning at cells marked `$25` that can be
//! stomped (`touch_spud_damage`) only while the shared word h0+$2d128, inside the bobs' animation
//! records, is 2 or more.

use crate::enemies::common::{
    TOUCH_SPUD_DAMAGE, follow_and_despawn, row_base, touch_spud_damage, walk_and_turn,
};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its animations, left (h0+$312c8) and right (h0+$3130c).
const LEFT: u32 = h0(0x312c8);
const RIGHT: u32 = h0(0x3130c);
/// h0+$2d128: a word in the bobs' animation records it reads.
const STOMPABLE: u32 = h0(0x2d128);

pub static CRAWLER: ObjectKind = ObjectKind {
    name: "w1 crawler",
    routines: &[
        (h0(0x24788), init),
        (h0(0x247b8), update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// `w1_crawler_init` (h0+$24788).
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
}

/// `w1_crawler_update` (h0+$247b8).
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let stompable = n_clear(b.w(STOMPABLE).wrapping_sub(2));
    b.set_w(slot + 0x2e, stompable as u16);
    walk_and_turn(b, slot, LEFT, RIGHT);
    follow_and_despawn(c, slot);
}
