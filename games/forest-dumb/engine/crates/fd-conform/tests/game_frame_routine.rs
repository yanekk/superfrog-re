//! `game_frame` (`h0+$1fec`) whole against the original (plans/forest-dumb-level-1 T63): the engine's
//! pass, `fd_core::level::game_frame_with` through `Game::game_frame_on`, on the routine rig over
//! 10 000 states of T39's generator, which lays the level sync point's RAM and pokes the play state
//! `game_frame`'s own unit reads; the order of its calls against `re/translate.toml`; both ways of its
//! `paused` branch; and its hits on the original in a case's coverage report.
//!
//! Each check transplants a [`Game`] from the state's memory (`Transplant`, every `LAYOUT` variable,
//! the creature, cell and display state, and what the callees keep outside `LAYOUT`), runs the pass
//! with a [`Pass`] over that memory, and stores what the engine changed back. The pass's
//! outside world goes to the memory as the original's does: the object ports and the creature
//! drawing read it in place and turn each blit and sound into the original's custom-chip writes, in
//! order, which the judge's blitter runs into the play screen; the object map and the collision
//! plane they write are read back into the typed state for the callees after them. Those writes are
//! the porting tasks' adapters (`collect_routines.rs`, `mechanic_routines.rs`,
//! `creature_drawing_routines.rs`), so a blit is checked against the same register order here as
//! there. No register is compared: `level3_handler` saves and restores them all around the call.

use std::cell::Cell;
use std::path::Path;
use std::sync::OnceLock;

use amiga_oracle::FrameInput;
use fd_conform::generators::rest::SLOW_BASE;
use fd_conform::generators::{self, h, rest};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_conform::{Case, Outcome, game_data, run};
use fd_core::level::collision::MAP;
use fd_core::level::redraw::{Blit, OBJECT_MAP};
use fd_core::level::{LevelCtx, Pass};
use fd_core::objects::{Io, MapObjects, Sound};
use fd_core::render::objects::{CreatureBlit, Creatures, DrawIo};
use fd_core::render::palette::Fade;
use fd_core::render::playfield::{SCREEN as SCREEN_AT, Scroll};
use fd_core::render::status::STATUS;
use fd_core::render::{COLOURS, objects::CharSprite};
use fd_core::{Game, LAYOUT, Mem, PORTED, Scratch, Start, State, Transplant};

const CASES: usize = 10_000;
const ENTRY: u32 = 0x1fec;
/// `tst.w paused; beq.w`: the branch `game_frame`'s own code turns on.
const PAUSED_BRANCH: u32 = 0x200a;
const PLAY_KEYS: u32 = 0x2104;

/// The custom chips the pass reads: the raster, JOY1DAT and CLXDAT.
const VHPOSR: u32 = 0xdf_f006;
const JOY1DAT: u32 = 0xdf_f00c;
const CLXDAT: u32 = 0xdf_f00e;
/// The play screen, the tiles, the samples and `_ELEMENTY`, where the blits and sounds point.
const SCREEN: u32 = h(3, 0x398);
const TILES: u32 = h(3, 0x1_3540);
const ELEMENTS: u32 = h(4, 0);
const ELEMENT_MASKS: u32 = h(4, 0x1900);
const ITEM_SAMPLE: u32 = h(4, 0x1_2acc);
const TIME_SAMPLE: u32 = h(4, 0x1_3a62);
const SPRING_SAMPLE: u32 = h(4, 0xb548);
/// `input_log`, a byte a pass at `input_log_index`, and where it stops.
const INPUT_LOG: u32 = 0x3_4c28;
const LOG_FULL: u32 = 0x2ee0;

/// `h0+o`, wrapping: a wild coordinate's offset can be any long.
fn h0(o: u32) -> u32 {
    h(0, 0).wrapping_add(o)
}

/// The engine at W1-1's start, built once; each state transplants a clone.
fn base_game() -> &'static Game {
    static GAME: OnceLock<Game> = OnceLock::new();
    GAME.get_or_init(|| Game::new(game_data().unwrap_or_else(|e| panic!("{e}")), Start::Level1))
}

/// The state's memory as the main program's hunks see it.
struct CpuMem<'a>(&'a Cpu);

impl Mem for CpuMem<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        // The play screen and the status bar the transplant takes are picture state the rig's pass
        // never reads (its blits run in this memory), and reading them is most of a state's time.
        if (hunk, offset) == (3, SCREEN_AT) || (hunk, offset) == (4, STATUS) {
            return vec![0; len];
        }
        let at = h(hunk as usize, offset);
        (0..len as u32).map(|k| self.0.mem.ram(at + k)).collect()
    }
}

/// The sync point's RAM as T39's generator lays it, with `paused` set to `paused`.
struct SyncMem {
    paused: u16,
}

impl Mem for SyncMem {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        let (chip, slow) = rest::sync_ram();
        let at = h(hunk as usize, offset);
        let paused = h(0, 0x2100);
        (0..len as u32)
            .map(|k| {
                let a = at + k;
                if a == paused || a == paused + 1 {
                    return self.paused.to_be_bytes()[(a - paused) as usize];
                }
                let a = a as usize;
                if a >= SLOW_BASE {
                    slow[a - SLOW_BASE]
                } else {
                    chip[a]
                }
            })
            .collect()
    }
}

// --- the outside world on the rig ----------------------------------------------------------------

/// The object ports' memory, as the routine last called reaches it.
struct ObjIo<'a> {
    cpu: &'a mut Cpu,
    /// The routine running: `h0_21c0` and `h0_62b6` set the blitter's modulos and masks for their
    /// first cell only, `h0_29ca` (under `char_object_cells` and `h0_2dd6`) for every one.
    at: u32,
    cells: u32,
    /// The object map bytes and collision longs written, for the typed state after.
    map: Vec<(u32, u8)>,
    plane: Vec<(u32, u32)>,
}

impl Io for ObjIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        self.cpu.mem.rb(h0(a))
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.cpu.mem.wb(h0(a), v);
        self.map.push((a, v));
    }
    fn blit(&mut self, b: Blit) {
        let m = &mut self.cpu.mem;
        match b {
            Blit::Cell { src, dst } => {
                let every = matches!(self.at, 0x2698 | 0x2dd6);
                m.wl(0xdf_f054, SCREEN.wrapping_add(dst));
                m.wl(0xdf_f050, TILES.wrapping_add(src));
                if every || self.cells == 0 {
                    m.ww(0xdf_f066, 0x2c);
                    m.ww(0xdf_f064, 0);
                    m.wl(0xdf_f044, 0xffff_ffff);
                    m.ww(0xdf_f040, 0x09f0);
                    m.ww(0xdf_f042, 0);
                }
                m.ww(0xdf_f058, 0x1401);
                self.cells += 1;
            }
            Blit::Picture { src, modulo, dst } => {
                m.wl(0xdf_f054, SCREEN.wrapping_add(dst));
                m.wl(0xdf_f050, h(4, 0).wrapping_add(src));
                m.ww(0xdf_f066, 0x2a);
                m.ww(0xdf_f064, modulo);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x09f0);
                m.ww(0xdf_f042, 0);
                m.ww(0xdf_f058, 0x1402);
            }
            Blit::Element { frame, tile, dst } => {
                m.ww(0xdf_f066, 0x2c);
                m.ww(0xdf_f064, 0x26);
                m.ww(0xdf_f062, 0x26);
                m.ww(0xdf_f060, 0);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x0fca);
                m.ww(0xdf_f042, 0);
                m.wl(0xdf_f054, SCREEN.wrapping_add(dst));
                m.wl(0xdf_f050, ELEMENT_MASKS.wrapping_add(2 * frame));
                m.wl(0xdf_f04c, ELEMENTS.wrapping_add(2 * frame));
                m.wl(0xdf_f048, TILES.wrapping_add(tile));
                m.ww(0xdf_f058, 0x1401);
            }
            Blit::Tile { .. } | Blit::Bob { .. } => {
                unreachable!("only the edge redraws (T55) start tile and bob blits")
            }
        }
    }
    fn sound(&mut self, s: Sound) {
        // DMACON off for the channel, its location, length, period and volume, DMACON on.
        let (ch, sample, len, period) = match s {
            Sound::Item => (2u32, ITEM_SAMPLE, 0x7cb, 0x165),
            Sound::Time => (3, TIME_SAMPLE, 0x1c4f, 0x1ff),
            Sound::Spring => (1, SPRING_SAMPLE, 0x9c4, 0x1bf),
        };
        let m = &mut self.cpu.mem;
        let aud = 0xdf_f0a0 + 0x10 * ch;
        m.ww(0xdf_f096, 1 << ch);
        m.wl(aud, sample);
        m.ww(aud + 4, len);
        m.ww(aud + 6, period);
        m.ww(aud + 8, 0x3f);
        m.ww(0xdf_f096, 0x8200 | 1 << ch);
    }
    fn collision(&mut self, at: u32, v: u32) {
        self.cpu.mem.wl(h(3, MAP).wrapping_add(at), v);
        self.plane.push((at, v));
    }
}

/// The creature drawing's memory.
struct DrawCpu<'a>(&'a mut Cpu);

impl DrawIo for DrawCpu<'_> {
    fn rb(&self, a: u32) -> u8 {
        self.0.mem.rb(h0(a))
    }
    fn blit(&mut self, b: CreatureBlit) {
        let m = &mut self.0.mem;
        match b {
            CreatureBlit::CutSetup => {
                m.ww(0xdf_f066, 0x28);
                m.ww(0xdf_f064, 0x22);
                m.ww(0xdf_f062, 0x22);
                m.ww(0xdf_f060, 0x28);
                m.wl(0xdf_f044, 0xffff_0000);
            }
            CreatureBlit::Restore { src, dst, first } => {
                m.wl(0xdf_f054, dst);
                m.wl(0xdf_f050, src);
                if first {
                    m.ww(0xdf_f066, 0x2c);
                    m.ww(0xdf_f064, 0);
                    m.wl(0xdf_f044, 0xffff_ffff);
                    m.ww(0xdf_f040, 0x09f0);
                    m.ww(0xdf_f042, 0);
                }
                m.ww(0xdf_f058, 0x1401);
            }
            CreatureBlit::Cut {
                mask,
                picture,
                dst,
                con0,
                con1,
                size,
            } => {
                m.wl(0xdf_f054, dst);
                m.wl(0xdf_f050, mask);
                m.wl(0xdf_f04c, picture);
                m.wl(0xdf_f048, dst);
                m.ww(0xdf_f040, con0);
                m.ww(0xdf_f042, con1);
                m.ww(0xdf_f058, size);
            }
        }
    }
}

/// [`Pass`] over the state's memory, recording the calls it is told of.
struct RigPass<'a> {
    cpu: &'a mut Cpu,
    at: u32,
    calls: Vec<u32>,
    /// What a wrong port leaves out, for the tests that show the rig catches it.
    mutant: Mutant,
}

/// A wrong pass the rig must catch.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mutant {
    None,
    /// `h0+$8dfa`'s raster copy left out.
    NoRaster,
    /// `h0_9754`'s copper pointers left out.
    NoScroll,
    /// The creatures neither restored nor cut.
    NoCreatures,
    /// `paused` read as 0: the whole pass runs where the original runs only `play_keys`.
    IgnorePause,
}

impl Pass for RigPass<'_> {
    fn call(&mut self, offset: u32) {
        self.at = offset;
        self.calls.push(offset);
    }
    fn objects(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut MapObjects)) {
        let mut io = ObjIo {
            cpu: &mut *self.cpu,
            at: self.at,
            cells: 0,
            map: Vec::new(),
            plane: Vec::new(),
        };
        f(&mut MapObjects {
            state: &mut *l.state,
            io: &mut io,
        });
        // What the callees after read from the typed state, the object map and the plane, as
        // `objects::PlayIo` keeps them.
        let map = &mut l.state.object_map;
        for (a, v) in io.map {
            if let Some(b) = map.get_mut(a.wrapping_sub(OBJECT_MAP) as usize) {
                *b = v;
            }
        }
        for (at, v) in io.plane {
            l.plane.set_word(at, (v >> 16) as u16);
            l.plane.set_word(at.wrapping_add(2), v as u16);
        }
    }
    fn creatures(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut Creatures)) {
        if self.mutant == Mutant::NoCreatures {
            return;
        }
        let s = &mut *l.state;
        let camera_y = s.camera.y;
        f(&mut Creatures {
            draw: &mut s.creature_draw,
            slots: &mut s.edges.creatures,
            camera_y,
            io: &mut DrawCpu(self.cpu),
        });
    }
    fn joy1dat(&mut self, _: &FrameInput) -> u16 {
        self.cpu.mem.rw(JOY1DAT)
    }
    fn scratch(&mut self) -> Scratch {
        Scratch {
            beam: self.cpu.mem.beam.get() as u8,
            ..Scratch::default()
        }
    }
    fn scratch_done(&mut self, s: Scratch) {
        let b = &self.cpu.mem.beam;
        b.set(
            b.get()
                .wrapping_add(s.beam.wrapping_sub(b.get() as u8) as u16),
        );
        self.cpu.mem.hw_writes.extend(s.hw);
    }
    fn scroll(&mut self, camera_x: u16, camera_y: u16) {
        if self.mutant == Mutant::NoScroll {
            return;
        }
        put(
            self.cpu,
            1,
            Scroll::from_camera(camera_x, camera_y).copper(),
        );
    }
    fn raster_copy(&mut self) {
        if self.mutant == Mutant::NoRaster {
            return;
        }
        let v = self.cpu.mem.rw(VHPOSR);
        self.cpu.mem.ww(h0(0x8dfa), v);
    }
}

fn put(cpu: &mut Cpu, hunk: usize, writes: Vec<(u32, Vec<u8>)>) {
    for (off, bytes) in writes {
        for (k, b) in bytes.into_iter().enumerate() {
            cpu.mem.wb(h(hunk, off) + k as u32, b);
        }
    }
}

// --- the store back -------------------------------------------------------------------------------

/// What the pass keeps in the typed state outside `LAYOUT`, as (hunk 0 or 2 address, bytes).
fn extras(s: &State) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut w = |a: u32, v: &[u8]| out.push((a, v.to_vec()));
    let c = &s.camera;
    w(h0(0x4d76), &c.screen_x.to_be_bytes());
    w(h0(0x4d7a), &c.screen_y.to_be_bytes());
    let crossed = c.edges.crossed.map(|x| if x { 0xff } else { 0 });
    w(h0(0x51f8), &crossed);
    w(h0(0x519e), &s.frame_flag.to_be_bytes());
    w(h0(0x2a6a), &s.collect.score_add.to_be_bytes());
    w(h0(0x2692), &s.collect.row.to_be_bytes());
    w(h0(0x29c6), &s.collect.taken.to_be_bytes());
    w(h0(0x2fe6), &s.moving.slot.to_be_bytes());
    w(h0(0x306e), &s.moving.stomped.to_be_bytes());
    let cs = &s.edges.creatures;
    w(h0(0x2faa), &cs.index.to_be_bytes());
    w(h0(0x67da), &cs.active);
    for (i, v) in cs.slots.iter().enumerate() {
        let mut b = Vec::new();
        b.extend(v.x.to_be_bytes());
        b.extend(v.y.to_be_bytes());
        b.extend(v.kind.to_be_bytes());
        b.extend([v.facing, v.phase]);
        w(h0(0x67ec + 8 * i as u32), &b);
    }
    w(h0(0x581c), &cs.current.to_be_bytes());
    w(h0(0x687e), &cs.last.to_be_bytes());
    let d = &s.creature_draw;
    w(h0(0x2c7a), &d.restore_index.to_be_bytes());
    w(h0(0x2c7c), &d.cut_index.to_be_bytes());
    w(h0(0x5818), &d.x.to_be_bytes());
    w(h0(0x581a), &d.y.to_be_bytes());
    w(h0(0x581e), &d.restoring.to_be_bytes());
    w(h0(0x67e0), &d.saved);
    w(h0(0x67e6), &d.drawn);
    for n in 0..48usize {
        let (g, i) = (n / 12, n % 12);
        let e = s.edges.cells.cells[g][i];
        w(h(2, 0x1680 + n as u32), &[s.edges.cells.used[g][i]]);
        let mut b = Vec::new();
        b.extend(e.x.to_be_bytes());
        b.extend(e.y.to_be_bytes());
        b.push(e.code);
        b.extend(e.rest);
        w(h(2, 0x16b0 + 8 * n as u32), &b);
    }
    out
}

/// The display state the pass changes, as the original's memory holds it: the fade and the sprites.
fn display(g: &Game, cpu: &Cpu) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    let Fade {
        on,
        pass,
        now,
        scratch,
    } = g.display().fade;
    out.push((h0(0x94b4), vec![on]));
    out.push((h0(0x94b6), pass.to_be_bytes().to_vec()));
    for (i, c) in now.iter().enumerate() {
        out.push((h(1, COLOURS + 4 * i as u32), c.to_be_bytes().to_vec()));
    }
    for (i, w) in scratch.iter().enumerate() {
        out.push((h0(0x961e + 2 * i as u32), w.to_be_bytes().to_vec()));
    }
    let s: &CharSprite = &g.display().sprite;
    let m = &cpu.mem;
    let frame = |a: u32, n: u32| {
        (0..n)
            .map(|k| m.ram(a.wrapping_add(k)))
            .collect::<Vec<u8>>()
    };
    for (off, b) in s.bytes(&frame) {
        out.push((h(1, off), b));
    }
    out
}

/// Store what the engine changed: every `LAYOUT` variable and extra it moved, the input log's
/// byte, and the fade and sprites as they stand (the original writes them every pass).
fn store(cpu: &mut Cpu, before: &Game, after: &Game) {
    let w = |cpu: &mut Cpu, a: u32, b: &[u8]| {
        for (k, v) in b.iter().enumerate() {
            cpu.mem.wb(a + k as u32, *v);
        }
    };
    for v in LAYOUT {
        let (was, now) = (before.probe(v), after.probe(v));
        if was != now {
            let n = v.width.bytes();
            w(
                cpu,
                h(v.hunk as usize, v.offset),
                &now.to_be_bytes()[4 - n..],
            );
        }
    }
    let (s0, s1) = (before.state(), after.state());
    for ((a, was), (_, now)) in extras(s0).into_iter().zip(extras(s1)) {
        if was != now {
            w(cpu, a, &now);
        }
    }
    let i = s0.input_log.index;
    if s1.input_log.index != i && i < LOG_FULL {
        w(cpu, h0(INPUT_LOG + i), &[s1.input_log.byte(i)]);
    }
    for (a, b) in display(after, cpu) {
        w(cpu, a, &b);
    }
}

thread_local! {
    /// States running and paused the port ran on, on this test's thread (the rig calls the port
    /// on its caller's).
    static WAYS: Cell<[usize; 2]> = const { Cell::new([0; 2]) };
}

/// The engine's pass over the state in `cpu`.
fn game_frame(cpu: &mut Cpu) {
    let paused = cpu.mem.rw(h0(0x2100)) != 0;
    WAYS.with(|w| {
        let mut n = w.get();
        n[paused as usize] += 1;
        w.set(n);
    });
    pass(cpu, Mutant::None);
}

fn pass(cpu: &mut Cpu, mutant: Mutant) {
    let mut before = base_game().clone();
    before.transplant(&CpuMem(cpu));
    let mut g = before.clone();
    if mutant == Mutant::IgnorePause {
        let mut m = cpu.mem.clone();
        m.ww(h0(0x2100), 0);
        g.transplant(&CpuMem(&Cpu {
            d: cpu.d,
            a: cpu.a,
            mem: m,
        }));
    }
    let clxdat = cpu.mem.rw(CLXDAT);
    let mut p = RigPass {
        cpu,
        at: 0,
        calls: Vec::new(),
        mutant,
    };
    g.game_frame_on(FrameInput::NONE, clxdat, &mut p);
    store(cpu, &before, &g);
}

fn rig() -> Rig {
    Rig::new().unwrap_or_else(|e| panic!("{e}"))
}

fn generator() -> generators::Generator {
    generators::generator("game_frame").expect("T39's game_frame generator")
}

#[test]
fn game_frame_is_registered() {
    let p = PORTED
        .iter()
        .find(|p| p.name == "game_frame")
        .expect("game_frame in PORTED");
    assert_eq!((p.hunk, p.offset), (0, ENTRY));
    assert_eq!(p.port, "level::game_frame");
}

/// The engine's pass against the original on the first `cases` states of T39's generator, and how
/// many ran each way of `paused`.
fn check(cases: usize) {
    WAYS.with(|w| w.set([0; 2]));
    let rep: RigReport = judge::check_regs(
        &mut rig(),
        "game_frame",
        cases,
        h(0, ENTRY),
        &generator(),
        RegsCompared::NONE,
        game_frame,
    )
    .unwrap_or_else(|f| panic!("{f}"));
    let [running, paused] = WAYS.with(|w| w.get());
    println!("game_frame: {rep:?}, running {running}, paused {paused}");
    assert_eq!(rep.hangs + rep.faults, 0, "{rep:?}");
    assert_eq!(rep.passed, cases, "{rep:?}");
    assert!(
        paused > 0 && running > 0,
        "running {running}, paused {paused}"
    );
}

/// The task's proof: 10 000 states, about 2 min.
#[test]
fn game_frame_matches_the_original_on_10_000_states() {
    check(CASES);
}

/// The first 500 of the same states, for smoke.
#[test]
fn game_frame_matches_the_original_on_500_states() {
    check(500);
}

#[test]
fn the_original_takes_the_paused_branch_both_ways() {
    let c = rig().coverage("game_frame", 400, h(0, ENTRY), &generator());
    let b = c
        .branches
        .iter()
        .find(|b| b.at == h(0, PAUSED_BRANCH))
        .unwrap_or_else(|| panic!("no branch at h0+${PAUSED_BRANCH:x}"));
    assert!(b.taken > 0 && b.not_taken > 0, "{b:?}");
}

#[test]
fn wrong_passes_fail() {
    let mutants: [(&str, fn(&mut Cpu)); 4] = [
        ("no raster copy", |c| pass(c, Mutant::NoRaster)),
        ("no copper pointers", |c| pass(c, Mutant::NoScroll)),
        ("no creatures drawn", |c| pass(c, Mutant::NoCreatures)),
        ("paused ignored", |c| pass(c, Mutant::IgnorePause)),
    ];
    let mut r = rig();
    let g = generator();
    for (what, port) in mutants {
        let res = judge::check_regs(
            &mut r,
            "game_frame",
            200,
            h(0, ENTRY),
            &g,
            RegsCompared::NONE,
            port,
        );
        assert!(res.is_err(), "{what}: passed {:?}", res.unwrap());
    }
}

/// `game_frame`'s calls in `re/translate.toml`, as hunk 0 offsets, in the listing's order.
fn listed_calls() -> Vec<u32> {
    let text = include_str!("../../../../re/translate.toml");
    let entry = text
        .split("[[routine]]")
        .find(|r| r.contains("name = \"game_frame\"\n"))
        .expect("game_frame in re/translate.toml");
    let line = entry
        .lines()
        .find(|l| l.starts_with("calls = "))
        .expect("its calls");
    line.split('"')
        .filter(|s| s.starts_with("h0+$"))
        .map(|s| u32::from_str_radix(&s[4..], 16).unwrap())
        .collect()
}

/// The calls the engine's pass makes from the sync point, `paused` as given, through [`RigPass`]'s
/// recording on a state that is not the rig's (the play I/O, so nothing is compared).
fn engine_calls(paused: u16) -> Vec<u32> {
    struct Record(Vec<u32>);
    impl Pass for Record {
        fn call(&mut self, offset: u32) {
            self.0.push(offset);
        }
        fn objects(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut MapObjects)) {
            fd_core::objects::in_play(l, f);
        }
        fn creatures(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut Creatures)) {
            fd_core::render::objects::in_play(l, f);
        }
    }
    let mut g = base_game().clone();
    g.transplant(&SyncMem { paused });
    assert_eq!(g.state().paused, paused);
    let mut r = Record(Vec::new());
    g.game_frame_on(FrameInput::NONE, 0, &mut r);
    r.0
}

#[test]
fn the_engine_makes_every_listed_call_in_the_listings_order() {
    let listed = listed_calls();
    assert_eq!(listed.len(), 20, "{listed:x?}");
    // The listing's first `bsr play_keys` is the paused way's, the last call of the other.
    let k = listed.iter().position(|&c| c == PLAY_KEYS).unwrap();
    let paused: Vec<u32> = listed[..=k].to_vec();
    let mut running: Vec<u32> = listed.iter().copied().filter(|&c| c != PLAY_KEYS).collect();
    running.push(PLAY_KEYS);
    assert_eq!(engine_calls(1), paused, "paused");
    assert_eq!(engine_calls(0), running, "running");
}

#[test]
fn a_case_shows_game_frames_hits_on_the_original() {
    let case = Case::load(Path::new("cases/w1-idle.toml")).unwrap_or_else(|e| panic!("{e}"));
    let v = run(&case).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    let hits = v
        .coverage
        .hits
        .iter()
        .find(|(n, _)| *n == "game_frame")
        .map_or(0, |(_, n)| *n);
    println!("game_frame: {hits} hits over {} passes", v.passes);
    assert!(hits >= v.passes && v.passes > 0, "{v}");
}
