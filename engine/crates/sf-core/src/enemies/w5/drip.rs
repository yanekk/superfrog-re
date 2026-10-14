//! Template 31 (marker 11): a drop that falls from its spawn point once its animation has shown
//! four steps, splashes on the first solid cell for 8 passes and starts again from where it
//! spawned. Passive: it touches the frog without hurting it (`touch_kill_1hit` on `$7d00` hit
//! points, so nothing happens).

use crate::enemies::common::{
    TOUCH_KILL_1HIT, follow_and_despawn, map_cell, solid, touch_kill_1hit,
};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its steps: falling (h0+$25274), splashing (h0+$25306).
const FALL: u32 = h0(0x25274);
const SPLASH: u32 = h0(0x25306);

pub static DRIP: ObjectKind = ObjectKind {
    name: "w5 drip",
    routines: &[
        (h0(0x2525e), init),
        (FALL, fall),
        (SPLASH, splash),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Marker 11 (h0+$188e2): `$b` down its cell.
pub fn spawn_31(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(31), cell, 0, 0xb);
}

/// h0+$2525e: its look (h0+$327a8).
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x327a8));
}

/// h0+$25274: once its bob's step count is 4 or more it falls a pixel a pass faster every other
/// pass, to `$f`; y moves by `+$0e` all the same. In a solid cell (the one under x, y) its bob's
/// count is cleared and it splashes for 8 passes, 4 pixels into the top of that cell.
fn fall(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 1);
    let bob = b.l(slot + 0x50);
    if n_clear(b.w(bob + 0x1a).wrapping_sub(4)) {
        let flip = !b.w(slot + 0x5c);
        b.set_w(slot + 0x5c, flip);
        if flip != 0 && b.w(slot + 0xe) != 0xf {
            b.set_w(slot + 0xe, b.w(slot + 0xe).wrapping_add(1));
        }
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa));
    let at = map_cell(b, (x >> 4).wrapping_mul(2), (y >> 4).wrapping_mul(4));
    if solid(b.w(at) & 0x3f) {
        b.set_w(bob + 0x1a, 0);
        b.set_l(slot + 4, SPLASH);
        b.set_w(slot + 0x5c, 8);
        b.set_w(slot + 0xa, (b.w(slot + 0xa) & 0xfff0).wrapping_add(4));
    }
    follow_and_despawn(c, slot);
}

/// h0+$25306: when `+$5c` runs out it is back at its spawn height (`+$46`), still, falling again.
fn splash(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 1);
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if t == 0 {
        b.set_w(slot + 0xe, 0);
        b.set_w(slot + 0xa, b.w(slot + 0x46));
        b.set_l(slot + 4, FALL);
    }
    follow_and_despawn(c, slot);
}
