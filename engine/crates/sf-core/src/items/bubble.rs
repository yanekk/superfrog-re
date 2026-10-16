//! Template 86: the bubble a frog standing on a pushing surface under a marker-`$39` cell lets
//! rise every 64 frames (`frog_air_anim`, h0+$11304; W3-4's start, T204). It rises 2 pixels a
//! pass, wobbling a pixel either side of where it rose, until the cells round it meet a marker
//! `$18`, `$e` or `$f` or a solid one, where it goes. Immune and passive; a touch does nothing.

use crate::layout::h0;
use crate::level::collision::MAP_ROWS;
use crate::level::objects::{nothing, obj_despawn_if_offscreen, obj_free, obj_spawn_at_xy};
use crate::level::{Dispatch, LevelCtx, ObjectKind, n_clear, register};

use super::template;

pub static BUBBLE: ObjectKind = ObjectKind {
    name: "bubble",
    routines: &[
        (h0(0x24216), init),
        (h0(0x24236), update),
        (h0(0x7d4), nothing),
    ],
};

/// Template 86, and the frog's spawn of it, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 86, &BUBBLE);
    d.bubble = Some(spawn);
}

/// h0+$11304: a bubble at level pixel `x`, `y` (`obj_spawn_at_xy`).
fn spawn(c: &mut LevelCtx, x: u16, y: u16) {
    obj_spawn_at_xy(c, template(86), x, y);
}

/// h0+$24216: its look, and the x it wobbles round (`+$5c`); the wobble's step (`+$60`) 0.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, h0(0x321c8));
    b.set_l(bob + 0x42, h0(0x321c8));
    b.set_w(slot + 0x5c, b.w(slot + 8));
    b.set_w(slot + 0x60, 0);
}

/// Whether a marker ends the bubble: `$18`, `$e` or `$f`, and, where `solid`, `$3a` and above.
fn ends(m: u16, water: bool, solid: bool) -> bool {
    m == 0x18 || (water && (m == 0xe || m == 0xf)) || (solid && n_clear(m.wrapping_sub(0x3a)))
}

/// h0+$24236: the wobble steps 0, 1, 2 (x one right, one left, back); it rises by `+$e`. The
/// cell under its x and 8 above its y, the one right of it, and the two pairs below are looked
/// at, each pair with its own markers (the original's tests, uneven as they are); one that ends
/// it frees it, otherwise its bob follows and it goes once off the screen.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let mut step = b.w(slot + 0x60).wrapping_add(1);
    if n_clear(step.wrapping_sub(3)) {
        step = 0;
    }
    b.set_w(slot + 0x60, step);
    let mut x = b.w(slot + 0x5c);
    if step == 0 {
        x = x.wrapping_add(1);
    }
    if step == 1 {
        x = x.wrapping_sub(1);
    }
    b.set_w(slot + 8, x);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let col = ((x >> 4).wrapping_add(x >> 4)) as u32;
    let row = (b.w(slot + 0xa).wrapping_sub(8) >> 4).wrapping_mul(4);
    let cell = b
        .l(MAP_ROWS.wrapping_add(row as i16 as i32 as u32))
        .wrapping_add(col);
    let m = |at: u32| b.w(at) & 0x3f;
    // (cell, marker $e/$f ends it, a solid marker ends it), in the original's order.
    let tests = [
        (cell, true, false),
        (cell.wrapping_add(2), true, true),
        (cell.wrapping_add(0x260), false, false),
        (cell.wrapping_add(0x262), true, true),
        (cell.wrapping_add(0x4c0), true, false),
        (cell.wrapping_add(0x4c2), false, false),
    ];
    if tests.iter().any(|&(at, water, solid)| ends(m(at), water, solid)) {
        return obj_free(c, slot, 0);
    }
    let bob = b.l(slot + 0x50);
    b.set_l(bob.wrapping_sub(4), b.l(slot + 8));
    obj_despawn_if_offscreen(c, slot, 0);
}
