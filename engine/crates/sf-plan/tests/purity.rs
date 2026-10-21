//! The purity test: the guard on the planner's core/shell boundary (plans/smart-planner DESIGN §3.1).
//!
//! `src/core/` takes game states and level facts as arguments and may step sf-core, nothing else:
//! no clock, file, environment, process, network, oracle or conformance runner (the T7MP arrives as
//! an argument). That is what makes the core testable in milliseconds and reproducible to the byte
//! (M8).
//!
//! If this test fails, move the code to the shell side. Never relax the test.

use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 8] = [
    "std::time",
    "std::fs",
    "std::env",
    "std::process",
    "std::net",
    "Instant",
    "sf_oracle",
    "sf_conform",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).expect("src/core").flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// `src` without its `//` comments (doc comments may name the forbidden things to say why).
fn code(src: &str) -> String {
    src.lines()
        .map(|l| l.find("//").map_or(l, |i| &l[..i]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The forbidden names in `src`'s code, with their line numbers.
fn violations(src: &str) -> Vec<(usize, &'static str)> {
    let mut v = Vec::new();
    for (i, line) in code(src).lines().enumerate() {
        for f in FORBIDDEN {
            let hit = line.match_indices(f).any(|(at, _)| {
                // A whole name: `Instant` but not `InstantX`, `sf_oracle` but not `sf_oracles`.
                let before = line[..at].chars().next_back();
                let after = line[at + f.len()..].chars().next();
                !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                    && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
            });
            if hit {
                v.push((i + 1, f));
            }
        }
    }
    v
}

#[test]
fn core_is_pure() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core");
    let mut files = Vec::new();
    rust_files(&dir, &mut files);
    files.sort();
    assert!(!files.is_empty(), "no sources under {}", dir.display());
    let mut bad = Vec::new();
    for p in &files {
        let src = std::fs::read_to_string(p).unwrap();
        for (line, f) in violations(&src) {
            bad.push(format!("{}:{line}: {f}", p.display()));
        }
    }
    assert!(
        bad.is_empty(),
        "src/core/ must not use a clock, file, environment, process, network, oracle or \
         conformance runner (DESIGN §3.1): move the code to the shell side, never relax this test.\n{}",
        bad.join("\n")
    );
}

#[test]
fn the_scan_sees_what_it_forbids() {
    assert_eq!(violations("use std::time::Instant;"), vec![(1, "std::time"), (1, "Instant")]);
    assert_eq!(violations("let t = std::fs::read(p);"), vec![(1, "std::fs")]);
    assert_eq!(violations("sf_conform::game_data()"), vec![(1, "sf_conform")]);
    assert!(violations("// std::env is named here to say why").is_empty());
    assert!(violations("let instant_ish = InstantX;").is_empty());
}
