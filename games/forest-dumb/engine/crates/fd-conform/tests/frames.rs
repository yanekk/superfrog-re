//! The frame bar's runner (`fd_conform::frames`, translator T29) against a stand-in whole-frame
//! engine: a second copy of the original from the same cached sync point, stepped a frame a call, so
//! the runner is shown to pass an engine equal in state, picture and Paula's writes, and to fail one
//! whose writes differ in order, in number, or in a later frame. Needs the disks and Kickstart (the
//! cached level sync point): absent, it fails naming where it looked.

mod common;

use amiga_oracle::{Amiga, AmigaMemory, FrameInput, PaulaWrite, Rgb12Frame};
use common::picture_of;
use fd_conform::frames::{FrameEngine, run_frames};
use fd_conform::reading::{Reading, Row};
use fd_conform::{Case, Options, Outcome, PASS_END, PLAY_WINDOW, Verdict, reach_case, run_with};
use fd_core::{Game, Mem, Start, Transplant, Var, layout_var};

/// A made-up case: 40 frames of W1-1 standing still from w1-idle's sync point, its state, its
/// picture and Paula's writes.
fn made_up(frames: u32) -> Case {
    Case::parse(
        "frames-made-up",
        &format!(
            "sync = {{ script = \"boot-to-level1\", frame = 6400 }}\nframes = {frames}\n\
             compare = [\"vars\", \"picture\", \"writes\"]\n\
             vars = [\"char_x\", \"camera_x\", \"irq_count\", \"countdown\"]\n\
             goal = {{ idle = true }}\n"
        ),
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// What the stand-in does to its writes on one frame (counted from 1 after the transplant).
#[derive(Clone, Copy)]
enum Meddle {
    None,
    /// Swap the first two writes of different registers.
    Swap(u32),
    /// Leave the last write out.
    Drop(u32),
    /// Report `countdown` one higher.
    Bump(u32),
}

struct StandIn {
    am: Amiga,
    inputs: fd_conform::Inputs,
    meddle: Vec<Meddle>,
    frames: u32,
    mem: Option<AmigaMemory>,
    pic: Option<Rgb12Frame>,
    paula: Vec<PaulaWrite>,
}

impl StandIn {
    fn new(case: &Case, meddle: &[Meddle]) -> StandIn {
        StandIn {
            am: reach_case(case, 4).unwrap_or_else(|e| panic!("{e}")),
            inputs: fd_conform::Inputs::load(case).unwrap_or_else(|e| panic!("{e}")),
            meddle: meddle.to_vec(),
            frames: 0,
            mem: None,
            pic: None,
            paula: Vec::new(),
        }
    }
}

impl Transplant for StandIn {
    /// Step the copy to the step end the runner transplants at: the first pass end after the sync.
    fn transplant(&mut self, mem: &dyn Mem) {
        let irq = layout_var("irq_count").unwrap();
        self.am.watch_write(Some(PASS_END));
        while self.am.watched_memory().is_none() {
            let f = self.am.frame();
            self.am.step(self.inputs.at(f)).unwrap();
        }
        self.am.watch_write(None);
        assert_eq!(
            fd_conform::Live(&self.am).value(irq),
            mem.value(irq),
            "the stand-in's start is the runner's"
        );
    }

    fn probe(&self, v: &Var) -> u32 {
        let n = fd_conform::OracleMem(self.mem.as_ref().expect("probed before a frame")).value(v);
        let bumped = self
            .meddle
            .iter()
            .any(|m| matches!(m, Meddle::Bump(f) if *f == self.frames));
        n + (bumped && v.name == "countdown") as u32
    }
}

impl FrameEngine for StandIn {
    fn frame(&mut self, input: FrameInput) {
        let c = self.am.step(input).unwrap();
        self.frames += 1;
        self.mem = Some(self.am.memory());
        self.pic = Some(self.am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1));
        self.paula = c.paula;
        for m in &self.meddle {
            match *m {
                Meddle::Swap(f) if f == self.frames => {
                    let k = (1..self.paula.len())
                        .find(|&k| self.paula[k].reg != self.paula[0].reg)
                        .expect("two writes of different registers");
                    self.paula.swap(0, k);
                }
                Meddle::Drop(f) if f == self.frames => {
                    self.paula.pop().expect("a write to drop");
                }
                _ => {}
            }
        }
    }

    fn picture(&self) -> Option<amiga_chips::Picture> {
        self.pic.as_ref().map(picture_of)
    }

    fn paula(&self) -> &[PaulaWrite] {
        &self.paula
    }

    fn stopped(&self) -> Option<String> {
        None
    }
}

fn run(case: &Case, meddle: &[Meddle], opts: &Options) -> Verdict {
    run_frames(case, &mut StandIn::new(case, meddle), opts).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn an_equal_engine_passes_on_state_picture_and_paulas_writes_every_frame() {
    let case = made_up(40);
    let v = run(&case, &[Meddle::None], &Options::default());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // The first step ends the transplanted pass; every frame after it is compared.
    assert_eq!(v.passes + 1, case.frames, "{v}");
    assert_eq!(v.pictures, v.passes, "{v}");
    assert_eq!(v.write_frames, v.passes, "{v}");
    // Level 1's music writes Paula every frame: the comparison is not of two empty lists.
    assert!(v.writes >= 12 * v.write_frames, "{v}");
}

#[test]
fn a_write_out_of_order_fails_naming_the_frame_and_the_write() {
    let v = run(&made_up(40), &[Meddle::Swap(7)], &Options::default());
    let Outcome::WritesDiffer { pass, why, .. } = &v.outcome else {
        panic!("{v}");
    };
    assert_eq!(*pass, 7, "{v}");
    assert!(why.starts_with("write #0: original AUD"), "{why}");
}

#[test]
fn a_missing_write_fails() {
    let v = run(&made_up(40), &[Meddle::Drop(3)], &Options::default());
    let Outcome::WritesDiffer { pass, why, .. } = &v.outcome else {
        panic!("{v}");
    };
    assert_eq!(*pass, 3, "{v}");
    assert!(why.contains("engine nothing"), "{why}");
}

#[test]
fn every_channel_goes_on_past_the_writes_to_the_states_first_difference() {
    let opts = Options {
        every_channel: true,
        ..Options::default()
    };
    let v = run(&made_up(40), &[Meddle::Swap(4), Meddle::Bump(9)], &opts);
    assert!(matches!(v.outcome, Outcome::WritesDiffer { pass: 4, .. }), "{v}");
    assert_eq!(v.firsts.len(), 2, "{v}");
    let Outcome::Differs { pass, var, .. } = &v.firsts[1] else {
        panic!("{v}");
    };
    assert_eq!((*pass, var.as_str()), (9, "countdown"), "{v}");
    // The picture never differed and was compared to the end.
    assert_eq!(v.pictures + 1, 40, "{v}");
    let row = Row::new("death-made-up", Ok(v));
    assert_eq!(row.kind(), "Paula writes");
    assert_eq!(row.then(), "state: countdown at frame 9");
    let md = Reading {
        engine: "stand-in",
        rows: vec![row],
    }
    .markdown();
    assert!(md.contains("## T31:"), "{md}");
}

#[test]
fn the_hand_engines_runner_refuses_a_case_comparing_writes() {
    let case = made_up(10);
    let mut game = Game::new(fd_conform::game_data().unwrap(), Start::Level1);
    let e = run_with(&case, &mut game, &Options::default()).unwrap_err();
    assert!(e.contains("compare \"writes\""), "{e}");
}
