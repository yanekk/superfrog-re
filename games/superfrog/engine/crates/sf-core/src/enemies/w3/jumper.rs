//! Template 95 (marker 28): `w3_jumper`, which walks its row turning at cells marked `$25`, and
//! after a while leaps up until it meets a ceiling or the top of its leap, circles there on the
//! hover's path (`w1_hover_path`) for a while, then drops to the ground and walks again. Walking,
//! a stomp flattens it (`touch_spud_damage` on `$1f` hit points) into a reward; in the air one hit
//! kills it (`touch_kill_1hit`). It hurts on any other touch.

use crate::enemies::common::{
    TOUCH_KILL_1HIT, TOUCH_SPUD_DAMAGE, follow_and_despawn, kill_fall, marker_at, row_base, solid,
    touch_kill_1hit, touch_spud_damage,
};
use crate::enemies::w1::hover;
use crate::enemies::w3::walkers::walk;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_path_xy};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `w3_jumper_update` (h0+$27a5e) and the leap (h0+$27b2a), circling (h0+$27bb4) and drop
/// (h0+$27be0) it turns to.
const WALK: u32 = h0(0x27a5e);
const LEAP: u32 = h0(0x27b2a);
const CIRCLE: u32 = h0(0x27bb4);
const DROP: u32 = h0(0x27be0);
/// Its animations: walking left (h0+$32140) and right (h0+$32170), in the air facing left
/// (h0+$3215c) and right (h0+$3218c).
const LEFT: u32 = h0(0x32140);
const RIGHT: u32 = h0(0x32170);
const AIR_LEFT: u32 = h0(0x3215c);
const AIR_RIGHT: u32 = h0(0x3218c);

pub static JUMPER: ObjectKind = ObjectKind {
    name: "w3 jumper",
    routines: &[
        (h0(0x27a18), init),
        (WALK, update),
        (LEAP, leap),
        (CIRCLE, circle),
        (DROP, drop),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
        (h0(0x23b34), kill_fall),
    ],
};

/// `w3_jumper_init` (h0+$27a18): its row's base, `$19` passes before it leaps, the hover's path.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LEFT);
    b.set_l(slot + 0x64, row_base(b, slot, 0x260));
    b.set_w(slot + 0x5c, 0x19);
    b.set_l(slot + 0x58, hover::PATH);
    b.set_l(slot + 0x54, hover::PATH);
}

/// `w3_jumper_update` (h0+$27a5e): walks while `+$5c` counts down; when the count is over it
/// leaps from the next pass: not stompable, killed by one hit, up at 9–12 pixels a pass, and it
/// will circle for 75–200 passes (`+$60`).
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if n_clear(t) {
        walk(b, slot, LEFT, RIGHT);
        return follow_and_despawn(c, slot);
    }
    b.set_l(slot + 4, LEAP);
    b.set_w(slot + 0x2e, 0);
    let anim = if n_clear(b.w(slot + 0xc)) {
        AIR_RIGHT
    } else {
        AIR_LEFT
    };
    bob_anim(b, slot, anim);
    b.set_l(slot + 0x10, TOUCH_KILL_1HIT);
    let vy = c.rng_range(3).wrapping_add(9).wrapping_neg();
    c.bus.set_w(slot + 0xe, vy);
    let circle = c.rng_range(0x7d).wrapping_add(0x4b);
    c.bus.set_w(slot + 0x60, circle);
    follow_and_despawn(c, slot);
}

/// The marker of the cell right of the one under (x − 10, y), `extra` bytes on (a row is `$260`).
fn marker_by(c: &LevelCtx, slot: u32, extra: u32) -> u16 {
    let b = &c.bus;
    let x = b.w(slot + 8).wrapping_sub(0xa);
    let y = b.w(slot + 0xa);
    marker_at(
        b,
        ((x >> 4).wrapping_add(x >> 4)) as u32,
        (y >> 4).wrapping_mul(4),
        extra,
    )
}

/// h0+$27b2a: it rises, slowing a pixel a pass. Once the cell up and right of the one under
/// (x − 10, y) is solid, or it stops, it circles the hover's path round 10 left of where it is.
fn leap(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let vy = b.w(slot + 0xe);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(vy));
    b.set_w(slot + 0xe, vy.wrapping_add(1));
    if !solid(marker_by(c, slot, 0u32.wrapping_sub(0x25e))) && c.bus.w(slot + 0xe) != 0 {
        return follow_and_despawn(c, slot);
    }
    let b = &mut c.bus;
    b.set_w(slot + 0xe, 0);
    b.set_l(slot + 0x58, hover::PATH);
    b.set_l(slot + 0x54, hover::PATH);
    b.set_l(slot + 4, CIRCLE);
    b.set_l(slot + 0x68, b.l(slot + 8));
    b.set_l(slot + 0x6c, b.l(slot + 0x68));
    b.set_w(slot + 0x68, b.w(slot + 0x68).wrapping_sub(0xa));
    b.set_w(slot + 0x6c, b.w(slot + 0x6c).wrapping_sub(0xa));
    follow_and_despawn(c, slot);
}

/// h0+$27bb4: the path's next offset from `+$68`; once `+$60` has counted down past 0 it drops.
fn circle(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (x, y) = (b.w(slot + 0x68), b.w(slot + 0x6a));
    obj_path_xy(b, slot, x, y);
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if !n_clear(t) {
        b.set_l(slot + 4, DROP);
    }
    follow_and_despawn(c, slot);
}

/// h0+$27be0: it falls, a pixel a pass faster. Once the cell down and right of the one under
/// (x − 10, y) is solid it stands 4 into its cell and walks again, stompable, the way it faced,
/// for 50–100 passes before the next leap.
fn drop(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let vy = b.w(slot + 0xe).wrapping_add(1);
    b.set_w(slot + 0xe, vy);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(vy));
    if solid(marker_by(c, slot, 0x262)) {
        let b = &mut c.bus;
        b.set_w(slot + 0xa, (b.w(slot + 0xa) & 0xfff0).wrapping_add(4));
        b.set_l(slot + 0x10, TOUCH_SPUD_DAMAGE);
        b.set_l(slot + 4, WALK);
        b.set_w(slot + 0x2e, 1);
        let bob = b.l(slot + 0x50);
        let anim = if b.l(bob + 8) == AIR_LEFT {
            LEFT
        } else {
            RIGHT
        };
        bob_anim(b, slot, anim);
        let walk = c.rng_range(0x32).wrapping_add(0x32);
        c.bus.set_w(slot + 0x5c, walk);
    }
    follow_and_despawn(c, slot);
}
