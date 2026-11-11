//! A creature the original cuts behind the beam (plans/forest-dumb-level-1 T80; docs/10, "A creature
//! cut behind the beam"). With several creatures drawn a pass runs into the next frame's play area,
//! and a creature `h0_5820` cuts after the beam has passed its lines is missing from that frame,
//! whole or from its top. The runner reads each cut's beam position off the original
//! (`fd_conform::creature_cuts`), works out the rows the beam had passed (`rows_behind_beam`) and
//! compares that frame with the engine's picture of it (`Engine::creatures_uncut`). Only there: a
//! creature missing on a frame with no such cut measured is a difference like any other.
//!
//! That the three cases meeting it pass with the play area compared is `mechanic_cases.rs`'s and
//! `death.rs`'s; that reading the cuts leaves the original unchanged is `edge_latch.rs`'s
//! (`sampling_for_the_edge_cut_leaves_the_original_unchanged_every_frame`). Needs the disks and the
//! Kickstart: absent, it fails naming where it looked.

use std::cell::RefCell;
use std::ops::Range;
use std::path::Path;

use amiga_chips::{Picture, to_rgb12};
use amiga_oracle::FrameInput;
use fd_conform::{Case, Engine, Options, Outcome, Verdict, game_data, run_with};
use fd_core::level::{EdgesCut, Event};
use fd_core::render::objects::CUT_ROWS;
use fd_core::{Game, Mem, Transplant, Var};

/// The first `frames` of a case, nothing excluded and no goal: a stretch of it.
fn stretch(name: &str, frames: u32) -> Case {
    let case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    Case {
        frames,
        goal: None,
        exclude: Vec::new(),
        ..case
    }
}

/// fd-core's game, counting its passes, with the hand-over taken or not and its creatures left out
/// of the picture on the passes of `left_out`.
struct Shown {
    game: Game,
    passes: u32,
    takes: bool,
    left_out: Range<u32>,
    /// The passes the runner asked for a picture with rows uncut.
    asked: RefCell<Vec<u32>>,
    /// The passes whose picture changes by more than the tolerance's few pixels with every
    /// creature left out: a creature in view.
    in_view: Vec<u32>,
}

impl Shown {
    fn new(takes: bool, left_out: Range<u32>) -> Shown {
        let data = game_data().unwrap_or_else(|e| panic!("{e}"));
        Shown {
            game: Game::new(data, fd_core::Start::Level1),
            passes: 0,
            takes,
            left_out,
            asked: RefCell::new(Vec::new()),
            in_view: Vec::new(),
        }
    }

    /// The picture with every creature the last pass cut left out.
    fn none_cut(&self) -> Option<Picture> {
        let n = self.game.display().cuts.creatures();
        self.game.creatures_uncut(&vec![CUT_ROWS; n])
    }
}

impl Transplant for Shown {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem)
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Shown {
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input);
        self.passes += 1;
        if let Some(p) = self.none_cut() {
            let (whole, none) = (to_rgb12(&self.game.picture()), to_rgb12(&p));
            if whole.iter().zip(&none).filter(|(a, b)| a != b).count() > 64 {
                self.in_view.push(self.passes);
            }
        }
    }
    fn picture(&self) -> Option<Picture> {
        if self.left_out.contains(&self.passes) {
            return self.none_cut();
        }
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        self.game.cut_redraw(cut)
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        self.game.cut_foreground(at)
    }
    fn creatures_uncut(&self, rows: &[u16]) -> Option<Picture> {
        self.asked.borrow_mut().push(self.passes);
        if self.takes {
            self.game.creatures_uncut(rows)
        } else {
            None
        }
    }
}

fn run(case: &Case, engine: &mut Shown) -> Verdict {
    run_with(case, engine, &Options::default()).unwrap_or_else(|e| panic!("{e}"))
}

/// `mechanic-spring` to a little past pass 475, where five creatures are in view and the two at
/// the view's top are cut behind the beam frame after frame. With the hand-over the stretch is the
/// original's; an engine that does not take it shows those creatures whole and fails at frame
/// 6779, the third such frame in a row (measured, T79).
#[test]
fn an_engine_that_does_not_take_the_hand_over_fails_at_frame_6779() {
    let case = stretch("mechanic-spring", 500);
    let (with, without) = std::thread::scope(|s| {
        let with = s.spawn(|| run(&case, &mut Shown::new(true, 0..0)));
        let without = s.spawn(|| run(&case, &mut Shown::new(false, 0..0)));
        (with.join().unwrap(), without.join().unwrap())
    });
    assert_eq!(with.outcome, Outcome::Same, "{with}");
    assert!(with.creature_cuts >= 15, "{with}");
    assert!(
        matches!(
            &without.outcome,
            Outcome::PictureDiffers { frame: 6779, .. }
        ),
        "{without}"
    );
    assert_eq!(without.creature_cuts, 0, "{without}");
}

/// The hand-over applies only on the frames the measurement names (owner, 2026-11-10): an engine
/// leaving its creatures out on three frames in a row where the original cut none behind the beam
/// fails on the third, the tolerance's two spent.
#[test]
fn a_creature_left_out_where_no_cut_was_measured_fails() {
    let case = stretch("mechanic-spring", 470);
    let mut honest = Shown::new(true, 0..0);
    let v = run(&case, &mut honest);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // Three passes in a row with a creature in view, none of them or their neighbours asked for.
    let asked = honest.asked.borrow().clone();
    let quiet = |p: u32| !(p.saturating_sub(3)..=p + 5).any(|q| asked.contains(&q));
    let from = honest
        .in_view
        .windows(3)
        .find(|w| w[2] == w[0] + 2 && quiet(w[0]))
        .map(|w| w[0])
        .unwrap_or_else(|| panic!("no creature in view for three quiet passes: {asked:?}"));
    let mut mutant = Shown::new(true, from..from + 3);
    let v = run(&case, &mut mutant);
    assert!(
        matches!(&v.outcome, Outcome::PictureDiffers { pass, .. } if *pass == from + 2),
        "left out from pass {from}: {v}"
    );
}

/// `mechanic-vanish`'s first 329 steps: the play area is the original's with nothing excluded, six
/// of its frames by the hand-over (measured 2026-11-10). Until T81's fix the case excluded the play
/// area from pass 330 on; the whole case now compares it (`mechanic_cases.rs`).
#[test]
fn mechanic_vanish_compares_the_play_area_until_the_early_creature() {
    let case = stretch("mechanic-vanish", 329);
    let v = run(&case, &mut Shown::new(true, 0..0));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.pictures >= 320, "{v}");
    assert!(v.creature_cuts >= 5, "{v}");
}
