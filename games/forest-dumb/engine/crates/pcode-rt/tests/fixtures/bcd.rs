// Generated from bcd.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0xe),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x20];

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
                // h0+$0 cmpi.b #1,d3b
                {
                    let mut u_20b00: u64 = 0;
                    let mut u_20c00: u64 = 0;
                    let v = r.get(0xf, 1);
                    u_20b00 = v & 0xff;
                    let v = int_less(u_20b00, 0x1u64, 1);
                    r.set(0x47, 1, v);
                    let v = int_sborrow(u_20b00, 0x1u64, 1);
                    r.set(0x46, 1, v);
                    let v = int_sub(u_20b00, 0x1u64, 1);
                    u_20c00 = v & 0xff;
                    let v = int_sless(u_20c00, 0x0u64, 1);
                    r.set(0x44, 1, v);
                    let v = int_equal(u_20c00, 0x0u64, 1);
                    r.set(0x45, 1, v);
                }
                // h0+$4 abcd d1b,d0b (corrected: Ghidra's P-code is wrong)
                {
                    let (v, x, o) = pcode_rt::bcd::bcd_add(r.get(0x7, 1), r.get(0x3, 1), r.get(0x43, 1));
                    r.set(0x3, 1, v);
                    pcode_rt::bcd::bcd_flags(r, v, x, o);
                }
                // h0+$6 abcd -(a1),-(a0) (corrected: Ghidra's P-code is wrong)
                {
                    let ay = (r.get(0x24, 4) as u32).wrapping_sub(1);
                    r.set(0x24, 4, ay as u64);
                    let s = load(bus, ay, 1);
                    let ax = (r.get(0x20, 4) as u32).wrapping_sub(1);
                    r.set(0x20, 4, ax as u64);
                    let d = load(bus, ax, 1);
                    let (v, x, o) = pcode_rt::bcd::bcd_add(s, d, r.get(0x43, 1));
                    store(bus, img, ax, 1, v)?;
                    pcode_rt::bcd::bcd_flags(r, v, x, o);
                }
                // h0+$8 abcd -(a1),-(a0) (corrected: Ghidra's P-code is wrong)
                {
                    let ay = (r.get(0x24, 4) as u32).wrapping_sub(1);
                    r.set(0x24, 4, ay as u64);
                    let s = load(bus, ay, 1);
                    let ax = (r.get(0x20, 4) as u32).wrapping_sub(1);
                    r.set(0x20, 4, ax as u64);
                    let d = load(bus, ax, 1);
                    let (v, x, o) = pcode_rt::bcd::bcd_add(s, d, r.get(0x43, 1));
                    store(bus, img, ax, 1, v)?;
                    pcode_rt::bcd::bcd_flags(r, v, x, o);
                }
                // h0+$a sbcd d2b,d4b (corrected: Ghidra's P-code is wrong)
                {
                    let (v, x, o) = pcode_rt::bcd::bcd_sub(r.get(0xb, 1), r.get(0x13, 1), r.get(0x43, 1));
                    r.set(0x13, 1, v);
                    pcode_rt::bcd::bcd_flags(r, v, x, o);
                }
                // h0+$c rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0xe);
            }
            k => return enter(k, r, bus, img),
        }
    }
}
