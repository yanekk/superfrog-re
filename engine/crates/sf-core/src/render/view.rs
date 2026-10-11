//! What the renderer hands out: the [`Picture`], the [`View`] it was drawn for, and its 12-bit
//! colours (DESIGN §2.7, §3.3).

/// Presentation choices that change the picture and nothing else (DESIGN §2.7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct View {
    /// Extra lores columns of the level drawn on each side for widescreen; 0 for 4:3.
    pub extra_columns_each_side: u16,
}

/// A frame as Denise would show it: palette indices inside the display window, and each line's
/// 32-colour palette as the copper leaves it at the window's left edge. `left` and `top` are the
/// window's position in DIWSTRT's units (lores pixel, raster line), as the oracle reports them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Picture {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
    /// Row-major, `width * height` palette indices; 32..=63 are extra-half-brite.
    pub pixels: Vec<u8>,
    /// One palette per line, 12-bit OCS colour `0x0RGB`.
    pub palettes: Vec<[u16; 32]>,
    /// Colour registers the copper rewrites inside a line, in order of `(y, x)`: from pixel `x`
    /// of line `y` on, colour `index` is `colour` (docs/12 "Copper timing").
    pub splits: Vec<Split>,
}

/// A colour register written while its line is being drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Split {
    pub y: u16,
    pub x: u16,
    pub index: u8,
    pub colour: u16,
}

/// The PAL display window the original opens on its title screen: DIWSTRT `$2C81`, 320×256.
const DEFAULT_WINDOW: (u16, u16, u16, u16) = (0x81, 0x2C, 320, 256);

impl Picture {
    /// A window of `width` × `height` at (`left`, `top`), every pixel colour 0, every colour
    /// black.
    pub fn blank(left: u16, top: u16, width: u16, height: u16) -> Picture {
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

    /// The default window, widened for `view`, every pixel colour 0, every colour black.
    pub fn black(view: View) -> Picture {
        let (left, top, width, height) = DEFAULT_WINDOW;
        let n = view.extra_columns_each_side;
        Picture::blank(left - n, top, width + 2 * n, height)
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
    fn widescreen_widens_the_window_by_the_extra_columns() {
        let p = Picture::black(View {
            extra_columns_each_side: 32,
        });
        assert_eq!((p.left, p.width), (0x81 - 32, 384));
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
