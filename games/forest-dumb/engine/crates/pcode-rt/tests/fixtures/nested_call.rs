// Generated from nested_call.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

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

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let mut pc: (u32, u32) = (0, 0x0);
    let mut steps: u64 = 0;
    'b: loop {
        steps += 1;
        if steps > STEP_CAP {
            return Err(Stop::Hang);
        }
        match pc {
            (0, 0x0) => {
                // h0+$0 bsr h0+$10
                {
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
                // h0+$4 move.l d2,d3
                {
                    let v = r.get(0x8, 4);
                    r.set(0xc, 4, v);
                }
                // h0+$6 rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0x8);
            }
            k => return enter(k, r, bus, img),
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let mut pc: (u32, u32) = (0, 0x10);
    let mut steps: u64 = 0;
    'b: loop {
        steps += 1;
        if steps > STEP_CAP {
            return Err(Stop::Hang);
        }
        match pc {
            (0, 0x10) => {
                // h0+$10 bsr h0+$20
                {
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
                // h0+$12 addq.l #1,d2
                {
                    let v = int_add(r.get(0x8, 4), 0x1u64, 4);
                    r.set(0x8, 4, v);
                }
                // h0+$14 rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0x16);
            }
            k => return enter(k, r, bus, img),
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_20(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let mut pc: (u32, u32) = (0, 0x20);
    let mut steps: u64 = 0;
    'b: loop {
        steps += 1;
        if steps > STEP_CAP {
            return Err(Stop::Hang);
        }
        match pc {
            (0, 0x20) => {
                // h0+$20 moveq #5,d2
                {
                    let v = 0x5u64;
                    r.set(0x8, 4, v);
                }
                // h0+$22 rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0x24);
            }
            k => return enter(k, r, bus, img),
        }
    }
}
