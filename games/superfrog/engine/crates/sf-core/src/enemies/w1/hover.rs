//! Template 8 (marker 1): `w1_hover`, which loops round a centre on one of four phases of a path
//! and drifts its centre after the frog, up to 42 pixels from where it spawned, facing it. One
//! hit or stomp kills it (`touch_kill_1hit`).

use crate::enemies::common::{TOUCH_KILL_1HIT, follow_and_despawn, kill_fall, touch_kill_1hit};
use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_path_xy};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// `w1_hover_phase` (h0+$243b4): the path phase the next one spawned starts on, 0..3.
pub const PHASE: u32 = h0(0x243b4);
/// Its path (h0+$24454): pixel offsets (x, y words), four phases of `$cc` bytes, a −1 long
/// after the last.
pub const PATH: u32 = h0(0x24454);
/// Its animations: facing right (h0+$31064) and left (h0+$31048).
const FACING_RIGHT: u32 = h0(0x31064);
const FACING_LEFT: u32 = h0(0x31048);

pub static HOVER: ObjectKind = ObjectKind {
    name: "w1 hover",
    routines: &[
        (h0(0x2435a), init),
        (h0(0x243b6), update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
        (h0(0x23b34), kill_fall),
    ],
};

/// `w1_hover_init` (h0+$2435a): its centre (`+$64`) and start (`+$68`) where it spawned, its
/// path from the next phase on.
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, FACING_RIGHT);
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x68, b.l(slot + 0x64));
    b.set_l(slot + 0x58, PATH);
    let phase = b.w(PHASE);
    b.set_l(slot + 0x54, PATH.wrapping_add(phase as u32 * 0xcc));
    let next = phase.wrapping_add(1);
    b.set_w(PHASE, if next < 4 { next } else { 0 });
}

/// `w1_hover_update` (h0+$243b6): the path's next offset from the centre; the centre a pixel
/// towards the frog on each axis while within 42 of the start; the animation facing the frog.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (cx, cy) = (b.w(slot + 0x64), b.w(slot + 0x66));
    obj_path_xy(b, slot, cx, cy);
    for (frog, pos, centre) in [(b.w(FROG + f::X), 8, 0x64), (b.w(FROG + f::Y), 0xa, 0x66)] {
        let off = b.w(slot + centre + 4).wrapping_sub(b.w(slot + centre));
        let here = b.w(slot + pos);
        if frog < here && !n_clear(off.wrapping_sub(0x2a)) {
            b.set_w(slot + centre, b.w(slot + centre).wrapping_sub(1));
        } else if frog > here && n_clear(off.wrapping_sub(0xffd6)) {
            b.set_w(slot + centre, b.w(slot + centre).wrapping_add(1));
        }
    }
    let anim = if n_clear(b.w(slot + 8).wrapping_sub(b.w(FROG + f::X))) {
        FACING_LEFT
    } else {
        FACING_RIGHT
    };
    let bob = b.l(slot + 0x50);
    if b.l(bob + 8) != anim + 8 {
        b.set_l(bob + 0x42, anim + 8);
        b.set_l(bob + 0x28, anim);
    }
    follow_and_despawn(c, slot);
}
