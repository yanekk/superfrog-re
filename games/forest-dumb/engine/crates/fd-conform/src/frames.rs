//! The frame bar's runner (translator DESIGN §2.7, T29): a level case on an engine that is a whole
//! machine, judged at the end of every frame on what the bar names: the case's state, the picture
//! in [`PLAY_WINDOW`], Paula's writes of the frame in order, and the CPU's other custom-chip writes
//! of the frame as a multiset (T45).
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
//! **The start** of a level case is the pass runner's: the first step after the sync point in which
//! a pass ends. The engine is transplanted at that step's end (`translated_engine::case_start` takes
//! its start there), and its first frame is the next step's. A title case's engine starts at the end
//! of the sync frame's step, its first frame the next step's.
//!
//! **From power-on** (`start = "entry"`, T32, [`run_entry_frames`]): the engine starts at the
//! program's entry and its first frame is the oracle step that first ran it. Its loads finish at
//! once (DESIGN §2.8), so the frames are lined up by loads (`align = "loads"`,
//! `amiga_conform::loads::ByReturns`): the original runs on, its frames not compared, until it has
//! returned from as many loads as the engine, and a frame with a load running at its end on either
//! side is not compared. The state is the hunks' memory (`compare = ["memory"]`) less the OS's
//! values ([`crate::sync::OS_VALUES`]), and the custom writes are compared only on frames the
//! program had the machine for on the original ([`crate::sync::LEVEL3_HANDLER`] in place at the
//! step's start and end): while the OS has it, its interrupts write registers the machine does not.

use std::collections::BTreeMap;
use std::ops::Range;

use amiga_chips::{Picture, is_paula, to_rgb12};
use amiga_conform::compare::{PictureClass, PictureJudge, classify, paula, picture_except};
use amiga_conform::loads::ByReturns;
use amiga_conform::{Mode, Tolerance, report};
use amiga_oracle::{Amiga, AmigaMemory, FrameCapture, FrameInput, PaulaWrite, SLOW_BASE};
use fd_core::{Mem, Transplant, Var};

use crate::sync::{self, HUNK_SIZES, LOAD_CAP, LOAD_RETURNS, LOAD_ROUTINES, OS_VALUES};
use crate::{
    Case, GAME_OVER_SCREEN, Goal, HIGH_SCORES, HUNK_BASES, Inputs, Live, NEXT_WORLD, Options,
    OracleMem, Outcome, PASS_END, PLAY_WINDOW, Verdict, layout_vars, reach_case,
};

/// The picture a title or power-on case compares: the PAL display window most screens use
/// (DIWSTRT `$2C81`, DIWSTOP `$2CC1`, 320 by 256 from line 44).
pub const SCREEN_WINDOW: (u16, u16) = (0x2C81, 0x2CC1);

/// An engine stepped a whole frame at a time: the translated program on its machine, or a stand-in.
pub trait FrameEngine: Transplant {
    /// Run one frame on `input`, the input the original was given in the same frame.
    fn frame(&mut self, input: FrameInput);
    /// The whole picture of the last frame (it is cut to the case's window here); `None` if none.
    fn picture(&self) -> Option<Picture>;
    /// Paula's writes in the last frame, in order; their beam positions are not compared.
    fn paula(&self) -> &[PaulaWrite];
    /// Every custom-register write the CPU made in the last frame, `(register offset from
    /// $DFF000, value)`, Paula's included; never the copper's, which the oracle does not keep.
    fn custom_writes(&self) -> &[(u16, u16)];
    /// How the engine stopped (translator DESIGN §2.4: `untranslated: hN+$off` and the rest), once
    /// it has; no frame runs after it.
    fn stopped(&self) -> Option<String>;
    /// Chip RAM then slow RAM at the last frame's end, for `compare = ["memory"]`.
    fn ram(&self) -> Option<&[u8]> {
        None
    }
    /// The loads ([`crate::sync::LOAD_ROUTINES`]) that returned in the last frame, and whether one
    /// runs at its end: for `align = "loads"`.
    fn loads(&self) -> (u32, bool) {
        (0, false)
    }
    /// Before a power-on case's first frame: the raster line the original's entry ran on, where
    /// the engine starts its program ([`crate::sync::reach_entry`]).
    fn start_line(&mut self, _line: u16) {}
}

/// The channels a frame is judged on, and how many: state, Paula's writes, the picture, the other
/// custom writes, the hunks' memory.
const CHANNELS: usize = 5;

/// What one frame is judged on and against, and the run's judgement so far.
struct Judge<'a> {
    case: &'a Case,
    opts: &'a Options<'a>,
    vars: Vec<&'static Var>,
    rows: Vec<Range<u16>>,
    window: (u16, u16),
    tol: Tolerance,
    pictures: PictureJudge,
    /// The channels still compared. With `Options::every_channel` a channel that differed is
    /// dropped and the run goes on with the rest.
    live: [bool; CHANNELS],
}

impl<'a> Judge<'a> {
    fn new(case: &'a Case, opts: &'a Options<'a>, window: (u16, u16)) -> Result<Self, String> {
        let rows = case
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
                "{}: the frame runner leaves out whole rows only; a box or a variable exclusion \
                 is the pass runner's",
                case.name
            ));
        }
        let tol = Tolerance::OWNER;
        let w = case.compare.writes;
        Ok(Judge {
            case,
            opts,
            vars: layout_vars(&case.vars)?,
            rows,
            window,
            tol,
            pictures: PictureJudge::new(tol, case.frames),
            live: [
                case.compare.vars,
                w,
                case.compare.picture,
                w,
                case.compare.memory,
            ],
        })
    }

    /// Judge the frame the engine just ran against the oracle's last step (`capture`, numbered
    /// `frame`, the engine's `pass`-th). `writes` is false on a frame whose writes are not the
    /// program's alone on the original. Every channel's first difference is returned, by channel.
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        engine: &dyn FrameEngine,
        am: &Amiga,
        capture: &FrameCapture,
        pass: u32,
        frame: u32,
        writes: bool,
        verdict: &mut Verdict,
    ) -> Result<Vec<(usize, Outcome)>, String> {
        let (case, opts, tol) = (self.case, self.opts, self.tol);
        let theirs = Live(am);
        let mut differs: Vec<(usize, Outcome)> = Vec::new();
        if self.live[0] {
            let first = self.vars.iter().find_map(|v| {
                let (ours, theirs) = (engine.probe(v), theirs.value(v));
                (ours != theirs).then(|| Outcome::Differs {
                    pass,
                    frame,
                    var: v.name.to_string(),
                    engine: ours,
                    oracle: theirs,
                })
            });
            match first {
                Some(o) => differs.push((0, o)),
                None => verdict.passes += 1,
            }
        }
        if self.live[4] {
            let ram = engine
                .ram()
                .ok_or("the case compares memory but the engine gives none")?;
            match memory_difference(ram, &am.memory()) {
                Some(d) => differs.push((
                    4,
                    Outcome::Differs {
                        pass,
                        frame,
                        var: format!("{} ({} bytes differ)", d.at, d.count),
                        engine: d.engine as u32,
                        oracle: d.original as u32,
                    },
                )),
                None => verdict.memory_frames += 1,
            }
        }
        if self.live[1] && writes {
            match paula(engine.paula(), &capture.paula) {
                Some(why) => differs.push((1, Outcome::WritesDiffer { pass, frame, why })),
                None => {
                    verdict.writes += capture.paula.len() as u32;
                    verdict.write_frames += 1;
                }
            }
        }
        if self.live[2] {
            let pic = am.picture_in(self.window.0, self.window.1);
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
            let class = classify(&ours, &pic, &self.rows, &tol);
            let failed = match opts.mode {
                _ if class == PictureClass::Same => self.pictures.frame(frame, class).err(),
                Mode::Strict => Some(format!("{class:?}, strict")),
                Mode::Faithful => self.pictures.frame(frame, class).err(),
            };
            verdict.pictures += 1;
            verdict.tolerated = [
                self.pictures.colour_step.len() as u32,
                self.pictures.few_pixels.len() as u32,
                self.pictures.glitches.len() as u32,
            ];
            if let Some(mut why) = failed {
                let shown = picture_except(&ours, &pic, &self.rows);
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
        if self.live[3] && writes {
            let theirs: Vec<(u16, u16)> = am
                .cpu_writes()
                .iter()
                .map(|w| (w.reg & 0x1FE, w.value))
                .collect();
            match custom_difference(engine.custom_writes(), &theirs) {
                Some(d) => differs.push((
                    3,
                    Outcome::CustomWritesDiffer {
                        pass,
                        frame,
                        reg: d.reg,
                        value: d.value,
                        original: d.original,
                        engine: d.engine,
                    },
                )),
                None => {
                    verdict.custom_writes +=
                        theirs.iter().filter(|(r, _)| !is_paula(*r)).count() as u32;
                    verdict.custom_frames += 1;
                }
            }
        }
        Ok(differs)
    }

    /// Record `differs`: each channel's first goes into `verdict.firsts` and the channel is no
    /// longer compared. Whether the run ends here.
    fn record(&mut self, differs: Vec<(usize, Outcome)>, verdict: &mut Verdict) -> bool {
        for (k, o) in differs {
            self.live[k] = false;
            verdict.firsts.push(o);
            if !self.opts.every_channel || !self.live.contains(&true) {
                return true;
            }
        }
        false
    }
}

/// Run level or title case `case` on `engine`, judged frame by frame; a power-on case goes to
/// [`run_entry_frames`].
pub fn run_frames(
    case: &Case,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    let inputs = Inputs::load(case)?;
    if case.is_entry() {
        return run_entry_frames(case, &inputs, engine, opts);
    }
    run_frames_inputs(case, &inputs, engine, opts)
}

/// [`run_frames`] on `inputs` rather than the scripts the case names, for a level or title case.
pub fn run_frames_inputs(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    if case.is_entry() {
        return Err(format!(
            "{}: a power-on case runs with `run_entry_frames`",
            case.name
        ));
    }
    let level = case.is_level();
    let goal = goal_vars(case)?;
    let window = if level { PLAY_WINDOW } else { SCREEN_WINDOW };
    let mut judge = Judge::new(case, opts, window)?;
    let scene_pcs =
        [GAME_OVER_SCREEN, HIGH_SCORES, NEXT_WORLD].map(|(h, o)| HUNK_BASES[h as usize] + o);
    let mut pcs: Vec<u32> = opts.register.iter().map(crate::coverage::address).collect();
    pcs.extend(scene_pcs);
    let mut am = if level {
        reach_case(case, 0)?
    } else {
        reach_title(case, inputs)?
    };
    am.watch_pcs(&pcs);
    if level {
        am.watch_write(Some(PASS_END));
    }

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    // Transplanted yet (a title case's engine starts at the sync frame), and the frames run since.
    let mut started = !level;
    let mut run = 0u32;

    for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        let capture = am.step(input).map_err(|e| e.to_string())?;
        let frame = am.frame();
        verdict.coverage.add(opts.register, am.pc_hits());
        if !started {
            if let Some(mem) = am.watched_memory() {
                engine.transplant(&OracleMem(&mem));
                started = true;
                am.watch_write(None);
            }
            continue;
        }
        engine.frame(input);
        run += 1;
        // `pass` in an outcome counts the engine's frames from the transplant.
        let pass = run;
        if let Some(stop) = engine.stopped() {
            verdict.firsts.push(Outcome::Stopped { pass, frame, stop });
            break;
        }
        let differs = judge.frame(&*engine, &am, &capture, pass, frame, true, &mut verdict)?;
        if judge.record(differs, &mut verdict) {
            break;
        }
        goal_check(case, &goal, &*engine, &am, frame, &mut verdict);
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
    let nothing = if !started {
        Some("no pass of the original ended after the sync point")
    } else if run == 0 {
        Some("the engine ran no frame after the transplant")
    } else {
        None
    };
    Ok(conclude(case, verdict, nothing))
}

/// Power-on case `case` (`start = "entry"`) on `engine`, which starts at the program's entry:
/// paired with the oracle step that first runs it, then lined up by loads.
pub fn run_entry_frames(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    if !case.is_entry() {
        return Err(format!("{}: not a `start = \"entry\"` case", case.name));
    }
    let goal = goal_vars(case)?;
    let mut judge = Judge::new(case, opts, SCREEN_WINDOW)?;
    let h = |(hunk, off): (u8, u32)| HUNK_BASES[hunk as usize] + off;
    let entries: Vec<u32> = LOAD_ROUTINES.iter().copied().map(h).collect();
    let returns: Vec<u32> = LOAD_RETURNS.iter().copied().map(h).collect();
    let verdict_line;
    let mut pcs: Vec<u32> = opts.register.iter().map(crate::coverage::address).collect();
    pcs.extend(&entries);
    pcs.extend(&returns);
    let (mut am, line) = sync::reach_entry(case, inputs, &pcs)?;
    engine.start_line(line);
    verdict_line = line;
    let handler = h(sync::LEVEL3_HANDLER);
    let program_owns = |am: &Amiga| {
        let v = am.peek(sync::LEVEL3_VECTOR, 4);
        u32::from_be_bytes([v[0], v[1], v[2], v[3]]) == handler
    };
    // Loads entered and returned on the original since the entry.
    let (mut entered, mut returned) = (0u32, 0u32);
    let mut count = |am: &Amiga| {
        let hits = |list: &[u32]| -> u32 {
            am.pc_hits()
                .iter()
                .filter(|(pc, _)| list.contains(pc))
                .map(|&(_, n)| n)
                .sum()
        };
        let r = hits(&returns);
        entered += hits(&entries);
        returned += r;
        (r, entered > returned)
    };
    let mut verdict = Verdict::new(opts.register);
    verdict.entry = Some((am.frame(), verdict_line));
    let mut align = ByReturns::new(LOAD_CAP);
    // The step just run: its input, its capture, whether the program had the machine before it.
    let mut input = inputs.at(am.frame() - 1);
    let mut capture = FrameCapture::default();
    let mut owned_before = false;
    let mut run = 0u32;
    let step = |am: &mut Amiga| -> Result<(FrameInput, FrameCapture), String> {
        let frame = am.frame();
        if let Some(s) = case.swap.filter(|s| s.frame == frame) {
            am.insert_disk(0, s.disk as usize - 1);
        }
        let input = inputs.at(frame);
        let c = am.step(input).map_err(|e| e.to_string())?;
        Ok((input, c))
    };
    // The entry's step's own hits: nothing loads before the entry.
    let (r, l) = count(&am);
    align.original(r, l);
    loop {
        engine.frame(input);
        run += 1;
        let pass = run;
        let frame = am.frame();
        if let Some(stop) = engine.stopped() {
            verdict.firsts.push(Outcome::Stopped { pass, frame, stop });
            break;
        }
        let (r, l) = engine.loads();
        align.engine(r, l);
        while align.behind() {
            if let Err(e) = align.skip() {
                verdict.firsts.push(Outcome::LoadApart {
                    pass,
                    frame: am.frame(),
                    why: format!("load {}: {e}", align.returned().0),
                });
                break;
            }
            if am.frame() >= case.frames {
                break;
            }
            owned_before = program_owns(&am);
            // The engine has run past this step's input: its next frame takes the next step's.
            (_, capture) = step(&mut am)?;
            verdict.coverage.add(opts.register, am.pc_hits());
            let (r, l) = count(&am);
            align.original(r, l);
        }
        if !verdict.firsts.is_empty() || align.behind() {
            break;
        }
        let frame = am.frame();
        if align.compared() {
            verdict.aligned_frames += 1;
            let writes = owned_before && program_owns(&am);
            if !writes {
                verdict.os_frames += 1;
            }
            let differs =
                judge.frame(&*engine, &am, &capture, pass, frame, writes, &mut verdict)?;
            if judge.record(differs, &mut verdict) {
                break;
            }
            goal_check(case, &goal, &*engine, &am, frame, &mut verdict);
        }
        if am.frame() >= case.frames {
            break;
        }
        owned_before = program_owns(&am);
        (input, capture) = step(&mut am)?;
        verdict.coverage.add(opts.register, am.pc_hits());
        let (r, l) = count(&am);
        align.original(r, l);
    }
    verdict.loads = align.returned().1;
    verdict.load_frames = align.skipped();
    if verdict.firsts.is_empty() && align.returned().0 != align.returned().1 {
        let (e, o) = align.returned();
        verdict.firsts.push(Outcome::LoadApart {
            pass: run,
            frame: am.frame(),
            why: format!(
                "the engine returned from {e} loads, the original from {o}, by the case's end"
            ),
        });
    }
    let nothing = (verdict.aligned_frames == 0).then_some("no frame outside a load on both sides");
    Ok(conclude(case, verdict, nothing))
}

/// The original from power-on to the end of title case `case`'s sync frame, its swap made on the way.
fn reach_title(case: &Case, inputs: &Inputs) -> Result<Amiga, String> {
    use amiga_oracle::{OracleConfig, kickstart_path};
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let cfg = OracleConfig::new(
        kickstart,
        crate::disks::load()?,
        case.sync_frame + case.frames,
    );
    let mut am = Amiga::power_on(&cfg).map_err(|e| e.to_string())?;
    for frame in 0..case.sync_frame {
        if let Some(s) = case.swap.filter(|s| s.frame == frame) {
            am.insert_disk(0, s.disk as usize - 1);
        }
        am.step(inputs.at(frame)).map_err(|e| e.to_string())?;
    }
    Ok(am)
}

fn goal_vars(case: &Case) -> Result<Vec<(&'static Var, u32)>, String> {
    match &case.goal {
        Some(Goal::All { all, .. }) => all
            .iter()
            .map(|(v, value)| Ok((layout_vars(std::slice::from_ref(v))?[0], *value)))
            .collect(),
        _ => Ok(Vec::new()),
    }
}

/// Whether an `all` goal holds on both sides at this compared frame.
fn goal_check(
    case: &Case,
    goal: &[(&Var, u32)],
    engine: &dyn FrameEngine,
    am: &Amiga,
    frame: u32,
    verdict: &mut Verdict,
) {
    if let Some(Goal::All { by_frame, .. }) = &case.goal
        && verdict.goal_met.is_none()
        && frame <= *by_frame
        && goal
            .iter()
            .all(|(v, value)| engine.probe(v) == *value && Live(am).value(v) == *value)
    {
        verdict.goal_met = Some(frame);
    }
}

/// The outcome of a run that ended: its first difference, or what it compared nothing of
/// (`nothing`), or the goal it missed, or an exclusion it did not need.
fn conclude(case: &Case, mut verdict: Verdict, nothing: Option<&str>) -> Verdict {
    if let Some(first) = verdict.firsts.first() {
        verdict.outcome = first.clone();
        return verdict;
    }
    let none = |what: &str| {
        Some(Outcome::ComparedNothing {
            what: what.to_string(),
        })
    };
    let only_writes =
        case.compare.writes && !case.compare.vars && !case.compare.picture && !case.compare.memory;
    let failed = if let Some(what) = nothing {
        none(what)
    } else if only_writes && verdict.writes == 0 {
        none("neither side wrote to Paula in any compared frame")
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
    verdict
}

/// The first byte of the program's hunks that differs between the engine's RAM (chip then slow)
/// and the original's, the OS's values left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDifference {
    /// `hN+$off`.
    pub at: String,
    pub engine: u8,
    pub original: u8,
    /// How many compared bytes differ.
    pub count: u32,
}

/// [`MemoryDifference`] of `ours` against `theirs`; `None` if every compared byte is equal.
pub fn memory_difference(ours: &[u8], theirs: &AmigaMemory) -> Option<MemoryDifference> {
    let chip = theirs.chip.len();
    let byte_ours = |a: u32| -> u8 {
        let i = if a >= SLOW_BASE {
            chip + (a - SLOW_BASE) as usize
        } else {
            a as usize
        };
        ours.get(i).copied().unwrap_or(0)
    };
    let mut first = None;
    let mut count = 0u32;
    for (h, &size) in HUNK_SIZES.iter().enumerate() {
        let base = HUNK_BASES[h];
        let left_out = |off: u32| {
            OS_VALUES
                .iter()
                .any(|&(oh, o, n)| oh as usize == h && (o..o + n).contains(&off))
        };
        for off in 0..size {
            let a = base + off;
            let (o, t) = (byte_ours(a), theirs.byte_at(a).unwrap_or(0));
            if o != t && !left_out(off) {
                count += 1;
                first.get_or_insert((format!("h{h}+${off:x}"), o, t));
            }
        }
    }
    first.map(|(at, engine, original)| MemoryDifference {
        at,
        engine,
        original,
        count,
    })
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
