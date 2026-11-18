// Generated from cycles_dynamic.json by pcode_rust.py --insn; never committed when made from a game.
// Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC, cycles as cy};

/// The bytes of every emitted instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x2),
    (0, 0x2, 0x4),
    (0, 0x4, 0x8),
    (0, 0x8, 0xc),
    (0, 0xc, 0x10),
    (0, 0x10, 0x14),
    (0, 0x14, 0x18),
    (0, 0x18, 0x1a),
    (0, 0x1a, 0x1c),
    (0, 0x1c, 0x20),
    (0, 0x20, 0x24),
    (0, 0x24, 0x28),
    (0, 0x28, 0x2c),
    (0, 0x2c, 0x30),
    (0, 0x30, 0x34),
    (0, 0x34, 0x38),
    (0, 0x38, 0x3c),
    (0, 0x3c, 0x40),
    (0, 0x40, 0x44),
    (0, 0x44, 0x48),
    (0, 0x48, 0x4e),
    (0, 0x4e, 0x52),
    (0, 0x52, 0x56),
    (0, 0x56, 0x5a),
    (0, 0x5a, 0x62),
    (0, 0x62, 0x64),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x64];

pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// (encoding, site, hunk, offset, length, hunk-relative data addresses, function), by site.
pub const SITES: &[(&str, &str, u32, u32, u32, &[(u32, u32)], Routine)] = &[
    ("c0c1", "h0+$0", 0, 0x0, 2, &[], i_h0_0),
    ("c1c1", "h0+$2", 0, 0x2, 2, &[], i_h0_2),
    ("c0fc", "h0+$4", 0, 0x4, 4, &[], i_h0_4),
    ("c0fc", "h0+$8", 0, 0x8, 4, &[], i_h0_8),
    ("c0fc", "h0+$c", 0, 0xc, 4, &[], i_h0_c),
    ("c1fc", "h0+$10", 0, 0x10, 4, &[], i_h0_10),
    ("c1fc", "h0+$14", 0, 0x14, 4, &[], i_h0_14),
    ("80c1", "h0+$18", 0, 0x18, 2, &[], i_h0_18),
    ("81c1", "h0+$1a", 0, 0x1a, 2, &[], i_h0_1a),
    ("80fc", "h0+$1c", 0, 0x1c, 4, &[], i_h0_1c),
    ("80fc", "h0+$20", 0, 0x20, 4, &[], i_h0_20),
    ("81fc", "h0+$24", 0, 0x24, 4, &[], i_h0_24),
    ("81fc", "h0+$28", 0, 0x28, 4, &[], i_h0_28),
    ("51c8", "h0+$2c", 0, 0x2c, 4, &[], i_h0_2c),
    ("50c8", "h0+$30", 0, 0x30, 4, &[], i_h0_30),
    ("57c8", "h0+$34", 0, 0x34, 4, &[], i_h0_34),
    ("5ec9", "h0+$38", 0, 0x38, 4, &[], i_h0_38),
    ("48e1", "h0+$3c", 0, 0x3c, 4, &[], i_h0_3c),
    ("48d1", "h0+$40", 0, 0x40, 4, &[], i_h0_40),
    ("48e7", "h0+$44", 0, 0x44, 4, &[], i_h0_44),
    ("48a9", "h0+$48", 0, 0x48, 6, &[], i_h0_48),
    ("4cd9", "h0+$4e", 0, 0x4e, 4, &[], i_h0_4e),
    ("4cd9", "h0+$52", 0, 0x52, 4, &[], i_h0_52),
    ("4c99", "h0+$56", 0, 0x56, 4, &[], i_h0_56),
    ("4cf9", "h0+$5a", 0, 0x5a, 8, &[], i_h0_5a),
    ("4e75", "h0+$62", 0, 0x62, 2, &[], i_h0_62),
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$0 mulu.w
    {
        r.tick(cy::mulu_cycles(r.d(1) as u16)); r.off_bus(cy::one_word_off(cy::mulu_cycles(r.d(1) as u16)));
        let mut u_2e600: u64 = 0;
        let mut u_2e700: u64 = 0;
        let v = int_zext(r.get(0x2, 2), 2);
        u_2e600 = v & 0xffffffff;
        let v = int_zext(r.get(0x6, 2), 2);
        u_2e700 = v & 0xffffffff;
        let v = int_mult(u_2e600, u_2e700, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2 muls.w
    {
        r.tick(cy::muls_cycles(r.d(1) as u16)); r.off_bus(cy::one_word_off(cy::muls_cycles(r.d(1) as u16)));
        let mut u_2e380: u64 = 0;
        let mut u_2e480: u64 = 0;
        let v = int_sext(r.get(0x2, 2), 2, 4);
        u_2e380 = v & 0xffffffff;
        let v = int_sext(r.get(0x6, 2), 2, 4);
        u_2e480 = v & 0xffffffff;
        let v = int_mult(u_2e380, u_2e480, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4 mulu.w
    {
        r.tick(const { cy::mulu_cycles(0x0000u16) + cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(const { cy::one_word_off(cy::mulu_cycles(0x0000u16)) });
        let mut u_2e600: u64 = 0;
        let mut u_2e700: u64 = 0;
        let v = int_zext(r.get(0x2, 2), 2);
        u_2e600 = v & 0xffffffff;
        let v = int_zext(0x0u64, 2);
        u_2e700 = v & 0xffffffff;
        let v = int_mult(u_2e600, u_2e700, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8 mulu.w
    {
        r.tick(const { cy::mulu_cycles(0xffffu16) + cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(const { cy::one_word_off(cy::mulu_cycles(0xffffu16)) });
        let mut u_2e600: u64 = 0;
        let mut u_2e700: u64 = 0;
        let v = int_zext(r.get(0x2, 2), 2);
        u_2e600 = v & 0xffffffff;
        let v = int_zext(0xffffu64, 2);
        u_2e700 = v & 0xffffffff;
        let v = int_mult(u_2e600, u_2e700, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c mulu.w
    {
        r.tick(const { cy::mulu_cycles(0x5555u16) + cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(const { cy::one_word_off(cy::mulu_cycles(0x5555u16)) });
        let mut u_2e600: u64 = 0;
        let mut u_2e700: u64 = 0;
        let v = int_zext(r.get(0x2, 2), 2);
        u_2e600 = v & 0xffffffff;
        let v = int_zext(0x5555u64, 2);
        u_2e700 = v & 0xffffffff;
        let v = int_mult(u_2e600, u_2e700, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x10) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$10 muls.w
    {
        r.tick(const { cy::muls_cycles(0xffffu16) + cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(const { cy::one_word_off(cy::muls_cycles(0xffffu16)) });
        let mut u_2e380: u64 = 0;
        let mut u_2e480: u64 = 0;
        let v = int_sext(r.get(0x2, 2), 2, 4);
        u_2e380 = v & 0xffffffff;
        let v = int_sext(0xffffu64, 2, 4);
        u_2e480 = v & 0xffffffff;
        let v = int_mult(u_2e380, u_2e480, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x14) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_14(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$14 muls.w
    {
        r.tick(const { cy::muls_cycles(0xaaaau16) + cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(const { cy::one_word_off(cy::muls_cycles(0xaaaau16)) });
        let mut u_2e380: u64 = 0;
        let mut u_2e480: u64 = 0;
        let v = int_sext(r.get(0x2, 2), 2, 4);
        u_2e380 = v & 0xffffffff;
        let v = int_sext(0xaaaau64, 2, 4);
        u_2e480 = v & 0xffffffff;
        let v = int_mult(u_2e380, u_2e480, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x18) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_18(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$18 divu.w (corrected: pcode_fix.arith)
    {
        let mut u_22100: u64 = 0;
        let mut u_22180: u64 = 0;
        r.tick(cy::divu_cycles(r.d(0), r.d(1) as u16)); r.off_bus(cy::one_word_off(cy::divu_cycles(r.d(0), r.d(1) as u16)));
        let v = int_zext(r.get(0x6, 2), 2);
        u_22100 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_22180 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divu_w(u_22180 as u32, u_22100) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$18".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x1a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1a divs.w (corrected: pcode_fix.arith)
    {
        let mut u_21c00: u64 = 0;
        let mut u_21c80: u64 = 0;
        r.tick(cy::divs_cycles(r.d(0) as i32, r.d(1) as i16)); r.off_bus(cy::one_word_off(cy::divs_cycles(r.d(0) as i32, r.d(1) as i16)));
        let v = int_sext(r.get(0x6, 2), 2, 4);
        u_21c00 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_21c80 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divs_w(u_21c80 as u32, u_21c00) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$1a".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x1c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1c divu.w (corrected: pcode_fix.arith)
    {
        let mut u_22100: u64 = 0;
        let mut u_22180: u64 = 0;
        r.tick(cy::divu_cycles(r.d(0), 0x0007u16) + const { cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(cy::one_word_off(cy::divu_cycles(r.d(0), 0x0007u16)));
        let v = int_zext(0x7u64, 2);
        u_22100 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_22180 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divu_w(u_22180 as u32, u_22100) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$1c".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x20) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_20(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$20 divu.w (corrected: pcode_fix.arith)
    {
        let mut u_22100: u64 = 0;
        let mut u_22180: u64 = 0;
        r.tick(cy::divu_cycles(r.d(0), 0x0001u16) + const { cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(cy::one_word_off(cy::divu_cycles(r.d(0), 0x0001u16)));
        let v = int_zext(0x1u64, 2);
        u_22100 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_22180 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divu_w(u_22180 as u32, u_22100) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$20".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x24) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_24(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$24 divs.w (corrected: pcode_fix.arith)
    {
        let mut u_21c00: u64 = 0;
        let mut u_21c80: u64 = 0;
        r.tick(cy::divs_cycles(r.d(0) as i32, 0xfffdu16 as i16) + const { cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(cy::one_word_off(cy::divs_cycles(r.d(0) as i32, 0xfffdu16 as i16)));
        let v = int_sext(0xfffdu64, 2, 4);
        u_21c00 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_21c80 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divs_w(u_21c80 as u32, u_21c00) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$24".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x28) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_28(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$28 divs.w (corrected: pcode_fix.arith)
    {
        let mut u_21c00: u64 = 0;
        let mut u_21c80: u64 = 0;
        r.tick(cy::divs_cycles(r.d(0) as i32, 0x0001u16 as i16) + const { cy::ea_cycles(cy::Ea::Imm, cy::Size::W) }); r.off_bus(cy::one_word_off(cy::divs_cycles(r.d(0) as i32, 0x0001u16 as i16)));
        let v = int_sext(0x1u64, 2, 4);
        u_21c00 = v & 0xffffffff;
        let v = r.get(0x0, 4);
        u_21c80 = v & 0xffffffff;
        let Some((q, v, n, z)) = pcode_rt::ops::divs_w(u_21c80 as u32, u_21c00) else {
            return Err(Stop::Trap { vector: 5, at: "h0+$28".into() });
        };
        r.set(0x0, 4, q as u64);
        r.set(0x47, 1, 0);
        r.set(0x46, 1, v);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
    }
    r.set(PC, 4, img.h(0, 0x2c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2c dbf
    {
        r.tick(cy::dbcc(cy::cc_holds(1, r), r.d(0) as u16)); r.off_bus(cy::dbcc_off(cy::cc_holds(1, r))); r.idle_first(cy::dbcc_off(cy::cc_holds(1, r)));
        let mut u_21a80: u64 = 0;
        let mut u_21b00: u64 = 0;
        if 0x0u64 != 0 { r.set(PC, 4, img.h(0, 0x30) as u64); return Ok(()); }
        let v = int_sub(r.get(0x2, 2), 0x1u64, 2);
        r.set(0x2, 2, v);
        let v = int_2comp(0x1u64, 2);
        u_21a80 = v & 0xffff;
        let v = int_notequal(r.get(0x2, 2), u_21a80, 2);
        u_21b00 = v & 0xff;
        if u_21b00 != 0 { r.set(PC, 4, img.h(0, 0x30) as u64); return Ok(()); }
    }
    r.set(PC, 4, img.h(0, 0x30) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_30(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$30 dbt
    {
        r.tick(cy::dbcc(cy::cc_holds(0, r), r.d(0) as u16)); r.off_bus(cy::dbcc_off(cy::cc_holds(0, r))); r.idle_first(cy::dbcc_off(cy::cc_holds(0, r)));
        let mut u_21a80: u64 = 0;
        let mut u_21b00: u64 = 0;
        if 0x1u64 != 0 { r.set(PC, 4, img.h(0, 0x34) as u64); return Ok(()); }
        let v = int_sub(r.get(0x2, 2), 0x1u64, 2);
        r.set(0x2, 2, v);
        let v = int_2comp(0x1u64, 2);
        u_21a80 = v & 0xffff;
        let v = int_notequal(r.get(0x2, 2), u_21a80, 2);
        u_21b00 = v & 0xff;
        if u_21b00 != 0 { r.set(PC, 4, img.h(0, 0x34) as u64); return Ok(()); }
    }
    r.set(PC, 4, img.h(0, 0x34) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_34(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$34 dbeq
    {
        r.tick(cy::dbcc(cy::cc_holds(7, r), r.d(0) as u16)); r.off_bus(cy::dbcc_off(cy::cc_holds(7, r))); r.idle_first(cy::dbcc_off(cy::cc_holds(7, r)));
        let mut u_21a80: u64 = 0;
        let mut u_21b00: u64 = 0;
        if r.get(0x45, 1) != 0 { r.set(PC, 4, img.h(0, 0x38) as u64); return Ok(()); }
        let v = int_sub(r.get(0x2, 2), 0x1u64, 2);
        r.set(0x2, 2, v);
        let v = int_2comp(0x1u64, 2);
        u_21a80 = v & 0xffff;
        let v = int_notequal(r.get(0x2, 2), u_21a80, 2);
        u_21b00 = v & 0xff;
        if u_21b00 != 0 { r.set(PC, 4, img.h(0, 0x38) as u64); return Ok(()); }
    }
    r.set(PC, 4, img.h(0, 0x38) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_38(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$38 dbgt
    {
        r.tick(cy::dbcc(cy::cc_holds(14, r), r.d(1) as u16)); r.off_bus(cy::dbcc_off(cy::cc_holds(14, r))); r.idle_first(cy::dbcc_off(cy::cc_holds(14, r)));
        let mut u_a600: u64 = 0;
        let mut u_a680: u64 = 0;
        let mut u_a780: u64 = 0;
        let mut u_21a80: u64 = 0;
        let mut u_21b00: u64 = 0;
        let v = bool_negate(r.get(0x45, 1));
        u_a600 = v & 0xff;
        let v = int_equal(r.get(0x46, 1), r.get(0x44, 1), 1);
        u_a680 = v & 0xff;
        let v = bool_and(u_a600, u_a680);
        u_a780 = v & 0xff;
        if u_a780 != 0 { r.set(PC, 4, img.h(0, 0x3c) as u64); return Ok(()); }
        let v = int_sub(r.get(0x6, 2), 0x1u64, 2);
        r.set(0x6, 2, v);
        let v = int_2comp(0x1u64, 2);
        u_21a80 = v & 0xffff;
        let v = int_notequal(r.get(0x6, 2), u_21a80, 2);
        u_21b00 = v & 0xff;
        if u_21b00 != 0 { r.set(PC, 4, img.h(0, 0x3c) as u64); return Ok(()); }
    }
    r.set(PC, 4, img.h(0, 0x3c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_3c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$3c movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::PreDec, cy::Size::L, 1) });
        bus.sync(r);
        let v = r.get(0x24, 4);
        r.set(0x304, 4, v);
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x0, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = r.get(0x304, 4);
        r.set(0x24, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x40) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_40(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$40 movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::Ind, cy::Size::L, 8) });
        bus.sync(r);
        let v = r.get(0x24, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x0, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x4, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x8, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0xc, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x10, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x14, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x18, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x1c, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x44) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_44(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$44 movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::PreDec, cy::Size::L, 14) });
        bus.sync(r);
        let v = r.get(0x3c, 4);
        r.set(0x304, 4, v);
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x38, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x34, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x30, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x2c, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x28, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x24, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x20, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x1c, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x18, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x14, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x10, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0xc, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x8, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x4, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = r.get(0x304, 4);
        r.set(0x3c, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x48) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_48(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$48 movem.w
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::D16An, cy::Size::W, 16) });
        bus.sync(r);
        let v = int_add(r.get(0x24, 4), 0x10u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x2, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x6, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0xa, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0xe, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x12, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x16, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x1a, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x1e, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x22, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x26, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x2a, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x2e, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x32, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x36, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x3a, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x3e, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x4e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4e movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::PostInc, cy::Size::L, 1) });
        bus.sync(r);
        let v = r.get(0x24, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x0, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x304, 4);
        r.set(0x24, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x52) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_52(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$52 movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::PostInc, cy::Size::L, 8) });
        bus.sync(r);
        let v = r.get(0x24, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x0, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x4, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x8, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0xc, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x10, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x14, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x18, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x1c, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x304, 4);
        r.set(0x24, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x56) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_56(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$56 movem.w
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::PostInc, cy::Size::W, 15) });
        bus.sync(r);
        let mut u_29900: u64 = 0;
        let mut u_29a80: u64 = 0;
        let mut u_29c00: u64 = 0;
        let mut u_29d80: u64 = 0;
        let mut u_29f00: u64 = 0;
        let mut u_2a080: u64 = 0;
        let mut u_2a200: u64 = 0;
        let mut u_2a380: u64 = 0;
        let mut u_2a500: u64 = 0;
        let mut u_2a800: u64 = 0;
        let mut u_2a980: u64 = 0;
        let mut u_2ab00: u64 = 0;
        let mut u_2ac80: u64 = 0;
        let mut u_2ae00: u64 = 0;
        let mut u_2af80: u64 = 0;
        let v = r.get(0x24, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_29900 = v & 0xffff;
        let v = int_sext(u_29900, 2, 4);
        r.set(0x0, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_29a80 = v & 0xffff;
        let v = int_sext(u_29a80, 2, 4);
        r.set(0x4, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_29c00 = v & 0xffff;
        let v = int_sext(u_29c00, 2, 4);
        r.set(0x8, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_29d80 = v & 0xffff;
        let v = int_sext(u_29d80, 2, 4);
        r.set(0xc, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_29f00 = v & 0xffff;
        let v = int_sext(u_29f00, 2, 4);
        r.set(0x10, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2a080 = v & 0xffff;
        let v = int_sext(u_2a080, 2, 4);
        r.set(0x14, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2a200 = v & 0xffff;
        let v = int_sext(u_2a200, 2, 4);
        r.set(0x18, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2a380 = v & 0xffff;
        let v = int_sext(u_2a380, 2, 4);
        r.set(0x1c, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2a500 = v & 0xffff;
        let v = int_sext(u_2a500, 2, 4);
        r.set(0x20, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2a800 = v & 0xffff;
        let v = int_sext(u_2a800, 2, 4);
        r.set(0x28, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2a980 = v & 0xffff;
        let v = int_sext(u_2a980, 2, 4);
        r.set(0x2c, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2ab00 = v & 0xffff;
        let v = int_sext(u_2ab00, 2, 4);
        r.set(0x30, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2ac80 = v & 0xffff;
        let v = int_sext(u_2ac80, 2, 4);
        r.set(0x34, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2ae00 = v & 0xffff;
        let v = int_sext(u_2ae00, 2, 4);
        r.set(0x38, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 2);
        u_2af80 = v & 0xffff;
        let v = int_sext(u_2af80, 2, 4);
        r.set(0x3c, 4, v);
        let v = int_add(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x304, 4);
        r.set(0x24, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x5a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_5a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$5a movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::AbsL, cy::Size::L, 8) });
        bus.sync(r);
        let v = 0x4000u64;
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x0, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x4, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x8, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0xc, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x10, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x14, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x18, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x1c, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x62) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_62(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$62 rts
    {
        r.tick(const { cy::of(cy::Op::Rts, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rts, cy::Size::L, cy::Ea::None, cy::Ea::None) });
        bus.sync(r);
        let v = load(bus, r.get(0x3c, 4) as u32, 4);
        r.set(0x50, 4, v);
        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = r.get(0x50, 4);
        r.set(PC, 4, v);
        return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x64) as u64);
    Ok(())
}
