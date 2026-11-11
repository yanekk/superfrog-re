// Generated from shift_bit.json by pcode_rust.py --insn; never committed when made from a game.
// Do not edit: regenerate.
#[allow(unused_imports)]
use pcode_rt::{Bus, Image, Regs, Stop, ops::*, load, store, PC, STEP_CAP, cycles as cy};

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
    (0, 0x18, 0x1a),
    (0, 0x1a, 0x1c),
    (0, 0x1c, 0x1e),
    (0, 0x1e, 0x20),
    (0, 0x20, 0x22),
    (0, 0x22, 0x24),
    (0, 0x24, 0x26),
    (0, 0x26, 0x28),
    (0, 0x28, 0x2a),
    (0, 0x2a, 0x2c),
    (0, 0x2c, 0x2e),
    (0, 0x2e, 0x30),
    (0, 0x30, 0x32),
    (0, 0x32, 0x34),
    (0, 0x34, 0x36),
    (0, 0x36, 0x38),
    (0, 0x38, 0x3a),
    (0, 0x3a, 0x3c),
    (0, 0x3c, 0x3e),
    (0, 0x3e, 0x40),
    (0, 0x40, 0x42),
    (0, 0x42, 0x44),
    (0, 0x44, 0x46),
    (0, 0x46, 0x48),
    (0, 0x48, 0x4a),
    (0, 0x4a, 0x4c),
    (0, 0x4c, 0x4e),
    (0, 0x4e, 0x50),
    (0, 0x50, 0x52),
    (0, 0x52, 0x54),
    (0, 0x54, 0x56),
    (0, 0x56, 0x58),
    (0, 0x58, 0x5a),
    (0, 0x5a, 0x5c),
    (0, 0x5c, 0x5e),
    (0, 0x5e, 0x60),
    (0, 0x60, 0x62),
    (0, 0x62, 0x64),
    (0, 0x64, 0x66),
    (0, 0x66, 0x68),
    (0, 0x68, 0x6a),
    (0, 0x6a, 0x6c),
    (0, 0x6c, 0x6e),
    (0, 0x6e, 0x70),
    (0, 0x70, 0x72),
    (0, 0x72, 0x74),
    (0, 0x74, 0x76),
    (0, 0x76, 0x78),
    (0, 0x78, 0x7a),
    (0, 0x7a, 0x7c),
    (0, 0x7c, 0x7e),
    (0, 0x7e, 0x80),
    (0, 0x80, 0x82),
    (0, 0x82, 0x84),
    (0, 0x84, 0x86),
    (0, 0x86, 0x88),
    (0, 0x88, 0x8a),
    (0, 0x8a, 0x8c),
    (0, 0x8c, 0x8e),
    (0, 0x8e, 0x90),
    (0, 0x90, 0x92),
    (0, 0x92, 0x94),
    (0, 0x94, 0x96),
    (0, 0x96, 0x98),
    (0, 0x98, 0x9a),
    (0, 0x9a, 0x9c),
    (0, 0x9c, 0x9e),
    (0, 0x9e, 0xa0),
    (0, 0xa0, 0xa2),
    (0, 0xa2, 0xa4),
    (0, 0xa4, 0xa8),
    (0, 0xa8, 0xac),
    (0, 0xac, 0xae),
    (0, 0xae, 0xb0),
    (0, 0xb0, 0xb4),
    (0, 0xb4, 0xb8),
    (0, 0xb8, 0xba),
    (0, 0xba, 0xbc),
    (0, 0xbc, 0xc0),
    (0, 0xc0, 0xc4),
    (0, 0xc4, 0xc6),
    (0, 0xc6, 0xc8),
    (0, 0xc8, 0xcc),
    (0, 0xcc, 0xd0),
    (0, 0xd0, 0xd2),
    (0, 0xd2, 0xd4),
    (0, 0xd4, 0xd6),
    (0, 0xd6, 0xd8),
    (0, 0xd8, 0xda),
    (0, 0xda, 0xdc),
    (0, 0xdc, 0xde),
    (0, 0xde, 0xe0),
    (0, 0xe0, 0xe2),
    (0, 0xe2, 0xe4),
    (0, 0xe4, 0xe6),
    (0, 0xe6, 0xe8),
    (0, 0xe8, 0xea),
    (0, 0xea, 0xec),
    (0, 0xec, 0xee),
    (0, 0xee, 0xf0),
    (0, 0xf0, 0xf2),
    (0, 0xf2, 0xf4),
    (0, 0xf4, 0xf6),
    (0, 0xf6, 0xf8),
    (0, 0xf8, 0xfa),
    (0, 0xfa, 0xfc),
];

/// Each hunk's length, as the export gives it.
pub const HUNK_SIZES: &[u32] = &[0xfc];

pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// (encoding, site, hunk, offset, length, hunk-relative data addresses, function), by site.
pub const SITES: &[(&str, &str, u32, u32, u32, &[(u32, u32)], Routine)] = &[
    ("e220", "h0+$0", 0, 0x0, 2, &[], i_h0_0),
    ("e200", "h0+$2", 0, 0x2, 2, &[], i_h0_2),
    ("e000", "h0+$4", 0, 0x4, 2, &[], i_h0_4),
    ("e260", "h0+$6", 0, 0x6, 2, &[], i_h0_6),
    ("e240", "h0+$8", 0, 0x8, 2, &[], i_h0_8),
    ("e040", "h0+$a", 0, 0xa, 2, &[], i_h0_a),
    ("e2a0", "h0+$c", 0, 0xc, 2, &[], i_h0_c),
    ("e280", "h0+$e", 0, 0xe, 2, &[], i_h0_e),
    ("e080", "h0+$10", 0, 0x10, 2, &[], i_h0_10),
    ("e0d0", "h0+$12", 0, 0x12, 2, &[], i_h0_12),
    ("e320", "h0+$14", 0, 0x14, 2, &[], i_h0_14),
    ("e300", "h0+$16", 0, 0x16, 2, &[], i_h0_16),
    ("e100", "h0+$18", 0, 0x18, 2, &[], i_h0_18),
    ("e360", "h0+$1a", 0, 0x1a, 2, &[], i_h0_1a),
    ("e340", "h0+$1c", 0, 0x1c, 2, &[], i_h0_1c),
    ("e140", "h0+$1e", 0, 0x1e, 2, &[], i_h0_1e),
    ("e3a0", "h0+$20", 0, 0x20, 2, &[], i_h0_20),
    ("e380", "h0+$22", 0, 0x22, 2, &[], i_h0_22),
    ("e180", "h0+$24", 0, 0x24, 2, &[], i_h0_24),
    ("e1d0", "h0+$26", 0, 0x26, 2, &[], i_h0_26),
    ("e228", "h0+$28", 0, 0x28, 2, &[], i_h0_28),
    ("e208", "h0+$2a", 0, 0x2a, 2, &[], i_h0_2a),
    ("e008", "h0+$2c", 0, 0x2c, 2, &[], i_h0_2c),
    ("e268", "h0+$2e", 0, 0x2e, 2, &[], i_h0_2e),
    ("e248", "h0+$30", 0, 0x30, 2, &[], i_h0_30),
    ("e048", "h0+$32", 0, 0x32, 2, &[], i_h0_32),
    ("e2a8", "h0+$34", 0, 0x34, 2, &[], i_h0_34),
    ("e288", "h0+$36", 0, 0x36, 2, &[], i_h0_36),
    ("e088", "h0+$38", 0, 0x38, 2, &[], i_h0_38),
    ("e2d0", "h0+$3a", 0, 0x3a, 2, &[], i_h0_3a),
    ("e328", "h0+$3c", 0, 0x3c, 2, &[], i_h0_3c),
    ("e308", "h0+$3e", 0, 0x3e, 2, &[], i_h0_3e),
    ("e108", "h0+$40", 0, 0x40, 2, &[], i_h0_40),
    ("e368", "h0+$42", 0, 0x42, 2, &[], i_h0_42),
    ("e348", "h0+$44", 0, 0x44, 2, &[], i_h0_44),
    ("e148", "h0+$46", 0, 0x46, 2, &[], i_h0_46),
    ("e3a8", "h0+$48", 0, 0x48, 2, &[], i_h0_48),
    ("e388", "h0+$4a", 0, 0x4a, 2, &[], i_h0_4a),
    ("e188", "h0+$4c", 0, 0x4c, 2, &[], i_h0_4c),
    ("e3d0", "h0+$4e", 0, 0x4e, 2, &[], i_h0_4e),
    ("e230", "h0+$50", 0, 0x50, 2, &[], i_h0_50),
    ("e210", "h0+$52", 0, 0x52, 2, &[], i_h0_52),
    ("e010", "h0+$54", 0, 0x54, 2, &[], i_h0_54),
    ("e270", "h0+$56", 0, 0x56, 2, &[], i_h0_56),
    ("e250", "h0+$58", 0, 0x58, 2, &[], i_h0_58),
    ("e050", "h0+$5a", 0, 0x5a, 2, &[], i_h0_5a),
    ("e2b0", "h0+$5c", 0, 0x5c, 2, &[], i_h0_5c),
    ("e290", "h0+$5e", 0, 0x5e, 2, &[], i_h0_5e),
    ("e090", "h0+$60", 0, 0x60, 2, &[], i_h0_60),
    ("e4d0", "h0+$62", 0, 0x62, 2, &[], i_h0_62),
    ("e330", "h0+$64", 0, 0x64, 2, &[], i_h0_64),
    ("e310", "h0+$66", 0, 0x66, 2, &[], i_h0_66),
    ("e110", "h0+$68", 0, 0x68, 2, &[], i_h0_68),
    ("e370", "h0+$6a", 0, 0x6a, 2, &[], i_h0_6a),
    ("e350", "h0+$6c", 0, 0x6c, 2, &[], i_h0_6c),
    ("e150", "h0+$6e", 0, 0x6e, 2, &[], i_h0_6e),
    ("e3b0", "h0+$70", 0, 0x70, 2, &[], i_h0_70),
    ("e390", "h0+$72", 0, 0x72, 2, &[], i_h0_72),
    ("e190", "h0+$74", 0, 0x74, 2, &[], i_h0_74),
    ("e5d0", "h0+$76", 0, 0x76, 2, &[], i_h0_76),
    ("e238", "h0+$78", 0, 0x78, 2, &[], i_h0_78),
    ("e218", "h0+$7a", 0, 0x7a, 2, &[], i_h0_7a),
    ("e018", "h0+$7c", 0, 0x7c, 2, &[], i_h0_7c),
    ("e278", "h0+$7e", 0, 0x7e, 2, &[], i_h0_7e),
    ("e258", "h0+$80", 0, 0x80, 2, &[], i_h0_80),
    ("e058", "h0+$82", 0, 0x82, 2, &[], i_h0_82),
    ("e2b8", "h0+$84", 0, 0x84, 2, &[], i_h0_84),
    ("e298", "h0+$86", 0, 0x86, 2, &[], i_h0_86),
    ("e098", "h0+$88", 0, 0x88, 2, &[], i_h0_88),
    ("e6d0", "h0+$8a", 0, 0x8a, 2, &[], i_h0_8a),
    ("e338", "h0+$8c", 0, 0x8c, 2, &[], i_h0_8c),
    ("e318", "h0+$8e", 0, 0x8e, 2, &[], i_h0_8e),
    ("e118", "h0+$90", 0, 0x90, 2, &[], i_h0_90),
    ("e378", "h0+$92", 0, 0x92, 2, &[], i_h0_92),
    ("e358", "h0+$94", 0, 0x94, 2, &[], i_h0_94),
    ("e158", "h0+$96", 0, 0x96, 2, &[], i_h0_96),
    ("e3b8", "h0+$98", 0, 0x98, 2, &[], i_h0_98),
    ("e398", "h0+$9a", 0, 0x9a, 2, &[], i_h0_9a),
    ("e198", "h0+$9c", 0, 0x9c, 2, &[], i_h0_9c),
    ("e7d0", "h0+$9e", 0, 0x9e, 2, &[], i_h0_9e),
    ("0300", "h0+$a0", 0, 0xa0, 2, &[], i_h0_a0),
    ("0310", "h0+$a2", 0, 0xa2, 2, &[], i_h0_a2),
    ("0800", "h0+$a4", 0, 0xa4, 4, &[], i_h0_a4),
    ("0810", "h0+$a8", 0, 0xa8, 4, &[], i_h0_a8),
    ("0340", "h0+$ac", 0, 0xac, 2, &[], i_h0_ac),
    ("0350", "h0+$ae", 0, 0xae, 2, &[], i_h0_ae),
    ("0840", "h0+$b0", 0, 0xb0, 4, &[], i_h0_b0),
    ("0850", "h0+$b4", 0, 0xb4, 4, &[], i_h0_b4),
    ("0380", "h0+$b8", 0, 0xb8, 2, &[], i_h0_b8),
    ("0390", "h0+$ba", 0, 0xba, 2, &[], i_h0_ba),
    ("0880", "h0+$bc", 0, 0xbc, 4, &[], i_h0_bc),
    ("0890", "h0+$c0", 0, 0xc0, 4, &[], i_h0_c0),
    ("03c0", "h0+$c4", 0, 0xc4, 2, &[], i_h0_c4),
    ("03d0", "h0+$c6", 0, 0xc6, 2, &[], i_h0_c6),
    ("08c0", "h0+$c8", 0, 0xc8, 4, &[], i_h0_c8),
    ("08d0", "h0+$cc", 0, 0xcc, 4, &[], i_h0_cc),
    ("50c0", "h0+$d0", 0, 0xd0, 2, &[], i_h0_d0),
    ("51c0", "h0+$d2", 0, 0xd2, 2, &[], i_h0_d2),
    ("52c0", "h0+$d4", 0, 0xd4, 2, &[], i_h0_d4),
    ("53c0", "h0+$d6", 0, 0xd6, 2, &[], i_h0_d6),
    ("54c0", "h0+$d8", 0, 0xd8, 2, &[], i_h0_d8),
    ("55c0", "h0+$da", 0, 0xda, 2, &[], i_h0_da),
    ("56c0", "h0+$dc", 0, 0xdc, 2, &[], i_h0_dc),
    ("57c0", "h0+$de", 0, 0xde, 2, &[], i_h0_de),
    ("58c0", "h0+$e0", 0, 0xe0, 2, &[], i_h0_e0),
    ("59c0", "h0+$e2", 0, 0xe2, 2, &[], i_h0_e2),
    ("5ac0", "h0+$e4", 0, 0xe4, 2, &[], i_h0_e4),
    ("5bc0", "h0+$e6", 0, 0xe6, 2, &[], i_h0_e6),
    ("5cc0", "h0+$e8", 0, 0xe8, 2, &[], i_h0_e8),
    ("5dc0", "h0+$ea", 0, 0xea, 2, &[], i_h0_ea),
    ("5ec0", "h0+$ec", 0, 0xec, 2, &[], i_h0_ec),
    ("5fc0", "h0+$ee", 0, 0xee, 2, &[], i_h0_ee),
    ("50d0", "h0+$f0", 0, 0xf0, 2, &[], i_h0_f0),
    ("56d0", "h0+$f2", 0, 0xf2, 2, &[], i_h0_f2),
    ("4ac0", "h0+$f4", 0, 0xf4, 2, &[], i_h0_f4),
    ("4ad0", "h0+$f6", 0, 0xf6, 2, &[], i_h0_f6),
    ("4840", "h0+$f8", 0, 0xf8, 2, &[], i_h0_f8),
    ("4e75", "h0+$fa", 0, 0xfa, 2, &[], i_h0_fa),
];

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$0 asr.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2 asr.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4 asr.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6 asr.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8 asr.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0xa) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$a asr.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0xc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c asr.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0xe) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e asr.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x10) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_10(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$10 asr.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x12) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_12(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$12 asr
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_cd80: u64 = 0;
        let mut u_ce00: u64 = 0;
        let mut u_15180: u64 = 0;
        let mut u_15280: u64 = 0;
        let mut u_15400: u64 = 0;
        let mut u_15480: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_15180 = v & 0xffff;
        let v = int_and(u_15180, 0x8000u64, 2);
        u_15280 = v & 0xffff;
        let v = int_right(u_15180, 0x0u64, 2);
        u_cd80 = v & 0xffff;
        let v = int_and(u_cd80, 0x1u64, 2);
        u_ce00 = v & 0xffff;
        let v = int_notequal(u_ce00, 0x0u64, 2);
        r.set(0x47, 1, v);
        let v = int_sright(u_15180, 0x1u64, 2);
        u_15180 = v & 0xffff;
        let v = int_sless(u_15180, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_15180, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = int_and(u_15180, 0x8000u64, 2);
        u_15400 = v & 0xffff;
        let v = int_xor(u_15280, u_15400, 2);
        u_15480 = v & 0xffff;
        let v = int_notequal(u_15480, 0x0u64, 2);
        r.set(0x46, 1, v);
        let v = u_15180;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x14) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_14(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$14 asl.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x16) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_16(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$16 asl.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x18) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_18(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$18 asl.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x1a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1a asl.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x1c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1c asl.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x1e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_1e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$1e asl.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x20) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_20(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$20 asl.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x22) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_22(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$22 asl.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x24) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_24(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$24 asl.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(0, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x26) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_26(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$26 asl
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_cd80: u64 = 0;
        let mut u_ce00: u64 = 0;
        let mut u_14a80: u64 = 0;
        let mut u_14b80: u64 = 0;
        let mut u_14d00: u64 = 0;
        let mut u_14d80: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_14a80 = v & 0xffff;
        let v = int_and(u_14a80, 0x8000u64, 2);
        u_14b80 = v & 0xffff;
        let v = int_right(u_14a80, 0xfu64, 2);
        u_cd80 = v & 0xffff;
        let v = int_and(u_cd80, 0x1u64, 2);
        u_ce00 = v & 0xffff;
        let v = int_notequal(u_ce00, 0x0u64, 2);
        r.set(0x47, 1, v);
        let v = int_left(u_14a80, 0x1u64, 2);
        u_14a80 = v & 0xffff;
        let v = int_sless(u_14a80, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_14a80, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = int_and(u_14a80, 0x8000u64, 2);
        u_14d00 = v & 0xffff;
        let v = int_xor(u_14b80, u_14d00, 2);
        u_14d80 = v & 0xffff;
        let v = int_notequal(u_14d80, 0x0u64, 2);
        r.set(0x46, 1, v);
        let v = u_14a80;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x28) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_28(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$28 lsr.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x2a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2a lsr.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x2c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2c lsr.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x2e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_2e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$2e lsr.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x30) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_30(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$30 lsr.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x32) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_32(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$32 lsr.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x34) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_34(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$34 lsr.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x36) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_36(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$36 lsr.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x38) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_38(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$38 lsr.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x3a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_3a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$3a lsr
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_cd80: u64 = 0;
        let mut u_ce00: u64 = 0;
        let mut u_24c00: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_24c00 = v & 0xffff;
        let v = int_right(u_24c00, 0x0u64, 2);
        u_cd80 = v & 0xffff;
        let v = int_and(u_cd80, 0x1u64, 2);
        u_ce00 = v & 0xffff;
        let v = int_notequal(u_ce00, 0x0u64, 2);
        r.set(0x47, 1, v);
        let v = int_right(u_24c00, 0x1u64, 2);
        u_24c00 = v & 0xffff;
        let v = int_sless(u_24c00, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_24c00, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = u_24c00;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x3c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_3c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$3c lsl.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x3e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_3e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$3e lsl.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x40) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_40(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$40 lsl.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x42) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_42(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$42 lsl.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x44) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_44(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$44 lsl.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x46) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_46(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$46 lsl.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x48) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_48(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$48 lsl.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x4a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4a lsl.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x4c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4c lsl.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(1, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x4e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_4e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$4e lsl
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_cd80: u64 = 0;
        let mut u_ce00: u64 = 0;
        let mut u_24b00: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_24b00 = v & 0xffff;
        let v = int_right(u_24b00, 0xfu64, 2);
        u_cd80 = v & 0xffff;
        let v = int_and(u_cd80, 0x1u64, 2);
        u_ce00 = v & 0xffff;
        let v = int_notequal(u_ce00, 0x0u64, 2);
        r.set(0x47, 1, v);
        let v = int_left(u_24b00, 0x1u64, 2);
        u_24b00 = v & 0xffff;
        let v = int_sless(u_24b00, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_24b00, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = u_24b00;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = r.get(0x47, 1);
        r.set(0x43, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x50) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_50(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$50 roxr.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x52) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_52(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$52 roxr.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x54) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_54(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$54 roxr.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x56) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_56(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$56 roxr.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x58) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_58(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$58 roxr.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x5a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_5a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$5a roxr.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x5c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_5c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$5c roxr.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x5e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_5e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$5e roxr.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x60) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_60(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$60 roxr.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x62) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_62(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$62 roxr
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_31480: u64 = 0;
        let mut u_31500: u64 = 0;
        let mut u_31600: u64 = 0;
        let mut u_31680: u64 = 0;
        let mut u_31700: u64 = 0;
        let mut u_31780: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_31480 = v & 0xffff;
        let v = int_and(u_31480, 0x1u64, 2);
        u_31500 = v & 0xffff;
        let v = int_notequal(u_31500, 0x0u64, 2);
        u_31600 = v & 0xff;
        let v = int_zext(r.get(0x43, 1), 1);
        u_31680 = v & 0xffff;
        let v = int_left(u_31680, 0xfu64, 2);
        u_31700 = v & 0xffff;
        let v = int_right(u_31480, 0x1u64, 2);
        u_31780 = v & 0xffff;
        let v = int_or(u_31700, u_31780, 2);
        u_31480 = v & 0xffff;
        let v = int_sless(u_31480, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_31480, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = u_31480;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = u_31600;
        r.set(0x43, 1, v);
        let v = r.get(0x43, 1);
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x64) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_64(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$64 roxl.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x66) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_66(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$66 roxl.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x68) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_68(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$68 roxl.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x6a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6a roxl.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x6c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6c roxl.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x6e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_6e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$6e roxl.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x70) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_70(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$70 roxl.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x72) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_72(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$72 roxl.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x74) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_74(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$74 roxl.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(2, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x76) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_76(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$76 roxl
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_31100: u64 = 0;
        let mut u_31180: u64 = 0;
        let mut u_31280: u64 = 0;
        let mut u_31300: u64 = 0;
        let mut u_31380: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_31100 = v & 0xffff;
        let v = int_and(u_31100, 0x8000u64, 2);
        u_31180 = v & 0xffff;
        let v = int_notequal(u_31180, 0x0u64, 2);
        u_31280 = v & 0xff;
        let v = int_left(u_31100, 0x1u64, 2);
        u_31300 = v & 0xffff;
        let v = int_zext(r.get(0x43, 1), 1);
        u_31380 = v & 0xffff;
        let v = int_or(u_31300, u_31380, 2);
        u_31100 = v & 0xffff;
        let v = int_sless(u_31100, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_31100, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = u_31100;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = u_31280;
        r.set(0x43, 1, v);
        let v = r.get(0x43, 1);
        r.set(0x47, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x78) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_78(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$78 ror.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x7a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_7a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$7a ror.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x7c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_7c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$7c ror.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x7e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_7e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$7e ror.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x80) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_80(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$80 ror.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x82) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_82(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$82 ror.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x84) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_84(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$84 ror.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x86) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_86(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$86 ror.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x88) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_88(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$88 ror.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, false, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x8a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8a ror
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_cd80: u64 = 0;
        let mut u_ce00: u64 = 0;
        let mut u_30f00: u64 = 0;
        let mut u_30f80: u64 = 0;
        let mut u_31000: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_30f00 = v & 0xffff;
        let v = int_left(u_30f00, 0xfu64, 2);
        u_30f80 = v & 0xffff;
        let v = int_right(u_30f00, 0x1u64, 2);
        u_31000 = v & 0xffff;
        let v = int_or(u_30f80, u_31000, 2);
        u_30f00 = v & 0xffff;
        let v = int_right(u_30f00, 0xfu64, 2);
        u_cd80 = v & 0xffff;
        let v = int_and(u_cd80, 0x1u64, 2);
        u_ce00 = v & 0xffff;
        let v = int_notequal(u_ce00, 0x0u64, 2);
        r.set(0x47, 1, v);
        let v = int_sless(u_30f00, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_30f00, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = u_30f00;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0x8c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8c rol.b (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x8e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_8e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$8e rol.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x90) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_90(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$90 rol.b (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::B, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 1, cnt, r.get(0x3, 1), r.get(0x43, 1));
        r.set(0x3, 1, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x92) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_92(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$92 rol.w (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x94) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_94(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$94 rol.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x96) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_96(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$96 rol.w (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::W, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 2, cnt, r.get(0x2, 2), r.get(0x43, 1));
        r.set(0x2, 2, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x98) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_98(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$98 rol.l (corrected: pcode_fix.shift_bit)
    {
        let cnt = r.get(0x4, 4) & 0x3f;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x9a) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_9a(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$9a rol.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 1u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x9c) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_9c(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$9c rol.l (corrected: pcode_fix.shift_bit)
    {
        r.tick(const { cy::of(cy::Op::Shift(8), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let cnt = 8u64;
        let [v, x, n, z, o, c] = pcode_rt::ops::shift68k(3, true, 4, cnt, r.get(0x0, 4), r.get(0x43, 1));
        r.set(0x0, 4, v);
        r.set(0x43, 1, x);
        r.set(0x44, 1, n);
        r.set(0x45, 1, z);
        r.set(0x46, 1, o);
        r.set(0x47, 1, c);
    }
    r.set(PC, 4, img.h(0, 0x9e) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_9e(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$9e rol
    {
        r.tick(const { cy::of(cy::Op::Shift(1), cy::Size::W, cy::Ea::Ind, cy::Ea::None) });
        let mut u_4f00: u64 = 0;
        let mut u_cd80: u64 = 0;
        let mut u_ce00: u64 = 0;
        let mut u_30d00: u64 = 0;
        let mut u_30d80: u64 = 0;
        let mut u_30e00: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 2);
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        u_30d00 = v & 0xffff;
        let v = int_left(u_30d00, 0x1u64, 2);
        u_30d80 = v & 0xffff;
        let v = int_right(u_30d00, 0xfu64, 2);
        u_30e00 = v & 0xffff;
        let v = int_or(u_30d80, u_30e00, 2);
        u_30d00 = v & 0xffff;
        let v = int_right(u_30d00, 0x0u64, 2);
        u_cd80 = v & 0xffff;
        let v = int_and(u_cd80, 0x1u64, 2);
        u_ce00 = v & 0xffff;
        let v = int_notequal(u_ce00, 0x0u64, 2);
        r.set(0x47, 1, v);
        let v = int_sless(u_30d00, 0x0u64, 2);
        r.set(0x44, 1, v);
        let v = int_equal(u_30d00, 0x0u64, 2);
        r.set(0x45, 1, v);
        let v = u_30d00;
        u_4f00 = v & 0xffff;
        let v = u_4f00;
        store(bus, img, r.get(0x20, 4) as u32, 2, v)?;
        let v = 0x0u64;
        r.set(0x46, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xa0) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_a0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$a0 btst.l
    {
        r.tick(const { cy::of(cy::Op::Btst, cy::Size::L, cy::Ea::Dn, cy::Ea::Dn) });
        let mut u_1a980: u64 = 0;
        let mut u_1aa80: u64 = 0;
        let mut u_1ab00: u64 = 0;
        let v = int_and(r.get(0x4, 4), 0x1fu64, 4);
        u_1a980 = v & 0xffffffff;
        let v = int_left(0x1u64, u_1a980, 4);
        u_1aa80 = v & 0xffffffff;
        let v = int_and(r.get(0x0, 4), u_1aa80, 4);
        u_1ab00 = v & 0xffffffff;
        let v = int_equal(u_1ab00, 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xa2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_a2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$a2 btst.b
    {
        r.tick(const { cy::of(cy::Op::Btst, cy::Size::B, cy::Ea::Dn, cy::Ea::Ind) });
        let mut u_5800: u64 = 0;
        let mut u_1a480: u64 = 0;
        let mut u_1a580: u64 = 0;
        let mut u_1a600: u64 = 0;
        let v = int_and(r.get(0x4, 4), 0x7u64, 4);
        u_1a480 = v & 0xffffffff;
        let v = int_left(0x1u64, u_1a480, 1);
        u_1a580 = v & 0xff;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = int_and(u_5800, u_1a580, 1);
        u_1a600 = v & 0xff;
        let v = int_equal(u_1a600, 0x0u64, 1);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xa4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_a4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$a4 btst.l
    {
        r.tick(const { cy::of(cy::Op::Btst, cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let mut u_1ac00: u64 = 0;
        let mut u_1ad00: u64 = 0;
        let mut u_1ad80: u64 = 0;
        let v = int_and(0x21u64, 0x1fu64, 1);
        u_1ac00 = v & 0xff;
        let v = int_left(0x1u64, u_1ac00, 4);
        u_1ad00 = v & 0xffffffff;
        let v = int_and(r.get(0x0, 4), u_1ad00, 4);
        u_1ad80 = v & 0xffffffff;
        let v = int_equal(u_1ad80, 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xa8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_a8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$a8 btst.b
    {
        r.tick(const { cy::of(cy::Op::Btst, cy::Size::B, cy::Ea::Imm, cy::Ea::Ind) });
        let mut u_8780: u64 = 0;
        let mut u_1a700: u64 = 0;
        let mut u_1a800: u64 = 0;
        let mut u_1a880: u64 = 0;
        let v = int_and(0x9u64, 0x7u64, 1);
        u_1a700 = v & 0xff;
        let v = int_left(0x1u64, u_1a700, 1);
        u_1a800 = v & 0xff;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_8780 = v & 0xff;
        let v = int_and(u_8780, u_1a800, 1);
        u_1a880 = v & 0xff;
        let v = int_equal(u_1a880, 0x0u64, 1);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xac) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_ac(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$ac bchg.l
    {
        let mut u_15c80: u64 = 0;
        let mut u_15d00: u64 = 0;
        let mut u_15e00: u64 = 0;
        let mut u_15e80: u64 = 0;
        let v = r.get(0x0, 4);
        u_15c80 = v & 0xffffffff;
        let v = int_and(r.get(0x4, 4), 0x1fu64, 4);
        u_15d00 = v & 0xffffffff;
        let v = int_left(0x1u64, u_15d00, 4);
        u_15e00 = v & 0xffffffff;
        let v = int_and(u_15c80, u_15e00, 4);
        u_15e80 = v & 0xffffffff;
        let v = int_equal(u_15e80, 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = int_xor(u_15c80, u_15e00, 4);
        r.set(0x0, 4, v);
    }
    r.set(PC, 4, img.h(0, 0xae) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_ae(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$ae bchg.b
    {
        r.tick(const { cy::of(cy::Op::Bset(0), cy::Size::B, cy::Ea::Dn, cy::Ea::Ind) });
        let mut u_5800: u64 = 0;
        let mut u_15580: u64 = 0;
        let mut u_15600: u64 = 0;
        let mut u_15700: u64 = 0;
        let mut u_15780: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        u_15580 = v & 0xff;
        let v = int_and(r.get(0x4, 4), 0x7u64, 4);
        u_15600 = v & 0xffffffff;
        let v = int_left(0x1u64, u_15600, 1);
        u_15700 = v & 0xff;
        let v = int_and(u_15580, u_15700, 1);
        u_15780 = v & 0xff;
        let v = int_equal(u_15780, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = int_xor(u_15580, u_15700, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xb0) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_b0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$b0 bchg.l
    {
        r.tick(const { cy::of(cy::Op::Bset(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let mut u_16000: u64 = 0;
        let mut u_16080: u64 = 0;
        let mut u_16180: u64 = 0;
        let mut u_16200: u64 = 0;
        let v = r.get(0x0, 4);
        u_16000 = v & 0xffffffff;
        let v = int_and(0x21u64, 0x1fu64, 1);
        u_16080 = v & 0xff;
        let v = int_left(0x1u64, u_16080, 4);
        u_16180 = v & 0xffffffff;
        let v = int_and(u_16000, u_16180, 4);
        u_16200 = v & 0xffffffff;
        let v = int_equal(u_16200, 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = int_xor(u_16000, u_16180, 4);
        r.set(0x0, 4, v);
    }
    r.set(PC, 4, img.h(0, 0xb4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_b4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$b4 bchg.b
    {
        r.tick(const { cy::of(cy::Op::Bset(9), cy::Size::B, cy::Ea::Imm, cy::Ea::Ind) });
        let mut u_8780: u64 = 0;
        let mut u_15900: u64 = 0;
        let mut u_15980: u64 = 0;
        let mut u_15a80: u64 = 0;
        let mut u_15b00: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_8780 = v & 0xff;
        let v = u_8780;
        u_15900 = v & 0xff;
        let v = int_and(0x9u64, 0x7u64, 1);
        u_15980 = v & 0xff;
        let v = int_left(0x1u64, u_15980, 1);
        u_15a80 = v & 0xff;
        let v = int_and(u_15900, u_15a80, 1);
        u_15b00 = v & 0xff;
        let v = int_equal(u_15b00, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = int_xor(u_15900, u_15a80, 1);
        u_8780 = v & 0xff;
        let v = u_8780;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xb8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_b8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$b8 bclr.l
    {
        let mut u_16b80: u64 = 0;
        let mut u_16c00: u64 = 0;
        let mut u_16d00: u64 = 0;
        let mut u_16d80: u64 = 0;
        let mut u_16e80: u64 = 0;
        let v = r.get(0x0, 4);
        u_16b80 = v & 0xffffffff;
        let v = int_and(r.get(0x4, 4), 0x1fu64, 4);
        u_16c00 = v & 0xffffffff;
        let v = int_left(0x1u64, u_16c00, 4);
        u_16d00 = v & 0xffffffff;
        let v = int_and(u_16b80, u_16d00, 4);
        u_16d80 = v & 0xffffffff;
        let v = int_equal(u_16d80, 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = int_negate(u_16d00, 4);
        u_16e80 = v & 0xffffffff;
        let v = int_and(u_16b80, u_16e80, 4);
        r.set(0x0, 4, v);
    }
    r.set(PC, 4, img.h(0, 0xba) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_ba(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$ba bclr.b
    {
        r.tick(const { cy::of(cy::Op::Bclr(0), cy::Size::B, cy::Ea::Dn, cy::Ea::Ind) });
        let mut u_5800: u64 = 0;
        let mut u_16380: u64 = 0;
        let mut u_16400: u64 = 0;
        let mut u_16500: u64 = 0;
        let mut u_16580: u64 = 0;
        let mut u_16680: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        u_16380 = v & 0xff;
        let v = int_and(r.get(0x4, 4), 0x7u64, 4);
        u_16400 = v & 0xffffffff;
        let v = int_left(0x1u64, u_16400, 1);
        u_16500 = v & 0xff;
        let v = int_and(u_16380, u_16500, 1);
        u_16580 = v & 0xff;
        let v = int_equal(u_16580, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = int_negate(u_16500, 1);
        u_16680 = v & 0xff;
        let v = int_and(u_16380, u_16680, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xbc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_bc(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$bc bclr.l
    {
        r.tick(const { cy::of(cy::Op::Bclr(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let mut u_16f80: u64 = 0;
        let mut u_17000: u64 = 0;
        let mut u_17100: u64 = 0;
        let mut u_17180: u64 = 0;
        let mut u_17280: u64 = 0;
        let v = r.get(0x0, 4);
        u_16f80 = v & 0xffffffff;
        let v = int_and(0x21u64, 0x1fu64, 1);
        u_17000 = v & 0xff;
        let v = int_left(0x1u64, u_17000, 4);
        u_17100 = v & 0xffffffff;
        let v = int_and(u_16f80, u_17100, 4);
        u_17180 = v & 0xffffffff;
        let v = int_equal(u_17180, 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = int_negate(u_17100, 4);
        u_17280 = v & 0xffffffff;
        let v = int_and(u_16f80, u_17280, 4);
        r.set(0x0, 4, v);
    }
    r.set(PC, 4, img.h(0, 0xc0) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c0 bclr.b
    {
        r.tick(const { cy::of(cy::Op::Bclr(9), cy::Size::B, cy::Ea::Imm, cy::Ea::Ind) });
        let mut u_8780: u64 = 0;
        let mut u_16780: u64 = 0;
        let mut u_16800: u64 = 0;
        let mut u_16900: u64 = 0;
        let mut u_16980: u64 = 0;
        let mut u_16a80: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_8780 = v & 0xff;
        let v = u_8780;
        u_16780 = v & 0xff;
        let v = int_and(0x9u64, 0x7u64, 1);
        u_16800 = v & 0xff;
        let v = int_left(0x1u64, u_16800, 1);
        u_16900 = v & 0xff;
        let v = int_and(u_16780, u_16900, 1);
        u_16980 = v & 0xff;
        let v = int_equal(u_16980, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = int_negate(u_16900, 1);
        u_16a80 = v & 0xff;
        let v = int_and(u_16780, u_16a80, 1);
        u_8780 = v & 0xff;
        let v = u_8780;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xc4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c4 bset.l
    {
        let mut u_19c00: u64 = 0;
        let mut u_19c80: u64 = 0;
        let mut u_19d80: u64 = 0;
        let mut u_19e00: u64 = 0;
        let v = r.get(0x0, 4);
        u_19c00 = v & 0xffffffff;
        let v = int_and(r.get(0x4, 4), 0x1fu64, 4);
        u_19c80 = v & 0xffffffff;
        let v = int_left(0x1u64, u_19c80, 4);
        u_19d80 = v & 0xffffffff;
        let v = int_and(u_19c00, u_19d80, 4);
        u_19e00 = v & 0xffffffff;
        let v = int_equal(u_19e00, 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = int_or(u_19c00, u_19d80, 4);
        r.set(0x0, 4, v);
    }
    r.set(PC, 4, img.h(0, 0xc6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c6 bset.b
    {
        r.tick(const { cy::of(cy::Op::Bset(0), cy::Size::B, cy::Ea::Dn, cy::Ea::Ind) });
        let mut u_5800: u64 = 0;
        let mut u_19500: u64 = 0;
        let mut u_19580: u64 = 0;
        let mut u_19680: u64 = 0;
        let mut u_19700: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        u_19500 = v & 0xff;
        let v = int_and(r.get(0x4, 4), 0x7u64, 4);
        u_19580 = v & 0xffffffff;
        let v = int_left(0x1u64, u_19580, 1);
        u_19680 = v & 0xff;
        let v = int_and(u_19500, u_19680, 1);
        u_19700 = v & 0xff;
        let v = int_equal(u_19700, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = int_or(u_19500, u_19680, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xc8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_c8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$c8 bset.l
    {
        r.tick(const { cy::of(cy::Op::Bset(1), cy::Size::L, cy::Ea::Imm, cy::Ea::Dn) });
        let mut u_19f80: u64 = 0;
        let mut u_1a000: u64 = 0;
        let mut u_1a100: u64 = 0;
        let mut u_1a180: u64 = 0;
        let v = r.get(0x0, 4);
        u_19f80 = v & 0xffffffff;
        let v = int_and(0x21u64, 0x1fu64, 1);
        u_1a000 = v & 0xff;
        let v = int_left(0x1u64, u_1a000, 4);
        u_1a100 = v & 0xffffffff;
        let v = int_and(u_19f80, u_1a100, 4);
        u_1a180 = v & 0xffffffff;
        let v = int_equal(u_1a180, 0x0u64, 4);
        r.set(0x45, 1, v);
        let v = int_or(u_19f80, u_1a100, 4);
        r.set(0x0, 4, v);
    }
    r.set(PC, 4, img.h(0, 0xcc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_cc(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$cc bset.b
    {
        r.tick(const { cy::of(cy::Op::Bset(9), cy::Size::B, cy::Ea::Imm, cy::Ea::Ind) });
        let mut u_8780: u64 = 0;
        let mut u_19880: u64 = 0;
        let mut u_19900: u64 = 0;
        let mut u_19a00: u64 = 0;
        let mut u_19a80: u64 = 0;
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_8780 = v & 0xff;
        let v = u_8780;
        u_19880 = v & 0xff;
        let v = int_and(0x9u64, 0x7u64, 1);
        u_19900 = v & 0xff;
        let v = int_left(0x1u64, u_19900, 1);
        u_19a00 = v & 0xff;
        let v = int_and(u_19880, u_19a00, 1);
        u_19a80 = v & 0xff;
        let v = int_equal(u_19a80, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = int_or(u_19880, u_19a00, 1);
        u_8780 = v & 0xff;
        let v = u_8780;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xd0) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_d0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$d0 st
    {
        let v = int_2comp(0x1u64, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xd2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_d2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$d2 sf
    {
        let v = int_2comp(0x0u64, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xd4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_d4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$d4 shi
    {
        let mut u_9d80: u64 = 0;
        let mut u_9e80: u64 = 0;
        let v = bool_or(r.get(0x47, 1), r.get(0x45, 1));
        u_9d80 = v & 0xff;
        let v = bool_negate(u_9d80);
        u_9e80 = v & 0xff;
        let v = int_2comp(u_9e80, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xd6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_d6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$d6 sls
    {
        let mut u_9f80: u64 = 0;
        let v = bool_or(r.get(0x47, 1), r.get(0x45, 1));
        u_9f80 = v & 0xff;
        let v = int_2comp(u_9f80, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xd8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_d8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$d8 scc
    {
        let mut u_a080: u64 = 0;
        let v = bool_negate(r.get(0x47, 1));
        u_a080 = v & 0xff;
        let v = int_2comp(u_a080, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xda) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_da(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$da scs
    {
        let v = int_2comp(r.get(0x47, 1), 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xdc) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_dc(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$dc sne
    {
        let mut u_a180: u64 = 0;
        let v = bool_negate(r.get(0x45, 1));
        u_a180 = v & 0xff;
        let v = int_2comp(u_a180, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xde) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_de(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$de seq
    {
        let v = int_2comp(r.get(0x45, 1), 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xe0) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e0 svc
    {
        let mut u_a280: u64 = 0;
        let v = bool_negate(r.get(0x46, 1));
        u_a280 = v & 0xff;
        let v = int_2comp(u_a280, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xe2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e2 svs
    {
        let v = int_2comp(r.get(0x46, 1), 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xe4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e4 spl
    {
        let mut u_a380: u64 = 0;
        let v = bool_negate(r.get(0x44, 1));
        u_a380 = v & 0xff;
        let v = int_2comp(u_a380, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xe6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e6 smi
    {
        let v = int_2comp(r.get(0x44, 1), 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xe8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_e8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$e8 sge
    {
        let mut u_a480: u64 = 0;
        let v = int_equal(r.get(0x46, 1), r.get(0x44, 1), 1);
        u_a480 = v & 0xff;
        let v = int_2comp(u_a480, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xea) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_ea(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$ea slt
    {
        let mut u_a580: u64 = 0;
        let v = int_notequal(r.get(0x46, 1), r.get(0x44, 1), 1);
        u_a580 = v & 0xff;
        let v = int_2comp(u_a580, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xec) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_ec(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$ec sgt
    {
        let mut u_a600: u64 = 0;
        let mut u_a680: u64 = 0;
        let mut u_a780: u64 = 0;
        let v = bool_negate(r.get(0x45, 1));
        u_a600 = v & 0xff;
        let v = int_equal(r.get(0x46, 1), r.get(0x44, 1), 1);
        u_a680 = v & 0xff;
        let v = bool_and(u_a600, u_a680);
        u_a780 = v & 0xff;
        let v = int_2comp(u_a780, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xee) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_ee(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$ee sle
    {
        let mut u_a800: u64 = 0;
        let mut u_a900: u64 = 0;
        let v = int_notequal(r.get(0x46, 1), r.get(0x44, 1), 1);
        u_a800 = v & 0xff;
        let v = bool_or(r.get(0x45, 1), u_a800);
        u_a900 = v & 0xff;
        let v = int_2comp(u_a900, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xf0) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_f0(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$f0 st
    {
        r.tick(const { cy::of(cy::Op::Scc, cy::Size::B, cy::Ea::Ind, cy::Ea::None) });
        let mut u_5800: u64 = 0;
        let v = int_2comp(0x1u64, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xf2) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_f2(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$f2 sne
    {
        r.tick(const { cy::of(cy::Op::Scc, cy::Size::B, cy::Ea::Ind, cy::Ea::None) });
        let mut u_5800: u64 = 0;
        let mut u_a180: u64 = 0;
        let v = bool_negate(r.get(0x45, 1));
        u_a180 = v & 0xff;
        let v = int_2comp(u_a180, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xf4) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_f4(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$f4 tas
    {
        r.tick(const { cy::of(cy::Op::Tas, cy::Size::B, cy::Ea::Dn, cy::Ea::None) });
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = int_sless(r.get(0x3, 1), 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x3, 1), 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = int_or(r.get(0x3, 1), 0x80u64, 1);
        r.set(0x3, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xf6) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_f6(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$f6 tas
    {
        r.tick(const { cy::of(cy::Op::Tas, cy::Size::B, cy::Ea::Ind, cy::Ea::None) });
        let mut u_5800: u64 = 0;
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = int_sless(u_5800, 0x0u64, 1);
        r.set(0x44, 1, v);
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = int_equal(u_5800, 0x0u64, 1);
        r.set(0x45, 1, v);
        let v = load(bus, r.get(0x20, 4) as u32, 1);
        u_5800 = v & 0xff;
        let v = int_or(u_5800, 0x80u64, 1);
        u_5800 = v & 0xff;
        let v = u_5800;
        store(bus, img, r.get(0x20, 4) as u32, 1, v)?;
    }
    r.set(PC, 4, img.h(0, 0xf8) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_f8(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$f8 swap
    {
        r.tick(const { cy::of(cy::Op::Swap, cy::Size::L, cy::Ea::Dn, cy::Ea::None) });
        let mut u_32d80: u64 = 0;
        let mut u_32e00: u64 = 0;
        let v = 0x0u64;
        r.set(0x46, 1, v);
        let v = 0x0u64;
        r.set(0x47, 1, v);
        let v = int_left(r.get(0x0, 4), 0x10u64, 4);
        u_32d80 = v & 0xffffffff;
        let v = int_right(r.get(0x0, 4), 0x10u64, 4);
        u_32e00 = v & 0xffffffff;
        let v = int_or(u_32d80, u_32e00, 4);
        r.set(0x0, 4, v);
        let v = int_sless(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x44, 1, v);
        let v = int_equal(r.get(0x0, 4), 0x0u64, 4);
        r.set(0x45, 1, v);
    }
    r.set(PC, 4, img.h(0, 0xfa) as u64);
    Ok(())
}

#[allow(unused_mut, unused_variables, unused_assignments, unreachable_code, unused_braces, unused_parens, unused_labels, non_snake_case, clippy::all)]
pub fn i_h0_fa(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    // h0+$fa rts
    {
        r.tick(const { cy::of(cy::Op::Rts, cy::Size::L, cy::Ea::None, cy::Ea::None) });
        let v = load(bus, r.get(0x3c, 4) as u32, 4);
        r.set(0x50, 4, v);
        let v = int_add(r.get(0x3c, 4), 0x4u64, 4);
        r.set(0x3c, 4, v);
        let v = r.get(0x50, 4);
        r.set(PC, 4, v);
        return Ok(());
    }
    r.set(PC, 4, img.h(0, 0xfc) as u64);
    Ok(())
}
