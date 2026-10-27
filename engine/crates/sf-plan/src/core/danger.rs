//! Danger, judged live (DESIGN §2.5, D14): a move is played on a copy of the live state, enemies
//! included, and one that costs a heart is tried again after a short wait on its start cell.
//!
//! There is no enemy timetable: enemies spawn as their row or column scrolls in, their phase depends
//! on when the frog arrived, so only the live state knows where they are (T00 Q5). Fixed hazards are
//! `Facts::hazards` and kept off the map; this module does not look at them.

use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, State};

/// Frames idled on the start node before a retry, tried in this order (§2.5).
pub const WAITS: [u16; 3] = [12, 25, 50];

/// `frog_hearts` (h0+$830), the count the old planners read.
const HEARTS: u32 = h0(0x830);

#[derive(Clone)]
pub struct Outcome {
    /// The state after the move (after the frame the frog died on, if it died).
    pub end: Game,
    /// Hearts dropped over the move, counted frame by frame, so a heart picked up later does not
    /// hide one lost earlier.
    pub hearts_lost: u8,
    /// A life lost (fewer lives than at the start; a 1-up is not a loss) or the frog dying.
    pub died: bool,
    /// Idle frames played before the inputs; 0 = the move as given.
    pub wait: u16,
}

impl Outcome {
    /// Lost nothing: no heart, no life.
    pub fn safe(&self) -> bool {
        self.hearts_lost == 0 && !self.died
    }
}

fn lives(st: &State) -> i16 {
    st.w(FROG + f::LIVES) as i16
}

/// `inputs` after `wait` idle frames, on a clone of `start`. Stops at the frame the frog dies on.
fn play_after(start: &Game, wait: u16, inputs: &[(u8, u16)]) -> Outcome {
    let mut g = start.clone();
    let lives0 = lives(g.state());
    let mut hearts = g.state().w(HEARTS);
    let (mut hearts_lost, mut died) = (0u8, false);
    let frames = std::iter::once((0u8, wait))
        .chain(inputs.iter().copied())
        .flat_map(|(joy, n)| std::iter::repeat_n(joy, n as usize));
    for joy in frames {
        g.step_unseen(FrameInput::joy(Joy(joy)));
        let st = g.state();
        let h = st.w(HEARTS);
        hearts_lost = hearts_lost.saturating_add(hearts.saturating_sub(h).min(255) as u8);
        hearts = h;
        // As `plan_exit` reads it: lives compared signed (`$ffff` is the game over mark), so a
        // 1-up raises the count and is never read as a loss.
        if lives(st) < lives0 || st.w(FROG + f::DYING) != 0 {
            died = true;
            break;
        }
    }
    Outcome {
        end: g,
        hearts_lost,
        died,
        wait,
    }
}

/// Plays `inputs` on a clone of `start`; hearts read as the old planners read them (a 1-up is not
/// a loss).
pub fn play(start: &Game, inputs: &[(u8, u16)]) -> Outcome {
    play_after(start, 0, inputs)
}

/// `play`, and if a heart is lost, the same inputs after each of `WAITS` idle frames in order;
/// returns the first variant that loses nothing with its wait (`Some(0)` when the move itself is
/// safe), else the original outcome and `None`. A variant with a wait starts stopped (the frog
/// bleeds its speed while idle), so it is a different move from the stopped node; its landing is
/// read from `end`, never assumed to be the original edge's (DESIGN §2.5).
pub fn play_safe(start: &Game, inputs: &[(u8, u16)]) -> (Outcome, Option<u16>) {
    let first = play(start, inputs);
    if first.safe() {
        return (first, Some(0));
    }
    for w in WAITS {
        let o = play_after(start, w, inputs);
        if o.safe() {
            return (o, Some(w));
        }
    }
    (first, None)
}

/// Cost the search adds for a lost heart: larger than any detour `max_frames` allows, so a route
/// that keeps its hearts always wins over one that does not if both fit the budget.
pub fn heart_cost(max_frames: u32) -> u64 {
    max_frames as u64 + 1
}
