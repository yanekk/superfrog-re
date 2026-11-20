//! Template 67 (marker 17): a spiked ball swinging along a path below its marker, and template
//! 68: the five links of its chain, each set between the ball and the chain's anchor in sixths.
//! Their updates are W3's (`enemies::w3::swing`), shared with templates 63–70.

use crate::enemies::common::NOTHING;
use crate::enemies::w3::swing::{LINK_UPDATE, UPDATE, link_update, update};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, nothing, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// `chain_link_count` (h0+$262ba): a long inside the code; the marker's stub counts the links it
/// spawns in it (1..5), and each link's init keeps its number.
pub use crate::enemies::w3::swing::LINK_INDEX as CHAIN_LINK_COUNT;
/// The swing (h0+$26398): positions (x, y words) ending with a −1 long.
pub use crate::enemies::w3::swing::PATH as SWING;

/// Template 67: the ball.
pub static MACE: ObjectKind = ObjectKind {
    name: "mace",
    routines: &[
        (h0(0x260c4), mace_init),
        (UPDATE, update),
        (h0(0x29708), nothing),
    ],
};

/// Template 68: a link of its chain.
pub static MACE_LINK: ObjectKind = ObjectKind {
    name: "mace chain link",
    routines: &[
        (h0(0x2627e), link_init),
        (LINK_UPDATE, link_update),
        (NOTHING, nothing),
    ],
};

/// Marker 17 (h0+$189ae): the ball, `$c` down its cell; unless that leaves `d0` set, five links
/// in the cells below, numbered 1..5, each told its ball (`+$64`), when its spawn leaves `d0`
/// clear.
pub fn spawn_67(c: &mut LevelCtx, cell: u32) {
    let r = obj_spawn_cell(c, template(67), cell, 0, 0xc);
    if r.d0 as u16 != 0 {
        return;
    }
    let ball = r.a0;
    c.bus.set_l(CHAIN_LINK_COUNT, 1);
    let mut at = cell;
    for k in 0..5 {
        if k > 0 {
            let b = &mut c.bus;
            b.set_l(CHAIN_LINK_COUNT, b.l(CHAIN_LINK_COUNT).wrapping_add(1));
        }
        at = at.wrapping_add(0x260);
        let r = obj_spawn_cell(c, template(68), at, 0, 0);
        // With no slot `a0` is 0 and the ball goes to $64, the level 1 autovector (the original's).
        if r.d0 as u16 == 0 {
            c.bus.set_l(r.a0.wrapping_add(0x64), ball);
        }
    }
}

/// h0+$260c4: its look; the swing's origin (`+$64`, its spawn position) and the swing; the
/// chain's ends: `+$6c` the ball's (`$40` lower) and `+$70` the anchor (its spawn point, 2 left
/// and 4 down).
fn mace_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x32bc4));
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x58, SWING);
    b.set_l(slot + 0x54, SWING);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(0x40));
    b.set_l(slot + 0x70, b.l(slot + 0x44));
    b.set_w(slot + 0x72, b.w(slot + 0x72).wrapping_add(4));
    b.set_w(slot + 0x70, b.w(slot + 0x70).wrapping_sub(2));
}

/// h0+$2627e: its look and its number.
fn link_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x32bb8));
    b.set_l(slot + 0x68, b.l(CHAIN_LINK_COUNT));
}
