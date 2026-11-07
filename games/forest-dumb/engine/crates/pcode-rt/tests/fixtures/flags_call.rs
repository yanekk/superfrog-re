// Generated from flags_call.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0xc),
    (0, 0x10, 0x14),
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
        _ => None,
    }
}

#[allow(dead_code)]
fn by_addr(k: (u32, u32)) -> Option<Routine> {
    match k {
        (0, 0x0) => Some(h0_0),
        (0, 0x10) => Some(h0_10),
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
                    let v = (img.h(0, 0x2) as u64);
                    store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                    h0_10(r, bus, img)?;
                    let ret = r.get(PC, 4) as u32;
                    if ret != img.h(0, 0x2) {
                        let a = ret as u32;
                        match img.hunk_of(a) {
                            Some(k) => { pc = k; continue 'b; }
                            None => return Err(Stop::Untranslated(img.describe(a))),
                        }
                    }
                }
                // h0+$2 beq.b h0+$8
                {
                    if r.get(0x45, 1) != 0 { pc = (0, 0x8); continue 'b; }
                }
                pc = (0, 0x4);
            }
            (0, 0x4) => {
                // h0+$4 moveq #1,d2
                {
                    let v = 0x1u64;
                    r.set(0x8, 4, v);
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
            (0, 0x8) => {
                // h0+$8 moveq #2,d2
                {
                    let v = 0x2u64;
                    r.set(0x8, 4, v);
                }
                // h0+$a rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0xc);
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
                // h0+$10 cmp.l d0,d1
                {
                    let mut u_2500: u64 = 0;
                    let v = int_less(r.get(0x4, 4), r.get(0x0, 4), 4);
                    r.set(0x47, 1, v);
                    let v = int_sborrow(r.get(0x4, 4), r.get(0x0, 4), 4);
                    r.set(0x46, 1, v);
                    let v = r.get(0x47, 1);
                    r.set(0x43, 1, v);
                    let v = int_sub(r.get(0x4, 4), r.get(0x0, 4), 4);
                    u_2500 = v & 0xffffffff;
                    let v = int_sless(u_2500, 0x0u64, 4);
                    r.set(0x44, 1, v);
                    let v = int_equal(u_2500, 0x0u64, 4);
                    r.set(0x45, 1, v);
                }
                // h0+$12 rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0x14);
            }
            k => return enter(k, r, bus, img),
        }
    }
}
