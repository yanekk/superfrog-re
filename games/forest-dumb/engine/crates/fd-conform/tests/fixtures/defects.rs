// Generated from defects.json by pcode_rust.py --insn; never committed when made from a game.
// Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC};

/// The bytes of every emitted instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x2),
    (0, 0x2, 0x4),
    (0, 0x4, 0x6),
    (0, 0x6, 0x8),
    (0, 0x8, 0xa),
    (0, 0xa, 0xc),
    (0, 0xc, 0xe),
    (0, 0xe, 0x10),
    (0, 0x10, 0x12),
    (0, 0x12, 0x14),
    (0, 0x14, 0x16),
    (0, 0x16, 0x18),
    (0, 0x18, 0x1c),
    (0, 0x1c, 0x20),
    (0, 0x20, 0x24),
    (0, 0x24, 0x2a),
    (0, 0x2a, 0x2c),
    (0, 0x2c, 0x2e),
    (0, 0x2e, 0x30),
    (0, 0x30, 0x32),
    (0, 0x32, 0x34),
    (0, 0x34, 0x38),
    (0, 0x38, 0x3c),
    (0, 0x3c, 0x42),
    (0, 0x42, 0x44),
    (0, 0x44, 0x46),
    (0, 0x46, 0x48),
    (0, 0x48, 0x4a),
    (0, 0x4a, 0x4e),
    (0, 0x52, 0x54),
    (0, 0x54, 0x56),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x58, 0x10];

pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// (encoding, site, hunk, offset, length, hunk-relative data addresses, function), by site.
pub const SITES: &[(&str, &str, u32, u32, u32, &[(u32, u32)], Routine)] = &[
    ("c101", "h0+$0", 0, 0x0, 2, &[], i_h0_0),
    ("c109", "h0+$2", 0, 0x2, 2, &[], i_h0_2),
    ("cf0f", "h0+$4", 0, 0x4, 2, &[], i_h0_4),
    ("8101", "h0+$6", 0, 0x6, 2, &[], i_h0_6),
    ("8109", "h0+$8", 0, 0x8, 2, &[], i_h0_8),
    ("8f0f", "h0+$a", 0, 0xa, 2, &[], i_h0_a),
    ("4800", "h0+$c", 0, 0xc, 2, &[], i_h0_c),
    ("4810", "h0+$e", 0, 0xe, 2, &[], i_h0_e),
    ("4818", "h0+$10", 0, 0x10, 2, &[], i_h0_10),
    ("4820", "h0+$12", 0, 0x12, 2, &[], i_h0_12),
    ("481f", "h0+$14", 0, 0x14, 2, &[], i_h0_14),
    ("4827", "h0+$16", 0, 0x16, 2, &[], i_h0_16),
    ("4828", "h0+$18", 0, 0x18, 4, &[], i_h0_18),
    ("4830", "h0+$1c", 0, 0x1c, 4, &[], i_h0_1c),
    ("4838", "h0+$20", 0, 0x20, 4, &[], i_h0_20),
    ("4839", "h0+$24", 0, 0x24, 6, &[(1, 0x4)], i_h0_24),
    ("b001", "h0+$2a", 0, 0x2a, 2, &[], i_h0_2a),
    ("b050", "h0+$2c", 0, 0x2c, 2, &[], i_h0_2c),
    ("b081", "h0+$2e", 0, 0x2e, 2, &[], i_h0_2e),
    ("b0c1", "h0+$30", 0, 0x30, 2, &[], i_h0_30),
    ("b1c9", "h0+$32", 0, 0x32, 2, &[], i_h0_32),
    ("0c03", "h0+$34", 0, 0x34, 4, &[], i_h0_34),
    ("0c50", "h0+$38", 0, 0x38, 4, &[], i_h0_38),
    ("0c80", "h0+$3c", 0, 0x3c, 6, &[], i_h0_3c),
    ("b109", "h0+$42", 0, 0x42, 2, &[], i_h0_42),
    ("b149", "h0+$44", 0, 0x44, 2, &[], i_h0_44),
    ("b189", "h0+$46", 0, 0x46, 2, &[], i_h0_46),
    ("bf0f", "h0+$48", 0, 0x48, 2, &[], i_h0_48),
    ("6000", "h0+$4a", 0, 0x4a, 4, &[], i_h0_4a),
    ("7402", "h0+$52", 0, 0x52, 2, &[], i_h0_52),
    ("4e75", "h0+$54", 0, 0x54, 2, &[], i_h0_54),
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$0 abcd (corrected: pcode_fix.bcd)
    {
        let (v, x, o) = pcode_rt::bcd::bcd_add(r.get(0x7, 1), r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0x2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2 abcd (corrected: pcode_fix.bcd)
    {
        let ay = (r.get(0x24, 4) as u32).wrapping_sub(1);
        r.set(0x24, 4, ay as u64);
        let s = load(bus, ay, 1);
        let ax = (r.get(0x20, 4) as u32).wrapping_sub(1);
        r.set(0x20, 4, ax as u64);
        let d = load(bus, ax, 1);
        let (v, x, o) = pcode_rt::bcd::bcd_add(s, d, r.get(0x43, 1));
        store(bus, img, ax, 1, v)?;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0x4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4 abcd (corrected: pcode_fix.bcd)
    {
        let ay = (r.get(0x3c, 4) as u32).wrapping_sub(2);
        r.set(0x3c, 4, ay as u64);
        let s = load(bus, ay, 1);
        let ax = (r.get(0x3c, 4) as u32).wrapping_sub(2);
        r.set(0x3c, 4, ax as u64);
        let d = load(bus, ax, 1);
        let (v, x, o) = pcode_rt::bcd::bcd_add(s, d, r.get(0x43, 1));
        store(bus, img, ax, 1, v)?;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0x6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6 sbcd (corrected: pcode_fix.bcd)
    {
        let (v, x, o) = pcode_rt::bcd::bcd_sub(r.get(0x7, 1), r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0x8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8 sbcd (corrected: pcode_fix.bcd)
    {
        let ay = (r.get(0x24, 4) as u32).wrapping_sub(1);
        r.set(0x24, 4, ay as u64);
        let s = load(bus, ay, 1);
        let ax = (r.get(0x20, 4) as u32).wrapping_sub(1);
        r.set(0x20, 4, ax as u64);
        let d = load(bus, ax, 1);
        let (v, x, o) = pcode_rt::bcd::bcd_sub(s, d, r.get(0x43, 1));
        store(bus, img, ax, 1, v)?;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0xa) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$a sbcd (corrected: pcode_fix.bcd)
    {
        let ay = (r.get(0x3c, 4) as u32).wrapping_sub(2);
        r.set(0x3c, 4, ay as u64);
        let s = load(bus, ay, 1);
        let ax = (r.get(0x3c, 4) as u32).wrapping_sub(2);
        r.set(0x3c, 4, ax as u64);
        let d = load(bus, ax, 1);
        let (v, x, o) = pcode_rt::bcd::bcd_sub(s, d, r.get(0x43, 1));
        store(bus, img, ax, 1, v)?;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0xc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c nbcd
    {
        let mut u_2f100: u64 = 0;
        let v = r.get(0x3, 1);
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        r.set(0x3, 1, v);
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0xe) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e nbcd
    {
        let mut u_5800: u64 = 0;
        let mut u_2f100: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        u_5800 = v & 0xff;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
        let v = u_5800;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0x10) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$10 nbcd
    {
        let mut u_5a00: u64 = 0;
        let mut u_5b00: u64 = 0;
        let mut u_2f100: u64 = 0;
        let v = r.get(0x20, 4);
        u_5a00 = v & 0xffffffff;
        let v = int_add(r.get(0x20, 4), 0x1u64, 4);
        r.set(0x20, 4, v);
        let v = load(bus, u_5a00 as u32, 1);
        u_5b00 = v & 0xff;
        let v = u_5b00;
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        u_5b00 = v & 0xff;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
        let v = u_5b00;
        store(bus, img, u_5a00 as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0x12) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_12(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$12 nbcd
    {
        let mut u_5d00: u64 = 0;
        let mut u_2f100: u64 = 0;
        let v = int_sub(r.get(0x20, 4), 0x1u64, 4);
        r.set(0x20, 4, v);
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5d00 = v & 0xff;
        let v = u_5d00;
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        u_5d00 = v & 0xff;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
        let v = u_5d00;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0x14) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_14(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$14 nbcd
    {
        let mut u_5880: u64 = 0;
        let mut u_5980: u64 = 0;
        let mut u_2f100: u64 = 0;
        let v = r.get(0x3c, 4);
        u_5880 = v & 0xffffffff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, u_5880 as u32, 1);
        u_5980 = v & 0xff;
        let v = u_5980;
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        u_5980 = v & 0xff;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
        let v = u_5980;
        store(bus, img, u_5880 as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0x16) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_16(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$16 nbcd
    {
        let mut u_5c00: u64 = 0;
        let mut u_2f100: u64 = 0;
        let v = int_sub(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, r.get(0x3c, 4) as u32, 1);
        u_5c00 = v & 0xff;
        let v = u_5c00;
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        u_5c00 = v & 0xff;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
        let v = u_5c00;
        store(bus, img, r.get(0x3c, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0x18) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_18(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$18 nbcd
    {
        let mut u_5e00: u64 = 0;
        let mut u_5e80: u64 = 0;
        let mut u_2f100: u64 = 0;
        let v = int_add(r.get(0x20, 4), 0x10u64, 4);
        u_5e00 = v & 0xffffffff;
        let v = load(bus, u_5e00 as u32, 1);
        u_5e80 = v & 0xff;
        let v = u_5e80;
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        u_5e80 = v & 0xff;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
        let v = u_5e80;
        store(bus, img, u_5e00 as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0x1c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1c nbcd
    {
        let mut u_380: u64 = 0;
        let mut u_480: u64 = 0;
        let mut u_780: u64 = 0;
        let mut u_880: u64 = 0;
        let mut u_5f00: u64 = 0;
        let mut u_2f100: u64 = 0;
        let v = int_sext(r.get(0x6, 2), 2, 4);
        u_380 = v & 0xffffffff;
        let v = int_mult(u_380, 0x1u64, 4);
        u_480 = v & 0xffffffff;
        let v = int_add(r.get(0x20, 4), 0x2u64, 4);
        u_780 = v & 0xffffffff;
        let v = int_add(u_780, u_480, 4);
        u_880 = v & 0xffffffff;
        let v = load(bus, u_880 as u32, 1);
        u_5f00 = v & 0xff;
        let v = u_5f00;
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        u_5f00 = v & 0xff;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
        let v = u_5f00;
        store(bus, img, u_880 as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0x20) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_20(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$20 nbcd
    {
        let mut u_2f100: u64 = 0;
        let v = load(bus, 0x400, 1);
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        store(bus, img, 0x400, 1, v)?;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0x24) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_24(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$24 nbcd
    {
        let mut u_2f100: u64 = 0;
        let v = load(bus, img.h(1, 0x4), 1);
        u_2f100 = v & 0xff;
        let (v, x, o) = pcode_rt::bcd::bcd_sub(u_2f100, 0, r.get(0x43, 1));
        store(bus, img, img.h(1, 0x4), 1, v)?;
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0x2a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2a cmp.b
    {
        let mut u_20180: u64 = 0;
        let mut u_20280: u64 = 0;
        let v = r.get(0x7, 1);
        u_20180 = v & 0xff;
        let v = int_less(r.get(0x3, 1), u_20180, 1);
        r.set(0x47, 1, v);
        let v = int_sborrow(r.get(0x3, 1), u_20180, 1);
        r.set(0x46, 1, v);
        let v = int_sub(r.get(0x3, 1), u_20180, 1);
        u_20280 = v & 0xff;
        let v = int_sless(u_20280, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_20280, 0x0u64, 1);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x2c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2c cmp.w
    {
        let mut u_4f00: u64 = 0;
        let mut u_20300: u64 = 0;
        let mut u_20400: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_20300 = v & 0xffff;
        let v = int_less(r.get(0x2, 2), u_20300, 2);
        r.set(0x47, 1, v);
        let v = int_sborrow(r.get(0x2, 2), u_20300, 2);
        r.set(0x46, 1, v);
        let v = int_sub(r.get(0x2, 2), u_20300, 2);
        u_20400 = v & 0xffff;
        let v = int_sless(u_20400, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_20400, 0x0u64, 2);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x2e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2e cmp.l
    {
        let mut u_20480: u64 = 0;
        let mut u_20580: u64 = 0;
        let v = r.get(0x4, 4);
        u_20480 = v & 0xffffffff;
        let v = int_less(r.get(0x0, 4), u_20480, 4);
        r.set(0x47, 1, v);
        let v = int_sborrow(r.get(0x0, 4), u_20480, 4);
        r.set(0x46, 1, v);
        let v = int_sub(r.get(0x0, 4), u_20480, 4);
        u_20580 = v & 0xffffffff;
        let v = int_sless(u_20580, 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(u_20580, 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x30) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_30(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$30 cmpa.w
    {
        let mut u_20680: u64 = 0;
        let mut u_20780: u64 = 0;
        let v = int_sext(r.get(0x6, 2), 2, 4);
        u_20680 = v & 0xffffffff;
        let v = int_less(r.get(0x20, 4), u_20680, 4);
        r.set(0x47, 1, v);
        let v = int_sborrow(r.get(0x20, 4), u_20680, 4);
        r.set(0x46, 1, v);
        let v = int_sub(r.get(0x20, 4), u_20680, 4);
        u_20780 = v & 0xffffffff;
        let v = int_sless(u_20780, 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(u_20780, 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x32) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_32(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$32 cmpa.l
    {
        let mut u_20800: u64 = 0;
        let mut u_20900: u64 = 0;
        let v = r.get(0x24, 4);
        u_20800 = v & 0xffffffff;
        let v = int_less(r.get(0x20, 4), u_20800, 4);
        r.set(0x47, 1, v);
        let v = int_sborrow(r.get(0x20, 4), u_20800, 4);
        r.set(0x46, 1, v);
        let v = int_sub(r.get(0x20, 4), u_20800, 4);
        u_20900 = v & 0xffffffff;
        let v = int_sless(u_20900, 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(u_20900, 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x34) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_34(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$34 cmpi.b
    {
        let mut u_20980: u64 = 0;
        let mut u_20a80: u64 = 0;
        let v = r.get(0xf, 1);
        u_20980 = v & 0xff;
        let v = int_less(u_20980, 0x1u64, 1);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_20980, 0x1u64, 1);
        r.set(0x46, 1, v);
        let v = int_sub(u_20980, 0x1u64, 1);
        u_20a80 = v & 0xff;
        let v = int_sless(u_20a80, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_20a80, 0x0u64, 1);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x38) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_38(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$38 cmpi.w
    {
        let mut u_7e80: u64 = 0;
        let mut u_20b00: u64 = 0;
        let mut u_20c00: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_7e80 = v & 0xffff;
        let v = u_7e80;
        u_20b00 = v & 0xffff;
        let v = int_less(u_20b00, 0x1234u64, 2);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_20b00, 0x1234u64, 2);
        r.set(0x46, 1, v);
        let v = int_sub(u_20b00, 0x1234u64, 2);
        u_20c00 = v & 0xffff;
        let v = int_sless(u_20c00, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_20c00, 0x0u64, 2);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x3c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_3c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$3c cmpi.l
    {
        let mut u_20c80: u64 = 0;
        let mut u_20d80: u64 = 0;
        let v = r.get(0x0, 4);
        u_20c80 = v & 0xffffffff;
        let v = int_less(u_20c80, 0x12345678u64, 4);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_20c80, 0x12345678u64, 4);
        r.set(0x46, 1, v);
        let v = int_sub(u_20c80, 0x12345678u64, 4);
        u_20d80 = v & 0xffffffff;
        let v = int_sless(u_20d80, 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(u_20d80, 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x42) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_42(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$42 cmpm.b
    {
        let mut u_20e80: u64 = 0;
        let mut u_21000: u64 = 0;
        let mut u_21180: u64 = 0;
        let v = load(bus, r.get(0x24, 4) as u32, 1);
        u_20e80 = v & 0xff;
        let v = int_add(r.get(0x24, 4), 0x1u64, 4);
        r.set(0x24, 4, v);
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_21000 = v & 0xff;
        let v = int_add(r.get(0x20, 4), 0x1u64, 4);
        r.set(0x20, 4, v);
        let v = int_less(u_21000, u_20e80, 1);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_21000, u_20e80, 1);
        r.set(0x46, 1, v);
        let v = int_sub(u_21000, u_20e80, 1);
        u_21180 = v & 0xff;
        let v = int_sless(u_21180, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_21180, 0x0u64, 1);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x44) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_44(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$44 cmpm.w
    {
        let mut u_21280: u64 = 0;
        let mut u_21400: u64 = 0;
        let mut u_21580: u64 = 0;
        let v = load(bus, r.get(0x24, 4) as u32, 2);
        u_21280 = v & 0xffff;
        let v = int_add(r.get(0x24, 4), 0x2u64, 4);
        r.set(0x24, 4, v);
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_21400 = v & 0xffff;
        let v = int_add(r.get(0x20, 4), 0x2u64, 4);
        r.set(0x20, 4, v);
        let v = int_less(u_21400, u_21280, 2);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_21400, u_21280, 2);
        r.set(0x46, 1, v);
        let v = int_sub(u_21400, u_21280, 2);
        u_21580 = v & 0xffff;
        let v = int_sless(u_21580, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_21580, 0x0u64, 2);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x46) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_46(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$46 cmpm.l
    {
        let mut u_21680: u64 = 0;
        let mut u_21800: u64 = 0;
        let mut u_21980: u64 = 0;
        let v = load(bus, r.get(0x24, 4) as u32, 4);
        u_21680 = v & 0xffffffff;
        let v = int_add(r.get(0x24, 4), 0x4u64, 4);
        r.set(0x24, 4, v);
        let v = load(bus, r.get(0x20, 4) as u32, 4);
        u_21800 = v & 0xffffffff;
        let v = int_add(r.get(0x20, 4), 0x4u64, 4);
        r.set(0x20, 4, v);
        let v = int_less(u_21800, u_21680, 4);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_21800, u_21680, 4);
        r.set(0x46, 1, v);
        let v = int_sub(u_21800, u_21680, 4);
        u_21980 = v & 0xffffffff;
        let v = int_sless(u_21980, 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(u_21980, 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x48) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_48(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$48 cmpm.b
    {
        let mut u_20e80: u64 = 0;
        let mut u_21000: u64 = 0;
        let mut u_21180: u64 = 0;
        let v = load(bus, r.get(0x3c, 4) as u32, 1);
        u_20e80 = v & 0xff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, r.get(0x3c, 4) as u32, 1);
        u_21000 = v & 0xff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = int_less(u_21000, u_20e80, 1);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_21000, u_20e80, 1);
        r.set(0x46, 1, v);
        let v = int_sub(u_21000, u_20e80, 1);
        u_21180 = v & 0xff;
        let v = int_sless(u_21180, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_21180, 0x0u64, 1);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x4a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4a bra.w
    {
        r.set(PC, 4, img.h(0, 0x52) as u64); return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x4e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_52(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$52 moveq
    {
        let v = 0x2u64;
        r.set(0x8, 4, v);
        let v = int_sless(r.get(0x8, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x8, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x54) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_54(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$54 rts
    {
        let v = load(bus, r.get(0x3c, 4) as u32, 4);
        r.set(0x50, 4, v);
        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = r.get(0x50, 4);
        r.set(PC, 4, v);
        return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x56) as u64);
    Ok(())
}
