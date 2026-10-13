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
    /// The original's main loop was still in its pass at this frame's vertical blank. The engine
    /// then runs the interrupt's work at one fixed point in the late pass. In the story intro
    /// (owner, 2026-10-12): the original was still in a long job (a draw, an unpack, a scene's
    /// set-up), and the engine's job in progress ends no sooner; [`Pacing::OnTime`] there means it
    /// had ended, and the engine's ends with the frame.
    Late,
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
                    Some(f) => (state, Scene::Front, None, Some(f)),
                    None => {
                        (state.audio, sdr) = level_io(&display, &memory);
                        (state, Scene::of(&memory), None, None)
                    }
                }
            }
            Start::Level(id) => {
                let mut state = State::default();
                crate::level::load::enter(&mut state, &display, id);
                (state, Scene::Level, None, None)
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
        }
    }

    /// One PAL frame of normal play: the main loop is never late.
    pub fn step(&mut self, input: FrameInput) -> FrameOut {
        self.step_paced(input, Pacing::Unpaced)
    }

    /// One PAL frame on the original's schedule (conformance runs; see [`Pacing`]).
    pub fn step_paced(&mut self, input: FrameInput, pacing: Pacing) -> FrameOut {
        // A key going down or up this frame is the byte the keyboard sends (the last one wins).
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
                self.shown = Some(out.picture.clone());
                return FrameOut {
                    picture: out.picture,
                    paula: out.paula,
                    load: out.load,
                    resync: out.resync,
                    jobs: out.jobs,
                };
            }
            // The intro has handed over: the game starts as a disk-1 power-on does, and its first
            // frame stands for this one (the original loads the game meanwhile).
            let seed = self.seed.take().unwrap_or_default();
            let mut front = crate::front::Front::power_on(seed);
            self.state = State::default();
            front.boot(&mut self.state, &self.display);
            self.front = Some(front);
            self.story = None;
            self.scene = Scene::Front;
            self.seed = Some(seed);
        }
        if let (Scene::Front, Some(front)) = (self.scene, self.front.as_mut()) {
            self.seed = None;
            let out = front.step(&mut self.state, &self.display, input);
            self.shown = Some(out.picture.clone());
            return FrameOut {
                picture: out.picture,
                paula: out.paula,
                load: out.load,
                resync: out.resync,
                jobs: out.jobs,
            };
        }
        let mut hw = Hw::of(input);
        hw.sdr = self.sdr;
        let (paula, shown) = {
            let mut m = Machine {
                state: &mut self.state,
                display: &self.display,
                hw,
                dispatch: if self.frog_only {
                    crate::level::frog_only_dispatch()
                } else {
                    crate::level::standard_dispatch()
                },
            };
            self.scene.step(&mut m, pacing, View::default())
        };
        FrameOut {
            picture: shown.unwrap_or_else(|| self.render(View::default())),
            paula,
            load: None,
            resync: Vec::new(),
            jobs: Vec::new(),
        }
    }

    /// The frame the original shows for this state (DESIGN §3.3), widened by `view`.
    pub fn render(&self, view: View) -> Picture {
        match (self.scene, &self.shown) {
            (Scene::Front | Scene::Story, Some(p)) => p.widened(view),
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

    /// For conformance, the story intro: the memory that runs on by itself while the original
    /// loads ([`Game::runs_during_load`]'s runs, as the original had them just before the frame
    /// its load ended on), put in place at the moment the engine's instant load happens in its
    /// next step, after what came before the load in that step.
    pub fn set_load_state(&mut self, runs: Vec<(u32, Vec<u8>)>) {
        if let Some(s) = self.story.as_mut() {
            s.load_state = runs;
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

    /// The seed a power-on start has not yet put in place.
    pub fn pending_seed(&self) -> Option<Rng> {
        self.seed
    }

    /// Run levels with no object kinds: the frog alone, for conformance against an original whose
    /// objects are all frozen out (T14's frog cases).
    pub fn frog_only(&mut self) {
        self.frog_only = true;
    }

    /// Frames stepped since the start.
    pub fn frame(&self) -> u32 {
        self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
