// Generated from os_call.json; never committed when made from a game.
// By pcode_rust.py. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC, cycles as cy};

/// The bytes of every translated instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x3e),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x40, 0x80];

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

#[allow(dead_code)]
pub type Resume = fn((u32, u32), &mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The translated function holding the instruction at `k`, entered at it.
#[allow(dead_code)]
pub fn resume(k: (u32, u32)) -> Option<Resume> {
    match k {
        (0, 0x0)
        | (0, 0x4)
        | (0, 0xa)
        | (0, 0xe)
        | (0, 0x10)
        | (0, 0x16)
        | (0, 0x1c)
        | (0, 0x20)
        | (0, 0x22)
        | (0, 0x24)
        | (0, 0x2a)
        | (0, 0x30)
        | (0, 0x34)
        | (0, 0x36)
        | (0, 0x38)
        | (0, 0x3c) => Some(h0_0_from),
        _ => None,
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    h0_0_from((0, 0x0), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_0_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x0) | (0, 0x4) | (0, 0xa) | (0, 0xe) | (0, 0x10) | (0, 0x16) | (0, 0x1c) | (0, 0x20) | (0, 0x22) | (0, 0x24) | (0, 0x2a) | (0, 0x30) | (0, 0x34) | (0, 0x36) | (0, 0x38) | (0, 0x3c) => {
                let entered = pc.1;
                if entered <= 0x0 {
                    // h0+$0 movea.l ($4).w,a6
                    {
                        let v = load(bus, 0x4u64 as u32, 4);
                        r.set(0x38, 4, v);
                    }
                }
                if entered <= 0x4 {
                    // h0+$4 lea (h1+$0).l,a1
                    {
                        let v = (img.h(1, 0x0) as u64);
                        r.set(0x24, 4, v);
                    }
                }
                if entered <= 0xa {
                    // h0+$a jsr (-408,a6)
                    {
                        let mut u_100: u64 = 0;
                        let v = int_add(r.get(0x38, 4), 0xfffffe68u64, 4);
                        u_100 = v & 0xffffffff;
                        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = (img.h(0, 0xe) as u64);
                        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                        let a = (u_100) as u32;
                        match img.hunk_of(a) {
                            Some(k) => match by_addr(k) {
                                Some(f) => f(r, bus, img)?,
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            },
                            None => bus.call_outside(a, r, img)?,
                        }
                        let ret = r.get(PC, 4) as u32;
                        if ret != img.h(0, 0xe) {
                            let a = ret as u32;
                            match img.hunk_of(a) {
                                Some(k) => { pc = k; continue 'b; }
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            }
                        }
                    }
                }
                if entered <= 0xe {
                    // h0+$e movea.l d0,a5
                    {
                        let v = r.get(0x0, 4);
                        r.set(0x34, 4, v);
                    }
                }
                if entered <= 0x10 {
                    // h0+$10 move.l #h1+$10,d1
                    {
                        let v = (img.h(1, 0x10) as u64);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0x16 {
                    // h0+$16 move.l #$3ed,d2
                    {
                        let v = 0x3edu64;
                        r.set(0x8, 4, v);
                    }
                }
                if entered <= 0x1c {
                    // h0+$1c jsr (-30,a5)
                    {
                        let mut u_100: u64 = 0;
                        let v = int_add(r.get(0x34, 4), 0xffffffe2u64, 4);
                        u_100 = v & 0xffffffff;
                        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = (img.h(0, 0x20) as u64);
                        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                        let a = (u_100) as u32;
                        match img.hunk_of(a) {
                            Some(k) => match by_addr(k) {
                                Some(f) => f(r, bus, img)?,
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            },
                            None => bus.call_outside(a, r, img)?,
                        }
                        let ret = r.get(PC, 4) as u32;
                        if ret != img.h(0, 0x20) {
                            let a = ret as u32;
                            match img.hunk_of(a) {
                                Some(k) => { pc = k; continue 'b; }
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            }
                        }
                    }
                }
                if entered <= 0x20 {
                    // h0+$20 move.l d0,d4
                    {
                        let v = r.get(0x0, 4);
                        r.set(0x10, 4, v);
                    }
                }
                if entered <= 0x22 {
                    // h0+$22 move.l d0,d1
                    {
                        let v = r.get(0x0, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0x24 {
                    // h0+$24 move.l #h1+$20,d2
                    {
                        let v = (img.h(1, 0x20) as u64);
                        r.set(0x8, 4, v);
                    }
                }
                if entered <= 0x2a {
                    // h0+$2a move.l #$60,d3
                    {
                        let v = 0x60u64;
                        r.set(0xc, 4, v);
                    }
                }
                if entered <= 0x30 {
                    // h0+$30 jsr (-42,a5)
                    {
                        let mut u_100: u64 = 0;
                        let v = int_add(r.get(0x34, 4), 0xffffffd6u64, 4);
                        u_100 = v & 0xffffffff;
                        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = (img.h(0, 0x34) as u64);
                        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                        let a = (u_100) as u32;
                        match img.hunk_of(a) {
                            Some(k) => match by_addr(k) {
                                Some(f) => f(r, bus, img)?,
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            },
                            None => bus.call_outside(a, r, img)?,
                        }
                        let ret = r.get(PC, 4) as u32;
                        if ret != img.h(0, 0x34) {
                            let a = ret as u32;
                            match img.hunk_of(a) {
                                Some(k) => { pc = k; continue 'b; }
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            }
                        }
                    }
                }
                if entered <= 0x34 {
                    // h0+$34 move.l d0,d5
                    {
                        let v = r.get(0x0, 4);
                        r.set(0x14, 4, v);
                    }
                }
                if entered <= 0x36 {
                    // h0+$36 move.l d4,d1
                    {
                        let v = r.get(0x10, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0x38 {
                    // h0+$38 jsr (-36,a5)
                    {
                        let mut u_100: u64 = 0;
                        let v = int_add(r.get(0x34, 4), 0xffffffdcu64, 4);
                        u_100 = v & 0xffffffff;
                        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = (img.h(0, 0x3c) as u64);
                        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                        let a = (u_100) as u32;
                        match img.hunk_of(a) {
                            Some(k) => match by_addr(k) {
                                Some(f) => f(r, bus, img)?,
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            },
                            None => bus.call_outside(a, r, img)?,
                        }
                        let ret = r.get(PC, 4) as u32;
                        if ret != img.h(0, 0x3c) {
                            let a = ret as u32;
                            match img.hunk_of(a) {
                                Some(k) => { pc = k; continue 'b; }
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            }
                        }
                    }
                }
                if entered <= 0x3c {
                    // h0+$3c rts
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
                }
                pc = (0, 0x3e);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}
