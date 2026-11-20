//! The status bar (plans/forest-dumb-level-1 T10, DESIGN §2.6; docs/09-play-measured.md, "Display").
//!
//! The bar is 320 x 56 below the play area, five planes interleaved a line, in its own buffer at
//! `h4+$6400`: the main program's own picture of it, which the copper shows from line 241. Every 32
//! interrupts `main_loop` runs `h0_6a96`, which redraws each number that changed since it last drew
//! it into that buffer: the countdown, the lives, the score and the items left. Digits are 8 x 8 from
//! the font at `h4+$8fc0`; the lives are a two-digit count of threes and one to three icons for the
//! rest (from `h4+$9150`). Ported from `h0_6a96`, `h0_6ae8` (score), `h0_6b70` and `h0_6b78` (a digit),
//! `h0_6bc0` (countdown), `h0_6c20` (items) and `h0_6c80` (lives), each proven on the routine rig.
//!
//! None of these is in `re/translate.toml`, which holds `game_frame`'s call tree and the edge
//! redraws: `main_loop` calls `h0_6a96` (`h0+$29c`). T10 ports them because DESIGN §2.6 has it draw
//! the bar.
//!
//! Interruptible points (porting rule 1): all of them run in the foreground, so the level-3
//! interrupt can take a pass between any two instructions, and `main_loop` keeps calling `h0_6a96`
//! for as long as `irq_count & 31` is 0. The engine draws the bar whole before the pass. Only the
//! picture can differ, for the frame a pass cuts a redraw short in; no game state is written.
//!
//! The buffer and what it last drew are picture state.

use crate::program::Program;

/// The bar's buffer, hunk 4.
pub const STATUS: u32 = 0x6400;
/// Its bytes: 56 lines of five 40-byte planes.
pub const STATUS_BYTES: u32 = 56 * 5 * 0x28;
/// The digits' font, hunk 4: ten 8 x 8 digits a line, `$50` bytes a plane.
pub const FONT: u32 = 0x8fc0;
/// The lives icons, hunk 4.
pub const ICONS: u32 = 0x9150;
/// The icon offsets, hunk 0: three bytes for each remainder of the lives by 3.
pub const ICON_TABLE: u32 = 0x6da0;

/// The numbers as `h0_6a96` last drew them (`h0+$6a8c` countdown, `$6a8e` lives, `$6a90` items,
/// `$6a92` score); `$ffff` as loaded, so each is drawn the first time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shown {
    pub countdown: u16,
    pub lives: u16,
    pub items: u16,
    pub score: u32,
}

/// The status bar.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Status {
    /// Hunk 4 from [`STATUS`], [`STATUS_BYTES`] long.
    pub bytes: Vec<u8>,
    pub shown: Shown,
}

impl std::fmt::Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status({:?})", self.shown)
    }
}

/// What the bar's routines read: the program's hunk 4 (font, icons) by offset, and hunk 0's table.
pub struct Sources<'a> {
    pub h4: &'a dyn Fn(u32) -> u8,
    pub program: &'a Program,
}

impl Status {
    fn put(&mut self, at: u32, v: u8) {
        if let Some(b) = self.bytes.get_mut(at.wrapping_sub(STATUS) as usize) {
            *b = v;
        }
    }

    /// `h0_6a96`: redraw what changed. `countdown` is the variable's first word.
    pub fn update(&mut self, countdown: u16, lives: u16, score: u32, items: u16, src: &Sources) {
        if countdown != self.shown.countdown {
            self.countdown(countdown, src);
        }
        if lives != self.shown.lives {
            self.lives(lives, src);
        }
        if score != self.shown.score {
            self.score(score, src);
        }
        if items != self.shown.items {
            self.items(items, src);
        }
    }

    /// `h0_6b78`: digit `d` (the font's byte column, a long) at hunk 4 offset `at`: five times eight
    /// bytes, the font's `$a` bytes a row apart, the bar's `$28`.
    pub fn digit(&mut self, d: u32, at: u32, src: &Sources) {
        let (mut from, mut to) = (FONT.wrapping_add(d), at);
        for _ in 0..5 {
            for r in 0..8 {
                self.put(
                    to.wrapping_add(0x28 * r),
                    (src.h4)(from.wrapping_add(0xa * r)),
                );
            }
            from = from.wrapping_add(0x50);
            to = to.wrapping_add(0x140);
        }
    }

    /// `h0_6b70`: digit `d` at the score's place `d1`.
    pub fn score_digit(&mut self, d: u32, d1: u32, src: &Sources) {
        self.digit(d, 0x84ed_u32.wrapping_add(d1), src);
    }

    /// `h0_6ae8`: the score's six low BCD digits, the lowest at place 7.
    pub fn score(&mut self, score: u32, src: &Sources) {
        for k in 0..6 {
            self.score_digit(score >> (4 * k) & 0xf, 7 - k, src);
        }
        self.shown.score = score;
    }

    /// Three BCD digits of `v` at `at`, the lowest at place 2 (`h0_6bc0`, `h0_6c20`).
    fn three(&mut self, v: u16, at: u32, src: &Sources) {
        for k in 0..3u32 {
            self.digit((v as u32) >> (4 * k) & 0xf, at + 2 - k, src);
        }
    }

    /// `h0_6bc0`: the countdown.
    pub fn countdown(&mut self, v: u16, src: &Sources) {
        self.three(v, 0x7aba, src);
        self.shown.countdown = v;
    }

    /// `h0_6c20`: the items left.
    pub fn items(&mut self, v: u16, src: &Sources) {
        self.three(v, 0x7ac2, src);
        self.shown.items = v;
    }

    /// `h0_6c80`: the lives, BCD: their value over 3 as two digits, then three icons, which the
    /// remainder picks from [`ICON_TABLE`], each 70 words a `$28`-byte row apart.
    pub fn lives(&mut self, v: u16, src: &Sources) {
        let n = (v >> 4 & 0xf) as u32 * 10 + (v & 0xf) as u32;
        let threes = n / 3;
        self.digit(threes % 10, 0x8664, src);
        self.digit(threes / 10 & 0xf, 0x8663, src);
        let row = ICON_TABLE + (n % 3) * 4;
        for k in 0..3u32 {
            let from = ICONS + src.program.byte(row + k) as u32;
            let to = 0x7d06 + 2 * k;
            for r in 0..70u32 {
                for b in 0..2 {
                    let v = (src.h4)(from + 2 * r + b);
                    self.put(to + 0x28 * r + b, v);
                }
            }
        }
        self.shown.lives = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A font whose byte is its offset's low byte, icons the same, a table of 0, `$8c`, `$8c`...
    fn sources(f: impl Fn(&Sources)) {
        let h4 = |a: u32| a as u8;
        let mut b = vec![0u8; 0x6dac];
        b[0x6da0..0x6dac].copy_from_slice(&[0x8c, 0, 0, 0, 0x8c, 0x8c, 0, 0, 0x8c, 0x8c, 0x8c, 0]);
        let program = Program::hunk0(b);
        f(&Sources {
            h4: &h4,
            program: &program,
        });
    }

    fn bar() -> Status {
        Status {
            bytes: vec![0; STATUS_BYTES as usize],
            shown: Shown {
                countdown: 0xffff,
                lives: 0xffff,
                items: 0xffff,
                score: 0xffff_ffff,
            },
        }
    }

    #[test]
    fn a_digit_copies_five_blocks_of_eight_rows() {
        sources(|src| {
            let mut s = bar();
            s.digit(3, 0x6400, src);
            assert_eq!(s.bytes[0], (FONT + 3) as u8);
            assert_eq!(s.bytes[0x28], (FONT + 3 + 0xa) as u8);
            assert_eq!(s.bytes[0x140], (FONT + 3 + 0x50) as u8);
            assert_eq!(s.bytes.iter().filter(|&&b| b != 0).count(), 40);
        });
    }

    #[test]
    fn only_what_changed_is_drawn_and_remembered() {
        sources(|src| {
            let mut s = bar();
            s.update(0x999, 0x32, 0x1234, 0x282, src);
            assert_eq!(
                s.shown,
                Shown {
                    countdown: 0x999,
                    lives: 0x32,
                    items: 0x282,
                    score: 0x1234
                }
            );
            let drawn = s.clone();
            s.bytes.fill(0);
            s.update(0x999, 0x32, 0x1234, 0x282, src);
            assert!(
                s.bytes.iter().all(|&b| b == 0),
                "nothing changed, nothing drawn"
            );
            s.update(0x998, 0x32, 0x1234, 0x282, src);
            assert!(s.bytes.iter().any(|&b| b != 0));
            assert_ne!(s.bytes, drawn.bytes);
        });
    }

    #[test]
    fn thirty_two_lives_are_ten_threes_and_two_icons() {
        sources(|src| {
            let mut s = bar();
            s.lives(0x32, src);
            // 32 = 10 * 3 + 2: digits 0 at place 1 and 1 at place 0; icons $8c, $8c, $8c.
            assert_eq!(s.bytes[(0x8664 - STATUS) as usize], FONT as u8);
            assert_eq!(s.bytes[(0x8663 - STATUS) as usize], (FONT + 1) as u8);
            assert_eq!(s.bytes[(0x7d06 - STATUS) as usize], (ICONS + 0x8c) as u8);
        });
    }
}
