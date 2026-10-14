//! Template 101 (marker 12): a cannon that turns to the frog in five steps and fires a shot
//! (template 109) its way every `$30`..`$50` passes. It cannot be touched.

use crate::enemies::common::{NOTHING, TOUCH_KILL_1HIT};
use crate::enemies::w6::shot::{SHOT_UPDATE, shot_update};
use crate::frog::{FROG, f};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    bob_anim, bob_follow, nothing, obj_despawn_if_offscreen, obj_spawn_at_xy_nodedup,
    obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `cannon_aim` (h0+$28454): two words inside the code: the next shot's vx and vy.
pub const CANNON_AIM: u32 = h0(0x28454);

/// Template 101: the cannon.
pub static CANNON: ObjectKind = ObjectKind {
    name: "cannon",
    routines: &[
        (h0(0x28324), cannon_init),
        (h0(0x28340), cannon_update),
        (NOTHING, nothing),
    ],
};

/// Template 109: its shot.
pub static CANNON_SHOT: ObjectKind = ObjectKind {
    name: "cannon shot",
    routines: &[
        (h0(0x27e1a), cannon_shot_init),
        (SHOT_UPDATE, shot_update),
        (TOUCH_KILL_1HIT, crate::enemies::common::touch_kill_1hit),
    ],
};

/// Marker 12 (h0+$1892a): a cannon, 4 pixels up its cell.
pub fn spawn_101(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(101), cell, 0, 0xfffc);
}

/// h0+$28324: its look (h0+$32e10); `$10` passes to the first shot.
fn cannon_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32e10));
    c.bus.set_w(slot + 0x5c, 0x10);
}

/// h0+$28340: by how far the frog is left or right of it (its x less `$20` from the frog's), one
/// of five looks and the shot's speed (`cannon_aim`); each time `+$5c` runs out, a shot 8 right
/// and `$b` down, and `$30` plus the RNG's 0..=`$20` passes to the next.
fn cannon_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let d = b
        .w(FROG + f::X)
        .wrapping_add(0x20)
        .wrapping_sub(b.w(slot + 8))
        .wrapping_neg();
    let (look, vx) = if !n_clear(d.wrapping_sub(0xff80)) {
        (0x32e10, 5)
    } else if !n_clear(d.wrapping_sub(0xffc0)) {
        (0x32e1c, 2)
    } else if !n_clear(d) {
        (0x32e28, 0)
    } else if !n_clear(d.wrapping_sub(0x40)) {
        (0x32e34, 0xfffe)
    } else {
        (0x32e40, 0xfffb)
    };
    b.set_w(CANNON_AIM, vx);
    b.set_w(CANNON_AIM + 2, 4);
    bob_anim(b, slot, h0(look));
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if t == 0 {
        let wait = c.rng_range(0x20).wrapping_add(0x30);
        let b = &mut c.bus;
        b.set_w(slot + 0x5c, wait);
        let (x, y) = (
            b.w(slot + 8).wrapping_add(8),
            b.w(slot + 0xa).wrapping_add(0xb),
        );
        let r = obj_spawn_at_xy_nodedup(c, template(109), x, y);
        if r.d0 == 0 && r.a0 != 0 {
            let b = &mut c.bus;
            b.set_w(r.a0 + 0xc, b.w(CANNON_AIM));
            b.set_w(r.a0 + 0xe, b.w(CANNON_AIM + 2));
        }
    }
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$27e1a: the shot's looks (h0+$32b0c, then h0+$32b00).
fn cannon_shot_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, h0(0x32b0c));
    b.set_l(bob + 0x42, h0(0x32b00));
    b.set_w(slot + 0x5c, 0);
}
