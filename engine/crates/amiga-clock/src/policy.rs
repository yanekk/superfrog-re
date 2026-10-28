//! How a display's code spends CPU time on the beam, one function per rounding. Each is the
//! arithmetic a client used inline, moved here unchanged: the clients' constants were fitted on
//! these roundings, so a finer or a shared rounding would move what they time (a note restart, a
//! blit's line). The fitted numbers (a steal per slot or per word) stay with the caller; only the
//! arithmetic is here.

use crate::{BeamPos, LINE_CYCLES};

/// Lines `cycles` of work take when only whole lines are counted: a part line counts whole.
pub fn whole_lines(cycles: u32) -> u32 {
    cycles.div_ceil(LINE_CYCLES)
}

/// Work of `cycles` from `into` cycles into a line, counted to the cycle: the lines it moves on
/// and the cycles into the line it ends on.
pub fn exact_on_line(into: u32, cycles: u32) -> (u32, u32) {
    let t = into + cycles;
    (t / LINE_CYCLES, t % LINE_CYCLES)
}

/// The line a CPU wait for `need` cycles ends on, from the start of `line`, counted in whole
/// lines: a line `fetching` says the display fetches on gives the CPU `LINE_CYCLES - per_line`,
/// any other the whole line. Stops at `u16::MAX`.
pub fn wait_lines(line: u16, need: u32, per_line: u32, fetching: impl Fn(u16) -> bool) -> u16 {
    let (mut line, mut need) = (line, need);
    while need > 0 && line < u16::MAX {
        let free = LINE_CYCLES - if fetching(line) { per_line } else { 0 };
        need = need.saturating_sub(free);
        line += 1;
    }
    line
}

/// Where `cycles` of work from `t` (CPU cycles from the frame's start) end, a line `fetching`
/// says the display fetches on (asked of the line wrapped at `wrap`) losing `taken` of its
/// `LINE_CYCLES` to it, spread evenly over the line; the end rounded up to the cycle.
pub fn spend_stolen(
    t: u32,
    cycles: u32,
    taken: u32,
    wrap: u16,
    fetching: impl Fn(u16) -> bool,
) -> u32 {
    let (mut t, mut left) = (t, cycles);
    while left > 0 {
        let at = BeamPos::from_nominal(t);
        let free = if fetching(at.line % wrap) {
            LINE_CYCLES - taken
        } else {
            LINE_CYCLES
        };
        let here = (LINE_CYCLES - at.cycle) * free / LINE_CYCLES;
        if left <= here {
            return t + (left * LINE_CYCLES).div_ceil(free);
        }
        left -= here;
        t += LINE_CYCLES - at.cycle;
    }
    t
}

/// The CPU cycles of work free between `t` and `until` under [`spend_stolen`]'s steal, each line's
/// share rounded down.
pub fn free_between(
    t: u32,
    until: u32,
    taken: u32,
    wrap: u16,
    fetching: impl Fn(u16) -> bool,
) -> u32 {
    (t / LINE_CYCLES..until.div_ceil(LINE_CYCLES))
        .map(|l| {
            let (from, to) = ((l * LINE_CYCLES).max(t), ((l + 1) * LINE_CYCLES).min(until));
            let free = if fetching((l % wrap as u32) as u16) {
                LINE_CYCLES - taken
            } else {
                LINE_CYCLES
            };
            (to - from) * free / LINE_CYCLES
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FRAME_LINES;

    use crate::LINE_CYCLES as LINE;

    // The clients' arithmetic as it was, the reference each policy is held to.

    /// `Cx::busy`, `into: None` (and `Cx::unpack`'s line count).
    fn old_whole(cycles: u32) -> u32 {
        cycles.div_ceil(LINE)
    }

    /// `Cx::busy`, `into: Some`.
    fn old_exact(into: u32, cycles: u32) -> (u32, u32) {
        let t = into + cycles;
        (t / LINE, t % LINE)
    }

    /// `Cx::blit_wait`'s loop, `shown` asked of `line % FRAME_LINES`.
    fn old_wait(line: u16, need: u32, per_line: u32, lines: std::ops::Range<u16>) -> u16 {
        let (mut line, mut need) = (line, need);
        while need > 0 && line < u16::MAX {
            let shown = lines.contains(&(line % FRAME_LINES));
            let free = LINE - if shown { per_line } else { 0 };
            need = need.saturating_sub(free);
            line += 1;
        }
        line
    }

    /// `slot/draw.rs` `spend`, `taken` already worked out.
    fn old_spend(lines: std::ops::Range<u16>, taken: u32, t: u32, cycles: u32) -> u32 {
        let (mut t, mut left) = (t, cycles);
        while left > 0 {
            let line = (t / LINE) as u16 % FRAME_LINES;
            let into = t % LINE;
            let free = if lines.contains(&line) {
                LINE - taken
            } else {
                LINE
            };
            let here = (LINE - into) * free / LINE;
            if left <= here {
                return t + (left * LINE).div_ceil(free);
            }
            left -= here;
            t += LINE - into;
        }
        t
    }

    /// `slot/draw.rs` `done`.
    fn old_done(lines: std::ops::Range<u16>, taken: u32, t: u32, until: u32) -> u32 {
        (t / LINE..until.div_ceil(LINE))
            .map(|l| {
                let (from, to) = ((l * LINE).max(t), ((l + 1) * LINE).min(until));
                let free = if lines.contains(&((l % FRAME_LINES as u32) as u16)) {
                    LINE - taken
                } else {
                    LINE
                };
                (to - from) * free / LINE
            })
            .sum()
    }

    /// The fetch windows the front end and the slot show (a 256-line lowres picture at `$2c`, the
    /// slot's, a short one, none, one running to the frame's end) and the steals they give: the
    /// slot's six planes of 20 words at 1.4 and 1.9 cycles a word (168, 228), `Cx::blit_wait`'s
    /// 1.23 a slot of 1 to 6 planes (24..147), and the cap at a line less one.
    fn windows() -> Vec<std::ops::Range<u16>> {
        vec![0x2c..0x12c, 0x2c..0x10c, 0x30..0x50, 0..0, 0x100..313]
    }
    const TAKEN: [u32; 8] = [0, 24, 49, 98, 147, 168, 228, 453];

    fn cycles() -> impl Iterator<Item = u32> {
        (0..2 * LINE)
            .chain((2 * LINE..3 * 313 * LINE).step_by(997))
            .chain([313 * LINE - 1, 313 * LINE, 313 * LINE + 1, 3 * 313 * LINE])
    }

    #[test]
    fn whole_lines_is_cx_busys_count_with_no_cycle_known() {
        for c in (0..4 * 313 * LINE)
            .step_by(7)
            .chain([453, 454, 455, u32::MAX])
        {
            assert_eq!(whole_lines(c), old_whole(c), "{c}");
        }
    }

    #[test]
    fn exact_on_line_is_cx_busys_count_to_the_cycle() {
        for into in [0, 1, 227, 453] {
            for c in cycles() {
                assert_eq!(exact_on_line(into, c), old_exact(into, c), "{into} {c}");
            }
        }
    }

    #[test]
    fn wait_lines_is_cx_blit_waits_loop() {
        for w in windows() {
            for per_line in [0, 24, 49, 73, 98, 123, 147] {
                for line in (0..FRAME_LINES + 40).chain([u16::MAX - 2, u16::MAX - 1, u16::MAX]) {
                    for need in [0, 1, 307, 454, 455, 10_000, 70_000, 313 * LINE * 2] {
                        let lines = w.clone();
                        let new = wait_lines(line, need, per_line, |l| {
                            lines.contains(&(l % FRAME_LINES))
                        });
                        assert_eq!(new, old_wait(line, need, per_line, w.clone()));
                    }
                }
            }
        }
    }

    #[test]
    fn spend_stolen_is_the_slot_draws_spend() {
        for w in windows() {
            for taken in TAKEN {
                for line in (0..FRAME_LINES + 20).step_by(3).chain([311, 312, 313, 314]) {
                    for into in [0, 1, 226, 227, 453] {
                        let t = line as u32 * LINE + into;
                        for c in cycles().step_by(5) {
                            let lines = w.clone();
                            let new =
                                spend_stolen(t, c, taken, FRAME_LINES, |l| lines.contains(&l));
                            assert_eq!(
                                new,
                                old_spend(w.clone(), taken, t, c),
                                "{w:?} {taken} {t} {c}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn free_between_is_the_slot_draws_line_sum() {
        for w in windows() {
            for taken in TAKEN {
                for line in (0..FRAME_LINES + 20).step_by(3).chain([311, 312, 313, 314]) {
                    for into in [0, 1, 226, 227, 453] {
                        let t = line as u32 * LINE + into;
                        for until in cycles().step_by(5).filter(|&u| u >= t) {
                            let lines = w.clone();
                            let new =
                                free_between(t, until, taken, FRAME_LINES, |l| lines.contains(&l));
                            assert_eq!(new, old_done(w.clone(), taken, t, until));
                        }
                    }
                }
            }
        }
    }
}
