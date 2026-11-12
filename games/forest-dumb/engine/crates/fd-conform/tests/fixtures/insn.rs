// Generated from insn.json by pcode_rust.py --insn; never committed when made from a game.
// Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, block_start, fn_entry, PC, cycles as cy};

/// The bytes of every emitted instruction, (hunk, start, end exclusive), sorted.
pub const CODE: &[(u32, u32, u32)] = &[
    (0, 0x0, 0x2),
    (0, 0x2, 0x4),
    (0, 0x4, 0x6),
    (0, 0x6, 0xa),
    (0, 0xa, 0xc),
    (0, 0xc, 0xe),
    (0, 0xe, 0x14),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0x20, 0x10];

pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// (encoding, site, hunk, offset, length, hunk-relative data addresses, function), by site.
pub const SITES: &[(&str, &str, u32, u32, u32, &[(u32, u32)], Routine)] = &[
    ("70ff", "h0+$0", 0, 0x0, 2, &[], i_h0_0),
    ("d081", "h0+$2", 0, 0x2, 2, &[], i_h0_2),
    ("c101", "h0+$4", 0, 0x4, 2, &[], i_h0_4),
    ("0c03", "h0+$6", 0, 0x6, 4, &[], i_h0_6),
    ("66f4", "h0+$a", 0, 0xa, 2, &[], i_h0_a),
    ("30c0", "h0+$c", 0, 0xc, 2, &[], i_h0_c),
    ("23c0", "h0+$e", 0, 0xe, 6, &[(1, 0x4)], i_h0_e),
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$0 moveq
    {
        r.tick(const { cy::of(cy::Op::Moveq, cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) }); r.off_bus(const { cy::off_of(cy::Op::Moveq, cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let v = 0xffffffffu64;
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2 add.l
    {
        r.tick(const { cy::of(cy::Op::AluToDn, cy::Size::L, cy::Ea::Dn, cy::Ea::Dn) }); r.off_bus(const { cy::off_of(cy::Op::AluToDn, cy::Size::L, cy::Ea::Dn, cy::Ea::Dn) });
        let v = int_carry(r.get(0x0, 4), r.get(0x4, 4), 4);
        r.set(0x47, 1, v);
        let v = int_scarry(r.get(0x0, 4), r.get(0x4, 4), 4);
        r.set(0x46, 1, v);
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
        let v = int_add(r.get(0x0, 4), r.get(0x4, 4), 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4 abcd (corrected: pcode_fix.bcd)
    {
        r.tick(const { cy::of(cy::Op::Bcd, cy::Size::L, cy::Ea::Dn, cy::Ea::Dn) }); r.off_bus(const { cy::off_of(cy::Op::Bcd, cy::Size::L, cy::Ea::Dn, cy::Ea::Dn) });
        let (v, x, o) = pcode_rt::bcd::bcd_add(r.get(0x7, 1), r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        pcode_rt::bcd::bcd_flags(r, v, x, o);
    }
    r.set(PC, 4, img.h(0, 0x6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6 cmpi.b
    {
        r.tick(const { cy::of(cy::Op::Cmpi, cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) }); r.off_bus(const { cy::off_of(cy::Op::Cmpi, cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let mut u_20b00: u64 = 0;
        let mut u_20c00: u64 = 0;
        let v = r.get(0xf, 1);
        u_20b00 = v & 0xff;
        let v = int_less(u_20b00, 0x1u64, 1);
        r.set(0x47, 1, v);
        let v = int_sborrow(u_20b00, 0x1u64, 1);
        r.set(0x46, 1, v);
        let v = int_sub(u_20b00, 0x1u64, 1);
        u_20c00 = v & 0xff;
        let v = int_sless(u_20c00, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(u_20c00, 0x0u64, 1);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xa) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$a bne.b
    {
        let mut u_100: u64 = 0;
        let v = bool_negate(r.get(0x45, 1));
        u_100 = v & 0xff;
        if u_100 != 0 { r.tick(const { cy::bcc(true, cy::Size::B) }); r.off_bus(const { cy::bcc_off(true) }); r.set(PC, 4, img.h(0, 0x0) as u64); return Ok(()); }
        r.tick(const { cy::bcc(false, cy::Size::B) }); r.off_bus(const { cy::bcc_off(false) });
    }
    r.set(PC, 4, img.h(0, 0xc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c move.w
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::W, cy::Ea::Dn, cy::Ea::PostInc) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::W, cy::Ea::Dn, cy::Ea::PostInc) });
        let mut u_200: u64 = 0;
        let v = r.get(0x20, 4);
        u_200 = v & 0xffffffff;
        let v = int_add(r.get(0x20, 4), 0x2u64, 4);
        r.set(0x20, 4, v);
        let v = r.get(0x2, 2);
        store(bus, img, u_200 as u32, 2, v)?;
        let v = int_sless(r.get(0x2, 2), 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x2, 2), 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xe) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e move.l
    {
        r.tick(const { cy::of(cy::Op::Move, cy::Size::L, cy::Ea::Dn, cy::Ea::AbsL) }); r.off_bus(const { cy::off_of(cy::Op::Move, cy::Size::L, cy::Ea::Dn, cy::Ea::AbsL) });
        let v = r.get(0x0, 4);
        store(bus, img, (img.h(1, 0x4) as u64) as u32, 4, v)?;
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x14) as u64);
    Ok(())
}
