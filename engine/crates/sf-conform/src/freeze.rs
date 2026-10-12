//! What a case leaves out of the original because the engine does not model it yet (the done-when
//! of T14: every exclusion listed in the case file with the task that will model it).
//!
//! `objects`: the original's objects never come into play. Its spawners return at once (the
//! markers' stubs in `obj_flag_jumptab`, the cell and pixel spawners) and so do the frog's
//! reactions that make items (bump blocks, tile pickups), patched into its own code from power-on,
//! as soon as the game is in memory. The frog then plays the level alone, as the engine does.
//! `panel`: the status panel's bitmap is not compared, as a variable or in the picture.

use sf_core::DisplayData;
use sf_core::layout::h0;
use sf_oracle::Amiga;

/// `obj_flag_jumptab` (h0+$178ec): 7 worlds × 64 `bra.w` stubs.
const JUMPTAB: u32 = 0x178ec;
const STUBS: u32 = 7 * 64;
/// The routines patched to return: the five spawners (h0+$1b3d8, $1b4e6, $1b5d0, $1b706,
/// $1b822), the bump blocks (h0+$f6be) and the tile pickups (h0+$ed8a).
const RETURN_AT_ONCE: [u32; 7] = [0x1b3d8, 0x1b4e6, 0x1b5d0, 0x1b706, 0x1b822, 0xf6be, 0xed8a];

fn word(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

/// Patch the original's object spawning out, once the game's hunk 0 is in memory (each call
/// checks its code is the executable's before writing; already patched code is left alone).
pub fn freeze_objects(a: &mut Amiga, display: &DisplayData) {
    let rom = &display.rom().h0;
    if rom.len() < (JUMPTAB + 4 * STUBS) as usize {
        return;
    }
    let mem = a.peek(h0(0), rom.len().min(0x30000));
    for &off in &RETURN_AT_ONCE {
        let o = off as usize;
        if word(&mem, o) == word(rom, o) && word(rom, o) != 0x4e75 {
            a.poke(h0(off), &[0x4e, 0x75]);
        }
    }
    for k in 0..STUBS {
        let o = (JUMPTAB + 4 * k) as usize;
        if mem[o..o + 4] == rom[o..o + 4] {
            a.poke(h0(JUMPTAB + 4 * k), &[0x4e, 0x75, 0x4e, 0x71]);
        }
    }
}
