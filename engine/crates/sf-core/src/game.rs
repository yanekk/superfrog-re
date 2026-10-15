//! The decision function (DESIGN §3.3): `Game` is a function of the [`GameData`], its [`Start`]
//! and the inputs, and nothing else.

use std::sync::Arc;

use sf_assets::{GameData, LevelId};
use sf_paula::PaulaWrite;

use crate::audio::AudioDriver;
use crate::input::{FrameInput, KeySet};
use crate::layout::{AmigaMemory, Ram, VarId, h0};
use crate::level::Hw;
use crate::render::{DisplayData, Picture, View};
use crate::rng::Rng;
use crate::scene::{Machine, Scene};
use crate::state::State;

/// Where a game starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// Switch on. The original seeds its RNG once, from a clock, while it boots (docs/08 "RNG");
    /// the seed is part of the start (owner, 2026-10-11): normal play passes one from the host
    /// clock, a power-on conformance run the oracle's. The boot scene (T20/T21) puts it in place
    /// at the point the original seeds; until then the state stays as power-on leaves it.
    PowerOn { seed: Rng },
    /// Continue from the original's memory at a sync point: every [`crate::LAYOUT`] variable,
    /// `rng_state` included, is copied in.
    Transplant(AmigaMemory),
    /// Enter a level as the original does (T14).
    Level(LevelId),
    /// Continue from the original's memory and its CPU at a sync point: the transplant, and where
    /// the original's main program was, so a front-end routine goes on from the same place with
    /// the same registers (T20). A place the engine does not know continues as a transplant.
    Resume { memory: AmigaMemory, cpu: Cpu },
    /// Switch on with disk 3 in the drive (T32): the Team 17 logo and the story intro, then, once
    /// the intro has asked for disk 1 and fire is pressed, the game as [`Start::PowerOn`] starts
    /// it, with `seed`.
    Disk3 { seed: Rng },
}

/// The original's CPU at the end of a frame, just before the vertical blank's interrupt: the main
/// program's registers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Cpu {
    pub d: [u32; 8],
    pub a: [u32; 8],
    pub pc: u32,
    pub sr: u16,
    pub usp: u32,
    pub ssp: u32,
}

/// For conformance, the story intro: where the original's vertical-blank handler ended, which
/// decides where a draw after it starts (owner, T46; T54).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerEnd {
    /// The beam (line, DMA cycle) of its acknowledge's write (`INTREQ = $20`).
    pub ack: (u16, u16),
    /// The main program's instruction the interrupt came in before (the PC it returns to), when the
    /// original's step began at the handler's entry.
    pub resumes: Option<u32>,
}

/// How the original's main loop kept time on this frame (DESIGN §2.1; owner, 2026-10-11).
///
/// In a level the original's main loop runs one 25 Hz pass every second vertical blank, and the
/// 50 Hz work (frog, camera, clock) runs in the vertical-blank interrupt. In busy stretches the
/// pass overruns, so a vertical blank arrives while it is still running (a late pass; docs/08
/// "Lag"). Normal play never lags; conformance runs feed the original's schedule from the oracle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Pacing {
    /// No schedule given: the engine keeps its own time (normal play, always). In a level this is
    /// [`Pacing::OnTime`]; in the story intro the engine's own timing model decides when its long
    /// jobs end.
    #[default]
    Unpaced,
    /// The main loop was waiting when this frame's vertical blank came.
    OnTime,
    /// The original's frame ended with its main loop in the middle of a pass, so the next
    /// vertical blank comes there: after the frame's `writes`th Paula write, the last at beam
    /// line `line`, slot `hpos`, in the pass's `phase`th call (`level_frame_loop`'s, in order from
    /// `spud_update`; [`crate::level::frame::pass`]), `step` steps into its loop (slots looked at,
    /// [`crate::level::beam::NO_STEP`] for a call without). The engine ends its frame between the
    /// same two writes, at the same step of the same call, or where its clock reaches the frame's
    /// end counting from that write (T36), and runs the rest of the pass in the next frame, after
    /// that frame's interrupt. In the story intro (owner, 2026-10-12): the original was still in a
    /// long job (a draw, an unpack, a scene's set-up), and the engine's job in progress ends no
    /// sooner; [`Pacing::OnTime`] there means it had ended, and the engine's ends with the frame.
    /// The intro reads none of the fields.
    Late { writes: u16, line: u16, hpos: u16, phase: u8, step: u16 },
}

/// What one frame produced.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FrameOut {
    pub picture: Picture,
    /// The frame's Paula register writes, in order (DESIGN §2.1 (2)).
    pub paula: Vec<PaulaWrite>,
    /// The frame loaded from disk, which the engine does at once (owner, 2026-10-12): where the
    /// original's main program is once its load is over, so conformance can skip the original's
    /// frames spent loading (DESIGN §2.1).
    pub load: Option<LoadEnd>,
    /// With `load`: memory the load wrote that the engine does not model, `(address, length)`.
    pub resync: Vec<(u32, u32)>,
    /// CPU jobs the engine did at once that the original's CPU may still be running at the end
    /// of this frame (owner, 2026-10-12): conformance compares their memory once it is done.
    pub jobs: Vec<Job>,
    /// Raster-line ranges `start..end` a drawing step wrote while the beam was within
    /// [`RACE_LINES`] of them (owner, 2026-10-13): the original's picture there depends on which
    /// side of the beam its write fell, so conformance excuses a pixel differing only on them.
    pub beam_races: Vec<(u16, u16)>,
}

/// How near the beam, in lines, a drawing step's write must be to race it.
pub const RACE_LINES: u16 = 4;

/// The lines `start..end` a drawing step wrote with the beam on line `beam`, when the beam was
/// within [`RACE_LINES`] of them (above, inside or just past them): the range to put in
/// [`FrameOut::beam_races`].
pub fn beam_race((start, end): (u16, u16), beam: u16) -> Option<(u16, u16)> {
    if start >= end {
        return None;
    }
    let gap = if beam < start {
        start - beam
    } else if beam >= end {
        beam - (end - 1)
    } else {
        0
    };
    (gap <= RACE_LINES).then_some((start, end))
}

/// How conformance knows the original's load is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadEnd {
    /// The original's main program has this return address on its stack (a call it makes after
    /// the load).
    Reached(u32),
    /// The original's main program has been in the call with this return address (the load) and
    /// is out of it again.
    Left(u32),
}

/// A CPU job of the original's that can run past the end of a frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Job {
    /// Return addresses on the original's stack while the job runs (any one of them).
    pub inside: Vec<u32>,
    /// The memory it writes, `(address, length)`.
    pub writes: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    state: State,
    /// What the display reads from the disks (shared: it never changes once built).
    display: Arc<DisplayData>,
    scene: Scene,
    /// The front end's program, while [`Scene::Front`] runs.
    front: Option<crate::front::Front>,
    /// The story intro's program, while [`Scene::Story`] runs.
    story: Option<Box<crate::front::Story>>,
    /// The front end's last picture: drawn as the beam saw the frame, mid-step.
    shown: Option<Picture>,
    /// The seed a power-on start is waiting to put in place.
    seed: Option<Rng>,
    frame: u32,
    /// The keyboard: the keys held last frame, and the byte it last sent, which CIA-A's serial
    /// register holds (the code shifted up one, bit 0 set for a key going up, all inverted).
    keys: KeySet,
    sdr: u8,
    /// The level runs with no object kinds ([`Game::frog_only`]).
    frog_only: bool,
    /// What a level's work costs on the beam ([`crate::level::beam`]), and the last frame's clock
    /// steps when kept ([`Game::keep_trace`]).
    costs: [u32; crate::level::beam::Cost::COUNT],
    trace: Option<Vec<crate::level::beam::Tick>>,
    /// A level's pass the last frame ended in the middle of ([`crate::level::beam::Pending`]).
    pending: Option<crate::level::beam::Pending>,
    /// The level a [`Start::Level`] game entered.
    entered: Option<LevelId>,
    /// For the next step: where the original made that frame's Paula writes.
    schedule: Vec<(u16, u16)>,
    blits: Vec<(u16, u16, u16)>,
    /// How the last level's loop ended (T37).
    outcome: Option<crate::level::LevelOutcome>,
    /// What the slot machine gave, once it has returned (T73).
    slot_outcome: Option<crate::slot::SlotOutcome>,
    /// Main-loop passes ended since the start, and whether the last step ended one
    /// ([`Game::passes`]).
    passes: u64,
    passes_ended: u32,
    pass_clock: PassClock,
    /// For conformance: what runs on by itself while the original loads the game after the story
    /// intro ([`crate::front::RUNS_DURING_LOAD`]), put in place when the intro hands over.
    handover_runs: Vec<(u32, Vec<u8>)>,
}

/// `key_down`, `key_up` (h0+$16078, h0+$16079).
const KEY_DOWN: u32 = h0(0x16078);
const KEY_UP: u32 = h0(0x16079);

/// The byte the keyboard sends for `code` going down or up.
fn key_byte(code: u8, up: bool) -> u8 {
    !(code << 1 | up as u8)
}

/// The sound driver and the keyboard's last byte as the original's memory `mem` has them.
fn level_io(display: &DisplayData, mem: &AmigaMemory) -> (AudioDriver, u8) {
    let word = |a: u32| (mem.byte(a) as u32) << 8 | mem.byte(a + 1) as u32;
    let long = |a: u32| word(a) << 16 | word(a + 2);
    let files = if display.sound().is_complete() {
        crate::audio::files_for(long(h0(0x40da)), long(h0(0x9ec)))
    } else {
        Vec::new()
    };
    let audio = AudioDriver::transplant(display.sound(), &|a| mem.byte(a), &files);
    // The keyboard's last byte, as the last poll decoded it.
    let sdr = match (mem.byte(KEY_DOWN), mem.byte(KEY_UP)) {
        (0, up) if up != 0 => key_byte(up, true),
        (down, _) => key_byte(down, false),
    };
    (audio, sdr)
}

impl Game {
    pub fn new(data: &GameData, start: Start) -> Game {
        Game::with_display(Arc::new(DisplayData::new(data)), start)
    }

    /// [`Game::new`] with the disks' display content already built, as a caller making many
    /// games from one [`GameData`] (conformance runs) keeps it.
    pub fn with_display(display: Arc<DisplayData>, start: Start) -> Game {
        let mut story = None;
        let mut sdr = 0xff;
        let entered = match start {
            Start::Level(id) => Some(id),
            _ => None,
        };
        let (state, scene, seed, front) = match start {
            Start::PowerOn { seed } => (
                State::default(),
                Scene::Front,
                Some(seed),
                Some(crate::front::Front::power_on(seed)),
            ),
            Start::Transplant(mem) => {
                let mut state = State::transplant(&mem);
                (state.audio, sdr) = level_io(&display, &mem);
                (state, Scene::of(&mem), None, None)
            }
            Start::Resume { memory, cpu } => {
                let mut state = State::transplant(&memory);
                match crate::front::Front::resume(&mut state, &display, &memory, &cpu) {
                    Some(f) => {
                        let scene = f.scene(&state, Scene::Front);
                        (state, scene, None, Some(f))
                    }
                    None => {
                        (state.audio, sdr) = level_io(&display, &memory);
                        (state, Scene::of(&memory), None, None)
                    }
                }
            }
            Start::Level(id) => {
                let mut state = State::default();
                crate::level::load::enter(&mut state, &display, id);
                (state, Scene::of_level(id), None, None)
            }
            Start::Disk3 { seed } => {
                story = Some(Box::new(crate::front::Story::power_on(&display.story)));
                (State::default(), Scene::Story, Some(seed), None)
            }
        };
        let (mut state, mut front) = (state, front);
        if let Some(f) = front.as_mut() {
            f.boot(&mut state, &display);
        }
        Game {
            state,
            display,
            scene,
            front,
            story,
            shown: None,
            seed,
            frame: 0,
            keys: KeySet::NONE,
            sdr,
            frog_only: false,
            costs: crate::level::beam::COSTS,
            trace: None,
            pending: None,
            entered,
            schedule: Vec::new(),
            blits: Vec::new(),
            outcome: None,
            slot_outcome: None,
            passes: 0,
            passes_ended: 0,
            pass_clock: PassClock::default(),
            handover_runs: Vec::new(),
        }
    }

    /// One PAL frame of normal play: the main loop is never late.
    pub fn step(&mut self, input: FrameInput) -> FrameOut {
        self.step_paced(input, Pacing::Unpaced)
    }

    /// One PAL frame on the original's schedule (conformance runs; see [`Pacing`]).
    pub fn step_paced(&mut self, input: FrameInput, pacing: Pacing) -> FrameOut {
        self.step_inner(input, pacing, true)
    }

    /// [`Game::step`] without drawing a level frame's picture, which nothing in the state reads:
    /// the game is left exactly as `step` leaves it, and the [`FrameOut`]'s picture is empty when
    /// a level frame would have been drawn. For planners only, which step the engine thousands of
    /// times a frame of plan and never look (T104, `examples/plan_exit_w6`); conformance runs and the
    /// app keep [`Game::step`]. Locked by `sf-conform`'s `tests/step_unseen.rs`.
    pub fn step_unseen(&mut self, input: FrameInput) -> FrameOut {
        self.step_inner(input, Pacing::Unpaced, false)
    }

    fn step_inner(&mut self, input: FrameInput, pacing: Pacing, draw: bool) -> FrameOut {
        let out = self.step_body(input, pacing, draw);
        // Paused, the main loop is in neither its wait nor its pass (T191).
        self.passes_ended = self.pass_clock.frame(
            self.scene.is_level() && self.state.pause.is_none(),
            self.pending.is_some(),
            self.state.w(VBL_COUNT),
        );
        self.passes += self.passes_ended as u64;
        out
    }

    fn step_body(&mut self, input: FrameInput, pacing: Pacing, draw: bool) -> FrameOut {
        // A key going down or up this frame is the byte the keyboard sends (the last one wins).
        let key_in = input.keys != self.keys;
        for code in input.keys.codes() {
            if !self.keys.contains(code) {
                self.sdr = key_byte(code, false);
            }
        }
        for code in self.keys.codes() {
            if !input.keys.contains(code) {
                self.sdr = key_byte(code, true);
            }
        }
        self.keys = input.keys;
        self.frame += 1;
        if let (Scene::Story, Some(story)) = (self.scene, self.story.as_mut()) {
            let out = story.step(&self.state, &self.display, input, pacing);
            if !out.over {
                self.handover_runs.clear();
                self.shown = Some(out.picture.clone());
                return FrameOut {
                    picture: out.picture,
                    paula: out.paula,
                    load: out.load,
                    resync: out.resync,
                    jobs: out.jobs,
                    beam_races: Vec::new(),
                };
            }
            // The intro has handed over: the game starts as a disk-1 power-on does, and its first
            // frame stands for this one (the original loads the game meanwhile).
            let seed = self.seed.take().unwrap_or_default();
            let mut front = crate::front::Front::power_on(seed);
            self.state = State::default();
            front.boot(&mut self.state, &self.display);
            for (at, bytes) in std::mem::take(&mut self.handover_runs) {
                self.state.poke(at, &bytes);
            }
            self.front = Some(front);
            self.story = None;
            self.scene = Scene::Front;
            self.seed = Some(seed);
        }
        if let (Scene::Front | Scene::Slot { .. }, Some(front)) = (self.scene, self.front.as_mut())
        {
            self.seed = None;
            front.anchor_writes(std::mem::take(&mut self.schedule));
            let out = front.step(&mut self.state, &self.display, input);
            self.scene = front.scene(&self.state, self.scene);
            self.shown = Some(out.picture.clone());
            if let Some(o) = front.outcome() {
                self.outcome = Some(o);
            }
            if let Some(o) = front.slot_outcome() {
                self.slot_outcome = Some(o);
            }
            if front.in_level_loop() {
                // `level_start` is over and `level_frame_loop` entered (T37): the level scene
                // from the next frame, its poll reading the keyboard's register as it is.
                self.sdr = front.keyboard_sdr();
                self.state.kbd_flag = front.keyboard_new();
                self.state.kbd_acked = false;
                self.front = None;
                self.scene = Scene::level(self.state.l(h0(0x40da)), self.state.l(h0(0x40de)));
                self.pending = None;
            }
            return FrameOut {
                picture: out.picture,
                paula: out.paula,
                load: out.load,
                resync: out.resync,
                jobs: out.jobs,
                beam_races: out.beam_races,
            };
        }
        let mut hw = Hw::of(input);
        hw.sdr = self.sdr;
        hw.key_in = key_in;
        let mut ended: Option<(crate::front::Front, Vec<PaulaWrite>, Vec<Job>)> = None;
        let display = Arc::clone(&self.display);
        let frog_only = self.frog_only;
        let sdr = self.sdr;
        let mut after = |s: &mut State, line: u16| {
            // `level_frame_loop` returned: what follows runs on the front end's program, from
            // where the loop's last pass ended (T37, `crate::level::exit`).
            let mut front = crate::front::Front::level_over(s, &display.front, sdr, frog_only);
            let (more, jobs) = front.finish_frame(s, &display, input, line);
            ended = Some((front, more, jobs));
        };
        let (mut paula, shown) = {
            let mut m = Machine {
                state: &mut self.state,
                display: &self.display,
                hw,
                dispatch: if self.frog_only {
                    crate::level::frog_only_dispatch()
                } else {
                    crate::level::standard_dispatch()
                },
                costs: &self.costs,
                trace: self.trace.as_mut(),
                schedule: std::mem::take(&mut self.schedule),
                blits: std::mem::take(&mut self.blits),
            };
            self.scene
                .step(&mut m, &mut self.pending, pacing, View::default(), &mut after)
        };
        let mut jobs = Vec::new();
        if let Some((front, more, begun)) = ended {
            self.outcome = Some(crate::level::exit::outcome_of_loop(&self.state));
            paula.extend(more);
            jobs = begun;
            self.front = Some(front);
            self.scene = Scene::Front;
            self.pending = None;
            self.shown = shown.clone();
        }
        FrameOut {
            picture: match shown {
                Some(p) => p,
                None if draw => self.render(View::default()),
                None => Picture::default(),
            },
            paula,
            load: None,
            resync: Vec::new(),
            jobs,
            beam_races: Vec::new(),
        }
    }

    /// The frame the original shows for this state (DESIGN §3.3), widened by `view`.
    pub fn render(&self, view: View) -> Picture {
        match (self.scene, &self.shown) {
            (Scene::Front | Scene::Slot { .. } | Scene::Story, Some(p)) => p.widened(view),
            _ => self.scene.render(&self.state, &self.display, view),
        }
    }

    /// For conformance: the line of the frame on which the original's load, which the engine does
    /// at once, ended. The engine's main program goes on from there after its next load (normal
    /// play goes on at once). The runner finds the line by trying them (DESIGN §2.1).
    pub fn set_load_line(&mut self, line: Option<u16>) {
        if let Some(f) = self.front.as_mut() {
            f.load_line = line;
        }
        if let Some(s) = self.story.as_mut() {
            s.load_line = line;
        }
    }

    /// For conformance, the story intro: where the original's CPU took the next frame's
    /// vertical-blank interrupt (line, DMA cycle), which decides when its handler's writes to the
    /// copper list land (owner, 2026-10-13). Normal play times it itself.
    pub fn set_handler_entry(&mut self, at: Option<(u16, u16)>) {
        if let Some(s) = self.story.as_mut() {
            s.handler_entry = at;
        }
    }

    /// For conformance, the story intro: where the original's handler of this frame ended and what
    /// it returned to ([`HandlerEnd`]), which decides where a draw after it starts (owner,
    /// 2026-10-13; T54). Normal play estimates it.
    pub fn set_handler_end(&mut self, end: Option<HandlerEnd>) {
        if let Some(s) = self.story.as_mut() {
            s.handler_end = end;
        }
    }

    /// For conformance, the story intro: whether the next frame's vertical blank goes without a
    /// handler, as the original's did when a blit held its CPU through the whole frame (its
    /// request is a latch; the next blank's handler serves both). `None`: the engine's own model.
    pub fn set_blank_merged(&mut self, merged: Option<bool>) {
        if let Some(s) = self.story.as_mut() {
            s.blank_merged = merged;
        }
    }

    /// For conformance: the memory that runs on by itself while the original
    /// loads ([`Game::runs_during_load`]'s runs, as the original had them just before the frame
    /// its load ended on), put in place at the moment the engine's instant load happens in its
    /// next step, after what came before the load in that step.
    pub fn set_load_state(&mut self, runs: Vec<(u32, Vec<u8>)>) {
        if let Some(s) = self.story.as_mut() {
            s.load_state = runs;
        } else if let Some(f) = self.front.as_mut() {
            // The front end's: after the work carried over from the last frame (that was the
            // frame the original's load began in), before this frame's interrupt (T183).
            f.load_state = runs;
        } else {
            for (at, bytes) in runs {
                self.state.poke(at, &bytes);
            }
        }
    }

    /// For conformance, the story intro's hand-over: what runs on by itself while the original
    /// loads the game ([`crate::front::RUNS_DURING_LOAD`]'s runs, as the original had them just
    /// before the frame its load ended on), put in place once the game has booted in the next
    /// step, if the intro hands over in it (the game's own power-on takes them before its step).
    pub fn set_handover_state(&mut self, runs: Vec<(u32, Vec<u8>)>) {
        if self.story.is_some() {
            self.handover_runs = runs;
        }
    }

    /// Front-end paths the engine has run into that are not ported yet (empty in every
    /// conformance case; a test fails on any).
    pub fn unported(&self) -> &[&'static str] {
        self.front.as_ref().map_or(&[], |f| &f.unported[..])
    }

    /// Every modelled variable's value, as the original would hold it at its [`crate::LAYOUT`]
    /// address.
    pub fn probe(&self) -> Vec<(VarId, u32)> {
        match &self.story {
            Some(s) => s
                .layout()
                .iter()
                .flat_map(|d| {
                    d.pieces()
                        .map(|p| (p.id, p.read(&s.mem)))
                        .collect::<Vec<_>>()
                })
                .collect(),
            None => self.state.probe(),
        }
    }

    /// The variables [`Game::probe`] gives, in its order: [`crate::LAYOUT`], or while the story
    /// intro runs, its own (its memory map is not the game's).
    pub fn layout(&self) -> &'static [crate::VarDef] {
        match &self.story {
            Some(s) => s.layout(),
            None => crate::LAYOUT,
        }
    }

    /// What runs on by itself while the original loads, which conformance takes from it (DESIGN
    /// §2.1): the front end's [`crate::front::RUNS_DURING_LOAD`], nothing yet for the story.
    pub fn runs_during_load(&self) -> &'static [(u32, u32)] {
        match &self.story {
            Some(s) => s.runs_during_load(),
            None => crate::front::RUNS_DURING_LOAD,
        }
    }

    /// What a load leaves that the engine does not model: [`crate::front::LOAD_LEAVES`], nothing
    /// for the story.
    pub fn load_leaves(&self) -> &'static [(u32, u32)] {
        match &self.story {
            Some(_) => &[],
            None => crate::front::LOAD_LEAVES,
        }
    }

    /// Write `bytes` at `addr` into the modelled variables, as [`State::poke`] does. For
    /// conformance tooling, which plants a known divergence to prove the runner finds it.
    pub fn poke(&mut self, addr: u32, bytes: &[u8]) {
        match self.story.as_mut() {
            Some(s) => s.mem.write(addr, bytes),
            None => self.state.poke(addr, bytes),
        }
    }

    /// `len` bytes of the original's memory as the engine holds it at `addr` (debugging and
    /// conformance tooling): the story's memory while it runs, else the modelled state.
    pub fn peek(&self, addr: u32, len: u32) -> Vec<u8> {
        match &self.story {
            Some(s) => s.mem.read(addr, len),
            None => self.state.read(addr, len),
        }
    }

    /// The story intro's program, while it runs (for tooling).
    pub fn story(&self) -> Option<&crate::front::Story> {
        self.story.as_deref()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn scene(&self) -> Scene {
        self.scene
    }

    /// The copper list the next frame starts with (COP1LC), which says what screen the game is
    /// on: a level's ([`crate::render::LEVEL_COPPER`]), the front end's as its program left it,
    /// `None` in the story intro (its own memory map) and the placeholder.
    pub fn cop1lc(&self) -> Option<u32> {
        match (self.scene, &self.front) {
            (Scene::Front | Scene::Slot { .. }, Some(f)) => Some(f.cop1lc()),
            (Scene::Level | Scene::Bonus(_), _) => Some(crate::render::LEVEL_COPPER),
            _ => None,
        }
    }

    /// The seed a power-on start has not yet put in place.
    pub fn pending_seed(&self) -> Option<Rng> {
        self.seed
    }

    /// Run levels with no object kinds: the frog alone, for conformance against an original whose
    /// objects are all frozen out (T14's frog cases).
    pub fn frog_only(&mut self) {
        self.frog_only = true;
        // A level entered and not yet played is entered again without its objects.
        if let (Some(id), 0) = (self.entered, self.frame) {
            self.state = State::default();
            crate::level::load::enter_with(
                &mut self.state,
                &self.display,
                id,
                crate::level::frog_only_dispatch(),
            );
        }
    }

    /// For conformance (owner, 2026-10-13): the next step's level work is put on the beam where
    /// the original made that frame's Paula writes, `(line, slot)` in order: as the engine makes
    /// its own frame's `k`th write, its clock is set to the `k`th. Its own costs place the work
    /// between them ([`crate::level::beam`]); normal play has only those. On the front end they
    /// time the audio DMA the slot machine's lamps run beside (T129, owner 2026-10-15).
    pub fn anchor_writes(&mut self, writes: &[(u16, u16)]) {
        self.schedule = writes.to_vec();
    }

    /// For conformance, with [`Game::anchor_writes`]: where the original started the next
    /// step's blits, `(BLTSIZE, line, slot)` in order; each of the engine's own frame's blits
    /// starts at the next of its size.
    pub fn anchor_blits(&mut self, blits: &[(u16, u16, u16)]) {
        self.blits = blits.to_vec();
    }

    /// For fitting the level's costs on the beam: run with `costs`.
    pub fn set_costs(&mut self, costs: [u32; crate::level::beam::Cost::COUNT]) {
        self.costs = costs;
    }

    /// For fitting the level's costs on the beam: keep each frame's clock steps.
    pub fn keep_trace(&mut self) {
        self.trace = Some(Vec::new());
    }

    /// The last frame's clock steps, while kept.
    pub fn trace(&self) -> &[crate::level::beam::Tick] {
        self.trace.as_deref().unwrap_or(&[])
    }

    /// How the last level's loop ended, once one has (T37): a death the level restarts after,
    /// the time up, the game over, or the exit.
    pub fn level_outcome(&self) -> Option<crate::level::LevelOutcome> {
        self.outcome
    }

    /// The slot machine, entered with `coins` coins put in as `level_tally` enters it (T73), from
    /// the state as it is (a level's tally over, its sound in chip RAM): `slot_machine` runs from
    /// the next frame, and returns to the tally's end ([`Game::slot_outcome`]).
    pub fn enter_slot(&mut self, coins: u16) {
        self.state.set_w(crate::slot::COINS, coins);
        self.front = Some(crate::front::Front::enter_slot(
            &self.state,
            &self.display.front,
            self.sdr,
            self.frog_only,
        ));
        self.scene = Scene::Slot { coins_in: coins };
        self.pending = None;
        self.slot_outcome = None;
    }

    /// What the slot machine gave, once `slot_machine` has returned (T73): the points its rounds
    /// added to the score and the lives.
    pub fn slot_outcome(&self) -> Option<crate::slot::SlotOutcome> {
        self.slot_outcome
    }

    /// Frames stepped since the start.
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Main-loop passes ended since the start ([`PassClock`]): what conformance matches the
    /// original's state by (DESIGN §2.1 check 1). In a level a pass is the 25 Hz one; outside, where
    /// the main loop waits every frame, a pass is a frame.
    pub fn passes(&self) -> u64 {
        self.passes
    }

    /// The passes the last step ended (0 to 2).
    pub fn passes_ended(&self) -> u32 {
        self.passes_ended
    }

    /// Whether the last step ended in the middle of a level's pass: the state is part way
    /// through it, not a pass's end.
    pub fn mid_pass(&self) -> bool {
        self.pending.is_some()
    }
}

/// `vbl_count` (h0+$9d8): vertical blanks since the level's pass last began.
const VBL_COUNT: u32 = h0(0x9d8);

/// Counts the main-loop passes frames end (DESIGN §2.1 check 1), by the same rule on both sides.
/// Outside a level, where the main loop waits every frame, a frame is a pass. In a level a pass
/// begins in a frame that ends with `vbl_count` 0 (`level_frame_loop` clears it as it begins one,
/// and the next blank has not come), and ends in the first frame that does not end in the middle
/// of it: the same frame, or, for a late pass, a later one (in which the next pass may begin and
/// still be running at the frame's end, its predecessor's end then never seen).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PassClock {
    /// A pass has begun and not been seen to end.
    running: bool,
}

impl PassClock {
    /// The passes a frame ended (0 to 2): `in_level`, in a level's loop at the frame's end;
    /// `mid_pass`, in the middle of a pass there.
    pub fn frame(&mut self, in_level: bool, mid_pass: bool, vbl_count: u16) -> u32 {
        if !in_level && !mid_pass {
            self.running = false;
            return 1;
        }
        let mut ended = 0;
        if vbl_count == 0 {
            ended += self.running as u32;
            self.running = true;
        }
        if !mid_pass && self.running {
            ended += 1;
            self.running = false;
        }
        ended
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_write_races_the_beam_within_four_lines_of_it() {
        use super::beam_race;
        assert_eq!(beam_race((120, 130), 118), Some((120, 130)));
        assert_eq!(beam_race((120, 130), 116), Some((120, 130)));
        assert_eq!(beam_race((120, 130), 125), Some((120, 130)));
        assert_eq!(beam_race((120, 130), 133), Some((120, 130)));
        assert_eq!(beam_race((120, 130), 110), None);
        assert_eq!(beam_race((120, 130), 115), None);
        assert_eq!(beam_race((120, 130), 134), None);
        assert_eq!(beam_race((120, 120), 120), None);
    }

    use super::*;

    /// DESIGN §2.1 check 1: what a frame's end says of the passes, as the original's main loop
    /// shows it (in a level: `vbl_count`, and whether it is in the middle of a pass).
    #[test]
    fn the_pass_clock_counts_passes_ended() {
        let mut c = PassClock::default();
        // Outside a level a frame is a pass.
        assert_eq!(c.frame(false, false, 7), 1);
        // On time: a pass begins and ends, then a frame's wait.
        assert_eq!(c.frame(true, false, 0), 1);
        assert_eq!(c.frame(true, false, 1), 0);
        // Late: begun, still running at the frame's end, ended in the next.
        assert_eq!(c.frame(true, true, 0), 0);
        assert_eq!(c.frame(true, false, 1), 1);
        // Late, and the next pass begun and running at the end of the frame the first ended in.
        assert_eq!(c.frame(true, true, 0), 0);
        assert_eq!(c.frame(true, true, 0), 1);
        assert_eq!(c.frame(true, false, 1), 1);
        // Out of the wait with no pass begun (the frame's end between its calls): nothing ended.
        assert_eq!(c.frame(true, true, 1), 0);
        assert_eq!(c.frame(true, true, 0), 0);
        assert_eq!(c.frame(true, false, 1), 1);
    }

    #[test]
    fn a_placeholder_game_steps_black_and_silent_and_keeps_its_state() {
        let mut mem = AmigaMemory {
            chip: vec![0; 16],
            slow: vec![0; 512 * 1024],
        };
        let at = (crate::layout::def(VarId::RngState).addr - crate::layout::SLOW_BASE) as usize;
        mem.slow[at + 3] = 9;
        let mut g = Game::new(&GameData::default(), Start::Transplant(mem));
        let before = g.probe();
        let out = g.step(FrameInput::NONE);
        assert!(out.paula.is_empty());
        assert_eq!((out.picture.width, out.picture.height), (320, 256));
        assert!((0..256).all(|y| (0..320).all(|x| out.picture.rgb12(x, y) == 0)));
        assert_eq!(g.probe(), before);
        assert_eq!(g.frame(), 1);
    }

    #[test]
    fn power_on_boots_the_front_end_with_its_seed() {
        // The boot is all load, so it is done at once: the RNG holds the seed the start gave.
        let seed = Rng::seeded(1, 2);
        let g = Game::new(&GameData::default(), Start::PowerOn { seed });
        assert_eq!(g.scene(), Scene::Front);
        assert_eq!(g.state().rng(), seed);
    }
}
