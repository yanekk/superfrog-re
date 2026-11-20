//! Game over's way back to the title as cases (forest-dumb-title T06, DESIGN §2.2, §2.3):
//! `cases/game-over-title.toml` and `cases/game-over-name.toml`, each lined up by loads from the
//! idle title at boot-to-level1's frame 4990, world 2 started as `world-2-start` does, given up by
//! Esc in play (docs/16-title.md "Game over"), through the high scores, and judged by the `enters`
//! goal: both sides enter `title_screen` in the same aligned frame. Full: the owner's disks and
//! Kickstart, the translated tree built in (`tools/translate.py --build`).
//!
//! Each case runs on the translated program against the original. `PASSING` and `DIFFER` name its
//! reading, so a known failure is named, not hidden: a case in `DIFFER` must fail with its recorded
//! first difference, and once T07 fixes it, its test fails until it is moved to `PASSING`.
//!
//! The run's coverage register is the routines of the way back, so the same run shows the original
//! went through the high scores (and, with a high score, the name entry) on the way to the title.

use fd_conform::translated_engine::run_translated;
use fd_conform::{Case, Options, Outcome, Verdict};
use fd_core::Ported;
use fd_translated::TranslatedGame;

/// Cases equal to the original on the translated program, and the oracle frame their goal is met:
/// the end of the step `title_screen` is entered in (T01 measured the steps starting at 7447 and
/// 7548, `tests/title_codes.rs`). Measured 2026-11-20 (T06).
const PASSING: &[(&str, u32)] = &[("game-over-title", 7448), ("game-over-name", 7549)];

/// Cases that differ on the translated program, and the start of their first difference.
const DIFFER: &[(&str, &str)] = &[];

const fn routine(name: &'static str, offset: u32) -> Ported {
    Ported {
        name,
        hunk: 0,
        offset,
        port: "",
        mounted: false,
    }
}

/// The way back, in the order the original runs it (`re/symbols.toml`). Only coverage reads this.
const WAY_BACK: &[Ported] = &[
    routine("game_over_screen", 0x4c4),
    routine("high_scores", 0x89d0),
    routine("hiscore_name_entry", 0x8b1a),
    routine("session_loop", 0x158),
];

fn translated(name: &str) -> Verdict {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
    let case = Case::load(std::path::Path::new(&format!("cases/{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let opts = Options {
        every_channel: true,
        register: WAY_BACK,
        ..Options::default()
    };
    run_translated(&case, &opts).unwrap_or_else(|e| panic!("{e}"))
}

/// The case's reading on the translated program, held against the two lists, and the original's
/// way back: every routine of `WAY_BACK` entered, the name entry only when `named`.
fn reads_as_recorded(name: &str, named: bool) {
    let v = translated(name);
    for &(routine, hits) in &v.coverage.hits {
        let entered = hits > 0;
        let expected = routine != "hiscore_name_entry" || named;
        assert_eq!(
            entered, expected,
            "{name}: the original entered {routine} {hits} times\n{v}"
        );
    }
    match (
        PASSING.iter().find(|(n, _)| *n == name),
        DIFFER.iter().find(|(n, _)| *n == name),
    ) {
        (Some(&(_, at)), None) => {
            assert_eq!(v.outcome, Outcome::Same, "{name}: {v}");
            assert_eq!(v.goal_met, Some(at), "{name}: {v}");
        }
        (None, Some(&(_, first))) => {
            assert!(
                v.to_string().starts_with(first),
                "{name}: the first difference moved (fixed? move it to PASSING): {v}"
            );
        }
        _ => panic!("{name} must be in exactly one of PASSING and DIFFER"),
    }
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; the original from power-on"]
fn game_over_title_reads_as_recorded_on_the_translated_program() {
    reads_as_recorded("game-over-title", false);
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; the original from power-on"]
fn game_over_name_reads_as_recorded_on_the_translated_program() {
    reads_as_recorded("game-over-name", true);
}
