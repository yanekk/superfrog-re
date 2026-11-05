//! The character's hardware sprites (plans/forest-dumb-level-1 T10; docs/09-play-measured.md, "Sprites,
//! bobs, playfields").
//!
//! The character is sprites 0 to 3: two attached pairs side by side, 32 pixels wide, 40 lines, 15
//! colours. Their control words and data sit in hunk 1 from [`SPRITES`], [`SPRITE_STRIDE`] bytes a
//! sprite, and the copper list points sprite DMA at them. Each pass `char_sprite_copy` copies the
//! frame at `char_anim_ptr` into their data, or clears it while the character blinks after a hit, and
//! `char_sprite_position` writes their control words from the character's place in the view.
//! Ported from `char_sprite_copy` (`h0+$9b06`) and `char_sprite_position` (`h0+$5462`), each proven
//! on the routine rig. Enemies and items are bobs blitted into the play screen (`playfield.rs`).
//!
//! Interruptible points (porting rule 1): none; both run inside the level-3 interrupt.
//!
//! What the sprites show is picture state; `char_unhurt_t`, which the copy counts, is game state
//! (`character::Motion`).

/// Hunk 1 offset of sprite 0's control words; its data follows.
pub const SPRITES: u32 = 0xd30;
/// Bytes from one sprite to the next: control words, 40 lines of two words, end words.
pub const SPRITE_STRIDE: u32 = 0xa8;
/// A sprite's data bytes: 40 lines of two words.
pub const SPRITE_DATA: u32 = 0xa0;

/// What the character's sprites hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharSprite {
    /// Sprite 0's and 1's control words as a long (`h1+$d30`, `$dd8`); sprites 2 and 3 have it plus
    /// `$80000`, 16 pixels right.
    pub ctl: u32,
    /// The frame whose data the sprites hold: the address `char_anim_ptr` held when it was copied,
    /// or `None` when the blink cleared them.
    pub frame: Option<u32>,
}

impl CharSprite {
    /// `char_sprite_copy`: copy the frame at `anim_ptr`, unless the character blinks. While
    /// `char_unhurt_t` is not 0 it counts up a pass, wrapping to 0, and the sprites are cleared on
    /// the passes it reaches 2 and 3 modulo 4.
    pub fn copy(&mut self, unhurt_t: &mut u8, anim_ptr: u32) {
        if *unhurt_t != 0 {
            *unhurt_t = unhurt_t.wrapping_add(1);
            if *unhurt_t & 3 >= 2 {
                self.frame = None;
                return;
            }
        }
        self.frame = Some(anim_ptr);
    }

    /// `char_sprite_position`: the control words from the character's place less the camera's.
    pub fn position(&mut self, char_x: u32, char_y: u32, camera_x: u16, camera_y: u16) {
        self.ctl = control(char_x, char_y, camera_x, camera_y);
    }

    /// The sprites' bytes in hunk 1 as the copy and position leave them, as (hunk 1 offset, bytes),
    /// the frame's read through `frame(address, len)`. Writes the original makes, in its order.
    pub fn bytes(&self, frame: &impl Fn(u32, u32) -> Vec<u8>) -> Vec<(u32, Vec<u8>)> {
        let mut out = Vec::new();
        for n in 0..4u32 {
            let at = SPRITES + 4 + n * SPRITE_STRIDE;
            let data = match self.frame {
                Some(p) => frame(p.wrapping_add(n * SPRITE_DATA), SPRITE_DATA),
                None => vec![0; SPRITE_DATA as usize],
            };
            out.push((at, data));
        }
        for n in 0..4u32 {
            let ctl = if n < 2 {
                self.ctl
            } else {
                self.ctl.wrapping_add(0x8_0000)
            };
            out.push((SPRITES + n * SPRITE_STRIDE, ctl.to_be_bytes().to_vec()));
        }
        out
    }
}

/// The control words (SPRxPOS in the high word, SPRxCTL in the low) the original computes for a
/// character at (`char_x`, `char_y`) in a view at (`camera_x`, `camera_y`), with its widths: the
/// differences are words in longs, so what the high words hold reaches the result as it does there.
pub fn control(char_x: u32, char_y: u32, camera_x: u16, camera_y: u16) -> u32 {
    let low = |v: u32, w: u16| v & 0xffff_0000 | (v as u16).wrapping_sub(w) as u32;
    let x = low(char_x, camera_x);
    let hbit = x & 1;
    let x = (x >> 1).wrapping_add(0x3c).rotate_left(16);
    let y = low(char_y, camera_y).wrapping_add(0x31);
    let mut d0 = x;
    if y & 0x100 != 0 {
        d0 |= 4;
    }
    let start = y.rotate_left(16) << 8;
    let stop = y.wrapping_add(0x28);
    if stop & 0x100 != 0 {
        d0 |= 2;
    }
    d0 | start | (stop & 0xff) << 8 | 0x80 | hbit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_words_place_the_sprite_and_attach_it() {
        // At the view's top-left: VSTART $31, HSTART $3c << 1, VSTOP $59, attached.
        let c = control(0x10, 0x10, 0x10, 0x10);
        assert_eq!(c, 0x313c_5980);
        assert_eq!(
            amiga_chips::sprite::lines((c >> 16) as u16, c as u16),
            (0x31, 0x59)
        );
        // An odd x sets HSTART's low bit; a low view sets the ninth bits.
        assert_eq!(control(0x11, 0xe0, 0x10, 0x10) & 0x7, 0x7);
    }

    #[test]
    fn the_blink_clears_two_passes_in_four_and_wraps_to_steady() {
        let mut s = CharSprite {
            ctl: 0,
            frame: None,
        };
        let mut t = 0xfcu8;
        let mut shown = Vec::new();
        for _ in 0..6 {
            s.copy(&mut t, 0x1234);
            shown.push(s.frame.is_some());
        }
        // $fd, $fe, $ff clear on $fe and $ff; 0 shows, and then it stays 0.
        assert_eq!(shown, [true, false, false, true, true, true]);
        assert_eq!(t, 0);
    }
}
