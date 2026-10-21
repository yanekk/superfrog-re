//! `core::map` and `core::moves` against the disks' levels (T04).
//!
//! A whole level's map costs minutes of engine time (W1-1 ~3.5 M frames), over DESIGN §5's ~30 s
//! for the test block, so the block checks a bounded walk (W1-1's first layers: every edge
//! replayed, and the part split) and the whole-map checks are `#[ignore]`d with their times
//! noted; run them with `cargo test --release -p sf-plan --test map -- --ignored`.

use std::collections::BTreeSet;

use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, LevelId, Start};
use sf_plan::core::map::{Builder, Probe, node_of, reach, replay};
use sf_plan::core::*;

fn id(s: &str) -> LevelId {
    s.parse().unwrap()
}

/// `id` entered as the original enters it, idled until the frog stands (frame 151, as T00's
/// spike), and its facts there.
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

/// Runs one layer's parts on threads, in part order.
fn layer(b: &Builder, n: usize) -> Option<Vec<Vec<Probe>>> {
    let parts = b.parts(n);
    if parts.is_empty() {
        return None;
    }
    Some(std::thread::scope(|s| {
        let hs: Vec<_> = parts.iter().map(|p| s.spawn(move || b.build_part(p))).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    }))
}

/// An edge's identity across a replay: where it starts and ends and how.
type Seen = (Node, Node, Vec<(u8, u16)>);

/// Walks `layers` layers (all if `None`), replaying every move that made an edge from the very
/// seed it was measured from: it must end in its `to` node, landed (or through the exit), alive,
/// no hazard touched, taking exactly its recorded coins. Returns the map and the edges checked.
fn walk_replaying(start: &Game, fa: &Facts, layers: Option<u32>) -> (MoveMap, BTreeSet<Seen>) {
    let mut b = Builder::new(start, fa, &default_moves(true));
    let mut checked = BTreeSet::new();
    let mut k = 0;
    while layers.is_none_or(|l| k < l)
        && let Some(results) = layer(&b, threads() * 4)
    {
        let jobs = b.jobs().to_vec();
        let probes: Vec<&Probe> = results.iter().flatten().collect();
        let work: Vec<(usize, &Probe)> = probes.iter().copied().enumerate().collect();
        let fails: Vec<String> = std::thread::scope(|s| {
            let chunk = work.len().div_ceil(threads()).max(1);
            let hs: Vec<_> = work
                .chunks(chunk)
                .map(|c| {
                    let (b, jobs) = (&b, &jobs);
                    s.spawn(move || {
                        let mut bad = Vec::new();
                        for &(j, p) in c {
                            let Ok(found) = &p.result else { continue };
                            let (key, seed) = &b.frontier()[jobs[j].0 as usize];
                            let t = replay(seed, fa, &found.inputs);
                            let ok = t.reject.is_none()
                                && t.end == found.to
                                && (t.landed || t.exited)
                                && t.exited == found.exited
                                && t.coins == found.coins
                                && t.frames == found.frames;
                            if !ok {
                                bad.push(format!("{key:?} {:?}: {t:?}", found.inputs));
                            }
                        }
                        bad
                    })
                })
                .collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        assert!(fails.is_empty(), "{} edges fail replay: {:?}", fails.len(), &fails[..fails.len().min(5)]);
        for (j, p) in work {
            if let Ok(found) = &p.result {
                let from = b.frontier()[jobs[j].0 as usize].0.node;
                checked.insert((from, found.to, found.inputs.clone()));
            }
        }
        b.absorb(results);
        k += 1;
    }
    (b.finish(), checked)
}

fn assert_map_edges_checked(map: &MoveMap, checked: &BTreeSet<Seen>) {
    assert!(!map.edges.is_empty());
    for e in &map.edges {
        let s = (map.nodes[e.from as usize], map.nodes[e.to as usize], e.inputs.clone());
        assert!(checked.contains(&s), "map edge never replayed: {e:?}");
        assert_eq!(e.frames, e.inputs.iter().map(|x| x.1 as u32).sum::<u32>(), "{e:?}");
        assert_ne!(e.from, e.to);
    }
}

#[test]
fn moves_expand_to_their_inputs() {
    let m = MoveFamily::RunJump { dir: Dir::Right, run_up: 8 }.expand();
    assert_eq!(m.holds, vec![(Joy::RIGHT, 8), (Joy::RIGHT | Joy::UP, 20)]);
    assert_eq!(m.tail, Joy::RIGHT);
    let f = MoveFamily::Fly { dir: Dir::Left, flaps: 2, hover: true }.expand();
    assert!(f.needs_wings);
    assert_eq!(f.holds.len(), 1 + 2 * 2);
    assert_eq!(f.holds[1], (Joy::FIRE, 4));
    let fl = MoveFamily::Float { dir: Dir::None, then: Dir::Right }.expand();
    assert_eq!(fl.tail, Joy::RIGHT | Joy::UP);
    // Flight only with wings; steering (a jump whose tail differs from its direction) always.
    let (no, yes) = (default_moves(false), default_moves(true));
    assert!(no.families.iter().all(|m| !m.needs_wings()));
    assert!(yes.families.len() > no.families.len());
    assert!(no.families.iter().any(|m| matches!(m, MoveFamily::Jump { dir, then, .. } if dir != then)));
    let set: BTreeSet<_> = yes.families.iter().collect();
    assert_eq!(set.len(), yes.families.len(), "a move twice");
}

/// Test block: W1-1's first two layers, every edge replayed exactly.
#[test]
fn w1_1_first_layers_replay() {
    let (g, fa) = at_start(id("W1-1"));
    let (map, checked) = walk_replaying(&g, &fa, Some(2));
    assert_map_edges_checked(&map, &checked);
    assert!(map.nodes.len() > 20, "{}", map.nodes.len());
    assert!(node_of(&map, g.state()).is_some());
    // Some edge takes a coin on the way (W1-1's start is among coins).
    assert!(map.edges.iter().any(|e| !e.coins.is_empty()));
}

/// The map is the same for any part count and for the threaded shell (M8, DESIGN §3.1). A quarter
/// of the move set, so three layers stay cheap: the split does not depend on which moves.
#[test]
fn parts_do_not_change_the_map() {
    let (g, fa) = at_start(id("W1-1"));
    let moves = MoveSet {
        families: default_moves(true).families.into_iter().step_by(4).collect(),
    };
    let bounded = |n: usize| {
        let mut b = Builder::new(&g, &fa, &moves);
        for _ in 0..3 {
            let parts = b.parts(n);
            let r = parts.iter().map(|p| b.build_part(p)).collect();
            b.absorb(r);
        }
        let stats = b.stats().clone();
        (b.finish(), stats)
    };
    let one = bounded(1);
    let eight = bounded(8);
    assert_eq!(one.0, eight.0);
    assert_eq!(one.1, eight.1);
    let mut b = Builder::new(&g, &fa, &moves);
    for _ in 0..3 {
        let r = layer(&b, 13).unwrap();
        b.absorb(r);
    }
    assert_eq!(b.finish(), one.0);
}

// ---- whole maps: `#[ignore]`, minutes each ----

/// Whole W1-1 map, every edge replayed: ~3.5 M frames built, as many again replayed; 414 s on 10
/// threads at load 26 (2026-10-21).
#[test]
#[ignore]
fn w1_1_every_edge_replays() {
    let (g, fa) = at_start(id("W1-1"));
    let (map, checked) = walk_replaying(&g, &fa, None);
    assert_map_edges_checked(&map, &checked);
    let start = node_of(&map, g.state()).unwrap();
    let r = reach(&map, &fa, start, u32::MAX, true);
    assert!(r.exit);
}

/// The whole map on `threads()` threads.
fn whole(level: &str) -> (Game, Facts, MoveMap) {
    let (g, fa) = at_start(id(level));
    let (map, _) = sf_plan::threads::build_threaded(&g, &fa, &default_moves(true), threads());
    (g, fa, map)
}

/// 200 edges per level, picked by a fixed stride, replayed from the seed a fresh walk measured them
/// from: the walk is deterministic, so a map edge's seed is the seed of the same walk's probe.
#[test]
#[ignore]
fn sample_edges_replay_on_w2_4_w4_2_w6_2() {
    for level in ["W2-4", "W4-2", "W6-2"] {
        let (g, fa) = at_start(id(level));
        let (map, checked) = walk_replaying(&g, &fa, None);
        assert_map_edges_checked(&map, &checked);
        let stride = (map.edges.len() / 200).max(1);
        assert!(map.edges.iter().step_by(stride).count() >= 200.min(map.edges.len()));
    }
}

#[test]
#[ignore]
fn w6_4_spikes_are_crossed_only_with_wings() {
    let (g, fa, map) = whole("W6-4");
    let hazards: BTreeSet<Cell> = fa.hazards.iter().copied().collect();
    assert!(map.nodes.iter().all(|n| !hazards.contains(&n.cell)));
    let row10: Vec<u16> = fa.hazards.iter().filter(|c| c.1 == 10).map(|c| c.0).collect();
    let (c0, c1) = (*row10.iter().min().unwrap(), *row10.iter().max().unwrap());
    // Over the spikes' columns at their row: only flight lands there or crosses them.
    for e in &map.edges {
        let (a, b) = (map.nodes[e.from as usize].cell, map.nodes[e.to as usize].cell);
        let crosses = a.1 <= 10 && b.1 <= 10 && a.0.min(b.0) < c0 && a.0.max(b.0) > c1;
        assert!(!crosses || e.needs_wings, "{e:?} {a:?} {b:?}");
    }
    let _ = g;
}

#[test]
#[ignore]
fn w4_2_exit_needs_the_switch_at_222_4() {
    let (g, fa, map) = whole("W4-2");
    let k = fa.switches.iter().position(|s| s.gate == (222, 4)).expect("switch 49");
    let start = node_of(&map, g.state()).unwrap();
    let all = (1u32 << fa.switches.len()) - 1;
    assert!(reach(&map, &fa, start, all, true).exit);
    assert!(!reach(&map, &fa, start, all & !(1 << k), true).exit);
}

/// D13: in the map alone, every normal level's exit is reachable through at least `coins_needed`
/// object coins (T03: only those count toward the exit).
#[test]
#[ignore]
fn every_level_reaches_its_exit_with_its_coins() {
    let data = sf_conform::game_data();
    let mut short = Vec::new();
    for &level in data.levels.keys() {
        let s = level.to_string();
        if level.alt || !(1..=6).any(|w| s.starts_with(&format!("W{w}-"))) {
            continue;
        }
        let (g, fa, map) = whole(&s);
        let start = node_of(&map, g.state()).unwrap();
        let r = reach(&map, &fa, start, u32::MAX, true);
        let objects = r
            .coins
            .iter()
            .filter(|&&k| matches!(fa.coins[k as usize].kind, CoinKind::Object(_)))
            .count();
        if !r.exit || objects < fa.coins_needed as usize {
            short.push(format!("{s}: exit {} coins {objects}/{}", r.exit, fa.coins_needed));
        }
    }
    assert!(short.is_empty(), "{short:?}");
}

/// T00's stall spots: all 44 of W1-4's stall coins are on the map (Q4); W6-3's map, wrong in the
/// spike without air steering, reaches the route cells 119,5 and 160,7 and the exit (D18).
#[test]
#[ignore]
fn t00_stall_spots() {
    let (g, fa, map) = whole("W6-3");
    let cells: BTreeSet<Cell> = map.nodes.iter().map(|n| n.cell).collect();
    let near = |c: Cell| cells.iter().any(|n| n.0.abs_diff(c.0) <= 1 && n.1.abs_diff(c.1) <= 1);
    assert!(near((119, 5)) && near((160, 7)));
    let start = node_of(&map, g.state()).unwrap();
    assert!(reach(&map, &fa, start, u32::MAX, true).exit);

    let (g, fa, map) = whole("W1-4");
    let start = node_of(&map, g.state()).unwrap();
    let r = reach(&map, &fa, start, u32::MAX, true);
    // Q4's stall coins are coins above open air and in the cave; the map takes every coin object
    // W1-4 has (T00: 122 of 122 cells touched by an edge).
    let objects = |ks: &mut dyn Iterator<Item = usize>| {
        ks.filter(|&k| matches!(fa.coins[k].kind, CoinKind::Object(_))).count()
    };
    let got = objects(&mut r.coins.iter().map(|&k| k as usize));
    let all = objects(&mut (0..fa.coins.len()));
    assert_eq!(got, all, "{got} of {all}");
}
