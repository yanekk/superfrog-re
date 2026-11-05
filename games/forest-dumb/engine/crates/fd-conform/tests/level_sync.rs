//! The level sync point and its cache (T03): reached from the cache the original is where a boot
//! leaves it, the key follows what decides the state, a damaged file rebuilds, and a missing input
//! names its path. Needs the disks and the Kickstart: absent, it fails naming where it looked.

use std::path::{Path, PathBuf};
use std::time::Instant;

use amiga_conform::sync_cache::{cache_path, decode};
use amiga_oracle::{Amiga, FrameInput, kickstart_path, read_only};
use fd_conform::sync::{
    LEVEL_1_SYNC, level_1_key, level_1_script, reach_level_1, reach_level_1_in,
};
use fd_conform::{HUNK_BASES, disks};

/// A fresh cache folder of this test's own, removed on drop.
struct TempCache(PathBuf);

impl TempCache {
    fn new(name: &str) -> TempCache {
        let d = std::env::temp_dir().join(format!("fd-level-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        TempCache(d)
    }
}

impl Drop for TempCache {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn inputs() -> (PathBuf, PathBuf) {
    (
        kickstart_path().unwrap_or_else(|e| panic!("{e}")),
        fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}")),
    )
}

fn key_here(kickstart: &Path, disk_dir: &Path) -> [u8; 20] {
    let rom = read_only(kickstart).unwrap();
    let disks = disks::read(disk_dir).unwrap();
    level_1_key(&rom, &disks, &level_1_script().unwrap().0)
}

fn assert_same_ram(a: &Amiga, b: &Amiga, what: &str) {
    let (ma, mb) = (a.memory(), b.memory());
    assert!(ma.chip == mb.chip, "chip RAM differs: {what}");
    assert!(ma.slow == mb.slow, "slow RAM differs: {what}");
}

/// A boot, a restore and a rebuild over a damaged file all land on the same frame with the same
/// chip and slow RAM, and that frame is play's first (`irq_count` 1, docs/09-play-measured.md).
#[test]
fn cached_uncached_and_rebuilt_reach_give_identical_ram() {
    let (ks, dir) = inputs();
    let cache = TempCache::new("same");
    let key = key_here(&ks, &dir);
    let path = cache_path(&cache.0, key);

    let booted = reach_level_1_in(&ks, &dir, &cache.0, 10).unwrap();
    assert!(
        path.is_file(),
        "the boot stored no snapshot at {}",
        path.display()
    );
    assert_eq!(booted.frame(), LEVEL_1_SYNC);
    let irq_count = booted.peek(HUNK_BASES[0] + 0x1e6a, 2);
    assert_eq!(irq_count, [0, 1], "one interrupt of play at the sync frame");

    let started = Instant::now();
    let mut cached = reach_level_1_in(&ks, &dir, &cache.0, 10).unwrap();
    let took = started.elapsed();
    assert_eq!(cached.frame(), LEVEL_1_SYNC);
    assert_same_ram(&booted, &cached, "boot against cache");
    // Generous for a loaded machine; the measured figure is in the commit.
    assert!(took.as_secs() < 5, "from the cache took {took:?}");

    // The cap is the sync point plus the frames asked for, cached or not.
    for _ in 0..10 {
        cached.step(FrameInput::NONE).unwrap();
    }
    assert!(cached.step(FrameInput::NONE).is_err(), "ran past the cap");

    // Cut short, and the snapshot's last byte changed: each rebuilds to the same state.
    let good = std::fs::read(&path).unwrap();
    for (how, bad) in [
        ("cut short", good[..good.len() / 2].to_vec()),
        ("a byte changed", {
            let mut b = good.clone();
            let n = b.len() - 30;
            b[n] ^= 0xff;
            b
        }),
    ] {
        std::fs::write(&path, &bad).unwrap();
        let rebuilt = reach_level_1_in(&ks, &dir, &cache.0, 10).unwrap();
        assert_same_ram(&booted, &rebuilt, how);
        // Two boots' snapshots need not be byte-equal, only sound and of the same state.
        let again = std::fs::read(&path).unwrap();
        assert!(
            decode(&again, b"FDSYNC\0\0", key).is_some(),
            "{how}: not rewritten"
        );
    }
}

/// From the owner's own cache: measured for the Done when, a boot the first time on a machine.
#[test]
fn reach_level_1_from_the_cache_takes_under_5_s() {
    reach_level_1(0).unwrap_or_else(|e| panic!("{e}"));
    let started = Instant::now();
    let a = reach_level_1(0).unwrap_or_else(|e| panic!("{e}"));
    let took = started.elapsed();
    eprintln!("reach_level_1 from the cache: {took:?}");
    assert_eq!(a.frame(), LEVEL_1_SYNC);
    assert!(took.as_secs() < 5, "{took:?}");
}

#[test]
fn a_changed_script_disk_or_kickstart_changes_the_key() {
    let (ks, dir) = inputs();
    let rom = read_only(&ks).unwrap();
    let disks = disks::read(&dir).unwrap();
    let text = level_1_script().unwrap().0;
    let base = level_1_key(&rom, &disks, &text);
    assert_eq!(base, key_here(&ks, &dir), "the key is not stable");

    let script = level_1_key(&rom, &disks, &text.replace("6300: F", "6301: F"));
    let mut d2 = disks.clone();
    d2[1][1000] ^= 1;
    let disk = level_1_key(&rom, &d2, &text);
    let mut rom2 = rom.clone();
    rom2[0] ^= 1;
    let kick = level_1_key(&rom2, &disks, &text);
    for (what, k) in [("script", script), ("disk", disk), ("kickstart", kick)] {
        assert_ne!(base, k, "a changed {what} kept the key");
    }
    // The emulator build is in the key text: amiga-conform's
    // `every_input_changes_the_key` holds that one.
}

#[test]
fn a_missing_kickstart_names_the_path() {
    let (_, dir) = inputs();
    let cache = TempCache::new("nokick");
    let rom = Path::new("/nonexistent/fd-level-sync/kick.rom");
    let err = reach_level_1_in(rom, &dir, &cache.0, 1).err().unwrap();
    assert!(err.contains(&rom.display().to_string()), "{err}");
}

#[test]
fn missing_disks_name_the_path() {
    let (ks, _) = inputs();
    let cache = TempCache::new("nodisks");
    let empty = TempCache::new("empty");
    std::fs::create_dir_all(&empty.0).unwrap();
    let err = reach_level_1_in(&ks, &empty.0, &cache.0, 1).err().unwrap();
    assert!(err.contains(&empty.0.display().to_string()), "{err}");
    assert!(!cache.0.exists(), "a failed reach wrote to the cache");
}
