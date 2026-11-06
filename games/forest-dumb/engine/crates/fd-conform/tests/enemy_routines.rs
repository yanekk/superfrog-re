//! T24's engine ports (`fd_core::objects::creature`, plans/forest-dumb-level-1 T24) against the
//! original on the routine rig, over 10 000 states of each T54 and T38 generator, on the main program
//! from the owner's disks. The stomp, `char_hit_check`'s creature half, is held with its routine in
//! `tests/death_routines.rs`.
//!
//! Each check loads the [`State`] fields the creature ports use from the state's memory (the five
//! flags and slots, `creature_slot_index`, `char_x`, `h0+$2fe6`, `h0+$306e`), runs the engine function
//! with a [`Ground`] over that memory (`collision_map` and the step tables read in place), and stores
//! what it changed, so the rig compares every byte the original writes. The helpers take their slot in
//! `a0`, which the adapters turn into a slot number. No register is compared but `h0_3dd8`'s `d4` to
//! `d7`: its caller tests `d7`. `h0_2fea`'s and the probes' callers reload what they read.

use fd_conform::generators::{self, h};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_core::level::redraw::CreatureSlot;
use fd_core::objects::creature::{self, Creatures, Ground};
use fd_core::{State, PORTED};

const CASES: usize = 10_000;

const CHAR_X: u32 = 0x1e5a;
const MOVING_SLOT: u32 = 0x2fe6;
const SLOT_INDEX: u32 = 0x2faa;
const STOMPED: u32 = 0x306e;
const ACTIVE: u32 = 0x67da;
const SLOTS: u32 = 0x67ec;
/// `h0_3698`'s copy of the sorted flags (6 bytes) and slots (40), which it copies back.
const SORTED: u32 = 0x366a;
/// `collision_map`.
const COLLISION: u32 = h(3, 0xfa50);

fn h0(o: u32) -> u32 {
    h(0, 0).wrapping_add(o)
}

/// The creature ports' ground, the state's memory.
struct CpuGround<'a>(&'a Cpu);

impl Ground for CpuGround<'_> {
    fn collision(&self, off: u32) -> u8 {
        self.0.mem.rb(COLLISION.wrapping_add(off))
    }
    fn program(&self, a: u32) -> u8 {
        self.0.mem.rb(h0(a))
    }
}

/// The state the creature ports use, as the original's memory holds it.
fn load(cpu: &Cpu) -> State {
    let m = &cpu.mem;
    let mut s = State::level_1();
    s.char_x = m.rl(h0(CHAR_X));
    s.moving.slot = m.rl(h0(MOVING_SLOT));
    s.moving.stomped = m.rw(h0(STOMPED));
    let c = &mut s.edges.creatures;
    c.index = m.rl(h0(SLOT_INDEX));
    for i in 0..5u32 {
        c.active[i as usize] = m.rb(h0(ACTIVE + i));
        let a = h0(SLOTS + 8 * i);
        c.slots[i as usize] = CreatureSlot {
            x: m.rw(a),
            y: m.rw(a + 2),
            kind: m.rw(a + 4),
            facing: m.rb(a + 6),
            phase: m.rb(a + 7),
        };
    }
    s
}

/// Every byte [`load`] reads, with its address, from `s`.
fn bytes(s: &State) -> Vec<(u32, u8)> {
    let mut out = Vec::new();
    let mut w = |a: u32, v: &[u8]| {
        for (k, b) in v.iter().enumerate() {
            out.push((h0(a + k as u32), *b));
        }
    };
    w(CHAR_X, &s.char_x.to_be_bytes());
    w(MOVING_SLOT, &s.moving.slot.to_be_bytes());
    w(STOMPED, &s.moving.stomped.to_be_bytes());
    let c = &s.edges.creatures;
    w(SLOT_INDEX, &c.index.to_be_bytes());
    for i in 0..5u32 {
        w(ACTIVE + i, &[c.active[i as usize]]);
        let v = c.slots[i as usize];
        let a = SLOTS + 8 * i;
        w(a, &v.x.to_be_bytes());
        w(a + 2, &v.y.to_be_bytes());
        w(a + 4, &v.kind.to_be_bytes());
        w(a + 6, &[v.facing, v.phase]);
    }
    out
}

/// Store what the engine changed, only that.
fn store(cpu: &mut Cpu, before: &State, s: &State) {
    for ((a, b), (_, was)) in bytes(s).into_iter().zip(bytes(before)) {
        if b != was {
            cpu.mem.wb(a, b);
        }
    }
}

/// Run `f` on the creature ports over the state loaded from `cpu`, then store it.
fn on_creatures(cpu: &mut Cpu, f: impl FnOnce(&mut Creatures)) {
    let before = load(cpu);
    let mut s = before.clone();
    f(&mut Creatures {
        state: &mut s,
        ground: &CpuGround(cpu),
    });
    store(cpu, &before, &s);
}

/// The slot `a0` points at. The generators put it on one of the five.
fn slot_a0(cpu: &Cpu) -> usize {
    (cpu.a[0].wrapping_sub(h0(SLOTS)) / 8) as usize
}

fn move_all(cpu: &mut Cpu) {
    on_creatures(cpu, |c| c.move_all());
}

/// `h0_2fea` takes the slot in `d0` (T54's states hold 0 to 4).
fn move_slot(cpu: &mut Cpu) {
    let i = cpu.d[0] as usize;
    on_creatures(cpu, |c| c.move_slot(i));
}

fn step_back(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.step_back(i));
}

fn step_on(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.step_on(i));
}

fn hop_back(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.hop_back(i));
}

fn hop_on(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.hop_on(i));
}

fn wall_left(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.wall_left(i));
}

fn edge_left(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.edge_left(i));
}

fn wall_right(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.wall_right(i));
}

fn edge_right(cpu: &mut Cpu) {
    let i = slot_a0(cpu);
    on_creatures(cpu, |c| c.edge_right(i));
}

/// `h0_3698` also leaves its sorted copy at `h0+$366a`, which the engine does not keep: it equals
/// the flags and slots after the sort, the flags' sixth byte 0. The original copies that 0 back over
/// `creature_active+5` (a word move from the copy's fifth byte), which `game_frame`'s set swap (T52)
/// carries.
fn sort(cpu: &mut Cpu) {
    on_creatures(cpu, |c| c.sort());
    let m = &mut cpu.mem;
    let after: Vec<u8> = (0..5).map(|i| m.rb(h0(ACTIVE + i))).collect();
    let slots: Vec<u8> = (0..40).map(|i| m.rb(h0(SLOTS + i))).collect();
    for (k, b) in after.iter().chain(&[0]).chain(&slots).enumerate() {
        m.wb(h0(SORTED + k as u32), *b);
    }
    m.wb(h0(ACTIVE + 5), 0);
}

/// `h0_3dd8`: the slot in `a0`, the point in `d0`, `d1`'s low words; the box's corners left in
/// `d4` to `d7`'s low words, and `d7` cleared when the point is outside or its low byte 1 inside.
fn overlaps(cpu: &mut Cpu) {
    let s = load(cpu).edges.creatures.slots[slot_a0(cpu)];
    let (px, py) = (cpu.d[0] as u16, cpu.d[1] as u16);
    let word = |r: u32, v: u16| (r & 0xffff_0000) | v as u32;
    cpu.d[4] = word(cpu.d[4], s.x);
    cpu.d[5] = word(cpu.d[5], s.y);
    cpu.d[6] = word(cpu.d[6], s.x.wrapping_add(creature::BOX_W));
    let d7 = word(cpu.d[7], s.y.wrapping_add(creature::BOX_H));
    cpu.d[7] = if creature::overlaps(&s, px, py) {
        d7 & !0xff | 1
    } else {
        0
    };
}

type Adapter = fn(&mut Cpu);

/// Each routine: its generator's name (T54's and T38's, as in `re/translate.toml`), entry in hunk 0,
/// the engine port's adapter and the registers its callers read.
fn routines() -> Vec<(&'static str, u32, Adapter, RegsCompared)> {
    let none = RegsCompared::NONE;
    vec![
        ("h0_2fae", 0x2fae, move_all as Adapter, none),
        ("h0_2fea", 0x2fea, move_slot, none),
        ("h0_320a", 0x320a, step_back, none),
        ("h0_3222", 0x3222, step_on, none),
        ("h0_32f8", 0x32f8, hop_back, none),
        ("h0_3322", 0x3322, hop_on, none),
        ("h0_3530", 0x3530, wall_left, none),
        ("h0_3566", 0x3566, edge_left, none),
        ("h0_35a0", 0x35a0, wall_right, none),
        ("h0_35dc", 0x35dc, edge_right, none),
        ("h0_3698", 0x3698, sort, none),
        ("h0_3dd8", 0x3dd8, overlaps, RegsCompared::d(&[4, 5, 6, 7])),
    ]
}

#[test]
fn every_t24_routine_is_registered_and_checked_here() {
    // The routines `re/translate.toml` gives T24; `char_hit_check` is held in death_routines.rs.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let t24: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "port_task").as_deref() == Some("T24"))
        .map(|b| {
            let off = field(b, "offset").unwrap();
            let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
            (field(b, "name").unwrap(), off)
        })
        .collect();
    let mut here: Vec<(String, u32)> = routines().iter().map(|r| (r.0.to_string(), r.1)).collect();
    here.push(("char_hit_check".to_string(), 0x3b2a));
    here.sort();
    let mut t24 = t24;
    t24.sort();
    assert_eq!(here, t24);
    for (name, off) in &t24 {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == *off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_t24_routine_matches_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let mut lines = Vec::new();
    for (name, off, port, regs) in routines() {
        let generator = generators::generator(name).unwrap();
        let rep: RigReport =
            judge::check_regs(&mut r, name, CASES, h0(off), &generator, regs, port)
                .unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(
            (rep.passed, rep.hangs, rep.faults, rep.unmodelled_blits),
            (CASES, 0, 0, 0),
            "{name}"
        );
        lines.push(format!("{name}: {} paths", rep.paths));
    }
    println!("{}", lines.join("\n"));
}
