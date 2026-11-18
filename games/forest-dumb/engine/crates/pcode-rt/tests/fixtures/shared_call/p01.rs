// Part 1 of the tree generated from shared_call.json. Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC, cycles as cy};
#[allow(unused_imports)]
use super::{by_addr, enter};

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
#[inline]
pub fn h0_20(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x20)) {
        return f(r, bus, img);
    }
    h0_20_from((0, 0x20), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_20_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x20) | (0, 0x24) | (0, 0x26) => {
                let entered = pc.1;
                if entered <= 0x20 {
                    // h0+$20 bsr h0+$30
                    {
                        bus.sync(r);
                        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
                        r.set(0x3c, 4, v);
                        let v = (img.h(0, 0x24) as u64);
                        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
                        h0_30(r, bus, img)?;
                        let ret = r.get(PC, 4) as u32;
                        if ret != img.h(0, 0x24) {
                            let a = ret as u32;
                            match img.hunk_of(a) {
                                Some(k) => { pc = k; continue 'b; }
                                None => return Err(Stop::Untranslated(img.describe(a))),
                            }
                        }
                    }
                }
                if entered <= 0x24 {
                    // h0+$24 addq.l #2,d3
                    {
                        let v = int_add(r.get(0xc, 4), 0x2u64, 4);
                        r.set(0xc, 4, v);
                    }
                }
                if entered <= 0x26 {
                    // h0+$26 rts
                    {
                        bus.sync(r);
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
                pc = (0, 0x28);
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
#[inline]
pub fn h0_30(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    if let Some(f) = bus.mounted((0, 0x30)) {
        return f(r, bus, img);
    }
    h0_30_from((0, 0x30), r, bus, img)
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn h0_30_from(start: (u32, u32), r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut pc: (u32, u32) = start;
    'b: loop {
        block_start(bus, r, img)?;
        match pc {
            (0, 0x30) | (0, 0x32) => {
                let entered = pc.1;
                if entered <= 0x30 {
                    // h0+$30 addq.l #1,d1
                    {
                        let v = int_add(r.get(0x4, 4), 0x1u64, 4);
                        r.set(0x4, 4, v);
                    }
                }
                if entered <= 0x32 {
                    // h0+$32 rts
                    {
                        bus.sync(r);
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
                pc = (0, 0x34);
            }
            k => {
                let res = enter(k, r, bus, img);
                r.depth -= 1;
                return res;
            }
        }
    }
}
