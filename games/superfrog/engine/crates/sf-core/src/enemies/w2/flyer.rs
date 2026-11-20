//! Template 38 (marker 1): flies a loop of positions around its spawn point, one of four loops in
//! turn; a stomp or the spud kills it (`touch_kill_1hit`).

use crate::enemies::common::follow_and_despawn;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_path_xy};
use crate::level::{LevelCtx, ObjectKind};

/// `flyer_loops` (h0+$24454): four loops of `$cc` bytes, positions (x, y longs) relative to the
/// spawn point, the last ending with a −1 long.
pub const LOOPS: u32 = h0(0x24454);
/// `flyer_next_loop` (h0+$243b4): the loop the next flyer takes (0..3).
pub const NEXT_LOOP: u32 = h0(0x243b4);
const LOOK: u32 = h0(0x31504);

pub static FLYER: ObjectKind = ObjectKind {
    name: "flyer",
    routines: &[(h0(0x258dc), flyer_init), (h0(0x25990), flyer_update)],
};

/// h0+$258dc: the spawn point kept (`+$64`, `+$68`), the next of the four loops taken.
fn flyer_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LOOK);
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x68, b.l(slot + 0x64));
    b.set_l(slot + 0x58, LOOPS);
    let k = b.w(NEXT_LOOP);
    b.set_l(slot + 0x54, LOOPS.wrapping_add(k as u32 * 0xcc));
    let k = k.wrapping_add(1);
    b.set_w(NEXT_LOOP, if k < 4 { k } else { 0 });
}

/// h0+$25990: the next position of the loop, from the spawn point.
fn flyer_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (dx, dy) = (b.w(slot + 0x64), b.w(slot + 0x66));
    obj_path_xy(b, slot, dx, dy);
    follow_and_despawn(c, slot);
}
