//! `abcd` and `sbcd`, which the translator carries out itself because Ghidra's P-code for them is
//! wrong (Ghidra 12.0's `68000.sinc`): the decimal adjust is a call to its user op `bcdAdjust`, which
//! nothing defines; `abcd` drops the adjusted value into X instead of the destination; the carry is the
//! binary one; and `bcdflags` sets Z when the result is non-zero, where the 68000 clears it.
//!
//! The arithmetic is Moira's (`MoiraALU_cpp.h`, `Moira::bcd`, 68000 core), the CPU the oracle runs,
//! transcribed with C's integer promotions kept: its `u16` digit sums wrap, its `u32` operand
//! differences wrap, and the second digit's adjust of `sbcd` reads the X it has just computed. That
//! is what makes non-decimal digits (a nibble above 9) come out as the oracle's do.

use crate::{CF, NF, Regs, VF, XF, ZF};

/// `dst + src + x`, decimal, on bytes: (the result byte, the new X and C, the new V).
#[inline]
pub fn bcd_add(src: u64, dst: u64, x: u64) -> (u64, u64, u64) {
    let (op1, op2, x) = (src as u32 & 0xff, dst as u32 & 0xff, x as u32 & 1);
    let lo = (op1 & 0x0f) + (op2 & 0x0f) + x;
    let hi = (op1 & 0xf0) + (op2 & 0xf0);
    let tmp = (hi + lo) as u64;
    let mut result = tmp;
    if lo > 9 {
        result += 0x06;
    }
    let x = if (result & 0x3f0) > 0x90 {
        result += 0x60;
        1
    } else {
        0
    };
    let v = (tmp & 0x80 == 0 && result & 0x80 == 0x80) as u64;
    (result & 0xff, x, v)
}

/// `dst - src - x`, decimal, on bytes: (the result byte, the new X and C, the new V).
#[inline]
pub fn bcd_sub(src: u64, dst: u64, x: u64) -> (u64, u64, u64) {
    let (op1, op2, mut x) = (src as u32 & 0xff, dst as u32 & 0xff, x as u32 & 1);
    // Moira's `u16 lo = lo2 - lo1 - x` and `u16 hi = hi2 - hi1`: computed as int, kept mod 2^16.
    let lo = (op2 & 0x0f).wrapping_sub(op1 & 0x0f).wrapping_sub(x) & 0xffff;
    let hi = (op2 & 0xf0).wrapping_sub(op1 & 0xf0) & 0xffff;
    let tmp = (hi + lo) as u64;
    let mut result = tmp;
    if lo & 0xf0 != 0 {
        result = result.wrapping_sub(0x06);
        x = (op2.wrapping_sub(op1).wrapping_sub(6).wrapping_sub(x) & 0x300 > 0xff) as u32;
    } else {
        x = (op2.wrapping_sub(op1).wrapping_sub(x) & 0x300 > 0xff) as u32;
    }
    // The new X, as Moira reads it here.
    if op2.wrapping_sub(op1).wrapping_sub(x) & 0x100 > 0xff {
        result = result.wrapping_sub(0x60);
    }
    let v = (tmp & 0x80 == 0x80 && result & 0x80 == 0) as u64;
    (result & 0xff, x as u64, v)
}

/// The flags after `abcd`/`sbcd` gave `result` with carry `x` and overflow `v`: X and C to `x`, V to
/// `v`, N to the result's top bit, and Z cleared when the result is non-zero, otherwise unchanged (so
/// a multi-byte sum's Z says whether every byte was zero).
#[inline]
pub fn bcd_flags(r: &mut Regs, result: u64, x: u64, v: u64) {
    r.set(XF, 1, x);
    r.set(CF, 1, x);
    r.set(VF, 1, v);
    r.set(NF, 1, (result >> 7) & 1);
    if result & 0xff != 0 {
        r.set(ZF, 1, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_sums_and_their_carry() {
        assert_eq!(bcd_add(0x28, 0x19, 0), (0x47, 0, 0));
        assert_eq!(bcd_add(0x01, 0x99, 0), (0x00, 1, 0));
        assert_eq!(bcd_add(0x00, 0x99, 1), (0x00, 1, 0));
        assert_eq!(bcd_add(0x99, 0x99, 1), (0x99, 1, 1));
        assert_eq!(bcd_add(0x05, 0x04, 1), (0x10, 0, 0));
    }

    #[test]
    fn decimal_differences_and_their_borrow() {
        assert_eq!(bcd_sub(0x01, 0x10, 0), (0x09, 0, 0));
        assert_eq!(bcd_sub(0x01, 0x00, 0), (0x99, 1, 0));
        assert_eq!(bcd_sub(0x00, 0x00, 1), (0x99, 1, 0));
        assert_eq!(bcd_sub(0x47, 0x47, 0), (0x00, 0, 0));
        assert_eq!(bcd_sub(0x19, 0x47, 1), (0x27, 0, 0));
    }

    // Non-decimal digits and the V flag, from a C transcription of `Moira::bcd` run over every
    // input (2026-11-07, all 262 144 agreed with these functions).
    #[test]
    fn non_decimal_digits_come_out_as_moiras() {
        assert_eq!(bcd_add(0x0f, 0x0f, 0), (0x24, 0, 0));
        assert_eq!(bcd_add(0xff, 0xff, 1), (0x65, 1, 0));
        assert_eq!(bcd_add(0x3a, 0x40, 0), (0x80, 0, 1));
        assert_eq!(bcd_sub(0x0f, 0x00, 0), (0x8b, 1, 0));
        assert_eq!(bcd_sub(0xff, 0x00, 1), (0x9a, 1, 0));
        assert_eq!(bcd_sub(0x01, 0x80, 0), (0x79, 0, 0));
        assert_eq!(bcd_sub(0x0b, 0x90, 0), (0x7f, 0, 1));
    }

    #[test]
    fn flags_clear_z_only_on_a_non_zero_byte() {
        let mut r = Regs::new();
        r.set(ZF, 1, 1);
        bcd_flags(&mut r, 0x00, 1, 0);
        assert_eq!(
            [r.get(XF, 1), r.get(CF, 1), r.get(ZF, 1), r.get(NF, 1)],
            [1, 1, 1, 0]
        );
        bcd_flags(&mut r, 0x80, 0, 1);
        assert_eq!(
            [
                r.get(XF, 1),
                r.get(CF, 1),
                r.get(ZF, 1),
                r.get(NF, 1),
                r.get(VF, 1)
            ],
            [0, 0, 0, 1, 1]
        );
        bcd_flags(&mut r, 0x00, 0, 0);
        assert_eq!(r.get(ZF, 1), 0, "a zero byte leaves Z cleared");
    }
}
