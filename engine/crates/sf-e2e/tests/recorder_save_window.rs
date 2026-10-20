//! bug-recorder T10: the save window after STOP and the "Recording in progress" question, the real
//! AppKit windows driven by real clicks and keys (DESIGN §2.2, §2.4, §2.11). The OS layer: each run
//! is scripted only to start the level recording (T08's dialog is its own task) and asks its
//! questions natively (`native-ui`); everything after is the person's way in.
//!
//! `SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1`

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sf_core::recording::{Location, Recording, ZIP_RECORDING, ZIP_SCREENSHOT};
use sf_e2e::{App, Captured, Image, Key, LaunchOpts, Mods};

fn os_layer() {
    assert!(
        std::env::var_os("SF_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests bring up real windows and type into them; run them with SF_E2E_OS=1"
    );
    sf_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
}

fn until(what: &str, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(30), "{what}: not within 30 s");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The app running `script` (after `native-ui`) in a temporary home at `scale`, saving bugs into
/// `<home>/test-bugs`.
fn launch(scale: u32, fullscreen: bool, max_frames: u64, script: &str) -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s: 150,
        scale: Some(scale),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
    if fullscreen {
        app.opts.extra_args.push("--allow-fullscreen".into());
        std::fs::write(app.home.join("settings.toml"), "fullscreen = true\n").unwrap();
    }
    app.start_script(&format!("native-ui\n{script}"));
    app.os_window();
    (app, bugs)
}

fn zips(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    v.retain(|p| p.extension().is_some_and(|x| x == "zip"));
    v.sort();
    v
}

fn read_zip(path: &Path) -> (Recording, Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let entries = sf_disk::zip::read_entries(&bytes, |_| true, 1 << 26).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [ZIP_RECORDING, ZIP_SCREENSHOT], "{}", path.display());
    let text = String::from_utf8(entries[0].bytes.clone()).unwrap();
    (Recording::parse(&text).unwrap(), entries[1].bytes.clone())
}

/// The step a menu command came after (`superfrog: menu <id> after step N`), the `k`th time.
fn after_step(stderr: &str, id: &str, k: usize) -> u64 {
    let pre = format!("superfrog: menu {id} after step ");
    stderr
        .lines()
        .filter_map(|l| l.strip_prefix(&pre))
        .nth(k)
        .unwrap_or_else(|| panic!("no {k}th `{pre}` in {stderr}"))
        .parse()
        .unwrap()
}

/// Type `keys` into whatever has the keyboard.
fn type_keys(app: &mut App, keys: &[Key]) {
    for &k in keys {
        app.os_key(k);
    }
}

fn clipboard() -> String {
    let out = std::process::Command::new("pbpaste").output().expect("pbpaste");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stop_recording_enabled(app: &App) -> bool {
    app.os_menu_item(&["Game"], "Stop Recording").enabled
}

/// The save window's screenshot area is the game picture, not black: its capture holds the
/// picture's colours somewhere.
fn save_window_capture(app: &App) -> Image {
    let windows = app.os_windows();
    // The sheet is the app's front window.
    app.os_capture_window(windows[0].0)
}

/// STOP at step 80 by the script; then the window, a note typed with keys the game also uses
/// (arrows, Space, letters), Save by Return, the notice's OK; the zip, the clipboard, and no frog
/// input after STOP's frame.
fn stop_save_note(scale: u32, fullscreen: bool) {
    os_layer();
    let (mut app, bugs) = launch(scale, fullscreen, 260, "1: start-level W1-1 record\n80: menu stop-recording\n");
    until("the save window", || app.os_buttons().iter().any(|b| b == "Save"));
    let location = app.os_text_id("location");
    let name = app.os_text_id("name");
    assert!(location.starts_with("Location: W1-1:"), "{location}");
    assert!(name.starts_with("Will save as bug-W1-1-") && name.ends_with(".zip"), "{name}");
    let shot = save_window_capture(&app);
    assert!(common::not_black(&shot), "the save window shows the screenshot");
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    type_keys(&mut app, &[Key::Char('b'), Key::Char('u'), Key::Char('g'), Key::Space, Key::Left, Key::Right, Key::Up, Key::Char('x')]);
    until("the note typed", || app.os_text_id("note") == "bug x");
    app.os_key(Key::Return);
    until("the saved notice", || app.os_buttons().iter().any(|b| b == "OK"));
    app.os_press_button("OK");
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{found:?}");
    let path = found[0].display().to_string();
    assert_eq!(name, format!("Will save as {}", found[0].file_name().unwrap().to_string_lossy()));
    assert_eq!(clipboard(), path, "the path is on the clipboard");
    assert!(c.prompts.contains(&format!("clipboard: {path}\n")), "{}", c.prompts);
    assert!(c.prompts.contains(&format!("notice: The bug recording is saved as {path}.")), "{}", c.prompts);
    let (rec, png) = read_zip(&found[0]);
    assert_eq!(rec.note, "bug x");
    let stop = after_step(&c.stderr, "stop-recording", 0);
    assert_eq!(rec.inputs.len() as u64, stop, "the recording ends at STOP's frame");
    assert_eq!(location, format!("Location: {}", rec.stopped_at.unwrap()));
    assert!(matches!(rec.stopped_at, Some(Location::Level { .. })));
    assert!(png.starts_with(b"\x89PNG"));
    // Typing in the note moved nothing: no step after STOP saw an input.
    let moved: Vec<&str> = c
        .inputs
        .lines()
        .filter(|l| l.split(' ').next().and_then(|n| n.parse::<u64>().ok()).is_some_and(|n| n > stop))
        .collect();
    assert!(moved.is_empty(), "the note's keys reached the game: {moved:?}");
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_stop_saves_the_typed_note_at_1x() {
    stop_save_note(1, false);
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_stop_saves_the_typed_note_at_3x() {
    stop_save_note(3, false);
}

#[test]
#[ignore = "OS layer, takes over the display: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_stop_saves_the_typed_note_fullscreen() {
    stop_save_note(1, true);
}

#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_discard_writes_nothing_and_the_game_resumes_not_recording() {
    os_layer();
    let (mut app, bugs) = launch(1, false, 400, "1: start-level W1-1 record\n80: menu stop-recording\n");
    until("the save window", || app.os_buttons().iter().any(|b| b == "Discard"));
    app.os_activate();
    type_keys(&mut app, &[Key::Char('n')]);
    app.os_press_id("discard");
    until("the window gone", || !app.os_buttons().iter().any(|b| b == "Discard"));
    assert!(!stop_recording_enabled(&app), "not recording");
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(zips(&bugs).is_empty(), "{:?}", zips(&bugs));
    assert!(c.stderr.contains("superfrog: recording discarded"), "{}", c.stderr);
    assert!(c.dir.join("frame-00400.png").exists(), "the game ran on to its cap");
    assert!(c.prompts.contains("-> discard\n"), "{}", c.prompts);
}

/// Load State 2 from the menu bar, while recording: the question; Cancel keeps recording, Discard
/// loads.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_load_state_asks_cancel_keeps_recording_and_discard_loads() {
    os_layer();
    let (mut app, bugs) = launch(1, false, 1500, "1: start-level W1-1 record\n20: menu save-state 2\n");
    until("recording", || stop_recording_enabled(&app));
    until("slot 2 filled", || app.os_menu_item(&["Game", "Load State"], "2").enabled);
    app.os_menu(&["Game", "Load State", "2"]);
    until("the question", || app.os_buttons().iter().any(|b| b == "Save…"));
    let buttons = app.os_buttons();
    for b in ["Save…", "Discard", "Cancel"] {
        assert!(buttons.iter().any(|x| x == b), "{b} in {buttons:?}");
    }
    app.os_press_button("Cancel");
    until("the question gone", || !app.os_buttons().iter().any(|b| b == "Save…"));
    assert!(stop_recording_enabled(&app), "Cancel: still recording");
    app.os_menu(&["Game", "Load State", "2"]);
    app.os_press_button("Discard");
    until("not recording", || !stop_recording_enabled(&app));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(zips(&bugs).is_empty());
    let p = &c.prompts;
    assert!(p.contains("recording: Load State 2 -> cancel\n"), "{p}");
    assert!(p.contains("recording: Load State 2 -> discard\n"), "{p}");
    assert_eq!(c.stderr.matches("superfrog: loaded state 2").count(), 1, "{}", c.stderr);
    let cancelled = c.stderr.find("Load State 2 cancelled; still recording").expect("cancelled");
    let loaded = c.stderr.find("superfrog: loaded state 2").unwrap();
    assert!(cancelled < loaded);
}

/// Load State 2's Save…: the save window, then the state loads after Save.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_load_state_save_opens_the_save_window_then_loads() {
    os_layer();
    let (mut app, bugs) = launch(1, false, 1500, "1: start-level W1-1 record\n20: menu save-state 2\n");
    until("slot 2 filled", || app.os_menu_item(&["Game", "Load State"], "2").enabled);
    app.os_menu(&["Game", "Load State", "2"]);
    app.os_press_button("Save…");
    until("the save window", || app.os_buttons().iter().any(|b| b == "Save"));
    app.os_activate();
    type_keys(&mut app, &[Key::Char('l')]);
    app.os_press_id("save");
    until("the saved notice", || app.os_buttons().iter().any(|b| b == "OK"));
    app.os_press_button("OK");
    until("not recording", || !stop_recording_enabled(&app));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(read_zip(&found[0]).0.note, "l");
    let saved = c.stderr.find("superfrog: recording saved to").expect("saved");
    let loaded = c.stderr.find("superfrog: loaded state 2").expect("loaded");
    assert!(saved < loaded, "saved, then loaded");
}

/// Quit while recording, by ⌘Q and by the window's close button: the same question.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_quit_and_the_close_button_ask_first() {
    os_layer();
    let (mut app, bugs) = launch(1, false, 1500, "1: start-level W1-1 record\n");
    until("recording", || stop_recording_enabled(&app));
    app.os_activate();
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    until("the question", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_press_button("Cancel");
    until("the question gone", || !app.os_buttons().iter().any(|b| b == "Save…"));
    assert!(stop_recording_enabled(&app), "Cancel: still recording, still running");
    app.os_close_window("Superfrog");
    until("the question again", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_press_button("Discard");
    let c: Captured = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert!(zips(&bugs).is_empty());
    let p = &c.prompts;
    assert_eq!(p.matches("recording: Quit -> ").count(), 2, "{p}");
    assert!(p.contains("recording: Quit -> cancel\n") && p.contains("recording: Quit -> discard\n"), "{p}");
}

/// A bugs folder that cannot be written: Save says so, the window stays open with the note, and
/// Discard after it writes nothing.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_save_window -- --ignored --test-threads 1"]
fn os_layer_a_read_only_bugs_dir_keeps_the_window_open_with_the_note() {
    use std::os::unix::fs::PermissionsExt;
    os_layer();
    let (mut app, bugs) = launch(1, false, 400, "1: start-level W1-1 record\n80: menu stop-recording\n");
    std::fs::create_dir_all(&bugs).unwrap();
    std::fs::set_permissions(&bugs, std::fs::Permissions::from_mode(0o555)).unwrap();
    until("the save window", || app.os_buttons().iter().any(|b| b == "Save"));
    app.os_activate();
    type_keys(&mut app, &[Key::Char('r'), Key::Char('o')]);
    until("the note typed", || app.os_text_id("note") == "ro");
    app.os_press_id("save");
    until("the error", || app.os_buttons().iter().any(|b| b == "OK"));
    app.os_press_button("OK");
    until("the error gone", || !app.os_buttons().iter().any(|b| b == "OK"));
    assert_eq!(app.os_text_id("note"), "ro", "the note is kept");
    assert!(app.os_buttons().iter().any(|b| b == "Save"), "the window stays open");
    app.os_press_id("discard");
    let c = app.finish_unread();
    std::fs::set_permissions(&bugs, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(std::fs::read_dir(&bugs).unwrap().count(), 0, "nothing written, no temporary left");
    let lines: Vec<&str> = c.prompts.lines().filter(|l| !l.starts_with("start-level:")).collect();
    assert_eq!(lines.len(), 3, "{}", c.prompts);
    assert!(lines[0].ends_with("-> note ro"), "{}", lines[0]);
    assert!(lines[1].starts_with("notice: The bug recording could not be saved as “"), "{}", lines[1]);
    assert!(lines[2].ends_with("-> discard"), "{}", lines[2]);
}
