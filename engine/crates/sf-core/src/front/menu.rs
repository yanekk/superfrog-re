//! The title's menu (START GAME / OPTIONS, h0+$1406) and the options screen (h0+$16be): lives,
//! game mode, saving the high scores and the level code.

use super::blit::Blit;
use super::common::{FadeKind, FireUp, call, fade, frames, palette_wait};
use super::title::{BUFFER_0, BUFFER_1, PLANE, eyes_init};
use super::{Cx, Routine, Yield, at, h2, password, sprite};
use crate::audio::INTENA;

/// The menu's flashing colour: a table of words ending in a negative one (h0+$168c), and the
/// pointer into it (h0+$1688).
const FLASH_TABLE: u32 = at(0x168c);
const FLASH_AT: u32 = at(0x1688);
/// The menu line's two colour MOVEs (START GAME, OPTIONS).
const ITEM: [u32; 2] = [h2(0x73d0), h2(0x73d8)];
pub const OPT_CURSOR: u32 = at(0x1958);
pub const OPT_LIVES: u32 = at(0x9e4);
pub const OPT_GAME_MODE: u32 = at(0x9e6);
pub const OPT_SAVE: u32 = at(0x195a);
const OPT_FLASH: u32 = at(0x1b16);
/// The options cursor's sprite record.
pub const CURSOR: u32 = at(0x1b17c);

/// `$1406`: the menu over the title. Returns its choice in `cx.ret`: 0 START GAME, 1 OPTIONS.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Menu {
    pc: u8,
    item: u16,
}

impl Menu {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            self.pc += 1;
            match self.pc {
                1 => {
                    cx.s.set_l(ITEM[0], 0x0098_0000);
                    cx.s.set_l(ITEM[1], 0x0098_0000);
                    return fade(
                        FadeKind::Step {
                            from: at(0x37e2),
                            target: at(0x3862),
                        },
                        0,
                        at(0x3ba0),
                        0x20,
                        at(0x1444),
                    );
                }
                2 => {
                    return fade(
                        FadeKind::Step {
                            from: at(0x3822),
                            target: at(0x38a2),
                        },
                        1,
                        at(0x3c20),
                        0x20,
                        at(0x146e),
                    );
                }
                3 => return palette_wait(0),
                4 => return palette_wait(1),
                // The menu's words into the picture once the beam is past them (line $8d).
                5 => return Yield::Line(0x8d),
                6 => {
                    cx.wr(INTENA, 0x0010);
                    let mut b = Blit {
                        con0: 0x09f0,
                        afwm: 0xffff,
                        alwm: 0xffff,
                        pt: [BUFFER_1 + 0xf000, 0, 0, BUFFER_0 + 0xb50],
                        modulo: [6, 0, 0, 0x10],
                        size: 0x8cc,
                        ..Blit::default()
                    };
                    for _ in 0..6 {
                        b.run(cx.s);
                        cx.busy(b.cycles());
                        b.pt[0] += 0x8e8;
                        b.pt[3] += PLANE;
                    }
                    cx.wr(INTENA, 0x8010);
                    cx.s.set_w(at(0x1686), 0);
                    return call(Routine::FireUp(FireUp));
                }
                7 => return frames(3),
                8 => {
                    cx.s.set_l(FLASH_AT, FLASH_TABLE);
                    cx.s.set_l(ITEM[0], 0x0194_0777);
                    cx.s.set_l(ITEM[1], 0x0194_0777);
                    self.item = 0;
                }
                // $1540: one frame of the menu.
                9 => return Yield::Raster,
                10 => {
                    let mut a = cx.s.l(FLASH_AT);
                    if (cx.w(a) as i16) < 0 {
                        a = FLASH_TABLE;
                        cx.s.set_l(FLASH_AT, a);
                    }
                    cx.s.set_l(FLASH_AT, cx.s.l(FLASH_AT).wrapping_add(2));
                    let colour = cx.w(a);
                    // Both mouse buttons held for 400 frames open the hidden info screen: the
                    // engine has no right mouse button, so this never counts.
                    let (on, off) = if self.item == 0 { (0, 1) } else { (1, 0) };
                    cx.s.set_w(ITEM[on] + 2, colour);
                    cx.s.set_w(ITEM[off] + 2, 0x999);
                    match cx.joy1dat() & 0x303 {
                        0x100 => self.item = 0,
                        0x001 => self.item = 1,
                        _ => {}
                    }
                    if !cx.fire() {
                        self.pc = 8;
                        continue;
                    }
                    cx.s.set_l(ITEM[0], 0x0098_0000);
                    cx.s.set_l(ITEM[1], 0x0098_0000);
                    return fade(FadeKind::Out, 0, at(0x3ba0), 0x20, at(0x163e));
                }
                11 => return fade(FadeKind::Out, 1, at(0x3c20), 0x20, at(0x1668)),
                12 => return palette_wait(0),
                13 => return palette_wait(1),
                _ => {
                    cx.ret = self.item as u32;
                    return Yield::Return;
                }
            }
        }
    }
}

/// `$16be`: the options screen, then back to the title's intro (h0+$1122).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Options {
    pc: u8,
    /// After a move: the direction held (`$100` up, `$1` down, `$300` left, `$3` right) and where
    /// to go on once it is let go.
    held: u16,
    then: u8,
}

impl Options {
    /// A move was made: one frame, then draw every frame while `held` stays held, then `then`.
    fn hold(&mut self, held: u16, then: u8) -> Yield {
        self.held = held;
        self.then = then;
        self.pc = 40;
        Yield::Raster
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    // Four planes, the menu's line at the top, a shorter window.
                    cx.s.set_w(h2(0x71da), 0x4200);
                    cx.s.set_l(h2(0x7338), 0x2001_ff00);
                    cx.s.set_l(h2(0x71e0), 0x0090_d2c1);
                    let eye = at(0x1b198);
                    cx.s.set_w(eye, 0x150);
                    cx.s.set_w(eye + 2, 0x75);
                    sprite::update(cx.s, eye);
                    sprite::attach(cx.s, CURSOR);
                    cx.s.set_w(CURSOR, 0x150);
                    cx.s.set_w(CURSOR + 2, 0x75);
                    sprite::update(cx.s, CURSOR);
                    for (k, v) in [0xff01_ff00u32, 0x0182_0f00, 0xffff_fffe]
                        .into_iter()
                        .enumerate()
                    {
                        cx.s.set_l(h2(0x73ec) + 4 * k as u32, v);
                    }
                    // The options picture, packed in the game (h0+$3464c), unpacked over buffer 0.
                    let packed: Vec<u8> = (0..0x215 * 4).map(|k| cx.b(at(0x3464c) + k)).collect();
                    cx.s.write(BUFFER_0, &packed);
                    let pic = cx.data.file("h0@3464c").to_vec();
                    cx.jobs.push(crate::game::Job {
                        inside: at(0x175c),
                        writes: vec![(BUFFER_0, 0x12930)],
                    });
                    cx.unpack(BUFFER_0, &pic, OPTIONS_UNPACK_CYCLES);
                    self.pc = 50;
                    return Yield::Interrupt;
                }
                50 => {
                    password::draw_code(cx, at(0x1760));
                    self.pc = 1;
                    return fade(
                        FadeKind::In { target: at(0x38e2) },
                        0,
                        at(0x3c20),
                        0x10,
                        at(0x178a),
                    );
                }
                1 => {
                    self.pc = 2;
                    return palette_wait(0);
                }
                2 => {
                    self.pc = 3;
                    return call(Routine::FireUp(FireUp));
                }
                3 => {
                    cx.s.set_w(OPT_CURSOR, 0);
                    self.pc = 4;
                    return frames(4);
                }
                // $17b4: one frame of the options.
                4 => {
                    self.pc = 5;
                    return Yield::Raster;
                }
                5 => {
                    draw(cx);
                    let cursor = cx.s.w(OPT_CURSOR);
                    if cx.joy1dat() & 0x303 == 0x100 && cursor != 0 {
                        cx.s.set_w(OPT_CURSOR, cursor - 1);
                        return self.hold(0x100, 6);
                    }
                    self.pc = 6;
                }
                6 => {
                    let cursor = cx.s.w(OPT_CURSOR);
                    if cx.joy1dat() & 0x303 == 0x001 && (cursor as i16) < 3 {
                        cx.s.set_w(OPT_CURSOR, cursor + 1);
                        return self.hold(0x001, 7);
                    }
                    self.pc = 7;
                }
                // options_input (h0+$195c): left and right change the option under the cursor.
                7 | 8 => {
                    let cursor = cx.s.w(OPT_CURSOR);
                    let (var, max) = match cursor {
                        1 => (OPT_GAME_MODE, 1),
                        2 => (OPT_SAVE, 1),
                        _ => (OPT_LIVES, 2),
                    };
                    if cursor == 3 {
                        self.pc = 9;
                        continue;
                    }
                    let joy = cx.joy1dat() & 0x303;
                    let v = cx.s.w(var);
                    if self.pc == 7 {
                        if joy == 0x300 && v != 0 {
                            cx.s.set_w(var, v - 1);
                            return self.hold(0x300, 8);
                        }
                        self.pc = 8;
                        continue;
                    }
                    if joy == 0x003 && max > v {
                        cx.s.set_w(var, v + 1);
                        return self.hold(0x003, 9);
                    }
                    self.pc = 9;
                }
                9 => {
                    let cursor = cx.s.w(OPT_CURSOR);
                    if cursor == 3 {
                        self.pc = 10;
                        return call(Routine::Password(password::Entry::default()));
                    }
                    self.pc = if !cx.fire() {
                        4
                    } else if (cursor as i16) < 2 {
                        30
                    } else {
                        cx.unported("saving the high scores to disk");
                        4
                    };
                }
                10 => {
                    cx.s.set_w(OPT_CURSOR, 0);
                    self.pc = 4;
                }
                // A direction held after a move: a frame, then draw every frame while held.
                40 => {
                    self.pc = 41;
                    return Yield::Raster;
                }
                41 => {
                    draw(cx);
                    if cx.joy1dat() & 0x303 == self.held {
                        return Yield::Raster;
                    }
                    self.pc = self.then;
                }
                // $1882: leave the options for the title's intro.
                30 => {
                    self.pc = 31;
                    return fade(FadeKind::Out, 0, at(0x3c20), 0x10, at(0x18ac));
                }
                31 => {
                    self.pc = 32;
                    return palette_wait(0);
                }
                _ => {
                    cx.s.set_w(h2(0x71da), 0x6200);
                    cx.s.set_l(h2(0x7338), 0x7401_ff00);
                    cx.s.set_l(h2(0x71e0), 0x0090_2cc1);
                    for k in 0..3 {
                        cx.s.set_l(h2(0x73ec) + 4 * k, 0x0098_0000);
                    }
                    sprite::attach(cx.s, CURSOR);
                    cx.s.set_w(CURSOR, 0x150);
                    cx.s.set_w(CURSOR + 2, 0x75);
                    sprite::update(cx.s, CURSOR);
                    eyes_init(cx);
                    super::title::blit_restore(cx, 0, 0x100, at(0x1924));
                    let packed: Vec<u8> = (0..0x458 * 4).map(|k| cx.b(at(0x34e92) + k)).collect();
                    cx.s.write(BUFFER_1 + 0xf000, &packed);
                    let eyes = cx.data.file("h0@34e92").to_vec();
                    cx.jobs.push(crate::game::Job {
                        inside: at(0x1950),
                        writes: vec![(BUFFER_1 + 0xf000, 0x3930)],
                    });
                    cx.unpack(BUFFER_1 + 0xf000, &eyes, EYES_UNPACK_CYCLES);
                    return Yield::Return;
                }
            }
        }
    }
}

/// How long unpacking the options picture (`$a000` bytes) takes: measured from line 50 of one
/// frame to line 270 twelve frames on.
const OPTIONS_UNPACK_CYCLES: u32 = (12 * 313 + 220) * super::BEAM_CYCLES_PER_LINE;
/// How long unpacking the eyes' graphics (`$3570` bytes, but the title is still shown) takes:
/// measured as ending between lines 255 and 299 twelve frames after it began on line 143.
const EYES_UNPACK_CYCLES: u32 = (12 * 313 + 140) * super::BEAM_CYCLES_PER_LINE;

/// `options_draw` (h0+$1a00): each option's three choices in grey, the chosen one white; the row
/// under the cursor flashes (shown 16 frames in 32). The game mode shown sets its tuning.
fn draw(cx: &mut Cx) {
    let flash = cx.s.b(OPT_FLASH).wrapping_add(1);
    cx.s.set_b(OPT_FLASH, flash);
    let on = flash & 0x10 != 0;
    let cursor = cx.s.w(OPT_CURSOR);
    for (row, (moves, var)) in [
        (h2(0x734a), OPT_LIVES),
        (h2(0x7356), OPT_GAME_MODE),
        (h2(0x7362), OPT_SAVE),
    ]
    .into_iter()
    .enumerate()
    {
        for k in 0..3 {
            cx.s.set_w(moves + 4 * k, 0x666);
        }
        if cursor == row as u16 && !on {
            continue;
        }
        let v = cx.s.w(var);
        if v <= 2 {
            if row == 1 {
                game_mode_tuning(cx, v);
            }
            cx.s.set_w(moves + 4 * v as u32, 0xfff);
        }
    }
}

/// `$730e`, `$7332`, `$7356`: the frog's tuning for each game mode (`frog+$98`..`$9e`).
fn game_mode_tuning(cx: &mut Cx, mode: u16) {
    let v: [u16; 4] = match mode {
        0 => [0xfffb, 6, 2, 0xfffe],
        1 => [0xfffa, 4, 1, 0xfffe],
        _ => [0xfffa, 4, 0, 0xffff],
    };
    for (k, w) in v.into_iter().enumerate() {
        cx.s.set_w(at(0x86e) + 2 * k as u32, w);
    }
}
