//! Level codes (docs/13 "Passwords"): `password_table` (h0+$24dc) and the entry on the options
//! screen (`password_entry` h0+$22ba).
//!
//! There is no algorithm: a code is one of the table's 28 six-digit entries, each with a routine
//! that sets the world and level the game starts at (`start_world`, `start_level`). Entries 0..23
//! start W1-1..W6-4, 24..27 the bonus level. The digits are the player's disks' (DESIGN §2.3), so
//! these functions take the table as the disks give it.

use super::menu::CURSOR;
use super::title::{BUFFER_0, START_LEVEL, START_WORLD, eyes_init};
use super::{Cx, Yield, at, h2, sprite, vbl};

/// Where the game is in the run: what a level code stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Progress {
    /// 1..6, 7 the bonus level.
    pub world: u8,
    /// 1..4.
    pub level: u8,
}

/// A level code: six ASCII digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Password(pub [u8; 6]);

/// One entry of the game's table: the digits and where its routine starts the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeEntry {
    pub password: Password,
    pub start: Progress,
}

/// The table as the game holds it: each entry's digits, and its routine read for the two values it
/// sets (`move.l #world,start_world; move.l #level,start_level; rts`).
pub fn table(data: &super::FrontData) -> Vec<CodeEntry> {
    (0..28)
        .filter_map(|k| {
            let e = 0x24dc + 10 * k;
            let digits: Vec<u8> = (0..6).map(|i| data.h0_b(e + i)).collect();
            let handler = data.h0_l(e + 6).checked_sub(crate::layout::H0)?;
            let world = data.h0_l(handler + 2);
            let level = data.h0_l(handler + 12);
            (data.h0_w(handler) == 0x23fc && data.h0_w(handler + 10) == 0x23fc).then(|| CodeEntry {
                password: Password(digits.try_into().expect("six")),
                start: Progress {
                    world: world as u8,
                    level: level as u8,
                },
            })
        })
        .collect()
}

/// The code that starts the game at `progress`: the first entry of the table that does.
pub fn password_for(table: &[CodeEntry], progress: &Progress) -> Option<Password> {
    table
        .iter()
        .find(|e| e.start == *progress)
        .map(|e| e.password)
}

/// Where a code starts the game, as `password_check` (h0+$2432) finds it: the first entry whose
/// six digits match; `None` for a code the game does not know (it starts W1-1 then).
pub fn decode_password(table: &[CodeEntry], p: &Password) -> Option<Progress> {
    table.iter().find(|e| e.password == *p).map(|e| e.start)
}

const BUF: u32 = at(0x242a);
const BUF_END: u32 = at(0x2430);
const BUF_AT: u32 = at(0x2426);
const FLASH: u32 = at(0x2424);
/// The code's six copper colours (h2+$737e, 16 MOVEs).
const CODE_COLOURS: u32 = h2(0x737e);

/// `$2412`: the typed code drawn into the options picture (plane 0, h0+$30fc's 8 × 8 font). It
/// runs late in the frame and can run past its end: its memory is compared once the original's
/// call (return address `inside`) is over.
pub(super) fn draw_code(cx: &mut Cx, inside: u32) {
    cx.jobs.push(crate::game::Job {
        inside,
        writes: vec![(BUFFER_0 + 0x127f, 8 * 0x28), (at(0x3258), 4)],
    });
    text(cx, BUF, BUFFER_0 + 0x127f);
}

/// `$30fc`: write the text at `from` (a negative byte ends it) into plane 0 at `to`, a byte a
/// character, eight lines `$28` apart; a space clears, a character the font lacks ends it.
pub(super) fn text(cx: &mut Cx, from: u32, mut to: u32) {
    let mut a = from;
    // About how long it takes, in CPU cycles: the charset searched a byte at a time, eight byte
    // moves a character.
    let mut cycles = 40u32;
    loop {
        cycles += 30;
        let c = cx.b(a);
        a += 1;
        if c & 0x80 != 0 {
            break;
        }
        if c == 0x20 {
            for r in 0..8 {
                cx.s.set_b(to + 0x28 * r, 0);
            }
            cycles += 8 * 20;
            to += 1;
            continue;
        }
        // The font's charset (h0+$322e), zero-ended; a character's glyph is its position less two.
        let mut k = 0u32;
        let found = loop {
            let d = cx.b(at(0x322e) + k);
            k += 1;
            if d == 0 {
                break false;
            }
            if d == c {
                break true;
            }
        };
        cycles += k * 26;
        if !found {
            break;
        }
        let glyph = at(0x378ca).wrapping_add(k).wrapping_sub(2);
        for r in 0..8 {
            let v = cx.b(glyph + 0x28 * r);
            cx.s.set_b(to + 0x28 * r, v);
        }
        cycles += 8 * 28;
        to += 1;
    }
    cx.s.set_l(at(0x3258), a);
    cx.busy(cycles);
}

/// `password_entry` (h0+$22ba): type six digits; Backspace starts again, Escape, Return or fire
/// end it. The code is then checked against the table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Entry {
    pc: u8,
}

impl Entry {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    sprite::attach(cx.s, CURSOR);
                    cx.s.set_w(CURSOR, 0x79);
                    cx.s.set_w(CURSOR + 2, 0x75);
                    sprite::update(cx.s, CURSOR);
                    cx.s.set_l(START_WORLD, 0);
                    cx.s.set_l(START_LEVEL, 0);
                    cx.s.set_l(BUF_AT, BUF);
                    for k in 0..6 {
                        cx.s.set_b(BUF + k, 0x20);
                    }
                    for k in 0..16 {
                        cx.s.set_w(CODE_COLOURS + 4 * k, 0xfff);
                    }
                    draw_code(cx, at(0x233c));
                    cx.s.set_w(FLASH, 0);
                    self.pc = 1;
                }
                1 => {
                    self.pc = 2;
                    return Yield::Raster;
                }
                2 => {
                    // The cursor flashes: hidden 8 frames in 16.
                    let x = cx.s.w(CURSOR);
                    let f = cx.s.w(FLASH).wrapping_add(1);
                    let f = if (f as i16) >= 0x10 { 0 } else { f };
                    cx.s.set_w(FLASH, f);
                    if (f as i16) >= 8 {
                        cx.s.set_w(CURSOR, 0x150);
                    }
                    sprite::update(cx.s, CURSOR);
                    cx.s.set_w(CURSOR, x);
                    let key = cx.s.b(vbl::KEY_DOWN);
                    match key {
                        0x41 => {
                            self.pc = 0;
                            continue;
                        }
                        0x45 | 0x44 => {
                            self.pc = 3;
                            continue;
                        }
                        _ if cx.fire() => {
                            self.pc = 3;
                            continue;
                        }
                        0 => {
                            self.pc = 1;
                            continue;
                        }
                        _ => {}
                    }
                    // The key's character: raw codes at h0+$2e30 ($ff-ended), characters at
                    // h0+$2e0a.
                    let mut k = 0;
                    let ch = loop {
                        let code = cx.b(at(0x2e30) + k);
                        if code == 0xff {
                            break None;
                        }
                        if code == key {
                            break Some(cx.b(at(0x2e0a) + k));
                        }
                        k += 1;
                    };
                    let Some(ch) = ch else {
                        self.pc = 1;
                        continue;
                    };
                    cx.s.set_b(vbl::KEY_DOWN, 0);
                    cx.s.set_w(CURSOR, cx.s.w(CURSOR).wrapping_add(8));
                    let at_ = cx.s.l(BUF_AT);
                    cx.s.set_b(at_, ch);
                    if at_ + 1 == BUF_END {
                        self.pc = 3;
                        continue;
                    }
                    cx.s.set_l(BUF_AT, at_ + 1);
                    draw_code(cx, at(0x23ea));
                    self.pc = 1;
                    return Yield::Interrupt;
                }
                3 => {
                    draw_code(cx, at(0x23f2));
                    self.pc = 4;
                    return Yield::Interrupt;
                }
                _ => {
                    check(cx);
                    for k in 0..16 {
                        cx.s.set_w(CODE_COLOURS + 4 * k, 0);
                    }
                    eyes_init(cx);
                    return Yield::Return;
                }
            }
        }
    }
}

/// `password_check` (h0+$2432): a code in the table sets where the game starts.
fn check(cx: &mut Cx) {
    let typed: Vec<u8> = (0..6).map(|k| cx.s.b(BUF + k)).collect();
    let table = table(cx.data);
    if let Some(e) = table.iter().find(|e| e.password.0[..] == typed[..]) {
        cx.s.set_l(START_WORLD, e.start.world as u32);
        cx.s.set_l(START_LEVEL, e.start.level as u32);
    }
}
