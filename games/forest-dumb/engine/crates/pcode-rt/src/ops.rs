//! P-code's integer and boolean operations, as Ghidra's own emulator carries them out.
//!
//! Every value is held in a `u64` and has a size of 1 to 8 bytes; inputs may carry junk above their
//! size, which each op masks off, and every result is masked to its output size. Comparisons and the
//! carry/borrow tests return 0 or 1, P-code's one-byte booleans. A shift amount is the plain value of
//! its own varnode; it is not masked to the shifted value's size, because P-code defines a shift by
//! the size or more (0, or all sign bits) rather than wrapping it.

/// The bits a value of `size` bytes holds.
#[inline]
pub fn mask(size: usize) -> u64 {
    debug_assert!((1..=8).contains(&size), "P-code size {size}");
    if size >= 8 {
        u64::MAX
    } else {
        (1u64 << (size * 8)) - 1
    }
}

#[inline]
fn bits(size: usize) -> u64 {
    size as u64 * 8
}

/// `v` of `size` bytes, sign-extended.
#[inline]
pub fn signed(v: u64, size: usize) -> i64 {
    let shift = 64 - bits(size) as u32;
    ((v << shift) as i64) >> shift
}

/// Whether `r` fits a signed value of `size` bytes.
#[inline]
fn fits_signed(r: i128, size: usize) -> bool {
    let half = 1i128 << (bits(size) - 1);
    (-half..half).contains(&r)
}

#[inline]
fn b(c: bool) -> u64 {
    c as u64
}

#[inline]
pub fn int_add(a: u64, b: u64, size: usize) -> u64 {
    a.wrapping_add(b) & mask(size)
}

#[inline]
pub fn int_sub(a: u64, b: u64, size: usize) -> u64 {
    a.wrapping_sub(b) & mask(size)
}

/// 1 when `a + b` carries out of the top bit.
#[inline]
pub fn int_carry(a: u64, b_: u64, size: usize) -> u64 {
    let m = mask(size);
    b((a & m) as u128 + (b_ & m) as u128 > m as u128)
}

/// 1 when `a + b` overflows as signed values.
#[inline]
pub fn int_scarry(a: u64, b_: u64, size: usize) -> u64 {
    b(!fits_signed(
        signed(a, size) as i128 + signed(b_, size) as i128,
        size,
    ))
}

/// 1 when `a - b` overflows as signed values.
#[inline]
pub fn int_sborrow(a: u64, b_: u64, size: usize) -> u64 {
    b(!fits_signed(
        signed(a, size) as i128 - signed(b_, size) as i128,
        size,
    ))
}

#[inline]
pub fn int_less(a: u64, b_: u64, size: usize) -> u64 {
    let m = mask(size);
    b((a & m) < (b_ & m))
}

#[inline]
pub fn int_lessequal(a: u64, b_: u64, size: usize) -> u64 {
    let m = mask(size);
    b((a & m) <= (b_ & m))
}

#[inline]
pub fn int_sless(a: u64, b_: u64, size: usize) -> u64 {
    b(signed(a, size) < signed(b_, size))
}

#[inline]
pub fn int_slessequal(a: u64, b_: u64, size: usize) -> u64 {
    b(signed(a, size) <= signed(b_, size))
}

#[inline]
pub fn int_equal(a: u64, b_: u64, size: usize) -> u64 {
    let m = mask(size);
    b((a & m) == (b_ & m))
}

#[inline]
pub fn int_notequal(a: u64, b_: u64, size: usize) -> u64 {
    let m = mask(size);
    b((a & m) != (b_ & m))
}

/// Two's complement negation.
#[inline]
pub fn int_2comp(a: u64, size: usize) -> u64 {
    a.wrapping_neg() & mask(size)
}

/// Bitwise not (P-code's `INT_NEGATE` is not arithmetic negation).
#[inline]
pub fn int_negate(a: u64, size: usize) -> u64 {
    !a & mask(size)
}

/// `a` of `in_size` bytes, zero-extended; the output size cannot add bits, so it is not needed.
#[inline]
pub fn int_zext(a: u64, in_size: usize) -> u64 {
    a & mask(in_size)
}

#[inline]
pub fn int_sext(a: u64, in_size: usize, out_size: usize) -> u64 {
    signed(a, in_size) as u64 & mask(out_size)
}

#[inline]
pub fn int_mult(a: u64, b_: u64, size: usize) -> u64 {
    a.wrapping_mul(b_) & mask(size)
}

/// Unsigned division; by zero gives 0 (the 68000's divide trap is decided by the caller's P-code).
#[inline]
pub fn int_div(a: u64, b_: u64, size: usize) -> u64 {
    let m = mask(size);
    (a & m).checked_div(b_ & m).unwrap_or(0)
}

/// Signed division, truncating toward zero; by zero gives 0; the most negative value by -1 wraps.
#[inline]
pub fn int_sdiv(a: u64, b_: u64, size: usize) -> u64 {
    let d = signed(b_, size);
    if d == 0 {
        return 0;
    }
    signed(a, size).wrapping_div(d) as u64 & mask(size)
}

#[inline]
pub fn int_rem(a: u64, b_: u64, size: usize) -> u64 {
    let m = mask(size);
    (a & m).checked_rem(b_ & m).unwrap_or(0)
}

/// Signed remainder, its sign the dividend's; by zero gives 0.
#[inline]
pub fn int_srem(a: u64, b_: u64, size: usize) -> u64 {
    let d = signed(b_, size);
    if d == 0 {
        return 0;
    }
    signed(a, size).wrapping_rem(d) as u64 & mask(size)
}

#[inline]
pub fn int_left(a: u64, sh: u64, size: usize) -> u64 {
    if sh >= bits(size) {
        0
    } else {
        (a << sh) & mask(size)
    }
}

#[inline]
pub fn int_right(a: u64, sh: u64, size: usize) -> u64 {
    if sh >= bits(size) {
        0
    } else {
        (a & mask(size)) >> sh
    }
}

#[inline]
pub fn int_sright(a: u64, sh: u64, size: usize) -> u64 {
    let x = signed(a, size);
    let r = if sh >= bits(size) { x >> 63 } else { x >> sh };
    r as u64 & mask(size)
}

#[inline]
pub fn int_and(a: u64, b_: u64, size: usize) -> u64 {
    a & b_ & mask(size)
}

#[inline]
pub fn int_or(a: u64, b_: u64, size: usize) -> u64 {
    (a | b_) & mask(size)
}

#[inline]
pub fn int_xor(a: u64, b_: u64, size: usize) -> u64 {
    (a ^ b_) & mask(size)
}

/// Ghidra's emulator flips the low bit (`in ^ 1`); P-code booleans are 0 or 1, so this is `!`.
#[inline]
pub fn bool_negate(a: u64) -> u64 {
    (a ^ 1) & 0xff
}

#[inline]
pub fn bool_and(a: u64, b_: u64) -> u64 {
    a & b_ & 0xff
}

#[inline]
pub fn bool_or(a: u64, b_: u64) -> u64 {
    (a | b_) & 0xff
}

#[inline]
pub fn bool_xor(a: u64, b_: u64) -> u64 {
    (a ^ b_) & 0xff
}

/// `out_size` bytes of `a` (of `in_size` bytes) after dropping its `off` least significant bytes.
#[inline]
pub fn subpiece(a: u64, in_size: usize, off: usize, out_size: usize) -> u64 {
    if off >= in_size {
        return 0;
    }
    ((a & mask(in_size)) >> (off * 8)) & mask(out_size)
}

/// `hi` above `lo` (of `lo_size` bytes), cut to `out_size` bytes.
#[inline]
pub fn piece(hi: u64, lo: u64, lo_size: usize, out_size: usize) -> u64 {
    let high = hi.checked_shl(lo_size as u32 * 8).unwrap_or(0);
    (high | (lo & mask(lo_size))) & mask(out_size)
}

#[inline]
pub fn popcount(a: u64, size: usize) -> u64 {
    (a & mask(size)).count_ones() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: [usize; 4] = [1, 2, 4, 8];

    /// The most negative value of each size, then its top bit, used by several tests.
    fn min(size: usize) -> u64 {
        1u64 << (size * 8 - 1)
    }

    #[test]
    fn add_and_sub_wrap_at_every_size() {
        assert_eq!(int_add(0xff, 1, 1), 0);
        assert_eq!(int_add(0xffff, 1, 2), 0);
        assert_eq!(int_add(0xffff_ffff, 1, 4), 0);
        assert_eq!(int_add(u64::MAX, 1, 8), 0);
        assert_eq!(int_add(0x12, 0x34, 1), 0x46);
        assert_eq!(int_add(0x1234, 0x0100, 2), 0x1334);
        assert_eq!(int_sub(0, 1, 1), 0xff);
        assert_eq!(int_sub(0, 1, 2), 0xffff);
        assert_eq!(int_sub(0, 1, 4), 0xffff_ffff);
        assert_eq!(int_sub(0, 1, 8), u64::MAX);
        assert_eq!(int_sub(0x8000_0000, 1, 4), 0x7fff_ffff);
        // Junk above the size is ignored.
        assert_eq!(int_add(0xab01, 0xcd01, 1), 0x02);
    }

    #[test]
    fn carry_out_of_the_top_bit() {
        for s in SIZES {
            let m = mask(s);
            assert_eq!(int_carry(m, 1, s), 1, "size {s}");
            assert_eq!(int_carry(m, 0, s), 0, "size {s}");
            assert_eq!(int_carry(m - 1, 1, s), 0, "size {s}");
            assert_eq!(int_carry(min(s), min(s), s), 1, "size {s}");
        }
        // Junk above the size does not carry.
        assert_eq!(int_carry(0x1_00, 0x1_00, 1), 0);
    }

    #[test]
    fn signed_overflow_both_ways() {
        for s in SIZES {
            let max = min(s) - 1;
            // add: max + 1 overflows up, min + -1 overflows down, -1 + 1 does not.
            assert_eq!(int_scarry(max, 1, s), 1, "size {s}");
            assert_eq!(int_scarry(min(s), mask(s), s), 1, "size {s}");
            assert_eq!(int_scarry(mask(s), 1, s), 0, "size {s}");
            assert_eq!(int_scarry(max, 0, s), 0, "size {s}");
            // sub: min - 1 overflows down, max - -1 overflows up, 0 - min overflows.
            assert_eq!(int_sborrow(min(s), 1, s), 1, "size {s}");
            assert_eq!(int_sborrow(max, mask(s), s), 1, "size {s}");
            assert_eq!(int_sborrow(0, min(s), s), 1, "size {s}");
            assert_eq!(int_sborrow(mask(s), min(s), s), 0, "size {s}");
            assert_eq!(int_sborrow(1, 1, s), 0, "size {s}");
        }
        assert_eq!(int_scarry(0x7f, 0x01, 1), 1);
        assert_eq!(int_sborrow(0x80, 0x01, 1), 1);
    }

    #[test]
    fn comparisons_unsigned_and_signed() {
        for s in SIZES {
            let m = mask(s);
            assert_eq!(int_less(1, m, s), 1, "size {s}");
            assert_eq!(int_sless(1, m, s), 0, "size {s}");
            assert_eq!(int_sless(m, 1, s), 1, "size {s}");
            assert_eq!(int_sless(min(s), min(s) - 1, s), 1, "size {s}");
            assert_eq!(int_less(min(s), min(s) - 1, s), 0, "size {s}");
            assert_eq!(int_lessequal(m, m, s), 1, "size {s}");
            assert_eq!(int_slessequal(m, 0, s), 1, "size {s}");
            assert_eq!(int_slessequal(0, m, s), 0, "size {s}");
            assert_eq!(int_equal(m, u64::MAX, s), 1, "size {s}");
            assert_eq!(int_notequal(m, u64::MAX, s), 0, "size {s}");
            assert_eq!(int_notequal(0, 1, s), 1, "size {s}");
        }
        assert_eq!(int_equal(0x100, 0, 1), 1);
    }

    #[test]
    fn negations() {
        for s in SIZES {
            assert_eq!(int_2comp(1, s), mask(s), "size {s}");
            assert_eq!(int_2comp(0, s), 0, "size {s}");
            assert_eq!(int_2comp(min(s), s), min(s), "size {s}");
            assert_eq!(int_negate(0, s), mask(s), "size {s}");
            assert_eq!(int_negate(mask(s), s), 0, "size {s}");
        }
        assert_eq!(int_2comp(5, 2), 0xfffb);
        assert_eq!(int_negate(0x0f, 1), 0xf0);
    }

    #[test]
    fn zext_and_sext_of_the_top_bit() {
        assert_eq!(int_zext(0x1_80, 1), 0x80);
        assert_eq!(int_zext(0xffff_8000, 2), 0x8000);
        assert_eq!(int_zext(0x8000_0000, 4), 0x8000_0000);
        assert_eq!(int_sext(0x80, 1, 2), 0xff80);
        assert_eq!(int_sext(0x80, 1, 4), 0xffff_ff80);
        assert_eq!(int_sext(0x7f, 1, 4), 0x7f);
        assert_eq!(int_sext(0x8000, 2, 4), 0xffff_8000);
        assert_eq!(int_sext(0x7fff, 2, 4), 0x7fff);
        assert_eq!(int_sext(0x8000_0000, 4, 8), 0xffff_ffff_8000_0000);
        assert_eq!(int_sext(0x8000, 2, 8), 0xffff_ffff_ffff_8000);
        assert_eq!(int_sext(0x8000_0000_0000_0000, 8, 8), 0x8000_0000_0000_0000);
    }

    #[test]
    fn multiply() {
        assert_eq!(int_mult(0x10, 0x10, 1), 0);
        assert_eq!(int_mult(0x0f, 0x11, 1), 0xff);
        assert_eq!(int_mult(0xffff, 0xffff, 2), 1);
        assert_eq!(int_mult(0xffff, 0xffff, 4), 0xfffe_0001);
        assert_eq!(int_mult(0xffff_ffff, 0xffff_ffff, 8), 0xffff_fffe_0000_0001);
        // A 68000 muls.w: both words sign-extended to a long first.
        assert_eq!(
            int_mult(int_sext(0xfffe, 2, 4), int_sext(3, 2, 4), 4),
            0xffff_fffa
        );
        assert_eq!(int_mult(u64::MAX, 2, 8), u64::MAX - 1);
    }

    #[test]
    fn divide_and_remainder() {
        assert_eq!(int_div(7, 2, 1), 3);
        assert_eq!(int_rem(7, 2, 1), 1);
        assert_eq!(int_div(0xff, 2, 1), 0x7f);
        assert_eq!(int_div(0xffff_ffff, 0x10, 4), 0x0fff_ffff);
        assert_eq!(int_rem(0xffff_ffff, 0x10, 4), 0xf);
        assert_eq!(int_div(u64::MAX, 2, 8), u64::MAX >> 1);
        // -7 / 2 = -3, remainder -1 (the dividend's sign).
        assert_eq!(int_sdiv(0xf9, 2, 1), 0xfd);
        assert_eq!(int_srem(0xf9, 2, 1), 0xff);
        assert_eq!(int_sdiv(7, 0xfffe, 2), 0xfffd);
        assert_eq!(int_srem(7, 0xfffe, 2), 1);
        assert_eq!(int_sdiv(0xffff_fff9, 2, 4), 0xffff_fffd);
        assert_eq!(int_sdiv(u64::MAX - 6, 2, 8), u64::MAX - 2);
        for s in SIZES {
            // By zero is 0, never a panic.
            assert_eq!(int_div(5, 0, s), 0, "size {s}");
            assert_eq!(int_rem(5, 0, s), 0, "size {s}");
            assert_eq!(int_sdiv(5, 0, s), 0, "size {s}");
            assert_eq!(int_srem(5, 0, s), 0, "size {s}");
            // Junk above the size does not make the divisor non-zero.
            if s < 8 {
                assert_eq!(int_div(5, mask(s) + 1, s), 0, "size {s}");
            }
            // The most negative value by -1 wraps to itself, remainder 0.
            assert_eq!(int_sdiv(min(s), mask(s), s), min(s), "size {s}");
            assert_eq!(int_srem(min(s), mask(s), s), 0, "size {s}");
        }
    }

    #[test]
    fn shifts_by_0_by_the_size_and_past_it() {
        for s in SIZES {
            let n = s as u64 * 8;
            let top = min(s);
            assert_eq!(int_left(0x81, 0, s), 0x81, "size {s}");
            assert_eq!(int_right(top, 0, s), top, "size {s}");
            assert_eq!(int_sright(top, 0, s), top, "size {s}");
            assert_eq!(int_left(1, n - 1, s), top, "size {s}");
            assert_eq!(int_right(top, n - 1, s), 1, "size {s}");
            assert_eq!(int_sright(top, n - 1, s), mask(s), "size {s}");
            assert_eq!(int_sright(top >> 1, n - 1, s), 0, "size {s}");
            for sh in [n, n + 1, 64, 1000, u64::MAX] {
                assert_eq!(int_left(mask(s), sh, s), 0, "size {s} by {sh}");
                assert_eq!(int_right(mask(s), sh, s), 0, "size {s} by {sh}");
                assert_eq!(int_sright(top, sh, s), mask(s), "size {s} by {sh}");
                assert_eq!(int_sright(top - 1, sh, s), 0, "size {s} by {sh}");
            }
        }
        assert_eq!(int_left(0xc0, 1, 1), 0x80);
        assert_eq!(int_right(0x1_80, 1, 1), 0x40);
        assert_eq!(int_sright(0xc000, 4, 2), 0xfc00);
        assert_eq!(int_sright(0x4000, 4, 2), 0x0400);
    }

    #[test]
    fn bitwise() {
        for s in SIZES {
            assert_eq!(int_and(u64::MAX, u64::MAX, s), mask(s), "size {s}");
            assert_eq!(int_or(0, u64::MAX, s), mask(s), "size {s}");
            assert_eq!(int_xor(u64::MAX, 0, s), mask(s), "size {s}");
            assert_eq!(int_xor(u64::MAX, u64::MAX, s), 0, "size {s}");
        }
        assert_eq!(int_and(0xf0f0, 0x3c3c, 2), 0x3030);
        assert_eq!(int_or(0xf000, 0x000f, 2), 0xf00f);
        assert_eq!(int_xor(0xff, 0x0f, 1), 0xf0);
    }

    #[test]
    fn booleans() {
        assert_eq!(bool_negate(0), 1);
        assert_eq!(bool_negate(1), 0);
        assert_eq!(bool_and(1, 1), 1);
        assert_eq!(bool_and(1, 0), 0);
        assert_eq!(bool_or(0, 1), 1);
        assert_eq!(bool_or(0, 0), 0);
        assert_eq!(bool_xor(1, 1), 0);
        assert_eq!(bool_xor(1, 0), 1);
    }

    #[test]
    fn subpiece_and_piece() {
        let v = 0x1122_3344_5566_7788;
        assert_eq!(subpiece(v, 8, 0, 1), 0x88);
        assert_eq!(subpiece(v, 8, 0, 4), 0x5566_7788);
        assert_eq!(subpiece(v, 8, 4, 4), 0x1122_3344);
        assert_eq!(subpiece(v, 8, 7, 1), 0x11);
        assert_eq!(subpiece(v, 8, 0, 8), v);
        assert_eq!(subpiece(0x1234_5678, 4, 2, 2), 0x1234);
        assert_eq!(subpiece(0x1234_5678, 4, 3, 1), 0x12);
        assert_eq!(subpiece(0x1234_5678, 4, 1, 2), 0x3456);
        // Bytes past the input are zero, never the junk above it.
        assert_eq!(subpiece(0xab_1234, 2, 1, 2), 0x12);
        assert_eq!(subpiece(0x1234, 2, 2, 1), 0);
        assert_eq!(piece(0x12, 0x34, 1, 2), 0x1234);
        assert_eq!(piece(0x1234, 0xff_5678, 2, 4), 0x1234_5678);
        assert_eq!(piece(0x1122_3344, 0x5566_7788, 4, 8), v);
        assert_eq!(piece(0x12, 0x34, 1, 1), 0x34);
        assert_eq!(piece(0x12, v, 8, 8), v);
    }

    #[test]
    fn popcount_counts_within_the_size() {
        assert_eq!(popcount(0, 1), 0);
        assert_eq!(popcount(0x1ff, 1), 8);
        assert_eq!(popcount(0xffff_ffff, 2), 16);
        assert_eq!(popcount(0x8000_0001, 4), 2);
        assert_eq!(popcount(u64::MAX, 8), 64);
    }

    #[test]
    fn signed_reads_the_top_bit() {
        assert_eq!(signed(0x80, 1), -128);
        assert_eq!(signed(0x7f, 1), 127);
        assert_eq!(signed(0x1_ff, 1), -1);
        assert_eq!(signed(0x8000, 2), -32768);
        assert_eq!(signed(0x8000_0000, 4), -2_147_483_648);
        assert_eq!(signed(u64::MAX, 8), -1);
    }
}
