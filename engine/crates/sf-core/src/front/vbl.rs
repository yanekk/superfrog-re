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

/// The handler's work on a frame the original spends loading that no screen times (T209): the
/// sound and the keyboard's poll, which run on whatever the main program is doing. A timed
/// screen's own work is run frame by frame as play; the rest waits for the load's end.
/// Returns where its sound writes fall ([`place`]).
pub(super) fn load_blank(cx: &mut Cx) -> Option<SoundMarks> {
    let mut marks = None;
    if let Some(d) = cx.data.sound.as_ref() {
        let (work, streaming) = (cx.s.audio.tick_work(d), cx.s.audio.streaming());
        cx.s.audio.stream_tick(d, cx.paula);
        let music = cx.paula.len();
        cx.s.audio.music_tick(d, cx.paula);
        let dma = cx.paula.len();
        cx.s.audio.music_dma_tick(d, cx.paula);
        cx.s.audio.sfx_tick(d, cx.paula);
        marks = Some(SoundMarks { music, dma, work, streaming });
    }
    kbd_poll(cx);
    marks
}

/// Where the handler's sound writes fall among its writes (T213): `music_tick`'s first and the DMA
/// stage's first (indices into the frame's writes), and what the tick does, read before it.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SoundMarks {
    pub music: usize,
    pub dma: usize,
    pub work: TickWork,
    pub streaming: bool,
}

/// The beam positions (`vpos`, `hpos`) of the handler's writes `from..to` (to its last before
/// the acknowledge), the handler entered on line `entry` (1 on time), by the cost model of
/// [`fade_start`] (T100): the writes before `music_tick`'s at its entry, `music_tick`'s spread
/// over it with the shadow copy's burst at its end, the DMA stage's and all after once the wait at
/// `h0+$12a14` is over, eight lines after `music_tick`'s.
///
/// A channel the replayer turns off and the DMA stage on again is idle by then, so Paula restarts
/// its note as on the original. The engine made them all at one instant before T213, and the
/// note was lost: the channel played its old sample on.
pub(super) fn place(from: usize, to: usize, m: SoundMarks, entry: u16) -> Vec<(u16, u16)> {
    let top = entry.max(1) as u32 - 1;
    let top = top * BEAM_CYCLES_PER_LINE;
    let m0 = top + cost::BASE + if m.streaming { cost::STREAM } else { 0 };
    let m1 = (top + music_cycles(&m.work, m.streaming)).max(m0);
    let wait = ((m1 + cost::TO_WAIT) / BEAM_CYCLES_PER_LINE + 8) * BEAM_CYCLES_PER_LINE;
    let (music, dma) = (m.music.clamp(from, to), m.dma.clamp(from, to));
    let mut at = Vec::with_capacity(to - from);
    for i in from..music {
        at.push((top + cost::ENTRY + (i - from) as u32 * cost::WRITE).min(m0));
    }
    let n = (dma - music) as u32;
    let burst = n.min(5 * m.work.shadowed);
    let (rest, copy) = (n - burst, m1 - burst * cost::COPY_WRITE);
    for k in 0..n {
        at.push(if k < rest {
            m0 + (copy.saturating_sub(m0)) * k / rest
        } else {
            copy + (k - rest) * cost::COPY_WRITE
        });
    }
    for i in dma..to {
        at.push(wait + cost::DMA_STAGE + (i - dma) as u32 * cost::WRITE);
    }
    let mut last = 0;
    at.into_iter()
        .map(|t| {
            last = t.max(last);
            ((last / BEAM_CYCLES_PER_LINE) as u16, (last % BEAM_CYCLES_PER_LINE / 2) as u16)
        })
        .collect()
}

/// The handler's work up to the fades. Returns whether the title's palette copy is due this frame
/// (it waits for line [`PALETTE_LINE`]; [`palette_copy`]), and where its sound writes fall
/// ([`place`]).
pub(super) fn run(cx: &mut Cx) -> (bool, Option<SoundMarks>) {
    let data = cx.data.sound.as_ref();
    let mut marks = data.map(|d| SoundMarks {
        work: cx.s.audio.tick_work(d),
        streaming: cx.s.audio.streaming(),
        ..SoundMarks::default()
    });
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
    if let (Some(d), Some(m)) = (data, marks.as_mut()) {
        m.music = cx.paula.len();
        cx.s.audio.music_tick(d, cx.paula);
        m.dma = cx.paula.len();
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
        return (false, marks);
    }
    let t = !cx.s.w(TITLE_TOGGLE);
    cx.s.set_w(TITLE_TOGGLE, t);
    if t == 0 {
        return (false, marks);
    }
    let due = !(fade::done(cx.s, fade::RECORDS[0]) && fade::done(cx.s, fade::RECORDS[1]));
    (due, marks)
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
    /// T213, the title's frames 200-230 and 514-518 on the oracle: the handler's first write, the
    /// copper interrupt's acknowledge or `sample_stream`'s; any write but the shadow copy's (a
    /// level's `Cost::AudioWrite`); the shadow copy's, a burst at `music_tick`'s end (20 writes
    /// in 188 clocks); and from the line `$12a14` waits for to the DMA stage's DMACON (147 clocks
    /// on).
    pub const ENTRY: u32 = 100;
    pub const WRITE: u32 = 88;
    pub const COPY_WRITE: u32 = 19;
    pub const DMA_STAGE: u32 = 294;
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

    /// A row's writes for channel 0, as the handler makes them: `row_note`'s DMACON off and the
    /// shadow copy's new sample (`music_tick`), then the DMA stage's DMACON on.
    fn retrigger() -> (Vec<sf_paula::PaulaWrite>, super::SoundMarks) {
        let w = |reg, value| sf_paula::PaulaWrite { reg, value, hpos: 0, vpos: 0 };
        let writes = vec![
            w(0x96, 0x0001),
            w(0xa0, 0),
            w(0xa2, 0x1000),
            w(0xa4, 0x400),
            w(0xa6, 400),
            w(0xa8, 64),
            w(0x96, 0x8001),
            w(0x9a, 0x4000),
        ];
        let work = TickWork { on: true, row: true, row_live: 1, row_notes: 1, shadowed: 1, ..TickWork::default() };
        (writes, super::SoundMarks { music: 0, dma: 6, work, streaming: false })
    }

    /// The handler's writes, placed: the DMA stage on the eighth line after the one
    /// `music_tick`'s last write is on, in order, none going back.
    #[test]
    fn the_dma_stage_is_placed_after_the_wait() {
        let (writes, m) = retrigger();
        let at = super::place(0, writes.len(), m, 1);
        let clock = |(v, h): (u16, u16)| v as u32 * 227 + h as u32;
        assert!(at.windows(2).all(|p| clock(p[0]) <= clock(p[1])), "{at:?}");
        assert_eq!(at[6].0, at[5].0 + 8, "{at:?}");
        // Entered late, all of it later.
        let late = super::place(0, writes.len(), m, 31);
        assert_eq!(late[6].0, at[6].0 + 30);
    }

    /// T213: a note retriggered on a channel still partway through a long sample restarts at the
    /// placed positions, as on the original; its off and on at one instant (the engine's before
    /// T213) leave the channel playing the old sample on.
    #[test]
    fn a_note_retriggered_mid_sample_restarts() {
        struct Chip;
        impl sf_paula::ChipMemory for Chip {
            fn chip_word(&self, addr: u32) -> u16 {
                // The old sample (16s) below $1000, the new one (-32s) from there.
                if addr < 0x1000 { 0x1010 } else { 0xe0e0 }
            }
        }
        let w = |reg, value| sf_paula::PaulaWrite { reg, value, hpos: 0, vpos: 0 };
        // 2 048 bytes at period 400: about 11 frames of the old sample.
        let first = [w(0xa0, 0), w(0xa2, 0), w(0xa4, 0x400), w(0xa6, 400), w(0xa8, 64), w(0x96, 0x8001)];
        let (writes, m) = retrigger();
        let placed: Vec<_> = writes
            .iter()
            .zip(super::place(0, writes.len(), m, 1))
            .map(|(w, (vpos, hpos))| sf_paula::PaulaWrite { vpos, hpos, ..*w })
            .collect();
        let third_frame = |retrigger: &[sf_paula::PaulaWrite]| {
            let mut p = sf_paula::Paula::booted(48_000);
            let mut out = Vec::new();
            for f in [&first[..], retrigger, &[]] {
                p.frame(f, &Chip, &mut out);
            }
            p.channel_taps()[0].to_vec()
        };
        let (new, old) = (-32 * 64, 16 * 64);
        assert!(third_frame(&placed).iter().all(|&v| v == new));
        assert!(third_frame(&writes).iter().all(|&v| v == old));
    }
}
