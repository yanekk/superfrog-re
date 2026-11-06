//! The play screen and how the copper shows it (plans/forest-dumb-level-1 T10; docs/09-play-measured.md,
//! "Display" and "Are the display buffers game state?").
//!
//! The play screen is a circular buffer of 256 lines from `h3+$398`, five planes interleaved a line
//! ([`SCREEN_LINE`] bytes). Level line `y` lives on buffer line `y & 255`. The edge redraws (T55,
//! `level/redraw.rs`) blit tiles and bobs into it as the camera crosses cells, and nothing redraws a
//! still screen, so the engine keeps it: [`Screen`] is the buffer as the original's blits leave it,
//! applied from the redraws' [`Blit`] list. It is picture state, not game state: nothing in play
//! logic reads it (measured, T00), so `Game`'s equality leaves it out.
//!
//! `h0_9754` points the copper list's bitplane pointers at it from the camera ([`Scroll`]): the top
//! of the window shows buffer line `(camera_y & 255) + 1`, a copper WAIT reloads the pointers at the
//! buffer's start where it wraps, and BPLCON1 scrolls the pixels inside a word.
//!
//! Interruptible points (porting rule 1): `h0_9754` none, inside the level-3 interrupt. `h0_3ab4`
//! waits on the blitter, which the engine's blits never leave busy.

use crate::level::redraw::{Blit, SCREEN_LINE};
use crate::program::h;

/// The play screen's hunk 3 offset.
pub const SCREEN: u32 = 0x398;
/// The bytes of hunk 3 the engine keeps from [`SCREEN`]: the 256 lines and the slack before
/// `collision_map` (`h3+$fa50`), which the pointers move into as the view scrolls right.
pub const SCREEN_BYTES: u32 = 0xfa50 - SCREEN;
/// A bitplane's bytes in a line.
pub const PLANE_BYTES: u32 = 0x2e;
/// Chip RAM's address lines: a blit's address wraps on them.
const CHIP_MASK: u32 = 0x7_ffff;

/// `h0_3ab4`: wait while the blitter is busy. It changes nothing; the engine's blits finish at once.
pub fn blit_done() {}

/// What `h0_9754` writes into the level copper list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scroll {
    /// The five bitplane pointers at the window's top (`h1+$1d0`), as byte offsets from [`SCREEN`]:
    /// plane `k` at `top + k * PLANE_BYTES`.
    pub top: u32,
    /// The pointers the WAIT reloads (`h1+$200`): the buffer's first line.
    pub wrap: u32,
    /// BPLCON1 (`h1+$1c2`).
    pub bplcon1: u16,
    /// The WAIT's line (`h1+$1fc`) when the buffer wraps inside the window; `None` when it does not,
    /// and the WAIT is left at line `$e8` and the MOVE before it (`h1+$1f8`) as it was.
    pub wait: Option<u8>,
}

impl Scroll {
    /// `h0_9754`: the pointers, scroll and wrap for camera (`x`, `y`).
    pub fn from_camera(x: u16, y: u16) -> Scroll {
        let column = ((x >> 4) as u32) << 1;
        let up = (y as u32 & 0xff) ^ 0xff;
        let line = 0xf0u32.wrapping_sub(up).wrapping_add(0x10);
        let wait = up + 0x2f;
        Scroll {
            top: (line & 0xffff) * SCREEN_LINE + column,
            wrap: column,
            bplcon1: ((x & 0xf) ^ 0xf) * 0x11,
            wait: (wait < 0xf0).then_some(wait as u8),
        }
    }

    /// The copper list's bytes this sets, as (hunk 1 offset, bytes), in the original's order.
    pub fn copper(&self) -> Vec<(u32, Vec<u8>)> {
        let mut out = Vec::new();
        // `move.w d1, 6(a1); swap d1; move.w d1, 2(a1)` for each plane, a line's planes apart.
        let pointers = |out: &mut Vec<(u32, Vec<u8>)>, list: u32, first: u32| {
            for k in 0..5 {
                let a = h(3, SCREEN)
                    .wrapping_add(first)
                    .wrapping_add(k * PLANE_BYTES);
                out.push((list + 8 * k + 6, (a as u16).to_be_bytes().to_vec()));
                out.push((list + 8 * k + 2, ((a >> 16) as u16).to_be_bytes().to_vec()));
            }
        };
        pointers(&mut out, 0x1d0, self.top);
        out.push((0x1c2, self.bplcon1.to_be_bytes().to_vec()));
        pointers(&mut out, 0x200, self.wrap);
        match self.wait {
            Some(w) => {
                out.push((0x1f8, 0x0180_0000u32.to_be_bytes().to_vec()));
                out.push((0x1fc, ((w as u32) << 24 | 0x1ff00).to_be_bytes().to_vec()));
            }
            None => out.push((0x1fc, 0xe801_ff00u32.to_be_bytes().to_vec())),
        }
        out.push((0x22c, 0xe901_ff00u32.to_be_bytes().to_vec()));
        out
    }
}

/// The play screen as the original's blits leave it: hunk 3 from [`SCREEN`], [`SCREEN_BYTES`] long.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Screen {
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Screen({} bytes)", self.bytes.len())
    }
}

impl Default for Screen {
    fn default() -> Screen {
        Screen {
            bytes: vec![0; SCREEN_BYTES as usize],
        }
    }
}

impl Screen {
    /// The buffer index of chip address `a`, if the engine keeps it.
    fn index(a: u32) -> Option<usize> {
        let i = (a & CHIP_MASK).wrapping_sub(h(3, SCREEN));
        (i < SCREEN_BYTES).then_some(i as usize)
    }

    fn word(&self, a: u32) -> u16 {
        let b = |a: u32| Screen::index(a).map_or(0, |i| self.bytes[i]);
        u16::from_be_bytes([b(a), b(a.wrapping_add(1))])
    }

    fn set_word(&mut self, a: u32, v: u16) {
        for (k, byte) in v.to_be_bytes().into_iter().enumerate() {
            if let Some(i) = Screen::index(a.wrapping_add(k as u32)) {
                self.bytes[i] = byte;
            }
        }
    }

    /// A tile's 80 words from `src` bytes past `h3+$13540` (A to D, `BLTCON0=$09f0`, modulo 0) to the
    /// screen at `dst`.
    fn copy(&mut self, src: u32, dst: u32, word: &impl Fn(u32) -> u16) {
        let src = h(3, 0x1_3540).wrapping_add(src);
        let dst = h(3, SCREEN).wrapping_add(dst);
        for r in 0..80 {
            let v = word(src.wrapping_add(2 * r));
            self.set_word(dst.wrapping_add(r * PLANE_BYTES), v);
        }
    }

    /// Apply one of the redraws' or the objects' blits, its sources read by chip address through
    /// `chip`. All are 80 rows (five planes of 16 lines) with D's modulo making a line: the tile, cell,
    /// bob and element blits one word wide (`BLTSIZE=$1401`), the picture two (`$1402`). A word whose
    /// destination falls outside the play screen lands elsewhere in chip RAM, where nothing the
    /// picture shows lies (the stray bob after a full creature table, docs/09), and is dropped.
    pub fn apply(&mut self, b: &Blit, chip: &impl Fn(u32) -> u8) {
        let word = |a: u32| u16::from_be_bytes([chip(a), chip(a.wrapping_add(1))]);
        match *b {
            // `h0_774`: A to D, `BLTCON0=$09f0`, A from tile `tile` of `h3+$13540`, modulo 0.
            Blit::Tile { tile, dst } => self.copy(tile as u32 * 0xa0, dst, &word),
            // `h0_29ca`: the same blit, from the tile table's offset (T22).
            Blit::Cell { src, dst } => self.copy(src, dst, &word),
            // `h0_a06` and its twins: `BLTCON0=$0fca`, D = A·B + ¬A·C; A the mask (`h4+$2580`), B the
            // picture (`h4+$c80`), modulos `$26`; C and D the screen.
            Blit::Bob { image, dst } => {
                let mask = h(4, 0x2580).wrapping_add(image);
                let pic = h(4, 0xc80).wrapping_add(image);
                let dst = h(3, SCREEN).wrapping_add(dst);
                for r in 0..80 {
                    let (a, b) = (word(mask + 0x28 * r), word(pic + 0x28 * r));
                    let d = dst.wrapping_add(r * PLANE_BYTES);
                    let c = self.word(d);
                    self.set_word(d, a & b | !a & c);
                }
            }
            // `h0_62b6`: the same cut with C the cell's tile, not the screen (T25).
            Blit::Element { frame, tile, dst } => {
                let mask = h(4, 0x1900).wrapping_add(2 * frame);
                let pic = h(4, 0).wrapping_add(2 * frame);
                let tile = h(3, 0x1_3540).wrapping_add(tile);
                let dst = h(3, SCREEN).wrapping_add(dst);
                for r in 0..80 {
                    let (a, b) = (
                        word(mask.wrapping_add(0x28 * r)),
                        word(pic.wrapping_add(0x28 * r)),
                    );
                    let c = word(tile.wrapping_add(2 * r));
                    self.set_word(dst.wrapping_add(r * PLANE_BYTES), a & b | !a & c);
                }
            }
            // `h0_21c0`, `h0_62b6`: two words a row from hunk 4 (T25).
            Blit::Picture { src, modulo, dst } => {
                let src = h(4, 0).wrapping_add(src);
                let dst = h(3, SCREEN).wrapping_add(dst);
                let stride = 4 + modulo as u32;
                for r in 0..80 {
                    for k in [0, 2] {
                        let v = word(src.wrapping_add(r * stride + k));
                        self.set_word(dst.wrapping_add(r * PLANE_BYTES + k), v);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_top_is_the_line_below_camera_y_and_the_wait_is_where_it_wraps() {
        let s = Scroll::from_camera(0x25, 0x80);
        assert_eq!(s.top, 0x81 * SCREEN_LINE + 4);
        assert_eq!(s.wrap, 4);
        assert_eq!(s.bplcon1, 0xaa);
        // 256 - $81 lines below the window's top (line $2f) the buffer wraps.
        assert_eq!(s.wait, Some(0x2f + 0x7f));
        // Near the buffer's start the window's 180 lines do not reach the wrap.
        assert_eq!(Scroll::from_camera(0, 0x17).wait, None);
        // At `y & 255 = 255` the top is one past the buffer and the wait is the window's first line.
        let s = Scroll::from_camera(0, 0xff);
        assert_eq!((s.top, s.wait), (0x100 * SCREEN_LINE, Some(0x2f)));
    }

    #[test]
    fn the_copper_words_hold_absolute_pointers() {
        let w = Scroll::from_camera(0x10, 0xff).copper();
        let first = h(3, SCREEN) + 0x100 * SCREEN_LINE + 2;
        assert_eq!(w[0], (0x1d6, (first as u16).to_be_bytes().to_vec()));
        assert_eq!(w[1], (0x1d2, ((first >> 16) as u16).to_be_bytes().to_vec()));
        assert_eq!(w.last().unwrap().0, 0x22c);
    }

    #[test]
    fn a_tile_lands_a_plane_row_apart_and_a_bob_cuts_through_its_mask() {
        let mut s = Screen::default();
        // Tile 1's bytes are k + 1; the bob's picture is all ones under a left-half mask.
        let chip = |a: u32| {
            let t = h(3, 0x1_3540) + 0xa0;
            if (t..t + 0xa0).contains(&a) {
                (a - t + 1) as u8
            } else if a >= h(4, 0x2580) && a < h(4, 0x2580) + 0x28 * 80 {
                if (a - h(4, 0x2580)) % 0x28 == 0 {
                    0xff
                } else {
                    0
                }
            } else if a >= h(4, 0xc80) && a < h(4, 0xc80) + 0x28 * 80 {
                0xff
            } else {
                0
            }
        };
        s.apply(&Blit::Tile { tile: 1, dst: 2 }, &chip);
        assert_eq!(&s.bytes[2..4], &[1, 2]);
        assert_eq!(&s.bytes[2 + 0x2e..4 + 0x2e], &[3, 4]);
        assert_eq!(s.bytes[2 + 79 * 0x2e + 1], 160);
        s.apply(&Blit::Bob { image: 0, dst: 2 }, &chip);
        assert_eq!(&s.bytes[2..4], &[0xff, 2]);
        // Far off the screen, nothing lands.
        let before = s.clone();
        s.apply(
            &Blit::Bob {
                image: 0,
                dst: 0x1fff_0000,
            },
            &chip,
        );
        assert_eq!(s, before);
    }
}
