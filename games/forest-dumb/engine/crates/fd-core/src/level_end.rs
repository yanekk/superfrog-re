//! The level's end (plans/forest-dumb-level-1 T26, DESIGN §2.5): `main_loop`'s check that finds it,
//! `next_world` (`h0+$1ba`) as far as the comparison follows it, and the "Level 1 complete" message
//! that stands in for world 2 until fire.
//!
//! `main_loop` (`h0+$252`), whenever `irq_count` is a multiple of 32, tests `give_up`, `lives` 0 and
//! `countdown` 0 first (the game over, `crate::game_over`), then `level_exit_touched` (`h0+$286`):
//! set with `items_left` 0, it branches to `next_world` before its status bar redraw (`bsr h0_6a96`).
//! `next_world` adds one to `world_index`, then `h0+$500` clears the 32 colours of the level copper
//! list (`h1+$13e`), and `level_init` loads world 2 from disk with the machine handed back to the OS
//! (`load_world`, `system_on`); its SCENA screen waits from about 1 000 frames later (measured, T00:
//! docs/09-play-measured.md "Level end"). No other end-of-level sequence runs between.
//!
//! **What is compared ends at `next_world`**: world 2's load and screen are not built, so the
//! conformance runner stops at the step the original runs `next_world` in, holding `world_index`
//! (`fd-conform`'s `AFTER_NEXT_WORLD`). The engine's [`LevelEnd`] is that step, its colours black as
//! `h0+$500` leaves them; the step after shows [`crate::Scene::Complete`].
//!
//! `level_exit_touched` is set by `char_object_cells` on an `$ff` cell (T22's) and by `help_key` with
//! `cheat_flag` on (`crate::character::death`); `items_left` falls as items are taken (T22).

use crate::State;
use crate::game_over;

/// `main_loop`'s level-end test (`h0+$286`..`$298`), when the foreground reaches its check after a
/// pass: the exit touched with no items left. It runs only while `irq_count` is a multiple of 32,
/// and only when the game is not over, which `main_loop` tests first.
pub fn level_is_over(s: &State) -> bool {
    s.irq_count & 0x1f == 0
        && !game_over::game_is_over(s)
        && s.level_exit_touched != 0
        && s.items_left == 0
}

/// `next_world`'s `addi.l #1, world_index`. World 6 done would go to `end_screen`; level 1 never
/// gets there.
pub fn next_world(s: &mut State) {
    s.world_index = s.world_index.wrapping_add(1);
}

/// The step `main_loop` found the level's end in: `next_world` has run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LevelEnd {
    /// The world `next_world` moved to (`world_index`): 1 after W1-1.
    pub world: u32,
}

/// The "Level 1 complete" message (DESIGN §2.5): not the original's. It stands where the original
/// loads world 2, and goes when world 2 is built.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Complete {
    /// Fire has been up since the message showed. Fire held through the level's end (play never
    /// reads it) must not skip the message: only a press after that counts.
    pub ready: bool,
}

impl Complete {
    /// One step with fire held or not: `true` when fire is pressed, which restarts the level.
    /// Every other input is ignored.
    pub fn step(&mut self, fire: bool) -> bool {
        if fire && self.ready {
            return true;
        }
        self.ready |= !fire;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_check_runs_on_multiples_of_32_after_the_game_over_tests() {
        let mut s = State::level_1();
        s.irq_count = 64;
        s.level_exit_touched = 1;
        assert!(!level_is_over(&s), "items left");
        s.items_left = 0;
        assert!(level_is_over(&s));
        s.irq_count = 65;
        assert!(!level_is_over(&s));
        s.irq_count = 64;
        s.give_up = 1;
        assert!(!level_is_over(&s), "the game over comes first");
        s.give_up = 0;
        s.level_exit_touched = 0;
        assert!(!level_is_over(&s), "no exit touched");
    }

    #[test]
    fn next_world_moves_on_one_world() {
        let mut s = State::level_1();
        next_world(&mut s);
        assert_eq!(s.world_index, 1);
    }

    #[test]
    fn the_message_takes_a_fire_press_made_after_it_showed() {
        let mut c = Complete::default();
        assert!(!c.step(true), "held through the end: ignored");
        assert!(!c.step(true));
        assert!(!c.step(false));
        assert!(c.step(true));
        let mut c = Complete::default();
        assert!(!c.step(false));
        assert!(c.step(true), "pressed in the first step fire was up before");
    }
}
