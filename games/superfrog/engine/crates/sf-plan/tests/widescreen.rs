//! `sf-plan --widescreen` (real-widescreen T12): `--app` with View ▸ Widescreen on from the level's
//! first frame, planned and confirmed on the engine alone, since the widescreen game has no original.
//! The routes it made are replayed to each level's end by sf-conform's `widescreen_complete.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sf_conform::InputScript;
use sf_plan::app;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sf-plan-wide-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn plan(args: &[&str], d: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sf-plan"))
        .args(args)
        .env("SF_PLAN_HINTS_LOG", d.join("hints.log"))
        .output()
        .unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn stat(o: &Output, key: &str) -> String {
    let e = stderr(o);
    let line = e.lines().last().unwrap_or("");
    assert!(line.starts_with("PLAN_STATS"), "last stderr line: {e}");
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("no {key} in {line}"))
        .to_string()
}

/// Nothing of the original: no route to follow from it, no snapshot of it, no confirming on it.
#[test]
fn widescreen_refuses_the_original() {
    let d = scratch("usage");
    let old = sf_conform::case::script_path("complete-w1-1").display().to_string();
    for (args, says) in [
        (&["W1-1", "--widescreen", "--follow", &old][..], "no --follow"),
        (&["W1-1", "--widescreen", "--engine-only"], "no --engine-only"),
        (&["W1-1", "--widescreen", "--snap", "x.snap"], "no --snap"),
        (&["WB-1", "--widescreen"], "not a normal level"),
    ] {
        let o = plan(args, &d);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", stderr(&o));
        let e = stderr(&o);
        assert!(e.contains(says) && e.contains("usage: sf-plan <level>"), "{args:?}: {e}");
    }
}

/// A run that ends in the settle: the partial route is app-timed with its seed and a widescreen
/// line, so it is read back as the wide start's, and nothing was played on the original.
#[test]
fn a_widescreen_route_says_so_and_never_touches_the_original() {
    let d = scratch("frames");
    let out = d.join("route.txt");
    let o = plan(&["W1-1", "--widescreen", "--seed", "2", "--frames", "100", "--out", out.to_str().unwrap()], &d);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert_eq!(stat(&o, "oracle_frames"), "0");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("# timing: app\n# seed: 2\n# widescreen: on\n"), "{text}");
    assert!(app::route_widescreen(&text));
    assert!(!app::route_widescreen("# timing: app\n# seed: 2\n0: -\n"));
    assert_eq!(InputScript::parse(&text).unwrap().lines.last().map(|l| l.0), Some(100));
}

/// The wide start is the app's with widescreen switched on before the first step: the level is
/// entered again wide, so its start camera is the wide one (real-widescreen T04: WB-1's moves from
/// x 17 to 82), not a 4:3 start widened afterwards.
#[test]
fn the_wide_start_enters_the_level_wide() {
    let camera = |g: &sf_core::Game| g.state().l(sf_core::level::camera::CAMERA_X);
    let wb1 = "WB-1".parse().unwrap();
    let (narrow, wide) = (app::app_start_in(wb1, 1, false), app::app_start_in(wb1, 1, true));
    assert!(wide.widescreen() && !narrow.widescreen());
    assert!(camera(&wide) > camera(&narrow), "{} vs {}", camera(&wide), camera(&narrow));
}
