//! The Amiga's beam, as one position type and one set of lengths (PAL).
//!
//! A raster line is 227 DMA slots, and the 68000 runs two cycles a slot, so a line is 454 CPU
//! cycles. A long PAL frame is 313 lines; an interlaced or non-LOF frame one fewer. Every beam
//! position and every frame or line length in the engine is written in terms of these, so the
//! numbers are written once.

/// Lines in a long PAL frame.
pub const FRAME_LINES: u16 = 313;
/// Lines in a short PAL frame (LOF clear).
pub const SHORT_FRAME_LINES: u16 = FRAME_LINES - 1;
/// CPU cycles in a raster line: two a DMA slot.
pub const LINE_CYCLES: u32 = 454;
/// DMA slots in a raster line: `h` runs `0..=$E2`.
pub const LINE_SLOTS: u16 = 227;

/// A beam position: a line (past [`FRAME_LINES`] when work runs on past the frame's end) and
/// CPU cycles into it.
///
/// Held in CPU cycles, not DMA slots, because the CPU's position on a line is not always on a
/// slot boundary; a coarser type would round it. Ordered on (line, cycle), which is the order of
/// (line, slot) for every slot-aligned position.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct BeamPos {
    pub line: u16,
    pub cycle: u32,
}

impl BeamPos {
    pub const fn new(line: u16, cycle: u32) -> BeamPos {
        BeamPos { line, cycle }
    }

    /// The position of DMA slot `h` on `line`.
    pub const fn from_slot(line: u16, h: u16) -> BeamPos {
        BeamPos {
            line,
            cycle: 2 * h as u32,
        }
    }

    /// The DMA slot the position is in (the copper's and the bus's `h`).
    pub fn slot(self) -> u16 {
        (self.cycle / 2) as u16
    }

    /// The horizontal position a Paula write is stamped with: the slot, held at the line's last
    /// when the position has run past it.
    pub fn paula_hpos(self) -> u16 {
        (self.cycle / 2).min(LINE_SLOTS as u32 - 1) as u16
    }

    /// CPU cycles from the frame's start.
    pub fn nominal(self) -> u32 {
        self.line as u32 * LINE_CYCLES + self.cycle
    }

    /// The position `t` CPU cycles from the frame's start; not wrapped at the frame's end.
    pub fn from_nominal(t: u32) -> BeamPos {
        BeamPos {
            line: (t / LINE_CYCLES) as u16,
            cycle: t % LINE_CYCLES,
        }
    }

    /// DMA slots from the frame's start.
    pub fn dma(self) -> u32 {
        self.line as u32 * LINE_SLOTS as u32 + self.slot() as u32
    }

    /// The position `t` DMA slots from the frame's start; not wrapped at the frame's end.
    pub fn from_dma(t: u32) -> BeamPos {
        BeamPos::from_slot(
            (t / LINE_SLOTS as u32) as u16,
            (t % LINE_SLOTS as u32) as u16,
        )
    }

    /// `n` DMA slots later, carrying into the following lines.
    pub fn plus_slots(self, n: u16) -> BeamPos {
        let h = self.slot() + n;
        BeamPos::from_slot(self.line + h / LINE_SLOTS, h % LINE_SLOTS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_round_trips_every_cycle_of_a_frame_and_past_it() {
        for line in 0..FRAME_LINES + 40 {
            for cycle in 0..LINE_CYCLES {
                let p = BeamPos::new(line, cycle);
                assert_eq!(BeamPos::from_nominal(p.nominal()), p);
            }
        }
        // Past the frame's end it counts on, never wrapping.
        let p = BeamPos::from_nominal(FRAME_LINES as u32 * LINE_CYCLES + 10);
        assert_eq!(p, BeamPos::new(FRAME_LINES, 10));
    }

    #[test]
    fn dma_and_slot_round_trip_every_slot_of_a_frame() {
        for line in 0..FRAME_LINES + 40 {
            for h in 0..LINE_SLOTS {
                let p = BeamPos::from_slot(line, h);
                assert_eq!(p.slot(), h);
                assert_eq!(BeamPos::from_dma(p.dma()), p);
                assert_eq!(p.dma(), line as u32 * 227 + h as u32);
            }
        }
    }

    #[test]
    fn paula_hpos_is_the_slot_held_at_the_lines_last() {
        // Level::beam's Clock::hpos as it was: (into / 2).min(LINE_SLOTS - 1).
        let old = |into: u32| (into / 2).min(227 - 1) as u16;
        for cycle in 0..LINE_CYCLES + 20 {
            assert_eq!(BeamPos::new(5, cycle).paula_hpos(), old(cycle));
        }
        assert_eq!(BeamPos::new(5, 453).paula_hpos(), 226);
        assert_eq!(BeamPos::new(5, 460).paula_hpos(), 226);
    }

    #[test]
    fn plus_slots_matches_the_coppers_beam_plus() {
        // amiga-chips' copper Beam::plus as it was.
        let old = |line: u16, h: u16, n: u16| {
            let h = h + n;
            (line + h / 0xE3, h % 0xE3)
        };
        for line in [0, 1, 0x2c, 255, 312] {
            for h in 0..LINE_SLOTS {
                for n in [0, 1, 2, 4, 226, 227, 500] {
                    let p = BeamPos::from_slot(line, h).plus_slots(n);
                    assert_eq!((p.line, p.slot()), old(line, h, n));
                }
            }
        }
    }

    #[test]
    fn ordering_equals_the_coppers_line_then_h_order() {
        let mut old = Vec::new();
        for line in (0..FRAME_LINES).step_by(7).chain([311, 312, 313]) {
            for h in (0..LINE_SLOTS).step_by(3).chain([0xE0, 0xE1, 0xE2]) {
                old.push((line, h));
            }
        }
        for a in &old {
            for b in &old {
                let (pa, pb) = (BeamPos::from_slot(a.0, a.1), BeamPos::from_slot(b.0, b.1));
                assert_eq!(pa.cmp(&pb), a.cmp(b), "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn the_lengths() {
        assert_eq!(LINE_CYCLES, 2 * LINE_SLOTS as u32);
        assert_eq!(SHORT_FRAME_LINES, 312);
        assert_eq!(LINE_SLOTS, 0xE3);
    }
}
