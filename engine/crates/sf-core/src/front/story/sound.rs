//! The intro's sound (docs/13 "Sound"): to come. These keep the main program's calls in place.

use sf_paula::PaulaWrite;

use super::mem::StoryMem;
use super::h0;

/// `timeline_ptr` and `timeline_wait` (h0+$b1f4, $b1f8).
const TIMELINE: u32 = h0(0xb1f4);
const TIMELINE_WAIT: u32 = h0(0xb1f8);
/// `music_on` (h0+$39296).
const MUSIC_ON: u32 = h0(0x39296);

/// `timeline_start` (h0+$b17c).
pub fn timeline_start(m: &mut StoryMem, list: u32) {
    m.set_l(TIMELINE, list);
    m.set_w(TIMELINE_WAIT, m.w(list + 2));
}

pub fn music_start(_m: &mut StoryMem, _module: u32, _samples: u32, _w: &mut dyn FnMut(u16, u16)) {}

pub fn music_stop(m: &mut StoryMem, w: &mut dyn FnMut(u16, u16)) {
    m.set_w(MUSIC_ON, 0);
    m.set_w(h0(0x3a556), 0);
    w(0x96, 0x000f);
}

pub fn music_fade(m: &mut StoryMem) {
    m.set_w(h0(0x3a556), 0x2001);
}

pub fn music_on(m: &StoryMem) -> bool {
    m.w(MUSIC_ON) != 0
}

pub fn sfx_play(_m: &mut StoryMem, _d0: u16, _d1: u16, _w: &mut dyn FnMut(u16, u16)) {}

pub fn stream_tick(_m: &mut StoryMem, _paula: &mut Vec<PaulaWrite>) {}

pub fn tick(_m: &mut StoryMem, _paula: &mut Vec<PaulaWrite>) {}
