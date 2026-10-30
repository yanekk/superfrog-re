//! The name check (toolkit-extract DESIGN §2.4): no file under `toolkit/` names a game. It fails
//! when a line of any file git tracks (or would track) there names the game the toolkit came from
//! or one of that game's crates, case-insensitively: code, comments, manifests, shaders, patches,
//! scripts and Markdown alike. A line that must say where a number came from carries `learned on:`
//! and is let through; it states the provenance and nothing more.
//!
//! The names are built from pieces, so that this file does not trip its own check.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The marker that lets a provenance line name the game a value was measured on.
const PROVENANCE: &str = "learned on:";

/// The game's own crates, after its two-letter prefix and a `-` or `_`.
const GAME_CRATES: [&str; 11] = [
    "core",
    "assets",
    "disk",
    "oracle",
    "conform",
    "app",
    "e2e",
    "plan",
    "bench",
    "routeview",
    "paula",
];

/// The names the check looks for, lowercase.
fn names() -> Vec<String> {
    let game = ["super", "frog"].concat();
    let studio = "team";
    let mut out = vec![game, format!("{studio}17"), format!("{studio} 17")];
    for sep in ["-", "_"] {
        for c in GAME_CRATES {
            out.push(format!("{}{sep}{c}", ["s", "f"].concat()));
        }
    }
    out
}

/// Every line of `text` naming a game, as `path:line: text`.
fn hits(path: &str, text: &str) -> Vec<String> {
    let names = names();
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.contains(PROVENANCE))
        .filter(|(_, l)| {
            let lower = l.to_ascii_lowercase();
            names.iter().any(|n| lower.contains(n.as_str()))
        })
        .map(|(i, l)| format!("{path}:{}: {}", i + 1, l.trim()))
        .collect()
}

fn toolkit_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The files under `toolkit/` that git tracks or would track (not ignored), relative to it.
fn toolkit_files() -> Vec<String> {
    let out = Command::new("git")
        .current_dir(toolkit_dir())
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            ".",
        ])
        .output()
        .expect("run git ls-files");
    assert!(
        out.status.success(),
        "git ls-files: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .split('\0')
        .filter(|f| !f.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn no_file_in_the_toolkit_names_a_game() {
    let files = toolkit_files();
    for want in ["Cargo.toml", "crates/toolkit-checks/tests/names.rs"] {
        assert!(
            files.iter().any(|f| f == want),
            "git ls-files did not list {want}: {files:?}"
        );
    }
    let mut found = Vec::new();
    for f in &files {
        let path = toolkit_dir().join(f);
        // A file listed but deleted in the working tree has nothing left to check.
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        found.extend(hits(f, &String::from_utf8_lossy(&bytes)));
    }
    assert!(
        found.is_empty(),
        "{} line(s) in toolkit/ name a game (DESIGN §2.4); say it generically, or, for where a \
         number came from, a `{PROVENANCE}` line:\n  {}",
        found.len(),
        found.join("\n  ")
    );
}

#[test]
fn a_game_name_is_found_in_any_case_and_form() {
    let game = ["Super", "frog"].concat();
    let krate = ["sf", "_core"].concat();
    let studio = ["Team", " 17"].concat();
    let text = format!(
        "// fine\n// drawn as {game} does\nuse {krate}::Game;\n# {studio}\n{}\n{}\n",
        ["team", "17"].concat(),
        ["SF", "-PAULA"].concat()
    );
    let found = hits("x.rs", &text);
    assert_eq!(found.len(), 5, "{found:?}");
    assert!(found[0].starts_with("x.rs:2: "), "{found:?}");
    assert!(found[4].starts_with("x.rs:6: "), "{found:?}");
    for clean in [
        ["sf", "-x"].concat().as_str(),
        "sfcore",
        "use amiga_chips::Picture;",
        "team 18",
        "super frog",
    ] {
        assert!(hits("x.rs", clean).is_empty(), "flagged: {clean}");
    }
}

#[test]
fn a_learned_on_line_is_let_through() {
    let game = ["Super", "frog"].concat();
    let line = format!("// learned on: {game}'s title screen, 2026-10-12");
    assert!(hits("x.rs", &line).is_empty());
    // Only that line: the marker on one line lets no other through.
    let text = format!("{line}\n// as {game} does\n");
    assert_eq!(hits("x.rs", &text), [format!("x.rs:2: // as {game} does")]);
}
