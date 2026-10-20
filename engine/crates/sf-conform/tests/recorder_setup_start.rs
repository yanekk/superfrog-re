//! `SyncPoint::Setup` (bug-recorder T03, DESIGN §2.8, §2.11, D2): the original started the
//! player's way with the Start Level dialog's values equals the engine's `Game::start_level` with
//! the same values and seed at the level's first frame, for every level the dialog offers, so a
//! recording's comparison starts from the same state on both sides.
//!
//! Equal by `level_entry.rs`'s rule, every `LAYOUT` variable, with its `NOT_MODELLED` list cut to
//! what the setup's start still leaves: the front end and the loads are run on both sides, and
//! what their timing decides is written into the original from the engine (`SETUP_FROM_ENGINE`).
//! No objects are frozen: the engine plays every kind.

use sf_conform::{Case, InputScript, SyncPoint, case::script_path, sync};
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarDef, h0};
use sf_core::recording::{Difficulty, LevelSetup, Lives, levels};
use sf_core::{Game, LevelId, Rng};

/// A seed that is not the default, so the random state is seen to come from it.
const SEED: Rng = Rng {
    lo: 0x0000_a5c3,
    hi: 0x1f00_e2b7,
};

/// What still differs at frame 0, and why: the display buffer's clean marks, presentation only
/// (DESIGN RE §2.1), which the engine's instant loads leave otherwise (T00).
const LEFT: &[&str] = &["playfield_clean"];

fn id(s: &str) -> LevelId {
    s.parse().unwrap()
}

fn setup(level: &str, difficulty: Difficulty) -> LevelSetup {
    LevelSetup {
        level: id(level),
        difficulty,
        ..LevelSetup::default()
    }
}

/// Every variable with a piece that differs: (name, pieces differing, the first one's original
/// and engine values).
fn diffs(theirs: &AmigaMemory, game: &Game) -> Vec<(&'static str, usize, u32, u32)> {
    let pieces: Vec<VarDef> = LAYOUT.iter().flat_map(|d| d.pieces()).collect();
    let mut out: Vec<(&'static str, usize, u32, u32)> = Vec::new();
    for (p, (_, ours)) in pieces.iter().zip(game.probe()) {
        let t = p.read(theirs as &dyn Ram);
        if t != ours {
            match out.last_mut() {
                Some(l) if l.0 == p.name => l.1 += 1,
                _ => out.push((p.name, 1, t, ours)),
            }
        }
    }
    out
}

/// What differs at `setup`'s frame 0 beyond [`LEFT`], as lines; empty when the starts are equal.
fn differences(setup: &LevelSetup) -> Vec<String> {
    let t0 = std::time::Instant::now();
    let (synced, game) = sync::setup_start(setup, SEED, 1).unwrap_or_else(|e| panic!("{e}"));
    let m = &synced.memory;
    let what = format!(
        "{} lives {} {:?} points {} energy {} spud {} wings {}",
        setup.level,
        setup.lives.count(),
        setup.difficulty,
        setup.points,
        setup.energy,
        setup.spud,
        setup.wings
    );
    eprintln!("{what}: frame 0 in {:.0?}", t0.elapsed());
    let mut bad: Vec<String> = diffs(m, &game)
        .into_iter()
        .filter(|d| !LEFT.contains(&d.0))
        .map(|(name, n, t, ours)| {
            format!("{what}: {name} ({n} pieces): original {t:x}, engine {ours:x}")
        })
        .collect();
    // The setup's values are where the game keeps them, on the original too (not merely equal).
    let w = |a: u32| (m.byte(a) as u32) << 8 | m.byte(a + 1) as u32;
    let l = |a: u32| w(a) << 16 | w(a + 2);
    for (name, got, want) in [
        ("frog_lives", w(h0(0x82e)), setup.lives.count() as u32 - 1),
        (
            "opt_game_mode",
            w(h0(0x9e6)),
            setup.difficulty.opt_game_mode() as u32,
        ),
        ("score", l(h0(0x834)), setup.points),
        ("frog_hearts", w(h0(0x830)), setup.energy as u32),
        ("frog_spud_on", w(h0(0x852)), setup.spud as u32),
        ("frog_flight", w(h0(0x866)), setup.wings as u32),
        ("rng_state lo", l(h0(0x160d4)), game.state().l(h0(0x160d4))),
    ] {
        if got != want {
            bad.push(format!(
                "{what}: the original's {name} {got}, the setup's {want}"
            ));
        }
    }
    bad
}

fn all_equal(setups: impl IntoIterator<Item = LevelSetup>) {
    let bad: Vec<String> = setups.into_iter().flat_map(|s| differences(&s)).collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// `levels()` from `from`, `n` of them, normal, the dialog's defaults.
fn levels_normal(from: usize, n: usize) {
    let ids = levels();
    assert_eq!(ids.len(), 26);
    all_equal(ids.into_iter().skip(from).take(n).map(|i| LevelSetup {
        level: i,
        ..LevelSetup::default()
    }));
}

#[test]
fn w1_1_normal_starts_as_the_original() {
    all_equal([setup("W1-1", Difficulty::Normal)]);
}

#[test]
fn a_setup_off_the_defaults_starts_as_the_original() {
    all_equal([LevelSetup {
        lives: Lives::Seven,
        points: 450_000,
        energy: 2,
        spud: true,
        wings: true,
        ..setup("W1-1", Difficulty::Normal)
    }]);
}

#[test]
#[ignore = "slow: about 30 s a level the first time (the original's menu and loads), cached after"]
fn levels_2_to_7_normal_start_as_the_original() {
    levels_normal(1, 6);
}

#[test]
#[ignore = "slow: about 30 s a level the first time (the original's menu and loads), cached after"]
fn levels_8_to_13_normal_start_as_the_original() {
    levels_normal(7, 6);
}

#[test]
#[ignore = "slow: about 30 s a level the first time (the original's menu and loads), cached after"]
fn levels_14_to_19_normal_start_as_the_original() {
    levels_normal(13, 6);
}

#[test]
#[ignore = "slow: about 30 s a level the first time (the original's menu and loads), cached after"]
fn levels_20_to_24_normal_start_as_the_original() {
    levels_normal(19, 5);
}

#[test]
#[ignore = "slow: about 30 s a level the first time; the final level runs W6-4, its tally and its load"]
fn the_bonus_and_final_levels_normal_start_as_the_original() {
    levels_normal(24, 2);
}

#[test]
#[ignore = "slow: about 30 s a start the first time, cached after"]
fn w1_1_and_w4_3_easy_start_as_the_original() {
    all_equal([
        setup("W1-1", Difficulty::Easy),
        setup("W4-3", Difficulty::Easy),
    ]);
}

/// The engine side of a `Setup` case is `Game::start_level` itself: a short stretch of play from
/// it runs green against the original.
#[test]
fn a_setup_case_runs_from_the_engines_own_start() {
    let text = "sync = { level = \"W1-1\", lives = 5, points = 1200, energy = 3, \
                difficulty = \"normal\", spud = false, wings = false, seed = \"a5c3 1f00e2b7\" }\n\
                script = \"frog-run-right\"\nframes = 300\n";
    let case = Case::parse("recorder-setup-w1-1", text).unwrap();
    assert!(script_path("frog-run-right").exists());
    let mut first = None;
    let r = sf_conform::run_with(&case, |f, g| {
        if f == 0 {
            first = Some(g.state().l(h0(0x834)));
        }
    });
    assert_eq!(
        first,
        Some(1200),
        "the engine starts with the setup's points"
    );
    if let Err(d) = r {
        panic!("{d}");
    }
}

#[test]
fn the_setup_form_parses() {
    let c = Case::parse(
        "x",
        "sync = { level = \"W4-3\", lives = 7, points = 450000, energy = 2, \
         difficulty = \"easy\", spud = true, wings = true, seed = \"1a2b3c4d 5e6f7a8b\" }\n\
         frames = 3",
    )
    .unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::Setup {
            setup: LevelSetup {
                level: id("W4-3"),
                lives: Lives::Seven,
                points: 450_000,
                energy: 2,
                difficulty: Difficulty::Easy,
                spud: true,
                wings: true,
            },
            seed: Rng {
                lo: 0x1a2b_3c4d,
                hi: 0x5e6f_7a8b,
            },
        }
    );
    assert_eq!(c.script, InputScript::idle());
}

#[test]
fn the_old_lives_form_keeps_its_meaning() {
    let c = Case::parse("x", "sync = { level = \"WB-1\", lives = 60 }\nframes = 3").unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::Lives {
            level: id("WB-1"),
            lives: 60
        }
    );
    let c = Case::parse("x", "sync = { level = \"W1-1\", lives = 3 }\nframes = 3").unwrap();
    assert_eq!(
        c.sync,
        SyncPoint::Lives {
            level: id("W1-1"),
            lives: 3
        }
    );
}

#[test]
fn a_bad_setup_form_is_refused() {
    let good = "level = \"W1-1\", lives = 3, points = 0, energy = 4, difficulty = \"normal\", \
                spud = false, wings = false, seed = \"a5c3 1f00e2b7\"";
    assert!(Case::parse("x", &format!("sync = {{ {good} }}\nframes = 3")).is_ok());
    for (from, to) in [
        // A missing key: no defaults, so a half-written setup is not a different start.
        (", seed = \"a5c3 1f00e2b7\"", ""),
        ("lives = 3, ", ""),
        // An unknown key, and `clock` or `exit` beside a setup.
        ("wings = false", "wings = false, hearts = 4"),
        ("wings = false", "wings = false, exit = true"),
        // Out of the dialog's ranges.
        ("lives = 3", "lives = 4"),
        ("lives = 3", "lives = 0"),
        ("points = 0", "points = 1000000"),
        ("points = 0", "points = -1"),
        ("energy = 4", "energy = 0"),
        ("energy = 4", "energy = 5"),
        ("difficulty = \"normal\"", "difficulty = \"hard\""),
        ("spud = false", "spud = 1"),
        ("wings = false", "wings = \"off\""),
        // Not a level a game plays from its start (W6's second map set, the unused W6-1 copy).
        ("level = \"W1-1\"", "level = \"W6-2alt\""),
        ("level = \"W1-1\"", "level = \"W9-1\""),
        // The seed: two hex longs.
        ("\"a5c3 1f00e2b7\"", "\"a5c3\""),
        ("\"a5c3 1f00e2b7\"", "\"a5c3 1ffff00e2b7\""),
        ("\"a5c3 1f00e2b7\"", "\"a5g3 1f00e2b7\""),
        ("\"a5c3 1f00e2b7\"", "12"),
    ] {
        assert!(good.contains(from), "{from}");
        let bad = format!("sync = {{ {} }}\nframes = 3", good.replacen(from, to, 1));
        assert!(Case::parse("x", &bad).is_err(), "{bad}");
    }
}
