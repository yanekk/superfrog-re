//! T16, the app drill: what a player would try that no feature task listed (DESIGN §2.7, §2.8),
//! driven through the app's script layer, whose events enter where the window's are translated.
//! Tab, Esc, a lost focus, the pad, the Controls window's keys, every menu item out of turn, the
//! windows of Start Level and the recorder answered every way, and a 2 000-character note. The
//! picture at each size is `drill_sizes.rs`.
//!
//! The real windows (the Controls window's buttons, Start Level's, the save window typed and
//! pasted into, focus lost to another app with a key held) need an unlocked screen and are T68's,
//! the OS layer under `FD_E2E_OS=1`.

mod common;

use std::path::{Path, PathBuf};

use common::{FIRE, RIGHT, UP};
use fd_e2e::{App, Captured, LaunchOpts};

/// The Amiga's Tab key, as `inputs.txt` lists a key held.
const AMIGA_TAB: u8 = 0x42;

/// The app in a temporary home, saving bugs into `<home>/test-bugs`.
fn launch(max_frames: u64, scale: Option<u32>) -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s: 120,
        scale,
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    (app, bugs)
}

fn run(max_frames: u64, scale: Option<u32>, script: &str) -> (App, PathBuf, Captured) {
    let (mut app, bugs) = launch(max_frames, scale);
    let c = app.run_script_unread(script);
    assert!(c.exit.success(), "{}", c.stderr);
    (app, bugs, c)
}

/// The steps the app took the menu item `id` after.
fn menu_after(c: &Captured, id: &str) -> Vec<u64> {
    let tag = format!("fd-app: menu {id} after step ");
    c.stderr
        .lines()
        .filter_map(|l| l.strip_prefix(&tag)?.parse().ok())
        .collect()
}

fn quit_after(c: &Captured) -> u64 {
    c.stderr
        .lines()
        .find_map(|l| l.strip_prefix("fd-app: quit after step ")?.parse().ok())
        .unwrap_or_else(|| panic!("{}", c.stderr))
}

/// The Amiga keys the game was fed at step `n`.
fn keys_at(c: &Captured, n: u64) -> Vec<u8> {
    let held = common::inputs(c).get(&n).map_or(0, |f| f.keys.0);
    (0..128u8).filter(|k| held & (1u128 << k) != 0).collect()
}

fn zips(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    v.retain(|p| p.extension().is_some_and(|x| x == "zip"));
    v.sort();
    v
}

fn recording_of(zip: &Path) -> fd_core::recording::Recording {
    let bytes = std::fs::read(zip).unwrap();
    fd_conform::bug::open_zip(&bytes, fd_tools::ENGINE_BUILD)
        .unwrap_or_else(|e| panic!("{}: {e}", zip.display()))
        .0
}

#[test]
fn tab_reaches_the_game_as_the_amigas_tab_and_esc_quits_without_reaching_it() {
    let (_app, _, c) = run(
        200,
        None,
        "10: press Tab\n20: down ArrowRight\n30: down Tab\n40: up Tab\n50: press Escape\n",
    );
    // Tab is no key of the app's: the game reads it as the Amiga's Tab, as every unbound key.
    assert_eq!(keys_at(&c, 10), [AMIGA_TAB]);
    assert!((30..40).all(|n| keys_at(&c, n) == [AMIGA_TAB]), "{}", c.inputs);
    assert_eq!(common::joy(&c, 20, 49), [RIGHT; 30], "Tab took nothing from the arrow");
    assert_eq!(menu_after(&c, "quit"), [49]);
    assert_eq!(quit_after(&c), 49, "Esc quit at once");
    assert!(keys_at(&c, 49).is_empty());
}

#[test]
fn losing_focus_lets_go_of_held_keys_not_the_pad_and_play_goes_on() {
    let (_app, _, c) = run(
        60,
        None,
        "5: pad connect 1\n10: down ArrowRight\n10: down Space\n12: pad down 1 South\n\
         20: blur\n30: up ArrowRight\n40: pad up 1 South\n",
    );
    assert_eq!(common::joy(&c, 12, 19), [RIGHT | UP | FIRE; 8]);
    assert_eq!(
        common::joy(&c, 20, 39),
        [FIRE; 20],
        "the keys let go at the blur, the pad's fire held on"
    );
    assert_eq!(common::joy(&c, 40, 60), [0; 21]);
    assert_eq!(quit_after(&c), 60, "play went on");
    assert_eq!(common::hashes(&c).len(), 61);
}

#[test]
fn the_pad_s_d_pad_stick_and_buttons_play_as_the_keys() {
    let (_app, _, c) = run(
        45,
        None,
        "5: pad connect 1\n10: pad down 1 DPadRight\n20: pad up 1 DPadRight\n25: pad press 1 South\n\
         30: pad stick 1 0.9 0\n35: pad stick 1 0 0\n40: pad press 1 East\n",
    );
    assert_eq!(common::joy(&c, 10, 19), [RIGHT; 10]);
    assert_eq!(common::joy(&c, 25, 25), [FIRE]);
    assert_eq!(common::joy(&c, 30, 34), [RIGHT; 5]);
    assert_eq!(common::joy(&c, 40, 40), [FIRE]);
    assert_eq!(common::joy(&c, 41, 45), [0; 5]);
}

#[test]
fn in_the_controls_window_esc_cancels_then_closes_it_and_never_quits() {
    let (app, _, c) = run(
        40,
        None,
        "5: down ArrowRight\n10: menu controls\n10: menu controls\n11: controls up keys\n\
         12: press Escape\n13: press Escape\n14: press Tab\n15: controls close\n20: up ArrowRight\n\
         25: down ArrowLeft\n30: up ArrowLeft\n",
    );
    assert_eq!(menu_after(&c, "controls"), [9, 9]);
    assert_eq!(
        common::said(&c),
        [
            "Click a binding, then press the key or pad button for it.",
            "Press the key for Up. Esc cancels.",
            "Nothing changed.",
        ],
        "one window, opened once"
    );
    assert!(c.stderr.contains("fd-app: controls closed"), "{}", c.stderr);
    assert!(menu_after(&c, "quit").is_empty(), "neither Esc quit: {}", c.stderr);
    assert_eq!(common::joy(&c, 5, 9), [RIGHT; 5]);
    assert_eq!(
        common::joy(&c, 10, 24),
        [0; 15],
        "the arrow held when the window opened was let go"
    );
    assert_eq!(keys_at(&c, 14), [AMIGA_TAB], "closed, the game has the keyboard again");
    assert_eq!(common::joy(&c, 25, 29), [common::LEFT; 5], "{}", c.stderr);
    assert!(!app.home.join("settings.toml").exists(), "nothing changed, nothing saved");
}

#[test]
fn start_level_opened_with_the_controls_window_open_records_and_the_window_keeps_the_keyboard() {
    let (app, bugs, c) = run(
        60,
        None,
        "5: menu controls\n6: controls up keys\n10: menu start-level\nform start\n15: press KeyW\n\
         20: down ArrowRight\n30: menu stop-recording\nnote x\n40: menu quit\n",
    );
    assert!(c.stderr.contains("fd-app: started W1-1 after step 9, recording"), "{}", c.stderr);
    assert_eq!(
        common::said(&c).last().unwrap(),
        "Up is now W; it no longer works as the Amiga's W key."
    );
    assert!(
        std::fs::read_to_string(app.home.join("settings.toml")).unwrap().contains("up = [\"KeyW\"]")
    );
    let rec = recording_of(&zips(&bugs)[0]);
    assert_eq!(rec.inputs.len(), 20, "steps 10 to 29");
    assert!(
        rec.inputs.iter().all(|f| f.joy.0 == 0 && f.keys.0 == 0),
        "the window had the keyboard: the W and the arrow never reached the game"
    );
    assert_eq!(quit_after(&c), 39);
}

#[test]
fn every_menu_item_out_of_turn_start_level_cancelled_and_stop_with_no_recording_change_nothing() {
    let (_app, bugs, c) = run(
        60,
        None,
        "10: down ArrowRight\n20: menu start-level\nform cancel\n25: menu stop-recording\n\
         45: up ArrowRight\n",
    );
    assert!(c.prompts.contains("start-level: -> cancel"), "{}", c.prompts);
    assert!(!c.stderr.contains("started W1-1"), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: not recording"), "{}", c.stderr);
    assert_eq!(common::joy(&c, 10, 44), [RIGHT; 35], "play went on through both");
    // The game was not restarted: its step 21 is the one after step 20, not a new level's first.
    let mut g = common::headless();
    let i = common::inputs(&c);
    for n in 1..=21 {
        g.step(i.get(&n).copied().unwrap_or(amiga_input::FrameInput::NONE));
    }
    assert_eq!(common::hashes(&c)[21], common::hash(&g));
    assert!(zips(&bugs).is_empty());
}

#[test]
fn esc_while_recording_asks_cancel_keeps_recording_and_a_2000_character_note_is_saved_whole() {
    let note: String = (0..400).map(|k| format!("w{k:03} ")).collect();
    let note = note.trim_end();
    let (_app, bugs, c) = run(
        100,
        Some(1),
        &format!(
            "10: menu start-level\nform start\nanswer-recording cancel\nnote {note}\n\
             30: down ArrowRight\n50: press Escape\n80: menu stop-recording\n"
        ),
    );
    assert!(c.prompts.contains("recording: Quit -> cancel\n"), "{}", c.prompts);
    assert!(c.stderr.contains("fd-app: Quit cancelled; still recording"), "{}", c.stderr);
    assert_eq!(common::joy(&c, 30, 79), [RIGHT; 50], "Esc was no key of the game's");
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{found:?}");
    let rec = recording_of(&found[0]);
    assert_eq!(rec.note.len(), 1999);
    assert_eq!(rec.note, note, "the whole note");
    assert_eq!(rec.inputs.len(), 70, "steps 10 to 79, through the question");
    assert_eq!(quit_after(&c), 100);
}

#[test]
fn start_level_while_recording_saves_first_and_quit_while_recording_discards() {
    let (_app, bugs, c) = run(
        200,
        Some(1),
        "10: menu start-level\nform start\n50: menu start-level\nanswer-recording save\nnote first\n\
         form record off\nform start\n100: menu start-level\nform start\n150: menu quit\n\
         answer-recording discard\n",
    );
    let p = &c.prompts;
    assert!(
        p.contains("recording: Start Level -> save\nsave-recording: ")
            && p.contains(" -> note first\n"),
        "{p}"
    );
    assert!(c.stderr.contains("fd-app: started W1-1 after step 49\n"), "not recording: {}", c.stderr);
    assert!(p.contains("recording: Quit -> discard\n"), "{p}");
    assert_eq!(quit_after(&c), 149);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "the discarded one wrote nothing: {found:?}");
    let rec = recording_of(&found[0]);
    assert_eq!((rec.note.as_str(), rec.inputs.len()), ("first", 40));
}

