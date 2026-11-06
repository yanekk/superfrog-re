//! The level's palette and its fade-in (plans/forest-dumb-level-1 T10; docs/06-graphics-and-rendering.md,
//! "The palette"; docs/09-play-measured.md, "Display").
//!
//! The level copper list sets COLOR00-31 from 32 words at `h1+$13e`, four bytes apart (each a MOVE's
//! value). `level_init` sets `h0+$94b4`; from then each pass's `h0_94ba` steps every colour one shade
//! towards the level palette at `h1+$306` for 32 passes, with `h0+$94b6` counting them. Whether a
//! component steps on a pass is the fade table's (`h0+$9624`, program data): byte
//! `16 * target + pass / 2` set means it steps. Ported from `h0_94ba`, `h0_94fe` and `h0_9550`, each
//! proven on the routine rig (`fd-conform/tests/drawing_routines.rs`).
//!
//! Interruptible points (porting rule 1): none. All three run inside the level-3 interrupt
//! (`game_frame`) with no wait, so the engine running them whole matches.
//!
//! The colours are picture state: nothing in play logic reads them.

use crate::program::Program;

/// The fade table's hunk 0 offset.
pub const FADE_TABLE: u32 = 0x9624;

/// The fade: where the copper list's colours are, and how far the fade has gone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fade {
    /// `h0+$94b4`: set by `level_init`, cleared when the fade ends.
    pub on: u8,
    /// `h0+$94b6`, a long: the passes the fade has run.
    pub pass: u32,
    /// COLOR00-31 as the copper list holds them (`h1+$13e`, every fourth byte pair).
    pub now: [u16; 32],
    /// `h0+$961e`, `$9620`, `$9622`: the last colour's three components as `h0_9550` left them.
    /// Nothing reads them back but `h0_9550` itself.
    pub scratch: [u16; 3],
}

impl Fade {
    /// `h0_94ba`: one pass of the fade towards `to` (`h1+$306`).
    pub fn step(&mut self, to: &[u16; 32], program: &Program) {
        if self.on == 0 {
            self.pass = 0;
            return;
        }
        self.all(to, self.pass >> 1, program);
        self.pass = self.pass.wrapping_add(1);
        if self.pass == 0x20 {
            self.pass = 0;
            self.on = 0;
        }
    }

    /// `h0_94fe`: every colour one step, at fade column `d6` (only its low word is used).
    pub fn all(&mut self, to: &[u16; 32], d6: u32, program: &Program) {
        for (now, &to) in self.now.iter_mut().zip(to) {
            let (c, s) = colour(to, *now, d6 as u16, program);
            *now = c;
            self.scratch = s;
        }
    }
}

/// `h0_9550`: colour `now` one step towards `to` at fade column `d6`: each component of `now` that
/// differs from `to`'s gains one when the fade table's byte for `to`'s component and the column is
/// set. Returns the new colour (the low word of `d3`) and the three component words it stores, each
/// the component's word as computed, so a blue of `$f` that steps carries into green as the
/// original's does.
pub fn colour(to: u16, now: u16, d6: u16, program: &Program) -> (u16, [u16; 3]) {
    // (mask, the shift taking the target's component to table row * 16, the step)
    let parts = [(0xf, 4i32, 1u16), (0xf0, 0, 0x10), (0xf00, -4, 0x100)];
    let mut s = [0u16; 3];
    for (k, (mask, shift, one)) in parts.into_iter().enumerate() {
        let (t, mut n) = (to & mask, now & mask);
        if t != n {
            let row = if shift >= 0 { t << shift } else { t >> -shift };
            if program.byte(FADE_TABLE + row.wrapping_add(d6) as u32) != 0 {
                n = n.wrapping_add(one);
            }
        }
        s[k] = n;
    }
    (s[2] | s[0] | s[1], s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fade table whose byte `16 * c + col` is set when `col < c`: a component steps while the
    /// column is below the target.
    fn table() -> Program {
        let mut b = vec![0u8; (FADE_TABLE + 0x200) as usize];
        for c in 0..16u32 {
            for col in 0..16u32 {
                b[(FADE_TABLE + 16 * c + col) as usize] = (col < c) as u8;
            }
        }
        Program::hunk0(b)
    }

    #[test]
    fn a_colour_steps_each_component_the_table_allows() {
        let p = table();
        assert_eq!(colour(0x0fa5, 0, 0, &p).0, 0x111);
        assert_eq!(colour(0x0fa5, 0x111, 4, &p).0, 0x222);
        // Column 5: blue's target 5 no longer steps, red and green do.
        assert_eq!(colour(0x0fa5, 0x444, 5, &p), (0x554, [4, 0x50, 0x500]));
        // Equal components never step.
        assert_eq!(colour(0x0fa5, 0x0fa5, 0, &p).0, 0x0fa5);
    }

    #[test]
    fn a_blue_past_its_target_carries_into_green() {
        // The original adds without a carry check: blue `$f` with a target of 3 whose table byte
        // is set steps to `$10`, into green.
        let mut b = vec![0u8; (FADE_TABLE + 0x200) as usize];
        b[(FADE_TABLE + 16 * 3) as usize] = 1;
        let p = Program::hunk0(b);
        assert_eq!(colour(0x003, 0x00f, 0, &p), (0x010, [0x10, 0, 0]));
    }

    #[test]
    fn the_fade_runs_thirty_two_passes_and_stops() {
        let p = table();
        let to = [0x0fff; 32];
        let mut f = Fade {
            on: 1,
            pass: 0,
            now: [0; 32],
            scratch: [0; 3],
        };
        for _ in 0..32 {
            f.step(&to, &p);
        }
        assert_eq!((f.on, f.pass), (0, 0));
        assert_eq!(f.now, to);
        let done = f.clone();
        f.step(&[0; 32], &p);
        assert_eq!(f, done, "off, a pass changes nothing");
    }
}
