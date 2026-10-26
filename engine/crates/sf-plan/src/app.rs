//! Shell: app mode (DESIGN §2.1 `--app`, §2.6, set C). The level as Game ▸ Start Level… starts it
//! under a script: the dialog's defaults, the script's power-on seed. Planned and confirmed on the
//! engine alone; there is no original, because the app never lags as the original does and a route
//! paced to the original's lag dies in the app (two timings, never crossed: §2.6).

use sf_conform::InputScript;
use sf_core::input::FrameInput;
use sf_core::recording::LevelSetup;
use sf_core::{Game, LevelId, Rng};

use crate::core::repair::{TracePoint, trace_engine};
use crate::core::{Facts, landed};
use crate::oracle::Look;

/// The level's first frame, as the app's `start-level <level>` script line makes it: the dialog's
/// defaults and `Rng::seeded(seed, seed)`. The app seeds Start Level from its power-on seed, which a
/// script sets with `seed A B` and leaves at `0 0` without one (sf-app `menu.rs` `start_level`,
/// `script.rs`); `sf-plan --app --seed N` is that seed with both halves N.
pub fn app_start(level: LevelId, seed: u32) -> Game {
    Game::start_level(
        sf_conform::display_data(),
        &LevelSetup {
            level,
            ..Default::default()
        },
        Rng::seeded(seed, seed),
    )
}

/// Idle from the level's first frame until the frog has landed past frame 150, at most 400, as
/// the original's start is settled (`oracle::settle`): the route begins with these frames.
pub fn settle(game: &mut Game) -> Vec<FrameInput> {
    let mut inputs = Vec::new();
    while !(landed(game.state()) && inputs.len() > 150) && inputs.len() < 400 {
        game.step_unseen(FrameInput::NONE);
        inputs.push(FrameInput::NONE);
    }
    inputs
}

/// The level's `level_state` is not −2 on its first frames (`sf-bench`'s verify waits 10 too).
const SETTLE: usize = 10;

/// A route played on the engine from [`app_start`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    /// Frames to the end of the one `level_state` first left −2 on; `None` if it never did.
    pub exit: Option<u32>,
    pub lives_lost: u32,
    pub hearts_lost: u32,
}

impl Replay {
    /// DESIGN §4.1 set C, the engine's half: the exit with no life lost.
    pub fn completed(&self) -> bool {
        self.exit.is_some() && self.lives_lost == 0
    }
}

/// `inputs` from `level`'s app start with `seed`, until the level ends or the inputs do.
pub fn replay(level: LevelId, seed: u32, inputs: &[FrameInput]) -> Replay {
    let mut g = app_start(level, seed);
    let mut before = Look::of(g.state());
    let mut r = Replay {
        exit: None,
        lives_lost: 0,
        hearts_lost: 0,
    };
    for (k, &x) in inputs.iter().enumerate() {
        g.step_unseen(x);
        let now = Look::of(g.state());
        r.lives_lost += before.lives.saturating_sub(now.lives) as u32;
        r.hearts_lost += before.hearts.saturating_sub(now.hearts) as u32;
        before = now;
        if k >= SETTLE && !now.playing {
            r.exit = Some(k as u32 + 1);
            break;
        }
    }
    r
}

/// A `# timing: app` script played on the engine from its own app start (DESIGN §2.11), sampled
/// for guide points: T09 left this branch of `trace.rs` to T12.
pub fn trace_app(level: LevelId, seed: u32, facts: &Facts, script: &InputScript) -> Vec<TracePoint> {
    let n = script.lines.last().map_or(0, |l| l.0);
    let inputs: Vec<FrameInput> = (0..n).map(|k| script.input_at(k)).collect();
    trace_engine(&app_start(level, seed), facts, &inputs)
}

/// The seed an app-timed route was planned with: its `# seed: N` line (`sf-plan --app` writes one
/// after the timing line). An app route only replays from the start its seed makes.
pub fn route_seed(text: &str) -> Option<u32> {
    text.lines()
        .take_while(|l| l.trim_start().starts_with('#'))
        .find_map(|l| l.trim().strip_prefix("# seed:"))
        .and_then(|s| s.trim().parse().ok())
}
