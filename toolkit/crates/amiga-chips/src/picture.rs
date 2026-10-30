//! The picture a frame shows: what Denise draws, as palette indices and each line's palette, and
//! its 12-bit colours. A game's renderer draws one; an emulator's frame is compared with it.

/// A frame as Denise would show it: palette indices inside the display window, and each line's
/// 32-colour palette as the copper leaves it at the window's left edge. `left` and `top` are the
/// window's position in DIWSTRT's units (lores pixel, raster line), as an emulator reports them;
/// a widescreen window can start left of the hardware's position 0, so `left` is signed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Picture {
    pub left: i32,
    pub top: u16,
    pub width: u16,
    pub height: u16,
    /// Row-major, `width * height` palette indices; 32..=63 are extra-half-brite.
    pub pixels: Vec<u8>,
    /// One palette per line, 12-bit OCS colour `0x0RGB`.
    pub palettes: Vec<[u16; 32]>,
    /// Colour registers the copper rewrites inside a line, in order of `(y, x)`: from pixel `x`
    /// of line `y` on, colour `index` is `colour` (a game's display notes, "Copper timing").
    pub splits: Vec<Split>,
}

/// A colour register written while its line is being drawn.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Split {
    pub y: u16,
    pub x: u16,
    pub index: u8,
    pub colour: u16,
}

impl Picture {
    /// A window of `width` × `height` at (`left`, `top`), every pixel colour 0, every colour
    /// black.
    pub fn blank(left: i32, top: u16, width: u16, height: u16) -> Picture {
        Picture {
            left,
            top,
            width,
            height,
            pixels: vec![0; width as usize * height as usize],
            palettes: vec![[0; 32]; height as usize],
            splits: Vec::new(),
        }
    }

    /// This picture seen through another display window: what lies outside this one shows each
    /// line's colour 0 (the border), and lines outside it show the first or last line's palette.
    pub fn rewindow(&self, left: i32, top: u16, width: u16, height: u16) -> Picture {
        let mut p = Picture::blank(left, top, width, height);
        for y in 0..height as i32 {
            let sy = y + top as i32 - self.top as i32;
            let src = sy.clamp(0, self.height as i32 - 1).max(0) as usize;
            if self.height > 0 {
                p.palettes[y as usize] = self.palettes[src];
            }
            if sy < 0 || sy >= self.height as i32 {
                continue;
            }
            for x in 0..width as i32 {
                let sx = x + left - self.left;
                if (0..self.width as i32).contains(&sx) {
                    p.pixels[(y * width as i32 + x) as usize] =
                        self.pixels[sy as usize * self.width as usize + sx as usize];
                }
            }
        }
        p.splits = self
            .splits
            .iter()
            .filter_map(|s| {
                let y = s.y as i32 + self.top as i32 - top as i32;
                let x = s.x as i32 + self.left - left;
                ((0..height as i32).contains(&y) && (0..width as i32).contains(&x)).then_some(
                    Split {
                        y: y as u16,
                        x: x as u16,
                        ..*s
                    },
                )
            })
            .collect();
        p
    }

    /// The 12-bit colour shown at (`x`, `y`).
    pub fn rgb12(&self, x: u16, y: u16) -> u16 {
        let i = self.pixels[y as usize * self.width as usize + x as usize];
        let mut pal = self.palettes[y as usize];
        for s in self.splits.iter().filter(|s| s.y == y && s.x <= x) {
            pal[s.index as usize] = s.colour;
        }
        colour(&pal, i)
    }
}

fn colour(pal: &[u16; 32], i: u8) -> u16 {
    if i < 32 {
        pal[i as usize]
    } else {
        pal[(i & 31) as usize] >> 1 & 0x777
    }
}

/// Every pixel's 12-bit colour, row-major: what the window shows.
pub fn to_rgb12(p: &Picture) -> Vec<u16> {
    let (w, h) = (p.width as usize, p.height as usize);
    let mut out = Vec::with_capacity(w * h);
    let mut splits = p.splits.iter().peekable();
    for y in 0..h {
        let mut pal = p.palettes[y];
        for x in 0..w {
            while let Some(s) = splits.next_if(|s| (s.y as usize, s.x as usize) <= (y, x)) {
                if s.y as usize == y {
                    pal[s.index as usize] = s.colour;
                }
            }
            out.push(colour(&pal, p.pixels[y * w + x]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_half_brite_halves_the_colour() {
        let mut p = Picture::blank(0x81, 0x2c, 320, 256);
        p.palettes[0][3] = 0xFE8;
        p.pixels[0] = 35;
        p.pixels[1] = 3;
        assert_eq!((p.rgb12(0, 0), p.rgb12(1, 0)), (0x774, 0xFE8));
    }

    #[test]
    fn rewindow_shows_the_border_outside_and_moves_the_splits() {
        let mut p = Picture::blank(10, 20, 2, 2);
        p.palettes = vec![[0x111; 32], [0x222; 32]];
        p.pixels = vec![1, 2, 3, 4];
        p.splits = vec![Split {
            y: 1,
            x: 1,
            index: 0,
            colour: 0xF00,
        }];
        let q = p.rewindow(9, 19, 4, 4);
        assert_eq!(q.pixels, [0, 0, 0, 0, 0, 1, 2, 0, 0, 3, 4, 0, 0, 0, 0, 0]);
        assert_eq!(q.palettes[0], [0x111; 32]);
        assert_eq!(q.palettes[3], [0x222; 32]);
        assert_eq!(
            q.splits,
            [Split {
                y: 2,
                x: 2,
                index: 0,
                colour: 0xF00
            }]
        );
        assert_eq!(q.rewindow(10, 20, 2, 2), p);
    }

    #[test]
    fn a_split_recolours_its_line_from_its_pixel_on() {
        let mut p = Picture::blank(0, 0, 4, 2);
        p.palettes = vec![[0x111; 32]; 2];
        p.splits = vec![Split {
            y: 0,
            x: 2,
            index: 0,
            colour: 0xF00,
        }];
        let rgb = to_rgb12(&p);
        assert_eq!(
            rgb,
            [0x111, 0x111, 0xF00, 0xF00, 0x111, 0x111, 0x111, 0x111]
        );
        assert_eq!(
            (p.rgb12(1, 0), p.rgb12(3, 0), p.rgb12(3, 1)),
            (0x111, 0xF00, 0x111)
        );
    }
}
