//! `hiscore_name_entry` (h0+$2bba): after a game whose score makes the high-score table, the
//! hall of talent's picture shows CONGRATULATIONS, the rank the score earns and a name typed on
//! the keyboard (eight places, a flashing cursor sprite); fire, Return or Escape end it, and the
//! score goes into `hiscore_table` with the name. In attract mode it returns at once (its caller,
//! [`super::attract::HiscoreCredits`], does not call it then).

use super::attract::text12;
use super::title::BUFFER_0;
use super::{Cx, Yield, at, sprite, vbl};

/// The cursor's sprite record (h0+$1b160).
const CURSOR: u32 = at(0x1b160);
/// The flash counter (h0+$2d8a) and its on/off toggle (h0+$2d8c).
const FLASH: u32 = at(0x2d8a);
const FLASH_OFF: u32 = at(0x2d8c);
/// The name typed: eight places, a dot where nothing is typed (h0+$2dfc..$2e04).
const NAME: u32 = at(0x2dfc);
const NAME_LAST: u32 = at(0x2e03);
/// The name's line ("NAME: " and the name, h0+$2deb) and where it is drawn.
const NAME_LINE: u32 = at(0x2deb);
const NAME_AT: u32 = BUFFER_0 + 0xc800;
/// The keys the name takes: raw codes at h0+$2e30 (`$ff`-ended), their characters at h0+$2e0a.
const KEY_CODES: u32 = at(0x2e30);
const KEY_CHARS: u32 = at(0x2e0a);
/// `hiscore_table` (h0+$2f90): ten scores, eight bytes apart; then ten names of eight bytes.
const SCORES: u32 = at(0x2f90);
const NAMES: u32 = at(0x2fe0);
const SCORE: u32 = at(0x834);
/// `$2a0e`: the ranks, each a score and its text, the best first, ended by a score of -1.
const RANKS: u32 = at(0x2a0e);

/// `hiscore_name_entry` with attract mode off.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct NameEntry {
    pc: u8,
    /// Where the next character goes (the original's `a0`).
    cur: u32,
    /// The key read at line `$a0`.
    key: u8,
}

impl NameEntry {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    rank(cx);
                    // `$2c26`: the cursor on, the four lines of text, the name's place.
                    sprite::attach(cx.s, CURSOR);
                    for (from, to, inside) in [
                        (0x2d8e, 0xbb80, 0x2c42),
                        (0x2dac, 0xbe00, 0x2c52),
                        (0x2dcb, 0xc080, 0x2c62),
                        (0x2dcd, 0xc580, 0x2c72),
                    ] {
                        text12(cx, at(from), BUFFER_0 + to, at(inside));
                    }
                    self.cur = NAME;
                    cx.s.set_w(FLASH, 0);
                    self.pc = 1;
                    // The text takes most of a frame: the loop goes on where it ends.
                    return Yield::Interrupt;
                }
                // $2c7e: fire ends it; the cursor under the next place, hidden 8 frames in 16.
                1 => {
                    if cx.fire() {
                        self.pc = 9;
                        continue;
                    }
                    cx.s.set_w(CURSOR, ((self.cur - NAME) << 3) as u16 + 0x88);
                    let f = cx.s.w(FLASH).wrapping_add(1);
                    if (f as i16) < 8 {
                        cx.s.set_w(FLASH, f);
                    } else {
                        cx.s.set_w(FLASH, 0);
                        cx.s.set_w(FLASH_OFF, !cx.s.w(FLASH_OFF));
                    }
                    if cx.s.w(FLASH_OFF) != 0 {
                        cx.s.set_w(CURSOR, 0x150);
                    }
                    sprite::update(cx.s, CURSOR);
                    self.pc = 2;
                    return Yield::Line(0xa0);
                }
                // At line $a0: a key pressed is taken, then the frame's end waited for.
                2 => {
                    self.key = cx.s.b(vbl::KEY_DOWN);
                    if self.key == 0 {
                        self.pc = 4;
                        continue;
                    }
                    cx.s.set_b(vbl::KEY_DOWN, 0);
                    self.pc = 3;
                    return Yield::Raster;
                }
                3 => {
                    self.pc = 4;
                    let key = self.key;
                    match char_of(cx, key) {
                        Some(c) => {
                            cx.s.set_b(self.cur, c);
                            if self.cur != NAME_LAST {
                                self.cur += 1;
                            }
                        }
                        // Space: a dot, and on, short of the last place.
                        None if key == 0x40 && self.cur < NAME_LAST => {
                            cx.s.set_b(self.cur, b'.');
                            self.cur += 1;
                        }
                        // Escape, Return.
                        None if key == 0x45 || key == 0x44 => self.pc = 9,
                        // Backspace: this place and the one before cleared, back to it.
                        None if key == 0x41 => {
                            cx.s.set_b(self.cur, b'.');
                            if self.cur != NAME {
                                self.cur -= 1;
                                cx.s.set_b(self.cur, b'.');
                            }
                        }
                        None => {}
                    }
                }
                // $2d5c: the name's line drawn again.
                4 => {
                    text12(cx, NAME_LINE, NAME_AT, at(0x2d70));
                    self.pc = 1;
                    return Yield::Interrupt;
                }
                _ => {
                    cx.s.set_w(CURSOR, 0x150);
                    sprite::update(cx.s, CURSOR);
                    enter(cx);
                    return Yield::Return;
                }
            }
        }
    }
}

/// The character a raw key code types into the name, if any.
fn char_of(cx: &Cx, key: u8) -> Option<u8> {
    let mut k = 0;
    loop {
        let code = cx.b(KEY_CODES + k);
        if code == key {
            return Some(cx.b(KEY_CHARS + k));
        }
        k += 1;
        if cx.b(KEY_CODES + k) == 0xff {
            return None;
        }
    }
}

/// `$29c2`: "RANKING:" and the rank of the score: the text of the first rank whose score it
/// reaches.
fn rank(cx: &mut Cx) {
    text12(cx, at(0x2a04), BUFFER_0 + 0xcd0a, at(0x29d2));
    let score = cx.s.l(SCORE);
    let mut e = RANKS;
    let mut text = cx.l(e + 4);
    loop {
        let min = cx.l(e);
        if min == 0xffff_ffff {
            break;
        }
        text = cx.l(e + 4);
        if (score.wrapping_sub(min) as i32) >= 0 {
            break;
        }
        e += 8;
    }
    text12(cx, text, BUFFER_0 + 0xcd13, at(0x2a02));
}

/// The rest of `hiscore_name_entry` (h0+$2bcc): the score into the table in its place, the
/// scores and names below it moved down one (the last falls off), the name with its dots as
/// spaces; the score cleared.
fn enter(cx: &mut Cx) {
    let score = cx.s.l(SCORE);
    cx.s.set_l(SCORE, 0);
    let mut k = 9u32;
    while k > 0 {
        let above = cx.s.l(SCORES + 8 * (k - 1));
        if (score.wrapping_sub(above) as i32) < 0 {
            break;
        }
        cx.s.set_l(SCORES + 8 * k, above);
        let name = cx.s.read(NAMES + 8 * (k - 1), 8);
        cx.s.write(NAMES + 8 * k, &name);
        k -= 1;
    }
    cx.s.set_l(SCORES + 8 * k, score);
    for i in 0..8 {
        let c = cx.s.b(NAME + i);
        cx.s.set_b(NAMES + 8 * k + i, if c == b'.' { b' ' } else { c });
    }
}
