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

use sf_paula::{AudioDma, PaulaWrite};

use super::bobs;
use super::fade::Rec;
use super::mem::StoryMem;
use super::ops::Op;
use super::{data, h0, scenes, sim};
use crate::game::{HandlerEnd, Job, LoadEnd, Pacing};
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

/// The handler's 8-line wait (h0+$38a02, after the music's tick) waits for VHPOSR's low byte to
/// come round to its read's plus eight; the lines from its end to the handler's end in
/// [`handler_lines`] (oracle, T97: with the music on, the read 6 lines after the entry, the
/// acknowledge one after the wait).
const WAIT_LINES: u64 = 8;
const AFTER_WAIT: u64 = 3;

/// The lines a handler's wait waits when read on line `read` of its frame: eight, or, when line
/// `read + 8` is past the frame's last (it does not exist), on to the line of the next frame
/// whose low byte is the same, `read + 8 - 256` (oracle frames 7591, 7984, 10087: the
/// acknowledge on lines 64, 61, 59 of the next frame).
pub fn wait_lines(read: u64) -> u64 {
    if (LINES - WAIT_LINES..LINES).contains(&read) {
        WAIT_LINES + LINES - 256
    } else {
        WAIT_LINES
    }
}

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
pub const IRQ_ENTRY: u32 = 10;

/// Where the main program waits for the blank before a draw, as the instruction the interrupt came
/// in before (the PC the handler returns to): `draw_frames`' `tst.w vbl_flag` (h0+$8434) and its
/// `beq.b` ($843a); `draw_next`'s `cmpi.w #2, vbl_count` ($84ca) and its `bmi.b` ($84d2).
pub const FRAMES_TST: u32 = h0(0x8434);
pub const FRAMES_BEQ: u32 = h0(0x843a);
pub const NEXT_CMPI: u32 = h0(0x84ca);
pub const NEXT_BMI: u32 = h0(0x84d2);

/// The CPU's work from the handler's acknowledge (`INTREQ = $20`) to `bobs_draw`'s first
/// instruction, the interrupt having come in before `resumes` (one of the four above): the
/// handler's tail, the wait's last turn (a branch the interrupt came in before is taken: the test
/// before it read the old value), the call of `draw_frame` (h0+$96b0) and its set-up. `None` for
/// another PC: the main program was not waiting to draw.
pub fn handler_to_draw(resumes: u32) -> Option<sim::Prog> {
    use super::cpu::op;
    let mut p = sim::Prog::new();
    p.ins(super::vbl::TAIL);
    match resumes {
        FRAMES_TST | FRAMES_BEQ => {
            if resumes == FRAMES_BEQ {
                p.branch(true, false);
            }
            p.ins(op::TST_W_ABS_L);
            p.branch(false, false);
            p.ins(op::MOVE_L_DN_PREDEC); // move.l d0, -(a7)
        }
        NEXT_CMPI | NEXT_BMI => {
            if resumes == NEXT_BMI {
                p.branch(true, false);
            }
            p.ins(op::CMPI_W_ABS_L);
            p.branch(false, false);
            p.ins(op::CLR_W_ABS_L); // clr.w vbl_count
        }
        _ => return None,
    }
    p.ins(op::BSR); // bsr.w draw_frame
    p.ins(op::LEA_ABS_L);
    p.ins(op::LEA_ABS_L);
    for _ in 0..3 {
        p.ins(op::MOVEA_L_IM); // move.l #i, d0 / d1 / d3
    }
    p.ins(op::BSR); // bsr.w bobs_draw
    Some(p)
}

/// DMACON as a draw finds it: DMA, the blitter's and the bitplanes' on.
const DRAW_DMACON: u16 = sim::dmacon::DMAEN | sim::dmacon::BLTEN | 0x0100;

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
/// What the set-up writes (from h0+$1a: `vbl_count` and the scene table's pointer).
const SETUP_WRITES: [(u32, u32); 7] = [
    (h0(0x1a), 6),
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
    chain.first().is_some_and(|pc| BUSY_BODIES.iter().any(|r| r.contains(pc)))
        || chain
            .iter()
            .any(|a| BUSY_RETURNS.contains(a) || scenes::UNPACK_FRAMES.iter().any(|(r, ..)| r == a))
}

/// The set-up's own code, where the PC can be between two of its calls at a blank (oracle frame
/// 7577: h0+$96e8, `objects_init`'s first loop): `buffers_clear` from its blits (h0+$834c; before
/// them it waits for the fades) and `objects_init` (h0+$96d4).
const BUSY_BODIES: [std::ops::Range<u32>; 2] = [h0(0x834c)..h0(0x83b4), h0(0x96d4)..h0(0x97be)];

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

/// A draw on the bus ([`Intro::time_draw`]), its times counted from its frame's start.
struct Timed {
    /// The cycle its program ends on.
    end: u32,
    /// Its DMACON `$8400` write.
    dmacon: u32,
    /// Where each handler it holds off started.
    handlers: Vec<u32>,
    /// The blanks it holds the CPU through: their requests are served by the next blank's handler.
    merged: Vec<u32>,
}

/// A long job of a scene's set-up (`buffers_clear`, `objects_init` with its masks: T56), its
/// CPU's program run on the bus ([`sim`]) with the handlers of the blanks it passes, timed again
/// at each frame from its start with the handlers met so far: their ends as conformance gives
/// them (the original's acknowledge), or the engine's length for each.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SimJob {
    prog: std::sync::Arc<sim::Prog>,
    /// Its first frame (since the intro's start), its first cycle in it, and the copper list that
    /// frame shows (a COP1LC written in it is the copper's from the next).
    frame: u64,
    start: u32,
    list: u32,
    /// The audio channels' DMA as of its first frame's start, and each frame's Paula writes since.
    audio: AudioDma,
    writes: Vec<Vec<PaulaWrite>>,
    /// Per frame since its first, the cycle of its handler's acknowledge, when conformance gives it.
    acks: Vec<Option<u32>>,
    /// Its DMACON writes as last scheduled (line since the intro's start, value), and its end
    /// (cycle since the intro's start).
    scheduled: Vec<(u64, u16)>,
    end: u64,
}

/// A [`SimJob`]'s run: times in cycles since the intro's start.
struct JobTimed {
    end: u64,
    dmacon: Vec<(u64, u16)>,
    handlers: Vec<u64>,
    merged: Vec<u64>,
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
    /// This frame's handler has just let the main program go from its wait (`vbl_flag`, or
    /// `vbl_count` reaching two): a draw now starts from the handler's end.
    fresh: bool,
    /// For conformance: where the original's handler of this frame ended ([`HandlerEnd`]).
    handler_end: Option<HandlerEnd>,
    /// A handler whose wait ran on past its frame's end ([`wait_lines`]): the line its wait ends
    /// on and the line it ends on (since the intro's start), its rest still to run.
    tail: Option<(u64, u64)>,
    /// The audio channels' DMA as of this frame's start, run on the frames' Paula writes: the
    /// slots it takes hold up the draws' blits and CPU (T96).
    audio: AudioDma,
    /// The set-up's job in progress ([`SimJob`]).
    job: Option<SimJob>,
    /// The cycle (since the intro's start) a [`SimJob`] ended on, for what the main program does
    /// next (a draw).
    resume: Option<u64>,
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
    /// The audio channels' DMA slots this frame (cycles from its line 0), which the handler's CPU
    /// waits for too.
    pub audio: Vec<u32>,
    /// A load this frame: the COP1LC then (the frame stands for the original's frame the load
    /// ended on, whose list is the one set before it).
    pub list_at_load: Option<u32>,
}

/// The bitplane fetch under copper list `list` at a frame's start: a scene's five lores planes, a
/// text page's three, or none (the black list).
fn fetch(list: u32) -> Fetch {
    let planes = match list {
        scenes::SCENE_LIST => 5,
        scenes::TEXT_LIST => 3,
        _ => 0,
    };
    Fetch {
        bplcon0: planes << 12 | 0x200,
        diwstrt: 0x4481,
        diwstop: 0x0cc1,
        ddfstrt: 0x38,
        ddfstop: 0xd0,
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
            fresh: false,
            handler_end: None,
            tail: None,
            audio: AudioDma::booted(),
            job: None,
            resume: None,
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
        handler_end: Option<HandlerEnd>,
        merged: Option<bool>,
    ) -> IntroFrame {
        self.pacing = pacing;
        self.handler_end = handler_end;
        // The main program a split handler let go of goes on once its rest has run, below.
        self.fresh &= self.tail.is_some();
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
            audio: Vec::new(),
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
        if let Some((wait_end, handler_end)) = self.tail.take() {
            // The rest of the last frame's handler, from its wait's end.
            let mut writes = Vec::new();
            super::vbl::after_wait(m, sound_data(data), &mut writes);
            for w in &mut writes {
                w.vpos = wait_end.saturating_sub(top) as u16;
            }
            out.paula.extend(writes);
            self.handler_done(handler_end);
        }
        if merged.unwrap_or(self.merged.first() == Some(&top)) {
            // No handler this frame: the next frame's serves this blank too.
            out.copper = m.read(list, 0x400);
        } else {
            let mut writes = Vec::new();
            out.walk = Some(super::vbl::to_wait(m, sound_data(data), input, &mut writes));
            // The wait's read, from where the interrupt was taken (conformance gives the
            // original's: the engine's own line can be far off while a draw's blits hold the CPU).
            let read =
                out.entry as u64 / super::cpu::LINE as u64 + handler - WAIT_LINES - AFTER_WAIT;
            let split = wait_lines(read) > WAIT_LINES;
            let wait_end = top + read + wait_lines(read);
            let handler_end = if split {
                wait_end + AFTER_WAIT
            } else {
                at + handler
            };
            if split {
                // Its acknowledge clears the next blank's request too: that blank has no handler.
                self.tail = Some((wait_end, handler_end));
                self.merged.insert(0, top + LINES);
            } else {
                super::vbl::after_wait(m, sound_data(data), &mut writes);
            }
            for w in &mut writes {
                w.vpos = (at - top) as u16;
                w.hpos = (out.entry % super::cpu::LINE) as u16;
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
                    until: until + (handler_end - at),
                    blank_extends: true,
                };
            }
            if split {
                self.t = self.t.max(handler_end);
            } else {
                self.handler_done(handler_end);
            }
        }
        if self.job.is_some() {
            // The set-up's job, timed again with this frame's handler.
            let k = frame - self.job.as_ref().map_or(frame, |j| j.frame);
            let ack = self
                .handler_end
                .map(|e| e.ack.0 as u32 * super::cpu::LINE + e.ack.1 as u32);
            if let Some(j) = &mut self.job {
                let k = k as usize;
                if j.acks.len() <= k {
                    j.acks.resize(k + 1, None);
                }
                j.acks[k] = ack;
            }
            self.retime(m, top);
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
        out.paula.sort_by_key(|w| (w.vpos, w.hpos));
        if let Some(j) = &mut self.job {
            let k = (frame - j.frame) as usize;
            if j.writes.len() <= k {
                j.writes.resize(k + 1, Vec::new());
            }
            j.writes[k] = out.paula.clone();
        }
        out.audio = self
            .audio
            .frame(&out.paula, LINES as u32)
            .iter()
            .map(|s| s.0)
            .collect();
        out
    }

    /// The handler has returned at `end`: the main program goes on from there, let go from its
    /// wait for the blank.
    fn handler_done(&mut self, end: u64) {
        if self.wait == Wait::Vbl {
            self.wait = Wait::Run;
            self.fresh = true;
        }
        self.t = self.t.max(end);
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
        if let Some(j) = self.job.take() {
            // Its writes still to come are made by now (the original's job was over).
            for w in &mut self.writes {
                if w.1.reg == 0x96 && j.scheduled.contains(&(w.0, w.1.value)) && w.0 > at {
                    w.0 = at;
                    w.1.vpos = (at % LINES) as u16;
                }
            }
            let line = super::cpu::LINE as u64;
            self.resume = Some(j.end.min(at * line));
        }
        match std::mem::replace(&mut self.then, Then::Nothing) {
            Then::Nothing => {}
            Then::DrawEnd => {
                let drawn = m.l(bobs::DRAWN);
                m.set_l(bobs::SHOWN, drawn);
                self.write_at(at, 0x96, 0x0400);
            }
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
                    self.fresh = true;
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
                    let line = super::cpu::LINE as u64;
                    if let Some(job) = self.job.as_ref().map(|j| j.end)
                        && until >= end
                        && job < end * line
                    {
                        // The set-up's job ends in the frame's last line: what follows it starts
                        // in this frame, at the job's end (scene 8's `objects_init`, O 7577).
                        self.finish_busy(m, end - 1);
                        self.resume = Some(job);
                    } else if until >= end {
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

    /// Start the set-up's job `prog` now ([`SimJob`]): from the cycle the last job ended on, or
    /// this line's start. `out`: this frame so far, whose Paula writes run the audio DMA.
    fn start_job(&mut self, m: &StoryMem, out: &IntroFrame, prog: sim::Prog, list: u32) {
        let line = super::cpu::LINE as u64;
        let at = self
            .resume
            .take()
            .filter(|r| r / line == self.t)
            .unwrap_or(self.t * line);
        let frame = self.t / LINES;
        let end = frame * LINES + LINES;
        let mut now = out.paula.clone();
        now.extend(self.writes.iter().filter(|w| w.0 < end).map(|w| w.1));
        now.sort_by_key(|w| w.vpos);
        self.job = Some(SimJob {
            prog: std::sync::Arc::new(prog),
            frame,
            start: (at - frame * LINES * line) as u32,
            list,
            audio: self.audio.clone(),
            writes: vec![now],
            acks: vec![None],
            scheduled: Vec::new(),
            end: at,
        });
        self.wait = Wait::Busy {
            until: self.t + 1,
            blank_extends: false,
        };
        self.then = Then::Nothing;
        self.retime(m, frame * LINES);
    }

    /// The set-up's job timed again from its start ([`Intro::run_job`]), in this frame (from line
    /// `top`): its end, its DMACON writes still to come, the handlers it holds off and the blanks
    /// it holds the CPU through.
    fn retime(&mut self, m: &StoryMem, top: u64) {
        let Some(mut j) = self.job.take() else {
            return;
        };
        let timed = self.run_job(m, &j);
        let line = super::cpu::LINE as u64;
        // What was due before this frame has been given out; the rest is scheduled anew.
        let sent = j.scheduled.iter().filter(|w| w.0 < top).count();
        let old: Vec<(u64, u16)> = j.scheduled[sent..].to_vec();
        self.writes
            .retain(|w| !(w.1.reg == 0x96 && old.contains(&(w.0, w.1.value))));
        j.scheduled.truncate(sent);
        for &(t, v) in timed.dmacon.iter().skip(sent) {
            let at = (t / line).max(top);
            self.write_at(at, 0x96, v);
            // Its cycle in the line, for its order beside the handler's writes.
            if let Some(w) = self.writes.last_mut() {
                w.1.hpos = if at == t / line { (t % line) as u16 } else { 0 };
            }
            j.scheduled.push((at, v));
        }
        j.end = timed.end;
        if let Wait::Busy { .. } = self.wait {
            self.wait = Wait::Busy {
                until: timed.end / line + 1,
                blank_extends: false,
            };
        }
        self.handler_at = timed.handlers.iter().map(|&t| t / line).collect();
        self.merged = timed.merged.iter().map(|&t| t / line).collect();
        self.job = Some(j);
    }

    /// A [`SimJob`]'s program run on the bus from its start: the CPU and the blitter on the shown
    /// list's copper and fetch and the audio DMA of each frame's writes ([`Intro::fixed`]), each
    /// blank's handler taking the CPU at the first instruction end that sees its request, to its
    /// acknowledge when given, else for the engine's length ([`handler_lines`]).
    fn run_job(&self, m: &StoryMem, j: &SimJob) -> JobTimed {
        // The furthest a handler's acknowledge is from its entry (the longest, a fade's, ~50 lines).
        const ACK_WITHIN: u32 = 60 * super::cpu::LINE;
        // Frames enough for the longest set-up (the masks' clearing alone is about nine).
        const FRAMES: usize = 24;
        let fixed = self.fixed_from(m, &j.audio, &j.writes, FRAMES, j.list);
        let mut s = sim::Sim::new(j.start, &fixed, DRAW_DMACON);
        let handler = (handler_lines(m) as u32 * super::cpu::LINE).saturating_sub(2);
        let (mut handlers, mut merged) = (Vec::new(), Vec::new());
        let mut next = sim::FRAME + IRQ_ENTRY;
        let mut take = |s: &mut sim::Sim| {
            while s.irq_seen(next + sim::FRAME) {
                merged.push(next - IRQ_ENTRY);
                next += sim::FRAME;
            }
            if s.irq_seen(next) {
                handlers.push(s.t);
                let k = (next / sim::FRAME) as usize;
                // Its acknowledge: this frame's, or, held past the original's step (which ends at
                // an instruction past the frame's end), the next step's, on this frame's beam.
                let from = s.t;
                let ack = [k, k + 1]
                    .iter()
                    .filter_map(|&i| j.acks.get(i).copied().flatten())
                    .map(|a| k as u32 * sim::FRAME + a)
                    .find(|&a| a > from && a < from + ACK_WITHIN);
                s.run("p");
                match ack {
                    Some(ack) => {
                        if s.t < ack + 1 {
                            s.idle(ack + 1 - s.t);
                        }
                        s.run(super::vbl::TAIL);
                    }
                    None => s.idle(handler),
                }
                next += sim::FRAME;
            }
        };
        j.prog.run(&mut s, 0, &mut take);
        let base = j.frame * LINES * super::cpu::LINE as u64;
        JobTimed {
            end: base + s.t as u64,
            dmacon: s
                .regs
                .iter()
                .filter(|w| w.1 == 0x96)
                .map(|w| (base + w.0 as u64, w.2))
                .collect(),
            handlers: handlers.iter().map(|&t| base + t as u64).collect(),
            merged: merged.iter().map(|&t| base + t as u64).collect(),
        }
    }

    /// A draw starting now (`bobs_draw`): its program run on the bus ([`Intro::time_draw`]), its
    /// end deferred. `after`: the main program has just left its wait for the blank, by default
    /// at this instruction ([`handler_to_draw`]); the draw starts from the handler's end.
    fn draw(&mut self, m: &mut StoryMem, out: &mut IntroFrame, after: Option<u32>) {
        let prog = bobs::draw(m);
        let lead = after.and_then(|default| {
            // Conformance gives the original's acknowledge and the instruction it returned to;
            // normal play takes the acknowledge at the start of the line the handler ends on.
            let (ack, resumes) = match self.handler_end {
                Some(HandlerEnd {
                    ack,
                    resumes: Some(r),
                }) => (ack.0 as u32 * super::cpu::LINE + ack.1 as u32, r),
                _ => ((self.t % LINES) as u32 * super::cpu::LINE, default),
            };
            Some((ack, handler_to_draw(resumes)?))
        });
        // This frame's Paula writes so far, for the audio DMA the draw runs beside.
        let end = self.t - self.t % LINES + LINES;
        let mut now = out.paula.clone();
        now.extend(self.writes.iter().filter(|w| w.0 < end).map(|w| w.1));
        now.sort_by_key(|w| w.vpos);
        let timed = self.time_draw(m, &prog, lead, &now);
        let base = self.t - self.t % LINES;
        let line = |t: u32| base + (t / super::cpu::LINE) as u64;
        self.write_at(line(timed.dmacon), 0x96, 0x8400);
        self.handler_at = timed.handlers.iter().map(|&t| line(t)).collect();
        self.merged = timed.merged.iter().map(|&t| line(t)).collect();
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
        let lines = (line(timed.end) + 1).saturating_sub(self.t);
        self.busy(lines, false, Then::DrawEnd);
    }

    /// A draw's program run on the bus from the start of this line, or after `lead` from its
    /// cycle on (the handler's acknowledge and the CPU's work from there to the draw): the CPU and
    /// the blitter on the shown list's copper and bitplane fetch ([`sim`]), the vertical blank's
    /// handler taking the CPU at the first instruction's end that sees each blank's request
    /// ([`sim::Sim::irq_seen`]), waiting for the bus and then working its length; a blank the CPU is held past the next one is served
    /// by that one's handler ([`BlitClock`]'s `merged`). The audio DMA runs on `now`, this
    /// frame's Paula writes so far ([`Intro::fixed`]). Times are counted from this frame's start.
    fn time_draw(
        &self,
        m: &StoryMem,
        prog: &sim::Prog,
        lead: Option<(u32, sim::Prog)>,
        now: &[PaulaWrite],
    ) -> Timed {
        let fixed = self.fixed(m, 3, now);
        let line = super::cpu::LINE as u64;
        let start = match &lead {
            // The acknowledge's write took its cycle: the CPU goes on from the next.
            Some((ack, _)) => ack + 1,
            // After a set-up's job: from its end.
            None => self
                .resume
                .filter(|r| r / line == self.t)
                .map_or((self.t % LINES) as u32 * super::cpu::LINE, |r| {
                    (r - (self.t - self.t % LINES) * line) as u32
                }),
        };
        let mut s = sim::Sim::new(start, &fixed, DRAW_DMACON);
        let handler = (handler_lines(m) as u32 * super::cpu::LINE).saturating_sub(2);
        let (mut handlers, mut merged) = (Vec::new(), Vec::new());
        let mut next = sim::FRAME + IRQ_ENTRY;
        let mut take = |s: &mut sim::Sim| {
            while s.irq_seen(next + sim::FRAME) {
                merged.push(next - IRQ_ENTRY);
                next += sim::FRAME;
            }
            if s.irq_seen(next) {
                handlers.push(s.t);
                s.run("p");
                s.idle(handler);
                next += sim::FRAME;
            }
        };
        if let Some((_, lead)) = &lead {
            lead.run(&mut s, 0, &mut take);
        }
        prog.run(&mut s, 0, &mut take);
        let dmacon = s.regs.iter().find(|w| w.1 == 0x96).map_or(start, |w| w.0);
        Timed {
            end: s.t,
            dmacon,
            handlers,
            merged,
        }
    }

    /// The fixed DMA of `frames` frames from this one under the shown copper list: its copper
    /// cycles, the bitplane fetch it sets up and the audio channels' slots, this frame's on its
    /// Paula writes `now`, the next ones' on none (their handlers' writes are not made yet: a
    /// channel plays on, an effect's next step is missed).
    fn fixed(&self, m: &StoryMem, frames: usize, now: &[PaulaWrite]) -> sim::Fixed {
        self.fixed_from(m, &self.audio, &[now.to_vec()], frames, self.shown_list)
    }

    /// [`Intro::fixed`] from the audio DMA `audio` at the first frame's start, frame `k`'s on its
    /// Paula writes `paula[k]` (none past them), the first frame under copper list `first`.
    fn fixed_from(
        &self,
        m: &StoryMem,
        audio: &AudioDma,
        paula: &[Vec<PaulaWrite>],
        frames: usize,
        first: u32,
    ) -> sim::Fixed {
        let mut fetch = fetch(first);
        let mut fixed = sim::Fixed::new();
        let mut audio = audio.clone();
        for k in 0..frames {
            let mut cycles = Vec::new();
            let list = if k == 0 { first } else { self.shown_list };
            let writes = crate::render::copper::run_bus(&|a, _, _| m.w(a), list, fetch, &mut cycles);
            let writes_k = paula.get(k).map_or(&[][..], |w| &w[..]);
            fixed = fixed
                .frame(&cycles, &writes, fetch)
                .audio(audio.frame(writes_k, LINES as u32));
            for w in &writes {
                fetch.write(w.reg, w.value);
            }
        }
        fixed
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
        let fresh = std::mem::take(&mut self.fresh);
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
                    + scenes::unpack_end_line(ret))
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
                out.jobs.push(Job {
                    inside: SETUP_INSIDE
                        .iter()
                        .copied()
                        .chain(BUSY_BODIES.iter().flat_map(|r| r.clone().step_by(2)))
                        .collect(),
                    writes: SETUP_WRITES.to_vec(),
                });
                let prog = bobs::clear_buffers(m);
                self.next();
                self.start_job(m, out, prog, list);
            }
            Op::InitObjects(table) => match phase {
                0 => {
                    m.set_l(bobs::SCENE_TABLE, table);
                    let prog = bobs::init(m);
                    self.set_phase(1);
                    self.start_job(m, out, prog, list);
                }
                1 | 2 => {
                    self.set_phase(phase + 1);
                    self.draw(m, out, None);
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
                    self.draw(m, out, fresh.then_some(FRAMES_BEQ));
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
                    self.draw(m, out, fresh.then_some(NEXT_BMI));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front::story::cpu::Bus;

    fn write(vpos: u16, reg: u16, value: u16) -> PaulaWrite {
        PaulaWrite {
            reg,
            value,
            hpos: 0,
            vpos,
        }
    }

    #[test]
    fn a_draw_runs_beside_the_audio_dma_of_this_frames_writes_and_the_channels_playing() {
        let mut m = StoryMem::default();
        let mut intro = Intro::start(&mut m, &data::StoryData::default());
        // The black list: the copper's end alone.
        m.set_w(scenes::BLACK_LIST, 0xffff);
        m.set_w(scenes::BLACK_LIST + 2, 0xfffe);
        // Channel 0 turned on by this frame's writes (line 10), with its bit alone as the sound
        // driver writes it: its first words in lines 11, 12.
        let on = [
            write(10, 0xa4, 4),
            write(10, 0xa6, 200),
            write(10, 0x96, 0x8001),
        ];
        let at = |line: u32, h: u32| line * super::super::cpu::LINE + h;
        let fixed = intro.fixed(&m, 1, &on);
        assert!(fixed.taken(at(11, 0x0d)) && fixed.taken(at(12, 0x0d)));
        assert!(!intro.fixed(&m, 1, &[]).taken(at(11, 0x0d)));
        // A frame on, the channel plays on through the frames a draw runs into.
        intro.audio.frame(&on, LINES as u32);
        let fixed = intro.fixed(&m, 2, &[]);
        let mut ahead = intro.audio.clone();
        for k in 0..2 {
            let slots = ahead.frame(&[], LINES as u32).to_vec();
            assert!(slots.len() > 100, "frame {k}: {} slots", slots.len());
            for (c, n) in slots {
                assert_eq!(n, 0);
                assert!(fixed.taken(k * sim::FRAME + c));
            }
        }
    }

    #[test]
    fn a_wait_read_on_line_305_or_later_runs_on_to_the_next_frame() {
        assert_eq!(wait_lines(0), 8);
        assert_eq!(wait_lines(304), 8);
        // Read on 305, eight lines on is 313, which no frame has: line 57 has the same low byte.
        assert_eq!(305 + wait_lines(305) - LINES, 57);
        assert_eq!(312 + wait_lines(312) - LINES, 64);
        // Read in the next frame already: an ordinary wait.
        assert_eq!(wait_lines(LINES), 8);
    }

    /// The handler taken on line `entry` of frame 0 (the main program over, the music off), then
    /// two frames more: per frame its `vbl_count` and its acknowledges' lines.
    fn handlers_from(entry: u16) -> Vec<(u16, Vec<u16>)> {
        let mut m = StoryMem::default();
        let data = data::StoryData::default();
        let mut intro = Intro::start(&mut m, &data);
        intro.first = false;
        intro.ended = true;
        (0..3u64)
            .map(|f| {
                let out = intro.frame(
                    &mut m,
                    &data,
                    crate::input::FrameInput::NONE,
                    f,
                    None,
                    scenes::BLACK_LIST,
                    Pacing::Unpaced,
                    Vec::new(),
                    (f == 0).then_some((entry, 0)),
                    None,
                    None,
                );
                let acks = out
                    .paula
                    .iter()
                    .filter(|w| w.reg == 0x9c && w.value == 0x20)
                    .map(|w| w.vpos)
                    .collect();
                (m.w(VBL_COUNT), acks)
            })
            .collect()
    }

    #[test]
    fn a_handler_whose_wait_runs_past_the_frames_end_ends_in_the_next_which_has_none_of_its_own() {
        // Idle, the wait is read a line after the entry: on 306 from 305, so it ends on line 58
        // of frame 1; frame 1's blank, cleared by that acknowledge, gets no handler.
        assert_eq!(
            handlers_from(305),
            vec![(1, vec![]), (1, vec![58]), (2, vec![0])]
        );
        // Read on 304: an ordinary handler, all in its frame.
        assert_eq!(
            handlers_from(303),
            vec![(1, vec![0]), (2, vec![0]), (3, vec![0])]
        );
    }
}
