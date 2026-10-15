//! World 1's enemies in frame cases (T42): one case per level, `engine/conform/cases/enemies-w1-*`,
//! run against the original with every object live. Each test checks the case is green but for
//! the re-syncs it pins (open lag faults, owner 2026-10-14),
//! and that the enemies it is meant to meet did what it says: their updates ran, the frog was
//! hurt, and the ones it kills died (their killed or stomped updates ran). And two cases of one
//! thing each (T110): activation (`enemies-w1-activation`) and a hover's kill
//! (`enemies-w1-hover-kill`).

use std::collections::BTreeSet;

use sf_core::enemies::common::SPLAT;
use sf_core::frog::{FROG, f};
use sf_core::items::SCORE;
use sf_core::layout::{h0, h1};
use sf_core::level::objects::OBJ_SLOTS;

/// What a case's run showed: every slot update the engine ran (by `h0` offset), and how many
/// times an enemy hurt the frog: its hearts (`+$5a`, `frog_hurt`) went down. Not `f::HURT`
/// rising: a stunned landing sets that too, as the level's opening drop does (T109 review).
/// And at the end of each frame (`frames[n]` after frame `n`): every live slot's map cell
/// (`+$40`), update and bob animation (its bob's `+8`), and the score. At each hurt, the update
/// of the live slot nearest the frog (what hurt it), if one is within 64 pixels.
struct Seen {
    updates: BTreeSet<u32>,
    hurts: u32,
    frames: Vec<(Vec<Live>, u32)>,
    hurt_by: Vec<Option<u32>>,
}

/// A live slot at the end of a frame: the map cell it spawned from as (col, row), its update and
/// its bob's animation.
#[derive(Clone, Copy)]
struct Live {
    at: (u32, u32),
    update: u32,
    anim: u32,
}

/// The map's row 0 (`map_rows[1]`, h1) and a row's length in bytes, as `examples/complete.rs`.
const ROW0: u32 = h1(0x218e0 + 0x4c0);
const ROW_BYTES: u32 = 0x260;

impl Seen {
    /// The frames at whose end the slot spawned from (`col`, `row`) went away (`false`) or came
    /// back (`true`), after the first frame.
    fn comings_and_goings(&self, at: (u32, u32)) -> Vec<(u32, bool)> {
        let here = |n: usize| self.frames[n].0.iter().any(|l| l.at == at);
        (1..self.frames.len())
            .filter(|&n| here(n) != here(n - 1))
            .map(|n| (n as u32, here(n)))
            .collect()
    }
}

/// The frog's hearts, which `frog_hurt` takes one from.
const HEARTS: u32 = 0x5a;

/// Run `name`, green but for re-syncs at exactly `resyncs`, and say what the engine's run showed.
fn run(name: &str, resyncs: &[u32]) -> Seen {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut seen = Seen {
        updates: BTreeSet::new(),
        hurts: 0,
        frames: Vec::new(),
        hurt_by: Vec::new(),
    };
    let mut hearts = None;
    // Called before each frame is stepped: what it sees is the end of the frame before.
    let o = sf_conform::run_with(&case, |n, g| {
        let s = g.state();
        let mut live = Vec::new();
        for k in 0..20 {
            let slot = OBJ_SLOTS + k * 0x7c;
            let cell = s.l(slot + 0x40);
            if cell != 0 {
                let update = s.l(slot + 4).wrapping_sub(h0(0));
                seen.updates.insert(update);
                let off = cell.wrapping_sub(ROW0);
                live.push(Live {
                    at: (off % ROW_BYTES / 2, off / ROW_BYTES),
                    update,
                    anim: s.l(s.l(slot + 0x50) + 8),
                });
            }
        }
        if n > 0 {
            seen.frames.push((live, s.l(SCORE)));
        }
        let now = s.w(FROG + HEARTS);
        if hearts.is_some_and(|was| now < was) {
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
        hearts = Some(now);
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
const HOVER: u32 = 0x243b6;
const WALKER: u32 = 0x2496a;
const CRAWLER: u32 = 0x247b8;
const STOMPED: u32 = 0x23bee;
const REWARD: u32 = 0x23e6c;
const WATCHER: u32 = 0x26904;
const PATROLLER: u32 = 0x26a2c;

/// W1-1: a sitter stomped into a reward; the runner and a spiker hurt the frog.
#[test]
fn w1_1() {
    let s = run("enemies-w1-1", &[372]);
    for u in [SITTER, RUNNER, SPIKER, STOMPED, REWARD] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 2, "hurt {} times", s.hurts);
}

/// W1-2: a walker stomped; a hover, the crawler and a spiker hurt the frog.
#[test]
fn w1_2() {
    let s = run("enemies-w1-2", &[]);
    for u in [HOVER, WALKER, CRAWLER, SPIKER, STOMPED] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 3, "hurt {} times", s.hurts);
}

/// World 1's types W1-4 has and W1-1 does not (`enemies::w1`): the dropper and its drop, the spout
/// and its shots (one update for all three).
const DROPPER: u32 = 0x24dca;
const DROP: u32 = 0x24b20;
const SPOUT: u32 = 0x25358;
const SHOT: u32 = 0x24aac;
/// What a killed hover runs (`kill_fall`): it falls until it splats on the ground.
const KILL_FALL: u32 = 0x23b34;

/// W1-4: hurt by the sitter, a spiker and a spout's shot; a crawler stomped; the dropper drops;
/// hovers fly by.
/// No re-syncs: none of its lags is followed by a divergence.
#[test]
fn w1_4() {
    let s = run("enemies-w1-4", &[]);
    for u in [SITTER, SPIKER, HOVER, CRAWLER, STOMPED, SPOUT, SHOT, DROPPER, DROP] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 3, "hurt {} times", s.hurts);
}

/// W1-1, activation (DESIGN §2.7): right until the camera frees the three hovers by the start
/// (`obj_despawn_if_offscreen`; two of them first spawn as the frog moves off, at 150), back
/// left until each spawns again from its marker, on the original's frames (the case is green,
/// so the original's slots did the same; these are the frames `sf-conform record` shows). The
/// sitter at 35,42, spawned on the way out, is freed on the way back.
#[test]
fn w1_activation() {
    let s = run("enemies-w1-activation", &[]);
    let (out, back) = (false, true);
    let seen = |at| s.comings_and_goings(at);
    assert_eq!(seen((5, 30)), [(201, out), (295, back)], "the hover at 5,30");
    assert_eq!(seen((8, 29)), [(150, back), (207, out), (287, back)], "at 8,29");
    assert_eq!(seen((11, 29)), [(150, back), (219, out), (281, back)], "at 11,29");
    assert_eq!(seen((35, 42)), [(200, back), (285, out)], "the sitter");
    assert!(s.updates.contains(&HOVER) && s.updates.contains(&SITTER));
    assert_eq!(s.hurts, 0);
}

/// W1-3: the frog jumps from the start onto the hover at 57,51; one stomp is enough for
/// `touch_kill_1hit` (its hit points are `$1f`): it falls (`kill_fall`), splats on the ground,
/// and is freed with 200 points, its marker cleared. The frog is not hurt.
#[test]
fn w1_hover_kill() {
    let s = run("enemies-w1-hover-kill", &[]);
    let at = (57, 51);
    let first = |p: &dyn Fn(&Live) -> bool| {
        s.frames.iter().position(|(live, _)| live.iter().any(|l| l.at == at && p(l)))
    };
    let falls = first(&|l| l.update == KILL_FALL).expect("the hover never fell") as u32;
    let splat = first(&|l| l.anim == SPLAT).expect("the hover never splatted") as u32;
    assert_eq!(falls, 189);
    assert!(splat > falls, "splat at {splat}");
    assert_eq!(s.comings_and_goings(at), [(239, false)]);
    let score = |n: u32| s.frames[n as usize].1;
    assert_eq!(score(238), score(0), "points before the hover was freed");
    assert_eq!(score(239), score(0) + 200);
    assert_eq!(s.hurts, 0);
}

/// W1-3: a hover hurts the frog under the start; the spring at 105,75 lifts it to the walker,
/// stomped into a reward, and on to the arena, where the watcher looks on and a patroller hurts
/// it; a hover dies falling.
#[test]
fn w1_3() {
    let s = run("enemies-w1-3", &[]);
    for u in [HOVER, WALKER, SPIKER, WATCHER, PATROLLER, KILL_FALL, STOMPED, REWARD] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    for u in [HOVER, PATROLLER] {
        assert!(s.hurt_by.contains(&Some(u)), "never hurt by h0+${u:x}: {:x?}", s.hurt_by);
    }
}
