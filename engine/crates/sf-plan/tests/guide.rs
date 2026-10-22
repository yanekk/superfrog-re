//! Guide paths (T09, DESIGN §2.11): waypoint files, the guide's weights, a script's trace on the
//! original, and the explorer steered by one.
//!
//! `trace_original` runs the oracle (~10 s for W1-1). W6-2 to its exit is `#[ignore]`d; run it with
//! `cargo test --release -p sf-plan --test guide -- --ignored`.

mod common;

use std::time::{Duration, Instant};

use common::{at_start, id, threads};
use sf_conform::{InputScript, SyncPoint, sync};
use sf_plan::core::repair::points_from_trace;
use sf_plan::core::*;
use sf_plan::guide_file::parse_waypoints;
use sf_plan::run::{RunOptions, plan_engine, run_round};
use sf_plan::trace::{timing, trace_original};

#[test]
fn waypoints_parse_both_forms_and_comments() {
    let text = "# W6-2's way up\n\n10,20\n 11 , 21 , 7  # with coins\n#12,22\n13,23,0\n";
    let p = parse_waypoints(text).unwrap();
    let pt = |c, r, coins| GuidePoint {
        cell: (c, r),
        coins_left: coins,
    };
    assert_eq!(
        p,
        vec![pt(10, 20, None), pt(11, 21, Some(7)), pt(13, 23, Some(0))]
    );
    assert_eq!(parse_waypoints("").unwrap(), vec![]);
    for (bad, line) in [("1,2\n3\n", 2), ("1,2\n\n#\n4,x\n", 4), ("1,2,3,4", 1), ("-1,2", 1)] {
        let e = parse_waypoints(bad).unwrap_err();
        assert!(e.starts_with(&format!("line {line}:")), "{bad:?}: {e}");
    }
}

#[test]
fn timing_line() {
    assert_eq!(timing("# timing: original\n0: -"), Some("original"));
    assert_eq!(timing("# timing: app\n"), Some("app"));
    assert_eq!(timing("# timing: engine. W1-1 planned by …"), Some("engine"));
    assert_eq!(timing("# W2-1 played to its exit\n"), None);
    assert_eq!(timing(""), None);
}

fn key(cell: Cell, band: u16) -> CellKey {
    CellKey {
        col: cell.0 / 2,
        row: cell.1 / 2,
        band,
        switches: 0,
        wings: false,
        via: 0,
    }
}

#[test]
fn guide_weights() {
    // A path along row 40, coins known; the last points share a band (0 coins left).
    let pts: Vec<GuidePoint> = (0..30u16)
        .map(|k| GuidePoint {
            cell: (10 + 2 * k, 40),
            coins_left: Some(if k < 20 { 12 } else { 0 }),
        })
        .collect();
    let g = GuideIndex::new(&pts);
    let p_max = 29;
    let first = key(pts[0].cell, band(12));
    let last = key(pts[29].cell, 0);
    assert_eq!(g.index(&first), Some(1), "point 1's region covers point 0's cell");
    assert_eq!(g.index(&last), Some(29));
    assert!(g.weight(&last, p_max) > g.weight(&first, p_max));
    assert!((g.weight(&last, p_max) - 3.0).abs() < 1e-12);
    // Off the path: far rows, or the right cell in another band.
    let off = key((40, 90), 0);
    assert_eq!(g.index(&off), None);
    assert_eq!(g.weight(&off, p_max), 0.15);
    assert_eq!(g.index(&key(pts[29].cell, band(12))), None, "a point with coins: its band only");
    assert_eq!(g.index(&key(pts[0].cell, 0)), None);
    // The region is the coarse cell ±1.
    assert_eq!(g.index(&key((pts[0].cell.0, 40 + 2), band(12))), Some(1));
    assert_eq!(g.index(&key((pts[0].cell.0, 40 + 4), band(12))), None);
    // A point without coins matches any band.
    let any = GuideIndex::new(&[GuidePoint {
        cell: (50, 50),
        coins_left: None,
    }]);
    for b in [0, 3, 12] {
        assert_eq!(any.index(&key((50, 50), b)), Some(0));
    }
}

/// `complete-w1-1` on the original from `AtLevel`: a point every 8 frames from the level's start
/// cell to near its exit, coins falling on the way.
#[test]
fn trace_original_of_complete_w1_1() {
    let script = InputScript::load(&sf_conform::case::script_path("complete-w1-1")).unwrap();
    let frames = script.lines.last().unwrap().0;
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id("W1-1")), &script, frames + 100)
        .unwrap()
        .amiga;
    let t = trace_original(&mut amiga, &script, frames).unwrap();
    // The level ends near frame 4473 (the script's header): its points stop there.
    assert!(t.len().abs_diff(4473 / 8) <= 1, "a point before every 8th frame of play: {}", t.len());
    let (g, fa) = at_start(id("W1-1"));
    assert_eq!(t[0].0, frog_cell(g.state()), "the first point is the level's start cell");
    assert_eq!(t[0].1, g.state().w(sf_core::layout::h0(0x832)));
    let end = t.last().unwrap();
    assert_eq!(end.1, 0, "the coins are met at the end");
    assert!(
        fa.exits
            .iter()
            .any(|x| x.0.abs_diff(end.0.0) <= 2 && x.1.abs_diff(end.0.1) <= 2),
        "the last point {:?} is at an exit {:?}",
        end.0,
        fa.exits
    );
}

/// The guide's trace, from a committed original-timed script at its level's `AtLevel`.
fn guide_of(script: &str, level: &str) -> Vec<GuidePoint> {
    let script = InputScript::load(&sf_conform::case::script_path(script)).unwrap();
    let frames = script.lines.last().unwrap().0;
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id(level)), &script, frames + 100)
        .unwrap()
        .amiga;
    points_from_trace(&trace_original(&mut amiga, &script, frames).unwrap())
}

/// M8 guided: a fixed number of rounds gives the same archive with one thread and with eight.
#[test]
fn guided_is_deterministic() {
    let guide = guide_of("complete-w1-1", "W1-1");
    let run = |threads| {
        let (g, fa) = at_start(id("W1-1"));
        let mut ex = Explorer::new(
            g,
            fa,
            Options {
                guide: guide.clone(),
                ..Options::default()
            },
        );
        for _ in 0..12 {
            assert!(matches!(run_round(&mut ex, threads), Progress::Working));
        }
        let keys: Vec<CellKey> = ex.archive().entries().iter().map(|e| e.key).collect();
        (keys, ex.best_partial())
    };
    let a = run(1);
    assert!(a.0.len() > 50);
    assert_eq!(a, run(8));
    assert_eq!(a, run(8));
}

/// W6-2 engine-only, guided by `enemies-w6-2-complete`'s trace on the original, exits within 5
/// minutes (the experiment: 80 s, 10 threads). Release, 10 threads: see FINDINGS for the time.
#[test]
#[ignore]
fn w6_2_guided_exits() {
    let guide = guide_of("enemies-w6-2-complete", "W6-2");
    let (g, fa) = at_start(id("W6-2"));
    let t0 = Instant::now();
    let run = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(300),
        ..RunOptions::default()
    };
    let opt = Options {
        guide,
        ..Options::default()
    };
    let r = plan_engine(g, fa, opt, &run).expect("W6-2 exits within 5 minutes, guided");
    eprintln!(
        "W6-2 guided: {:.0} s, {} rounds, {} cells, route {} frames, {} engine frames",
        t0.elapsed().as_secs_f64(),
        r.rounds,
        r.cells,
        r.inputs.len(),
        r.engine_frames
    );
}
