//! The game's display timing.

use amiga_clock::{DisplayTiming, Timing};

/// The game's display timing, carrying the frame length measured on the oracle
/// (games/forest-dumb/docs/frame-length.md).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FdTiming;

impl Timing for FdTiming {
    // The measurement, not the toolkit's PAL constant, though they agree: 313 on every frame of the
    // game proper. The 312-line frames sit under the crack's interlaced `Show` picture, before the
    // engine has anything to model.
    //
    // Nothing is known of the game's bitplane fetch yet, and the first case uses only the frame
    // length, so the fetch is empty, which steals no cycle, rather than a guess at a window.
    const TIMING: DisplayTiming = DisplayTiming {
        frame_lines: 313,
        fetch_lines: 0..0,
        fetch_slots: 0..0,
        blitter_free: 0,
        stolen_cycles: 0,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_lines_is_the_measured_value() {
        // T00's FINDINGS row "Frame length: 313 every frame from the crack intro on".
        assert_eq!(FdTiming::TIMING.frame_lines, 313);
    }

    #[test]
    fn no_line_fetches() {
        assert!(FdTiming::TIMING.fetch_lines.is_empty());
        assert!(FdTiming::TIMING.fetch_slots.is_empty());
    }
}
