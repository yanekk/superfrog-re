//! World 3's enemies in frame cases (T44): `engine/conform/cases/enemies-w3-*`, run against the
//! original with every object live. Each test checks the case is green but for the re-syncs it
//! pins (T101's, as `enemies_w1` pins its own), and that the enemies it is meant to meet did what
//! it says: their updates ran, the frog was hurt, and the ones it kills died (their stomped
//! updates ran).

use std::collections::BTreeSet;

use sf_core::frog::{FROG, f};
use sf_core::layout::{h0, h1};
use sf_core::level::objects::OBJ_SLOTS;

/// What a case's run showed: every slot update the engine ran (by `h0` offset), how many
/// times the frog went from unhurt to hurt (a stunned landing counts too), and how many hearts
/// it lost (`+$5a`, which only `frog_hurt` takes: an enemy's hurt alone). And at the end of each
/// frame (`live[n]` after frame `n`), the map cell (col, row) every live slot spawned from (its
/// `+$40`).
struct Seen {
    updates: BTreeSet<u32>,
    hurts: u32,
    hearts_lost: u32,
    live: Vec<Vec<(u32, u32)>>,
}

/// The map's row 0 (`map_rows[1]`, h1) and a row's length in bytes, as `examples/complete.rs`.
const ROW0: u32 = h1(0x218e0 + 0x4c0);
const ROW_BYTES: u32 = 0x260;

impl Seen {
    /// The frames at whose end the slot spawned from (`col`, `row`) went away (`false`) or came
    /// back (`true`), after the first frame.
    fn comings_and_goings(&self, at: (u32, u32)) -> Vec<(u32, bool)> {
        let here = |n: usize| self.live[n].contains(&at);
        (1..self.live.len())
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
        hearts_lost: 0,
        live: Vec::new(),
    };
    let mut hurt = false;
    let mut hearts = None;
    // Called before each frame is stepped: what it sees is the end of the frame before.
    let o = sf_conform::run_with(&case, |n, g| {
        let s = g.state();
        let mut live = Vec::new();
        for k in 0..20 {
            let slot = OBJ_SLOTS + k * 0x7c;
            let cell = s.l(slot + 0x40);
            if cell != 0 {
                seen.updates.insert(s.l(slot + 4).wrapping_sub(h0(0)));
                let off = cell.wrapping_sub(ROW0);
                live.push((off % ROW_BYTES / 2, off / ROW_BYTES));
            }
        }
        if n > 0 {
            seen.live.push(live);
        }
        let now = s.w(FROG + f::HURT) != 0;
        if now && !hurt {
            seen.hurts += 1;
        }
        hurt = now;
        let now = s.w(FROG + HEARTS);
        if hearts.is_some_and(|was| now < was) {
            seen.hearts_lost += 1;
        }
        hearts = Some(now);
    })
    .unwrap_or_else(|d| panic!("{name}: {d}"));
    let got: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(got, resyncs, "{name}: {:?}", o.resynced);
    seen
}

/// The updates of world 3's types (`enemies::w3`), and what a stomped one runs.
const PAD: u32 = 0x27894;
const DART_WAIT: u32 = 0x25fa8;
const DART_FLY: u32 = 0x25f42;
const SHUFFLER: u32 = 0x278d8;
const SPIKER: u32 = 0x27d4c;
const HOPPER: u32 = 0x2861e;
const FLOW: u32 = 0x287a8;
const SWING: u32 = 0x2619c;
const SWING_LINK: u32 = 0x262be;
const JUMPER: u32 = 0x27a5e;
const STOMPABLE: u32 = 0x27cc0;
const CRUMBLE: u32 = 0x2747c;
const BOUNCER: u32 = 0x25caa;
const SPIKER_STAYING: u32 = 0x27da8;
const SCENERY_34: u32 = 0x249e8;
const DRIP: u32 = 0x287e0;
const STOMPED: u32 = 0x23bee;
const REWARD: u32 = 0x23e6c;

/// W3-1: the shuffler, the jumper and a stompable stomped (the jumper into a reward); the frog
/// hurt by a dart and the swing's ball.
#[test]
fn w3_1() {
    let s = run("enemies-w3-1", &[]);
    for u in [
        PAD, DART_WAIT, DART_FLY, SHUFFLER, SPIKER, HOPPER, FLOW, SWING, SWING_LINK, JUMPER,
        STOMPABLE, STOMPED, REWARD,
    ] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 3, "hurt {} times", s.hurts);
}

/// W3-2 to the staying spikers: the bouncer (88) and the staying spiker (97), which no other case
/// meets, and every other type in the level; a stompable stomped; the frog hurt by the hopper, the
/// swing's ball, a bouncer and a staying spiker.
#[test]
fn w3_2() {
    let s = run("enemies-w3-2", &[]);
    for u in [
        BOUNCER, SPIKER_STAYING, PAD, DART_WAIT, DART_FLY, SPIKER, HOPPER, FLOW, SWING, SWING_LINK,
        STOMPABLE, CRUMBLE, STOMPED,
    ] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hearts_lost >= 4, "lost {} hearts", s.hearts_lost);
}

/// W3-3 from the start to the cave below its middle: hoppers, flows, the swing, a spiker, a dart,
/// the bouncer, the shufflers (one stomped into a reward), crumbling platforms and a jumper; the
/// frog hurt on the way. The pads and the scenery below marker 14 stand in the level's lower left,
/// which the case does not reach (T115's completion meets them).
#[test]
fn w3_3() {
    let s = run("enemies-w3-3", &[256]);
    for u in [
        HOPPER, FLOW, SWING, SWING_LINK, SPIKER, DART_WAIT, DART_FLY, BOUNCER, SHUFFLER, STOMPED,
        REWARD, CRUMBLE, JUMPER,
    ] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 3, "hurt {} times", s.hurts);
}

/// W3-4 round the level to the exit room (T113): the types no other case meets, the scenery of
/// marker 17 (34) at 270,69, the scenery below marker 15 (90, at 95,41, whose update is the pads')
/// and the drips (99), and every other type in the level; two stompables stomped; the frog hurt.
#[test]
fn w3_4() {
    let s = run("enemies-w3-4", &[]);
    for u in [
        SCENERY_34, DRIP, CRUMBLE, PAD, FLOW, DART_WAIT, DART_FLY, SHUFFLER, SPIKER, HOPPER, SWING,
        SWING_LINK, JUMPER, STOMPABLE, STOMPED,
    ] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    let met = |at| s.live.iter().any(|l| l.contains(&at));
    assert!(met((95, 41)), "the scenery below marker 15 never spawned");
    assert!(s.hearts_lost >= 3, "lost {} hearts", s.hearts_lost);
}

/// W3-1, activation (DESIGN §2.7): the frog runs right past the hopper at 62,44 until the
/// camera's left edge frees it (`obj_despawn_if_offscreen`), steps back until it spawns again from
/// its marker, and on until it is freed again; then climbs past the spiker at 72,42 until the
/// camera's bottom edge frees it, drops until it spawns again, and climbs until it is freed
/// again. On the original's frames: the case is green, and it compares every slot (`obj_slots`)
/// each frame. Freed, not evicted: the slots are full only by the start, as the coins there spawn
/// (173–198).
#[test]
fn activation() {
    let s = run("enemies-w3-activation", &[]);
    let (out, back) = (false, true);
    let seen = |at| s.comings_and_goings(at);
    assert_eq!(seen((62, 44)), [(363, back), (461, out), (481, back), (515, out)], "the hopper");
    assert_eq!(seen((72, 42)), [(391, back), (573, out), (708, back), (817, out)], "the spiker");
    assert!(s.updates.contains(&HOPPER) && s.updates.contains(&SPIKER));
    assert!(s.live[300..].iter().all(|l| l.len() < 20), "a slot could have been evicted");
}

/// `name` run green to its end with re-syncs at exactly `pins` (as [`run`]), the level's outcome
/// first set on `exit_at`, the exit (`level_state` 1, back to 0 in the same pass), then two frames
/// more: `top_loop` going on (T183), the level completed with the points taken, no slot machine
/// (T115, as `enemies_w5_cases::completes`).
fn completes(name: &str, pins: &[u32], exit_at: u32) {
    use sf_core::Game;
    use sf_core::level::LevelOutcome;
    let mut case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut exit = None;
    let o = sf_conform::run_with(&case, |fr, g: &mut Game| {
        if exit.is_none() && g.level_outcome().is_some() {
            exit = Some((fr, g.level_outcome()));
        }
    })
    .unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, pins, "{name}: {:?}", o.resynced);
    assert!(
        matches!(exit, Some((f, Some(LevelOutcome::Completed { .. }))) if f == exit_at),
        "{name}: {exit:?}"
    );
    // Two frames on (the hook runs before each step): `top_loop` goes on to what follows (T183).
    case.frames += 2;
    let (mut outcome, mut unported) = (None, Vec::new());
    if let Err(d) = sf_conform::run_with(&case, |_, g: &mut Game| {
        outcome = g.level_outcome();
        unported = g.unported().to_vec();
    }) {
        panic!("{name}: {d}");
    }
    assert_eq!(unported, Vec::<&str>::new(), "{name}");
    assert!(
        matches!(outcome, Some(LevelOutcome::Completed { tally }) if !tally.slot_machine),
        "{name}: {outcome:?}"
    );
}

/// W3-1 played from its start to its exit and through the tally (T115): the exit at 5493, fire
/// taking the points. No re-sync (`bob_draw_vars`' at 1018 and the panel's at 1629 went with
/// T212's steps in `bob_draw` and `panel_update`). Its passes run past 105 frames' ends in a row from 1658 (the case's
/// `lag-limit`); alone it meets every milestone.
#[test]
fn enemies_w3_1_complete() {
    completes("enemies-w3-1-complete", &[], 5493);
}

/// W3-2 played from its start to its exit and through the tally (T115): the three switches, the
/// bottom corridor and the right half, the exit at 6994, fire taking the points. One re-sync
/// pinned, the frame after a lag: the play field buffer at 2758 (`bob_draw_vars`' at 6445 went
/// with T212). Alone it meets every milestone.
#[test]
fn enemies_w3_2_complete() {
    completes("enemies-w3-2-complete", &[2758], 6994);
}

/// W3-3 played from its start to its exit and through the tally (T115): the coins round the
/// level, the switch at 57,64 and along the bottom to the exit at 7530, fire taking the points.
/// No re-sync (the panel's at 4179 went with T212). Alone it meets every milestone.
#[test]
fn enemies_w3_3_complete() {
    completes("enemies-w3-3-complete", &[], 7530);
}

/// W3-4 played from its start to its exit and through the tally (T115): round the level as
/// `enemies-w3-4` goes, back up for the coins and down again to the exit at 17774, fire taking the
/// points. No re-sync (the panel's at 8779 went with T212; `obj_slots` at 16318 was
/// the bubble the frog let rise at 16317, ported by T204). Alone it meets every milestone.
#[test]
fn enemies_w3_4_complete() {
    completes("enemies-w3-4-complete", &[], 17774);
}

/// W3-3 to its exit on T115's first route (T195): green under checks 1–4, every object live.
/// Twice (4026, 5280) the original's copper interrupt holds the CPU through the frame's end with
/// the pass due and not begun; read as a pass run to its end, the engine alone ran a pass the
/// original had not and stayed one ahead, its slots in another order from 4053, and missed the
/// coin that leaves 17 at 6338 (taken at 6335 before T212, the original's 6338 since). Three
/// re-syncs pinned, each the frame after a lag (6979, 7429 and 16477 went with T212).
#[test]
fn enemies_w3_3_slot_order() {
    use sf_conform::Milestone;
    let path = sf_conform::cases_dir().join("enemies-w3-3-slot-order.toml");
    let case = sf_conform::Case::load(&path).unwrap_or_else(|e| panic!("{e}"));
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("{d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, [3634, 4028, 12237], "{:?}", o.resynced);
    let met = o.milestones.expect("check 4 ran");
    let coin = |&(f, m): &(u64, Milestone)| m == Milestone::CoinTaken { left: 17 } && f == 6338;
    assert!(met.iter().any(coin), "the coin leaving 17 at 6338");
    assert!(met.iter().any(|(_, m)| *m == Milestone::LevelExit { world: 3, level: 3 }));
}

/// The case's object slots, the engine run alone on the original's pacing (check 4's run), in
/// the original's order at the first frames from 4053 and 4900 that end a pass on both sides:
/// every slot's update routine, map cell and position (T195). And the frame the copper interrupt
/// held over is read as the pass not begun (`Pacing::Late` at its first call before a step, two
/// blanks met).
#[test]
fn enemies_w3_3_slots_alone_in_the_originals_order() {
    use sf_conform::sync;
    use sf_core::layout::Ram;
    use sf_core::{Game, Pacing, Start};
    let path = sf_conform::cases_dir().join("enemies-w3-3-slot-order.toml");
    let case = sf_conform::Case::load(&path).unwrap_or_else(|e| panic!("{e}"));
    let synced = sync::reach(&case.sync, &case.script, 2 * case.frames + 1000).unwrap();
    let mut amiga = synced.amiga;
    let mut g = Game::with_display(sf_conform::display_data(), Start::Transplant(synced.memory));
    // Each slot's update (`+4`), map cell (`+$40`), x and y (`+8`, `+$a`).
    let slots = |r: &dyn Fn(u32) -> u32| -> Vec<[u32; 4]> {
        (0..20)
            .map(|k| OBJ_SLOTS + k * 0x7c)
            .map(|s| [r(s + 4), r(s + 0x40), r(s + 8) >> 16, r(s + 0xa) >> 16])
            .collect()
    };
    let mut held = Vec::new();
    let mut seen = Vec::new();
    for f in 0..4950 {
        let input = case.script.input_at(f);
        let cap = sync::step_inserting(&mut amiga, input).unwrap();
        let mem = sync::core_memory(amiga.memory());
        let pacing = sync::pacing(&amiga.cpu(), &mem, &cap.paula);
        if let Pacing::Late { phase: 0, step: 0, blanks, .. } = pacing {
            held.push((f, blanks));
        }
        for _ in 0..amiga.frames_run() {
            g.step_paced(input, pacing);
        }
        let due = |from| f >= from && !seen.iter().any(|&(at, _, _)| at >= from);
        if (due(4053) || due(4900)) && !g.mid_pass() && !sync::mid_pass(&amiga.cpu(), &mem) {
            let long = |a: u32| u32::from_be_bytes([0, 1, 2, 3].map(|i| mem.byte(a + i)));
            seen.push((f, slots(&long), slots(&|a| g.state().l(a))));
        }
    }
    assert_eq!(held, [(4026, 2)]);
    let at: Vec<u32> = seen.iter().map(|s| s.0).collect();
    assert_eq!(at, [4054, 4901]);
    for (f, theirs, ours) in &seen {
        assert_eq!(ours, theirs, "frame {f}");
    }
}
