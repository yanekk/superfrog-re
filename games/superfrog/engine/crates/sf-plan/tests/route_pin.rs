//! Route pins (forest-dumb-level-1 T02, DESIGN §3.6, §7): the routes the planner returns, held
//! across builds. The other determinism tests (`explore.rs`, `walk.rs`) compare runs within one
//! build, so a refactor that changes every route alike, the planner's move to the toolkit (T17)
//! included, would pass them. These compare against SHA-1s committed in `tests/pins/routes.txt`.
//!
//! A pin is a hash and nothing else: no disk data, no inputs (the routes are derived from the
//! disks). Each run is bounded by rounds, never by the clock, so it is the same on any machine
//! and thread count.
//!
//! Re-pinning: a pin changes only when a route is meant to change, and the commit that re-pins
//! says why. On a mismatch the failure prints the new line; paste it over the old one in
//! `tests/pins/routes.txt`. The whole-level pin is `#[ignore]`d (W1-1 to its exit, ~40 s on 10
//! threads, minutes on one): `cargo test --release -p sf-plan --test route_pin -- --ignored`.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use common::*;
use sf_core::input::FrameInput;
use sf_plan::core::{Compass, Explorer, Options, Progress, Reason, Route};
use sf_plan::run::{RunOptions, plan_engine, run_round};

/// Rounds of each short search: enough that the best partial route rests on several rounds'
/// merges over dozens of cells, few enough that the file stays well inside smoke's 30 s.
const ROUNDS: u64 = 6;

fn pins() -> BTreeMap<String, String> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/pins/routes.txt");
    std::fs::read_to_string(&p)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (k, v) = l.split_once(' ').expect("`<name> <sha1>`");
            (k.to_string(), v.trim().to_string())
        })
        .collect()
}

fn hex(h: &[u8; 20]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// Every field of a route, little-endian, in a fixed order: each frame's joystick byte and key
/// set, then the counts. `rounds`, `cells` and `engine_frames` are in too: a refactor that keeps
/// the route but changes how the search got there has changed the planner.
fn route_bytes(out: &mut Vec<u8>, r: &Route) {
    out.extend((r.inputs.len() as u64).to_le_bytes());
    for x in &r.inputs {
        let FrameInput { joy, keys } = *x;
        out.push(joy.0);
        out.extend(keys.0.to_le_bytes());
    }
    out.extend(r.engine_frames.to_le_bytes());
    out.push(r.hearts_lost);
    out.extend(r.rounds.to_le_bytes());
    out.extend(r.cells.to_le_bytes());
}

fn reason_bytes(out: &mut Vec<u8>, r: &Reason) {
    out.extend(r.coins_left.to_le_bytes());
    match r.exit_distance_with_coins_met {
        Some(d) => {
            out.push(1);
            out.extend(d.to_le_bytes());
        }
        None => out.push(0),
    }
    out.extend(r.cells.to_le_bytes());
}

/// `ROUNDS` rounds of the explorer from `level`'s start with `compass` and `seed`, on `threads`
/// threads: the bytes of its best partial route and why.
fn short_search(level: &str, seed: u64, compass: Compass, threads: usize) -> Vec<u8> {
    let (g, fa) = at_start(id(level));
    let opt = Options {
        seed,
        compass,
        ..Options::default()
    };
    let mut ex = Explorer::new(g, fa, opt);
    for _ in 0..ROUNDS {
        if let Progress::Exit(_) = run_round(&mut ex, threads) {
            panic!("{level} exited within {ROUNDS} rounds: pin a level still searching");
        }
    }
    let (route, reason) = ex.best_partial();
    assert!(
        reason.cells > 20,
        "{level}: the search went somewhere: {} cells",
        reason.cells
    );
    let mut out = Vec::new();
    route_bytes(&mut out, &route);
    reason_bytes(&mut out, &reason);
    out
}

/// SHA-1 of the routes the planner returns for `level` and `seed`: a short search with the
/// default (straight) compass, then one with the walk compass, on `threads` threads.
fn pinned_on(level: &str, seed: u64, threads: usize) -> [u8; 20] {
    let mut bytes = short_search(level, seed, Compass::Straight, threads);
    bytes.extend(short_search(level, seed, Compass::Walk, threads));
    amiga_disk::sha1::sha1(&bytes)
}

/// As `pinned_on`, on several threads (at least 2, so the merge of parallel parts is held).
fn pinned(level: &str, seed: u64) -> [u8; 20] {
    pinned_on(level, seed, threads().max(2))
}

/// Fails naming every pin that moved, with the lines to re-pin it.
fn check(got: &[(&str, [u8; 20])]) {
    let want = pins();
    let moved: Vec<String> = got
        .iter()
        .map(|(name, h)| (name, hex(h)))
        .filter(|(name, h)| want.get(**name) != Some(h))
        .map(|(name, h)| format!("{name} {h}"))
        .collect();
    assert!(
        moved.is_empty(),
        "route pins moved; if the change is meant, re-pin with the lines\n{}\nand say why in the \
         commit",
        moved.join("\n")
    );
}

/// The short searches on W1-1 and W2-1 (seed 1) and W1-1 on seed 2 give the pinned routes.
#[test]
fn short_routes_hold_their_pins() {
    check(&[
        ("W1-1/1", pinned("W1-1", 1)),
        ("W1-1/2", pinned("W1-1", 2)),
        ("W2-1/1", pinned("W2-1", 1)),
    ]);
}

/// One thread gives the same pin as several.
#[test]
fn one_thread_gives_the_same_pin() {
    check(&[("W1-1/1", pinned_on("W1-1", 1, 1))]);
}

/// W1-1 planned to its exit (default options, seed 1): the whole route the planner returns.
#[test]
#[ignore]
fn w1_1_exit_route_holds_its_pin() {
    let (g, fa) = at_start(id("W1-1"));
    let run = RunOptions {
        threads: threads(),
        budget: Duration::from_secs(1200),
        ..RunOptions::default()
    };
    let r = plan_engine(g, fa, Options::default(), &run).expect("W1-1 exits within 20 minutes");
    let mut bytes = Vec::new();
    route_bytes(&mut bytes, &r);
    check(&[("W1-1/1/exit", amiga_disk::sha1::sha1(&bytes))]);
}

/// The pins file holds a name and a SHA-1 a line and nothing else: no route, no input, nothing
/// derived from the disks beyond the hash.
#[test]
fn pins_are_hashes_only() {
    let p = pins();
    assert!(p.len() >= 4);
    for (name, h) in &p {
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-/".contains(c)),
            "{name}"
        );
        assert!(
            h.len() == 40 && h.chars().all(|c| c.is_ascii_hexdigit()),
            "{name}: {h}"
        );
    }
}
