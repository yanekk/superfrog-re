// Generated from code_write.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x12),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x20, 0x10];

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
                // h0+$0 move.l d0,(h1+$4).l
                {
                    let v = r.get(0x0, 4);
                    store(bus, img, img.h(1, 0x4), 4, v)?;
                }
                // h0+$6 lea (h0+$0).l,a0
                {
                    let v = (img.h(0, 0x0) as u64);
                    r.set(0x20, 4, v);
                }
                // h0+$c move.w d1w,(2,a0)
                {
                    let mut u_1080: u64 = 0;
                    let v = int_add(r.get(0x20, 4), 0x2u64, 4);
                    u_1080 = v & 0xffffffff;
                    let v = r.get(0x6, 2);
                    store(bus, img, u_1080 as u32, 2, v)?;
                }
                // h0+$10 rts
                {
                    let v = load(bus, r.get(0x3c, 4) as u32, 4);
                    r.set(0x50, 4, v);
                    let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = r.get(0x50, 4);
                    r.set(PC, 4, v);
                    return Ok(());
                }
                pc = (0, 0x12);
            }
            k => return enter(k, r, bus, img),
        }
    }
}
