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

use fd_conform::Case;
use fd_conform::reading::{EngineKind, run_mounted};
use fd_conform::translated_engine::{case_start, frame_bar};
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
