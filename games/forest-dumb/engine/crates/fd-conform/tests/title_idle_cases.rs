//! The idle title and its rows on the translated program (forest-dumb-title T03, DESIGN §2.2):
//! `cases/title-idle-loop.toml` (power-on, the song's first loop back to position 0) and
//! `cases/title-rows.toml` (Down and Up through the three rows from the idle title at 4990), each
//! run translated against the original. A case is in `PASSING`, with the oracle frame its goal is
//! met at (both sides reach it, so the original does), or in `DIFFER`, with the first difference
//! the translated program shows, so a known failure is named for T07 and not hidden. Full: the
//! owner's disks and Kickstart, the translated tree built in; the lists' own check takes neither.

use fd_conform::translated_engine::run_translated;
use fd_conform::{Case, Options, Outcome, conform_dir};
use fd_translated::TranslatedGame;

/// The cases that pass translated, every channel equal, and the oracle frame each goal is met at
/// (measured 2026-11-20): the release of the last Up seen in step 5140 (T01: row 0 in step 5140),
/// and `mt_songpos` back at 0 in step 15013 (T01: "The song's loop").
const PASSING: [(&str, u32); 2] = [("title-rows", 5141), ("title-idle-loop", 15_014)];

/// The cases that differ translated, and the start of their first difference as the verdict
/// prints it. None today.
const DIFFER: [(&str, &str); 0] = [];

fn case(name: &str) -> Case {
    Case::load(&conform_dir().join("cases").join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_case_of_the_task_is_in_one_list_and_has_a_goal_counted_after_its_start() {
    let mut names: Vec<&str> = PASSING
        .iter()
        .map(|(n, _)| *n)
        .chain(DIFFER.iter().map(|(n, _)| *n))
        .collect();
    names.sort();
    assert_eq!(names, ["title-idle-loop", "title-rows"]);
    for n in names {
        let c = case(n);
        assert!(
            c.by_loads(),
            "{n}: lined up by loads, the title runner that takes a goal"
        );
        // Both paths end where they start: a goal counted from the start is met on frame one.
        assert!(
            matches!(c.goal, Some(fd_conform::Goal::All { after: Some(_), .. })),
            "{n}: an `all` goal with `after`"
        );
    }
}

fn run(name: &str) {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
    let v = run_translated(&case(name), &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    if let Some((_, met)) = PASSING.iter().find(|(n, _)| *n == name) {
        assert_eq!(v.outcome, Outcome::Same, "{v}");
        assert_eq!(v.goal_met, Some(*met), "{v}");
    } else {
        let (_, first) = DIFFER.iter().find(|(n, _)| *n == name).expect("listed");
        assert_ne!(
            v.outcome,
            Outcome::Same,
            "{name} passes now: move it to PASSING. {v}"
        );
        assert!(
            v.to_string().contains(first),
            "{name}: first difference moved. {v}"
        );
    }
}

#[test]
#[ignore = "full: the disks, Kickstart and the translated tree; from power-on to 15 100, about 2 min"]
fn title_idle_loop_translated() {
    run("title-idle-loop");
}

#[test]
#[ignore = "full: the disks, Kickstart and the translated tree; 180 frames from 4990, about 30 s"]
fn title_rows_translated() {
    run("title-rows");
}
