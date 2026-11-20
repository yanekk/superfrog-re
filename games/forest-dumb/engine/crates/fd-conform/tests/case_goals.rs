//! The `enters` and `exited` goals (title T02, DESIGN §2.3) on the runner of cases lined up by loads,
//! `fd_conform::frames::run_entry_frames`. Full: the owner's disks and Kickstart, the original
//! from power-on twice.
//!
//! The stand-in is a second copy of the original, started where the runner's engine starts (the
//! end of the sync frame's step, or the entry's step from power-on), that reports a routine's entry
//! when its first instruction runs and its exit when it reaches the address its entry returns to,
//! read off its own entry's step or handed over by the runner as the translated program's is
//! (`FrameEngine::exit_to`). Meddled, it reports the event a frame late, never, or as a stop.
//!
//! `load_world` stands for `title_screen` in the `enters` tests: Start at frame 5000 enters it a few
//! frames after the snapshot, where `title_screen` is already running. The `exited` tests pick
//! KONIEC (`scripts/case-goals-koniec.txt`): the original quits in step 5073.

use amiga_oracle::{Amiga, FrameInput, OracleConfig, PaulaWrite, Rgb12Frame, kickstart_path};
use fd_conform::frames::{FrameEngine, SCREEN_WINDOW, run_entry_frames};
use fd_conform::sync::{self, LOAD_RETURNS, LOAD_ROUTINES};
use fd_conform::{Case, HUNK_BASES, Inputs, Options, Outcome, Verdict, disks};
use fd_conform::translated_engine::run_translated;
use fd_core::{Mem, Transplant, Var};
use fd_translated::TranslatedGame;

mod common;
use common::picture_of;

/// How the stand-in reports the goal's event.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Report {
    /// In the frame it happened.
    AsIs,
    /// A frame after it happened.
    Late,
    /// Never.
    Never,
    /// Its exit as a stop that is not the exit: the translated program's jump to an address
    /// with no translation.
    AsStop,
}

struct StandIn {
    am: Amiga,
    case: Case,
    report: Report,
    /// No step yet: a power-on stand-in is at the end of the entry's step, as the runner's.
    first: bool,
    /// The PC of the watched routine's entry, and of the exit.
    entry_pc: Option<u32>,
    exit_pc: Option<u32>,
    /// The event happened in the frame before this one and is held a frame (`Late`).
    held: bool,
    entered: bool,
    exited: bool,
    stop: Option<String>,
    load_entered: u32,
    load_returned: u32,
    loads: (u32, bool),
    ram: Vec<u8>,
    pic: Option<Rgb12Frame>,
    paula: Vec<PaulaWrite>,
    custom: Vec<(u16, u16)>,
}

fn h((hunk, off): (u8, u32)) -> u32 {
    HUNK_BASES[hunk as usize] + off
}

fn load_pcs() -> Vec<u32> {
    LOAD_ROUTINES
        .iter()
        .chain(LOAD_RETURNS.iter())
        .copied()
        .map(h)
        .collect()
}

impl StandIn {
    /// The original as the runner's engine starts: at the end of the sync frame's step, or of the
    /// entry's step from power-on, with its own exit read there.
    fn new(case: &Case, report: Report) -> StandIn {
        let inputs = Inputs::load(case).unwrap();
        let (am, exit_pc) = if case.is_entry() {
            let (am, _, exit) =
                sync::reach_entry(case, &inputs, &load_pcs()).unwrap_or_else(|e| panic!("{e}"));
            (am, exit.map(|(back, _)| back))
        } else {
            let kickstart = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
            let cfg = OracleConfig::new(
                kickstart,
                disks::load().unwrap(),
                case.sync_frame + case.frames + 1,
            );
            let mut am = Amiga::power_on(&cfg).unwrap();
            for frame in 0..case.sync_frame {
                if let Some(s) = case.swap.filter(|s| s.frame == frame) {
                    am.insert_disk(0, s.disk as usize - 1);
                }
                am.step(inputs.at(frame)).unwrap();
            }
            (am, None)
        };
        let mut s = StandIn {
            am,
            case: case.clone(),
            report,
            first: case.is_entry(),
            entry_pc: None,
            exit_pc,
            held: false,
            entered: false,
            exited: false,
            stop: None,
            load_entered: 0,
            load_returned: 0,
            loads: (0, false),
            ram: Vec::new(),
            pic: None,
            paula: Vec::new(),
            custom: Vec::new(),
        };
        s.watch();
        s
    }

    fn watch(&mut self) {
        let mut pcs = load_pcs();
        pcs.extend(self.entry_pc);
        pcs.extend(self.exit_pc);
        self.am.watch_pcs(&pcs);
    }

    fn hit(&self, pc: Option<u32>) -> bool {
        pc.is_some_and(|pc| self.am.pc_hits().iter().any(|&(p, n)| p == pc && n > 0))
    }

    fn count(&self, l: &[(u8, u32)]) -> u32 {
        let l: Vec<u32> = l.iter().copied().map(h).collect();
        self.am
            .pc_hits()
            .iter()
            .filter(|(pc, _)| l.contains(pc))
            .map(|&(_, n)| n)
            .sum()
    }

    /// The event as reported this frame, given whether it happened in it.
    fn reported(&mut self, now: bool) -> bool {
        match self.report {
            Report::AsIs | Report::AsStop => now,
            Report::Late => std::mem::replace(&mut self.held, now),
            Report::Never => false,
        }
    }
}

impl Transplant for StandIn {
    fn transplant(&mut self, _: &dyn Mem) {
        unreachable!("a case lined up by loads transplants nothing")
    }
    fn probe(&self, _: &Var) -> u32 {
        unreachable!("these cases compare memory, not variables")
    }
}

impl FrameEngine for StandIn {
    fn frame(&mut self, input: FrameInput) {
        if std::mem::take(&mut self.first) {
            // The entry's step is the runner's first; the stand-in has run it already.
            self.loads = (0, false);
            self.read(Vec::new());
            return;
        }
        let f = self.am.frame();
        if let Some(s) = self.case.swap.filter(|s| s.frame == f) {
            self.am.insert_disk(0, s.disk as usize - 1);
        }
        let c = self.am.step(input).unwrap();
        let r = self.count(&LOAD_RETURNS);
        self.load_entered += self.count(&LOAD_ROUTINES);
        self.load_returned += r;
        self.loads = (r, self.load_entered > self.load_returned);
        // One event a case: `held` is the watched one's.
        let (entered, exited) = (self.hit(self.entry_pc), self.hit(self.exit_pc));
        if self.entry_pc.is_some() {
            self.entered = self.reported(entered);
        } else if exited && self.report == Report::AsStop {
            self.stop = Some(format!("untranslated: ${:06x}", self.exit_pc.unwrap()));
        } else if self.reported(exited) {
            self.exited = true;
            self.stop = Some("exit: the program returned from its entry".into());
        }
        self.read(c.paula);
    }
    fn picture(&self) -> Option<amiga_chips::Picture> {
        self.pic.as_ref().map(picture_of)
    }
    fn paula(&self) -> &[PaulaWrite] {
        &self.paula
    }
    fn custom_writes(&self) -> &[(u16, u16)] {
        &self.custom
    }
    fn pass_ends(&self, _: &[&Var]) -> Vec<Vec<u32>> {
        unreachable!("lined up by loads, not pass ends")
    }
    fn stopped(&self) -> Option<String> {
        self.stop.clone()
    }
    fn ram(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }
    fn loads(&self) -> (u32, bool) {
        self.loads
    }
    fn watch_entry(&mut self, at: (u8, u32)) -> Result<(), String> {
        self.entry_pc = Some(h(at));
        self.watch();
        Ok(())
    }
    fn entered(&self) -> bool {
        self.entered
    }
    fn exit_to(&mut self, back: u32, _sp: u32) {
        self.exit_pc = Some(back);
        self.watch();
    }
    fn exited(&self) -> bool {
        self.exited
    }
}

impl StandIn {
    fn read(&mut self, paula: Vec<PaulaWrite>) {
        let m = self.am.memory();
        self.ram = m.chip.clone();
        self.ram.extend_from_slice(&m.slow);
        self.pic = Some(self.am.picture_in(SCREEN_WINDOW.0, SCREEN_WINDOW.1));
        self.paula = paula;
        self.custom = self
            .am
            .cpu_writes()
            .iter()
            .map(|w| (w.reg & 0x1FE, w.value))
            .collect();
    }
}

/// From the idle title at 4990, Start at 5000 (world-2-start's input) and `load_world` entered;
/// 60 frames, before any load returns on the original.
fn enters(routine: &str, by_frame: u32) -> Case {
    Case::parse(
        "enters-made-up",
        &format!(
            "sync = {{ script = \"boot-to-level1\", frame = 4990 }}\ninput = \"world-2-start\"\n\
             align = \"loads\"\nswap = {{ frame = 3600, disk = 2 }}\nframes = 60\n\
             compare = [\"memory\", \"picture\", \"writes\"]\n\
             goal = {{ enters = \"{routine}\", by_frame = {by_frame} }}\n"
        ),
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// KONIEC, from the idle title at 4990 or from power-on.
fn exited(power_on: bool) -> Case {
    let start = if power_on {
        "sync = { script = \"case-goals-koniec\", frame = 0 }\nstart = \"entry\"\n"
    } else {
        "sync = { script = \"case-goals-koniec\", frame = 4990 }\n"
    };
    Case::parse(
        "exited-made-up",
        &format!(
            "{start}align = \"loads\"\nswap = {{ frame = 3600, disk = 2 }}\nframes = 5100\n\
             compare = [\"memory\", \"picture\", \"writes\"]\n\
             goal = {{ exited = true, by_frame = 5100 }}\n"
        )
        .replace("frames = 5100", if power_on { "frames = 5100" } else { "frames = 110" }),
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn run(case: &Case, report: Report) -> Verdict {
    let inputs = Inputs::load(case).unwrap();
    let mut s = StandIn::new(case, report);
    run_entry_frames(case, &inputs, &mut s, &Options::default()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn both_sides_entering_the_routine_in_the_same_frame_pass() {
    let v = run(&enters("load_world", 5050), Report::AsIs);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    let at = v.goal_met.expect("the goal met");
    assert!((5000..=5002).contains(&at), "load_world entered on Start at 5000: {v}");
}

#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn one_side_entering_a_frame_later_fails_naming_both_frames() {
    let v = run(&enters("load_world", 5050), Report::Late);
    match &v.outcome {
        Outcome::SceneDiffers { frame, why } => {
            let want = format!(
                "the original entered load_world at oracle frame {frame}, the engine at oracle \
                 frame {}",
                frame + 1
            );
            assert_eq!(why, &want, "{v}");
        }
        other => panic!("expected the entries a frame apart, got {other}"),
    }
}

#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn the_engine_never_entering_fails_naming_the_originals_frame() {
    let v = run(&enters("load_world", 5050), Report::Never);
    match &v.outcome {
        Outcome::SceneDiffers { why, .. } => assert!(
            why.starts_with("the original entered load_world at oracle frame 50")
                && why.ends_with("the engine not by oracle frame 5050"),
            "{v}"
        ),
        other => panic!("expected the engine's entry missing, got {other}"),
    }
}

/// Neither side enters a routine the path never reaches (`next_world`, a level's end), or one
/// they enter only after `by_frame`: both fail as a goal missed.
#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn neither_entering_by_by_frame_fails_as_the_goal_missed() {
    for case in [enters("next_world", 5050), enters("load_world", 4995)] {
        let v = run(&case, Report::AsIs);
        match &v.outcome {
            Outcome::GoalMissed { goal } => assert!(goal.starts_with("both sides entering "), "{v}"),
            other => panic!("expected the goal missed, got {other}"),
        }
        assert_eq!(v.goal_met, None, "{v}");
    }
}

/// KONIEC from a snapshot: the runner hands the stand-in the exit it read off its own original's
/// entry, as it hands the translated program's (`TranslatedGame::exit_to`).
#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn both_sides_returning_from_the_entry_together_pass_from_a_snapshot() {
    let v = run(&exited(false), Report::AsIs);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.goal_met, Some(5074), "quit_to_dos returns in step 5073: {v}");
}

#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn both_sides_returning_from_the_entry_together_pass_from_power_on() {
    let v = run(&exited(true), Report::AsIs);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.goal_met, Some(5074), "{v}");
}

/// The translated program's jump to an address with no translation where the original returns
/// to AmigaDOS is no exit: a stop, as before (translator T41).
#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn a_stop_where_the_original_exits_fails_as_a_stop() {
    let v = run(&exited(false), Report::AsStop);
    match &v.outcome {
        Outcome::Stopped { frame, stop, .. } => {
            assert_eq!(*frame, 5074, "{v}");
            assert!(stop.starts_with("untranslated: $"), "{v}");
        }
        other => panic!("expected a stop, got {other}"),
    }
}

#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn the_engine_exiting_a_frame_late_fails_naming_both_frames() {
    let v = run(&exited(false), Report::Late);
    match &v.outcome {
        Outcome::SceneDiffers { frame, why } => {
            assert_eq!(*frame, 5074, "{v}");
            assert_eq!(
                why,
                "the original returned from the program's entry at oracle frame 5074, the engine \
                 at oracle frame 5075",
                "{v}"
            );
        }
        other => panic!("expected the exits a frame apart, got {other}"),
    }
}

fn translated(case: &Case) -> Verdict {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
    run_translated(case, &Options::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// The translated program from a snapshot quits on KONIEC in the original's frame: the exit it is
/// handed (`TranslatedGame::exit_to`) is the one its `quit_to_dos` returns to. Without it the
/// return to AmigaDOS stopped `untranslated` (translator T41).
#[test]
#[ignore = "full: the disks and Kickstart, the translated tree built in"]
fn the_translated_program_exits_with_the_original_from_a_snapshot() {
    let v = translated(&exited(false));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.goal_met, Some(5074), "{v}");
}

/// The translated program enters `load_world` on Start in the original's frame.
#[test]
#[ignore = "full: the disks and Kickstart, the translated tree built in"]
fn the_translated_program_enters_with_the_original() {
    let v = translated(&enters("load_world", 5050));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.goal_met.is_some(), "{v}");
}
