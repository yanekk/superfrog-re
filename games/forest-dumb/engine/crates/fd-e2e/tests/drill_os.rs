//! T68, the app drill's OS half: the real windows driven as a person would, through real key
//! events, real clicks (the Accessibility API) and the clipboard (DESIGN §2.7, §2.8). The Controls
//! window's buttons, Start Level's, "Recording in progress" answered every way, the save window
//! typed and pasted into, focus lost to another app with a key held, and the game's own keys. Every
//! window is captured to the scratch folder (`$TMPDIR/fd-drill-os`) for judging. T16's
//! `drill.rs` is the script-layer half.
//!
//! `FD_E2E_OS=1 cargo test --release -p fd-e2e --test drill_os -- --ignored --test-threads 1`.
//! Release: a debug build writing every frame's capture keeps the main thread too busy to answer
//! the Accessibility API (`AXError -25204`, T16). Each test refuses a locked screen first: locked,
//! the windows never become key and the Accessibility API does not see their buttons (T16).

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{FIRE, LEFT, RIGHT, UP};
use fd_e2e::{App, Captured, Key, LaunchOpts, Mods};

/// The Amiga's Tab key, as `inputs.txt` lists a key held.
const AMIGA_TAB: u8 = 0x42;
/// The app menu is titled with the process name (an unbundled binary).
const QUIT: [&str; 2] = ["fd-app", "Quit Forest Dumb"];

fn os_layer() {
    assert!(
        std::env::var_os("FD_E2E_OS").is_some_and(|v| v == "1"),
        "OS-layer tests bring up real windows and type into them; run them with FD_E2E_OS=1"
    );
    fd_e2e::os_permissions().unwrap_or_else(|e| panic!("{e}"));
    assert!(!screen_locked(), "{LOCKED}");
}

const LOCKED: &str = "the screen is locked: unlock it and run again. Locked, the app's windows \
                      never become key and the Accessibility API does not see their buttons (T16)";

/// The console session's screen is locked: `CGSSessionScreenIsLocked` in the session dictionary
/// (as `sf-e2e/tests/menus.rs`).
fn screen_locked() -> bool {
    let out = std::process::Command::new("osascript")
        .args([
            "-l",
            "JavaScript",
            "-e",
            "ObjC.import('CoreGraphics'); \
             const d = ObjC.deepUnwrap(ObjC.castRefToObject($.CGSessionCopyCurrentDictionary())); \
             String(!!(d && d.CGSSessionScreenIsLocked))",
        ])
        .output()
        .expect("running osascript");
    String::from_utf8_lossy(&out.stdout).trim() == "true"
}

fn until(what: &str, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        let locked = if screen_locked() { format!("; {LOCKED}") } else { String::new() };
        assert!(t.elapsed() < Duration::from_secs(30), "{what}: not within 30 s{locked}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Wait for the field `id` to read `want`; on timeout say what it read instead, where it first
/// differs, so a long note's failure names its cause.
fn until_text(app: &App, id: &str, want: &str) {
    let t = Instant::now();
    let mut last;
    loop {
        last = app.os_text_id(id);
        if last == want || t.elapsed() > Duration::from_secs(30) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if last != want {
        let at = last.chars().zip(want.chars()).take_while(|(a, b)| a == b).count();
        let near = |s: &str| s.chars().skip(at.saturating_sub(10)).take(30).collect::<String>();
        panic!(
            "field {id:?}: {} characters, wanted {}; first differs at {at}: {:?} against {:?}",
            last.chars().count(),
            want.chars().count(),
            near(&last),
            near(want)
        );
    }
}

fn pause(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// The app on its real windows (no script) in a temporary home, saving bugs into
/// `<home>/test-bugs`, its game window on screen.
fn os_launch(scale: Option<u32>) -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames: 30_000,
        timeout_s: 240,
        scale,
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    app.start_os();
    app.os_window();
    pause(500);
    (app, bugs)
}

/// Quit through the menu bar and wait for the app to end.
fn quit(app: &mut App) -> Captured {
    app.os_menu(&QUIT);
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    c
}

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join("fd-drill-os");
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Every window of the app on screen, captured to the scratch folder as `<name>-<k>.png`, front to
/// back, for the drill to judge. Each is checked to lie on the screen and not to be blank.
fn capture(app: &App, name: &str) {
    for (k, (id, b)) in app.os_windows().into_iter().enumerate() {
        let path = scratch().join(format!("{name}-{k}.png"));
        let ok = std::process::Command::new("screencapture")
            .args(["-x", "-o", &format!("-l{id}")])
            .arg(&path)
            .status()
            .expect("screencapture");
        assert!(ok.success(), "{name}: capturing window {id}");
        assert!(b[0] >= 0 && b[1] >= 0 && b[2] > 0 && b[3] > 0, "{name}: window {k} off screen: {b:?}");
        let shot = fd_e2e::Image::read_png(&path).unwrap();
        assert!(
            shot.rgba.chunks_exact(4).any(|p| p[3] != 0),
            "{name}: window {k} captured blank (Screen Recording?)"
        );
        eprintln!("captured {} {b:?}", path.display());
    }
}

fn has(app: &App, title: &str) -> bool {
    app.os_buttons().iter().any(|b| b == title)
}

fn recording(app: &App) -> bool {
    app.os_menu_item(&["Game"], "Stop Recording").enabled
}

fn controls_open(app: &App) -> bool {
    has(app, "Restore Defaults")
}

fn start_level_open(app: &App) -> bool {
    has(app, "Start") && has(app, "Cancel")
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

fn clipboard() -> String {
    let out = std::process::Command::new("pbpaste").output().expect("pbpaste");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn set_clipboard(text: &str) {
    use std::io::Write;
    let mut p = std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("pbcopy");
    p.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    p.wait().unwrap();
}

/// The steps whose joystick had `bit`, in order.
fn steps_with(c: &Captured, bit: u8) -> Vec<u64> {
    common::inputs(c)
        .into_iter()
        .filter(|(_, f)| f.joy.0 & bit != 0)
        .map(|(n, _)| n)
        .collect()
}

fn steps_with_key(c: &Captured, key: u8) -> Vec<u64> {
    common::inputs(c)
        .into_iter()
        .filter(|(_, f)| f.keys.0 & (1u128 << key) != 0)
        .map(|(n, _)| n)
        .collect()
}

/// Steps in a run with no gap.
fn contiguous(v: &[u64]) -> bool {
    v.windows(2).all(|w| w[1] == w[0] + 1)
}

/// The Controls window through its buttons and real keys: the titles, Up rebound to W, Esc
/// cancelling then closing, Restore Defaults, the close button; the game has the keyboard after.
#[test]
#[ignore = "OS layer"]
fn os_layer_controls_window_buttons_rebind_esc_defaults_and_close() {
    os_layer();
    let (mut app, _) = os_launch(None);
    app.os_menu(&["Game", "Controls…"]);
    until("the Controls window", || controls_open(&app));
    assert_eq!(app.os_text_id("up-keys"), "Space", "Up shows Space");
    assert_eq!(app.os_text_id("fire-keys"), "Left Option", "Fire shows Left Option");
    for id in ["down-keys", "left-keys", "right-keys"] {
        let t = app.os_text_id(id);
        assert!(!t.contains("Space") && !t.contains("Left Option"), "{id}: {t}");
    }
    capture(&app, "controls");

    app.os_press_id("up-keys");
    until("asking for Up", || app.os_text_id("status") == "Press the key for Up. Esc cancels.");
    app.os_key(Key::Char('w'));
    until("Up is W", || app.os_text_id("up-keys") == "W");
    capture(&app, "controls-rebound");
    let saved = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(saved.contains("up = [\"KeyW\"]"), "{saved}");

    app.os_press_id("fire-keys");
    until("asking for Fire", || app.os_text_id("status").starts_with("Press the key for Fire"));
    app.os_key(Key::Escape);
    until("cancelled", || app.os_text_id("status") == "Nothing changed.");
    assert_eq!(app.os_text_id("fire-keys"), "Left Option");
    assert!(controls_open(&app), "the first Esc cancels, the window stays");

    app.os_press_id("defaults");
    until("Up is Space again", || app.os_text_id("up-keys") == "Space");
    assert_eq!(app.os_text_id("fire-keys"), "Left Option");

    app.os_key(Key::Escape);
    until("closed by Esc", || !controls_open(&app));
    app.os_menu(&["Game", "Controls…"]);
    until("the Controls window again", || controls_open(&app));
    app.os_close_window("Controls");
    until("closed by its close button", || !controls_open(&app));

    // The game window has the keyboard again: Right held reaches the game.
    pause(300);
    app.os_key_down(Key::Right);
    pause(500);
    app.os_key_up(Key::Right);
    pause(200);
    let c = quit(&mut app);
    assert_eq!(c.stderr.matches("fd-app: controls closed").count(), 2, "{}", c.stderr);
    let right = steps_with(&c, RIGHT);
    assert!(right.len() >= 10 && contiguous(&right), "Right after the window closed: {right:?}");
    let saved = std::fs::read_to_string(app.home.join("settings.toml")).unwrap();
    assert!(saved.contains("up = [\"Space\"]"), "{saved}");
}

/// Start Level through its real controls: W1-1 only, Esc cancels, Return starts recording, the
/// recorder unticked starts without one.
#[test]
#[ignore = "OS layer"]
fn os_layer_start_level_offers_w1_1_esc_cancels_return_records_unticked_does_not() {
    os_layer();
    let (mut app, bugs) = os_launch(None);
    app.os_menu(&["Game", "Start Level…"]);
    until("the window", || start_level_open(&app));
    capture(&app, "start-level");
    // The modal window is on layer 8, above the game's: listed, so captured with it.
    assert_eq!(app.os_windows().len(), 2, "the game and Start Level windows");
    assert_eq!(app.os_popup_titles("level"), ["W1-1"]);
    assert!(app.os_checked_id("record"), "the recorder is ticked by default");
    app.os_activate();
    app.os_key(Key::Escape);
    until("closed by Esc", || !start_level_open(&app));
    pause(300);
    assert!(!recording(&app));

    app.os_menu(&["Game", "Start Level…"]);
    until("the window again", || start_level_open(&app));
    app.os_key(Key::Return);
    until("closed by Return", || !start_level_open(&app));
    until("recording", || recording(&app));

    // While recording, Start Level asks first; Discard opens the window, recorder unticked.
    app.os_menu(&["Game", "Start Level…"]);
    until("the question", || has(&app, "Save…"));
    app.os_press_button("Discard");
    until("the window after Discard", || start_level_open(&app));
    app.os_press_id("record");
    assert!(!app.os_checked_id("record"));
    app.os_key(Key::Return);
    until("closed", || !start_level_open(&app));
    pause(500);
    assert!(!recording(&app), "unticked: no recording");
    let c = quit(&mut app);
    let started: Vec<&str> = c.stderr.lines().filter(|l| l.starts_with("fd-app: started W1-1")).collect();
    assert_eq!(started.len(), 2, "{}", c.stderr);
    assert!(started[0].ends_with(", recording") && !started[1].ends_with(", recording"), "{started:?}");
    assert!(c.stderr.contains("fd-app: recording discarded"), "{}", c.stderr);
    assert!(zips(&bugs).is_empty());
}

/// "Recording in progress", from ⌘Q, Esc and Start Level: Cancel (the button and Esc) keeps
/// recording, Discard drops it, Save… opens the save window.
#[test]
#[ignore = "OS layer"]
fn os_layer_recording_in_progress_cancel_discard_and_save() {
    os_layer();
    let (mut app, bugs) = os_launch(None);
    app.os_menu(&["Game", "Start Level…"]);
    until("the window", || start_level_open(&app));
    app.os_key(Key::Return);
    until("recording", || recording(&app));

    app.os_activate();
    pause(300);
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    until("⌘Q: the question", || has(&app, "Save…"));
    capture(&app, "recording-in-progress");
    app.os_key(Key::Escape);
    until("⌘Q: the question gone", || !has(&app, "Save…"));
    assert!(recording(&app), "Esc is Cancel: still recording");

    app.os_activate();
    pause(300);
    app.os_key(Key::Escape);
    until("Esc: the question", || has(&app, "Save…"));
    app.os_press_button("Cancel");
    until("Esc: the question gone", || !has(&app, "Save…"));
    assert!(recording(&app));

    // The question's buttons go before its window does: a menu click while the window is still
    // closing is lost, so wait for the game's window alone.
    until("the question's window closed", || app.os_windows().len() == 1);
    app.os_menu(&["Game", "Start Level…"]);
    until("Start Level: the question", || has(&app, "Save…"));
    app.os_press_button("Save…");
    until("the save window", || has(&app, "Save") && has(&app, "Discard"));
    app.os_press_id("discard");
    until("the Start Level window after the save window", || start_level_open(&app));
    app.os_press_button("Cancel");
    until("closed", || !start_level_open(&app));
    assert!(!recording(&app), "discarded in the save window");

    // Recording again, then ⌘Q → Discard quits.
    app.os_menu(&["Game", "Start Level…"]);
    until("the window", || start_level_open(&app));
    app.os_key(Key::Return);
    until("recording again", || recording(&app));
    app.os_activate();
    pause(300);
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    until("⌘Q again: the question", || has(&app, "Save…"));
    app.os_press_button("Discard");
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    for line in ["fd-app: Quit cancelled; still recording", "fd-app: recording discarded"] {
        assert!(c.stderr.contains(line), "{line}: {}", c.stderr);
    }
    assert_eq!(c.stderr.matches("cancelled; still recording").count(), 2, "{}", c.stderr);
    assert!(zips(&bugs).is_empty());
}

/// The save window: Esc and ⌘. keep it; a 2 000-character note pasted with line breaks is one line,
/// a word typed after it, saved whole; Return saves; the notice's OK.
#[test]
#[ignore = "OS layer"]
fn os_layer_save_window_keeps_on_esc_and_saves_a_long_pasted_note_whole() {
    os_layer();
    let kept = clipboard();
    let (mut app, bugs) = os_launch(Some(1));
    app.os_menu(&["Game", "Start Level…"]);
    until("the window", || start_level_open(&app));
    app.os_key(Key::Return);
    until("recording", || recording(&app));
    pause(1000);
    app.os_activate();
    pause(300);
    app.os_key_with(Mods::COMMAND, Key::Char('.'));
    until("the save window", || has(&app, "Save") && has(&app, "Discard"));
    capture(&app, "save-window-1x");
    for (mods, key) in [(Mods::default(), Key::Escape), (Mods::COMMAND, Key::Char('.'))] {
        app.os_key_with(mods, key);
        pause(800);
        assert!(has(&app, "Save") && has(&app, "Discard"), "{key:?} kept the save window");
    }
    // 400 words of five characters, a line break every 50: 2 000 characters with the breaks.
    let words: Vec<String> = (0..400).map(|k| format!("w{k:03}")).collect();
    let pasted: String = words
        .chunks(50)
        .map(|c| c.join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(pasted.len(), 1999);
    set_clipboard(&pasted);
    app.os_focus_id("note");
    app.os_key_with(Mods::COMMAND, Key::Char('a'));
    app.os_key_with(Mods::COMMAND, Key::Char('v'));
    let one_line = pasted.replace('\n', " ");
    until_text(&app, "note", &one_line);
    for k in [Key::Space, Key::Char('e'), Key::Char('n'), Key::Char('d')] {
        app.os_key(k);
    }
    let want = format!("{one_line} end");
    until_text(&app, "note", &want);
    capture(&app, "save-window-note");
    app.os_key(Key::Return);
    until("the saved notice", || has(&app, "OK"));
    capture(&app, "saved-notice");
    app.os_press_button("OK");
    until("not recording", || !recording(&app));
    let c = quit(&mut app);
    set_clipboard(&kept);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{found:?}");
    let rec = recording_of(&found[0]);
    assert_eq!(rec.note, want, "the whole note, one line");
    assert!(c.stderr.contains("fd-app: recording saved to "), "{}", c.stderr);
}

/// Left held, focus lost to Finder: the game lets go of Left and play goes on; back, nothing is
/// held, and a key pressed then reaches the game.
#[test]
#[ignore = "OS layer"]
fn os_layer_focus_lost_with_left_held_lets_go_and_play_goes_on() {
    os_layer();
    let (mut app, _) = os_launch(None);
    app.os_activate();
    pause(300);
    app.os_key_down(Key::Left);
    pause(600);
    let away = std::process::Command::new("open").args(["-a", "Finder"]).status().expect("open");
    assert!(away.success());
    pause(1500);
    app.os_activate();
    pause(800);
    // Left was never let go by the keyboard until now: the release reaches a game holding nothing.
    app.os_key_up(Key::Left);
    pause(300);
    app.os_key_down(Key::Right);
    pause(400);
    app.os_key_up(Key::Right);
    pause(300);
    let c = quit(&mut app);
    let left = steps_with(&c, LEFT);
    let right = steps_with(&c, RIGHT);
    assert!(left.len() >= 10 && contiguous(&left), "Left held until the blur: {left:?}");
    assert!(!right.is_empty() && contiguous(&right), "{right:?}");
    // From the blur to Right, nothing: about 1.5 s away and 1.1 s back, at 50 steps a second.
    let gap = right[0] - left.last().unwrap() - 1;
    assert!(gap >= 75, "Left let go at the blur, not held through it: {gap} steps of nothing");
    let quit_at = common::hashes(&c).len() as u64;
    assert!(quit_at > right.last().unwrap() + 5, "play went on through the switch");
}

/// The game's keys by real key events: Fire on Left Option, Up on Space, Tab as the Amiga's Tab.
#[test]
#[ignore = "OS layer"]
fn os_layer_fire_on_left_option_up_on_space_and_tab_reach_the_game() {
    os_layer();
    let (mut app, _) = os_launch(Some(1));
    capture(&app, "game-1x");
    app.os_activate();
    pause(300);
    for key in [Key::OptionLeft, Key::Space, Key::Tab] {
        app.os_key_down(key);
        pause(400);
        app.os_key_up(key);
        pause(400);
    }
    let c = quit(&mut app);
    let (fire, up, tab) = (steps_with(&c, FIRE), steps_with(&c, UP), steps_with_key(&c, AMIGA_TAB));
    assert!(fire.len() >= 5 && contiguous(&fire), "Fire: {fire:?}\n{}", c.inputs);
    assert!(up.len() >= 5 && contiguous(&up), "Up: {up:?}");
    assert!(tab.len() >= 5 && contiguous(&tab), "Tab: {tab:?}");
    assert!(fire.last() < up.first() && up.last() < tab.first(), "in order, one at a time");
    let only = |v: &[u64], bit: u8| v.iter().all(|n| common::joy(&c, *n, *n)[0] == bit);
    assert!(only(&fire, FIRE) && only(&up, UP), "each its own bit only");
    assert!(tab.iter().all(|n| common::joy(&c, *n, *n)[0] == 0), "Tab is no joystick bit");
}

/// Every window at the default scale (2×), for judging beside the 1× captures the tests above
/// take: the game, Controls, Start Level, the question and the save window.
#[test]
#[ignore = "OS layer"]
fn os_layer_every_window_at_the_default_scale_is_on_screen_and_captured() {
    os_layer();
    let (mut app, _) = os_launch(None);
    capture(&app, "game-2x");
    app.os_menu(&["Game", "Controls…"]);
    until("the Controls window", || controls_open(&app));
    capture(&app, "controls-2x");
    app.os_close_window("Controls");
    until("closed", || !controls_open(&app));
    app.os_menu(&["Game", "Start Level…"]);
    until("the window", || start_level_open(&app));
    capture(&app, "start-level-2x");
    // The modal window is on layer 8, above the game's: listed, so captured with it.
    assert_eq!(app.os_windows().len(), 2, "the game and Start Level windows");
    app.os_key(Key::Return);
    until("recording", || recording(&app));
    pause(500);
    capture(&app, "recording-bar-2x");
    app.os_menu(&["Game", "Stop Recording"]);
    until("the save window", || has(&app, "Save") && has(&app, "Discard"));
    capture(&app, "save-window-2x");
    app.os_press_id("discard");
    until("not recording", || !recording(&app));
    let c = quit(&mut app);
    assert!(c.stderr.contains("fd-app: recording discarded"), "{}", c.stderr);
}
