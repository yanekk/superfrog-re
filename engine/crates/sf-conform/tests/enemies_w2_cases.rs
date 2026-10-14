//! World 2's enemies in frame cases against the original (T43): the activation case, and each
//! level's encounter case (`engine/conform/cases/enemies-w2-*.toml`), with what each must meet
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

/// Run `name` green (pinning the listed re-syncs, T121's), recording what it saw.
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
            .all(|r| r.1.starts_with("playfield_buffer_")),
        "{:?}",
        o.resynced
    );
    seen
}

/// Run right past the walker and the flyers and back: each spawns as the camera reaches it and is
/// freed once the camera has left it, the same frames as the original.
#[test]
fn enemies_activation_w2_1() {
    let seen = run("enemies-activation-w2-1", &[]);
    for (what, u) in [("walker", 0x256ea), ("flyer", 0x25990)] {
        assert!(seen.spawned(u) && seen.left(u), "{what}");
    }
    // A flyer is back on the way back.
    assert!(seen.arrived_after(0x25990, 200));
}

/// Asserts `seen` met each of `types` (update routine, name).
fn met(seen: &Seen, types: &[(u32, &str)]) {
    for &(u, what) in types {
        assert!(seen.spawned(u), "{what} (h0+${u:x}) never met");
    }
}

/// W2-1: a flyer stomped (its fall, h0+$23b34, and its end), and nine of its eleven types.
#[test]
fn enemies_w2_1() {
    let seen = run("enemies-w2-1", &[]);
    assert!(
        seen.spawned(0x23b34) && seen.left(0x23b34),
        "a flyer killed"
    );
    met(
        &seen,
        &[
            (0x25990, "flyer"),
            (0x256ea, "walker"),
            (0x25d30, "hopper"),
            (0x25fa8, "dart"),
            (0x26786, "bouncer"),
            (0x25ffe, "backdrop"),
            (0x24ef6, "scenery"),
            (0x2740c, "crumbling ledge"),
        ],
    );
}

/// W2-3: over the crumbling ledges above the spike pit (none breaks on this route: the frog's box
/// never meets a ledge's hitbox; T122).
#[test]
fn enemies_w2_3() {
    let seen = run("enemies-w2-3", &[]);
    met(&seen, &[(0x2740c, "crumbling ledge"), (0x24ef6, "scenery")]);
}

/// W2-2: down the drop and past the dart, which comes out of hiding and flies; the
/// bouncer, the scenery. One re-sync (T121's), pinned.
#[test]
fn enemies_w2_2() {
    let seen = run("enemies-w2-2", &[560]);
    met(
        &seen,
        &[
            (0x26786, "bouncer"),
            (0x25fa8, "dart (hidden)"),
            (0x25f42, "dart (flying)"),
            (0x24ef6, "scenery"),
        ],
    );
}
