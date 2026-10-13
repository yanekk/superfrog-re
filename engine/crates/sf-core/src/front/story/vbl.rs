//! The intro's vertical-blank handler (`intro_vbl` h0+$b102), every frame at the top: the frame's
//! flag and count for the main program, the bitplane pointers of the buffer to show, the music's
//! stream, the fades' steps, fire or the mouse button for `intro_skip`, the music, the effects
//! and the timeline of sound events.

use sf_paula::PaulaWrite;

use super::bobs::{PLANE, PLANES_COPPER, SHOWN};
use super::cpu::{self, op};
use super::fade::Walk;
use super::h0;
use super::intro::{FADES, FROZEN, SKIP, VBL_COUNT, VBL_FLAG};
use super::mem::StoryMem;
use crate::input::{FrameInput, Joy};

/// What the handler's code went through this frame, for the timing of its writes to the copper
/// list ([`colour_writes`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handler {
    /// `planes_frozen` was set: `planes_show` returned at once.
    pub frozen: bool,
    pub stream: Stream,
    /// The fades in, out and across (`fades_step` h0+$b7bc, $ba42, $b5c0): per call, each
    /// record's path.
    pub calls: Vec<(u16, Vec<Walk>)>,
}

/// The sample stream's tick (h0+$3a65a).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Off,
    /// It copies the next 160 bytes; `ends`: they were the last.
    On {
        ends: bool,
    },
}

/// The interrupt's entry (the exception, Moira's 68000 order) and `intro_vbl` up to its call of
/// the fades: saving the registers, the frame flag and count, `planes_show`'s call.
const ENTRY: &[&str] = &[
    "nnnwnnnnwwrrpp",
    "pwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwp",
    "ppprp",
    op::BCC_W_NOT,
    "pppwp",
    "pprpw",
    op::BSR,
];

/// The cycle of each write the handler starting at cycle `entry` (its interrupt's first cycle)
/// makes to a fade's colour words: `(address, cycle, value)`, the component cleared then set.
pub fn colour_writes(h: &Handler, entry: u32, bus: &dyn cpu::Bus) -> Vec<(u32, u32, u16)> {
    let mut c = cpu::Cpu::new(entry, bus);
    for o in ENTRY {
        c.run(o);
    }
    // `planes_show` (h0+$b438): `tst.w abs.l`.
    c.run("pprp");
    if h.frozen {
        c.run(op::BCC_TAKEN);
    } else {
        c.run(op::BCC_B_NOT);
        // `move.l abs.l, d1`, `lea abs.l, a1`, `move.l #i, d3`, `addq.l #2, d1`.
        c.run("pprrp ppp ppp pnn");
        // Per plane: two `move.w d1, d(a1)` and two `swap`, `addq.l #8, a1`, `add.l d3, d1`;
        // the fifth plane's ends after its second move.
        for k in 0..5 {
            c.run("pwp p pwp");
            if k < 4 {
                c.run("p pnn pnn");
            }
        }
    }
    c.run(op::RTS);
    // The stream's tick: `jsr`, `cmpi.w #i, abs.l`.
    c.run("pwwpp ppprp");
    match h.stream {
        Stream::Off => c.run(op::BCC_TAKEN),
        Stream::On { ends } => {
            c.run(op::BCC_W_NOT);
            // Two `move.w #i, abs.l`, `movea.l abs.l`, `lea abs.l`.
            c.run("pppwp pppwp pprrp ppp");
            for k in 0..4 {
                c.run(&format!("p{}p", "r".repeat(25)));
                c.run(&format!("p{}p", "w".repeat(24)));
                if k < 3 {
                    c.run("ppnn");
                }
            }
            // `addi.l #i, abs.l`, `subi.w #i, abs.l`, `tst.w abs.l`.
            c.run("pppprrpww ppprpw pprp");
            if ends {
                c.run(op::BCC_TAKEN);
                // Two `move.w #i, abs.l`, `clr.w abs.l`, `move.w #i`, `move.b #i, abs.l`.
                c.run("pppwp pppwp pprpw pppwp pppwp");
            } else {
                c.run(op::BCC_B_NOT);
                // `move.l #i, abs.l`, `move.w abs.l, abs.l`, three `move.w #i, abs.l`.
                c.run("ppppwwp pprpwpp pppwp pppwp pppwp");
            }
        }
    }
    c.run(op::RTS);
    let mut out = Vec::new();
    for (kind, walks) in &h.calls {
        c.run(op::BSR);
        for (r, w) in walks.iter().enumerate() {
            c.run(op::LEA_PC);
            if r < 3 {
                c.run(op::BSR);
            }
            fade_call(&mut c, *kind, w, &mut out);
        }
    }
    out
}

/// One record's turn in a call of the fade of `kind`.
fn fade_call(c: &mut cpu::Cpu, kind: u16, w: &Walk, out: &mut Vec<(u32, u32, u16)>) {
    c.run(op::CMPI_W_DI);
    if *w == Walk::Other {
        c.run(op::BCC_TAKEN);
        c.run(op::RTS);
        return;
    }
    c.run(op::BCC_W_NOT);
    c.run(op::TST_W_DI);
    if *w == Walk::Done {
        c.run(op::BCC_TAKEN);
        c.run(op::RTS);
        return;
    }
    c.run(op::BCC_W_NOT);
    let clears = |c: &mut cpu::Cpu| {
        for _ in 0..6 {
            c.run(op::MOVEA_L_IM);
        }
        for _ in 0..8 {
            c.run(op::CLR_L_DN);
        }
    };
    if kind == 1 {
        clears(c);
    }
    c.run(op::ADDI_W_DI);
    c.run(op::MOVE_W_DI_DN);
    c.run(op::CMP_W_DI_DN);
    let Walk::Stepped { colours, changed } = w else {
        c.run(op::BCC_TAKEN);
        c.run(op::RTS);
        return;
    };
    c.run(op::BCC_W_NOT);
    let mut write = |c: &mut cpu::Cpu, at: u32, (clear, set): (u16, u16)| {
        // `andi.w #mask, (a0)` clears the component, `add.w`/`or.w` sets it.
        c.run(op::ANDI_W_AI);
        let t = *c.writes.last().unwrap();
        out.push((at, t, clear));
        c.run(op::ADD_W_DN_AI);
        let t = *c.writes.last().unwrap();
        out.push((at, t, set));
    };
    match kind {
        2 => {
            clears(c);
            c.run(op::CLR_W_DI);
            c.run(op::MOVEA_L_DI);
            c.run(op::MOVEA_L_DI);
            c.run(op::CLR_L_DN);
            c.run(op::MOVE_W_DI_DN);
            c.run(op::REG);
            c.run(op::MULU_3);
            c.run(op::REG);
            c.run(op::ADDA_L_IM);
            c.run(op::REG);
            c.run(op::ADDA_L_IM);
            c.run(op::REG);
            c.run(op::REG);
            for (k, col) in colours.iter().enumerate() {
                c.run(op::ADDQ_L_AN);
                c.run(op::ADDQ_L_AN);
                for (j, wr) in col.writes.iter().enumerate() {
                    c.run(op::MOVE_W_AI_DN);
                    c.run(op::MOVE_W_AI_DN);
                    c.run(op::ANDI_W_DN);
                    c.run(op::ANDI_W_DN);
                    c.run(op::REG);
                    match wr {
                        Some(v) => {
                            c.run(op::BCC_W_NOT);
                            c.run(op::MOVE_W_AI_DN);
                            c.run(op::ADD_W_DN_AI);
                            c.run(op::MOVE_W_AI_DN);
                            c.run(op::ANDI_W_DN);
                            if j > 0 {
                                c.run(&op::shift(4 * j as u32));
                            }
                            write(c, col.at, *v);
                            c.run(op::SUBQ_W_DN);
                        }
                        None => c.run(op::BCC_TAKEN),
                    }
                    c.run(op::ADDA_L_IM);
                    c.run(op::ADDA_L_IM);
                }
                c.run(op::SUBQ_W_DN);
                c.run(if k + 1 < colours.len() {
                    op::BCC_TAKEN
                } else {
                    op::BCC_W_NOT
                });
            }
            c.run(op::CMP_L_DN);
            if *changed {
                c.run(op::BCC_TAKEN);
            } else {
                c.run(op::BCC_W_NOT);
                c.run(op::MOVE_W_IM_DI);
                c.run(op::CLR_W_DI);
            }
        }
        3 => {
            clears(c);
            c.run(op::CLR_W_DI);
            c.run(op::CLR_L_DN);
            c.run(op::MOVE_W_DI_DN);
            c.run(op::MOVEA_L_DI);
            c.run(op::REG);
            c.run(op::CLR_L_DN);
            c.run(op::REG);
            c.run(op::ADDA_L_IM);
            c.run(op::REG);
            c.run(op::ADDA_L_IM);
            for (k, col) in colours.iter().enumerate() {
                c.run(op::ADDQ_L_AN);
                for (j, wr) in col.writes.iter().enumerate() {
                    c.run(op::MOVE_W_AI_DN);
                    c.run(op::ANDI_W_DN);
                    c.run(op::REG);
                    match wr {
                        Some(v) => {
                            c.run(op::BCC_W_NOT);
                            c.run(op::MOVE_W_AI_DN);
                            c.run(op::ADD_W_DN_AI);
                            c.run(op::MOVE_W_AI_DN);
                            c.run(op::ANDI_W_DN);
                            match j {
                                1 => c.run(&op::shift(4)),
                                // `lsr.l #8`: two cycles more than a word's.
                                2 => c.run(&op::shift(9)),
                                _ => {}
                            }
                            write(c, col.at, *v);
                            c.run(op::REG);
                        }
                        None => c.run(op::BCC_TAKEN),
                    }
                    c.run(op::ADDA_L_IM);
                    c.run(op::ADDA_L_IM);
                }
                c.run(op::SUBQ_W_DN);
                c.run(if k + 1 < colours.len() {
                    op::BCC_TAKEN
                } else {
                    op::BCC_W_NOT
                });
            }
            c.run(op::REG);
            if *changed {
                c.run(op::BCC_TAKEN);
            } else {
                c.run(op::BCC_W_NOT);
                c.run(op::MOVE_W_IM_DI);
                c.run(op::CLR_W_DI);
            }
        }
        _ => {
            c.run(op::CLR_W_DI);
            c.run(op::MOVE_W_DI_DN);
            c.run(op::MOVEA_L_DI);
            c.run(op::MOVEA_L_DI);
            c.run(op::REG);
            c.run(op::ADDA_L_IM);
            c.run(op::ADDQ_L_AN);
            for (k, col) in colours.iter().enumerate() {
                for (j, wr) in col.writes.iter().enumerate() {
                    c.run(op::MOVE_W_AI_DN);
                    c.run(op::MOVE_W_AI_DN);
                    c.run(op::ANDI_W_DN);
                    c.run(op::ANDI_W_DN);
                    c.run(op::REG);
                    match wr {
                        Some(v) => {
                            c.run(op::BCC_TAKEN);
                            c.run(op::MOVE_W_AI_DN);
                            let n = [8, 4, 0][j];
                            if n > 0 {
                                c.run(&op::shift(n));
                            }
                            c.run(op::REG);
                            if n > 0 {
                                c.run(&op::shift(n));
                            }
                            write(c, col.at, *v);
                        }
                        None => {
                            c.run(op::BCC_B_NOT);
                            c.run(op::REG);
                            c.run(op::ADDQ_L_AN);
                            c.run(op::BCC_TAKEN);
                        }
                    }
                }
                c.run(op::ADDQ_L_AN);
                c.run(op::REG);
                c.run(if k + 1 < colours.len() {
                    op::BCC_TAKEN
                } else {
                    op::BCC_B_NOT
                });
            }
            // `divu.w #3, d7` and the rest: no colour write follows.
        }
    }
    c.run(op::RTS);
}

/// One run of the handler. Its Paula writes go to `paula`, in order. Returns the paths its code
/// took, for the copper's view of its writes.
pub fn run(
    m: &mut StoryMem,
    sd: &sf_assets::sound::SoundData,
    input: FrameInput,
    paula: &mut Vec<PaulaWrite>,
) -> Handler {
    let frozen = m.w(FROZEN) != 0;
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
    let left = m.w(h0(0x3a72a));
    let stream = if left == 0xffff {
        Stream::Off
    } else {
        Stream::On {
            ends: (left.wrapping_sub(0x50) as i16) < 0,
        }
    };
    super::sound::stream_tick(m, paula);
    // `fades_step` (h0+$b7bc, $ba42, $b5c0): fades in, out and across, each record once.
    let mut calls = Vec::new();
    for kind in [2, 3, 1] {
        let walks = FADES
            .iter()
            .map(|r| {
                if m.w(r.at + super::fade::KIND) == kind {
                    r.step_walk(m)
                } else {
                    Walk::Other
                }
            })
            .collect();
        calls.push((kind, walks));
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
    Handler {
        frozen,
        stream,
        calls,
    }
}
