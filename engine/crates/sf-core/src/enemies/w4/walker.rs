//! Template 93 (marker 12): a walker stepping along the ground, moving only on its animation's
//! stepping frames and turning round at marker `$25` above the row it walks on. The spud stuns it;
//! it can be stomped.

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, row_base, touch_spud_damage};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// The turning marker.
const TURN: u16 = 0x25;

/// Template 93.
pub static WALKER: ObjectKind = ObjectKind {
    name: "walker (W4)",
    routines: &[
        (h0(0x2795e), walker_init),
        (h0(0x27992), walker_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 12 (h0+$185da): `$a` down its cell.
pub fn spawn_93(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(93), cell, 0, 0xa);
}

/// h0+$2795e: its look (h0+$3230c); `+$64` the start of the row below its cell; `+$5c` 0.
fn walker_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x3230c));
    let base = row_base(b, slot, 0x260);
    b.set_l(slot + 0x64, base);
    b.set_w(slot + 0x5c, 0);
}

/// h0+$27992: steps `+$0c` on frames whose `+$06` is 0. At the turning marker (a row above
/// `+$64`'s, a column on from x + 3) it turns, steps back, and takes the look and stunned look
/// for its new way: h0+$3230c and h0+$32430 left, h0+$32398 and h0+$32424 right.
fn walker_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    let frame = b.l(bob + 0x16);
    let dx = if b.w(frame + 6) == 0 {
        b.w(slot + 0xc)
    } else {
        0
    };
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(dx));
    let col = (b.w(slot + 8).wrapping_add(3) >> 4).wrapping_mul(2) as u32;
    let cell = col.wrapping_add(b.l(slot + 0x64));
    if b.w(cell.wrapping_sub(0x25e)) & 0x3f == TURN {
        let vx = b.w(slot + 0xc).wrapping_neg();
        b.set_w(slot + 0xc, vx);
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
        let (look, stunned) = if vx & 0x8000 != 0 {
            (h0(0x3230c), h0(0x32430))
        } else {
            (h0(0x32398), h0(0x32424))
        };
        b.set_l(slot + 0x18, stunned);
        bob_anim(b, slot, look);
    }
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
