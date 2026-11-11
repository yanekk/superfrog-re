//! The shift_bit family on Moira (translator T09): shifts, rotates, bit ops, scc, tas, swap, on a
//! made-up program of every encoding the family's rules turn on (`tools/test_shift_bit.py` lists
//! them). `fixtures/shift_bit.json` is Ghidra 12.0's own P-code for it; `shift_bit.rs` is its
//! translation with every `pcode_fix` correction, `shift_bit_raw.rs` without `pcode_fix.shift_bit`'s.
//! The corrected one passes every state; the raw one fails exactly where a correction says, so each
//! correction has a test that fails without it. Regenerate with `python3 tools/test_shift_bit.py
//! --regen` from `games/forest-dumb/` (`tools/test_shift_bit.py` checks they are current).

use amiga_oracle::{AmigaMemory, Cpu68k, MEM_SIZE, Regs as MRegs};
use fd_conform::judge::insn::{Columns, InsnRig, Site};
use pcode_rt::{Bus, CF, Image, NF, PC, Regs, SR, VF, XF, ZF};

// Byte for byte as pcode_rust.py writes them, which test_shift_bit.py compares: never rustfmt them.
#[rustfmt::skip]
#[allow(dead_code, unused_imports, unused_mut, unused_variables)]
#[path = "fixtures/shift_bit.rs"]
mod fixed;
#[rustfmt::skip]
#[allow(dead_code, unused_imports, unused_mut, unused_variables)]
#[path = "fixtures/shift_bit_raw.rs"]
mod raw;

include!("fixtures/shift_bit_texts.rs");

const PROGRAM: &[u8] = include_bytes!("fixtures/shift_bit.bin");
const BASE: u32 = 0x1000;
const STATES: usize = 200;
/// Where a0 points in the direct tests: clear of the program.
const DATA: u32 = 0x4000;

fn memory() -> Vec<u8> {
    let mut mem = vec![0u8; 0x8000];
    mem[BASE as usize..BASE as usize + PROGRAM.len()].copy_from_slice(PROGRAM);
    mem
}

fn rig() -> InsnRig {
    InsnRig::new(memory(), &[BASE], &[PROGRAM.len() as u32])
}

fn cols() -> Columns {
    Columns::parse("regs,ccr,mem").unwrap()
}

type Row = (
    &'static str,
    &'static str,
    u32,
    u32,
    u32,
    &'static [(u32, u32)],
    fd_conform::judge::insn::Entry,
);

fn row(sites: &'static [Row], at: &str) -> &'static Row {
    sites.iter().find(|r| r.1 == at).expect(at)
}

/// The text of a fixture site, as `tools/test_shift_bit.py` writes it.
fn text(at: &str) -> &'static str {
    TEXTS[fixed::SITES.iter().position(|r| r.1 == at).expect(at)]
}

#[test]
fn every_encoding_passes_regs_ccr_and_mem_on_moira() {
    let mut rig = rig();
    assert_eq!(fixed::SITES.len(), TEXTS.len());
    for r in fixed::SITES {
        let s = Site::of(r);
        let rep = rig.check(&s, STATES, "shift_bit", cols());
        assert_eq!(rep.fail, 0, "{} {}: {:?}", s.at, text(s.at), rep.first);
        // Odd a0 takes an address error on the word memory forms: skipped, never most states.
        assert!(
            rep.pass > STATES / 3,
            "{} {}: {}",
            s.at,
            text(s.at),
            rep.first_skip
        );
    }
}

/// Where Ghidra's own P-code fails a register-form shift, the only form the correction takes, as
/// (column, what) of the first failure: V from the sign before and after (a logical shift must clear
/// it; `asl` must set it on a change at any step, which a count of 1 cannot miss), the value of a
/// rotate by a register count, X of `asr` by a register. Every other site's raw translation is the
/// corrected one, which the test above passes.
fn raw_fails(text: &str) -> Option<(&'static str, &'static str)> {
    let (mnem, operands) = text.split_once(' ')?;
    let by_register = operands.starts_with("d1");
    match mnem.split('.').next()? {
        "lsl" | "lsr" => Some(("ccr", "V")),
        "asl" if operands != "#1,d0" => Some(("ccr", "V")),
        "asr" if by_register => Some(("ccr", "X")),
        "rol" | "ror" | "roxl" | "roxr" if by_register => Some(("regs", "d0")),
        _ => None,
    }
}

/// A shift or rotate of d0, by d1 or an immediate.
fn register_shift(text: &str) -> bool {
    text.starts_with(['a', 'l', 'r']) && text.contains('.') && text.ends_with(",d0")
}

#[test]
fn without_the_shift_bit_correction_ghidras_p_code_fails_where_it_says() {
    let mut rig = rig();
    let mut corrected = 0;
    for r in raw::SITES {
        let s = Site::of(r);
        let t = text(s.at);
        if !register_shift(t) {
            continue;
        }
        corrected += 1;
        let rep = rig.check(&s, STATES, "shift_bit", cols());
        let got = rep.first.as_ref().map(|f| (f.column, f.what.as_str()));
        assert_eq!(got, raw_fails(t), "{} {t}: {:?}", s.at, rep.first);
        if got.is_some() {
            assert!(rep.fail > STATES / 10, "{t}: only {} fail", rep.fail);
        }
    }
    // Eight kinds, two directions, three sizes, by d1, #1 and #8.
    assert_eq!(corrected, 4 * 2 * 3 * 3);
}

fn ram() -> Vec<u8> {
    let mut mem = memory();
    mem.resize(MEM_SIZE, 0);
    mem
}

struct Ram(Vec<u8>);

impl Bus for Ram {
    fn rb(&mut self, a: u32) -> u8 {
        self.0[a as usize]
    }
    fn rw(&mut self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
    }
    fn rl(&mut self, a: u32) -> u32 {
        ((self.rw(a) as u32) << 16) | self.rw(a + 2) as u32
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.0[a as usize] = v;
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.0[a as usize..a as usize + 2].copy_from_slice(&v.to_be_bytes());
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.0[a as usize..a as usize + 4].copy_from_slice(&v.to_be_bytes());
    }
}

/// (d0, X N Z V C as bits 4..0, the byte at a0) after the instruction at `at` from d0, d1, the
/// flags and that byte, on Moira and translated; asserts the two agree and returns them.
fn run(at: &str, d0: u32, d1: u32, ccr: u16, byte: u8) -> (u32, u16, u8) {
    let r = row(fixed::SITES, at);
    let pc = BASE + r.3;
    let mut mem = ram();
    mem[DATA as usize] = byte;
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
    regs.a[0] = DATA;
    regs.a[7] = 0x7000;
    let m = cpu.step(pc, regs).expect("moira steps");
    let mut moira_byte = byte;
    for &(a, _, new) in &m.writes {
        if a == DATA {
            moira_byte = new;
        }
    }

    let mut t = Regs::new();
    t.set_d(0, d0);
    t.set_d(1, d1);
    t.set_a(0, DATA);
    t.set_a(7, 0x7000);
    for (bit, off) in [(4, XF), (3, NF), (2, ZF), (1, VF), (0, CF)] {
        t.set(off, 1, (ccr >> bit) as u64 & 1);
    }
    t.set(SR, 2, (0x2700 | ccr) as u64);
    t.set(PC, 4, pc as u64);
    let code = [(0, r.3, r.3 + r.4)];
    let img = Image {
        bases: &[BASE],
        sizes: &[PROGRAM.len() as u32],
        code: &code,
    };
    let mut bus = Ram(mem);
    (r.6)(&mut t, &mut bus, &img).expect("translated runs");
    let mut tccr = 0u16;
    for (bit, off) in [(4, XF), (3, NF), (2, ZF), (1, VF), (0, CF)] {
        tccr |= (t.get(off, 1) as u16 & 1) << bit;
    }
    let got = (t.d(0), tccr, bus.0[DATA as usize]);
    assert_eq!(
        got,
        (m.regs.d[0], m.regs.sr & 0x1f, moira_byte),
        "{at}: translated vs moira"
    );
    got
}

const X: u16 = 0x10;
const Z: u16 = 0x04;
const V: u16 = 0x02;
const C: u16 = 0x01;

/// `hN+$off` of an encoding by its text in `tools/test_shift_bit.py`, as the fixture's SITES has it.
fn site(text: &str) -> &'static str {
    let i = TEXTS.iter().position(|t| *t == text).expect(text);
    fixed::SITES[i].1
}

#[test]
fn a_shift_by_a_register_count_of_zero_clears_c_and_keeps_x() {
    for t in [
        "asl.l d1,d0",
        "asr.w d1,d0",
        "lsl.b d1,d0",
        "lsr.l d1,d0",
        "rol.w d1,d0",
        "ror.l d1,d0",
    ] {
        // Count 64 is 0 mod 64.
        for (d1, x) in [(0, X), (64, X), (0, 0)] {
            let (d0, ccr, _) = run(site(t), 0x8000_8081, d1, x | C | V, 0);
            assert_eq!(d0, 0x8000_8081, "{t}");
            assert_eq!(ccr & (X | C | V), x, "{t} d1={d1}");
        }
    }
}

#[test]
fn roxl_and_roxr_by_zero_copy_x_to_c() {
    for t in ["roxl.l d1,d0", "roxr.b d1,d0", "roxl.w d1,d0"] {
        for x in [0, X] {
            let (d0, ccr, _) = run(site(t), 0x1234_5678, 0, x | (C * (x == 0) as u16), 0);
            assert_eq!(d0, 0x1234_5678, "{t}");
            assert_eq!(ccr & (X | C), x | (x >> 4), "{t} x={x}");
        }
    }
}

#[test]
fn asl_sets_v_when_the_sign_changes_at_any_step() {
    // $40 << 2 is $00 in a byte: the sign is 0 before and after, but 1 after the first step.
    let (d0, ccr, _) = run(site("asl.b d1,d0"), 0x40, 2, 0, 0);
    assert_eq!((d0, ccr & V), (0, V));
    // $c0 << 1 keeps the sign: no change, no V.
    let (_, ccr, _) = run(site("asl.b d1,d0"), 0xc0, 1, 0, 0);
    assert_eq!(ccr & V, 0);
    // $c0 << 2 changes it at the second step.
    let (_, ccr, _) = run(site("asl.b d1,d0"), 0xc0, 2, 0, 0);
    assert_eq!(ccr & V, V);
    // By 40, past the size: V unless the value was 0.
    let (_, ccr, _) = run(site("asl.l d1,d0"), 0xffff_ffff, 40, 0, 0);
    assert_eq!(ccr & (V | C | Z), V | Z);
    let (_, ccr, _) = run(site("asl.l d1,d0"), 0, 40, 0, 0);
    assert_eq!(ccr & V, 0);
    // By 8 on a long, through the immediate form the game uses.
    let (_, ccr, _) = run(site("asl.l #8,d0"), 0x00c0_0000, 0, 0, 0);
    assert_eq!(ccr & V, V);
}

#[test]
fn a_logical_shift_never_sets_v() {
    for t in ["lsl.l d1,d0", "lsl.b #8,d0", "lsr.w #1,d0"] {
        let (_, ccr, _) = run(site(t), 0x4000_4040, 1, V, 0);
        assert_eq!(ccr & V, 0, "{t}");
    }
}

#[test]
fn btst_counts_the_bit_mod_32_on_a_register_and_mod_8_in_memory() {
    // Bit 33 of d0 is bit 1; bit 9 of the byte is bit 1.
    let (_, ccr, _) = run(site("btst d1,d0"), 0b10, 33, 0, 0);
    assert_eq!(ccr & Z, 0);
    let (_, ccr, _) = run(site("btst d1,d0"), 0b01, 33, 0, 0);
    assert_eq!(ccr & Z, Z);
    let (_, ccr, _) = run(site("btst d1,(a0)"), 0, 9, 0, 0b10);
    assert_eq!(ccr & Z, 0);
    let (_, ccr, _) = run(site("btst d1,(a0)"), 0, 9, 0, 0b01);
    assert_eq!(ccr & Z, Z);
    let (_, ccr, _) = run(site("btst #33,d0"), 0b10, 0, 0, 0);
    assert_eq!(ccr & Z, 0);
    let (_, ccr, _) = run(site("btst #9,(a0)"), 0, 0, 0, 0b10);
    assert_eq!(ccr & Z, 0);
    let (d0, _, _) = run(site("bset d1,d0"), 0, 33, 0, 0);
    assert_eq!(d0, 0b10);
    let (_, _, b) = run(site("bchg d1,(a0)"), 0, 9, 0, 0xff);
    assert_eq!(b, 0xfd);
}
