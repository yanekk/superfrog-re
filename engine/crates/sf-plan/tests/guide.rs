//! Guide paths (T09, DESIGN §2.11): waypoint files, the guide's weights, a script's trace on the
//! original, and the explorer steered by one.
//!
//! `trace_original` runs the oracle (~10 s for W1-1). W6-2 to its exit is `#[ignore]`d; run it with
//! `cargo test --release -p sf-plan --test guide -- --ignored`.

mod common;

use std::time::{Duration, Instant};

use common::{at_start, id, threads};
use sf_conform::{InputScript, SyncPoint, sync};
use sf_plan::core::repair::{TracePoint, points_from_trace, trace_engine};
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
        switches: None,
    };
    assert_eq!(
        p,
        vec![pt(10, 20, None), pt(11, 21, Some(7)), pt(13, 23, Some(0))]
    );
    assert_eq!(parse_waypoints("").unwrap(), vec![]);
    for (bad, line) in [
        ("1,2\n3\n", 2),
        ("1,2\n\n#\n4,x\n", 4),
        ("1,2,3,4", 1),
        ("-1,2", 1),
    ] {
        let e = parse_waypoints(bad).unwrap_err();
        assert!(e.starts_with(&format!("line {line}:")), "{bad:?}: {e}");
    }
}

#[test]
fn timing_line() {
    assert_eq!(timing("# timing: original\n0: -"), Some("original"));
    assert_eq!(timing("# timing: app\n"), Some("app"));
    assert_eq!(
        timing("# timing: engine. W1-1 planned by …"),
        Some("engine")
    );
    assert_eq!(timing("# W2-1 played to its exit\n"), None);
    assert_eq!(timing(""), None);
}

fn key(cell: Cell, band: u16) -> CellKey {
    key_sw(cell, band, 0)
}

fn key_sw(cell: Cell, band: u16, switches: u32) -> CellKey {
    CellKey {
        col: cell.0 / 2,
        row: cell.1 / 2,
        band,
        switches,
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
            switches: Some(0),
        })
        .collect();
    let g = GuideIndex::new(&pts);
    let p_max = 29;
    let first = key(pts[0].cell, band(12));
    let last = key(pts[29].cell, 0);
    assert_eq!(
        g.index(&first),
        Some(1),
        "point 1's region covers point 0's cell"
    );
    assert_eq!(g.index(&last), Some(29));
    assert!(g.weight(&last, p_max) > g.weight(&first, p_max));
    assert!((g.weight(&last, p_max) - 3.0).abs() < 1e-12);
    // Off the path: far rows, or the right cell in another band.
    let off = key((40, 90), 0);
    assert_eq!(g.index(&off), None);
    assert_eq!(g.weight(&off, p_max), 0.15);
    assert_eq!(
        g.index(&key(pts[29].cell, band(12))),
        None,
        "a point with coins: no frog with more coins left"
    );
    assert_eq!(
        g.index(&key(pts[0].cell, 0)),
        Some(1),
        "a frog with fewer coins left than the route had there is on it"
    );
    // The region is the coarse cell ±1.
    assert_eq!(g.index(&key((pts[0].cell.0, 40 + 2), band(12))), Some(1));
    assert_eq!(g.index(&key((pts[0].cell.0, 40 + 4), band(12))), None);
    // A point without coins matches any band, without switches any switch set.
    let any = GuideIndex::new(&[GuidePoint {
        cell: (50, 50),
        coins_left: None,
        switches: None,
    }]);
    for b in [0, 3, 12] {
        for sw in [0, 1, 3] {
            assert_eq!(any.index(&key_sw((50, 50), b, sw)), Some(0));
        }
    }
}

/// W1-4 guided by `complete-w1-4` (T23): the explorer took the coins in another order than the
/// route, so its frog held fewer coins left than the route had at the points it stood on. Matched by
/// band only, its guide index stopped at the last point with its own band (p712 of 921), every
/// coins-met cell was off the path, and the straight exit pull drew it to the dead end at 158,45.
/// A point matches its band and every lower one: a frog ahead on coins is on the path.
#[test]
fn a_frog_ahead_on_coins_keeps_its_place_on_the_path() {
    let pt = |c, coins| GuidePoint {
        cell: (c, 40),
        coins_left: Some(coins),
        switches: Some(0),
    };
    // Right along row 40 taking a coin every 4 points, 30 left to 21; then the coins met.
    let mut pts: Vec<GuidePoint> = (0..40u16).map(|k| pt(10 + 2 * k, 30 - k / 4)).collect();
    pts.extend((40..50u16).map(|k| pt(10 + 2 * k, 0)));
    let g = GuideIndex::new(&pts);
    let at = |k: usize, coins| g.index(&key(pts[k].cell, band(coins)));
    // As the route: the furthest point of the region in its band.
    assert_eq!(at(20, 25), Some(21));
    // Behind on coins: only the points with as many left (the first, 30 left).
    assert_eq!(at(0, 30), Some(1));
    assert_eq!(at(20, 30), None);
    // Ahead on coins, as a whole: the region's furthest point at its band or above.
    assert_eq!(at(20, 0), Some(21));
    assert_eq!(at(45, 0), Some(46));
    assert_eq!(at(45, 22), None, "the coins-met stretch wants them met");
    // A point without coins still matches any band.
    let mut with_any = pts.clone();
    with_any.push(GuidePoint {
        cell: pts[20].cell,
        coins_left: None,
        switches: None,
    });
    assert_eq!(
        GuideIndex::new(&with_any).index(&key(pts[20].cell, band(30))),
        Some(50)
    );
}

/// Points with and without coins or switches (a waypoint among a trace's points) on one cell: the
/// cell takes the furthest index of all of them, whichever kind it is.
#[test]
fn guide_index_is_furthest_across_point_kinds() {
    let known = |k| GuidePoint {
        cell: (20, 20),
        coins_left: Some(k),
        switches: Some(0),
    };
    let any = GuidePoint {
        cell: (20, 20),
        coins_left: None,
        switches: None,
    };
    let at = key_sw((20, 20), band(5), 0);
    assert_eq!(GuideIndex::new(&[known(5), any]).index(&at), Some(1));
    assert_eq!(GuideIndex::new(&[any, known(5)]).index(&at), Some(1));
    assert_eq!(
        GuideIndex::new(&[any, known(6), known(5)]).index(&at),
        Some(2)
    );
}

/// A route back over its own ground once a switch is pressed (W6-2): the ground after the press
/// is credited only to a frog that pressed it (owner, 2026-10-22).
#[test]
fn guide_regions_know_switches() {
    let pt = |c, sw| GuidePoint {
        cell: (c, 40),
        coins_left: Some(0),
        switches: Some(sw),
    };
    // Right to a switch at col 40, then back left over the same cells, the switch pressed.
    let mut pts: Vec<GuidePoint> = (0..=15).map(|k| pt(10 + 2 * k, 0)).collect();
    pts.extend((0..=15).map(|k| pt(40 - 2 * k, 1)));
    let g = GuideIndex::new(&pts);
    let start = (10, 40);
    assert_eq!(
        g.index(&key_sw(start, 0, 0)),
        Some(1),
        "unpressed: the way out"
    );
    assert_eq!(
        g.index(&key_sw(start, 0, 1)),
        Some(31),
        "pressed: the way back"
    );
    assert_eq!(
        g.index(&key_sw(start, 0, 2)),
        None,
        "another switch set is off the path"
    );
    // The switch's own cell is the furthest point of the way out, unpressed.
    assert_eq!(g.index(&key_sw((40, 40), 0, 0)), Some(15));
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
    let (g, fa) = at_start(id("W1-1"));
    let t = trace_original(&mut amiga, &fa, &script, frames).unwrap();
    // The level ends near frame 4473 (the script's header): its points stop there.
    assert!(
        t.len().abs_diff(4473 / 8) <= 1,
        "a point before every 8th frame of play: {}",
        t.len()
    );
    assert_eq!(
        t[0].0,
        frog_cell(g.state()),
        "the first point is the level's start cell"
    );
    assert!(t.iter().all(|p| p.2 == 0), "W1-1 has no switches");
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

/// A committed original-timed script's trace at its level's `AtLevel`.
fn trace_of(script: &str, level: &str) -> Vec<TracePoint> {
    let script = InputScript::load(&sf_conform::case::script_path(script)).unwrap();
    let frames = script.lines.last().unwrap().0;
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id(level)), &script, frames + 100)
        .unwrap()
        .amiga;
    let (_, fa) = at_start(id(level));
    trace_original(&mut amiga, &fa, &script, frames).unwrap()
}

fn guide_of(script: &str, level: &str) -> Vec<GuidePoint> {
    points_from_trace(&trace_of(script, level))
}

/// W6-2's committed route presses switch 50 (at 96,4) and then 48 (at 126,8), the order its case
/// file names: the trace's switch bits go 0, 1, 3 and never back.
#[test]
fn trace_original_records_switches() {
    let t = trace_of("enemies-w6-2-complete", "W6-2");
    let mut seen: Vec<u32> = t.iter().map(|p| p.2).collect();
    seen.dedup();
    assert_eq!(seen, vec![0, 1, 3]);
    let pressed_at = |bits| t.iter().find(|p| p.2 == bits).unwrap().0;
    let near = |a: Cell, b: Cell| a.0.abs_diff(b.0) <= 1 && a.1.abs_diff(b.1) <= 1;
    assert!(near(pressed_at(1), (96, 4)), "{:?}", pressed_at(1));
    assert!(near(pressed_at(3), (126, 8)), "{:?}", pressed_at(3));
}

/// M8 guided: a fixed number of rounds gives the same archive on two threads and on eight (one
/// thread is `tests/explore.rs`'s; here it would take the test over 30 s).
#[test]
fn guided_is_deterministic() {
    // The fixture's engine trace: the oracle's would cost the test its 30 s on a busy machine.
    let guide = {
        let (g, fa) = at_start(id("W1-1"));
        points_from_trace(&trace_engine(&g, &fa, &common::fixture()))
    };
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
        for _ in 0..8 {
            assert!(matches!(run_round(&mut ex, threads), Progress::Working));
        }
        let keys: Vec<CellKey> = ex.archive().entries().iter().map(|e| e.key).collect();
        (keys, ex.best_partial())
    };
    let a = run(2);
    assert!(a.0.len() > 50);
    assert_eq!(a, run(8));
    assert_eq!(a, run(8));
}

/// W6-2 engine-only, guided by `enemies-w6-2-complete`'s trace on the original, exits within 5
/// minutes (the experiment: 80 s, 10 threads). Release, 10 threads: ~2 min (FINDINGS, T09); with
/// no switches in the guide every seed tried stalled at a closed gate.
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
    let r = plan_engine(g, fa, opt, &run)
        .unwrap_or_else(|(_, why)| panic!("W6-2 exits within 5 minutes, guided: {why:?}"));
    eprintln!(
        "W6-2 guided: {:.0} s, {} rounds, {} cells, route {} frames, {} engine frames",
        t0.elapsed().as_secs_f64(),
        r.rounds,
        r.cells,
        r.inputs.len(),
        r.engine_frames
    );
}
