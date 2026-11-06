//! T14: a recording made in the app replays headless to the same result (DESIGN §1, §2.8): the
//! real `fd-app` records W1-1 into a zip, and the real `fd-conform bug replay <zip>` plays it and
//! prints `screenshot equal`. A zip from another build or level is refused, naming the difference
//! (DESIGN §2.12).

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fd_core::recording::{ForestDumb, Recording};
use fd_e2e::{App, LaunchOpts};

fn bug_replay(zip: &Path) -> Output {
    Command::new(fd_e2e::conform_binary())
        .args(["bug", "replay"])
        .arg(zip)
        .output()
        .expect("running fd-conform")
}

/// One zip of the app's: W1-1 with right held, then up, stopped after 200 steps.
fn recorded() -> (App, PathBuf) {
    let mut app = App::launch(LaunchOpts {
        max_frames: 220,
        timeout_s: 120,
        scale: Some(1),
        ..Default::default()
    });
    let bugs = app.home.join("test-bugs");
    app.opts
        .extra_args
        .extend(["--bugs-dir".into(), bugs.display().to_string()]);
    let c = app.run_script(
        "note replay me\n1: menu start-level\nform start\n10: down ArrowRight\n120: press Space\n\
         180: up ArrowRight\n201: menu stop-recording\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    let zip: Vec<PathBuf> = std::fs::read_dir(&bugs)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(zip.len(), 1, "{}", c.prompts);
    (app, zip[0].clone())
}

#[test]
fn a_zip_saved_in_the_app_replays_headless_to_its_screenshot() {
    let (_app, zip) = recorded();
    let out = bug_replay(&zip);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("frames 200\n"), "{stdout}");
    assert!(stdout.contains("screenshot equal\n"), "{stdout}");
    let place = |key: &str| {
        stdout
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .unwrap_or_else(|| panic!("no {key}: {stdout}"))
            .to_string()
    };
    assert_eq!(place("stopped-at "), place("replay-at "));
    assert_ne!(place("replay-at "), "W1-1:7,7", "the character moved");
}

/// `zip`'s recording with its text changed by `edit`, written as a new zip beside it.
fn edited(zip: &Path, name: &str, edit: impl Fn(&str) -> String) -> PathBuf {
    let (rec, png) = amiga_record::read_zip::<ForestDumb>(&std::fs::read(zip).unwrap()).unwrap();
    let text = edit(&Recording::to_text(&rec));
    let bytes = amiga_disk::zip::write_stored(&[
        ("recording.txt", text.as_bytes()),
        ("screenshot.png", &png),
    ]);
    let out = zip.with_file_name(name);
    std::fs::write(&out, bytes).unwrap();
    out
}

#[test]
fn a_zip_from_another_build_or_level_is_refused_naming_the_difference() {
    let (_app, zip) = recorded();
    let other = edited(&zip, "other-build.zip", |t| {
        t.replacen(
            &format!("build {}", fd_tools::ENGINE_BUILD),
            "build forest-dumb 0.0.1 (0000000000000000)",
            1,
        )
    });
    let out = bug_replay(&other);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        err.contains("`forest-dumb 0.0.1 (0000000000000000)`")
            && err.contains(&format!("`{}`", fd_tools::ENGINE_BUILD)),
        "{err}"
    );
    let level = edited(&zip, "other-level.zip", |t| {
        t.replacen("level W1-1", "level W2-1", 1)
    });
    let out = bug_replay(&level);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(
        err.contains("level `W2-1` is not one this engine starts (W1-1)"),
        "{err}"
    );
    let version = edited(&zip, "other-version.zip", |t| {
        t.replacen("forest-dumb recording 1", "forest-dumb recording 9", 1)
    });
    let out = bug_replay(&version);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success());
    assert!(err.contains("recording format `9`"), "{err}");
}
