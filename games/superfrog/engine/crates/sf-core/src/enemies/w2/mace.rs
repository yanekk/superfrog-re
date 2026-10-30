//! Templates 63 (marker 17) and 64: a ball swinging on a chain of five links, hanging from its
//! marker's cell; the ball hurts (`touch_hurt`), the links only show. Only the inits and the
//! marker's stub are world 2's: the updates are [`crate::enemies::w3::swing`]'s (h0+$2619c,
//! h0+$262be), each link at its own fraction of the line from the chain's anchor to the ball.

use crate::enemies::w3::swing::{self, LINK_UPDATE, UPDATE, link_update, update};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// `swing_path` (h0+$26398): the ball's positions (x, y words) about its spawn point, ending with
/// a −1 long.
pub const SWING: u32 = swing::PATH;
/// `chain_link_count` (h0+$262ba): the fraction the next link spawned takes.
pub const LINK_COUNT: u32 = swing::LINK_INDEX;
const BALL: u32 = h0(0x3156c);
const LINK: u32 = h0(0x31560);

pub static MACE: ObjectKind = ObjectKind {
    name: "mace",
    routines: &[(h0(0x26154), mace_init), (UPDATE, update)],
};

pub static CHAIN: ObjectKind = ObjectKind {
    name: "mace chain link",
    routines: &[(h0(0x26242), link_init), (LINK_UPDATE, link_update)],
};

/// Marker 17 (h0+$181a6): the ball at the cell (12 pixels down), then a link in each of the five
/// cells below it, each told its ball (`+$64`) and its fraction. A link that finds no slot
/// (`a0` 0) writes its ball to `$64`, the level 1 interrupt's vector (`level1_vector`), as the
/// original does.
pub fn spawn(c: &mut LevelCtx, cell: u32) {
    let ball = obj_spawn_cell(c, template(63), cell, 0, 0xc);
    if ball.d0 as u16 != 0 {
        return;
    }
    c.bus.set_l(LINK_COUNT, 1);
    let mut at = cell;
    for k in 0..5 {
        at = at.wrapping_add(0x260);
        let link = obj_spawn_cell(c, template(64), at, 0, 0);
        if link.d0 as u16 == 0 {
            c.bus.set_l(link.a0.wrapping_add(0x64), ball.a0);
        }
        // (Not after the last.)
        if k < 4 {
            c.bus.set_l(LINK_COUNT, c.bus.l(LINK_COUNT).wrapping_add(1));
        }
    }
}

/// h0+$26154: `+$64` its spawn point; `+$6c` where the chain meets it, `+$70` the anchor (its
/// spawn point 2 left, 4 down).
fn mace_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, BALL);
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x58, SWING);
    b.set_l(slot + 0x54, SWING);
    b.set_l(slot + 0x6c, b.l(slot + 8));
    b.set_w(slot + 0x6e, b.w(slot + 0x6e).wrapping_add(0x40));
    b.set_l(slot + 0x70, b.l(slot + 0x44));
    b.set_w(slot + 0x72, b.w(slot + 0x72).wrapping_add(4));
    b.set_w(slot + 0x70, b.w(slot + 0x70).wrapping_sub(2));
}

/// h0+$26242: `+$68` its fraction.
fn link_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LINK);
    b.set_l(slot + 0x68, b.l(LINK_COUNT));
}
