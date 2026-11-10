//! The level case runner (T06) against a stand-in engine: a second copy of the original
//! (`common::StandIn`), so the runner is shown to pass a matching engine and to fail, and name, each
//! way an engine can differ. Needs the disks and the Kickstart: absent, it fails naming where it
//! looked.

mod common;

use std::path::Path;

use amiga_conform::Mode;
use common::{Feed, StandIn};
use fd_conform::{Case, Exclusion, Goal, Options, Outcome, Verdict, run_with};
use fd_core::Ported;

fn stub() -> Case {
    Case::load(Path::new("cases/stub-level.toml")).unwrap_or_else(|e| panic!("{e}"))
}

/// The stub's first `frames` frames, standing still: a goal of `char_x` at its start.
fn short(frames: u32) -> Case {
    Case {
        frames,
        goal: Some(Goal::All {
            all: vec![("char_x".into(), 0x70)],
            by_frame: 6301 + frames,
        }),
        ..stub()
    }
}

fn run_on(case: &Case, engine: &mut StandIn, opts: &Options) -> Verdict {
    run_with(case, engine, opts).unwrap_or_else(|e| panic!("{e}"))
}

fn honest(case: &Case) -> StandIn {
    StandIn::new(case, Feed::Given)
}

#[test]
fn the_stub_level_case_matches_on_state_and_picture_and_reaches_its_goal() {
    let case = stub();
    let v = run_on(&case, &mut honest(&case), &Options::default());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // One pass a frame; the transplant's own pass and passes ended unseen are not compared.
    assert_eq!(v.passes + v.unseen + 1, case.frames, "{v}");
    // A picture a frame: each frame after the transplant's shows a pass (T44's pairing), the
    // passes ended unseen included.
    assert_eq!(v.pictures + 1, case.frames, "{v}");
    assert!(v.passes > 100, "{v}");
    assert!(v.goal_met.is_some(), "{v}");
}

#[test]
fn a_one_off_value_is_named_by_pass_frame_and_values() {
    let case = short(30);
    let mut e = honest(&case);
    e.bump = Some((10, "camera_x"));
    let v = run_on(&case, &mut e, &Options::default());
    let Outcome::Differs {
        pass,
        frame,
        ref var,
        engine,
        oracle,
    } = v.outcome
    else {
        panic!("{v}")
    };
    assert_eq!((pass, var.as_str(), engine), (10, "camera_x", oracle + 1));
    // Pass 10 after the transplant (pass 1) ends in the step to frame 6312 or the next.
    assert!((6311..=6313).contains(&frame), "{v}");
    let text = v.outcome.to_string();
    for part in [
        "pass 10".to_string(),
        format!("frame {frame}"),
        format!("{engine} ({engine:#x}) in the engine"),
        format!("{oracle} ({oracle:#x}) in the original"),
    ] {
        assert!(text.contains(&part), "{text}");
    }
}

#[test]
fn one_pixel_off_fails_strict_and_is_counted_faithful() {
    let case = short(30);
    let mut e = honest(&case);
    e.paint = Some((10..11, 100, 1));
    let strict = Options {
        mode: Mode::Strict,
        ..Options::default()
    };
    let v = run_on(&case, &mut e, &strict);
    assert!(
        matches!(v.outcome, Outcome::PictureDiffers { pass: 10, .. }),
        "{v}"
    );
    assert!(v.outcome.to_string().contains("first at (0, 100)"), "{v}");

    let mut e = honest(&case);
    e.paint = Some((10..11, 100, 1));
    let v = run_on(&case, &mut e, &Options::default());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.tolerated, [0, 1, 0], "{v}");
}

#[test]
fn a_difference_past_the_tolerance_fails_and_an_excluded_area_passes_and_is_reported() {
    // A whole status-bar row off: a glitch, and one glitch frame of 30 is over the owner's 1%.
    let case = short(30);
    let mut e = honest(&case);
    e.paint = Some((10..15, 200, 320));
    let v = run_on(&case, &mut e, &Options::default());
    assert!(
        matches!(v.outcome, Outcome::PictureDiffers { pass: 10, .. }),
        "{v}"
    );

    let bar = Exclusion {
        rows: 194..250,
        cols: None,
        frames: None,
        var: None,
        what: "rows 194..250".into(),
        task: "T10".into(),
    };
    let case = Case {
        exclude: vec![bar.clone()],
        ..short(30)
    };
    let mut e = honest(&case);
    e.paint = Some((10..15, 200, 320));
    let v = run_on(&case, &mut e, &Options::default());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.excluded, vec![(bar, 5)]);
    assert!(
        v.to_string()
            .contains("excluded rows 194..250 (T10): differed on 5 frames"),
        "{v}"
    );

    // An exclusion that hides nothing is no longer needed.
    let v = run_on(&case, &mut honest(&case), &Options::default());
    assert!(
        matches!(v.outcome, Outcome::ExclusionUnused { ref task, .. } if task == "T10"),
        "{v}"
    );
}

/// T82 (owner, 2026-11-10): "in a row" is counted as the toolkit's `PictureJudge` counts it, the
/// judge told of every compared frame. Two glitch frames, an identical one and a third glitch are
/// three tolerated frames; three glitch frames with none between fail on the third. Before, the
/// judge heard only of differing frames, so the identical frame ended no run and the first failed
/// too. 300 frames, so that three glitches are inside the 1% budget.
#[test]
fn an_identical_frame_ends_a_run_of_glitches_and_three_in_a_row_still_fail() {
    let case = short(300);
    let (apart, together) = std::thread::scope(|s| {
        let apart = s.spawn(|| {
            let mut e = honest(&case);
            e.paint = Some((10..12, 200, 320));
            e.paint_again = Some(13..14);
            run_on(&case, &mut e, &Options::default())
        });
        let together = s.spawn(|| {
            let mut e = honest(&case);
            e.paint = Some((10..13, 200, 320));
            run_on(&case, &mut e, &Options::default())
        });
        (apart.join().unwrap(), together.join().unwrap())
    });
    assert_eq!(apart.outcome, Outcome::Same, "{apart}");
    assert_eq!(apart.tolerated, [0, 0, 3], "{apart}");
    match &together.outcome {
        Outcome::PictureDiffers { pass: 12, why, .. } => {
            assert!(why.contains("3 frames in a row"), "{why}")
        }
        other => panic!("{other}\n{together}"),
    }
}

/// T48 review: a box naming frames hides its area on those frames alone. The same box a frame
/// short, or on frames the difference never shows on, leaves the difference to fail the case.
#[test]
fn a_box_naming_frames_hides_nothing_on_any_other_frame() {
    let paint = || Some((10..15, 200, 320));
    let case = short(30);
    let mut e = honest(&case);
    e.paint = paint();
    let v = run_on(&case, &mut e, &Options::default());
    let Outcome::PictureDiffers {
        pass: 10, frame, ..
    } = v.outcome
    else {
        panic!("{v}")
    };
    let boxed = |frames: std::ops::Range<u32>| {
        let bar = Exclusion {
            rows: 194..250,
            cols: Some(0..320),
            frames: Some(frames.clone()),
            var: None,
            what: format!(
                "rows 194..250 cols 0..320 frames {}..{}",
                frames.start, frames.end
            ),
            task: "T82".into(),
        };
        let case = Case {
            exclude: vec![bar],
            ..short(30)
        };
        let mut e = honest(&case);
        e.paint = paint();
        run_on(&case, &mut e, &Options::default())
    };
    // Named on the five frames that differ: hidden, and counted on those five.
    let v = boxed(frame..frame + 5);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.excluded[0].1, 5, "{v}");
    // One frame short at either end: that frame is compared and fails.
    let v = boxed(frame + 1..frame + 5);
    assert!(
        matches!(v.outcome, Outcome::PictureDiffers { pass: 10, .. }),
        "{v}"
    );
    let v = boxed(frame..frame + 4);
    assert!(
        matches!(v.outcome, Outcome::PictureDiffers { pass: 14, .. }),
        "{v}"
    );
    // Named on other frames only: the difference fails as with no box.
    let v = boxed(frame + 5..frame + 10);
    assert!(
        matches!(v.outcome, Outcome::PictureDiffers { pass: 10, .. }),
        "{v}"
    );
}

#[test]
fn a_case_that_compares_nothing_fails() {
    // One frame: the only pass end seen is the transplant's.
    for frames in [0, 1] {
        let case = short(frames);
        let v = run_on(&case, &mut honest(&case), &Options::default());
        assert!(
            matches!(v.outcome, Outcome::ComparedNothing { .. }),
            "{frames}: {v}"
        );
    }
}

#[test]
fn two_sides_that_agree_but_miss_the_goal_fail() {
    let case = Case {
        goal: Some(Goal::All {
            all: vec![("char_x".into(), 0x1a3)],
            by_frame: 6380,
        }),
        ..stub()
    };
    let v = run_on(&case, &mut honest(&case), &Options::default());
    assert!(
        matches!(v.outcome, Outcome::GoalMissed { ref goal } if goal.contains("char_x = 419 by oracle frame 6380")),
        "{v}"
    );
    assert_eq!(
        v.passes + v.unseen + 1,
        case.frames,
        "it still ran to the end: {v}"
    );
}

#[test]
fn an_idle_case_passes_only_with_every_pass_compared_and_no_event() {
    let idle = |frames| Case {
        frames,
        goal: Some(Goal::Idle),
        ..stub()
    };
    let case = idle(20);
    let v = run_on(&case, &mut honest(&case), &Options::default());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.passes, 19, "{v}");

    // An unported object fails it, as it fails every case (tests/level_entry.rs: an `all` goal).
    let mut e = honest(&case);
    e.event = Some(5);
    let v = run_on(&case, &mut e, &Options::default());
    assert!(
        matches!(
            v.outcome,
            Outcome::Unported {
                pass: 5,
                value: 0xee,
                ..
            }
        ),
        "{v}"
    );
    assert!(v.outcome.to_string().contains("0xee"), "{v}");

    // Past main_loop's first 32-interrupt check, where one step ends two passes.
    let case = idle(40);
    let v = run_on(&case, &mut honest(&case), &Options::default());
    assert!(v.unseen > 0, "{v}");
    assert!(
        matches!(v.outcome, Outcome::NotIdle { ref why } if why.contains("every pass must be")),
        "{v}"
    );
}

#[test]
fn the_coverage_report_counts_each_registered_routine_and_shows_an_unreached_one_at_zero() {
    let register = [
        Ported {
            name: "game_frame",
            hunk: 0,
            offset: 0x1fec,
            port: "test",
        },
        Ported {
            name: "title_screen",
            hunk: 0,
            offset: 0x6e06,
            port: "test",
        },
    ];
    let case = short(20);
    let opts = Options {
        register: &register,
        ..Options::default()
    };
    let v = run_on(&case, &mut honest(&case), &opts);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // game_frame starts once a pass, one a frame (docs/09-play-measured.md).
    let hits = v.coverage.hits.clone();
    assert_eq!(hits[1], ("title_screen", 0), "{v}");
    assert_eq!(hits[0].0, "game_frame");
    assert!((19..=21).contains(&hits[0].1), "{v}");
    assert_eq!(v.coverage.unreached(), vec!["title_screen"]);
    assert!(v.to_string().contains("never reached"), "{v}");
}
