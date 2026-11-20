//! The data-dependent cycle times (translator T12, DESIGN §2.6) on Moira: multiply by the source's
//! bits, divide by its steps (overflow included), `dbcc` in its three outcomes, `movem` with 1 to 16
//! registers, on a made-up program (`tools/test_cycles_dynamic.py` lists it; `fixtures/cycles_dynamic.json`
//! is Ghidra 12.0's own P-code, `cycles_dynamic.rs` its translation). Every site passes every column on
//! 200 states. A zero divisor is the one outcome Moira's step cannot time (it reports the exception), so
//! those states are skipped, counted, and the translated run's stop is checked here instead. Regenerate
//! with `python3 tools/test_cycles_dynamic.py --regen` from `games/forest-dumb/`.

use amiga_oracle::{AmigaMemory, Cpu68k, MEM_SIZE, Regs as MRegs};
use fd_conform::judge::insn::{Columns, InsnRig, Site};
use pcode_rt::{Bus, CF, Image, NF, PC, Regs, SR, Stop, VF, XF, ZF};

// Byte for byte as pcode_rust.py writes it, which test_cycles_dynamic.py compares: never rustfmt it.
#[rustfmt::skip]
#[allow(dead_code, unused_imports, unused_mut, unused_variables)]
#[path = "fixtures/cycles_dynamic.rs"]
mod fixed;

include!("fixtures/cycles_dynamic_texts.rs");

const PROGRAM: &[u8] = include_bytes!("fixtures/cycles_dynamic.bin");
const BASE: u32 = 0x1000;
const STATES: usize = 200;

fn rig() -> InsnRig {
    let mut mem = vec![0u8; 0x8000];
    mem[BASE as usize..BASE as usize + PROGRAM.len()].copy_from_slice(PROGRAM);
    InsnRig::new(mem, &[BASE], &[PROGRAM.len() as u32])
}

fn site(text: &str) -> Site<'static> {
    let i = TEXTS.iter().position(|t| *t == text).expect(text);
    Site::of(&fixed::SITES[i])
}

#[test]
fn every_site_takes_moiras_time_in_every_column() {
    let mut rig = rig();
    let cols = Columns::parse("regs,ccr,mem,cycles").unwrap();
    for (r, text) in fixed::SITES.iter().zip(TEXTS) {
        let s = Site::of(r);
        let rep = rig.check(&s, STATES, "cycles", cols);
        assert_eq!(rep.fail, 0, "{} {text}: {:?}", s.at, rep.first);
        assert!(rep.cycles_counted, "{} {text}: not counted", s.at);
        // A divide skips its zero-divisor states, the closing rts half its own (an odd popped
        // address); nothing else may lose many.
        let least = if *text == "rts" {
            STATES / 3
        } else {
            STATES * 9 / 10
        };
        assert!(
            rep.pass >= least,
            "{} {text}: {} passed, {}",
            s.at,
            rep.pass,
            rep.first_skip
        );
    }
}

/// Moira's cycles and the translation's for the site written `text`, from d0, d1 and the flags
/// (X N Z V C as bits 4..0); a1 points at zeroed memory clear of the program, a7 at a stack.
fn both(text: &str, d0: u32, d1: u32, ccr: u16) -> (u64, u64) {
    let s = site(text);
    let pc = BASE + s.off;
    let mut mem = vec![0u8; MEM_SIZE];
    mem[BASE as usize..BASE as usize + PROGRAM.len()].copy_from_slice(PROGRAM);
    let mut cpu = Cpu68k::new(AmigaMemory {
        chip: vec![],
        slow: vec![],
    });
    cpu.memory_mut().copy_from_slice(&mem);
    let mut regs = MRegs {
        sr: 0x2700 | ccr,
        ..MRegs::default()
    };
    regs.d[0] = d0;
    regs.d[1] = d1;
    regs.a[1] = 0x4000;
    regs.a[7] = 0x7000;
    let m = cpu.step(pc, regs).expect("moira steps");

    let mut r = Regs::new();
    r.set_d(0, d0);
    r.set_d(1, d1);
    r.set_a(1, 0x4000);
    r.set_a(7, 0x7000);
    for (bit, off) in [(4, XF), (3, NF), (2, ZF), (1, VF), (0, CF)] {
        r.set(off, 1, (ccr >> bit) as u64 & 1);
    }
    r.set(SR, 2, (0x2700 | ccr) as u64);
    r.set(pcode_rt::ops::USP, 4, 0x7000);
    r.set(PC, 4, pc as u64);
    let mut bus = Plain(mem);
    (s.run)(&mut r, &mut bus, &image()).expect("the translation runs");
    (m.cycles as u64, r.cycles())
}

fn same(text: &str, d0: u32, d1: u32, ccr: u16) -> u64 {
    let (m, t) = both(text, d0, d1, ccr);
    assert_eq!(t, m, "{text} d0=${d0:x} d1=${d1:x} ccr={ccr:05b}");
    m
}

#[test]
fn multiply_by_the_named_sources() {
    // The immediates are the sources: 0, $ffff, $5555 for mulu, -1 and $aaaa for muls.
    let mut seen = Vec::new();
    for text in [
        "mulu.w #0,d0",
        "mulu.w #$ffff,d0",
        "mulu.w #$5555,d0",
        "muls.w #-1,d0",
        "muls.w #$aaaa,d0",
    ] {
        seen.push(same(text, 0x1234, 0, 0));
    }
    assert_eq!(
        seen,
        [42, 74, 58, 44, 72],
        "mulu 38+4+2n, muls by bit changes"
    );
    // And by a register, the same values.
    for d1 in [0, 0xffff, 0x5555, 0xaaaa, 0x8000_0001] {
        same("mulu.w d1,d0", 7, d1, 0);
        same("muls.w d1,d0", 7, d1, 0);
    }
}

#[test]
fn divide_on_200_random_pairs_and_overflow() {
    let mut x: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x as u32
    };
    let (mut over, mut under) = (0, 0);
    for i in 0..200 {
        let d0 = next();
        // Half the divisors small, so the quotient fits a word as often as not.
        let d1 = if i % 2 == 0 {
            next() & 0xff | 1
        } else {
            next() & 0xffff | 1
        };
        for text in ["divu.w d1,d0", "divs.w d1,d0"] {
            same(text, d0, d1, 0);
        }
        if (d0 >> 16) >= d1 & 0xffff {
            over += 1;
        } else {
            under += 1;
        }
    }
    assert!(over > 20 && under > 20, "over {over}, under {under}");
    // The edges: a quotient overflowing, signs both ways, the largest dividend.
    for (d0, d1) in [
        (0x0001_0000, 1),
        (0xffff_ffff, 0xffff),
        (0x8000_0000, 0xffff),
        (0x7fff_ffff, 2),
        (5, 0xfffd),
        (0xffff_fffb, 3),
    ] {
        same("divu.w d1,d0", d0, d1, 0);
        same("divs.w d1,d0", d0, d1, 0);
    }
    assert_eq!(
        same("divu.w d1,d0", 0x0001_0000, 1, 0),
        10,
        "an overflow's 10"
    );
}

#[test]
fn dbcc_in_its_three_outcomes() {
    const Z: u16 = 0b00100;
    // Condition true: 12, no decrement.
    assert_eq!(same("dbt d0,next", 5, 0, 0), 12);
    assert_eq!(same("dbeq d0,next", 5, 0, Z), 12);
    // False, the counter goes on: 10, the branch taken.
    assert_eq!(same("dbf d0,next", 5, 0, 0), 10);
    assert_eq!(same("dbeq d0,next", 0x1_0001, 0, 0), 10);
    // False, the counter expires (0 before the decrement): 14.
    assert_eq!(same("dbf d0,next", 0xffff_0000, 0, 0), 14);
    assert_eq!(same("dbgt d1,next", 0, 0, Z), 14);
}

#[test]
fn movem_with_1_8_and_16_registers() {
    assert_eq!(same("movem.l d0,-(a1)", 1, 2, 0), 16);
    assert_eq!(same("movem.l (a1)+,d0", 1, 2, 0), 20);
    assert_eq!(same("movem.l d0-d7,(a1)", 1, 2, 0), 72);
    assert_eq!(same("movem.l (a1)+,d0-d7", 1, 2, 0), 76);
    assert_eq!(same("movem.w d0-d7/a0-a7,16(a1)", 1, 2, 0), 76);
    same("movem.w (a1)+,d0-d7/a0/a2-a7", 1, 2, 0);
    same("movem.l d1-d7/a0-a6,-(a7)", 1, 2, 0);
    same("movem.l $4000.l,d0-d7", 1, 2, 0);
}

#[test]
fn a_zero_divisor_counts_the_exception_and_stops() {
    for text in ["divu.w d1,d0", "divs.w d1,d0"] {
        let s = site(text);
        let mut r = Regs::new();
        r.set_d(0, 1000);
        r.set(PC, 4, (BASE + s.off) as u64);
        let mut mem = Plain(vec![0u8; MEM_SIZE]);
        let stop = (s.run)(&mut r, &mut mem, &image()).unwrap_err();
        assert!(
            matches!(stop, Stop::Trap { vector: 5, .. }),
            "{text}: {stop}"
        );
        assert_eq!(r.cycles(), pcode_rt::cycles::DIV_ZERO as u64, "{text}");
    }
}

const SIZES: &[u32] = &[PROGRAM.len() as u32];
const BASES: &[u32] = &[BASE];

fn image() -> Image<'static> {
    Image {
        bases: BASES,
        sizes: SIZES,
        code: fixed::CODE,
    }
}

struct Plain(Vec<u8>);

impl Bus for Plain {
    fn rb(&mut self, a: u32) -> u8 {
        self.0[a as usize]
    }
    fn rw(&mut self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
    }
    fn rl(&mut self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a + 2) as u32
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.0[a as usize] = v;
    }
    fn ww(&mut self, a: u32, v: u16) {
        for (i, b) in v.to_be_bytes().into_iter().enumerate() {
            self.wb(a + i as u32, b);
        }
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a + 2, v as u16);
    }
}
