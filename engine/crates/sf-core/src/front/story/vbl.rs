//! The intro's vertical-blank handler (`intro_vbl` h0+$b102), every frame at the top: the frame's
//! flag and count for the main program, the bitplane pointers of the buffer to show, the music's
//! stream, the fades' steps, fire or the mouse button for `intro_skip`, the music, the effects
//! and the timeline of sound events.

use sf_paula::PaulaWrite;

use super::bobs::{PLANE, PLANES_COPPER, SHOWN};
use super::intro::{FADES, FROZEN, SKIP, VBL_COUNT, VBL_FLAG};
use super::mem::StoryMem;
use crate::input::{FrameInput, Joy};

/// One run of the handler. Its Paula writes go to `paula`, in order.
pub fn run(m: &mut StoryMem, input: FrameInput, paula: &mut Vec<PaulaWrite>) {
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
    for kind in [2, 3, 1] {
        for r in &FADES {
            if m.w(r.at + super::fade::KIND) == kind {
                r.step(m);
            }
        }
    }
    // CIA-A PRA bits 6 (the mouse button, port 1) and 7 (fire, port 2), low when pressed.
    if input.joy.contains(Joy::PORT1_BUTTON) || input.joy.contains(Joy::FIRE) {
        m.set_w(SKIP, 1);
    }
    super::sound::tick(m, paula);
    paula.push(PaulaWrite {
        reg: 0x9c,
        value: 0x20,
        hpos: 0,
        vpos: 0,
    });
}
