//! What the renderer hands out: the [`Picture`] (the toolkit's, `amiga_chips::picture`), the
//! [`View`] it was drawn for, and its 12-bit colours (DESIGN §2.7, §3.3).

pub use amiga_chips::picture::{Picture, Split, to_rgb12};

/// Presentation choices that change the picture and nothing else (DESIGN §2.7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct View {
    /// Extra lores columns of the level drawn on each side for widescreen; 0 for 4:3.
    pub extra_columns_each_side: u16,
}

/// The PAL display window the original opens on its title screen: DIWSTRT `$2C81`, 320×256.
const DEFAULT_WINDOW: (u16, u16, u16, u16) = (0x81, 0x2C, 320, 256);

/// The picture's widescreen side: the title screen's window and a [`View`]'s extra columns. A trait
/// because [`Picture`] is the toolkit's and widescreen is this game's (toolkit-extract DESIGN §2.2).
pub trait PictureView {
    /// The default window, widened for `view`, every pixel colour 0, every colour black.
    fn black(view: View) -> Picture;

    /// This picture widened by `view`'s extra columns on each side, which show each line's colour
    /// 0 (the border outside the display window) where the screen is not a level's.
    fn widened(&self, view: View) -> Picture;

    /// The 4:3 picture this one was widened from by `view`: its centre.
    fn narrowed(&self, view: View) -> Picture;
}

impl PictureView for Picture {
    fn black(view: View) -> Picture {
        let (left, top, width, height) = DEFAULT_WINDOW;
        let n = view.extra_columns_each_side;
        Picture::blank(left as i32 - n as i32, top, width + 2 * n, height)
    }

    fn widened(&self, view: View) -> Picture {
        let n = view.extra_columns_each_side as usize;
        if n == 0 {
            return self.clone();
        }
        let (w, h) = (self.width as usize, self.height as usize);
        let mut pixels = vec![0u8; (w + 2 * n) * h];
        for y in 0..h {
            pixels[y * (w + 2 * n) + n..y * (w + 2 * n) + n + w]
                .copy_from_slice(&self.pixels[y * w..(y + 1) * w]);
        }
        Picture {
            left: self.left - n as i32,
            top: self.top,
            width: (w + 2 * n) as u16,
            height: self.height,
            pixels,
            palettes: self.palettes.clone(),
            splits: self
                .splits
                .iter()
                .map(|s| Split {
                    x: s.x + n as u16,
                    ..*s
                })
                .collect(),
        }
    }

    fn narrowed(&self, view: View) -> Picture {
        let n = view.extra_columns_each_side;
        if n == 0 {
            return self.clone();
        }
        self.rewindow(self.left + n as i32, self.top, self.width - 2 * n, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widescreen_widens_the_window_by_the_extra_columns() {
        let p = Picture::black(View {
            extra_columns_each_side: 32,
        });
        assert_eq!((p.left, p.width), (0x81 - 32, 384));
    }

    #[test]
    fn a_window_wider_than_the_hardware_s_left_edge_starts_below_0() {
        let p = Picture::black(View {
            extra_columns_each_side: 150,
        });
        assert_eq!((p.left, p.width), (0x81 - 150, 620));
    }

    #[test]
    fn narrowed_undoes_widened() {
        let mut p = Picture::blank(0x81, 0x2c, 4, 2);
        p.palettes = vec![[0x123; 32], [0x456; 32]];
        p.pixels = vec![1, 2, 3, 4, 5, 6, 7, 8];
        p.splits = vec![Split {
            y: 1,
            x: 2,
            index: 0,
            colour: 0xF00,
        }];
        let view = View {
            extra_columns_each_side: 3,
        };
        assert_eq!(p.widened(view).narrowed(view), p);
    }

    #[test]
    fn extra_half_brite_halves_the_colour() {
        let mut p = Picture::black(View::default());
        p.palettes[0][3] = 0xFE8;
        p.pixels[0] = 35;
        p.pixels[1] = 3;
        assert_eq!((p.rgb12(0, 0), p.rgb12(1, 0)), (0x774, 0xFE8));
    }

    #[test]
    fn picture_save_state_bytes_are_unchanged_by_the_move() {
        // bincode bytes of this Picture written before it moved to amiga_chips (toolkit-extract
        // T01): a save state holds Pictures, so the layout must not change.
        let p = Picture {
            left: -3,
            top: 0x2c,
            width: 2,
            height: 1,
            pixels: vec![1, 35],
            palettes: vec![[0x0FE8; 32]],
            splits: vec![Split { y: 0, x: 1, index: 3, colour: 0xF00 }],
        };
        // left, top, width, height; then each Vec as a u64 length and its items, in field order.
        let mut want = vec![253, 255, 255, 255, 44, 0, 2, 0, 1, 0];
        want.extend([2, 0, 0, 0, 0, 0, 0, 0, 1, 35]);
        want.extend([1, 0, 0, 0, 0, 0, 0, 0]);
        for _ in 0..32 {
            want.extend([232, 15]);
        }
        want.extend([1, 0, 0, 0, 0, 0, 0, 0]);
        want.extend([0, 0, 1, 0, 3, 0, 15]);
        assert_eq!(bincode::serialize(&p).unwrap(), want);
        assert_eq!(bincode::deserialize::<Picture>(&want).unwrap(), p);
    }
}
