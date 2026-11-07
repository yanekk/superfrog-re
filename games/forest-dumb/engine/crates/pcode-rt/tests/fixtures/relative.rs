// Generated from relative.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x6),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x10];

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
                // h0+$0 made-up: d2 = d0 == d1 ? 2 : 1
                {
                    let mut u_200: u64 = 0;
                    let mut i: usize = 0;
                    'i: loop {
                        match i {
                            0 => {
                                let v = int_equal(r.get(0x0, 4), r.get(0x4, 4), 4);
                                u_200 = v & 0xff;
                            }
                            1 => {
                                if u_200 != 0 { i = 4; continue 'i; }
                            }
                            2 => {
                                let v = 0x1u64;
                                r.set(0x8, 4, v);
                            }
                            3 => {
                                i = 5; continue 'i;
                            }
                            4 => {
                                let v = 0x2u64;
                                r.set(0x8, 4, v);
                            }
                            _ => break 'i,
                        }
                        i += 1;
                    }
                }
                // h0+$4 rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0x6);
            }
            k => return enter(k, r, bus, img),
        }
    }
}
