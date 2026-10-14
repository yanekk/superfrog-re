//! Template 105 (marker 16): a walker (as template 104) that stops 8 passes after setting off or
//! turning and throws: on its throwing animation's 16th pass, a falling shot (template 108) up and
//! toward where it was walking; then it walks on for `$7d0` passes. Stomps bounce off it.

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, touch_spud_damage};
use crate::enemies::w6::walker::{at_turn, step, turn, walker_start};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_follow, obj_spawn_at_xy_nodedup, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `thrower_vx` (h0+$281d6): a word inside the code: the thrower's `+$0c` as it throws.
pub const THROWER_VX: u32 = h0(0x281d6);
const WALK: u32 = h0(0x28086);
const THROW: u32 = h0(0x2813e);

/// Template 105.
pub static THROWER: ObjectKind = ObjectKind {
    name: "thrower",
    routines: &[
        (h0(0x2804c), thrower_init),
        (WALK, thrower_walk),
        (THROW, thrower_throw),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 16 (h0+$1898e): a thrower, `$c` down its cell.
pub fn spawn_105(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(105), cell, 0, 0xc);
}

/// h0+$2804c: its look (h0+$32c58); 8 passes to the first throw.
fn thrower_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    walker_start(c, slot, h0(0x32c58));
    c.bus.set_w(slot + 0x60, 8);
}

/// h0+$28086: when `+$60` runs out it starts throwing (h0+$32ce0 left, h0+$32d5c right; then
/// back to the look it had), unstompable, `+$68` 1. It steps on frames whose `+$06` is 0 and
/// turns as the walker (h0+$32c58 left, h0+$32c9c right), 8 passes to its next throw.
fn thrower_walk(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if t == 0 {
        b.set_w(slot + 0x68, 1);
        b.set_l(slot + 4, THROW);
        b.set_w(slot + 0x2e, 0);
        let bob = b.l(slot + 0x50);
        b.set_l(bob + 0x42, b.l(bob + 8));
        let look = if b.w(slot + 0xc) & 0x8000 != 0 {
            h0(0x32ce0)
        } else {
            h0(0x32d5c)
        };
        b.set_l(bob + 0x28, look);
    }
    step(c, slot, 0);
    if at_turn(c, slot) {
        turn(c, slot, h0(0x32c58), h0(0x32c9c), 8);
        c.bus.set_w(slot + 0x60, 8);
    }
    bob_follow(&mut c.bus, slot);
}

/// h0+$2813e: while its animation runs (`+$1a` from 2 below `$fa0`) `+$68` counts down; at 0
/// the shot, 8 right, thrown up at 5 and 7 sideways its way, and `+$68` `$10`. Once the
/// animation is over (`$7530`) it walks again, stompable, `$7d0` passes to its next throw.
fn thrower_throw(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    let k = b.w(bob + 0x1a);
    if n_clear(k.wrapping_sub(2)) && !n_clear(k.wrapping_sub(0xfa0)) {
        let n = b.w(slot + 0x68).wrapping_sub(1);
        b.set_w(slot + 0x68, n);
        if n == 0 {
            b.set_w(THROWER_VX, b.w(slot + 0xc));
            let (x, y) = (b.w(slot + 8).wrapping_add(8), b.w(slot + 0xa));
            let r = obj_spawn_at_xy_nodedup(c, template(108), x, y);
            let b = &mut c.bus;
            if r.d0 == 0 && r.a0 != 0 {
                b.set_w(r.a0 + 0xe, 0xfffb);
                b.set_w(r.a0 + 0xc, 0xfff9);
                if n_clear(b.w(THROWER_VX)) {
                    b.set_w(r.a0 + 0xc, 7);
                }
            }
            b.set_w(slot + 0x68, 0x10);
        }
    }
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    if n_clear(b.w(bob + 0x1a).wrapping_sub(0x7530)) {
        b.set_w(bob + 0x1a, 0);
        b.set_w(slot + 0x60, 0x7d0);
        b.set_l(slot + 4, WALK);
        b.set_w(slot + 0x2e, 1);
    }
    bob_follow(b, slot);
}
