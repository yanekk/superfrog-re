// Generated from jump_table.json; never committed when made from a game.
// By pcode_rust.py (forest-dumb-translator-trial T03). Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

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
                // h0+$0 add.w d0w,d0w
                {
                    let v = int_add(r.get(0x2, 2), r.get(0x2, 2), 2);
                    r.set(0x2, 2, v);
                }
                // h0+$2 add.w d0w,d0w
                {
                    let v = int_add(r.get(0x2, 2), r.get(0x2, 2), 2);
                    r.set(0x2, 2, v);
                }
                // h0+$4 lea (h1+$0).l,a0
                {
                    let v = (img.h(1, 0x0) as u64);
                    r.set(0x20, 4, v);
                }
                // h0+$a movea.l (0,a0,d0w.w),a1
                {
                    let mut u_1100: u64 = 0;
                    let mut u_1180: u64 = 0;
                    let v = int_sext(r.get(0x2, 2), 2, 4);
                    u_1100 = v & 0xffffffff;
                    let v = int_add(r.get(0x20, 4), u_1100, 4);
                    u_1180 = v & 0xffffffff;
                    let v = load(bus, u_1180 as u32, 4);
                    r.set(0x24, 4, v);
                }
                // h0+$e jmp (a1)
                {
                    let a = r.get(0x24, 4) as u32;
                    match img.hunk_of(a) {
                        Some(k) => { pc = k; continue 'b; }
                        None => return Err(Stop::Untranslated(img.describe(a))),
                    }
                }
                pc = (0, 0x10);
            }
            k => return enter(k, r, bus, img),
        }
    }
}
