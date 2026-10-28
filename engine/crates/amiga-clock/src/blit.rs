//! The blitter's timing: how many DMA slots its words take, which slots it may have, and roughly
//! how long a blit holds the CPU. Each is the engine's calculation as it was, in one home; what a
//! blit does to memory is `amiga-chips`'.

use crate::{FRAME_LINES, LINE_SLOTS};

/// Whether DMA slot `slot` of a line is the memory refresh's (vAmiga `DAS_REFRESH`: `$01`, `$03`,
/// `$05`, `$E2`).
pub fn is_refresh(slot: u32) -> bool {
    matches!(slot, 0x01 | 0x03 | 0x05 | 0xe2)
}

/// DMA slots one word of a blit takes, by the channels BLTCON0 enables (A, B, C, D): the
/// hardware reference's table.
pub fn slots_per_word(con0: u16) -> u32 {
    let (a, b, c, d) = (con0 & 0x800 != 0, con0 & 0x400 != 0, con0 & 0x200 != 0, con0 & 0x100 != 0);
    match (a, b, c, d) {
        (_, true, true, _) => 4,
        (_, true, false, _) | (_, false, true, true) => 3,
        _ => 2,
    }
}

/// Roughly how long a blit of `words` words with BLTCON0 `con0` takes, in CPU cycles: a word per
/// channel used and two per word at the least, as the hardware reference gives them, with the
/// display's DMA taking its share (approximate: only the order of a blit against the copper's
/// writes depends on it).
pub fn rough_blit_cycles(con0: u16, words: u32) -> u32 {
    let channels = (con0 >> 8 & 15).count_ones().max(2);
    words * channels * 3
}

/// The last fixed DMA slot (refresh to `$07`, disk to `$0d`, audio to `$13`).
pub const FIXED_TO: u16 = 0x0d;

/// When each destination word of a blit with priority is written, as the blitter takes the bus
/// cycles nothing else does: not the fixed slots (refresh, disk, audio: the odd cycles to `$13`),
/// not the bitplanes' (`takes(line, h)`, the line wrapped to the frame), two a word (an A-to-D
/// copy). Starts on (`line`, `h`); returns each word's (line, h) in order. Copper and sprite slots
/// are not counted (measured on the credits).
pub fn word_times(
    words: u32,
    line: u16,
    h: u16,
    takes: impl Fn(u16, u16) -> bool,
) -> Vec<(u16, u16)> {
    let mut out = Vec::with_capacity(words as usize);
    let (mut l, mut c) = (line, h);
    let mut half = false;
    while (out.len() as u32) < words {
        let fixed = c % 2 == 1 && c <= FIXED_TO;
        if !fixed && !takes(l % FRAME_LINES, c) {
            if half {
                out.push((l, c));
            }
            half = !half;
        }
        c += 1;
        if c >= LINE_SLOTS {
            c = 0;
            l += 1;
        }
    }
    out
}
