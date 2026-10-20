//! The original's random number generator (`rng_seed` h0+$1607a, `rng_next` h0+$16084,
//! `rng_range` h0+$160c0; docs/08 "RNG"). Proven against the routines themselves on Moira by
//! `sf-conform/tests/runner.rs`.
//!
//! The state is 64 bits held as two longs at `rng_state` (h0+$160d4): `lo` first (the routine's
//! `d0`), then `hi` (`d1`). Each step forces bits 0, 4 and 5 of the low byte to 0, 0 and 1 and
//! multiplies the whole 64-bit value by `2^17 + 3`; the result is the new high long.

/// `2^17 + 3`: `rng_next` adds the value, twice the value, and twice the value shifted up 16.
const MUL: u64 = (1 << 17) + 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Rng {
    /// The long at `rng_state`.
    pub lo: u32,
    /// The long at `rng_state + 4`.
    pub hi: u32,
}

impl Rng {
    /// `rng_seed`: the state becomes `(d0, d1 + d0)`, then the routine runs on into `rng_next`
    /// (it has no `rts`), so a seeded state has already advanced once. The original calls it once,
    /// with `d0` read from CIA-B's time-of-day counter (`rng_seed_from_cia`).
    pub fn seeded(d0: u32, d1: u32) -> Rng {
        let mut r = Rng {
            lo: d0,
            hi: d1.wrapping_add(d0),
        };
        r.next();
        r
    }

    /// `rng_next`: advance and return the new high long (the routine's `d0` and `d1`).
    pub fn next(&mut self) -> u32 {
        let lo = self.lo & !0xFF | (self.lo & 0x0E | 0x20);
        let x = ((self.hi as u64) << 32 | lo as u64).wrapping_mul(MUL);
        self.lo = x as u32;
        self.hi = (x >> 32) as u32;
        self.hi
    }

    /// `rng_range`: a number in `0..=n`, the remainder of the new high long's top word divided by
    /// `n + 1`. For `n = 0xFFFF` the routine returns 0 without advancing (its `n + 1` is 0).
    pub fn range(&mut self, n: u16) -> u16 {
        let div = n.wrapping_add(1);
        if div == 0 {
            return 0;
        }
        ((self.next() >> 16) as u16) % div
    }

    /// The eight bytes at `rng_state`.
    pub fn to_bytes(self) -> [u8; 8] {
        let mut b = [0; 8];
        b[..4].copy_from_slice(&self.lo.to_be_bytes());
        b[4..].copy_from_slice(&self.hi.to_be_bytes());
        b
    }

    pub fn from_bytes(b: [u8; 8]) -> Rng {
        Rng {
            lo: u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
            hi: u32::from_be_bytes([b[4], b[5], b[6], b[7]]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_adds_d0_into_the_high_long_then_advances() {
        let mut r = Rng { lo: 5, hi: 3 };
        r.next();
        assert_eq!(Rng::seeded(5, 0xFFFF_FFFE), r);
    }

    #[test]
    fn next_masks_the_low_byte_then_multiplies() {
        // lo's low byte 0xFF becomes 0x2E; 0x2E * (2^17 + 3) = 0x5C_008A.
        let mut r = Rng { lo: 0xFF, hi: 0 };
        assert_eq!(r.next(), 0);
        assert_eq!(
            r,
            Rng {
                lo: 0x5C_008A,
                hi: 0
            }
        );
        // A carry into the high long.
        let mut r = Rng {
            lo: 0x8000_0020,
            hi: 1,
        };
        let x = (1u64 << 32 | 0x8000_0020).wrapping_mul(MUL);
        assert_eq!(r.next(), (x >> 32) as u32);
        assert_eq!(r.lo, x as u32);
    }

    #[test]
    fn range_stays_in_bounds_and_ffff_does_not_advance() {
        let mut r = Rng::seeded(0x1234_5678, 0x9ABC_DEF0);
        for n in [0u16, 1, 0x4b, 1000, 0xFFFE] {
            for _ in 0..100 {
                assert!(r.range(n) <= n);
            }
        }
        let before = r;
        assert_eq!(r.range(0xFFFF), 0);
        assert_eq!(r, before);
        assert_eq!(Rng::from_bytes(before.to_bytes()), before);
    }
}
