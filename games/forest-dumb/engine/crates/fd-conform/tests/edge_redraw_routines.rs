//! The edge redraws' engine port (`fd_core::level::redraw`, plans/forest-dumb-level-1 T55) against the
//! original on the routine rig, over 10 000 states of each T56 generator, on the main program from
//! the owner's disks.
//!
//! Each check loads [`Edges`] from the state's memory, runs the engine function with an [`Io`] over the
//! state's memory (the level buffers read, an object cell cleared, a `collision_map` word written, a
//! blit turned into the original's blitter writes, all in the order the engine makes them), and stores
//! [`Edges`] back, so the rig compares every byte and every custom-chip write. Registers are compared
//! where a caller reads them: `d0` after `h0_681c` and after the bob blits of rows (`h0_a06`, `h0_ce4`),
//! whose high word the scan carries to the next bob; none after the rest (`main_loop` reloads `d0`
//! before it reads it, and the redraws reload every register a helper leaves).

use amiga_oracle::{FrameInput, Joy};
use fd_conform::Live;
use fd_conform::generators::{self, Gen, Generator};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport, Setup};
use fd_conform::sync::reach_level_1;
use fd_core::level::collision::{BEFORE, CollisionPlane, KEPT, MAP};
use fd_core::level::redraw::{
    self, AnimCell, AnimatedCells, Blit, CreatureSlot, CreatureSlots, Edges, Io, Ptr,
};
use fd_core::{Game, Mem, PORTED, Start, Transplant};

const CASES: usize = 10_000;

fn h0(o: u32) -> u32 {
    generators::h(0, 0).wrapping_add(o)
}

const SCREEN: u32 = generators::h(3, 0x398);
const COLLISION: u32 = generators::h(3, 0xfa50);
const TILES: u32 = generators::h(3, 0x1_3540);
const BOBS: u32 = generators::h(4, 0xc80);
const MASKS: u32 = generators::h(4, 0x2580);
const POS: u32 = 0x1e66;
const P840: u32 = 0x840;
const P83C: u32 = 0x83c;
const ACTIVE: u32 = 0x67da;
const SLOTS: u32 = 0x67ec;
const CURRENT: u32 = 0x581c;
const LAST: u32 = 0x687e;
const CELLS_USED: u32 = generators::h(2, 0x1680);
const CELLS: u32 = generators::h(2, 0x16b0);

/// The redraws' memory, seen through [`Io`].
struct CpuIo<'a>(&'a mut Cpu);

impl Io for CpuIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        self.0.mem.rb(h0(a))
    }
    fn clear_object(&mut self, a: u32) {
        self.0.mem.wb(h0(a), 0);
    }
    fn blit(&mut self, b: Blit) {
        let m = &mut self.0.mem;
        // The register writes `h0_774` and `h0_a06` make, in their order; BLTSIZE starts the blit.
        match b {
            Blit::Tile { tile, dst } => {
                let d = SCREEN.wrapping_add(dst);
                m.wl(0xdf_f054, d);
                m.wl(0xdf_f050, TILES.wrapping_add(tile as u32 * 0xa0));
                m.ww(0xdf_f066, 0x2c);
                m.ww(0xdf_f064, 0);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x09f0);
                m.ww(0xdf_f042, 0);
                m.ww(0xdf_f058, 0x1401);
            }
            Blit::Cell { .. } | Blit::Element { .. } | Blit::Picture { .. } => {
                unreachable!("only the object ports (T22, T25) start these blits")
            }
            Blit::Bob { image, dst } => {
                let d = SCREEN.wrapping_add(dst);
                m.wl(0xdf_f054, d);
                m.wl(0xdf_f050, MASKS.wrapping_add(image));
                m.wl(0xdf_f04c, BOBS.wrapping_add(image));
                m.wl(0xdf_f048, d);
                m.ww(0xdf_f066, 0x2c);
                m.ww(0xdf_f064, 0x26);
                m.ww(0xdf_f062, 0x26);
                m.ww(0xdf_f060, 0x2c);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x0fca);
                m.ww(0xdf_f042, 0);
                m.ww(0xdf_f058, 0x1401);
            }
        }
    }
    fn collision_word(&mut self, at: u32, v: u16) {
        self.0.mem.ww(COLLISION.wrapping_add(at), v);
    }
}

/// [`Edges`] as the original's memory holds it. `h0+$840` and `$83c` load as plain longs: the
/// redraws write them before reading.
fn load(cpu: &Cpu) -> Edges {
    let m = &cpu.mem;
    let mut creatures = CreatureSlots {
        current: m.rw(h0(CURRENT)),
        last: m.rl(h0(LAST)),
        ..CreatureSlots::default()
    };
    for i in 0..5u32 {
        creatures.active[i as usize] = m.rb(h0(ACTIVE + i));
        let s = h0(SLOTS + 8 * i);
        creatures.slots[i as usize] = CreatureSlot {
            x: m.rw(s),
            y: m.rw(s + 2),
            kind: m.rw(s + 4),
            facing: m.rb(s + 6),
            phase: m.rb(s + 7),
        };
    }
    let mut cells = AnimatedCells::default();
    for g in 0..4u32 {
        for i in 0..12u32 {
            let n = g * 12 + i;
            cells.used[g as usize][i as usize] = m.rb(CELLS_USED + n);
            let c = CELLS + 8 * n;
            cells.cells[g as usize][i as usize] = AnimCell {
                x: m.rw(c),
                y: m.rw(c + 2),
                code: m.rb(c + 4),
                rest: [m.rb(c + 5), m.rb(c + 6), m.rb(c + 7)],
            };
        }
    }
    Edges {
        pos: (m.rw(h0(POS)), m.rw(h0(POS + 2))),
        p840: Ptr::Raw(m.rl(h0(P840))),
        p83c: Ptr::Raw(m.rl(h0(P83C))),
        creatures,
        cells,
        deferred: Vec::new(),
    }
}

fn long(p: Ptr) -> u32 {
    match p {
        Ptr::Raw(v) => v,
        Ptr::Screen(o) => SCREEN.wrapping_add(o),
        Ptr::Collision(o) => COLLISION.wrapping_add(o),
    }
}

/// Every byte [`Edges`] holds, with its address, as the original lays it out.
fn bytes(e: &Edges) -> Vec<(u32, u8)> {
    let mut out = Vec::new();
    let mut w = |a: u32, v: &[u8]| {
        for (k, b) in v.iter().enumerate() {
            out.push((a + k as u32, *b));
        }
    };
    w(h0(POS), &e.pos.0.to_be_bytes());
    w(h0(POS + 2), &e.pos.1.to_be_bytes());
    w(h0(P840), &long(e.p840).to_be_bytes());
    w(h0(P83C), &long(e.p83c).to_be_bytes());
    let c = &e.creatures;
    w(h0(CURRENT), &c.current.to_be_bytes());
    w(h0(LAST), &c.last.to_be_bytes());
    for i in 0..5u32 {
        w(h0(ACTIVE + i), &[c.active[i as usize]]);
        let s = h0(SLOTS + 8 * i);
        let v = c.slots[i as usize];
        w(s, &v.x.to_be_bytes());
        w(s + 2, &v.y.to_be_bytes());
        w(s + 4, &v.kind.to_be_bytes());
        w(s + 6, &[v.facing, v.phase]);
    }
    for g in 0..4u32 {
        for i in 0..12u32 {
            let n = g * 12 + i;
            w(CELLS_USED + n, &[e.cells.used[g as usize][i as usize]]);
            let a = CELLS + 8 * n;
            let v = e.cells.cells[g as usize][i as usize];
            w(a, &v.x.to_be_bytes());
            w(a + 2, &v.y.to_be_bytes());
            w(a + 4, &[v.code]);
            w(a + 5, &v.rest);
        }
    }
    out
}

/// Store what the engine changed. Only that: a bob blit the original sends past the screen can land
/// on the animated cells (chip RAM, hunk 2), and writing back what was loaded would undo it.
fn store(cpu: &mut Cpu, before: &Edges, e: &Edges) {
    for ((a, b), (_, was)) in bytes(e).into_iter().zip(bytes(before)) {
        if b != was {
            cpu.mem.wb(a, b);
        }
    }
}

/// Run `f` on [`Edges`] loaded from `cpu` with an [`Io`] over it, then store it.
fn on_edges(cpu: &mut Cpu, f: impl FnOnce(&mut Edges, &mut CpuIo)) {
    let before = load(cpu);
    let mut e = before.clone();
    f(&mut e, &mut CpuIo(cpu));
    store(cpu, &before, &e);
}

fn pos(cpu: &Cpu) -> (u16, u16) {
    (cpu.mem.rw(h0(POS)), cpu.mem.rw(h0(POS + 2)))
}

/// The bob helpers take the cell's byte at `a2`, the position and `d4`.
fn bob_args(cpu: &Cpu) -> (u8, (u16, u16), u32) {
    (cpu.mem.rb(cpu.a[2]), pos(cpu), cpu.d[4])
}

fn edge_a(cpu: &mut Cpu) {
    on_edges(cpu, |e, io| e.redraw_below(io));
}
fn edge_b(cpu: &mut Cpu) {
    on_edges(cpu, |e, io| e.redraw_above(io));
}
fn edge_c(cpu: &mut Cpu) {
    on_edges(cpu, |e, io| e.redraw_left(io));
}
fn edge_d(cpu: &mut Cpu) {
    on_edges(cpu, |e, io| e.redraw_right(io));
}

fn tile_blit(cpu: &mut Cpu) {
    let (tile, dst) = (cpu.d[5] as u16, cpu.a[3].wrapping_sub(SCREEN));
    redraw::tile_blit(&mut CpuIo(cpu), tile, dst);
}

fn shape_copy(cpu: &mut Cpu) {
    let (n, at) = (cpu.d[5], cpu.a[3].wrapping_sub(COLLISION));
    redraw::shape_copy(&mut CpuIo(cpu), n, at);
}

fn bob_a(cpu: &mut Cpu) {
    let (b, p, d4) = bob_args(cpu);
    let d0 = cpu.d[0];
    cpu.d[0] = redraw::bob_below(&mut CpuIo(cpu), b, p, d4, d0);
}
fn bob_b(cpu: &mut Cpu) {
    let (b, p, d4) = bob_args(cpu);
    let d0 = cpu.d[0];
    cpu.d[0] = redraw::bob_above(&mut CpuIo(cpu), b, p, d4, d0);
}
fn bob_c(cpu: &mut Cpu) {
    let (b, p, d4) = bob_args(cpu);
    redraw::bob_left(&mut CpuIo(cpu), b, p, d4);
}
fn bob_d(cpu: &mut Cpu) {
    let (b, p, d4) = bob_args(cpu);
    redraw::bob_right(&mut CpuIo(cpu), b, p, d4);
}

fn blitter_wait(_: &mut Cpu) {
    redraw::blitter_wait();
}

/// `h0_681c` takes x, y, kind and facing in the low words of `d0`-`d2` and the low byte of `d3`.
fn free_slot(cpu: &mut Cpu) {
    let d = cpu.d;
    let mut refused = false;
    on_edges(cpu, |e, _| {
        refused = e
            .creatures
            .start(d[0] as u16, d[1] as u16, d[2] as u16, d[3] as u8);
    });
    cpu.d[0] = if refused { 0xffff_ffff } else { 0 };
}

/// `h0_6852`: slot `d7`.
fn fill_slot(cpu: &mut Cpu) {
    let d = cpu.d;
    on_edges(cpu, |e, _| {
        e.creatures.fill(
            d[7] as usize,
            d[0] as u16,
            d[1] as u16,
            d[2] as u16,
            d[3] as u8,
        );
    });
}

/// `h0_6882`: the cell at `d0`, `d1` (words), its value at `a2`.
fn add_animated_cell(cpu: &mut Cpu) {
    let (x, y, b) = (cpu.d[0] as u16, cpu.d[1] as u16, cpu.mem.rb(cpu.a[2]));
    on_edges(cpu, |e, _| e.cells.add(x, y, b));
}

/// T56's `h0_6852` states with `d7` taken into the five slots. One in 50 of its states has `d7` 5 to
/// `$3f`, which the original writes past the slot table (over its own code at `h0+$681c`); its one
/// caller, `h0_681c`, only passes a free slot 0 to 4, so the engine models five slots.
fn fill_slot_in_range(g: &mut Gen) -> Setup {
    let mut s = (generators::generator("h0_6852").unwrap().case)(g);
    s.d[7] %= 5;
    s
}

/// Each routine: its generator's name (T56's, as in `re/translate.toml`), entry in hunk 0, the engine
/// port's adapter and the registers its callers read.
fn routines() -> Vec<(&'static str, u32, fn(&mut Cpu), RegsCompared)> {
    let d0 = RegsCompared::d(&[0]);
    let none = RegsCompared::NONE;
    vec![
        ("h0_774", 0x774, tile_blit, none),
        ("h0_7cc", 0x7cc, shape_copy, none),
        ("redraw_edge_a", 0x844, edge_a, none),
        ("h0_a06", 0xa06, bob_a, d0),
        ("redraw_edge_b", 0xb20, edge_b, none),
        ("h0_ce4", 0xce4, bob_b, d0),
        ("redraw_edge_c", 0xdf6, edge_c, none),
        ("h0_104c", 0x104c, bob_c, none),
        ("redraw_edge_d", 0x114a, edge_d, none),
        ("h0_1390", 0x1390, bob_d, none),
        ("h0_3ac2", 0x3ac2, blitter_wait, none),
        ("h0_681c", 0x681c, free_slot, d0),
        ("h0_6852", 0x6852, fill_slot, none),
        ("h0_6882", 0x6882, add_animated_cell, none),
    ]
}

/// The memory a redraw generator's states start from (the program and T56's level base), set once
/// before a check so the filters below can run the engine on a state.
static BASE: std::sync::OnceLock<std::sync::Arc<Vec<u8>>> = std::sync::OnceLock::new();

/// The animated-cell table, chip RAM in hunk 2: flags then cells.
const CELL_TABLE: std::ops::Range<u32> = CELLS_USED..CELLS + 48 * 8;

/// The four redraws, by name.
fn run_redraw(name: &str, e: &mut Edges, io: &mut impl Io) {
    match name {
        "redraw_edge_a" => e.redraw_below(io),
        "redraw_edge_b" => e.redraw_above(io),
        "redraw_edge_c" => e.redraw_left(io),
        "redraw_edge_d" => e.redraw_right(io),
        _ => unreachable!("{name}"),
    }
}

/// Whether the engine, run on `s` with redraw `name`, starts a bob blit that writes into
/// [`CELL_TABLE`]. The original's bob blits a row redraw sends past the screen (a column from a wild
/// x, or one past `d0`'s carried high word) are masked into chip RAM, where they can overwrite the
/// animated cells the same scan then adds to. The engine keeps those cells as typed state and its
/// blits as the picture's, so such a state is one it does not model (docs/11-translation.md, "T55").
fn hits_the_cells(s: &Setup, name: &str) -> bool {
    struct Probe<'a> {
        io: CpuIo<'a>,
        hit: bool,
    }
    impl Io for Probe<'_> {
        fn rb(&self, a: u32) -> u8 {
            self.io.rb(a)
        }
        fn clear_object(&mut self, a: u32) {
            self.io.clear_object(a)
        }
        fn blit(&mut self, b: Blit) {
            if let Blit::Bob { dst, .. } = b {
                // 80 lines `$2e` bytes apart, the last line's word.
                let start = SCREEN.wrapping_add(dst) & 0x7_fffe;
                let end = start + 79 * 0x2e + 2;
                self.hit |= start < CELL_TABLE.end && CELL_TABLE.start < end;
            }
            self.io.blit(b)
        }
        fn collision_word(&mut self, at: u32, v: u16) {
            self.io.collision_word(at, v)
        }
    }
    let base = BASE.get().expect("BASE is set before a check").clone();
    let mut cpu = Cpu {
        d: s.d,
        a: s.a,
        mem: judge::Mem::new(base, &s.pokes),
    };
    let mut e = load(&cpu);
    let mut p = Probe {
        io: CpuIo(&mut cpu),
        hit: false,
    };
    run_redraw(name, &mut e, &mut p);
    p.hit
}

/// T56's state for redraw `name`, drawn again while it [`hits_the_cells`].
fn redraw_state(g: &mut Gen, name: &str) -> Setup {
    let case = generators::generator(name).unwrap().case;
    loop {
        let s = case(g);
        if !hits_the_cells(&s, name) {
            return s;
        }
    }
}

fn state_a(g: &mut Gen) -> Setup {
    redraw_state(g, "redraw_edge_a")
}
fn state_b(g: &mut Gen) -> Setup {
    redraw_state(g, "redraw_edge_b")
}
fn state_c(g: &mut Gen) -> Setup {
    redraw_state(g, "redraw_edge_c")
}
fn state_d(g: &mut Gen) -> Setup {
    redraw_state(g, "redraw_edge_d")
}

/// The generator a check runs: T56's, filtered for `h0_6852` and the four redraws as above.
fn generator(name: &str) -> Generator {
    let g = generators::generator(name).unwrap();
    let case: fn(&mut Gen) -> Setup = match name {
        "h0_6852" => fill_slot_in_range,
        "redraw_edge_a" => state_a,
        "redraw_edge_b" => state_b,
        "redraw_edge_c" => state_c,
        "redraw_edge_d" => state_d,
        _ => g.case,
    };
    Generator { base: g.base, case }
}

/// Set [`BASE`] from the rig: the program and the redraws' level base.
fn set_base(r: &Rig) {
    BASE.get_or_init(|| r.memory_for(&generators::generator("redraw_edge_a").unwrap()));
}

#[test]
fn every_t55_routine_is_registered_and_checked_here() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let mut t55: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "port_task").as_deref() == Some("T55"))
        .map(|b| {
            let off = field(b, "offset").unwrap();
            let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
            (field(b, "name").unwrap(), off)
        })
        .collect();
    t55.sort();
    let mut here: Vec<(String, u32)> = routines().iter().map(|r| (r.0.to_string(), r.1)).collect();
    here.sort();
    assert_eq!(here, t55);
    for (name, off) in &t55 {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == *off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_edge_redraw_routine_matches_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    set_base(&r);
    let mut lines = Vec::new();
    for (name, off, port, regs) in routines() {
        let rep: RigReport =
            judge::check_regs(&mut r, name, CASES, h0(off), &generator(name), regs, port)
                .unwrap_or_else(|f| panic!("{f}"));
        assert_eq!((rep.passed, rep.hangs, rep.faults), (CASES, 0, 0), "{name}");
        lines.push(format!("{name}: {} paths", rep.paths));
    }
    println!("{}", lines.join("\n"));
}

#[test]
fn the_pointer_tables_are_sixteen_rotated_bands() {
    // `h0+$1496` and `h0+$18d6` are the program's own data; the engine computes them.
    let r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let mem = r.memory_for(&generators::generator("h0_3ac2").unwrap());
    let rl = |a: u32| u32::from_be_bytes(mem[a as usize..a as usize + 4].try_into().unwrap());
    for k in 0..256u32 {
        assert_eq!(
            rl(h0(0x1496 + 4 * k)),
            SCREEN + redraw::screen_band(k),
            "screen {k}"
        );
        assert_eq!(
            rl(h0(0x18d6 + 4 * k)),
            COLLISION + redraw::collision_band(k),
            "collision {k}"
        );
    }
}

/// `h0_a06` with the high word of `d0` dropped: what a port would do that missed the column the scan
/// carries in it (a creature that found no slot leaves `$ffff` there, and the next bob lands past it).
fn bob_a_without_the_carry(cpu: &mut Cpu) {
    cpu.d[0] &= 0xffff;
    bob_a(cpu);
}

#[test]
fn a_row_bob_dropping_the_high_word_of_d0_fails() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let f = judge::check_regs(
        &mut r,
        "h0_a06",
        CASES,
        h0(0xa06),
        &generator("h0_a06"),
        RegsCompared::d(&[0]),
        bob_a_without_the_carry,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}

#[test]
fn a_state_with_d7_past_the_slots_is_one_the_engine_does_not_model() {
    // Unfiltered, T56's `h0_6852` states include `d7` 5 to `$3f`; the original writes past the five
    // slots there, which the engine cannot (its slots are an array of five).
    let mut g = Gen::new(1);
    let gen_ = generators::generator("h0_6852").unwrap();
    let past = (0..2000).filter(|_| (gen_.case)(&mut g).d[7] >= 5).count();
    assert!(past > 0);
}

#[test]
fn the_cell_table_filter_drops_few_states_and_not_none() {
    let r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    set_base(&r);
    let mut lines = Vec::new();
    let mut any = 0;
    for name in [
        "redraw_edge_a",
        "redraw_edge_b",
        "redraw_edge_c",
        "redraw_edge_d",
    ] {
        let case = generators::generator(name).unwrap().case;
        let mut g = Gen::new(0x55);
        let hit = (0..2000)
            .filter(|_| hits_the_cells(&case(&mut g), name))
            .count();
        // Under 1 in 20: the filter is an exclusion of wild positions, not of the redraw.
        assert!(hit < 100, "{name}: {hit} of 2000");
        any += hit;
        lines.push(format!("{name}: {hit} of 2000 states dropped"));
    }
    assert!(any > 0);
    println!("{}", lines.join("\n"));
}

/// What [`collision_after_redraws`] saw.
#[derive(Debug, Default)]
struct Seen {
    /// Frames the engine redrew each edge (down, up, left, right) and matched.
    redrawn: [u32; 4],
    /// Frames where the original's foreground was cut by the next pass and matched one frame later.
    cut: Vec<u32>,
    /// The first frame that did not match, even a frame later.
    differ: Option<String>,
}

/// The original from the level sync point with `script` (frame after it, input from then on), and
/// the engine started from its memory each frame and run through its foreground. After the original's
/// next frame, `collision_map` must equal the engine's; where the original's foreground was cut short
/// by the next pass (it runs between interrupts, its blits waiting on the raster), the bytes that
/// differ must equal the engine's a frame later, when the original has finished, and that frame is
/// not compared on its own.
fn collision_after_redraws(script: &[(u32, FrameInput)], frames: u32) -> Seen {
    let mut am = reach_level_1(frames + 1).unwrap_or_else(|e| panic!("{e}"));
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let input = |f: u32| {
        script
            .iter()
            .rev()
            .find(|(at, _)| *at <= f)
            .map_or(FrameInput::NONE, |(_, i)| *i)
    };
    let theirs =
        |am: &amiga_oracle::Amiga| Live(am).read(3, MAP - BEFORE, (BEFORE + KEPT) as usize);
    let mut seen = Seen::default();
    let mut f = 0;
    while f < frames {
        let mut g = Game::new(data.clone(), Start::Level1);
        g.transplant(&Live(&am));
        // main_loop's order: down, up, left, right; `Dir` indexes right, left, up, down.
        let latched: Vec<bool> = [3, 2, 1, 0]
            .iter()
            .map(|&i| g.state().camera.edges.latch[i] != 0)
            .collect();
        g.foreground();
        let ours = g.plane();
        am.step(input(f)).unwrap_or_else(|e| panic!("{e}"));
        let differ: Vec<u32> = CollisionPlane::kept_offsets()
            .zip(theirs(&am))
            .filter(|&(at, b)| ours.kept_byte(at) != b)
            .map(|(at, _)| at)
            .collect();
        if !differ.is_empty() {
            am.step(input(f + 1)).unwrap_or_else(|e| panic!("{e}"));
            let later = theirs(&am);
            let at = |a: u32| (a.wrapping_add(BEFORE)) as usize;
            let still: Vec<u32> = differ
                .iter()
                .copied()
                .filter(|&a| ours.kept_byte(a) != later[at(a)])
                .collect();
            if !still.is_empty() {
                seen.differ = Some(format!(
                    "frame +{f}: {} byte(s) differ, {} still a frame later, first at \
                     collision_map{:+#x}; latched (down, up, left, right) {latched:?}",
                    differ.len(),
                    still.len(),
                    still[0] as i32
                ));
                return seen;
            }
            seen.cut.push(f);
            f += 1;
        }
        for (n, l) in seen.redrawn.iter_mut().zip(&latched) {
            *n += *l as u32;
        }
        f += 1;
    }
    seen
}

#[test]
fn after_a_scroll_each_way_the_collision_map_equals_the_originals() {
    // From the sync point: the camera settles down onto the character (down), the character walks
    // right (right), jumps (up is the jump; the camera follows it up), and walks back left (left).
    let script = [
        (0, FrameInput::NONE),
        (40, FrameInput::joy(Joy::RIGHT)),
        (200, FrameInput::joy(Joy::RIGHT | Joy::UP)),
        (215, FrameInput::joy(Joy::RIGHT)),
        (260, FrameInput::joy(Joy::LEFT | Joy::UP)),
        (275, FrameInput::joy(Joy::LEFT)),
        (400, FrameInput::joy(Joy::LEFT | Joy::UP)),
        (415, FrameInput::joy(Joy::LEFT)),
    ];
    let seen = collision_after_redraws(&script, 520);
    println!("{seen:?}");
    assert_eq!(seen.differ, None, "{seen:?}");
    assert!(seen.redrawn.iter().all(|&n| n > 0), "{seen:?}");
}

#[test]
fn the_edge_latches_step_with_the_original_over_the_idle_case() {
    // `Game::step` runs the foreground's redraws before the pass, because a pass's state is the
    // original's before `main_loop` has cleared the latches it set (docs/09, "The edge redraws"). At
    // x 0 the left scroll latches every pass, so a foreground run after the pass would leave
    // `edge_latch_left` 0 where the original reads `$ffff` (T55 review: that mutation fails here).
    use fd_conform::{Case, Outcome, run};
    let base =
        Case::load(std::path::Path::new("cases/w1-idle.toml")).unwrap_or_else(|e| panic!("{e}"));
    let mut vars = base.vars.clone();
    for d in ["right", "left", "up", "down"] {
        vars.push(format!("edge_latch_{d}"));
        vars.push(format!("edge_at_{d}_x"));
        vars.push(format!("edge_at_{d}_y"));
    }
    let v = run(&Case { vars, ..base }).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}
