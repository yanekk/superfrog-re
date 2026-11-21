//! The KODY cases (forest-dumb-title T04, DESIGN §2.2, `docs/16-title.md`): one per code the title
//! takes (`title-code-w1`, `-w3`, `-w5`, `-cheat`), a wrong one (`title-code-wrong`) and keys rolled
//! (`title-code-rolled`, T11's drill), each typing its code on the keyboard from the idle title at 4990 (T01's scripts, `scripts/title-code-*.txt`),
//! with no `set`: the original's own `title_code_world` writes `world_index` and `cheat_flag`.
//!
//! Full: the owner's disks and Kickstart and the generated tree. Each case runs on the translated
//! program against the original; it reaches its goal (both sides) in the frame named here, and its
//! reading is `PASSING` or the first difference in `DIFFER`. T07 moves a case from one list to the
//! other when its fix lands. The no-disk check keeps every `title-code-*` case in one list and
//! holds that none carries a `set`.

use fd_conform::translated_engine::run_translated;
use fd_conform::{Case, Options, Outcome, conform_dir};
use fd_translated::TranslatedGame;

/// Cases that pass on the translated program (2026-11-20, T07). T04 read all five differing from
/// 5051, a frame after the first digit's key went down: the machine landed the key byte at the
/// frame's start, so `level2_handler`'s two-line wait ran across `title_pass`'s `wait_line_1` and
/// the title lost a pass (`pcode-machine` `cia.rs`, the idle keyboard's first byte).
/// `title-code-rolled` passes since T11 sent a frame's key changes in code order, as vAmiga does.
const PASSING: [&str; 6] = [
    "title-code-w1",
    "title-code-w3",
    "title-code-w5",
    "title-code-cheat",
    "title-code-wrong",
    "title-code-rolled",
];

/// Cases that differ on the translated program, with the start of their reading's first line.
/// None today.
const DIFFER: [(&str, &str); 0] = [];

fn case(name: &str) -> Case {
    Case::load(&conform_dir().join("cases").join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_code_case_is_in_one_list_and_none_sets_a_variable() {
    let mut on_disk: Vec<String> = std::fs::read_dir(conform_dir().join("cases"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("title-code-"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = PASSING
        .iter()
        .copied()
        .chain(DIFFER.iter().map(|(n, _)| *n))
        .map(String::from)
        .collect();
    listed.sort();
    assert_eq!(
        listed, on_disk,
        "a title-code case is in neither list, or in both"
    );
    for n in &on_disk {
        let c = case(n);
        assert!(
            c.set.is_empty(),
            "{n}: the code alone chooses the world, no `set`"
        );
        assert_eq!(
            c.input.as_deref(),
            Some(n.as_str()),
            "{n}: its own T01 script"
        );
        assert!(c.by_loads(), "{n}: lined up by loads");
    }
}

/// `name` on the translated program: its goal reached in oracle frame `goal_at` on both sides, and
/// its reading the list's.
fn reads(name: &str, goal_at: u32) {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
    // Past a channel's first difference, as `run --engine translated` reads a case: else the run
    // ends at 5051 and the goal is never seen.
    let opts = Options {
        every_channel: true,
        ..Options::default()
    };
    let v = run_translated(&case(name), &opts).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.goal_met, Some(goal_at), "{name}: the goal's frame\n{v}");
    if PASSING.contains(&name) {
        assert_eq!(v.outcome, Outcome::Same, "{v}");
        return;
    }
    let (_, want) = DIFFER.iter().find(|(n, _)| *n == name).expect("listed");
    let first = v.to_string();
    let first = first.lines().next().unwrap_or("");
    assert!(
        first.starts_with(want),
        "{name}'s reading changed: {first}\n{v}"
    );
}

/// 0556: world 1, `cheat_flag` 0, judged to play's first frame (level3_handler's first run).
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; about 90 s"]
fn code_0556_starts_world_1() {
    reads("title-code-w1", 6429);
}

/// 2400: world 1 with `cheat_flag` 1, to play's first frame.
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; about 90 s"]
fn code_2400_starts_world_1_with_the_cheat() {
    reads("title-code-cheat", 6429);
}

/// 1234: no code, world 1 as 0556, `cheat_flag` left 0, to play's first frame.
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; about 90 s"]
fn a_wrong_code_starts_world_1() {
    reads("title-code-wrong", 6429);
}

/// 2424: world 3 (`world_index` 2), 300 passes of play.
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; about 90 s"]
fn code_2424_starts_world_3() {
    reads("title-code-w3", W3_GOAL);
}

/// 7531: world 5 (`world_index` 4), 300 passes of play.
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; about 90 s"]
fn code_7531_starts_world_5() {
    reads("title-code-w5", W5_GOAL);
}

/// Play's 300th pass: world 3's first frame of play is 6299, world 5's 6139 (measured 2026-11-20).
const W3_GOAL: u32 = 6598;
const W5_GOAL: u32 = 6438;

/// 2424 typed rolling, each key let go in the frame the next goes down: the frame's changes go out
/// in key-code order, so key 2's rolled releases are followed by key 4's press and lost; two
/// digits count, both 4, the entry still on (T11's drill).
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; about 20 s"]
fn keys_rolled_count_only_the_releases_sent_last() {
    reads("title-code-rolled", 5081);
}
