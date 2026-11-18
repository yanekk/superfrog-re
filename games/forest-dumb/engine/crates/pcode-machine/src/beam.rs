//! The beam from counted cycles: where `t` CPU cycles into a frame put it (`amiga-clock`'s `Clock`,
//! in closed form); this module only wraps at the frame and encodes VPOSR and VHPOSR.

use amiga_clock::{FRAME_LINES, LINE_CYCLES};
#[cfg(test)]
use amiga_clock::{Clock, DisplayTiming, Timing};

/// A display that takes no cycles from the CPU: the beam runs on regardless of what the bus gives
/// the CPU (contention is `dma`'s, as waits added to the count), so every line is `LINE_CYCLES`.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct NoFetch;

#[cfg(test)]
impl Timing for NoFetch {
    const TIMING: DisplayTiming = DisplayTiming {
        frame_lines: FRAME_LINES,
        fetch_lines: 0..0,
        fetch_slots: 0..0,
        blitter_free: 0,
        stolen_cycles: 0,
    };
}

/// CPU cycles in a frame of `frame_lines` lines.
pub fn frame_cycles(frame_lines: u16) -> u64 {
    frame_lines as u64 * LINE_CYCLES as u64
}

/// The line and horizontal position (DMA slots, 227 a line) `t` CPU cycles after line 0's start,
/// frames of `frame_lines` lines repeating.
pub fn position(t: u64, frame_lines: u16) -> (u16, u16) {
    // [`clock_position`] in closed form: with nothing taken from the CPU every line is
    // `LINE_CYCLES` long. The bus (`dma`) asks this for every slot it runs, and the clock walks the
    // frame a line at a time (T49).
    let into = t % frame_cycles(frame_lines);
    let line = (into / LINE_CYCLES as u64) as u16;
    let h = ((into % LINE_CYCLES as u64) / 2).min(amiga_clock::LINE_SLOTS as u64 - 1) as u16;
    (line, h)
}

/// [`position`] as `amiga-clock`'s [`Clock`] counts it.
#[cfg(test)]
fn clock_position(t: u64, frame_lines: u16) -> (u16, u16) {
    let mut c = Clock::<NoFetch>::START;
    c.cpu((t % frame_cycles(frame_lines)) as u32);
    (c.line, c.pos().slot())
}

/// CPU cycles from line 0's start to `(line, h)`.
pub fn offset(line: u16, h: u16) -> u64 {
    line as u64 * LINE_CYCLES as u64 + 2 * h as u64
}

/// VPOSR at `line`: LOF (bit 15) for a long frame, the OCS Agnus's chip id 0, V8 in bit 0.
pub fn vposr(line: u16, frame_lines: u16) -> u16 {
    let lof = if frame_lines >= FRAME_LINES {
        0x8000
    } else {
        0
    };
    lof | (line >> 8) & 1
}

/// VHPOSR at `(line, h)`: V7-V0 in the high byte, H8-H1 (the DMA slot) in the low.
pub fn vhposr(line: u16, h: u16) -> u16 {
    (line & 0xFF) << 8 | h & 0xFF
}

/// The `(line, h)` a VPOSR and VHPOSR pair read.
pub fn decode(vposr: u16, vhposr: u16) -> (u16, u16) {
    ((vposr & 1) << 8 | vhposr >> 8, vhposr & 0xFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_closed_form_is_the_clocks() {
        for lines in [312u16, 313] {
            for t in (0..3 * frame_cycles(lines)).step_by(7).chain([0, 1, 453, 454, 455, 908]) {
                assert_eq!(position(t, lines), clock_position(t, lines), "t {t}, {lines} lines");
            }
        }
    }

    #[test]
    fn a_line_is_454_cycles_and_227_slots() {
        assert_eq!(position(0, 313), (0, 0));
        assert_eq!(position(1, 313), (0, 0));
        assert_eq!(position(2, 313), (0, 1));
        assert_eq!(position(453, 313), (0, 226));
        assert_eq!(position(454, 313), (1, 0));
        assert_eq!(position(454 * 255 + 2 * 0xE2, 313), (255, 0xE2));
        assert_eq!(position(454 * 256, 313), (256, 0));
    }

    #[test]
    fn the_frame_wraps_after_313_lines() {
        let f = frame_cycles(313);
        assert_eq!(f, 142_102);
        assert_eq!(position(f - 1, 313), (312, 226));
        assert_eq!(position(f, 313), (0, 0));
        assert_eq!(position(5 * f + 454 * 44 + 20, 313), (44, 10));
        // A short frame wraps a line earlier.
        assert_eq!(position(frame_cycles(312), 312), (0, 0));
        assert_eq!(position(frame_cycles(312), 313), (312, 0));
    }

    #[test]
    fn the_registers_encode_and_decode() {
        assert_eq!(vposr(0, 313), 0x8000);
        assert_eq!(vposr(312, 313), 0x8001);
        assert_eq!(vposr(255, 313), 0x8000);
        assert_eq!(vposr(256, 312), 0x0001);
        assert_eq!(vhposr(0x2C, 0x40), 0x2C40);
        assert_eq!(vhposr(312, 0xE2), 0x38E2);
        for (line, h) in [(0, 0), (255, 0xE2), (256, 1), (312, 0x77)] {
            assert_eq!(decode(vposr(line, 313), vhposr(line, h)), (line, h));
            let (l, hh) = position(offset(line, h), 313);
            assert_eq!((l, hh), (line, h));
        }
    }
}
