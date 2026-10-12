//! What the title runs between games and in attract mode (docs/13): `top_loop` (h0+$5c) around
//! the title, the award pages (`award_pages` h0+$1e98: the features and bonus tables), the hall of
//! talent and the credits (`hiscore_and_credits` h0+$20e0). The attract demo that follows is
//! T33's; a game's start is T21's.

use super::blit::Blit;
use super::common::{FadeKind, call, fade, frames};
use super::title::{ATTRACT_ABORT, ATTRACT_MODE, BLANK_COPPER, BUFFER_0, Title};
use super::{Cx, Routine, Yield, at, h2, vbl};
use crate::audio::{DMACON, INTENA};
use crate::game::LoadEnd;

const LEVEL_STARTED: u32 = at(0x9f0);
const TEXT: u32 = at(0x3258);
/// The award pages' copper list, its five bitplane MOVEs and its palette.
const AWARD_COPPER: u32 = h2(0x6110);
const AWARD_PLANES: u32 = h2(0x6234);
const AWARD_PALETTE: u32 = h2(0x61b4);
/// The hall of talent's copper list (h2+$6ff0), its bitplane MOVEs and palette.
const HALL_COPPER: u32 = h2(0x6ff0);
const HALL_PLANES: u32 = h2(0x7118);
const HALL_PALETTE: u32 = h2(0x7098);

/// `top_loop` (h0+$5c) from the title on: the title, then (attract mode or a finished game) the
/// award pages and the hall of talent, then the attract demo or the game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct TopLoop {
    pc: u8,
}

impl TopLoop {
    /// After the boot's title.
    pub fn after_title() -> TopLoop {
        TopLoop { pc: 1 }
    }

    /// In `end_seq_load`, after the final level (h0+$7b6 on the stack): the end sequence next.
    pub fn after_end_seq_load() -> TopLoop {
        TopLoop { pc: 21 }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    self.pc = 1;
                    return call(Routine::Title(Title::default()));
                }
                1 => {
                    let attract = cx.s.l(ATTRACT_MODE) != 0;
                    let pages = cx.s.w(at(0x16)) == 0 || (!attract && cx.s.w(at(0x1c)) == 0);
                    if !pages {
                        cx.unported("a game's start (T21)");
                        return Yield::Raster;
                    }
                    self.pc = 2;
                    return call(Routine::AwardPages(AwardPages::default()));
                }
                2 => {
                    if cx.ret != 0 {
                        // Fire on an award page: back to the title.
                        self.pc = 0;
                        continue;
                    }
                    self.pc = 3;
                    return call(Routine::HiscoreCredits(HiscoreCredits::default()));
                }
                3 => {
                    if cx.s.w(ATTRACT_ABORT) != 0 {
                        self.pc = 0;
                        continue;
                    }
                    cx.s.set_w(LEVEL_STARTED, 0);
                    if cx.s.l(ATTRACT_MODE) == 0 {
                        cx.unported("a game's start (T21)");
                    } else {
                        cx.unported("the attract demo (T33)");
                    }
                    return Yield::Raster;
                }
                // h0+$7b6: the end sequence, then `game_full_mode` decides: the title again, or
                // the game's end ($7c6: back to the system).
                21 => {
                    self.pc = 22;
                    return call(Routine::EndSequence(Default::default()));
                }
                22 => {
                    if cx.s.w(at(0x6)) == 0 {
                        cx.unported("the game's end after the end sequence (h0+$7c6)");
                        return Yield::Raster;
                    }
                    self.pc = 0;
                }
                _ => unreachable!("TopLoop pc {}", self.pc),
            }
        }
    }
}

/// Whether the last game's score is in the high-score table (`hiscore_table` h0+$2f90).
pub(super) fn new_hiscore(cx: &Cx) -> bool {
    let score = cx.s.l(at(0x834));
    (0..10).any(|k| (score.wrapping_sub(cx.s.l(at(0x2f90) + 8 * k)) as i32) >= 0)
}

/// `award_pages` (h0+$1e98): the two award tables (`AWA1`, `AWA2`), each faded in, shown 300
/// frames (fire or the mouse button cut it short, back to the title) and faded out. Outside
/// attract mode only after a game with a new high score. Hands back 1 when cut short.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct AwardPages {
    pc: u8,
    page: u8,
    count: u16,
}

impl AwardPages {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    cx.ret = 0;
                    if cx.s.l(ATTRACT_MODE) == 0 && !new_hiscore(cx) {
                        return Yield::Return;
                    }
                    for k in 0..5 {
                        let p = BUFFER_0 + k * 0x2800;
                        cx.s.set_w(AWARD_PLANES + 8 * k + 6, p as u16);
                        cx.s.set_w(AWARD_PLANES + 8 * k + 2, (p >> 16) as u16);
                    }
                    self.pc = 1;
                }
                // A page: load it, fade it in, show it.
                1 => {
                    let (name, ret) = [("AWA1", 0x1f06), ("AWA2", 0x1fc8)][self.page as usize];
                    let pic = cx.data.file(name).to_vec();
                    cx.s.write(BUFFER_0, &pic);
                    cx.loaded(LoadEnd::Left(at(ret)));
                    self.pc = 2;
                    let inside = [0x1f4a, 0x200c][self.page as usize];
                    return fade(
                        FadeKind::In {
                            target: BUFFER_0 + 0xc800,
                        },
                        0,
                        AWARD_PALETTE,
                        0x20,
                        at(inside),
                    );
                }
                2 => {
                    self.pc = 3;
                    return Yield::Raster;
                }
                3 => {
                    cx.cop1lc = AWARD_COPPER;
                    self.pc = 4;
                    return Yield::Fade(super::fade::RECORDS[0]);
                }
                // $2076: 300 frames, fire or the mouse button end it.
                4 => {
                    self.count = 0x12c;
                    self.pc = 5;
                }
                5 => {
                    if cx.mouse_button() || cx.fire() {
                        self.pc = 20;
                        continue;
                    }
                    self.pc = 6;
                    return Yield::Raster;
                }
                6 => {
                    self.count -= 1;
                    self.pc = if self.count != 0 { 5 } else { 7 };
                }
                7 => {
                    self.pc = 8;
                    let inside = [0x1f98, 0x205a][self.page as usize];
                    return fade(FadeKind::Out, 0, AWARD_PALETTE, 0x20, at(inside));
                }
                8 => {
                    self.pc = 9;
                    return Yield::Raster;
                }
                9 => {
                    self.pc = 10;
                    return Yield::Fade(super::fade::RECORDS[0]);
                }
                10 => {
                    self.page += 1;
                    if self.page < 2 {
                        self.pc = 1;
                        continue;
                    }
                    cx.cop1lc = BLANK_COPPER;
                    return Yield::Return;
                }
                // $2096: cut short.
                20 => {
                    self.pc = 21;
                    return fade(FadeKind::Out, 0, AWARD_PALETTE, 0x20, at(0x20c0));
                }
                21 => {
                    self.pc = 22;
                    return Yield::Raster;
                }
                22 => {
                    self.pc = 23;
                    return Yield::Fade(super::fade::RECORDS[0]);
                }
                _ => {
                    cx.cop1lc = BLANK_COPPER;
                    cx.ret = 1;
                    return Yield::Return;
                }
            }
        }
    }
}

/// `hiscore_and_credits` (h0+$20e0) in attract mode: the hall of talent's picture (`HPIC`, kept
/// packed since the title loaded it), the table, then the credits scrolling up until they end or
/// fire, the mouse button or Escape cut them short (`attract_abort`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct HiscoreCredits {
    pc: u8,
    d6: u16,
    d7: u16,
}

/// How long copying `HPIC` out of hunk 1 and unpacking it takes: measured from line 299 to line
/// 184 forty-nine frames on.
const HALL_UNPACK_CYCLES: u32 = (49 * 313 - 115) * super::BEAM_CYCLES_PER_LINE;

impl HiscoreCredits {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    cx.s.set_w(ATTRACT_ABORT, 0);
                    if cx.s.w(at(0x24)) != 0 {
                        return Yield::Return;
                    }
                    if cx.s.l(ATTRACT_MODE) == 0 {
                        if !new_hiscore(cx) {
                            return Yield::Return;
                        }
                        cx.unported("hiscore_and_credits after a game with a new high score");
                        return Yield::Return;
                    }
                    self.pc = 1;
                    return Yield::Raster;
                }
                1 => {
                    cx.cop1lc = BLANK_COPPER;
                    let pic = cx.data.file("HPIC").to_vec();
                    // The packed copy (h0+$15fb0), then atn_decompress.
                    let packed: Vec<u8> = (0..0x59d8)
                        .map(|k| cx.b(crate::layout::h1(0x20d00) + k))
                        .collect();
                    cx.s.write(BUFFER_0, &packed);
                    cx.unpack(
                        BUFFER_0,
                        &pic,
                        HALL_UNPACK_CYCLES,
                        &[at(0x2156), at(0x2168)],
                    );
                    self.pc = 2;
                    return Yield::Interrupt;
                }
                2 => {
                    for k in 0..5 {
                        let p = BUFFER_0 + k * 0x2d00;
                        cx.s.set_w(HALL_PLANES + 8 * k + 6, p as u16);
                        cx.s.set_w(HALL_PLANES + 8 * k + 2, (p >> 16) as u16);
                    }
                    self.pc = 3;
                    return fade(
                        FadeKind::In {
                            target: BUFFER_0 + 0xe100,
                        },
                        0,
                        HALL_PALETTE,
                        0x20,
                        at(0x21bc),
                    );
                }
                3 => {
                    self.pc = 4;
                    return frames(2);
                }
                4 => {
                    cx.cop1lc = HALL_COPPER;
                    self.pc = 5;
                    return Yield::Fade(super::fade::RECORDS[0]);
                }
                5 => {
                    // hiscore_name_entry returns at once in attract mode.
                    cx.s.set_l(TEXT, at(0x340d));
                    self.pc = 6;
                    return call(Routine::HallOfTalent(HallOfTalent::default()));
                }
                6 => {
                    if cx.s.l(ATTRACT_MODE) == 0 {
                        self.pc = 20;
                        continue;
                    }
                    self.d6 = 0;
                    self.d7 = 0;
                    self.pc = 7;
                }
                // $21fa: the credits, a line a frame... once the beam is at line $c5.
                7 => {
                    if cx.mouse_button() || cx.fire() || cx.s.b(vbl::KEY_DOWN) == 0x45 {
                        cx.s.set_w(ATTRACT_ABORT, 1);
                        self.pc = 20;
                        continue;
                    }
                    self.pc = 8;
                    return Yield::Line(0xc4);
                }
                8 => {
                    self.pc = 9;
                    return Yield::Line(0xc5);
                }
                9 => {
                    self.d6 = !self.d6;
                    if self.d6 != 0 {
                        scroll(cx, 0x28, at(0x223a));
                        self.pc = 7;
                        continue;
                    }
                    self.d7 += 1;
                    if (self.d7 as i16) < 0xc {
                        self.pc = 7;
                        continue;
                    }
                    self.d7 = 0;
                    self.pc = 7;
                    if !text_line(cx, at(0x224c)) {
                        // The credits' end.
                        self.pc = 20;
                    }
                }
                // $224e: fade out, the title's picture back in buffer 0, sprites off.
                20 => {
                    self.pc = 21;
                    return fade(FadeKind::Out, 0, HALL_PALETTE, 0x20, at(0x2278));
                }
                21 => {
                    self.pc = 22;
                    return frames(2);
                }
                22 => {
                    self.pc = 23;
                    return Yield::Fade(super::fade::RECORDS[0]);
                }
                _ => {
                    cx.cop1lc = BLANK_COPPER;
                    super::title::blit_restore(cx, 0, 0x100, at(0x22a2));
                    cx.wr(DMACON, 0x0020);
                    return Yield::Return;
                }
            }
        }
    }
}

/// `hall_of_talent` (h0+$2e56): the table's text, its twenty lines scrolled up into view, held
/// 350 frames (fire, the mouse button or Escape go on at once), then scrolled out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct HallOfTalent {
    pc: u8,
    d5: u16,
    d6: u16,
}

impl HallOfTalent {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    build_table(cx);
                    cx.s.set_l(TEXT, at(0x325c));
                    self.d5 = 0;
                    self.pc = 1;
                }
                // Twenty lines in, a frame each.
                1 | 13 => {
                    self.d5 += 1;
                    if (self.d5 as i16) >= 0x14 {
                        if self.pc == 13 {
                            return Yield::Return;
                        }
                        self.d6 = 0x15e;
                        self.pc = 10;
                        continue;
                    }
                    self.pc += 1;
                    return Yield::Raster;
                }
                2 | 14 => {
                    let (s_ret, t_ret) = if self.pc == 2 {
                        (0x2e78, 0x2e7c)
                    } else {
                        (0x2ebe, 0x2ec2)
                    };
                    scroll(cx, 0x1e0, at(s_ret));
                    if !text_line(cx, at(t_ret)) {
                        cx.unported("the hall of talent's text running into the credits' end");
                    }
                    self.pc -= 1;
                }
                // Held 350 frames.
                10 => {
                    if cx.mouse_button() || cx.fire() || cx.s.b(vbl::KEY_DOWN) == 0x45 {
                        return Yield::Return;
                    }
                    self.pc = 11;
                    return Yield::Raster;
                }
                11 => {
                    self.d6 -= 1;
                    if self.d6 != 0 {
                        self.pc = 10;
                        continue;
                    }
                    self.d5 = 0;
                    self.pc = 13;
                }
                _ => unreachable!("HallOfTalent pc {}", self.pc),
            }
        }
    }
}

/// `$3096`, `credits_scroll` (h0+$3030): the text plane (buffer 0's fifth, at `+$b400`) moved up
/// by `bytes` (twelve lines, or one), 272 lines of it, by the blitter with priority.
fn scroll(cx: &mut Cx, bytes: u32, inside: u32) {
    cx.wr(INTENA, 0x0010);
    cx.wr(DMACON, 0x8400);
    let start = cx.line;
    let dest = BUFFER_0 + 0xb400;
    let n = 272 * 20u32;
    let old = cx.s.read(dest, 2 * n);
    let b = Blit {
        con0: 0x09f0,
        afwm: 0xffff,
        alwm: 0xffff,
        pt: [dest + bytes, 0, 0, dest],
        size: 0x4414,
        ..Blit::default()
    };
    b.run(cx.s);
    // The beam overtakes the blit: each word shows new only if written before its fetch (the
    // plane is the fifth, fetched sixth in each unit of eight cycles from DDFSTRT).
    let times = super::blit::word_times(n, start, SCROLL_START_H, &cx.fetch);
    if start < super::FRAME_LINES {
        let mut shown = cx.s.clone();
        let first = cx.fetch.first_cycle();
        for (i, &(l, h)) in times.iter().enumerate() {
            let (y, w) = (i as u32 / 20, i as u32 % 20);
            let fetch_at = (0x2c + y as u16, first + 8 * w as u16 + 6);
            if (l, h) >= fetch_at {
                let k = 2 * i;
                shown.write(dest + k as u32, &old[k..k + 2]);
            }
        }
        cx.reveal = Some(super::FRAME_LINES);
        cx.reveal_state = Some(shown);
    }
    let end = times.last().map_or(start, |t| t.0 + 1);
    cx.line = cx.line.max(end);
    super::title::stalled_past(cx, start, (dest, 2 * n), inside);
    cx.wr(INTENA, 0x8010);
}

/// The DMA cycle of its line the scroll's blit starts on (after the CPU's wait and set-up).
const SCROLL_START_H: u16 = 0x80;

/// `$3190`: the next line of text (from `h0+$3258`, a zero byte ends it) written into the text
/// plane's bottom (`+$dc50`) in the twelve-line font (h0+$376ea). False when the text ends (a
/// negative byte: the credits are over).
fn text_line(cx: &mut Cx, inside: u32) -> bool {
    cx.jobs.push(crate::game::Job {
        inside: vec![inside],
        writes: vec![(BUFFER_0 + 0xdc50, 12 * 0x28), (TEXT, 4)],
    });
    let mut a = cx.s.l(TEXT);
    let mut to = BUFFER_0 + 0xdc50;
    let mut cycles = 60u32;
    let ok = loop {
        let c = cx.b(a);
        a += 1;
        cycles += 30;
        if c == 0 {
            break true;
        }
        if c & 0x80 != 0 {
            break false;
        }
        if c == 0x20 {
            to += 1;
            continue;
        }
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
            break true;
        }
        let glyph = at(0x376ea).wrapping_add(k).wrapping_sub(2);
        for r in 0..12 {
            let v = cx.b(glyph + 0x28 * r);
            cx.s.set_b(to + 0x28 * r, v);
        }
        cycles += 12 * 28;
        to += 1;
    };
    if ok {
        cx.s.set_l(TEXT, a);
    }
    cx.busy(cycles);
    ok
}

/// `$2ec8`: the hall of talent's ten lines (h0+$32b4, `$1f` apart): each name, and each score
/// right-aligned in eight places.
fn build_table(cx: &mut Cx) {
    for k in 0..10u32 {
        for i in 0..8 {
            let v = cx.b(at(0x2fe0) + 8 * k + i);
            cx.s.set_b(at(0x32bd) + 0x1f * k + i, v);
        }
    }
    for k in 0..10u32 {
        let score = cx.s.l(at(0x2f90) + 8 * k);
        number_text(cx, score);
        let mut to = at(0x32b4) + 0x1f * k;
        for i in 0..8 {
            cx.s.set_b(to + i, 0x20);
        }
        // Right-aligned: a place for each empty digit from the buffer's end back (h0+$2f56; its
        // count register is the byte it reads, so it goes on to the first one that is not zero).
        let mut a = at(0x161ba);
        loop {
            a -= 1;
            if cx.s.b(a) != 0 {
                break;
            }
            to += 1;
        }
        let mut a = at(0x161b2);
        for _ in 0..8 {
            let v = cx.s.b(a);
            a += 1;
            cx.s.set_b(to, v);
            to += 1;
            if cx.s.b(a) == 0 {
                break;
            }
        }
    }
}

/// `$160dc`: `n` as decimal digits at h0+$161b2 without leading zeros (nine 9s above 999 999 999),
/// zero-filled after.
fn number_text(cx: &mut Cx, n: u32) {
    let buf = at(0x161b2);
    for k in 0..12 {
        cx.s.set_b(buf + k, 0);
    }
    let mut a = buf;
    if n > 999_999_999 {
        for _ in 0..9 {
            cx.s.set_b(a, b'9');
            a += 1;
        }
    } else {
        cx.s.set_l(at(0x161aa), 0);
        let mut d0 = n;
        let mut k = 0;
        loop {
            let p = cx.l(at(0x16182) + 4 * k);
            if p == 0 {
                break;
            }
            let mut digit = b'0';
            loop {
                d0 = d0.wrapping_sub(p);
                digit = digit.wrapping_add(1);
                if d0 == 0 {
                    break;
                }
                if (d0 as i32) > 0 {
                    continue;
                }
                d0 = d0.wrapping_add(p);
                digit = digit.wrapping_sub(1);
                break;
            }
            k += 1;
            if cx.s.b(at(0x161aa)) != 0 || digit != b'0' {
                cx.s.set_b(a, digit);
                a += 1;
                cx.s.set_b(at(0x161aa), 1);
            }
        }
    }
    while cx.s.b(a) != 0 {
        cx.s.set_b(a, 0);
        a += 1;
    }
}
