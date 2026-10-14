//! World 5's enemies in frame cases against the original (T39): the activation case, and each
//! level's encounter case (`engine/conform/cases/enemies-w5-*.toml`), with what each must meet
//! checked through the engine's state while it runs (green, so the original's is the same).

use sf_core::Game;
use sf_core::frog::{FROG, f};
use sf_core::layout::h0;

const SLOTS: u32 = h0(0x1ba6c);

/// What a case run saw: each frame, every live slot's update routine (h0 offsets), the frog
/// hurt flag and lives, the score.
#[derive(Default)]
struct Seen {
    frames: Vec<(u32, Vec<u32>, bool, u16)>,
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
        let live = (0..20)
            .map(|k| SLOTS + k * 0x7c)
            .filter(|&s| st.l(s + 0x40) != 0)
            .map(|s| st.l(s + 4).wrapping_sub(h0(0)))
            .collect();
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

/// Asserts `seen` met each of `types` (update routine, name).
fn met(seen: &Seen, types: &[(u32, &str)]) {
    for &(u, what) in types {
        assert!(seen.spawned(u), "{what} (h0+${u:x}) never met");
    }
}

/// W5-1 from its start to the second pacer (T39): over the first pacer and the spike pits, under
/// the watchers, the mace and the fallers (each shakes and falls as the frog comes near), past the
/// drips, the crushers, the flows and the dart; every W5-1 type but the shot. Two re-syncs pinned,
/// both seen with enemies frozen too: T121's play field buffer at 551, the panel at 979.
#[test]
fn enemies_w5_1() {
    let seen = run("enemies-w5-1", &[551, 979]);
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
