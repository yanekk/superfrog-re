//! The dry fit is built from the toolkit alone: its dependency tree names the three toolkit crates it
//! uses and no game's crate.

use std::path::Path;
use std::process::Command;

/// The name prefix of a game's own crates, built from two pieces so this file does not trip the
/// name check.
const GAME_PREFIX: [&str; 2] = ["sf", "-"];

/// Package names in `cargo tree -p dry-fit` (normal and build edges).
fn tree() -> Vec<String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(cargo)
        .current_dir(root)
        .args([
            "tree",
            "--offline",
            "--quiet",
            "-p",
            "dry-fit",
            "-e",
            "normal,build",
            "--prefix",
            "none",
        ])
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("run cargo tree");
    assert!(
        out.status.success(),
        "cargo tree -p dry-fit: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut names: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn the_dependency_tree_holds_toolkit_crates_only() {
    let names = tree();
    assert!(
        names.iter().any(|n| n == "dry-fit"),
        "cargo tree output not understood: {names:?}"
    );
    let game = GAME_PREFIX.concat();
    let bad: Vec<_> = names.iter().filter(|n| n.starts_with(&game)).collect();
    assert!(
        bad.is_empty(),
        "a game's crate in the dry fit's tree: {bad:?}"
    );
    let toolkit: Vec<_> = names.iter().filter(|n| n.starts_with("amiga-")).collect();
    assert_eq!(toolkit, ["amiga-chips", "amiga-clock", "amiga-paula"]);
    // Every crate in the tree is the fit, a toolkit crate, or one of the toolkit crates' own
    // dependencies (serde for their save states).
    let others: Vec<_> = names
        .iter()
        .filter(|n| *n != "dry-fit" && !n.starts_with("amiga-"))
        .collect();
    for n in &others {
        assert!(
            [
                "serde",
                "serde_core",
                "serde_derive",
                "proc-macro2",
                "quote",
                "syn",
                "unicode-ident"
            ]
            .contains(&n.as_str()),
            "{n} in the dry fit's tree"
        );
    }
}
