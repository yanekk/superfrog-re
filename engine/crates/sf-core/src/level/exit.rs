//! What follows a level's loop (`top_loop` after `level_frame_loop`, docs/08 "Scene flow"): the
//! exit fade (`level_exit_fade`, h0+$75f2: `game_over_seq` first, then the palette faded out
//! while the display window closes from the top a line a frame), `game_over_check` (h0+$749c),
//! then the level again after a death or, the exit reached, the tally (`level_tally`, T38).
//!
//! These run on the front end's program (`crate::front`): the main program waits on the beam
//! between them, and the vertical blank runs with the level's logic stopped.

use crate::front::common::{FadeKind, call, fade};
use crate::front::{Cx, Routine, Yield};
use crate::layout::h0;
use crate::level::LevelOutcome;
use crate::render::LEVEL_COPPER;
use crate::state::State;

/// `level_copper+6`: DIWSTRT's value in the level's copper list.
const DIWSTRT: u32 = LEVEL_COPPER + 6;
/// The level's palette in its copper list (`level_copper+$2d4`).
const PALETTE: u32 = LEVEL_COPPER + 0x2d4;
/// The blank copper list the screens between levels show (h2+$6fd8).
const BLANK_COPPER: u32 = crate::layout::h2(0x6fd8);
/// `attract_mode` (h0+$18), `abort_flag` (h0+$9de), `level_state` (h0+$a06).
const ATTRACT_MODE: u32 = h0(0x18);
const ABORT_FLAG: u32 = h0(0x9de);
const LEVEL_STATE: u32 = h0(0xa06);
/// `frog_lives` (h0+$82e), `level_clock` (h0+$876).
const LIVES: u32 = h0(0x82e);
const LEVEL_CLOCK: u32 = h0(0x876);
/// Where `level_exit_fade` is when its fade set-up returns (h0+$762a).
const AFTER_FADE_SETUP: u32 = h0(0x762a);
/// Lines the window closes by (`moveq #$e`).
const CLOSE_LINES: u16 = 0xe;

/// How the level's loop that has just ended ended: the last life gone (with the clock at 0:00,
/// the time up), ESC, a death with lives left, or the exit.
pub fn outcome_of_loop(s: &State) -> LevelOutcome {
    if s.l(ATTRACT_MODE) == 0 && (s.w(LIVES) as i16) < 0 {
        return if s.l(LEVEL_CLOCK) == 0 {
            LevelOutcome::TimeUp
        } else {
            LevelOutcome::GameOver
        };
    }
    if s.w(ABORT_FLAG) != 0 {
        LevelOutcome::GameOver
    } else if (s.l(LEVEL_STATE) as i32) < 0 {
        LevelOutcome::Died
    } else {
        LevelOutcome::Completed {
            tally: super::Tally::default(),
        }
    }
}

/// `level_exit_fade` and `game_over_check`, then what comes next.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct LevelOver {
    pc: u8,
    /// Lines the window has still to close.
    left: u16,
}

impl LevelOver {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    // `game_over_seq` (T34) runs first when the last life is gone.
                    if cx.s.l(ATTRACT_MODE) == 0 && (cx.s.w(LIVES) as i16) < 0 {
                        self.pc = 9;
                        return call(Routine::Outcome(outcome_of_loop(cx.s)));
                    }
                    self.pc = 1;
                    return fade(FadeKind::Out, 0, PALETTE, 0x20, AFTER_FADE_SETUP);
                }
                1 => {
                    cx.s.set_w(DIWSTRT, 0x2779);
                    self.left = CLOSE_LINES;
                    self.pc = 2;
                    return Yield::Line(0xd0);
                }
                2 => {
                    self.pc = 3;
                    return Yield::Line(0xd1);
                }
                3 => {
                    // The window's top a line lower (DIWSTRT's high byte).
                    let v = cx.s.b(DIWSTRT).wrapping_add(1);
                    cx.s.set_b(DIWSTRT, v);
                    self.left -= 1;
                    self.pc = if self.left != 0 { 2 } else { 4 };
                    if self.left != 0 {
                        return Yield::Line(0xd0);
                    }
                    return Yield::Fade(crate::front::fade::RECORDS[0]);
                }
                4 => {
                    cx.cop1lc = BLANK_COPPER;
                    self.pc = 9;
                    // `game_over_check`: ESC (`abort_flag`) leaves the game (T34), a death goes
                    // round `top_loop` again, the exit to the tally (T38).
                    return call(Routine::Outcome(outcome_of_loop(cx.s)));
                }
                _ => return Yield::Return,
            }
        }
    }
}
