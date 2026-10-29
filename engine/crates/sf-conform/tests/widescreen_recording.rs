//! real-widescreen T07 (DESIGN §2.8, D9): a recording's widescreen. `bug replay` plays the mode the
//! recording says at frame 0 and switches it at each `W`, drawing the wide picture while it is on;
//! `bug compare` and `bug case` refuse any recording with widescreen in it, since a widescreen game
//! has no original. The format itself is `sf_core::recording`'s unit tests. Needs the disks, as
//! every sf-conform test does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sf_conform::{InputScript, bug, cases_dir, display_data, scripts_dir};
use sf_core::recording::{Header, LevelSetup, Recording, RecordingRng, seed_bytes, ZIP_RECORDING, ZIP_SCREENSHOT};
use sf_core::{Game, Rng};

/// Past the frog's drop-in and its first jumps: both toggles fall inside the level.
const FRAMES: u32 = 400;
/// Where [`toggled`] switches widescreen on, and off again.
const ON: u32 = 150;
const OFF: u32 = 300;

/// W1-1 from the dialog's defaults, `frog-run-right`'s inputs, widescreen at frame 0 as `wide`,
/// toggled at `toggles`.
fn recording(wide: bool, toggles: &[u32]) -> Recording {
    let script = InputScript::load(&scripts_dir().join("frog-run-right.txt")).unwrap();
    Recording {
        build: "test".into(),
        seed: seed_bytes(Rng::default()),
        note: "built by widescreen_recording.rs".into(),
        inputs: (0..FRAMES).map(|f| script.input_at(f)).collect(),
        header: Header {
            setup: LevelSetup::default(),
            stopped_at: None,
            widescreen: wide,
            toggles: toggles.to_vec(),
        },
    }
}

/// Off at frame 0, on at [`ON`], off at [`OFF`].
fn toggled() -> Recording {
    recording(false, &[ON, OFF])
}

/// `rec` zipped with a picture of the engine as the screenshot (any PNG the zip reader takes).
fn zip(rec: &Recording) -> Vec<u8> {
    let g = Game::start_level(display_data(), &rec.setup, rec.rng());
    let png = bug::picture_png(&g.render(g.view()));
    sf_disk::zip::write_stored(&[
        (ZIP_RECORDING, rec.to_text().as_bytes()),
        (ZIP_SCREENSHOT, &png),
    ])
}

/// A fresh folder under the system's temp dir.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "sf-widescreen-recording-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

fn sf_conform(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sf-conform"))
        .args(args)
        .output()
        .unwrap()
}

fn files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|r| {
            r.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// A PNG's width.
fn png_width(path: &Path) -> u32 {
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    dec.read_info().unwrap().info().width
}

#[test]
fn a_replay_with_two_toggles_is_the_engine_run_with_the_same_switches() {
    let rec = toggled();
    // The direct run: the engine started as the app starts it, its mode switched by hand at the
    // same frames, stepped in lockstep with the replay and held to it after every step.
    let mut direct = Game::start_level(display_data(), &rec.setup, rec.rng());
    let mut wide = false;
    let mut seen = 0;
    bug::replay(&rec, |k, game, _| {
        if k == ON || k == OFF {
            wide = !wide;
            direct.set_widescreen(wide);
        }
        direct.step(rec.inputs[k as usize]);
        assert_eq!(game.widescreen(), wide, "frame {k}");
        assert!(game.state() == direct.state(), "the state differs after frame {k}");
        seen += 1;
    });
    assert_eq!(seen, FRAMES);
}

#[test]
fn a_replay_draws_the_wide_picture_while_the_recording_is_in_widescreen() {
    let mut widths = Vec::new();
    let r = bug::replay(&toggled(), |_, _, pic| widths.push(pic.width));
    let narrow = widths[0];
    let wide = narrow + 2 * sf_core::level::screen::WIDE_EXTRA;
    for (k, &w) in widths.iter().enumerate() {
        let on = (ON as usize..OFF as usize).contains(&k);
        assert_eq!(w, if on { wide } else { narrow }, "frame {k}");
    }
    assert_eq!(r.last.width, narrow);
    // On at frame 0 and never toggled: wide from the first frame, and the zero-frame picture too.
    let mut rec = recording(true, &[]);
    let r = bug::replay(&rec, |k, _, pic| assert_eq!(pic.width, wide, "frame {k}"));
    assert_eq!(r.last.width, wide);
    rec.inputs.clear();
    assert_eq!(bug::replay(&rec, |_, _, _| {}).last.width, wide);
}

#[test]
fn bug_replay_writes_wide_frames_from_the_toggle_on() {
    let dir = scratch("replay");
    let mut rec = recording(false, &[ON]);
    rec.stopped_at = Some(sf_core::recording::Location::Screen("tally"));
    let zip = write(&dir, "bug-W1-1-wide.zip", &zip(&rec));
    let out = dir.join("out");
    let range = format!("{}..{}", ON - 3, ON + 3);
    let o = sf_conform(&[
        "bug",
        "replay",
        zip.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
        "--frames",
        &range,
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let names = files(&out);
    assert_eq!(names.len(), 6, "{names:?}");
    let widths: Vec<u32> = names.iter().map(|n| png_width(&out.join(n))).collect();
    let narrow = widths[0];
    let wide = narrow + 2 * u32::from(sf_core::level::screen::WIDE_EXTRA);
    assert_eq!(widths, [narrow, narrow, narrow, wide, wide, wide], "{names:?}");
    assert_eq!(names[3], format!("frame-{ON:06}.png"));
}

#[test]
fn bug_compare_and_bug_case_refuse_a_widescreen_recording() {
    let dir = scratch("refused");
    for (tag, rec) in [
        ("header", recording(true, &[])),
        ("toggle", recording(false, &[ON])),
        ("toggled-back", toggled()),
    ] {
        assert_eq!(
            bug::compare(&rec, sf_conform::Mode::Faithful).err().as_deref(),
            Some(bug::WIDESCREEN_REFUSED),
            "{tag}"
        );
        let zip = write(&dir, &format!("{tag}.zip"), &zip(&rec));
        let out = dir.join(format!("{tag}-out"));
        let o = sf_conform(&[
            "bug",
            "compare",
            zip.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]);
        assert_eq!(o.status.code(), Some(1), "{tag}");
        assert_eq!(
            String::from_utf8_lossy(&o.stderr),
            format!("{}\n", bug::WIDESCREEN_REFUSED),
            "{tag}"
        );
        assert!(o.stdout.is_empty(), "{tag}");
        assert_eq!(files(&out), Vec::<String>::new(), "{tag}: nothing written");

        let name = format!("widescreen-recording-test-{tag}");
        let o = sf_conform(&["bug", "case", zip.to_str().unwrap(), &name]);
        assert_eq!(o.status.code(), Some(1), "{tag}");
        assert_eq!(
            String::from_utf8_lossy(&o.stderr),
            format!("{}\n", bug::WIDESCREEN_REFUSED),
            "{tag}"
        );
        for p in [
            cases_dir().join(format!("{name}.toml")),
            scripts_dir().join(format!("{name}.txt")),
            scripts_dir().join(format!("{name}-steps.txt")),
        ] {
            assert!(!p.exists(), "{}: no case written", p.display());
        }
        let cases = scratch(&format!("{tag}-cases"));
        assert_eq!(
            bug::write_case(&rec, &name, "z.zip", &cases, &cases).err().as_deref(),
            Some(bug::WIDESCREEN_REFUSED),
            "{tag}"
        );
        assert_eq!(files(&cases), Vec::<String>::new(), "{tag}");
    }
}

#[test]
fn a_recording_with_no_widescreen_replays_as_before() {
    // Off and never toggled: the 4:3 game and picture, the same as a recording of version 1.
    let rec = recording(false, &[]);
    let v1 = rec
        .to_text()
        .replace("superfrog recording 2", "superfrog recording 1")
        .replace("widescreen off\n", "");
    assert_eq!(Recording::parse(&v1).as_ref(), Ok(&rec));
    assert_eq!(bug::comparable(&rec), Ok(()));
    let mut direct = Game::start_level(display_data(), &rec.setup, rec.rng());
    bug::replay(&rec, |k, game, pic| {
        let out = direct.step(rec.inputs[k as usize]);
        assert!(!game.widescreen());
        assert!(game.state() == direct.state() && *pic == out.picture, "frame {k}");
    });
}
