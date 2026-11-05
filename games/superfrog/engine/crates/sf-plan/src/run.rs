//! Shell side of the explorer (DESIGN §2.4, §3.1), now the toolkit's (`amiga_plan::run`): a
//! round's parts on threads, the wall-clock budget, shortening and peak memory. What stays here is
//! Superfrog's run options (the original's lag bar beside the toolkit's) and wrappers that take a
//! `Game` and Superfrog's `Facts`, so callers did not change.

use std::time::Instant;

pub use amiga_plan::run::{
    COMPASS_BUILD_NS, COMPASS_FILL_BYTES, COMPASS_FILLS, PassResult, over_slack, peak_rss_bytes,
};
use sf_core::Game;
use sf_core::input::FrameInput;

use crate::core::{Explorer, Facts, Frog, Options, Progress, Reason, Route};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct RunOptions {
    pub threads: usize,
    /// Wall-clock: the search stops between rounds once it is spent; a stopped run is not held to
    /// M8.
    pub budget: Duration,
    /// Shortening after the first exit (DESIGN §2.12, T22); the default keeps it off.
    pub shorten_share: f32,
    pub shorten_min_rounds: u64,
    /// When set, `(frames, hearts)`: `shorten_min_rounds` applies only to a first exit more than
    /// `MIN_OVER_SLACK` longer than `frames` or losing more than `hearts` hearts; otherwise the
    /// share alone. A repair stretch
    /// passes the frames its old plan still had and the hearts that plan lost: its route is mostly
    /// that plan's, already shortened, unless its search went another way (W1-2's repair took a
    /// 7 200-frame, 3-heart exit for a 4 718-frame, no-heart plan, T14).
    pub shorten_min_over: Option<(usize, u8)>,
    /// Original mode's lag bar (`oracle::Lag`): the most frames in a row the original may end
    /// mid-pass, as the faithful check of the level's completion case allows (`sf-plan` looks it
    /// up, `lag_limit_for`).
    pub lag_limit: u32,
    /// When set, the search gives up at this many rounds with no goal found, as at the budget's
    /// end (original mode's reach stretches, T24).
    pub max_rounds: Option<u64>,
    /// Original mode's stretches (T33): the search gives up, as at the budget's end, once it is a
    /// dead end (`oracle::dead_end`: `DEAD_ROUNDS` rounds with at most `DEAD_CELLS` cells), so
    /// the repair can rewind instead of spending the budget there.
    pub dead_end: bool,
}

impl Default for RunOptions {
    fn default() -> RunOptions {
        RunOptions {
            threads: 1,
            budget: Duration::from_secs(30 * 60),
            shorten_share: 0.0,
            shorten_min_rounds: 0,
            shorten_min_over: None,
            lag_limit: sf_conform::LAG_LIMIT,
            max_rounds: None,
            dead_end: false,
        }
    }
}

/// Shortening's defaults (DESIGN §2.12): 20% of the rounds to the first exit, at least
/// `SHORTEN_MIN_ROUNDS`, about 60 s of rounds on this machine (10 threads, 80 bursts a round;
/// measured in T22, FINDINGS). T14 tunes both.
pub const SHORTEN_SHARE: f32 = 0.2;
pub const SHORTEN_MIN_ROUNDS: u64 = 180;

impl RunOptions {
    /// The defaults with shortening on (`sf-plan`'s, T11).
    pub fn shortening() -> RunOptions {
        RunOptions {
            shorten_share: SHORTEN_SHARE,
            shorten_min_rounds: SHORTEN_MIN_ROUNDS,
            ..RunOptions::default()
        }
    }
}

impl RunOptions {
    /// The toolkit's run options: these less the lag bar (the oracle's alone), logging when
    /// `SF_PLAN_LOG` is set.
    pub fn plan(&self) -> amiga_plan::run::RunOptions {
        amiga_plan::run::RunOptions {
            threads: self.threads,
            budget: self.budget,
            shorten_share: self.shorten_share,
            shorten_min_rounds: self.shorten_min_rounds,
            shorten_min_over: self.shorten_min_over,
            max_rounds: self.max_rounds,
            dead_end: self.dead_end,
            log: std::env::var_os("SF_PLAN_LOG").is_some(),
        }
    }
}

/// Plays one round on `threads` threads and merges it (`amiga_plan::run::run_round`).
pub fn run_round(ex: &mut Explorer, threads: usize) -> Progress {
    amiga_plan::run::run_round(&mut ex.0, threads)
}

/// The explorer from `start` to the goal within `run`'s budget (`amiga_plan::run::plan_engine`).
pub fn plan_engine(
    start: Game,
    facts: Facts,
    opt: Options,
    run: &RunOptions,
) -> Result<Route, (Route, Reason)> {
    let plan = facts.plan();
    amiga_plan::run::plan_engine(Frog::new(start, &facts), plan, opt, &run.plan())
}

/// The shortcut pass (DESIGN §2.12) over `route` (`amiga_plan::run::shortcut_pass`).
pub fn shortcut_pass(
    start: &Game,
    facts: &Facts,
    opt: &Options,
    route: Vec<FrameInput>,
    rounds: u64,
    threads: usize,
    deadline: Instant,
) -> PassResult {
    let g = Frog::new(start.clone(), facts);
    amiga_plan::run::shortcut_pass(&g, &facts.plan(), opt, route, rounds, threads, deadline)
}
