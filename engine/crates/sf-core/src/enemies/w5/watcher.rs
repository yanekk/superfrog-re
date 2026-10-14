//! Template 29 (marker 22): scenery that turns to look at the frog, as world 1's watcher does, by
//! the direction from it to the frog on the screen (h0+$da62), and every so often blinks: after 8
//! passes, then after 50–70, it shows its blink for 5. Passive, no hitbox, touches nothing.

use crate::enemies::common::{NOTHING, direction, follow_and_despawn};
use crate::frog::{FROG, f};
use crate::items::template;
use crate::layout::h0;
use crate::level::camera::{CAMERA_X, CAMERA_Y};
use crate::level::objects::{bob_anim, nothing, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// Its animations by direction (h0+$25106): 16 longs.
pub const LOOKS: u32 = h0(0x25106);
/// Its steps: looking (h0+$2502a), blinking (h0+$250d0).
const LOOK: u32 = h0(0x2502a);
const BLINK: u32 = h0(0x250d0);

pub static WATCHER: ObjectKind = ObjectKind {
    name: "w5 watcher",
    routines: &[
        (h0(0x2500e), init),
        (LOOK, look),
        (BLINK, blink),
        (NOTHING, nothing),
    ],
};

/// Marker 22 (h0+$188be): `$c` down its cell.
pub fn spawn_29(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(29), cell, 0, 0xc);
}

/// h0+$2500e: looking right (h0+$329a4), blinking in 8 passes.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x329a4));
    c.bus.set_w(slot + 0x5c, 8);
}

/// h0+$2502a: when `+$5c` runs out it blinks (h0+$32a04, its `+$42` looking right) for 5 passes;
/// until then it looks at the frog, positions relative to the camera's low words.
fn look(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if t == 0 {
        b.set_l(slot + 4, BLINK);
        b.set_w(slot + 0x5c, 5);
        let bob = b.l(slot + 0x50);
        b.set_l(bob + 0x28, h0(0x32a04));
        b.set_l(bob + 0x42, h0(0x329a4));
        follow_and_despawn(c, slot);
        return;
    }
    let (cx, cy) = (b.w(CAMERA_X + 2), b.w(CAMERA_Y + 2));
    let d = direction(
        b.w(slot + 8).wrapping_sub(cx),
        b.w(slot + 0xa).wrapping_sub(cy),
        b.w(FROG + f::X).wrapping_sub(cx),
        b.w(FROG + f::Y).wrapping_sub(cy),
    );
    let d = (d as i8).clamp(0, 0xf) as u32;
    let look = b.l(LOOKS + 4 * d);
    bob_anim(b, slot, look);
    follow_and_despawn(c, slot);
}

/// h0+$250d0: when `+$5c` runs out it looks again, its next blink 50–70 passes on (the RNG's).
fn blink(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let t = c.bus.w(slot + 0x5c).wrapping_sub(1);
    c.bus.set_w(slot + 0x5c, t);
    if t == 0 {
        c.bus.set_l(slot + 4, LOOK);
        let n = c.rng_range(0x14).wrapping_add(0x32);
        c.bus.set_w(slot + 0x5c, n);
    }
    follow_and_despawn(c, slot);
}
