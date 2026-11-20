//! The game over screen (plans/forest-dumb-level-1 T23, DESIGN §2.4): `main_loop`'s check that
//! enters it, `game_over_screen` (`h0+$4c4`) while it shows, and what follows it here.
//!
//! The original's `main_loop` (`h0+$252`) checks, whenever `irq_count` is a multiple of 32, for
//! `give_up`, then `lives` 0, then `countdown` 0, and goes to `h0+$1aa`: `game_over_screen`, then the
//! high scores (`h0+$89d0`), then the title. `game_over_screen` points the copper list at `h1+$810`
//! and its bitplanes at a picture in hunk 4, waits 21 frames (`h0+$8a78`, `d7 = $14`), then waits
//! for fire to be pressed and released. No pass of play runs meanwhile: that copper list raises no
//! interrupt (measured, T23: `irq_count` stays, `game_frame` never runs).
//!
//! **The high scores are skipped** (DESIGN §2.4, owner 2026-11-01): the original never saves them
//! and there is no title to return to, so where it would enter `h0+$89d0` the engine restarts the
//! level with full lives and score zero, as [`crate::Game::new`] starts it. A conformance case ends
//! its comparison there (`fd-conform`).
//!
//! Measured on the oracle (T23, `fd-conform/tests/death.rs`): Esc released in a pass sets `give_up`;
//! the step after pass 32k ends enters the screen; fire held in the 21st step after that or later is
//! taken as pressed, and the step it is no longer held in is the one the high scores start in.

use crate::State;

/// Steps after the one that entered the screen before fire counts (`h0+$8a78`'s 21 frames).
pub const FIRE_AFTER: u32 = 21;

/// `main_loop`'s check (`h0+$264`..`$282`), when the foreground reaches it after a pass: whether
/// the game is over. It runs only while `irq_count` is a multiple of 32.
pub fn game_is_over(s: &State) -> bool {
    s.irq_count & 0x1f == 0 && (s.give_up != 0 || s.lives == 0 || s.countdown >> 16 == 0)
}

/// `game_over_screen`'s wait, a step at a time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GameOver {
    /// Steps since the one that entered the screen.
    pub steps: u32,
    /// Fire has been pressed since the wait ended; its release ends the screen.
    pub pressed: bool,
}

impl GameOver {
    /// One step with fire held or not: `true` when the screen ends, which is where the original
    /// enters the high scores.
    pub fn step(&mut self, fire: bool) -> bool {
        self.steps += 1;
        if self.pressed {
            return !fire;
        }
        self.pressed = fire && self.steps >= FIRE_AFTER;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Steps until the screen ends with fire held from step `from` for `len` steps.
    fn ends(from: u32, len: u32) -> Option<u32> {
        let mut g = GameOver::default();
        (1..200).find(|&k| g.step(k >= from && k < from + len))
    }

    #[test]
    fn fire_counts_from_the_21st_step_and_its_release_ends_the_screen() {
        // Measured on the oracle (T23): one step of fire at 20 or before is ignored; at 21 the
        // high scores start at 22; two steps from 20 end at 22; thirty from 17 end at 47.
        assert_eq!(ends(20, 1), None);
        assert_eq!(ends(21, 1), Some(22));
        assert_eq!(ends(23, 1), Some(24));
        assert_eq!(ends(20, 2), Some(22));
        assert_eq!(ends(17, 30), Some(47));
    }

    #[test]
    fn the_check_runs_on_multiples_of_32_only() {
        let mut s = State::level_1();
        s.irq_count = 128;
        assert!(!game_is_over(&s));
        s.give_up = 1;
        assert!(game_is_over(&s));
        s.irq_count = 129;
        assert!(!game_is_over(&s));
        let mut s = State::level_1();
        s.irq_count = 32;
        s.lives = 0;
        assert!(game_is_over(&s));
        s.lives = 1;
        s.countdown = 0x0000_1234;
        assert!(game_is_over(&s), "only the long's first word counts");
    }
}
