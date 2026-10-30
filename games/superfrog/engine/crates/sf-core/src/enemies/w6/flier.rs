//! Template 103 (marker 14): a flier that loops along a path from its spawn point, faces the frog,
//! and drops a shot (template 107) toward it every `$40` passes. One hit or stomp kills it.

use crate::enemies::common::{TOUCH_KILL_1HIT, touch_kill_1hit};
use crate::frog::{FROG, f};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    bob_anim, bob_follow, obj_despawn_if_offscreen, obj_path_xy, obj_spawn_at_xy_nodedup,
    obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `flier_facing_right` (h0+$28322): a word inside the code: 1 while the frog is right of the
/// flier being updated.
pub const FLIER_FACING_RIGHT: u32 = h0(0x28322);
/// Its path (h0+$2820a): positions (x, y words) ending with a −1 long.
pub const FLIER_PATH: u32 = h0(0x2820a);

/// Template 103.
pub static FLIER: ObjectKind = ObjectKind {
    name: "flier",
    routines: &[
        (h0(0x281d8), flier_init),
        (h0(0x2826e), flier_update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Marker 14 (h0+$1894a): a flier, 7 down its cell.
pub fn spawn_103(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(103), cell, 0, 7);
}

/// h0+$281d8: its look (h0+$32ae0, going on as h0+$32ad8), its path, its origin (`+$64`, its
/// position), `$40` passes to the first shot.
fn flier_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, h0(0x32ae0));
    b.set_l(bob + 0x42, h0(0x32ad8));
    b.set_l(slot + 0x54, FLIER_PATH);
    b.set_l(slot + 0x58, FLIER_PATH);
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_w(slot + 0x5c, 0x40);
}

/// h0+$2826e: the next path position from its origin; facing the frog (h0+$32ad8 left,
/// h0+$32aec right, restarted only on a change); every `$40` passes a shot from `$c` lower,
/// falling at 5 and moving 6 toward the frog (from `$18` further right when right).
fn flier_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (dx, dy) = (b.w(slot + 0x64), b.w(slot + 0x66));
    obj_path_xy(b, slot, dx, dy);
    b.set_w(FLIER_FACING_RIGHT, 0);
    let mut look = h0(0x32ad8);
    if n_clear(b.w(FROG + f::X).wrapping_add(8).wrapping_sub(b.w(slot + 8))) {
        look = h0(0x32aec);
        b.set_w(FLIER_FACING_RIGHT, 1);
    }
    let bob = b.l(slot + 0x50);
    if b.l(bob + 8) != look {
        bob_anim(b, slot, look);
    }
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if t == 0 {
        b.set_w(slot + 0x5c, 0x40);
        let (x, y) = (b.w(slot + 8), b.w(slot + 0xa).wrapping_add(0xc));
        let r = obj_spawn_at_xy_nodedup(c, template(107), x, y);
        if r.d0 == 0 && r.a0 != 0 {
            let b = &mut c.bus;
            let shot = r.a0;
            if b.w(FLIER_FACING_RIGHT) == 0 {
                b.set_w(shot + 0xc, 0xfffa);
                b.set_w(shot + 0xe, 5);
            } else {
                b.set_w(shot + 0xc, 6);
                b.set_w(shot + 0xe, 5);
                b.set_w(shot + 8, b.w(shot + 8).wrapping_add(0x18));
            }
        }
    }
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
