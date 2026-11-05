//! The character's runs against the original on clear stretches of W1-1 (plans/forest-dumb-level-1
//! T41): running left to the wall and back, the slide right, standing jumps tapped and held, and input
//! on the frame of a landing. Each case runs fd-core's `Game` from the level sync point and compares
//! the character's `LAYOUT` variables on every pass; together they reach every routine T08 and T51
//! ported. Needs the disks and the Kickstart: absent, it fails naming where it looked.

use std::path::Path;
use std::thread;

use fd_conform::{Case, Goal, Outcome, Verdict, run};
use fd_core::PORTED;

const CASES: [&str; 4] = [
    "character-run-left",
    "character-run-slide",
    "character-run-jumps",
    "character-run-landing",
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
fn every_run_matches_the_original_and_reaches_its_goal_and_every_character_routine() {
    let verdicts = verdicts();
    for (case, v) in &verdicts {
        assert_eq!(v.outcome, Outcome::Same, "{}: {v}", case.name);
        // A goal on the run's end, never idle: a pass end the original hides is allowed here.
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
        // The character's variables, not only where it is.
        for var in ["char_x", "char_y", "char_jump_t", "char_anim_ptr", "joy_up"] {
            assert!(case.vars.iter().any(|v| v == var), "{}: {var}", case.name);
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
