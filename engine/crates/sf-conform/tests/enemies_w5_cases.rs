//! World 5's enemies in frame cases against the original (T39): the activation case, and each
//! level's encounter case (`engine/conform/cases/enemies-w5-*.toml`), with what each must meet
//! checked through the engine's state while it runs (green, so the original's is the same).

use sf_core::Game;
use sf_core::frog::{FROG, f};
use sf_core::layout::h0;

const SLOTS: u32 = h0(0x1ba6c);

/// What a case run saw: each frame, every live slot's update routine (h0 offsets), the frog
/// hurt flag and lives, the score; and every live slot's init routine (its template's, telling
/// apart types that share their updates, as darts 53 and 54 do).
#[derive(Default)]
struct Seen {
    frames: Vec<(u32, Vec<u32>, bool, u16)>,
    inits: std::collections::BTreeSet<u32>,
}

impl Seen {
    fn spawned(&self, update: u32) -> bool {
        self.frames.iter().any(|f| f.1.contains(&update))
    }

    /// A slot with `update` went away (freed, or turned to another routine) at some frame.
    fn left(&self, update: u32) -> bool {
        self.frames.windows(2).any(|w| {
            w[0].1.iter().filter(|&&u| u == update).count()
                > w[1].1.iter().filter(|&&u| u == update).count()
        })
    }

    /// A slot took `update` (spawned, or turned to it) after frame `from`.
    fn arrived_after(&self, update: u32, from: u32) -> bool {
        let n = |f: &(u32, Vec<u32>, bool, u16)| f.1.iter().filter(|&&u| u == update).count();
        self.frames
            .windows(2)
            .any(|w| w[1].0 > from && n(&w[1]) > n(&w[0]))
    }
}

/// Run `name` green (pinning the listed re-syncs: T121's in a play field buffer, and the panel's
/// after a lag, both with enemies frozen too), recording what it saw.
fn run(name: &str, pins: &[u32]) -> Seen {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut seen = Seen::default();
    let o = sf_conform::run_with(&case, |fr, g: &mut Game| {
        let st = g.state();
        let slots: Vec<u32> = (0..20)
            .map(|k| SLOTS + k * 0x7c)
            .filter(|&s| st.l(s + 0x40) != 0)
            .collect();
        let live = slots.iter().map(|&s| st.l(s + 4).wrapping_sub(h0(0))).collect();
        seen.inits
            .extend(slots.iter().map(|&s| st.l(s).wrapping_sub(h0(0))));
        seen.frames
            .push((fr, live, st.w(FROG + f::HURT) != 0, st.w(FROG + f::LIVES)));
    })
    .unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, pins, "{name}: {:?}", o.resynced);
    assert!(
        o.resynced
            .iter()
            .all(|r| r.1.starts_with("playfield_buffer_") || r.1.starts_with("panel_bitmap")),
        "{:?}",
        o.resynced
    );
    seen
}

/// Asserts `seen` had a slot spawned from each of `inits` (init routine, name).
fn met_init(seen: &Seen, inits: &[(u32, &str)]) {
    for &(i, what) in inits {
        assert!(seen.inits.contains(&i), "{what} (init h0+${i:x}) never met");
    }
}

/// Asserts `seen` met each of `types` (update routine, name).
fn met(seen: &Seen, types: &[(u32, &str)]) {
    for &(u, what) in types {
        assert!(seen.spawned(u), "{what} (h0+${u:x}) never met");
    }
}

/// W5-1 from its start to the second pacer (T39): over the first pacer and the spike pits, under
/// the watchers, the mace and the fallers (each shakes and falls as the frog comes near), past the
/// drips, the crushers, the flows and the dart; every W5-1 type but the shot. One re-sync pinned,
/// seen with enemies frozen too: the panel at 979 (the play field's at 551 was the bob drawer's
/// wait for the vertical blank's flag, T102).
#[test]
fn enemies_w5_1() {
    let seen = run("enemies-w5-1", &[979]);
    met(
        &seen,
        &[
            (0x26b14, "pacer (13/14)"),
            (0x26c08, "shooter walking"),
            (0x26cb6, "shooter firing"),
            (0x24f1c, "faller hanging"),
            (0x24f58, "faller shaking"),
            (0x24f88, "faller falling"),
            (0x24ff8, "faller down"),
            (0x2502a, "watcher looking"),
            (0x250d0, "watcher blinking"),
            (0x25274, "drip falling"),
            (0x25306, "drip splashing"),
            (0x270a8, "crusher waiting"),
            (0x270d2, "crusher striking"),
            (0x27100, "crusher going back"),
            (0x25fa8, "dart (hidden)"),
            (0x25f42, "dart (flying)"),
            (0x2619c, "mace"),
            (0x262be, "mace's chain"),
            (0x287a8, "flow"),
            (0x24ef6, "scenery"),
        ],
    );
}

/// W5-1: the frog runs into the first pacer and is hurt (frame 246), then waits by the shooter,
/// which fires twice (its shot, template 85, flies left under the frog).
#[test]
fn enemies_w5_1_hurt() {
    let seen = run("enemies-w5-1-hurt", &[]);
    assert!(
        seen.frames.windows(2).any(|w| w[0].0 >= 200 && w[1].2 && !w[0].2),
        "hurt by the pacer"
    );
    met(
        &seen,
        &[
            (0x26b14, "pacer (13/14)"),
            (0x26cb6, "shooter firing"),
            (0x26d14, "shot"),
        ],
    );
    assert!(seen.left(0x26d14), "a shot freed");
}

/// W5-1: right past the first pacer and the shooter, then back left: the pacer is freed once the
/// camera has left it and spawns again on the way back, the same frames as the original.
#[test]
fn enemies_w5_1_activation() {
    let seen = run("enemies-w5-1-activation", &[]);
    for (what, u) in [("pacer", 0x26b14), ("shooter", 0x26c08)] {
        assert!(seen.spawned(u), "{what}");
    }
    assert!(seen.left(0x26b14), "the pacer freed");
    assert!(seen.arrived_after(0x26b14, 600), "the pacer back");
}


/// W5-2 (T123) from its start west over the spike pits to the second dart (54, marker 19 at
/// 56,17), which hurts the frog (frame 383), down past the drips and the faller into the
/// shooters' room (it waits for the shooter's shot), under the crushers and east to the pacer
/// (13/14) on row 30. The 2/3 pacer cannot spawn in W5-2 (no marker 3 within `$98` cells right
/// of either marker 2); the mace, the flow and dart 53 are W5-1's (FINDINGS 2026-10-15, T123).
#[test]
fn enemies_w5_2() {
    let seen = run("enemies-w5-2", &[]);
    assert!(
        seen.frames.windows(2).any(|w| w[0].0 >= 200 && w[1].2 && !w[0].2),
        "hurt by an enemy"
    );
    met_init(&seen, &[(0x25e46, "dart 54 (marker 19)")]);
    met(
        &seen,
        &[
            (0x25fa8, "dart (hidden)"),
            (0x25f42, "dart (flying)"),
            (0x26b14, "pacer (13/14)"),
            (0x26c08, "shooter walking"),
            (0x26cb6, "shooter firing"),
            (0x26d14, "shot"),
            (0x24f1c, "faller hanging"),
            (0x24f58, "faller shaking"),
            (0x24f88, "faller falling"),
            (0x24ff8, "faller down"),
            (0x25274, "drip falling"),
            (0x25306, "drip splashing"),
            (0x270a8, "crusher waiting"),
            (0x270d2, "crusher striking"),
            (0x27100, "crusher going back"),
            (0x24ef6, "scenery"),
        ],
    );
    assert!(seen.left(0x26d14), "a shot freed");
}


/// W5-3 (T124) from its start west along the top past the first shooters and dart 54 (marker 19
/// at 96,16), down the shaft at col 133 to the watchers (22), east past the mace and the darts
/// (53) and down the chute into the bottom corridor, west to the switch at 149,58 and back east
/// to the pacer between markers 2 and 3 (80/79), which hurts the frog (frame 2571). One re-sync
/// pinned, seen with enemies frozen too: the panel at 1129 after a lag. The faller (28, marker 10
/// at 92,39) and the shooter's shot (85) are enemies-w5-1's and enemies-w5-2's (FINDINGS
/// 2026-10-15, T124).
#[test]
fn enemies_w5_3() {
    let seen = run("enemies-w5-3", &[1129]);
    assert!(
        seen.frames.windows(2).any(|w| w[0].0 >= 2500 && w[1].2 && !w[0].2),
        "hurt by the 2/3 pacer"
    );
    met_init(
        &seen,
        &[
            (0x26806, "pacer 79 (marker 3)"),
            (0x26840, "pacer 80 (marker 2)"),
            (0x25e46, "dart 54 (marker 19)"),
            (0x2500e, "watcher (marker 22)"),
        ],
    );
    met(
        &seen,
        &[
            (0x26874, "pacer (2/3)"),
            (0x26b14, "pacer (13/14)"),
            (0x2502a, "watcher looking"),
            (0x250d0, "watcher blinking"),
            (0x25fa8, "dart (hidden)"),
            (0x25f42, "dart (flying)"),
            (0x26c08, "shooter walking"),
            (0x25274, "drip falling"),
            (0x25306, "drip splashing"),
            (0x270a8, "crusher waiting"),
            (0x270d2, "crusher striking"),
            (0x27100, "crusher going back"),
            (0x2619c, "mace"),
            (0x262be, "mace's chain"),
            (0x287a8, "flow"),
            (0x24ef6, "scenery"),
        ],
    );
}

/// W5-4 (T125) from its start: up, then east down the ice slope, past the 2/3 pacers (79/80,
/// markers 2 and 3, spawned as the camera passes 17/28,19 and 64/81,36), to the switch at 98,37,
/// which opens the gate at 135,34; round the east side and back west along the top into the
/// watchers' room, where the template-30 fallers (marker 12) fall once in view and the one at
/// 150,9 hurts the frog (frame 4747). One re-sync pinned: the play field buffer at 414 (T121's
/// class; freezing enemies freezes nothing in W5, and with every object frozen there is no lag).
/// On the way it meets every other W5-4 type too.
#[test]
fn enemies_w5_4() {
    let seen = run("enemies-w5-4", &[414]);
    assert!(
        seen.frames.windows(2).any(|w| w[0].0 >= 200 && w[1].2 && !w[0].2),
        "hurt by an enemy"
    );
    met_init(
        &seen,
        &[
            (0x25146, "faller 30 (marker 12)"),
            (0x26806, "pacer 79 (marker 3)"),
            (0x26840, "pacer 80 (marker 2)"),
            (0x24f06, "faller 28 (marker 10)"),
            (0x2525e, "drip (marker 11)"),
            (0x24eca, "scenery (marker 15)"),
            (0x26aa6, "pacer 81 (marker 14)"),
            (0x26ae0, "pacer 82 (marker 13)"),
            (0x2708c, "crusher (marker 16)"),
            (0x26154, "mace (marker 17)"),
            (0x26242, "mace's chain"),
            (0x25e2a, "dart 53 (marker 18)"),
            (0x25e46, "dart 54 (marker 19)"),
            (0x26b8e, "shooter 83 (marker 21)"),
            (0x26bce, "shooter 84 (marker 20)"),
            (0x26cfa, "shot 85"),
            (0x2500e, "watcher (marker 22)"),
            (0x2872e, "flow (marker 24)"),
        ],
    );
    met(
        &seen,
        &[
            (0x2515c, "faller 30 hanging"),
            (0x251a8, "faller 30 shaking"),
            (0x251d8, "faller 30 falling"),
            (0x25248, "faller 30 down"),
            (0x26874, "pacer (2/3)"),
            (0x2502a, "watcher looking"),
            (0x24f1c, "faller 28 hanging"),
            (0x25274, "drip falling"),
            (0x26b14, "pacer (13/14)"),
            (0x270a8, "crusher waiting"),
            (0x2619c, "mace"),
            (0x25f42, "dart (flying)"),
            (0x26cb6, "shooter firing"),
            (0x26d14, "shot"),
            (0x287a8, "flow"),
        ],
    );
}

/// `name` run green to its end with re-syncs at exactly `pins` (as [`run`]), the level's outcome
/// first set on `exit_at`, the exit (`level_state` 1, back to 0 in the same pass), then two frames
/// more: `top_loop` going on (T183), the level completed with the points taken, no slot machine
/// (T126).
fn completes(name: &str, pins: &[u32], exit_at: u32) {
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
        matches!(exit, Some((f, Some(sf_core::level::LevelOutcome::Completed { .. }))) if f == exit_at),
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
        matches!(
            outcome,
            Some(sf_core::level::LevelOutcome::Completed { tally }) if !tally.slot_machine
        ),
        "{name}: {outcome:?}"
    );
}

/// W5-1 played from its start to its exit and through the tally (T126): the exit at 2254, fire
/// taking the points. One re-sync pinned: the panel at 2182 after a lag (as `enemies-w5-1`'s at
/// 979); the play field buffer's at 1497 was T184's (the engine's pass a frame off). Frame 744's
/// picture (the frog's sprites, `enemies-w5-1` 595's class) is within the faithful picture
/// tolerance (T181).
#[test]
fn enemies_w5_1_complete() {
    completes("enemies-w5-1-complete", &[2182], 2254);
}

/// W5-2 played from its start to its exit and through the tally (T126): the exit at 6095, fire
/// taking the points. No re-sync since T184 kept the engine's pass in step after late frames
/// (2196 and 4681 were its class). The frog on the spring pad (967) and the exit fade's palette
/// step (6095) are within the faithful picture tolerance.
#[test]
fn enemies_w5_2_complete() {
    completes("enemies-w5-2-complete", &[], 6095);
}

/// W5-3 played from its start to its exit and through the tally (T126): the exit at 6864, fire
/// taking the points, nothing left out. No re-sync: the play field buffer's at 1608 was T184's
/// (the engine's pass a frame off).
#[test]
fn enemies_w5_3_complete() {
    completes("enemies-w5-3-complete", &[], 6864);
}

/// W5-4 played from its start to its exit and through the tally (T126): the exit at 8193, fire
/// taking the points. One re-sync pinned: the panel at 7980 after a lag. The frog on the spring
/// pad (657) and the exit fade's palette step (8193) are within the faithful picture tolerance.
/// Green faithful since T181: the vertical blank at 3378 falls inside `sfx_play`'s rest, so
/// `sfx_tick` ends the new effect and music takes channel 3 back, as in the original (the
/// play field buffer's re-sync at 3384 went with it).
#[test]
fn enemies_w5_4_complete() {
    completes("enemies-w5-4-complete", &[7980], 8193);
}
