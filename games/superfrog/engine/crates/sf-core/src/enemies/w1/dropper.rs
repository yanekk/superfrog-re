//! Template 23 (marker 11): `w1_dropper`, which flies left on a bobbing path for `$4b` passes,
//! dropping a [`DROP`] every `$1e`, then climbs away. Its touch does nothing but it hurts the
//! frog (immune, not passive). Template 17: `w1_drop`, which falls until the cell under it is
//! solid and splats there, harmless.

use crate::enemies::common::{
    NOTHING, SPLAT, follow_and_despawn, marker_right_of, solid,
};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, nothing, obj_free, obj_spawn_at_xy_nodedup};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its bobbing path (h0+$24d74): y offsets, ending at `$80`.
pub const PATH: u32 = h0(0x24d74);
/// The update it climbs away with (h0+$24e48), and the drop's once it has splatted (h0+$24ba2).
pub const CLIMB: u32 = h0(0x24e48);
pub const SPLATTED: u32 = h0(0x24ba2);

pub static DROPPER: ObjectKind = ObjectKind {
    name: "w1 dropper",
    routines: &[
        (h0(0x24d3c), init),
        (h0(0x24dca), update),
        (CLIMB, climb),
        (NOTHING, nothing),
    ],
};

pub static DROP: ObjectKind = ObjectKind {
    name: "w1 drop",
    routines: &[
        (h0(0x24b0a), drop_init),
        (h0(0x24b20), drop_update),
        (SPLATTED, drop_splatted),
        (NOTHING, nothing),
    ],
};

/// `w1_dropper_init` (h0+$24d3c): its path from the start, its row (`+$6c`), drops every `$1e`
/// passes (`+$5c`), `$4b` passes (`+$60`) before it climbs.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x31290));
    b.set_l(slot + 0x64, PATH);
    b.set_l(slot + 0x68, PATH);
    b.set_w(slot + 0x6c, b.w(slot + 0xa));
    b.set_w(slot + 0x5c, 0x1e);
    b.set_w(slot + 0x60, 0x4b);
}

/// `w1_dropper_update` (h0+$24dca). Once its count runs out it climbs from the next pass on
/// (this one still bobs). A drop goes 2 pixels right of it and `$20` down. Freed left of
/// x `$20`.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let left = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, left);
    if !n_clear(left) {
        b.set_l(slot + 4, CLIMB);
    }
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if !n_clear(t) {
        b.set_w(slot + 0x5c, 0x1e);
        let (x, y) = (b.w(slot + 8).wrapping_add(2), b.w(slot + 0xa).wrapping_add(0x20));
        obj_spawn_at_xy_nodedup(c, template(17), x, y);
    }
    let b = &mut c.bus;
    let mut at = b.l(slot + 0x64);
    if b.w(at) == 0x80 {
        at = b.l(slot + 0x68);
        b.set_l(slot + 0x64, at);
    }
    b.set_l(slot + 0x64, b.l(slot + 0x64).wrapping_add(2));
    b.set_w(slot + 0xa, b.w(slot + 0x6c).wrapping_add(b.w(at)));
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    crate::level::objects::bob_follow(b, slot);
    if !n_clear(b.w(slot + 8).wrapping_sub(0x20)) {
        return obj_free(c, slot, 0);
    }
    crate::level::objects::obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$24e48: it climbs, faster each pass, still flying left; freed left of x `$20` or above
/// y `$10`.
fn climb(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let vy = b.w(slot + 0xe).wrapping_sub(1);
    b.set_w(slot + 0xe, vy);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(vy));
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    if !n_clear(b.w(slot + 8).wrapping_sub(0x20)) || !n_clear(b.w(slot + 0xa).wrapping_sub(0x10)) {
        return obj_free(c, slot, 0);
    }
    follow_and_despawn(c, slot);
}

/// `w1_drop_init` (h0+$24b0a): its animation (h0+$312a4).
fn drop_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x312a4));
}

/// `w1_drop_update` (h0+$24b20): it falls, a pixel faster each pass up to 12; once the cell
/// right of the one under x − 10 is solid it splats, passive and touching nothing.
fn drop_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if !n_clear(b.w(slot + 0xe).wrapping_sub(0xc)) {
        b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let (x, y) = (b.w(slot + 8).wrapping_sub(0xa), b.w(slot + 0xa));
    if solid(marker_right_of(b, x, y)) {
        b.set_l(slot + 4, SPLATTED);
        b.set_w(slot + 0x30, 1);
        b.set_l(slot + 0x10, NOTHING);
        bob_anim(b, slot, SPLAT);
    }
    follow_and_despawn(c, slot);
}

/// h0+$24ba2: freed once the splat's step timer reaches `$4e20` (its animation over).
fn drop_splatted(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &c.bus;
    let bob = b.l(slot + 0x50);
    if n_clear(b.w(bob + 0x1a).wrapping_sub(0x4e20)) {
        return obj_free(c, slot, 0);
    }
    follow_and_despawn(c, slot);
}
