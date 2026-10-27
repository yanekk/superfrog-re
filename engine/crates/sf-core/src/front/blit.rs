//! The blitter in area mode, as the front end uses it (copies, clears, shifted copies): what one
//! blit leaves in memory. Timing is not modelled; the front end's blits all finish before the CPU
//! looks again (it waits on the blitter after each, h0+$19054).

use crate::state::State;

/// Memory a blit reads and writes: the front end's [`State`], or the story intro's RAM.
pub trait BlitMem {
    fn blit_w(&self, addr: u32) -> u16;
    fn blit_set_w(&mut self, addr: u32, v: u16);
}

impl BlitMem for State {
    fn blit_w(&self, addr: u32) -> u16 {
        self.w(addr)
    }

    fn blit_set_w(&mut self, addr: u32, v: u16) {
        self.set_w(addr, v)
    }
}

impl BlitMem for crate::front::story::StoryMem {
    fn blit_w(&self, addr: u32) -> u16 {
        self.w(addr)
    }

    fn blit_set_w(&mut self, addr: u32, v: u16) {
        self.set_w(addr, v)
    }
}

/// The level's memory: each word goes through [`crate::mem::Bus::set_w`], so the [`crate::mem::WriteLog`]
/// and a blit's `span` see it.
impl BlitMem for crate::mem::Bus<'_> {
    fn blit_w(&self, addr: u32) -> u16 {
        self.w(addr)
    }

    fn blit_set_w(&mut self, addr: u32, v: u16) {
        self.set_w(addr, v)
    }
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
    pub fn run<M: BlitMem>(&self, s: &mut M) -> [u32; 4] {
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
                }
            }
        }
        pt
    }
}

/// The last fixed DMA slot (refresh to `$07`, disk to `$0d`, audio to `$13`).
pub const FIXED_TO: u16 = 0x0d;

/// When each destination word of a blit with priority is written, as the blitter takes the bus
/// cycles nothing else does: not the fixed slots (refresh, disk, audio: the odd cycles to `$13`),
/// not the bitplanes' (`fetch`), two a word (an A-to-D copy). Starts on (`line`, `h`); returns each
/// word's (line, h) in order. Copper and sprite slots are not counted (measured on the credits).
pub fn word_times(
    words: u32,
    line: u16,
    h: u16,
    fetch: &crate::render::copper::Fetch,
) -> Vec<(u16, u16)> {
    let mut out = Vec::with_capacity(words as usize);
    let (mut l, mut c) = (line, h);
    let mut half = false;
    while (out.len() as u32) < words {
        let fixed = c % 2 == 1 && c <= FIXED_TO;
        if !fixed && !fetch.takes(l % 313, c) {
            if half {
                out.push((l, c));
            }
            half = !half;
        }
        c += 1;
        if c >= crate::render::copper::LINE_CYCLES {
            c = 0;
            l += 1;
        }
    }
    out
}

/// The minterm function of A, B and C.
pub(crate) fn logic(m: u8, a: u16, b: u16, c: u16) -> u16 {
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

    /// DESIGN §2.3: the level drawer (`objects::blit`) masks each pointer with `$7fffe` after
    /// every modulo add, `Blit::run` once at the start. A level-shaped blit whose source pointers
    /// pass `$7fffe` through a modulo, run both ways over a level `Bus`: the source memory differs
    /// at `$0` (where the per-modulo mask wraps to) and `$80000` (where an unmasked pointer goes).
    #[test]
    fn a_pointer_crossing_7fffe_through_a_modulo() {
        use crate::layout::{VarId, def};
        use crate::level::objects;
        use crate::mem::{Bus, Rom};
        let rom = Rom {
            regions: vec![
                (0x0, vec![0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44]),
                (0x7_fff8, vec![0xa5, 0xa5, 0x5a, 0x5a, 0x0f, 0x0f, 0xf0, 0xf0]),
                (0x8_0000, vec![0x99, 0x99, 0x88, 0x88, 0x77, 0x77, 0x66, 0x66]),
            ],
            ..Rom::default()
        };
        let dst = def(VarId::PlayfieldBuffer0).addr;
        // (con0, con1, a, b, c): A→D, and A/B/C→D with C and D the destination, as the drawer's.
        let cases = [(0x09f0u16, 0u16, 0x7_fffcu32, 0u32, 0u32), (0x1fca, 0x1000, 0x7_fffc, 0x7_fffa, dst)];
        for (con0, con1, a, b, c) in cases {
            let (amod, size) = (2u16, 3 << 6 | 1);
            let drawer = {
                let mut st = State::default();
                let mut bus = Bus::new(&mut st, &rom);
                objects::blit(
                    &mut bus,
                    objects::Blit {
                        con0,
                        con1,
                        afwm: 0xffff,
                        alwm: 0xffff,
                        a,
                        b,
                        c,
                        d: dst,
                        amod,
                        bmod: amod,
                        cmod: 0,
                        dmod: 0,
                        size,
                    },
                );
                (0..3).map(|k| bus.w(dst + 2 * k)).collect::<Vec<_>>()
            };
            let general = {
                let mut st = State::default();
                let mut bus = Bus::new(&mut st, &rom);
                Blit {
                    con0,
                    con1,
                    afwm: 0xffff,
                    alwm: 0xffff,
                    pt: [a, b, c, dst],
                    modulo: [amod as i16, amod as i16, 0, 0],
                    size,
                }
                .run(&mut bus);
                (0..3).map(|k| bus.w(dst + 2 * k)).collect::<Vec<_>>()
            };
            // The probe's answer: they differ (the drawer reads `$0` on the second line, the
            // general path `$80000`).
            assert_eq!(drawer[0], general[0], "con0 {con0:04x}: the first line, before the wrap");
            assert_ne!(drawer, general, "con0 {con0:04x}");
        }
    }
}
