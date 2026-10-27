//! Original mode's reach stretches (T24): a repair plans back onto the last whole plan's path
//! `HORIZON` frames on, not to the exit. The rules are unit tests; the explorer's reach goal runs
//! on T08's W1-1 fixture (~1 s) and the tail-less lockstep on W1-1's original start (~5 s).

mod common;

use common::{at_start, fixture, id};
use sf_core::input::FrameInput;
use sf_plan::core::repair::{TRACE_EVERY, points_from_trace, trace_engine};
use sf_plan::core::*;
use sf_plan::oracle::*;

fn key(col: u16, row: u16, band: u16, switches: u32) -> CellKey {
    CellKey {
        col,
        row,
        band,
        switches,
        wings: false,
        via: 0,
    }
}

#[test]
fn a_reach_target_is_a_horizon_on_and_leaves_the_end_to_an_exit_plan() {
    let h = HORIZON_POINTS;
    assert_eq!(HORIZON_POINTS * TRACE_EVERY as usize, HORIZON);
    assert_eq!(reach_target(10 * h, 0), Some(h));
    assert_eq!(reach_target(10 * h, 3 * h + 5), Some(4 * h + 5));
    // Under 1½ horizons left past the target: the repair plans to the exit.
    assert_eq!(reach_target(2 * h + h / 2, h), None);
    assert_eq!(reach_target(2 * h + h / 2 + 1, 0), Some(h));
    assert_eq!(reach_target(h, 0), None);
}

#[test]
fn a_guide_point_covers_its_coarse_cell_and_its_neighbours() {
    let p = GuidePoint {
        cell: (20, 31),
        coins_left: Some(12),
        switches: Some(1),
    };
    // Coarse (10, 15) ±1.
    assert!(p.covers(&key(10, 15, band(12), 1)));
    assert!(p.covers(&key(9, 16, band(12), 1)));
    assert!(p.covers(&key(11, 14, band(12), 1)));
    assert!(!p.covers(&key(12, 15, band(12), 1)));
    assert!(!p.covers(&key(10, 13, band(12), 1)));
    // Its band or a lower one (fewer coins left), as `GuideIndex` matches.
    assert!(p.covers(&key(10, 15, band(3), 1)));
    assert!(!p.covers(&key(10, 15, band(30), 1)));
    // Its switches when known; any when not.
    assert!(!p.covers(&key(10, 15, band(12), 0)));
    let any = GuidePoint {
        coins_left: None,
        switches: None,
        ..p
    };
    assert!(any.covers(&key(10, 15, band(80), 7)));
}

/// `Reference::locate`: the furthest point from `at` to `upto` whose region holds the frog, never
/// one behind `at` (the path may pass the same ground twice).
#[test]
fn a_reference_locates_the_frog_forward_of_where_it_was() {
    let pt = |c: u16| GuidePoint {
        cell: (c, 10),
        coins_left: None,
        switches: None,
    };
    // Points walking right two coarse cells a point, then back over the same ground.
    let cols = [0u16, 4, 8, 12, 16, 12, 8, 4, 0];
    let r = Reference {
        inputs: vec![FrameInput::NONE; cols.len() * TRACE_EVERY as usize],
        points: cols.iter().map(|&c| pt(c)).collect(),
        trace: Vec::new(),
        lost: vec![0, 0, 1, 1, 1, 2, 2, 2, 2],
        at: 1,
        hearts_lost: 0,
    };
    let at = |c: u16| key(c / 2, 5, 0, 0);
    assert_eq!(r.locate(&at(8), 3), 2);
    // With room to look further, the way back over col 8 is further along.
    assert_eq!(r.locate(&at(8), 100), 6);
    // Off the path, or only behind `at`: stays at `at`.
    assert_eq!(r.locate(&at(60), 100), 1);
    assert_eq!(r.locate(&at(0), 7), 1);
    assert_eq!(r.inputs_from(2).len(), (cols.len() - 2) * TRACE_EVERY as usize);
    assert!(r.inputs_from(100).is_empty());
    assert_eq!(r.lost_between(1, 4), 1);
    assert_eq!(r.lost_between(2, 4), 0);
    assert_eq!(r.lost_between(0, 100), 2);
}

/// The explorer's reach goal on W1-1's fixture: seeded with the route's inputs, it ends the moment
/// the frog enters the region of the route's 100th point (frame 800), with no round played.
#[test]
fn a_reach_goal_ends_where_the_frog_enters_the_points_region() {
    let (g, fa) = at_start(id("W1-1"));
    let inputs = fixture();
    let trace = trace_engine(&g, &fa, &inputs);
    let points = points_from_trace(&trace);
    let t = 100;
    let opt = Options {
        seed_inputs: Some(inputs.clone()),
        goal: Goal::Reach {
            point: points[t],
            within_frames: REACH_FRAMES,
            hearts: 0,
        },
        ..Options::default()
    };
    let ex = Explorer::new(g.clone(), fa.clone(), opt);
    let r = ex.found().expect("the seed reaches the point");
    assert_eq!(r.rounds, 0);
    assert!(r.inputs.len() <= t * TRACE_EVERY as usize, "{} frames", r.inputs.len());
    assert_eq!(r.inputs[..], inputs[..r.inputs.len()]);
    let mut e = g.clone();
    for &x in &r.inputs {
        e.step_unseen(x);
    }
    assert!(points[t].covers(&cell_key(&fa, e.state(), 0)));
    // One frame short, the frog was not yet in the region.
    let mut e = g.clone();
    for &x in &r.inputs[..r.inputs.len() - 1] {
        e.step_unseen(x);
    }
    assert!(!points[t].covers(&cell_key(&fa, e.state(), 0)));

    // A point further than `within_frames` from the start is not reached.
    let far = Options {
        seed_inputs: Some(inputs),
        goal: Goal::Reach {
            point: points[t],
            within_frames: 200,
            hearts: 0,
        },
        ..Options::default()
    };
    assert!(Explorer::new(g.clone(), fa.clone(), far).found().is_none());

    // Nor with more hearts than the start has: a reach is no worse for hearts than the plan.
    let hearts = g.state().w(sf_core::layout::h0(0x830));
    let greedy = Options {
        seed_inputs: Some(fixture()),
        goal: Goal::Reach {
            point: points[t],
            within_frames: REACH_FRAMES,
            hearts: hearts + 1,
        },
        ..Options::default()
    };
    assert!(Explorer::new(g, fa, greedy).found().is_none());
}

/// A reference's trace is `trace_engine`'s, the hearts lost read beside each point, and it starts
/// at the point of the frames the original used.
#[test]
fn a_reference_traces_the_plan_from_its_own_start() {
    let (g, fa) = at_start(id("W1-1"));
    let inputs = fixture();
    let plan = Route {
        inputs: inputs.clone(),
        engine_frames: 0,
        hearts_lost: 0,
        rounds: 0,
        cells: 0,
    };
    let r = Reference::new(&g, &fa, &plan, 803);
    assert_eq!(r.trace, trace_engine(&g, &fa, &inputs));
    assert_eq!(r.points, points_from_trace(&r.trace));
    assert_eq!(r.lost.len(), r.trace.len());
    assert_eq!(r.at, 100);
    assert!(r.lost.windows(2).all(|w| w[0] <= w[1]));
}

/// A reach stretch's plan has no lag tail: played to its end with the original agreeing, it ends
/// with no parting and every frame agreed; with the tail it would part at the tail's end. W1-1's
/// original start, 200 idle frames.
#[test]
fn a_tailless_lockstep_ends_at_the_plans_end() {
    let run = start_at_level(id("W1-1"), 2_000);
    let eng = sf_core::Game::with_display(
        sf_conform::display_data(),
        sf_core::Start::Transplant(sf_conform::sync::core_memory(run.amiga.memory())),
    );
    let plan = [FrameInput::NONE; 200];
    let no_lag = Lag {
        limit: u32::MAX,
        run: 0,
    };
    let mut amiga = run.amiga;
    let snap = amiga.snapshot();
    let mut frames = 0;
    let ls = lockstep_with(&mut amiga, eng.clone(), &plan, &mut frames, no_lag, 0);
    assert_eq!(ls.parting, None, "{ls:?}");
    assert!(!ls.exited);
    assert_eq!(ls.agreed, ls.fed.len());
    assert_eq!(ls.used_at(ls.agreed), plan.len());
    amiga.restore(&snap);
    let ls = lockstep(&mut amiga, eng, &plan, &mut frames, no_lag);
    assert_eq!(ls.parting.map(|p| p.0), Some(Parting::Apart));
}
