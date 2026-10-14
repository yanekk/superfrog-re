//! Template 76 (marker 18): `w1_watcher`, scenery drawn into the background that turns to look
//! at the frog: one of eight animations (two directions each) by the direction from it to the
//! frog on the screen (h0+$da62).

use crate::enemies::common::{direction, follow_and_despawn};
use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::camera::{CAMERA_X, CAMERA_Y};
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind};

/// Its animations by direction (h0+$2697e): 16 longs.
pub const LOOKS: u32 = h0(0x2697e);

pub static WATCHER: ObjectKind = ObjectKind {
    name: "w1 watcher",
    routines: &[(h0(0x268ee), init), (h0(0x26904), update)],
};

/// `w1_watcher_init` (h0+$268ee): looking right (h0+$313d4).
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x313d4));
}

/// `w1_watcher_update` (h0+$26904): positions relative to the camera's low words.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (cx, cy) = (b.w(CAMERA_X + 2), b.w(CAMERA_Y + 2));
    let d = direction(
        b.w(slot + 8).wrapping_sub(cx),
        b.w(slot + 0xa).wrapping_sub(cy),
        b.w(FROG + f::X).wrapping_sub(cx),
        b.w(FROG + f::Y).wrapping_sub(cy),
    );
    let d = (d as i8).clamp(0, 0xf) as u32;
    let look = b.l(LOOKS + 4 * d);
    bob_anim(b, slot, look);
    follow_and_despawn(c, slot);
}
