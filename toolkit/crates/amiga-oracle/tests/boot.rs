//! Game-free: Kickstart 1.3 with no disk, booted to its insert-disk screen. Needs the Kickstart
//! (`AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`) and `toolkit/tools/setup/oracle.sh`'s build;
//! a missing Kickstart fails with where it looked.

use amiga_oracle::*;

/// The first frame showing the insert-disk hand, measured on vAmiga 899da5dd with the patches here
/// (2026-10-29); from it the picture stays the same.
const HAND: u32 = 207;
/// Frames each run steps: the hand and a stretch of it standing still.
const FRAMES: u32 = HAND + 30;
/// Where the snapshot is taken: mid-boot, before the screen is drawn.
const MID: u32 = 120;

fn no_disk(frames: u32) -> OracleConfig {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    OracleConfig::new(rom, Vec::new(), frames)
}

/// Every frame's picture and the RAM digest after it.
fn run(a: &mut Amiga, frames: u32) -> Vec<(Rgb12Frame, u64)> {
    (0..frames)
        .map(|_| {
            let c = a.step(FrameInput::NONE).unwrap();
            (c.picture, a.memory_hash())
        })
        .collect()
}

#[test]
fn a_disk_less_boot_reaches_the_insert_disk_screen_the_same_way_twice() {
    let cfg = no_disk(FRAMES);
    assert_eq!(cfg.boot, None);
    let mut a = Amiga::power_on(&cfg).unwrap();
    assert_eq!(a.df0(), None);
    let first = run(&mut a, FRAMES);
    let second = run(&mut Amiga::power_on(&cfg).unwrap(), FRAMES);
    assert!(first == second, "two boots with no disk differ");

    let hand = HAND as usize;
    assert_ne!(first[hand - 1].0, first[hand].0, "the hand is drawn at frame {HAND}");
    assert!(
        first[hand..].iter().all(|f| f.0 == first[hand].0),
        "the insert-disk screen stands still"
    );
    // The hand, the disk and its label: six colours in a full PAL-height window.
    let mut colours = first[hand].0.pixels.clone();
    colours.sort_unstable();
    colours.dedup();
    assert_eq!((first[hand].0.width, colours.len()), (320, 6));
}

#[test]
fn a_snapshot_taken_mid_boot_continues_as_the_uninterrupted_run() {
    let cfg = no_disk(FRAMES);
    let whole = run(&mut Amiga::power_on(&cfg).unwrap(), FRAMES);

    let mut a = Amiga::power_on(&cfg).unwrap();
    run(&mut a, MID);
    let snap = a.snapshot();
    assert_eq!(snap.frame, MID);
    // Run on past it, then come back.
    run(&mut a, 40);
    a.restore(&snap);
    assert_eq!(a.frame(), MID);
    assert!(run(&mut a, FRAMES - MID) == whole[MID as usize..], "restored in place");

    // Restored into a fresh machine.
    let mut b = Amiga::power_on(&cfg).unwrap();
    b.restore(&snap);
    assert!(run(&mut b, FRAMES - MID) == whole[MID as usize..], "restored into a new machine");
}

#[test]
fn disks_are_named_by_index_and_df0_can_be_emptied() {
    // A blank double-density image: not bootable, so the Kickstart keeps asking.
    let blank = vec![0u8; amiga_disk::ADF_SIZE];
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let cfg = OracleConfig::new(rom, vec![blank], 10);
    assert_eq!(cfg.boot, Some(0));
    let mut a = Amiga::power_on(&cfg).unwrap();
    assert_eq!(a.df0(), Some(0));
    a.step(FrameInput::NONE).unwrap();
    a.eject_disk(0);
    assert_eq!(a.df0(), None);
    a.step(FrameInput::NONE).unwrap();
    a.insert_disk(0, 0);
    assert_eq!(a.df0(), Some(0));

    let missing = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a.insert_disk(0, 1)));
    let why = missing.unwrap_err();
    let why = why.downcast_ref::<String>().unwrap();
    assert!(why.contains("no disk 1"), "{why}");
    let no_drive = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a.insert_disk(1, 0)));
    let why = no_drive.unwrap_err();
    assert!(why.downcast_ref::<String>().unwrap().contains("no drive df1"));
    assert_eq!(a.df0(), Some(0));
}
