//! The intro's vertical-blank handler (`intro_vbl` h0+$b102), every frame at the top: the frame's
//! flag and count for the main program, the bitplane pointers of the buffer to show, the music's
//! stream, the fades' steps, fire or the mouse button for `intro_skip`, the music, the effects
//! and the timeline of sound events.

use sf_paula::PaulaWrite;

use super::bobs::{PLANE, PLANES_COPPER, SHOWN};
use super::intro::{FADES, FROZEN, SKIP, VBL_COUNT, VBL_FLAG};
use super::mem::StoryMem;
use crate::input::{FrameInput, Joy};

/// A fade record's step in the handler: per colour, its copper word's address and how many of its
/// three components changed.
pub struct FadeStep {
    /// Each colour word's address, value before and value after.
    pub colours: Vec<(u32, u16, u16)>,
}

/// DMA cycles from the handler's start to its first fade step's first colour, and per colour of
/// the step: a fixed part and a part per component compared, per component changed (`fade_step`
/// h0+$b7d2's loop, from its instructions' 68000 cycles, two CPU cycles a DMA cycle).
const FADE_START: u32 = 1750;
const FADE_PER_COLOUR: u32 = 15;
const FADE_PER_SAME: u32 = 39;
const FADE_PER_CHANGED: u32 = 76;

/// The writes the handler starting on line `start` makes to the colour words its fade steps
/// change, one per component (red, green, blue in turn): `(address, line, DMA cycle, value)`.
pub fn fade_write_times(steps: &[FadeStep], start: u16) -> Vec<(u32, u16, u16, u16)> {
    let l = crate::render::copper::LINE_CYCLES as u32;
    let mut t = start as u32 * l + FADE_START;
    let mut out = Vec::new();
    for s in steps {
        for &(addr, old, new) in &s.colours {
            t += FADE_PER_COLOUR;
            let mut v = old;
            for mask in [0xf00u16, 0xf0, 0xf] {
                if old & mask != new & mask {
                    t += FADE_PER_CHANGED;
                    v = v & !mask | new & mask;
                    out.push((addr, (t / l) as u16, (t % l) as u16, v));
                } else {
                    t += FADE_PER_SAME;
                }
            }
        }
    }
    out
}

/// One run of the handler. Its Paula writes go to `paula`, in order. Returns the fade steps it
/// made, for the copper's view of them.
pub fn run(
    m: &mut StoryMem,
    sd: &sf_assets::sound::SoundData,
    input: FrameInput,
    paula: &mut Vec<PaulaWrite>,
) -> Vec<FadeStep> {
    m.set_w(VBL_FLAG, 1);
    let c = m.w(VBL_COUNT).wrapping_add(1);
    m.set_w(VBL_COUNT, c);
    // `planes_show` (h0+$b438): the five planes of the buffer to show, from its second word.
    if m.w(FROZEN) == 0 {
        let at = m.l(SHOWN).wrapping_add(2);
        for k in 0..5u32 {
            let v = at.wrapping_add(PLANE * k);
            m.set_w(PLANES_COPPER + 8 * k + 6, v as u16);
            m.set_w(PLANES_COPPER + 8 * k + 2, (v >> 16) as u16);
        }
    }
    super::sound::stream_tick(m, paula);
    // `fades_step` (h0+$b7bc, $ba42, $b5c0): fades in, out and across, each record once.
    let mut steps = Vec::new();
    for kind in [2, 3, 1] {
        for r in &FADES {
            if m.w(r.at + super::fade::KIND) == kind {
                let n = m.w(r.at + super::fade::COUNT) as u32;
                let list = m.l(r.at + super::fade::LIST);
                // The colour words: the fades in and out address list - 2 + 4 (k + 1), across
                // list + 2 + 4 k: both the word after each move's register.
                let first = if kind == 1 { list + 2 } else { list + 4 };
                let before: Vec<u16> = (0..n).map(|k| m.w(first + 4 * k)).collect();
                if r.step(m) || m.w(r.at + super::fade::COUNTER) == 0 {
                    let colours = (0..n)
                        .map(|k| (first + 4 * k, before[k as usize], m.w(first + 4 * k)))
                        .collect();
                    steps.push(FadeStep { colours });
                }
            }
        }
    }
    // CIA-A PRA bits 6 (the mouse button, port 1) and 7 (fire, port 2), low when pressed.
    if input.joy.contains(Joy::PORT1_BUTTON) || input.joy.contains(Joy::FIRE) {
        m.set_w(SKIP, 1);
    }
    super::sound::tick(m, sd, paula);
    paula.push(PaulaWrite {
        reg: 0x9c,
        value: 0x20,
        hpos: 0,
        vpos: 0,
    });
    steps
}
