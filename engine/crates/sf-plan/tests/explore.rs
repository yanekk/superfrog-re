//! The explorer (T08, DESIGN §2.4, D22) on the disks' levels.
//!
//! The whole-level runs are bounded: W1-1 to its exit is ~40 s on 10 threads; M8 compares a fixed
//! number of rounds in the block and the full exit `#[ignore]`d (one thread to W1-1's exit is
//! several minutes). Run the ignored ones with
//! `cargo test --release -p sf-plan --test explore -- --ignored`.

use std::path::Path;
use std::time::Duration;

use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, LevelId, Start};
use sf_plan::core::archive::{Offer, cell_key, replay};
use sf_plan::core::*;
use sf_plan::run::{RunOptions, plan_engine, run_round};

fn id(s: &str) -> LevelId {
    s.parse().unwrap()
}

/// `id` entered as the original enters it, idled until the frog stands (frame 151, as the map's
/// tests and the experiment), and its facts there. The explorer's routes count from here.
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

fn threads() -> usize {
    std::thread::available_parallelism().map_or(4, |n| n.get())
}

/// `rounds` rounds on `threads` threads; the archive's keys in order and the best partial route.
fn rounds(opt: Options, rounds: u64, threads: usize) -> (Explorer, Vec<CellKey>, (Route, Reason)) {
    let (g, fa) = at_start(id("W1-1"));
    let mut ex = Explorer::new(g, fa, opt);
    for _ in 0..rounds {
        if let Progress::Exit(_) = run_round(&mut ex, threads) {
            panic!("W1-1 exited within {rounds} rounds: the comparison wants a search still going");
        }
    }
    let keys = ex.archive().entries().iter().map(|e| e.key).collect();
    let partial = ex.best_partial();
    (ex, keys, partial)
}

/// The engine-timed W1-1 fixture's inputs, from `at_start`.
fn fixture() -> Vec<FrameInput> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/w1-1-engine.txt");
    let s = sf_conform::InputScript::load(&p).unwrap();
    let n = s.lines.last().unwrap().0;
    (0..n).map(|k| s.input_at(k)).collect()
}

fn joys(inputs: &[FrameInput]) -> Vec<u8> {
    inputs.iter().map(|x| x.joy.0).collect()
}

#[test]
fn w1_1_reaches_the_exit() {
    let (g, fa) = at_start(id("W1-1"));
    let run = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(120),
        ..RunOptions::default()
    };
    let r =
        plan_engine(g.clone(), fa, Options::default(), &run).expect("W1-1 exits within 2 minutes");
    // The route replays to the level's end, the frog alive.
    let mut t = g;
    replay(&mut t, &joys(&r.inputs));
    assert_ne!(
        t.state().l(sf_core::layout::h0(0xa06)),
        0xffff_fffe,
        "the route ends the level"
    );
    assert!(r.rounds > 0 && r.cells > 1 && r.engine_frames > r.inputs.len() as u64);
}

/// M8 at core level: a fixed number of rounds gives the same archive and best partial route with
/// one thread and with eight, and twice with eight.
#[test]
fn same_seed_same_archive_any_threads() {
    let (_, k1, p1) = rounds(Options::default(), 12, 1);
    let (_, k8, p8) = rounds(Options::default(), 12, 8);
    let (_, k8b, p8b) = rounds(Options::default(), 12, 8);
    assert!(
        k1.len() > 50,
        "the search went somewhere: {} cells",
        k1.len()
    );
    assert_eq!(k1, k8);
    assert_eq!(p1, p8);
    assert_eq!(k8, k8b);
    assert_eq!(p8, p8b);
    let (_, k2, _) = rounds(
        Options {
            seed: 2,
            ..Options::default()
        },
        12,
        8,
    );
    assert_ne!(k1, k2, "another seed searches differently");
}

/// M8 to the exit, 1 and 8 threads. ~4–6 min on one thread (release, quiet machine).
#[test]
#[ignore]
fn same_seed_same_route_any_threads() {
    let plan = |threads| {
        let (g, fa) = at_start(id("W1-1"));
        let run = RunOptions {
            threads,
            budget: Duration::from_secs(900),
            ..RunOptions::default()
        };
        plan_engine(g, fa, Options::default(), &run).unwrap().inputs
    };
    let a = plan(8);
    assert_eq!(a, plan(1));
    assert_eq!(a, plan(8));
}

/// D22: every checkpoint's state and every entry's, rebuilt after the states are dropped, equals
/// the one stored.
#[test]
fn rebuilt_states_equal_the_stored_ones() {
    let (mut ex, _, _) = rounds(Options::default(), 6, threads());
    let a = ex.archive();
    let n = a.checkpoints() as u32;
    assert!(n > 10, "checkpoints taken: {n}");
    let stored: Vec<(u32, Game)> = (0..n)
        .filter_map(|k| a.stored(k).map(|g| (k, g.clone())))
        .collect();
    let mut f = 0;
    let entries: Vec<Game> = (0..a.entries().len() as u32)
        .map(|i| a.entry_state(i, &mut f, &mut Vec::new()))
        .collect();
    ex.archive_mut().drop_states();
    let a = ex.archive();
    assert_eq!(a.stored_states(), 1, "only the start keeps its state");
    for (k, g) in &stored {
        let r = a.state_of(*k, &mut f);
        assert_eq!(r.frame(), g.frame(), "checkpoint {k}");
        assert!(
            r.state() == g.state(),
            "checkpoint {k}'s rebuilt state differs"
        );
    }
    for (i, g) in entries.iter().enumerate() {
        let r = a.entry_state(i as u32, &mut f, &mut Vec::new());
        assert!(
            r.state() == g.state() && r.frame() == g.frame(),
            "entry {i}'s rebuilt state differs"
        );
    }
}

/// DESIGN §2.4, §2.5: of two states reaching a cell, the more-hearted one is kept (then the one in
/// fewer frames).
#[test]
fn more_hearts_win_a_cell() {
    let (g, fa) = at_start(id("W1-1"));
    let key = cell_key(&fa, g.state(), 0);
    let mut a = Archive::new(&g, key, u64::MAX);
    let other = CellKey {
        col: key.col + 3,
        ..key
    };
    let inputs = [0u8; 10];
    let offer = |hearts, n: usize| Offer {
        key: other,
        hearts,
        coins_left: 5,
        base: 0,
        inputs: &inputs[..n],
        game: None,
    };
    assert_eq!(a.offer(offer(2, 10), 1).0, Some(1));
    assert_eq!(
        a.offer(offer(1, 4), 1).0,
        None,
        "fewer hearts lose, even sooner"
    );
    assert_eq!(a.get(&other).unwrap().hearts, 2);
    assert_eq!(
        a.offer(offer(3, 10), 1).0,
        Some(1),
        "more hearts win, even later"
    );
    assert_eq!(
        a.offer(offer(3, 8), 1).0,
        Some(1),
        "same hearts: fewer frames win"
    );
    assert_eq!(a.offer(offer(3, 9), 1).0, None);
    let e = a.get(&other).unwrap();
    assert_eq!((e.hearts, e.frames, e.reached), (3, 8, 5));
}

/// A burst that loses a heart goes on and its states are archived (DESIGN §2.5).
#[test]
fn a_heart_lost_is_archived() {
    let (ex, _, _) = rounds(Options::default(), 12, threads());
    let start = ex.archive().entries()[0].hearts;
    assert!(
        ex.archive().entries().iter().any(|e| e.hearts < start),
        "some archived state lost a heart"
    );
}

#[test]
fn coins_band() {
    assert_eq!(band(0), 0);
    for n in 1..=4 {
        assert_eq!(band(n), 1);
    }
    for n in 5..=8 {
        assert_eq!(band(n), 2);
    }
    assert_eq!(band(9), 3);
}

/// The frog's cells along `inputs` from `g`.
fn cells_along(g: &Game, inputs: &[u8]) -> Vec<Cell> {
    let mut t = g.clone();
    inputs
        .iter()
        .map(|&j| {
            t.step_unseen(FrameInput::joy(Joy(j)));
            frog_cell(t.state())
        })
        .collect()
}

/// `--via` (DESIGN §2.10): entries count the via cells passed in order; one that has passed both
/// passed the first before the second.
#[test]
fn via_cells_are_passed_in_order() {
    let (g, fa) = at_start(id("W1-1"));
    let (c, r) = frog_cell(g.state());
    let via = vec![(c + 8, r), (c + 2, r)];
    let mut ex = Explorer::new(
        g.clone(),
        fa,
        Options {
            via: via.clone(),
            ..Options::default()
        },
    );
    let mut k = 0;
    while !ex.archive().entries().iter().any(|e| e.key.via == 2) {
        k += 1;
        assert!(k <= 60, "both via cells passed within 60 rounds");
        run_round(&mut ex, threads());
    }
    let near = |a: Cell, b: Cell| a.0.abs_diff(b.0) <= 1 && a.1.abs_diff(b.1) <= 1;
    for (i, e) in ex.archive().entries().iter().enumerate() {
        let cells = cells_along(&g, &ex.archive().entry_inputs(i as u32));
        let first = cells.iter().position(|&x| near(x, via[0]));
        let then = first.and_then(|f| cells[f..].iter().position(|&x| near(x, via[1])));
        let passed = match (first, then) {
            (None, _) => 0,
            (Some(_), None) => 1,
            _ => 2,
        };
        assert_eq!(e.key.via, passed, "entry {i}");
    }
}

/// `--avoid` (DESIGN §2.10): a burst entering the box ends there, so with a box across the whole
/// map to the right of the start, nothing is archived past it, nor in it.
#[test]
fn avoid_box_ends_bursts() {
    let (g, fa) = at_start(id("W1-1"));
    let (c, _) = frog_cell(g.state());
    let wall = ((c + 6, 0), (c + 7, sf_plan::core::facts::ROWS - 1));
    let mut ex = Explorer::new(
        g.clone(),
        fa,
        Options {
            avoid: vec![wall],
            ..Options::default()
        },
    );
    for _ in 0..12 {
        run_round(&mut ex, threads());
    }
    let a = ex.archive();
    assert!(a.entries().len() > 20);
    for i in 0..a.entries().len() as u32 {
        let cells = cells_along(&g, &a.entry_inputs(i));
        assert!(
            cells.iter().all(|x| x.0 < c + 6),
            "entry {i} went into or past the box"
        );
    }
}

/// `Compass::Map` on a synthetic map: the start's distance is the edge frames to the exit node in
/// cells; a cell the map lacks falls back to straight distance; `Compass::Straight` needs no map.
#[test]
fn map_compass_gives_a_finite_pull() {
    let (g, fa) = at_start(id("W1-1"));
    let start = frog_cell(g.state());
    let exit = fa.exits[0];
    let nodes = vec![
        Node {
            cell: start,
            wings: false,
        },
        Node {
            cell: exit,
            wings: false,
        },
    ];
    let edge = Edge {
        from: 0,
        to: 1,
        inputs: vec![(0, 80)],
        frames: 80,
        coins: vec![],
        needs_switches: 0,
        needs_wings: false,
    };
    let map = MoveMap::new(nodes, vec![edge]);
    // The start's coins are not met; the distance is still the compass's.
    let ex = Explorer::new(
        g.clone(),
        fa.clone(),
        Options {
            compass: Compass::Map(map),
            ..Options::default()
        },
    );
    let d = ex.archive().entries()[0].dist;
    assert!(
        d.is_finite() && (d - 10.0).abs() < 1e-9,
        "80 edge frames, 8 a cell: {d}"
    );
    let straight = Explorer::new(g.clone(), fa.clone(), Options::default());
    let k = straight.archive().entries()[0].key;
    let s = fa
        .exits
        .iter()
        .map(|x| (k.col * 2).abs_diff(x.0) as f64 + (k.row * 2).abs_diff(x.1) as f64)
        .fold(f64::MAX, f64::min);
    assert_eq!(straight.archive().entries()[0].dist, s);
    // No path: an empty map gives straight distance.
    let empty = Explorer::new(
        g,
        fa,
        Options {
            compass: Compass::Map(MoveMap::default()),
            ..Options::default()
        },
    );
    assert_eq!(empty.archive().entries()[0].dist, s);
}

/// W1-1's real map as the compass (~150 s to build, then the search).
#[test]
#[ignore]
fn map_compass_plans_w1_1() {
    let (g, fa) = at_start(id("W1-1"));
    let (map, _) = sf_plan::threads::build_threaded(&g, &fa, &default_moves(true), threads());
    let run = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(600),
        ..RunOptions::default()
    };
    plan_engine(
        g,
        fa,
        Options {
            compass: Compass::Map(map),
            ..Options::default()
        },
        &run,
    )
    .unwrap();
}

/// D22: with the cap low the archive's own count stays under it, and the search is the same as
/// with the cap high.
#[test]
fn memory_cap_holds_and_changes_nothing() {
    let (g, _) = at_start(id("W1-1"));
    let cap = 12 * archive::state_size(&g) + (4 << 20);
    let (g, fa) = at_start(id("W1-1"));
    let mut ex = Explorer::new(
        g,
        fa,
        Options {
            max_mem_bytes: cap,
            ..Options::default()
        },
    );
    for _ in 0..12 {
        run_round(&mut ex, threads());
        assert!(
            ex.archive().stored_bytes() <= cap,
            "{} > {cap}",
            ex.archive().stored_bytes()
        );
    }
    let (pl, rl) = ex.best_partial();
    let low = ex
        .archive()
        .entries()
        .iter()
        .map(|e| e.key)
        .collect::<Vec<_>>();
    let (high, kh, (ph, rh)) = rounds(Options::default(), 12, threads());
    assert!(
        high.archive().stored_states() > 12,
        "the high cap kept more states"
    );
    assert_eq!(low, kh);
    assert_eq!(rl, rh);
    // Rebuilding costs engine frames; the route itself is the same.
    assert_eq!(
        (pl.inputs, pl.hearts_lost, pl.rounds, pl.cells),
        (ph.inputs, ph.hearts_lost, ph.rounds, ph.cells)
    );
}

/// `seed_inputs` (DESIGN §2.7): a route that exits on the engine seeds the archive and is the exit
/// at once, in round 0.
#[test]
fn seed_inputs_exit_in_round_zero() {
    let (g, fa) = at_start(id("W1-1"));
    let route = fixture();
    let opt = Options {
        seed_inputs: Some(route.clone()),
        ..Options::default()
    };
    let ex = Explorer::new(g.clone(), fa.clone(), opt.clone());
    let r = ex.found().expect("the seed alone exits");
    assert_eq!(r.rounds, 0);
    assert_eq!(r.inputs, route);
    assert!(
        ex.archive().entries().len() > 50,
        "the seed's states are archived"
    );
    let r = plan_engine(g, fa, opt, &RunOptions::default()).unwrap();
    assert_eq!((r.rounds, r.inputs), (0, route));
}

/// `Goal::Key` (T22's shortcut pass): a key a few cells from the start is reached within its frame
/// cap, and the inputs end on it.
#[test]
fn goal_key_is_reached_within_its_frames() {
    let (g, fa) = at_start(id("W1-1"));
    let mut t = g.clone();
    replay(&mut t, &[Joy::RIGHT.0; 48]);
    let key = cell_key(&fa, t.state(), 0);
    assert_ne!(
        key,
        cell_key(&fa, g.state(), 0),
        "the key is away from the start"
    );
    let goal = Goal::Key {
        key,
        within_frames: 120,
    };
    let run = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(60),
        ..RunOptions::default()
    };
    let r = plan_engine(
        g.clone(),
        fa.clone(),
        Options {
            goal,
            ..Options::default()
        },
        &run,
    )
    .unwrap();
    assert!(r.inputs.len() <= 120);
    let mut t = g;
    replay(&mut t, &joys(&r.inputs));
    assert_eq!(cell_key(&fa, t.state(), 0), key);
}
