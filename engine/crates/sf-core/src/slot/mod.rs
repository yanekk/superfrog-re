//! The slot machine (T92, docs/13): `slot_machine` (h0+$49dc), which `level_tally` calls when the
//! player picks it, as a front-end routine ([`crate::front`]): its loads, the overlay's fade-out,
//! the music's start, the win counted (none yet), the reels, lamps and digits set up and the
//! picture's fade-in set up, up to the attract loop's first raster wait (h0+$4d24).
//!
//! The loads are instant (owner, 2026-10-12): the engine's first frame stands for the original's
//! frame its last load ended on, `overlays_end` waiting on the overlay's fade (h0+$b7e2).

use crate::front::blit::Blit;
use crate::front::common::{FadeKind, MusicStart, call, fade};
use crate::front::{Cx, Routine as FrontRoutine, Yield, at, h2};
use crate::game::{Job, LoadEnd};
use crate::layout::{h1, h3};

/// Display buffers 0 and 1 (`playfield_buffer_0`, `_1`): `FRUT`, the machine, in 0 (with `FRUM`,
/// the lamps' mask, at +$f000); `FRUC`, the reels and digits, in 1.
const BUFFER_0: u32 = h3(0x10600);
const BUFFER_1: u32 = h3(0x22f30);
const PLANE: u32 = 0x2800;
const ROW: u32 = 0x28;
/// Chip RAM from the effect bank's place that `slot_machine` keeps over the level's map while the
/// slot's own sound is loaded there (h0+$4b44).
const SOUND_KEPT: u32 = h3(0x64c70);
const SOUND_KEPT_AT: u32 = h1(0x20d00);
const SOUND_KEPT_LEN: u32 = 0xfa00;

/// `slot_state` (h0+$49c6): the coins put in, the three reels' positions, the hi-lo count, ...
const COINS: u32 = at(0x49c6);
const REELS: u32 = at(0x49c8);
const HILO: u32 = at(0x49ce);
const STATE_49D0: u32 = at(0x49d0);
const STATE_49D2: u32 = at(0x49d2);
const SCORE_SHOWN: u32 = at(0x49d6);
const STATE_49DA: u32 = at(0x49da);
/// `score` (h0+$834).
const SCORE: u32 = at(0x834);
/// `slot_count_win`'s count (h0+$515a: over; $515c: the score shown) and its text (h0+$5160).
const COUNT_OVER: u32 = at(0x515a);
const COUNT_SHOWN: u32 = at(0x515c);
const COUNT_TEXT: u32 = at(0x5160);
/// `slot_win` (h0+$57b2).
const WIN: u32 = at(0x57b2);
/// `slot_menu` (h0+$6152), the attract pattern (h0+$62ec: its row, $62f0 frames left on it,
/// $62f2 patterns shown) and its first pattern (h0+$3b2e8).
const MENU: u32 = at(0x6152);
const ATTRACT_ROW: u32 = at(0x62ec);
const ATTRACT_LEFT: u32 = at(0x62f0);
const ATTRACT_PASSES: u32 = at(0x62f2);
const ATTRACT_FIRST: u32 = at(0x3b2e8);
/// The lamps (h0+$6842: 26 records of 8 bytes, `+0` lit, `+2` the rectangle, `+6`) and their
/// rectangles (h0+$670a: x, y, width, height and two words).
const LAMPS: u32 = at(0x6842);
const LAMP_RECTS: u32 = at(0x670a);
const LAMP_COUNT: u32 = 0x1a;
/// The mouse pointer's position (h0+$11b66, $11b68) and its sprite record (h0+$1b10c).
const POINTER_X: u32 = at(0x11b66);
const POINTER_Y: u32 = at(0x11b68);
const POINTER: u32 = at(0x1b10c);
/// The slot's copper list: its bitplane MOVEs (h2+$6708), its colours (h2+$6688), and the
/// palette the picture fades in to (h0+$6912).
const SLOT_COPPER: u32 = h2(0x65e4);
const PLANE_MOVES: u32 = h2(0x6708);
const SLOT_COLOURS: u32 = h2(0x6688);
const SLOT_PALETTE: u32 = at(0x6912);
/// The text printer's plane (h0+$706e), its colour (h0+$706f), its characters (h0+$701c) and
/// its font (h0+$3414c, `$50` a row).
const TEXT_PLANE: u32 = at(0x706e);
const TEXT_COLOUR: u32 = at(0x706f);
const CHARSET: u32 = at(0x701c);
const FONT: u32 = at(0x3414c);

/// `p41_init` on the slot's module: INTENA off on line 23, on again on line 41 (measured, T72).
const P41_INIT_LINES: u16 = 18;
/// 68000 cycles, measured on the oracle (T72): `slot_lamps_off` ($6238), `slot_draw_lamps`
/// ($645e) less its blits (about 520 a lamp and 120 to start), `slot_digits` ($62f4, byte copies),
/// and the text printer ($6f3a) on `       0PAGE`, 25 962: about 38 a character looked at in its
/// table, the rest a glyph's eight rows.
const LAMPS_OFF_CYCLES: u32 = 1_650;
const LAMP_CYCLES: u32 = 520;
const LAMPS_START_CYCLES: u32 = 120;
const DIGITS_CYCLES: u32 = 47_502;
const CHAR_LOOK_CYCLES: u32 = 38;
const GLYPH_CYCLES: u32 = 250;
/// `slot_count_win` around its print (the count, `number_to_text`, h0+$5174) and `slot_init`:
/// small next to the rest.
const COUNT_CYCLES: u32 = 1_500;
const INIT_CYCLES: u32 = 1_200;

/// `slot_machine`, from just after its first raster wait (h0+$49e2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Routine {
    pc: u8,
}

impl Routine {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        self.pc += 1;
        match self.pc {
            1 => {
                cx.cop1lc = crate::front::title::BLANK_COPPER;
                access_overlay_show(cx);
                load(cx);
                // Over once the original waits on the overlay's fade-out (`overlays_end`); the
                // line its load ended on decides where that fade falls.
                cx.loaded(LoadEnd::Reached(at(0xb7e2)));
                call(FrontRoutine::Overlay(Default::default()))
            }
            2 => call(FrontRoutine::MusicStart(MusicStart::keeping_module(
                at(0x4c64),
                P41_INIT_LINES,
            ))),
            3 => {
                cx.s.set_w(HILO, 1);
                let score = cx.s.l(SCORE);
                cx.s.set_l(SCORE_SHOWN, score);
                cx.s.set_l(WIN, 0);
                count_win_lamps(cx);
                Yield::Raster
            }
            // `slot_count_win`'s count after its raster wait (h0+$5108): nothing won, it is over
            // at once; the score is printed, into the next frame.
            4 => {
                let shown = cx.s.l(COUNT_SHOWN).wrapping_add(0x3e8);
                cx.s.set_l(COUNT_SHOWN, shown);
                let score = cx.s.l(SCORE_SHOWN);
                if (score.wrapping_sub(shown) as i32) < 0 {
                    cx.s.set_l(COUNT_SHOWN, score);
                    cx.s.set_w(COUNT_OVER, 1);
                }
                if cx.s.w(COUNT_OVER) == 0 {
                    cx.unported("slot_count_win counting a win");
                }
                count_text(cx);
                let cycles = COUNT_CYCLES + print_text(cx, COUNT_TEXT, BUFFER_0 + 0x7683);
                cx.jobs.push(Job {
                    inside: vec![at(0x5150)],
                    writes: vec![(BUFFER_0 + 0x7683, 8 * ROW)],
                });
                cx.busy(cycles);
                Yield::Return
            }
            5 => {
                cx.s.set_w(STATE_49D0, 0);
                cx.s.set_l(STATE_49D2, 0);
                cx.s.set_w(STATE_49DA, 0);
                slot_init(cx);
                slot_digits(cx);
                cx.busy(INIT_CYCLES + DIGITS_CYCLES);
                cx.s.set_w(POINTER_X, 0x20);
                cx.s.set_w(POINTER_Y, 0xe8);
                for k in 0..6 {
                    let p = BUFFER_0 + k * PLANE;
                    cx.s.set_w(PLANE_MOVES + 8 * k + 6, p as u16);
                    cx.s.set_w(PLANE_MOVES + 8 * k + 2, (p >> 16) as u16);
                }
                crate::front::sprite::attach(cx.s, POINTER);
                let rec = crate::front::fade::RECORDS[0];
                cx.s.set_w(rec.at + crate::front::fade::SPEED, 1);
                Yield::Fade(rec)
            }
            6 => fade(
                FadeKind::In {
                    target: SLOT_PALETTE,
                },
                0,
                SLOT_COLOURS,
                0x20,
                at(0x4d14),
            ),
            7 => Yield::Raster,
            8 => {
                cx.cop1lc = SLOT_COPPER;
                Yield::Raster
            }
            _ => {
                cx.unported("the slot machine's attract loop");
                self.pc -= 1;
                Yield::Raster
            }
        }
    }
}

/// The bottom of the slot machine's stack: `level_tally`, which the engine does not run yet; it
/// waits and changes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct SlotOver;

impl SlotOver {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        cx.unported("level_tally after the slot machine");
        Yield::Raster
    }
}

/// `access_overlay_show` (h0+$b6f8): the Accessing-disk overlay, unless one is up or no
/// trackloader runs.
fn access_overlay_show(cx: &mut Cx) {
    use crate::front::title::OVERLAY_PALETTE;
    if cx.s.l(at(0x445a)) == 0 && cx.s.l(OVERLAY_PALETTE) == 0 && cx.s.w(at(0x8)) != 0 {
        crate::front::title::overlay_show(cx);
    }
}

/// h0+$49f2..$4c58 with every load at once: `FRUT`, `FRUC` and `FRUM` into the display buffers,
/// the music faded out and stopped, the sound's chip RAM kept over the level's map, the slot's
/// effects (`LFFX`) at the bank's place and its music (`LFMU`) at the level's module address.
fn load(cx: &mut Cx) {
    for (name, to) in [
        ("FRUT", BUFFER_0),
        ("FRUC", BUFFER_1),
        ("FRUM", BUFFER_0 + 0xf000),
    ] {
        let bytes = cx.data.file(name).to_vec();
        cx.s.write(to, &bytes);
    }
    let Some(d) = cx.data.sound.as_ref() else {
        return;
    };
    // music_fade_wait: faded and stopped while the original loads.
    cx.s.audio.fade_out();
    let mut quiet = Vec::new();
    cx.s.audio.stop_music(d, &mut quiet);
    // h0+$4b44: the effect bank and the level's module as the driver sees chip RAM, over the
    // level's map.
    let kept: Vec<u8> = (0..SOUND_KEPT_LEN)
        .map(|i| cx.s.audio.mem.b(d, SOUND_KEPT + i))
        .collect();
    cx.s.write(SOUND_KEPT_AT, &kept);
    use crate::audio::File;
    use sf_assets::sound::{BANK_ADDR, BankId, TuneId};
    cx.s.audio.file_loaded(d, BANK_ADDR, File::Bank(BankId::Slot));
    cx.s.audio.sfx_bank_init(d);
    let module = cx.s.audio.module_addr();
    cx.s.audio.file_loaded(d, module, File::Tune(TuneId::Slot));
}

/// `slot_count_win` (h0+$50b6) up to its raster wait: the lamps off and drawn.
fn count_win_lamps(cx: &mut Cx) {
    for k in 0..LAMP_COUNT {
        cx.s.set_w(LAMPS + 8 * k, 0);
        cx.s.set_w(LAMPS + 8 * k + 6, 0);
    }
    cx.busy(LAMPS_OFF_CYCLES);
    draw_lamps(cx);
    if cx.s.l(WIN) >= 0x27100 {
        cx.unported("slot_count_win with a win of 160 000 or more");
    }
    let shown = cx.s.l(SCORE_SHOWN);
    cx.s.set_l(COUNT_SHOWN, shown);
    let win = cx.s.l(WIN);
    cx.s.set_l(SCORE_SHOWN, shown.wrapping_add(win));
    cx.s.set_w(COUNT_OVER, 0);
}

/// `slot_draw_lamps` (h0+$645e): each lamp's rectangle of the sixth plane, ORed with the lamps'
/// mask (`FRUM`) when lit, cleared under it when not; with the blitter given priority and the
/// copper's interrupt off.
pub(crate) fn draw_lamps(cx: &mut Cx) {
    use crate::audio::{DMACON, INTENA};
    cx.wr(INTENA, 0x0010);
    cx.wr(DMACON, 0x8400);
    cx.busy(LAMPS_START_CYCLES);
    for k in 0..LAMP_COUNT {
        let r = LAMP_RECTS + 12 * k;
        let (x, y, w, h) = (cx.w(r), cx.w(r + 2), cx.w(r + 4), cx.w(r + 6));
        let at = (x >> 3) as u32 + y as u32 * ROW;
        let words = w >> 4;
        let modulo = (ROW as u16).wrapping_sub(2 * words) as i16;
        let b = Blit {
            con0: if cx.s.w(LAMPS + 8 * k) != 0 { 0xdfc } else { 0xd0c },
            con1: 0,
            afwm: 0xffff,
            alwm: 0xffff,
            pt: [BUFFER_0 + 0xf000 + at, BUFFER_0 + 0xc800 + at, 0, BUFFER_0 + 0xc800 + at],
            modulo: [modulo, modulo, 0, modulo],
            size: h << 6 | words,
        };
        b.run(cx.s);
        cx.blit_wait(&b);
        cx.busy(LAMP_CYCLES);
    }
    cx.wr(INTENA, 0x8010);
}

/// h0+$5134..$514a before the print: the score as text (`number_to_text`), then h0+$5174's
/// `       0PAGE` with its digits right-aligned over the spaces (`PAGE` ends the text).
fn count_text(cx: &mut Cx) {
    let n = cx.s.l(COUNT_SHOWN);
    crate::front::attract::number_text(cx, n);
    let digits = at(0x161b2);
    let blanks = (0..8).filter(|&k| cx.s.b(digits + k) == 0).count() as u32;
    cx.s.write(COUNT_TEXT, b"       0PAGE");
    let mut k = 0;
    while cx.s.b(digits + k) != 0 {
        let c = cx.s.b(digits + k);
        cx.s.set_b(COUNT_TEXT + blanks + k, c);
        k += 1;
    }
}

/// The text printer (h0+$6f3a): `text`'s characters from the font at `to`, a byte each; `9`
/// moves on eight, `10` a line of eight rows, `COLORn` sets the colour, `PAGE` or a 0 after a
/// character not in its table ends it. Returns its 68000 cycles.
fn print_text(cx: &mut Cx, text: u32, to: u32) -> u32 {
    cx.s.set_w(TEXT_PLANE, 1);
    let (mut a0, mut a1, mut a5) = (text, to, to);
    let mut cycles = 0;
    loop {
        let c = cx.b(a0);
        a0 += 1;
        match c {
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
        if c == b'C' && (0..4).all(|k| cx.b(a0 + k) == b"OLOR"[k as usize]) {
            let n = cx.b(a0 + 4).wrapping_sub(b'0');
            cx.s.set_b(TEXT_COLOUR, n);
            a0 += 5;
            a1 += 6;
            continue;
        }
        if c == b'P' && (0..3).all(|k| cx.b(a0 + k) == b"AGE"[k as usize]) {
            return cycles;
        }
        // The character's place in the table; the search ends where the text does.
        let mut k = 0u32;
        let found = loop {
            cycles += CHAR_LOOK_CYCLES;
            if cx.b(CHARSET + k) == c {
                break true;
            }
            k += 1;
            if cx.b(a0) == 0 {
                break false;
            }
        };
        if !found {
            return cycles;
        }
        let glyph = FONT + k;
        let dst = if cx.s.w(TEXT_PLANE) == 2 { a1 + PLANE } else { a1 };
        for r in 0..8 {
            let v = cx.b(glyph + 0x50 * r);
            cx.s.set_b(dst + ROW * r, v);
        }
        cycles += GLYPH_CYCLES;
        a1 += 1;
    }
}

/// `slot_init` (h0+$661e): the reels at 0, $b, $16, hi-lo 1, the attract pattern from its start,
/// the lamps off and pointed at their rectangles.
fn slot_init(cx: &mut Cx) {
    cx.s.set_w(MENU, 0);
    for (k, v) in [0, 0xb, 0x16].into_iter().enumerate() {
        cx.s.set_w(REELS + 2 * k as u32, v);
    }
    cx.s.set_w(HILO, 1);
    cx.s.set_w(ATTRACT_PASSES, 0);
    cx.s.set_w(ATTRACT_LEFT, 3);
    cx.s.set_l(ATTRACT_ROW, ATTRACT_FIRST);
    for k in 0..LAMP_COUNT {
        cx.s.set_w(LAMPS + 8 * k, 0);
        cx.s.set_w(LAMPS + 8 * k + 6, 0);
        cx.s.set_l(LAMPS + 8 * k + 2, LAMP_RECTS + 12 * k);
    }
}

/// `slot_digits` (h0+$62f4): the hi-lo count (`--` for $ffff) and the coins, two digits each
/// (99 at most), copied a byte at a time from `FRUC`'s digits into the machine's six planes.
fn slot_digits(cx: &mut Cx) {
    let hilo = cx.s.w(HILO);
    let (tens, units) = if hilo == 0xffff {
        (10, 10)
    } else {
        split(hilo)
    };
    two_digits(cx, BUFFER_1 + 0x21c8, tens, units, BUFFER_0 + 0x1ae1);
    let coins = cx.s.w(COINS);
    let (tens, units) = split(coins);
    two_digits(cx, BUFFER_1 + 0x1e08, tens, units, BUFFER_0 + 0xf01);
}

/// A count's tens and units as `slot_digits` takes them: at most 99 (a negative word as it is).
fn split(n: u16) -> (u16, u16) {
    let n = if (n.wrapping_sub(0x63) as i16) < 0 { n } else { 0x63 };
    let tens = n / 10;
    (tens, n.wrapping_sub(tens.wrapping_mul(10)))
}

/// Two digits of 16×24 from `font` (a digit every two bytes) to `to`, in each of six planes.
fn two_digits(cx: &mut Cx, font: u32, tens: u16, units: u16, to: u32) {
    let off = |d: u16| (d.wrapping_add(d) as i16) as i32 as u32;
    for p in 0..6 {
        for (src, dst) in [
            (font.wrapping_add(off(tens)), to),
            (font.wrapping_add(off(units)), to + 2),
        ] {
            for r in 0..0x18 {
                for b in 0..2 {
                    let v = cx.b(src + p * PLANE + ROW * r + b);
                    cx.s.set_b(dst + p * PLANE + ROW * r + b, v);
                }
            }
        }
    }
}

/// The slot machine's routines on the original's call stack, when its main program is in
/// `slot_machine` (`level_tally`'s call, h0+$478e on the stack): the slot machine's start.
pub(crate) fn resume(chain: &[u32]) -> Option<Vec<(FrontRoutine, Option<Yield>)>> {
    chain.contains(&at(0x478e)).then(|| {
        vec![
            (FrontRoutine::SlotOver(SlotOver), None),
            (FrontRoutine::Slot(Routine::default()), None),
        ]
    })
}
