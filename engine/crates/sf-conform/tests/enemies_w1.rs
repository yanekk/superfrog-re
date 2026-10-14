//! World 1's enemies in frame cases (T42): one case per level, `engine/conform/cases/enemies-w1-*`,
//! run against the original with every object live. Each test checks the case is green but for
//! the re-syncs it pins (open lag faults, owner 2026-10-14),
//! and that the enemies it is meant to meet did what it says: their updates ran, the frog was
//! hurt, and the ones it kills died (their killed or stomped updates ran).

use std::collections::BTreeSet;

use sf_core::frog::{FROG, f};
use sf_core::layout::h0;
use sf_core::level::objects::OBJ_SLOTS;

/// What a case's run showed: every slot update the engine ran (by `h0` offset), and how many
/// times the frog went from unhurt to hurt.
struct Seen {
    updates: BTreeSet<u32>,
    hurts: u32,
}

/// Run `name`, green but for re-syncs at exactly `resyncs`, and say what the engine's run showed.
fn run(name: &str, resyncs: &[u32]) -> Seen {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut seen = Seen {
        updates: BTreeSet::new(),
        hurts: 0,
    };
    let mut hurt = false;
    let o = sf_conform::run_with(&case, |_, g| {
        let s = g.state();
        for k in 0..20 {
            let slot = OBJ_SLOTS + k * 0x7c;
            if s.l(slot + 0x40) != 0 {
                seen.updates.insert(s.l(slot + 4).wrapping_sub(h0(0)));
            }
        }
        let now = s.w(FROG + f::HURT) != 0;
        if now && !hurt {
            seen.hurts += 1;
        }
        hurt = now;
    })
    .unwrap_or_else(|d| panic!("{name}: {d}"));
    let got: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(got, resyncs, "{name}: {:?}", o.resynced);
    seen
}

/// The updates of world 1's types (`enemies::w1`), and what a killed or stomped one runs.
const SITTER: u32 = 0x249c2;
const RUNNER: u32 = 0x24854;
const SPIKER: u32 = 0x248e2;
const STOMPED: u32 = 0x23bee;
const REWARD: u32 = 0x23e6c;

/// W1-1: a sitter stomped into a reward; the runner and a spiker hurt the frog.
#[test]
fn w1_1() {
    let s = run("enemies-w1-1", &[372]);
    for u in [SITTER, RUNNER, SPIKER, STOMPED, REWARD] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 2, "hurt {} times", s.hurts);
}

/// World 1's types W1-4 has and W1-1 does not (`enemies::w1`): the dropper and its drop, the spout
/// and its shots (one update for all three), the crawler.
const DROPPER: u32 = 0x24dca;
const DROP: u32 = 0x24b20;
const SPOUT: u32 = 0x25358;
const SHOT: u32 = 0x24aac;
const CRAWLER: u32 = 0x247b8;

/// W1-4: hurt by the sitter, a spiker and a spout's shot; a crawler stomped; the dropper drops.
/// No re-syncs: none of its lags is followed by a divergence.
#[test]
fn w1_4() {
    let s = run("enemies-w1-4", &[]);
    for u in [SITTER, SPIKER, CRAWLER, STOMPED, SPOUT, SHOT, DROPPER, DROP] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 3, "hurt {} times", s.hurts);
}
