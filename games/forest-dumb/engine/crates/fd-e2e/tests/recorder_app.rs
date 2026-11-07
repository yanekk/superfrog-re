//! T14: the recorder in the real app, through the script layer (DESIGN §2.7, §2.8). Game ▸ Start
//! Level… restarts W1-1 with "Enable recorder?" on; every stepped frame is recorded and the bar
//! shows until Stop Recording saves one zip into `--bugs-dir`, which replays headless to the
//! screenshot. With the recorder off there is no bar and no zip. Start Level and Quit during a
//! recording ask Save, Discard or Cancel.

mod common;

use std::path::{Path, PathBuf};

use amiga_input::FrameInput;
use fd_core::recording::{LevelId, Recording, ZIP_RECORDING, ZIP_SCREENSHOT};
use fd_e2e::{App, Captured, Image, LaunchOpts};

/// The app in a temporary home at 1×, saving bugs into `<home>/test-bugs`.
fn launch(max_frames: u64) -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s: 120,
        scale: Some(1),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    (app, bugs)
}

/// The zips in `dir`, by name.
fn zips(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    v.retain(|p| p.extension().is_some_and(|x| x == "zip"));
    v.sort();
    v
}

/// A saved zip's recording and screenshot, its entries in the app's order, read as the replay reads
/// it (`fd_conform::bug::open_zip`, refusing another build).
fn saved_zip(path: &Path) -> (Recording, Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let entries = amiga_disk::zip::read_entries(&bytes, |_| true, 1 << 26).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [ZIP_RECORDING, ZIP_SCREENSHOT], "{}", path.display());
    fd_conform::bug::open_zip(&bytes, fd_tools::ENGINE_BUILD)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn png_image(png: &[u8], scratch: &Path) -> Image {
    let p = scratch.join("screenshot-check.png");
    std::fs::write(&p, png).unwrap();
    Image::read_png(&p).unwrap()
}

/// The capture's input lines for steps `from..=to`, step `n` written as `n - shift`.
fn captured_inputs(c: &Captured, from: u64, to: u64, shift: u64) -> Vec<(u64, FrameInput)> {
    common::inputs(c)
        .into_iter()
        .filter(|(n, _)| (from..=to).contains(n))
        .map(|(n, f)| (n - shift, f))
        .collect()
}

/// The recording's inputs that are not empty, by index.
fn recorded_inputs(rec: &Recording) -> Vec<(u64, FrameInput)> {
    rec.inputs
        .iter()
        .enumerate()
        .filter(|(_, f)| **f != FrameInput::NONE)
        .map(|(i, &f)| (i as u64, f))
        .collect()
}

/// Everything in `stderr` after the first line holding `a`.
fn after<'a>(stderr: &'a str, a: &str) -> &'a str {
    let at = stderr
        .find(a)
        .unwrap_or_else(|| panic!("no {a:?} in:\n{stderr}"));
    &stderr[at..]
}

#[test]
fn a_recording_started_from_the_window_and_stopped_saves_one_zip_that_replays_to_its_screenshot() {
    let (mut app, bugs) = launch(170);
    let c = app.run_script(
        "note slid into the wall\n20: down ArrowRight\n41: menu start-level\nform level W1-1\n\
         form start\n60: down Space\n70: up Space\n150: up ArrowRight\n161: menu stop-recording\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    // The window offered level 1 with the recorder on, and Start restarted the level.
    assert!(
        c.prompts.contains("start-level: -> W1-1 record"),
        "{}",
        c.prompts
    );
    assert!(
        c.stderr
            .contains("fd-app: started W1-1 after step 40, recording"),
        "{}",
        c.stderr
    );
    let mut g = common::headless();
    g.step(
        common::inputs(&c)
            .get(&41)
            .copied()
            .unwrap_or(FrameInput::NONE),
    );
    assert_eq!(
        common::hashes(&c)[41],
        common::hash(&g),
        "step 41 is the restarted level's first step"
    );
    // The bar showed from the start until Stop.
    assert!(
        after(&c.stderr, "started W1-1").contains("fd-app: recording bar shown after step 40"),
        "{}",
        c.stderr
    );
    assert!(
        c.stderr
            .contains("fd-app: recording bar hidden after step 160"),
        "{}",
        c.stderr
    );
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "one zip: {found:?}\n{}", c.prompts);
    let name = found[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("bug-W1-1-20") && name.len() == "bug-W1-1-YYYY-MM-DD-HHMM.zip".len(),
        "{name}"
    );
    assert!(
        c.prompts
            .contains(&format!("save-recording: {name} at W1-1:")),
        "{}",
        c.prompts
    );
    let (rec, png) = saved_zip(&found[0]);
    assert_eq!(rec.level, LevelId::W1_1);
    assert_eq!(rec.inputs.len(), 120, "steps 41..=160");
    assert_eq!(rec.note, "slid into the wall");
    assert_eq!(rec.build, fd_tools::ENGINE_BUILD);
    // The inputs are what reached the game, step for step: held right carried into the recording.
    let want = captured_inputs(&c, 41, 160, 41);
    assert!(want.len() > 100, "the script pressed keys: {want:?}");
    assert_eq!(recorded_inputs(&rec), want);
    // The screenshot is frame 160 at 1×, and the recording replays headless to it and its place.
    let shot = png_image(&png, &c.dir);
    assert_eq!(
        shot,
        c.read_frame(160),
        "screenshot.png is the captured frame 160"
    );
    let r = fd_conform::bug::replay(common::data(), &rec, |_, _, _| {});
    assert_eq!(
        fd_conform::bug::screenshot_differs(&png, &r.last).unwrap(),
        0,
        "the replay's last picture is the screenshot"
    );
    assert_eq!(rec.stopped_at, Some(r.location));
    assert_ne!(r.location.to_string(), "W1-1:7,7", "the character moved");
    assert!(
        c.prompts.contains("notice: The bug recording is saved as "),
        "{}",
        c.prompts
    );
}

#[test]
fn with_the_recorder_off_the_level_restarts_with_no_bar_and_no_zip() {
    let (mut app, bugs) = launch(60);
    let c = app.run_script(
        "note never\n10: down ArrowRight\n21: menu start-level\nform record off\nform start\n\
         51: menu stop-recording\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.prompts.contains("start-level: -> W1-1\n"),
        "{}",
        c.prompts
    );
    assert!(
        c.stderr.contains("fd-app: started W1-1 after step 20\n"),
        "{}",
        c.stderr
    );
    assert!(!c.stderr.contains("recording bar"), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: not recording"), "{}", c.stderr);
    assert!(zips(&bugs).is_empty());
    assert!(!c.prompts.contains("save-recording"), "{}", c.prompts);
    // The level restarted all the same: step 21 is its first step.
    let mut g = common::headless();
    g.step(
        common::inputs(&c)
            .get(&21)
            .copied()
            .unwrap_or(FrameInput::NONE),
    );
    assert_eq!(common::hashes(&c)[21], common::hash(&g));
}

#[test]
fn cancelling_the_start_level_window_changes_nothing() {
    let (mut app, _) = launch(30);
    let c = app.run_script("10: menu start-level\nform cancel\n20: menu start-level\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        c.prompts.matches("start-level: -> cancel").count(),
        2,
        "the second had no answer left: {}",
        c.prompts
    );
    assert!(!c.stderr.contains("fd-app: started"), "{}", c.stderr);
    let mut g = common::headless();
    for _ in 0..30 {
        g.step(FrameInput::NONE);
    }
    assert_eq!(common::hashes(&c)[30], common::hash(&g), "play went on");
}

#[test]
fn an_empty_recording_with_an_empty_note_saves_and_replays_to_the_level_s_first_frame() {
    let (mut app, bugs) = launch(20);
    let c = app.run_script("note\n10: menu start-level\nform start\n10: menu stop-recording\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{}", c.prompts);
    let (rec, png) = saved_zip(&found[0]);
    assert_eq!((rec.inputs.len(), rec.note.as_str()), (0, ""));
    assert_eq!(rec.stopped_at.unwrap().to_string(), "W1-1:7,7");
    assert_eq!(
        png_image(&png, &c.dir),
        common::picture(&common::headless()),
        "the level's first frame"
    );
    let r = fd_conform::bug::replay(common::data(), &rec, |_, _, _| {});
    assert_eq!(
        fd_conform::bug::screenshot_differs(&png, &r.last).unwrap(),
        0
    );
}

#[test]
fn a_long_note_is_kept_whole_and_discarding_the_save_window_saves_nothing() {
    let (mut app, bugs) = launch(80);
    let long = "a very long note ".repeat(120);
    let long = long.trim();
    let c = app.run_script(&format!(
        "note {long}\nnote discard\n1: menu start-level\nform start\n31: menu stop-recording\n\
         41: menu start-level\nform start\n71: menu stop-recording\n"
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "the second was discarded: {}", c.prompts);
    let (rec, _) = saved_zip(&found[0]);
    assert_eq!(rec.note, long);
    assert_eq!(rec.inputs.len(), 30);
    assert!(
        after(&c.stderr, "menu stop-recording after step 70")
            .contains("fd-app: recording discarded"),
        "{}",
        c.stderr
    );
    assert!(
        c.stderr
            .contains("fd-app: recording bar hidden after step 70"),
        "{}",
        c.stderr
    );
}

#[test]
fn quit_and_esc_during_a_recording_ask_and_cancel_keeps_recording() {
    let (mut app, bugs) = launch(100);
    let c = app.run_script(
        "1: menu start-level\nform start\nanswer-recording cancel\nanswer-recording cancel\n\
         answer-recording discard\n21: menu quit\n31: press Escape\n41: menu quit\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        c.prompts.matches("recording: Quit -> cancel").count(),
        2,
        "{}",
        c.prompts
    );
    assert!(
        c.prompts.contains("recording: Quit -> discard"),
        "{}",
        c.prompts
    );
    assert!(
        c.stderr.contains("fd-app: Quit cancelled; still recording"),
        "{}",
        c.stderr
    );
    assert!(
        c.stderr.contains("fd-app: quit after step 40"),
        "{}",
        c.stderr
    );
    assert!(zips(&bugs).is_empty());
}

#[test]
fn start_level_during_a_recording_saved_writes_the_zip_then_restarts() {
    let (mut app, bugs) = launch(80);
    let c = app.run_script(
        "note first\n1: menu start-level\nform start\nanswer-recording save\nform record off\n\
         form start\n31: menu start-level\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.prompts.contains("recording: Start Level -> save"),
        "{}",
        c.prompts
    );
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{}", c.prompts);
    let (rec, _) = saved_zip(&found[0]);
    assert_eq!((rec.inputs.len(), rec.note.as_str()), (30, "first"));
    assert!(
        after(&c.stderr, "recording saved to").contains("fd-app: started W1-1 after step 30\n"),
        "{}",
        c.stderr
    );
    assert!(
        c.stderr
            .contains("fd-app: recording bar hidden after step 30"),
        "{}",
        c.stderr
    );
}

#[test]
fn start_level_during_a_recording_discarded_or_cancelled() {
    let (mut app, bugs) = launch(80);
    let c = app.run_script(
        "1: menu start-level\nform start\nanswer-recording cancel\nanswer-recording discard\n\
         form start\n21: menu start-level\n41: menu start-level\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.prompts.contains("recording: Start Level -> cancel"),
        "{}",
        c.prompts
    );
    assert!(
        c.prompts.contains("recording: Start Level -> discard"),
        "{}",
        c.prompts
    );
    assert!(
        c.stderr
            .contains("fd-app: Start Level cancelled; still recording"),
        "{}",
        c.stderr
    );
    // Cancel took no action, so the window opened only for the start and after Discard.
    assert_eq!(
        c.prompts.matches("start-level: ->").count(),
        2,
        "{}",
        c.prompts
    );
    assert!(
        c.stderr
            .contains("fd-app: started W1-1 after step 40, recording"),
        "{}",
        c.stderr
    );
    assert!(zips(&bugs).is_empty());
}
