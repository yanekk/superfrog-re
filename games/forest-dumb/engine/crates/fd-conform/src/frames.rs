//! The frame bar's runner (translator DESIGN §2.7, T29): a level case on an engine that is a whole
//! machine, stepped a frame at a time and judged on what the bar names: the case's state and the
//! custom-chip writes, Paula's in order and the CPU's others as a multiset (T45), at every pass end;
//! the picture in [`PLAY_WINDOW`] at every frame's end.
//!
//! **At the pass end, not the frame's** (owner, 2026-11-12, T30). The original's `game_frame`
//! returns after the frame's end on almost every moving frame (docs/09, 2 977 of 3 000), so its
//! state at a frame's end is mid-pass, and a machine with no DMA contention ends the same work some
//! lines earlier. Both sides' state is therefore taken where the CPU writes [`PASS_END`]
//! (`Amiga::watch_write`, `FrameEngine::pass_ends`) and paired by `irq_count`, and the writes are
//! cut into passes at that same write in each side's own stream and paired in order. A pass may end
//! up to [`PASS_LAG`] frames apart on the two sides; one further apart fails
//! (`Outcome::PassEndsApart`). The picture is what a frame showed, so it stays per frame.
//!
//! **Custom writes.** §2.7 asks order of Paula's writes only, and the bar does not chase sub-frame
//! timing, so every other register's writes are compared as (register, value) counts: the same
//! writes, each as many times (owner, 2026-11-12). Paula's registers are left out of that channel,
//! being compared in order already. Both sides give the CPU's writes alone, tagged where made: the
//! oracle keeps no Copper write (`Amiga::cpu_writes`), and the engine hands `pcode-machine`'s
//! `cpu_writes`, not its log with the copper's MOVEs in it.
//!
//! **Why not [`crate::run_with`].** The pass runner pairs the original's pass ends with an engine
//! that knows only passes: it hands each pass the input that pass read ([`crate::PassInputs`]),
//! compares at [`crate::PASS_END`], and shows each pass in the frame after it. A machine running the
//! original's program has frames, not passes: it reads the joystick when its own code does, so it
//! takes the input of each frame as the oracle got it, and its state, picture and writes are those
//! of the frame's end, which is where an oracle step ends. So the two step together, one engine
//! frame an oracle step, and nothing of the hand engine's measured cuts (T71, T74, T80, T82) applies.
//!
//! **Every channel.** A run ends at its first difference unless [`Options::every_channel`] is set:
//! then a channel that differed is no longer compared, the others go on, and
//! [`Verdict::firsts`] holds each channel's first difference in the order met (a stop always ends
//! the run). A whole-machine engine can differ in one channel from the first frame (its timing
//! moves a write across a frame's end) and still be right in the others, which a first reading
//! needs to see.
//!
//! **The start** is the pass runner's: the first step after the sync point in which a pass ends.
//! The engine is transplanted at that step's end (`translated_engine::case_start` takes its start
//! there), and its first frame is the next step's.

use std::collections::BTreeMap;

use amiga_chips::{Picture, is_paula, to_rgb12};
use amiga_conform::compare::{PictureClass, PictureJudge, classify, paula, picture_except};
use amiga_conform::{Mode, Tolerance, report};
use amiga_oracle::{FrameInput, PaulaWrite};
use std::collections::VecDeque;

use fd_core::{Mem, Transplant, Var, layout_var};

use crate::{
    Case, GAME_OVER_SCREEN, Goal, HIGH_SCORES, HUNK_BASES, Inputs, NEXT_WORLD, Options, OracleMem,
    Outcome, PASS_END, PLAY_WINDOW, Verdict, layout_vars, reach_case,
};

/// How many frames apart one pass may end on the two sides: the original's ends near line 310 or
/// in the next frame's first lines (docs/09), so a frame either way is the sub-frame timing the bar
/// does not chase (DESIGN §2.7); two is a different pace.
pub const PASS_LAG: u32 = 1;

/// An engine stepped a whole frame at a time: the translated program on its machine, or a stand-in.
pub trait FrameEngine: Transplant {
    /// Run one frame on `input`, the input the original was given in the same frame.
    fn frame(&mut self, input: FrameInput);
    /// The whole picture of the last frame (it is cut to [`PLAY_WINDOW`] here); `None` if none.
    fn picture(&self) -> Option<Picture>;
    /// Paula's writes in the last frame, in order; their beam positions are not compared.
    fn paula(&self) -> &[PaulaWrite];
    /// Every custom-register write the CPU made in the last frame, `(register offset from
    /// $DFF000, value)`, Paula's included; never the copper's, which the oracle does not keep.
    fn custom_writes(&self) -> &[(u16, u16)];
    /// `vars`' values at each pass end of the last frame (each CPU write of [`PASS_END`]), in
    /// order; empty if no pass ended in it.
    fn pass_ends(&self, vars: &[&Var]) -> Vec<Vec<u32>>;
    /// How the engine stopped (translator DESIGN §2.4: `untranslated: hN+$off` and the rest), once
    /// it has; no frame runs after it.
    fn stopped(&self) -> Option<String>;
}

/// Run level case `case` on `engine`, judged frame by frame.
pub fn run_frames(
    case: &Case,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    run_frames_inputs(case, &Inputs::load(case)?, engine, opts)
}

/// [`run_frames`] on `inputs` rather than the scripts the case names.
pub fn run_frames_inputs(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    if !case.is_level() {
        return Err(format!(
            "{}: the frame runner runs level cases; a title case runs with `run`",
            case.name
        ));
    }
    let vars = layout_vars(&case.vars)?;
    let goal: Vec<(&Var, u32)> = match &case.goal {
        Some(Goal::All { all, .. }) => all
            .iter()
            .map(|(v, value)| Ok((layout_vars(std::slice::from_ref(v))?[0], *value)))
            .collect::<Result<_, String>>()?,
        _ => Vec::new(),
    };
    let rows: Vec<std::ops::Range<u16>> = case
        .exclude
        .iter()
        .filter(|e| e.var.is_none() && e.cols.is_none())
        .map(|e| e.rows.clone())
        .collect();
    if case
        .exclude
        .iter()
        .any(|e| e.var.is_some() || e.cols.is_some())
    {
        return Err(format!(
            "{}: the frame runner leaves out whole rows only; a box or a variable exclusion is the \
             pass runner's",
            case.name
        ));
    }
    let scene_pcs =
        [GAME_OVER_SCREEN, HIGH_SCORES, NEXT_WORLD].map(|(h, o)| HUNK_BASES[h as usize] + o);
    let mut pcs: Vec<u32> = opts.register.iter().map(crate::coverage::address).collect();
    pcs.extend(scene_pcs);
    let mut am = reach_case(case, 0)?;
    am.watch_pcs(&pcs);
    am.watch_write(Some(PASS_END));

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    let tol = Tolerance::OWNER;
    let mut judge = PictureJudge::new(tol, case.frames);
    // Transplanted yet, and the engine frames run since.
    let mut started = false;
    let mut run = 0u32;
    // The channels still compared: state, Paula's writes, the picture, the other custom writes.
    // With `Options::every_channel` a channel that differed is dropped and the run goes on with
    // the rest.
    let w = case.compare.writes;
    let mut live = [case.compare.vars, w, case.compare.picture, w];
    // What is read at a pass end: the case's vars, the goal's, and `irq_count`, which names it.
    let irq = layout_var("irq_count").ok_or("irq_count is not in LAYOUT")?;
    let mut probe: Vec<&Var> = vars.clone();
    probe.extend(goal.iter().map(|(v, _)| *v));
    probe.push(irq);
    let at_irq = probe.len() - 1;
    let mut ends = [PassEnds::default(), PassEnds::default()];
    let mut start_irq = 0u32;
    let mut paula_passes = [Passes::default(), Passes::default()];
    let mut custom_passes = [Passes::default(), Passes::default()];
    let mut carried: BTreeMap<(u16, u16), Carried> = BTreeMap::new();

    'steps: for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        let capture = am.step(input).map_err(|e| e.to_string())?;
        let frame = am.frame();
        verdict.coverage.add(opts.register, am.pc_hits());
        if !started {
            if let Some(mem) = am.watched_memory() {
                engine.transplant(&OracleMem(&mem));
                start_irq = OracleMem(&mem).value(irq);
                started = true;
            }
            continue;
        }
        engine.frame(input);
        run += 1;
        if let Some(stop) = engine.stopped() {
            verdict.firsts.push(Outcome::Stopped {
                pass: run,
                frame,
                stop,
            });
            break;
        }
        let mut differs: Vec<(usize, Outcome)> = Vec::new();

        // State, at each pass end both sides have reached.
        if let Some(mem) = am.watched_memory() {
            let m = OracleMem(&mem);
            ends[0].add(frame, probe.iter().map(|v| m.value(v)).collect(), at_irq);
        }
        for values in engine.pass_ends(&probe) {
            ends[1].add(frame, values, at_irq);
        }
        while let Some((k, theirs, ours)) = PassEnds::pair(&mut ends, &mut verdict.unseen) {
            let pass = k.wrapping_sub(start_irq);
            let at = theirs.0;
            if live[0]
                && let Some(o) = too_far(pass, theirs.0, ours.0, &format!("irq_count {k}"))
            {
                differs.push((0, o));
                break;
            }
            if live[0] {
                let first = (0..vars.len())
                    .find(|&i| ours.1[i] != theirs.1[i])
                    .map(|i| Outcome::Differs {
                        pass,
                        frame: at,
                        var: vars[i].name.to_string(),
                        engine: ours.1[i],
                        oracle: theirs.1[i],
                    });
                match first {
                    Some(o) => differs.push((0, o)),
                    None => verdict.passes += 1,
                }
            }
            if let Some(Goal::All { by_frame, .. }) = &case.goal
                && verdict.goal_met.is_none()
                && at <= *by_frame
                && goal.iter().enumerate().all(|(i, (_, value))| {
                    let i = vars.len() + i;
                    ours.1[i] == *value && theirs.1[i] == *value
                })
            {
                verdict.goal_met = Some(at);
            }
        }
        if live[0]
            && let Some(o) = PassEnds::apart(&ends, frame)
        {
            differs.push((0, o));
        }

        // The writes, cut into passes at each side's own PASS_END write.
        if live[1] {
            let theirs: Vec<(u16, u16)> = capture.paula.iter().map(|w| (w.reg, w.value)).collect();
            let ours: Vec<(u16, u16)> = engine.paula().iter().map(|w| (w.reg, w.value)).collect();
            paula_passes[0].add(frame, &theirs);
            paula_passes[1].add(frame, &ours);
            while let Some((n, at, ours_at, theirs, ours)) = Passes::pair(&mut paula_passes) {
                if let Some(o) = too_far(n, at, ours_at, "Paula's writes") {
                    differs.push((1, o));
                    break;
                }
                match paula(&as_paula(&ours), &as_paula(&theirs)) {
                    Some(why) => {
                        differs.push((
                            1,
                            Outcome::WritesDiffer {
                                pass: n,
                                frame: at,
                                why,
                            },
                        ));
                        break;
                    }
                    None => {
                        verdict.writes += theirs.len() as u32;
                        verdict.write_frames += 1;
                    }
                }
            }
            if !differs.iter().any(|(k, _)| *k == 1)
                && let Some(o) = Passes::apart(&paula_passes, frame, "Paula's writes")
            {
                differs.push((1, o));
            }
        }
        if live[3] {
            let theirs: Vec<(u16, u16)> = am
                .cpu_writes()
                .iter()
                .map(|w| (w.reg & 0x1FE, w.value))
                .collect();
            custom_passes[0].add(frame, &theirs);
            custom_passes[1].add(frame, engine.custom_writes());
            while let Some((n, at, ours_at, theirs, ours)) = Passes::pair(&mut custom_passes) {
                if let Some(o) = too_far(n, at, ours_at, "custom writes") {
                    differs.push((3, o));
                    break;
                }
                match custom_carried(&mut carried, &ours, &theirs) {
                    Some(d) => {
                        differs.push((
                            3,
                            Outcome::CustomWritesDiffer {
                                pass: n,
                                frame: at,
                                reg: d.reg,
                                value: d.value,
                                original: d.original,
                                engine: d.engine,
                            },
                        ));
                        break;
                    }
                    None => {
                        verdict.custom_writes +=
                            theirs.iter().filter(|(r, _)| !is_paula(*r)).count() as u32;
                        verdict.custom_frames += 1;
                    }
                }
            }
            if !differs.iter().any(|(k, _)| *k == 3)
                && let Some(o) = Passes::apart(&custom_passes, frame, "custom writes")
            {
                differs.push((3, o));
            }
        }

        // The picture, at the frame's end: what the frame showed.
        if live[2] {
            let pass = run;
            let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
            let whole = engine
                .picture()
                .ok_or("the case compares the picture but the engine draws none")?;
            let ours = whole.rewindow(pic.left as i32, pic.top, pic.width, pic.height);
            let colours = to_rgb12(&ours);
            for (i, e) in case.exclude.iter().enumerate() {
                let differs = (e.rows.start..e.rows.end.min(pic.height)).any(|y| {
                    (0..pic.width).any(|x| colours[(y * pic.width + x) as usize] != pic.pixel(x, y))
                });
                verdict.excluded[i].1 += differs as u32;
            }
            let class = classify(&ours, &pic, &rows, &tol);
            let failed = match opts.mode {
                _ if class == PictureClass::Same => judge.frame(frame, class).err(),
                Mode::Strict => Some(format!("{class:?}, strict")),
                Mode::Faithful => judge.frame(frame, class).err(),
            };
            verdict.pictures += 1;
            verdict.tolerated = [
                judge.colour_step.len() as u32,
                judge.few_pixels.len() as u32,
                judge.glitches.len() as u32,
            ];
            if let Some(mut why) = failed {
                let shown = picture_except(&ours, &pic, &rows);
                if let Some((x, y, o, e)) = shown.as_ref().and_then(|m| m.first) {
                    why += &format!(
                        "; first at ({x}, {y}): {o:03x} in the original, {e:03x} in the engine"
                    );
                }
                if let (Some(dir), Some(m)) = (&opts.diff_dir, shown) {
                    let d = report::write_diff_png(dir, &case.name, frame, &ours, &pic, m);
                    why += &format!("; {}", d.path.display());
                }
                differs.push((2, Outcome::PictureDiffers { pass, frame, why }));
            }
        }
        differs.sort_by_key(|(k, _)| *k);
        for (k, o) in differs {
            live[k] = false;
            verdict.firsts.push(o);
            if !opts.every_channel || !live.contains(&true) {
                break 'steps;
            }
        }
        // The scenes are the program's own on both sides, so they are judged by the frame's state
        // and picture above; here they only end the comparison where the pass runner ends it.
        let ran = |pc: u32| am.pc_hits().iter().any(|&(p, n)| p == pc && n > 0);
        if ran(scene_pcs[2]) {
            verdict.level_end = Some(frame);
            break;
        }
        if ran(scene_pcs[0]) && verdict.game_over.is_none() {
            verdict.game_over = Some((frame, None));
        }
        if ran(scene_pcs[1]) {
            verdict.game_over = Some((verdict.game_over.map_or(frame, |g| g.0), Some(frame)));
            break;
        }
    }
    if let Some(first) = verdict.firsts.first() {
        verdict.outcome = first.clone();
        return Ok(verdict);
    }

    let nothing = |what: &str| {
        Some(Outcome::ComparedNothing {
            what: what.to_string(),
        })
    };
    let only_writes = case.compare.writes && !case.compare.vars && !case.compare.picture;
    let failed = if !started {
        nothing("no pass of the original ended after the sync point")
    } else if run == 0 {
        nothing("the engine ran no frame after the transplant")
    } else if only_writes && verdict.writes == 0 {
        nothing("neither side wrote to Paula in any compared frame")
    } else {
        match &case.goal {
            Some(Goal::All { all, by_frame }) if verdict.goal_met.is_none() => {
                let want: Vec<String> = all.iter().map(|(v, n)| format!("{v} = {n}")).collect();
                Some(Outcome::GoalMissed {
                    goal: format!("{} by oracle frame {by_frame}", want.join(", ")),
                })
            }
            Some(Goal::GameOver { by_frame }) => match verdict.game_over {
                Some((_, Some(left))) if left <= *by_frame => {
                    verdict.goal_met = Some(left);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!(
                        "the game over screen left for the high scores by oracle frame {by_frame}"
                    ),
                }),
            },
            Some(Goal::LevelEnd { by_frame }) => match verdict.level_end {
                Some(end) if end <= *by_frame => {
                    verdict.goal_met = Some(end);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!("the level's end (`next_world`) by oracle frame {by_frame}"),
                }),
            },
            _ => None,
        }
    };
    let unused = || {
        verdict
            .excluded
            .iter()
            .find(|(_, n)| *n == 0)
            .map(|(e, _)| Outcome::ExclusionUnused {
                what: e.what.clone(),
                task: e.task.clone(),
            })
    };
    verdict.outcome = failed.or_else(unused).unwrap_or(Outcome::Same);
    Ok(verdict)
}

/// The first (register, value) whose count differs between two frames' custom writes, by register
/// then value; `None` if every count is equal. Paula's registers are left out (compared in order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustomDifference {
    pub reg: u16,
    pub value: u16,
    pub original: u32,
    pub engine: u32,
}

/// [`CustomDifference`] of `ours` (the engine's) against `theirs` (the original's), each
/// `(register, value)`.
pub fn custom_difference(ours: &[(u16, u16)], theirs: &[(u16, u16)]) -> Option<CustomDifference> {
    let mut counts: BTreeMap<(u16, u16), (u32, u32)> = BTreeMap::new();
    for &(reg, value) in theirs.iter().filter(|(r, _)| !is_paula(*r)) {
        counts.entry((reg, value)).or_default().0 += 1;
    }
    for &(reg, value) in ours.iter().filter(|(r, _)| !is_paula(*r)) {
        counts.entry((reg, value)).or_default().1 += 1;
    }
    counts
        .into_iter()
        .find(|(_, (o, e))| o != e)
        .map(|((reg, value), (original, engine))| CustomDifference {
            reg,
            value,
            original,
            engine,
        })
}

/// One side's pass ends not yet paired: `irq_count` → (oracle frame, the probed values).
#[derive(Default)]
struct PassEnds {
    open: BTreeMap<u32, (u32, Vec<u32>)>,
    /// The highest `irq_count` this side has ended a pass at.
    last: Option<u32>,
    /// The original's side: the last pass paired, so a pass it never showed is counted unseen.
    paired: Option<u32>,
}

type Ended = (u32, Vec<u32>);

impl PassEnds {
    fn add(&mut self, frame: u32, values: Vec<u32>, at_irq: usize) {
        let k = values[at_irq];
        self.last = Some(self.last.map_or(k, |l| l.max(k)));
        self.open.entry(k).or_insert((frame, values));
    }

    /// The lowest pass both sides ended, (irq_count, the original's, the engine's). The oracle
    /// shows a step's first pass end only (docs/09): a pass the engine ended that the original has
    /// gone past unseen is dropped, and every pass the original skipped is counted in `unseen`.
    fn pair(ends: &mut [PassEnds; 2], unseen: &mut u32) -> Option<(u32, Ended, Ended)> {
        if let Some(o) = ends[0].last {
            let [theirs, ours] = ends;
            ours.open
                .retain(|k, _| *k >= o || theirs.open.contains_key(k));
        }
        let k = *ends[0].open.keys().find(|k| ends[1].open.contains_key(k))?;
        if let Some(p) = ends[0].paired {
            *unseen += k.saturating_sub(p + 1);
        }
        ends[0].paired = Some(k);
        Some((k, ends[0].open.remove(&k)?, ends[1].open.remove(&k)?))
    }

    /// A pass one side ended more than [`PASS_LAG`] frames before `frame` that the other has not.
    fn apart(ends: &[PassEnds; 2], frame: u32) -> Option<Outcome> {
        (0..2).find_map(|side| {
            let (k, (at, _)) = ends[side].open.iter().next()?;
            (frame > at + PASS_LAG).then(|| Outcome::PassEndsApart {
                pass: *k,
                frame: *at,
                why: format!(
                    "irq_count {k} ended at oracle frame {at} in the {}, not by frame {} in the {}",
                    SIDE[side],
                    at + PASS_LAG,
                    SIDE[1 - side]
                ),
            })
        })
    }
}

const SIDE: [&str; 2] = ["original", "engine"];

/// A pass the original ended in oracle frame `theirs` and the engine in `ours`, more than
/// [`PASS_LAG`] frames apart.
fn too_far(pass: u32, theirs: u32, ours: u32, what: &str) -> Option<Outcome> {
    (theirs.abs_diff(ours) > PASS_LAG).then(|| Outcome::PassEndsApart {
        pass,
        frame: theirs,
        why: format!(
            "{what}: the pass ended at oracle frame {theirs} in the original, {ours} in the engine"
        ),
    })
}

/// One side's custom writes cut into passes at its own [`PASS_END`] write (which closes a pass),
/// the closed passes not yet paired with the other side's.
#[derive(Default)]
struct Passes {
    open: Vec<(u16, u16)>,
    closed: VecDeque<(u32, Vec<(u16, u16)>)>,
    /// Passes paired so far.
    paired: u32,
}

impl Passes {
    fn add(&mut self, frame: u32, writes: &[(u16, u16)]) {
        for &w in writes {
            self.open.push(w);
            if w == PASS_END {
                self.closed
                    .push_back((frame, std::mem::take(&mut self.open)));
            }
        }
    }

    /// The next pass both sides closed: (its number from the transplant, the original's frame,
    /// the engine's, the original's writes, the engine's).
    #[allow(clippy::type_complexity)]
    fn pair(p: &mut [Passes; 2]) -> Option<(u32, u32, u32, Vec<(u16, u16)>, Vec<(u16, u16)>)> {
        if p[0].closed.is_empty() || p[1].closed.is_empty() {
            return None;
        }
        let (at, theirs) = p[0].closed.pop_front()?;
        let (ours_at, ours) = p[1].closed.pop_front()?;
        p[0].paired += 1;
        Some((p[0].paired, at, ours_at, theirs, ours))
    }

    /// A pass one side closed more than [`PASS_LAG`] frames before `frame` that the other has not.
    fn apart(p: &[Passes; 2], frame: u32, what: &str) -> Option<Outcome> {
        (0..2).find_map(|side| {
            let (at, _) = p[side].closed.front()?;
            (frame > at + PASS_LAG).then(|| Outcome::PassEndsApart {
                pass: p[0].paired + 1,
                frame: *at,
                why: format!(
                    "{what}: the pass ended at oracle frame {at} in the {}, not by frame {} in the {}",
                    SIDE[side],
                    at + PASS_LAG,
                    SIDE[1 - side]
                ),
            })
        })
    }
}

fn as_paula(w: &[(u16, u16)]) -> Vec<PaulaWrite> {
    w.iter()
        .map(|&(reg, value)| PaulaWrite {
            reg,
            value,
            hpos: 0,
            vpos: 0,
        })
        .collect()
}

/// A (register, value)'s writes since the two sides' counts of it last agreed: the original's,
/// the engine's, and how many pass ends they have disagreed across.
#[derive(Debug, Clone, Copy, Default)]
pub struct Carried {
    original: u32,
    engine: u32,
    passes: u32,
}

/// [`custom_difference`] with a pass of slack (owner, 2026-11-12, T30): `main_loop`'s blits are
/// cut by the pass end wherever they have got to, so a write may fall a pass early or late on a
/// machine of another pace. Each (register, value)'s counts are carried from pass to pass; they
/// must agree again by the next pass end, or differ (the counts then cover both passes). A write
/// missing or added for good fails a pass after it, however often the same write recurs.
pub fn custom_carried(
    carried: &mut BTreeMap<(u16, u16), Carried>,
    ours: &[(u16, u16)],
    theirs: &[(u16, u16)],
) -> Option<CustomDifference> {
    for &(reg, value) in theirs.iter().filter(|(r, _)| !is_paula(*r)) {
        carried.entry((reg, value)).or_default().original += 1;
    }
    for &(reg, value) in ours.iter().filter(|(r, _)| !is_paula(*r)) {
        carried.entry((reg, value)).or_default().engine += 1;
    }
    carried.retain(|_, c| c.original != c.engine);
    for c in carried.values_mut() {
        c.passes += 1;
    }
    carried
        .iter()
        .find(|(_, c)| c.passes > PASS_LAG)
        .map(|(&(reg, value), c)| CustomDifference {
            reg,
            value,
            original: c.original,
            engine: c.engine,
        })
}
