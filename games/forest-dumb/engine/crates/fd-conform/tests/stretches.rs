//! Stretches of W1-1 with nothing else in play, compared by variables and picture at once
//! (plans/forest-dumb-level-1 T11, DESIGN §2.2): T41 to T44 matched the character, the camera and the
//! picture case by case; each stretch here runs fd-core's `Game` from the level sync point on the
//! original's input and compares every character, camera and status variable on every pass and the
//! picture on every frame, in one run. Needs the disks and the Kickstart: absent, it fails naming
//! where it looked.

use std::path::Path;
use std::sync::Arc;
use std::thread;

use fd_conform::{Case, Goal, Options, Outcome, Verdict, game_data, run, run_with};
use fd_core::program::Program;
use fd_core::{Game, GameData, LAYOUT, PORTED, Start};

/// Run, jump, slope, pit edge and scroll, and walls in the air.
const STRETCHES: [&str; 6] = [
    "stretch-run",
    "stretch-jump",
    "stretch-slope",
    "stretch-pit",
    "stretch-scroll",
    "stretch-wall",
];

/// The routines T08, T09 and T10 registered (`fd-core/src/ported.rs`, their sections).
const T08: [&str; 14] = [
    "char_update",
    "char_step",
    "h0_3ea8",
    "h0_410c",
    "h0_416c",
    "h0_4274",
    "h0_4696",
    "h0_4718",
    "h0_47e2",
    "h0_48be",
    "h0_497c",
    "h0_4b0e",
    "char_probe",
    "char_probe_flags",
];
const T09: [&str; 10] = [
    "camera_update",
    "h0_4e9a",
    "h0_4ea6",
    "h0_4eb4",
    "h0_4ec2",
    "h0_513c",
    "h0_51fc",
    "h0_526a",
    "h0_52d8",
    "h0_5346",
];
const T10: [&str; 16] = [
    "h0_3ab4",
    "char_sprite_position",
    "h0_94ba",
    "h0_94fe",
    "h0_9550",
    "h0_9754",
    "char_sprite_copy",
    "h0_6a96",
    "h0_6ae8",
    "h0_6b70",
    "h0_6b78",
    "h0_6bc0",
    "h0_6c20",
    "h0_6c80",
    "h0_9a34",
    "h0_988a",
];

/// T10's routines no stretch can reach, each with the task whose cases do. The status bar redraws
/// a number only when it changes, and on a stretch with nothing to collect only the countdown and,
/// on the pit's (T70), the lives do (measured 2026-11-06: 0 hits over every stretch). `picture_cases.rs` shows all of
/// them on the original but `h0_3ab4`.
const NOT_IN_A_STRETCH: [(&str, &str); 6] = [
    // The blitter wait, called only by the spring's, the creatures' and the animated cells' blits.
    ("h0_3ab4", "T46, T47"),
    // The score and its digits: a coin or bonus taken.
    ("h0_6ae8", "T22"),
    ("h0_6b70", "T22"),
    // The items left: an item taken.
    ("h0_6c20", "T22"),
    // The copper list's sprite and status pointers, set at start-up before the sync point
    // (`picture_cases.rs`, the boot from power-on).
    ("h0_9a34", "T44"),
    ("h0_988a", "T44"),
];

/// `h0_4274`'s rise table for a jump with neither `char_jump_3b28` nor `char_jump_41b4` set: the
/// standing and running jumps of `stretch-jump` (`fd-core/src/character/physics.rs`).
const RISE_TABLE: u32 = 0x431e;

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

#[test]
fn every_stretch_matches_the_original_on_variables_and_picture_together() {
    let verdicts = verdicts(STRETCHES.iter().map(|n| load(n)).collect());
    for (case, v) in &verdicts {
        let name = &case.name;
        assert_eq!(v.outcome, Outcome::Same, "{name}: {v}");
        assert!(
            case.compare.vars && case.compare.picture,
            "{name}: compares variables and picture"
        );
        // The goal is a place: the character's two position variables, and it was reached.
        let Some(Goal::All { all, .. }) = &case.goal else {
            panic!("{name}: states no goal");
        };
        let mut goal: Vec<&str> = all.iter().map(|(n, _)| n.as_str()).collect();
        goal.sort_unstable();
        assert_eq!(goal, ["char_x", "char_y"], "{name}");
        assert!(v.goal_met.is_some(), "{name}: {v}");
        assert!(v.events.is_empty(), "{name}: {v}");
        // Nearly every pass and every frame compared; nothing tolerated, so every picture not
        // excluded is the original's to the pixel.
        assert!(v.passes + v.unseen + 1 >= case.frames - 5, "{name}: {v}");
        assert!(v.pictures + 3 >= case.frames, "{name}: {v}");
        assert_eq!(v.tolerated, [0, 0, 0], "{name}: {v}");
        // Nor a bar accepted as a cut redraw draws it (T71).
        assert_eq!(v.cut_redraws, 0, "{name}: {v}");
        // Every exclusion names its task (the parser requires one) and hid something.
        for (e, n) in &v.excluded {
            assert!(*n > 0, "{name}: {} hid nothing", e.what);
        }
        // Every character and camera variable LAYOUT holds is compared or excluded with its task.
        for var in LAYOUT
            .iter()
            .filter(|v| v.name.starts_with("char_") || v.name.starts_with("camera_"))
        {
            let compared = case.vars.iter().any(|v| v == var.name);
            let excluded = case
                .exclude
                .iter()
                .any(|e| e.var.as_deref() == Some(var.name));
            assert!(compared || excluded, "{name}: {} not compared", var.name);
        }
    }

    // Every routine T08, T09 and T10 registered ran on the original in a stretch, but the ones
    // named with the task that reaches them.
    let mut none = Vec::new();
    for name in T08.iter().chain(&T09).chain(&T10) {
        assert!(
            PORTED.iter().any(|p| p.name == *name),
            "{name} is not in PORTED"
        );
        let h = hits(&verdicts, name);
        let named = NOT_IN_A_STRETCH.iter().any(|(n, _)| n == name);
        if h == 0 && !named {
            none.push(*name);
        }
        // A named one that a stretch now reaches comes off the list.
        assert!(
            !(named && h > 0),
            "{name} ran {h} times: no longer an exclusion"
        );
    }
    assert!(none.is_empty(), "never reached: {none:?}");
}

/// A rise table one pixel higher at one step: the jump stretch's character rises a pixel more.
#[test]
fn a_mutated_jump_constant_fails_the_jump_stretch() {
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut bytes = data.program.bytes().to_vec();
    let at = (RISE_TABLE + 2) as usize;
    assert!(
        bytes[at] & 0x7f > 0 && bytes[at] != 0xff,
        "{:#x}",
        bytes[at]
    );
    bytes[at] += 1;
    let mutated = Arc::new(GameData {
        program: Program::hunk0(bytes),
        ..(*data).clone()
    });
    let case = load("stretch-jump");
    let mut game = Game::new(mutated, Start::Level1);
    let v = run_with(&case, &mut game, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        matches!(&v.outcome, Outcome::Differs { var, .. } if var == "char_y"),
        "{v}"
    );
}

/// Each stretch's goal moved one cell (16 pixels) each way: both sides agree and miss it.
#[test]
fn each_stretch_fails_with_its_goal_moved_one_cell() {
    let mut moved = Vec::new();
    for name in STRETCHES {
        for (var, by) in [
            ("char_x", 16i64),
            ("char_x", -16),
            ("char_y", 16),
            ("char_y", -16),
        ] {
            let mut case = load(name);
            let Some(Goal::All { all, .. }) = &mut case.goal else {
                panic!("{name}: states no goal");
            };
            let (_, value) = all.iter_mut().find(|(n, _)| n == var).unwrap();
            *value = (*value as i64 + by) as u32;
            case.name = format!("{name} {var} {by:+}");
            moved.push(case);
        }
    }
    for (case, v) in verdicts(moved) {
        assert!(
            matches!(v.outcome, Outcome::GoalMissed { .. }),
            "{}: {v}",
            case.name
        );
    }
}
