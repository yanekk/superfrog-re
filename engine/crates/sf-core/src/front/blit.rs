//! The blitter in area mode, as the front end uses it (copies, clears, shifted copies): what one
//! blit leaves in memory. Timing is not modelled; the front end's blits all finish before the CPU
//! looks again (it waits on the blitter after each, h0+$19054).

use crate::state::State;

/// Memory a blit reads and writes: the front end's [`State`], or the story intro's RAM.
pub trait BlitMem {
    /// Whether each pointer is masked to chip RAM (`$7fffe`) after every modulo add, not only as
    /// the blit starts. The level's drawer does (DESIGN amiga-chips §2.3: a pointer that passes
    /// `$7fffe` through a modulo wraps to `$0` there), so [`crate::mem::Bus`] keeps it; the front
    /// end's blits mask once.
    const MASK_EACH_LINE: bool = false;

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
    const MASK_EACH_LINE: bool = true;

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
        let (h, w) = bltsize_words(self.size);
        let words = h * w;
        let channels = (self.con0 >> 8 & 15).count_ones().max(2);
        words * channels * 3
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

/// Whether a custom register is Paula's (DMACON, INTENA, INTREQ, ADKCON, the audio channels):
/// the writes the sound side takes from the custom-register stream.
pub fn is_paula(reg: u16) -> bool {
    matches!(reg, 0x96 | 0x9a | 0x9c | 0x9e) || (0xa0..0xe0).contains(&reg)
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

    use crate::layout::{VarId, def};
    use crate::mem::{Bus, Rom, WriteLog};

    /// The first- and last-word masks on A, each line, over the level's [`Bus`]: the bob drawer
    /// (`objects.rs` `draw_bob`) runs with `alwm` 0, and a width of 1 takes both masks.
    #[test]
    fn a_bus_blit_masks_a_s_first_and_last_word() {
        let rom = Rom::default();
        let (src, dst) = (def(VarId::PlayfieldBuffer1).addr, def(VarId::PlayfieldBuffer0).addr);
        for (w, want) in [(3u16, vec![0xff00u16, 0xffff, 0x00ff]), (1, vec![0x0000])] {
            let mut st = State::default();
            for k in 0..16 {
                st.set_w(src + 2 * k, 0xffff);
            }
            let mut bus = Bus::new(&mut st, &rom);
            Blit {
                con0: 0x09f0,
                con1: 0,
                afwm: 0xff00,
                alwm: 0x00ff,
                pt: [src, 0, 0, dst],
                modulo: [0; 4],
                size: 2 << 6 | w,
            }
            .run(&mut bus);
            let got = (0..2 * w as u32).map(|k| bus.w(dst + 2 * k)).collect::<Vec<_>>();
            assert_eq!(got, [want.clone(), want].concat(), "width {w}");
        }
    }

    /// The level drawer's own blitter as it was before amiga-chips T01 merged it into
    /// [`Blit::run`] (`level/objects.rs` `blit`), kept as the reference the merge is held to:
    /// ascending, each pointer masked to `$7fffe` at the start and after every modulo add.
    #[allow(clippy::too_many_arguments)]
    fn old_drawer(b: &mut Bus, con0: u16, con1: u16, pt: [u32; 4], modulo: [u16; 4], size: u16) {
        let (h, w) = bltsize_words(size);
        let uses = [con0 & 0x800 != 0, con0 & 0x400 != 0, con0 & 0x200 != 0, con0 & 0x100 != 0];
        let (ash, bsh) = ((con0 >> 12) as u32, (con1 >> 12) as u32);
        let mut p = pt.map(|p| p & 0x7_fffe);
        let (mut aold, mut bold) = (0u16, 0u16);
        for _ in 0..h {
            for _ in 0..w {
                let mut v = [0u16; 3];
                for k in 0..3 {
                    if uses[k] {
                        v[k] = b.w(p[k]);
                        p[k] = p[k].wrapping_add(2);
                    }
                }
                let sa = (((aold as u32) << 16 | v[0] as u32) >> ash) as u16;
                let sb = (((bold as u32) << 16 | v[1] as u32) >> bsh) as u16;
                (aold, bold) = (v[0], v[1]);
                let mut d = 0u16;
                for (bit, term) in [
                    (0x80, sa & sb & v[2]),
                    (0x40, sa & sb & !v[2]),
                    (0x20, sa & !sb & v[2]),
                    (0x10, sa & !sb & !v[2]),
                    (0x08, !sa & sb & v[2]),
                    (0x04, !sa & sb & !v[2]),
                    (0x02, !sa & !sb & v[2]),
                    (0x01, !sa & !sb & !v[2]),
                ] {
                    if con0 as u8 & bit != 0 {
                        d |= term;
                    }
                }
                if uses[3] {
                    b.set_w(p[3], d);
                    p[3] = p[3].wrapping_add(2);
                }
            }
            for k in 0..4 {
                if uses[k] {
                    p[k] = p[k].wrapping_add(modulo[k] as i16 as i32 as u32) & 0x7_fffe;
                }
            }
        }
    }

    /// Memory that differs at `$0` (where a pointer masked after its modulo wraps to) and
    /// `$80000` (where an unmasked one goes), with the bytes just below `$80000` between.
    fn wrap_rom() -> Rom {
        Rom {
            regions: vec![
                (0x0, vec![0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44]),
                (0x7_fff8, vec![0xa5, 0xa5, 0x5a, 0x5a, 0x0f, 0x0f, 0xf0, 0xf0]),
                (0x8_0000, vec![0x99, 0x99, 0x88, 0x88, 0x77, 0x77, 0x66, 0x66]),
            ],
            ..Rom::default()
        }
    }

    /// DESIGN amiga-chips §2.3: the level drawer masked each pointer with `$7fffe` after every
    /// modulo add, `Blit::run` once at the start. The probe (T01's first commit) found the two
    /// differ for a level-shaped blit whose source pointers pass `$7fffe` through a modulo (the
    /// drawer reads `$0` on the second line, the start-only mask `$80000`), so `Blit::run` masks
    /// after each modulo for a [`Bus`] ([`BlitMem::MASK_EACH_LINE`]); this holds it to the drawer.
    #[test]
    fn a_pointer_crossing_7fffe_through_a_modulo() {
        let rom = wrap_rom();
        let dst = def(VarId::PlayfieldBuffer0).addr;
        // (con0, con1, a, b, c): A→D, and A/B/C→D with C and D the destination, as the drawer's.
        let cases = [(0x09f0u16, 0u16, 0x7_fffcu32, 0u32, 0u32), (0x1fca, 0x1000, 0x7_fffc, 0x7_fffa, dst)];
        for (con0, con1, a, b, c) in cases {
            let (amod, size) = (2u16, 3 << 6 | 1);
            let drawer = {
                let mut st = State::default();
                let mut bus = Bus::new(&mut st, &rom);
                old_drawer(&mut bus, con0, con1, [a, b, c, dst], [amod, amod, 0, 0], size);
                (0..3).map(|k| bus.w(dst + 2 * k)).collect::<Vec<_>>()
            };
            let mut st = State::default();
            let mut bus = Bus::new(&mut st, &rom);
            let blit = Blit {
                con0,
                con1,
                afwm: 0xffff,
                alwm: 0xffff,
                pt: [a, b, c, dst],
                modulo: [amod as i16, amod as i16, 0, 0],
                size,
            };
            blit.run(&mut bus);
            let general = (0..3).map(|k| bus.w(dst + 2 * k)).collect::<Vec<_>>();
            assert_eq!(drawer, general, "con0 {con0:04x}");
            if con0 == 0x09f0 {
                assert_eq!(general, [bus.w(0x7_fffc), bus.w(0), bus.w(4)], "the second line reads $0");
            }
        }
    }

    /// A level copy through `Blit::run` over a [`Bus`] logs the same [`WriteLog`] entries, in the
    /// same order, as the hand-written copy it replaced (`level/scroll.rs` `blit`, rebuilt here):
    /// each word read, then written, the pointers masked once at the start.
    #[test]
    fn a_bus_copy_logs_as_the_old_scroll_copy() {
        let rom = wrap_rom();
        let (src, dst) = (def(VarId::PlayfieldBuffer1).addr + 0x40, def(VarId::PlayfieldBuffer0).addr + 0x10);
        let (size, modulo) = (3u16 << 6 | 2, 0x2cu16);
        let logged = |f: &dyn Fn(&mut Bus)| {
            let mut st = State::default();
            for k in 0..0x200u32 {
                st.write_byte(src + k, k as u8 ^ 0x5a);
            }
            let mut bus = Bus::new(&mut st, &rom);
            bus.log = Some(WriteLog::default());
            bus.at = 77;
            f(&mut bus);
            bus.log.take().unwrap()
        };
        let old = logged(&|b| {
            let (mut s, mut d) = (src & 0x7_fffe, dst & 0x7_fffe);
            for _ in 0..3 {
                for _ in 0..2 {
                    b.set_w(d, b.w(s));
                    s = s.wrapping_add(2);
                    d = d.wrapping_add(2);
                }
                s = s.wrapping_add(modulo as i16 as i32 as u32);
                d = d.wrapping_add(modulo as i16 as i32 as u32);
            }
        });
        let new = logged(&|b| {
            Blit {
                con0: 0x09f0,
                con1: 0,
                afwm: 0xffff,
                alwm: 0xffff,
                pt: [src, 0, 0, dst],
                modulo: [modulo as i16, 0, 0, modulo as i16],
                size,
            }
            .run(b);
        });
        assert_eq!(old.writes.len(), 12);
        assert_eq!(old, new);
    }
}
