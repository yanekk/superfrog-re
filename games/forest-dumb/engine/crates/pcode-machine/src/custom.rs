//! The custom registers' and the CIAs' bus rules, as the oracle's emulator applies them to a CPU
//! access (vAmiga `peekCustom16`, `pokeCustom16`, `peekCIA8`, `pokeCIA8` and their word forms).

/// VPOSR and VHPOSR: the beam, read-only.
pub const VPOSR: u16 = 0x004;
pub const VHPOSR: u16 = 0x006;
/// JOY1DAT: the port-2 joystick's counters, read-only; the machine's input sets it.
pub const JOY1DAT: u16 = 0x00C;

/// A register a CPU write changes nothing in (vAmiga `pokeCustom16`'s "READ-ONLY"): `$000..=$01E`
/// and DENISEID; the word is still on the data bus. BLTDDAT (`$000`) is also [`write_only`]: a read
/// takes the bus's word and the write it makes is dropped here.
pub fn read_only(reg: u16) -> bool {
    reg <= 0x01E || reg == 0x07C
}

/// A register a CPU read of puts the data bus's last word into (vAmiga `peekCustomFaulty16`):
/// every one but the readable ones and the colour registers, as the routine oracle and the
/// translated `clr` agree (`pcode_rt::ops::custom_write_only`).
pub fn write_only(reg: u16) -> bool {
    pcode_rt::ops::custom_write_only(0xDF_F000 | reg as u32)
}

/// One of the two CIAs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cia {
    /// CIA-A, `$BFE001`: on the low byte of the bus, selected by address bit 12 low.
    A = 0,
    /// CIA-B, `$BFD000`: on the high byte, selected by address bit 13 low.
    B = 1,
}

/// A CIA register: bits 8 to 11 of the address.
pub fn cia_reg(a: u32) -> u16 {
    ((a >> 8) & 0xF) as u16
}

/// Which CIAs a byte read of `a` reads: at most one, by the byte lane (A odd, B even) among those
/// selected. `None` reads no chip: the bus then holds the instruction word being fetched.
pub fn cia_read8(a: u32) -> Option<Cia> {
    let sel = (a >> 12) & 3;
    match (sel, a & 1) {
        (0 | 2, 1) => Some(Cia::A),
        (0 | 1, 0) => Some(Cia::B),
        _ => None,
    }
}

/// Which CIAs a write of `a` reaches, whatever its byte lane: A, B, both or neither. A word write
/// gives A its low byte and B its high one; a byte write gives each selected chip that byte.
pub fn cia_write(a: u32) -> (bool, bool) {
    (a & 0x1000 == 0, a & 0x2000 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_and_write_only_split_the_registers() {
        // BLTDDAT ($000) is both: a read takes the bus's word, a write is dropped (vAmiga).
        for reg in (2..0x200u16).step_by(2) {
            if read_only(reg) {
                assert!(!write_only(reg), "${reg:03x}");
            }
        }
        assert!(read_only(VPOSR) && read_only(VHPOSR) && read_only(0x01E));
        assert!(!read_only(0x020) && !read_only(0x180));
        assert!(write_only(0x000) && read_only(0x000), "BLTDDAT");
        assert!(write_only(0x096), "DMACON");
        assert!(write_only(0x0A8), "AUD0VOL");
        assert!(!write_only(0x180), "COLOR00 reads back on the oracle");
        assert!(!write_only(0x002), "DMACONR");
    }

    #[test]
    fn the_cias_by_address() {
        assert_eq!(cia_reg(0xBF_E001), 0);
        assert_eq!(cia_reg(0xBF_EE01), 0xE);
        assert_eq!(cia_reg(0xBF_D100), 1);
        assert_eq!(cia_read8(0xBF_E001), Some(Cia::A));
        assert_eq!(cia_read8(0xBF_D000), Some(Cia::B));
        // $BFE000 selects A, but A answers on odd addresses only.
        assert_eq!(cia_read8(0xBF_E000), None);
        assert_eq!(cia_read8(0xBF_D001), None);
        // Both selected ($BFC0xx): the byte lane decides.
        assert_eq!(cia_read8(0xBF_C001), Some(Cia::A));
        assert_eq!(cia_read8(0xBF_C000), Some(Cia::B));
        assert_eq!(cia_read8(0xBF_F001), None);
        assert_eq!(cia_write(0xBF_E001), (true, false));
        assert_eq!(cia_write(0xBF_D000), (false, true));
        assert_eq!(cia_write(0xBF_C000), (true, true));
        assert_eq!(cia_write(0xBF_F000), (false, false));
    }
}
