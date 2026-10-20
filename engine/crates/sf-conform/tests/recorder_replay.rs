//! `sf-conform bug replay` (bug-recorder T04, DESIGN §2.5, §2.7): a bug zip built here as the app
//! will write one (T06) — W1-1 started from the dialog's defaults, `frog-run-right`'s inputs, the
//! engine's own last picture as the screenshot, its place as `stopped-at` — replayed by the real
//! binary. Needs the disks, as every sf-conform test does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use sf_conform::{InputScript, bug, scripts_dir};
use sf_core::Rng;
use sf_core::input::{FrameInput, Joy};
use sf_core::recording::{LevelSetup, Recording, ZIP_RECORDING, ZIP_SCREENSHOT};

/// Past the frog's drop-in and the first two jumps: long enough that a change at frame 0 shows.
const FRAMES: u32 = 400;

fn recording(first: FrameInput) -> Recording {
    let script = InputScript::load(&scripts_dir().join("frog-run-right.txt")).unwrap();
    let mut inputs: Vec<FrameInput> = (0..FRAMES).map(|f| script.input_at(f)).collect();
    // `0: R` holds to frame 172; the changed recording holds `first` there instead.
    for i in inputs.iter_mut().take(172) {
        *i = first;
    }
    Recording {
        build: "test".into(),
        setup: LevelSetup::default(),
        seed: Rng::default(),
        stopped_at: None,
        note: "built by recorder_replay.rs".into(),
        inputs,
    }
}

/// The recording played once, its end written in as the app writes it at STOP, and zipped.
fn faithful_zip() -> &'static [u8] {
    static ZIP: OnceLock<Vec<u8>> = OnceLock::new();
    ZIP.get_or_init(|| {
        let mut rec = recording(FrameInput {
            joy: Joy::RIGHT,
            ..FrameInput::NONE
        });
        let end = bug::replay(&rec, |_, _, _| {});
        rec.stopped_at = Some(end.location);
        zip(&rec, &bug::picture_png(&end.last))
    })
}

fn zip(rec: &Recording, png: &[u8]) -> Vec<u8> {
    sf_disk::zip::write_stored(&[
        (ZIP_RECORDING, rec.to_text().as_bytes()),
        (ZIP_SCREENSHOT, png),
    ])
}

/// A fresh folder under the system's temp dir.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sf-recorder-replay-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

fn sf_conform(args: &[&str], home: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_sf-conform"));
    c.args(args);
    if let Some(h) = home {
        c.env("HOME", h);
    }
    c.output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn pngs(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn the_replay_ends_on_the_screenshot_and_the_place() {
    let dir = scratch("equal");
    let zip = write(&dir, "bug-W1-1-test.zip", faithful_zip());
    let (rec, _) = bug::read_zip(faithful_zip()).unwrap();
    let at = rec.stopped_at.expect("written");
    assert!(at.to_string().starts_with("W1-1:"), "{at}");
    let o = sf_conform(
        &[
            "bug",
            "replay",
            zip.to_str().unwrap(),
            "--out",
            dir.join("out").to_str().unwrap(),
        ],
        None,
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        stdout(&o),
        format!("frames {FRAMES}\nstopped-at {at}\nreplay-at {at}\nscreenshot equal\n")
    );
}

#[test]
fn a_changed_input_differs() {
    let dir = scratch("differs");
    let (faithful, png) = bug::read_zip(faithful_zip()).unwrap();
    // `0: R` became `0: L`: the frog runs the other way, and the end is not the screenshot.
    let mut rec = recording(FrameInput {
        joy: Joy::LEFT,
        ..FrameInput::NONE
    });
    rec.stopped_at = faithful.stopped_at;
    let zip = write(&dir, "changed.zip", &zip(&rec, &png));
    let o = sf_conform(
        &[
            "bug",
            "replay",
            zip.to_str().unwrap(),
            "--out",
            dir.join("out").to_str().unwrap(),
        ],
        None,
    );
    assert!(o.status.success(), "a mismatch is reported, not fatal");
    let out = stdout(&o);
    let line = out.lines().last().unwrap();
    assert!(
        line.starts_with("screenshot differs (") && line.ends_with(" pixels)"),
        "{out}"
    );
    let at = faithful.stopped_at.unwrap().to_string();
    assert!(!out.contains(&format!("replay-at {at}")), "{out}");
}

#[test]
fn what_is_not_a_bug_zip_is_refused_with_the_reason() {
    let dir = scratch("refused");
    let bad_header = bug::read_zip(faithful_zip())
        .unwrap()
        .0
        .to_text()
        .replace("energy 4", "energy 9");
    for (name, bytes, why) in [
        ("plain.zip", b"just some text".to_vec(), "not a bug zip"),
        (
            "no-recording.zip",
            sf_disk::zip::write_stored(&[(ZIP_SCREENSHOT, b"png")]),
            "no recording.txt",
        ),
        (
            "bad-header.zip",
            sf_disk::zip::write_stored(&[
                (ZIP_RECORDING, bad_header.as_bytes()),
                (ZIP_SCREENSHOT, b"png"),
            ]),
            "recording.txt: line 6: energy `9` is not 1 to 4 hearts",
        ),
    ] {
        let zip = write(&dir, name, &bytes);
        let o = sf_conform(
            &[
                "bug",
                "replay",
                zip.to_str().unwrap(),
                "--out",
                dir.join("out").to_str().unwrap(),
            ],
            None,
        );
        assert!(!o.status.success(), "{name}: refused");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains(why), "{name}: {err}");
        assert!(stdout(&o).is_empty(), "{name}: nothing replayed");
    }
    assert!(pngs(&dir.join("out")).is_empty());
}

#[test]
fn frames_and_every_write_exactly_the_frames_asked() {
    let dir = scratch("frames");
    let zip = write(&dir, "bug-W1-1-picked.zip", faithful_zip());
    let out = dir.join("out");
    let o = sf_conform(
        &[
            "bug",
            "replay",
            zip.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--frames",
            "10..30",
            "--every",
            "7",
        ],
        None,
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(
        pngs(&out),
        ["frame-000010.png", "frame-000017.png", "frame-000024.png"]
    );
}

/// With no `--out` the frames go under `~/.cache/superfrog-re/bugs/<zip stem>/` — a temp `HOME`
/// here — the last 250 of them, and nothing is written under the repo.
#[test]
fn by_default_the_last_250_frames_go_to_the_cache_not_the_repo() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let untracked = || {
        let o = Command::new("git")
            .args([
                "status",
                "--porcelain",
                "--untracked-files=all",
                "--ignored",
            ])
            .current_dir(&repo)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| l.ends_with(".png") && !l.contains("/target/"))
            .map(String::from)
            .collect::<Vec<_>>()
    };
    let before = untracked();
    let home = scratch("home");
    let zip = write(&home, "bug-W1-1-default.zip", faithful_zip());
    let o = sf_conform(&["bug", "replay", zip.to_str().unwrap()], Some(&home));
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let written = pngs(&home.join(".cache/superfrog-re/bugs/bug-W1-1-default"));
    let want: Vec<String> = (FRAMES - 250..FRAMES)
        .map(|f| format!("frame-{f:06}.png"))
        .collect();
    assert_eq!(written, want);
    assert_eq!(untracked(), before, "no picture written under the repo");
}
