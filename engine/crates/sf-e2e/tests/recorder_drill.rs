//! bug-recorder T11, the recorder drill: the recorder used as the owner will, end to end. Start
//! Level, W1-1 played to its exit (no forced exit: a recording holds inputs only, so a replay of a
//! forced one never reaches the tally), the tally, the slot machine, W1-2, STOP with a note; then
//! the zip played back by `superfrog --replay` and headless by `sf-conform bug replay`'s code, both
//! ending on its screenshot (DESIGN §2.2–§2.7).


use std::path::{Path, PathBuf};

use sf_conform::bug;
use sf_conform::{InputScript, case::script_path};
use sf_core::input::{FrameInput, Joy};
use sf_core::recording::{LevelSetup, Location, Recording, ZIP_RECORDING, ZIP_SCREENSHOT};
use sf_core::{Game, Rng, Scene};
use sf_e2e::{App, Captured, Image, LaunchOpts};

const LEVEL_STATE: u32 = sf_core::layout::h0(0xa06);
/// Fire a frame long every `SPIN_EVERY` frames from `SPIN_FROM` after the exit, as `slot-fire`
/// plays the slot machine: the menu's pointer never moves, so each round spins, until the coins
/// (8 on this route) are spent and the slot machine hands over to the world map.
const SPIN_FROM: u32 = 172;
const SPIN_EVERY: u32 = 80;
/// Frames of W1-2 played before STOP.
const W1_2_FRAMES: u32 = 300;

/// Where the chain's stretches begin, in recording frames (0 the level's first).
#[derive(Debug, Clone, Copy)]
struct Marks {
    exit: u32,
    slot: u32,
    w1_2: u32,
}

/// The chain's inputs, as the recording should hold them: `w1-1-exit-engine.txt` to W1-1's exit,
/// `tally-slot` in the tally's wait (up, then up and fire: the slot machine), `slot-fire`'s spins
/// till its coins run out, then W1-2 for [`W1_2_FRAMES`]. Found on the engine headless from the
/// start the app's Start Level makes under a script (the dialog's defaults, seed 0 0).
fn chain() -> (Vec<FrameInput>, Marks) {
    let route = InputScript::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/w1-1-exit-engine.txt")).unwrap();
    let tally = InputScript::load(&script_path("tally-slot")).unwrap();
    let mut g = Game::start_level(sf_conform::display_data(), &LevelSetup::default(), Rng::seeded(0, 0));
    let (mut exit, mut slot, mut w1_2) = (None, None, None);
    let mut inputs = Vec::new();
    let mut n = 0u32;
    while w1_2.is_none_or(|s| n < s + W1_2_FRAMES) {
        // The level runs at −2; the exit sets 1 (the first frame or two are −1, T132).
        if exit.is_none() && n > 10 && (g.state().l(LEVEL_STATE) as i32) >= 0 {
            exit = Some(n);
        }
        if slot.is_none() && matches!(g.scene(), Scene::Slot { .. }) {
            slot = Some(n);
        }
        if w1_2.is_none() && slot.is_some() && g.progress().level == 2 && g.scene() == Scene::Level {
            w1_2 = Some(n);
        }
        let i = match (exit, w1_2) {
            (None, _) => route.input_at(n),
            (Some(e), None) if n >= e + SPIN_FROM => {
                let spin = (n - e - SPIN_FROM) % SPIN_EVERY == 0;
                if spin { FrameInput::joy(Joy::FIRE) } else { FrameInput::NONE }
            }
            (Some(e), None) => tally.input_at(n - e),
            (Some(_), Some(_)) => FrameInput::NONE,
        };
        g.step(i);
        inputs.push(i);
        n += 1;
        assert!(n < 12_000, "W1-2 not reached: exit {exit:?}, slot {slot:?}");
    }
    let marks = Marks { exit: exit.unwrap(), slot: slot.unwrap(), w1_2: w1_2.unwrap() };
    (inputs, marks)
}

/// The host keys that hold each joystick bit (`input.rs`'s defaults).
const KEYS: [(Joy, &str); 5] = [
    (Joy::UP, "ArrowUp"),
    (Joy::DOWN, "ArrowDown"),
    (Joy::LEFT, "ArrowLeft"),
    (Joy::RIGHT, "ArrowRight"),
    (Joy::FIRE, "Space"),
];

/// `inputs` as the app's script lines: recording frame `i` is the app's step `i + 1` after a
/// `1: start-level`, and a line at `N:` lands before step N, so each change is a key down or up at
/// `i + 1`.
fn key_lines(inputs: &[FrameInput]) -> String {
    let mut s = String::new();
    let mut held = Joy::NONE;
    for (i, f) in inputs.iter().enumerate() {
        for (bit, key) in KEYS {
            let (was, now) = (held.0 & bit.0 != 0, f.joy.0 & bit.0 != 0);
            if was != now {
                s += &format!("{}: {} {key}\n", i + 1, if now { "down" } else { "up" });
            }
        }
        held = f.joy;
    }
    s
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
    let entries = sf_disk::zip::read_entries(&std::fs::read(path).unwrap(), |_| true, 1 << 26).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [ZIP_RECORDING, ZIP_SCREENSHOT], "{}", path.display());
    let rec = Recording::parse(std::str::from_utf8(&entries[0].bytes).unwrap()).unwrap();
    (rec, entries[1].bytes.clone())
}

fn png_image(png: &[u8], scratch: &Path) -> Image {
    let p = scratch.join("screenshot-check.png");
    std::fs::write(&p, png).unwrap();
    Image::read_png(&p).unwrap()
}

/// Records the chain in the app and returns it with its zip and the capture.
fn record_chain(note: &str) -> (App, Vec<FrameInput>, Marks, PathBuf, Captured) {
    let (inputs, marks) = chain();
    let n = inputs.len() as u64;
    let mut app = App::launch(LaunchOpts {
        max_frames: n + 20,
        timeout_s: 600,
        scale: Some(1),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
    let script = format!(
        "note {note}\n1: start-level W1-1 record\n{}{}: menu stop-recording\n",
        key_lines(&inputs),
        n + 1
    );
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "one zip: {found:?}\n{}", c.prompts);
    (app, inputs, marks, found[0].clone(), c)
}

#[test]
#[ignore = "slow: 3.5–6 min (7 487 frames captured twice); cargo test -p sf-e2e --test recorder_drill -- --ignored w1_1_played"]
fn w1_1_played_to_its_exit_through_the_slot_machine_to_w1_2_saves_and_replays_to_its_screenshot() {
    let note = "drill: W1-1 to W1-2 through the slot machine";
    let (mut app, inputs, marks, zip, c) = record_chain(note);
    let n = inputs.len();
    eprintln!("{marks:?}, {n} frames");
    let name = zip.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("bug-W1-2-"), "named for the level STOP was pressed in: {name}");
    let (rec, png) = read_zip(&zip);
    assert_eq!(rec.inputs, inputs, "the recording holds every frame's input, as played");
    assert_eq!(rec.note, note);
    assert!(matches!(rec.stopped_at, Some(Location::Level { id, .. }) if id.to_string() == "W1-2"), "{:?}", rec.stopped_at);
    let shot = png_image(&png, &c.dir);
    assert_eq!(shot, c.read_frame(n as u64), "screenshot.png is the frame STOP was pressed on");

    // Headless, as `sf-conform bug replay` plays it: through the tally and the slot machine to
    // W1-2, ending on the screenshot and where it says it stopped.
    let mut screens: Vec<String> = Vec::new();
    let r = bug::replay(&rec, |_, g, _| {
        if let Location::Screen(s) = g.location()
            && screens.last().map(String::as_str) != Some(s)
        {
            screens.push(s.to_string());
        }
    });
    assert_eq!(screens, ["tally", "slot machine", "world map"], "the stretches it went through");
    assert_eq!(Some(r.location), rec.stopped_at);
    assert_eq!(bug::screenshot_differs(&png, &r.last), Ok(0), "the headless replay ends on screenshot.png");

    // `superfrog --replay`: the same frames, ending on the screenshot.
    app.opts.max_frames = n as u64 + 10;
    app.opts.extra_args = vec!["--replay".into(), zip.display().to_string()];
    let p = app.run_script_unread("");
    assert!(p.exit.success(), "{}", p.stderr);
    assert!(p.stderr.contains(&format!("superfrog: replay finished at frame {n}\n")), "{}", p.stderr);
    for k in [marks.exit, marks.slot, marks.w1_2, n as u32] {
        assert!(p.read_frame(k as u64) == c.read_frame(k as u64), "the replay's frame {k} is the recording run's");
    }
    assert_eq!(p.read_frame(n as u64), shot);
}

// The OS layer (`SF_E2E_OS=1`): what a person would try that no feature task listed, through the
// real windows, keys and clipboard.

use std::time::{Duration, Instant};

use sf_e2e::{Key, Mods};

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

/// The app running `script` (after `native-ui`) in a temporary home at 1×, unbounded by its
/// script's end (`max_frames`), saving bugs into `<home>/test-bugs`.
fn os_launch(max_frames: u64, script: &str) -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s: 180,
        scale: Some(1),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
    app.start_script(&format!("native-ui\n{script}"));
    app.os_window();
    (app, bugs)
}

fn clipboard() -> String {
    let out = std::process::Command::new("pbpaste").output().expect("pbpaste");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn set_clipboard(text: &str) {
    use std::io::Write;
    let mut p = std::process::Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn().expect("pbcopy");
    p.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    p.wait().unwrap();
}

fn recording(app: &App) -> bool {
    app.os_menu_item(&["Game"], "Stop Recording").enabled
}

fn count(buttons: &[String], title: &str) -> usize {
    buttons.iter().filter(|b| *b == title).count()
}


/// The save window after STOP: Esc and ⌘. do nothing (Discard has no key, so a stray key never
/// drops the recording); a long note pasted with line breaks is one line, saved whole; Return
/// saves.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_drill -- --ignored --test-threads 1"]
fn os_layer_in_the_save_window_esc_and_cmd_period_keep_it_and_a_long_pasted_note_is_saved_whole() {
    os_layer();
    let kept = clipboard();
    let (mut app, bugs) = os_launch(2000, "1: start-level W1-1 record\n80: menu stop-recording\n");
    until("the save window", || app.os_buttons().iter().any(|b| b == "Save"));
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    for (mods, key) in [(Mods::default(), Key::Escape), (Mods::COMMAND, Key::Char('.'))] {
        app.os_key_with(mods, key);
        std::thread::sleep(Duration::from_millis(800));
        let b = app.os_buttons();
        assert_eq!((count(&b, "Save"), count(&b, "Discard"), count(&b, "Save…")), (1, 1, 0), "{key:?}: {b:?}");
    }
    let words: String = (0..60).map(|k| format!("word{k:02} ")).collect();
    set_clipboard(&format!("{words}line\ntwo\nend"));
    app.os_focus_id("note");
    app.os_key_with(Mods::COMMAND, Key::Char('a'));
    app.os_key_with(Mods::COMMAND, Key::Char('v'));
    let want = format!("{words}line two end");
    until("the note pasted", || app.os_text_id("note") == want);
    app.os_key(Key::Return);
    until("the saved notice", || app.os_buttons().iter().any(|b| b == "OK"));
    app.os_press_button("OK");
    std::thread::sleep(Duration::from_millis(300));
    app.os_activate();
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    set_clipboard(&kept);
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{found:?}");
    let (rec, _) = read_zip(&found[0]);
    assert_eq!(rec.note, want, "the whole note, one line");
    assert_eq!(rec.inputs.len(), 79, "Esc, ⌘. and the note's keys added no frame");
}

/// "Recording in progress" (⌘Q while recording): Esc is Cancel, Return is Save…, which opens the
/// save window; its Discard then quits.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_drill -- --ignored --test-threads 1"]
fn os_layer_in_the_question_esc_cancels_and_return_is_save() {
    os_layer();
    let (mut app, bugs) = os_launch(20_000, "1: start-level W1-1 record\n");
    until("recording", || recording(&app));
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    until("the question", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_key(Key::Escape);
    until("the question gone", || !app.os_buttons().iter().any(|b| b == "Save…"));
    assert!(recording(&app), "Esc: still recording");
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    until("the question again", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_key(Key::Return);
    until("the save window", || app.os_buttons().iter().any(|b| b == "Save"));
    app.os_press_id("discard");
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    assert!(zips(&bugs).is_empty());
    let p = &c.prompts;
    assert!(p.contains("recording: Quit -> cancel\n") && p.contains("recording: Quit -> save\n"), "{p}");
}

/// ⌘. while paused: the save window; after Save the game is as the player left it, paused, and
/// not recording.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_drill -- --ignored --test-threads 1"]
fn os_layer_cmd_period_while_paused_saves_and_leaves_the_game_paused() {
    os_layer();
    let (mut app, bugs) = os_launch(20_000, "1: start-level W1-1 record\n");
    until("recording", || recording(&app));
    app.os_menu(&["Game", "Pause"]);
    until("paused", || app.os_menu_item(&["Game"], "Pause").checked);
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('.'));
    until("the save window", || app.os_buttons().iter().any(|b| b == "Save"));
    app.os_key(Key::Return);
    until("the saved notice", || app.os_buttons().iter().any(|b| b == "OK"));
    app.os_press_button("OK");
    until("not recording", || !recording(&app));
    assert!(app.os_menu_item(&["Game"], "Pause").checked, "still paused");
    app.os_activate();
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(zips(&bugs).len(), 1);
}

/// The Start Level window through its real controls, no script: half filled and closed with Esc,
/// it opens again on the defaults and Return starts them, recording; while recording, Start Level
/// asks first: Cancel keeps recording and opens no window, Discard opens it on the defaults, and a
/// level chosen there starts.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_drill -- --ignored --test-threads 1"]
fn os_layer_a_half_filled_start_level_window_reopens_on_the_defaults_and_a_new_start_asks_first() {
    os_layer();
    let mut app = App::launch(LaunchOpts { max_frames: 20_000, timeout_s: 180, scale: Some(1), ..Default::default() });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
    app.start_os();
    app.os_window();
    std::thread::sleep(Duration::from_millis(500));
    let dialog = |app: &App| {
        let b = app.os_buttons();
        b.iter().any(|t| t == "Start") && b.iter().any(|t| t == "Cancel")
    };
    app.os_menu(&["Game", "Start Level…"]);
    until("the window", || dialog(&app));
    app.os_choose_id("level", "W1-2");
    app.os_choose_id("lives", "7");
    app.os_press_id("wings");
    app.os_activate();
    app.os_key(Key::Escape);
    until("the window closed", || !dialog(&app));
    app.os_menu(&["Game", "Start Level…"]);
    until("the window again", || dialog(&app));
    assert_eq!((app.os_text_id("level"), app.os_text_id("lives")), ("W1-1".into(), "3".into()));
    assert!(!app.os_checked_id("wings") && app.os_checked_id("record"));
    app.os_activate();
    app.os_key(Key::Return);
    until("recording", || recording(&app));
    // While recording: asked first.
    app.os_menu(&["Game", "Start Level…"]);
    until("the question", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_press_button("Cancel");
    until("the question gone", || !app.os_buttons().iter().any(|b| b == "Save…"));
    std::thread::sleep(Duration::from_millis(500));
    assert!(!dialog(&app) && recording(&app), "Cancel: no window, still recording");
    app.os_menu(&["Game", "Start Level…"]);
    until("the question again", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_press_button("Discard");
    until("the window after Discard", || dialog(&app));
    assert_eq!(app.os_text_id("level"), "W1-1");
    app.os_choose_id("level", "W1-2");
    app.os_activate();
    app.os_key(Key::Return);
    until("the window closed", || !dialog(&app));
    until("recording W1-2", || recording(&app));
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    app.os_press_button("Discard");
    let c = app.finish_unread();
    assert_eq!(c.exit.code(), Some(0), "{}", c.stderr);
    let started: Vec<&str> = c.stderr.lines().filter(|l| l.starts_with("superfrog: started ")).collect();
    assert_eq!(started.len(), 2, "{started:?}");
    assert!(started[0].starts_with("superfrog: started W1-1 ") && started[1].starts_with("superfrog: started W1-2 "), "{started:?}");
    assert!(zips(&bugs).is_empty());
}

/// Every ending action through the real menu bar asks first and Cancel keeps recording; its
/// shortcut too (⌘2, Load State 2): Discard there loads; Reset's Save… opens the save window, and
/// after Save the game resets.
#[test]
#[ignore = "OS layer: SF_E2E_OS=1 cargo test -p sf-e2e --test recorder_drill -- --ignored --test-threads 1"]
fn os_layer_every_ending_action_and_its_shortcut_asks_with_each_answer() {
    os_layer();
    let (mut app, bugs) = os_launch(20_000, "1: start-level W1-1 record\n20: menu save-state 2\nform start\n");
    until("recording", || recording(&app));
    until("slot 2 filled", || app.os_menu_item(&["Game", "Load State"], "2").enabled);
    let game = app.os_menu_items(&["Game"]);
    let cont = game.iter().find(|i| i.title.starts_with("Continue")).expect("Continue").clone();
    let mut paths: Vec<Vec<String>> = vec![
        vec!["Game".into(), "Start Level…".into()],
        vec!["Game".into(), "Reset".into()],
        vec!["Game".into(), "Load State".into(), "2".into()],
        vec!["File".into(), "Import Disks…".into()],
        vec!["File".into(), "Play Level File…".into()],
        vec!["superfrog".into(), "Quit Superfrog".into()],
    ];
    if cont.enabled {
        paths.push(vec!["Game".into(), cont.title.clone()]);
    }
    for path in &paths {
        let p: Vec<&str> = path.iter().map(String::as_str).collect();
        app.os_menu(&p);
        until(&format!("{p:?}: the question"), || app.os_buttons().iter().any(|b| b == "Save…"));
        app.os_press_button("Cancel");
        until(&format!("{p:?}: the question gone"), || !app.os_buttons().iter().any(|b| b == "Save…"));
        assert!(recording(&app), "{p:?}: Cancel keeps recording");
    }
    app.os_activate();
    std::thread::sleep(Duration::from_millis(300));
    app.os_key_with(Mods::COMMAND, Key::Char('2'));
    until("⌘2: the question", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_press_button("Discard");
    until("⌘2 Discard: not recording", || !recording(&app));
    // Recording again, then Reset → Save… → Save.
    app.os_menu(&["Game", "Start Level…"]);
    std::thread::sleep(Duration::from_millis(1500));
    until("recording again", || recording(&app));
    app.os_menu(&["Game", "Reset"]);
    until("Reset: the question", || app.os_buttons().iter().any(|b| b == "Save…"));
    app.os_press_button("Save…");
    until("the save window", || app.os_buttons().iter().any(|b| b == "Save"));
    app.os_press_id("save");
    until("the saved notice", || app.os_buttons().iter().any(|b| b == "OK"));
    app.os_press_button("OK");
    until("not recording", || !recording(&app));
    app.os_activate();
    app.os_key_with(Mods::COMMAND, Key::Char('q'));
    let c = app.finish_unread();
    assert!(c.exit.success(), "{}", c.stderr);
    let cancels = c.prompts.lines().filter(|l| l.starts_with("recording: ") && l.ends_with("-> cancel")).count();
    assert_eq!(cancels, paths.len(), "{}", c.prompts);
    assert!(c.prompts.contains("recording: Load State 2 -> discard\n"), "{}", c.prompts);
    assert!(c.prompts.contains("recording: Reset -> save\n"), "{}", c.prompts);
    assert_eq!(c.stderr.matches("superfrog: loaded state 2").count(), 1, "{}", c.stderr);
    assert_eq!(zips(&bugs).len(), 1);
}
