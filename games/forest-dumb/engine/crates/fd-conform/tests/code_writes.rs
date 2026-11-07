//! T05 (translator trial DESIGN §2.6): changes to the bytes of executed instructions, on level 1's
//! cases on the original. `w1-idle` from the cached level sync point, a few seconds warm.

use std::path::{Path, PathBuf};

use amiga_oracle::kickstart_path;
use fd_conform::Case;
use fd_conform::code_writes::{Report, glob_match, run_case, run_case_in};
use fd_conform::routine::h;

/// `read_joystick`'s first instruction, `move.w #$0, joy_left`: run every pass that is not paused.
/// Its fourth byte is the immediate's low byte, so flipping it changes data, not the flow.
const READ_JOYSTICK: u32 = h(0, 0x8dfc);

fn idle() -> Case {
    Case::load(Path::new("cases/w1-idle.toml")).unwrap_or_else(|e| panic!("{e}"))
}

/// `w1-idle` for `frames` steps, `poke_at` given the frame before which `READ_JOYSTICK + 3` is
/// flipped.
fn run_poked(frames: u32, poke_at: Option<u32>) -> Report {
    run_case(&idle(), Some(frames), |am, frame| {
        if Some(frame) == poke_at {
            let b = am.peek(READ_JOYSTICK + 3, 1)[0];
            am.poke(READ_JOYSTICK + 3, &[b ^ 1]);
        }
    })
    .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_short_level_1_case_runs_code_and_changes_none() {
    let r = run_case(&idle(), None, |_, _| {}).unwrap_or_else(|e| panic!("{e}"));
    // 99 frames from the level sync point to the case's 6400, then its 200.
    assert_eq!(r.frames, 299);
    assert!(r.executed_bytes > 0 && r.executed > 0, "{}", r.json());
    assert!(r.changed.is_empty(), "{}", r.json());
}

#[test]
fn a_poke_into_an_executed_instruction_is_reported_with_its_address_and_frame() {
    let r = run_poked(30, Some(6321));
    assert_eq!(r.changed.len(), 1, "{}", r.json());
    let c = &r.changed[0];
    assert_eq!(
        (c.at.as_str(), c.frame, c.since),
        ("h0+$8dfc", 6321, "run"),
        "{}",
        r.json()
    );
    assert_eq!(c.from.len(), 8);
    assert_eq!(c.from[3] ^ c.to[3], 1);
    assert!(
        r.json().contains(r#""at":"h0+$8dfc","frame":6321"#),
        "{}",
        r.json()
    );
}

#[test]
fn a_write_before_the_first_run_is_reported_against_the_loaded_program() {
    let r = run_poked(10, Some(6301));
    assert_eq!(r.changed.len(), 1, "{}", r.json());
    let c = &r.changed[0];
    assert_eq!(
        (c.at.as_str(), c.since),
        ("h0+$8dfc", "load"),
        "{}",
        r.json()
    );
    assert_eq!(c.from[3] ^ c.to[3], 1);
}

#[test]
fn a_case_not_at_level_1_is_refused() {
    let c = Case::load(Path::new("cases/first-case.toml")).unwrap();
    let err = run_case(&c, Some(1), |_, _| {}).unwrap_err();
    assert!(err.contains("does not start at level 1"), "{err}");
}

#[test]
fn the_case_glob_matches_stems() {
    assert!(glob_match("*", "w1-idle"));
    assert!(glob_match("stretch-*", "stretch-run"));
    assert!(glob_match("w?-idle", "w1-idle"));
    assert!(!glob_match("stretch-*", "w1-idle"));
    assert!(!glob_match("w1", "w1-idle"));
}

struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Temp {
        let d = std::env::temp_dir().join(format!("fd-code-writes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        Temp(d)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_missing_kickstart_names_the_path() {
    let disks = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    let cache = Temp::new("nokick");
    let rom = Path::new("/nonexistent/fd-code-writes/kick.rom");
    let err = run_case_in(rom, &disks, &cache.0, &idle(), Some(1), |_, _| {}).unwrap_err();
    assert!(err.contains(&rom.display().to_string()), "{err}");
}

#[test]
fn missing_disks_name_the_path() {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let cache = Temp::new("nodisks");
    let empty = Temp::new("empty");
    std::fs::create_dir_all(&empty.0).unwrap();
    let err = run_case_in(&rom, &empty.0, &cache.0, &idle(), Some(1), |_, _| {}).unwrap_err();
    assert!(err.contains(&empty.0.display().to_string()), "{err}");
}
