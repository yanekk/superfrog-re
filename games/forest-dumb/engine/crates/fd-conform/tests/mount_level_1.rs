//! Level 1's hand engine mounted into the translated program (translator T36, DESIGN §2.9):
//! `game_frame` (`h0+$1fec`) run by `fd-translated`'s frame mount, through `--mount level-1`.
//!
//! Full (the disks, Kickstart and the generated tree): every case that passes mounted today still
//! does, and the mount really replaces the translated `game_frame` in play. The cases that differ
//! mounted are T55's (mount-timing): the mount takes no machine time, so CLXDAT, read by
//! `char_hit_check`, is read before the beam has drawn the lines the original's read sees. A no-disk
//! check keeps every case in one of the two lists.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use fd_conform::reading::{EngineKind, run_mounted};
use fd_conform::translated_engine::{case_start, frame_bar};
use fd_conform::{Case, Outcome};
use fd_translated::{Control, RUN_STACK, Start, TranslatedGame};
use pcode_machine::{Disks, Input};

/// Cases that pass on the translated program with level 1 mounted (T36, 2026-11-19).
const PASS_MOUNTED: [&str; 32] = [
    "camera-top-left",
    "character-edge-left",
    "character-edge-slope",
    "character-edge-top",
    "character-edge-wall",
    "character-run-jumps",
    "character-run-landing",
    "character-run-left",
    "character-run-right-jump",
    "character-run-slide",
    "death-countdown",
    "death-esc",
    "death-pit",
    "death-spikes",
    "first-case",
    "level-end",
    "picture-first-frame",
    "picture-scroll",
    "power-on-title",
    "stretch-jump",
    "stretch-pit",
    "stretch-run",
    "stretch-scroll",
    "stretch-slope",
    "stretch-wall",
    "stub-level",
    "w1-idle",
    "world-2-start",
    "world-3-start",
    "world-4-start",
    "world-5-start",
    "world-6-start",
];

/// Cases that differ mounted, T55's to make pass (or, for the last three, not the seam's: they fail
/// on the translated program alone, `power-on-level-1` from frame 6301 (T53) and the two routes
/// from pass 4161 (T54)).
const T55: [&str; 18] = [
    "camera-bottom-right",
    "camera-right-left",
    "character-edge-pit",
    "character-run-slide-buttons",
    "collect-ledge",
    "collect-return",
    "contact-route",
    "death-water",
    "edge-latch-route",
    "enemy-contact",
    "enemy-hop",
    "enemy-kind4",
    "enemy-route",
    "mechanic-spring",
    "mechanic-vanish",
    "power-on-level-1",
    "route-W1-1",
    "route-both-W1-1",
];

/// The level 1 cases that hear a sound effect on the original whose Paula writes are the
/// original's in order with level 1 mounted, and the effects each hears (T57, 2026-11-19): the
/// mount makes each effect's writes as the hand engine starts it (`seam::frame`, `Sound`, the
/// character's `Scratch::hw`). `run_with`, which the hand engine's case tests use, refuses a case
/// comparing writes, so `writes` is turned on here, not in the case files.
const HEARS: [(&str, &str); 11] = [
    ("character-edge-slope", "slide, hurt"),
    ("character-edge-wall", "slide"),
    ("character-run-landing", "slide"),
    ("collect-ledge", "item, stomp"),
    ("collect-return", "item, stomp"),
    ("death-pit", "hurt"),
    ("death-spikes", "hurt"),
    ("stretch-jump", "slide"),
    ("stretch-pit", "hurt"),
    ("stretch-slope", "slide, hurt"),
    ("stretch-wall", "slide"),
];

/// The cases that hear an effect whose Paula writes differ mounted, T55's: the mount takes no
/// machine time, so a pass ends a frame early and the next frame's effect falls in the next pass
/// (`mechanic-spring` from pass 492: a stomp in pass 493, the original's in 492). Every one fails
/// mounted on its state as well (`T55` above). Named, not run.
const HEARS_T55: [(&str, &str); 14] = [
    ("camera-bottom-right", "item, stomp, hurt"),
    ("camera-right-left", "stomp, hurt"),
    ("character-edge-pit", "stomp, hurt"),
    ("contact-route", "item, stomp, hurt"),
    ("death-water", "item, stomp, slide, hurt"),
    ("edge-latch-route", "item, stomp, hurt"),
    ("enemy-contact", "item, stomp, hurt"),
    ("enemy-hop", "stomp, hurt"),
    ("enemy-kind4", "item, stomp, slide, hurt"),
    ("enemy-route", "item, stomp, hurt"),
    ("mechanic-spring", "item, spring, stomp, hurt"),
    ("mechanic-vanish", "item, stomp"),
    ("route-W1-1", "item, stomp, slide"),
    ("route-both-W1-1", "item, stomp, slide"),
];

fn cases_dir() -> std::path::PathBuf {
    fd_conform::conform_dir().join("cases")
}

fn generated() {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
}

#[test]
fn every_case_is_in_one_list() {
    let mut on_disk: Vec<String> = std::fs::read_dir(cases_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = PASS_MOUNTED
        .iter()
        .chain(T55.iter())
        .map(|s| s.to_string())
        .collect();
    listed.sort();
    assert_eq!(listed, on_disk, "a case is in neither list, or in both");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; 32 cases, about 10 min"]
fn every_case_that_passes_mounted_still_does() {
    generated();
    let cases: Vec<Case> = PASS_MOUNTED
        .iter()
        .map(|n| Case::load(&cases_dir().join(format!("{n}.toml"))).unwrap())
        .collect();
    let next = AtomicUsize::new(0);
    let failed = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers.min(cases.len()) {
            s.spawn(|| {
                while let Some(c) = cases.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let line = match run_mounted(c, EngineKind::Translated, true) {
                        Ok(v) if v.passed() => None,
                        Ok(v) => Some(format!(
                            "{}: {}",
                            c.name,
                            v.to_string().lines().next().unwrap_or("")
                        )),
                        Err(e) => Some(format!("{}: not run: {e}", c.name)),
                    };
                    if let Some(l) = line {
                        failed.lock().unwrap().push(l);
                    }
                }
            });
        }
    });
    let failed = failed.into_inner().unwrap();
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// Over `frames` frames from w1-idle's start, level 1 mounted or not: how often `game_frame`
/// returned (a watched return counts a mounted routine too) and how many passes the hand engine
/// played.
fn game_frame_runs(mounted: bool, frames: u32) -> (usize, u32) {
    let case = Case::load(&cases_dir().join("w1-idle.toml")).unwrap();
    let start = case_start(&frame_bar(&case)).unwrap_or_else(|e| panic!("{e}"));
    let data = mounted.then(|| fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")));
    std::thread::Builder::new()
        .stack_size(RUN_STACK)
        .spawn(move || {
            let mut g = TranslatedGame::new(Start::Snapshot {
                mem: &start.mem,
                regs: start.regs,
                custom: &start.custom,
                disks: Disks {
                    df0: start.disk2.clone(),
                },
            });
            g.held_at_start(&Input::from(start.input));
            if let Some(d) = data {
                g.mount_level_1(d);
                let at: Vec<_> = g.seam().addresses().collect();
                assert_eq!(at, [(0, 0x1fec)], "level 1's mount is the frame");
            }
            g.watch_returns(&[(0, 0x1fec)]);
            let (mut n, mut returns) = (0, 0);
            let stop = g.run(&mut |out| {
                returns += out.returns.len();
                n += 1;
                if n < frames {
                    Control::Next(Input::default())
                } else {
                    Control::Stop
                }
            });
            assert_eq!(stop, None, "the program stopped");
            (returns, g.level_1().map_or(0, |l| l.passes()))
        })
        .unwrap()
        .join()
        .unwrap()
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; the original from power-on twice"]
fn the_mount_replaces_the_translated_game_frame_in_play() {
    generated();
    let (alone, hand) = game_frame_runs(false, 30);
    assert!(
        alone >= 25 && hand == 0,
        "translated alone: {alone} returns, {hand} hand passes"
    );
    let (mounted, hand) = game_frame_runs(true, 30);
    assert_eq!(
        hand as usize, mounted,
        "mounted: every pass the hand engine's"
    );
    assert!(hand >= 25, "mounted: {hand} hand passes");
}

#[test]
fn the_cases_that_hear_an_effect_are_cases_and_the_t55_ones_fail_mounted() {
    let on_disk = |n: &str| cases_dir().join(format!("{n}.toml")).exists();
    for (n, _) in HEARS.iter().chain(&HEARS_T55) {
        assert!(on_disk(n), "{n}: no such case");
    }
    for (n, _) in &HEARS_T55 {
        assert!(
            T55.contains(n),
            "{n}: hears an effect late mounted but passes on its state"
        );
    }
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; 11 cases, about 4 min"]
fn the_effects_each_case_hears_are_the_originals_paula_writes_in_order_mounted() {
    generated();
    let cases: Vec<Case> = HEARS
        .iter()
        .map(|(n, _)| {
            let mut c = Case::load(&cases_dir().join(format!("{n}.toml"))).unwrap();
            c.compare.writes = true;
            c
        })
        .collect();
    let next = AtomicUsize::new(0);
    let failed = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..workers.min(cases.len()) {
            s.spawn(|| {
                while let Some(c) = cases.get(next.fetch_add(1, Ordering::Relaxed)) {
                    // Paula's channel alone: collect-ledge and collect-return differ on their state
                    // (T55's) while every effect they hear is in order.
                    let line = match run_mounted(c, EngineKind::Translated, true) {
                        Ok(v) => v
                            .firsts
                            .iter()
                            .find(|o| matches!(o, Outcome::WritesDiffer { .. }))
                            .map(|o| format!("{}: {o}", c.name))
                            .or_else(|| {
                                (v.writes == 0)
                                    .then(|| format!("{}: no Paula write compared", c.name))
                            }),
                        Err(e) => Some(format!("{}: not run: {e}", c.name)),
                    };
                    if let Some(l) = line {
                        failed.lock().unwrap().push(l);
                    }
                }
            });
        }
    });
    let failed = failed.into_inner().unwrap();
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
