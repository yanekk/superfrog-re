// Generated from arith.json by pcode_rust.py --insn; never committed when made from a game.
// Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC, cycles as cy};

/// The bytes of every emitted instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x2),
    (0, 0x2, 0x4),
    (0, 0x4, 0x6),
    (0, 0x6, 0x8),
    (0, 0x8, 0xc),
    (0, 0xc, 0x10),
    (0, 0x10, 0x16),
    (0, 0x18, 0x1e),
    (0, 0x20, 0x22),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x30, 0x10];

pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// (encoding, site, hunk, offset, length, hunk-relative data addresses, function), by site.
pub const SITES: &[(&str, &str, u32, u32, u32, &[(u32, u32)], Routine)] = &[
    ("d181", "h0+$0", 0, 0x0, 2, &[], i_h0_0),
    ("9181", "h0+$2", 0, 0x2, 2, &[], i_h0_2),
    ("4080", "h0+$4", 0, 0x4, 2, &[], i_h0_4),
    ("80c1", "h0+$6", 0, 0x6, 2, &[], i_h0_6),
    ("81fc", "h0+$8", 0, 0x8, 4, &[], i_h0_8),
    ("80fc", "h0+$c", 0, 0xc, 4, &[], i_h0_c),
    ("4279", "h0+$10", 0, 0x10, 6, &[], i_h0_10),
    ("42b9", "h0+$18", 0, 0x18, 6, &[], i_h0_18),
    ("4250", "h0+$20", 0, 0x20, 2, &[], i_h0_20),
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$0 addx.l
    {
        r.tick(const { cy::of(cy::Op::Addx, cy::Size::L, cy::Ea::Dn, cy::Ea::Dn) });
        let mut u_af80: u64 = 0;
        let mut u_b000: u64 = 0;
        let mut u_b080: u64 = 0;
        let mut u_b100: u64 = 0;
        let mut u_b200: u64 = 0;
        let mut u_b280: u64 = 0;
        let mut u_b300: u64 = 0;
        let mut u_e600: u64 = 0;
        let mut u_e680: u64 = 0;
        let mut u_14480: u64 = 0;
        let mut u_14500: u64 = 0;
        let v = int_add(r.get(0x4, 4), r.get(0x0, 4), 4);
        u_af80 = v & 0xffffffff;
        let v = int_carry(r.get(0x4, 4), r.get(0x0, 4), 4);
        u_b000 = v & 0xff;
        let v = int_zext(r.get(0x43, 1), 1);
        u_b080 = v & 0xffffffff;
        let v = int_carry(u_af80, u_b080, 4);
        u_b100 = v & 0xff;
        let v = bool_or(u_b000, u_b100);
        r.set(0x47, 1, v);
        let v = int_scarry(r.get(0x4, 4), r.get(0x0, 4), 4);
        u_b200 = v & 0xff;
        let v = int_zext(r.get(0x43, 1), 1);
        u_b280 = v & 0xffffffff;
        let v = int_scarry(u_af80, u_b280, 4);
        u_b300 = v & 0xff;
        let v = bool_xor(u_b200, u_b300);
        r.set(0x46, 1, v);
        let v = int_add(r.get(0x4, 4), r.get(0x0, 4), 4);
        u_14480 = v & 0xffffffff;
        let v = int_zext(r.get(0x43, 1), 1);
        u_14500 = v & 0xffffffff;
        let v = int_add(u_14480, u_14500, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        u_e600 = v & 0xff;
        let v = int_equal(r.get(0x45, 1), 0x1u64, 1);
        u_e680 = v & 0xff;
        let v = bool_and(u_e600, u_e680);
        r.set(0x45, 1, v);
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2 subx.l
    {
        r.tick(const { cy::of(cy::Op::Addx, cy::Size::L, cy::Ea::Dn, cy::Ea::Dn) });
        let mut u_c000: u64 = 0;
        let mut u_c080: u64 = 0;
        let mut u_c100: u64 = 0;
        let mut u_c180: u64 = 0;
        let mut u_c200: u64 = 0;
        let mut u_c280: u64 = 0;
        let mut u_e600: u64 = 0;
        let mut u_e680: u64 = 0;
        let v = int_zext(r.get(0x43, 1), 1);
        u_c000 = v & 0xffffffff;
        let v = int_less(r.get(0x0, 4), r.get(0x4, 4), 4);
        u_c080 = v & 0xff;
        let v = int_equal(r.get(0x43, 1), 0x1u64, 1);
        u_c100 = v & 0xff;
        let v = int_equal(r.get(0x0, 4), r.get(0x4, 4), 4);
        u_c180 = v & 0xff;
        let v = bool_and(u_c100, u_c180);
        u_c200 = v & 0xff;
        let v = bool_or(u_c080, u_c200);
        r.set(0x47, 1, v);
        let v = int_sborrow(r.get(0x0, 4), r.get(0x4, 4), 4);
        r.set(0x46, 1, v);
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
        let v = int_add(u_c000, r.get(0x4, 4), 4);
        u_c280 = v & 0xffffffff;
        let v = int_sub(r.get(0x0, 4), u_c280, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        u_e600 = v & 0xff;
        let v = int_equal(r.get(0x45, 1), 0x1u64, 1);
        u_e680 = v & 0xff;
        let v = bool_and(u_e600, u_e680);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4 negx.l
    {
        r.tick(const { cy::of(cy::Op::Clr, cy::Size::L, cy::Ea::Dn, cy::Ea::None) });
        let mut u_d000: u64 = 0;
        let mut u_d080: u64 = 0;
        let mut u_e600: u64 = 0;
        let mut u_e680: u64 = 0;
        let v = int_zext(r.get(0x43, 1), 1);
        u_d000 = v & 0xffffffff;
        let v = int_add(r.get(0x0, 4), u_d000, 4);
        u_d080 = v & 0xffffffff;
        let v = int_sless(0x0u64, u_d080, 4);
        r.set(0x47, 1, v);
        let v = int_sborrow(0x0u64, u_d080, 4);
        r.set(0x46, 1, v);
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
        let v = int_2comp(u_d080, 4);
        u_d080 = v & 0xffffffff;
        let v = u_d080;
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        u_e600 = v & 0xff;
        let v = int_equal(r.get(0x45, 1), 0x1u64, 1);
        u_e680 = v & 0xff;
        let v = bool_and(u_e600, u_e680);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6 divu.w (corrected: pcode_fix.arith)
    {
        let mut u_22100: u64 = 0;
        let mut u_22180: u64 = 0;
        r.tick(cy::divu_cycles(r.d(0), r.d(1) as u16));
        let v = int_zext(r.get(0x6, 2), 2);
        u_22100 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_22180 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divu_w(u_22180 as u32, u_22100) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$6".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8 divs.w (corrected: pcode_fix.arith)
    {
        let mut u_22100: u64 = 0;
        let mut u_22180: u64 = 0;
        r.tick(cy::divs_cycles(r.d(0) as i32, 0x0003u16 as i16) + const { cy::ea_cycles(cy::Ea::Imm, cy::Size::W) });
        let v = int_sext(0x3u64, 2, 4);
        u_22100 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_22180 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divs_w(u_22180 as u32, u_22100) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$8".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0xc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c divu.w (corrected: pcode_fix.arith)
    {
        let mut u_22100: u64 = 0;
        let mut u_22180: u64 = 0;
        r.tick(cy::divu_cycles(r.d(0), 0x0000u16) + const { cy::ea_cycles(cy::Ea::Imm, cy::Size::W) });
        let v = int_zext(0x0u64, 2);
        u_22100 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_22180 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divu_w(u_22180 as u32, u_22100) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$c".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x10) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$10 clr.w (corrected: pcode_fix.arith)
    {
        let mut u_100: u64 = 0;
        r.tick(const { cy::of(cy::Op::Clr, cy::Size::W, cy::Ea::AbsL, cy::Ea::None) });
        let v = 0xdff0a8u64;
        u_100 = v & 0xffffffff;
        let a = u_100 as u32;
        if pcode_rt::ops::custom_write_only(a) {
            let w = load(bus, img.h(0, 0x16), 2);
            store(bus, img, a, 2, w)?;
        }
        let v = 0x0u64;
        store(bus, img, 0xdff0a8, 2, v)?;
        let v = 0x0u64;
        r.set(0x44, 1, v);
        let v = 0x1u64;
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x16) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_18(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$18 clr.l (corrected: pcode_fix.arith)
    {
        let mut u_100: u64 = 0;
        r.tick(const { cy::of(cy::Op::Clr, cy::Size::L, cy::Ea::AbsL, cy::Ea::None) });
        let v = 0xdff0a8u64;
        u_100 = v & 0xffffffff;
        let a = u_100 as u32;
        if pcode_rt::ops::custom_write_only(a) {
            let w = load(bus, img.h(0, 0x1e), 2);
            store(bus, img, a, 2, w)?;
        }
        if pcode_rt::ops::custom_write_only(a.wrapping_add(2)) {
            let w = load(bus, img.h(0, 0x1e), 2);
            store(bus, img, a.wrapping_add(2), 2, w)?;
        }
        store(bus, img, a.wrapping_add(2), 2, 0)?;
        store(bus, img, a, 2, 0)?;
        let v = 0x0u64;
        r.set(0x44, 1, v);
        let v = 0x1u64;
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x1e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_20(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$20 clr.w (corrected: pcode_fix.arith)
    {
        let mut u_4f00: u64 = 0;
        let mut u_5000: u64 = 0;
        r.tick(const { cy::of(cy::Op::Clr, cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let v = 0x0u64;
        u_4f00 = v & 0xffff;
        let v = r.get(0x20, 4);
        u_5000 = v & 0xffffffff;
        let a = u_5000 as u32;
        if pcode_rt::ops::custom_write_only(a) {
            let w = load(bus, img.h(0, 0x22), 2);
            store(bus, img, a, 2, w)?;
        }
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = 0x0u64;
        r.set(0x44, 1, v);
        let v = 0x1u64;
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x22) as u64);
    Ok(())
}
