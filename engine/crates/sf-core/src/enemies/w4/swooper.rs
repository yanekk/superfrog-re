//! Template 135 (marker 1): flies a loop of positions around its spawn point. Spawns take the
//! loop's four quarters in turn (`w1_hover_phase`, which W1's hover shares), so a flock spreads out. One hit kills it; it
//! can be stomped.

use crate::enemies::common::{TOUCH_KILL_1HIT, touch_kill_1hit};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    bob_anim, bob_follow, obj_despawn_if_offscreen, obj_path_xy, obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind};

/// `w1_hover_phase` (h0+$243b4): a word inside the code, the quarter the next spawn starts on.
pub const SWOOPER_PHASE: u32 = h0(0x243b4);
/// The loop (`w1_hover_path`, h0+$24454): 204 positions (x, y words), four quarters of `$cc` bytes, ending with
/// a −1 long.
pub const LOOP: u32 = h0(0x24454);
const QUARTER: u32 = 0xcc;

/// Template 135.
pub static SWOOPER: ObjectKind = ObjectKind {
    name: "swooper",
    routines: &[
        (h0(0x25936), swooper_init),
        (h0(0x25990), swooper_update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// Marker 1 (h0+$1856c): at its cell's corner.
pub fn spawn_135(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(135), cell, 0, 0);
}

/// h0+$25936: its look (h0+$32220); its origin (`+$64`, also kept at `+$68`) its spawn point;
/// the loop from the quarter `w1_hover_phase` names, which moves on (0..3).
fn swooper_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, h0(0x32220));
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x68, b.l(slot + 0x64));
    b.set_l(slot + 0x58, LOOP);
    b.set_l(slot + 0x54, LOOP);
    let phase = b.w(SWOOPER_PHASE);
    b.set_l(slot + 0x54, LOOP.wrapping_add(phase as u32 * QUARTER));
    let next = phase.wrapping_add(1);
    b.set_w(SWOOPER_PHASE, if next < 4 { next } else { 0 });
}

/// h0+$25990: the loop's next position from its origin; gone once off the screen.
fn swooper_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (dx, dy) = (b.w(slot + 0x64), b.w(slot + 0x66));
    obj_path_xy(b, slot, dx, dy);
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
