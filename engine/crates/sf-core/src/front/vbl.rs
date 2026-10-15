//! The vertical-blank interrupt outside a level (`vbl_handler` h0+$11bf8 with `logic_running`
//! clear): sound, the mouse pointer, the keyboard, `display_timer`, the palette fades and the
//! title's palette copy.

use super::BEAM_CYCLES_PER_LINE;
use super::{Cx, fade, sprite};
use crate::audio::{INTENA, TickWork, wr};
use crate::layout::h0;

/// `h0+$11b64`: the last JOY0DAT read; `$11b66`, `$11b68`: the pointer's x and y.
const MOUSE: u32 = h0(0x11b64);
/// The pointer's sprite record.
const POINTER: u32 = h0(0x1b10c);
pub const DISPLAY_TIMER: u32 = h0(0x123d4);
const KBD_TOGGLE: u32 = h0(0x16076);
pub const KEY_DOWN: u32 = h0(0x16078);
pub const KEY_UP: u32 = h0(0x16079);
/// Set while the title's fades run: the VBL copies their palettes into the copper list.
pub const TITLE_FADING: u32 = h0(0x9e0);
pub const TITLE_TOGGLE: u32 = h0(0x3ca0);
/// The line the title's palette copy waits for (h0+$3b54).
pub const PALETTE_LINE: u16 = 0x7c;
const INTREQ: u16 = 0x9c;

/// The handler's work up to the fades. Returns whether the title's palette copy is due this frame
/// (it waits for line [`PALETTE_LINE`]; [`palette_copy`]).
pub(super) fn run(cx: &mut Cx) -> bool {
    let data = cx.data.sound.as_ref();
    if let Some(d) = data {
        cx.s.audio.stream_tick(d, cx.paula);
    }
    if cx.s.w(h0(0x9f8)) != 0 {
        if cx.s.l(super::chain::INTRO_SHOWN) == 0 {
            cx.unported("VBL with h0+$9f8 set ($11e62: the end sequence's frog)");
        }
        super::chain::exit_walk_vbl(cx);
    }
    mouse(cx);
    if let Some(d) = data {
        cx.s.audio.music_tick(d, cx.paula);
        cx.s.audio.music_dma_tick(d, cx.paula);
        cx.s.audio.sfx_tick(d, cx.paula);
    }
    if cx.s.l(super::chain::MAP_SHOWN) != 0 {
        super::chain::progress_map_vbl(cx);
    }
    kbd_poll(cx);
    let t = cx.s.w(DISPLAY_TIMER);
    if (t as i16) >= 0 {
        cx.s.set_w(DISPLAY_TIMER, t.wrapping_sub(1));
    }
    if cx.s.w(TITLE_FADING) == 0 || cx.s.w(TITLE_TOGGLE) != 0 {
        fade::step_in(cx.s);
        fade::step_out(cx.s);
        fade::step_set(cx.s);
    }
    // $3b22: on the title, every other frame while a fade runs.
    if cx.s.w(TITLE_FADING) == 0 {
        return false;
    }
    let t = !cx.s.w(TITLE_TOGGLE);
    cx.s.set_w(TITLE_TOGGLE, t);
    if t == 0 {
        return false;
    }
    !(fade::done(cx.s, fade::RECORDS[0]) && fade::done(cx.s, fade::RECORDS[1]))
}

/// `$3b5e`: the title's two fade palettes (kept as copper MOVEs at h0+$3ba0 and $3c20) into the
/// title's copper list.
pub(super) fn palette_copy(cx: &mut Cx) {
    for (from, to) in [
        (h0(0x3ba2), super::h2(0x7236)),
        (h0(0x3c22), super::h2(0x733e)),
    ] {
        for k in 0..32 {
            let v = cx.s.w(from + 4 * k);
            cx.s.set_w(to + 4 * k, v);
        }
    }
}

/// The end of the handler: the interrupt acknowledged.
pub(super) fn end(cx: &mut Cx) {
    wr(cx.paula, INTREQ, 0x20);
}

/// `$11aa6` and the pointer's sprite: JOY0DAT never moves on the oracle (no mouse movement is
/// scripted), so only the clamps change anything.
fn mouse(cx: &mut Cx) {
    pointer(cx.s);
}

/// [`mouse`] on the state alone: the paused level's vertical blank runs it too (T191).
pub(crate) fn pointer(s: &mut crate::state::State) {
    let joy0dat = 0u16;
    s.set_w(MOUSE, joy0dat);
    let x = s.w(MOUSE + 2) as i16;
    let x = x.clamp(0, 0x13f);
    let x = if x <= 0 { 0 } else { x };
    s.set_w(MOUSE + 2, x as u16);
    let y = s.w(MOUSE + 4) as i16;
    let y = if y <= 0xc8 { 0xc8 } else { y };
    let y = if (y as u16) >= 0xf0 { 0xf0 } else { y };
    s.set_w(MOUSE + 4, y as u16);
    s.set_w(POINTER, x as u16);
    s.set_w(POINTER + 2, y as u16);
    sprite::update(s, POINTER);
}

/// `kbd_poll` (`$15ff2`): every second frame, the keyboard's last byte into `key_down`/`key_up`.
fn kbd_poll(cx: &mut Cx) {
    let t = !cx.s.b(KBD_TOGGLE);
    cx.s.set_b(KBD_TOGGLE, t);
    if t == 0 {
        return;
    }
    wr(cx.paula, INTENA, 0x0008);
    let d0 = !cx.keyboard.sdr;
    let (code, up) = (d0 >> 1 & 0x7f, d0 & 1 == 1);
    if std::mem::take(&mut cx.keyboard.new) {
        cx.keyboard.sdr = 0xff;
    }
    if up {
        cx.s.set_b(KEY_UP, code);
        cx.s.set_b(KEY_DOWN, 0);
    } else {
        cx.s.set_b(KEY_DOWN, code);
        cx.s.set_b(KEY_UP, 0);
    }
    wr(cx.paula, INTREQ, 0x0008);
}

/// How long the handler's work up to its fades takes (T100, measured on `frog-pit-w1-1`'s exit
/// fades and fade ins, frames 278-332 and 613-667): `music_tick` from the handler's entry, in CPU
/// cycles, to its last write to Paula, by what the tick does ([`TickWork`]; 102 frames, 18 cycles
/// RMS). Effects not seen there are estimated from their code against the measured ones.
mod cost {
    /// Entry, `sample_stream` idle, the mouse and its sprite, `p41_play`'s set-up.
    pub const BASE: u32 = 521;
    /// `sample_stream` stepping a sample on channel 3.
    pub const STREAM: u32 = 1566;
    /// A row: its set-up, each channel with a row, each starting a note.
    pub const ROW: u32 = 339;
    pub const ROW_LIVE: u32 = 169;
    pub const ROW_NOTE: u32 = 170;
    /// Between rows, a channel's effect step by effect (`p41_fx_tab`): 0, 3, 4, 6 and A measured.
    pub const EFFECT: [u32; 16] = [
        84, 200, 200, 134, 279, 300, 376, 280, 250, 84, 164, 84, 84, 84, 150, 84,
    ];
    /// `fetch_rows`: its set-up, then per channel an empty row, a repeated row, a row read (more
    /// for a reference), an instrument taken.
    pub const FETCH: u32 = 116;
    pub const FETCH_EMPTY: u32 = 67;
    pub const FETCH_REPEAT: u32 = 170;
    pub const FETCH_READ: u32 = 227;
    pub const FETCH_REF: u32 = 55;
    pub const FETCH_INST: u32 = 55;
    /// The shadow copy, per channel music holds (five Paula writes).
    pub const SHADOW: u32 = 519;
    /// From the last write to `h0+$12a14`'s read of the line (measured between 69 and 113).
    pub const TO_WAIT: u32 = 90;
    /// From the line `$12a14` waits for to the first fade's first channel: `music_dma`,
    /// `sfx_tick`, `progress_map_vbl`, `display_timer` and the fades' dispatch; more when
    /// `kbd_poll` runs and when `music_dma` starts a row's notes (T94 measured the same two on
    /// the slot screen).
    pub const AFTER_WAIT: u32 = 170;
    pub const KEYBOARD: u32 = 454;
    pub const ROW_DMA: u32 = 775;
}

/// The cycles from the handler's entry to the end of `music_tick` ([`TickWork`]).
fn music_cycles(w: &TickWork, streaming: bool) -> u32 {
    let mut c = cost::BASE + if streaming { cost::STREAM } else { 0 };
    if !w.on {
        return c;
    }
    if w.row {
        c += cost::ROW + w.row_live * cost::ROW_LIVE + w.row_notes * cost::ROW_NOTE;
    } else {
        c += (0..16).map(|e| w.effects[e] * cost::EFFECT[e]).sum::<u32>();
        if w.fetch {
            c += cost::FETCH
                + w.fetch_empty * cost::FETCH_EMPTY
                + w.fetch_repeat * cost::FETCH_REPEAT
                + w.fetch_read * cost::FETCH_READ
                + w.fetch_ref * cost::FETCH_REF
                + w.fetch_inst * cost::FETCH_INST;
        }
    }
    c + w.shadowed * cost::SHADOW
}

/// Where in the frame (CPU cycles from its top) the handler entered on `entry` starts its fades.
/// After `music_tick` it waits at `h0+$12a14` until eight lines after the one it reads, so the
/// fades start on a line boundary decided by how long the music took (T100: the colours the
/// copper reads on line `$34` mid-fade depend on it).
pub(super) fn fade_start(entry: u16, music: &TickWork, streaming: bool, keyboard: bool) -> u32 {
    let read =
        (entry as u32 - 1) * BEAM_CYCLES_PER_LINE + music_cycles(music, streaming) + cost::TO_WAIT;
    let line = read / BEAM_CYCLES_PER_LINE + 8;
    line * BEAM_CYCLES_PER_LINE
        + cost::AFTER_WAIT
        + if keyboard { cost::KEYBOARD } else { 0 }
        + if music.row { cost::ROW_DMA } else { 0 }
}

/// From [`fade_start`] to the acknowledge when no fade is due (T100: the oracle's acknowledges
/// of `level_init`'s frames, 1 492 to 1 510).
pub(super) const NO_FADE_TAIL: u32 = 1_500;

/// Whether this frame's handler polls the keyboard ([`kbd_poll`] runs every second frame).
pub(super) fn polls_keyboard(cx: &Cx) -> bool {
    cx.s.b(KBD_TOGGLE) == 0
}

#[cfg(test)]
mod tests {
    use super::{cost, fade_start};
    use crate::audio::TickWork;
    use crate::front::BEAM_CYCLES_PER_LINE as LINE;

    fn tick(effects: &[usize], shadowed: u32) -> TickWork {
        let mut w = TickWork {
            on: true,
            shadowed,
            ..TickWork::default()
        };
        for &e in effects {
            w.effects[e] += 1;
        }
        w
    }

    /// The fades start eight lines after the line `music_tick` ends on (`h0+$12a14`'s wait), so a
    /// longer tick moves them a whole line or not at all.
    #[test]
    fn the_fades_start_eight_lines_after_the_music() {
        // Four channels, no effect: 521 + 4 × 84 + 4 × 519 + 90 = 3 023 cycles, line 6.
        let w = tick(&[0, 0, 0, 0], 4);
        assert_eq!(fade_start(1, &w, false, false), 14 * LINE + cost::AFTER_WAIT);
        // A vibrato and a volume slide more: line 7 (frame 644's).
        let w = tick(&[4, 0, 0, 10, 0, 0], 4);
        assert_eq!(fade_start(1, &w, false, false), 15 * LINE + cost::AFTER_WAIT);
        // No channel playing: line 5 (frames 314-317's).
        let w = tick(&[], 4);
        assert_eq!(fade_start(1, &w, false, false), 13 * LINE + cost::AFTER_WAIT);
    }

    /// The keyboard's poll and a row's notes started come after the wait, so they add their time
    /// whole; a handler entered late starts everything late.
    #[test]
    fn the_keyboard_and_a_row_come_after_the_wait() {
        let w = tick(&[0, 0, 0, 0], 4);
        let base = fade_start(1, &w, false, false);
        assert_eq!(fade_start(1, &w, false, true), base + cost::KEYBOARD);
        let row = TickWork {
            row: true,
            ..TickWork { on: true, shadowed: 4, ..TickWork::default() }
        };
        // A row with no channel playing: 521 + 339 + 4 × 519 + 90 = 3 026 cycles, line 6.
        let start = fade_start(1, &row, false, false);
        assert_eq!(start, 14 * LINE + cost::AFTER_WAIT + cost::ROW_DMA);
        assert_eq!(fade_start(31, &w, false, false), base + 30 * LINE);
    }
}
