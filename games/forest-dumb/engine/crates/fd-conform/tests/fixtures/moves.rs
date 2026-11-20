// Generated from moves.json by pcode_rust.py --insn; never committed when made from a game.
// Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, insn_start, fn_entry, PC, cycles as cy};

/// The bytes of every emitted instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x2),
    (0, 0x2, 0x4),
    (0, 0x4, 0x6),
    (0, 0x6, 0x8),
    (0, 0x8, 0xc),
    (0, 0xc, 0xe),
    (0, 0xe, 0x10),
    (0, 0x10, 0x12),
    (0, 0x12, 0x16),
    (0, 0x16, 0x1a),
    (0, 0x1a, 0x1e),
    (0, 0x1e, 0x22),
    (0, 0x22, 0x26),
    (0, 0x26, 0x2a),
    (0, 0x2a, 0x2e),
    (0, 0x2e, 0x30),
    (0, 0x30, 0x32),
    (0, 0x32, 0x36),
    (0, 0x36, 0x3a),
    (0, 0x3a, 0x3e),
    (0, 0x3e, 0x42),
    (0, 0x42, 0x46),
    (0, 0x46, 0x4a),
    (0, 0x4a, 0x4c),
    (0, 0x4c, 0x4e),
    (0, 0x4e, 0x50),
    (0, 0x50, 0x52),
    (0, 0x52, 0x56),
    (0, 0x56, 0x58),
    (0, 0x58, 0x5a),
    (0, 0x5a, 0x5e),
    (0, 0x5e, 0x62),
    (0, 0x62, 0x64),
    (0, 0x64, 0x66),
    (0, 0x66, 0x6a),
    (0, 0x6a, 0x6e),
    (0, 0x6e, 0x70),
    (0, 0x70, 0x72),
    (0, 0x72, 0x76),
    (0, 0x76, 0x78),
    (0, 0x78, 0x7c),
    (0, 0x7c, 0x7e),
    (0, 0x7e, 0x80),
    (0, 0x80, 0x82),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x88];

pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// (encoding, site, hunk, offset, length, hunk-relative data addresses, function), by site.
pub const SITES: &[(&str, &str, u32, u32, u32, &[(u32, u32)], Routine)] = &[
    ("4e73", "h0+$0", 0, 0x0, 2, &[], i_h0_0),
    ("4e77", "h0+$2", 0, 0x2, 2, &[], i_h0_2),
    ("46c0", "h0+$4", 0, 0x4, 2, &[], i_h0_4),
    ("46d0", "h0+$6", 0, 0x6, 2, &[], i_h0_6),
    ("46fc", "h0+$8", 0, 0x8, 4, &[], i_h0_8),
    ("40c0", "h0+$c", 0, 0xc, 2, &[], i_h0_c),
    ("40e7", "h0+$e", 0, 0xe, 2, &[], i_h0_e),
    ("44c0", "h0+$10", 0, 0x10, 2, &[], i_h0_10),
    ("44fc", "h0+$12", 0, 0x12, 4, &[], i_h0_12),
    ("027c", "h0+$16", 0, 0x16, 4, &[], i_h0_16),
    ("007c", "h0+$1a", 0, 0x1a, 4, &[], i_h0_1a),
    ("0a7c", "h0+$1e", 0, 0x1e, 4, &[], i_h0_1e),
    ("023c", "h0+$22", 0, 0x22, 4, &[], i_h0_22),
    ("003c", "h0+$26", 0, 0x26, 4, &[], i_h0_26),
    ("0a3c", "h0+$2a", 0, 0x2a, 4, &[], i_h0_2a),
    ("4e60", "h0+$2e", 0, 0x2e, 2, &[], i_h0_2e),
    ("4e69", "h0+$30", 0, 0x30, 2, &[], i_h0_30),
    ("48e7", "h0+$32", 0, 0x32, 4, &[], i_h0_32),
    ("4cdf", "h0+$36", 0, 0x36, 4, &[], i_h0_36),
    ("48a1", "h0+$3a", 0, 0x3a, 4, &[], i_h0_3a),
    ("4c98", "h0+$3e", 0, 0x3e, 4, &[], i_h0_3e),
    ("48e0", "h0+$42", 0, 0x42, 4, &[], i_h0_42),
    ("4cd8", "h0+$46", 0, 0x46, 4, &[], i_h0_46),
    ("1f00", "h0+$4a", 0, 0x4a, 2, &[], i_h0_4a),
    ("101f", "h0+$4c", 0, 0x4c, 2, &[], i_h0_4c),
    ("1f1f", "h0+$4e", 0, 0x4e, 2, &[], i_h0_4e),
    ("1ed8", "h0+$50", 0, 0x50, 2, &[], i_h0_50),
    ("4e56", "h0+$52", 0, 0x52, 4, &[], i_h0_52),
    ("4e5e", "h0+$56", 0, 0x56, 2, &[], i_h0_56),
    ("4850", "h0+$58", 0, 0x58, 2, &[], i_h0_58),
    ("4878", "h0+$5a", 0, 0x5a, 4, &[], i_h0_5a),
    ("487a", "h0+$5e", 0, 0x5e, 4, &[(0, 0x5e)], i_h0_5e),
    ("c188", "h0+$62", 0, 0x62, 2, &[], i_h0_62),
    ("c34f", "h0+$64", 0, 0x64, 2, &[], i_h0_64),
    ("57c8", "h0+$66", 0, 0x66, 4, &[], i_h0_66),
    ("51c9", "h0+$6a", 0, 0x6a, 4, &[], i_h0_6a),
    ("4e71", "h0+$6e", 0, 0x6e, 2, &[], i_h0_6e),
    ("4e90", "h0+$70", 0, 0x70, 2, &[(0, 0x72)], i_h0_70),
    ("4eba", "h0+$72", 0, 0x72, 4, &[(0, 0x76)], i_h0_72),
    ("4ed0", "h0+$76", 0, 0x76, 2, &[], i_h0_76),
    ("6100", "h0+$78", 0, 0x78, 4, &[(0, 0x7c)], i_h0_78),
    ("6102", "h0+$7c", 0, 0x7c, 2, &[(0, 0x7e)], i_h0_7c),
    ("4e75", "h0+$7e", 0, 0x7e, 2, &[], i_h0_7e),
    ("6602", "h0+$80", 0, 0x80, 2, &[], i_h0_80),
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$0 rte
    {
        r.tick(const { cy::of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None) });
        bus.sync(r);
        let mut u_fff000: u64 = 0;
        let v = load(bus, r.get(0x3c, 4) as u32, 2);
        u_fff000 = v & 0xffff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, r.get(0x3c, 4) as u32, 4);
        r.set(0x50, 4, v);
        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        pcode_rt::ops::sr_set(r, u_fff000);
        let v = r.get(0x50, 4);
        r.set(PC, 4, v);
        return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2 rtr
    {
        r.tick(const { cy::of(cy::Op::Rtr, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Rtr, cy::Size::L, cy::Ea::None, cy::Ea::None) });
        bus.sync(r);
        let mut u_fff000: u64 = 0;
        let v = load(bus, r.get(0x3c, 4) as u32, 2);
        u_fff000 = v & 0xffff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, r.get(0x3c, 4) as u32, 4);
        r.set(0x50, 4, v);
        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        pcode_rt::ops::ccr_set(r, u_fff000);
        let v = r.get(0x50, 4);
        r.set(PC, 4, v);
        return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4 move
    {
        let mut u_fff000: u64 = 0;
        let v = r.get(0x2, 2);
        u_fff000 = v & 0xffff;
        pcode_rt::ops::sr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6 move
    {
        bus.sync(r);
        let mut u_4f00: u64 = 0;
        let mut u_fff000: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_fff000 = v & 0xffff;
        pcode_rt::ops::sr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8 move
    {
        let mut u_fff000: u64 = 0;
        let v = 0x2015u64;
        u_fff000 = v & 0xffff;
        pcode_rt::ops::sr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0xc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c move
    {
        let mut u_fff000: u64 = 0;
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = u_fff000;
        r.set(0x2, 2, v);
    }
    r.set(PC, 4, img.h(0, 0xe) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e move
    {
        bus.sync(r);
        let mut u_5180: u64 = 0;
        let mut u_fff000: u64 = 0;
        let v = int_sub(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = u_fff000;
        u_5180 = v & 0xffff;
        let v = u_5180;
        store(bus, img, r.get(0x3c, 4) as u32, 2, v)?;
    }
    r.set(PC, 4, img.h(0, 0x10) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$10 move
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::L, cy::Ea::Dn, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::L, cy::Ea::Dn, cy::Ea::None) });
        pcode_rt::ops::ccr_set(r, r.get(0x2, 2));
    }
    r.set(PC, 4, img.h(0, 0x12) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_12(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$12 move
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::L, cy::Ea::Imm, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::L, cy::Ea::Imm, cy::Ea::None) });
        pcode_rt::ops::ccr_set(r, 0x15u64);
    }
    r.set(PC, 4, img.h(0, 0x16) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_16(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$16 andi
    {
        let mut u_fff000: u64 = 0;
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = int_and(u_fff000, 0xf8ffu64, 2);
        u_fff000 = v & 0xffff;
        pcode_rt::ops::sr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x1a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1a ori
    {
        let mut u_fff000: u64 = 0;
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = int_or(u_fff000, 0x700u64, 2);
        u_fff000 = v & 0xffff;
        pcode_rt::ops::sr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x1e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1e eori
    {
        let mut u_fff000: u64 = 0;
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = int_xor(u_fff000, 0x711u64, 2);
        u_fff000 = v & 0xffff;
        pcode_rt::ops::sr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x22) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_22(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$22 andi
    {
        r.tick(const { cy::of(cy::Op::Imm, cy::Size::L, cy::Ea::Imm, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Imm, cy::Size::L, cy::Ea::Imm, cy::Ea::None) });
        let mut u_14600: u64 = 0;
        let mut u_fff000: u64 = 0;
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = int_zext(0xf0u64, 1);
        u_14600 = v & 0xffff;
        let v = int_and(u_fff000, u_14600, 2);
        u_fff000 = v & 0xffff;
        pcode_rt::ops::ccr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x26) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_26(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$26 ori
    {
        r.tick(const { cy::of(cy::Op::Imm, cy::Size::L, cy::Ea::Imm, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Imm, cy::Size::L, cy::Ea::Imm, cy::Ea::None) });
        let mut u_30200: u64 = 0;
        let mut u_fff000: u64 = 0;
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = int_zext(0x5u64, 1);
        u_30200 = v & 0xffff;
        let v = int_or(u_fff000, u_30200, 2);
        u_fff000 = v & 0xffff;
        pcode_rt::ops::ccr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x2a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2a eori
    {
        r.tick(const { cy::of(cy::Op::Imm, cy::Size::L, cy::Ea::Imm, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Imm, cy::Size::L, cy::Ea::Imm, cy::Ea::None) });
        let mut u_23880: u64 = 0;
        let mut u_fff000: u64 = 0;
        u_fff000 = pcode_rt::ops::sr_get(r) & 0xffff;
        let v = int_zext(0x1fu64, 1);
        u_23880 = v & 0xffff;
        let v = int_xor(u_fff000, u_23880, 2);
        u_fff000 = v & 0xffff;
        pcode_rt::ops::ccr_set(r, u_fff000);
    }
    r.set(PC, 4, img.h(0, 0x2e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2e move
    {
        let v = r.get(0x20, 4);
        r.set(0x11c, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x30) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_30(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$30 move
    {
        let v = r.get(0x11c, 4);
        r.set(0x24, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x32) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_32(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$32 movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::PreDec, cy::Size::L, 15) });
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
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x0, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = r.get(0x304, 4);
        r.set(0x3c, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x36) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_36(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$36 movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::PostInc, cy::Size::L, 15) });
        bus.sync(r);
        let v = r.get(0x3c, 4);
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
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x20, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x24, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x28, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x2c, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x30, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x34, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x38, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x304, 4);
        r.set(0x3c, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x3a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_3a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$3a movem.w
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::PreDec, cy::Size::W, 2) });
        bus.sync(r);
        let v = r.get(0x24, 4);
        r.set(0x304, 4, v);
        let v = int_sub(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x22, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = int_sub(r.get(0x304, 4), 0x2u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x2, 2);
        store(bus, img, r.get(0x304, 4) as u32, 2, v)?;
        let v = r.get(0x304, 4);
        r.set(0x24, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x3e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_3e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$3e movem.w
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::PostInc, cy::Size::W, 2) });
        bus.sync(r);
        let mut u_29900: u64 = 0;
        let mut u_29a80: u64 = 0;
        let v = r.get(0x20, 4);
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
        let v = r.get(0x304, 4);
        r.set(0x20, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x42) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_42(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$42 movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToMem, cy::Ea::PreDec, cy::Size::L, 3) });
        bus.sync(r);
        let v = r.get(0x20, 4);
        r.set(0x304, 4, v);
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x20, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x4, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = int_sub(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x0, 4);
        store(bus, img, r.get(0x304, 4) as u32, 4, v)?;
        let v = r.get(0x304, 4);
        r.set(0x20, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x46) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_46(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$46 movem.l
    {
        r.tick(const { cy::movem_cycles(cy::Dir::ToRegs, cy::Ea::PostInc, cy::Size::L, 2) });
        bus.sync(r);
        let v = r.get(0x20, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x0, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = load(bus, r.get(0x304, 4) as u32, 4);
        r.set(0x20, 4, v);
        let v = int_add(r.get(0x304, 4), 0x4u64, 4);
        r.set(0x304, 4, v);
        let v = r.get(0x304, 4);
        r.set(0x20, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x4a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4a move.b
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::B, cy::Ea::Dn, cy::Ea::PreDec) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::B, cy::Ea::Dn, cy::Ea::PreDec) }); r.idle_first(const { cy::lead_of(cy::Op::Move, cy::Size::B, cy::Ea::Dn, cy::Ea::PreDec) });
        bus.sync(r);
        let mut u_8b80: u64 = 0;
        let mut u_24d00: u64 = 0;
        let v = r.get(0x3, 1);
        u_24d00 = v & 0xff;
        let v = int_sub(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = u_24d00;
        u_8b80 = v & 0xff;
        let v = u_8b80;
        store(bus, img, r.get(0x3c, 4) as u32, 1, v)?;
        let v = int_sless(u_24d00, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_24d00, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x4c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4c move.b
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::B, cy::Ea::PostInc, cy::Ea::Dn) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::B, cy::Ea::PostInc, cy::Ea::Dn) });
        bus.sync(r);
        let mut u_5880: u64 = 0;
        let mut u_5980: u64 = 0;
        let mut u_24d00: u64 = 0;
        let v = r.get(0x3c, 4);
        u_5880 = v & 0xffffffff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, u_5880 as u32, 1);
        u_5980 = v & 0xff;
        let v = u_5980;
        u_24d00 = v & 0xff;
        let v = u_24d00;
        r.set(0x3, 1, v);
        let v = int_sless(u_24d00, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_24d00, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x4e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4e move.b
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::B, cy::Ea::PostInc, cy::Ea::PreDec) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::B, cy::Ea::PostInc, cy::Ea::PreDec) }); r.idle_first(const { cy::lead_of(cy::Op::Move, cy::Size::B, cy::Ea::PostInc, cy::Ea::PreDec) });
        bus.sync(r);
        let mut u_5880: u64 = 0;
        let mut u_5980: u64 = 0;
        let mut u_8b80: u64 = 0;
        let mut u_24d00: u64 = 0;
        let v = r.get(0x3c, 4);
        u_5880 = v & 0xffffffff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, u_5880 as u32, 1);
        u_5980 = v & 0xff;
        let v = u_5980;
        u_24d00 = v & 0xff;
        let v = int_sub(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = u_24d00;
        u_8b80 = v & 0xff;
        let v = u_8b80;
        store(bus, img, r.get(0x3c, 4) as u32, 1, v)?;
        let v = int_sless(u_24d00, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_24d00, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x50) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_50(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$50 move.b
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::B, cy::Ea::PostInc, cy::Ea::PostInc) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::B, cy::Ea::PostInc, cy::Ea::PostInc) });
        bus.sync(r);
        let mut u_5a00: u64 = 0;
        let mut u_5b00: u64 = 0;
        let mut u_8800: u64 = 0;
        let mut u_8900: u64 = 0;
        let mut u_24d00: u64 = 0;
        let v = r.get(0x20, 4);
        u_5a00 = v & 0xffffffff;
        let v = int_add(r.get(0x20, 4), 0x1u64, 4);
        r.set(0x20, 4, v);
        let v = load(bus, u_5a00 as u32, 1);
        u_5b00 = v & 0xff;
        let v = u_5b00;
        u_24d00 = v & 0xff;
        let v = r.get(0x3c, 4);
        u_8800 = v & 0xffffffff;
        let v = int_add(r.get(0x3c, 4), 0x2u64, 4);
        r.set(0x3c, 4, v);
        let v = u_24d00;
        u_8900 = v & 0xff;
        let v = u_8900;
        store(bus, img, u_8800 as u32, 1, v)?;
        let v = int_sless(u_24d00, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_24d00, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x52) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_52(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$52 link.w
    {
        bus.sync(r);
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = r.get(0x38, 4);
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
        let v = r.get(0x3c, 4);
        r.set(0x38, 4, v);
        let v = int_add(r.get(0x3c, 4), 0xfffffff8u64, 4);
        r.set(0x3c, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x56) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_56(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$56 unlk
    {
        r.tick(const { cy::of(cy::Op::Unlk, cy::Size::L, cy::Ea::An, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Unlk, cy::Size::L, cy::Ea::An, cy::Ea::None) });
        bus.sync(r);
        let v = r.get(0x38, 4);
        r.set(0x3c, 4, v);
        let v = load(bus, r.get(0x3c, 4) as u32, 4);
        r.set(0x38, 4, v);
        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x58) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_58(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$58 pea
    {
        r.tick(const { cy::of(cy::Op::Pea, cy::Size::L, cy::Ea::Ind, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Pea, cy::Size::L, cy::Ea::Ind, cy::Ea::None) });
        bus.sync(r);
        let mut u_30c00: u64 = 0;
        let v = r.get(0x20, 4);
        u_30c00 = v & 0xffffffff;
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = u_30c00;
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
    }
    r.set(PC, 4, img.h(0, 0x5a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_5a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$5a pea
    {
        r.tick(const { cy::of(cy::Op::Pea, cy::Size::L, cy::Ea::AbsW, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Pea, cy::Size::L, cy::Ea::AbsW, cy::Ea::None) });
        bus.sync(r);
        let mut u_30c00: u64 = 0;
        let v = 0x1234u64;
        u_30c00 = v & 0xffffffff;
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = u_30c00;
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
    }
    r.set(PC, 4, img.h(0, 0x5e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_5e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$5e pea
    {
        r.tick(const { cy::of(cy::Op::Pea, cy::Size::L, cy::Ea::D16Pc, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Pea, cy::Size::L, cy::Ea::D16Pc, cy::Ea::None) });
        bus.sync(r);
        let mut u_9400: u64 = 0;
        let mut u_9500: u64 = 0;
        let mut u_30c00: u64 = 0;
        let v = int_add((img.h(0, 0x5e) as u64), 0x2u64, 4);
        u_9400 = v & 0xffffffff;
        let v = int_add(u_9400, 0x10u64, 4);
        u_9500 = v & 0xffffffff;
        let v = u_9500;
        u_30c00 = v & 0xffffffff;
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = u_30c00;
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
    }
    r.set(PC, 4, img.h(0, 0x62) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_62(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$62 exg
    {
        r.tick(const { cy::of(cy::Op::Exg, cy::Size::L, cy::Ea::Dn, cy::Ea::An) }); r.off_bus(const { cy::off_of(cy::Op::Exg, cy::Size::L, cy::Ea::Dn, cy::Ea::An) });
        let mut u_23b00: u64 = 0;
        let v = r.get(0x0, 4);
        u_23b00 = v & 0xffffffff;
        let v = r.get(0x20, 4);
        r.set(0x0, 4, v);
        let v = u_23b00;
        r.set(0x20, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x64) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_64(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$64 exg
    {
        r.tick(const { cy::of(cy::Op::Exg, cy::Size::L, cy::Ea::An, cy::Ea::An) }); r.off_bus(const { cy::off_of(cy::Op::Exg, cy::Size::L, cy::Ea::An, cy::Ea::An) });
        let mut u_23a80: u64 = 0;
        let v = r.get(0x24, 4);
        u_23a80 = v & 0xffffffff;
        let v = r.get(0x3c, 4);
        r.set(0x24, 4, v);
        let v = u_23a80;
        r.set(0x3c, 4, v);
    }
    r.set(PC, 4, img.h(0, 0x66) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_66(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$66 dbeq
    {
        r.tick(cy::dbcc(cy::cc_holds(7, r), r.d(0) as u16)); r.off_bus(cy::dbcc_off(cy::cc_holds(7, r))); r.idle_first(cy::dbcc_off(cy::cc_holds(7, r)));
        let mut u_21a80: u64 = 0;
        let mut u_21b00: u64 = 0;
        if r.get(0x45, 1) != 0 { r.set(PC, 4, img.h(0, 0x6a) as u64); return Ok(()); }
        let v = int_sub(r.get(0x2, 2), 0x1u64, 2);
        r.set(0x2, 2, v);
        let v = int_2comp(0x1u64, 2);
        u_21a80 = v & 0xffff;
        let v = int_notequal(r.get(0x2, 2), u_21a80, 2);
        u_21b00 = v & 0xff;
        if u_21b00 != 0 { r.set(PC, 4, img.h(0, 0x66) as u64); return Ok(()); }
    }
    r.set(PC, 4, img.h(0, 0x6a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6a dbf
    {
        r.tick(cy::dbcc(cy::cc_holds(1, r), r.d(1) as u16)); r.off_bus(cy::dbcc_off(cy::cc_holds(1, r))); r.idle_first(cy::dbcc_off(cy::cc_holds(1, r)));
        let mut u_21a80: u64 = 0;
        let mut u_21b00: u64 = 0;
        if 0x0u64 != 0 { r.set(PC, 4, img.h(0, 0x6e) as u64); return Ok(()); }
        let v = int_sub(r.get(0x6, 2), 0x1u64, 2);
        r.set(0x6, 2, v);
        let v = int_2comp(0x1u64, 2);
        u_21a80 = v & 0xffff;
        let v = int_notequal(r.get(0x6, 2), u_21a80, 2);
        u_21b00 = v & 0xff;
        if u_21b00 != 0 { r.set(PC, 4, img.h(0, 0x70) as u64); return Ok(()); }
    }
    r.set(PC, 4, img.h(0, 0x6e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6e nop
    {
        r.tick(const { cy::of(cy::Op::Nop, cy::Size::L, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Nop, cy::Size::L, cy::Ea::None, cy::Ea::None) });
    }
    r.set(PC, 4, img.h(0, 0x70) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_70(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$70 jsr
    {
        r.tick(const { cy::of(cy::Op::Jsr, cy::Size::L, cy::Ea::Ind, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Jsr, cy::Size::L, cy::Ea::Ind, cy::Ea::None) }); r.idle_first(const { cy::lead_of(cy::Op::Jsr, cy::Size::L, cy::Ea::Ind, cy::Ea::None) });
        bus.sync(r);
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = (img.h(0, 0x72) as u64);
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
        r.set(PC, 4, (r.get(0x20, 4)) as u32 as u64); return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x72) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_72(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$72 jsr
    {
        r.tick(const { cy::of(cy::Op::Jsr, cy::Size::L, cy::Ea::D16Pc, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Jsr, cy::Size::L, cy::Ea::D16Pc, cy::Ea::None) }); r.idle_first(const { cy::lead_of(cy::Op::Jsr, cy::Size::L, cy::Ea::D16Pc, cy::Ea::None) });
        bus.sync(r);
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = (img.h(0, 0x76) as u64);
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
        r.set(PC, 4, img.h(0, 0x78) as u64); return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x76) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_76(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$76 jmp
    {
        r.tick(const { cy::of(cy::Op::Jmp, cy::Size::L, cy::Ea::Ind, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Jmp, cy::Size::L, cy::Ea::Ind, cy::Ea::None) }); r.idle_first(const { cy::lead_of(cy::Op::Jmp, cy::Size::L, cy::Ea::Ind, cy::Ea::None) });
        r.set(PC, 4, (r.get(0x20, 4)) as u32 as u64); return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x78) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_78(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$78 bsr.w
    {
        r.tick(const { cy::of(cy::Op::Bsr, cy::Size::W, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Bsr, cy::Size::W, cy::Ea::None, cy::Ea::None) }); r.idle_first(const { cy::lead_of(cy::Op::Bsr, cy::Size::W, cy::Ea::None, cy::Ea::None) });
        bus.sync(r);
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = (img.h(0, 0x7c) as u64);
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
        r.set(PC, 4, img.h(0, 0x7e) as u64); return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x7c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_7c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$7c bsr.b
    {
        r.tick(const { cy::of(cy::Op::Bsr, cy::Size::B, cy::Ea::None, cy::Ea::None) }); r.off_bus(const { cy::off_of(cy::Op::Bsr, cy::Size::B, cy::Ea::None, cy::Ea::None) }); r.idle_first(const { cy::lead_of(cy::Op::Bsr, cy::Size::B, cy::Ea::None, cy::Ea::None) });
        bus.sync(r);
        let v = int_sub(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = (img.h(0, 0x7e) as u64);
        store(bus, img, r.get(0x3c, 4) as u32, 4, v)?;
        r.set(PC, 4, img.h(0, 0x80) as u64); return Ok(());
    }
    r.set(PC, 4, img.h(0, 0x7e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_7e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$7e rts
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
    r.set(PC, 4, img.h(0, 0x80) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_80(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$80 bne.b
    {
        let mut u_a180: u64 = 0;
        let v = bool_negate(r.get(0x45, 1));
        u_a180 = v & 0xff;
        if u_a180 != 0 { r.tick(const { cy::bcc(true, cy::Size::B) }); r.off_bus(const { cy::bcc_off(true) }); r.idle_first(const { cy::bcc_off(true) }); r.set(PC, 4, img.h(0, 0x84) as u64); return Ok(()); }
        r.tick(const { cy::bcc(false, cy::Size::B) }); r.off_bus(const { cy::bcc_off(false) }); r.idle_first(const { cy::bcc_off(false) });
    }
    r.set(PC, 4, img.h(0, 0x82) as u64);
    Ok(())
}
