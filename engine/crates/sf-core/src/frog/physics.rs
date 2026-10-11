//! The frog's motion: velocity, gravity, running and jumping, and the camera that follows it.

use super::{FROG, f};
use crate::level::LevelCtx;

/// h0+$db58: the frog moves by its velocity. A coordinate that goes negative is put back to 0 and
/// that velocity stopped.
pub fn frog_move(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::FROZEN) != 0 {
        return;
    }
    for (pos, vel) in [(f::X, f::VX), (f::Y, f::VY)] {
        let p = b.w(FROG + pos).wrapping_add(b.w(FROG + vel));
        b.set_w(FROG + pos, p);
        if (p as i16) < 0 {
            b.set_w(FROG + pos, 0);
            b.set_w(FROG + vel, 0);
        }
    }
}
