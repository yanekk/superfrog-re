// Generated from nested_call.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, insn_start, fn_entry, PC, cycles as cy};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x8),
    (0, 0x10, 0x16),
    (0, 0x20, 0x24),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x40];

#[allow(dead_code)]
pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The translated function entered at `addr` (`hN+$off`).
pub fn entry(addr: &str) -> Option<Routine> {
    match addr {
        "h0+$0" => Some(h0_0),
        "h0+$10" => Some(h0_10),
        "h0+$20" => Some(h0_20),
        _ => None,
    }
}

#[allow(dead_code)]
fn by_addr(k: (u32, u32)) -> Option<Routine> {
    match k {
        (0, 0x0) => Some(h0_0),
        (0, 0x10) => Some(h0_10),
        (0, 0x20) => Some(h0_20),
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
        | (0, 0x4)
        | (0, 0x6) => Some(h0_0_from),
        (0, 0x10)
        | (0, 0x12)
        | (0, 0x14) => Some(h0_10_from),
        (0, 0x20)
        | (0, 0x22) => Some(h0_20_from),
        _ => None,
    }
}

/// The shapes of the instructions of the function entered at `k`, by offset (translator T56).
#[allow(dead_code)]
pub fn shapes(k: (u32, u32)) -> Option<&'static [pcode_rt::shape::Shape]> {
    match k {
        (0, 0x0) => Some(SHAPES_h0_0),
        (0, 0x10) => Some(SHAPES_h0_10),
        (0, 0x20) => Some(SHAPES_h0_20),
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
            (0, 0x0) | (0, 0x4) | (0, 0x6) => {
                let entered = pc.1;
                if entered <= 0x0 {
                    // h0+$0 bsr h0+$10
                    {
                        bus.sync(r);
                        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = (img.h(0, 0x4) as u64);
                        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                        h0_10(r, bus, img)?;
                        let ret = r.get(PC, 4) as u32;
                        if ret != img.h(0, 0x4) {
                            let a = ret as u32;
                            match img.hunk_of(a) {
                                Some(k) => { pc = k; continue 'b; }
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            }
                        }
                    }
                }
                if entered <= 0x4 {
                    insn_start(bus, r, img)?;
                    // h0+$4 move.l d2,d3
                    {
                        let v = r.get(0x8, 4);
                        r.set(0xc, 4, v);
                    }
                }
                if entered <= 0x6 {
                    insn_start(bus, r, img)?;
                    // h0+$6 rts
                    {
                        bus.sync(r);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0x8);
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
    pcode_rt::shape::Shape { h: 0, at: 0x0, len: 4, first: true, mem: true, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Call { to: 0x10 } },
    pcode_rt::shape::Shape { h: 0, at: 0x4, len: 2, first: false, mem: false, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Next },
    pcode_rt::shape::Shape { h: 0, at: 0x6, len: 2, first: false, mem: true, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Ret },
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x10)) {
        return f(r, bus, img);
    }
    h0_10_from((0, 0x10), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_10_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x10) | (0, 0x12) | (0, 0x14) => {
                let entered = pc.1;
                if entered <= 0x10 {
                    // h0+$10 bsr h0+$20
                    {
                        bus.sync(r);
                        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = (img.h(0, 0x12) as u64);
                        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                        h0_20(r, bus, img)?;
                        let ret = r.get(PC, 4) as u32;
                        if ret != img.h(0, 0x12) {
                            let a = ret as u32;
                            match img.hunk_of(a) {
                                Some(k) => { pc = k; continue 'b; }
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            }
                        }
                    }
                }
                if entered <= 0x12 {
                    insn_start(bus, r, img)?;
                    // h0+$12 addq.l #1,d2
                    {
                        let v = int_add(r.get(0x8, 4), 0x1u64, 4);
                        r.set(0x8, 4, v);
                    }
                }
                if entered <= 0x14 {
                    insn_start(bus, r, img)?;
                    // h0+$14 rts
                    {
                        bus.sync(r);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0x16);
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
pub static SHAPES_h0_10: &[pcode_rt::shape::Shape] = &[
    pcode_rt::shape::Shape { h: 0, at: 0x10, len: 2, first: true, mem: true, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Call { to: 0x20 } },
    pcode_rt::shape::Shape { h: 0, at: 0x12, len: 2, first: false, mem: false, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Next },
    pcode_rt::shape::Shape { h: 0, at: 0x14, len: 2, first: false, mem: true, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Ret },
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_20(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x20)) {
        return f(r, bus, img);
    }
    h0_20_from((0, 0x20), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_20_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x20) | (0, 0x22) => {
                let entered = pc.1;
                if entered <= 0x20 {
                    // h0+$20 moveq #5,d2
                    {
                        let v = 0x5u64;
                        r.set(0x8, 4, v);
                    }
                }
                if entered <= 0x22 {
                    insn_start(bus, r, img)?;
                    // h0+$22 rts
                    {
                        bus.sync(r);
                        let v = load(bus, r.get(0x3c, 4) as u32, 4);
                        r.set(0x50, 4, v);
                        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = r.get(0x50, 4);
                        r.set(PC, 4, v);
                        r.depth -= 1;
                        return Ok(());
                    }
                }
                pc = (0, 0x24);
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
pub static SHAPES_h0_20: &[pcode_rt::shape::Shape] = &[
    pcode_rt::shape::Shape { h: 0, at: 0x20, len: 2, first: true, mem: false, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Next },
    pcode_rt::shape::Shape { h: 0, at: 0x22, len: 2, first: false, mem: true, dynamic: false, uncounted: false, custom: None, tick: |_: &mut Regs| {}, flow: pcode_rt::shape::Flow::Ret },
];
