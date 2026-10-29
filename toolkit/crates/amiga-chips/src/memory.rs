//! An A500's RAM: chip RAM at 0, slow RAM at [`SLOW_BASE`].

/// A machine's memory read one byte per CPU address.
pub trait Ram {
    fn byte(&self, addr: u32) -> u8;
}

/// Chip RAM at 0 and slow RAM at `$C00000`, as an A500 with a trapdoor expansion has them. Two
/// readers: [`Ram::byte`] reads 0 outside both RAMs, [`AmigaMemory::byte_at`] gives `None` there.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AmigaMemory {
    pub chip: Vec<u8>,
    pub slow: Vec<u8>,
}

/// Where slow ("ranger") RAM sits on the A500.
pub const SLOW_BASE: u32 = 0xC0_0000;

impl Ram for AmigaMemory {
    fn byte(&self, addr: u32) -> u8 {
        self.byte_at(addr).unwrap_or(0)
    }
}

impl AmigaMemory {
    /// The byte at a CPU address, or `None` outside chip and slow RAM.
    pub fn byte_at(&self, addr: u32) -> Option<u8> {
        if (addr as usize) < self.chip.len() {
            Some(self.chip[addr as usize])
        } else {
            addr.checked_sub(SLOW_BASE)
                .and_then(|a| self.slow.get(a as usize).copied())
        }
    }

    /// The big-endian word at `addr`, or `None` if either byte is outside both RAMs.
    pub fn word(&self, addr: u32) -> Option<u16> {
        Some(u16::from_be_bytes([
            self.byte_at(addr)?,
            self.byte_at(addr + 1)?,
        ]))
    }

    /// The big-endian long at `addr`, or `None` if any byte is outside both RAMs.
    pub fn long(&self, addr: u32) -> Option<u32> {
        Some((self.word(addr)? as u32) << 16 | self.word(addr + 2)? as u32)
    }
}

/// What the mixer's DMA reads: chip RAM only, as Agnus sees it (an address past the RAM reads 0).
impl amiga_paula::ChipMemory for AmigaMemory {
    fn chip_word(&self, addr: u32) -> u16 {
        let a = addr as usize;
        match self.chip.get(a..a + 2) {
            Some(w) => u16::from_be_bytes([w[0], w[1]]),
            None => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amiga_memory_reads_0_or_none_off_the_map() {
        use amiga_paula::ChipMemory;
        let m = AmigaMemory {
            chip: vec![1, 2, 3, 4],
            slow: vec![5, 6],
        };
        assert_eq!((m.byte(1), m.byte_at(1)), (2, Some(2)));
        assert_eq!(
            (m.byte(SLOW_BASE + 1), m.byte_at(SLOW_BASE + 1)),
            (6, Some(6))
        );
        assert_eq!(m.word(2), Some(0x0304));
        assert_eq!(m.long(0), Some(0x0102_0304));
        for past in [4, 0x10_0000, SLOW_BASE + 2, 0xff_ffff] {
            assert_eq!((m.byte(past), m.byte_at(past)), (0, None), "{past:#x}");
        }
        assert_eq!(m.word(3), None);
        assert_eq!(m.long(SLOW_BASE), None);
        assert_eq!(m.chip_word(2), 0x0304);
        assert_eq!(m.chip_word(3), 0, "past chip RAM");
        assert_eq!(m.chip_word(SLOW_BASE), 0, "slow RAM is not chip RAM");
    }
}
