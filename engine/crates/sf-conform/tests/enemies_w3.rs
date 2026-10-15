//! World 3's enemies in frame cases (T44): `engine/conform/cases/enemies-w3-*`, run against the
//! original with every object live. Each test checks the case is green but for the re-syncs it
//! pins (T101's, as `enemies_w1` pins its own), and that the enemies it is meant to meet did what
//! it says: their updates ran, the frog was hurt, and the ones it kills died (their stomped
//! updates ran).

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
const STOMPED: u32 = 0x23bee;
const REWARD: u32 = 0x23e6c;

/// W3-1: the shuffler, the jumper and a stompable stomped (the jumper into a reward); the frog
/// hurt by a dart and the swing's ball.
#[test]
fn w3_1() {
    let s = run("enemies-w3-1", &[201, 863]);
    for u in [
        PAD, DART_WAIT, DART_FLY, SHUFFLER, SPIKER, HOPPER, FLOW, SWING, SWING_LINK, JUMPER,
        STOMPABLE, STOMPED, REWARD,
    ] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
    assert!(s.hurts >= 3, "hurt {} times", s.hurts);
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

/// W3-4 to its first crumbling platform: the platforms, the scenery below marker 14 (whose update
/// is the pads') and the flows.
#[test]
fn w3_4() {
    let s = run("enemies-w3-4", &[]);
    for u in [CRUMBLE, PAD, FLOW] {
        assert!(s.updates.contains(&u), "h0+${u:x} never ran: {:x?}", s.updates);
    }
}
