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

    /// Run the blit. Returns the pointers as the blitter leaves them.
    pub fn run<M: BlitMem>(&self, s: &mut M) -> [u32; 4] {
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
        for _ in 0..h {
            for i in 0..w {
                let mut v = [0u16; 3];
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
        pt
    }
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
        // The set-ups the engine draws with: no display, the front end's and story's lores
        // screens of four to six planes, the level's scrolled fetch, hires, and a window past
        // line $ff.
        let fetches = [
            Fetch::default(),
            f(0x4200, 0x2c81, 0x2cc1, 0x38, 0xd0),
            f(0x5200, 0x2c81, 0x2cc1, 0x38, 0xd0),
            f(0x6200, 0x2c81, 0x2cc1, 0x38, 0xd0),
            f(0x5200, 0x2781, 0x2cc1, 0x30, 0xd8),
            f(0xc200, 0x2c81, 0x2cc1, 0x3c, 0xd4),
            f(0x3200, 0x1081, 0xf4c1, 0x28, 0xd8),
        ];
        for fetch in &fetches {
            for line in (0..313).step_by(5).chain([255, 256, 311, 312, 313, 400]) {
                for h in (0..227).step_by(3).chain([0x0d, 0x0e, 0xe2]) {
                    for words in [0, 1, 2, 3, 40, 321, 2000] {
                        assert_eq!(
                            word_times(words, line, h, |l, h| fetch.takes(l, h)),
                            old_word_times(words, line, h, fetch),
                            "{fetch:?} {line} {h} {words}"
                        );
                    }
                }
            }
        }
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
