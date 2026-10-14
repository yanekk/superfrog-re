//! Templates 104 (marker 15) and 106 (marker 20): walkers that step along the ground, moving
//! only on the animation frames that take a step, and turn round at marker `$25` (an invisible
//! turning marker) above the row they walk on. A stomp bounces off them; the spud does nothing.

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, row_base, touch_spud_damage};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// The turning marker.
pub const TURN: u16 = 0x25;

/// Template 104.
pub static WALKER: ObjectKind = ObjectKind {
    name: "walker",
    routines: &[
        (h0(0x27fa0), walker_init),
        (h0(0x27fd4), walker_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Template 106: steps on other frames, faster.
pub static STRIDER: ObjectKind = ObjectKind {
    name: "strider",
    routines: &[
        (h0(0x27efa), strider_init),
        (h0(0x27f2e), strider_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 15 (h0+$1897e): a walker, `$c` down its cell.
pub fn spawn_104(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(104), cell, 0, 0xc);
}

/// Marker 20 (h0+$1899e): a strider, `$c` down its cell.
pub fn spawn_106(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(106), cell, 0, 0xc);
}

/// The walkers' inits: the look, `+$64` the start of the row two below its cell, `+$5c` 0.
pub fn walker_start(c: &mut LevelCtx, slot: u32, look: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, look);
    let base = row_base(b, slot, 0x4c0);
    b.set_l(slot + 0x64, base);
    b.set_w(slot + 0x5c, 0);
}

/// Whether the cell under a walker's front, a row above `+$64`'s row, holds the turning marker:
/// its x plus 3, column `(x >> 4) * 2` bytes into that row.
pub fn at_turn(c: &LevelCtx, slot: u32) -> bool {
    let b = &c.bus;
    let col = (b.w(slot + 8).wrapping_add(3) >> 4).wrapping_mul(2);
    let cell = b.l(slot + 0x64).wrapping_add(col as u32);
    b.w(cell.wrapping_sub(0x25e)) & 0x3f == TURN
}

/// A walker's step: `+$0c` when the animation step's `+$06` is `step`, else none.
pub fn step(c: &mut LevelCtx, slot: u32, on: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    let frame = b.l(bob + 0x16);
    let dx = if b.w(frame + 6) == on {
        b.w(slot + 0xc)
    } else {
        0
    };
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(dx));
}

/// Turn round: `+$0c` negated and taken once; its look `left` (moving left) or `right`, the
/// restart `restart_offset` bytes into it.
pub fn turn(c: &mut LevelCtx, slot: u32, left: u32, right: u32, restart_offset: u32) {
    let b = &mut c.bus;
    let vx = b.w(slot + 0xc).wrapping_neg();
    b.set_w(slot + 0xc, vx);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
    let look = if vx & 0x8000 != 0 { left } else { right };
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, look);
    b.set_l(bob + 0x42, look.wrapping_add(restart_offset));
}

/// h0+$27fa0: its look (h0+$32b30).
fn walker_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    walker_start(c, slot, h0(0x32b30));
}

/// h0+$27fd4: steps on frames whose `+$06` is 0; turns (h0+$32b30 left, h0+$32b74 right, each
/// restarting 8 bytes in).
fn walker_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, 0);
    if at_turn(c, slot) {
        turn(c, slot, h0(0x32b30), h0(0x32b74), 8);
    }
    bob_follow(&mut c.bus, slot);
}

/// h0+$27efa: its look (h0+$32bd0).
fn strider_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    walker_start(c, slot, h0(0x32bd0));
}

/// h0+$27f2e: steps on frames whose `+$06` is 1; turns (h0+$32bd0 left, h0+$32c14 right).
fn strider_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, 1);
    if at_turn(c, slot) {
        turn(c, slot, h0(0x32bd0), h0(0x32c14), 0);
    }
    bob_follow(&mut c.bus, slot);
}
