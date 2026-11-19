//! Translator T39 (DESIGN §2.4, §2.10): what the app does on a stop of the translated program. The
//! picture freezes, a band over its foot names the stop, a bug zip of the play from its start is
//! saved to the bugs folder at once, and Esc or ⌘Q (`menu quit`, the command ⌘Q sends) quit. The
//! stop is `--stop-after N`'s test stop; the zip is then replayed by the real `fd-conform bug
//! replay` on the translated program, to the same frame, the same stop and the same screenshot.
//! Start Level…'s recorder on the translated program is here too: its zip replays the same way.
//!
//! Full: needs the owner's disks and the generated tree (`translate.py --build`), and fails naming
//! the command without it. Sizes: the smallest window, the picture at 1× (`--scale 1`).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use fd_core::recording::{EngineKind, ForestDumb, Recording, StartPoint};
use fd_e2e::{App, Captured, Image, LaunchOpts};
use fd_translated::TranslatedGame;

/// The translated program's picture, as the app shows it (`fd_app::CANVAS`).
const W: u32 = fd_app::CANVAS.2 as u32;
const H: u32 = fd_app::CANVAS.3 as u32;

/// Steps past the stop the run goes on for: 2 s at 50 a second, the time a zip must appear in.
const TWO_SECONDS: u64 = 100;

fn require_tree() {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
}

/// The translated program at the smallest window, its zips in its home's `test-bugs`, `args` added.
fn launch(max_frames: u64, args: &[&str]) -> (App, PathBuf) {
    let mut app = App::launch_translated(LaunchOpts {
        max_frames,
        timeout_s: 120,
        scale: Some(1),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    app.opts.extra_args.extend(args.iter().map(|a| a.to_string()));
    (app, bugs)
}

/// The one zip in `bugs`, read: its path, recording and screenshot as an image.
fn the_zip(bugs: &Path) -> (PathBuf, Recording, Image) {
    let zips: Vec<PathBuf> = std::fs::read_dir(bugs)
        .unwrap_or_else(|e| panic!("{}: {e}", bugs.display()))
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(zips.len(), 1, "{zips:?}");
    let (rec, png) =
        amiga_record::read_zip::<ForestDumb>(&std::fs::read(&zips[0]).unwrap()).unwrap();
    let (width, height, rgb) = amiga_conform::screenshot::decode_png(&png).unwrap();
    assert_eq!(rgb.len(), (width * height * 3) as usize, "the screenshot is RGB");
    let rgba = rgb.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    (zips[0].clone(), rec, Image { width, height, rgba })
}

/// `fd-conform bug replay <zip>`'s output, which must succeed.
fn replay(zip: &Path) -> String {
    let out = Command::new(fd_e2e::conform_binary())
        .args(["bug", "replay"])
        .arg(zip)
        .output()
        .expect("running fd-conform");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
}

/// A replay's `key` line's value.
fn line<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix(' ')))
        .unwrap_or_else(|| panic!("no {key}: {stdout}"))
}

/// The rows where `a` and `b` differ: the first and last, and the rightmost column.
fn differ(a: &Image, b: &Image) -> Option<(u32, u32, u32)> {
    assert_eq!((a.width, a.height), (b.width, b.height));
    let mut out: Option<(u32, u32, u32)> = None;
    for y in 0..a.height {
        for x in 0..a.width {
            let i = ((y * a.width + x) * 4) as usize;
            if a.rgba[i..i + 3] != b.rgba[i..i + 3] {
                let (top, bottom, right) = out.unwrap_or((y, y, x));
                out = Some((top.min(y), bottom.max(y), right.max(x)));
            }
        }
    }
    out
}

/// The rows of the band's text: runs of rows where `shown` has white pixels `shot` lacks, each
/// as (first, last).
fn text_lines(shown: &Image, shot: &Image) -> Vec<(u32, u32)> {
    let text = |y: u32| {
        (0..shown.width).any(|x| {
            let i = ((y * shown.width + x) * 4) as usize;
            shown.rgba[i..i + 3] == [255, 255, 255] && shot.rgba[i..i + 3] != [255, 255, 255]
        })
    };
    let mut out: Vec<(u32, u32)> = Vec::new();
    for y in 0..shown.height {
        if text(y) {
            match out.last_mut() {
                Some((_, last)) if *last + 1 == y => *last = y,
                _ => out.push((y, y)),
            }
        }
    }
    out
}

/// Fire, the default keys' Left Option, pressed at the disk 2 prompt: the title follows.
const PAST_THE_PROMPT: &str = "20: down AltLeft\n30: up AltLeft\n";

#[test]
fn a_stop_freezes_the_picture_names_it_saves_a_zip_and_the_zip_replays_to_the_same_stop() {
    require_tree();
    let stop = 120;
    let end = stop + TWO_SECONDS;
    let (mut app, bugs) = launch(end, &["--stop-after", "120"]);
    let c = app.run_script(&format!("{PAST_THE_PROMPT}{}: shot\n", stop + 10));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains(
            "fd-app: the translated program stopped at step 120: test-stop: --stop-after 120\n"
        ),
        "{}",
        c.stderr
    );
    assert!(c.stderr.contains("(120 frames from power-on)"), "{}", c.stderr);
    // The run went on to `end`, 2 s past the stop, and the zip was there by then: written in the
    // stop's own step (the line above comes before the next step's).
    assert!(c.stderr.contains(&format!("quit after step {end}")), "{}", c.stderr);

    // Frozen: every frame from the stop on is the stop's, band and all.
    let at_stop = &c.frames[stop as usize];
    for n in stop + 1..=end {
        assert!(&c.frames[n as usize] == at_stop, "frame {n} moved after the stop");
    }
    assert!(c.frames[stop as usize - 1] != *at_stop, "the band is drawn at the stop");

    let (zip, rec, shot) = the_zip(&bugs);
    let name = zip.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("bug-power-on-") && name.ends_with(".zip"), "{name}");
    assert_eq!(
        (rec.start, rec.engine, rec.inputs.len()),
        (StartPoint::PowerOn, EngineKind::Translated, stop as usize)
    );
    assert_eq!(rec.stop.as_deref(), Some("test-stop: --stop-after 120"));
    assert_eq!(rec.stopped_at, None, "the prompt is in no level");
    assert!(rec.inputs[19].joy.contains(amiga_input::Joy::FIRE), "fire at the prompt is recorded");

    // The band: what the frame shown has and the screenshot does not, at the picture's foot,
    // inside it, with text on each of its three lines (stop, saved, how to quit).
    let (top, bottom, right) = differ(at_stop, &shot).expect("the band");
    assert!(top >= H / 2 && bottom < H && right < W - 4, "{top}..{bottom}, {right}");
    let groups = text_lines(at_stop, &shot);
    assert!(groups.len() >= 3, "{groups:?}");
    assert!(groups.iter().all(|&(a, b)| b - a <= 7), "a line is one glyph tall: {groups:?}");
    // The window at its smallest shows the whole picture, band included: nothing is cut off.
    let (shown_at, window) = common::shown(&c, stop + 10, W, H);
    assert!(shown_at >= stop);
    assert!(window == *at_stop, "the window shows the frozen frame and its band whole");

    // The zip replays on the translated program to the same frame, stop and picture.
    let out = replay(&zip);
    assert_eq!(line(&out, "frames"), "120", "{out}");
    assert_eq!(line(&out, "start"), "power-on", "{out}");
    assert_eq!(line(&out, "engine"), "translated", "{out}");
    assert_eq!(line(&out, "stop"), "test-stop: --stop-after 120", "{out}");
    assert_eq!(
        line(&out, "replay-stop"),
        "test-stop: --stop-after 120 at frame 120",
        "{out}"
    );
    assert_eq!(line(&out, "replay-at"), "-", "{out}");
    assert!(out.contains("screenshot equal\n"), "{out}");
}

#[test]
fn a_stop_in_level_1_saves_a_level_1_zip_that_replays_through_the_title_to_its_place() {
    require_tree();
    let (mut app, bugs) = launch(90, &["--level", "1", "--stop-after", "60"]);
    let c = app.run_script_unread("5: down ArrowRight\n40: up ArrowRight\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("(60 frames from level-1)"), "{}", c.stderr);
    let (zip, rec, _) = the_zip(&bugs);
    assert!(zip.file_name().unwrap().to_string_lossy().starts_with("bug-W1-1-"));
    assert_eq!((rec.start, rec.engine), (StartPoint::Level1, EngineKind::Translated));
    let at = rec.stopped_at.expect("in level 1").to_string();
    assert_ne!(at, "W1-1:7,7", "the character moved");
    let out = replay(&zip);
    assert_eq!(line(&out, "frames"), "60", "{out}");
    assert_eq!(line(&out, "replay-at"), at, "{out}");
    assert_eq!(line(&out, "replay-stop"), "test-stop: --stop-after 60 at frame 60", "{out}");
    assert!(out.contains("screenshot equal\n"), "{out}");
}

/// A run stopped at step 30 that `quit` ends before step 60 (a script's line `N` lands after step
/// N - 1).
fn quits(quit: &str) -> Captured {
    let (mut app, bugs) = launch(200, &["--stop-after", "30"]);
    let c = app.run_script_unread(&format!("60: {quit}\n"));
    assert!(c.exit.success(), "{quit}: {}", c.stderr);
    assert!(c.stderr.contains("stopped at step 30"), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: quit after step 59\n"), "{quit}: {}", c.stderr);
    the_zip(&bugs);
    c
}

#[test]
fn esc_after_a_stop_quits_cleanly() {
    require_tree();
    let c = quits("press Escape");
    assert!(!c.stderr.contains("Recording in progress"), "{}", c.stderr);
}

#[test]
fn cmd_q_after_a_stop_quits_cleanly() {
    require_tree();
    quits("menu quit");
}

#[test]
fn start_level_s_recorder_on_the_translated_program_saves_a_zip_that_replays() {
    require_tree();
    let (mut app, bugs) = launch(400, &[]);
    let c = app.run_script_unread(
        "note on the translated program\n1: menu start-level\nform start\n\
         20: down ArrowRight\n80: up ArrowRight\n121: menu stop-recording\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("recording saved to"), "{}", c.stderr);
    let (zip, rec, _) = the_zip(&bugs);
    assert_eq!((rec.start, rec.engine, rec.stop.clone()), (StartPoint::Level1, EngineKind::Translated, None));
    assert_eq!(rec.note, "on the translated program");
    let out = replay(&zip);
    assert_eq!(line(&out, "frames"), "120", "{out}");
    assert_eq!(line(&out, "replay-stop"), "-", "{out}");
    assert_eq!(line(&out, "replay-at"), line(&out, "stopped-at"), "{out}");
    assert!(out.contains("screenshot equal\n"), "{out}");
}
