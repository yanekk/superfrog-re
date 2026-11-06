//! Shell, test tooling: Forest Dumb Forever's conformance runner. A case brings the original on the
//! oracle to a sync point, starts the engine there, feeds both the same input and compares the
//! modelled variables and the picture (plans/forest-dumb-level-1 DESIGN §2.1, §4, §4.1). The format
//! is in `engine/conform/README.md`.
//!
//! Two runners share the case format. A title case (forest-dumb-start's first case) steps the
//! title scene once a frame from power-on. A level case starts at the level sync point
//! (`sync.rs`) and compares at the end of each pass of `game_frame`: in play a pass usually ends
//! after the frame does, so the state at a frame's end is mid-pass (docs/09-play-measured.md).

pub mod bug;
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

/// `game_over_screen` (`h0+$4c4`): `main_loop` has found the game over; no pass runs after it.
pub const GAME_OVER_SCREEN: (u8, u32) = (0, 0x4c4);
/// The high scores (`h0+$89d0`), after the game over screen: where the engine restarts the level
/// instead (DESIGN §2.4), so where a comparison ends.
pub const HIGH_SCORES: (u8, u32) = (0, 0x89d0);

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
    /// A step the original spends on the game over screen, where no pass runs: the engine steps on
    /// the step's input (T23). An engine without the screen does nothing.
    fn off_play(&mut self, _input: FrameInput) {}
    /// Whether the engine shows the game over screen.
    fn game_over(&self) -> bool {
        false
    }
}

/// The level engine, fd-core's [`Game`], as a case runs it: one [`Game::step`] a pass, and its
/// picture ([`Game::picture`], T10) as the pass leaves the display.
impl Engine for Game {
    fn pass(&mut self, input: FrameInput) {
        self.step(input);
    }

    fn picture(&self) -> Option<Picture> {
        Some(Game::picture(self))
    }

    fn events(&mut self) -> Vec<Event> {
        self.take_events()
    }

    fn off_play(&mut self, input: FrameInput) {
        self.step(input);
    }

    fn game_over(&self) -> bool {
        matches!(self.scene(), fd_core::Scene::GameOver(_))
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
    /// One side went to the game over screen, or left it, and the other did not (T23).
    SceneDiffers { frame: u32, why: String },
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
    /// The oracle frames after whose steps both sides showed the game over screen and both left it
    /// for the high scores (T23); the comparison ends at the second.
    pub game_over: Option<(u32, Option<u32>)>,
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
            game_over: None,
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
            Outcome::SceneDiffers { frame, why } => {
                write!(f, "scene differs at oracle frame {frame}: {why}")
            }
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
        if let Some((from, to)) = self.game_over {
            write!(f, "\ngame over screen from oracle frame {from}")?;
            if let Some(to) = to {
                write!(
                    f,
                    "; the high scores start at {to}, where the engine restarts: compared up to there"
                )?;
            }
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

    /// The case's sync script, and `after` from its sync frame on: inputs held in memory.
    pub fn with_after(case: &Case, after: script::InputScript) -> Result<Inputs, String> {
        Ok(Inputs {
            sync: load_script(&case.script)?,
            after: Some(after),
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
    reach_case_with(case, slack, |_, _| Ok(()))
}

/// [`reach_case`], calling `each` once at the level sync point with `None` and after every step
/// up to the case's frame with the input that step ran on.
fn reach_case_with(
    case: &Case,
    slack: u32,
    mut each: impl FnMut(&mut Amiga, Option<FrameInput>) -> Result<(), String>,
) -> Result<Amiga, String> {
    if !case.is_level() {
        return Err(format!("{}: not a level case", case.name));
    }
    let inputs = Inputs::load(case)?;
    let lead = case.sync_frame - sync::LEVEL_1_SYNC;
    let mut am = sync::reach_level_1(lead + case.frames + slack)?;
    each(&mut am, None)?;
    for frame in sync::LEVEL_1_SYNC..case.sync_frame {
        let input = inputs.at(frame);
        am.step(input).map_err(|e| e.to_string())?;
        each(&mut am, Some(input))?;
    }
    if case.freeze {
        freeze::apply(&mut am, freeze::FROZEN)?;
    }
    Ok(am)
}

/// `read_joystick`, `h0+$8dfc`: `game_frame` reads JOY1DAT here once a pass while not paused.
pub const READ_JOYSTICK: (u8, u32) = (0, 0x8dfc);
/// `game_frame`'s paused branch, `h0+$200e` (`bsr.w play_keys` after `tst.w paused`): a paused
/// pass runs this and no `read_joystick`.
pub const PAUSED_BRANCH: (u8, u32) = (0, 0x200e);
/// `read_joystick` stops counting `input_log_index` here (`cmpi.l #$2ee0`).
const INPUT_LOG_FULL: u32 = 12000;

/// The input each pass of the original read, pass by pass, from T31's counter (T67).
///
/// Every pass of `game_frame` runs exactly one of [`READ_JOYSTICK`] and [`PAUSED_BRANCH`], and
/// passes run one after another (`level3_handler` is not re-entered), so the hits of the two, in
/// step order, are the passes' in pass order: the `k`th hit after the anchor is pass `anchor + k`'s,
/// whatever step that pass started in. While the camera scrolls a pass usually reads in the step
/// after its start (57 of 76 slide passes, T41), so the step a pass started in is not the one whose
/// joystick it saw.
///
/// A pass's keys are the keys of the step it started in (measured, T67: P held from step `N`, idle
/// or sliding, reached `pause_key_down` in the pass started in step `N`, whose `play_keys` ran in
/// step `N + 1` when sliding; `level2_handler` puts the byte in `last_key_byte` before that step's
/// level-3 interrupt). A paused pass reads no joystick: it gets the joystick of the step its paused
/// branch ran in, which nothing reads.
pub struct PassInputs {
    /// `irq_count` at the last step's end: passes up to it have started.
    started: u32,
    /// The last pass whose `game_frame` read the joystick or took the paused branch.
    decided: u32,
    /// `input_log_index` as the counter says it must be.
    log: u32,
    /// (pass, the input of the step it started in) for passes started and not yet decided.
    starts: Vec<(u32, FrameInput)>,
    /// (pass, its input) for passes decided and not yet taken.
    ready: Vec<(u32, FrameInput)>,
    pcs: [u32; 2],
}

impl PassInputs {
    /// Anchored at the original as it is now. It holds only where every pass so far read the
    /// joystick, the last started one included: `input_log_index` equals `irq_count` and nothing
    /// is paused. W1-1's sync point is such a place (measured: both 1).
    pub fn anchor(am: &Amiga) -> Result<PassInputs, String> {
        let var = |n: &str| layout_var(n).ok_or_else(|| format!("no {n} in LAYOUT"));
        let started = Live(am).value(var("irq_count")?);
        let log = Live(am).value(var("input_log_index")?);
        let paused = Live(am).value(var("paused")?);
        if log != started || paused != 0 {
            return Err(format!(
                "cannot anchor the input pairing: irq_count {started}, input_log_index {log}, \
                 paused {paused}; it needs every pass so far to have read the joystick"
            ));
        }
        Ok(PassInputs {
            started,
            decided: started,
            log,
            starts: Vec::new(),
            ready: Vec::new(),
            pcs: [READ_JOYSTICK, PAUSED_BRANCH].map(|(h, o)| HUNK_BASES[h as usize] + o),
        })
    }

    /// The addresses [`Amiga::watch_pcs`] must count for [`PassInputs::step`].
    pub fn pcs(&self) -> [u32; 2] {
        self.pcs
    }

    /// Account for the step just run on `input`. The counter must be watching [`PassInputs::pcs`].
    /// `input_log_index` checks the count: a read the counter missed, or one it saw twice, is an
    /// error, never a silent mispairing.
    pub fn step(&mut self, am: &Amiga, input: FrameInput) -> Result<(), String> {
        let var = |n: &str| layout_var(n).ok_or_else(|| format!("no {n} in LAYOUT"));
        // irq_count wraps at $10000, far past any level case.
        let now = Live(am).value(var("irq_count")?);
        self.starts
            .extend((self.started + 1..=now).map(|n| (n, input)));
        self.started = now;
        let hits = |pc: u32| {
            am.pc_hits()
                .iter()
                .find(|(p, _)| *p == pc)
                .map_or(0, |(_, n)| *n)
        };
        let (reads, paused) = (hits(self.pcs[0]), hits(self.pcs[1]));
        for _ in 0..reads + paused {
            self.decided += 1;
            let n = self.decided;
            let i = self
                .starts
                .iter()
                .position(|(p, _)| *p == n)
                .ok_or_else(|| format!("pass {n} read its input before it started"))?;
            let (_, start) = self.starts.remove(i);
            self.ready.push((
                n,
                FrameInput {
                    joy: input.joy,
                    keys: start.keys,
                },
            ));
        }
        self.log = (self.log + reads).min(INPUT_LOG_FULL);
        let log = Live(am).value(var("input_log_index")?);
        // A read that started just before the step's end counts at `h0+$8dfc` in this step but adds
        // to `input_log_index` (`h0+$8eb4`) in the next: one behind is allowed only in a step that
        // counted a read, and the next step's check holds it to the count (T15, measured at frame
        // 7715 of a long varied recording, the character dying).
        let reading = reads > 0 && log + 1 == self.log && self.log < INPUT_LOG_FULL;
        if log != self.log && !reading {
            return Err(format!(
                "input_log_index is {log} on the original, the counter says {}: a joystick read \
                 was missed or counted twice",
                self.log
            ));
        }
        Ok(())
    }

    /// Forget passes up to `n`: the engine starts after them.
    pub fn drop_through(&mut self, n: u32) {
        self.ready.retain(|(p, _)| *p > n);
        self.starts.retain(|(p, _)| *p > n);
    }

    /// The pass whose `game_frame` reads the joystick next: the input fed in the next step is
    /// its joystick, if it reads in that step.
    pub fn next_read(&self) -> u32 {
        self.decided + 1
    }

    /// The pass the next step starts, if it starts one: its keys are that step's.
    pub fn next_start(&self) -> u32 {
        self.started + 1
    }

    /// Pass `n`'s input, once its `game_frame` has read it.
    pub fn take(&mut self, n: u32) -> Option<FrameInput> {
        let i = self.ready.iter().position(|(p, _)| *p == n)?;
        Some(self.ready.remove(i).1)
    }
}

/// [`reach_case`] with its passes' inputs counted from the level sync point, and the counter
/// watching `extra` as well (the coverage register), counts reset at the case's sync point.
pub fn reach_case_counting(
    case: &Case,
    slack: u32,
    extra: &[u32],
) -> Result<(Amiga, PassInputs), String> {
    let mut tracker: Option<PassInputs> = None;
    let am = reach_case_with(case, slack, |am, input| {
        match (input, &mut tracker) {
            (None, _) => {
                let t = PassInputs::anchor(am)?;
                let mut pcs = extra.to_vec();
                pcs.extend(t.pcs());
                am.watch_pcs(&pcs);
                tracker = Some(t);
            }
            (Some(input), Some(t)) => t.step(am, input)?,
            (Some(_), None) => unreachable!("anchored before the first step"),
        }
        Ok(())
    })?;
    Ok((am, tracker.expect("anchored at the sync point")))
}

/// Whether any pixel in exclusion `e`'s rows (and columns, for a box), clipped to both pictures,
/// differs. `ours` is row-major colours of a picture `width` wide.
fn differs_in(ours: &[u16], width: u16, theirs: &Rgb12Frame, e: &Exclusion) -> bool {
    let height = (ours.len() / width.max(1) as usize) as u16;
    let (w, h) = (width.min(theirs.width), height.min(theirs.height));
    let cols = e.cols.clone().unwrap_or(0..w);
    (e.rows.start..e.rows.end.min(h)).any(|y| {
        (cols.start..cols.end.min(w))
            .any(|x| ours[y as usize * width as usize + x as usize] != theirs.pixel(x, y))
    })
}

/// The original's frame with the engine's colours in every excluded box, so a box's differences
/// are left out of the judgement and the rest of its rows is still compared.
fn with_boxes_ours(
    theirs: &Rgb12Frame,
    ours: &[u16],
    width: u16,
    exclude: &[Exclusion],
) -> Rgb12Frame {
    let mut f = theirs.clone();
    let height = (ours.len() / width.max(1) as usize) as u16;
    let (w, h) = (width.min(f.width), height.min(f.height));
    for e in exclude.iter().filter(|e| e.var.is_none()) {
        let Some(cols) = &e.cols else { continue };
        for y in e.rows.start..e.rows.end.min(h) {
            for x in cols.start..cols.end.min(w) {
                f.pixels[y as usize * f.width as usize + x as usize] =
                    ours[y as usize * width as usize + x as usize];
            }
        }
    }
    f
}

const NO_PICTURE: &str = "the case compares the picture but the engine draws none";

/// Run a level case with `engine` on the engine's side.
///
/// The original's passes are numbered by its own `irq_count`, which `level3_handler` raises as a
/// pass starts: pass `n`'s joystick is the input of the step in which its `read_joystick` ran, and
/// its keys that of the step in which the count reached `n` ([`PassInputs`], T67); its
/// state is RAM at the pass's end ([`PASS_END`], `Amiga::watch_write`), captured in whichever step
/// the end falls. The engine is transplanted at the first pass end after the sync point and then
/// runs each later pass on that pass's input, so it is compared with the original pass for pass
/// whatever step a pass ends in. A step that ends two passes shows only the first; the other is
/// counted in [`Verdict::unseen`]. The picture of pass `n` is the oracle frame of the step its end
/// was seen in. `amiga_conform::compare::PassMatch` is not used: it pairs ends step by step, and
/// here the original numbers its own passes.
pub fn run_with(case: &Case, engine: &mut dyn Engine, opts: &Options) -> Result<Verdict, String> {
    run_inputs(case, &Inputs::load(case)?, engine, opts)
}

/// [`run_with`] on `inputs` rather than the scripts the case names: a bug zip's retimed inputs
/// (`bug::compare`), which are no file. The way to the sync point is still the case's.
pub fn run_inputs(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn Engine,
    opts: &Options,
) -> Result<Verdict, String> {
    if !case.is_level() {
        return Err(format!(
            "{}: run_with runs level cases; a title case runs with `run`",
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
    let irq = layout_var("irq_count").ok_or("no irq_count in LAYOUT")?;
    let mut pcs: Vec<u32> = opts.register.iter().map(coverage::address).collect();
    let scene_pcs = [GAME_OVER_SCREEN, HIGH_SCORES].map(|(h, o)| HUNK_BASES[h as usize] + o);
    pcs.extend(scene_pcs);
    let (mut am, mut read) = reach_case_counting(case, 0, &pcs)?;
    am.watch_write(Some(PASS_END));

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    // Whole excluded rows; a box is left out by [`with_boxes_ours`].
    let rows: Vec<Range<u16>> = case
        .exclude
        .iter()
        .filter(|e| e.var.is_none() && e.cols.is_none())
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
    // (the pass the engine was transplanted at, the last pass it ran).
    let mut at: Option<(u32, u32)> = None;
    let mut passes_run = 0u32;

    // The engine's picture after each pass it has run, until the frame showing it is drawn.
    let mut drawn: Vec<(u32, Picture)> = Vec::new();
    // The last pass whose picture was compared.
    let mut shown_last: Option<u32> = None;

    for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        am.step(input).map_err(|e| e.to_string())?;
        let frame = am.frame();
        verdict.coverage.add(opts.register, am.pc_hits());
        read.step(&am, input)?;
        if let Some(mem) = am.watched_memory() {
            let theirs = OracleMem(&mem);
            let ended = theirs.value(irq);
            match at {
                None => {
                    engine.transplant(&theirs);
                    at = Some((ended, ended));
                    read.drop_through(ended);
                    if case.compare.picture {
                        drawn.push((ended, engine.picture().ok_or(NO_PICTURE)?));
                    }
                }
                Some((first, last)) if ended > last => {
                    for n in last + 1..=ended {
                        let pass_input = read.take(n).ok_or_else(|| {
                            format!("pass {n} ended on the original but never read its input")
                        })?;
                        engine.pass(pass_input);
                        passes_run += 1;
                        if case.compare.picture {
                            drawn.push((n, engine.picture().ok_or(NO_PICTURE)?));
                        }
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
                    if let Some(Goal::All { by_frame, .. }) = &case.goal {
                        let reached = |v: &Var, value: u32| {
                            engine.probe(v) == value && theirs.value(v) == value
                        };
                        if verdict.goal_met.is_none()
                            && frame <= *by_frame
                            && goal.iter().all(|(v, value)| reached(v, *value))
                        {
                            verdict.goal_met = Some(frame);
                        }
                    }
                }
                Some(_) => {}
            }
        }

        // After the pass ends: pass 32k can end in the step `main_loop` then finds the game over in.
        let ran = |pc: u32| am.pc_hits().iter().any(|&(p, n)| p == pc && n > 0);
        let (entered, scores) = (ran(scene_pcs[0]), ran(scene_pcs[1]));
        if verdict.game_over.is_some() || entered {
            // The game over screen (T23): no pass runs; the engine steps on the step's input and
            // must enter it, stay on it and leave it in the steps the original does.
            let first = verdict.game_over.is_none();
            if first && engine.game_over() {
                return Ok(scene_differs(
                    verdict,
                    frame,
                    "the engine went to the game over screen a step before the original",
                ));
            }
            engine.off_play(input);
            for e in engine.events() {
                verdict.events.push((at.map_or(0, |(f, l)| l - f), e));
            }
            if first {
                verdict.game_over = Some((frame, None));
            }
            if engine.game_over() == scores {
                let why = if scores {
                    "the original left the game over screen for the high scores; the engine did not"
                } else if first {
                    "the original went to the game over screen; the engine did not"
                } else {
                    "the engine left the game over screen; the original did not"
                };
                return Ok(scene_differs(verdict, frame, why));
            }
            if scores {
                verdict.game_over = verdict.game_over.map(|(from, _)| (from, Some(frame)));
                break;
            }
            // The step that entered the screen draws play above the copper's switch and the
            // screen below it: its frame is not compared.
            if case.compare.picture && !first {
                let ours = engine.picture().ok_or(NO_PICTURE)?;
                let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
                verdict.pictures += 1;
                let class = classify(&ours, &pic, &[], &tol);
                if class != PictureClass::Same {
                    let mut why = format!("the game over screen: {class:?}");
                    if let (Some(dir), Some(m)) = (&opts.diff_dir, picture_except(&ours, &pic, &[]))
                    {
                        let d = report::write_diff_png(dir, &case.name, frame, &ours, &pic, m);
                        why += &format!("; {}", d.path.display());
                    }
                    verdict.outcome = Outcome::PictureDiffers {
                        pass: at.map_or(0, |(f, l)| l - f),
                        frame,
                        why,
                    };
                    return Ok(verdict);
                }
            }
            continue;
        } else if scores {
            return Ok(scene_differs(
                verdict,
                frame,
                "the original entered its high scores without its game over screen",
            ));
        }
        // The frame this step drew shows the pass that started at its vertical blank: the last
        // pass started by the step's end less one (measured, T44: every pixel of scrolled frames
        // agrees so, and the frame of the step a pass ended in is a pass behind once passes run
        // past the frame's end). A pass no frame shows (a step starting two) is not compared.
        if !case.compare.picture {
            continue;
        }
        let Some((first, _)) = at else { continue };
        let shown = Live(&am).value(irq).wrapping_sub(1);
        if shown_last.is_some_and(|s| shown <= s) {
            continue;
        }
        let Some(k) = drawn.iter().position(|(n, _)| *n == shown) else {
            continue;
        };
        shown_last = Some(shown);
        let ours = drawn.swap_remove(k).1;
        drawn.retain(|(n, _)| *n > shown);
        let pass = shown - first;
        let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
        let colours = to_rgb12(&ours);
        for (i, e) in case.exclude.iter().enumerate() {
            if e.var.is_none() && differs_in(&colours, ours.width, &pic, e) {
                verdict.excluded[i].1 += 1;
            }
        }
        let theirs = with_boxes_ours(&pic, &colours, ours.width, &case.exclude);
        let class = classify(&ours, &theirs, &rows, &tol);
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
            let shown = picture_except(&ours, &theirs, &rows);
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

/// A scene difference ends the run.
fn scene_differs(mut verdict: Verdict, frame: u32, why: &str) -> Verdict {
    verdict.outcome = Outcome::SceneDiffers {
        frame,
        why: why.to_string(),
    };
    verdict
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
