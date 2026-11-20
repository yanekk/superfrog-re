// Generated from jump_table.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, insn_start, fn_entry, PC, cycles as cy};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x10),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x40, 0x10];

#[allow(dead_code)]
pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The translated function entered at `addr` (`hN+$off`).
pub fn entry(addr: &str) -> Option<Routine> {
    match addr {
        "h0+$0" => Some(h0_0),
        _ => None,
    }
}

#[allow(dead_code)]
fn by_addr(k: (u32, u32)) -> Option<Routine> {
    match k {
        (0, 0x0) => Some(h0_0),
        _ => None,
    }
}

/// Control reached `k`, which is no block of the function it was in: enter the translated
/// function there, whose return our caller checks, or stop.
#[allow(dead_code)]
fn enter(k: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    match by_addr(k) {
        Some(f) => f(r, bus, img),
        None => Err(Stop::Untranslated(format!("h{}+${:x}", k.0, k.1))),
    }
}

#[allow(dead_code)]
pub type Resume = fn((u32, u32), &mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The translated function holding the instruction at `k`, entered at it.
#[allow(dead_code)]
pub fn resume(k: (u32, u32)) -> Option<Resume> {
    match k {
        (0, 0x0)
        | (0, 0x2)
        | (0, 0x4)
        | (0, 0xa)
        | (0, 0xe) => Some(h0_0_from),
        _ => None,
    }
}

/// The shapes of the instructions of the function entered at `k`, by offset (translator T56).
#[allow(dead_code)]
pub fn shapes(k: (u32, u32)) -> Option<&'static [pcode_rt::shape::Shape]> {
    match k {
        (0, 0x0) => Some(SHAPES_h0_0),
        _ => None,
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x0)) {
        return f(r, bus, img);
    }
    h0_0_from((0, 0x0), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_0_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x0) | (0, 0x2) | (0, 0x4) | (0, 0xa) | (0, 0xe) => {
                let entered = pc.1;
                if entered <= 0x0 {
                    // h0+$0 add.w d0w,d0w
                    {
                        let v = int_add(r.get(0x2, 2), r.get(0x2, 2), 2);
                        r.set(0x2, 2, v);
                    }
                }
                if entered <= 0x2 {
                    insn_start(bus, r, img)?;
                    // h0+$2 add.w d0w,d0w
                    {
                        let v = int_add(r.get(0x2, 2), r.get(0x2, 2), 2);
                        r.set(0x2, 2, v);
                    }
                }
                if entered <= 0x4 {
                    insn_start(bus, r, img)?;
                    // h0+$4 lea (h1+$0).l,a0
                    {
                        let v = (img.h(1, 0x0) as u64);
                        r.set(0x20, 4, v);
                    }
                }
                if entered <= 0xa {
                    insn_start(bus, r, img)?;
                    // h0+$a movea.l (0,a0,d0w.w),a1
                    {
                        bus.sync(r);
                        let mut u_1100: u64 = 0;
                        let mut u_1180: u64 = 0;
                        let v = int_sext(r.get(0x2, 2), 2, 4);
                        u_1100 = v & 0xffffffff;
                        let v = int_add(r.get(0x20, 4), u_1100, 4);
                        u_1180 = v & 0xffffffff;
                        let v = load(bus, u_1180 as u32, 4);
                        r.set(0x24, 4, v);
                    }
                }
                if entered <= 0xe {
                    insn_start(bus, r, img)?;
                    // h0+$e jmp (a1)
                    {
                        let a = r.get(0x24, 4) as u32;
                        match img.hunk_of(a) {
                            Some(k) => { pc = k; continue 'b; }
                            None => return Err(Stop::Untranslated(img.describe(a))),
                        }
                    }
                }
                pc = (0, 0x10);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(non_upper_case_globals)]
pub static SHAPES_h0_0: &[pcode_rt::shape::Shape] = &[
    pcode_rt::shape::Shape { h: 0, at: 0x0, len: 2, first: true, mem: false, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Next },
    pcode_rt::shape::Shape { h: 0, at: 0x2, len: 2, first: false, mem: false, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Next },
    pcode_rt::shape::Shape { h: 0, at: 0x4, len: 6, first: false, mem: false, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Next },
    pcode_rt::shape::Shape { h: 0, at: 0xa, len: 4, first: false, mem: true, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Next },
    pcode_rt::shape::Shape { h: 0, at: 0xe, len: 2, first: false, mem: false, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::JumpInd },
];
