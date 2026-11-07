// Generated from dbf_loop.json; never committed when made from a game.
// By pcode_rust.py (forest-dumb-translator-trial T03). Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0xc),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x40];

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
                // h0+$0 moveq #3,d0
                {
                    let v = 0x3u64;
                    r.set(0x0, 4, v);
                    let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
                    r.set(0x44, 1, v);
                    let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
                    r.set(0x45, 1, v);
                    let v = 0x0u64;
                    r.set(0x46, 1, v);
                    let v = 0x0u64;
                    r.set(0x47, 1, v);
                }
                // h0+$2 moveq #0,d1
                {
                    let v = 0x0u64;
                    r.set(0x4, 4, v);
                    let v = int_sless(r.get(0x4, 4), 0x0u64, 4);
                    r.set(0x44, 1, v);
                    let v = int_equal(r.get(0x4, 4), 0x0u64, 4);
                    r.set(0x45, 1, v);
                    let v = 0x0u64;
                    r.set(0x46, 1, v);
                    let v = 0x0u64;
                    r.set(0x47, 1, v);
                }
                pc = (0, 0x4);
            }
            (0, 0x4) => {
                // h0+$4 addq.l #2,d1
                {
                    let v = int_carry(r.get(0x4, 4), 0x2u64, 4);
                    r.set(0x47, 1, v);
                    let v = int_scarry(r.get(0x4, 4), 0x2u64, 4);
                    r.set(0x46, 1, v);
                    let v = r.get(0x47, 1);
                    r.set(0x43, 1, v);
                    let v = int_add(r.get(0x4, 4), 0x2u64, 4);
                    r.set(0x4, 4, v);
                    let v = int_sless(r.get(0x4, 4), 0x0u64, 4);
                    r.set(0x44, 1, v);
                    let v = int_equal(r.get(0x4, 4), 0x0u64, 4);
                    r.set(0x45, 1, v);
                }
                // h0+$6 dbf d0w,h0+$4
                {
                    let mut u_1300: u64 = 0;
                    if 0x0u64 != 0 { pc = (0, 0xa); continue 'b; }
                    let v = int_sub(r.get(0x2, 2), 0x1u64, 2);
                    r.set(0x2, 2, v);
                    let v = int_notequal(r.get(0x2, 2), 0xffffu64, 2);
                    u_1300 = v & 0xff;
                    if u_1300 != 0 { pc = (0, 0x4); continue 'b; }
                }
                pc = (0, 0xa);
            }
            (0, 0xa) => {
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
