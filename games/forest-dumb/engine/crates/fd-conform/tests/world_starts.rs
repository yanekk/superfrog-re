//! World starts (translator T33): `cases/world-2-start.toml` … `world-6-start.toml`, lined up by
//! loads from a snapshot of the idle title, `world_index` set as `load_world` is entered on both
//! sides (`set` with `when`). Full: the owner's disks and Kickstart; the translated tests also the
//! tree built in (`tools/translate.py --build`).
//!
//! The runner, `fd_conform::frames::run_entry_frames`, against a stand-in: a second copy of the
//! original from power-on to the same sync frame, which makes the case's write as the runner's
//! original makes it (`Amiga::poke_at` at the routine's first instruction), or between frames at
//! its start, where `title_code_world` overwrites it in the step `load_world` reads it.

mod common;

use amiga_oracle::{Amiga, FrameInput, OracleConfig, PaulaWrite, Rgb12Frame, kickstart_path};
use common::picture_of;
use fd_conform::frames::{FrameEngine, SCREEN_WINDOW, run_entry_frames};
use fd_conform::sync::{LOAD_RETURNS, LOAD_ROUTINES};
use fd_conform::translated_engine::{TranslatedEngine, case_start, frame_bar, run_translated};
use fd_conform::{Case, HUNK_BASES, Inputs, Options, Outcome, Verdict, disks};
use fd_core::{Mem, Transplant, Var, layout_var};
use fd_translated::TranslatedGame;

/// How the stand-in makes the case's write.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Write {
    /// As the runner makes it on the original: when the CPU reaches the routine.
    AtRoutine,
    /// Into RAM at once, between frames, before its first frame.
    BetweenFrames,
}

struct StandIn {
    am: Amiga,
    case: Case,
    write: Write,
    entered: u32,
    returned: u32,
    loads: (u32, bool),
    ram: Vec<u8>,
    pic: Option<Rgb12Frame>,
    paula: Vec<PaulaWrite>,
    custom: Vec<(u16, u16)>,
}

fn h((hunk, off): (u8, u32)) -> u32 {
    HUNK_BASES[hunk as usize] + off
}

impl StandIn {
    /// The original from power-on to the end of `case`'s sync frame's step, as the runner's.
    fn new(case: &Case, write: Write) -> StandIn {
        let inputs = Inputs::load(case).unwrap();
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
        let pcs: Vec<u32> = LOAD_ROUTINES
            .iter()
            .chain(LOAD_RETURNS.iter())
            .copied()
            .map(h)
            .collect();
        am.watch_pcs(&pcs);
        StandIn {
            am,
            case: case.clone(),
            write,
            entered: 0,
            returned: 0,
            loads: (0, false),
            ram: Vec::new(),
            pic: None,
            paula: Vec::new(),
            custom: Vec::new(),
        }
    }
}

impl Transplant for StandIn {
    fn transplant(&mut self, _: &dyn Mem) {
        unreachable!("a case lined up by loads transplants nothing")
    }
    fn probe(&self, v: &Var) -> u32 {
        let b = self.am.peek(h((v.hunk, v.offset)), v.width.bytes());
        b.iter().fold(0, |acc, &x| acc << 8 | x as u32)
    }
}

impl FrameEngine for StandIn {
    fn frame(&mut self, input: FrameInput) {
        let f = self.am.frame();
        if let Some(s) = self.case.swap.filter(|s| s.frame == f) {
            self.am.insert_disk(0, s.disk as usize - 1);
        }
        let c = self.am.step(input).unwrap();
        let count = |l: &[(u8, u32)]| -> u32 {
            let l: Vec<u32> = l.iter().copied().map(h).collect();
            self.am
                .pc_hits()
                .iter()
                .filter(|(pc, _)| l.contains(pc))
                .map(|&(_, n)| n)
                .sum()
        };
        let r = count(&LOAD_RETURNS);
        self.entered += count(&LOAD_ROUTINES);
        self.returned += r;
        self.loads = (r, self.entered > self.returned);
        let m = self.am.memory();
        self.ram = m.chip.clone();
        self.ram.extend_from_slice(&m.slow);
        self.pic = Some(self.am.picture_in(SCREEN_WINDOW.0, SCREEN_WINDOW.1));
        self.paula = c.paula;
        self.custom = self
            .am
            .cpu_writes()
            .iter()
            .map(|w| (w.reg & 0x1FE, w.value))
            .collect();
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
        None
    }
    fn ram(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }
    fn loads(&self) -> (u32, bool) {
        self.loads
    }
    fn write_at(&mut self, at: (u8, u32), addr: u32, bytes: &[u8]) -> Result<(), String> {
        match self.write {
            Write::AtRoutine => self.am.poke_at(h(at), addr, bytes),
            Write::BetweenFrames => self.am.poke(addr, bytes),
        }
        Ok(())
    }
    fn writes_at_pending(&self) -> usize {
        self.am.pokes_at_pending()
    }
}

/// World 2's case cut to 40 frames of play past fire on SCENA 2 (frame 6150): its whole load, the
/// screen and play's start; 30 passes by frame 6190. `when` as the case file has it, or `when`.
fn world_2_short(when: &str) -> Case {
    let text = std::fs::read_to_string(fd_conform::conform_dir().join("cases/world-2-start.toml"))
        .unwrap()
        .replace("frames = 1480", "frames = 1200")
        .replace("value = 300 }], by_frame = 6470", "value = 30 }], by_frame = 6190")
        .replace("when = \"load_world\"", &format!("when = \"{when}\""));
    Case::parse("world-2-short", &text).unwrap_or_else(|e| panic!("{e}"))
}

/// The case against a stand-in, and the stand-in's `world_index` at the end.
fn stand_in(case: &Case, write: Write) -> (Verdict, u32) {
    let inputs = Inputs::load(case).unwrap();
    let mut s = StandIn::new(case, write);
    let opts = Options {
        every_channel: true,
        ..Options::default()
    };
    let v = run_entry_frames(case, &inputs, &mut s, &opts).unwrap_or_else(|e| panic!("{e}"));
    (v, s.probe(layout_var("world_index").unwrap()))
}

/// The `set` applied once, to both sides, at the same point: a copy of the original that writes
/// `world_index` as `load_world` is entered loads world 2 with the runner's original, every frame
/// outside a load equal, and both reach play in world 2.
#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn a_set_made_on_both_sides_as_load_world_is_entered_loads_the_same_world() {
    let case = world_2_short("load_world");
    let (v, world) = stand_in(&case, Write::AtRoutine);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(world, 1, "{v}");
    assert_eq!(v.loads, 13, "world 2's load list, title's loads not among them: {v}");
    assert!(v.memory_frames >= 150, "the frames outside the load compared: {v}");
    assert!(v.goal_met.is_some(), "world_index 1 and 30 passes on both sides: {v}");
}

/// The same write made between frames, at the stand-in's start, is seen at once (world_index's
/// low byte `h0+$6e01`, frame 4991, before the original's Start) and then overwritten by
/// `title_code_world` in the step `load_world` runs: the stand-in loads world 1, the runner's
/// original world 2 (measured 2026-11-18: then the custom writes at 5001, Paula and the picture
/// on the SCENA screen).
#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn the_same_set_made_between_frames_is_lost_and_fails() {
    let case = world_2_short("load_world");
    let (v, world) = stand_in(&case, Write::BetweenFrames);
    match &v.outcome {
        Outcome::Differs { var, frame, .. } => {
            assert!(var.starts_with("h0+$6e01 (1 bytes"), "{v}");
            assert_eq!(*frame, 4991, "{v}");
        }
        other => panic!("expected world_index apart at once, got {other}"),
    }
    assert!(v.firsts.len() >= 3, "a different world's load and screen: {v}");
    assert_eq!(world, 0, "title_code_world's 0 on the stand-in: {v}");
}

/// A `set` waiting for a routine neither side enters in the case (`next_world`: level 1 never
/// ends here) is never made, and the case fails saying so, the comparison otherwise equal.
#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn a_set_never_made_fails_the_case() {
    let mut case = world_2_short("next_world");
    // World 1 loads on both sides: the goal's world_index is 0.
    case.goal = None;
    let (v, _) = stand_in(&case, Write::AtRoutine);
    match &v.outcome {
        Outcome::ComparedNothing { what } => assert!(
            what.contains("world_index at next_world")
                && what.contains("1 writes never made on the original, 1 on the engine"),
            "{v}"
        ),
        other => panic!("expected the set never made, got {other}"),
    }
}

fn generated() {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
}

fn case(w: u32) -> Case {
    Case::load(std::path::Path::new(&format!("cases/world-{w}-start.toml")))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// The translated program alone from each world case's start, on the case's inputs a frame each:
/// its loads are instant, so it waits on SCENA until the script's fire and then plays. Measured
/// 2026-11-18: in each of worlds 2 to 6 it loads that world and plays 300 passes with no stop.
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; five worlds from power-on"]
fn each_world_plays_300_passes_on_the_translated_program_with_no_stop() {
    generated();
    let irq_count = layout_var("irq_count").unwrap();
    let world_index = layout_var("world_index").unwrap();
    std::thread::scope(|s| {
        for w in 2..=6 {
            s.spawn(move || {
                let c = frame_bar(&case(w));
                let inputs = Inputs::load(&c).unwrap();
                let mut e = TranslatedEngine::new(case_start(&c).unwrap());
                for set in &c.set {
                    let at = set.when.as_ref().unwrap().at;
                    e.write_at(at, set.address().unwrap(), &set.bytes()).unwrap();
                }
                let mut passes = 0;
                for k in 1..=c.frames {
                    e.frame(inputs.at(c.sync_frame + k));
                    assert_eq!(e.stopped(), None, "world {w}, frame {k}");
                    passes = e.probe(irq_count);
                    if passes >= 300 {
                        break;
                    }
                }
                assert!(passes >= 300, "world {w}: {passes} passes by the case's end");
                assert_eq!(e.writes_at_pending(), 0, "world {w}");
                assert_eq!(e.probe(world_index), w - 1, "world {w}");
            });
        }
    });
}

fn translated(w: u32) -> Verdict {
    generated();
    run_translated(&case(w), &Options::default()).unwrap_or_else(|e| panic!("{e}"))
}

// The finish line (T33's, then T51's Done when): each world's case passes on the translated
// program. Measured 2026-11-19 (T51): worlds 5 and 6 pass; 2, 3 and 4 fail in play on a frame's
// end inside game_frame, carried by T53 (docs/14-translator.md, "Worlds 2 to 6").
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; world 2 from the title"]
fn world_2_start_passes_on_the_translated_program() {
    let v = translated(2);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; world 3 from the title"]
fn world_3_start_passes_on_the_translated_program() {
    let v = translated(3);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; world 4 from the title"]
fn world_4_start_passes_on_the_translated_program() {
    let v = translated(4);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; world 5 from the title"]
fn world_5_start_passes_on_the_translated_program() {
    let v = translated(5);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; world 6 from the title"]
fn world_6_start_passes_on_the_translated_program() {
    let v = translated(6);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}
