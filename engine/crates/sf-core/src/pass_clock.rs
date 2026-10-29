//! The level's pass and its frame boundary, by one rule wherever it is asked: the vertical blank
//! counts `vbl_count`, the main loop's 25 Hz pass is due once it reaches two
//! (`level_frame_loop`'s wait, h0+$aa6), and [`PassClock`] counts the passes frames end, what
//! conformance matches the state by (DESIGN §2.1 check 1).

use crate::layout::h0;
use crate::level::n_clear;

/// `vbl_count` (h0+$9d8): vertical blanks since the level's pass last began.
pub const VBL_COUNT: u32 = h0(0x9d8);

/// Counts the main-loop passes frames end (DESIGN §2.1 check 1), by the same rule on both sides.
/// Outside a level, where the main loop waits every frame, a frame is a pass. In a level a pass
/// begins in a frame that ends with `vbl_count` 0 (`level_frame_loop` clears it as it begins one,
/// and the next blank has not come), and ends in the first frame that does not end in the middle
/// of it: the same frame, or, for a late pass, a later one (in which the next pass may begin and
/// still be running at the frame's end, its predecessor's end then never seen).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct PassClock {
    /// A pass has begun and not been seen to end.
    running: bool,
}

impl PassClock {
    /// The passes a frame ended (0 to 2): `in_level`, in a level's loop at the frame's end;
    /// `mid_pass`, in the middle of a pass there.
    pub fn frame(&mut self, in_level: bool, mid_pass: bool, vbl_count: u16) -> u32 {
        if !in_level && !mid_pass {
            self.running = false;
            return 1;
        }
        let mut ended = 0;
        if vbl_count == 0 {
            ended += self.running as u32;
            self.running = true;
        }
        if !mid_pass && self.running {
            ended += 1;
            self.running = false;
        }
        ended
    }
}

/// Whether `level_frame_loop`'s wait is over: `vbl_count` has counted two blanks or more since
/// the last pass began (`cmp #2`, `bpl`; N alone, as [`n_clear`] tests it).
pub fn pass_due(vbl_count: u16) -> bool {
    n_clear(vbl_count.wrapping_sub(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DESIGN §2.1 check 1: what a frame's end says of the passes, as the original's main loop
    /// shows it (in a level: `vbl_count`, and whether it is in the middle of a pass).
    #[test]
    fn the_pass_clock_counts_passes_ended() {
        let mut c = PassClock::default();
        // Outside a level a frame is a pass.
        assert_eq!(c.frame(false, false, 7), 1);
        // On time: a pass begins and ends, then a frame's wait.
        assert_eq!(c.frame(true, false, 0), 1);
        assert_eq!(c.frame(true, false, 1), 0);
        // Late: begun, still running at the frame's end, ended in the next.
        assert_eq!(c.frame(true, true, 0), 0);
        assert_eq!(c.frame(true, false, 1), 1);
        // Late, and the next pass begun and running at the end of the frame the first ended in.
        assert_eq!(c.frame(true, true, 0), 0);
        assert_eq!(c.frame(true, true, 0), 1);
        assert_eq!(c.frame(true, false, 1), 1);
        // Out of the wait with no pass begun (the frame's end between its calls): nothing ended.
        assert_eq!(c.frame(true, true, 1), 0);
        assert_eq!(c.frame(true, true, 0), 0);
        assert_eq!(c.frame(true, false, 1), 1);
    }

    #[test]
    fn a_pass_is_due_from_the_second_blank() {
        assert!(!pass_due(0));
        assert!(!pass_due(1));
        assert!(pass_due(2));
        assert!(pass_due(0x7fff));
        // N alone: a count of $8002 or more reads as negative, as on the original.
        assert!(!pass_due(0x8002));
    }
}
