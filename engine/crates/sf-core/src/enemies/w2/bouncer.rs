//! Templates 75 (marker 2) and 74 (marker 3): bounces along between a pair of markers 2 and 3 in
//! the map, its height from a hop table; immune, it hurts on contact. Only one spawns per pair: a
//! marker's stub looks for its partner and spawns nothing while that one is live.

use crate::enemies::common::{follow_and_despawn, marker_at};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{OBJ_SLOTS, bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `bouncer_hops` (h0+$26730): y offsets a word apart, ending at `$ffff`.
pub const HOPS: u32 = h0(0x26730);
const LOOK_LEFT: u32 = h0(0x31548);
const LOOK_RIGHT: u32 = h0(0x31554);
const UPDATE: u32 = h0(0x26786);

pub static BOUNCER: ObjectKind = ObjectKind {
    name: "bouncer",
    routines: &[(h0(0x266ec), bouncer_init), (UPDATE, bouncer_update)],
};

/// Template 74: spawned at marker 3, the pair's right end, 32 pixels left of it.
pub static BOUNCER_LEFT: ObjectKind = ObjectKind {
    name: "bouncer (from the right)",
    routines: &[(h0(0x266a2), bouncer_left_init), (UPDATE, bouncer_update)],
};

/// Marker 2, the pair's left end (h0+$1812a): a 75, unless no marker 3 lies within `$98` cells right of it or one
/// does with a live slot.
pub fn spawn_at_left_end(c: &mut LevelCtx, cell: u32) {
    if partner(c, cell, 3, true) {
        obj_spawn_cell(c, template(75), cell, 0, 0xa);
    }
}

/// Marker 3, the pair's right end (h0+$18148): a 74, unless no marker 2 lies within `$98` cells left of it or one does
/// with a live slot.
pub fn spawn_at_right_end(c: &mut LevelCtx, cell: u32) {
    if partner(c, cell, 2, false) {
        obj_spawn_cell(c, template(74), cell, 0, 0xa);
    }
}

/// h0+$1b350 (right) / h0+$1b396 (left): whether marker `m` lies within `$98` cells of `cell`
/// on that side with no live slot there.
fn partner(c: &LevelCtx, cell: u32, m: u16, right: bool) -> bool {
    let b = &c.bus;
    let found = (1..=0x98u32)
        .map(|k| {
            if right {
                cell.wrapping_add(2 * k)
            } else {
                cell.wrapping_sub(2 * k)
            }
        })
        .find(|&a| marker_at(b, a) == m);
    match found {
        None => false,
        Some(a) => !(0..20).any(|k| b.l(OBJ_SLOTS + k * 0x7c + 0x40) == a),
    }
}

/// h0+$266ec: the spawn point kept (`+$68`), the hop table started, `+$64` the map row's start
/// (its cell less its column).
fn bouncer_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, LOOK_LEFT);
    b.set_l(slot + 0x68, b.l(slot + 8));
    b.set_l(slot + 0x58, HOPS);
    b.set_l(slot + 0x54, HOPS);
    b.set_l(slot + 0x64, row_start(b.l(slot + 0x40), b.w(slot + 8)));
}

/// h0+$266a2: as the 75's, then 32 pixels left.
fn bouncer_left_init(c: &mut LevelCtx, slot: u32, d7: u16) {
    bouncer_init(c, slot, d7);
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_sub(0x20));
}

/// A cell less the column of pixel `x` (`ext.l`, `lsr.w #4`, `add.w`, `sub.l`: the high word
/// of a negative `x` is subtracted too).
pub fn row_start(cell: u32, x: u16) -> u32 {
    let high = (x as i16 as i32 as u32) & 0xffff_0000;
    cell.wrapping_sub(high | (x >> 4).wrapping_mul(2) as u32)
}

/// h0+$26786: the next hop's height (`+$6a` the spawn y; `h0+$1c462`), `vx` along; at a marker
/// 2 or 3 two cells right of its column it turns, facing its way (and, going right, a step on).
fn bouncer_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let y0 = b.w(slot + 0x6a);
    path_y(b, slot, y0);
    let vx = b.w(slot + 0xc);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
    let at = b
        .l(slot + 0x64)
        .wrapping_add((b.w(slot + 8) >> 4).wrapping_mul(2) as u32)
        .wrapping_add(4);
    if matches!(marker_at(b, at), 2 | 3) {
        let vx = vx.wrapping_neg();
        b.set_w(slot + 0xc, vx);
        bob_anim(b, slot, LOOK_LEFT);
        if n_clear(vx) {
            bob_anim(b, slot, LOOK_RIGHT);
            b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx.wrapping_mul(2)));
        }
    }
    follow_and_despawn(c, slot);
}

/// h0+$1c462: the next of a table of y offsets (`+$54`, starting over from `+$58` at `$ffff`),
/// from `y0`.
pub fn path_y(b: &mut crate::mem::Bus, slot: u32, y0: u16) {
    let mut at = b.l(slot + 0x54);
    if b.w(at) == 0xffff {
        at = b.l(slot + 0x58);
        b.set_l(slot + 0x54, at);
    }
    b.set_w(slot + 0xa, b.w(at).wrapping_add(y0));
    b.set_l(slot + 0x54, b.l(slot + 0x54).wrapping_add(2));
}
