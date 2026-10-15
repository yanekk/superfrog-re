//! The slot machine's attract frame drawn on the beam (T93): `slot_attract_frame` (h0+$6276: the
//! reels set to 0, 9, $18 and drawn, the attract pattern's row into the lamps) and
//! `slot_draw_lamps` (h0+$645e), as the attract loop (h0+$4d24) runs them after its raster wait.
//!
//! The original starts drawing on line 300 and goes on into the next frame: seven of the reels'
//! 18 blits before the interrupt, the other eleven and the 26 lamps' after it, the lamps while
//! the beam draws the picture. [`Draw`] runs one blit a step, its memory at once, and yields
//! [`Yield::Busy`] on the line the CPU is on after it, so the front end's per-part picture shows
//! each blit from the line it started on. Time is kept in CPU cycles from the frame's start; on a
//! line the display fetches bitplanes, each word fetched takes 1.4 cycles from the CPU and 1.9
//! from the blitter (measured, T72). A lamp written within [`crate::game::RACE_LINES`] of the
//! beam is a beam race ([`crate::game::FrameOut::beam_races`]).

use super::{ATTRACT_FIRST, ATTRACT_LEFT, ATTRACT_PASSES, ATTRACT_ROW};
use super::{BUFFER_0, BUFFER_1, LAMP_COUNT, LAMP_RECTS, LAMPS, PLANE, REELS, ROW};
use crate::front::blit::Blit;
use crate::front::story::cpu::{self, Bus};
use crate::front::story::sim::{self, BlitEventKind, Prog, Sim};
use crate::front::{BEAM_CYCLES_PER_LINE as LINE, Cx, FRAME_LINES, PICTURE_END, Yield, at};
use crate::render::copper::Fetch;
use crate::state::State;

/// The reels' symbols in `FRUC` by position (h0+$668a: x, y), where the reels are drawn
/// (`playfield_buffer_0+$11b2`, $11b8, $11be) and their size (64 lines of two words).
const REEL_FRAMES: u32 = at(0x668a);
const REEL_AT: [u32; 3] = [BUFFER_0 + 0x11b2, BUFFER_0 + 0x11b8, BUFFER_0 + 0x11be];
const REEL_SIZE: u16 = 0x1002;
/// The later attract pattern (h0+$3b1ae), from the 10th pass; a row is `$34` bytes, `$ff80` ends
/// a pattern.
const ATTRACT_LATER: u32 = at(0x3b1ae);
const ROW_BYTES: u32 = 0x34;
const PATTERN_END: u16 = 0xff80;

/// 68000 cycles between the steps, measured on the oracle (T93, original frames 775–800): from the
/// raster wait to the first reel's INTENA `$0010` (h0+$653a's moves, the symbol looked up, the
/// blitter wait); from a reel's INTENA `$8010` to the next's `$0010`; from INTENA `$0010` to the
/// first blit; between a blit's end and the next blit; from a reel's last blit's end to its
/// INTENA `$8010`.
const REELS_START: u32 = 560;
const REEL_LOOKUP: u32 = 296;
const REEL_REGS: u32 = 172;
const REEL_GAP: u32 = 176;
const REEL_END: u32 = 198;
/// The attract pattern's row (h0+$627a..$62ea, a 26-word copy) and `slot_draw_lamps`' start, from
/// the last reel's INTENA `$8010` to the lamps' `$0010`; from there to the first lamp's blit; a
/// lamp's set-up between blits (T72: about 520; T94: 512 puts the lamps of frames 40-300 on their
/// lines, T93's 528 drifted a line late by the last); from the last lamp's blit's end to INTENA
/// `$8010`.
const PATTERN: u32 = 1_426;
const LAMPS_REGS: u32 = 514;
const LAMP_GAP: u32 = 512;
const LAMPS_END: u32 = 398;
/// A blit due on this line or later waits for the frame's interrupt (measured: the eighth reel
/// blit, due at 312.61, starts after it), and goes on this many cycles into the line the
/// interrupt ends on: reels (the CPU back from the handler, out of the blitter wait) and lamps.
const CUT_LINE: u32 = super::FRAME_LINES as u32;
const REEL_RESUME: u32 = 600;
const LAMP_RESUME: u32 = 700;
/// Tenths of a CPU cycle a word the display fetches takes from the CPU and from the blitter on its
/// line (T72).
const CPU_LOST: u32 = 14;
const BLITTER_LOST: u32 = 19;

/// What a [`Draw`] draws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum Part {
    /// The attract frame (h0+$4d2a): the reels at 0, 9, $18, the pattern's row and the lamps.
    #[default]
    Attract,
    /// `slot_draw_lamps` alone (T73), `lead` cycles from the call to its INTENA `$0010`.
    Lamps { lead: u32 },
    /// `slot_draw_reels` alone (T73), `lead` cycles from the call to the first reel's INTENA.
    Reels { lead: u32 },
}

/// The attract frame's drawing: 18 reel blits (steps 0–17, a reel's six planes each), then the
/// pattern's row and the 26 lamps (steps 18–43); or the lamps or the reels alone ([`Part`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Draw {
    part: Part,
    step: u8,
    /// Where the CPU is: cycles since this frame's start.
    t: u32,
    /// The step was cut by the frame's end: these cycles into the line the interrupt ends on, its
    /// blit is started (`blit`), or what was left of it is done (not `blit`).
    resume: Option<(u32, bool)>,
    /// Lamps drawn after the picture: each lamp's BLTSIZE write and its blit's end, in DMA cycles
    /// from the frame's start, as many as start before the frame's interrupt ([`plan`]).
    plan: Option<Vec<(u32, u32)>>,
}

impl Draw {
    pub(crate) fn new(part: Part) -> Draw {
        Draw {
            part,
            step: if matches!(part, Part::Lamps { .. }) {
                18
            } else {
                0
            },
            ..Draw::default()
        }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        let line_start = cx.line as u32 * LINE;
        let (mut t, set_up) = match self.resume.take() {
            Some((c, true)) => (line_start + c, false),
            Some((c, false)) => return self.next(cx, line_start + c),
            None => (self.t.max(line_start), true),
        };
        let fetch = cx.fetch;
        let k = self.step as u32;
        let (b, rows) = if k < 18 {
            let (reel, plane) = (k / 6, k % 6);
            if set_up {
                if plane == 0 {
                    let lead = match (reel, self.part) {
                        (0, Part::Reels { lead }) => lead,
                        (0, _) => {
                            // h0+$653a: the attract frame's reels.
                            for (r, v) in [0, 9, 0x18].into_iter().enumerate() {
                                cx.s.set_w(REELS + 2 * r as u32, v);
                            }
                            REELS_START
                        }
                        _ => REEL_LOOKUP,
                    };
                    t = spend(&fetch, t, lead, CPU_LOST);
                    write_at(cx, t, crate::audio::INTENA, 0x0010);
                    write_at(cx, t, crate::audio::DMACON, 0x8400);
                    t = spend(&fetch, t, REEL_REGS, CPU_LOST);
                } else {
                    t = spend(&fetch, t, REEL_GAP, CPU_LOST);
                }
            }
            reel_blit(cx, reel, plane)
        } else {
            let lamp = k - 18;
            if set_up {
                if lamp == 0 {
                    let lead = match self.part {
                        Part::Lamps { lead } => lead,
                        _ => {
                            attract_row(cx.s, &|a| cx.data.h0_w(a - at(0)));
                            PATTERN
                        }
                    };
                    t = spend(&fetch, t, lead, CPU_LOST);
                    write_at(cx, t, crate::audio::INTENA, 0x0010);
                    write_at(cx, t, crate::audio::DMACON, 0x8400);
                    if matches!(self.part, Part::Lamps { .. }) && t >= PICTURE_END as u32 * LINE {
                        // DMACON's write: where the original made it, else the engine's cycle.
                        let writes = frame_writes(cx);
                        let at = match cx.schedule.get(writes.len().wrapping_sub(1)) {
                            Some(&(line, slot)) => line as u32 * cpu::LINE + slot as u32,
                            None => t / 2,
                        };
                        self.plan = plan(cx, at);
                    }
                    if self.plan.is_none() {
                        t = spend(&fetch, t, LAMPS_REGS, CPU_LOST);
                    }
                } else {
                    t = spend(&fetch, t, LAMP_GAP, CPU_LOST);
                }
            }
            lamp_blit(cx, lamp)
        };
        let planned = (self.plan.as_ref())
            .zip(k.checked_sub(18))
            .map(|(p, lamp)| p.get(lamp as usize).copied());
        if let Some(Some((write, _))) = planned {
            t = 2 * write;
        }
        if matches!(planned, Some(None)) || (planned.is_none() && t >= CUT_LINE * LINE) {
            // The CPU takes the interrupt in the blitter wait; the blit starts after it.
            let resume = if k < 18 { REEL_RESUME } else { LAMP_RESUME };
            self.resume = Some((resume, true));
            return self.cut(cx);
        }
        b.run(cx.s);
        let start = (t / LINE) as u16;
        let top = fetch.lines().start;
        let raster = (top + rows.0, top + rows.1);
        if let Some(r) = crate::game::beam_race(raster, start) {
            cx.races.push(r);
        }
        if start > crate::front::VBL_END_LINE && start < crate::front::PICTURE_END {
            cx.reveal = Some(start);
        }
        t = match planned {
            Some(Some((_, end))) => 2 * end,
            // Two cycles a word a channel (T72), not the 3 of `Blit::cycles`.
            _ => spend(&fetch, t, b.cycles() * 2 / 3, BLITTER_LOST),
        };
        if self.plan.as_ref().is_some_and(|p| p.len() as u32 == k - 17) {
            // The last lamp before the interrupt: the next one's set-up is cut by it.
            self.step += 1;
            self.resume = Some((LAMP_RESUME, true));
            return self.cut(cx);
        }
        if k < 18 && k % 6 == 5 {
            t = spend(&fetch, t, REEL_END, CPU_LOST);
            write_at(cx, t, crate::audio::INTENA, 0x8010);
        } else if k == 18 + LAMP_COUNT - 1 {
            t = spend(&fetch, t, LAMPS_END, CPU_LOST);
            write_at(cx, t, crate::audio::INTENA, 0x8010);
        }
        self.next(cx, t)
    }

    /// The step is over at `t`: the next one, or the drawing's end.
    fn next(&mut self, cx: &mut Cx, t: u32) -> Yield {
        if t >= FRAME_LINES as u32 * LINE {
            self.resume = Some((t - FRAME_LINES as u32 * LINE, false));
            return self.cut(cx);
        }
        self.t = t;
        cx.line = (t / LINE) as u16;
        self.step += 1;
        let last = match self.part {
            Part::Reels { .. } => 18,
            _ => 18 + LAMP_COUNT,
        };
        if self.step as u32 == last {
            return Yield::Return;
        }
        Yield::Busy(cx.line as u32)
    }

    /// The frame ends inside the step: the rest after the next frame's interrupt.
    fn cut(&mut self, cx: &mut Cx) -> Yield {
        self.t = 0;
        self.plan = None;
        cx.line = FRAME_LINES;
        Yield::Interrupt
    }
}

/// The frame's Paula writes so far, in the order they are compared: the copper's, then the CPU's
/// by line (as [`crate::front::Front::step`] orders them at the frame's end).
fn frame_writes(cx: &Cx) -> Vec<sf_paula::PaulaWrite> {
    let mut all = cx.early.clone();
    all.extend(cx.paula.iter().copied());
    all.sort_by_key(|w| w.vpos);
    all
}

/// When a polling access sees the frame's vertical blank: from the next frame's line 0, cycle 0
/// on, as [`Sim::irq_seen`] counts (oracle, T129: 190 lamp draws of four slot cases, a lamp at
/// either side of it; not the intro's [`crate::front::story::intro::IRQ_ENTRY`]).
const IRQ_SEEN: u32 = 1;

/// The bus the lamps run on (DMA cycles from the frame's start, on into the next frame): refresh,
/// the copper's cycles and the bitplanes' fetch of the shown list, the audio channels' slots.
struct LampBus {
    frame: u32,
    fetch: Fetch,
    copper: Vec<u32>,
    audio: Vec<u32>,
}

impl Bus for LampBus {
    fn taken(&self, t: u32) -> bool {
        let r = t % self.frame;
        cpu::refresh(t)
            || self.copper.binary_search(&r).is_ok()
            || self.audio.binary_search(&t).is_ok()
            || self
                .fetch
                .takes((r / cpu::LINE) as u16, (r % cpu::LINE) as u16)
    }
}

/// `slot_draw_lamps` (h0+$645e) from its DMACON `$8400`, written at DMA cycle `dmacon`, run on the
/// bus ([`sim`]): each lamp's BLTSIZE write and its blit's end, for the lamps whose write comes
/// before the CPU takes the frame's interrupt (the rest are cut). `None` without the audio DMA.
/// The audio slots are this frame's on its writes so far ([`crate::front::anchored`]), then the
/// next frame's on none (its interrupt's writes come after the lamps' interrupt).
fn plan(cx: &Cx, dmacon: u32) -> Option<Vec<(u32, u32)>> {
    let lines = super::FRAME_LINES as u32;
    let frame = lines * cpu::LINE;
    let mut dma = cx.audio?.clone();
    let now = crate::front::anchored(&frame_writes(cx), cx.schedule);
    let mut audio: Vec<u32> = dma.frame(&now, lines).iter().map(|s| s.0).collect();
    audio.extend(dma.frame(&[], lines).iter().map(|s| s.0 + frame));
    let mut beams = Vec::new();
    crate::render::copper::run_bus(&|a, _, _| cx.w(a), cx.cop1lc, cx.fetch, &mut beams);
    let mut copper: Vec<u32> = beams
        .iter()
        .map(|b| b.line as u32 * cpu::LINE + b.h as u32)
        .filter(|&c| c < frame)
        .collect();
    copper.sort_unstable();
    let bus = LampBus {
        frame,
        fetch: cx.fetch,
        copper,
        audio,
    };
    let prog = lamps_prog(cx);
    let mut s = Sim::new(dmacon + 1, &bus, sim::dmacon::DMAEN | sim::dmacon::BLTEN);
    s.poke(dmacon, crate::audio::DMACON, 0x8400);
    let mut taken = None;
    prog.run(&mut s, 0, &mut |s: &mut Sim| {
        if taken.is_none() && s.irq_seen(frame + IRQ_SEEN) {
            taken = Some(s.t);
        }
    });
    let taken = taken.unwrap_or(u32::MAX);
    let ends = s.events.iter().filter(|e| e.kind == BlitEventKind::End);
    Some(
        s.regs
            .iter()
            .filter(|r| r.1 == 0x058 && r.0 < taken)
            .zip(ends)
            .map(|(r, e)| (r.0, e.t))
            .collect(),
    )
}

/// `slot_draw_lamps` from DMACON's write on, as the 68000 runs it ([`cpu`]'s letters, Moira's
/// timings: `mulu.w #$28` works 38 clocks after its prefetch, `jsr abs.l` reads one extension
/// word): BLTCON1 and the masks, then per lamp (h0+$6496) its BLTCON0, the pointers and moduli
/// worked out from its rectangle, BLTSIZE, and the blitter wait (h0+$19054).
fn lamps_prog(cx: &Cx) -> Prog {
    let mut p = Prog::new();
    // DMACON's last prefetch; `clr.w BLTCON1`; `move.l #-1, BLTAFWM`; two `lea`; `move.l #$1a, d7`.
    for ins in ["p", "ppr^pw", "ppppwwp", "ppp", "ppp", "ppp"] {
        p.ins(ins);
    }
    for k in 0..LAMP_COUNT {
        let (b, _) = lamp_blit(cx, k);
        // `move.w #$d0c, d0`; `tst.w (a1)`; `bne` to h0+$64a4, else `move.w #$dfc, d0`.
        p.ins("pp");
        p.ins("prp");
        if b.con0 == 0xd0c {
            p.ins("npp");
        } else {
            p.ins("nnp");
            p.ins("pp");
        }
        p.ins_regs("ppWp", &[(0x040, b.con0)]);
        // Two `lea`, `clr.l`, `move.w (a0)`, `lsr.w #3`, two `adda.l`, `move.w 2(a0)`,
        // `mulu.w #$28`, `ext.l`, two `adda.l`, two `clr.l`, `move.w 4(a0)`, `lsr.w #4`,
        // `move.w 6(a0)`, `lsl.w #6`, `or.w`, `add.w`, `move.w #$28`, `sub.w`.
        for ins in [
            "ppp",
            "ppp",
            "pn",
            "rp",
            "pnnnn",
            "pnn",
            "pnn",
            "prp",
            "ppnnnnnnnnnnnnnnnnnnn",
            "p",
            "pnn",
            "pnn",
            "pn",
            "pn",
            "prp",
            "pnnnnn",
            "prp",
            "pnnnnnnn",
            "p",
            "p",
            "pp",
            "p",
        ] {
            p.ins(ins);
        }
        // The three moduli, the three pointers (a `move.l` polls at its second write), BLTSIZE.
        for _ in 0..3 {
            p.ins("ppwp");
        }
        for _ in 0..3 {
            p.ins("ppw^wp");
        }
        p.ins_regs("ppWp", &[(0x058, b.size)]);
        // `jsr` the wait, its loop, `rts`; two `adda.l`, `subq.w`, `bne.w`.
        p.ins("pwwpp");
        p.wait_blit(false);
        p.ins("rrpp");
        p.ins("pppnn");
        p.ins("pppnn");
        p.ins("p");
        p.ins(if k + 1 < LAMP_COUNT { "npp" } else { "nnpp" });
    }
    p
}

/// A Paula register written by the CPU at `t`.
fn write_at(cx: &mut Cx, t: u32, reg: u16, value: u16) {
    let line = cx.line;
    cx.line = (t / LINE) as u16;
    cx.wr(reg, value);
    cx.line = line;
}

/// The CPU's own work on the beam (T73: `slot_digits`' byte copies), `left` cycles of it, from
/// where the CPU is: through the frame's end and the interrupt when it runs past it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Work {
    pub(crate) left: u32,
}

impl Work {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        let fetch = cx.fetch;
        let t = cx.line as u32 * LINE;
        let end = spend(&fetch, t, self.left, CPU_LOST);
        let frame = FRAME_LINES as u32 * LINE;
        if end >= frame {
            self.left -= done(&fetch, t, frame, CPU_LOST);
            cx.line = FRAME_LINES;
            return Yield::Interrupt;
        }
        cx.line = (end / LINE) as u16;
        Yield::Return
    }
}

/// The work [`spend`] gets done between `t` and `until`.
fn done(fetch: &Fetch, t: u32, until: u32, lost: u32) -> u32 {
    let taken = (fetch.planes() as u32 * fetch.words() as u32 * lost / 10).min(LINE - 1);
    let lines = fetch.lines();
    (t / LINE..until.div_ceil(LINE))
        .map(|l| {
            let (from, to) = ((l * LINE).max(t), ((l + 1) * LINE).min(until));
            let free = if lines.contains(&((l % FRAME_LINES as u32) as u16)) {
                LINE - taken
            } else {
                LINE
            };
            (to - from) * free / LINE
        })
        .sum()
}

/// `cycles` of work from `t` (CPU cycles since the frame's start), a line the display fetches
/// bitplanes on taking `lost` tenths of a cycle from it for each word fetched: where it ends.
fn spend(fetch: &Fetch, t: u32, cycles: u32, lost: u32) -> u32 {
    let taken = (fetch.planes() as u32 * fetch.words() as u32 * lost / 10).min(LINE - 1);
    let lines = fetch.lines();
    let (mut t, mut left) = (t, cycles);
    while left > 0 {
        let line = (t / LINE) as u16 % FRAME_LINES;
        let into = t % LINE;
        let free = if lines.contains(&line) {
            LINE - taken
        } else {
            LINE
        };
        let here = (LINE - into) * free / LINE;
        if left <= here {
            return t + (left * LINE).div_ceil(free);
        }
        left -= here;
        t += LINE - into;
    }
    t
}

/// The reel blit of `plane` (h0+$65e4): the symbol at the reel's position in `FRUC` to the reel.
fn reel_blit(cx: &Cx, reel: u32, plane: u32) -> (Blit, (u16, u16)) {
    let pos = cx.s.w(REELS + 2 * reel);
    let f = REEL_FRAMES.wrapping_add((pos.wrapping_mul(4) as i16) as i32 as u32);
    let src = BUFFER_1 + (cx.w(f) >> 3) as u32 + cx.w(f + 2) as u32 * ROW;
    let to = REEL_AT[reel as usize];
    let b = Blit {
        con0: 0x09f0,
        con1: 0,
        afwm: 0xffff,
        alwm: 0xffff,
        pt: [src + plane * PLANE, 0, 0, to + plane * PLANE],
        modulo: [0x24, 0, 0, 0x24],
        size: REEL_SIZE,
    };
    let row = ((to - BUFFER_0) / ROW) as u16;
    (b, (row, row + (REEL_SIZE >> 6)))
}

/// Lamp `k`'s blit (h0+$6496): its rectangle of the sixth plane, ORed with the lamps' mask
/// (`FRUM`) when lit, cleared under it when not.
pub(super) fn lamp_blit(cx: &Cx, k: u32) -> (Blit, (u16, u16)) {
    let r = LAMP_RECTS + 12 * k;
    let (x, y, w, h) = (cx.w(r), cx.w(r + 2), cx.w(r + 4), cx.w(r + 6));
    let off = (x >> 3) as u32 + y as u32 * ROW;
    let words = w >> 4;
    let modulo = (ROW as u16).wrapping_sub(2 * words) as i16;
    let b = Blit {
        con0: if cx.s.w(LAMPS + 8 * k) != 0 {
            0xd0c
        } else {
            0xdfc
        },
        con1: 0,
        afwm: 0xffff,
        alwm: 0xffff,
        pt: [
            BUFFER_0 + 0xf000 + off,
            BUFFER_0 + 0xc800 + off,
            0,
            BUFFER_0 + 0xc800 + off,
        ],
        modulo: [modulo, modulo, 0, modulo],
        size: h << 6 | words,
    };
    (b, (y, y + h))
}

/// h0+$627a..$62ea: the attract pattern's row into the lamps' `+0` words. A row starting `$ff80`
/// ends a pattern: the first again, the later one from the 10th time, the pass count reset at
/// the 30th. A row is shown 3 frames; the row moved on is copied from the next frame (the copy
/// reads the row as it was). `table` reads the patterns (hunk 0).
pub(crate) fn attract_row(s: &mut State, table: &dyn Fn(u32) -> u16) {
    let mut a0 = s.l(ATTRACT_ROW);
    if table(a0) == PATTERN_END {
        a0 = ATTRACT_FIRST;
        let passes = s.w(ATTRACT_PASSES).wrapping_add(1);
        s.set_w(ATTRACT_PASSES, passes);
        if (passes.wrapping_sub(0xa) as i16) >= 0 {
            a0 = ATTRACT_LATER;
            if (passes.wrapping_sub(0x1e) as i16) >= 0 {
                s.set_w(ATTRACT_PASSES, 0);
            }
        }
        s.set_l(ATTRACT_ROW, a0);
    }
    let left = s.w(ATTRACT_LEFT).wrapping_sub(1);
    s.set_w(ATTRACT_LEFT, left);
    if left == 0 {
        s.set_w(ATTRACT_LEFT, 3);
        let row = s.l(ATTRACT_ROW).wrapping_add(ROW_BYTES);
        s.set_l(ATTRACT_ROW, row);
    }
    for k in 0..LAMP_COUNT {
        s.set_w(LAMPS + 8 * k, table(a0 + 2 * k));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Patterns of two rows each (lamp `k` of row `r` reads `$100 * r + k`), ended by `$ff80`.
    fn patterns() -> BTreeMap<u32, u16> {
        let mut m = BTreeMap::new();
        for (base, tag) in [(ATTRACT_FIRST, 0x1000), (ATTRACT_LATER, 0x2000)] {
            for r in 0..2 {
                for k in 0..LAMP_COUNT {
                    m.insert(
                        base + ROW_BYTES * r + 2 * k,
                        tag | (r as u16) << 8 | k as u16,
                    );
                }
            }
            m.insert(base + 2 * ROW_BYTES, PATTERN_END);
        }
        m
    }

    fn lamps(s: &State) -> Vec<u16> {
        (0..LAMP_COUNT).map(|k| s.w(LAMPS + 8 * k)).collect()
    }

    fn row(tag: u16, r: u16) -> Vec<u16> {
        (0..LAMP_COUNT as u16).map(|k| tag | r << 8 | k).collect()
    }

    #[test]
    fn the_attract_pattern_shows_a_row_three_frames_and_turns_at_its_end() {
        let m = patterns();
        let table = |a: u32| m.get(&a).copied().unwrap_or(0);
        let mut s = State::default();
        // As `slot_init` leaves it.
        s.set_w(ATTRACT_PASSES, 0);
        s.set_w(ATTRACT_LEFT, 3);
        s.set_l(ATTRACT_ROW, ATTRACT_FIRST);
        let mut shown = Vec::new();
        for _ in 0..12 {
            attract_row(&mut s, &table);
            shown.push(lamps(&s));
        }
        // Rows 0 and 1, three frames each; at `$ff80` the pattern starts again (pass 1).
        let first = [0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1];
        for (f, &r) in first.iter().enumerate() {
            assert_eq!(shown[f], row(0x1000, r), "frame {f}");
        }
        assert_eq!(s.w(ATTRACT_PASSES), 1);
    }

    #[test]
    fn the_later_pattern_comes_at_the_10th_pass_and_the_count_resets_at_the_30th() {
        let m = patterns();
        let table = |a: u32| m.get(&a).copied().unwrap_or(0);
        let mut s = State::default();
        s.set_w(ATTRACT_LEFT, 3);
        // At a pattern's end with 9 passes made: the 10th is the later pattern.
        s.set_w(ATTRACT_PASSES, 9);
        s.set_l(ATTRACT_ROW, ATTRACT_FIRST + 2 * ROW_BYTES);
        attract_row(&mut s, &table);
        assert_eq!((lamps(&s), s.w(ATTRACT_PASSES)), (row(0x2000, 0), 10));
        assert_eq!(s.l(ATTRACT_ROW), ATTRACT_LATER);
        // The 30th: still the later pattern, and the count back to 0.
        s.set_w(ATTRACT_PASSES, 29);
        s.set_w(ATTRACT_LEFT, 3);
        s.set_l(ATTRACT_ROW, ATTRACT_LATER + 2 * ROW_BYTES);
        attract_row(&mut s, &table);
        assert_eq!((lamps(&s), s.w(ATTRACT_PASSES)), (row(0x2000, 0), 0));
        // The next end goes back to the first pattern (pass 1).
        s.set_l(ATTRACT_ROW, ATTRACT_LATER + 2 * ROW_BYTES);
        attract_row(&mut s, &table);
        assert_eq!((lamps(&s), s.w(ATTRACT_PASSES)), (row(0x1000, 0), 1));
    }

    #[test]
    fn a_displayed_line_slows_the_cpu_and_more_the_blitter() {
        let fetch = Fetch {
            bplcon0: 0x6200,
            diwstrt: 0x2c81,
            diwstop: 0x2cc1,
            ddfstrt: 0x38,
            ddfstop: 0xd0,
        };
        // Above the picture a line is 454 cycles.
        assert_eq!(spend(&fetch, 0, 454, CPU_LOST), 454);
        // On it, six lowres planes take 120 words: 168 cycles from the CPU, 228 from the blitter.
        let l = 0x30 * LINE;
        assert_eq!(spend(&fetch, l, 454 - 168, CPU_LOST), l + 454);
        assert_eq!(spend(&fetch, l, 454 - 228, BLITTER_LOST), l + 454);
    }
}
