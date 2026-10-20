//! The decision function (DESIGN §3.3): `Game` is a function of the [`GameData`], its [`Start`]
//! and the inputs, and nothing else.

use std::sync::Arc;

use sf_assets::{GameData, LevelData, LevelId};
use serde::{Deserialize, Serialize};
use sf_paula::PaulaWrite;

use crate::audio::AudioDriver;
use crate::front::password::Progress;
use crate::input::{FrameInput, KeySet};
use crate::layout::{AmigaMemory, Ram, VarId, h0};
use crate::level::Hw;
use crate::render::{DisplayData, Picture, View};
use crate::recording::{LevelSetup, Location};
use crate::rng::Rng;
use crate::scene::{Machine, Scene, SceneKind};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Pacing {
    /// No schedule given: the engine keeps its own time (normal play, always). In a level this is
    /// [`Pacing::OnTime`]; in the story intro the engine's own timing model decides when its long
    /// jobs end.
    #[default]
    Unpaced,
    /// The main loop was waiting when this frame's vertical blank came.
    OnTime,
    /// As [`Pacing::OnTime`], with the main loop's next pass already due as the frame ended
    /// (`vbl_count` 2 or more in `level_frame_loop`'s wait): the pass before it returned too
    /// late in the frame to begin it, so it begins after the next vertical blank, not in this
    /// frame (T196: W6-4 5230, a pass held over the late frames 5228-5229 returning near the
    /// frame's end). The engine run alone, with no Paula writes to place it, begins none here.
    PassDue,
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
    /// `blanks` is how many vertical blanks the pass had met by then, counted from its start
    /// (its `vbl_count`, which the pass zeroes as it begins): which of the engine's passes is the
    /// original's, so one the engine gets to the end of is cut there, not run on into the next
    /// (T184). `point` is where in the step running the frame ended
    /// ([`crate::level::beam::Cut::point`]; 0: none). The intro reads none of the fields.
    Late { writes: u16, line: u16, hpos: u16, phase: u8, step: u16, point: u8, blanks: u16 },
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
    /// The game asked for a disk that was not in the drive (T21): the original's insert-disk
    /// prompt. The engine has every disk (the app's cache), so it goes on at once: the app may
    /// show the swap, never wait for it.
    pub disk_prompt: Option<DiskPrompt>,
    /// Paula writes the original made in frames conformance skips, inside a load the engine does
    /// at once (T213: `music_fade_wait`'s `music_stop` before the slot machine's, the end
    /// sequence's and the title's loads). Never compared; a mixer plays them before `paula`
    /// ([`FrameOut::mixed`]), or the channels the original turned off play on after the load.
    pub unheard: Vec<PaulaWrite>,
}

impl FrameOut {
    /// What a mixer plays for this frame: [`FrameOut::unheard`], then [`FrameOut::paula`].
    pub fn mixed(&self) -> Vec<PaulaWrite> {
        self.unheard.iter().chain(&self.paula).copied().collect()
    }
}

/// What the original asks the player to do with the drive (T21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiskPrompt {
    /// Put this disk in (`disk_request`, h0+$ade4, found it in no drive).
    Insert(sf_disk::DiskSlot),
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Job {
    /// Return addresses on the original's stack while the job runs (any one of them).
    pub inside: Vec<u32>,
    /// The memory it writes, `(address, length)`.
    pub writes: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Game {
    state: State,
    /// What the display reads from the disks (shared: it never changes once built). Not in a save
    /// state: [`Game::load`] builds it again from the `GameData` it is given.
    #[serde(skip)]
    display: Arc<DisplayData>,
    scene: Scene,
    /// The front end's program, while [`Scene::Front`] runs.
    front: Option<crate::front::Front>,
    /// The story intro's program, while [`Scene::Story`] runs.
    story: Option<Box<crate::front::Story>>,
    /// The front end's last picture: drawn as the beam saw the frame, mid-step.
    shown: Option<Picture>,
    /// How [`Game::step`]'s picture is drawn: presentation only, nothing the game reads (DESIGN
    /// §2.7). Not in a save state: the app sets it from its own setting.
    #[serde(skip)]
    view: View,
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
    #[serde(with = "crate::savestate::big_array")]
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
    /// [`Game::progress`].
    progress: Progress,
    /// For conformance: what runs on by itself while the original loads the game after the story
    /// intro ([`crate::front::RUNS_DURING_LOAD`]), put in place when the intro hands over.
    handover_runs: Vec<(u32, Vec<u8>)>,
    /// What the levels met and do not follow, since the start ([`LevelCtx::unported`]), and that
    /// with the front end's, as [`Game::unported`] gives it.
    #[serde(with = "crate::savestate::static_str::vec")]
    level_unported: Vec<&'static str>,
    #[serde(with = "crate::savestate::static_str::vec")]
    unported: Vec<&'static str>,
    /// The `.sflevel` maps put in `display` in place of the disks' ([`Game::with_override`]),
    /// encoded: a save state carries them, as it does not carry `display`.
    overrides: Vec<(LevelId, Vec<u8>)>,
    /// The last place [`Game::location`] named, for a frame that names none (a fade's blank list).
    place: Option<Location>,
    /// The Paula writes of each vertical blank the last [`Game::load_blanks`] ran, until
    /// [`Game::take_load_paula`] takes them (T30). Not in a save state.
    #[serde(skip)]
    load_paula: Vec<Vec<PaulaWrite>>,
}

/// [`Game::type_code`] gives up waiting for the title after this many frames.
const START_AT_FRAMES: u32 = 4000;

/// `opt_lives` and `opt_game_mode` as power-on leaves them: 3 lives, normal.
const POWER_ON_OPTIONS: (u8, u8) = (0, 1);

/// [`Game::start_level`] gives up on reaching the level after this many frames from START GAME
/// (the original's loads, held instant here, and the final level's way through W6-4's tally).
const START_LEVEL_FRAMES: u32 = 20_000;

/// `level_state` (h0+$a06): 1 ends the level as its exit does.
const LEVEL_STATE: u32 = h0(0xa06);

/// When [`Game::type_code`] presses what on the options screen, in frames: OPTIONS taken after
/// the menu's down, the first down to LEVEL CODE after that, each next down after the last, the
/// first digit after the third down, each digit after the last, and fire the last digit's
/// `digit + leave` frames on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeTimes {
    pub options: u32,
    pub first_down: u32,
    pub down: u32,
    pub first_digit: u32,
    pub digit: u32,
    pub leave: u32,
}

/// [`Game::start_at`]'s: `sf-conform`'s `sync::drive_to` (and `chain-level-code`'s script),
/// proven on the original.
pub const CODE_TIMES: CodeTimes = CodeTimes {
    options: 50,
    first_down: 100,
    down: 40,
    first_digit: 100,
    digit: 16,
    leave: 40,
};

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

    /// Play `ov` (a `.sflevel`, DESIGN §2.10) in place of the disk level it names: entered as
    /// [`Start::Level`] enters it, through the same loader, so an unchanged export plays as the
    /// disk level does. Every other level stays the disks'.
    pub fn new_with_override(data: &GameData, ov: LevelData) -> Game {
        Game::with_override(Arc::new(DisplayData::new(data)), ov)
    }

    /// [`Game::new_with_override`] with the disks' display content already built.
    pub fn with_override(display: Arc<DisplayData>, ov: LevelData) -> Game {
        let map = sf_assets::t7mp_encode(&ov.t7mp);
        let mut d = (*display).clone();
        d.levels.insert(ov.id, map.clone());
        let mut game = Game::with_display(Arc::new(d), Start::Level(ov.id));
        game.overrides.push((ov.id, map));
        game
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
        let mut level_unported = Vec::new();
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
                level_unported = crate::level::load::enter(&mut state, &display, id);
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
            view: View::default(),
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
            progress: Progress { world: 0, level: 0 },
            handover_runs: Vec::new(),
            unported: level_unported.clone(),
            level_unported,
            overrides: Vec::new(),
            place: None,
            load_paula: Vec::new(),
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
        if let Some(p) = self.place_now() {
            self.place = Some(p);
        }
        if let Some(p) = self.playing() {
            self.progress = p;
        }
        self.gather_unported();
        out
    }

    /// [`Game::unported`]'s list: the front end's, then the levels'.
    fn gather_unported(&mut self) {
        self.unported.clear();
        if let Some(f) = self.front.as_ref() {
            self.unported.extend_from_slice(&f.unported);
        }
        for u in &self.level_unported {
            if !self.unported.contains(u) {
                self.unported.push(u);
            }
        }
    }

    /// The level a game (not the attract demo) is playing this frame, as a [`Progress`]: the final
    /// level as W6-4, whose code reaches it.
    fn playing(&self) -> Option<Progress> {
        let s = &self.state;
        if !self.scene.is_level() || s.l(h0(0x18)) != 0 {
            return None;
        }
        let (world, level) = (s.l(h0(0x40da)), s.l(h0(0x40de)));
        if s.w(h0(0x1e)) != 0 {
            return Some(Progress { world: 6, level: 4 });
        }
        ((1..=7).contains(&world) && (1..=4).contains(&level)).then_some(Progress {
            world: world as u8,
            level: level as u8,
        })
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
            let mut out = story.step(&self.state, &self.display, input, pacing);
            self.load_paula.append(&mut out.load_paula);
            if !out.over {
                self.handover_runs.clear();
                self.shown = Some(out.picture.clone());
                return FrameOut {
                    picture: out.picture.widened(self.view),
                    paula: out.paula,
                    load: out.load,
                    resync: out.resync,
                    jobs: out.jobs,
                    beam_races: Vec::new(),
                    disk_prompt: None,
                    unheard: Vec::new(),
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
            let level_list = front.cop1lc() == crate::render::LEVEL_COPPER;
            self.scene = front.scene(&self.state, self.scene);
            self.shown = Some(out.picture.clone());
            if let Some(o) = front.outcome() {
                self.outcome = Some(o);
            }
            if let Some(o) = front.slot_outcome() {
                self.slot_outcome = Some(o);
            }
            let mut paula = out.paula;
            if front.in_level_loop() {
                // `level_start` is over and `level_frame_loop` entered (T37): the level scene
                // from the next frame, its poll reading the keyboard's register as it is.
                let entered = front.loop_entered_on();
                self.sdr = front.keyboard_sdr();
                self.state.kbd_flag = front.keyboard_new();
                self.state.kbd_acked = false;
                self.front = None;
                self.scene = Scene::level(self.state.l(h0(0x40da)), self.state.l(h0(0x40de)));
                self.pending = None;
                // With `vbl_count` left at 2 or more by the last level's last pass, the loop's
                // first pass is due at once and begins in this frame, where `level_start` left
                // the beam (T209: W2-2 in `game-whole`, the original's pass ending this frame
                // in its middle).
                if let Some(line) = entered
                    && line < crate::level::beam::FRAME_LINES
                    && crate::level::n_clear(self.state.w(VBL_COUNT).wrapping_sub(2))
                {
                    self.pending = Some(crate::level::beam::Pending {
                        start: Box::new(self.state.clone()),
                        clock: crate::level::beam::Clock { line, into: 0 },
                        hw: Hw {
                            sdr: self.sdr,
                            key_in,
                            ..Hw::of(input)
                        },
                        pre: paula.len() as u16,
                        frames: Vec::new(),
                        vbl: false,
                    });
                    paula.extend(self.level_frame(input, pacing, key_in, false).paula);
                }
            }
            let picture = if level_list {
                crate::render::level_widened(&out.picture, &self.state, &self.display, self.view)
            } else {
                out.picture.widened(self.view)
            };
            return FrameOut {
                picture,
                paula,
                load: out.load,
                resync: out.resync,
                jobs: out.jobs,
                beam_races: out.beam_races,
                disk_prompt: out.disk_prompt,
                unheard: out.unheard,
            };
        }
        self.level_frame(input, pacing, key_in, draw)
    }

    /// A frame of the level's main program (or the rest of one, a pass begun in it pending).
    fn level_frame(
        &mut self,
        input: FrameInput,
        pacing: Pacing,
        key_in: bool,
        draw: bool,
    ) -> FrameOut {
        let mut hw = Hw::of(input);
        hw.sdr = self.sdr;
        hw.key_in = key_in;
        let mut ended: Option<(crate::front::Front, Vec<PaulaWrite>, Vec<Job>)> = None;
        let display = Arc::clone(&self.display);
        let frog_only = self.frog_only;
        let sdr = self.sdr;
        let mut after = |s: &mut State, line: u16, into: u32| {
            // `level_frame_loop` returned: what follows runs on the front end's program, from
            // where the loop's last pass ended (T37, `crate::level::exit`).
            let mut front = crate::front::Front::level_over(s, &display.front, sdr, frog_only);
            let (more, jobs) = front.finish_frame(s, &display, input, line, into);
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
                unported: &mut self.level_unported,
            };
            self.scene
                .step(&mut m, &mut self.pending, pacing, self.view, &mut after)
        };
        // Each frame's writes are its own: one placed before the one ahead of it (the engine's
        // clock past where the pacing puts the original's) is no later frame's, though the mixer
        // would take it for one (T213: W6-2's `sfx_play`, its DMACON on a frame late).
        crate::front::in_beam_order(&mut paula);
        let mut jobs = Vec::new();
        if let Some((front, more, begun)) = ended {
            self.outcome = Some(crate::level::exit::outcome_of_loop(&self.state));
            paula.extend(more);
            jobs = begun;
            self.front = Some(front);
            self.scene = Scene::Front;
            self.pending = None;
            // Kept 4:3, as the front end's are: `render` widens it.
            self.shown = shown.as_ref().map(|p| p.narrowed(self.view));
        }
        FrameOut {
            picture: match shown {
                Some(p) => p,
                None if draw => self.render(self.view),
                None => Picture::default(),
            },
            paula,
            load: None,
            resync: Vec::new(),
            jobs,
            beam_races: Vec::new(),
            disk_prompt: None,
            unheard: Vec::new(),
        }
    }

    /// Frames the original's last load took beyond the one the engine's instant load stands
    /// for, which the engine cannot know (the disk's timing): the vertical blank's sound and
    /// keyboard poll on each, as the original runs them while the trackloader has the main
    /// program. Conformance's engine run alone is given
    /// them as inputs, as the clock's seeds are (DESIGN §2.1 check 4; owner, T209). Their Paula
    /// writes, a frame's for each blank, are kept for [`Game::take_load_paula`]. In the story
    /// intro they run at its load in the next step, after the music it starts just before (T214).
    pub fn load_blanks(&mut self, frames: u32) {
        self.load_paula.clear();
        if let Some(story) = self.story.as_mut() {
            // The intro's handler's, run at the load in the next step (T214).
            story.blanks = frames;
        } else if let Some(front) = self.front.as_mut() {
            front.load_blanks(&mut self.state, &self.display, frames, &mut self.load_paula);
        }
    }

    /// The Paula writes of each vertical blank the last [`Game::load_blanks`] ran: the notes the
    /// music started while the original loaded. A mixer runs them a frame each before the step
    /// after them, or its channels come out of the load where they went in (T30: the paced app's
    /// stayed on their silent loops). In the story intro the step's writes before its load come
    /// first, the frame the original's load began in, and the last blank is in the step's own.
    pub fn take_load_paula(&mut self) -> Vec<Vec<PaulaWrite>> {
        std::mem::take(&mut self.load_paula)
    }

    /// Draw [`Game::step`]'s pictures for `view` from the next step on (widescreen, DESIGN §2.7).
    /// The game itself is untouched: every mode steps the same state.
    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// The view [`Game::step`] draws for.
    pub fn view(&self) -> View {
        self.view
    }

    /// The frame the original shows for this state (DESIGN §3.3), widened by `view`.
    pub fn render(&self, view: View) -> Picture {
        match (self.scene, &self.shown) {
            (Scene::Front, Some(p)) if self.cop1lc() == Some(crate::render::LEVEL_COPPER) => {
                crate::render::level_widened(p, &self.state, &self.display, view)
            }
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

    /// For conformance's engine run alone: how many frames the original held the bonus level's
    /// loading picture up while it loaded, the load's length (DESIGN §2.1 check 4; owner, T209).
    pub fn set_screen_frames(&mut self, frames: Option<u16>) {
        if let Some(f) = self.front.as_mut() {
            f.screen_frames = frames;
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

    /// Paths of the original the engine has run into that are not ported yet: the front end's,
    /// and any a level met (an object template with no kind among them, T204). Empty in every
    /// conformance case; a test fails on any.
    pub fn unported(&self) -> &[&'static str] {
        &self.unported
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

    /// The word of chip RAM at `addr` as the audio channels' DMA reads it: the tunes and effect
    /// banks loaded, and what the sound driver wrote there; in the story intro, its own memory
    /// (T29: the app's mixer).
    pub fn chip_word(&self, addr: u32) -> u16 {
        match &self.story {
            Some(s) => {
                let b = s.mem.read(addr, 2);
                u16::from_be_bytes([b[0], b[1]])
            }
            None => self.state.audio.mem.w(self.display.sound(), addr),
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

    /// Where the game is (T21, DESIGN §2.9): the level a game last played (`world`, `level`,
    /// h0+$40da, $40de, read while a level runs; the bonus level is world 7, the final level
    /// counts as W6-4, whose code reaches it), for the app to remember the furthest of. World 0
    /// until a game has played a level: the attract demo is no game, and the executable's own
    /// `world` and `level` read W1-1 from power-on.
    pub fn progress(&self) -> Progress {
        self.progress
    }

    /// A game started at `progress` as the original's level code for it starts it (T21, DESIGN
    /// §2.9: the app's "Continue from World N"): power-on, then the code typed as a player types
    /// it ([`Game::type_code`]), its digits the player's disks' own
    /// ([`crate::front::password::password_for`]). The game is handed over on the frame after
    /// START GAME is taken: the title fades out, then `game_start_at` checks the code and the
    /// world loads. With the seed [`Rng::default`]; see [`Game::start_at_with`]. A progress no
    /// code starts (world 0) starts at W1-1, as START GAME with no code does.
    pub fn start_at(data: &GameData, progress: &Progress) -> Game {
        Game::start_at_with(Arc::new(DisplayData::new(data)), progress, Rng::default())
    }

    /// [`Game::start_at`] with the disks' display content already built and the RNG's seed.
    pub fn start_at_with(display: Arc<DisplayData>, progress: &Progress, seed: Rng) -> Game {
        use crate::front::password::{password_for, table};
        let code = password_for(&table(&display.front), progress);
        let mut g = Game::with_display(display, Start::PowerOn { seed });
        g.type_code(code.map(|p| p.0), &CODE_TIMES);
        g
    }

    /// From the title: the menu's OPTIONS, down to LEVEL CODE, `code`'s digits on the number keys
    /// (raw codes 01..0A), fire to leave the options, then the menu again and START GAME; with no
    /// code, START GAME straight away. The menu is driven by what it shows ([`Game::title_waits`]);
    /// the options screen, which shows nothing the engine reads back, by `times`, frames from
    /// OPTIONS taken. Returns on the frame after START GAME is taken.
    pub fn type_code(&mut self, code: Option<[u8; 6]>, times: &CodeTimes) {
        self.type_code_with(code, (POWER_ON_OPTIONS, POWER_ON_OPTIONS), times);
    }

    /// [`Game::type_code`], the options screen's LIVES and GAME MODE moved on the way to LEVEL CODE
    /// from what the options screen shows now (`from`, power-on's 0 and 1 for a game just switched
    /// on) to `to` (`opt_lives`, `opt_game_mode`): from the cursor on LIVES, right or left once per
    /// step, down, the same for the mode, down twice. Each press is held 10 frames, the next 40
    /// after it, as the first down was; bug-recorder T00 measured this on the original.
    pub fn type_code_with(&mut self, code: Option<[u8; 6]>, (from, to): ((u8, u8), (u8, u8)), times: &CodeTimes) {
        use crate::input::Joy;
        let joy = |joy: Joy| FrameInput { joy, ..FrameInput::NONE };
        let wait = |g: &mut Game| {
            for _ in 0..START_AT_FRAMES {
                if g.title_waits() {
                    return;
                }
                g.step(FrameInput::NONE);
            }
        };
        if let Some(code) = code {
            wait(self);
            self.step(joy(Joy::FIRE));
            wait(self);
            let mut script = vec![
                (0, joy(Joy::DOWN)),
                (5, FrameInput::NONE),
                (times.options, joy(Joy::FIRE)),
                (times.options + 5, FrameInput::NONE),
            ];
            // The presses from LIVES to LEVEL CODE, `times.down` apart.
            let mut presses = Vec::new();
            let change = |presses: &mut Vec<Joy>, from: u8, to: u8| {
                let dir = if to > from { Joy::RIGHT } else { Joy::LEFT };
                presses.extend((0..from.abs_diff(to)).map(|_| dir));
            };
            change(&mut presses, from.0, to.0);
            presses.push(Joy::DOWN);
            change(&mut presses, from.1, to.1);
            presses.extend([Joy::DOWN, Joy::DOWN]);
            let mut t = times.options + times.first_down;
            for (k, j) in presses.into_iter().enumerate() {
                if k > 0 {
                    t += times.down;
                }
                script.push((t, joy(j)));
                script.push((t + 10, FrameInput::NONE));
            }
            t += times.first_digit;
            for d in code {
                let raw = if d == b'0' { 0x0a } else { d.wrapping_sub(b'0') };
                let keys = crate::input::KeySet::key(raw);
                script.push((t, FrameInput { keys, ..FrameInput::NONE }));
                script.push((t + 6, FrameInput::NONE));
                t += times.digit;
            }
            t += times.leave;
            script.push((t, joy(Joy::FIRE)));
            script.push((t + 5, FrameInput::NONE));
            let mut held = FrameInput::NONE;
            for f in 0..t + 10 {
                if let Some(&(_, i)) = script.iter().find(|(at, _)| *at == f) {
                    held = i;
                }
                self.step(held);
            }
        }
        // The menu (after the options, the title's intro again), START GAME.
        wait(self);
        self.step(joy(Joy::FIRE));
        wait(self);
        self.step(joy(Joy::FIRE));
    }

    /// The level `setup.level` at its first frame with the setup's values (bug-recorder DESIGN
    /// §2.1, D2 as T00 measured it): switched on with `seed`, the options screen's LIVES and GAME
    /// MODE set and the level's code typed as a player does it ([`Game::type_code_with`]), the game
    /// run unseen until the level's first frame (the final level: W6-4's code, its exit forced on
    /// its first frame as `sf-conform`'s sync does, the tally and the final level's load run
    /// through), then `score`, `frog_next_life`, `frog_hearts`, `frog_spud_on` and `frog_flight`
    /// written. At frame 0 every gameplay variable equals the original's started the same way
    /// (T00). Callers validate the setup first; an invalid one panics in debug builds.
    pub fn start_level(display: Arc<DisplayData>, setup: &LevelSetup, seed: Rng) -> Game {
        use crate::front::password::{password_for, table};
        use sf_assets::World;
        if cfg!(debug_assertions)
            && let Err(why) = setup.validate()
        {
            panic!("start_level: {why}");
        }
        let id = setup.level;
        let progress = match id.world {
            World::Bonus => Progress { world: 7, level: 1 },
            World::Final => Progress { world: 6, level: 4 },
            w => Progress {
                world: World::ALL.iter().position(|&x| x == w).expect("a world") as u8 + 1,
                level: id.level,
            },
        };
        let code = password_for(&table(&display.front), &progress)
            .unwrap_or_else(|| panic!("start_level: the disks have no level code for {id}"));
        let mut g = Game::with_display(display, Start::PowerOn { seed });
        let options = (setup.lives.opt_lives(), setup.difficulty.opt_game_mode());
        g.type_code_with(Some(code.0), (POWER_ON_OPTIONS, options), &CODE_TIMES);
        let mut forced = false;
        for _ in 0..START_LEVEL_FRAMES {
            if g.scene.is_level() {
                let current = crate::level::load::current_of(&g.state);
                if current == Some(id) {
                    break;
                }
                // W6-4, on the way to the final level: its exit, as the exit sets it.
                if id.world == World::Final && !forced {
                    g.poke(LEVEL_STATE, &1u32.to_be_bytes());
                    forced = true;
                }
            }
            g.step_unseen(FrameInput::NONE);
        }
        assert!(
            g.scene.is_level() && crate::level::load::current_of(&g.state) == Some(id),
            "start_level: {id} not reached in {START_LEVEL_FRAMES} frames"
        );
        g.poke(h0(0x834), &setup.points.to_be_bytes());
        g.poke(h0(0x880), &setup.next_life().to_be_bytes());
        g.poke(h0(0x830), &(setup.energy as u16).to_be_bytes());
        g.poke(h0(0x852), &(setup.spud as u16).to_be_bytes());
        g.poke(h0(0x866), &(setup.wings as u16).to_be_bytes());
        g.place = g.place_now();
        g
    }

    /// Where play is (bug-recorder DESIGN §2.6): in a level, its id and the frog's cell as the
    /// owner addresses tiles (`((frog_x + 0x17) / 16, (frog_y - 0x10) / 16)`, the frog's lower
    /// half, T00 Q3); otherwise the screen's name ([`SceneKind::screen_name`]). A frame on no named
    /// screen (a fade's blank list, a list no name covers) is where the last named one was; a game
    /// that has shown none yet is at the title.
    pub fn location(&self) -> Location {
        self.place_now()
            .or(self.place)
            .unwrap_or(Location::Screen("title"))
    }

    /// [`Game::location`] as this frame alone says it.
    fn place_now(&self) -> Option<Location> {
        let s = &self.state;
        let kind = SceneKind::classify(
            self.scene == Scene::Story,
            self.cop1lc(),
            s.l(h0(0x18)) != 0,
            s.w(h0(0x82e)),
            s.l(h0(0x40da)) as u16,
            s.l(h0(0x40de)) as u16,
        )?;
        match kind {
            SceneKind::Level { .. } | SceneKind::BonusLevel { .. } => {
                let id = crate::level::load::current_of(s)?;
                let (x, y) = (s.w(h0(0x7d6)), s.w(h0(0x7d8)));
                Some(Location::Level {
                    id,
                    col: x.wrapping_add(0x17) / 16,
                    row: y.saturating_sub(0x10) / 16,
                })
            }
            k => k.screen_name().map(Location::Screen),
        }
    }

    /// The title waits in its idle, or its menu for a choice: fire in the next frame opens the
    /// menu, or takes the item lit (START GAME unless moved). For [`Game::start_at`], and tests
    /// that drive the title by what it shows rather than by frame.
    pub fn title_waits(&self) -> bool {
        self.front.as_ref().is_some_and(|f| f.title_waits())
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
            self.level_unported = crate::level::load::enter_with(
                &mut self.state,
                &self.display,
                id,
                crate::level::frog_only_dispatch(),
            );
            self.gather_unported();
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

    /// After a save state is read: the disks' display content, which is not in it, with the
    /// state's own `.sflevel` maps put back in place of the disks'.
    pub(crate) fn restore_display(&mut self, display: Arc<DisplayData>) {
        self.display = if self.overrides.is_empty() {
            display
        } else {
            let mut d = (*display).clone();
            for (id, map) in &self.overrides {
                d.levels.insert(*id, map.clone());
            }
            Arc::new(d)
        };
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// Chip RAM as the audio channels' DMA reads it ([`Game::chip_word`]).
impl sf_paula::ChipMemory for Game {
    fn chip_word(&self, addr: u32) -> u16 {
        Game::chip_word(self, addr)
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
