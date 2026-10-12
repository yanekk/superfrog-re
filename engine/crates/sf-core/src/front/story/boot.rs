//! Disk 3's boot program (`d3_c02`, `d3boot_main` h0+$2b326, docs/13 "The story intro"): the
//! Team 17 logo, a hires interlaced picture, fades in, stays while the intro and its chip hunks
//! load, and fades out; then the program unpacks and relocates the intro and starts it.
//!
//! The program paces itself on the beam alone: `wait_raster` (h0+$2b494) waits for line `$ff`,
//! then for a line whose low byte is `$20`, which is line `$120` of the same frame. Every step of
//! a fade and every palette copy happens there, below the picture.

use super::data::BOOT_BASE;
use super::fade::Rec;
use super::mem::StoryMem;
use super::{Cx, Wait};

/// `h0+$off` of the boot program.
pub const fn at(off: u32) -> u32 {
    BOOT_BASE + off
}

/// The two copper lists that alternate by field (each sets COP1LC to the other at line `$a0`),
/// and the black one set at the start and the end.
pub const LIST_A: u32 = at(0x2c00c);
pub const LIST_B: u32 = at(0x2c160);
pub const LIST_BLACK: u32 = at(0x2c2e8);
/// List A's colour moves, which the fades work on; `copy_palette` copies the first sixteen to
/// list B's.
const COLOURS_A: u32 = at(0x2c0b0);
const COLOURS_B: u32 = at(0x2c204);
/// The logo's sixteen colours, in the picture's data.
const PALETTE: u32 = at(0x28006);
/// The picture: four hires planes of 640 × 512 interlaced, `$a000` apart from h0+6; list B shows
/// the odd lines (from h0+$56).
const PICTURE: u32 = at(0x6);
const PLANE_BYTES: u32 = 0xa000;
/// The bitplane pointer moves of each list.
const PLANES_A: u32 = at(0x2c130);
const PLANES_B: u32 = at(0x2c284);
/// The COP1LC moves that chain the lists.
const CHAIN_A: u32 = at(0x2c154);
const CHAIN_B: u32 = at(0x2c2a8);
/// The program's one fade record.
pub const FADE: Rec = Rec {
    at: at(0x2be18),
    dirs: at(0x2be2a),
    steps: at(0x2be8c),
    accs: at(0x2bf4c),
};
/// What a fade step and the palette copy after it write: the record and both lists' colours.
/// The step can run past the frame's end (the display takes most of the bus), so they are
/// compared once it is over.
const STEP_WRITES: &[(u32, u32)] = &[(FADE.accs, 0xc0), (COLOURS_A, 0x80), (COLOURS_B, 0x40)];
/// Where the original's main program is: return addresses of its calls (for conformance).
pub const RET_FIRST_WAIT: u32 = at(0x2b3c6);
pub const RET_FADE_IN_STEP: u32 = at(0x2b41e);
pub const RET_UNPACK: u32 = at(0x2b45e);
pub const RET_FADE_OUT_STEP: u32 = at(0x2b47e);

/// Where the boot program is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boot {
    /// Set up, in its first `wait_raster`.
    Started,
    /// Colours cleared and the picture's BPLCON0 set: waiting for line `$a0` of a short frame.
    WaitField,
    /// In the fade-in's loop, about to step it.
    FadeIn,
    /// In the fade-out's loop, about to step it.
    FadeOut,
    /// Done: the intro takes over.
    Over,
}

/// The program's start (`d3boot_main` up to its first wait), done at power-on.
pub fn start(m: &mut StoryMem) -> u32 {
    let set_long = |m: &mut StoryMem, mv: u32, v: u32| {
        m.set_w(mv + 2, (v >> 16) as u16);
        m.set_w(mv + 6, v as u16);
    };
    for k in 0..4 {
        set_long(m, PLANES_A + 8 * k, PICTURE + PLANE_BYTES * k);
        set_long(m, PLANES_B + 8 * k, PICTURE + 0x50 + PLANE_BYTES * k);
    }
    set_long(m, CHAIN_A, LIST_B);
    set_long(m, CHAIN_B, LIST_A);
    FADE.start_in(m, COLOURS_A, PALETTE, 16);
    LIST_BLACK
}

/// `copy_palette` (h0+$2b4aa): list A's first sixteen colour moves into list B.
fn copy_palette(m: &mut StoryMem) {
    let bytes = m.read(COLOURS_A, 0x40);
    m.write(COLOURS_B, &bytes);
}

impl Boot {
    /// Run until the program waits beyond this frame.
    pub fn run(&mut self, cx: &mut Cx) -> Wait {
        loop {
            match self {
                Boot::Started => {
                    // The first wait_raster: line $ff, then $120.
                    if !cx.wait_bytes(&[0xff, 0x20]) {
                        return Wait::Frame;
                    }
                    // The colour registers cleared (the copper sets them all again) and the
                    // picture's BPLCON0 (hires, four planes, interlaced) written by the CPU.
                    cx.cpu_bplcon0(0xc204);
                    *self = Boot::WaitField;
                }
                Boot::WaitField => {
                    if !cx.wait_line(0xa0) || cx.lof {
                        return Wait::Frame;
                    }
                    cx.cop1lc = LIST_A;
                    if !cx.wait_bytes(&[0xff, 0x20]) {
                        return Wait::Frame;
                    }
                    copy_palette(cx.m);
                    *self = Boot::FadeIn;
                    return Wait::Frame;
                }
                Boot::FadeIn => {
                    if !cx.wait_bytes(&[0xff, 0x20]) {
                        return Wait::Frame;
                    }
                    if FADE.step(cx.m) {
                        cx.job(RET_FADE_IN_STEP, STEP_WRITES);
                    }
                    if !FADE.done(cx.m) {
                        copy_palette(cx.m);
                        return Wait::Frame;
                    }
                    // The intro and its chip hunks load (instant here); the intro is unpacked in
                    // place.
                    cx.loaded(crate::game::LoadEnd::Left(RET_UNPACK));
                    FADE.start_out(cx.m, COLOURS_A, 16);
                    copy_palette(cx.m);
                    *self = Boot::FadeOut;
                }
                Boot::FadeOut => {
                    if !cx.wait_bytes(&[0xff, 0x20]) {
                        return Wait::Frame;
                    }
                    if FADE.step(cx.m) {
                        cx.job(RET_FADE_OUT_STEP, STEP_WRITES);
                    }
                    if !FADE.done(cx.m) {
                        copy_palette(cx.m);
                        return Wait::Frame;
                    }
                    cx.cop1lc = LIST_BLACK;
                    *self = Boot::Over;
                    return Wait::Over;
                }
                Boot::Over => return Wait::Over,
            }
        }
    }
}
