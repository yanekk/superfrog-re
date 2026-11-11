// Part 0 of the tree generated from shared_call.json. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC};
#[allow(unused_imports)]
use super::{by_addr, enter};

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = (0, 0x0);
    'b: loop {
        block_start(bus, r, img)?;
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
                // h0+$4 bsr h0+$20
                {
                    let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = (img.h(0, 0x8) as u64);
                    store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                    super::p01::h0_20(r, bus, img)?;
                    let ret = r.get(PC, 4) as u32;
                    if ret != img.h(0, 0x8) {
                        let a = ret as u32;
                        match img.hunk_of(a) {
                            Some(k) => { pc = k; continue 'b; }
                            None => return Err(Stop::Untranslated(img.describe(a))),
                        }
                    }
                }
                // h0+$8 rts
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
                pc = (0, 0xa);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = (0, 0x10);
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x10) => {
                // h0+$10 bsr h0+$30
                {
                    let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                    r.set(0x3c, 4, v);
                    let v = (img.h(0, 0x14) as u64);
                    store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                    super::p01::h0_30(r, bus, img)?;
                    let ret = r.get(PC, 4) as u32;
                    if ret != img.h(0, 0x14) {
                        let a = ret as u32;
                        match img.hunk_of(a) {
                            Some(k) => { pc = k; continue 'b; }
                            None => return Err(Stop::Untranslated(img.describe(a))),
                        }
                    }
                }
                // h0+$14 addq.l #1,d2
                {
                    let v = int_add(r.get(0x8, 4), 0x1u64, 4);
                    r.set(0x8, 4, v);
                }
                // h0+$16 rts
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
                pc = (0, 0x18);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}
