//! The title's two quits as cases (forest-dumb-title T05, DESIGN §2.2, §2.3): `cases/title-koniec.toml`
//! and `cases/title-esc.toml`, each lined up by loads from the idle title at boot-to-level1's frame
//! 4990 and judged by the `exited` goal: both sides return from the program's entry to AmigaDOS in
//! the same aligned frame. T01 measured Esc as a quit (`tests/title_codes.rs`), so `title-esc` is
//! judged as KONIEC is. Full: the owner's disks and Kickstart, the translated tree built in
//! (`tools/translate.py --build`).
//!
//! Each case runs on the translated program against the original. `PASSING` and `DIFFER` name its
//! reading, so a known failure is named, not hidden: a case in `DIFFER` must fail with its recorded
//! first difference, and once T07 fixes it, its test fails until it is moved to `PASSING`.

use fd_conform::translated_engine::run_translated;
use fd_conform::{Case, Options, Outcome, Verdict};
use fd_translated::TranslatedGame;

/// Cases equal to the original on the translated program, and the oracle frame their goal is met.
/// KONIEC: fire released at the script's frame 70 (oracle 5060), `title_selected` in step 5060;
/// `quit_to_dos` returns in the step ending at 5063 (measured 2026-11-20, T05). Esc: the return
/// in the step ending at 5012 on both sides (2026-11-20, T07; T05 read the translated program a
/// frame late, its key byte landing at the frame's start, `pcode-machine` `cia.rs`).
const PASSING: &[(&str, u32)] = &[("title-koniec", 5063), ("title-esc", 5012)];

/// Cases that differ on the translated program, and the start of their first difference. None
/// today.
const DIFFER: &[(&str, &str)] = &[];

/// What the original does in each case, read from the verdict whichever side differs: the
/// oracle frame it returned from the program's entry.
const ORIGINAL_EXIT: &[(&str, u32)] = &[("title-koniec", 5063), ("title-esc", 5012)];

fn translated(name: &str) -> Verdict {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
    let case = Case::load(std::path::Path::new(&format!("cases/{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    // Every channel, so a differing case still names the original's exit (the scene channel).
    let opts = Options {
        every_channel: true,
        ..Options::default()
    };
    run_translated(&case, &opts).unwrap_or_else(|e| panic!("{e}"))
}

/// The case's reading on the translated program, held against the two lists.
fn reads_as_recorded(name: &str) {
    let v = translated(name);
    let exit = ORIGINAL_EXIT.iter().find(|(n, _)| *n == name).unwrap().1;
    match (
        PASSING.iter().find(|(n, _)| *n == name),
        DIFFER.iter().find(|(n, _)| *n == name),
    ) {
        (Some(&(_, at)), None) => {
            assert_eq!(v.outcome, Outcome::Same, "{name}: {v}");
            // Met only when both sides exited, so the original's exit is the goal's frame.
            assert_eq!(v.goal_met, Some(at), "{name}: {v}");
            assert_eq!(at, exit, "{name}");
        }
        (None, Some(&(_, first))) => {
            let text = v.to_string();
            assert!(
                text.starts_with(first),
                "{name}: the first difference moved (fixed? move it to PASSING): {v}"
            );
            // The original still reached the goal: its exit is named in the run's scene channel.
            assert!(
                text.contains(&format!(
                    "the original returned from the program's entry at oracle frame {exit}"
                )),
                "{name}: the original's exit at {exit}: {v}"
            );
        }
        _ => panic!("{name} must be in exactly one of PASSING and DIFFER"),
    }
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; the original from power-on"]
fn title_koniec_reads_as_recorded_on_the_translated_program() {
    reads_as_recorded("title-koniec");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; the original from power-on"]
fn title_esc_reads_as_recorded_on_the_translated_program() {
    reads_as_recorded("title-esc");
}
