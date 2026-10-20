//! The oracle on the owner's disks and Kickstart: deterministic, restorable, steerable, capped,
//! cached and read-only (T05). Needs `tools/setup/oracle.sh` to have run (the test block's setup),
//! the four disks (DESIGN §5) and Kickstart 1.3 in `~/amiga-roms/` or `SUPERFROG_KICKSTART`; a test
//! that cannot find them fails with where it looked.
//!
//! One power-on-to-title run is shared (`title_run`): the determinism test races a second run
//! against it, and the input test starts from its title snapshot.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::SystemTime;

use sf_oracle::*;

/// The crack intro on disk 1 skips on a port-1 click (FINDINGS 2026-10-11); with the click held
/// for frames 1 000-1 004 the title is up by frame 4 500 (seen: Superfrog in cape, ©1993 Team 17).
const CLICK: std::ops::Range<u32> = 1_000..1_005;
const TITLE: u32 = 4_500;

fn cfg() -> OracleConfig {
    OracleConfig::locate(DEFAULT_MAX_FRAMES).unwrap_or_else(|e| panic!("{e}"))
}

fn boot_input(frame: u32) -> FrameInput {
    if CLICK.contains(&frame) {
        FrameInput::joy(Joy::PORT1_BUTTON)
    } else {
        FrameInput::NONE
    }
}

/// What a frame is compared on: the picture, the ordered Paula writes, and the RAM.
#[derive(Debug, PartialEq, Eq)]
struct Digest {
    picture: u64,
    paula: Vec<PaulaWrite>,
    memory: u64,
}

fn digest(amiga: &Amiga, c: &FrameCapture) -> Digest {
    let mut h = DefaultHasher::new();
    c.picture.hash(&mut h);
    Digest {
        picture: h.finish(),
        paula: c.paula.clone(),
        memory: amiga.memory_hash(),
    }
}

/// Power on and run to the title, recording every frame. Returns the digests, the snapshot taken
/// at the title and the title's capture.
fn run_to_title() -> (Vec<Digest>, Snapshot, FrameCapture) {
    let mut a = Amiga::power_on(&cfg()).unwrap();
    let mut out = Vec::with_capacity(TITLE as usize);
    let mut last = FrameCapture::default();
    for f in 0..TITLE {
        last = a.step(boot_input(f)).unwrap();
        out.push(digest(&a, &last));
    }
    (out, a.snapshot(), last)
}

fn title_run() -> &'static (Vec<Digest>, Snapshot, FrameCapture) {
    static RUN: OnceLock<(Vec<Digest>, Snapshot, FrameCapture)> = OnceLock::new();
    RUN.get_or_init(run_to_title)
}

fn first_difference(a: &[Digest], b: &[Digest]) -> Option<(usize, String)> {
    a.iter()
        .zip(b)
        .enumerate()
        .find(|(_, (x, y))| x != y)
        .map(|(i, (x, y))| {
            let what = if x.picture != y.picture {
                "picture"
            } else if x.paula != y.paula {
                "Paula writes"
            } else {
                "memory"
            };
            (i, what.to_string())
        })
}

#[test]
fn power_on_to_the_title_twice_is_identical_on_every_frame() {
    let second = std::thread::spawn(|| run_to_title().0);
    let (first, _, title) = title_run();
    let second = second.join().unwrap();
    assert_eq!(first.len(), second.len());
    if let Some((frame, what)) = first_difference(first, &second) {
        panic!("the two power-on runs differ first at frame {frame}, in the {what}");
    }
    // The title screen is up: the game's 320 x 256 window, lores, and the runs made Paula writes.
    let p = &title.picture;
    assert_eq!(
        (p.left, p.top, p.width, p.height),
        (0x81, 0x2C, 320, 256),
        "{p:?}"
    );
    assert!(!p.hires_detail);
    assert!(first.iter().map(|d| d.paula.len()).sum::<usize>() > 10_000);
    assert!(
        p.pixels.iter().any(|&c| c != p.pixels[0]),
        "the title frame is one flat colour"
    );
}

/// The boot click, plus port-2 fire and left held over frames 990-1 009 and key P over
/// 995-1 007: every kind of input is held across a snapshot at frame 1 000.
fn held_across_1000(frame: u32) -> FrameInput {
    let mut i = boot_input(frame);
    if (990..1_010).contains(&frame) {
        i.joy = i.joy | Joy::FIRE | Joy::LEFT;
    }
    if (995..1_008).contains(&frame) {
        i.keys = i.keys.with(0x19);
    }
    i
}

#[test]
fn a_restored_snapshot_continues_identically() {
    let c = cfg();
    let mut a = Amiga::power_on(&c).unwrap();
    for f in 0..1_000 {
        a.step(held_across_1000(f)).unwrap();
    }
    let snap = a.snapshot();
    assert_eq!(snap.frame, 1_000);
    assert_eq!(snap.input, held_across_1000(999));
    let run = |a: &mut Amiga| {
        (0..500)
            .map(|i| {
                let c = a.step(held_across_1000(1_000 + i)).unwrap();
                digest(a, &c)
            })
            .collect::<Vec<_>>()
    };
    let first = run(&mut a);
    let after = a.memory();

    // Restored into the instance that ran ahead (whose ports and keyboard last saw frame 1 499)...
    a.restore(&snap);
    assert_eq!(a.frame(), 1_000);
    let second = run(&mut a);
    if let Some((frame, what)) = first_difference(&first, &second) {
        panic!(
            "after restoring, frame {} differs in the {what}",
            1_000 + frame
        );
    }
    assert_eq!(a.memory(), after);
    assert_eq!(a.frame(), 1_500);

    // ...and into a fresh one.
    let mut fresh = Amiga::power_on(&c).unwrap();
    fresh.restore(&snap);
    let third = run(&mut fresh);
    if let Some((frame, what)) = first_difference(&first, &third) {
        panic!(
            "restored into a fresh instance, frame {} differs in the {what}",
            1_000 + frame
        );
    }
}

#[test]
fn input_reaches_the_game_on_the_title() {
    let (_, title, _) = title_run();
    let mut a = Amiga::power_on(&cfg()).unwrap();
    let mut run = |input: FrameInput| {
        a.restore(title);
        (0..20)
            .map(|_| {
                let c = a.step(input).unwrap();
                digest(&a, &c)
            })
            .collect::<Vec<_>>()
    };
    let idle = run(FrameInput::NONE);
    assert_eq!(
        idle,
        run(FrameInput::NONE),
        "the title is not reproducible from its snapshot"
    );

    // Port-2 fire opens the title menu: RAM changes on the frame it is pressed, the picture within
    // a few frames (measured: 4).
    let fire = run(FrameInput::joy(Joy::FIRE));
    let first = |field: fn(&Digest) -> u64, a: &[Digest]| {
        (0..a.len()).find(|&i| field(&a[i]) != field(&idle[i]))
    };
    assert!(
        first(|d| d.memory, &fire).is_some_and(|f| f <= 1),
        "fire did not reach RAM within 2 frames"
    );
    assert!(
        first(|d| d.picture, &fire).is_some_and(|f| f <= 8),
        "fire did not change the picture within 9 frames"
    );

    // A key reaches RAM within 2 frames, as P does in play (T00).
    let p = run(FrameInput::key(0x19));
    assert!(
        first(|d| d.memory, &p).is_some_and(|f| f <= 1),
        "key P did not reach RAM within 2 frames"
    );
}

#[test]
fn max_frames_is_enforced() {
    let mut c = cfg();
    c.max_frames = 10;
    let mut a = Amiga::power_on(&c).unwrap();
    for _ in 0..10 {
        a.step(FrameInput::NONE).unwrap();
    }
    assert_eq!(
        a.step(FrameInput::NONE),
        Err(OracleError::FrameCap { max_frames: 10 })
    );
    assert_eq!(a.frame(), 10, "a refused step must not run");

    // A trace longer than the cap errors too, and nothing is cached.
    let dir = scratch("cap");
    let e = cached_trace_in(&dir, &c, &SyncPoint::PowerOn, &InputScript::idle(), 11).unwrap_err();
    assert_eq!(e, OracleError::FrameCap { max_frames: 10 });
    assert!(
        std::fs::read_dir(&dir)
            .map(|d| d.count() == 0)
            .unwrap_or(true)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sf-oracle-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn the_trace_cache_hits_misses_and_rebuilds() {
    let c = cfg();
    let dir = scratch("traces");
    let script = InputScript::new(vec![
        (5, FrameInput::joy(Joy::PORT1_BUTTON)),
        (8, FrameInput::NONE),
    ]);
    let start = SyncPoint::PowerOn;

    let (made, outcome) = cached_trace_in(&dir, &c, &start, &script, 30).unwrap();
    assert_eq!(outcome, CacheOutcome::Miss);
    assert_eq!(made.frames.len(), 30);
    let last = made.memories().last().unwrap();
    assert_eq!(
        last.hash(),
        made.frames[29].memory_hash,
        "memory deltas do not replay"
    );

    let (again, outcome) = cached_trace_in(&dir, &c, &start, &script, 30).unwrap();
    assert_eq!(outcome, CacheOutcome::Hit);
    assert_eq!(again, made);

    // Another script, length or start is another key.
    let other = InputScript::new(vec![(5, FrameInput::joy(Joy::FIRE))]);
    assert_eq!(
        cached_trace_in(&dir, &c, &start, &other, 30).unwrap().1,
        CacheOutcome::Miss
    );
    assert_eq!(
        cached_trace_in(&dir, &c, &start, &script, 31).unwrap().1,
        CacheOutcome::Miss
    );
    // The commit, Kickstart and disks are in the key too (unit-tested in trace.rs; the owner has
    // one disk set and one build here): the key names this build.
    assert_ne!(
        trace_key("other-commit", "k", "d", &start, &script, 30),
        trace_key(BUILD, "k", "d", &start, &script, 30)
    );

    // A damaged file is rebuilt, and the rebuilt trace is the same run.
    let path = dir.join(format!("{}.trace", made.key));
    let mut bytes = std::fs::read(&path).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x55;
    std::fs::write(&path, &bytes).unwrap();
    let (rebuilt, outcome) = cached_trace_in(&dir, &c, &start, &script, 30).unwrap();
    assert!(matches!(outcome, CacheOutcome::Rebuilt(_)), "{outcome:?}");
    assert_eq!(rebuilt, made);
    std::fs::write(&path, &bytes[..100]).unwrap();
    assert!(matches!(
        cached_trace_in(&dir, &c, &start, &script, 30).unwrap().1,
        CacheOutcome::Rebuilt(_)
    ));
    assert_eq!(
        cached_trace_in(&dir, &c, &start, &script, 30).unwrap().1,
        CacheOutcome::Hit
    );

    // A snapshot start: the trace continues from the snapshot's frame.
    let mut a = Amiga::power_on(&c).unwrap();
    for _ in 0..12 {
        a.step(FrameInput::NONE).unwrap();
    }
    let snap = SyncPoint::Snapshot(a.snapshot());
    let (from_snap, outcome) = cached_trace_in(&dir, &c, &snap, &InputScript::idle(), 5).unwrap();
    assert_eq!(outcome, CacheOutcome::Miss);
    let direct = a.step(FrameInput::NONE).unwrap();
    assert_eq!(from_snap.frames[0].capture, direct);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_disks_and_rom_are_opened_read_only() {
    use std::os::unix::fs::MetadataExt;
    let c = cfg();
    let files: Vec<PathBuf> = std::iter::once(c.kickstart.clone())
        .chain(c.disks.iter().cloned())
        .collect();
    let stat = |p: &PathBuf| {
        let m = std::fs::metadata(p).unwrap();
        (
            m.mode(),
            m.len(),
            m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        )
    };
    let listing = |p: &PathBuf| {
        let mut names: Vec<_> = std::fs::read_dir(p.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .filter(|n| !n.to_string_lossy().starts_with('.'))
            .collect();
        names.sort();
        names
    };
    let before: Vec<_> = files.iter().map(|p| (stat(p), listing(p))).collect();
    {
        let mut a = Amiga::power_on(&c).unwrap();
        for f in 0..300 {
            a.step(boot_input(f)).unwrap();
        }
        a.insert_disk(0, DiskSlot::D2);
        for _ in 0..50 {
            a.step(FrameInput::NONE).unwrap();
        }
    }
    let after: Vec<_> = files.iter().map(|p| (stat(p), listing(p))).collect();
    for ((p, b), a) in files.iter().zip(&before).zip(&after) {
        assert_eq!(
            b.0,
            a.0,
            "{}: mode, size or modification time changed",
            p.display()
        );
        assert_eq!(
            b.1,
            a.1,
            "a file appeared or vanished beside {}",
            p.display()
        );
    }
}
