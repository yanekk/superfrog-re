// Generated from self_call.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC, cycles as cy};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x4),
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
    fn_entry(r)?;
    let mut pc: (u32, u32) = (0, 0x0);
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x0) => {
                // h0+$0 bsr.b h0+$0
                {
                    let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = (img.h(0, 0x2) as u64);
                    store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                    h0_0(r, bus, img)?;
                    let ret = r.get(PC, 4) as u32;
                    if ret != img.h(0, 0x2) {
                        let a = ret as u32;
                        match img.hunk_of(a) {
                            Some(k) => { pc = k; continue 'b; }
                            None => return Err(Stop::Untranslated(img.describe(a))),
                        }
                    }
                }
                // h0+$2 rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    r.depth -= 1;
                    return Ok(());
                }
                pc = (0, 0x4);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}
