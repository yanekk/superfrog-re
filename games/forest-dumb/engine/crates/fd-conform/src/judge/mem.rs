//! What a translated routine is written against (plans/forest-dumb-level-1 T33), carried over from
//! the spike's judge (`games/forest-dumb/spikes/muse-spark/judge/src/lib.rs`): the 68000's data and
//! address registers and the 24-bit memory, big-endian, as the original sees them.
//!
//! The custom chips ($DFF000-$DFF1FF) and the CIAs ($BFxxxx) behave as the routine oracle's
//! hardware mode (amiga-oracle shim, `FlatCpu`): custom writes are recorded in order, a byte write
//! as that byte on both halves; a BLTSIZE write runs the blit into memory at once; VHPOSR's bytes
//! read a counter that steps on every read; other hardware reads return what the case set, else 0;
//! CIA writes are dropped. Not modelled: a read of a write-only register (the shim writes the data
//! bus's last word into it), line mode, fill mode. It moves to the toolkit when a second game uses it.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;

/// The memory a port sees: the loaded program and the case's pokes, under the port's own writes,
/// and the hardware as the oracle's hardware mode answers it.
#[derive(Clone)]
pub struct Mem {
    base: Arc<Vec<u8>>,
    over: HashMap<u32, u8>,
    /// The next VHPOSR byte (the shim's `beam`).
    pub beam: Cell<u16>,
    /// Hardware bytes a read returns, by address.
    pub hw_reads: HashMap<u32, u8>,
    /// Custom-chip writes in order: (offset from $DFF000, word).
    pub hw_writes: Vec<(u16, u16)>,
    blt: [u16; 0x1c],
}

fn is_hardware(a: u32) -> bool {
    (a & 0xFF_FE00) == 0xDF_F000 || (a & 0xFF_0000) == 0xBF_0000
}

fn is_custom(a: u32) -> bool {
    (a & 0xFF_FE00) == 0xDF_F000
}

impl Mem {
    pub fn new(base: Arc<Vec<u8>>, pokes: &[(u32, Vec<u8>)]) -> Mem {
        let mut m = Mem {
            base,
            over: HashMap::new(),
            beam: Cell::new(0),
            hw_reads: HashMap::new(),
            hw_writes: Vec::new(),
            blt: [0; 0x1c],
        };
        for (a, bytes) in pokes {
            for (i, &b) in bytes.iter().enumerate() {
                m.over.insert((a + i as u32) & 0xFF_FFFF, b);
            }
        }
        m
    }

    /// Every RAM byte now different from `other` (the same memory before the routine), by address.
    pub fn changed_since(&self, other: &Mem) -> Vec<(u32, u8)> {
        let mut v: Vec<(u32, u8)> = self
            .over
            .iter()
            .filter(|&(&a, &b)| other.ram(a) != b)
            .map(|(&a, &b)| (a, b))
            .collect();
        v.sort_unstable();
        v
    }

    /// A RAM byte, never the hardware (the judge's own reads).
    pub fn ram(&self, a: u32) -> u8 {
        let a = a & 0xFF_FFFF;
        match self.over.get(&a) {
            Some(&b) => b,
            None => self.base[a as usize],
        }
    }

    pub fn rb(&self, a: u32) -> u8 {
        let a = a & 0xFF_FFFF;
        if is_hardware(a) {
            if a == 0xDF_F006 || a == 0xDF_F007 {
                let b = self.beam.get();
                self.beam.set(b.wrapping_add(1));
                return b as u8;
            }
            return self.hw_reads.get(&a).copied().unwrap_or(0);
        }
        self.ram(a)
    }
    pub fn rw(&self, a: u32) -> u16 {
        (self.rb(a) as u16) << 8 | self.rb(a.wrapping_add(1)) as u16
    }
    pub fn rl(&self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a.wrapping_add(2)) as u32
    }
    pub fn wb(&mut self, a: u32, v: u8) {
        let a = a & 0xFF_FFFF;
        if is_hardware(a) {
            if is_custom(a) {
                self.custom_write(a, (v as u16) << 8 | v as u16);
            }
            return;
        }
        self.over.insert(a, v);
    }
    pub fn ww(&mut self, a: u32, v: u16) {
        let a = a & 0xFF_FFFF;
        if is_hardware(a) {
            if is_custom(a) {
                self.custom_write(a, v);
            }
            return;
        }
        self.wb(a, (v >> 8) as u8);
        self.wb(a.wrapping_add(1), v as u8);
    }
    pub fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a.wrapping_add(2), v as u16);
    }

    fn custom_write(&mut self, a: u32, v: u16) {
        let reg = (a & 0x1FE) as u16;
        self.hw_writes.push((reg, v));
        if (0x40..0x78).contains(&reg) {
            self.blt[(reg as usize - 0x40) / 2] = v;
            if reg == 0x58 {
                self.blit(v);
            }
        }
    }

    fn mem16(&self, a: u32) -> u16 {
        let a = a & 0xFF_FFFE;
        (self.ram(a) as u16) << 8 | self.ram(a + 1) as u16
    }
    fn put16(&mut self, a: u32, v: u16) {
        let a = a & 0xFF_FFFE;
        self.over.insert(a, (v >> 8) as u8);
        self.over.insert(a + 1, v as u8);
    }

    fn reg(&self, off: u32) -> u16 {
        self.blt[(off as usize - 0x40) / 2]
    }
    fn ptr(&self, off: u32) -> u32 {
        ((self.reg(off) as u32) << 16 | self.reg(off + 2) as u32) & 0x7FFFE
    }

    /// The shim's `blit`, line for line: area mode only, pointers masked to chip RAM at the start.
    fn blit(&mut self, size: u16) {
        let (con0, con1) = (self.reg(0x40), self.reg(0x42));
        if con1 & 1 != 0 {
            return;
        }
        let desc = con1 & 2 != 0;
        let (ash, bsh) = ((con0 >> 12) as u32, (con1 >> 12) as u32);
        let (use_a, use_b, use_c, use_d) = (
            con0 & 0x800 != 0,
            con0 & 0x400 != 0,
            con0 & 0x200 != 0,
            con0 & 0x100 != 0,
        );
        let mt = con0 as u8;
        let h = if size >> 6 != 0 {
            (size >> 6) as u32
        } else {
            1024
        };
        let w = if size & 63 != 0 {
            (size & 63) as u32
        } else {
            64
        };
        let (mut apt, mut bpt, mut cpt, mut dpt) = (
            self.ptr(0x50),
            self.ptr(0x4c),
            self.ptr(0x48),
            self.ptr(0x54),
        );
        let step: i32 = if desc { -2 } else { 2 };
        let mut m = [0x64, 0x62, 0x60, 0x66].map(|o| self.reg(o) as i16 as i32);
        if desc {
            m = m.map(|x| -x);
        }
        let [amod, bmod, cmod, dmod] = m;
        let (adat, bdat, cdat) = (self.reg(0x74), self.reg(0x72), self.reg(0x70));
        let (afwm, alwm) = (self.reg(0x44), self.reg(0x46));
        let (mut aold, mut bold) = (0u16, 0u16);
        for _ in 0..h {
            for x in 0..w {
                let mut a = if use_a { self.mem16(apt) } else { adat };
                if use_a {
                    apt = apt.wrapping_add(step as u32);
                }
                if x == 0 {
                    a &= afwm;
                }
                if x == w - 1 {
                    a &= alwm;
                }
                let b = if use_b { self.mem16(bpt) } else { bdat };
                if use_b {
                    bpt = bpt.wrapping_add(step as u32);
                }
                let c = if use_c { self.mem16(cpt) } else { cdat };
                if use_c {
                    cpt = cpt.wrapping_add(step as u32);
                }
                let (sa, sb) = if !desc {
                    (
                        (((aold as u32) << 16 | a as u32) >> ash) as u16,
                        (((bold as u32) << 16 | b as u32) >> bsh) as u16,
                    )
                } else {
                    (
                        ((((a as u32) << 16 | aold as u32) << ash) >> 16) as u16,
                        ((((b as u32) << 16 | bold as u32) << bsh) >> 16) as u16,
                    )
                };
                aold = a;
                bold = b;
                let mut d = 0u16;
                for (bit, v) in [
                    (0x80, sa & sb & c),
                    (0x40, sa & sb & !c),
                    (0x20, sa & !sb & c),
                    (0x10, sa & !sb & !c),
                    (0x08, !sa & sb & c),
                    (0x04, !sa & sb & !c),
                    (0x02, !sa & !sb & c),
                    (0x01, !sa & !sb & !c),
                ] {
                    if mt & bit != 0 {
                        d |= v;
                    }
                }
                if use_d {
                    self.put16(dpt, d);
                    dpt = dpt.wrapping_add(step as u32);
                }
            }
            if use_a {
                apt = apt.wrapping_add(amod as u32);
            }
            if use_b {
                bpt = bpt.wrapping_add(bmod as u32);
            }
            if use_c {
                cpt = cpt.wrapping_add(cmod as u32);
            }
            if use_d {
                dpt = dpt.wrapping_add(dmod as u32);
            }
        }
    }
}

/// The registers and memory a routine is entered with and leaves. `a[7]` is the stack pointer.
pub struct Cpu {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub mem: Mem,
}
