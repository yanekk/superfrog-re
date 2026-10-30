//! bug-recorder T07: `superfrog --replay <zip>` in the real app. A zip recorded through the app
//! (T06's path) plays back watch-only from its level start, ends on its screenshot, says the frame
//! it finished at and holds there; the player's keys and the menu commands that would change the
//! game change nothing, Pause still pauses; a zip that cannot be played says why and fails
//! (DESIGN §2.7, §5.2).

mod common;

use std::path::{Path, PathBuf};

use sf_e2e::{App, Captured, Image, LaunchOpts};

/// The script T06's first test records: W1-1 started with the recorder on, 300 steps of keys, STOP.
const RECORD: &str = "note x\n1: start-level W1-1 record\n20: down ArrowRight\n90: down Space\n\
                      95: up Space\n150: up ArrowRight\n200: down ArrowLeft\n260: up ArrowLeft\n\
                      301: menu stop-recording\n";

/// A temporary home with the cache, and a bug zip of 300 frames recorded in it by the app.
fn recorded() -> (App, PathBuf, Captured) {
    let mut app = App::launch(LaunchOpts {
        max_frames: 320,
        timeout_s: 120,
        scale: Some(1),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts.extra_args = vec!["--bugs-dir".into(), bugs.display().to_string()];
    let c = app.run_script(RECORD);
    assert!(c.exit.success(), "{}", c.stderr);
    let zips: Vec<PathBuf> = std::fs::read_dir(&bugs).unwrap().flatten().map(|e| e.path()).collect();
    assert_eq!(zips.len(), 1, "{zips:?}\n{}", c.stderr);
    (app, zips[0].clone(), c)
}

/// `zip` replayed in `app` with `script` for at most `max_frames` steps.
fn replay(app: &mut App, zip: &Path, max_frames: u64, script: &str) -> Captured {
    app.opts.max_frames = max_frames;
    app.opts.extra_args = vec!["--replay".into(), zip.display().to_string()];
    app.run_script(script)
}

/// The zip's screenshot.
fn screenshot(zip: &Path, scratch: &Path) -> Image {
    let bytes = std::fs::read(zip).unwrap();
    let entries = sf_disk::zip::read_entries(&bytes, |n| n == "screenshot.png", 1 << 24).unwrap();
    let p = scratch.join("screenshot-check.png");
    std::fs::write(&p, &entries[0].bytes).unwrap();
    Image::read_png(&p).unwrap()
}

fn finished_lines(c: &Captured) -> Vec<&str> {
    c.stderr.lines().filter(|l| l.contains("replay finished")).collect()
}

#[test]
fn a_replay_plays_the_recording_to_its_screenshot_and_holds_there() {
    let (mut app, zip, rec) = recorded();
    let progress = std::fs::read(app.home.join("progress.toml")).ok();
    let c = replay(&mut app, &zip, 330, "");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(finished_lines(&c), ["superfrog: replay finished at frame 300"], "{}", c.stderr);
    // Held: nothing captured after the last input's frame, and it is the screenshot.
    assert_eq!(c.frames.len(), 301, "frames 0..=300");
    assert_eq!(*c.frames.last().unwrap(), screenshot(&zip, &c.dir), "the last frame is screenshot.png");
    // Step for step what the recording run showed and fed the game once its level started.
    for n in 1..=300 {
        assert!(c.frames[n] == rec.frames[n], "frame {n} differs from the recording run's");
    }
    let fed = |c: &Captured| -> Vec<String> {
        c.inputs.lines().filter(|l| l.split(' ').next().unwrap().parse::<u64>().unwrap() <= 300).map(str::to_string).collect()
    };
    assert!(fed(&rec).len() > 100);
    assert_eq!(fed(&c), fed(&rec), "the game was fed the recording's inputs");
    // A replay never moves the saved progress (it is a Start Level game).
    assert_eq!(std::fs::read(app.home.join("progress.toml")).ok(), progress);
}

#[test]
fn keys_and_game_menu_commands_change_nothing_and_pause_still_pauses() {
    let (mut app, zip, _) = recorded();
    let clean = replay(&mut app, &zip, 320, "");
    assert!(clean.exit.success(), "{}", clean.stderr);
    let meddled = replay(
        &mut app,
        &zip,
        320,
        "5: start-level W1-2 record\n10: down ArrowLeft\n40: down Space\n45: up Space\n60: up ArrowLeft\n\
         70: down KeyP\n71: up KeyP\n80: down Escape\n81: up Escape\n100: menu reset\n110: menu start-level\n\
         120: menu save-state-1\n130: menu stop-recording\n140: menu controls\n150: menu import\n\
         160: menu play-level-file\n170: menu continue\n180: down ArrowRight\n250: up ArrowRight\n",
    );
    assert!(meddled.exit.success(), "{}", meddled.stderr);
    assert_eq!(meddled.frames.len(), clean.frames.len());
    for (n, (a, b)) in meddled.frames.iter().zip(&clean.frames).enumerate() {
        assert!(a == b, "frame {n} changed by keys or menu commands during the replay");
    }
    assert_eq!(meddled.inputs, clean.inputs);
    for id in ["start-level", "reset", "save-state-1", "stop-recording", "controls", "import", "play-level-file", "continue"] {
        assert!(meddled.stderr.contains(&format!("superfrog: {id} is disabled during a replay")), "{id}: {}", meddled.stderr);
    }
    assert!(!meddled.prompts.contains("start-level"), "no dialog was answered: {}", meddled.prompts);
    assert_eq!(finished_lines(&meddled), ["superfrog: replay finished at frame 300"]);

    // Pause holds the replay for its frames and it carries on after, to the same end.
    let paused = replay(&mut app, &zip, 340, "100: menu pause\n121: menu pause\n");
    assert!(paused.exit.success(), "{}", paused.stderr);
    assert_eq!(finished_lines(&paused), ["superfrog: replay finished at frame 300"], "{}", paused.stderr);
    for n in 100..=120 {
        assert!(paused.frames[n] == clean.frames[99], "paused frame {n} shows frame 99");
    }
    assert!(paused.frames[121] == clean.frames[100]);
    assert_eq!(paused.frames.len(), 322, "frames 0..=321: 300 played, 21 paused");
    assert!(paused.frames.last() == clean.frames.last());
}

#[test]
fn a_zip_that_cannot_be_played_says_why_and_fails() {
    let mut app = App::launch(LaunchOpts { max_frames: 50, timeout_s: 60, ..Default::default() });
    let garbage = app.home.join("garbage.zip");
    std::fs::write(&garbage, b"this is not a zip").unwrap();
    let c = replay(&mut app, &garbage, 50, "");
    assert!(!c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("garbage.zip cannot be replayed: not a bug zip"), "{}", c.stderr);
    assert!(c.prompts.contains("notice: ") && c.prompts.contains("not a bug zip"), "{}", c.prompts);
    assert!(c.frames.is_empty(), "no frame played");

    let bad = app.home.join("bad.zip");
    std::fs::write(
        &bad,
        sf_disk::zip::write_stored(&[("recording.txt", b"superfrog bug recording 9\n".as_slice()), ("screenshot.png", b"")]),
    )
    .unwrap();
    let c = replay(&mut app, &bad, 50, "");
    assert!(!c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("bad.zip cannot be replayed: recording.txt: "), "{}", c.stderr);

    let missing = app.home.join("missing.zip");
    let c = replay(&mut app, &missing, 50, "");
    assert!(!c.exit.success());
    assert!(c.stderr.contains("missing.zip cannot be replayed: "), "{}", c.stderr);
}
