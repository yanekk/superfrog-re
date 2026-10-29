//! The story's memory: the intro is a program of its own with its own memory map (docs/13), so
//! the engine keeps the whole of the machine's RAM for it as the original's bytes, chip RAM at 0
//! and slow RAM at `$C00000`, and its routines read and write it at the original's addresses.

use crate::layout::{AmigaMemory, Ram, SLOW_BASE};

/// 512 KB of each.
pub const RAM_BYTES: usize = 512 * 1024;

/// The machine's RAM as the story's routines see it: every address taken as 24 bits, as the
/// 68000 puts it on the bus. A newtype over [`AmigaMemory`], so a save state written when it was
/// a `{chip, slow}` struct of its own still loads (serde encodes a newtype as what it wraps).
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StoryMem(pub AmigaMemory);

impl std::fmt::Debug for StoryMem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoryMem").finish_non_exhaustive()
    }
}

impl Default for StoryMem {
    /// The oracle's RAM is zeroed at power-on (FINDINGS 2026-10-11).
    fn default() -> StoryMem {
        StoryMem(AmigaMemory {
            chip: vec![0; RAM_BYTES],
            slow: vec![0; RAM_BYTES],
        })
    }
}

impl Ram for StoryMem {
    fn byte(&self, addr: u32) -> u8 {
        self.b(addr)
    }
}

impl StoryMem {
    /// The byte a 24-bit CPU address reads; outside both RAMs, 0.
    fn at(&self, addr: u32) -> Option<(bool, usize)> {
        let addr = addr & 0xff_ffff;
        if (addr as usize) < self.0.chip.len() {
            Some((true, addr as usize))
        } else {
            let o = addr.checked_sub(SLOW_BASE)? as usize;
            (o < self.0.slow.len()).then_some((false, o))
        }
    }

    pub fn b(&self, addr: u32) -> u8 {
        match self.at(addr) {
            Some((true, o)) => self.0.chip[o],
            Some((false, o)) => self.0.slow[o],
            None => 0,
        }
    }

    pub fn w(&self, addr: u32) -> u16 {
        u16::from_be_bytes([self.b(addr), self.b(addr.wrapping_add(1))])
    }

    pub fn l(&self, addr: u32) -> u32 {
        (self.w(addr) as u32) << 16 | self.w(addr.wrapping_add(2)) as u32
    }

    pub fn set_b(&mut self, addr: u32, v: u8) {
        match self.at(addr) {
            Some((true, o)) => self.0.chip[o] = v,
            Some((false, o)) => self.0.slow[o] = v,
            None => {}
        }
    }

    pub fn set_w(&mut self, addr: u32, v: u16) {
        let [a, b] = v.to_be_bytes();
        self.set_b(addr, a);
        self.set_b(addr.wrapping_add(1), b);
    }

    pub fn set_l(&mut self, addr: u32, v: u32) {
        self.set_w(addr, (v >> 16) as u16);
        self.set_w(addr.wrapping_add(2), v as u16);
    }

    /// `bytes` written from `addr` on.
    pub fn write(&mut self, addr: u32, bytes: &[u8]) {
        if let Some((chip, o)) = self.at(addr) {
            let ram = if chip {
                &mut self.0.chip
            } else {
                &mut self.0.slow
            };
            if let Some(dst) = ram.get_mut(o..o + bytes.len()) {
                dst.copy_from_slice(bytes);
                return;
            }
        }
        for (k, &v) in bytes.iter().enumerate() {
            self.set_b(addr + k as u32, v);
        }
    }

    /// `len` bytes from `addr` on.
    pub fn read(&self, addr: u32, len: u32) -> Vec<u8> {
        self.slice(addr, len)
            .map(<[u8]>::to_vec)
            .unwrap_or_else(|| (0..len).map(|k| self.b(addr + k)).collect())
    }

    /// The bytes at `addr..addr + len` when they lie in one RAM.
    pub fn slice(&self, addr: u32, len: u32) -> Option<&[u8]> {
        let (chip, o) = self.at(addr)?;
        let ram = if chip { &self.0.chip } else { &self.0.slow };
        ram.get(o..o + len as usize)
    }

    /// `len` bytes from `addr` set to zero.
    pub fn clear(&mut self, addr: u32, len: u32) {
        self.write(addr, &vec![0; len as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_and_slow_ram_are_where_the_cpu_sees_them() {
        let mut m = StoryMem::default();
        m.set_l(0x400, 0x1234_5678);
        m.set_w(SLOW_BASE + 0xfa0, 0xabcd);
        assert_eq!(m.l(0x400), 0x1234_5678);
        assert_eq!(m.0.chip[0x401], 0x34);
        assert_eq!(m.w(0xc0_0fa0), 0xabcd);
        assert_eq!(m.0.slow[0xfa0], 0xab);
        // Past both RAMs reads zero and ignores writes.
        m.set_b(0x90_0000, 1);
        assert_eq!(m.b(0x90_0000), 0);
        assert_eq!(m.read(0x7_fffe, 4), [0, 0, 0, 0]);
        // A 24-bit address: the top byte is not on the bus.
        m.set_w(0x400, 0xbeef);
        assert_eq!(m.b(0x0100_0400), 0xbe);
        assert_eq!(m.byte(0xff00_0401), 0xef);
    }

    /// Save states hold `StoryMem` (rust-engine T26): the newtype must encode as the
    /// `{chip, slow}` struct it was, so a state written before amiga-chips T01 still loads.
    #[test]
    fn story_mem_encodes_as_the_struct_it_was() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct OldStoryMem {
            chip: Vec<u8>,
            slow: Vec<u8>,
        }
        let old = OldStoryMem {
            chip: vec![1, 2, 3, 0xff],
            slow: vec![9, 8],
        };
        // The save state's options (`savestate.rs` `opts`).
        use bincode::Options;
        let opts = || {
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .with_little_endian()
                .reject_trailing_bytes()
        };
        let bytes = opts().serialize(&old).unwrap();
        let new: StoryMem = opts().deserialize(&bytes).unwrap();
        assert_eq!(
            (new.0.chip.as_slice(), new.0.slow.as_slice()),
            (&old.chip[..], &old.slow[..])
        );
        assert_eq!(opts().serialize(&new).unwrap(), bytes);
    }
}
