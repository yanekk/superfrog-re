//! Templates 59–62 (markers 18–21): a dart that waits hidden at its spawn point, then flies at
//! its template's speed until it reaches solid ground, and waits again. Only the inits are W4's;
//! the update (h0+$25fa8, h0+$25f42) is shared with templates 53–62, W6's darts among them (T40
//! ports the same two; one copy should stay when both have merged).

use crate::enemies::common::{NOTHING, TOUCH_HURT, cell_at, solid, touch_hurt};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, nothing, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// The darts' update while waiting (h0+$25fa8) and while flying (h0+$25f42).
pub const WAIT: u32 = h0(0x25fa8);
pub const FLY: u32 = h0(0x25f42);

/// Templates 59–62, by marker 18–21.
pub static DARTS: [ObjectKind; 4] = [
    ObjectKind {
        name: "dart (W4 marker 18)",
        routines: &[
            (h0(0x25ed2), init_59),
            (WAIT, dart_wait),
            (FLY, dart_fly),
            (TOUCH_HURT, touch_hurt),
            (NOTHING, nothing),
        ],
    },
    ObjectKind {
        name: "dart (W4 marker 19)",
        routines: &[
            (h0(0x25eee), init_60),
            (WAIT, dart_wait),
            (FLY, dart_fly),
            (TOUCH_HURT, touch_hurt),
            (NOTHING, nothing),
        ],
    },
    ObjectKind {
        name: "dart (W4 marker 20)",
        routines: &[
            (h0(0x25f0a), init_61),
            (WAIT, dart_wait),
            (FLY, dart_fly),
            (TOUCH_HURT, touch_hurt),
            (NOTHING, nothing),
        ],
    },
    ObjectKind {
        name: "dart (W4 marker 21)",
        routines: &[
            (h0(0x25f26), init_62),
            (WAIT, dart_wait),
            (FLY, dart_fly),
            (TOUCH_HURT, touch_hurt),
            (NOTHING, nothing),
        ],
    },
];

/// Marker 18 (h0+$18744): 2 left and `$18` down its cell.
pub fn spawn_59(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(59), cell, 0xfffe, 0x18);
}

/// Marker 19 (h0+$18756): `$18` down its cell.
pub fn spawn_60(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(60), cell, 0, 0x18);
}

/// Marker 20 (h0+$18768): `$15` down its cell.
pub fn spawn_61(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(61), cell, 0, 0x15);
}

/// Marker 21 (h0+$1877a): 8 right and `$11` down its cell.
pub fn spawn_62(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(62), cell, 8, 0x11);
}

/// The inits: a look; `$20` passes before it flies.
fn start(c: &mut LevelCtx, slot: u32, look: u32) {
    bob_anim(&mut c.bus, slot, look);
    c.bus.set_w(slot + 0x64, 0x20);
}

/// h0+$25ed2 (look h0+$32294).
fn init_59(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot, h0(0x32294));
}

/// h0+$25eee (look h0+$322a0).
fn init_60(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot, h0(0x322a0));
}

/// h0+$25f0a (look h0+$322ac).
fn init_61(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot, h0(0x322ac));
}

/// h0+$25f26 (look h0+$322b8).
fn init_62(c: &mut LevelCtx, slot: u32, _d7: u16) {
    start(c, slot, h0(0x322b8));
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
