//! T13: the shipped app holds no emulator and no test tooling (DESIGN §3.1): `fd-app`'s normal and
//! build dependencies never reach `amiga-oracle`, the conformance crates or the end-to-end driver.

use std::process::Command;

const NEVER: [&str; 5] = [
    "amiga-oracle",
    "fd-conform",
    "amiga-conform",
    "fd-e2e",
    "amiga-e2e",
];

#[test]
fn the_app_does_not_link_the_oracle_or_test_tooling() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .args([
            "tree",
            "--offline",
            "--quiet",
            "-p",
            "fd-app",
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
        "cargo tree: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8_lossy(&out.stdout);
    let names: Vec<&str> = tree
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert!(
        names.contains(&"fd-app") && names.contains(&"fd-core"),
        "cargo tree not understood: {names:?}"
    );
    for n in NEVER {
        assert!(!names.contains(&n), "fd-app depends on {n}");
    }
}
