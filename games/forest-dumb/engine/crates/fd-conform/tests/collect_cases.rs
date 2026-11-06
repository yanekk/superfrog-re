//! Collecting in play against the original (plans/forest-dumb-level-1 T45, DESIGN §2.1, §2.2, §4.3):
//! T22 ported taking items and bonuses and proved each routine on the rig; these cases run fd-core's
//! `Game` from the level sync point on W1-1's ledge of oranges, apples and coins and compare every
//! character, camera and status variable on every pass and the picture on every frame, the score and
//! items left with their status bar digits included. Needs the disks and the Kickstart: absent, it
//! fails naming where it looked.

use std::path::Path;
use std::sync::Arc;
use std::thread;

use amiga_conform::script;
use fd_conform::{Case, Goal, Options, Outcome, Verdict, conform_dir, game_data, run, run_with};
use fd_core::objects::collectible::{BONUS_SCORES, MAP_COLUMN};
use fd_core::program::Program;
use fd_core::{Game, GameData, LAYOUT, PORTED, Start};

/// Taking on the ledge, and the same then away and back.
const CASES: [&str; 2] = ["collect-ledge", "collect-return"];

/// The routines T22 registered (`fd-core/src/ported.rs`, its section), and the status bar's numbers
/// a pickup redraws, which no stretch reaches (`stretches.rs`, `NOT_IN_A_STRETCH`).
const T22: [&str; 8] = [
    "char_object_cells",
    "h0_26ec",
    "h0_29ca",
    "score_add_routine",
    "h0_2dd6",
    "h0_2e0e",
    "countdown_plus_100",
    "items_left_minus_one",
];
const STATUS: [&str; 3] = ["h0_6ae8", "h0_6b70", "h0_6c20"];

/// T22's routines no collecting case reaches, each with why and the task whose cases do.
const NOT_REACHED: [(&str, &str); 2] = [
    // Writes a creature back into the map as it leaves the view: while `freeze.rs` keeps creatures
    // from taking a slot (T24) no slot is ever filled, so it never runs.
    ("h0_2e0e", "T46"),
    // The time bonus `$4c`: W1-1 holds none (docs/10-object-system.md, "Values W1-1 does not hold").
    ("countdown_plus_100", "T48, as a named exception"),
];

/// The block above the ledge, W1-1:29..33,8..10, which both cases empty: five oranges (`$40`, +20),
/// two apples (`$41`, +30) and four coins (+100 each): 560.
const BLOCK: [(u32, u32); 11] = [
    (30, 8),
    (31, 8),
    (32, 8),
    (29, 9),
    (30, 9),
    (31, 9),
    (32, 9),
    (33, 9),
    (30, 10),
    (31, 10),
    (32, 10),
];

fn load(name: &str) -> Case {
    Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

/// Each case on the level engine, each on its own thread (one oracle each).
fn verdicts(cases: Vec<Case>) -> Vec<(Case, Verdict)> {
    let runs: Vec<_> = cases
        .into_iter()
        .map(|case| {
            thread::spawn(move || {
                let v = run(&case).unwrap_or_else(|e| panic!("{}: {e}", case.name));
                (case, v)
            })
        })
        .collect();
    runs.into_iter().map(|t| t.join().unwrap()).collect()
}

fn hits(verdicts: &[(Case, Verdict)], name: &str) -> u32 {
    verdicts
        .iter()
        .flat_map(|(_, v)| &v.coverage.hits)
        .filter(|(n, _)| *n == name)
        .map(|(_, h)| h)
        .sum()
}

fn goal(case: &Case) -> (&[(String, u32)], u32) {
    match &case.goal {
        Some(Goal::All { all, by_frame }) => (all, *by_frame),
        _ => panic!("{}: states no goal of variables", case.name),
    }
}

#[test]
fn collecting_matches_the_original_on_variables_and_picture_together() {
    let verdicts = verdicts(CASES.iter().map(|n| load(n)).collect());
    for (case, v) in &verdicts {
        let name = &case.name;
        assert_eq!(v.outcome, Outcome::Same, "{name}: {v}");
        assert!(
            case.compare.vars && case.compare.picture,
            "{name}: compares variables and picture"
        );
        // The goal is what was collected, and it was reached on both sides.
        let (all, _) = goal(case);
        for var in ["score", "items_left"] {
            assert!(all.iter().any(|(n, _)| n == var), "{name}: goal has {var}");
        }
        assert!(v.goal_met.is_some(), "{name}: {v}");
        assert!(v.events.is_empty(), "{name}: {v}");
        // Nearly every pass and every frame compared, nothing tolerated.
        assert!(v.passes + v.unseen + 1 >= case.frames - 5, "{name}: {v}");
        assert!(v.pictures + 10 >= case.frames, "{name}: {v}");
        assert_eq!(v.tolerated, [0, 0, 0], "{name}: {v}");
        // No exclusion is left to T22, and every one hid something.
        for (e, n) in &v.excluded {
            assert!(e.task != "T22", "{name}: {} is T22's", e.what);
            assert!(*n > 0, "{name}: {} hid nothing", e.what);
        }
        // Every character, camera and status variable LAYOUT holds that a stretch compares.
        for var in LAYOUT.iter().filter(|v| {
            v.name.starts_with("char_")
                || v.name.starts_with("camera_")
                || ["score", "items_left", "lives", "countdown"].contains(&v.name)
        }) {
            let compared = case.vars.iter().any(|v| v == var.name);
            let excluded = case
                .exclude
                .iter()
                .any(|e| e.var.as_deref() == Some(var.name));
            assert!(compared || excluded, "{name}: {} not compared", var.name);
        }
    }

    // Every routine T22 registered ran on the original, but the ones named with the task that
    // reaches them; and the status bar redrew the score and the items left.
    let mut none = Vec::new();
    for name in T22.iter().chain(&STATUS) {
        assert!(
            PORTED.iter().any(|p| p.name == *name),
            "{name} is not in PORTED"
        );
        let h = hits(&verdicts, name);
        let named = NOT_REACHED.iter().any(|(n, _)| n == name);
        if h == 0 && !named {
            none.push(*name);
        }
        assert!(
            !(named && h > 0),
            "{name} ran {h} times: no longer an exclusion"
        );
    }
    assert!(none.is_empty(), "never reached: {none:?}");
}

/// The case's input script, from its sync frame.
fn input(case: &Case) -> script::InputScript {
    let name = case
        .input
        .as_deref()
        .expect("a collecting case names its input");
    let path = conform_dir().join("scripts").join(format!("{name}.txt"));
    script::parse(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

/// The engine alone, started at its own level start rather than transplanted from the original,
/// meets each case's goal by its frame on the same input, and leaves the block empty.
#[test]
fn the_engine_alone_meets_each_collecting_goal() {
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    for name in CASES {
        let case = load(name);
        let (all, by_frame) = goal(&case);
        let input = input(&case);
        let mut game = Game::new(data.clone(), Start::Level1);
        // Frozen in the original, so no creature drawn on the engine (T52; T24 lifts both).
        if case.freeze {
            game.freeze_creature_drawing();
        }
        let mut met = None;
        let mut scores = Vec::new();
        for f in 0..by_frame - case.sync_frame {
            game.step(input.input_at(f));
            assert!(game.take_events().is_empty(), "{name}: frame {f}");
            let s = game.state();
            if scores.last() != Some(&s.score) {
                scores.push(s.score);
            }
            let value = |var: &str| match var {
                "char_x" => s.char_x,
                "char_y" => s.char_y,
                "score" => s.score,
                "items_left" => s.items_left as u32,
                other => panic!("{other}"),
            };
            if met.is_none() && all.iter().all(|(v, want)| value(v) == *want) {
                met = Some(f);
            }
        }
        assert!(met.is_some(), "{name}: goal missed; scores {scores:x?}");
        // Each pickup in its order: the orange, a coin and an apple, a coin, then the rest of the
        // block, and nothing after (collect-return takes nothing more).
        assert_eq!(
            scores,
            [
                0, 0x20, 0x150, 0x250, 0x270, 0x370, 0x400, 0x420, 0x440, 0x540, 0x560
            ],
            "{name}"
        );
        let map = &game.state().object_map;
        for (c, r) in BLOCK {
            assert_eq!(
                map[(c * MAP_COLUMN + r) as usize],
                0,
                "{name}: W1-1:{c},{r}"
            );
        }
    }
}

/// The orange's worth one more in the program the engine reads: the ledge case fails on the score.
#[test]
fn a_mutated_bonus_score_fails_the_ledge_case() {
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut bytes = data.program.bytes().to_vec();
    // `move.l #$20, score_add`'s immediate, the orange's (`$40`): its last byte.
    let (first, _, at) = BONUS_SCORES[0];
    assert_eq!(first, 0x40);
    let at = at as usize + 3;
    assert_eq!(bytes[at], 0x20, "the orange's immediate");
    bytes[at] = 0x21;
    let mutated = Arc::new(GameData {
        program: Program::hunk0(bytes),
        ..(*data).clone()
    });
    let case = load("collect-ledge");
    let mut game = Game::new(mutated, Start::Level1);
    let v = run_with(&case, &mut game, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        matches!(&v.outcome, Outcome::Differs { var, .. } if var == "score"),
        "{v}"
    );
}
