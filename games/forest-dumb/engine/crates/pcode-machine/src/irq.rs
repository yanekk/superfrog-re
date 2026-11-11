//! Interrupts as Paula and the 68000 take them (translator DESIGN §2.8): INTENA and INTREQ with
//! their set/clear writes, Paula's map from request bits to the 68000's levels 1 to 6, and the
//! exception the machine's `poll` takes at a translated block's start.

use pcode_rt::{Bus, Image, Regs, Stop};

/// INTENA, INTREQ and their read-back registers, as `$DFF000` offsets.
pub const INTENAR: u16 = 0x01C;
pub const INTREQR: u16 = 0x01E;
pub const INTENA: u16 = 0x09A;
pub const INTREQ: u16 = 0x09C;

/// INTENA's master enable; in a write to either register, bit 15 sets the bits given, else clears.
pub const INTEN: u16 = 0x4000;
const SETCLR: u16 = 0x8000;

/// The 68000 cycles an interrupt takes from acknowledge to the handler's first instruction
/// (MC68000UM table 8-14, "interrupt": 44). Counted when `poll` takes one, so the beam moves on.
pub const IRQ_CYCLES: u32 = 44;

/// The PC pushed in the exception frame: odd, so no instruction's address, and a handler that
/// changes it is caught. `poll` is not told which block it starts, so it cannot push the real
/// return address; and with block-boundary delivery the original's own stacked PC names another
/// instruction anyway (up to one block away). Only the stack's dead bytes below SP differ.
pub const IRQ_RETURN: u32 = 0x00FF_FFFF;

/// A translated function: the type every generated module's `Routine` is.
pub type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// A generated module's `entry`: the function entered at `hN+$off`, if one was translated there.
pub type Entry = fn(&str) -> Option<Routine>;

/// Paula's level for INTREQ bit `bit` (0 to 13): TBE DSKBLK SOFT 1, PORTS 2, COPER VERTB BLIT 3,
/// AUD0 to AUD3 4, RBF DSKSYN 5, EXTER 6.
pub fn level_of(bit: u16) -> u8 {
    match bit {
        0..=2 => 1,
        3 => 2,
        4..=6 => 3,
        7..=10 => 4,
        11 | 12 => 5,
        13 => 6,
        _ => panic!("INTREQ bit {bit} raises no interrupt"),
    }
}

/// A write of `v` to INTENA or INTREQ applied to `old`: bit 15 says set or clear. Bit 15 itself
/// is never held.
pub fn set_clear(old: u16, v: u16) -> u16 {
    if v & SETCLR != 0 {
        (old | v) & !SETCLR
    } else {
        old & !v
    }
}

/// The level Paula asks the 68000 for: the highest among bits both enabled and requested, with
/// the master enable on; 0 for none. INTREQ's bit 14, set by a write with bit 15, asks for level 6
/// like EXTER: Paula ands the two registers before mapping, INTEN included (vAmiga
/// `Paula::interruptLevel`, the oracle).
pub fn ipl(intena: u16, intreq: u16) -> u8 {
    if intena & INTEN == 0 {
        return 0;
    }
    let on = intena & intreq & 0x7FFF;
    if on & INTEN != 0 {
        return 6;
    }
    (0..14u16)
        .rev()
        .find(|&b| on & (1 << b) != 0)
        .map_or(0, level_of)
}

/// The level to take with the status register `sr`: Paula's, if above SR's mask. Level 7 (NMI)
/// is never asked for by Paula.
pub fn above_mask(level: u8, sr: u16) -> Option<u8> {
    let mask = ((sr >> 8) & 7) as u8;
    (level > mask).then_some(level)
}

/// The SR a 68000 runs a level `level` handler with: T cleared, S set, the mask raised to
/// `level`, the condition codes kept.
pub fn handler_sr(old: u16, level: u8) -> u16 {
    (old & 0x00FF) | 0x2000 | (level as u16) << 8
}

/// The exception vector's address for `level` (VBR is 0 on a 68000): `$64` for level 1 to `$78`
/// for level 6.
pub fn vector(level: u8) -> u32 {
    0x60 + 4 * level as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_set_or_clear_and_never_hold_bit_15() {
        assert_eq!(set_clear(0, 0x8020), 0x0020);
        assert_eq!(set_clear(0x0021, 0x0020), 0x0001);
        assert_eq!(set_clear(0x0001, 0xC020), 0x4021);
        assert_eq!(set_clear(0x4021, 0x7FFF), 0);
    }

    #[test]
    fn paulas_levels_by_bit() {
        let want = [1, 1, 1, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 6];
        for (bit, &l) in want.iter().enumerate() {
            assert_eq!(level_of(bit as u16), l, "bit {bit}");
        }
    }

    #[test]
    fn the_highest_enabled_request_is_asked_for_only_with_the_master_on() {
        assert_eq!(ipl(0x0028, 0x0028), 0, "INTEN off");
        assert_eq!(ipl(0x4028, 0x0028), 3);
        assert_eq!(ipl(0x4008, 0x0028), 2, "VERTB not enabled");
        assert_eq!(ipl(0x4028, 0x0000), 0);
        assert_eq!(ipl(0x6000, 0x2000), 6);
        assert_eq!(
            ipl(0x7FFF, 0x4000),
            6,
            "INTREQ bit 14 with INTEN on is level 6, as vAmiga Paula::interruptLevel"
        );
        assert_eq!(ipl(0x3FFF, 0x4000), 0, "and nothing with INTEN off");
    }

    #[test]
    fn the_mask_lets_through_only_higher_levels() {
        assert_eq!(above_mask(3, 0x2000), Some(3));
        assert_eq!(above_mask(3, 0x2300), None);
        assert_eq!(above_mask(3, 0x2200), Some(3));
        assert_eq!(above_mask(2, 0x2300), None);
        assert_eq!(above_mask(0, 0x0000), None);
    }

    #[test]
    fn the_handlers_sr_and_vector() {
        assert_eq!(handler_sr(0x8015, 3), 0x2315, "T cleared, S set, CCR kept");
        assert_eq!(handler_sr(0x2700, 2), 0x2200, "the mask is set, not or-ed");
        assert_eq!(vector(1), 0x64);
        assert_eq!(vector(3), 0x6C);
        assert_eq!(vector(6), 0x78);
    }
}
