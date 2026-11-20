//! A mid-run input reaching the engine a frame early or late fails the case (T06), which closes
//! forest-dumb-start's surviving mutant: there the title ignored input, so feeding the engine the
//! previous frame's input passed. Here the stub level case holds right from frame 6321 and the
//! engine is a stand-in copy of the original fed the case's input shifted by a frame. Needs the
//! disks and the Kickstart: absent, it fails naming where it looked.

mod common;

use std::path::Path;

use common::{Feed, StandIn};
use fd_conform::{Case, Goal, Options, Outcome, run_with};

/// The stub up to the jump: standing for 20 frames, then right held.
fn case() -> Case {
    let stub = Case::load(Path::new("cases/stub-level.toml")).unwrap_or_else(|e| panic!("{e}"));
    Case {
        frames: 40,
        goal: Some(Goal::All {
            all: vec![("char_x".into(), 0x70)],
            by_frame: 6341,
        }),
        ..stub
    }
}

fn outcome(feed: Feed) -> Outcome {
    let case = case();
    let mut engine = StandIn::new(&case, feed);
    run_with(&case, &mut engine, &Options::default())
        .unwrap_or_else(|e| panic!("{e}"))
        .outcome
}

/// The engine takes the input the runner gives it: a runner feeding a pass another step's input
/// fails here (the mutant forest-dumb-start's review left alive).
#[test]
fn the_input_on_its_own_frame_matches() {
    assert_eq!(outcome(Feed::Given), Outcome::Same);
}

/// Right reaches the engine a frame late or early: the character moves a pass later or sooner,
/// and the first pass of the walk is named.
#[test]
fn the_input_a_frame_late_or_early_fails_at_the_walks_first_pass() {
    for shift in [1, -1] {
        match outcome(Feed::Shifted(shift)) {
            Outcome::Differs { pass, frame, .. } => {
                // Right is held from the step starting at frame 6321, the engine's pass 20.
                assert!((19..=21).contains(&pass), "shift {shift}: pass {pass}");
                assert!(
                    (6320..=6323).contains(&frame),
                    "shift {shift}: frame {frame}"
                );
            }
            other => panic!("shift {shift}: {other}"),
        }
    }
}
