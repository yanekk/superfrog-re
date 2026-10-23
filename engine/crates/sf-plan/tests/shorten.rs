//! Route shortening (T22, DESIGN §2.12): the shortening budget, the shortcut pass, and M8 with
//! shortening on.
//!
//! The detoured route is T08's engine-timed W1-1 fixture with 32 frames right then 32 left spliced
//! in at frame 0: of detours tried at twelve points (24–48 frames each way), the one that still
//! exits on the engine.

use std::path::Path;
use std::time::{Duration, Instant};

use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, LevelId, Start};
use sf_plan::core::shorten::{self, Shortcut, budget, replay_exit};
use sf_plan::core::*;
use sf_plan::run::{RunOptions, over_slack, plan_engine, shortcut_pass};

/// The detour's frames, each way.
const DETOUR: usize = 32;

/// `id` entered and idled until the frog stands (as `tests/explore.rs`).
fn at_start(id: LevelId) -> (Game, Facts) {
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    (g, fa)
}

fn fixture() -> Vec<FrameInput> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/w1-1-engine.txt");
    let s = sf_conform::InputScript::load(&p).unwrap();
    let n = s.lines.last().unwrap().0;
    (0..n).map(|k| s.input_at(k)).collect()
}

/// The fixture with a walk out and back at its start.
fn detoured() -> Vec<FrameInput> {
    let mut r = vec![FrameInput::joy(Joy::RIGHT); DETOUR];
    r.extend(vec![FrameInput::joy(Joy::LEFT); DETOUR]);
    r.extend(fixture());
    r
}

fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

fn far() -> Instant {
    Instant::now() + Duration::from_secs(3600)
}

#[test]
fn budget_is_a_share_with_a_floor() {
    assert_eq!(budget(0, 0.0, 0), 0, "off");
    assert_eq!(
        budget(0, 0.2, 30),
        30,
        "an exit in round 0 gets the minimum"
    );
    assert_eq!(budget(100, 0.2, 30), 30);
    assert_eq!(budget(1000, 0.2, 30), 200);
    assert_eq!(budget(11, 0.2, 0), 2, "rounded");
}

/// The detoured route replays to the exit, so it is a fair input; the pass cuts at least the
/// detour and the result still exits, with no more hearts lost.
#[test]
fn a_detour_is_cut() {
    let (g, fa) = at_start("W1-1".parse().unwrap());
    let route = detoured();
    let mut f = 0;
    let (len, lost) = replay_exit(&g, &route, &mut f).expect("the detoured route exits");
    assert_eq!(len, route.len());
    let p = shortcut_pass(
        &g,
        &fa,
        &Options::default(),
        route.clone(),
        40,
        threads(),
        far(),
    );
    assert!(
        p.inputs.len() + 2 * DETOUR <= route.len(),
        "{} frames from {}: {} pairs tried, {} kept",
        p.inputs.len(),
        route.len(),
        p.pairs_tried,
        p.stretches_kept
    );
    assert_eq!(
        replay_exit(&g, &p.inputs, &mut f),
        Some((p.inputs.len(), p.hearts_lost))
    );
    assert!(p.hearts_lost <= lost);
    assert!(p.rounds <= 40);
}

/// A stretch that breaks the route is refused and the route stands; one that replays to the exit
/// is kept. Neither ever leaves more hearts lost than the route had.
#[test]
fn a_stretch_is_kept_only_if_the_route_still_exits() {
    let (g, fa) = at_start("W1-1".parse().unwrap());
    let route = detoured();
    let mut sc = Shortcut::new(&g, &fa, route.clone(), 1000);
    let lost = sc.hearts_lost();
    let mut bad = 0;
    let mut f = 0;
    // Offer each pair a stretch of standing still: most break the route (the frog is not where
    // the rest expects it), some may not; the route only ever gets shorter and keeps exiting.
    for _ in 0..12 {
        let Some((_, goal)) = sc.next() else { break };
        let Goal::Key { within_frames, .. } = goal else {
            panic!("a key goal")
        };
        let before = sc.route().to_vec();
        sc.offer(Some(vec![FrameInput::NONE; (within_frames / 2) as usize]));
        let after = sc.route();
        if after == before.as_slice() {
            bad += 1;
        } else {
            assert!(after.len() < before.len());
        }
        assert_eq!(
            replay_exit(&g, after, &mut f),
            Some((after.len(), sc.hearts_lost()))
        );
        assert!(sc.hearts_lost() <= lost);
    }
    assert!(bad > 0, "some stretch was refused");
    // A stretch no shorter than the pair it replaces is refused unreplayed.
    if let Some((_, Goal::Key { within_frames, .. })) = sc.next() {
        let before = sc.route().to_vec();
        sc.offer(Some(vec![FrameInput::NONE; within_frames as usize + 1]));
        assert_eq!(sc.route(), before.as_slice());
    }
}

/// The detoured route with frames `CUT..CUT + 8` dropped still exits, a heart down (found by a
/// review probe over cuts of 8–64 frames every 80 frames; the only one that exits losing a heart).
const CUT: usize = 4816;

/// A stretch that still reaches the exit but loses a heart the route did not is refused: the pair
/// spanning `CUT` is offered the route's own stretch less those 8 frames.
#[test]
fn a_stretch_losing_a_heart_is_refused() {
    let (g, fa) = at_start("W1-1".parse().unwrap());
    let route = detoured();
    let mut cut = route[..CUT].to_vec();
    cut.extend_from_slice(&route[CUT + 8..]);
    let mut f = 0;
    assert_eq!(replay_exit(&g, &cut, &mut f).map(|(_, lost)| lost), Some(1));
    let mut sc = Shortcut::new(&g, &fa, route.clone(), u64::MAX);
    assert_eq!(sc.hearts_lost(), 0);
    let mut offered = false;
    while let Some((s, goal)) = sc.next() {
        let Goal::Key { within_frames, .. } = goal else {
            panic!("a key goal")
        };
        let i = (s.frame() - g.frame()) as usize;
        let j = i + within_frames as usize + 1;
        if i <= CUT && CUT + 8 <= j {
            sc.offer(Some(cut[i..j - 8].to_vec()));
            offered = true;
            break;
        }
        sc.offer(None);
    }
    assert!(offered, "no pair spans the cut");
    assert_eq!(sc.route(), route.as_slice());
    assert_eq!(sc.hearts_lost(), 0);
}

/// The shortening search keeps a later exit only if it is shorter and loses no more hearts.
#[test]
fn a_shorter_exit_losing_more_hearts_is_not_better() {
    let r = |len, hearts_lost| Route {
        inputs: vec![FrameInput::NONE; len],
        engine_frames: 0,
        hearts_lost,
        rounds: 0,
        cells: 0,
    };
    assert!(shorten::better(&r(9, 0), &r(10, 0)));
    assert!(shorten::better(&r(9, 1), &r(10, 1)));
    assert!(!shorten::better(&r(10, 0), &r(10, 0)), "not shorter");
    assert!(!shorten::better(&r(9, 1), &r(10, 0)), "a heart down");
}

/// A route that does not reach the exit gives no pairs.
#[test]
fn a_route_short_of_the_exit_gives_nothing() {
    let (g, fa) = at_start("W1-1".parse().unwrap());
    let mut route = fixture();
    route.truncate(1000);
    let mut sc = Shortcut::new(&g, &fa, route, 1000);
    assert!(sc.next().is_none());
}

/// M8 for the shortcut pass: same seed and rounds, the same route with 1 thread and with 8.
#[test]
fn the_pass_is_the_same_on_any_threads() {
    let (g, fa) = at_start("W1-1".parse().unwrap());
    let pass = |threads| shortcut_pass(&g, &fa, &Options::default(), detoured(), 4, threads, far());
    let a = pass(1);
    assert_eq!(a, pass(8));
    assert!(a.pairs_tried >= 2, "{a:?}");
}

/// M8 with shortening on, through `plan_engine`: same seed and rounds, the same inputs with 1
/// thread and with 8. The search starts from the detoured route (an exit in round 0), so the
/// budget is the minimum. ~15 s (release, 10 cores).
#[test]
fn shortening_is_the_same_on_any_threads() {
    let plan = |threads| {
        let (g, fa) = at_start("W1-1".parse().unwrap());
        let opt = Options {
            seed_inputs: Some(detoured()),
            ..Options::default()
        };
        let run = RunOptions {
            threads,
            budget: Duration::from_secs(3600),
            shorten_share: 0.2,
            shorten_min_rounds: 6,
            shorten_min_over: None,
            lag_limit: sf_conform::LAG_LIMIT,
            max_rounds: None,
        };
        plan_engine(g, fa, opt, &run).unwrap()
    };
    let a = plan(1);
    let b = plan(8);
    assert_eq!(a.inputs, b.inputs);
    assert_eq!((a.rounds, a.hearts_lost), (b.rounds, b.hearts_lost));
    assert!(
        a.inputs.len() < detoured().len(),
        "{} frames",
        a.inputs.len()
    );
}

/// A later exit losing fewer hearts replaces a shorter one (DESIGN §2.5, M6; T14).
#[test]
fn a_longer_exit_losing_fewer_hearts_is_better() {
    let r = |len, hearts_lost| Route {
        inputs: vec![FrameInput::NONE; len],
        engine_frames: 0,
        hearts_lost,
        rounds: 0,
        cells: 0,
    };
    assert!(shorten::better(&r(14, 0), &r(10, 1)));
    assert!(shorten::better(&r(14, 1), &r(10, 3)));
    assert!(
        !shorten::better(&r(14, 1), &r(10, 1)),
        "as many hearts, longer"
    );
}

/// `shorten_min_over` (T14): the minimum applies to a first exit longer than the bound and its
/// slack (`over_slack`), never to one within them that loses no heart. The detoured route exits in round 0, so the shortening
/// search's rounds are the minimum or none.
#[test]
fn the_shortening_minimum_applies_only_over_the_bound() {
    assert_eq!(over_slack(1000), 1164);
    let n = detoured().len();
    let plan = |over| {
        let (g, fa) = at_start("W1-1".parse().unwrap());
        let opt = Options {
            seed_inputs: Some(detoured()),
            ..Options::default()
        };
        let run = RunOptions {
            threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
            budget: Duration::from_secs(3600),
            shorten_share: 0.2,
            shorten_min_rounds: 6,
            shorten_min_over: over,
            lag_limit: sf_conform::LAG_LIMIT,
            max_rounds: None,
        };
        plan_engine(g, fa, opt, &run).unwrap()
    };
    let within = plan(Some((n - 64, 0)));
    assert_eq!(within.rounds, 0, "an exit within the bound gets no minimum");
    assert_eq!(within.inputs.len(), n);
    let over = plan(Some((n / 2, 0)));
    assert!(over.rounds >= 6, "{} rounds", over.rounds);
    assert!(over.inputs.len() < n, "{} frames", over.inputs.len());
}
