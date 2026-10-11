//! T07's measurable claims about the original (docs/08, docs/09), each checked against the oracle
//! on the owner's disks. Addresses are the game's (`d1_c03`) hunk offsets, `h0+$x`, placed at the
//! RAM addresses the bootstrap gives them (docs/08 "Memory map").
//!
//! Three runs from power-on are shared between the tests:
//! - `play`: through the title menu into W1-1, then 3 000 frames of walking and jumping right.
//! - `attract`: left idle on the title until the attract-mode W1-1 demo has played a while.
//! - `late_click`: the crack intro clicked 100 frames later than in `play`, up to the RNG seeding.
//!
//! Measurements are printed (`cargo test -p sf-oracle --test exe_map_claims -- --nocapture`).

use std::sync::OnceLock;

use sf_disk::{DiskSlot, carve, decompress};
use sf_oracle::*;

/// Where the bootstrap puts `d1_c03`'s hunks (docs/08): hunk 0 in slow RAM, hunk 1 (BSS) after it,
/// hunks 2 and 3 in chip RAM.
const H0: u32 = 0xC0_0FA0;
const H1: u32 = 0xC4_92EC;
const H3: u32 = 0x77FC;

const fn h0(off: u32) -> u32 {
    H0 + off
}

const VBL_HANDLER: u32 = h0(0x11bf8);
/// `level_frame_loop`'s wait: `cmpi.w #2,h0+$9d8 / bmi` at h0+$aae..$ab8.
const LEVEL_WAIT: std::ops::Range<u32> = h0(0xaae)..h0(0xab8);
/// `wait_frame_raster` (VPOS $ff, then $2b): the title, menus, tables and tally pace on it.
const RASTER_WAIT: std::ops::Range<u32> = h0(0x73d0)..h0(0x73e6);
const LOGIC_FLAG: u32 = h0(0x9d6);
const VBL_COUNT: u32 = h0(0x9d8);
const LEVEL_STATE: u32 = h0(0xa06);
const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);
const RNG_STATE: u32 = h0(0x160d4);
const OBJ_COUNT: u32 = h0(0x1ba6a);
const OBJ_SLOTS: u32 = h0(0x1ba6c);
const FROG: u32 = h0(0x7d6);
const MAP_ROWS: u32 = h0(0x17334);
const FILE_TABLE: u32 = h0(0xcbfe);

/// What one frame boundary showed.
#[derive(Debug, Clone, Copy)]
struct Rec {
    /// Where the main program was when the frame ended: the PC the vertical-blank interrupt
    /// interrupted, or the PC itself when the interrupt had not been taken yet.
    main_pc: u32,
    logic_flag: u16,
    vbl_count: u16,
    level_state: i32,
    world: u32,
    level: u32,
    rng: u64,
    frog_x: u16,
    obj_count: u16,
    /// x of each object slot, 0 for a free slot.
    obj_x: [u16; 20],
    vbl_vector: u32,
}

impl Rec {
    fn in_level(&self) -> bool {
        self.logic_flag != 0 && self.level_state < 0
    }
}

struct Run {
    recs: Vec<Rec>,
    /// RAM at the first frame of W1-1 play (`play` only).
    level_start: Option<(u32, AmigaMemory)>,
}

fn rec(a: &Amiga) -> Rec {
    let r = a.cpu();
    let m = a.memory();
    let main_pc = if r.pc == VBL_HANDLER {
        m.long(r.ssp + 2).unwrap()
    } else {
        r.pc
    };
    let mut obj_x = [0u16; 20];
    for (i, x) in obj_x.iter_mut().enumerate() {
        let slot = OBJ_SLOTS + i as u32 * 0x7c;
        if m.long(slot + 0x40).unwrap() != 0 {
            *x = m.word(slot + 8).unwrap();
        }
    }
    Rec {
        main_pc,
        logic_flag: m.word(LOGIC_FLAG).unwrap(),
        vbl_count: m.word(VBL_COUNT).unwrap(),
        level_state: m.long(LEVEL_STATE).unwrap() as i32,
        world: m.long(WORLD).unwrap(),
        level: m.long(LEVEL).unwrap(),
        rng: (m.long(RNG_STATE).unwrap() as u64) << 32 | m.long(RNG_STATE + 4).unwrap() as u64,
        frog_x: m.word(FROG).unwrap(),
        obj_count: m.word(OBJ_COUNT).unwrap(),
        obj_x,
        vbl_vector: m.long(0x6c).unwrap(),
    }
}

fn run(script: InputScript, frames: u32, keep_level_start: bool) -> Run {
    let cfg = OracleConfig::locate(frames).unwrap_or_else(|e| panic!("{e}"));
    let mut a = Amiga::power_on(&cfg).unwrap();
    let mut recs = Vec::with_capacity(frames as usize);
    let mut level_start = None;
    for f in 0..frames {
        a.step(script.input_at(f)).unwrap();
        let r = rec(&a);
        if keep_level_start && level_start.is_none() && r.in_level() && r.world == 1 && r.level == 1
        {
            level_start = Some((f, a.memory()));
        }
        recs.push(r);
    }
    Run { recs, level_start }
}

fn joy(bits: u8) -> FrameInput {
    FrameInput::joy(Joy(bits))
}

/// The crack intro skips on a port-1 click (FINDINGS 2026-10-11); the title is up by 4 500.
fn boot(click: u32) -> Vec<(u32, FrameInput)> {
    vec![(click, joy(0x20)), (click + 5, FrameInput::NONE)]
}

/// First frame of the 3 000 frames of movement in `play`; W1-1 accepts input from ~7 290.
const MOVE_FROM: u32 = 7_300;
const PLAY_FRAMES: u32 = MOVE_FROM + 3_000;

/// Fire opens the title menu at 4 600 and starts the game at 4 700; W1-1 is playable by ~7 290.
/// Then right with jump (up) pulses: 10 frames right, 20 frames right+up, repeated. This walk
/// reaches the busiest W1-1 stretch found by reading the object list (15 live slots, docs/09).
fn play() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| {
        let mut s = boot(1_000);
        s.extend([
            (4_600, joy(0x10)),
            (4_605, FrameInput::NONE),
            (4_700, joy(0x10)),
            (4_705, FrameInput::NONE),
            (7_280, joy(0x09)),
        ]);
        let mut f = MOVE_FROM;
        while f < PLAY_FRAMES {
            s.push((f, joy(0x08)));
            s.push((f + 10, joy(0x09)));
            f += 30;
        }
        run(InputScript::new(s), PLAY_FRAMES, true)
    })
}

/// Idle from power-on: title, features and bonus tables, hall of talent, credits, then the
/// attract-mode W1-1 demo (from ~8 340).
const ATTRACT_FRAMES: u32 = 9_100;
const DEMO_FROM: u32 = 8_300;

fn attract() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| run(InputScript::new(boot(1_000)), ATTRACT_FRAMES, false))
}

/// A boundary where the 25 Hz pass was due (the counter reaches 2 at this vertical blank) but the
/// main loop was still busy with the previous one.
fn is_lag(r: &Rec) -> bool {
    r.in_level() && !LEVEL_WAIT.contains(&r.main_pc) && r.vbl_count >= 1
}

#[derive(Debug, Default)]
struct LagReport {
    level_frames: usize,
    /// Boundaries where the pass was still running but not yet late.
    spilled: usize,
    lag: usize,
    /// Highest VBL count seen while the main loop was busy: 3 means it was two ticks late (seen
    /// once in W1-1 play).
    max_count: u16,
}

fn lag_report(recs: &[Rec]) -> LagReport {
    let mut r = LagReport::default();
    for x in recs.iter().filter(|x| x.in_level()) {
        r.level_frames += 1;
        if !LEVEL_WAIT.contains(&x.main_pc) {
            if is_lag(x) {
                r.lag += 1;
            } else {
                r.spilled += 1;
            }
            r.max_count = r.max_count.max(x.vbl_count);
        }
    }
    r
}

/// docs/08 "Memory map": the game's hunk 0 is at 0xC00FA0, found by its position-independent
/// ATN! decompressor (h0+$ce58), and its level-3 vector is h0+$11bf8 from the title on.
#[test]
fn the_game_sits_where_the_map_says_and_owns_the_vertical_blank() {
    let run = play();
    let (_, mem) = run.level_start.as_ref().expect("W1-1 never started");
    let exe = chunk_bytes(DiskSlot::D1, 3);
    let code = hunk0(&exe);
    for k in 0..0x150u32 {
        assert_eq!(
            mem.byte(h0(0xce58) + k),
            Some(code[0xce58 + k as usize]),
            "atn_decompress byte {k:#x} is not at h0+$ce58 = {:#x}",
            h0(0xce58)
        );
    }
    for f in [4_500usize, 6_000, 8_000, PLAY_FRAMES as usize - 1] {
        assert_eq!(
            run.recs[f].vbl_vector, VBL_HANDLER,
            "frame {f}: level-3 vector"
        );
    }
}

/// Lag, title: the title loop paces on the raster wait and is parked in it at every frame
/// boundary, so it never takes more than one frame.
#[test]
fn the_title_never_lags() {
    let run = play();
    // Seen: the title (Superfrog in cape) from ~4 360; fire at 4 600 opens the menu.
    let window = &run.recs[4_400..4_600];
    let parked = window
        .iter()
        .filter(|r| RASTER_WAIT.contains(&r.main_pc))
        .count();
    println!(
        "title: {parked}/{} boundaries in the raster wait",
        window.len()
    );
    assert_eq!(parked, window.len());
    assert!(window.iter().all(|r| r.logic_flag == 0));
}

/// Lag, W1-1 played: the 25 Hz pass usually ends in the frame it starts, sometimes spills over
/// one frame boundary, and in busy stretches overruns (docs/08 "Lag"). The owner chose to feed
/// that schedule to the engine as an input (FINDINGS 2026-10-11).
#[test]
fn w1_1_play_lags_in_busy_stretches() {
    let run = play();
    let recs = &run.recs[MOVE_FROM as usize..];
    let r = lag_report(recs);
    let busiest = recs.iter().map(|x| x.obj_count).max().unwrap();
    let at = recs.iter().position(|x| x.obj_count == busiest).unwrap() + MOVE_FROM as usize;
    println!("W1-1 play: {r:?}; busiest: {busiest} object slots at frame {at}");
    assert!(r.level_frames > 2_000, "{r:?}");
    assert!(busiest >= 12, "busiest scene had only {busiest} live slots");
    assert!(r.lag > 0, "no lag in W1-1 play: {r:?}");
    assert!(
        r.lag * 20 < r.level_frames,
        "lag on over 5% of frames: {r:?}"
    );
}

/// Lag, attract mode: every title-side screen paces on a raster wait; the demo lags like play.
#[test]
fn attract_screens_never_lag_and_the_demo_does() {
    let run = attract();
    let demo = lag_report(&run.recs[DEMO_FROM as usize..]);
    println!("attract demo: {demo:?}");
    assert!(demo.level_frames > 500, "{demo:?}");
    assert!(demo.lag > 0, "{demo:?}");
    // Hall of talent and credits wait for raster line $c4 then $c5 (h0+$221e..$2232).
    let credits = h0(0x221e)..h0(0x2232);
    let window = &run.recs[6_300..7_200];
    assert!(
        window.iter().all(|r| credits.contains(&r.main_pc)),
        "credits: a boundary outside the line-$c4 wait"
    );
}

/// Interrupt work: the frog runs in the vertical-blank handler (50 Hz), the objects in the main
/// loop's pass (25 Hz). Walking right, the frog moves on consecutive frames; no object moves on two
/// consecutive frames unless the pass was late.
#[test]
fn the_frog_moves_every_frame_and_objects_every_other() {
    let run = play();
    let recs = &run.recs[MOVE_FROM as usize..];
    let frog_pairs = recs
        .windows(3)
        .filter(|w| w.iter().all(|r| r.in_level()))
        .filter(|w| w[0].frog_x != w[1].frog_x && w[1].frog_x != w[2].frog_x)
        .count();
    let moved = |a: &Rec, b: &Rec| {
        (0..20).any(|i| a.obj_x[i] != 0 && b.obj_x[i] != 0 && a.obj_x[i] != b.obj_x[i])
    };
    let mut both = 0;
    let mut objects_moved = 0;
    for w in recs.windows(3) {
        if !w.iter().all(|r| r.in_level()) {
            continue;
        }
        let m1 = moved(&w[0], &w[1]);
        let m2 = moved(&w[1], &w[2]);
        objects_moved += m2 as usize;
        if m1 && m2 && !w.iter().any(is_lag) && !w.iter().any(|r| !LEVEL_WAIT.contains(&r.main_pc))
        {
            both += 1;
        }
    }
    println!(
        "frog moved on {frog_pairs} consecutive-frame pairs; objects moved on {objects_moved} frames, on two in a row {both} times"
    );
    assert!(
        frog_pairs > 500,
        "frog moved on consecutive frames only {frog_pairs} times"
    );
    assert!(objects_moved > 500);
    assert_eq!(
        both, 0,
        "objects moved on consecutive frames with the main loop on time"
    );
}

/// RNG: seeded once at start-up (h0+$7722) from CIA-B's time-of-day counter. Clicking through the
/// crack intro 100 frames later shifts the whole run by 100 frames, and the seed alone differs.
#[test]
fn the_rng_seed_comes_from_the_clock() {
    let play = play();
    let late = run(InputScript::new(boot(1_100)), 3_501, false);
    let seeded_at = |recs: &[Rec]| {
        // The RNG state is first set when d1_c03 is in place: after ~3 300 frames.
        (3_000..recs.len())
            .find(|&f| recs[f].vbl_vector == VBL_HANDLER && recs[f].rng != recs[f - 1].rng)
            .unwrap_or_else(|| panic!("no RNG seeding seen"))
    };
    let a = seeded_at(&play.recs);
    let b = seeded_at(&late.recs);
    println!(
        "seeded at frame {a}: {:016x}; clicked 100 later, at frame {b}: {:016x}",
        play.recs[a].rng, late.recs[b].rng
    );
    assert_eq!(b, a + 100);
    assert_ne!(play.recs[a].rng, late.recs[b].rng);
    // Nothing else the game keeps differs (only the stack and the bootstrap's scratch below it).
    let ra = &play.recs[a];
    let rb = &late.recs[b];
    assert_eq!(
        (ra.world, ra.level, ra.level_state),
        (rb.world, rb.level, rb.level_state)
    );
}

/// Stragglers (docs/03): the game's file table asks for one sector more than the carver keeps for
/// d1_c16, d2_c39 and d4_c16; after W1's load, d1_c16 (World 1 music) in RAM equals sf-disk's
/// output with the next sector appended, apart from the sample pointers the replayer relocates.
/// d2_c39 (bonus-level loading picture) and d4_c16 (W6-3) load only late in a game: not reached.
#[test]
fn the_stragglers_are_what_the_game_loads() {
    let run = play();
    let (frame, mem) = run.level_start.as_ref().expect("W1-1 never started");
    for (slot, index, entry) in [
        (DiskSlot::D1, 16, 22u32),
        (DiskSlot::D2, 39, 85),
        (DiskSlot::D4, 16, 8),
    ] {
        let c = chunk(slot, index);
        let e = FILE_TABLE + entry * 6;
        let (sector, count) = (mem.word(e).unwrap() as u32, mem.word(e + 2).unwrap() as u32);
        assert_eq!(sector * 512, c.offset, "file-table entry {entry}");
        assert!(
            (sector + count) * 512 > c.offset + c.raw.len() as u32,
            "entry {entry}: the game reads no further than the carver"
        );
    }
    let music = decompress(&extended(DiskSlot::D1, 16)).unwrap();
    let at = 0x73ef6u32;
    let got: Vec<u8> = (0..music.len() as u32)
        .map(|k| mem.byte(at + k).unwrap())
        .collect();
    assert_eq!(&got[..4], &music[..4], "module tag");
    let differ = (0xc0..music.len()).filter(|&k| got[k] != music[k]).count();
    let header = (0..0xc0).filter(|&k| got[k] != music[k]).count();
    println!(
        "d1_c16 at {at:#x} (frame {frame}): body differs in {differ} bytes, sample table in {header}"
    );
    assert_eq!(differ, 0);
}

/// Level composition: at the first frame of W1-1, the world's files from the file table are in
/// RAM where the loader put them, and the T7MP BODY is the map as `tile << 6 | flags`.
#[test]
fn a_level_is_the_world_files_plus_its_map() {
    let run = play();
    let (_, mem) = run.level_start.as_ref().expect("W1-1 never started");
    for (what, index, at) in [
        ("L1FX sound effects", 10, H3 + 0x64c70),
        ("L1BM tiles", 18, H1),
        ("L1BO bobs", 19, H3),
    ] {
        let data = decompress(&chunk(DiskSlot::D1, index).raw).unwrap();
        let same = (0..data.len() as u32)
            .filter(|&k| mem.byte(at + k) == Some(data[k as usize]))
            .count();
        assert_eq!(same, data.len(), "{what} (d1_c{index:02}) at {at:#x}");
    }
    let level = decompress(&chunk(DiskSlot::D1, 21).raw).unwrap();
    let body = &level[level.windows(4).position(|w| w == b"BODY").unwrap() + 8..][..300 * 84 * 2];
    let rows = mem.long(MAP_ROWS).unwrap() + 0x260;
    let mut same = 0;
    for r in 0..84u32 {
        for c in 0..300u32 {
            let k = ((r * 300 + c) * 2) as usize;
            let cell = u16::from_be_bytes([body[k], body[k + 1]]);
            if mem.word(rows + r * 0x260 + c * 2) == Some((cell & 0x3ff) << 6 | cell >> 10) {
                same += 1;
            }
        }
    }
    println!("W1-1 map: {same}/25200 cells equal tile << 6 | flags");
    // The rest are animated tiles and object markers the start-up scan has already cleared.
    assert!(same * 10 > 25_200 * 7, "{same}");
}

// --- the disks, through sf-disk ---

fn disks() -> &'static sf_disk::DiskSet {
    static SET: OnceLock<sf_disk::DiskSet> = OnceLock::new();
    SET.get_or_init(|| {
        let cfg = OracleConfig::locate(1).unwrap_or_else(|e| panic!("{e}"));
        let offered: Vec<sf_disk::Offered> = cfg
            .disks
            .iter()
            .map(|p| sf_disk::Offered {
                name: p.display().to_string(),
                bytes: read_disk(p),
            })
            .collect();
        sf_disk::identify(&offered).unwrap_or_else(|e| panic!("{e:?}"))
    })
}

fn read_disk(p: &std::path::Path) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn chunk(slot: DiskSlot, index: u16) -> sf_disk::Chunk {
    carve(disks().disk(slot))
        .into_iter()
        .find(|c| c.index == index)
        .unwrap()
}

fn chunk_bytes(slot: DiskSlot, index: u16) -> Vec<u8> {
    decompress(&chunk(slot, index).raw).unwrap()
}

/// A straggler with the sector after its carve appended (FINDINGS 2026-10-11).
fn extended(slot: DiskSlot, index: u16) -> Vec<u8> {
    let c = chunk(slot, index);
    let adf = &disks().disk(slot).bytes;
    let end = (c.offset as usize + c.raw.len() + 512).min(adf.len());
    adf[c.offset as usize..end].to_vec()
}

/// Hunk 0's bytes of a hunk executable (HUNK_HEADER, no names, then HUNK_CODE).
fn hunk0(exe: &[u8]) -> &[u8] {
    let be = |at: usize| u32::from_be_bytes(exe[at..at + 4].try_into().unwrap()) as usize;
    assert_eq!(be(0), 0x3f3);
    let (first, last) = (be(12), be(16));
    let code = 20 + 4 * (last - first + 1);
    assert_eq!(be(code) & 0x3fff_ffff, 0x3e9);
    &exe[code + 8..code + 8 + 4 * be(code + 4)]
}
