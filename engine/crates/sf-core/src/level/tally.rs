//! The level-complete tally (`level_tally`, h0+$4466; docs/08 "Scene flow"): once the exit's
//! fade is over, the tally's picture unpacked over display buffer 0 (packed in the game at
//! h0+$33234), the level's number, the pages (credits) earned for the time left, the tile coins
//! and the coins, up to five each, shown as icons, the pages' worth and the score printed; the
//! picture faded in; a wait for fire with up and down choosing between the slot machine (up) and
//! the points (down); the points added (5 000 a page), the extra-life check and a hundred frames'
//! pause, or the slot machine; the picture faded out.
//!
//! It runs on the front end's program (`crate::front`), like `level_exit_fade` before it.

use crate::front::blit::Blit;
use crate::front::common::{FadeKind, call, fade, frames};
use crate::front::fade::{RECORDS, SPEED};
use crate::front::{Cx, Routine, Yield, at, h2};
use crate::level::{LevelCtx, LevelOutcome, Tally};
use crate::mem::Bus;
use crate::state::State;

/// Display buffer 0 (`playfield_buffer_0`): the tally's picture, five planes of `$2800` bytes,
/// `$28` a row, its palette at `+$c800`.
const BUFFER_0: u32 = crate::layout::h3(0x10600);
const PLANE: u32 = 0x2800;
const ROW: u32 = 0x28;
/// The picture, packed in the game's hunk 0.
const PACKED: u32 = 0x33234;
/// The blank copper list (h2+$6fd8) and the tally's (h2+$6268): its bitplane pointers
/// (h2+$638c, a MOVE pair every 8 bytes), DIWSTOP's MOVE (h2+$62f0), its colours (h2+$630c) and
/// the two choices' colours (h2+$63be the slot machine's, h2+$63c6 the points').
const BLANK_COPPER: u32 = h2(0x6fd8);
const TALLY_COPPER: u32 = h2(0x6268);
const PLANE_MOVES: u32 = h2(0x638c);
const DIWSTOP_MOVE: u32 = h2(0x62f0);
const COLOURS: u32 = h2(0x630c);
const SLOT_COLOUR: u32 = h2(0x63be);
const POINTS_COLOUR: u32 = h2(0x63c6);
/// The chosen line's colour and the other's.
const LIT: u16 = 0xd33;
const UNLIT: u16 = 0x611;
/// DIWSTOP's MOVE while the picture fades (bottom on line `$ff`) and while it waits (`$12c`).
const WINDOW_FADING: u32 = 0x0090_ffc1;
const WINDOW_SHOWN: u32 = 0x0090_2cc1;
/// `level_tally`'s pages and choice (h0+$4796, $4798) and the level's text (h0+$49be, `0-0`).
pub const PAGES: u32 = at(0x4796);
pub const CHOICE: u32 = at(0x4798);
const LEVEL_TEXT: u32 = at(0x49be);
/// `world`, `level`, `level_clock` (four digits, minutes first), `frog_tile_coins`,
/// `coins_collected` (its low word is the coins), `items_total`, `score`.
const WORLD: u32 = at(0x40da);
const LEVEL: u32 = at(0x40de);
const LEVEL_CLOCK: u32 = at(0x876);
const TILE_COINS: u32 = at(0x854);
const COINS_COLLECTED: u32 = at(0x445e);
const ITEMS_TOTAL: u32 = at(0x163d2);
const SCORE: u32 = at(0x834);
/// `number_text` (h0+$161b2), which `number_to_text` writes.
const NUMBER_TEXT: u32 = at(0x161b2);
/// Points a page is worth.
const PAGE_POINTS: u32 = 5000;
/// Frames the points are shown for before the fade out (`moveq #$64` to h0+$5ea4).
const POINTS_PAUSE: u16 = 100;
/// The slot machine's coins (h0+$49c6).
const SLOT_COINS: u32 = at(0x49c6);

/// 68000 cycles, fitted to `tally-w1-1` (the original starts the unpacking on line 20 of the frame
/// its exit's fade ends in, and makes its first page blit on line 196 seventeen frames later): the
/// packed copy (h0+$ce38) and `atn_decompress` of the picture; `number_to_text`; the pages'
/// arithmetic.
const UNPACK_CYCLES: u32 = (17 * 313 + 150) * crate::front::BEAM_CYCLES_PER_LINE;
const NUMBER_CYCLES: u32 = 2_000;
const PAGES_CYCLES: u32 = 1_000;
/// The text printer's: a character looked at in its table, a glyph's eight rows (measured for the
/// slot machine's print, T72).
const CHAR_LOOK_CYCLES: u32 = 38;
const GLYPH_CYCLES: u32 = 250;
/// The text printer's plane (h0+$706e; its colour the byte after), its characters (h0+$701c) and
/// its font (h0+$3414c).
const TEXT_PLANE: u32 = at(0x706e);
const CHARSET: u32 = at(0x701c);
const FONT: u32 = at(0x3414c);

/// `level_tally` (h0+$4466), from where `level_exit_fade` returns into `top_loop`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LevelTally {
    pc: u8,
}

impl LevelTally {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    cx.cop1lc = BLANK_COPPER;
                    for k in 0..5 {
                        let p = BUFFER_0 + k * PLANE;
                        cx.s.set_w(PLANE_MOVES + 8 * k + 6, p as u16);
                        cx.s.set_w(PLANE_MOVES + 8 * k + 2, (p >> 16) as u16);
                    }
                    cx.s.set_w(PAGES, 0);
                    // h0+$ce38: the packed picture copied a long at a time over the buffer, then
                    // unpacked there.
                    let len = (cx.data.h0_l(PACKED + 8).wrapping_add(0x40) & !3).min(0x10000);
                    let packed = cx.data.h0_bytes(PACKED, len);
                    cx.s.write(BUFFER_0, &packed);
                    let pic = sf_disk::decompress(&packed).unwrap_or_default();
                    cx.unpack(BUFFER_0, &pic, UNPACK_CYCLES, &[at(0x44ba)]);
                    self.pc = 1;
                    return Yield::Interrupt;
                }
                1 => {
                    let (time, tile, coins) = pages_of(cx.s);
                    let mut cycles = 0;
                    crate::level::exit::with_level(cx, false, |c| {
                        cycles += level_text(c);
                        for (n, to) in [(time, 0xdac), (tile, 0x107c), (coins, 0x134c)] {
                            let p = c.bus.w(PAGES).wrapping_add(n);
                            c.bus.set_w(PAGES, p);
                            cycles += PAGES_CYCLES + hide_icons(c, n as u32, BUFFER_0 + to);
                        }
                        c.bus.set_l(COINS_COLLECTED, 0);
                        cycles += pages_text(c) + points_text(c) + score_text(c);
                    });
                    // The score's print runs into the next frame: what it and the rest write is
                    // compared once the original is out of it (h0+$479a, called from $4614).
                    cx.jobs.push(crate::game::Job {
                        inside: vec![at(0x4618)],
                        writes: vec![
                            (NUMBER_TEXT, 0x10),
                            (BUFFER_0 + 0x1fa5, 8 * ROW),
                            (DIWSTOP_MOVE, 4),
                        ],
                    });
                    cx.busy(cycles);
                    cx.s.set_l(DIWSTOP_MOVE, WINDOW_FADING);
                    cx.s.set_w(RECORDS[0].at + SPEED, 1);
                    self.pc = 2;
                    return Yield::Fade(RECORDS[0]);
                }
                2 => {
                    self.pc = 3;
                    let target = BUFFER_0 + 0xc800;
                    return fade(FadeKind::In { target }, 0, COLOURS, 0x20, at(0x4652));
                }
                3 => {
                    self.pc = 4;
                    return Yield::Raster;
                }
                4 => {
                    cx.cop1lc = TALLY_COPPER;
                    self.pc = 5;
                    return Yield::Fade(RECORDS[0]);
                }
                5 => {
                    cx.s.set_l(DIWSTOP_MOVE, WINDOW_SHOWN);
                    cx.s.set_w(CHOICE, 0);
                    self.pc = 6;
                }
                // h0+$467e: a busy loop, the choice's colours set, the joystick read, until fire
                // (or the mouse button) is down.
                6 => {
                    choice_colours(cx);
                    match cx.joy1dat() & 0x303 {
                        0x001 => cx.s.set_w(CHOICE, 0),
                        0x100 => cx.s.set_w(CHOICE, 1),
                        _ => {}
                    }
                    if !(cx.fire() || cx.mouse_button()) {
                        choice_colours(cx);
                        return Yield::Poll;
                    }
                    cx.s.set_l(DIWSTOP_MOVE, WINDOW_FADING);
                    self.pc = 8;
                    if cx.s.w(CHOICE) == 0 {
                        let pages = cx.s.w(PAGES) as u32;
                        let score = cx.s.l(SCORE).wrapping_add(pages * PAGE_POINTS);
                        cx.s.set_l(SCORE, score);
                        let mut cycles = 0;
                        crate::level::exit::with_level(cx, false, |c| {
                            cycles = score_text(c);
                            crate::level::panel::extra_life(c);
                        });
                        cx.busy(cycles);
                        if pages != 0 {
                            return frames(POINTS_PAUSE);
                        }
                    }
                }
                // h0+$4724: the picture faded out.
                8 => {
                    cx.s.set_l(DIWSTOP_MOVE, WINDOW_FADING);
                    self.pc = 9;
                    return fade(FadeKind::Out, 0, COLOURS, 0x20, at(0x4758));
                }
                9 => {
                    self.pc = 10;
                    return Yield::Fade(RECORDS[0]);
                }
                10 => {
                    cx.cop1lc = BLANK_COPPER;
                    let pages = cx.s.w(PAGES);
                    if pages == 0 || cx.s.w(CHOICE) == 0 {
                        self.pc = 13;
                        return call(Routine::Outcome(outcome(cx)));
                    }
                    cx.s.set_w(SLOT_COINS, pages);
                    // `slot_machine` starts with a raster wait; the engine's routine runs from
                    // after it.
                    self.pc = 11;
                    return Yield::Raster;
                }
                11 => {
                    self.pc = 12;
                    return call(Routine::Slot(Default::default()));
                }
                12 => {
                    self.pc = 13;
                    return after_slot(cx);
                }
                _ => return Yield::Return,
            }
        }
    }
}

/// `level_tally` after `slot_machine` returns (h0+$478e): the extra-life check, then the level is
/// over.
pub(crate) fn after_slot(cx: &mut Cx) -> Yield {
    crate::level::exit::with_level(cx, false, crate::level::panel::extra_life);
    call(Routine::Outcome(outcome(cx)))
}

/// The level's outcome as the tally leaves it.
fn outcome(cx: &Cx) -> LevelOutcome {
    LevelOutcome::Completed {
        tally: Tally {
            pages: cx.s.w(PAGES),
            slot_machine: cx.s.w(CHOICE) != 0,
        },
    }
}

/// h0+$467e: the line the choice is on lit, the other not.
fn choice_colours(cx: &mut Cx) {
    let (slot, points) = if cx.s.w(CHOICE) != 0 {
        (LIT, UNLIT)
    } else {
        (UNLIT, LIT)
    };
    cx.s.set_w(SLOT_COLOUR, slot);
    cx.s.set_w(POINTS_COLOUR, points);
}

/// The pages earned for the time left, the tile coins and the coins (h0+$44be..$4568), each
/// 0..=5: a page for every two minutes on the clock, for every 20 tile coins, and for every fifth
/// of the map's items taken as coins.
pub fn pages_of(s: &State) -> (u16, u16, u16) {
    pages(
        [s.b(LEVEL_CLOCK), s.b(LEVEL_CLOCK + 1)],
        s.w(TILE_COINS),
        s.w(COINS_COLLECTED + 2),
        s.w(ITEMS_TOTAL),
    )
}

/// [`pages_of`] on its inputs: the clock's minutes digits, the tile coins, the coins and the
/// map's items. A word quotient the 68000's `ext.l` makes negative counts as none. With fewer
/// than five items and a coin taken the original divides by zero and crashes (DESIGN §2.11): the
/// coins give none.
pub fn pages(minutes: [u8; 2], tile_coins: u16, coins: u16, items: u16) -> (u16, u16, u16) {
    let clamp = |q: u16| (q as i16).clamp(0, 5) as u16;
    let mins = (minutes[0] as u16)
        .wrapping_mul(10)
        .wrapping_add(minutes[1] as u16);
    let time = clamp(mins >> 1);
    let tile = clamp(tile_coins / 0x14);
    let coins = match items / 5 {
        _ if coins == 0 => 0,
        0 => 0,
        per => clamp(coins / per),
    };
    (time, tile, coins)
}

/// h0+$47f6 (`d0` = `n`, `a0` = `to`): the icons from the `n`th on (of five) cleared out of the
/// picture at `to`, a 16-line, five-word clear in each of its five planes, the copper interrupt
/// off meanwhile; nothing for `n` outside 0..=5. Returns the blits' cycles.
pub fn hide_icons(c: &mut LevelCtx, n: u32, to: u32) -> u32 {
    use crate::audio::{DMACON, INTENA};
    if (n as i32) < 0 || n >= 6 {
        return 0;
    }
    c.custom(INTENA, 0x0010);
    c.custom(DMACON, 0x8400);
    let mut cycles = 0;
    for p in 0..5 {
        let b = Blit {
            con0: 0x0100,
            pt: [0, 0, 0, to.wrapping_add(2 * n).wrapping_add(p * PLANE)],
            modulo: [0, 0, 0, 0x1e],
            size: 0x405,
            ..Blit::default()
        };
        b.run(c.bus.st);
        cycles += b.cycles();
    }
    c.custom(INTENA, 0x8010);
    cycles
}

/// h0+$4992: `0-0` with the world's and level's digits (h0+$49be), printed. Returns its cycles.
pub fn level_text(c: &mut LevelCtx) -> u32 {
    let b = &mut c.bus;
    let w = b.l(WORLD) as u8;
    let l = b.l(LEVEL) as u8;
    b.set_b(LEVEL_TEXT, w.wrapping_add(b'0'));
    b.set_b(LEVEL_TEXT + 2, l.wrapping_add(b'0'));
    print(b, LEVEL_TEXT, BUFFER_0 + 0x426)
}

/// h0+$457e..$45c6: the pages as two digits (a leading `0` for one), printed.
fn pages_text(c: &mut LevelCtx) -> u32 {
    let b = &mut c.bus;
    let n = b.w(PAGES) as i16 as i32 as u32;
    crate::level::panel::number_to_text(b, n);
    for (k, ch) in b"PAGE".iter().enumerate() {
        b.set_b(NUMBER_TEXT + 2 + k as u32, *ch);
    }
    if b.b(NUMBER_TEXT + 1) == 0 {
        let first = b.b(NUMBER_TEXT);
        b.set_b(NUMBER_TEXT + 1, first);
        b.set_b(NUMBER_TEXT, b' ');
    }
    zero_if_blank(b);
    NUMBER_CYCLES + print(b, NUMBER_TEXT, BUFFER_0 + 0x18e3)
}

/// h0+$45cc..$460e: the pages' worth in points, printed.
fn points_text(c: &mut LevelCtx) -> u32 {
    let b = &mut c.bus;
    let n = b.w(PAGES) as u32 * PAGE_POINTS;
    number_padded(b, n);
    NUMBER_CYCLES + print(b, NUMBER_TEXT, BUFFER_0 + 0x1cfc)
}

/// h0+$479a: the score, printed. Returns its cycles.
pub fn score_text(c: &mut LevelCtx) -> u32 {
    let b = &mut c.bus;
    let n = b.l(SCORE);
    number_padded(b, n);
    NUMBER_CYCLES + print(b, NUMBER_TEXT, BUFFER_0 + 0x1fa5)
}

/// `n` as `number_to_text` writes it, `PAGE` after nine places (ending the print), the sixteen
/// bytes from the text's start that are 0 made spaces (h0+$47de), a blank first place `0`.
fn number_padded(b: &mut Bus, n: u32) {
    crate::level::panel::number_to_text(b, n);
    for (k, ch) in b"PAGE".iter().enumerate() {
        b.set_b(NUMBER_TEXT + 9 + k as u32, *ch);
    }
    for k in 0..16 {
        if b.b(NUMBER_TEXT + k) == 0 {
            b.set_b(NUMBER_TEXT + k, b' ');
        }
    }
    zero_if_blank(b);
}

fn zero_if_blank(b: &mut Bus) {
    if b.b(NUMBER_TEXT) == b' ' {
        b.set_b(NUMBER_TEXT, b'0');
    }
}

/// The text printer (h0+$6f3a) on display buffer 0's planes: `text`'s characters from the font
/// (h0+$3414c, `$50` a row) at `to`, a byte each, into the plane `text_plane` names (it sets 1);
/// `9` moves on eight, `10` a line of eight rows, `COLORn` sets the colour, `PAGE` or a 0 after a
/// character not in its table (h0+$701c) ends it. Returns its 68000 cycles.
pub fn print(b: &mut Bus, text: u32, to: u32) -> u32 {
    b.set_w(TEXT_PLANE, 1);
    let (mut a0, mut a1, mut a5) = (text, to, to);
    let mut cycles = 0;
    loop {
        let ch = b.b(a0);
        a0 += 1;
        match ch {
            9 => {
                a1 += 8;
                continue;
            }
            10 => {
                a5 += 0x140;
                a1 = a5;
                continue;
            }
            _ => {}
        }
        if ch == b'C' && (0..4).all(|k| b.b(a0 + k) == b"OLOR"[k as usize]) {
            let n = b.b(a0 + 4).wrapping_sub(b'0');
            b.set_b(TEXT_PLANE + 1, n);
            a0 += 5;
            a1 += 6;
            continue;
        }
        if ch == b'P' && (0..3).all(|k| b.b(a0 + k) == b"AGE"[k as usize]) {
            return cycles;
        }
        let mut k = 0u32;
        let found = loop {
            cycles += CHAR_LOOK_CYCLES;
            if b.b(CHARSET + k) == ch {
                break true;
            }
            k += 1;
            if b.b(a0) == 0 {
                break false;
            }
        };
        if !found {
            return cycles;
        }
        let dst = if b.w(TEXT_PLANE) == 2 { a1 + PLANE } else { a1 };
        for r in 0..8 {
            let v = b.b(FONT + k + 0x50 * r);
            b.set_b(dst + ROW * r, v);
        }
        cycles += GLYPH_CYCLES;
        a1 += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::pages;

    #[test]
    fn pages_count_two_minutes_twenty_tile_coins_and_a_fifth_of_the_items() {
        // 4:xx left, nothing collected: two pages.
        assert_eq!(pages([0, 4], 0, 0, 40), (2, 0, 0));
        assert_eq!(pages([0, 5], 0, 0, 40), (2, 0, 0));
        assert_eq!(pages([1, 3], 0, 0, 40), (5, 0, 0));
        assert_eq!(pages([0, 0], 19, 0, 40), (0, 0, 0));
        assert_eq!(pages([0, 0], 20, 0, 40), (0, 1, 0));
        assert_eq!(pages([0, 0], 200, 0, 40), (0, 5, 0));
        // 40 items: a page every 8 coins.
        assert_eq!(pages([0, 0], 0, 7, 40), (0, 0, 0));
        assert_eq!(pages([0, 0], 0, 8, 40), (0, 0, 1));
        assert_eq!(pages([0, 0], 0, 40, 40), (0, 0, 5));
        assert_eq!(pages([0, 0], 0, 99, 40), (0, 0, 5));
    }

    #[test]
    fn a_quotient_ext_l_makes_negative_counts_none() {
        // 40 000 tile coins / 20 = 2 000: positive, five. 65 535 coins / 1: $ffff, negative.
        assert_eq!(pages([0, 0], 40_000, 0, 5), (0, 5, 0));
        assert_eq!(pages([0, 0], 0, 0xffff, 5), (0, 0, 0));
        assert_eq!(pages([0, 0], 0, 0x7fff, 5), (0, 0, 5));
    }

    #[test]
    fn fewer_than_five_items_give_no_coin_pages_where_the_original_divides_by_zero() {
        assert_eq!(pages([0, 0], 0, 3, 4), (0, 0, 0));
    }
}
