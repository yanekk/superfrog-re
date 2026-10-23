//! `Game::start_level` and `Game::location` (bug-recorder T02, DESIGN §2.1, §2.6, D2): a level
//! started the player's way with the Start Level dialog's values, and where play is as a bug
//! recording names it. Against the original: sf-conform's `SyncPoint::Setup` (T03) and T00's
//! measurements (FINDINGS).

use std::sync::Arc;

use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::recording::{Difficulty, LevelSetup, Lives, Location, levels};
use sf_core::{DisplayData, Game, LevelId, Rng, Start};

fn display() -> Arc<DisplayData> {
    sf_conform::display_data()
}

fn id(s: &str) -> LevelId {
    s.parse().unwrap()
}

fn w(g: &Game, off: u32) -> u16 {
    let b = g.peek(h0(off), 2);
    u16::from_be_bytes([b[0], b[1]])
}

fn l(g: &Game, off: u32) -> u32 {
    let b = g.peek(h0(off), 4);
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

fn start(setup: LevelSetup) -> Game {
    Game::start_level(display(), &setup, Rng::default())
}

/// The variables that place a level's first frame: the frog, the camera and the clock, as
/// `Start::Level` (the level's entry, held to the original's frame 0 by `level_entry.rs`) has them.
const FIRST_FRAME: [(&str, u32, u32); 5] = [
    ("frog_x", 0x7d6, 2),
    ("frog_y", 0x7d8, 2),
    ("camera_x", 0x16664, 4),
    ("camera_y", 0x16660, 4),
    ("level_clock", 0x876, 4),
];

/// `levels()`'s `part`th quarter-ish (a test each, so the harness runs them side by side): each
/// started with the defaults is a level, the setup's, at its first frame.
fn levels_start_at_their_first_frame(part: usize) {
    let ids = levels();
    assert_eq!(ids.len(), 26);
    assert!(ids.iter().all(|i| !i.alt), "W6's second map set is not startable (owner, T02)");
    let mut bad = Vec::new();
    for &i in ids.iter().skip(part * 4).take(4) {
        let g = start(LevelSetup { level: i, ..LevelSetup::default() });
        let entry = Game::with_display(display(), Start::Level(i));
        if !g.scene().is_level() {
            bad.push(format!("{i}: scene {:?}", g.scene()));
        }
        match g.location() {
            Location::Level { id: at, .. } if at == i => {}
            other => bad.push(format!("{i}: location {other}")),
        }
        for (name, off, len) in FIRST_FRAME {
            let (ours, first) = (g.peek(h0(off), len), entry.peek(h0(off), len));
            if ours != first {
                bad.push(format!("{i}: {name} {ours:x?}, the level's first frame {first:x?}"));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn levels_1_to_4_start_at_their_first_frame() {
    levels_start_at_their_first_frame(0);
}
#[test]
fn levels_5_to_8_start_at_their_first_frame() {
    levels_start_at_their_first_frame(1);
}
#[test]
fn levels_9_to_12_start_at_their_first_frame() {
    levels_start_at_their_first_frame(2);
}
#[test]
fn levels_13_to_16_start_at_their_first_frame() {
    levels_start_at_their_first_frame(3);
}
#[test]
fn levels_17_to_20_start_at_their_first_frame() {
    levels_start_at_their_first_frame(4);
}
#[test]
fn levels_21_to_24_start_at_their_first_frame() {
    levels_start_at_their_first_frame(5);
}
#[test]
fn the_bonus_and_final_levels_start_at_their_first_frame() {
    levels_start_at_their_first_frame(6);
}

/// The setup started at W2-3: each value where the game keeps it.
fn values_land(lives: Lives, difficulty: Difficulty, energy: u8, points: u32, spud: bool, wings: bool) {
    let setup = LevelSetup {
        level: id("W2-3"),
        lives,
        points,
        energy,
        difficulty,
        spud,
        wings,
    };
    let g = start(setup);
    let what = format!("{setup:?}");
    assert_eq!(w(&g, 0x82e), lives.count() as u16 - 1, "frog_lives, {what}");
    assert_eq!(w(&g, 0x9e4), lives.opt_lives() as u16, "opt_lives, {what}");
    assert_eq!(l(&g, 0x834), points, "score, {what}");
    assert_eq!(l(&g, 0x880), setup.next_life(), "frog_next_life, {what}");
    assert_eq!(w(&g, 0x830), energy as u16, "frog_hearts, {what}");
    assert_eq!(w(&g, 0x852), spud as u16, "frog_spud_on, {what}");
    assert_eq!(w(&g, 0x866), wings as u16, "frog_flight, {what}");
    assert_eq!(w(&g, 0x9e6), difficulty.opt_game_mode() as u16, "opt_game_mode, {what}");
    // What `frog_new_game` derives from the mode: the bounce, gravity and air tuning
    // (h0+$86e..$874) and the base speed (h0+$886).
    let (tuning, base) = match difficulty {
        Difficulty::Easy => ([0xfffb, 6, 2, 0xfffe], 5),
        Difficulty::Normal => ([0xfffa, 4, 1, 0xfffe], 6),
    };
    let ours: Vec<u16> = (0..4).map(|k| w(&g, 0x86e + 2 * k)).collect();
    assert_eq!(ours, tuning, "frog_new_game's tuning, {what}");
    assert_eq!(w(&g, 0x886), base, "frog_base_speed, {what}");
}

#[test]
fn the_defaults_land() {
    values_land(Lives::Three, Difficulty::Normal, 4, 0, false, false);
}
#[test]
fn five_lives_easy_one_heart_points_spud_and_wings_land() {
    values_land(Lives::Five, Difficulty::Easy, 1, 450_000, true, true);
}
#[test]
fn seven_lives_normal_two_hearts_the_most_points_and_wings_land() {
    values_land(Lives::Seven, Difficulty::Normal, 2, 999_999, false, true);
}
#[test]
fn three_lives_easy_three_hearts_and_spud_land() {
    values_land(Lives::Three, Difficulty::Easy, 3, 199_999, true, false);
}

/// A walk with jumps and fire, varied so the play is not still.
fn input(f: u32) -> FrameInput {
    let mut joy = if f % 90 < 60 { Joy::RIGHT } else { Joy::LEFT };
    if f % 37 < 4 {
        joy = joy | Joy::UP;
    }
    if f % 53 < 2 {
        joy = joy | Joy::FIRE;
    }
    FrameInput::joy(joy)
}

#[test]
fn the_same_setup_and_seed_play_the_same_and_the_seed_reaches_the_rng() {
    let setup = LevelSetup {
        level: id("W1-1"),
        lives: Lives::Five,
        difficulty: Difficulty::Easy,
        ..LevelSetup::default()
    };
    let seed = Rng { lo: 0x1234_5678, hi: 0x9abc_def0 };
    let mut a = Game::start_level(display(), &setup, seed);
    let mut b = Game::start_level(display(), &setup, seed);
    let other = Game::start_level(display(), &setup, Rng::default());
    let rng = |g: &Game| g.peek(h0(0x160d4), 8);
    assert_ne!(rng(&a), rng(&other), "a different seed leaves another rng_state");
    for f in 0..1000 {
        a.step(input(f));
        b.step(input(f));
    }
    assert!(a.probe() == b.probe(), "the probes differ after 1 000 frames");
    assert!(a == b, "the games differ after 1 000 frames");
}

/// The places a run goes through, a level's cells taken as one: `g` stepped `frames` frames of
/// `input`, from its exit forced (`level_state` 1, the exit's own value).
fn places_after_exit(mut g: Game, frames: u32, input: impl Fn(u32) -> FrameInput) -> Vec<String> {
    g.poke(h0(0xa06), &1u32.to_be_bytes());
    let name = |l: Location| match l {
        Location::Level { id, .. } => id.to_string(),
        Location::Screen(s) => s.to_string(),
    };
    let mut seen = vec![name(g.location())];
    for f in 0..frames {
        g.step(input(f));
        let n = name(g.location());
        if seen.last() != Some(&n) {
            seen.push(n);
        }
    }
    seen
}

#[test]
fn the_location_is_the_frogs_cell_then_each_screen_as_the_chain_goes() {
    let g = start(LevelSetup::default());
    // W1-1's start: the frog's lower half, as T00 checked the cell formula.
    let (x, y) = (w(&g, 0x7d6), w(&g, 0x7d8));
    let at = Location::Level { id: id("W1-1"), col: 10, row: 41 };
    assert_eq!(g.location(), at);
    assert_eq!(((x + 0x17) / 16, (y - 0x10) / 16), (10, 41));
    assert_eq!(at.to_string(), "W1-1:10,41");
    // Up then fire on the tally takes the slot machine, which plays itself out on fire. The
    // blank frames the screens fade through keep the screen before.
    let slot = |f: u32| match f % 50 {
        10..20 => FrameInput::joy(Joy::UP),
        30..32 => FrameInput::joy(Joy::FIRE),
        _ => FrameInput::NONE,
    };
    let places = places_after_exit(g, 1400, slot);
    assert_eq!(places, ["W1-1", "tally", "slot machine", "world map", "W1-2"]);
}

#[test]
fn after_a_worlds_fourth_level_come_its_end_picture_and_the_next_worlds_intro() {
    let g = start(LevelSetup { level: id("W1-4"), ..LevelSetup::default() });
    let fire = |f: u32| if f % 50 < 2 { FrameInput::joy(Joy::FIRE) } else { FrameInput::NONE };
    let places = places_after_exit(g, 700, fire);
    assert_eq!(places, ["W1-4", "tally", "world end", "intro", "world map", "W2-1"]);
}

#[test]
#[should_panic(expected = "start_level: energy 0 is not 1 to 4 hearts")]
fn an_invalid_setup_panics_with_the_reason() {
    start(LevelSetup { energy: 0, ..LevelSetup::default() });
}

#[test]
#[should_panic(expected = "start_level: level W6-2alt is not one a game plays")]
fn the_second_map_set_is_refused() {
    start(LevelSetup { level: id("W6-2alt"), ..LevelSetup::default() });
}
