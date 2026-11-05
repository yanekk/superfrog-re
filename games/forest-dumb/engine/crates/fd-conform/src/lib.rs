//! Shell, test tooling: Forest Dumb Forever's conformance runner. A case brings the original on the
//! oracle to a sync point, starts the engine there, feeds both the same input and compares the
//! modelled variables and the picture (plans/forest-dumb-level-1 DESIGN §2.1, §4, §4.1). The format
//! is in `engine/conform/README.md`.
//!
//! Two runners share the case format. A title case (forest-dumb-start's first case) steps the
//! title scene once a frame from power-on. A level case starts at the level sync point
//! (`sync.rs`) and compares at the end of each pass of `game_frame`: in play a pass usually ends
//! after the frame does, so the state at a frame's end is mid-pass (docs/09-play-measured.md).

pub mod case;
pub mod coverage;
pub mod disks;
pub mod freeze;
// `gen` is a reserved word in edition 2024; the files keep the task's folder name.
#[path = "gen/mod.rs"]
pub mod generators;
pub mod judge;
pub mod routine;
pub mod sync;

use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use amiga_chips::{Picture, to_rgb12};
use amiga_conform::compare::{PictureClass, PictureJudge, classify, picture_except};
use amiga_conform::{Mode, Tolerance, report, script};
use amiga_oracle::{
    Amiga, AmigaMemory, FrameInput, OracleConfig, Rgb12Frame, SLOW_BASE, kickstart_path,
};
use std::sync::Arc;

use fd_core::{
    Event, Game, GameData, Mem, PORTED, Ported, Start, TitleScene, Transplant, Var, layout_var,
};

pub use case::{Case, Compare, Exclusion, Goal, Swap};
pub use coverage::Coverage;

/// The main program's hunks where the boot of `boot-to-title` loads them, and `boot-to-level1`'s too
/// (it shares the boot up to the title)
/// (`docs/07-executable-map.md`). AmigaDOS picks them when the program loads, so another boot may
/// differ; there a wrong base shows as a comparison failing on its first change.
pub const HUNK_BASES: [u32; 5] = fd_core::HUNK_BASES;

/// The display window a level case compares, in DIWSTRT/DIWSTOP's encoding: the play area's top
/// (line 47), the status bar's left, right and bottom (lores 129..449, to line 297). The copper
/// reloads DIW between the two, so the frame-end window holds only the bar
/// (docs/09-play-measured.md "Display"; `tests/play_picture.rs`). Row 0 is line 47; the bar starts
/// at row 194.
pub const PLAY_WINDOW: (u16, u16) = (0x2F81, 0x29C1);

/// The end of a pass: `level3_handler` writes `INTREQ=$4010` at `h0+$1f02`, just after
/// `game_frame` returns (custom register `$9c`, value).
pub const PASS_END: (u16, u16) = (0x9c, 0x4010);

/// `engine/conform/`, wherever this is run from.
pub fn conform_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conform")
}

/// Where variable `name` is in the original's memory, on the boot the case runs.
pub fn address(name: &str) -> Result<u32, String> {
    let v = layout_var(name).ok_or_else(|| format!("`{name}` is not in fd-core's LAYOUT"))?;
    Ok(HUNK_BASES[v.hunk as usize] + v.offset)
}

/// The original's RAM as the main program's hunks see it.
pub struct OracleMem<'a>(pub &'a AmigaMemory);

impl Mem for OracleMem<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        let m = self.0;
        let slow = SLOW_BASE as usize;
        (0..len as u32)
            .map(|k| {
                let a = (HUNK_BASES[hunk as usize] + offset + k) as usize & 0xFF_FFFF;
                if a < m.chip.len() {
                    m.chip[a]
                } else if a >= slow && a - slow < m.slow.len() {
                    m.slow[a - slow]
                } else {
                    0
                }
            })
            .collect()
    }
}

/// The original as it runs, read through [`Mem`] without copying its RAM.
pub struct Live<'a>(pub &'a Amiga);

impl Mem for Live<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        self.0.peek(HUNK_BASES[hunk as usize] + offset, len)
    }
}

/// What a level case runs on the engine's side: started from the original's memory at a pass's
/// end ([`Transplant`]), then one pass of play per call.
pub trait Engine: Transplant {
    /// One pass of play on the input the original's pass read.
    fn pass(&mut self, input: FrameInput);
    /// The picture after the last pass, in [`PLAY_WINDOW`]; `None` if the engine draws none.
    fn picture(&self) -> Option<Picture>;
    /// What happened in the last pass that a case must see. An [`Event::Unported`] fails every
    /// case (DESIGN §4.1); an `idle` goal fails on any event.
    fn events(&mut self) -> Vec<Event> {
        Vec::new()
    }
}

/// The level engine, fd-core's [`Game`], as a case runs it: one [`Game::step`] a pass. It draws
/// nothing until T10.
impl Engine for Game {
    fn pass(&mut self, input: FrameInput) {
        self.step(input);
    }

    fn picture(&self) -> Option<Picture> {
        None
    }

    fn events(&mut self) -> Vec<Event> {
        self.take_events()
    }
}

/// The game's data from the owner's disks, which [`fd_tools::disk_dir`] finds; a missing disk or
/// file is an error naming it.
pub fn game_data() -> Result<Arc<GameData>, String> {
    let dir = fd_tools::disk_dir()?;
    game_data_in(&dir)
}

/// [`game_data`] from the zips in `dir`.
pub fn game_data_in(dir: &Path) -> Result<Arc<GameData>, String> {
    let files = |name: &str| {
        // The main program is disk 1's; every level file is disk 2's.
        let disk = if name == fd_core::program::PROGRAM_FILE {
            1
        } else {
            2
        };
        fd_tools::game_file(dir, &format!("disk{disk}/{name}"))
            .map_err(|e| fd_assets::AssetError::missing(name, e))
    };
    GameData::read(&files)
        .map(Arc::new)
        .map_err(|e| format!("game data from {}: {e}", dir.display()))
}

/// How a run is made: what a case file cannot choose.
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// Faithful unless a diagnosis asks for strict: a case cannot loosen its own bar.
    pub mode: Mode,
    /// The routines whose hits the coverage report counts: fd-core's `PORTED`.
    pub register: &'a [Ported],
    /// Where a picture difference's PNG goes (it shows the original, so outside git).
    pub diff_dir: Option<PathBuf>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            mode: Mode::Faithful,
            register: PORTED,
            diff_dir: None,
        }
    }
}

/// How a case came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every comparison held and the goal was reached.
    Same,
    /// The first variable that differs. `pass` counts from the engine's start (a title case's sync
    /// point is pass 0); `frame` is the oracle's frames run when it was read.
    Differs {
        pass: u32,
        frame: u32,
        var: String,
        engine: u32,
        oracle: u32,
    },
    /// A picture past the tolerance.
    PictureDiffers { pass: u32, frame: u32, why: String },
    /// The case compared nothing of what it says (porting-routines §4).
    ComparedNothing { what: String },
    /// Both sides agreed and missed the goal.
    GoalMissed { goal: String },
    /// An `idle` goal failed: an event, or a pass not compared.
    NotIdle { why: String },
    /// The engine met an object value nobody has ported (DESIGN §4.1): every case fails on it.
    Unported { pass: u32, frame: u32, value: u8 },
    /// An exclusion hid no difference over the whole run: it is no longer needed.
    ExclusionUnused { what: String, task: String },
}

/// A case's result and its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub outcome: Outcome,
    /// Passes whose variables were compared (a title case: frames, the sync point included).
    pub passes: u32,
    /// Frames whose picture was compared.
    pub pictures: u32,
    /// Passes the original ended unseen (two ends in one step), so not compared.
    pub unseen: u32,
    /// Frames tolerated as a colour step, as a few pixels, as a short glitch.
    pub tolerated: [u32; 3],
    /// Each exclusion and the frames on which its rows differed, or the passes on which its
    /// variable did.
    pub excluded: Vec<(Exclusion, u32)>,
    /// `(pass, event)` as the engine raised them.
    pub events: Vec<(u32, Event)>,
    /// The oracle frame at whose pass end the goal held.
    pub goal_met: Option<u32>,
    pub coverage: Coverage,
}

impl Verdict {
    fn new(register: &[Ported]) -> Verdict {
        Verdict {
            outcome: Outcome::Same,
            passes: 0,
            pictures: 0,
            unseen: 0,
            tolerated: [0; 3],
            excluded: Vec::new(),
            events: Vec::new(),
            goal_met: None,
            coverage: Coverage::new(register),
        }
    }

    pub fn passed(&self) -> bool {
        self.outcome == Outcome::Same
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Same => write!(f, "same"),
            Outcome::Differs {
                pass,
                frame,
                var,
                engine,
                oracle,
            } => write!(
                f,
                "differs at pass {pass} (oracle frame {frame}): {var} is {engine} ({engine:#x}) in \
                 the engine, {oracle} ({oracle:#x}) in the original"
            ),
            Outcome::PictureDiffers { pass, frame, why } => write!(
                f,
                "picture differs at pass {pass} (oracle frame {frame}): {why}"
            ),
            Outcome::ComparedNothing { what } => write!(f, "compared nothing: {what}"),
            Outcome::GoalMissed { goal } => {
                write!(f, "both sides agree but miss the goal: {goal}")
            }
            Outcome::NotIdle { why } => write!(f, "not idle: {why}"),
            Outcome::Unported { pass, frame, value } => write!(
                f,
                "unported object value {value:#04x} met at pass {pass} (oracle frame {frame}): \
                 no ObjectKind is registered for it"
            ),
            Outcome::ExclusionUnused { what, task } => write!(
                f,
                "exclusion `{what}` ({task}) hid no difference: remove it"
            ),
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.outcome {
            Outcome::Same => write!(
                f,
                "same on all {} passes compared, {} pictures",
                self.passes, self.pictures
            )?,
            ref other => write!(f, "{other}")?,
        }
        if self.unseen > 0 {
            write!(f, "\n{} pass ends unseen, not compared", self.unseen)?;
        }
        let [c, p, g] = self.tolerated;
        if c + p + g > 0 {
            write!(
                f,
                "\ntolerated: {c} frames a colour step off, {p} a few pixels, {g} glitches"
            )?;
        }
        for (e, n) in &self.excluded {
            write!(
                f,
                "\nexcluded {} ({}): differed on {n} {}",
                e.what,
                e.task,
                if e.var.is_some() { "passes" } else { "frames" }
            )?;
        }
        for (pass, e) in &self.events {
            write!(f, "\nevent at pass {pass}: {e:?}")?;
        }
        if let Some(fr) = self.goal_met {
            write!(f, "\ngoal reached at oracle frame {fr}")?;
        }
        write!(f, "\n{}", self.coverage)
    }
}

/// Run `case` on the real disks and Kickstart. Every missing input is an error naming its path, and
/// the cheap checks come first so a wrong setup fails before any emulation.
pub fn run(case: &Case) -> Result<Verdict, String> {
    if case.is_level() {
        let mut game = Game::new(game_data()?, Start::Level1);
        return run_with(case, &mut game, &Options::default());
    }
    run_title(case, &Options::default())
}

/// A script under `engine/conform/scripts/`.
fn load_script(name: &str) -> Result<script::InputScript, String> {
    let path = conform_dir().join("scripts").join(format!("{name}.txt"));
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("script {}: {e}", path.display()))?;
    script::parse(&text).map_err(|e| format!("script {}: {e}", path.display()))
}

/// The input of the step that starts at oracle frame `frame`: the sync script's before the sync
/// point, and after it the case's `input` (counted from the sync frame) or the sync script's.
pub struct Inputs {
    sync: script::InputScript,
    after: Option<script::InputScript>,
    sync_frame: u32,
}

impl Inputs {
    pub fn load(case: &Case) -> Result<Inputs, String> {
        Ok(Inputs {
            sync: load_script(&case.script)?,
            after: case.input.as_deref().map(load_script).transpose()?,
            sync_frame: case.sync_frame,
        })
    }

    pub fn at(&self, frame: u32) -> FrameInput {
        match &self.after {
            Some(after) if frame >= self.sync_frame => {
                script::input_at(&after.lines, frame - self.sync_frame)
            }
            _ => script::input_at(&self.sync.lines, frame),
        }
    }
}

/// The title scene's value of `var`. Each title variable needs its arm here.
fn title_value(scene: &TitleScene, var: &Var) -> u32 {
    match var.name {
        "mt_counter" => scene.variable() as u32,
        other => panic!("{other} is not modelled by the title scene"),
    }
}

fn layout_vars(names: &[String]) -> Result<Vec<&'static Var>, String> {
    names
        .iter()
        .map(|v| layout_var(v).ok_or_else(|| format!("`{v}` is not in fd-core's LAYOUT")))
        .collect()
}

/// forest-dumb-start's runner: power-on to the sync frame, then the title scene and the original a
/// frame at a time, every variable each frame, the sync point included.
fn run_title(case: &Case, opts: &Options) -> Result<Verdict, String> {
    let inputs = Inputs::load(case)?;
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disks = disks::load()?;
    let vars = layout_vars(&case.vars)?;

    let cap = case.sync_frame + case.frames;
    let cfg = OracleConfig::new(kickstart, disks, cap);
    let mut amiga = Amiga::power_on(&cfg).map_err(|e| e.to_string())?;
    let step = |amiga: &mut Amiga, frame: u32| {
        let input = inputs.at(frame);
        amiga.step(input).map_err(|e| e.to_string())?;
        Ok::<_, String>(input)
    };
    for frame in 0..case.sync_frame {
        if let Some(swap) = case.swap.filter(|s| s.frame == frame) {
            amiga.insert_disk(0, swap.disk as usize - 1);
        }
        step(&mut amiga, frame)?;
    }
    let pcs: Vec<u32> = opts.register.iter().map(coverage::address).collect();
    amiga.watch_pcs(&pcs);
    let mut verdict = Verdict::new(opts.register);
    let mut scene = TitleScene::new();
    for pass in 0..=case.frames {
        let frame = case.sync_frame + pass;
        if pass > 0 {
            let input = step(&mut amiga, frame - 1)?;
            verdict.coverage.add(opts.register, amiga.pc_hits());
            scene.frame(input);
        }
        for v in &vars {
            let (engine, oracle) = (title_value(&scene, v), Live(&amiga).value(v));
            if engine != oracle {
                verdict.outcome = Outcome::Differs {
                    pass,
                    frame,
                    var: v.name.to_string(),
                    engine,
                    oracle,
                };
                return Ok(verdict);
            }
        }
        verdict.passes += 1;
    }
    Ok(verdict)
}

/// The original at a level case's sync frame, frozen as the case asks, with room for its frames
/// and `slack` more. From the cached level sync point, then the sync script up to the case's frame.
pub fn reach_case(case: &Case, slack: u32) -> Result<Amiga, String> {
    if !case.is_level() {
        return Err(format!("{}: not a level case", case.name));
    }
    let inputs = Inputs::load(case)?;
    let lead = case.sync_frame - sync::LEVEL_1_SYNC;
    let mut am = sync::reach_level_1(lead + case.frames + slack)?;
    for frame in sync::LEVEL_1_SYNC..case.sync_frame {
        am.step(inputs.at(frame)).map_err(|e| e.to_string())?;
    }
    if case.freeze {
        freeze::apply(&mut am, freeze::FROZEN)?;
    }
    Ok(am)
}

/// Whether any pixel in `rows` (clipped to both pictures) differs. `ours` is row-major colours of
/// a picture `width` wide.
fn differs_in(ours: &[u16], width: u16, theirs: &Rgb12Frame, rows: &Range<u16>) -> bool {
    let height = (ours.len() / width.max(1) as usize) as u16;
    let (w, h) = (width.min(theirs.width), height.min(theirs.height));
    (rows.start..rows.end.min(h)).any(|y| {
        (0..w).any(|x| ours[y as usize * width as usize + x as usize] != theirs.pixel(x, y))
    })
}

/// Run a level case with `engine` on the engine's side.
///
/// The original's passes are numbered by its own `irq_count`, which `level3_handler` raises as a
/// pass starts: pass `n`'s input is the input of the step in which the count reached `n`, and its
/// state is RAM at the pass's end ([`PASS_END`], `Amiga::watch_write`), captured in whichever step
/// the end falls. The engine is transplanted at the first pass end after the sync point and then
/// runs each later pass on that pass's input, so it is compared with the original pass for pass
/// whatever step a pass ends in. A step that ends two passes shows only the first; the other is
/// counted in [`Verdict::unseen`]. The picture of pass `n` is the oracle frame of the step its end
/// was seen in. `amiga_conform::compare::PassMatch` is not used: it pairs ends step by step, and
/// here the original numbers its own passes.
pub fn run_with(case: &Case, engine: &mut dyn Engine, opts: &Options) -> Result<Verdict, String> {
    if !case.is_level() {
        return Err(format!(
            "{}: run_with runs level cases; a title case runs with `run`",
            case.name
        ));
    }
    let inputs = Inputs::load(case)?;
    let vars = layout_vars(&case.vars)?;
    let goal: Vec<(&Var, u32)> = match &case.goal {
        Some(Goal::All { all, .. }) => all
            .iter()
            .map(|(v, value)| Ok((layout_vars(std::slice::from_ref(v))?[0], *value)))
            .collect::<Result<_, String>>()?,
        _ => Vec::new(),
    };
    let irq = layout_var("irq_count").ok_or("no irq_count in LAYOUT")?;
    let mut am = reach_case(case, 0)?;
    let pcs: Vec<u32> = opts.register.iter().map(coverage::address).collect();
    am.watch_pcs(&pcs);
    am.watch_write(Some(PASS_END));

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    let rows: Vec<Range<u16>> = case
        .exclude
        .iter()
        .filter(|e| e.var.is_none())
        .map(|e| e.rows.clone())
        .collect();
    // (index in `excluded`, the variable) for each excluded variable.
    let excluded_vars: Vec<(usize, &Var)> = case
        .exclude
        .iter()
        .enumerate()
        .filter_map(|(i, e)| Some((i, layout_var(e.var.as_deref()?)?)))
        .collect();
    let tol = Tolerance::OWNER;
    let mut judge = PictureJudge::new(tol, case.frames);
    let mut started = Live(&am).value(irq);
    // (pass, its input) for passes started on the original and not yet run on the engine.
    let mut pending: Vec<(u32, FrameInput)> = Vec::new();
    // (the pass the engine was transplanted at, the last pass it ran).
    let mut at: Option<(u32, u32)> = None;
    let mut passes_run = 0u32;

    for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        am.step(input).map_err(|e| e.to_string())?;
        let frame = am.frame();
        verdict.coverage.add(opts.register, am.pc_hits());
        // irq_count wraps at $10000, far past any level case.
        let now = Live(&am).value(irq);
        pending.extend((started + 1..=now).map(|n| (n, input)));
        started = now;
        let Some(mem) = am.watched_memory() else {
            continue;
        };
        let theirs = OracleMem(&mem);
        let ended = theirs.value(irq);
        let Some((first, last)) = at else {
            engine.transplant(&theirs);
            at = Some((ended, ended));
            pending.retain(|(n, _)| *n > ended);
            continue;
        };
        if ended <= last {
            continue;
        }
        for n in last + 1..=ended {
            let i = pending
                .iter()
                .position(|(p, _)| *p == n)
                .ok_or_else(|| format!("pass {n} ended on the original but never started"))?;
            let (_, pass_input) = pending.remove(i);
            engine.pass(pass_input);
            passes_run += 1;
            for e in engine.events() {
                verdict.events.push((n - first, e));
                if let Event::Unported(value) = e {
                    // Every case fails on an object nobody ported (DESIGN §4.1).
                    verdict.outcome = Outcome::Unported {
                        pass: n - first,
                        frame,
                        value,
                    };
                    return Ok(verdict);
                }
            }
        }
        verdict.unseen += ended - last - 1;
        at = Some((first, ended));
        let pass = ended - first;

        if case.compare.vars {
            for v in &vars {
                let (ours, theirs) = (engine.probe(v), theirs.value(v));
                if ours != theirs {
                    verdict.outcome = Outcome::Differs {
                        pass,
                        frame,
                        var: v.name.to_string(),
                        engine: ours,
                        oracle: theirs,
                    };
                    return Ok(verdict);
                }
            }
            for &(i, v) in &excluded_vars {
                if engine.probe(v) != theirs.value(v) {
                    verdict.excluded[i].1 += 1;
                }
            }
            verdict.passes += 1;
        }
        if case.compare.picture {
            let ours = engine
                .picture()
                .ok_or("the case compares the picture but the engine draws none")?;
            let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
            let colours = to_rgb12(&ours);
            for (i, e) in case.exclude.iter().enumerate() {
                if e.var.is_none() && differs_in(&colours, ours.width, &pic, &e.rows) {
                    verdict.excluded[i].1 += 1;
                }
            }
            let class = classify(&ours, &pic, &rows, &tol);
            let failed = match (opts.mode, class) {
                (_, PictureClass::Same) => None,
                (Mode::Strict, other) => Some(format!("{other:?}, strict")),
                (Mode::Faithful, other) => judge.frame(frame, other).err(),
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
                verdict.outcome = Outcome::PictureDiffers { pass, frame, why };
                return Ok(verdict);
            }
        }
        if let Some(Goal::All { by_frame, .. }) = &case.goal {
            let reached =
                |v: &Var, value: u32| engine.probe(v) == value && theirs.value(v) == value;
            if verdict.goal_met.is_none()
                && frame <= *by_frame
                && goal.iter().all(|(v, value)| reached(v, *value))
            {
                verdict.goal_met = Some(frame);
            }
        }
    }

    let nothing = |what: &str| {
        Some(Outcome::ComparedNothing {
            what: what.to_string(),
        })
    };
    let failed = if at.is_none() {
        nothing("no pass of the original ended after the sync point")
    } else if case.compare.vars && verdict.passes == 0 {
        nothing("no pass ended on both sides, so no variable was compared")
    } else if case.compare.picture && verdict.pictures == 0 {
        nothing("no frame's picture was compared")
    } else {
        match &case.goal {
            Some(Goal::All { all, by_frame }) if verdict.goal_met.is_none() => {
                let want: Vec<String> = all.iter().map(|(v, n)| format!("{v} = {n}")).collect();
                Some(Outcome::GoalMissed {
                    goal: format!("{} by oracle frame {by_frame}", want.join(", ")),
                })
            }
            Some(Goal::Idle) => if let Some((pass, e)) = verdict.events.first() {
                Some(format!("the engine raised `{e:?}` at pass {pass}"))
            } else if verdict.unseen > 0 || verdict.passes != passes_run {
                Some(format!(
                    "{} of {passes_run} passes compared: every pass must be",
                    verdict.passes
                ))
            } else {
                None
            }
            .map(|why| Outcome::NotIdle { why }),
            _ => None,
        }
    };
    // Every exclusion names the task that removes it, and fails once it hides nothing
    // (porting-routines §4).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variable_resolves_to_the_address_the_map_records() {
        // docs/07-executable-map.md: "one byte, `$66c31` on the oracle".
        assert_eq!(address("mt_counter"), Ok(0x66c31));
        assert_eq!(address("char_x"), Ok(0xc0_77d8 + 0x1e5a));
    }

    #[test]
    fn a_difference_names_the_pass_the_frame_and_the_two_values() {
        let text = Outcome::Differs {
            pass: 3,
            frame: 4925,
            var: "char_x".into(),
            engine: 0x71,
            oracle: 0x70,
        }
        .to_string();
        for part in [
            "pass 3",
            "4925",
            "char_x",
            "113 (0x71) in the engine",
            "112 (0x70) in the original",
        ] {
            assert!(text.contains(part), "{text}");
        }
    }

    #[test]
    fn oracle_memory_reads_chip_and_slow_ram_by_hunk() {
        let mut m = AmigaMemory {
            chip: vec![0; 0x8_0000],
            slow: vec![0; 0x8_0000],
        };
        m.slow[0x77d8 + 0x1e5a..][..4].copy_from_slice(&[0, 0, 0x12, 0x34]);
        m.chip[0x4_d668 + 0x195c9] = 5;
        let mem = OracleMem(&m);
        assert_eq!(mem.value(layout_var("char_x").unwrap()), 0x1234);
        assert_eq!(mem.value(layout_var("mt_counter").unwrap()), 5);
    }
}
