//! Route repair (T09, DESIGN §2.7): an old route's inputs seed the archive and its trace guides.
//!
//! The dropped-frame comparison plays W1-1 twice, guided and not (~1 min on 10 threads, release);
//! the set-B leg needs W2-1's whole-game snapshot. Both are `#[ignore]`d; run them with
//! `cargo test --release -p sf-plan --test repair -- --ignored --nocapture`.

mod common;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use common::{at_start, fixture, id, threads};
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::input::FrameInput;
use sf_core::{Game, Start};
use sf_plan::core::repair::{follow_options, points_from_trace, trace_engine};
use sf_plan::core::*;
use sf_plan::run::{RunOptions, plan_engine};
use sf_plan::trace::{script_inputs, trace_original};

#[test]
fn trace_engine_of_the_fixture_ends_at_an_exit() {
    let (g, fa) = at_start(id("W1-1"));
    let route = fixture();
    let t = trace_engine(&g, &fa, &route);
    assert_eq!(
        t[0],
        (
            frog_cell(g.state()),
            g.state().w(sf_core::layout::h0(0x832)),
            0
        )
    );
    assert!(t.len().abs_diff(route.len() / 8) <= 1, "{} points", t.len());
    let end = t.last().unwrap();
    assert_eq!(end.1, 0);
    assert!(
        fa.exits
            .iter()
            .any(|x| x.0.abs_diff(end.0.0) <= 2 && x.1.abs_diff(end.0.1) <= 2),
        "the last point {:?} is at an exit {:?}",
        end.0,
        fa.exits
    );
    // A trace stops at the level's end: inputs past it change nothing.
    let mut longer = route.clone();
    longer.extend([FrameInput::NONE; 200]);
    assert_eq!(trace_engine(&g, &fa, &longer), t);
}

#[test]
fn follow_options_sets_seed_and_guide() {
    let trace = [((3, 4), 9, 0), ((5, 4), 8, 1)];
    let inputs = vec![FrameInput::NONE; 5];
    let opt = Options {
        seed: 7,
        guide: vec![GuidePoint {
            cell: (1, 1),
            coins_left: None,
            switches: None,
        }],
        ..Options::default()
    };
    let o = follow_options(opt, inputs.clone(), &trace);
    assert_eq!(o.seed, 7);
    assert_eq!(o.seed_inputs, Some(inputs));
    assert_eq!(o.guide, points_from_trace(&trace));
    assert_eq!(
        o.guide[1],
        GuidePoint {
            cell: (5, 4),
            coins_left: Some(8),
            switches: Some(1),
        }
    );
}

/// An engine-timed route replayed from its own start through `follow_options` exits at once (the
/// seed alone) and comes back unchanged.
#[test]
fn followed_route_exits_unchanged() {
    let (g, fa) = at_start(id("W1-1"));
    let route = fixture();
    let opt = follow_options(
        Options::default(),
        route.clone(),
        &trace_engine(&g, &fa, &route),
    );
    let r = plan_engine(g, fa, opt, &RunOptions::default()).unwrap();
    assert_eq!(r.rounds, 0);
    assert_eq!(r.inputs, route);
}

/// A carried-state stand-in: the fixture with one input frame dropped no longer exits as replayed;
/// followed, the explorer exits in fewer engine frames than unguided, same level and seed.
#[test]
#[ignore]
fn followed_with_a_frame_dropped_beats_unguided() {
    let (g, fa) = at_start(id("W1-1"));
    let route = fixture();
    let trace = trace_engine(&g, &fa, &route);
    let mut dropped = route.clone();
    dropped.remove(DROP_AT);
    let run = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(600),
        ..RunOptions::default()
    };
    let opt = follow_options(Options::default(), dropped, &trace);
    let ex = Explorer::new(g.clone(), fa.clone(), opt.clone());
    assert!(
        ex.found().is_none(),
        "the dropped frame makes the seed miss the exit"
    );
    let t0 = Instant::now();
    let guided = plan_engine(g.clone(), fa.clone(), opt, &run)
        .unwrap_or_else(|(_, why)| panic!("guided exits: {why:?}"));
    let tg = t0.elapsed().as_secs_f64();
    let t0 = Instant::now();
    let unguided = plan_engine(g, fa, Options::default(), &run)
        .unwrap_or_else(|(_, why)| panic!("unguided exits: {why:?}"));
    let tu = t0.elapsed().as_secs_f64();
    eprintln!(
        "W1-1 one frame dropped at {DROP_AT}: followed {} engine frames, {} rounds, {tg:.0} s; \
         unguided {} engine frames, {} rounds, {tu:.0} s",
        guided.engine_frames, guided.rounds, unguided.engine_frames, unguided.rounds
    );
    assert!(guided.engine_frames < unguided.engine_frames);
}

/// The fixture frame dropped: mid-route, so the seed reaches far before it parts.
const DROP_AT: usize = 2000;

/// Done-when: W2-1's set-B leg (carried RNG, rust-engine FINDINGS T205), engine only, from its
/// whole-game snapshot, following its standalone route `w2-1-complete` (inputs from the leg's
/// first frame, trace on the original from `AtLevel`). The snapshot is `SF_LEG_SNAP` or sf-bench's
/// `~/.cache/superfrog-re/bench-legs/leg-04.snap` (`whole_game --until W2-2 --scratch <dir>`).
#[test]
#[ignore]
fn set_b_w2_1_followed() {
    let snap = std::env::var_os("SF_LEG_SNAP")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap())
                .join(".cache/superfrog-re/bench-legs/leg-04.snap")
        });
    let script = InputScript::load(&sf_conform::case::script_path("w2-1-complete")).unwrap();
    let frames = script.lines.last().unwrap().0;
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id("W2-1")), &script, frames + 100)
        .unwrap()
        .amiga;
    let (_, standalone) = at_start(id("W2-1"));
    let trace = trace_original(&mut amiga, &standalone, &script, frames).unwrap();
    let leg = sync::at_snapshot_file(&snap, 100).unwrap();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(
        display.clone(),
        Start::Transplant(sync::core_memory(leg.memory())),
    );
    // The leg's first frame is the level's first, before its play begins: played on with the
    // route's own inputs until the frog stands past the level-start freeze, as `at_start`.
    let mut inputs = script_inputs(&script);
    let mut k = 0;
    while !(landed(g.state()) && k > 150) && k < 400 {
        g.step_unseen(inputs[k]);
        k += 1;
    }
    inputs.drain(..k);
    let data = sf_conform::game_data();
    let fa = facts(&g, &data.levels[&id("W2-1")], display.rom());
    let opt = follow_options(Options::default(), inputs, &trace);
    let seeded = Explorer::new(g.clone(), fa.clone(), opt.clone());
    eprintln!(
        "W2-1 leg: the standalone route seeds {} cells, exits at once: {}",
        seeded.archive().entries().len(),
        seeded.found().is_some()
    );
    let t0 = Instant::now();
    let run = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(600),
        ..RunOptions::default()
    };
    let r = plan_engine(g, fa, opt, &run)
        .unwrap_or_else(|(_, why)| panic!("the W2-1 leg exits within 10 minutes: {why:?}"));
    eprintln!(
        "W2-1 leg followed: {:.0} s, {} rounds, {} cells, route {} frames, {} engine frames, {} hearts lost",
        t0.elapsed().as_secs_f64(),
        r.rounds,
        r.cells,
        r.inputs.len(),
        r.engine_frames,
        r.hearts_lost
    );
}
