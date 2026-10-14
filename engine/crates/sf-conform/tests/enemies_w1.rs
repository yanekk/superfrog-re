//! World 1's enemies in frame cases (T42): one case per level, `engine/conform/cases/enemies-w1-*`,
//! run against the original with every object live. Each test checks the case is green but for
//! the re-syncs it pins (open lag faults, owner 2026-10-14),
//! and that the enemies it is meant to meet did what it says: their updates ran, the frog was
//! hurt, and the ones it kills died (their killed or stomped updates ran).

use std::collections::BTreeSet;

use sf_core::frog::{FROG, f};
use sf_core::layout::h0;
use sf_core::level::objects::OBJ_SLOTS;

/// What a case's run showed: every slot update the engine ran (by `h0` offset), how many
/// times the frog went from unhurt to hurt, and at each of those the update of the live slot
/// nearest the frog (what hurt it, or none on the level's drop-in).
struct Seen {
    updates: BTreeSet<u32>,
    hurts: u32,
    hurt_by: Vec<Option<u32>>,
}

/// Run `name`, green but for re-syncs at exactly `resyncs`, and say what the engine's run showed.
fn run(name: &str, resyncs: &[u32]) -> Seen {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut seen = Seen {
        updates: BTreeSet::new(),
        hurts: 0,
        hurt_by: Vec::new(),
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
            // The frog's middle against a slot's top middle, in map pixels.
            let w = |a| s.w(a) as i16 as i32;
            let (fx, fy) = (w(FROG + f::X) + 0x17, w(FROG + f::Y) + 0x2a);
            let near = (0..20)
                .map(|k| OBJ_SLOTS + k * 0x7c)
                .filter(|&slot| s.l(slot + 0x40) != 0)
                .map(|slot| {
                    let (x, y) = (w(slot + 8) + 8, w(slot + 0xa));
                    ((x - fx).abs() + (y - fy).abs(), s.l(slot + 4).wrapping_sub(h0(0)))
                })
                .min();
            seen.hurt_by.push(near.filter(|n| n.0 < 64).map(|n| n.1));
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
const HOVER: u32 = 0x243b6;
const WALKER: u32 = 0x2496a;
const WATCHER: u32 = 0x26904;
const PATROLLER: u32 = 0x26a2c;
const KILLED_FALL: u32 = 0x23b34;

/// W1-1: a sitter stomped into a reward; the runner and a spiker hurt the frog.
#[test]
fn w1_1() {
    let s = run("enemies-w1-1", &[372]);
    for u in [SITTER, RUNNER, SPIKER, STOMPED, REWARD] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 2, "hurt {} times", s.hurts);
}

/// W1-3: a hover hurts the frog under the start; the spring at 105,75 lifts it to the walker,
/// stomped into a reward, and on to the arena, where the watcher looks on and a patroller hurts
/// it; a hover dies falling.
#[test]
fn w1_3() {
    let s = run("enemies-w1-3", &[]);
    for u in [HOVER, WALKER, SPIKER, WATCHER, PATROLLER, KILLED_FALL, STOMPED, REWARD] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    for u in [HOVER, PATROLLER] {
        assert!(s.hurt_by.contains(&Some(u)), "never hurt by h0+${u:x}: {:x?}", s.hurt_by);
    }
}
