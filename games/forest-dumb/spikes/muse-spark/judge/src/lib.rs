//! What a translated routine is written against: the 68000's data and address registers and the
//! 24-bit memory, big-endian, as the original sees them. `h(n, off)` is where `hN+$off` of the
//! listing landed. The port module is one model's translation, put in by build.rs.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// The memory a port sees: the loaded program and the case's pokes, under the port's own writes.
#[derive(Clone)]
pub struct Mem {
    base: Arc<Vec<u8>>,
    over: HashMap<u32, u8>,
}

impl Mem {
    pub fn new(base: Arc<Vec<u8>>, pokes: &[(u32, Vec<u8>)]) -> Mem {
        let mut m = Mem { base, over: HashMap::new() };
        for (a, bytes) in pokes {
            for (i, &b) in bytes.iter().enumerate() {
                m.wb(a + i as u32, b);
            }
        }
        m
    }

    /// Every byte now different from `other` (the same memory before the routine), by address.
    pub fn changed_since(&self, other: &Mem) -> Vec<(u32, u8)> {
        let mut v: Vec<(u32, u8)> = self
            .over
            .iter()
            .filter(|&(&a, &b)| other.rb(a) != b)
            .map(|(&a, &b)| (a, b))
            .collect();
        v.sort_unstable();
        v
    }

    pub fn rb(&self, a: u32) -> u8 {
        let a = a & 0xFF_FFFF;
        match self.over.get(&a) {
            Some(&b) => b,
            None => self.base[a as usize],
        }
    }
    pub fn rw(&self, a: u32) -> u16 {
        (self.rb(a) as u16) << 8 | self.rb(a.wrapping_add(1)) as u16
    }
    pub fn rl(&self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a.wrapping_add(2)) as u32
    }
    pub fn wb(&mut self, a: u32, v: u8) {
        self.over.insert(a & 0xFF_FFFF, v);
    }
    pub fn ww(&mut self, a: u32, v: u16) {
        self.wb(a, (v >> 8) as u8);
        self.wb(a.wrapping_add(1), v as u8);
    }
    pub fn wl(&mut self, a: u32, v: u32) {
        self.ww(a, (v >> 16) as u16);
        self.ww(a.wrapping_add(2), v as u16);
    }
}

/// The registers and memory a routine is entered with and leaves. `a[7]` is the stack pointer.
pub struct Cpu {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub mem: Mem,
}

/// Each hunk's base, set once by the judge after loading the program.
pub static HUNK_BASES: OnceLock<Vec<u32>> = OnceLock::new();

/// The address of `hN+$off` in the listing.
pub fn h(hunk: usize, off: u32) -> u32 {
    HUNK_BASES.get().expect("hunk bases not set")[hunk] + off
}

pub mod port {
    #[allow(unused_imports)]
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/port.rs"));
}
