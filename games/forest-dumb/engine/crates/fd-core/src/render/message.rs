//! "Level 1 complete" (plans/forest-dumb-level-1 T26, DESIGN §2.5): the app's own message where the
//! original loads world 2, white on black in the play picture's window. Not the original's, so it
//! is never compared; it goes when world 2 is built.
//!
//! The status bar's font has digits only (`render::status`), so the letters are a 5×7 font drawn
//! here, at twice its size.

use amiga_chips::Picture;

/// The message, as drawn.
pub const MESSAGE: &str = "LEVEL 1 COMPLETE";

/// Pixels a font pixel covers, each way.
const SCALE: u16 = 2;
/// A glyph's size in font pixels, and the advance from one to the next.
const GLYPH: (u16, u16) = (5, 7);
const ADVANCE: u16 = 6;

/// The glyph of `c`: seven rows, bit 4 the left column. `None` for a letter the message never uses.
fn glyph(c: char) -> Option<[u8; 7]> {
    Some(match c {
        ' ' => [0; 7],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        _ => return None,
    })
}

/// The message's size in picture pixels.
pub fn text_size() -> (u16, u16) {
    let n = MESSAGE.chars().count() as u16;
    ((n * ADVANCE - (ADVANCE - GLYPH.0)) * SCALE, GLYPH.1 * SCALE)
}

/// The message centred in a black picture with `play`'s window, so the app scales it as it scales
/// play.
pub fn level_complete(play: &Picture) -> Picture {
    let (w, h) = (play.width, play.height);
    let mut pixels = vec![0u8; w as usize * h as usize];
    let (tw, th) = text_size();
    let (x0, y0) = (w.saturating_sub(tw) / 2, h.saturating_sub(th) / 2);
    for (k, c) in MESSAGE.chars().enumerate() {
        let rows = glyph(c).expect("every letter of MESSAGE has a glyph");
        let gx = x0 + k as u16 * ADVANCE * SCALE;
        for (r, bits) in rows.iter().enumerate() {
            for col in 0..GLYPH.0 {
                if bits >> (GLYPH.0 - 1 - col) & 1 == 0 {
                    continue;
                }
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        let (x, y) = (gx + col * SCALE + dx, y0 + r as u16 * SCALE + dy);
                        if x < w && y < h {
                            pixels[y as usize * w as usize + x as usize] = 1;
                        }
                    }
                }
            }
        }
    }
    let mut palette = [0u16; 32];
    palette[1] = 0xfff;
    Picture {
        left: play.left,
        top: play.top,
        width: w,
        height: h,
        pixels,
        palettes: vec![palette; h as usize],
        splits: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(width: u16, height: u16) -> Picture {
        Picture {
            left: 3,
            top: 47,
            width,
            height,
            pixels: vec![0; width as usize * height as usize],
            palettes: vec![[0; 32]; height as usize],
            splits: Vec::new(),
        }
    }

    #[test]
    fn every_letter_has_a_glyph() {
        assert!(MESSAGE.chars().all(|c| glyph(c).is_some()));
        assert_eq!(text_size(), (190, 14));
    }

    #[test]
    fn the_message_is_centred_whole_inside_the_window() {
        let p = level_complete(&blank(320, 250));
        assert_eq!((p.left, p.top, p.width, p.height), (3, 47, 320, 250));
        let lit: Vec<(usize, usize)> = (0..p.pixels.len())
            .filter(|&i| p.pixels[i] == 1)
            .map(|i| (i % 320, i / 320))
            .collect();
        let (x0, x1) = (
            lit.iter().map(|p| p.0).min().unwrap(),
            lit.iter().map(|p| p.0).max().unwrap(),
        );
        let (y0, y1) = (
            lit.iter().map(|p| p.1).min().unwrap(),
            lit.iter().map(|p| p.1).max().unwrap(),
        );
        // The box from L's left column to E's right one, 2 px a font pixel.
        assert_eq!((x0, x1 + 1, y0, y1 + 1), (65, 255, 118, 132));
        assert_eq!(p.palettes[0][1], 0xfff);
    }
}
