//! Templates 57 and 58 (markers 18 and 19): a dart that waits hidden at its spawn point, then
//! flies left (57) or right (58) at 8 pixels a pass until it reaches solid ground, and waits again.
//! Only the darts' inits are W6's; their update is shared with templates 53–62 of other worlds.

use crate::enemies::common::{NOTHING, TOUCH_HURT, cell_at, solid};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// The darts' update while waiting (h0+$25fa8) and while flying (h0+$25f42).
pub const WAIT: u32 = h0(0x25fa8);
pub const FLY: u32 = h0(0x25f42);

/// Template 57: flies left.
pub static DART_LEFT: ObjectKind = ObjectKind {
    name: "dart (left)",
    routines: &[
        (h0(0x25e9a), dart_left_init),
        (WAIT, dart_wait),
        (FLY, dart_fly),
    ],
};

/// Template 58: flies right.
pub static DART_RIGHT: ObjectKind = ObjectKind {
    name: "dart (right)",
    routines: &[
        (h0(0x25eb6), dart_right_init),
        (WAIT, dart_wait),
        (FLY, dart_fly),
    ],
};

/// Marker 18 (h0+$1895a): a left dart, `$15` pixels down its cell.
pub fn spawn_57(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(57), cell, 0, 0x15);
}

/// Marker 19 (h0+$1896c): a right dart, 8 right and `$11` down.
pub fn spawn_58(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(58), cell, 8, 0x11);
}

/// h0+$25e9a: its look (h0+$32b18); `$20` passes before it flies.
fn dart_left_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32b18));
    c.bus.set_w(slot + 0x64, 0x20);
}

/// h0+$25eb6: its look (h0+$32b24); `$20` passes before it flies.
fn dart_right_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32b24));
    c.bus.set_w(slot + 0x64, 0x20);
}

/// h0+$25fa8: harmless and out of sight (its bob at y `$2ee0`) while `+$64` counts down; then
/// back at its spawn point, flying for `$60` passes' worth of count, and hurting.
pub fn dart_wait(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 1);
    let t = b.w(slot + 0x64).wrapping_sub(1);
    b.set_w(slot + 0x64, t);
    let bob = b.l(slot + 0x50);
    if n_clear(t) {
        b.set_w(bob.wrapping_sub(2), 0x2ee0);
        return;
    }
    b.set_w(slot + 0x64, 0x60);
    b.set_w(bob.wrapping_sub(2), 0x2ee0);
    b.set_l(slot + 4, FLY);
    b.set_l(slot + 8, b.l(slot + 0x44));
    b.set_l(slot + 0x10, TOUCH_HURT);
}

/// h0+$25f42: it moves `+$0c` a pass; once the cell two steps ahead, 8 pixels up, is solid it
/// waits again, harmless.
pub fn dart_fly(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 0);
    let vx = b.w(slot + 0xc);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
    bob_follow(b, slot);
    b.set_w(slot + 0x64, b.w(slot + 0x64).wrapping_sub(1));
    let x = b.w(slot + 8).wrapping_add(vx.wrapping_add(vx));
    let cell = cell_at(b, x, b.w(slot + 0xa).wrapping_sub(8));
    if solid(b, cell) {
        b.set_l(slot + 4, WAIT);
        b.set_l(slot + 0x10, NOTHING);
    }
}
