//! Translator T26, the machine's OS answers on the whole translated program (full: Ghidra's export,
//! the tree built into `fd-judge`, the owner's disks and Kickstart). It wraps the tool, as
//! `insn_gate.rs` does: extract the disks, export the whole program (`pcode_export.py --whole`),
//! translate it (`pcode_rust.py --out build/translator/tree/`), then `fd-judge --os-run`, built with
//! `FD_TRANSLATED` naming the tree. About 30 s warm. Leaves fd-conform built with the tree until the
//! next build without it.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

/// One export and one build at a time: Ghidra's project and the build directory are shared.
static ONE: Mutex<()> = Mutex::new(());

fn game() -> PathBuf {
    // Canonical: fd_tools::extract refuses an output path holding `..`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn ok(cmd: &mut Command, what: &str) -> String {
    let out = cmd.output().unwrap_or_else(|e| panic!("{what}: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{what} exited {}:\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
}

/// `fd-judge --os-run <which>`'s line, on the whole program translated afresh.
fn os_run(which: &str) -> String {
    let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let disks = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    fd_tools::extract(&disks, &game().join("build")).unwrap_or_else(|e| panic!("{e}"));
    let py = |args: &[&str]| {
        ok(
            Command::new("python3")
                .arg("-I")
                .args(args)
                .current_dir(game()),
            &format!("{args:?}"),
        )
    };
    py(&["tools/pcode_export.py", "--whole"]);
    let tree = game().join("build/translator/tree");
    py(&[
        "tools/pcode_rust.py",
        "build/translator/pcode.json",
        "--out",
        tree.to_str().unwrap(),
    ]);
    let line = ok(
        Command::new("cargo")
            .args([
                "run",
                "-q",
                "--release",
                "-p",
                "fd-conform",
                "--bin",
                "fd-judge",
                "--",
            ])
            .args(["--os-run", which])
            .env("FD_TRANSLATED", &tree)
            .env_remove("FD_PORT")
            .current_dir(game().join("engine")),
        &format!("fd-judge --os-run {which}"),
    );
    line.lines().last().expect("a result line").to_string()
}

#[test]
#[ignore = "full: Ghidra's export, the tree built in, the original booted to frame 4075; about 30 s warm"]
fn load_file_on_the_machine_loads_napis_as_the_original_does() {
    let l = os_run("load-napis");
    assert!(l.contains("\"stop\":\"returned\""), "{l}");
    assert!(l.contains("\"opened\":[\"_NAPIS\"],\"missing\":[]"), "{l}");
    assert!(l.contains("\"differ\":0,"), "{l}");
    assert!(l.contains("\"equal\":true"), "{l}");
    // The picture is there on the original, not an empty match: measured 2026-11-12.
    assert!(!l.contains("\"nonzero\":0,"), "{l}");
}

#[test]
#[ignore = "full: Ghidra's export and the tree built in; power-on to the title, under a second once built"]
fn power_on_reaches_the_title_with_no_stop() {
    let l = os_run("power-on");
    assert!(l.contains("\"at_title\":true"), "{l}");
    for f in ["df0:!EAGLE_SOFT!", "_NAPIS", "_CZOLOWKA", "_OPCJE"] {
        assert!(l.contains(&format!("\"{f}\"")), "{f} not opened: {l}");
    }
    assert!(l.contains("\"missing\":[]"), "{l}");
}
