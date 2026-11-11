// Generated from shared_call.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP};

#[path = "p00.rs"]
mod p00;
#[path = "p01.rs"]
mod p01;

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0xa),
    (0, 0x10, 0x18),
    (0, 0x20, 0x28),
    (0, 0x30, 0x34),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x40];

#[allow(dead_code)]
pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The translated function entered at `addr` (`hN+$off`).
pub fn entry(addr: &str) -> Option<Routine> {
    match addr {
        "h0+$0" => Some(p00::h0_0),
        "h0+$10" => Some(p00::h0_10),
        "h0+$20" => Some(p01::h0_20),
        "h0+$30" => Some(p01::h0_30),
        _ => None,
    }
}

#[allow(dead_code)]
fn by_addr(k: (u32, u32)) -> Option<Routine> {
    match k {
        (0, 0x0) => Some(p00::h0_0),
        (0, 0x10) => Some(p00::h0_10),
        (0, 0x20) => Some(p01::h0_20),
        (0, 0x30) => Some(p01::h0_30),
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
