//! Shell: original mode (DESIGN §2.6, D24), stretch and repair. The engine plans a whole stretch to
//! the exit from the original's own state; the original plays it in lockstep with an engine copy;
//! where they part, the plan is kept to 16 frames before the parting and planning continues from
//! the original's state there, the plan's rest as `--follow` (§2.7). The original lags in busy
//! passes and the engine does not, so only what the original played is ever kept.

use std::path::Path;
use std::time::{Duration, Instant};

use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_core::layout::h0;
use sf_core::{Game, LevelId, Start, State};
use sf_oracle::Amiga;

use crate::core::archive::HEARTS;
use crate::core::repair::{follow_options, trace_engine};
use crate::core::{Cell, Facts, Options, Reason, Route, facts, frog_cell, landed};
use crate::run::{RunOptions, plan_engine};

/// `level_state` (h0+$a06): −2 while the level is played.
const LEVEL_STATE: u32 = h0(0xa06);
const PLAYING: u32 = 0xffff_fffe;
/// Frames of an agreed plan given up before a parting (§2.6 step 3): the frames just before it
/// already carry what made the two machines differ.
pub const BACK_OFF: usize = 16;
/// Repairs running parting at one cell after which the original's way there is accepted (§2.6
/// step 4).
pub const ACCEPT_AFTER: u32 = 3;
/// Frames the original may run on past the plan's end, the last input held, to reach the exit it
/// lags behind the engine to.
const LAG_TAIL: usize = 50;

/// The oracle at the planner's start, and what it took to get there.
pub struct OracleRun {
    pub amiga: Amiga,
    /// Inputs the original already played from the start the caller named (the level-start
    /// freeze): the route returned begins with them.
    pub inputs: Vec<FrameInput>,
    pub oracle_frames: u64,
    /// Wall-clock spent on the original so far; the start's time is all of it.
    pub confirm_time: Duration,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub engine_frames: u64,
    pub oracle_frames: u64,
    /// Wall-clock on the original over the whole run's wall-clock, the start's included.
    pub confirm_share: f32,
    pub repairs: u32,
    /// The last frame the original agreed to, from the run's start (DESIGN §2.9's reason).
    pub confirmed_frames: u32,
}

/// One machine's frog after a frame, as the parting rule reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Look {
    pub cell: Cell,
    pub lives: u16,
    pub hearts: u16,
    pub dying: bool,
    pub playing: bool,
}

impl Look {
    pub fn of(st: &State) -> Look {
        Look {
            cell: frog_cell(st),
            lives: st.w(FROG + f::LIVES),
            hearts: st.w(HEARTS),
            dying: st.w(FROG + f::DYING) != 0,
            playing: st.l(LEVEL_STATE) == PLAYING,
        }
    }
}

/// Why the original and the engine parted (§2.6 step 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parting {
    /// Frog cells more than one apart.
    Apart,
    /// The original lost a life (a 1-up gained is not a loss).
    LifeLost,
    Dying,
    /// `sync::in_wall_loop`: the original hung in `frog_tiles`.
    WallLoop,
    /// The original has fewer hearts than the engine.
    Heart,
    /// The oracle refused the frame (its cap, an emulator error; §2.9).
    Oracle(String),
}

impl Parting {
    /// Partings the original can be let have its way at (§2.6 step 4: "a heart lost"); a death or
    /// a hang never makes a route, so those are repaired until the budget ends.
    pub fn acceptable(&self) -> bool {
        matches!(self, Parting::Apart | Parting::Heart)
    }
}

/// The parting rule on one frame: the original's look before and after it, the engine's after it,
/// and whether the original ended the frame in the wall loop.
pub fn parts(before: &Look, orig: &Look, eng: &Look, wall_loop: bool) -> Option<Parting> {
    if wall_loop {
        Some(Parting::WallLoop)
    } else if orig.dying {
        Some(Parting::Dying)
    } else if orig.lives < before.lives {
        Some(Parting::LifeLost)
    } else if orig.hearts < eng.hearts {
        Some(Parting::Heart)
    } else if orig.cell.0.abs_diff(eng.cell.0) > 1 || orig.cell.1.abs_diff(eng.cell.1) > 1 {
        Some(Parting::Apart)
    } else {
        None
    }
}

/// Partings at one cell, repairs running (§2.6 step 4).
#[derive(Debug, Clone, Default)]
pub struct Repeats {
    last: Option<Cell>,
    count: u32,
}

impl Repeats {
    /// Takes a parting at the original's `cell`: true if it is accepted (the third running there,
    /// of a kind that may be), false if it is repaired. An accepted parting starts the count over.
    pub fn accept(&mut self, cell: Cell, why: &Parting) -> bool {
        if self.last == Some(cell) {
            self.count += 1;
        } else {
            (self.last, self.count) = (Some(cell), 1);
        }
        let yes = self.count >= ACCEPT_AFTER && why.acceptable();
        if yes {
            (self.last, self.count) = (None, 0);
        }
        yes
    }
}

/// The original's frog looked at now (a ~1 MB copy of its memory).
fn look(amiga: &Amiga) -> Look {
    Look::of(&State::transplant(&sync::core_memory(amiga.memory())))
}

/// From the first frame a start's level is played on (a sync point's or a leg snapshot's first
/// frame has `level_state` ≠ −2, FINDINGS T09), idle until the frog has landed past frame 150, as
/// the engine-side starts do.
pub fn settle(mut amiga: Amiga, t0: Instant) -> Result<OracleRun, String> {
    let mut inputs = Vec::new();
    loop {
        let st = State::transplant(&sync::core_memory(amiga.memory()));
        if (landed(&st) && inputs.len() > 150) || inputs.len() >= 400 {
            break;
        }
        sync::step_inserting(&mut amiga, FrameInput::NONE)?;
        inputs.push(FrameInput::NONE);
    }
    Ok(OracleRun {
        amiga,
        oracle_frames: inputs.len() as u64,
        inputs,
        confirm_time: t0.elapsed(),
    })
}

/// The oracle at `id`'s sync point (`sync::reach(AtLevel)`), with room for `cap` more frames, past
/// the level-start freeze. Panics if the oracle cannot be started, as the conformance cases do.
pub fn start_at_level(id: LevelId, cap: u32) -> OracleRun {
    let t0 = Instant::now();
    let a = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), cap)
        .unwrap_or_else(|e| panic!("{id}: the oracle cannot reach the level: {e}"))
        .amiga;
    settle(a, t0).unwrap_or_else(|e| panic!("{id}: {e}"))
}

/// The oracle at a `whole_game` leg snapshot (`sync::at_snapshot_file`), with room for `cap` more
/// frames, past the level-start freeze.
pub fn start_at_snap(path: &Path, cap: u32) -> OracleRun {
    let t0 = Instant::now();
    let a = sync::at_snapshot_file(path, cap).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    settle(a, t0).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The level's facts as the original has them at the run's start.
pub fn facts_at(run: &OracleRun, id: LevelId) -> Facts {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let g = Game::with_display(
        display.clone(),
        Start::Transplant(sync::core_memory(run.amiga.memory())),
    );
    facts(&g, &data.levels[&id], display.rom())
}

/// What one stretch's lockstep found.
#[derive(Debug, Clone, PartialEq)]
pub struct Lockstep {
    /// Plan frames (and lag-tail frames past them) the original agreed to.
    pub agreed: usize,
    /// The original left the level alive after `agreed` frames.
    pub exited: bool,
    /// Why it stopped short, and the original's cell there.
    pub parting: Option<(Parting, Cell)>,
    /// The inputs played past the plan's end, the last held, before the exit.
    pub tail: Vec<FrameInput>,
    /// Hearts the original lost over the frames agreed to (a heart gained is not netted).
    pub hearts_lost: u16,
}

/// Plays `plan` on the original and on `eng` (the original's state transplanted) side by side
/// until they part or the original exits, counting the original's frames into `frames`.
pub fn lockstep(
    amiga: &mut Amiga,
    mut eng: Game,
    plan: &[FrameInput],
    frames: &mut u64,
) -> Lockstep {
    let mut before = look(amiga);
    let mut out = Lockstep {
        agreed: 0,
        exited: false,
        parting: None,
        tail: Vec::new(),
        hearts_lost: 0,
    };
    let last = plan.last().copied().unwrap_or(FrameInput::NONE);
    for k in 0..plan.len() + LAG_TAIL {
        let x = plan.get(k).copied().unwrap_or(last);
        let past = k >= plan.len();
        if let Err(e) = sync::step_inserting(amiga, x) {
            out.parting = Some((Parting::Oracle(e), before.cell));
            return out;
        }
        *frames += 1;
        let o = look(amiga);
        // Past the plan's end the engine has left the level: only the original is judged.
        let e = if past {
            o
        } else {
            eng.step_unseen(x);
            Look::of(eng.state())
        };
        if let Some(why) = parts(&before, &o, &e, sync::in_wall_loop(amiga)) {
            out.parting = Some((why, o.cell));
            return out;
        }
        if past {
            out.tail.push(x);
        }
        out.hearts_lost += before.hearts.saturating_sub(o.hearts);
        out.agreed = k + 1;
        // Only an end of play counts: a start's first frames (`level_state` not yet −2) are not
        // an exit.
        if before.playing && !o.playing {
            out.exited = true;
            return out;
        }
        before = o;
    }
    // The original played the whole plan and the lag tail with no exit: it parts at the end.
    out.parting = Some((Parting::Apart, before.cell));
    out
}

/// Plans `run`'s level against the original (§2.6): the route from the run's start (its `inputs`
/// first) to the frame the original leaves the level, or the best partial route and why, its
/// confirmed part `Stats::confirmed_frames` long. `ro.budget` covers the whole run.
pub fn plan_original(
    run: OracleRun,
    facts: Facts,
    opt: Options,
    ro: &RunOptions,
) -> Result<(Route, Stats), (Route, Stats, Reason)> {
    let t0 = Instant::now();
    let display = sf_conform::display_data();
    let OracleRun {
        mut amiga,
        inputs: mut route,
        oracle_frames,
        confirm_time: start_time,
    } = run;
    let mut confirm = Duration::ZERO;
    let mut stats = Stats {
        oracle_frames,
        ..Stats::default()
    };
    // Hearts lost on the original over the frames kept, counted as `verify` counts them.
    let mut lost = 0u16;
    let hearts = |a: &Amiga| {
        let b = a.peek(HEARTS, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    let mut repeats = Repeats::default();
    let mut rest: Option<Vec<FrameInput>> = None;
    let mut rounds = 0u64;
    let finish = |stats: &mut Stats, confirm: Duration, route: &[FrameInput]| {
        let all = start_time + t0.elapsed();
        stats.confirm_share = ((start_time + confirm).as_secs_f64() / all.as_secs_f64().max(1e-9))
            .clamp(0.0, 1.0) as f32;
        stats.confirmed_frames = route.len() as u32;
    };
    loop {
        let start = Game::with_display(
            display.clone(),
            Start::Transplant(sync::core_memory(amiga.memory())),
        );
        let left = ro.budget.saturating_sub(t0.elapsed());
        let mut o = Options {
            seed: opt.seed.wrapping_add(stats.repairs as u64),
            max_frames: opt.max_frames.saturating_sub(route.len() as u32),
            ..opt.clone()
        };
        let rest_len = rest.as_ref().map(Vec::len);
        if let Some(rest) = rest.take().filter(|_| !left.is_zero()) {
            // The rest of the old plan, traced on the engine from the original's state here, guides
            // and seeds; an explicit guide still steers (§2.7).
            let trace = trace_engine(&start, &facts, &rest);
            o = follow_options(o, rest, &trace);
            if !opt.guide.is_empty() {
                o.guide = opt.guide.clone();
                o.guide_pace = opt.guide_pace;
            }
        }
        // A repair stretch plans to the exit too, but gets the shortening minimum only if its
        // exit is longer than the old plan's rest or loses a heart: otherwise its route is mostly
        // the last plan's, already shortened (FINDINGS, T10, T14).
        let stretch_ro = RunOptions {
            budget: left,
            shorten_min_over: rest_len,
            ..ro.clone()
        };
        let plan = plan_engine(start.clone(), facts.clone(), o, &stretch_ro);
        let plan = match plan {
            Ok(r) => {
                stats.engine_frames += r.engine_frames;
                rounds += r.rounds;
                r
            }
            Err((partial, reason)) => {
                stats.engine_frames += partial.engine_frames;
                rounds += partial.rounds;
                finish(&mut stats, confirm, &route);
                let confirmed = route.len();
                route.extend(partial.inputs.iter().copied());
                let r = Route {
                    inputs: route,
                    engine_frames: stats.engine_frames,
                    hearts_lost: lost
                        .saturating_add(partial.hearts_lost as u16)
                        .min(u8::MAX as u16) as u8,
                    rounds,
                    cells: partial.cells,
                };
                stats.confirmed_frames = confirmed as u32;
                return Err((r, stats, reason));
            }
        };

        let c0 = Instant::now();
        let snap = amiga.snapshot();
        let ls = lockstep(&mut amiga, start, &plan.inputs, &mut stats.oracle_frames);
        if ls.exited {
            route.extend_from_slice(&plan.inputs[..ls.agreed.min(plan.inputs.len())]);
            route.extend_from_slice(&ls.tail);
            confirm += c0.elapsed();
            finish(&mut stats, confirm, &route);
            let r = Route {
                inputs: route,
                engine_frames: stats.engine_frames,
                hearts_lost: lost.saturating_add(ls.hearts_lost).min(u8::MAX as u16) as u8,
                rounds,
                cells: plan.cells,
            };
            return Ok((r, stats));
        }
        let (why, cell) = ls.parting.expect("a lockstep ends in an exit or a parting");
        if std::env::var_os("SF_PLAN_LOG").is_some() {
            eprintln!(
                "  {:6.1}s stretch {}: plan {} frames ({} rounds), agreed {}, {why:?} at {cell:?}, route {}",
                t0.elapsed().as_secs_f64(),
                stats.repairs,
                plan.inputs.len(),
                plan.rounds,
                ls.agreed,
                route.len()
            );
        }
        let agreed = ls.agreed.min(plan.inputs.len());
        // Accepted: the original's frame of the parting is kept too, and planning goes on from
        // the state it insists on. Repaired: back off before the parting.
        let keep = if repeats.accept(cell, &why) {
            (agreed + 1).min(plan.inputs.len())
        } else {
            agreed.saturating_sub(BACK_OFF)
        };
        amiga.restore(&snap);
        let mut h = hearts(&amiga);
        for &x in &plan.inputs[..keep] {
            // These frames were just played from the same snapshot: they replay the same.
            sync::step_inserting(&mut amiga, x).expect("the kept frames replay");
            stats.oracle_frames += 1;
            let now = hearts(&amiga);
            lost += h.saturating_sub(now);
            h = now;
        }
        confirm += c0.elapsed();
        route.extend_from_slice(&plan.inputs[..keep]);
        rest = Some(plan.inputs[keep..].to_vec());
        stats.repairs += 1;
        stats.confirmed_frames = route.len() as u32;
    }
}
