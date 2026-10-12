//! The vertical-blank interrupt outside a level (`vbl_handler` h0+$11bf8 with `logic_running`
//! clear): sound, the mouse pointer, the keyboard, `display_timer`, the palette fades and the
//! title's palette copy.

use super::{Cx, fade, sprite};
use crate::audio::{INTENA, wr};
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
        cx.unported("VBL with h0+$9f8 set ($11e62: the end sequence's frog)");
    }
    mouse(cx);
    if let Some(d) = data {
        cx.s.audio.music_tick(d, cx.paula);
        cx.s.audio.music_dma_tick(d, cx.paula);
        cx.s.audio.sfx_tick(d, cx.paula);
    }
    if cx.s.l(h0(0x4112)) != 0 {
        cx.unported("progress_map_vbl");
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
    let joy0dat = 0u16;
    cx.s.set_w(MOUSE, joy0dat);
    let x = cx.s.w(MOUSE + 2) as i16;
    let x = x.clamp(0, 0x13f);
    let x = if x <= 0 { 0 } else { x };
    cx.s.set_w(MOUSE + 2, x as u16);
    let y = cx.s.w(MOUSE + 4) as i16;
    let y = if y <= 0xc8 { 0xc8 } else { y };
    let y = if (y as u16) >= 0xf0 { 0xf0 } else { y };
    cx.s.set_w(MOUSE + 4, y as u16);
    cx.s.set_w(POINTER, x as u16);
    cx.s.set_w(POINTER + 2, y as u16);
    sprite::update(cx.s, POINTER);
}

/// `kbd_poll` (`$15ff2`): every second frame, the keyboard's last byte into `key_down`/`key_up`.
fn kbd_poll(cx: &mut Cx) {
    let t = !cx.s.b(KBD_TOGGLE);
    cx.s.set_b(KBD_TOGGLE, t);
    if t == 0 {
        return;
    }
    wr(cx.paula, INTENA, 0x0008);
    let (code, up) = cx.keyboard.last;
    if up {
        cx.s.set_b(KEY_UP, code);
        cx.s.set_b(KEY_DOWN, 0);
    } else {
        cx.s.set_b(KEY_DOWN, code);
        cx.s.set_b(KEY_UP, 0);
    }
    wr(cx.paula, INTREQ, 0x0008);
}
