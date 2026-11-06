//! The map page made by the real `fd-routeview` from a scripted run, read back against an
//! independent replay of the same script on the engine; where it is written by default; and what a
//! missing script says.

use std::path::{Path, PathBuf};
use std::process::Command;

use amiga_conform::script;
use fd_core::recording::{LevelId, Location};
use fd_core::{Game, Start};

const BIN: &str = env!("CARGO_BIN_EXE_fd-routeview");

fn script_path() -> PathBuf {
    fd_conform::conform_dir().join("scripts/stretch-run.txt")
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("fd-routeview-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// The JSON array after `"key":` in `s`, brackets matched.
fn array<'a>(s: &'a str, key: &str) -> &'a str {
    let at = s
        .find(&format!("\"{key}\":["))
        .unwrap_or_else(|| panic!("no {key}"))
        + key.len()
        + 3;
    let mut depth = 0;
    for (i, ch) in s[at..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' if depth == 1 => return &s[at..at + i + 1],
            ']' => depth -= 1,
            _ => {}
        }
    }
    panic!("{key} not closed")
}

#[test]
fn a_scripted_run_makes_a_page_whose_track_matches_the_replay() {
    let out = temp("page");
    let run = Command::new(BIN)
        .arg("--script")
        .arg(script_path())
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );

    // The same script on the engine, stepped here without the tool.
    let text = std::fs::read_to_string(script_path()).unwrap();
    let s = script::parse(&text).unwrap();
    let frames = s.lines.last().unwrap().0 + fd_routeview::TAIL;
    let mut g = Game::new(fd_conform::game_data().unwrap(), Start::Level1);
    for k in 0..frames {
        g.step(s.input_at(k));
    }
    let end = Location::of_character(LevelId::W1_1, g.state().char_x, g.state().char_y);
    // The script runs left to the wall and back: it must end somewhere else than it started, or
    // the comparison below proves nothing.
    assert_ne!(end.to_string(), "W1-1:7,7");

    let html = std::fs::read_to_string(out.join("index.html")).unwrap();
    let manifest = html
        .split(r#"<script id="manifest" type="application/json">"#)
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert!(
        manifest.starts_with(r#"[{"id":"W1-1","label":"W1-1","group":"W1","status":"unknown""#),
        "{manifest}"
    );
    assert!(manifest.contains(r#""data":"data/W1-1.js""#), "{manifest}");
    assert!(
        manifest.contains(&format!(r#""route_file":"{}""#, script_path().display())),
        "{manifest}"
    );
    assert!(html.contains("<title>Forest Dumb Forever Route Viewer</title>"));
    assert!(
        !html.contains("http://") && !html.contains("https://"),
        "the page asks for nothing outside its folder"
    );

    let js = std::fs::read_to_string(out.join("data/W1-1.js")).unwrap();
    assert!(
        js.starts_with(
            r#"window.ROUTE_DATA=window.ROUTE_DATA||{};ROUTE_DATA["W1-1"]={"level":"W1-1","#
        ),
        "{}",
        &js[..120]
    );
    assert!(manifest.contains(fd_routeview::NO_VERDICT), "{manifest}");
    assert!(
        js.contains(r#""cols":280,"rows":96,"coins_needed":282,"#),
        "{}",
        &js[..200]
    );
    assert_eq!(
        js.matches(r#","coin","coin"]"#).count(),
        332,
        "docs/10: W1-1 holds 332 coins"
    );
    let track = array(&js, "track");
    let points: Vec<&str> = track[1..track.len() - 1].split("],[").collect();
    assert_eq!(points.len() as u32, frames, "a track point a frame");
    let last: Vec<u32> = points
        .last()
        .unwrap()
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|n| n.parse().unwrap())
        .collect();
    assert_eq!((last[0], last[1]), (g.state().char_x, g.state().char_y));
    assert_eq!(Location::of_character(LevelId::W1_1, last[0], last[1]), end);
    assert_eq!(array(&js, "body").split(',').count(), 280 * 96);
    assert!(
        array(&js, "events").contains(r#""jump","take-off""#),
        "the running jump is on the page"
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains(&format!("{frames} frames, ends at {end}")),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn the_page_goes_under_build_which_git_ignores() {
    let out = fd_routeview::default_out();
    let game = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    assert_eq!(out, game.join("build/route-viz"));
    for f in ["index.html", "data/W1-1.js"] {
        let ignored = Command::new("git")
            .args(["check-ignore", "-q"])
            .arg(out.join(f))
            .current_dir(&game)
            .status()
            .unwrap();
        assert!(
            ignored.success(),
            "{} is not git-ignored",
            out.join(f).display()
        );
    }
}

#[test]
fn a_missing_script_names_the_path() {
    let out = temp("missing");
    let missing = out.join("no-such-script.txt");
    let run = Command::new(BIN)
        .arg("--script")
        .arg(&missing)
        .arg("--out")
        .arg(&out)
        .output()
        .unwrap();
    assert!(!run.status.success());
    let err = String::from_utf8_lossy(&run.stderr);
    assert!(err.contains(&missing.display().to_string()), "{err}");
    assert!(!out.exists(), "nothing is written");
}
