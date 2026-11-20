//! The character against walls, a pit, a slope and the level's left and top edges on W1-1, against
//! the original (plans/forest-dumb-level-1 T42). Each case runs fd-core's `Game` from the level sync
//! point and compares the character's `LAYOUT` variables on every pass; together they reach every
//! routine T08 and T51 ported. Needs the disks and the Kickstart: absent, it fails naming where it
//! looked.

use std::path::Path;
use std::thread;

use fd_conform::{Case, Goal, Outcome, Verdict, run};
use fd_core::{LAYOUT, PORTED};

const CASES: [&str; 5] = [
    "character-edge-wall",
    "character-edge-pit",
    "character-edge-slope",
    "character-edge-left",
    "character-edge-top",
];

/// T08's movement and T51's animation and joystick routines, as `fd-core/src/ported.rs` names them.
const CHARACTER: [&str; 20] = [
    "char_step_4be6",
    "h0_4c28",
    "h0_4c6a",
    "h0_4d32",
    "h0_4d54",
    "read_joystick",
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

fn load(name: &str) -> Case {
    Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

/// Every case on the level engine, each on its own thread (one oracle each).
fn verdicts() -> Vec<(Case, Verdict)> {
    let runs: Vec<_> = CASES
        .iter()
        .map(|name| {
            thread::spawn(move || {
                let case = load(name);
                let v = run(&case).unwrap_or_else(|e| panic!("{name}: {e}"));
                (case, v)
            })
        })
        .collect();
    runs.into_iter().map(|t| t.join().unwrap()).collect()
}

#[test]
fn every_edge_case_matches_the_original_and_reaches_its_goal_and_every_character_routine() {
    let verdicts = verdicts();
    for (case, v) in &verdicts {
        assert_eq!(v.outcome, Outcome::Same, "{}: {v}", case.name);
        assert!(
            matches!(case.goal, Some(Goal::All { .. })),
            "{}: states no goal",
            case.name
        );
        assert!(v.goal_met.is_some(), "{}: {v}", case.name);
        assert!(v.events.is_empty(), "{}: {v}", case.name);
        // Most passes compared: a case that stopped comparing early would show it here.
        assert!(
            v.passes + v.unseen + 1 >= case.frames - 1,
            "{}: {v}",
            case.name
        );
        // Where the character is, how it moves and what it shows, on both sides. The pit excludes
        // the hurt frame of a life lost on its spikes (T24), and compares the rest.
        for var in [
            "char_x",
            "char_y",
            "char_jump_t",
            "char_slide_t",
            "joy_up",
            "camera_y",
        ] {
            assert!(case.vars.iter().any(|v| v == var), "{}: {var}", case.name);
        }
        // Every character variable LAYOUT holds is compared or excluded with its task, so one a
        // later task appends is not silently left out (T42 review: char_unhurt_t was).
        for var in LAYOUT.iter().filter(|v| v.name.starts_with("char_")) {
            let compared = case.vars.iter().any(|v| v == var.name);
            let excluded = case
                .exclude
                .iter()
                .any(|e| e.var.as_deref() == Some(var.name));
            assert!(
                compared || excluded,
                "{}: {} not compared",
                case.name,
                var.name
            );
        }
    }

    // Each routine T08 and T51 ported runs on the original in at least one case.
    for name in CHARACTER {
        assert!(
            PORTED.iter().any(|p| p.name == name),
            "{name} is not in PORTED"
        );
        let hits: u32 = verdicts
            .iter()
            .flat_map(|(_, v)| &v.coverage.hits)
            .filter(|(n, _)| *n == name)
            .map(|(_, h)| h)
            .sum();
        assert!(hits > 0, "{name}: no case reaches it");
    }
}
