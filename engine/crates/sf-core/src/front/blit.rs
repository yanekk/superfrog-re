//! The blitter in area mode, as the front end uses it (copies, clears, shifted copies): what one
//! blit leaves in memory. Timing is not modelled; the front end's blits all finish before the CPU
//! looks again (it waits on the blitter after each, h0+$19054).

use crate::state::State;

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
        let words = (match self.size >> 6 {
            0 => 1024,
            n => n as u32,
        }) * (match self.size & 63 {
            0 => 64,
            n => n as u32,
        });
        let channels = (self.con0 >> 8 & 15).count_ones().max(2);
        words * channels * 3
    }

    /// Run the blit. Returns the pointers as the blitter leaves them.
    pub fn run(&self, s: &mut State) -> [u32; 4] {
        let h = match self.size >> 6 {
            0 => 1024,
            n => n as u32,
        };
        let w = match self.size & 63 {
            0 => 64,
            n => n as u32,
        };
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
                        *val = s.w(pt[k]);
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
                    s.set_w(pt[3], d);
                    pt[3] = (pt[3] as i32 + step) as u32;
                }
            }
            for k in 0..4 {
                if uses[k] {
                    let m = self.modulo[k] as i32;
                    pt[k] = (pt[k] as i32 + if desc { -m } else { m }) as u32;
                }
            }
        }
        pt
    }
}

/// The minterm function of A, B and C.
fn logic(m: u8, a: u16, b: u16, c: u16) -> u16 {
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
}
