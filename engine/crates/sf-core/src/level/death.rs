//! Losing energy, lives and the level (docs/09 "The frog"): an enemy's touch takes a heart
//! (`frog_hurt`), a hazard or the last heart kills the frog (`frog_kill`, T14's), time running out
//! ends the game (`frog_die`); a dying frog sets `level_state` to `$190` and the level's loop ends
//! when it has counted down.

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::collision::{LEFT_ANIMS, LEVEL_STATE, NO_DEATH, frog_kill, marker};
use crate::level::{ATTRACT_MODE, LevelCtx, n_clear};

/// `opt_game_mode` (h0+$9e6): 0 easy, 1 normal, 2 hard (one touch kills).
const OPT_GAME_MODE: u32 = h0(0x9e6);
/// `level_clock` (h0+$876): four digits, minutes first.
const LEVEL_CLOCK: u32 = h0(0x876);

/// `frog_hurt` (h0+$1c5bc): an enemy's touch (`slot`) takes a heart (`+$5a`): the frog is thrown
/// up and, unless a wall or solid ground stands beside it, two pixels a frame sideways (left for
/// the animations from `$10b7e` on, right for those before), and is hurt (not hurt again) until it
/// lands. With no heart left, or in the bonus level or the hard game, it dies.
pub fn frog_hurt(c: &mut LevelCtx, _slot: u32) {
    let b = &mut c.bus;
    if b.l(ATTRACT_MODE) != 0 || b.w(NO_DEATH) != 0 {
        return;
    }
    if b.w(crate::level::camera::SHOOTER_MODE) != 0 || b.w(OPT_GAME_MODE) == 2 {
        return frog_kill(c);
    }
    for o in [f::BLINK, f::SHIELD, f::HURT, f::DYING] {
        if b.w(FROG + o) != 0 {
            return;
        }
    }
    let hearts = b.w(FROG + 0x5a).wrapping_sub(1);
    b.set_w(FROG + 0x5a, hearts);
    if !n_clear(hearts) {
        return frog_kill(c);
    }
    b.set_w(FROG + f::HURT, 1);
    let effect = c.rng_range(1).wrapping_add(0x1a);
    c.sfx(effect as u8, 3);
    let b = &mut c.bus;
    b.set_w(FROG + f::VY, 0xfffc);
    let high = crate::level::collision::n_clear_l(b.l(FROG + f::ANIM).wrapping_sub(LEFT_ANIMS));
    let (wall, cells, vx) = if high {
        (f::WALL_LEFT, [0x4c4, 0x4c2], 0xfffe)
    } else {
        (f::WALL_RIGHT, [0x4be, 0x4bc], 2)
    };
    if b.w(FROG + wall) == 0 {
        let ground = b.l(FROG + f::GROUND_CELL);
        if cells
            .iter()
            .any(|&o| marker(b, ground.wrapping_sub(o)) >= 0x3a)
        {
            return;
        }
        b.set_w(FROG + f::VX, vx);
    }
    b.set_w(FROG + f::AIR, 1);
}

/// `time_up_check` (h0+$dd6): at 0:00 the frog dies (`frog_die`).
pub fn time_up_check(c: &mut LevelCtx) {
    if c.bus.l(LEVEL_CLOCK) == 0 {
        frog_die(c);
    }
}

/// `frog_die` (h0+$de0): time is up. The lives go to -1 first, so this death ends the game
/// (unless the attract demo or the no-death cheat runs); the frog dies as `frog_kill` has it,
/// but where it stands, and the level ends `$190` passes later.
pub fn frog_die(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_w(FROG + f::LIVES, 0xffff);
    if b.l(ATTRACT_MODE) != 0 || b.w(NO_DEATH) != 0 || b.w(FROG + f::DYING) != 0 {
        return;
    }
    b.set_l(LEVEL_STATE, 0x190);
    b.set_w(FROG + f::DYING, 1);
    b.set_w(FROG + f::HURT, 1);
    b.set_w(FROG + f::LIVES, b.w(FROG + f::LIVES).wrapping_sub(1));
    crate::frog::frog_life_reset(c);
    c.sample(0);
    let b = &mut c.bus;
    b.set_w(FROG + f::ON_GROUND, 0);
    crate::frog::anim::frog_set_anim(c, h0(0x10646));
    let b = &mut c.bus;
    b.set_w(FROG + f::VX, 0);
    b.set_w(FROG + f::VY, 0);
    b.set_w(FROG + f::AIR, 1);
}
