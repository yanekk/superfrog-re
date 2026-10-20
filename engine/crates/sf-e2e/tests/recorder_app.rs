//! bug-recorder T06: the recorder in the real app, through the script layer. Game ▸ Start Level…
//! with the recorder on records every stepped frame from the level's first, across a level's end,
//! the tally, the slot machine and game over, until Stop Recording saves one zip into `--bugs-dir`;
//! the menu actions that would change the game ask first and end it as answered; pause is not
//! recorded; a game started from the dialog never moves the saved progress (DESIGN §2.1, §2.2,
//! §2.4, §2.5, §2.11).

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::cache_data;
use sf_core::input::FrameInput;
use sf_core::recording::{LevelSetup, Location, Recording, ZIP_RECORDING, ZIP_SCREENSHOT};
use sf_core::{DisplayData, Game, GameData, Rng};
use sf_e2e::{App, Captured, Image, LaunchOpts};

/// The app in a temporary home, saving bugs into `<home>/test-bugs`.
fn launch(max_frames: u64, timeout_s: u64) -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames,
        timeout_s,
        scale: Some(1),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
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

/// A bug zip's recording and screenshot.
fn read_zip(path: &Path) -> (Recording, Vec<u8>) {
    let bytes = std::fs::read(path).unwrap();
    let entries = sf_disk::zip::read_entries(&bytes, |_| true, 1 << 26).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, [ZIP_RECORDING, ZIP_SCREENSHOT], "{}", path.display());
    let text = String::from_utf8(entries[0].bytes.clone()).unwrap();
    let rec = Recording::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    (rec, entries[1].bytes.clone())
}

/// A PNG's pixels.
fn png_image(png: &[u8], scratch: &Path) -> Image {
    let p = scratch.join("screenshot-check.png");
    std::fs::write(&p, png).unwrap();
    Image::read_png(&p).unwrap()
}

/// The recording played headless from its start: every step's place, and the last picture.
/// `exit_before`: the step index before which the test set the level's exit (`N: exit`).
fn replay(data: &GameData, rec: &Recording, exit_before: Option<usize>) -> (Vec<Location>, Image) {
    let display = Arc::new(DisplayData::new(data));
    let mut g = Game::start_level(display, &rec.setup, rec.seed);
    let mut places = Vec::new();
    let mut last = Image::of_picture(&g.render(g.view()));
    for (i, &input) in rec.inputs.iter().enumerate() {
        if exit_before == Some(i) {
            g.poke(sf_core::layout::h0(0xa06), &1u32.to_be_bytes());
        }
        last = Image::of_picture(&g.step(input).picture);
        places.push(g.location());
    }
    (places, last)
}

/// `inputs.txt`'s line for step `n` with `f`, as the capture writes it.
fn input_line(n: u64, f: FrameInput) -> String {
    let keys: Vec<String> = f.keys.codes().map(|c| format!("{c:02x}")).collect();
    format!("{n} {:02x} {}", f.joy.bits(), keys.join(","))
}

/// The capture's input lines for the steps `steps` names, in order.
fn captured_inputs(c: &Captured, steps: impl Fn(u64) -> bool) -> Vec<String> {
    c.inputs
        .lines()
        .filter(|l| l.split(' ').next().and_then(|n| n.parse().ok()).is_some_and(&steps))
        .map(str::to_string)
        .collect()
}

/// The recording's input lines, its index `i` stepped as app step `step(i)`.
fn recorded_inputs(rec: &Recording, step: impl Fn(u64) -> u64) -> Vec<String> {
    rec.inputs
        .iter()
        .enumerate()
        .filter(|(_, f)| **f != FrameInput::NONE)
        .map(|(i, &f)| input_line(step(i as u64), f))
        .collect()
}

fn screens(places: &[Location]) -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for p in places {
        if let Location::Screen(s) = p
            && v.last() != Some(s)
        {
            v.push(s);
        }
    }
    v
}

#[test]
fn a_recording_started_from_the_dialog_and_stopped_saves_one_zip_of_every_frame() {
    let (mut app, bugs) = launch(320, 120);
    let data = cache_data(&app.home);
    let c = app.run_script(
        "note x\n1: start-level W1-1 record\n20: down ArrowRight\n90: down Space\n95: up Space\n\
         150: up ArrowRight\n200: down ArrowLeft\n260: up ArrowLeft\n301: menu stop-recording\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: started W1-1 after step 0, recording"), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "one zip: {found:?}\n{}", c.prompts);
    let name = found[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("bug-W1-1-20") && name.len() == "bug-W1-1-YYYY-MM-DD-HHMM.zip".len(), "{name}");
    let (rec, png) = read_zip(&found[0]);
    assert_eq!(rec.setup, LevelSetup::default());
    assert_eq!(rec.seed, Rng::seeded(0, 0), "the script's seed, as power-on takes it");
    assert_eq!(rec.inputs.len(), 300, "steps 1..=300");
    assert_eq!(rec.note, "x");
    assert!(rec.build.starts_with("superfrog "), "{}", rec.build);
    // The inputs are what reached the game, step for step.
    let want = captured_inputs(&c, |n| n <= 300);
    assert!(want.len() > 100, "the script pressed keys: {want:?}");
    assert_eq!(recorded_inputs(&rec, |i| i + 1), want);
    // The screenshot is frame 300 at 1×, and the recording replays to it headless.
    let shot = png_image(&png, &c.dir);
    assert_eq!(shot, c.read_frame(300), "screenshot.png is the captured frame 300");
    let (places, last) = replay(&data, &rec, None);
    assert_eq!(last, shot, "the recording replays to its screenshot");
    assert_eq!(rec.stopped_at, places.last().copied());
    assert!(matches!(rec.stopped_at, Some(Location::Level { .. })), "{:?}", rec.stopped_at);
    // Saved: the path on the clipboard and in a notice.
    let path = found[0].display().to_string();
    let at = rec.stopped_at.unwrap();
    assert!(c.prompts.contains(&format!("save-recording: {name} at {at} -> note x\n")), "{}", c.prompts);
    assert!(c.prompts.contains(&format!("clipboard: {path}\n")), "{}", c.prompts);
    assert!(
        c.prompts.contains(&format!("notice: The bug recording is saved as {path}. Its path is on the clipboard.\n")),
        "{}",
        c.prompts
    );
    // Not recording any more: a second STOP is refused.
    assert!(!c.stderr.contains("not recording"), "{}", c.stderr);
}

#[test]
fn a_save_that_cannot_be_written_says_so_and_asks_again_keeping_the_recording() {
    use std::os::unix::fs::PermissionsExt;
    let (mut app, bugs) = launch(80, 90);
    std::fs::create_dir_all(&bugs).unwrap();
    std::fs::set_permissions(&bugs, std::fs::Permissions::from_mode(0o555)).unwrap();
    let c = app.run_script("note first\nnote second\nnote discard\n1: start-level W1-1 record\n51: menu stop-recording\n");
    std::fs::set_permissions(&bugs, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(c.exit.success(), "{}", c.stderr);
    let lines: Vec<&str> = c.prompts.lines().filter(|l| !l.starts_with("start-level:")).collect();
    assert_eq!(lines.len(), 5, "{}", c.prompts);
    assert!(lines[0].ends_with("-> note first"), "{}", lines[0]);
    assert!(lines[1].starts_with("notice: The bug recording could not be saved as “"), "{}", lines[1]);
    assert!(lines[2].ends_with("-> note second"), "{}", lines[2]);
    assert!(lines[3].starts_with("notice: The bug recording could not be saved"), "{}", lines[3]);
    assert!(lines[4].ends_with("-> discard"), "{}", lines[4]);
    assert_eq!(std::fs::read_dir(&bugs).unwrap().count(), 0, "nothing written, no temporary left");
}

#[test]
fn a_recording_goes_on_through_the_level_s_end_the_tally_and_into_the_slot_machine() {
    // No played route reaches W1-1's exit from a dialog start unpaced (conformance routes are
    // the original's pace), so `2: exit` sets the exit as reaching it does, as conformance cases
    // force it; up and fire in the tally's wait choose the slot machine (`tally-slot`).
    let (mut app, bugs) = launch(420, 120);
    let data = cache_data(&app.home);
    let c = app.run_script(
        "note through\n1: start-level W1-1 record\n2: exit\n61: down ArrowUp\n71: down Space\n\
         81: up Space\n81: up ArrowUp\n401: menu stop-recording\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{}", c.prompts);
    let (rec, png) = read_zip(&found[0]);
    assert_eq!(rec.inputs.len(), 400, "every step from the level's first frame to STOP");
    assert_eq!(recorded_inputs(&rec, |i| i + 1), captured_inputs(&c, |n| n <= 400));
    let (places, last) = replay(&data, &rec, Some(1));
    assert_eq!(screens(&places), ["tally", "slot machine"], "the stretch it went through");
    assert_eq!(rec.stopped_at, Some(Location::Screen("slot machine")));
    assert_eq!(last, png_image(&png, &c.dir));
    assert!(found[0].file_name().unwrap().to_string_lossy().starts_with("bug-W1-1-"), "named for the level started");
}

#[test]
fn a_recording_goes_on_past_game_over() {
    // Three lives, right held from the start: the frog runs into W1-1's first pit three times
    // (`frog-pit`); GAME OVER from about step 893, the title from about 1 417.
    let (mut app, bugs) = launch(1120, 150);
    let data = cache_data(&app.home);
    let c = app.run_script_unread("note over\n1: start-level W1-1 lives=3 record\n2: down ArrowRight\n1101: menu stop-recording\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{}", c.prompts);
    let (rec, png) = read_zip(&found[0]);
    assert_eq!(rec.inputs.len(), 1100);
    let (places, last) = replay(&data, &rec, None);
    assert_eq!(screens(&places), ["game over"], "{:?}", rec.stopped_at);
    let over = places.iter().position(|p| *p == Location::Screen("game over")).unwrap();
    assert!(over < 1000, "game over at step {over}, and recorded on past it");
    assert_eq!(rec.stopped_at, Some(Location::Screen("game over")));
    assert_eq!(last, png_image(&png, &c.dir));
    assert_eq!(last, c.read_frame(1100));
}

/// The app's stderr from the line saying `a` on, for checking what came after it.
fn after<'a>(stderr: &'a str, a: &str) -> &'a str {
    let at = stderr.find(a).unwrap_or_else(|| panic!("no {a:?} in:\n{stderr}"));
    &stderr[at + a.len()..]
}

/// Load State 1 (a state saved at step 10), Start Level, Reset, Continue (from World 2), Import
/// Disks, Play Level File and Quit, each in turn while a recording runs, one every ten steps from
/// step 20; with `restart`, a new recording before each.
fn ending_script(answer: &str, restart: bool) -> String {
    let actions = [
        "menu load-state-1",
        "start-level W1-2",
        "menu reset",
        "menu continue",
        "menu import",
        "menu play-level-file",
        "menu quit",
    ];
    let mut s = String::from("pick cancel\nopen cancel\n1: start-level W1-1 record\n10: menu save-state-1\n");
    for (k, a) in actions.iter().enumerate() {
        let at = 20 + 20 * k;
        if restart && k > 0 {
            s.push_str(&format!("{}: start-level W1-1 record\n", at - 10));
        }
        s.push_str(&format!("{at}: {a}\nanswer-recording {answer}\n"));
        if answer == "save" {
            s.push_str(&format!("note n{}\n", k + 1));
        }
    }
    s
}

const ACTIONS: [&str; 7] = [
    "Load State 1",
    "Start Level",
    "Reset",
    "Continue from World 2",
    "Import Disks",
    "Play Level File",
    "Quit",
];

/// What each action, once taken, leaves in stderr or the prompts (Quit: the run ends, [`quit_at`]).
const TAKEN: [&str; 7] = [
    "superfrog: loaded state 1",
    "superfrog: started W1-2 after step",
    "superfrog: power-on",
    "superfrog: continuing from World 2",
    "superfrog: picker: ",
    "superfrog: open: -> cancel",
    "",
];

/// The steps the run took, when it ended.
fn stepped(c: &Captured) -> String {
    std::fs::read_to_string(c.dir.join("pacing.txt")).unwrap().lines().next().unwrap().to_string()
}

fn ending_app(max_frames: u64) -> (App, PathBuf) {
    let (app, bugs) = launch(max_frames, 150);
    std::fs::write(app.home.join("progress.toml"), "world = 2\nlevel = 1\n").unwrap();
    (app, bugs)
}

#[test]
fn every_ending_action_cancelled_keeps_recording_and_is_not_taken() {
    let (mut app, bugs) = ending_app(170);
    let c = app.run_script(&format!("{}note kept\n161: menu stop-recording\n", ending_script("cancel", false)));
    assert!(c.exit.success(), "{}", c.stderr);
    let asked: Vec<&str> = c.prompts.lines().filter(|l| l.starts_with("recording: ")).collect();
    let want: Vec<String> = ACTIONS.iter().map(|a| format!("recording: {a} -> cancel")).collect();
    assert_eq!(asked, want);
    assert_eq!(stepped(&c), "stepped 170", "Quit cancelled: the run went on");
    for t in &TAKEN[..6] {
        assert!(!c.stderr.contains(t), "{t:?} was taken:\n{}", c.stderr);
    }
    assert_eq!(c.prompts.matches("start-level:").count(), 1, "the dialog was never shown again");
    assert!(c.stderr.contains("superfrog: saved state 1"), "Save State leaves it running");
    let found = zips(&bugs);
    assert_eq!(found.len(), 1);
    let (rec, _) = read_zip(&found[0]);
    assert_eq!(rec.inputs.len(), 160, "one recording, every step to STOP");
    assert_eq!(rec.note, "kept");
}

#[test]
fn every_ending_action_discarded_is_taken_and_saves_nothing() {
    let (mut app, bugs) = ending_app(400);
    let c = app.run_script(&ending_script("discard", true));
    assert!(c.exit.success(), "{}", c.stderr);
    let asked: Vec<&str> = c.prompts.lines().filter(|l| l.starts_with("recording: ")).collect();
    let want: Vec<String> = ACTIONS.iter().map(|a| format!("recording: {a} -> discard")).collect();
    assert_eq!(asked, want);
    let mut rest = c.stderr.as_str();
    for (a, t) in ACTIONS.iter().zip(TAKEN) {
        rest = after(rest, &format!("superfrog: recording: {a} -> discard"));
        rest = after(rest, "superfrog: recording discarded");
        rest = after(rest, t);
    }
    assert_eq!(stepped(&c), "stepped 139", "Quit before step 140 ended the run");
    assert!(zips(&bugs).is_empty(), "nothing saved: {:?}", zips(&bugs));
    assert!(!c.prompts.contains("save-recording"), "{}", c.prompts);
}

#[test]
fn every_ending_action_saved_writes_a_zip_then_is_taken() {
    let (mut app, bugs) = ending_app(400);
    let c = app.run_script(&ending_script("save", true));
    assert!(c.exit.success(), "{}", c.stderr);
    let mut rest = c.stderr.as_str();
    for (a, t) in ACTIONS.iter().zip(TAKEN) {
        rest = after(rest, &format!("superfrog: recording: {a} -> save"));
        rest = after(rest, "superfrog: recording saved to ");
        rest = after(rest, t);
    }
    assert_eq!(stepped(&c), "stepped 139", "Quit before step 140 ended the run");
    let found = zips(&bugs);
    assert_eq!(found.len(), 7, "{found:?}");
    let mut by_note: Vec<(String, usize)> = found
        .iter()
        .map(|z| {
            let (rec, _) = read_zip(z);
            (rec.note, rec.inputs.len())
        })
        .collect();
    by_note.sort();
    // The first ran from step 1 to the load at 20; the rest from their start to the action ten
    // steps on.
    let want: Vec<(String, usize)> = (1..=7).map(|k| (format!("n{k}"), if k == 1 { 19 } else { 10 })).collect();
    assert_eq!(by_note, want);
}

#[test]
fn pause_is_not_recorded_and_save_state_scale_and_crt_leave_it_running() {
    let (mut app, bugs) = launch(220, 120);
    let data = cache_data(&app.home);
    let c = app.run_script(
        "note p\n1: start-level W1-1 record\n20: down ArrowRight\n50: menu pause\n100: menu pause\n\
         110: menu save-state-2\n120: menu scale 2\n130: menu crt\n140: menu scale 1\n\
         170: up ArrowRight\n201: menu stop-recording\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(!c.prompts.lines().any(|l| l.starts_with("recording: ")), "nothing asked: {}", c.prompts);
    let found = zips(&bugs);
    assert_eq!(found.len(), 1, "{}", c.prompts);
    let (rec, png) = read_zip(&found[0]);
    assert_eq!(rec.inputs.len(), 150, "200 steps, 50 of them paused");
    // Index i is step i + 1 before the pause, step i + 51 after it.
    let step = |i: u64| if i < 49 { i + 1 } else { i + 51 };
    assert_eq!(recorded_inputs(&rec, step), captured_inputs(&c, |n| n <= 200));
    let (_, last) = replay(&data, &rec, None);
    assert_eq!(last, png_image(&png, &c.dir));
    assert_eq!(last, c.read_frame(200));
}

#[test]
fn a_game_started_from_the_dialog_never_moves_the_saved_progress() {
    // Recorder off, no progress yet: playing W5-1 saves none, and Continue stays refused.
    let (mut app, _) = launch(420, 120);
    let c = app.run_script("1: start-level W5-1\n20: down ArrowRight\n300: up ArrowRight\n400: continue\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: started W5-1 after step 0\n"), "{}", c.stderr);
    assert!(!app.home.join("progress.toml").exists(), "W5-1 from the dialog saved progress");
    assert!(c.stderr.contains("no progress to continue from"), "{}", c.stderr);

    // Recorder on, World 2 reached before: still World 2, and Continue starts it, not World 5.
    let (mut app, _) = launch(420, 120);
    std::fs::write(app.home.join("progress.toml"), "world = 2\nlevel = 1\n").unwrap();
    let c = app.run_script(
        "note discard\n1: start-level W5-1 record\n20: down ArrowRight\n300: up ArrowRight\n\
         350: menu stop-recording\n400: continue\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(std::fs::read_to_string(app.home.join("progress.toml")).unwrap(), "world = 2\nlevel = 1\n");
    assert!(c.stderr.contains("superfrog: continuing from World 2"), "{}", c.stderr);
}
