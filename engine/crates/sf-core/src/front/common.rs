//! Small routines the front-end screens share: fade set-ups with interrupts off, the title's
//! palette wait, waiting for fire to be let go, a number of frames, the music's start and fade.

use sf_assets::sound::BANK_ADDR;

use super::fade::{self, RECORDS, Rec};
use super::{Cx, Routine, Yield, at, h2, vbl};
use crate::audio::INTENA;

/// The title's every-other-frame toggle, cleared by every fade set-up.
const TOGGLE: u32 = at(0x3ca0);

/// What a fade set-up sets up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) enum FadeKind {
    /// `$151b2`: the list gets `from`'s colours, then steps to `target` one at a time.
    Step { from: u32, target: u32 },
    /// `$153a4`: fade in to `target`.
    In { target: u32 },
    /// `$155a0`: fade out to black.
    Out,
}

/// A fade record's set-up as the game calls it: its speed set, then the set-up between INTENA off
/// and on. It is long (divisions for each colour), so it can run into the next frame, which then
/// takes its interrupt only when the set-up is over.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct FadeSetup {
    pub kind: FadeKind,
    pub rec: usize,
    /// The copper list (its first MOVE) the fade works on.
    pub list: u32,
    pub colours: u16,
    /// The return address of the caller's `jsr` to the set-up.
    pub inside: u32,
    pub pc: u8,
    /// The set-up's CPU cycles where they were measured for its caller ([`fade_timed`]).
    pub cycles: Option<u32>,
}

impl FadeSetup {
    pub fn new(kind: FadeKind, rec: usize, list: u32, colours: u16, inside: u32) -> FadeSetup {
        FadeSetup {
            kind,
            rec,
            list,
            colours,
            inside,
            pc: 0,
            cycles: None,
        }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        let rec = RECORDS[self.rec];
        if self.pc == 0 {
            cx.s.set_w(rec.at + fade::SPEED, 1);
            self.pc = 1;
            let n = self.colours as u32;
            let cycles = self.cycles.unwrap_or(match self.kind {
                // Three compares and a byte store for each channel, then the copy.
                FadeKind::Step { .. } => 300 + n * 265,
                FadeKind::In { target } => fade::fade_in_cycles(cx.s, target, self.colours),
                FadeKind::Out => 1200 + n * 3 * 280,
            });
            if let Some(y) = cx.masked_job(cycles, self.inside, vec![(rec.at, 0x1f2), (TOGGLE, 2)])
            {
                return y;
            }
        }
        cx.s.set_w(TOGGLE, 0);
        match self.kind {
            FadeKind::Step { from, target } => {
                fade::set_and_step(cx.s, rec, from, target, self.list, self.colours)
            }
            FadeKind::In { target } => fade::fade_in(cx.s, rec, self.list, target, self.colours),
            FadeKind::Out => fade::fade_out(cx.s, rec, self.list, self.colours),
        }
        cx.wr(INTENA, fade::INTENA_ON);
        Yield::Return
    }
}

/// `$3ca2`: until record `rec`'s fade is over, copy three of the title's lower palette's colours
/// to its menu line; then both fade palettes into the title's copper list (`$3b5e`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct PaletteWait {
    pub rec: usize,
}

impl PaletteWait {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        for (from, to) in [(0x7340, 0x73e0), (0x738c, 0x73e4), (0x73b8, 0x73e8)] {
            let v = cx.s.l(h2(from));
            cx.s.set_l(h2(to), v);
        }
        if !fade::done(cx.s, RECORDS[self.rec]) {
            return Yield::Poll;
        }
        vbl::palette_copy(cx);
        Yield::Return
    }
}

/// A busy wait for the fire button to be let go (`btst #7,$bfe001; beq`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct FireUp;

impl FireUp {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        if cx.fire() {
            Yield::Poll
        } else {
            Yield::Return
        }
    }
}

/// `n` calls of `wait_frame_raster`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct Frames(pub u16);

impl Frames {
    pub(super) fn run(&mut self, _cx: &mut Cx) -> Yield {
        if self.0 == 0 {
            return Yield::Return;
        }
        self.0 -= 1;
        Yield::Raster
    }
}

/// `music_fade_wait` (h0+$1327a): fade the music out, wait up to 100 frames for it to stop, stop.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct MusicFadeWait {
    pc: u8,
    count: u16,
}

impl MusicFadeWait {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        if self.pc == 0 {
            cx.s.audio.fade_out();
            self.count = 0x64;
            self.pc = 1;
            return Yield::Raster;
        }
        if self.pc == 2 {
            // The rest of `music_stop` after the frame's end: interrupts back on.
            cx.wr(INTENA, 0xc000);
            return Yield::Return;
        }
        self.count -= 1;
        if self.count != 0 && cx.s.audio.music_on() {
            return Yield::Raster;
        }
        if let Some(d) = cx.data.sound.as_ref() {
            let mut out = Vec::new();
            cx.s.audio.stop_music(d, &mut out);
            // `music_stop` runs with interrupts off (its last write turns them on) for about 15
            // lines, most of them copying the module's header back (measured: from line 299 to
            // line 1 of the next frame); begun late, it ends in the next frame.
            let on = out.pop();
            cx.paula.extend(out);
            cx.busy(MUSIC_STOP_LINES as u32 * super::BEAM_CYCLES_PER_LINE);
            if cx.line >= super::FRAME_LINES {
                self.pc = 2;
                return Yield::Masked(cx.line - super::FRAME_LINES);
            }
            cx.paula.extend(on);
        }
        Yield::Return
    }
}

const MUSIC_STOP_LINES: u16 = 15;

/// `music_start` (h0+$13126) with the title's module: interrupts off, `p41_init`, interrupts on.
/// `p41_init` takes 20 lines on the title's module (measured: INTENA off on line 309, on again on
/// line 16 of the next frame), so started late in a frame it delays the next frame's interrupt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct MusicStart {
    pc: u8,
    /// The caller's return address (where the original is once `music_start` is over).
    pub ret: u32,
    /// The module stays where the game loaded it (`module_addr`), not the title's place; and
    /// `p41_init`'s lines when they are not the title module's.
    keep_module: bool,
    lines: Option<u16>,
}

const P41_INIT_LINES: u16 = 20;

impl MusicStart {
    pub fn returning_to(ret: u32) -> MusicStart {
        MusicStart {
            ret,
            ..MusicStart::default()
        }
    }

    /// `music_start` on a module the game loaded at its `module_addr` (the slot machine's), whose
    /// `p41_init` takes `lines` lines.
    pub fn keeping_module(ret: u32, lines: u16) -> MusicStart {
        MusicStart {
            ret,
            keep_module: true,
            lines: Some(lines),
            ..MusicStart::default()
        }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        if self.pc == 0 {
            if cx.s.audio.music_on() {
                return Yield::Return;
            }
            if !self.keep_module {
                cx.s.audio.set_module_addr(BANK_ADDR);
            }
            use crate::audio::vars;
            let v = |(off, len): (u32, usize)| (at(off), len as u32);
            let writes = vec![
                v(vars::MUSIC_ON),
                v(vars::P41),
                v(vars::SHADOW),
                v(vars::HEADER_BACKUP),
            ];
            let module = match cx.data.sound.as_ref() {
                Some(d) => cx.s.audio.p41_init_head(d),
                None => false,
            };
            self.pc = if module { 1 } else { 2 };
            if let Some(y) = cx.masked_job(
                self.lines.unwrap_or(P41_INIT_LINES) as u32 * super::BEAM_CYCLES_PER_LINE,
                self.ret,
                writes,
            ) {
                return y;
            }
        }
        if let (1, Some(d)) = (self.pc, cx.data.sound.as_ref()) {
            let mut out = Vec::new();
            cx.s.audio.p41_init_tail(d, &mut out);
            cx.paula.extend(out);
        }
        cx.wr(INTENA, 0xc000);
        Yield::Return
    }
}

/// A call of a routine, for brevity.
pub(crate) fn call(r: Routine) -> Yield {
    Yield::Call(Box::new(r))
}

pub(crate) fn fade(kind: FadeKind, rec: usize, list: u32, colours: u16, inside: u32) -> Yield {
    call(Routine::FadeSetup(FadeSetup::new(
        kind, rec, list, colours, inside,
    )))
}

/// [`fade`], its set-up taking `cycles` (measured for the caller).
pub(crate) fn fade_timed(
    kind: FadeKind,
    rec: usize,
    list: u32,
    colours: u16,
    inside: u32,
    cycles: u32,
) -> Yield {
    let mut f = FadeSetup::new(kind, rec, list, colours, inside);
    f.cycles = Some(cycles);
    call(Routine::FadeSetup(f))
}

pub(crate) fn palette_wait(rec: usize) -> Yield {
    call(Routine::PaletteWait(PaletteWait { rec }))
}

pub(crate) fn frames(n: u16) -> Yield {
    call(Routine::Frames(Frames(n)))
}

#[allow(dead_code)]
pub(crate) fn rec(r: usize) -> Rec {
    RECORDS[r]
}
