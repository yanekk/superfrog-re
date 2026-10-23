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
use crate::core::repair::{
    Sampler, TRACE_EVERY, TracePoint, follow_options, points_from_trace,
};
use crate::core::{
    Cell, CellKey, Facts, Goal, GuidePoint, Options, Reason, Route, cell_key, facts, frog_cell,
    landed,
};
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
    /// The original ended more frames in a row mid-pass than the faithful check allows (the
    /// level's completion case's `lag-limit`, `LAG_LIMIT` without one): the check would compare
    /// nothing there and fail the route (T14: W5-3, 101 from frame 5 310).
    Lag,
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

/// The frog's block of state, as the lockstep's hold compares it.
const FROG_BLOCK: u32 = 0x100;

fn frog_block(st: &State) -> Vec<u8> {
    (0..FROG_BLOCK).map(|k| st.byte(FROG + k).unwrap_or(0)).collect()
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

/// The faithful check's lag bar over a lockstep: its limit, and the frames the original has ended
/// mid-pass in a row so far (`sf_conform`'s `LagRun`, check 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lag {
    pub limit: u32,
    pub run: u32,
}

/// What one stretch's lockstep found.
#[derive(Debug, Clone, PartialEq)]
pub struct Lockstep {
    /// Original frames the original agreed to (the plan's frames, the frames held through its lag
    /// and the lag-tail frames past the plan's end).
    pub agreed: usize,
    /// The original left the level alive after `agreed` frames.
    pub exited: bool,
    /// Why it stopped short, and the original's cell there.
    pub parting: Option<(Parting, Cell)>,
    /// The input played on each original frame, the parting frame included: the plan's, its next
    /// frame held while the original lags, its last held past its end.
    pub fed: Vec<FrameInput>,
    /// The plan frames used after each original frame of `fed`.
    pub used: Vec<usize>,
    /// The original's frames ended mid-pass in a row after each frame of `fed`.
    pub mid_run: Vec<u32>,
    /// Hearts the original lost over the frames agreed to (a heart gained is not netted).
    pub hearts_lost: u16,
}

impl Lockstep {
    /// The plan frames used once the first `k` original frames are played.
    pub fn used_at(&self, k: usize) -> usize {
        k.checked_sub(1).map_or(0, |j| self.used[j])
    }

    /// The mid-pass run once the first `k` original frames are played, `start` before any.
    pub fn mid_run_at(&self, k: usize, start: u32) -> u32 {
        k.checked_sub(1).map_or(start, |j| self.mid_run[j])
    }
}

/// Plays `plan` on the original and on `eng` (the original's state transplanted) side by side
/// until they part or the original exits, counting the original's frames into `frames`.
///
/// The original lags in busy passes and the engine never does (docs/08 "Lag"). Most late passes
/// still move the frog, but some freeze it: W1-2's scroller held the original's frog block
/// unchanged through nine frames at x 1625 while the engine ran on, and without a hold the two
/// parted there on every repair, 18 repairs in 25 minutes (T14). A frame on which the original's
/// frog block (`FROG`, 256 bytes) does not change while the engine's would is held: the engine is
/// not stepped and the plan's next frame is played again. Holding on every mid-pass frame instead
/// parted engine-planned W1-2, W2-1 and W1-3 routes within 10 frames of their start.
pub fn lockstep(
    amiga: &mut Amiga,
    eng: Game,
    plan: &[FrameInput],
    frames: &mut u64,
    lag: Lag,
) -> Lockstep {
    lockstep_with(amiga, eng, plan, frames, lag, LAG_TAIL)
}

/// `lockstep` with `tail` frames past the plan's end, the last input held. With no tail (a reach
/// stretch's plan, which ends on the old plan's path, not at the exit) a plan the original played
/// to its end without a parting ends with `parting` `None`, every frame agreed (T24).
pub fn lockstep_with(
    amiga: &mut Amiga,
    mut eng: Game,
    plan: &[FrameInput],
    frames: &mut u64,
    lag: Lag,
    tail: usize,
) -> Lockstep {
    let mut before = look(amiga);
    let mut out = Lockstep {
        agreed: 0,
        exited: false,
        parting: None,
        fed: Vec::new(),
        used: Vec::new(),
        mid_run: Vec::new(),
        hearts_lost: 0,
    };
    let mut run = lag.run;
    let last = plan.last().copied().unwrap_or(FrameInput::NONE);
    let mut e = before;
    let mut i = 0usize;
    // A lag bounded by the oracle's own frame cap; this bounds a pass that never ends.
    let most = 2 * (plan.len() + tail);
    while i < plan.len() + tail && out.fed.len() < most {
        let x = plan.get(i).copied().unwrap_or(last);
        let past = i >= plan.len();
        out.fed.push(x);
        let frog_before = amiga.peek(FROG, FROG_BLOCK as usize);
        if let Err(err) = sync::step_inserting(amiga, x) {
            out.used.push(i);
            out.mid_run.push(run);
            out.parting = Some((Parting::Oracle(err), before.cell));
            return out;
        }
        *frames += 1;
        let mem = sync::core_memory(amiga.memory());
        run = if sync::mid_pass(&amiga.cpu(), &mem) { run + 1 } else { 0 };
        out.mid_run.push(run);
        let o = Look::of(&State::transplant(&mem));
        if past {
            // Past the plan's end the engine has left the level: only the original is judged.
            e = o;
            i += 1;
        } else {
            let mut next = eng.clone();
            next.step_unseen(x);
            let frozen = amiga.peek(FROG, FROG_BLOCK as usize) == frog_before
                && frog_block(next.state()) != frog_block(eng.state());
            if !frozen {
                eng = next;
                e = Look::of(eng.state());
                i += 1;
            }
        }
        out.used.push(i);
        if run > lag.limit {
            out.parting = Some((Parting::Lag, o.cell));
            return out;
        }
        if let Some(why) = parts(&before, &o, &e, sync::in_wall_loop(amiga)) {
            out.parting = Some((why, o.cell));
            return out;
        }
        out.hearts_lost += before.hearts.saturating_sub(o.hearts);
        out.agreed = out.fed.len();
        // Only an end of play counts: a start's first frames (`level_state` not yet −2) are not
        // an exit.
        if before.playing && !o.playing {
            out.exited = true;
            return out;
        }
        before = o;
    }
    // The original played the whole plan and the lag tail with no exit: it parts at the end. A
    // plan with no tail that was played to its end has reached its goal.
    if tail > 0 || i < plan.len() {
        out.parting = Some((Parting::Apart, before.cell));
    }
    out
}

/// The faithful check's lag bar for a route from `level`'s start (T14, owner 2026-10-23; a deviation
/// from DESIGN §2.6): the `lag-limit` of the case `sf-bench`'s §4.1 check takes it from, the
/// level's completion case (or `game-whole`'s for a leg snapshot), else `sf_conform::LAG_LIMIT`.
/// The names follow `sf-bench`'s `verify::committed_script`.
pub fn lag_limit_for(level: LevelId, snap: bool) -> u32 {
    let l = level.to_string().to_lowercase();
    let name = if snap {
        "game-whole".to_string()
    } else {
        match l.as_bytes()[1] {
            b'1' | b'4' => format!("complete-{l}"),
            b'2' => format!("{l}-complete"),
            _ => format!("enemies-{l}-complete"),
        }
    };
    sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .ok()
        .and_then(|c| c.lag_limit)
        .map_or(sf_conform::LAG_LIMIT, |l| l.frames)
}

/// What follows a parting (§2.6 steps 3, 4).
#[derive(Debug, Clone, PartialEq)]
pub enum Next {
    /// Accepted: the plan's frames from `used` on, played on from the original's state (no
    /// planning; the route carries no engine frames or rounds of its own).
    Resume(Route),
    /// Repaired: the plan's rest from `used`, the next stretch's guide and seed.
    Repair(Vec<FrameInput>),
}

/// What a parting in `ls` keeps of a `plan_len`-frame plan (§2.6 steps 3, 4): the original frames
/// of `ls.fed` kept, the plan frames they used, and the mid-pass run at their end (`start` before
/// the stretch). Accepted, the parting frame is kept too; repaired, `BACK_OFF` frames before it,
/// counted within the plan (not its lag tail). Held frames use no plan frame (T14).
pub fn kept(ls: &Lockstep, plan_len: usize, accepted: bool, start: u32) -> (usize, usize, u32) {
    let agreed = (0..=ls.agreed)
        .rev()
        .find(|&k| ls.used_at(k) <= plan_len)
        .unwrap_or(0);
    let keep = if accepted {
        (agreed + 1).min(ls.fed.len())
    } else {
        agreed.saturating_sub(BACK_OFF)
    };
    (keep, ls.used_at(keep).min(plan_len), ls.mid_run_at(keep, start))
}

/// After a parting with `used` of `plan`'s frames kept: an accepted parting plays the same plan
/// on, unless nothing of it is left.
pub fn next_stretch(accepted: bool, plan: &Route, used: usize) -> Next {
    let rest = plan.inputs[used.min(plan.inputs.len())..].to_vec();
    if accepted && !rest.is_empty() {
        Next::Resume(Route {
            inputs: rest,
            engine_frames: 0,
            rounds: 0,
            ..plan.clone()
        })
    } else {
        Next::Repair(rest)
    }
}

/// Frames of the last whole plan a repair stretch plans ahead of the original (T24): a repair
/// plans back onto that plan's path this far on, not to the exit. Repairs to the exit re-planned
/// the whole level's rest each time: W4-4 parted 1 863 frames into a 10 710-frame plan and its
/// repair found no exit in 24 minutes; W1-4's ran 2 383 rounds and stalled 15 cells short.
pub const HORIZON: usize = 1024;
/// The reference trace's points in `HORIZON` frames.
pub const HORIZON_POINTS: usize = HORIZON / TRACE_EVERY as usize;
/// Rounds a reach stretch may take before the repair plans to the exit instead.
pub const REACH_ROUNDS: u64 = 300;
/// A reach stretch's frames, at most, from its start to the point it reaches.
pub const REACH_FRAMES: u32 = 3 * HORIZON as u32;

/// The point of a `points`-point reference a repair stretch from point `p` reaches for, or `None`
/// when the reference's end is near enough to plan to the exit (less than 1½ horizons left).
pub fn reach_target(points: usize, p: usize) -> Option<usize> {
    let t = p + HORIZON_POINTS;
    (t + HORIZON_POINTS / 2 < points).then_some(t)
}

/// The last whole plan (a plan to the exit) that parted, which repair stretches plan back onto:
/// its inputs and its trace on the engine from its own start, where it reached the exit (T24).
/// The plan's rest traced from the original's state instead dies where the replay does (W1-4
/// ~2 000 frames on), and its trace past a death steers nowhere.
#[derive(Debug, Clone)]
pub struct Reference {
    pub inputs: Vec<FrameInput>,
    pub points: Vec<GuidePoint>,
    pub trace: Vec<TracePoint>,
    /// The hearts the plan had lost on the engine at each trace point.
    pub lost: Vec<u16>,
    /// The point the original is known to have passed.
    pub at: usize,
    /// The hearts the plan lost on the engine.
    pub hearts_lost: u8,
}

impl Reference {
    pub fn new(start: &Game, facts: &Facts, plan: &Route, used: usize) -> Reference {
        // `trace_engine`, the hearts lost read beside each sample.
        let mut g = start.clone();
        let h0 = g.state().w(HEARTS);
        let mut s = Sampler::new(facts);
        let mut lost = Vec::new();
        for k in 0..=plan.inputs.len() {
            if Sampler::due(k as u32) {
                if !s.sample(g.state()) {
                    break;
                }
                lost.resize(s.points.len(), h0.saturating_sub(g.state().w(HEARTS)));
            }
            if let Some(&x) = plan.inputs.get(k) {
                g.step_unseen(x);
            }
        }
        let trace = s.points;
        Reference {
            inputs: plan.inputs.clone(),
            points: points_from_trace(&trace),
            lost,
            at: (used / TRACE_EVERY as usize).min(trace.len().saturating_sub(1)),
            trace,
            hearts_lost: plan.hearts_lost,
        }
    }

    /// The furthest point from `at` to `upto` whose region holds `key`, else `at`.
    pub fn locate(&self, key: &CellKey, upto: usize) -> usize {
        let hi = upto.min(self.points.len().saturating_sub(1));
        (self.at..=hi)
            .rev()
            .find(|&k| self.points[k].covers(key))
            .unwrap_or(self.at)
    }

    /// The hearts the plan lost on the engine from point `p` to point `t`.
    pub fn lost_between(&self, p: usize, t: usize) -> u16 {
        let at = |k: usize| self.lost.get(k.min(self.lost.len().saturating_sub(1))).copied();
        at(t).unwrap_or(0).saturating_sub(at(p).unwrap_or(0))
    }

    /// The plan's inputs from point `p` on.
    pub fn inputs_from(&self, p: usize) -> Vec<FrameInput> {
        self.inputs[(p * TRACE_EVERY as usize).min(self.inputs.len())..].to_vec()
    }
}

/// What a stretch's plan heads for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The exit: a whole plan.
    Exit,
    /// The reference's point `t` (a reach stretch).
    Reach(usize),
}

/// Plans `run`'s level against the original (§2.6): the route from the run's start (its `inputs`
/// first) to the frame the original leaves the level, or the best partial route and why, its
/// confirmed part `Stats::confirmed_frames` long. `ro.budget` covers the whole run.
///
/// A repair plans back onto the last whole plan's path `HORIZON` frames on (`Kind::Reach`), and to
/// the exit once that plan's end is near or a reach stretch finds nothing in `REACH_ROUNDS` (T24).
pub fn plan_original(
    run: OracleRun,
    facts: Facts,
    opt: Options,
    ro: &RunOptions,
) -> Result<(Route, Stats), (Route, Stats, Reason)> {
    let t0 = Instant::now();
    let display = sf_conform::display_data();
    let log = std::env::var_os("SF_PLAN_LOG").is_some();
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
    let mut reference: Option<Reference> = None;
    // After an accepted parting, the plan's rest, played on from the original's state as it is.
    let mut resume: Option<(Route, Kind)> = None;
    // Frames the original has ended mid-pass in a row at the kept route's end (`Lag`).
    let mut mid_run = 0u32;
    let mut rounds = 0u64;
    let mut stretch = 0u32;
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
        let base = Options {
            seed: opt.seed.wrapping_add(stretch as u64),
            max_frames: opt.max_frames.saturating_sub(route.len() as u32),
            ..opt.clone()
        };
        // The reference from where the original is, guiding and seeding (§2.7); an explicit guide
        // still steers.
        let follow = |r: &Reference, p: usize| {
            let mut o = follow_options(base.clone(), r.inputs_from(p), &r.trace[p..]);
            if !opt.guide.is_empty() {
                o.guide = opt.guide.clone();
                o.guide_pace = opt.guide_pace;
            }
            o
        };
        let mut planned = match resume.take() {
            Some((r, kind)) => Some((Ok(r), kind)),
            None => None,
        };
        let mut p = 0;
        if planned.is_none()
            && let Some(r) = reference.as_mut().filter(|_| !left.is_zero())
        {
            p = r.locate(&cell_key(&facts, start.state(), 0), r.at + HORIZON_POINTS);
            r.at = p;
            if let Some(t) = reach_target(r.points.len(), p) {
                let o = Options {
                    goal: Goal::Reach {
                        point: r.points[t],
                        within_frames: REACH_FRAMES,
                        hearts: start.state().w(HEARTS).saturating_sub(r.lost_between(p, t)),
                    },
                    ..follow(r, p)
                };
                let reach_ro = RunOptions {
                    budget: left,
                    shorten_share: 0.0,
                    shorten_min_rounds: 0,
                    max_rounds: Some(REACH_ROUNDS),
                    ..ro.clone()
                };
                match plan_engine(start.clone(), facts.clone(), o, &reach_ro) {
                    Ok(r) => planned = Some((Ok(r), Kind::Reach(t))),
                    Err((partial, _)) => {
                        stats.engine_frames += partial.engine_frames;
                        rounds += partial.rounds;
                        if log {
                            eprintln!(
                                "  {:6.1}s stretch {stretch}: no reach from point {p} to {t} {:?} of {} in {} rounds (frog at {:?}), to the exit",
                                t0.elapsed().as_secs_f64(),
                                r.points[t],
                                r.points.len(),
                                partial.rounds,
                                frog_cell(start.state()),
                            );
                        }
                    }
                }
            }
        }
        let (plan, kind) = match planned {
            Some(x) => x,
            None => {
                let left = ro.budget.saturating_sub(t0.elapsed());
                // A repair's whole plan gets the shortening minimum only if its exit is longer
                // than the reference's rest or loses a heart: otherwise its route is mostly the
                // last plan's, already shortened (FINDINGS, T10, T14).
                let (o, over) = match reference.as_ref().filter(|_| !left.is_zero()) {
                    Some(r) => (
                        follow(r, p),
                        Some((r.inputs.len().saturating_sub(p * TRACE_EVERY as usize), r.hearts_lost)),
                    ),
                    None => (base.clone(), None),
                };
                let stretch_ro = RunOptions {
                    budget: left,
                    shorten_min_over: over,
                    ..ro.clone()
                };
                (plan_engine(start.clone(), facts.clone(), o, &stretch_ro), Kind::Exit)
            }
        };
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
        let lag = Lag {
            limit: ro.lag_limit,
            run: mid_run,
        };
        let tail = if kind == Kind::Exit { LAG_TAIL } else { 0 };
        let ls = lockstep_with(
            &mut amiga,
            start.clone(),
            &plan.inputs,
            &mut stats.oracle_frames,
            lag,
            tail,
        );
        stretch += 1;
        if ls.exited {
            route.extend_from_slice(&ls.fed[..ls.agreed]);
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
        let Some((why, cell)) = ls.parting.clone() else {
            // A reach stretch the original played to its end: all of it is kept, and the next
            // stretch starts there, on the reference's path.
            let Kind::Reach(t) = kind else {
                unreachable!("an exit plan with a tail ends in an exit or a parting")
            };
            if log {
                eprintln!(
                    "  {:6.1}s stretch {}: reach to point {t} of {}: plan {} frames ({} rounds, {} hearts), agreed {} ({} hearts), route {}",
                    t0.elapsed().as_secs_f64(),
                    stretch - 1,
                    reference.as_ref().map_or(0, |r| r.points.len()),
                    plan.inputs.len(),
                    plan.rounds,
                    plan.hearts_lost,
                    ls.hearts_lost,
                    ls.agreed,
                    route.len()
                );
            }
            lost += ls.hearts_lost;
            route.extend_from_slice(&ls.fed[..ls.agreed]);
            confirm += c0.elapsed();
            mid_run = ls.mid_run_at(ls.agreed, mid_run);
            if let Some(r) = reference.as_mut() {
                r.at = t;
            }
            stats.confirmed_frames = route.len() as u32;
            continue;
        };
        if log {
            eprintln!(
                "  {:6.1}s stretch {}: {kind:?} plan {} frames ({} rounds), agreed {} ({} held), {why:?} at {cell:?}, route {}",
                t0.elapsed().as_secs_f64(),
                stretch - 1,
                plan.inputs.len(),
                plan.rounds,
                ls.agreed,
                ls.agreed - ls.used_at(ls.agreed).min(ls.agreed),
                route.len()
            );
        }
        // Accepted: the same plan plays on from the state the original insists on, the engine
        // copy taken from it again. Repaired: plan again. Planning again after an acceptance
        // moved set-B W2-2 one frame past a heart the original loses there, then parted again at
        // once, ten repairs at one cell (T14).
        let accepted = repeats.accept(cell, &why);
        let (keep, used, run_then) = kept(&ls, plan.inputs.len(), accepted, mid_run);
        amiga.restore(&snap);
        let mut h = hearts(&amiga);
        for &x in &ls.fed[..keep] {
            // These frames were just played from the same snapshot: they replay the same.
            sync::step_inserting(&mut amiga, x).expect("the kept frames replay");
            stats.oracle_frames += 1;
            let now = hearts(&amiga);
            lost += h.saturating_sub(now);
            h = now;
        }
        confirm += c0.elapsed();
        route.extend_from_slice(&ls.fed[..keep]);
        mid_run = run_then;
        match next_stretch(accepted, &plan, used) {
            Next::Resume(r) => resume = Some((r, kind)),
            // A whole plan that parts is the reference repairs plan back onto; a reach stretch's
            // plan is only a way onto it.
            Next::Repair(_) if kind == Kind::Exit => {
                reference = Some(Reference::new(&start, &facts, &plan, used))
            }
            Next::Repair(_) => {}
        }
        stats.repairs += 1;
        stats.confirmed_frames = route.len() as u32;
    }
}
