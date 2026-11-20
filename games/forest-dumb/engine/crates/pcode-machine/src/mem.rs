//! The A500's address map as the CPU sees it, after the oracle's emulator (vAmiga
//! `Memory::updateCpuMemSrcTable`, configuration `A500_OCS_1MB`): which device a 24-bit address
//! reaches.

/// Where slow ("ranger") RAM starts.
pub const SLOW_BASE: u32 = 0xC0_0000;
/// Chip RAM's window: chip RAM and its mirrors fill `$000000..$200000`.
pub const CHIP_WINDOW: u32 = 0x20_0000;
/// Slow RAM's window: past the fitted slow RAM it mirrors the custom registers, up to `$D80000`.
pub const SLOW_WINDOW_END: u32 = 0xD8_0000;
/// Kickstart ROM's window, 512 KB up to the top of the 24-bit space.
pub const ROM_BASE: u32 = 0xF8_0000;
/// Most chip RAM an OCS or ECS Agnus addresses, and most slow RAM the window holds.
pub const MAX_CHIP: u32 = CHIP_WINDOW;
pub const MAX_SLOW: u32 = SLOW_WINDOW_END - SLOW_BASE;

/// What an address reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    /// Chip RAM, at this index (a mirror folded back).
    Chip(usize),
    /// Slow RAM, at this index.
    Slow(usize),
    /// A custom register, by its even offset from `$DFF000` (`$000..=$1FE`).
    Custom(u16),
    /// The CIAs' window, the address itself: which CIA answers depends on its bits 12, 13 and 0.
    Cia(u32),
    /// Kickstart's ROM window, `$F80000..$FFFFFF`, at this offset from [`ROM_BASE`]: where the
    /// machine puts its made-up OS structures (`os`). What they do not cover is outside.
    Rom(u32),
    /// Nothing the machine models: ROM's mirror, autoconfig, the clock chip's window,
    /// unmapped space.
    Outside,
}

/// Where `a` (its low 24 bits, as the 68000 drives them) leads with `chip` bytes of chip RAM (a
/// power of two) and `slow` of slow RAM.
///
/// `$D80000..$DC0000` is custom on the A500 bank map (vAmiga puts the clock chip there only on the
/// A1000 and A2000A maps). The clock chip's `$DC0000` and the reserved `$DD0000` are left outside:
/// the oracle has none, and a game reading there reads what no model here would answer for.
pub fn region(a: u32, chip: u32, slow: u32) -> Region {
    let a = a & 0xFF_FFFF;
    match a >> 16 {
        0x00..=0x1F if chip != 0 => Region::Chip((a & (chip - 1)) as usize),
        0xA0..=0xBF => Region::Cia(a),
        0xC0..=0xD7 if a - SLOW_BASE < slow => Region::Slow((a - SLOW_BASE) as usize),
        0xC0..=0xDB | 0xDE | 0xDF => Region::Custom((a & 0x1FE) as u16),
        0xF8..=0xFF => Region::Rom(a - ROM_BASE),
        _ => Region::Outside,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K512: u32 = 0x8_0000;

    #[test]
    fn chip_ram_mirrors_to_two_megabytes() {
        assert_eq!(region(0, K512, K512), Region::Chip(0));
        assert_eq!(region(K512 - 1, K512, K512), Region::Chip(0x7_ffff));
        assert_eq!(region(K512, K512, K512), Region::Chip(0));
        assert_eq!(region(0x1F_FFFF, K512, K512), Region::Chip(0x7_ffff));
        assert_eq!(region(0x20_0000, K512, K512), Region::Outside);
        // The top byte of a 32-bit address is not on the bus.
        assert_eq!(region(0xFF00_0010, K512, K512), Region::Chip(0x10));
        // A 1 MB chip RAM mirrors at $100000.
        assert_eq!(region(0xF_0004, 0x10_0000, 0), Region::Chip(0xF_0004));
        assert_eq!(region(0x10_0004, 0x10_0000, 0), Region::Chip(4));
        assert_eq!(region(0x1F_0004, 0x10_0000, 0), Region::Chip(0xF_0004));
    }

    #[test]
    fn slow_ram_then_the_custom_mirror() {
        assert_eq!(region(SLOW_BASE, K512, K512), Region::Slow(0));
        assert_eq!(region(0xC7_FFFF, K512, K512), Region::Slow(0x7_ffff));
        assert_eq!(region(0xC8_0000, K512, K512), Region::Custom(0));
        assert_eq!(region(0xD7_F096, K512, K512), Region::Custom(0x096));
        assert_eq!(region(0xC0_0000, K512, 0), Region::Custom(0));
        assert_eq!(region(0xD8_0000, K512, K512), Region::Custom(0));
        assert_eq!(region(0xDB_F1FE, K512, K512), Region::Custom(0x1FE));
        assert_eq!(region(0xDC_0000, K512, K512), Region::Outside);
        assert_eq!(region(0xDD_FFFF, K512, K512), Region::Outside);
    }

    #[test]
    fn custom_cia_and_the_rest() {
        assert_eq!(region(0xDF_F006, K512, K512), Region::Custom(6));
        assert_eq!(region(0xDF_F007, K512, K512), Region::Custom(6));
        assert_eq!(region(0xDE_F1FE, K512, K512), Region::Custom(0x1FE));
        assert_eq!(region(0xBF_E001, K512, K512), Region::Cia(0xBF_E001));
        assert_eq!(region(0xA0_0000, K512, K512), Region::Cia(0xA0_0000));
        for a in [0x20_0000, 0x9F_FFFF, 0xE0_0000, 0xE8_0000, 0xF7_FFFF] {
            assert_eq!(region(a, K512, K512), Region::Outside, "${a:x}");
        }
        assert_eq!(region(0xF8_0000, K512, K512), Region::Rom(0));
        assert_eq!(region(0xFF_FFFF, K512, K512), Region::Rom(0x7_FFFF));
    }
}
