//! The first comparison of our code with the original (DESIGN §2.8), and how a run fails when its
//! inputs are missing. Needs the disks and the Kickstart: absent, it fails naming where it looked.

use std::path::Path;
use std::process::Command;

use fd_conform::{Case, Verdict, disks, run};

fn first_case() -> Case {
    Case::load(Path::new("cases/first-case.toml")).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_idle_title_matches_the_original_on_every_pass() {
    let case = first_case();
    let verdict = run(&case).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        verdict,
        Verdict::Same {
            passes: case.frames + 1
        },
        "{verdict}"
    );
}

/// The comparison can fail: T06's trap, one frame early, is seen and named. Both sides read 0 at
/// frame 4921 and the original stays there one frame more.
#[test]
fn a_start_one_frame_early_is_caught_and_named() {
    let case = Case {
        sync_frame: 4921,
        frames: 12,
        ..first_case()
    };
    assert_eq!(
        run(&case).unwrap_or_else(|e| panic!("{e}")),
        Verdict::Differs {
            pass: 1,
            frame: 4922,
            var: "mt_counter".into(),
            engine: 1,
            oracle: 0,
        }
    );
}

#[test]
fn a_missing_disk_names_the_path() {
    let dir = std::env::temp_dir().join(format!("fd-conform-nodisks-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let err = disks::read(&dir).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(err.contains(&dir.display().to_string()), "{err}");
    assert!(err.contains("disk 1 of 2"), "{err}");
}

#[test]
fn a_missing_kickstart_names_the_path_and_does_not_panic() {
    let rom = "/nonexistent/fd-conform/kick.rom";
    let out = Command::new(env!("CARGO_BIN_EXE_fd-conform"))
        .args(["run", "cases/first-case.toml"])
        .env("AMIGA_KICKSTART", rom)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains(rom), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn a_missing_case_names_the_path_and_does_not_panic() {
    let out = Command::new(env!("CARGO_BIN_EXE_fd-conform"))
        .args(["run", "cases/no-such-case.toml"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains("no-such-case.toml"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}
