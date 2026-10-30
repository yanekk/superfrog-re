//! The oracle with more than one drive, and its memory write (T31): one drive stays the default,
//! a connected df1 is what the original's own `probe_drives` sees, the two set-ups never share a
//! trace, and a write lands where it was aimed. Needs what `determinism.rs` needs.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use sf_core::layout::h0;
use sf_oracle::*;

/// The crack intro skips on a port-1 click; the game has probed the drives well before frame 4 000.
const CLICK: std::ops::Range<u32> = 1_000..1_005;
const PROBED: u32 = 4_000;
/// `probe_drives` (h0+$b666) stores one long per drive at h0+$b6c8: 0 connected, $1e not.
const PROBE_DF1: u32 = h0(0xb6cc);

fn cfg() -> OracleConfig {
    OracleConfig::locate(DEFAULT_MAX_FRAMES).unwrap_or_else(|e| panic!("{e}"))
}

fn probe_df1(c: &OracleConfig) -> u32 {
    let mut a = Amiga::power_on(c).unwrap();
    for f in 0..PROBED {
        let input = if CLICK.contains(&f) {
            FrameInput::joy(Joy::PORT1_BUTTON)
        } else {
            FrameInput::NONE
        };
        a.step(input).unwrap();
    }
    a.memory().long(PROBE_DF1).unwrap()
}

#[test]
fn one_drive_is_the_default_and_df1_is_refused_there() {
    let c = cfg();
    assert_eq!(c.drives, 1);
    let mut a = Amiga::power_on(&c).unwrap();
    let e = catch_unwind(AssertUnwindSafe(|| a.insert_disk(1, DiskSlot::D2))).unwrap_err();
    let msg = e.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(msg.contains("no drive df1"), "{msg}");
}

#[test]
fn the_original_sees_df1_only_when_it_is_connected() {
    assert_eq!(probe_df1(&cfg()), 0x1e, "one drive: df1 absent");
    assert_eq!(probe_df1(&cfg().with_drives(2)), 0, "two drives: df1 there");
}

#[test]
fn a_disk_goes_into_a_connected_df1() {
    let mut a = Amiga::power_on(&cfg().with_drives(2)).unwrap();
    a.insert_disk(1, DiskSlot::D4);
    a.step(FrameInput::NONE).unwrap();
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sf-oracle-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn one_and_two_drives_never_share_a_cached_trace() {
    let dir = scratch("drives");
    let script = InputScript::idle();
    let run = |c: &OracleConfig| {
        cached_trace_in(&dir, c, &SyncPoint::PowerOn, &script, 2)
            .unwrap()
            .1
    };
    assert_eq!(run(&cfg()), CacheOutcome::Miss);
    assert_eq!(run(&cfg().with_drives(2)), CacheOutcome::Miss);
    assert_eq!(run(&cfg()), CacheOutcome::Hit);
    assert_eq!(run(&cfg().with_drives(2)), CacheOutcome::Hit);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_memory_write_lands_in_chip_and_slow_ram_and_nowhere_else() {
    let mut a = Amiga::power_on(&cfg()).unwrap();
    a.step(FrameInput::NONE).unwrap();
    // Just after reset the Kickstart's overlay maps ROM at 0: refused, not written to ROM.
    let e = catch_unwind(AssertUnwindSafe(|| a.write_memory(0x1000, &[1]))).unwrap_err();
    let msg = e.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(msg.contains("not chip or slow RAM"), "{msg}");
    for _ in 0..100 {
        a.step(FrameInput::NONE).unwrap();
    }
    a.write_memory(0x1000, &[0xde, 0xad, 0xbe, 0xef]);
    a.write_memory(0xc0_2000, &[0x12, 0x34]);
    let m = a.memory();
    assert_eq!(m.long(0x1000), Some(0xdead_beef));
    assert_eq!(m.word(0xc0_2000), Some(0x1234));
}
