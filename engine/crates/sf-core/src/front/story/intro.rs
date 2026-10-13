//! The intro program (`d3_c04`, docs/13 "The story intro"): its main program run step by step
//! ([`Op`], the scenes in [`super::scenes`]) on a clock in raster lines, and its vertical-blank
//! handler ([`super::vbl`]) at the top of each frame.
//!
//! The main program paces itself on the vertical blank: `wait_vbl` (h0+$c37e) clears `vbl_flag`
//! and waits for the handler to set it; `draw_frames` (h0+$841e) waits for `vbl_count` to reach
//! two, then for the next blank, then draws. Its long jobs (unpacking, clearing the masks, a
//! draw's blits) take lines, during which the handler takes the CPU at each blank; a draw that is
//! not over by the next blank shows from the blank after. A fade's set-up runs with interrupts
//! off, so a blank that comes during it is taken when it ends.

use sf_paula::PaulaWrite;

use super::bobs;
use super::clock::{Beam, BlitClock, Bus};
use super::fade::Rec;
use super::mem::StoryMem;
use super::ops::Op;
use super::{data, h0, scenes};
use crate::game::{Job, LoadEnd, Pacing};
use crate::render::copper::Fetch;

/// `intro_skip`: set by the vertical blank on fire or the mouse button.
pub const SKIP: u32 = h0(0x16);
/// `vbl_flag` and `vbl_count`.
pub const VBL_FLAG: u32 = h0(0x18);
pub const VBL_COUNT: u32 = h0(0x1a);
/// `planes_frozen`: while set the vertical blank leaves the bitplane pointers alone.
pub const FROZEN: u32 = h0(0xb436);
/// The four fade records the vertical blank steps; the main program uses the first.
pub const FADES: [Rec; 4] = [
    Rec::intro(h0(0xbb70)),
    Rec::intro(h0(0xbd62)),
    Rec::intro(h0(0xbf54)),
    Rec::intro(h0(0xc146)),
];

/// Lines the vertical blank's handler takes when it has nothing but its own work (its 8-line
/// wait included), with the music playing, and per colour a fade step works on (oracle: its
/// acknowledge on line 12 idle, 17 with music, about 46 while a 32-colour fade steps).
const HANDLER_BASE: u64 = 12;
const HANDLER_MUSIC: u64 = 5;
const HANDLER_PER_32_COLOURS: u64 = 34;

/// The handler's length this frame, from what it is about to do.
pub fn handler_lines(m: &StoryMem) -> u64 {
    let mut lines = HANDLER_BASE;
    if super::sound::music_on(m) {
        lines += HANDLER_MUSIC;
    }
    for r in &FADES {
        let kind = m.w(r.at + super::fade::KIND);
        let steps =
            m.w(r.at + super::fade::COUNTER).wrapping_add(1) >= m.w(r.at + super::fade::SPEED);
        if (1..=3).contains(&kind) && !r.done(m) && steps {
            lines += HANDLER_PER_32_COLOURS * m.w(r.at + super::fade::COUNT) as u64 / 32;
        }
    }
    lines
}
/// The DMA cycle of its line the vertical blank's interrupt is taken on when the main program waits
/// for the blank (oracle: line 0, cycle 10, in every idle frame of the first scene).
const IRQ_ENTRY: u32 = 10;

/// Lines in each of the intro's frames (interlace is off and the frames stay long).
pub const LINES: u64 = 313;

/// A fade's set-up's CPU time, with interrupts off (oracle: a fade in takes 14 lines for 8
/// colours and 42 for 32; a fade out 17 for 8).
fn fade_setup_lines(kind: u8, count: u16) -> u64 {
    let c = count as u64;
    match kind {
        3 => 5 + 3 * c / 2,
        _ => 5 + 37 * c / 32,
    }
}
/// The fade set-ups' code, by kind (`fade_across` h0+$b4b4, `fade_in` $b6d8, `fade_out` $b906).
const FADE_SETUP_CODE: [(u32, u32); 3] = [(0xb4b4, 0xb5c0), (0xb6d8, 0xb7bc), (0xb906, 0xba42)];
/// `objects_init`'s CPU time before its blits, in DMA cycles, but for the masks' clearing: its set-up
/// plus each animation frame it prepares (h0+$8724). Oracle: the first scene's 145 frames and the
/// second's 199 put their mask blits 3022 and 3071 lines on (656 124 cycles with the clearing,
/// which took 619 850 there on refresh's bus).
const INIT_CPU: u32 = 656_124 - 619_850;
const INIT_CPU_PER_FRAME: u32 = 206;
/// The masks' clearing (h0+$8408, `clr.b (a0)+; subq.l #1, d0; bne.b`): its turns and each turn's
/// bus accesses, timed on the bus as it finds it (a scene's display slows it: T50).
const CLEAR_TURNS: u32 = 0xa000;
const CLEAR_LOOP: &str = "rpw pnn npp";

/// Where the original is while it clears the buffers and sets up the objects (`buffers_clear`'s
/// waits, `objects_init`'s calls, the draws in it): the return addresses of their calls.
const SETUP_INSIDE: [u32; 15] = [
    h0(0x8324),
    h0(0x8330),
    h0(0x833c),
    h0(0x8348),
    h0(0x8358),
    h0(0x8392),
    h0(0x83b2),
    h0(0x972e),
    h0(0x974c),
    h0(0x976a),
    h0(0x9782),
    h0(0x9792),
    h0(0x97b4),
    h0(0x97b8),
    h0(0x97bc),
];
/// What the set-up writes.
const SETUP_WRITES: [(u32, u32); 7] = [
    (h0(0x1a), 4),
    (h0(0x84e0), 4 + 400),
    (h0(0x8b68), 0x8bb0 - 0x8b68 + 16 * 0xb0),
    (h0(0x97be), 0xafd6 - 0x97be),
    (h0(0xb42e), 8),
    (bobs::BUFFER_A, 2 * 5 * bobs::PLANE),
    (bobs::MASKS, 0xa000),
];

/// Whether the original's main program is in one of its long jobs (a draw, a scene's set-up, an
/// unpack) at the end of a frame, from where it is (`chain`: its PC, then the return addresses on
/// its stack): what conformance feeds the intro as [`Pacing`] (owner, 2026-10-12).
pub fn busy(chain: &[u32]) -> bool {
    chain
        .iter()
        .any(|a| BUSY_RETURNS.contains(a) || scenes::UNPACK_FRAMES.iter().any(|(r, _)| r == a))
}

/// Return addresses of the calls that make the long jobs: `draw_frame` in `draw_frames`,
/// `draw_next` and `objects_init`; `objects_init`'s own calls; `buffers_clear`'s blitter waits.
const BUSY_RETURNS: [u32; 13] = [
    h0(0x8442),
    h0(0x84de),
    h0(0x97b8),
    h0(0x97bc),
    h0(0x972e),
    h0(0x974c),
    h0(0x976a),
    h0(0x9782),
    h0(0x9792),
    h0(0x97b4),
    h0(0x8358),
    h0(0x8392),
    h0(0x83b2),
];

/// Return addresses on the original's stack while a step runs (for conformance's loads and jobs).
const RET_TEXT_1_UNPACK: u32 = h0(0x4ec);

/// What the main program waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Wait {
    /// Nothing: it runs.
    Run,
    /// The next vertical blank's handler (`vbl_flag` set).
    Vbl,
    /// `vbl_count` reaching two (checked after each blank).
    Count,
    /// Fade record `n`'s `done`.
    Fade(u8),
    /// All four records' `done`.
    AllFades,
    /// The CPU is busy until line `until` (since the intro's start). `blank_extends`: a blank
    /// that comes meanwhile lengthens it by the handler's time (CPU work; a draw's clock counts
    /// the handler itself).
    Busy { until: u64, blank_extends: bool },
    /// Interrupts are off until `until`: a blank that comes meanwhile is taken then.
    Masked(u64),
}

/// What runs at the end of a busy stretch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Then {
    Nothing,
    /// A draw is over: `shown_buffer` from `draw_buffer`, the blitter's priority off.
    DrawEnd,
    /// The masks are built: the blitter's priority off.
    MasksEnd,
    /// A fade's set-up is over: interrupts back on.
    IntenaOn,
}

/// A routine in progress: its script, its next step, and that step's phase (0 before it
/// starts) and counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Frame {
    script: &'static [Op],
    pc: usize,
    phase: u8,
    n: u32,
}

impl Frame {
    fn new(script: &'static [Op]) -> Frame {
        Frame {
            script,
            pc: 0,
            phase: 0,
            n: 0,
        }
    }
}

/// The program, its memory, the copper list and the jobs reported, kept before a job's end.
type Checkpoint = Box<(Intro, StoryMem, Option<u32>, usize)>;

/// The intro's main program.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Intro {
    stack: Vec<Frame>,
    wait: Wait,
    then: Then,
    /// `Swoop`'s registers.
    d6: u16,
    d7: u16,
    /// The main program's clock: lines since the intro's first frame began.
    pub t: u64,
    /// Interrupts held off until this line (a fade's set-up).
    masked_until: u64,
    /// The first frame (the load's end) is still to run.
    first: bool,
    /// The program has reached its end (the insert-disk-1 screen and the game).
    pub ended: bool,
    /// The copper list the display runs (the last COP1LC written).
    shown_list: u32,
    /// For conformance: the program and memory just before the last job that ended this frame.
    checkpoint: Option<Checkpoint>,
    /// For conformance: what runs on during this frame's load, put in place at the load.
    load_state: Vec<(u32, Vec<u8>)>,
    /// Where the handlers of coming frames start, when a draw's blit holds them off (line since
    /// the intro's start).
    handler_at: Vec<u64>,
    /// Frames (their first line since the intro's start) a blit holds the CPU through: their
    /// blank's request is still set at the next blank, and one handler serves both.
    merged: Vec<u64>,
    /// This frame's schedule from conformance ([`Pacing`]); `Unpaced` in normal play.
    pacing: Pacing,
    /// The main program's Paula writes not yet given out, at their times (some fall in a later
    /// frame: a job's last write).
    writes: Vec<(u64, PaulaWrite)>,
}

/// What a frame of the intro gives back besides memory.
pub struct IntroFrame {
    pub paula: Vec<PaulaWrite>,
    pub load: Option<LoadEnd>,
    pub resync: Vec<(u32, u32)>,
    pub jobs: Vec<Job>,
    /// The last COP1LC the CPU wrote this frame.
    pub cop1lc: Option<u32>,
    /// The line the handler ran on and the line it ended on.
    pub handler: (u16, u16),
    /// The copper list `list` as the handler left it.
    pub copper: Vec<u8>,
    /// The paths the handler's code took, for the times of its writes to the copper list.
    pub walk: Option<super::vbl::Handler>,
    /// The DMA cycle (from the frame's line 0) the handler's interrupt was taken on.
    pub entry: u32,
    /// A load this frame: the COP1LC then (the frame stands for the original's frame the load
    /// ended on, whose list is the one set before it).
    pub list_at_load: Option<u32>,
}

/// The bus under copper list `list`: a scene's five lores planes, a text page's three, or none
/// (the black list); audio when the music plays.
fn bus(list: u32, audio: bool) -> Bus {
    let planes = match list {
        scenes::SCENE_LIST => 5,
        scenes::TEXT_LIST => 3,
        _ => 0,
    };
    Bus {
        fetch: Fetch {
            bplcon0: planes << 12 | 0x200,
            diwstrt: 0x4481,
            diwstop: 0x0cc1,
            ddfstrt: 0x38,
            ddfstop: 0xd0,
        },
        audio,
    }
}

/// The sound driver's tables (empty without disks).
fn sound_data(data: &data::StoryData) -> &sf_assets::sound::SoundData {
    static EMPTY: std::sync::OnceLock<sf_assets::sound::SoundData> = std::sync::OnceLock::new();
    data.sound.as_ref().unwrap_or_else(|| {
        EMPTY.get_or_init(|| {
            sf_assets::sound::SoundData::from_parts(Vec::new(), Vec::new(), Vec::new(), Vec::new())
        })
    })
}

/// One draw, as a routine of its own: `draw_frames(1)`.
static DRAW_1: &[Op] = &[Op::Draw(1)];
/// `draw_next`.
static DRAW_ONCE: &[Op] = &[Op::DrawOnce];

/// What runs on by itself while the original loads (DESIGN §2.1): the vertical blank's counters,
/// the fades and the colours they work on, `intro_skip`.
pub const RUNS_DURING_LOAD: &[(u32, u32)] = &[
    (h0(0x16), 6),
    (h0(0xb1f4), 6),
    (h0(0x38efc), 0xfe),
    (h0(0x39296), 4),
    (h0(0x3a352), 0xda),
    (h0(0x3a556), 0x44),
    (h0(0x3a726), 8),
    (h0(0xbb70), 4 * 0x1f2),
    (super::h2(0xdb02), 0x20),
    (super::h2(0xdbe6), 0x80),
];

impl Intro {
    /// The intro as `d3_c02` starts it: its program and chip hunk in memory, the fade records
    /// done (`intro_main`'s first steps, run during the load).
    pub fn start(m: &mut StoryMem, data: &data::StoryData) -> Intro {
        // The relocator places hunk 0 and hunk 2's data and clears the BSS hunks 1 and 3.
        m.write(data::INTRO_H0, &data.intro);
        m.clear(data::INTRO_H1, data::INTRO_H1_BYTES);
        m.write(data::INTRO_H2, &data.chip);
        m.clear(data::INTRO_H3, data::INTRO_H3_BYTES);
        for r in &FADES {
            m.set_w(r.at + super::fade::DONE, 1);
        }
        Intro {
            stack: vec![Frame::new(scenes::MAIN)],
            wait: Wait::Run,
            then: Then::Nothing,
            d6: 0,
            d7: 0,
            t: 0,
            masked_until: 0,
            first: true,
            ended: false,
            shown_list: scenes::BLACK_LIST,
            pacing: Pacing::Unpaced,
            handler_at: Vec::new(),
            merged: Vec::new(),
            checkpoint: None,
            load_state: Vec::new(),
            writes: Vec::new(),
        }
    }

    /// One frame (number `frame` since the intro started): the vertical blank's handler at its
    /// top, then the main program until it waits past the frame's end.
    #[allow(clippy::too_many_arguments)]
    pub fn frame(
        &mut self,
        m: &mut StoryMem,
        data: &data::StoryData,
        input: crate::input::FrameInput,
        frame: u64,
        load_line: Option<u16>,
        list: u32,
        pacing: Pacing,
        load_state: Vec<(u32, Vec<u8>)>,
        entry: Option<(u16, u16)>,
    ) -> IntroFrame {
        self.pacing = pacing;
        self.load_state = load_state;
        let top = frame * LINES;
        let end = top + LINES;
        let mut out = IntroFrame {
            paula: Vec::new(),
            load: None,
            resync: Vec::new(),
            jobs: Vec::new(),
            cop1lc: None,
            handler: (0, 0),
            copper: Vec::new(),
            walk: None,
            entry: 0,
            list_at_load: None,
        };
        // The handler: at the top, once a masked set-up lets it in, or once the blit in progress
        // is over.
        self.handler_at.retain(|&a| a >= top);
        let held = self
            .handler_at
            .first()
            .copied()
            .filter(|&a| a < end)
            .unwrap_or(top);
        let at = top.max(self.masked_until).max(held);
        // Conformance gives where the original took the interrupt, for its writes' times only:
        // the schedule keeps the engine's own lines.
        out.entry = match entry {
            Some((line, h)) => line as u32 * super::cpu::LINE + h as u32,
            None => (at - top) as u32 * super::cpu::LINE + IRQ_ENTRY,
        };
        if let Wait::Masked(until) = self.wait
            && until > top
        {
            // The set-up's own end (interrupts on) comes first, before the handler's writes.
            self.finish_busy(m, until);
            let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.writes)
                .into_iter()
                .partition(|(t, _)| *t <= until);
            self.writes = later;
            out.paula.extend(due.into_iter().map(|(_, mut w)| {
                w.vpos = 0;
                w
            }));
        }
        self.merged.retain(|&t| t >= top);
        let handler = handler_lines(m);
        if self.merged.first() == Some(&top) {
            // No handler this frame: the next frame's serves this blank too.
            out.copper = m.read(list, 0x400);
        } else {
            let mut writes = Vec::new();
            out.walk = Some(super::vbl::run(m, sound_data(data), input, &mut writes));
            let handler_end = at + handler;
            for w in &mut writes {
                w.vpos = (at - top) as u16;
            }
            out.handler = ((at - top) as u16, (handler_end - top) as u16);
            out.copper = m.read(list, 0x400);
            out.paula.extend(writes);
            // A CPU job the blank fell into goes on after the handler.
            if let Wait::Busy {
                until,
                blank_extends: true,
            } = self.wait
                && until > at
            {
                self.wait = Wait::Busy {
                    until: until + handler,
                    blank_extends: true,
                };
            }
            if self.wait == Wait::Vbl {
                self.wait = Wait::Run;
            }
            self.t = self.t.max(handler_end);
        }
        if std::mem::take(&mut self.first) {
            // The intro's start, its first text page unpacked, all during the load: the
            // original is compared once it has left that unpack, on the line it left it.
            out.load = Some(LoadEnd::Left(RET_TEXT_1_UNPACK));
            self.after_load(m, data, input, &mut out);
            out.resync.push((VBL_FLAG, 4));
            out.resync.push((h0(0xb1f4), 6));
            self.t = top + load_line.map_or(handler, u64::from);
        }
        self.checkpoint = None;
        self.run(m, data, input, top, end, &mut out, load_line, list);
        if matches!(self.pacing, Pacing::Late { .. })
            && !matches!(self.wait, Wait::Busy { .. })
            && let Some(c) = self.checkpoint.take()
        {
            // The original is still in a job at this blank and the engine is not: its last job of
            // the frame goes on past it.
            let (mut back, mem, cop1lc, jobs) = *c;
            if let Wait::Busy { blank_extends, .. } = back.wait {
                back.wait = Wait::Busy {
                    until: end + 1,
                    blank_extends,
                };
            }
            *self = back;
            *m = mem;
            out.cop1lc = cop1lc;
            out.jobs.truncate(jobs);
        }
        self.checkpoint = None;
        // The main program's writes due this frame, among the handler's in beam order.
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.writes)
            .into_iter()
            .partition(|(at, _)| *at < end);
        self.writes = later;
        out.paula.extend(due.into_iter().map(|(_, w)| w));
        out.paula.sort_by_key(|w| w.vpos);
        out
    }

    /// An instant load is over. Under conformance, the frame now stands for the original's frame
    /// its load ended on: what ran on during the load is put in place, and that frame's handler
    /// runs on it in place of the one this step began with.
    fn after_load(
        &mut self,
        m: &mut StoryMem,
        data: &data::StoryData,
        input: crate::input::FrameInput,
        out: &mut IntroFrame,
    ) {
        let state = std::mem::take(&mut self.load_state);
        if state.is_empty() {
            return;
        }
        for (at, b) in state {
            m.write(at, &b);
        }
        // What the main program wrote before the load was in earlier frames of the original's.
        let now = self.t;
        self.writes.retain(|(at, _)| *at > now);
        out.paula.clear();
        let mut writes = Vec::new();
        out.walk = Some(super::vbl::run(m, sound_data(data), input, &mut writes));
        out.handler.0 = 0;
        out.paula.extend(writes);
    }

    /// The end of a busy stretch at `at`: its deferred part.
    fn finish_busy(&mut self, m: &mut StoryMem, at: u64) {
        self.t = self.t.max(at);
        self.wait = Wait::Run;
        match std::mem::replace(&mut self.then, Then::Nothing) {
            Then::Nothing => {}
            Then::DrawEnd => {
                let drawn = m.l(bobs::DRAWN);
                m.set_l(bobs::SHOWN, drawn);
                self.write_at(at, 0x96, 0x0400);
            }
            Then::MasksEnd => self.write_at(at, 0x96, 0x0400),
            Then::IntenaOn => {
                self.write_at(at, 0x9a, 0xc000);
                self.masked_until = 0;
            }
        }
    }

    /// Run the main program from `self.t` until it waits beyond `end`.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        m: &mut StoryMem,
        data: &data::StoryData,
        input: crate::input::FrameInput,
        top: u64,
        end: u64,
        out: &mut IntroFrame,
        load_line: Option<u16>,
        list: u32,
    ) {
        loop {
            match self.wait {
                Wait::Run => {}
                Wait::Vbl => return,
                Wait::Count => {
                    // `cmpi.w #2, vbl_count; bmi`: polled until the count reaches two.
                    if (m.w(VBL_COUNT).wrapping_sub(2) as i16) < 0 {
                        return;
                    }
                    self.wait = Wait::Run;
                }
                Wait::Fade(r) => {
                    if !FADES[r as usize].done(m) {
                        return;
                    }
                    self.wait = Wait::Run;
                }
                Wait::AllFades => {
                    if !FADES.iter().all(|r| r.done(m)) {
                        return;
                    }
                    self.wait = Wait::Run;
                }
                Wait::Masked(until) => {
                    if until >= end {
                        return;
                    }
                    self.finish_busy(m, until);
                }
                Wait::Busy { until, .. } => {
                    if until >= end {
                        if self.pacing != Pacing::OnTime {
                            return;
                        }
                        // The original's job was over by this blank: so is the engine's.
                        self.finish_busy(m, end - 1);
                    } else {
                        if matches!(self.pacing, Pacing::Late { .. }) {
                            // Kept in case the original turns out to be still in this job.
                            self.checkpoint = Some(Box::new((
                                self.clone(),
                                m.clone(),
                                out.cop1lc,
                                out.jobs.len(),
                            )));
                        }
                        self.finish_busy(m, until);
                    }
                }
            }
            if self.ended || !self.step(m, data, input, top, end, out, load_line, list) {
                return;
            }
        }
    }

    fn write(&mut self, out: &mut IntroFrame, reg: u16, value: u16) {
        let _ = out;
        self.write_at(self.t, reg, value);
    }

    /// Writes made now, in order.
    fn writes_now(&mut self, ws: Vec<PaulaWrite>) {
        for w in ws {
            self.write_at(self.t, w.reg, w.value);
        }
    }

    /// A Paula write at line `at` (since the intro's start).
    fn write_at(&mut self, at: u64, reg: u16, value: u16) {
        self.writes.push((
            at,
            PaulaWrite {
                reg,
                value,
                hpos: 0,
                vpos: (at % LINES) as u16,
            },
        ));
    }

    fn busy(&mut self, lines: u64, blank_extends: bool, then: Then) {
        self.wait = Wait::Busy {
            until: self.t + lines,
            blank_extends,
        };
        self.then = then;
    }

    /// A blit run's clock from now.
    fn clock(&self, m: &StoryMem) -> BlitClock {
        let start = Beam {
            line: (self.t % LINES) as u32,
            h: 0,
        };
        let b = bus(self.shown_list, super::sound::music_on(m));
        BlitClock::new(b, start, (handler_lines(m) * 227) as u32)
    }

    /// Where a blit run started now puts the handlers of the frames it passes, and the blanks it
    /// holds the CPU through.
    fn hold(&mut self, clock: &BlitClock) {
        let base = self.t - self.t % LINES;
        self.handler_at = clock
            .handlers
            .iter()
            .map(|b| base + b.line as u64)
            .collect();
        self.merged = clock.merged.iter().map(|&l| base + l as u64).collect();
    }

    /// Lines from now to the clock's end.
    fn lines_of(&self, clock: &BlitClock) -> u64 {
        (clock.now.line as u64 + 1).saturating_sub(self.t % LINES)
    }

    /// A draw starting now (`bobs_draw`): its blits timed, its end deferred.
    fn draw(&mut self, m: &mut StoryMem, out: &mut IntroFrame) {
        self.write(out, 0x96, 0x8400);
        let mut clock = self.clock(m);
        bobs::draw(m, &mut clock);
        self.hold(&clock);
        out.jobs.push(Job {
            inside: vec![h0(0x8442), h0(0x84de), h0(0x97b8), h0(0x97bc)],
            writes: vec![
                (bobs::BUFFER_A, 0x2260 * 5),
                (bobs::BUFFER_B, 0x2260 * 5),
                (h0(0x8bb0), 0xb0 * 16),
                (bobs::DRAWN, 8),
                (bobs::TOGGLE, 2),
            ],
        });
        let lines = self.lines_of(&clock);
        self.busy(lines, false, Then::DrawEnd);
    }

    fn next(&mut self) {
        if let Some(top) = self.stack.last_mut() {
            top.pc += 1;
            top.phase = 0;
        }
    }

    fn phase(&self) -> u8 {
        self.stack.last().map_or(0, |f| f.phase)
    }

    fn set_phase(&mut self, p: u8) {
        if let Some(top) = self.stack.last_mut() {
            top.phase = p;
        }
    }

    fn n(&self) -> u32 {
        self.stack.last().map_or(0, |f| f.n)
    }

    fn set_n(&mut self, n: u32) {
        if let Some(top) = self.stack.last_mut() {
            top.n = n;
        }
    }

    fn call(&mut self, script: &'static [Op]) {
        self.stack.push(Frame::new(script));
    }

    /// A fade's set-up: wait for all four records, then (interrupts off) the set-up.
    fn fade_setup(
        &mut self,
        m: &mut StoryMem,
        out: &mut IntroFrame,
        count: u16,
        kind: u8,
        set: impl FnOnce(&mut StoryMem),
    ) -> bool {
        if self.phase() == 0 {
            self.set_phase(1);
            if !FADES.iter().all(|r| r.done(m)) {
                self.wait = Wait::AllFades;
                return true;
            }
        }
        self.write(out, 0x9a, 0x4000);
        set(m);
        // With interrupts off it runs across the frame's end: its record and the copper lists are
        // half written there. The PC heads the original's chain, so every word of the routine.
        let (from, to) = FADE_SETUP_CODE[kind as usize - 1];
        out.jobs.push(Job {
            inside: (from..to).step_by(2).map(h0).collect(),
            writes: vec![(FADES[0].at, 0x1f2), (scenes::TEXT_LIST, 0x240)],
        });
        let lines = fade_setup_lines(kind, count);
        self.masked_until = self.t + lines;
        self.wait = Wait::Masked(self.t + lines);
        self.then = Then::IntenaOn;
        self.next();
        true
    }

    /// Run the next step: false when the program waits beyond this frame.
    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        m: &mut StoryMem,
        data: &data::StoryData,
        input: crate::input::FrameInput,
        top: u64,
        end: u64,
        out: &mut IntroFrame,
        load_line: Option<u16>,
        list: u32,
    ) -> bool {
        if self.t >= end {
            return false;
        }
        let Some(&Frame { script, pc, .. }) = self.stack.last() else {
            self.ended = true;
            return false;
        };
        let Some(&op) = script.get(pc) else {
            self.stack.pop();
            return true;
        };
        let phase = self.phase();
        match op {
            Op::Call(sub) => {
                self.next();
                self.call(sub);
            }
            Op::SkipCheck => {
                self.next();
                if m.w(SKIP) != 0 {
                    // `intro_main`'s end path (h0+$3a6): the black list, then the end.
                    out.cop1lc = Some(scenes::BLACK_LIST);
                    self.stack.clear();
                    self.call(scenes::END);
                }
            }
            Op::End => {
                self.ended = true;
                return false;
            }
            Op::SetW(a, v) => {
                m.set_w(a, v);
                self.next();
            }
            Op::AddW(a, v) => {
                let x = m.w(a).wrapping_add(v as u16);
                m.set_w(a, x);
                self.next();
            }
            Op::NotW(a) => {
                let x = !m.w(a);
                m.set_w(a, x);
                self.next();
            }
            Op::Cop1lc(a) => {
                out.cop1lc = Some(a);
                self.shown_list = a;
                self.next();
            }
            Op::SetPlanes {
                moves,
                at,
                row_bytes,
                count,
            } => {
                bobs::set_planes(m, moves, at, row_bytes, count);
                self.next();
            }
            Op::Unpack { from, to, ret } | Op::UnpackCopy { from, to, ret } => {
                let packed = m.read(from, 12);
                let size = u32::from_be_bytes([packed[8], packed[9], packed[10], packed[11]]);
                let src = m.read(from, size + 0x40);
                let bytes = sf_disk::decompress(&src).unwrap_or_default();
                if let Op::UnpackCopy { .. } = op {
                    // The packed block copied first: `(size + $32) / 4 + 4` longs.
                    let n = ((size + 0x32) >> 2) + 4;
                    let copy = m.read(from, 4 * n);
                    m.write(to, &copy);
                }
                m.write(to, &bytes);
                out.jobs.push(Job {
                    inside: vec![ret],
                    writes: vec![(
                        to,
                        bytes.len().max(4 * (((size + 0x32) >> 2) + 4) as usize) as u32,
                    )],
                });
                self.next();
                if ret == RET_TEXT_1_UNPACK {
                    // Part of the intro's load: the original is compared once it is over.
                    return true;
                }
                let start = self.t / LINES;
                let until = ((start + scenes::unpack_frames(ret)) * LINES
                    + scenes::UNPACK_END_LINE)
                    .max(self.t);
                self.wait = Wait::Busy {
                    until,
                    blank_extends: false,
                };
                self.then = Then::Nothing;
            }
            Op::Load { file, to, ret } | Op::LoadRaw { file, to, ret } => {
                let raw = &data.spans[file as usize];
                let bytes = match op {
                    Op::Load { .. } => sf_disk::decompress(raw).unwrap_or_default(),
                    _ => raw.clone(),
                };
                m.write(to, &bytes);
                self.after_load(m, data, input, out);
                out.load = Some(LoadEnd::Left(ret));
                out.list_at_load = Some(out.cop1lc.unwrap_or(list));
                out.resync.extend_from_slice(RUNS_DURING_LOAD);
                self.t = top + load_line.map_or(self.t - top, u64::from);
                self.next();
            }
            Op::CopyLongs { from, to, n } => {
                let b = m.read(from, 4 * n);
                m.write(to, &b);
                self.next();
            }
            Op::ClearBuffers => {
                if phase == 0 {
                    self.set_phase(1);
                    if !FADES.iter().all(|r| r.done(m)) {
                        self.wait = Wait::AllFades;
                        return true;
                    }
                }
                self.write(out, 0x96, 0x8400);
                out.jobs.push(Job {
                    inside: SETUP_INSIDE.to_vec(),
                    writes: SETUP_WRITES.to_vec(),
                });
                let mut clock = self.clock(m);
                bobs::clear_buffers(m, &mut clock);
                let lines = self.lines_of(&clock);
                self.next();
                self.busy(lines, false, Then::Nothing);
            }
            Op::InitObjects(table) => match phase {
                0 => {
                    m.set_l(bobs::SCENE_TABLE, table);
                    // The CPU's set-up and its clearing of the masks, then the mask blits.
                    let mut clock = self.clock(m);
                    let frames = bobs::init(m);
                    clock.cpu(INIT_CPU + INIT_CPU_PER_FRAME * frames);
                    clock.cpu_loop(CLEAR_LOOP, CLEAR_TURNS);
                    let start = self.lines_of(&clock);
                    bobs::init_masks(m, &mut clock);
                    let lines = self.lines_of(&clock);
                    self.hold(&clock);
                    let mut then = Then::MasksEnd;
                    let last = clock.last_start;
                    if clock.bus.fetched(last.line)
                        && let Some(h) = clock.handlers.last().filter(|h| **h > last)
                        && (h.line % LINES as u32 != 0 || h.h != 0)
                    {
                        // The last blit started on a line the display fetches and held the blank's
                        // handler: the main program writes DMACON first, the handler is entered
                        // after it (oracle: frames 3191, 3760, 4238; T46, T50).
                        let base = self.t - self.t % LINES;
                        let end = base + h.line as u64;
                        self.write_at(end, 0x96, 0x0400);
                        if let Some(a) = self.handler_at.last_mut() {
                            *a = end + 1;
                        }
                        then = Then::Nothing;
                    }
                    self.write_at(self.t + start, 0x96, 0x8400);
                    self.set_phase(1);
                    self.busy(lines, false, then);
                }
                1 | 2 => {
                    self.set_phase(phase + 1);
                    self.draw(m, out);
                }
                _ => self.next(),
            },
            Op::Object { obj, x, y, anim } => {
                m.set_w(obj, x);
                m.set_w(obj + 2, y);
                m.set_l(obj + 0x28, anim);
                self.next();
            }
            Op::Anim { obj, anim } => {
                m.set_l(obj + 0x28, anim);
                self.next();
            }
            Op::Draw(count) => match phase {
                0 => {
                    self.set_n(count as u32);
                    self.set_phase(1);
                }
                1 => {
                    if (m.w(VBL_COUNT).wrapping_sub(2) as i16) < 0 {
                        self.wait = Wait::Count;
                        return true;
                    }
                    m.set_w(VBL_COUNT, 0);
                    m.set_w(VBL_FLAG, 0);
                    self.set_phase(2);
                    self.wait = Wait::Vbl;
                }
                2 => {
                    self.set_phase(3);
                    self.draw(m, out);
                }
                _ => {
                    let n = self.n() - 1;
                    self.set_n(n);
                    if n == 0 {
                        self.next();
                    } else {
                        self.set_phase(1);
                    }
                }
            },
            Op::DrawOnce => match phase {
                0 => {
                    if (m.w(VBL_COUNT).wrapping_sub(2) as i16) < 0 {
                        self.wait = Wait::Count;
                        return true;
                    }
                    m.set_w(VBL_COUNT, 0);
                    self.set_phase(1);
                    self.draw(m, out);
                }
                _ => self.next(),
            },
            Op::FadeIn {
                list,
                palette,
                count,
                speed,
            } => {
                if phase == 0 {
                    m.set_w(FADES[0].at + super::fade::SPEED, speed);
                }
                return self.fade_setup(m, out, count, 2, |m| {
                    FADES[0].start_in(m, list, palette, count)
                });
            }
            Op::FadeOut { list, count, speed } => {
                if phase == 0 {
                    m.set_w(FADES[0].at + super::fade::SPEED, speed);
                }
                return self.fade_setup(m, out, count, 3, |m| FADES[0].start_out(m, list, count));
            }
            Op::FadeAcross {
                from,
                to,
                list,
                count,
                speed,
            } => {
                if phase == 0 {
                    m.set_w(FADES[0].at + super::fade::SPEED, speed);
                }
                return self.fade_setup(m, out, count, 1, |m| {
                    FADES[0].start_across(m, from, to, list, count)
                });
            }
            Op::WaitFade => {
                self.next();
                if !FADES[0].done(m) {
                    self.wait = Wait::Fade(0);
                }
            }
            Op::WaitVbl => {
                m.set_w(VBL_FLAG, 0);
                self.next();
                self.wait = Wait::Vbl;
            }
            Op::WaitVbls(count) => match phase {
                0 => {
                    self.set_n(count as u32);
                    self.set_phase(1);
                    m.set_w(VBL_FLAG, 0);
                    self.wait = Wait::Vbl;
                }
                _ => {
                    let n = self.n() - 1;
                    self.set_n(n);
                    if n == 0 {
                        self.next();
                    } else {
                        m.set_w(VBL_FLAG, 0);
                        self.wait = Wait::Vbl;
                    }
                }
            },
            Op::WaitOrFire(count) => {
                if phase == 0 {
                    self.set_n(count as u32);
                    self.set_phase(1);
                } else {
                    let n = self.n() - 1;
                    self.set_n(n);
                    if n == 0 {
                        self.next();
                        return true;
                    }
                }
                // `btst #7, CIAA PRA`: fire is down.
                if input.joy.contains(crate::input::Joy::FIRE) {
                    self.next();
                    return true;
                }
                m.set_w(VBL_FLAG, 0);
                self.wait = Wait::Vbl;
            }
            Op::Timeline(list) => {
                super::sound::timeline_start(m, list);
                self.next();
            }
            Op::Music { module, samples } => {
                let mut w = Vec::new();
                super::sound::music_start(m, sound_data(data), module, samples, &mut w);
                self.writes_now(w);
                self.next();
            }
            Op::MusicStop => {
                let mut w = Vec::new();
                super::sound::music_stop(m, &mut w);
                self.writes_now(w);
                self.next();
            }
            Op::MusicFadeStop => match phase {
                0 => {
                    super::sound::music_fade(m);
                    self.set_n(0x64);
                    self.set_phase(1);
                    m.set_w(VBL_FLAG, 0);
                    self.wait = Wait::Vbl;
                }
                _ => {
                    let n = self.n() - 1;
                    self.set_n(n);
                    if n == 0 || !super::sound::music_on(m) {
                        let mut w = Vec::new();
                        super::sound::music_stop(m, &mut w);
                        self.writes_now(w);
                        self.next();
                    } else {
                        m.set_w(VBL_FLAG, 0);
                        self.wait = Wait::Vbl;
                    }
                }
            },
            Op::Sfx(d0, d1) => {
                let mut w = Vec::new();
                super::sound::sfx_play(m, sound_data(data), d0, d1, &mut w);
                self.writes_now(w);
                self.next();
            }
            Op::MoveLeftUntil {
                obj,
                by,
                until,
                once,
            } => {
                if once {
                    // `do { x -= by; draw_next } while x >= until`.
                    if phase == 0 {
                        self.set_phase(1);
                        let x = m.w(obj).wrapping_sub(by);
                        m.set_w(obj, x);
                        self.call(DRAW_ONCE);
                    } else {
                        self.set_phase(0);
                        if (m.w(obj).wrapping_sub(until as u16) as i16) < 0 {
                            self.next();
                        }
                    }
                } else {
                    // `loop { x -= by; if x < until break; draw_frames(1) }`.
                    let x = m.w(obj).wrapping_sub(by);
                    m.set_w(obj, x);
                    if (x.wrapping_sub(until as u16) as i16) < 0 {
                        self.next();
                    } else {
                        self.call(DRAW_1);
                    }
                }
            }
            Op::Swoop { obj } => {
                if phase == 0 {
                    self.set_phase(1);
                    self.d7 = 0;
                    self.d6 = 1;
                    self.call(DRAW_1);
                } else {
                    if (m.w(obj).wrapping_sub(0xd2) as i16) >= 0 {
                        self.d6 = 0xffff;
                    }
                    self.d7 = self.d7.wrapping_add(self.d6);
                    let y = m.w(obj + 2).wrapping_add(1);
                    m.set_w(obj + 2, y);
                    let x = m.w(obj).wrapping_add(self.d7);
                    m.set_w(obj, x);
                    if (x.wrapping_sub(0x150) as i16) < 0 {
                        self.call(DRAW_1);
                    } else {
                        self.next();
                    }
                }
            }
            Op::Glide { obj, flag } => {
                let f = !m.w(flag);
                m.set_w(flag, f);
                if f != 0 {
                    let y = m.w(obj + 2).wrapping_add(1);
                    m.set_w(obj + 2, y);
                }
                let x = m.w(obj).wrapping_sub(2);
                m.set_w(obj, x);
                if (x.wrapping_sub(0xcc) as i16) < 0 {
                    self.next();
                } else {
                    self.call(DRAW_1);
                }
            }
        }
        true
    }
}
