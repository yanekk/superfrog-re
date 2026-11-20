//! Title plan T10 (DESIGN §2.2, §2.6): the title's paths in the real app by key presses, the app's
//! state the headless run's every step (`tests/title/mod.rs`), each judged against what T01
//! measured on the original (`docs/16-title.md`). World 1's and world 5's codes, a wrong code, the
//! cheat code with HELP ending the level, the Amiga's Esc on the title through a key binding, and
//! game over through the high scores back to the title and START again. The rows and world 3's
//! code are `title_paths_short.rs`; KONIEC is `translated_drill.rs`'s.
//!
//! Full: six launches, about 1 000 to 1 700 steps each. Needs the owner's disks and the generated
//! tree (`translate.py --build`), and fails naming the command without it.

mod common;
mod title;

use amiga_input::{FrameInput, Joy};
use title::{ESC, Run, key};

/// `code` typed on KODY and START: the world `world_index` names and `cheat_flag` at play's first
/// pass, then 100 steps of play, all in the app as headless. The run, for what it says after.
fn code_then_start(code: [u8; 4], world_index: u32, cheat: u32) -> Run {
    let mut r = Run::to_the_title();
    let want = code.iter().fold(0u32, |acc, &d| acc << 8 | d as u32);
    assert_eq!(r.type_code(code), want, "code_digits");
    let play = r.start_and_play();
    assert_eq!(
        (r.at(play, "world_index"), r.at(play, "cheat_flag")),
        (world_index, cheat),
        "{code:?}: world_index and cheat_flag at play's first pass (step {play})"
    );
    r.hold(FrameInput::NONE, 100);
    r
}

/// World 1 runs on level 1's hand engine through the seam: the app says where it began.
fn level_1_begins_alike(r: &Run, stderr: &str) {
    let begins = r.level_1_begins.expect("level 1 began headless");
    assert!(
        stderr.contains(&format!("fd-app: level 1 begins at step {begins}\n")),
        "level 1 begins at step {begins} headless: {stderr}"
    );
}

/// 0556, the code the title starts with: world 1 (docs/16).
#[test]
fn world_1s_code_then_start_plays_world_1_as_headless() {
    let r = code_then_start([0, 5, 5, 6], 0, 0);
    let (_app, c) = r.in_the_app(false, 180);
    level_1_begins_alike(&r, &c.stderr);
}

/// 7531: world 5 (`world_index` 4).
#[test]
fn world_5s_code_then_start_plays_world_5_as_headless() {
    let r = code_then_start([7, 5, 3, 1], 4, 0);
    let _ = r.in_the_app(false, 180);
}

/// 1234, no code: world 1 as 0556 does, `cheat_flag` left as it was (0 from power-on), and the
/// KODY box back to 0556 as world 1's load list writes it (docs/16, "A wrong code").
#[test]
fn a_wrong_code_starts_world_1_without_the_cheat_as_headless() {
    let r = code_then_start([1, 2, 3, 4], 0, 0);
    assert_eq!(
        r.now("code_digits"),
        0x0005_0506,
        "world 1's load list wrote its code"
    );
    let (_app, c) = r.in_the_app(false, 180);
    level_1_begins_alike(&r, &c.stderr);
}

/// 2400, the cheat: world 1 with `cheat_flag` set; HELP (bound to F12 as `"key $5f"`) then ends
/// the level, `next_world` moving `world_index` to 1, and world 2's load goes on without a stop.
#[test]
fn the_cheat_code_then_help_ends_level_1_as_headless() {
    let mut r = code_then_start([2, 4, 0, 0], 0, 1);
    let ended = r.help_until_the_level_ends();
    assert_eq!(r.at(ended, "world_index"), 1, "next_world after HELP");
    r.hold(FrameInput::NONE, 200);
    let (_app, c) = r.in_the_app(false, 180);
    level_1_begins_alike(&r, &c.stderr);
}

/// The Amiga's Esc, bound to F11 (fd-app's own Esc quits the app and never reaches the game): on
/// the title it quits to AmigaDOS as KONIEC does (docs/16, "Esc"), so the app quits with no stop
/// and no bug zip.
#[test]
fn the_amiga_esc_on_the_title_quits_the_app_with_no_zip_as_headless() {
    let mut r = Run::to_the_title();
    let from = r.steps();
    r.until(
        "the program's exit on Esc",
        10,
        |_| key(ESC),
        |r| r.exited.is_some(),
    );
    let quit = r.exited.unwrap();
    // quit_flag, then quit_to_dos's return the step after, as docs/16 measured on the original. The
    // step after the press it lands on is the T05 case's to judge on the oracle; here, within three.
    let flag = r.changes("quit_flag", from + 1);
    assert_eq!(flag.len(), 1, "quit_flag set once: {flag:?}");
    assert_eq!(flag[0].0 + 1, quit, "the return the step after quit_flag");
    assert!(
        quit - from <= 3,
        "quit at step {quit}, Esc from {}",
        from + 1
    );
    let (_app, c) = r.in_the_app(false, 120);
    assert!(
        c.stderr.contains(&format!(
            "fd-app: the program quit to AmigaDOS at step {quit}\nfd-app: menu quit after step {quit}\n"
        )),
        "quit at step {quit} as headless, and the app with it: {}",
        c.stderr
    );
}

/// World 3 by its code (worlds 2, 4 and 6 have none, docs/16), Esc in play to give up, the game
/// over screen (play's `irq_count` stands still on it) and the high scores each let go by fire,
/// then the title taking Down again; Up and START with nothing typed go back to world 3, whose
/// load list left 2424 in the KODY box. Score 0 is under the eighth entry: no name is asked.
#[test]
fn game_over_through_the_high_scores_back_to_the_title_and_start_again_as_headless() {
    let mut r = code_then_start([2, 4, 2, 4], 2, 0);
    let esc = r.steps();
    r.press(key(ESC), 3, 0);
    // The game over screen: play's irq_count stands still for 20 steps.
    let mut still = 0;
    r.until(
        "the game over screen after Esc",
        200,
        |_| FrameInput::NONE,
        |r| {
            let n = r.steps();
            still = if r.at(n, "irq_count") == r.at(n - 1, "irq_count") {
                still + 1
            } else {
                0
            };
            still == 20
        },
    );
    assert_eq!(
        r.now("lives"),
        r.at(esc, "lives"),
        "giving up loses no life"
    );
    let fire = FrameInput::joy(Joy::FIRE);
    r.press(fire, 10, 100);
    r.press(fire, 10, 100);
    assert!(
        r.changes("title_cursor", esc).is_empty(),
        "not the title yet"
    );
    // The title is back when its rows take Down again: tapped every 40 steps until one moves.
    let after = r.steps();
    let back = r.until(
        "the title after the high scores",
        3_000,
        |n| {
            if (n - after) % 40 < 10 {
                FrameInput::joy(Joy::DOWN)
            } else {
                FrameInput::NONE
            }
        },
        |r| r.now("title_cursor") != 0,
    );
    assert_eq!(r.at(back, "title_cursor"), 1, "Down from START: KODY");
    r.hold(FrameInput::NONE, 10);
    r.press(FrameInput::joy(Joy::UP), 10, 10);
    assert_eq!(r.now("title_cursor"), 0, "Up: START");
    assert_eq!(
        r.now("code_digits"),
        0x0204_0204,
        "the KODY box kept world 3's code"
    );
    let play = r.start_and_play();
    assert_eq!(
        (r.at(play, "world_index"), r.at(play, "lives")),
        (2, 0x32),
        "world 3 again with the lives reset (BCD 32)"
    );
    r.hold(FrameInput::NONE, 50);
    let _ = r.in_the_app(false, 240);
}
