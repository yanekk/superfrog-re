//! `seam::walk` without disks (translator T60): a walk through a function's `SHAPES_*` makes the
//! calls the generated function makes, and each decision and stop behaves as documented.
//!
//! The fixtures are pcode-rt's committed translations of made-up exports (`tools/pcode_rust.py`),
//! so a walk is checked against real generated code: the generated function runs on a recording
//! bus, then the walk runs on another with the decisions that run took, and the two recordings of
//! `poll`/`insn`/`sync` with the cycles at each call must be equal. The decision and stop cases run
//! on hand-written shapes holding custom-chip accesses, which no fixture has.

#[allow(dead_code)]
#[path = "../../pcode-rt/tests/fixtures/dbf_loop.rs"]
mod dbf_loop;
#[allow(dead_code)]
#[path = "../../pcode-rt/tests/fixtures/flags_call.rs"]
mod flags_call;
#[allow(dead_code)]
#[path = "../../pcode-rt/tests/fixtures/nested_call.rs"]
mod nested_call;
#[allow(dead_code)]
#[path = "../../pcode-rt/tests/fixtures/shared_call.rs"]
mod shared_call;

use std::collections::VecDeque;

use fd_translated::seam::walk::{Decision, Event, Until, Walk};
use pcode_rt::shape::{Flow, Shape};
use pcode_rt::{Bus, Image, Regs, Stop};

const BASES: [u32; 1] = [0x1000];
const STACK: u32 = 0x8000;
const RETURN_TO: u32 = 0xf0_0000;
const CHIPS: u32 = 0xdf_f000;

/// A bus call, with the cycles and the cycles off the bus counted when it was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Call {
    Poll(u64, u64),
    Insn(u64, u64),
    Sync(u64, u64),
    /// A custom-chip write: offset, value, size in bytes.
    Write(u16, u32, u8),
    /// A custom-chip read: offset, value.
    Read(u16, u16),
}

/// RAM below `CHIPS`; custom reads answered from `reads` in order (0 once they run out), every
/// custom access and every `poll`/`insn`/`sync` recorded.
struct Rec {
    ram: Vec<u8>,
    calls: Vec<Call>,
    reads: VecDeque<u16>,
}

impl Rec {
    fn new() -> Rec {
        Rec {
            ram: vec![0; 0x10000],
            calls: Vec::new(),
            reads: VecDeque::new(),
        }
    }

    fn custom(a: u32) -> Option<u16> {
        (a & !0x1ff == CHIPS).then_some((a & 0x1ff) as u16)
    }

    fn read(&mut self, reg: u16) -> u16 {
        let v = self.reads.pop_front().unwrap_or(0);
        self.calls.push(Call::Read(reg, v));
        v
    }

    /// The `poll`/`insn`/`sync` calls alone.
    fn timing(&self) -> Vec<Call> {
        self.calls
            .iter()
            .copied()
            .filter(|c| matches!(c, Call::Poll(..) | Call::Insn(..) | Call::Sync(..)))
            .collect()
    }
}

impl Bus for Rec {
    fn poll(&mut self, r: &mut Regs, _: &Image) -> Result<(), Stop> {
        self.calls.push(Call::Poll(r.cycles(), r.off_bus_cycles()));
        Ok(())
    }
    fn insn(&mut self, r: &mut Regs, _: &Image) -> Result<(), Stop> {
        self.calls.push(Call::Insn(r.cycles(), r.off_bus_cycles()));
        Ok(())
    }
    fn sync(&mut self, r: &Regs) {
        self.calls.push(Call::Sync(r.cycles(), r.off_bus_cycles()));
    }
    fn rb(&mut self, a: u32) -> u8 {
        match Rec::custom(a) {
            Some(reg) => self.read(reg) as u8,
            None => self.ram[a as usize],
        }
    }
    fn rw(&mut self, a: u32) -> u16 {
        match Rec::custom(a) {
            Some(reg) => self.read(reg),
            None => u16::from_be_bytes([self.ram[a as usize], self.ram[a as usize + 1]]),
        }
    }
    fn rl(&mut self, a: u32) -> u32 {
        ((self.rw(a) as u32) << 16) | self.rw(a + 2) as u32
    }
    fn wb(&mut self, a: u32, v: u8) {
        match Rec::custom(a) {
            Some(reg) => self.calls.push(Call::Write(reg, v as u32, 1)),
            None => self.ram[a as usize] = v,
        }
    }
    fn ww(&mut self, a: u32, v: u16) {
        match Rec::custom(a) {
            Some(reg) => self.calls.push(Call::Write(reg, v as u32, 2)),
            None => self.ram[a as usize..a as usize + 2].copy_from_slice(&v.to_be_bytes()),
        }
    }
    fn wl(&mut self, a: u32, v: u32) {
        match Rec::custom(a) {
            Some(reg) => self.calls.push(Call::Write(reg, v, 4)),
            None => self.ram[a as usize..a as usize + 4].copy_from_slice(&v.to_be_bytes()),
        }
    }
}

type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;
type Shapes = fn((u32, u32)) -> Option<&'static [Shape]>;

/// The generated function `f` and a walk of its shapes from (0, 0) with `decisions`, from the
/// same registers: the timing calls each made and the cycles each counted must be equal.
fn same_calls(
    f: Routine,
    shapes: Shapes,
    sizes: &[u32],
    code: &[(u32, u32, u32)],
    regs: &Regs,
    decisions: &[Decision],
) {
    let img = Image {
        bases: &BASES,
        sizes,
        code,
    };
    let (mut gen_r, mut gen_bus) = (regs.clone(), Rec::new());
    gen_r.set_a(7, STACK - 4);
    gen_bus.wl(STACK - 4, RETURN_TO);
    f(&mut gen_r, &mut gen_bus, &img).unwrap();

    let (mut walk_r, mut walk_bus) = (regs.clone(), Rec::new());
    let mut w = Walk::new((0, 0), shapes);
    w.decide(decisions.iter().copied());
    assert_eq!(
        w.run(&mut walk_bus, &mut walk_r, &img, Until::End),
        Ok(Event::Done)
    );
    assert!(w.done());
    assert_eq!(w.left(), (0, 0), "decisions or writes left over");

    assert_eq!(walk_bus.timing(), gen_bus.timing());
    assert!(!gen_bus.timing().is_empty());
    assert_eq!(
        (walk_r.cycles(), walk_r.off_bus_cycles()),
        (gen_r.cycles(), gen_r.off_bus_cycles())
    );
    assert_eq!(walk_r.steps, gen_r.steps, "blocks entered");
}

#[test]
fn a_dbf_loop_walked_by_its_counts_makes_the_generated_calls() {
    // moveq #3,d0; moveq #0,d1; loop: addq.l #2,d1; dbf d0,loop; rts. The counter before each dbf.
    let counts = [3, 2, 1, 0].map(Decision::Count);
    same_calls(
        dbf_loop::h0_0,
        dbf_loop::shapes,
        dbf_loop::HUNK_SIZES,
        dbf_loop::CODE,
        &Regs::new(),
        &counts,
    );
}

#[test]
fn a_branch_after_a_call_either_way_makes_the_generated_calls() {
    // bsr cmp; beq.b; the branch's two paths tick differently, so the cycles tell them apart.
    for (d0, d1) in [(5, 5), (5, 6)] {
        let mut r = Regs::new();
        r.set_d(0, d0);
        r.set_d(1, d1);
        same_calls(
            flags_call::h0_0,
            flags_call::shapes,
            flags_call::HUNK_SIZES,
            flags_call::CODE,
            &r,
            &[Decision::Branch(d0 == d1)],
        );
    }
}

#[test]
fn nested_and_shared_calls_make_the_generated_calls() {
    same_calls(
        nested_call::h0_0,
        nested_call::shapes,
        nested_call::HUNK_SIZES,
        nested_call::CODE,
        &Regs::new(),
        &[],
    );
    same_calls(
        shared_call::h0_0,
        shared_call::shapes,
        shared_call::HUNK_SIZES,
        shared_call::CODE,
        &Regs::new(),
        &[],
    );
}

#[test]
fn a_walk_that_misses_a_decision_stops() {
    let img = Image {
        bases: &BASES,
        sizes: dbf_loop::HUNK_SIZES,
        code: dbf_loop::CODE,
    };
    let mut w = Walk::new((0, 0), dbf_loop::shapes);
    w.decide([Decision::Branch(true)]);
    let got = w.run(&mut Rec::new(), &mut Regs::new(), &img, Until::End);
    assert!(
        matches!(&got, Err(Stop::Unsupported { op, at }) if op.contains("Branch(true) at a dbcc") && at == "h0+$6"),
        "{got:?}"
    );
}

// Hand-written shapes: the time of each is its own number of cycles, so a test sees which ran.

fn t4(r: &mut Regs) {
    r.tick(4);
}
fn t8(r: &mut Regs) {
    r.tick(8);
}
fn t10(r: &mut Regs) {
    r.tick(10);
}
fn t16(r: &mut Regs) {
    r.tick(16);
}
fn t20(r: &mut Regs) {
    r.tick(20);
}

const fn op(at: u32, len: u8, first: bool, custom: Option<(u16, u8, bool)>, tick: fn(&mut Regs), flow: Flow) -> Shape {
    Shape {
        h: 0,
        at,
        len,
        first,
        mem: custom.is_some() || matches!(flow, Flow::Call { .. } | Flow::Ret),
        dynamic: false,
        uncounted: false,
        custom,
        tick,
        flow,
    }
}

/// h0+$0:
///   $0  move.w d0,$dff096
///   $6  bsr $40
///   $8  btst #6,$dff002      (a wait on the blitter)
///   $10 bne $8
///   $12 move.b $dff006,d1
///   $18 cmp.b $dff006,d1     (a wait for the next line)
///   $1e beq $18
///   $20 rts
/// h0+$40:
///   $40 move.w d1,$dff09a
///   $46 bsr $60
///   $48 rts
/// h0+$60:
///   $60 move.l d2,$dff080
///   $66 rts
static MAIN: &[Shape] = &[
    op(0x0, 6, true, Some((0x96, 2, true)), t16, Flow::Next),
    op(0x6, 2, false, None, t20, Flow::Call { to: 0x40 }),
    op(0x8, 8, true, Some((0x2, 1, false)), t20, Flow::Next),
    op(0x10, 2, false, None, t8, Flow::Bcc { taken: t10, to: 0x8 }),
    op(0x12, 6, true, Some((0x6, 1, false)), t16, Flow::Next),
    op(0x18, 6, true, Some((0x6, 1, false)), t16, Flow::Next),
    op(0x1e, 2, false, None, t8, Flow::Bcc { taken: t10, to: 0x18 }),
    op(0x20, 2, true, None, t16, Flow::Ret),
];
static SUB: &[Shape] = &[
    op(0x40, 6, true, Some((0x9a, 2, true)), t16, Flow::Next),
    op(0x46, 2, false, None, t20, Flow::Call { to: 0x60 }),
    op(0x48, 2, true, None, t16, Flow::Ret),
];
static LEAF: &[Shape] = &[
    op(0x60, 6, true, Some((0x80, 4, true)), t20, Flow::Next),
    op(0x66, 2, false, None, t4, Flow::Ret),
];

fn hand(k: (u32, u32)) -> Option<&'static [Shape]> {
    match k {
        (0, 0x0) => Some(MAIN),
        (0, 0x40) => Some(SUB),
        (0, 0x60) => Some(LEAF),
        _ => None,
    }
}

const HAND_SIZES: &[u32] = &[0x100];
const HAND_CODE: &[(u32, u32, u32)] = &[(0, 0, 0x22), (0, 0x40, 0x4a), (0, 0x60, 0x68)];

fn hand_img() -> Image<'static> {
    Image {
        bases: &BASES,
        sizes: HAND_SIZES,
        code: HAND_CODE,
    }
}

/// A walk of `MAIN` with all three writes handed and the two waits decided by the reads.
fn hand_walk() -> Walk {
    let mut w = Walk::new((0, 0), hand);
    w.write(0x96, 0x8200, 2);
    w.write(0x9a, 0xc020, 2);
    w.write(0x80, 0x1_2345, 4);
    w.decide([Decision::Busy, Decision::Same]);
    w
}

fn reads(bus: &Rec) -> Vec<(u16, u16)> {
    bus.calls
        .iter()
        .filter_map(|c| match *c {
            Call::Read(reg, v) => Some((reg, v)),
            _ => None,
        })
        .collect()
}

fn writes(bus: &Rec) -> Vec<(u16, u32, u8)> {
    bus.calls
        .iter()
        .filter_map(|c| match *c {
            Call::Write(reg, v, n) => Some((reg, v, n)),
            _ => None,
        })
        .collect()
}

#[test]
fn busy_and_same_stay_at_the_front_until_their_loops_end() {
    let mut bus = Rec::new();
    // DMACONR's byte: busy, busy, free. Then VHPOSR's: the first read, the same twice, the next.
    bus.reads.extend([0x40, 0x40, 0x00, 0x30, 0x30, 0x30, 0x31]);
    let mut r = Regs::new();
    let mut w = hand_walk();
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::End), Ok(Event::Done));
    assert_eq!(w.left(), (0, 0));
    assert_eq!(
        reads(&bus),
        [(2, 0x40), (2, 0x40), (2, 0), (6, 0x30), (6, 0x30), (6, 0x30), (6, 0x31)]
    );
    assert_eq!(writes(&bus), [(0x96, 0x8200, 2), (0x9a, 0xc020, 2), (0x80, 0x1_2345, 4)]);
    // Each wait: its branch taken (10) per loop round and not taken (8) once.
    let main = 16 + 20 + 3 * 20 + 2 * 10 + 8 + 16 + 3 * 16 + 2 * 10 + 8 + 16;
    let sub = 16 + 20 + 16;
    let leaf = 20 + 4;
    assert_eq!(r.cycles(), main + sub + leaf);
}

#[test]
fn a_wait_left_unended_stays_queued() {
    let mut bus = Rec::new();
    bus.reads.extend([0x40, 0x40]);
    let mut w = hand_walk();
    // Stop at the second DMACONR read: Busy, decided taken by the first, is still at the front.
    let mut r = Regs::new();
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::Read(0x2)), Ok(Event::Read(0x2, 0x40)));
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::Read(0x2)), Ok(Event::Read(0x2, 0x40)));
    assert_eq!(w.left(), (2, 0));
    assert_eq!(w.at(), Some((0, 0x10)));
}

#[test]
fn a_custom_write_with_no_value_stops_naming_the_register() {
    let mut w = Walk::new((0, 0), hand);
    let got = w.run(&mut Rec::new(), &mut Regs::new(), &hand_img(), Until::End);
    assert!(
        matches!(&got, Err(Stop::Unsupported { op, at }) if op.contains("writes $dff096 here") && at == "h0+$0"),
        "{got:?}"
    );
    // Until::Write stops there instead, before the instruction: nothing was spent.
    let (mut bus, mut r) = (Rec::new(), Regs::new());
    let mut w = Walk::new((0, 0), hand);
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::Write), Ok(Event::Write(0x96)));
    assert_eq!((r.cycles(), bus.calls.len(), w.at()), (0, 0, Some((0, 0))));
}

#[test]
fn a_write_of_the_wrong_register_or_size_stops() {
    for (reg, size) in [(0x9a, 2), (0x96, 4), (0x96, 1)] {
        let mut w = Walk::new((0, 0), hand);
        w.write(reg, 0, size);
        let got = w.run(&mut Rec::new(), &mut Regs::new(), &hand_img(), Until::End);
        let want = format!("the original writes $dff096 (2 bytes), the engine $dff{reg:03x} ({size})");
        assert!(
            matches!(&got, Err(Stop::Unsupported { op, .. }) if op.contains(&want)),
            "{got:?}"
        );
    }
}

#[test]
fn free_writes_write_zero() {
    let mut bus = Rec::new();
    bus.reads.extend([0, 0x30, 0x31]);
    let mut w = Walk::new((0, 0), hand);
    w.free_writes();
    w.decide([Decision::Busy, Decision::Same]);
    assert_eq!(w.run(&mut bus, &mut Regs::new(), &hand_img(), Until::End), Ok(Event::Done));
    assert_eq!(writes(&bus), [(0x96, 0, 2), (0x9a, 0, 2), (0x80, 0, 4)]);
}

#[test]
fn until_call_stops_before_the_outermost_call_and_skip_call_steps_over_it() {
    let (mut bus, mut r) = (Rec::new(), Regs::new());
    bus.reads.extend([0, 0x30, 0x31]);
    let mut w = Walk::new((0, 0), hand);
    w.write(0x96, 1, 2);
    w.decide([Decision::Busy, Decision::Same]);
    // The callee's own call of $60 is not the outermost function's: not a stop.
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::Call(0x40)), Ok(Event::Call));
    assert_eq!((w.at(), r.cycles()), (Some((0, 0x6)), 16));
    w.skip_call(&mut bus, &mut r, &hand_img()).unwrap();
    assert_eq!((w.at(), r.cycles()), (Some((0, 0x8)), 36));
    // The callee's writes are not the walk's: none is handed and none is wanted.
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::End), Ok(Event::Done));
    assert_eq!(writes(&bus), [(0x96, 1, 2)]);
    // skip_call made the call's bsr: its sync, and the block start after it at $8.
    let after: Vec<Call> = bus.timing().into_iter().skip(2).take(2).collect();
    assert_eq!(after, [Call::Insn(16, 0), Call::Sync(36, 0)]);
    assert!(matches!(bus.timing()[4], Call::Poll(36, 0)));
}

#[test]
fn until_call_with_a_write_unmade_stops() {
    let mut w = Walk::new((0, 0), hand);
    w.write(0x96, 1, 2);
    w.write(0x9a, 2, 2);
    // The second write belongs to the callee, which the caller is about to skip.
    let mut bus = Rec::new();
    let mut r = Regs::new();
    let got = w.run(&mut bus, &mut r, &hand_img(), Until::Call(0x40));
    assert!(
        matches!(&got, Err(Stop::Unsupported { op, .. }) if op.contains("the engine wrote a custom register the original does not")),
        "{got:?}"
    );
}

#[test]
fn skip_call_off_a_call_stops() {
    let mut w = Walk::new((0, 0), hand);
    let got = w.skip_call(&mut Rec::new(), &mut Regs::new(), &hand_img());
    assert!(
        matches!(&got, Err(Stop::Unsupported { op, .. }) if op.contains("not a call")),
        "{got:?}"
    );
}

#[test]
fn until_read_stops_just_after_the_read() {
    let (mut bus, mut r) = (Rec::new(), Regs::new());
    bus.reads.extend([0x00, 0x30, 0x30, 0x31]);
    let mut w = hand_walk();
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::Read(0x2)), Ok(Event::Read(0x2, 0)));
    assert_eq!(w.at(), Some((0, 0x10)));
    // A word register's byte read at its odd address names the same register.
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::Read(0x7)), Ok(Event::Read(0x6, 0x30)));
    assert_eq!(w.at(), Some((0, 0x18)));
    assert_eq!(w.run(&mut bus, &mut r, &hand_img(), Until::End), Ok(Event::Done));
}

#[test]
fn until_write_stops_at_each_unhanded_write_and_each_return_to_the_outermost() {
    let (mut bus, mut r) = (Rec::new(), Regs::new());
    bus.reads.extend([0, 0x30, 0x31]);
    let img = hand_img();
    let mut w = Walk::new((0, 0), hand);
    w.decide([Decision::Busy, Decision::Same]);
    let mut events = Vec::new();
    loop {
        let e = w.run(&mut bus, &mut r, &img, Until::Write).unwrap();
        events.push((e, w.at()));
        match e {
            Event::Write(0x96) => w.write(0x96, 1, 2),
            Event::Write(0x9a) => w.write(0x9a, 2, 2),
            Event::Write(0x80) => w.write(0x80, 3, 4),
            Event::Done => break,
            _ => {}
        }
    }
    assert_eq!(
        events,
        [
            (Event::Write(0x96), Some((0, 0x0))),
            (Event::Write(0x9a), Some((0, 0x40))),
            (Event::Write(0x80), Some((0, 0x60))),
            // The leaf's return is to depth 2, not a stop; the callee's to the outermost is.
            (Event::Returned, Some((0, 0x8))),
            (Event::Done, None),
        ]
    );
    assert_eq!(writes(&bus), [(0x96, 1, 2), (0x9a, 2, 2), (0x80, 3, 4)]);
}

#[test]
fn after_a_skipped_call_the_next_instruction_goes_on_in_its_block() {
    // flags_call: bsr $10 then beq at $2, which is no block start in the generated code.
    let img = Image {
        bases: &BASES,
        sizes: flags_call::HUNK_SIZES,
        code: flags_call::CODE,
    };
    let (mut bus, mut r) = (Rec::new(), Regs::new());
    let mut w = Walk::new((0, 0), flags_call::shapes);
    w.decide([Decision::Branch(false)]);
    assert_eq!(w.run(&mut bus, &mut r, &img, Until::Call(0x10)), Ok(Event::Call));
    w.skip_call(&mut bus, &mut r, &img).unwrap();
    assert_eq!(w.run(&mut bus, &mut r, &img, Until::End), Ok(Event::Done));
    assert_eq!(
        bus.timing(),
        [Call::Poll(0, 0), Call::Sync(0, 0), Call::Insn(0, 0), Call::Poll(8, 4), Call::Insn(8, 4), Call::Sync(8, 4)]
    );
}
