//! Templates 143 (marker 22) and 144 (marker 24): a floater drifting sideways (143 right, 144
//! left) at its template's speed and sinking, pulled up 2 pixels every fourth pass. At marker
//! `$17` it stops sinking and its drift dies away by 1 every 9 passes. Out of the spud's reach
//! (`$7d00`); contact hurts.

use crate::enemies::common::{TOUCH_SPUD_DAMAGE, map_cell, touch_spud_damage};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// The marker that stops it.
const STOP: u16 = 0x17;
/// Its update once stopped (h0+$27792).
const STOPPED: u32 = h0(0x27792);

/// Templates 143 and 144.
pub static FLOATER: ObjectKind = ObjectKind {
    name: "floater",
    routines: &[
        (h0(0x27704), floater_init),
        (h0(0x27720), floater_update),
        (STOPPED, floater_stopped),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 22 (h0+$1878c): `$12` down its cell.
pub fn spawn_143(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(143), cell, 0, 0x12);
}

/// Marker 24 (h0+$1879e): `$12` down its cell.
pub fn spawn_144(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(144), cell, 0, 0x12);
}

/// h0+$27704: its look (h0+$324f8); first pull in 3 passes.
fn floater_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x324f8));
    c.bus.set_w(slot + 0x5c, 2);
}

/// Its bob where it is; gone once off the screen.
fn show(c: &mut LevelCtx, slot: u32) {
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$27720: at a `$17` in the cell under (x + `$11`, y) it stops (`+$60` 8); else every
/// fourth pass 2 up. It moves by `+$0c`, `+$0e`.
fn floater_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let x = b.w(slot + 8).wrapping_add(0x11);
    let y = b.w(slot + 0xa);
    let cell = map_cell(b, (x >> 4).wrapping_mul(2), (y >> 4).wrapping_mul(4));
    if b.w(cell) & 0x3f == STOP {
        b.set_l(slot + 4, STOPPED);
        b.set_w(slot + 0x60, 8);
    } else {
        let t = b.w(slot + 0x5c).wrapping_sub(1);
        b.set_w(slot + 0x5c, t);
        if !n_clear(t) {
            b.set_w(slot + 0x5c, 3);
            b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_sub(2));
        }
    }
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    show(c, slot);
}

/// h0+$27792: every 9th pass `+$0c` steps toward 0; x moves by it.
fn floater_stopped(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x60).wrapping_sub(1);
    b.set_w(slot + 0x60, t);
    if !n_clear(t) {
        b.set_w(slot + 0x60, 8);
        let vx = b.w(slot + 0xc);
        if vx != 0 {
            b.set_w(
                slot + 0xc,
                if vx & 0x8000 == 0 {
                    vx - 1
                } else {
                    vx.wrapping_add(1)
                },
            );
        }
    }
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    show(c, slot);
}
