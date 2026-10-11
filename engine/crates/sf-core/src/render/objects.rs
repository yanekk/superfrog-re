//! Hardware sprites, and the picture's composition (docs/12 "Sprites").
//!
//! Bobs need nothing here: the original blits them into the display buffers, which are state,
//! so they arrive with the play field's fetched lines.
//!
//! Sprites follow Agnus's sprite DMA (vAmiga `Agnus::executeFirstSpriteCycle`,
//! `updateSpriteDMA`): positions and pointers from the copper, data words fetched per line between
//! the vertical start and stop, the next control words fetched at the stop line.

use super::copper::reg;
use super::playfield::Field;
use super::{Mem, Picture, Row, Under};

/// One hardware sprite's channel: Agnus's pointer and vertical window, Denise's position and
/// data.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Sprite {
    pt: u32,
    vstart: u16,
    vstop: u16,
    dma: bool,
    pos: u16,
    ctl: u16,
    armed: bool,
    data: [u16; 2],
}

/// What a sprite shows on one line.
#[derive(Debug, Clone, Copy)]
pub(super) struct SpriteLine {
    pub hstart: u16,
    pub data: [u16; 2],
    pub attached: bool,
    /// A control write in the middle of the line turns it off from this picture pixel on.
    pub cut: Option<i32>,
}

/// The sprite a register belongs to.
pub(super) fn sprite_of(r: u16) -> usize {
    if r < reg::SPR0POS {
        ((r - reg::SPR0PTH) / 4) as usize
    } else {
        ((r - reg::SPR0POS) / 8) as usize
    }
}

/// The line on which every sprite fetches its control words afresh (PAL).
pub(super) const RESET_LINE: u16 = 25;

impl Sprite {
    /// A write to one of the sprite's registers on `line`; true when it turns the sprite off (a
    /// control write).
    pub fn write(&mut self, r: u16, value: u16, line: u16) -> bool {
        if r < reg::SPR0POS {
            self.pt = if r & 2 == 0 {
                (value as u32 & 0x1F) << 16 | self.pt & 0xFFFF
            } else {
                self.pt & 0xFFFF_0000 | value as u32 & 0xFFFE
            };
            return false;
        }
        match (r - reg::SPR0POS) % 8 {
            0 => self.set_pos(value, line),
            2 => {
                self.set_ctl(value, line);
                self.armed = false;
                return true;
            }
            4 => {
                self.data[0] = value;
                self.armed = true;
            }
            _ => self.data[1] = value,
        }
        false
    }

    fn set_pos(&mut self, value: u16, line: u16) {
        self.pos = value;
        self.vstart = value >> 8 | self.vstart & 0x100;
        self.window_at(line);
    }

    fn set_ctl(&mut self, value: u16, line: u16) {
        self.ctl = value;
        self.vstart = (value & 4) << 6 | self.vstart & 0xFF;
        self.vstop = (value & 2) << 7 | value >> 8;
        self.window_at(line);
    }

    fn window_at(&mut self, line: u16) {
        if line >= RESET_LINE {
            if self.vstart == line {
                self.dma = true;
            }
            if self.vstop == line {
                self.dma = false;
            }
        }
    }

    /// The line's sprite DMA: at the stop line the next control words, inside the window a line
    /// of data. Nothing when the bitplane fetch has taken the slot (`blocked`).
    pub fn dma(&mut self, line: u16, mem: &Mem, blocked: bool) {
        if line == self.vstop {
            self.dma = false;
            if !blocked {
                let (pos, ctl) = (mem.word(self.pt), mem.word(self.pt + 2));
                self.pt += 4;
                self.set_pos(pos, line);
                self.set_ctl(ctl, line);
                self.armed = false;
            }
        } else if self.dma && !blocked {
            self.data = [mem.word(self.pt), mem.word(self.pt + 2)];
            self.pt += 4;
            self.armed = true;
        }
    }

    /// The window opens and closes as the next line begins (vAmiga `updateSpriteDMA`); on the
    /// reset line every sprite fetches its control words.
    pub fn end_of_line(&mut self, next: u16, last: u16) {
        if next == RESET_LINE {
            self.vstop = RESET_LINE;
        } else if next == last {
            self.dma = false;
        } else if next > RESET_LINE {
            self.window_at(next);
        }
    }

    pub fn shown(&self) -> Option<SpriteLine> {
        self.armed.then_some(SpriteLine {
            hstart: (self.pos & 0xFF) << 1 | self.ctl & 1,
            data: self.data,
            attached: self.ctl & 0x80 != 0,
            cut: None,
        })
    }
}

/// Lores pixels between a sprite's HSTART and the window position its first pixel shows at.
const SPRITE_DELAY: i32 = 1;

/// The picture's pixels: each line's fetched bitplane indices (on a play-field line, outside the
/// original's window — widescreen's extra columns — the level from the map), then the sprites, a pair in front of the play field when its number is below the play field's
/// priority, the lower pair in front of the higher, an attached pair as one 16-colour sprite.
pub(super) fn compose(pic: &mut Picture, rows: &[Row], field: &Field, extra: usize) {
    let width = pic.width as usize;
    for (y, row) in rows.iter().enumerate() {
        let line = &mut pic.pixels[y * width..(y + 1) * width];
        match row.under {
            Under::Nothing => line.fill(0),
            Under::Fetched => {}
            Under::Field { y: ly, x0 } => {
                for (x, px) in line.iter_mut().enumerate() {
                    if x < extra || x >= width - extra {
                        *px = field.at(x0 + x as i32, ly);
                    }
                }
            }
        }
        if row.sprites.iter().all(Option::is_none) {
            continue;
        }
        for (x, px) in line.iter_mut().enumerate() {
            let at = pic.left as i32 + x as i32;
            let bits = |n: usize| {
                let s = row.sprites[n]?;
                let i = at - s.hstart as i32 - SPRITE_DELAY;
                if !(0..16).contains(&i) || s.cut.is_some_and(|c| x as i32 >= c) {
                    return None;
                }
                let b = 15 - i;
                Some((s.data[0] >> b & 1 | (s.data[1] >> b & 1) << 1) as u8)
            };
            for pair in 0..4 {
                let (even, odd) = (bits(2 * pair), bits(2 * pair + 1));
                let colour = if row.sprites[2 * pair + 1].is_some_and(|s| s.attached) {
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
                    if pair < row.priority as usize || *px == 0 {
                        *px = c;
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_registers_and_their_numbers() {
        assert_eq!(sprite_of(0x120), 0);
        assert_eq!(sprite_of(0x13e), 7);
        assert_eq!(sprite_of(0x140), 0);
        assert_eq!(sprite_of(0x14a), 1);
        assert_eq!(sprite_of(0x17e), 7);
        let mut s = Sprite::default();
        s.write(0x140, 0x2681, 0x20);
        assert!(s.write(0x142, 0x3981, 0x20));
        assert_eq!((s.vstart, s.vstop, s.ctl & 0x80), (0x26, 0x39, 0x80));
        s.write(0x142, 0x0506, 0x20);
        assert_eq!((s.vstart, s.vstop), (0x126, 0x105));
    }
}
