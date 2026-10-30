//! Template 140 (marker 16): a crusher that waits harmless for `$15` passes, strikes `$40` pixels
//! down in four passes, then creeps back two pixels a pass for `$20` passes, over and over: world
//! 4's crusher (template 141) with a longer strike. The spud passes over it and a stomp cannot
//! flatten it (`touch_spud_damage` on `$7d00` hit points).

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, touch_spud_damage};
use crate::enemies::w4::crusher::step;
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// Its steps: waiting (h0+$270a8), striking (h0+$270d2), going back (h0+$27100).
const WAIT: u32 = h0(0x270a8);
const STRIKE: u32 = h0(0x270d2);
const BACK: u32 = h0(0x27100);

pub static CRUSHER: ObjectKind = ObjectKind {
    name: "w5 crusher",
    routines: &[
        (h0(0x2708c), init),
        (WAIT, wait),
        (STRIKE, strike),
        (BACK, back),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 16 (h0+$18918): 2 left and `$c` down its cell.
pub fn spawn_140(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(140), cell, 0xfffe, 0xc);
}

/// h0+$2708c: its look (h0+$324ec); `$14` passes' wait.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x324ec));
    c.bus.set_w(slot + 0x5c, 0x14);
}

fn wait(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, STRIKE, 3, 1, 0);
}

fn strike(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, BACK, 0x1f, 0, 0x10);
}

fn back(c: &mut LevelCtx, slot: u32, _d7: u16) {
    step(c, slot, WAIT, 0x14, 0, 0xfffe);
}
