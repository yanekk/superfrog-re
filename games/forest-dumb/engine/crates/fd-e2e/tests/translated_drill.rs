//! Translator T41, the translated app's drill (DESIGN §2.10, §2.4): what a person does with the
//! app that the feature tests (`translated_app.rs`, `app_stop.rs`, `translated_whole.rs`) did not
//! drive. KONIEC on the title, which quits the app as Esc does (owner, T41); a key held through
//! level 1's load and the window resized during the load and in play, focus lost after; the stop
//! band at the default scale and in a resized window, its zip replayed. The app's state is held to
//! a headless run of the same program on the inputs it fed, every step.
//!
//! Full: needs the owner's disks and the generated tree (`translate.py --build`), and fails naming
//! the command without it. The drill's screens and what was judged on them are in the plan's
//! FINDINGS (2026-11-20), worker-driven.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{DOWN, RIGHT};
use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_e2e::{App, Captured, LaunchOpts};
use fd_translated::TranslatedGame;

/// Chip RAM's size, where slow RAM starts in the engine's memory.
const CHIP: usize = 512 << 10;

/// The translated picture, as the app shows it (`fd_app::CANVAS`).
const W: u32 = fd_app::CANVAS.2 as u32;
const H: u32 = fd_app::CANVAS.3 as u32;

/// Fire, the default keys' Left Option, pressed at the disk 2 prompt: the title is up from about
/// step 184 (T37).
const PAST_THE_PROMPT: &str = "20: down AltLeft\n30: up AltLeft\n";

fn require_tree() {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
}

/// The translated program from power-on, level 1 mounted, as the app runs it.
fn headless() -> TranslatedEngine {
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut e = TranslatedEngine::power_on(&disks).unwrap_or_else(|e| panic!("{e}"));
    e.mount_level_1(common::data());
    e
}

fn state(e: &TranslatedEngine) -> String {
    let m = e.ram().expect("a frame has run");
    format!("{:016x}", fd_app::memory_hash(&m[..CHIP], &m[CHIP..]))
}

/// The headless run on the app's inputs, steps 1 to `to`, its state equal to the app's at each.
/// The headless run, for what it says after.
fn follow(c: &Captured, to: u64) -> TranslatedEngine {
    let fed = common::inputs(c);
    let hashes = common::hashes(c);
    let mut e = headless();
    for n in 1..=to {
        e.frame(fed.get(&n).copied().unwrap_or_default());
        assert_eq!(state(&e), hashes[n as usize], "state at step {n}");
    }
    e
}

/// The translated app, its zips in its home's `test-bugs`, `args` added.
fn launch(max_frames: u64, scale: Option<u32>, args: &[&str]) -> (App, PathBuf) {
    let mut app = App::launch_translated(LaunchOpts {
        max_frames,
        timeout_s: 180,
        scale,
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    app.opts.extra_args.extend(args.iter().map(|a| a.to_string()));
    (app, bugs)
}

/// The zips in `bugs`, none when the folder was never made.
fn zips(bugs: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(bugs)
        .map(|d| d.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

/// `key` pressed for ten steps from `at`.
fn press(at: u64, key: &str) -> String {
    format!("{at}: down {key}\n{}: up {key}\n", at + 10)
}

/// The step an stderr line `prefix N…` names.
fn step_after(c: &Captured, prefix: &str) -> u64 {
    c.stderr
        .lines()
        .find_map(|l| l.strip_prefix(prefix))
        .and_then(|l| l.split(|ch: char| !ch.is_ascii_digit()).next())
        .unwrap_or_else(|| panic!("no `{prefix}`: {}", c.stderr))
        .parse()
        .unwrap()
}

/// The window's size in points, as `resize` asks for it.
fn points(s: &fd_e2e::Shot) -> (u32, u32) {
    let p = |v: u32| (v as f64 / s.scale_factor).round() as u32;
    (p(s.surface.0), p(s.surface.1))
}

/// The `shot` at step `n` shows the frame it names whole, centred and as captured; its shot.
fn shown_whole(c: &Captured, n: u64) -> fd_e2e::Shot {
    let (frame, img) = common::shown(c, n, W, H);
    assert!(img == c.read_frame(frame), "step {n}: the window shows frame {frame} as captured");
    let (_, shot) = c.read_shot(n);
    let (x, y, vw, vh) = shot.viewport;
    assert!(
        (shot.surface.0 - vw) / 2 == x && (shot.surface.1 - vh) / 2 == y,
        "step {n}: {shot:?} is not centred"
    );
    shot
}

/// KONIEC, the title's third row, with fire: the original returns to AmigaDOS (`quit_to_dos`),
/// which the app does not run, so the app quits as Esc does, with no stop and no bug zip. Before
/// T41 it froze on `Stopped: untranslated: $000000` and saved a zip every time. The headless run
/// reports the exit by name, so a case reaching it fails there.
#[test]
fn koniec_on_the_title_quits_the_app_as_esc_does_with_no_stop_and_no_zip() {
    require_tree();
    let (mut app, bugs) = launch(800, Some(1), &[]);
    // Down twice from START: KODY, then KONIEC; fire on it.
    let c = app.run_script_unread(&format!(
        "{PAST_THE_PROMPT}{}{}{}",
        press(300, "ArrowDown"),
        press(350, "ArrowDown"),
        press(400, "AltLeft")
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(!c.stderr.contains("stopped at step"), "{}", c.stderr);
    let quit = step_after(&c, "fd-app: the program quit to AmigaDOS at step ");
    assert!((400..800).contains(&quit), "quit at step {quit}");
    assert!(
        c.stderr
            .contains(&format!("fd-app: menu quit after step {quit}\nfd-app: quit after step {quit}\n")),
        "quit as Esc does, at once: {}",
        c.stderr
    );
    assert_eq!(zips(&bugs), Vec::<PathBuf>::new(), "no bug zip");
    assert_eq!(common::joy(&c, 300, 309), vec![DOWN; 10], "the arrow reached the game");
    let e = follow(&c, quit);
    let stop = e.stopped().expect("the headless run ended at the quit");
    assert!(stop.contains("exit: the program returned from its entry"), "{stop}");
}

/// Right held from the title through level 1's load and into play, the window resized on the
/// SCENA screen and again in play, then focus lost: the key reaches the game every step until the
/// focus goes, and not after; every shot shows its frame whole and centred, and the last has the
/// last size. At the default scale.
#[test]
fn a_key_held_through_level_1s_load_and_resizes_during_it_and_in_play() {
    require_tree();
    let end = 800;
    let (mut app, _) = launch(end, None, &[]);
    // Fire ten steps in twenty from the title until play begins (step 620, T40).
    let fire: String = (300..=700).step_by(20).map(|at| press(at, "AltLeft")).collect();
    let shots: String = (480..=end - 10).step_by(10).map(|n| format!("{n}: shot\n")).collect();
    let c = app.run_script_unread(&format!(
        "{PAST_THE_PROMPT}{fire}500: down ArrowRight\n560: resize 900 640\n\
         680: resize 500 420\n760: blur\n{shots}"
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    let begins = step_after(&c, "fd-app: level 1 begins at step ");
    assert!((500..680).contains(&begins), "play begins at {begins}, inside the held stretch");
    for (n, j) in (500..760).zip(common::joy(&c, 500, 759)) {
        assert_eq!(j & RIGHT, RIGHT, "right held at step {n}");
    }
    assert!(
        common::joy(&c, 760, end).iter().all(|j| j & RIGHT == 0),
        "the lost focus let go of right"
    );
    follow(&c, end);
    for n in (480..=end - 10).step_by(10) {
        shown_whole(&c, n);
    }
    let last = shown_whole(&c, end - 10);
    assert_eq!(points(&last), (500, 420), "not resized by step {}: {last:?}", end - 10);
}

/// The stop band at the default scale and in a 1000×700 window: the window shows the captured
/// frame, band included, whole and centred, the band's white text in its lower rows, and the zip
/// replays to the same stop and screenshot. `app_stop.rs` drives the smallest window.
#[test]
fn the_stop_band_shows_whole_at_the_default_scale_and_resized_and_its_zip_replays() {
    require_tree();
    for resize in ["", "50: resize 1000 700\n"] {
        let (mut app, bugs) = launch(500, None, &["--stop-after", "400"]);
        let c = app.run_script_unread(&format!("{PAST_THE_PROMPT}{resize}480: shot\n"));
        assert!(c.exit.success(), "{}", c.stderr);
        assert!(
            c.stderr.contains(
                "fd-app: the translated program stopped at step 400: test-stop: --stop-after 400\n"
            ),
            "{}",
            c.stderr
        );
        let shot = shown_whole(&c, 480);
        if !resize.is_empty() {
            assert_eq!(points(&shot), (1000, 700), "{shot:?}");
        }
        let (_, img) = common::shown(&c, 480, W, H);
        let white = |y: u32| {
            (0..W).any(|x| img.rgba[((y * W + x) * 4) as usize..][..3] == [255, 255, 255])
        };
        assert!((H - 40..H).any(white), "{resize:?}: the band's text at the picture's foot");
        let zips = zips(&bugs);
        assert_eq!(zips.len(), 1, "{zips:?}");
        let out = Command::new(fd_e2e::conform_binary())
            .args(["bug", "replay"])
            .arg(&zips[0])
            .output()
            .expect("running fd-conform");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
        assert!(
            stdout.contains("replay-stop test-stop: --stop-after 400 at frame 400\n")
                && stdout.contains("screenshot equal\n"),
            "{stdout}"
        );
    }
}

/// KONIEC with a recording running, Quit cancelled at "Recording in progress": the program has
/// ended and its last frame stays, so the app asks once and not again every step (before the T41
/// review's fix it asked every step, 188 times by step 5100); Esc then quits as usual. The recording starts at
/// level 1 (Start Level), right held into the spikes to game over, fire past it and the high
/// scores to the title (up by step 4700), then KONIEC.
#[test]
fn koniec_while_recording_asks_once_and_a_cancel_is_not_asked_again() {
    require_tree();
    let (mut app, bugs) = launch(5100, Some(1), &[]);
    let c = app.run_script_unread(&format!(
        "fast-until 4750\n10: menu start-level\nform level W1-1\nform record on\nform start\n\
         answer-recording cancel\nanswer-recording discard\n\
         20: down ArrowRight\n4300: up ArrowRight\n{}{}{}{}{}5000: press Escape\n",
        press(4450, "AltLeft"),
        press(4600, "AltLeft"),
        press(4800, "ArrowDown"),
        press(4850, "ArrowDown"),
        press(4900, "AltLeft")
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    let quit = step_after(&c, "fd-app: the program quit to AmigaDOS at step ");
    assert!((4900..5000).contains(&quit), "quit at step {quit}");
    assert_eq!(c.stderr.matches("the program quit to AmigaDOS").count(), 1, "{}", c.stderr);
    assert_eq!(c.stderr.matches("Quit cancelled; still recording").count(), 1, "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: recording discarded\n"), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: quit after step 4999\n"), "Esc quits: {}", c.stderr);
    assert_eq!(zips(&bugs), Vec::<PathBuf>::new(), "no bug zip");
}
