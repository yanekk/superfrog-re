//! Template 145 (marker 25): spikes that shoot out and back with their animation. They hurt
//! only while the animation's counter (its bob's `+$1a`) is below 1; otherwise they are harmless.
//! Out of the spud's reach (`$7d00`).

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, touch_spud_damage};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Template 145.
pub static SPIKES: ObjectKind = ObjectKind {
    name: "spikes",
    routines: &[
        (h0(0x2712e), spikes_init),
        (h0(0x27144), spikes_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 25 (h0+$187b0): 4 up its cell.
pub fn spawn_145(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(145), cell, 0, 0xfffc);
}

/// h0+$2712e: its look (h0+$32504).
fn spikes_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32504));
}

/// h0+$27144: harmless (`+$30`) while its bob's counter is 1 or more (signed).
fn spikes_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    let held = n_clear(b.w(bob + 0x1a).wrapping_sub(1));
    b.set_w(slot + 0x30, held as u16);
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
