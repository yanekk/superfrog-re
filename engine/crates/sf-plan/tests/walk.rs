//! The walk compass (T28, D25): a breadth-first fill over the open cells, to the exit and to the
//! object coins.
//!
//! The whole-level M8 run is `#[ignore]`d (W1-1 to its exit on one thread is several minutes); run
//! it with `cargo test --release -p sf-plan --test walk -- --ignored`.

mod common;

use std::time::Duration;

use common::*;
use sf_plan::core::facts::{COLS, ROWS};
use sf_plan::core::walk::{SEALED, WalkCompass, fill};
use sf_plan::core::*;
use sf_plan::run::{RunOptions, plan_engine, run_round};

fn at((c, r): Cell) -> usize {
    r as usize * COLS as usize + c as usize
}

/// All open but a wall at column 10, rows 0–20.
fn walled() -> Vec<bool> {
    let mut open = vec![true; COLS as usize * ROWS as usize];
    for r in 0..=20 {
        open[at((10, r))] = false;
    }
    open
}

fn key(col: u16, row: u16, switches: u32) -> CellKey {
    CellKey {
        col,
        row,
        band: 0,
        switches,
        wings: false,
        via: 0,
    }
}

#[test]
fn fill_goes_around_a_wall() {
    let d = fill(&walled(), &[(12, 5)]);
    // Straight from 8,5 to 12,5 is 4, through the wall; walking goes down to row 21 and back up.
    assert_eq!(d[at((12, 5))], 0);
    assert_eq!(d[at((8, 5))], 4 + 2 * (21 - 5));
    assert_eq!(d[at((10, 5))], SEALED, "a wall cell is never reached");
    // Open space: Manhattan steps.
    assert_eq!(d[at((20, 30))], 8 + 25);
}

#[test]
fn exit_steps_least_over_the_key_cells() {
    let mut w = WalkCompass::from_grid(walled(), Vec::new(), vec![(12, 5)], Vec::new());
    // Key 4,2 covers cells 8–9 × 4–5; the nearest is 9,5.
    assert_eq!(w.exit_steps(&key(4, 2, 0)), Some(3 + 2 * (21 - 5)));
    assert_eq!(w.exit_steps(&key(6, 2, 0)), Some(0), "the key holding the exit");
}

#[test]
fn a_sealed_cell_has_no_walking_distance() {
    // A box of wall around 50,50; the exit outside it.
    let mut open = vec![true; COLS as usize * ROWS as usize];
    for c in 48..=53 {
        for r in 48..=53 {
            let edge = c == 48 || c == 53 || r == 48 || r == 53;
            open[at((c, r))] = !edge;
        }
    }
    let mut w = WalkCompass::from_grid(open.clone(), Vec::new(), vec![(5, 5)], vec![(0, (5, 6))]);
    assert_eq!(w.exit_steps(&key(25, 25, 0)), None, "cells 50–51 × 50–51 are boxed in");
    assert!(w.exit_steps(&key(10, 10, 0)).is_some());
    // The coin pull falls back to straight for a sealed frog: 45 + 44 cells.
    assert_eq!(w.coin_dist((50, 50), &[1], 0), 89.0);
    // No coin live: as `archive::coin_dist`.
    assert_eq!(w.coin_dist((50, 50), &[0], 0), f64::MAX);
    // Outside the box it walks: 5 + 4 steps from 10,10, less one from its neighbour 9,10 (the
    // least over the frog's cell and its 4 neighbours).
    assert_eq!(w.coin_dist((10, 10), &[1], 0), 8.0);
    // A gate in the box's wall opens it in its switch's set only.
    let gate = vec![vec![(53, 50)]];
    let mut g = WalkCompass::from_grid(open, gate, vec![(60, 50)], Vec::new());
    assert_eq!(g.exit_steps(&key(25, 25, 0)), None);
    assert_eq!(g.exit_steps(&key(25, 25, 1)), Some(60 - 51));
}

/// W2-1's one switch (lever 116,39) opens the gate at 143,37: its block's ground cells are open
/// in that switch's set only.
#[test]
fn a_gate_opens_only_in_its_switch_set() {
    let (g, fa) = at_start(id("W2-1"));
    assert_eq!(fa.switches.len(), 1);
    assert_eq!(fa.switches[0].gate, (143, 37));
    let w = WalkCompass::new(g.state(), &fa);
    let block: Vec<Cell> = (142..=146)
        .flat_map(|c| (35..=41).map(move |r| (c, r)))
        .filter(|&c| !w.is_open(c, 0))
        .collect();
    assert!(!block.is_empty(), "the gate is shut at the start");
    for &c in &block {
        assert!(w.is_open(c, 1), "{c:?} opens with the switch pressed");
    }
    // Outside the gate's block nothing changes.
    let wall = (0..ROWS)
        .flat_map(|r| (0..COLS).map(move |c| (c, r)))
        .find(|&c| !w.is_open(c, 0) && !(142..=146).contains(&c.0))
        .unwrap();
    assert!(!w.is_open(wall, 1));
}

/// On a real level the start key's walking distance is finite and never less than the least
/// straight distance over its 2×2 cells (a 4-neighbour walk is at least the Manhattan distance).
/// Not W6-4: its exit is behind a gate, so the start, no switch pressed, falls back to straight.
#[test]
fn the_start_walks_at_least_as_far_as_straight() {
    for level in ["W1-1", "W2-3"] {
        let (g, fa) = at_start(id(level));
        let k = cell_key(&fa, g.state(), 0);
        let mut w = WalkCompass::new(g.state(), &fa);
        let steps = w.exit_steps(&k).unwrap_or_else(|| panic!("{level}: the start walks to an exit"));
        let straight = (0..2)
            .flat_map(|dc| (0..2).map(move |dr| (k.col * 2 + dc, k.row * 2 + dr)))
            .flat_map(|(c, r)| fa.exits.iter().map(move |x| x.0.abs_diff(c) + x.1.abs_diff(r)))
            .min()
            .unwrap();
        assert!(steps >= straight, "{level}: {steps} steps, straight {straight}");
        // The explorer's start entry takes it.
        let ex = Explorer::new(
            g,
            fa,
            Options {
                compass: Compass::Walk,
                ..Options::default()
            },
        );
        assert_eq!(ex.archive().entries()[0].dist, steps as f64);
    }
}

fn walk() -> Options {
    Options {
        compass: Compass::Walk,
        ..Options::default()
    }
}

/// M8 at core level with the walk compass: a fixed number of rounds gives the same archive and
/// best partial route with one thread and with four.
#[test]
fn walk_same_archive_any_threads() {
    let run = |threads| {
        let (g, fa) = at_start(id("W1-1"));
        let mut ex = Explorer::new(g, fa, walk());
        for _ in 0..12 {
            if let Progress::Exit(_) = run_round(&mut ex, threads) {
                panic!("W1-1 exited within 12 rounds");
            }
        }
        let keys: Vec<(CellKey, u64, u64)> = ex
            .archive()
            .entries()
            .iter()
            .map(|e| (e.key, e.dist.to_bits(), e.coin_dist.to_bits()))
            .collect();
        (keys, ex.best_partial(), ex.compass_fills())
    };
    let (k1, p1, f1) = run(1);
    let (k4, p4, f4) = run(4);
    assert!(k1.len() > 50, "the search went somewhere: {} cells", k1.len());
    assert!(f1.0 >= 1, "the exit fill was made");
    assert_eq!(k1, k4);
    assert_eq!(p1, p4);
    assert_eq!(f1, f4);
}

/// M8 to the exit with the walk compass, 1 and 4 threads (the last 8 coins' walking pull
/// included). Several minutes on one thread.
#[test]
#[ignore]
fn walk_same_route_any_threads() {
    let plan = |threads| {
        let (g, fa) = at_start(id("W1-1"));
        let run = RunOptions {
            threads,
            budget: Duration::from_secs(1200),
            ..RunOptions::default()
        };
        plan_engine(g, fa, walk(), &run).unwrap()
    };
    let a = plan(4);
    let b = plan(1);
    assert_eq!(a.inputs, b.inputs);
    assert_eq!((a.engine_frames, a.rounds), (b.engine_frames, b.rounds));
}
