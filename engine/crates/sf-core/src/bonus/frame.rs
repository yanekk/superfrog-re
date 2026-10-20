//! The bonus level's loop as `top_loop` runs it (h0+$618..$666): after a death with lives left
//! the level is not set up again, the frog is put back on the screen where the camera is
//! (`bonus_frame`, h0+$6a64) and the loop runs on, in the same frame.

use crate::frog;
use crate::layout::h0;
use crate::level::LevelCtx;
use crate::mem::Bus;

/// `level_state` (h0+$a06), `frog_lives` (h0+$82e), `level_started` (h0+$9f0).
const LEVEL_STATE: u32 = h0(0xa06);
const LIVES: u32 = h0(0x82e);
const LEVEL_STARTED: u32 = h0(0x9f0);

/// Whether `top_loop` goes round the bonus level's loop again once `level_frame_loop` has
/// returned (h0+$63e): in the bonus level, `level_state` still negative (a death, or ESC) and
/// a life left. Otherwise the exit fade follows.
pub fn goes_on(b: &Bus) -> bool {
    crate::level::load::current(b).is_some_and(|id| id.world == sf_assets::World::Bonus)
        && (b.l(LEVEL_STATE) as i32) < 0
        && (b.w(LIVES) as i16) >= 0
}

/// `top_loop`'s h0+$626: `frog_reset`, then [`bonus_frame`].
pub fn back_on_screen(c: &mut LevelCtx) {
    frog::frog_reset(&mut c.bus);
    bonus_frame(c);
}

/// `bonus_frame` (h0+$6a64): the frog's step and sprites as `level_start` makes them, the
/// panel's sprite digits, the lives count and icon, and the frog placed on the screen
/// (`frog_drop_in_start`), which sets the loop running again. The music plays on.
pub fn bonus_frame(c: &mut LevelCtx) {
    c.bus.set_w(LEVEL_STARTED, 1);
    crate::level::load::frog_first_step(c);
    crate::level::panel::sprite_digits(&mut c.bus);
    crate::level::panel::lives_count(c);
    crate::level::panel::lives_icon_draw(c);
    crate::level::load::drop_in(c);
}
