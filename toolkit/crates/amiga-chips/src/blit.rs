//! The blitter in area mode: what one blit leaves in memory, and when its words are written.

/// Memory a blit reads and writes. The game implements it for each of its memories.
pub trait BlitMem {
    /// Whether each pointer is masked to chip RAM (`$7fffe`) after every modulo add, not only as
    /// the blit starts. A caller whose original drawer masked that way sets it (DESIGN amiga-chips
    /// §2.3: a pointer that passes `$7fffe` through a modulo wraps to `$0` there); otherwise the
    /// pointers are masked once.
    const MASK_EACH_LINE: bool = false;

    fn blit_w(&self, addr: u32) -> u16;
    fn blit_set_w(&mut self, addr: u32, v: u16);
}

/// One area blit's registers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Blit {
    pub con0: u16,
    pub con1: u16,
    pub afwm: u16,
    pub alwm: u16,
    /// A, B, C, D pointers.
    pub pt: [u32; 4],
    /// A, B, C, D modulos (bytes, signed).
    pub modulo: [i16; 4],
    /// BLTSIZE: height in the top ten bits, width in words in the low six (0 means 1024 / 64).
    pub size: u16,
}

impl Blit {
    /// Roughly how long the blit takes, in CPU cycles: a word per channel used and two per word
    /// at the least, as the hardware reference gives them, with the display's DMA taking its share
    /// (approximate: only the order of a blit against the copper's writes depends on it).
    pub fn cycles(&self) -> u32 {
        let (h, w) = bltsize_words(self.size);
        amiga_clock::rough_blit_cycles(self.con0, h * w)
    }

    /// Run the blit. Returns the pointers as the blitter leaves them. A channel that is off reads
    /// 0 here; [`Blit::run_with`] gives it its data register, as the hardware does.
    pub fn run<M: BlitMem>(&self, s: &mut M) -> [u32; 4] {
        self.run_with([0; 3], s).pt
    }

    /// Run the blit, a channel that is off (BLTCON0 USEA, USEB, USEC clear) reading its data
    /// register from `dat` (BLTADAT, BLTBDAT, BLTCDAT) on every word, through A's masks and the
    /// shifters as a fetched word would go. Returns the pointers as the blitter leaves them and
    /// whether every D result was zero (DMACONR's BZERO), counted whether or not D is written.
    pub fn run_with<M: BlitMem>(&self, dat: [u16; 3], s: &mut M) -> BlitEnd {
        let (h, w) = bltsize_words(self.size);
        let uses = [
            self.con0 & 0x800 != 0,
            self.con0 & 0x400 != 0,
            self.con0 & 0x200 != 0,
            self.con0 & 0x100 != 0,
        ];
        let desc = self.con1 & 2 != 0;
        let (ash, bsh) = ((self.con0 >> 12) as u32, (self.con1 >> 12) as u32);
        let minterm = self.con0 as u8;
        let mut pt = self.pt.map(|p| p & 0x7_fffe);
        let step: i32 = if desc { -2 } else { 2 };
        let (mut a_old, mut b_old) = (0u16, 0u16);
        let mut zero = true;
        for _ in 0..h {
            for i in 0..w {
                let mut v = dat;
                for (k, val) in v.iter_mut().enumerate() {
                    if uses[k] {
                        *val = s.blit_w(pt[k]);
                        pt[k] = (pt[k] as i32 + step) as u32;
                    }
                }
                let mut a = v[0];
                if i == 0 {
                    a &= self.afwm;
                }
                if i == w - 1 {
                    a &= self.alwm;
                }
                let (a_sh, b_sh) = if desc {
                    (
                        ((a as u32) << 16 | a_old as u32) << ash >> 16,
                        ((v[1] as u32) << 16 | b_old as u32) << bsh >> 16,
                    )
                } else {
                    (
                        ((a_old as u32) << 16 | a as u32) >> ash,
                        ((b_old as u32) << 16 | v[1] as u32) >> bsh,
                    )
                };
                a_old = a;
                b_old = v[1];
                let d = logic(minterm, a_sh as u16, b_sh as u16, v[2]);
                zero &= d == 0;
                if uses[3] {
                    s.blit_set_w(pt[3], d);
                    pt[3] = (pt[3] as i32 + step) as u32;
                }
            }
            for k in 0..4 {
                if uses[k] {
                    let m = self.modulo[k] as i32;
                    pt[k] = (pt[k] as i32 + if desc { -m } else { m }) as u32;
                    if M::MASK_EACH_LINE {
                        pt[k] &= 0x7_fffe;
                    }
                }
            }
        }
        BlitEnd { pt, zero }
    }
}

/// What a blit leaves: the pointers, and DMACONR's BZERO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlitEnd {
    /// A, B, C, D pointers as the blitter leaves them.
    pub pt: [u32; 4],
    /// Every D result was zero.
    pub zero: bool,
}

/// BLTSIZE as (rows, words): height in bits 15–6, width in bits 5–0, 0 meaning 1024 / 64.
pub fn bltsize_words(size: u16) -> (u32, u32) {
    let h = match size >> 6 {
        0 => 1024,
        n => n as u32,
    };
    let w = match size & 63 {
        0 => 64,
        n => n as u32,
    };
    (h, w)
}

/// The blitter's timing lives with the rest of it in `amiga-clock` (one home, DESIGN one-clock
/// §2.4); its old paths stay.
pub use amiga_clock::{FIXED_TO, word_times};

/// The minterm function of A, B and C.
pub fn logic(m: u8, a: u16, b: u16, c: u16) -> u16 {
    let mut d = 0;
    for bit in 0..8 {
        if m >> bit & 1 == 1 {
            let x = if bit & 4 != 0 { a } else { !a };
            let y = if bit & 2 != 0 { b } else { !b };
            let z = if bit & 1 != 0 { c } else { !c };
            d |= x & y & z;
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minterms() {
        assert_eq!(logic(0xf0, 0x1234, 0, 0), 0x1234);
        assert_eq!(logic(0xcc, 0, 0xabcd, 0), 0xabcd);
        assert_eq!(logic(0xaa, 0, 0, 0x5555), 0x5555);
        assert_eq!(logic(0x00, 0xffff, 0xffff, 0xffff), 0);
        assert_eq!(logic(0xca, 0xff00, 0x1234, 0x5678), 0x1278);
    }

    #[test]
    fn bltsize_decodes_rows_and_words() {
        assert_eq!(bltsize_words(0), (1024, 64));
        assert_eq!(bltsize_words(0x0040), (1, 64));
        assert_eq!(bltsize_words(0x0001), (1024, 1));
        assert_eq!(bltsize_words(0x0382), (14, 2));
        assert_eq!(bltsize_words(0xffff), (1023, 63));
    }

    /// `word_times` as it was before it moved to `amiga-clock` (one-clock T02), with `&Fetch`:
    /// the reference the moved one, given `|l, h| fetch.takes(l, h)`, is held to.
    fn old_word_times(
        words: u32,
        line: u16,
        h: u16,
        fetch: &crate::copper::Fetch,
    ) -> Vec<(u16, u16)> {
        let mut out = Vec::with_capacity(words as usize);
        let (mut l, mut c) = (line, h);
        let mut half = false;
        while (out.len() as u32) < words {
            let fixed = c % 2 == 1 && c <= 0x0d;
            if !fixed && !fetch.takes(l % 313, c) {
                if half {
                    out.push((l, c));
                }
                half = !half;
            }
            c += 1;
            if c >= 227 {
                c = 0;
                l += 1;
            }
        }
        out
    }

    #[test]
    fn the_moved_word_times_are_the_old_ones() {
        use crate::copper::Fetch;
        let f = |bplcon0, diwstrt, diwstop, ddfstrt, ddfstop| Fetch {
            bplcon0,
            diwstrt,
            diwstop,
            ddfstrt,
            ddfstop,
        };
        // Synthetic set-ups: no display, lores of one to six planes with assorted windows and
        // fetches, hires, and a window past line $ff. (A game's own set-ups are tested in the
        // game's crate.)
        let fetches = [
            Fetch::default(),
            f(0x1200, 0x3081, 0x2ac1, 0x40, 0xc8),
            f(0x2200, 0x2c91, 0x2cb1, 0x48, 0xc0),
            f(0x4200, 0x2481, 0x34c1, 0x30, 0xd0),
            f(0x5200, 0x2081, 0x30c1, 0x30, 0xd0),
            f(0x6200, 0x1a81, 0x2ec1, 0x38, 0xc8),
            f(0xa200, 0x3081, 0x28c1, 0x40, 0xd0),
            f(0x3200, 0x0881, 0xfcc1, 0x20, 0xd8),
        ];
        for fetch in &fetches {
            for line in (0..313).step_by(5).chain([255, 256, 311, 312, 313, 400]) {
                for h in (0..227).step_by(3).chain([0x0d, 0x0e, 0xe2]) {
                    for words in [0, 1, 2, 3, 40, 321, 2000] {
                        assert_eq!(
                            word_times(words, line, h, 313, |l, h| fetch.takes(l, h)),
                            old_word_times(words, line, h, fetch),
                            "{fetch:?} {line} {h} {words}"
                        );
                    }
                }
            }
        }
    }

    /// Chip RAM as a flat array for the tests.
    struct Flat(Vec<u8>);
    impl BlitMem for Flat {
        fn blit_w(&self, a: u32) -> u16 {
            u16::from_be_bytes([self.0[a as usize], self.0[a as usize + 1]])
        }
        fn blit_set_w(&mut self, a: u32, v: u16) {
            self.0[a as usize..a as usize + 2].copy_from_slice(&v.to_be_bytes());
        }
    }

    #[test]
    fn a_channel_that_is_off_reads_its_data_register() {
        // D = A AND B with only B fetched: A comes from BLTADAT through the first-word mask.
        let b = Blit {
            con0: 0x05c0, // USEB, USED, D = AB
            afwm: 0x0ff0,
            alwm: 0xffff,
            pt: [0, 0x10, 0, 0x20],
            size: 0x0042, // 1 row, 2 words
            ..Blit::default()
        };
        let mut m = Flat(vec![0; 0x40]);
        m.0[0x10..0x14].copy_from_slice(&[0xff, 0xff, 0x12, 0x34]);
        let end = b.run_with([0xf00f, 0, 0], &mut m);
        assert_eq!(&m.0[0x20..0x24], &[0x00, 0x00, 0x10, 0x04]);
        assert_eq!(end.pt, [0, 0x14, 0, 0x24]);
        assert!(!end.zero);
        // `run` keeps its old meaning: an unused channel reads 0.
        let mut m0 = Flat(m.0.clone());
        b.run(&mut m0);
        assert_eq!(&m0.0[0x20..0x24], &[0, 0, 0, 0]);
    }

    #[test]
    fn bzero_counts_results_whether_or_not_d_is_written() {
        let mut m = Flat(vec![0; 0x40]);
        m.0[0x10] = 0x80;
        // A to nowhere (USED clear): a non-zero A clears BZERO, memory unchanged.
        let a_only = Blit {
            con0: 0x09f0 & !0x0100,
            afwm: 0xffff,
            alwm: 0xffff,
            pt: [0x10, 0, 0, 0x20],
            size: 0x0041,
            ..Blit::default()
        };
        assert!(!a_only.run_with([0; 3], &mut m).zero);
        assert_eq!(m.0[0x20], 0);
        let zeros = Blit {
            pt: [0x30, 0, 0, 0x20],
            ..a_only
        };
        assert!(zeros.run_with([0; 3], &mut m).zero);
    }

    #[test]
    fn blit_cycles_are_unchanged() {
        // `Blit::cycles` as it was, before its formula moved to `amiga-clock`.
        let old = |b: &Blit| {
            let (h, w) = bltsize_words(b.size);
            let words = h * w;
            let channels = (b.con0 >> 8 & 15).count_ones().max(2);
            words * channels * 3
        };
        for ch in 0..16u16 {
            for con0 in [ch << 8, ch << 8 | 0xca, 0xf000 | ch << 8 | 0xf0] {
                for size in [0, 1, 0x41, 0x0382, 0x1014, 0xffff, 0x8000] {
                    let b = Blit { con0, size, ..Blit::default() };
                    assert_eq!(b.cycles(), old(&b), "{con0:04x} {size:04x}");
                }
            }
        }
    }
}
