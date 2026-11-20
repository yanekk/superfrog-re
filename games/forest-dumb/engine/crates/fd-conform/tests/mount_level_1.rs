//! Level 1's hand engine mounted into the translated program (translator T36, DESIGN §2.9):
//! `game_frame` (`h0+$1fec`) run by `fd-translated`'s frame mount, through `--mount level-1`.
//!
//! Full (the disks, Kickstart and the generated tree): every case that passes mounted today still
//! does, the mount really replaces the translated `game_frame` in play, and (T55) its CLXDAT read
//! lands on the translated program's line. The mount spends the original's time by walking each
//! routine's path through the original's instructions (T56, T58, `fd-translated/src/seam/rules.rs`);
//! the one case that differs mounted differs on the translated program alone. A no-disk check keeps
//! every case in one of the two lists.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use fd_conform::reading::{EngineKind, run_mounted};
use fd_conform::translated_engine::{case_start, frame_bar};
use fd_conform::{Case, Outcome};
use fd_translated::{Control, RUN_STACK, Start, TranslatedGame};
use pcode_machine::{Disks, Input};
use pcode_rt::{Image, Regs, Stop};

/// Cases that pass on the translated program with level 1 mounted (T36, 2026-11-19; the eight
/// after `world-6-start`, T55, 2026-11-19; the ten after `enemy-route`, T58, 2026-11-20).
const PASS_MOUNTED: [&str; 49] = [
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
    "camera-right-left",
    "character-edge-pit",
    "character-run-slide-buttons",
    "collect-ledge",
    "collect-return",
    "edge-latch-route",
    "enemy-hop",
    "enemy-route",
    "camera-bottom-right",
    "contact-route",
    "death-spikes",
    "death-water",
    "enemy-contact",
    "enemy-kind4",
    "mechanic-spring",
    "mechanic-vanish",
    "route-W1-1",
    "route-both-W1-1",
];

/// Cases that differ mounted, not the mount's doing: `power-on-level-1` differs on the translated
/// program alone from frame 6301 (T53; measured again 2026-11-20, T58).
const DIFFER: [&str; 1] = ["power-on-level-1"];

/// The level 1 cases that hear a sound effect on the original whose Paula writes are the
/// original's in order with level 1 mounted, and the effects each hears (T57, 2026-11-19, measured
/// again on T55's mount time; the three T56's walk added, T59, and both routes with T58's,
/// 2026-11-20): the mount makes each effect's writes as the hand engine starts it (`seam::frame`,
/// `Sound`, the character's `Scratch::hw`). `run_with`, which the hand engine's case tests use,
/// refuses a case comparing writes, so `writes` is turned on here, not in the case files.
const HEARS: [(&str, &str); 25] = [
    ("camera-bottom-right", "item, stomp, hurt"),
    ("camera-right-left", "stomp, hurt"),
    ("character-edge-pit", "stomp, hurt"),
    ("character-edge-slope", "slide, hurt"),
    ("character-edge-wall", "slide"),
    ("character-run-landing", "slide"),
    ("collect-ledge", "item, stomp"),
    ("collect-return", "item, stomp"),
    ("contact-route", "item, stomp, hurt"),
    ("death-pit", "hurt"),
    ("death-spikes", "hurt"),
    ("death-water", "item, stomp, slide, hurt"),
    ("edge-latch-route", "item, stomp, hurt"),
    ("enemy-contact", "item, stomp, hurt"),
    ("enemy-hop", "stomp, hurt"),
    ("enemy-kind4", "item, stomp, slide, hurt"),
    ("enemy-route", "item, stomp, hurt"),
    ("mechanic-spring", "item, spring, stomp, hurt"),
    ("mechanic-vanish", "item, stomp"),
    ("stretch-jump", "slide"),
    ("stretch-pit", "hurt"),
    ("stretch-slope", "slide, hurt"),
    ("stretch-wall", "slide"),
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
        .chain(DIFFER.iter())
        .map(|s| s.to_string())
        .collect();
    listed.sort();
    assert_eq!(listed, on_disk, "a case is in neither list, or in both");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; 49 cases, about 25 min"]
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
    // The mount takes the pass's time (T55), so the run may stop at a frame's end inside a pass
    // the hand engine has started and not returned from.
    assert!(
        hand as usize == mounted || hand as usize == mounted + 1,
        "mounted: every pass the hand engine's: {hand} hand passes, {mounted} returns"
    );
    assert!(hand >= 25, "mounted: {hand} hand passes");
}

/// CLXDAT as the first pass from contact-route's start reads it, level 1 mounted or not: the value
/// and VHPOSR's high byte (the line) read just after it.
static CLX: Mutex<Option<(u16, u8)>> = Mutex::new(None);

/// The bus inside `game_frame`, noting the first CLXDAT read.
struct NoteClx<'a>(&'a mut dyn pcode_rt::Bus);

impl pcode_rt::Bus for NoteClx<'_> {
    fn poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.0.poll(r, img)
    }
    fn insn(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.0.insn(r, img)
    }
    fn call_outside(&mut self, a: u32, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.0.call_outside(a, r, img)
    }
    fn sync(&mut self, r: &Regs) {
        self.0.sync(r)
    }
    fn mounted(&self, k: (u32, u32)) -> Option<pcode_rt::Mounted> {
        self.0.mounted(k)
    }
    fn mount_data(&self) -> Option<&dyn std::any::Any> {
        self.0.mount_data()
    }
    fn rb(&mut self, a: u32) -> u8 {
        self.0.rb(a)
    }
    fn rw(&mut self, a: u32) -> u16 {
        let v = self.0.rw(a);
        if a == 0xdf_f00e {
            let mut c = CLX.lock().unwrap();
            if c.is_none() {
                *c = Some((v, self.0.rb(0xdf_f006)));
            }
        }
        v
    }
    fn rl(&mut self, a: u32) -> u32 {
        self.0.rl(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.0.wb(a, v)
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.0.ww(a, v)
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.0.wl(a, v)
    }
    fn ww_leftover(&mut self, a: u32, v: u16) {
        self.0.ww_leftover(a, v)
    }
}

fn translated_frame(r: &mut Regs, bus: &mut dyn pcode_rt::Bus, img: &Image) -> Result<(), Stop> {
    let k = (0, 0x1fec);
    let f = fd_translated::tree::resume(k).expect("game_frame translated");
    f(k, r, &mut NoteClx(bus), img)
}

fn mounted_frame(r: &mut Regs, bus: &mut dyn pcode_rt::Bus, img: &Image) -> Result<(), Stop> {
    fd_translated::seam::frame::game_frame(r, &mut NoteClx(bus), img)
}

/// The first pass's CLXDAT read from contact-route's start, through `frame` mounted at
/// `game_frame` (the engine handed to it when `mounted`).
fn first_clx(frame: pcode_rt::Mounted, mounted: bool) -> (u16, u8) {
    let case = Case::load(&cases_dir().join("contact-route.toml")).unwrap();
    let start = case_start(&frame_bar(&case)).unwrap_or_else(|e| panic!("{e}"));
    let data = mounted.then(|| fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")));
    *CLX.lock().unwrap() = None;
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
            }
            g.mount((0, 0x1fec), frame);
            let mut n = 0;
            g.run(&mut |_| {
                n += 1;
                if n < 4 && CLX.lock().unwrap().is_none() {
                    Control::Next(Input::default())
                } else {
                    Control::Stop
                }
            });
        })
        .unwrap()
        .join()
        .unwrap();
    CLX.lock().unwrap().expect("a pass read CLXDAT")
}

/// How many lines the mount's CLXDAT read may fall from the translated program's on the pass
/// below: 2 with T55's cost rule (measured 2026-11-19, line 304 against 302); 0 since every routine
/// before it is walked by its path (T56, T58, 2026-11-20).
const CLX_LINES_OFF: u8 = 0;

/// T36's first mounted difference: contact-route's pass 2 (oracle frame 6304) read `$8060`, the
/// original `$8061`, the mount having read CLXDAT at the pass's start. With the mount's time
/// (T55, T56) it reads the original's bit, on the translated program's line ([`CLX_LINES_OFF`]).
#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; about 20 s"]
fn the_mounts_clxdat_read_lands_on_the_translated_programs_line() {
    generated();
    let translated = first_clx(translated_frame, false);
    let mounted = first_clx(mounted_frame, true);
    assert_eq!(translated.0, 0x8061, "the translated program's read");
    assert_eq!(mounted.0, translated.0, "CLXDAT mounted against translated");
    let off = mounted.1.wrapping_sub(translated.1);
    assert!(
        off <= CLX_LINES_OFF,
        "mounted read on line byte {}, translated {}",
        mounted.1,
        translated.1
    );
}

#[test]
fn the_cases_that_hear_an_effect_are_cases() {
    for (n, _) in &HEARS {
        assert!(
            cases_dir().join(format!("{n}.toml")).exists(),
            "{n}: no such case"
        );
    }
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; 25 cases, about 15 min"]
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
                    // Paula's channel alone; the state is `every_case_that_passes_mounted_still_does`'.
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
