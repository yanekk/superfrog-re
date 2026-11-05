//! Hardware sprites as Denise puts them over the play field: the control words' window and
//! position, and which sprite's colour a pixel shows (`sprite_at`). How a game feeds the sprites
//! (copper writes, sprite DMA, a mid-line control write) stays with its renderer.
//!
//! learned on: Superfrog's frog and HUD against the vAmiga oracle (its display notes, "Sprites").
//! learned on: Forest Dumb Forever's character, sprites 0-3 as two attached pairs.

/// Lores pixels between a sprite's HSTART and the window position its first pixel shows at.
pub const SPRITE_DELAY: i32 = 1;

/// What a sprite shows on one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteLine {
    /// HSTART in lores pixels (POS's low byte doubled, CTL's bit 0).
    pub hstart: u16,
    /// DATA and DATB: the line's plane 0 and plane 1 words.
    pub data: [u16; 2],
    /// CTL's attach bit; read on the odd sprite of a pair.
    pub attached: bool,
    /// A control write in the middle of the line turns it off from this picture pixel on.
    pub cut: Option<i32>,
}

/// HSTART from the control words, in lores pixels.
pub fn hstart(pos: u16, ctl: u16) -> u16 {
    (pos & 0xFF) << 1 | ctl & 1
}

/// The first line a sprite shows and the line it stops on (its next control words are fetched
/// there), from the control words: VSTART's and VSTOP's ninth bits are in CTL.
pub fn lines(pos: u16, ctl: u16) -> (u16, u16) {
    ((ctl & 4) << 6 | pos >> 8, (ctl & 2) << 7 | ctl >> 8)
}

/// The sprites' colour at picture pixel `x` (window position `at`), and the pair that gives it:
/// the lower pair in front of the higher, an attached pair as one 16-colour sprite (colours 16 to
/// 31), an unattached sprite its pair's three colours.
pub fn sprite_at(sprites: &[Option<SpriteLine>; 8], at: i32, x: i32) -> Option<(u8, usize)> {
    let bits = |n: usize| {
        let s = sprites[n]?;
        let i = at - s.hstart as i32 - SPRITE_DELAY;
        if !(0..16).contains(&i) || s.cut.is_some_and(|c| x >= c) {
            return None;
        }
        let b = 15 - i;
        Some((s.data[0] >> b & 1 | (s.data[1] >> b & 1) << 1) as u8)
    };
    for pair in 0..4 {
        let (even, odd) = (bits(2 * pair), bits(2 * pair + 1));
        let colour = if sprites[2 * pair + 1].is_some_and(|s| s.attached) {
            let v = even.unwrap_or(0) | odd.unwrap_or(0) << 2;
            (v != 0).then_some(16 + v)
        } else {
            [even, odd]
                .into_iter()
                .flatten()
                .find(|&v| v != 0)
                .map(|v| 16 + 4 * pair as u8 + v)
        };
        if let Some(c) = colour {
            return Some((c, pair));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(hstart: u16, data: [u16; 2], attached: bool) -> Option<SpriteLine> {
        Some(SpriteLine {
            hstart,
            data,
            attached,
            cut: None,
        })
    }

    #[test]
    fn the_control_words_give_the_window_and_the_position() {
        assert_eq!(lines(0x2681, 0x3980), (0x26, 0x39));
        assert_eq!(lines(0x2681, 0x0506), (0x126, 0x105));
        assert_eq!(hstart(0x2681, 0x3981), 0x103);
    }

    #[test]
    fn a_pixel_shows_one_after_hstart_and_sixteen_wide() {
        let mut s = [None; 8];
        s[0] = line(0x100, [0x8001, 0], false);
        assert_eq!(sprite_at(&s, 0x100, 0), None);
        assert_eq!(sprite_at(&s, 0x101, 0), Some((17, 0)));
        assert_eq!(sprite_at(&s, 0x110, 0), Some((17, 0)));
        assert_eq!(sprite_at(&s, 0x111, 0), None);
    }

    #[test]
    fn an_attached_pair_is_one_sixteen_colour_sprite_and_lower_pairs_win() {
        let mut s = [None; 8];
        s[2] = line(0x100, [0x8000, 0x8000], false);
        s[3] = line(0x100, [0x8000, 0], true);
        assert_eq!(sprite_at(&s, 0x101, 0), Some((16 + 0b0111, 1)));
        s[0] = line(0x100, [0, 0x8000], false);
        assert_eq!(sprite_at(&s, 0x101, 0), Some((18, 0)));
        // Unattached, the odd sprite shows its own pair's colours where the even one is clear.
        s[0] = None;
        s[2] = None;
        s[3] = line(0x100, [0x8000, 0], false);
        assert_eq!(sprite_at(&s, 0x101, 0), Some((16 + 4 + 1, 1)));
    }

    #[test]
    fn a_cut_ends_the_sprite_from_its_pixel() {
        let mut s = [None; 8];
        s[0] = Some(SpriteLine {
            cut: Some(5),
            ..line(0x100, [0xffff, 0], false).unwrap()
        });
        assert!(sprite_at(&s, 0x105, 4).is_some());
        assert!(sprite_at(&s, 0x106, 5).is_none());
    }
}
