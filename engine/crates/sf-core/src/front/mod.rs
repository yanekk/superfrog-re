//! The front end (T20, docs/13-front-end.md): everything around the levels, as the original's own
//! routines run it: boot, the title and its intro, the menu, options and level codes, the attract
//! screens, game over and the end sequence.
//!
//! The original paces these screens on the beam: its main program waits for a raster line
//! (`wait_frame_raster`: line `$12b`, below the picture), for a fade to finish, or for a given
//! line, and the vertical-blank interrupt runs sound, fades and the keyboard at the top of every
//! frame. The engine keeps that schedule: each routine is a resumable state machine ([`Routine`])
//! that runs until it waits ([`Yield`]), and a frame runs the interrupt, then each part of the main
//! program in the order of the lines it wakes on, drawing the picture as the beam would see the
//! memory (writes to the copper list after the copper read a word show from the next frame).
//!
//! State is the original's memory, kept in its own form ([`State`]'s modelled runs), read and
//! written at the original's addresses, so a transplanted state continues and a probe compares
//! byte for byte. Loading from disk is instant (owner, 2026-10-12): a step that loads reports the
//! return address the original reaches once its load is over ([`Front::load`]), so conformance can
//! skip the original's frames spent loading.

pub mod anim;
pub(crate) mod attract;
pub mod blit;
pub(crate) mod chain;
pub(crate) mod common;
pub mod data;
pub(crate) mod ending;
pub mod fade;
mod gameover;
mod hero;
mod hiscore;
mod menu;
pub mod password;
pub mod sprite;
pub mod story;
pub(crate) mod title;
pub(crate) mod vbl;

pub use data::FrontData;
pub(crate) use gameover::{GameOverCheck, GameOverSeq};
pub use gameover::GAME_OVER;
pub use story::{Story, StoryData};

use sf_paula::PaulaWrite;

use crate::input::FrameInput;
use crate::layout::{H2, h0};
use crate::render::{self, DisplayData, Picture};
use crate::rng::Rng;
use crate::state::State;

pub(crate) const fn h2(off: u32) -> u32 {
    H2 + off
}

/// Main-program work from this line on shows next frame (the picture ends on line `$12b`; work
/// that wakes on it starts after the beam has drawn it).
pub(crate) const PICTURE_END: u16 = 0x12b;
/// Where the interrupt's work is over and a busy-waiting main program goes on (measured).
pub(crate) const VBL_END_LINE: u16 = 0x14;
/// The vertical blank's fades (T37, fitted to `frog-pit-w1-1`'s fade out and in): the line they
/// start on from where the handler starts, the cycles a colour channel looked at and changed
/// takes in a fade in and out, and the lines after the last to the handler's end.
const FADE_FROM: u16 = 16;
const FADE_IN_LOOK: u32 = 95;
const FADE_IN_CHANGE: u32 = 96;
const FADE_OUT_LOOK: u32 = 71;
const FADE_OUT_CHANGE: u32 = 119;
const FADE_TAIL: u16 = 2;
/// The line of the next frame on which the CPU, held off by a blit with priority, takes the
/// interrupt (measured on the options screen's exit: the interrupt's acknowledge on line 41).
pub(crate) const STALL_VBL_LINE: u16 = 30;
/// Lines in a PAL frame.
pub const FRAME_LINES: u16 = 313;
/// CPU cycles in one raster line (227 DMA cycles of two).
pub(crate) const BEAM_CYCLES_PER_LINE: u32 = 454;
/// `wait_frame_raster` returns on this line (`$12b`) after passing `$ff`.
const RASTER_LINE: u16 = 0x12b;

/// What a routine waits for when it gives the CPU back.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Yield {
    /// `wait_frame_raster` (h0+$73d0): line `$ff`, then line `$12b`.
    Raster,
    /// A busy wait for the beam to reach line `n`.
    Line(u16),
    /// `$1519e`: until the fade record's done flag is set (by the vertical blank).
    Fade(fade::Rec),
    /// A busy loop with work in it: run again after the next vertical blank.
    Poll,
    /// Run a subroutine first.
    Call(Box<Routine>),
    /// The CPU went on past the end of the frame with interrupts off: it goes on in the next
    /// frame up to this line before the vertical blank's interrupt is taken.
    Masked(u16),
    /// Interrupts back on: an interrupt that waited is taken now, then this goes on.
    Interrupt,
    /// The CPU is busy (interrupts on) up to this line counted from this frame's start, which
    /// can be frames away: unpacking a picture.
    Busy(u32),
    /// This routine is over.
    Return,
    /// A `jmp` out of the call stack: the program goes on with this routine alone (game over's
    /// `jmp top_loop`).
    Jump(Box<Routine>),
}

/// The front end's routines, each a state machine over the original's code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Routine {
    Title(title::Title),
    Overlay(title::OverlayEnd),
    /// `access_overlay_show` on the beam, before the final level (T208).
    OverlayShow(title::OverlayShow),
    IntroAnim(title::IntroAnim),
    FadeIn(title::TitleFadeIn),
    Password(password::Entry),
    MusicFadeWait(common::MusicFadeWait),
    MusicStart(common::MusicStart),
    FadeSetup(common::FadeSetup),
    PaletteWait(common::PaletteWait),
    FireUp(common::FireUp),
    Frames(common::Frames),
    Menu(menu::Menu),
    Options(menu::Options),
    TitleExit(title::TitleExit),
    TopLoop(attract::TopLoop),
    AwardPages(attract::AwardPages),
    HiscoreCredits(attract::HiscoreCredits),
    HallOfTalent(attract::HallOfTalent),
    /// `hiscore_name_entry` after a game with a new high score (T200).
    NameEntry(hiscore::NameEntry),
    /// The attract demo's load and set-up (T33).
    Demo(attract::Demo),
    EndSeqLoad(ending::EndSeqLoad),
    EndSequence(ending::EndSequence),
    CpuWork(ending::CpuWork),
    /// After a level's loop: `level_exit_fade` and `game_over_check` (T37, `crate::level::exit`).
    LevelOver(crate::level::exit::LevelOver),
    /// `level_init_wN` and `level_start` again after a death (T37).
    Restart(crate::level::exit::Restart),
    LevelStart(crate::level::exit::LevelStart),
    /// `level_tally` after the exit (T38).
    Tally(crate::level::tally::LevelTally),
    /// Level work done at once, replayed on the beam (T37).
    Timeline(crate::level::timeline::Timeline),
    /// `level_frame_loop` is entered again: the game goes back to the level scene.
    LevelLoop,
    /// The level is over this way; what follows is not ported yet.
    Outcome(crate::level::LevelOutcome),
    /// `game_over_seq` and `game_over_check`'s way out of the game (T34).
    GameOverSeq(gameover::GameOverSeq),
    GameOverCheck(gameover::GameOverCheck),
    /// The slot machine (T92) and, below it, the level's tally that called it.
    Slot(crate::slot::Routine),
    SlotOver(crate::slot::SlotOver),
    /// The slot machine's attract frame drawn on the beam (T93), and its lamps or reels (T73).
    SlotDraw(crate::slot::draw::Draw),
    /// The slot machine's rounds and their parts (T73).
    SlotRounds(crate::slot::rounds::Rounds),
    SlotHold(crate::slot::rounds::HoldMenu),
    SlotMenu(crate::slot::rounds::Menu),
    SlotSpin(crate::slot::rounds::Spin),
    SlotFlash(crate::slot::rounds::Flash),
    SlotAward(crate::slot::rounds::Award),
    SlotCount(crate::slot::rounds::Count),
    /// The hi-lo game's number turning, and the jackpot (T74).
    SlotHiloSpin(crate::slot::rounds::HiloSpin),
    SlotJackpot(crate::slot::rounds::Jackpot),
    SlotWork(crate::slot::draw::Work),
    /// The game's flow (T183, [`chain`]): `top_loop`'s calls, the world's and the level's loads
    /// and the screens between levels.
    Chain(chain::Chain),
    WorldLoad(chain::WorldLoad),
    WorldIntro(chain::WorldIntro),
    LevelLoad(chain::LevelLoad),
    MapShow(chain::MapShow),
    MapEnd(chain::MapEnd),
    IntroEnd(chain::IntroEnd),
    EndPic(chain::EndPic),
    EndPicEnd(chain::EndPicEnd),
}

impl Routine {
    fn run(&mut self, cx: &mut Cx) -> Yield {
        match self {
            Routine::Title(r) => r.run(cx),
            Routine::Overlay(r) => r.run(cx),
            Routine::OverlayShow(r) => r.run(cx),
            Routine::IntroAnim(r) => r.run(cx),
            Routine::FadeIn(r) => r.run(cx),
            Routine::Password(r) => r.run(cx),
            Routine::MusicFadeWait(r) => r.run(cx),
            Routine::MusicStart(r) => r.run(cx),
            Routine::FadeSetup(r) => r.run(cx),
            Routine::PaletteWait(r) => r.run(cx),
            Routine::FireUp(r) => r.run(cx),
            Routine::Frames(r) => r.run(cx),
            Routine::Menu(r) => r.run(cx),
            Routine::Options(r) => r.run(cx),
            Routine::TitleExit(r) => r.run(cx),
            Routine::TopLoop(r) => r.run(cx),
            Routine::AwardPages(r) => r.run(cx),
            Routine::HiscoreCredits(r) => r.run(cx),
            Routine::HallOfTalent(r) => r.run(cx),
            Routine::NameEntry(r) => r.run(cx),
            Routine::Demo(r) => r.run(cx),
            Routine::EndSeqLoad(r) => r.run(cx),
            Routine::EndSequence(r) => r.run(cx),
            Routine::CpuWork(r) => r.run(cx),
            Routine::LevelOver(r) => r.run(cx),
            Routine::Restart(r) => r.run(cx),
            Routine::LevelStart(r) => r.run(cx),
            Routine::Tally(r) => r.run(cx),
            Routine::Timeline(r) => r.run(cx),
            Routine::LevelLoop => Yield::Poll,
            // The level is over this way: said, then back to the caller (`top_loop`'s next call).
            Routine::Outcome(o) => {
                cx.outcome = Some(*o);
                Yield::Return
            }
            Routine::GameOverSeq(r) => r.run(cx),
            Routine::GameOverCheck(r) => r.run(cx),
            Routine::Slot(r) => r.run(cx),
            Routine::SlotOver(r) => r.run(cx),
            Routine::SlotDraw(r) => r.run(cx),
            Routine::SlotRounds(r) => r.run(cx),
            Routine::SlotHold(r) => r.run(cx),
            Routine::SlotMenu(r) => r.run(cx),
            Routine::SlotSpin(r) => r.run(cx),
            Routine::SlotFlash(r) => r.run(cx),
            Routine::SlotAward(r) => r.run(cx),
            Routine::SlotCount(r) => r.run(cx),
            Routine::SlotHiloSpin(r) => r.run(cx),
            Routine::SlotJackpot(r) => r.run(cx),
            Routine::SlotWork(r) => r.run(cx),
            Routine::Chain(r) => r.run(cx),
            Routine::WorldLoad(r) => r.run(cx),
            Routine::WorldIntro(r) => r.run(cx),
            Routine::LevelLoad(r) => r.run(cx),
            Routine::MapShow(r) => r.run(cx),
            Routine::MapEnd(r) => r.run(cx),
            Routine::IntroEnd(r) => r.run(cx),
            Routine::EndPic(r) => r.run(cx),
            Routine::EndPicEnd(r) => r.run(cx),
        }
    }
}

/// The keyboard as the original's `kbd_poll` sees it: CIA-A's serial data register, which holds
/// the last byte the keyboard sent (`!(code << 1 | released)`), and whether that byte is new
/// (the serial-port flag). Once `kbd_poll` has handled a new byte it writes `$ff` there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Keyboard {
    pub sdr: u8,
    pub new: bool,
    held: crate::input::KeySet,
}

impl Default for Keyboard {
    /// At power-on the register holds what the keyboard's start-up ends with: `$7e` released
    /// (measured), never acknowledged.
    fn default() -> Keyboard {
        Keyboard {
            sdr: !(0x7e << 1 | 1),
            new: false,
            held: crate::input::KeySet::NONE,
        }
    }
}

impl Keyboard {
    /// The register as `kbd_poll` last left `key_down` and `key_up` (h0+$16078): the last byte
    /// it handled, or `$ff` once it has (both read 0).
    fn from_keys(key_down: u8, key_up: u8) -> Keyboard {
        let sdr = match (key_down, key_up) {
            (_, up) if up != 0 => !(up << 1 | 1),
            (down, _) if down != 0 => !(down << 1),
            _ => 0xff,
        };
        Keyboard {
            sdr,
            ..Keyboard::default()
        }
    }

    /// The keyboard sends a byte for each key pressed or released since the last frame.
    fn feed(&mut self, keys: crate::input::KeySet) {
        let changed = self.held.0 ^ keys.0;
        for code in 0..128u8 {
            if changed >> code & 1 == 1 {
                let up = !keys.contains(code) as u8;
                self.sdr = !(code << 1 | up);
                self.new = true;
            }
        }
        self.held = keys;
    }
}

/// What a routine sees while it runs.
pub(crate) struct Cx<'a> {
    pub s: &'a mut State,
    pub data: &'a FrontData,
    pub display: &'a DisplayData,
    pub input: FrameInput,
    pub paula: &'a mut Vec<PaulaWrite>,
    pub keyboard: Keyboard,
    /// The line the beam is on while this code runs.
    pub line: u16,
    /// COP1LC: the copper list the next frame starts.
    pub cop1lc: u32,
    /// Where the original is once a load this frame is over.
    pub load: Option<crate::game::LoadEnd>,
    /// The line the runner says the original's load ended on (conformance only).
    pub load_line: Option<u16>,
    /// At a load: the copper list then, and the line the load ends on with the list's bytes.
    pub load_point: Option<(u32, (u16, Snapshot))>,
    /// Memory a load this frame wrote that the engine does not model (a file kept packed),
    /// taken from the original when its load is over.
    pub resync: Vec<(u32, u32)>,
    /// CPU jobs done at once that the original may run past the frame's end.
    pub jobs: Vec<crate::game::Job>,
    /// What a routine hands back to its caller (the menu's choice).
    pub ret: u32,
    /// The line a long job began on (the memory it wrote before it shows from there).
    pub busy_from: Option<u16>,
    /// The line from which a part of the main program shows in this frame's picture, when it is not
    /// where the part ends (a blit the beam overtakes: [`Cx::reveal`]).
    pub reveal: Option<u16>,
    /// With `reveal`: what the beam saw of the part's memory, when not simply before or after.
    pub reveal_state: Option<State>,
    /// The display registers this frame (as the copper left them last frame).
    pub fetch: render::copper::Fetch,
    /// A blit holding the CPU off the bus ran past the frame's end: the next frame's interrupt,
    /// and the end of this frame for conformance, come on this line of the next frame.
    pub stall: Option<u16>,
    /// An unpacking begun this frame (see [`Unpack`]).
    pub unpack: Option<Unpack>,
    /// Frames since the front end started (for jobs that span frames).
    pub frame: u32,
    pub seed: Option<Rng>,
    pub unported: Vec<&'static str>,
    /// [`crate::game::FrameOut::beam_races`]: lines a drawing step wrote near the beam.
    pub races: Vec<(u16, u16)>,
    /// `slot_machine` returned this frame, giving this (T73).
    pub slot_outcome: Option<crate::slot::SlotOutcome>,
    /// The audio channels' DMA as this frame began, which the slot machine's lamps drawn after the
    /// picture run beside (T129, [`crate::slot`]'s `draw`).
    pub audio: Option<&'a sf_paula::AudioDma>,
    /// For conformance: where the original made this frame's Paula writes, in order
    /// ([`crate::Game::anchor_writes`]); empty in normal play.
    pub schedule: &'a [(u16, u16)],
    /// The copper's writes to Paula's registers this frame, first among the frame's writes.
    pub early: Vec<PaulaWrite>,
    /// How the level is over, once the tally (or the exit's check) has said (T183).
    pub outcome: Option<crate::level::LevelOutcome>,
    /// For conformance, a frame that loads: what runs on during the original's load as this
    /// frame's interrupt left it, put back at the load (the main program's work before the load
    /// was done in the frame the original's load began).
    pub load_runs: Vec<(u32, Vec<u8>)>,
    /// A load this frame was given the original's state at its end (conformance only): the
    /// original's load is over, however long the engine models it as taking.
    pub load_given: bool,
    /// A disk the game asked for this frame that was not in the drive (T21).
    pub disk_prompt: Option<crate::game::DiskPrompt>,
    /// The CPU cycles into [`Cx::line`] the beam is, where that is known to the cycle (T203: a
    /// level's loop ending on the beam, [`Front::finish_frame`]); `None` counts whole lines.
    pub into: Option<u32>,
}

impl Cx<'_> {
    /// The fire button of the joystick in port 2 (CIA-A PRA bit 7, low when pressed).
    pub fn fire(&self) -> bool {
        self.input.joy.contains(crate::input::Joy::FIRE)
    }

    /// The left mouse button in port 1 (CIA-A PRA bit 6).
    pub fn mouse_button(&self) -> bool {
        self.input.joy.contains(crate::input::Joy::PORT1_BUTTON)
    }

    /// JOY1DAT's direction bits as the code tests them (`andi.w #$303`): `$100` up, `$1` down,
    /// `$200` left, `$2` right, as the counters' XOR encodes a held direction.
    pub fn joy1dat(&self) -> u16 {
        use crate::input::Joy;
        let j = self.input.joy;
        let (up, down, left, right) = (
            j.contains(Joy::UP),
            j.contains(Joy::DOWN),
            j.contains(Joy::LEFT),
            j.contains(Joy::RIGHT),
        );
        // Y1 = up xor left... as JOY1DAT encodes it: bit 9 = left, bit 1 = right, bit 8 = up
        // xor left, bit 0 = down xor right.
        let b9 = left as u16;
        let b1 = right as u16;
        let b8 = (up ^ left) as u16;
        let b0 = (down ^ right) as u16;
        b9 << 9 | b8 << 8 | b1 << 1 | b0
    }

    /// A write to a Paula register (DMACON, INTENA...) by the CPU, on the line the beam is on.
    pub fn wr(&mut self, reg: u16, value: u16) {
        self.paula.push(PaulaWrite {
            reg,
            value,
            hpos: 0,
            vpos: self.line.max(1),
        });
    }

    /// A byte of the original's memory: a modelled run's, else what the engine wrote there, else
    /// what the disks put there (hunk 0's code and tables, hunk 2), else 0.
    pub fn b(&self, addr: u32) -> u8 {
        if self.s.raw(addr, 1).is_some() || self.s.scratch.contains_key(&addr) {
            return self.s.b(addr);
        }
        if (crate::layout::H0..crate::layout::H1).contains(&addr) {
            return self.data.h0_b(addr - crate::layout::H0);
        }
        if (crate::layout::H2..crate::layout::H3).contains(&addr) {
            return self.display.h2_bytes(addr - crate::layout::H2, 1)[0];
        }
        self.s.b(addr)
    }

    pub fn w(&self, addr: u32) -> u16 {
        u16::from_be_bytes([self.b(addr), self.b(addr + 1)])
    }

    pub fn l(&self, addr: u32) -> u32 {
        (self.w(addr) as u32) << 16 | self.w(addr + 2) as u32
    }

    /// The CPU starts a job of about `cycles` with interrupts off (`INTENA $4000`): the beam moves
    /// on, and a job that runs past the frame's end returns the [`Yield::Masked`] to finish it with
    /// in the next frame. The job's memory, `writes`, is compared once the original is out of the
    /// call whose return address is `inside` (owner, 2026-10-12).
    pub fn masked_job(
        &mut self,
        cycles: u32,
        inside: u32,
        writes: Vec<(u32, u32)>,
    ) -> Option<Yield> {
        self.wr(crate::audio::INTENA, 0x4000);
        self.jobs.push(crate::game::Job {
            inside: vec![inside],
            writes,
        });
        self.busy(cycles);
        (self.line >= FRAME_LINES).then(|| Yield::Masked(self.line - FRAME_LINES))
    }

    /// `atn_decompress` of `bytes` into `at`, taking `cycles`: the memory holds the result at
    /// once; the picture shows it arriving as the original's decompressor writes it, backwards
    /// from its end at an even pace, over the frames it takes ([`Unpack`]).
    pub fn unpack(&mut self, at: u32, bytes: &[u8], cycles: u32, inside: &[u32]) {
        self.jobs.push(crate::game::Job {
            inside: inside.to_vec(),
            writes: vec![(at, bytes.len() as u32)],
        });
        let old = self.s.read(at, bytes.len() as u32);
        self.s.write(at, bytes);
        let start = self.frame as u64 * FRAME_LINES as u64 + self.line as u64;
        self.busy(cycles);
        let end = start + cycles.div_ceil(BEAM_CYCLES_PER_LINE) as u64;
        self.unpack = Some(Unpack {
            at,
            old,
            start,
            end,
        });
    }

    /// An instant load is over: conformance finds the original past it as `end` says; the main
    /// program goes on from the line the runner gave, if it gave one.
    pub fn loaded(&mut self, end: crate::game::LoadEnd) {
        self.load = Some(end);
        if let Some(l) = self.load_line.take() {
            // The load ended frames after it began, on the runner's line, not past where this
            // frame had got to; a load begun on line 0 counts as begun last frame.
            self.line = l.max(1);
        }
        self.load_given = !self.load_runs.is_empty();
        for (at, bytes) in std::mem::take(&mut self.load_runs) {
            self.s.poke(at, &bytes);
        }
        let snap = Snapshot {
            at: self.cop1lc,
            bytes: self.s.read(self.cop1lc, 0x800),
        };
        self.load_point = Some((self.cop1lc, (self.line, snap)));
    }

    /// The CPU waits for a blit: [`blit::Blit::cycles`], and on each line the display fetches
    /// bitplanes on, the slots the fetch takes from the blitter as well (1.23 CPU cycles a slot,
    /// measured: the title intro's 64-line restore under six planes runs from line 161 to 308).
    pub fn blit_wait(&mut self, b: &blit::Blit) {
        if self.busy_from.is_none() {
            self.busy_from = Some(self.line);
        }
        let per_line = (self.fetch.planes() * self.fetch.words()) as u32 * 123 / 100;
        let mut need = b.cycles();
        while need > 0 && self.line < u16::MAX {
            let shown = self.fetch.lines().contains(&(self.line % FRAME_LINES));
            let free = BEAM_CYCLES_PER_LINE - if shown { per_line } else { 0 };
            need = need.saturating_sub(free);
            self.line += 1;
        }
    }

    /// The CPU is busy for `cycles` 7 MHz cycles (a blit it waits for): the beam moves on.
    pub fn busy(&mut self, cycles: u32) {
        if self.busy_from.is_none() {
            self.busy_from = Some(self.line);
        }
        let lines = match self.into.as_mut() {
            Some(into) => {
                let t = *into + cycles;
                *into = t % BEAM_CYCLES_PER_LINE;
                t / BEAM_CYCLES_PER_LINE
            }
            None => cycles.div_ceil(BEAM_CYCLES_PER_LINE),
        };
        self.line = (self.line as u32 + lines).min(u16::MAX as u32) as u16;
    }

    /// The file of `file_table`'s entry at h0+`entry` is read (`track_read`, h0+$b098): its disk
    /// (the entry's third word, an index into `disk_id_table`, h0+$b11e) is looked for in the
    /// drive, and when the game last asked for another, `disk_request` (h0+$ade4) asks for it
    /// ([`Cx::disk_prompt`]), the player puts it in and the load goes on (T21). The engine's loads
    /// are instant, so the answer is too.
    pub fn read_entry(&mut self, entry: u32) {
        let disk = self.data.h0_w(entry + 4) & 3;
        let id = self.data.h0_l(DISK_ID_TABLE + 4 * disk as u32);
        if self.s.l(at(0xaf54)) != id {
            self.s.set_l(at(0xaf54), id);
            self.disk_prompt = Some(crate::game::DiskPrompt::Insert(sf_disk::DiskSlot::ALL[disk as usize]));
        }
    }

    /// [`Cx::read_entry`] for every `file_table` entry the original's code between h0+`from` and
    /// h0+`to` names (`lea file_table+n,a2`), in order.
    pub fn read_entries(&mut self, from: u32, to: u32) {
        for e in table_entries(self.data, from, to) {
            self.read_entry(e);
        }
    }

    /// A path of the original's code the engine does not follow yet: recorded, so a test fails.
    pub fn unported(&mut self, what: &'static str) {
        self.unported.push(what);
    }
}

/// `file_table` (h0+$cbfe): six bytes a file, its first sector, its sectors and its disk.
pub(crate) const FILE_TABLE: std::ops::Range<u32> = 0xcbfe..0xce58;
/// `disk_id_table` (h0+$b11e, an h0 offset): the four disks' IDs, `2222` to `5555`.
const DISK_ID_TABLE: u32 = 0xb11e;

/// The `file_table` entries (h0 offsets) the code between h0+`from` and h0+`to` loads, in order:
/// each `lea file_table+n,a2` (`$45f9`), which `load_file`'s callers set before the call.
pub(crate) fn table_entries(data: &FrontData, from: u32, to: u32) -> Vec<u32> {
    (from..to)
        .step_by(2)
        .filter(|&pc| data.h0_w(pc) == 0x45f9)
        .map(|pc| data.h0_l(pc + 2).wrapping_sub(crate::layout::H0))
        .filter(|e| FILE_TABLE.contains(e))
        .collect()
}

/// A routine on the call stack and what it waits for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Frame {
    routine: Routine,
    /// What it waits for, and the line it started waiting on.
    wait: Option<(Yield, u16)>,
    /// While the CPU is busy past the frame's end ([`Yield::Busy`]): what the routine gave back
    /// when it ran, to act on once the CPU is free.
    deferred: Option<Yield>,
}

/// The front end's program: the main program's call stack and the hardware state the engine
/// tracks outside memory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Front {
    /// The display registers (window, fetch, bitplanes on) as the last frame's copper left them.
    fetch: Option<render::copper::Fetch>,
    /// For conformance: the line the next load ends on in the original ([`Cx::load_line`]).
    pub(crate) load_line: Option<u16>,
    /// Frames stepped.
    frame: u32,
    /// Paula writes the CPU made after the frame's end, which belong to the next frame.
    carry: Vec<PaulaWrite>,
    /// After a stall: the line this frame's interrupt comes on, and the copper's writes before it
    /// went with the last frame.
    stalled: Option<u16>,
    /// An unpacking the picture shows in progress.
    unpack: Option<Unpack>,
    /// The last value a routine handed back (kept across frames).
    ret: u32,
    /// Power-on: the boot's work is still to do (it runs at the start of the first frame).
    booting: bool,
    stack: Vec<Frame>,
    cop1lc: u32,
    keyboard: Keyboard,
    seed: Option<Rng>,
    /// A copper interrupt requested (INTREQ `$8010`) while it is masked, and not acknowledged: the
    /// next vertical blank's handler takes that branch first (h0+$11d1a) and acknowledges it (T37).
    coper: bool,
    /// After a level run with no object kinds (`crate::Game::frog_only`): the level's vertical
    /// blank, when it runs here (game over), runs so too.
    frog_only: bool,
    /// DMACON's sprite DMA bit as the last frame left it (followed on the slot machine's screen,
    /// [`render::SpriteDma`]).
    spren: bool,
    /// Paths not ported that this front end has run into (empty in every conformance case).
    pub unported: Vec<&'static str>,
    /// What the slot machine gave when it returned (T73).
    slot_outcome: Option<crate::slot::SlotOutcome>,
    /// How the last level was over, as its tally said (T183).
    outcome: Option<crate::level::LevelOutcome>,
    /// The audio channels' DMA, run on each frame's Paula writes: the slots it takes time the slot
    /// machine's lamps (T129). It starts idle; the slot machine stops every channel first.
    audio: sf_paula::AudioDma,
    /// For conformance, the next frame's: where the original made its Paula writes
    /// ([`Front::anchor_writes`]).
    schedule: Vec<(u16, u16)>,
    /// For conformance, a frame that loads: what runs on by itself while the original loads, as
    /// it had it ([`crate::Game::set_load_state`]), put in place after the work carried over.
    pub(crate) load_state: Vec<(u32, Vec<u8>)>,
}

/// An unpacking in progress, for the picture: where, what was there, and when it began and ends
/// (in lines since the front end started).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Unpack {
    at: u32,
    old: Vec<u8>,
    start: u64,
    end: u64,
}

impl Unpack {
    /// Bytes of the unpacked data not yet written by the end of frame `frame`: from the start.
    fn pending(&self, frame: u32) -> usize {
        let t = (frame as u64 + 1) * FRAME_LINES as u64;
        if t >= self.end {
            return 0;
        }
        let done = t.saturating_sub(self.start) as u128 * self.old.len() as u128
            / (self.end - self.start) as u128;
        self.old.len() - done as usize
    }
}

/// A frame's Paula writes, in the order they are compared, where they were made: the original's
/// positions given by conformance ([`Front::anchor_writes`]) when they are as many, else the
/// engine's own lines, at their start (T129).
pub(crate) fn anchored(writes: &[PaulaWrite], schedule: &[(u16, u16)]) -> Vec<PaulaWrite> {
    let mut all = writes.to_vec();
    if schedule.len() >= all.len() {
        for (w, &(vpos, hpos)) in all.iter_mut().zip(schedule) {
            (w.vpos, w.hpos) = (vpos, hpos);
        }
    } else {
        for w in &mut all {
            w.hpos = 0;
        }
    }
    all
}

/// One frame of the front end.
pub struct FrontOut {
    pub picture: Picture,
    pub paula: Vec<PaulaWrite>,
    pub load: Option<crate::game::LoadEnd>,
    pub resync: Vec<(u32, u32)>,
    pub jobs: Vec<crate::game::Job>,
    /// [`crate::game::FrameOut::beam_races`].
    pub beam_races: Vec<(u16, u16)>,
    /// [`crate::game::FrameOut::disk_prompt`].
    pub disk_prompt: Option<crate::game::DiskPrompt>,
}

impl Front {
    /// Switch on: the game program starts (`init_and_script`) with the RNG seed it will take.
    pub fn power_on(seed: Rng) -> Front {
        Front {
            frame: 0,
            fetch: None,
            load_line: None,
            carry: Vec::new(),
            stalled: None,
            unpack: None,
            ret: 0,
            booting: true,
            stack: vec![
                Frame {
                    routine: Routine::TopLoop(attract::TopLoop::after_title()),
                    wait: None,
                    deferred: None,
                },
                Frame {
                    routine: Routine::Title(title::Title::after_boot()),
                    wait: None,
                    deferred: None,
                },
            ],
            cop1lc: h2(0x6fd8),
            keyboard: Keyboard::default(),
            seed: Some(seed),
            coper: false,
            frog_only: false,
            spren: true,
            unported: Vec::new(),
            slot_outcome: None,
            outcome: None,
            audio: sf_paula::AudioDma::booted(),
            schedule: Vec::new(),
            load_state: Vec::new(),
        }
    }

    /// The front end as the original's memory and CPU leave it at the end of a frame, when its main
    /// program is in a routine the engine knows (`None` otherwise).
    pub fn resume(
        s: &mut State,
        display: &DisplayData,
        mem: &crate::layout::AmigaMemory,
        cpu: &crate::game::Cpu,
    ) -> Option<Front> {
        let chain = main_chain(mem, cpu);
        let slot = crate::slot::resume(&chain);
        let in_slot = slot.is_some();
        let stack = slot
            .or_else(|| ending::resume(&chain))
            .or_else(|| title::resume(&chain, cpu, mem))?;
        // The sound driver reads the files the game put in chip RAM: the level's bank and module
        // in the slot machine (it loads its own once resumed), else the title's module.
        use crate::layout::Ram;
        if let Some(d) = display.front.sound.as_ref() {
            let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | mem.byte(a + k) as u32);
            let files = if in_slot {
                crate::audio::files_for(long(h0(0x40da)), long(h0(0x9ec)))
            } else if s.audio.module_addr() == sf_assets::sound::BANK_ADDR {
                vec![(
                    sf_assets::sound::BANK_ADDR,
                    crate::audio::File::Tune(sf_assets::sound::TuneId::Title),
                )]
            } else {
                Vec::new()
            };
            s.audio = crate::audio::AudioDriver::transplant(d, &|a| mem.byte(a), &files);
        }
        let mut front = Front {
            frame: 0,
            fetch: None,
            load_line: None,
            carry: Vec::new(),
            stalled: None,
            unpack: None,
            ret: 0,
            booting: false,
            stack: stack
                .into_iter()
                .map(|(routine, wait)| Frame {
                    routine,
                    wait: wait.map(|w| (w, 0)),
                    deferred: None,
                })
                .collect(),
            cop1lc: title::cop1lc_of(&chain),
            keyboard: Keyboard::from_keys(s.b(vbl::KEY_DOWN), s.b(vbl::KEY_UP)),
            seed: None,
            coper: false,
            frog_only: false,
            spren: true,
            unported: Vec::new(),
            slot_outcome: None,
            outcome: None,
            audio: sf_paula::AudioDma::booted(),
            schedule: Vec::new(),
            load_state: Vec::new(),
        };
        if in_slot {
            front.chain_below(s, &display.front);
        }
        Some(front)
    }

    /// The program a level hands over to when its loop ends (`level_frame_loop` returns into
    /// `top_loop`): `routine` on top, the level's copper list showing. `sdr` is the keyboard's
    /// last byte: one the level's poll has handled reads `$ff` from then on (ESC's is not read
    /// again, measured on `frog-esc-w1-1`), except the power-on byte, never acknowledged; one sent
    /// since that poll is still new.
    pub(crate) fn after_level(s: &State, sdr: u8, routine: Routine, frog_only: bool) -> Front {
        let handled = Keyboard::from_keys(s.b(vbl::KEY_DOWN), s.b(vbl::KEY_UP));
        let keyboard = if sdr != handled.sdr {
            Keyboard {
                sdr,
                new: true,
                ..Keyboard::default()
            }
        } else if sdr == Keyboard::default().sdr {
            handled
        } else {
            Keyboard::from_keys(0, 0)
        };
        Front {
            frame: 0,
            fetch: None,
            load_line: None,
            carry: Vec::new(),
            stalled: None,
            unpack: None,
            ret: 0,
            booting: false,
            stack: vec![Frame {
                routine,
                wait: None,
                deferred: None,
            }],
            cop1lc: render::LEVEL_COPPER,
            keyboard,
            seed: None,
            coper: false,
            frog_only,
            spren: true,
            unported: Vec::new(),
            slot_outcome: None,
            outcome: None,
            audio: sf_paula::AudioDma::booted(),
            schedule: Vec::new(),
            load_state: Vec::new(),
        }
    }

    /// [`Front::after_level`] with what follows the level's loop: `top_loop`'s own code from its
    /// `jsr level_exit_fade` when the level is one of its (T183), else the exit fade alone.
    pub(crate) fn level_over(s: &State, data: &FrontData, sdr: u8, frog_only: bool) -> Front {
        let routine = match chain::level_call(s, data) {
            Some(init) => Routine::Chain(chain::Chain::after_loop(data, init, frog_only)),
            None => Routine::LevelOver(crate::level::exit::LevelOver::new(frog_only)),
        };
        Front::after_level(s, sdr, routine, frog_only)
    }

    /// The slot machine entered from the level's tally (T73, [`crate::Game::enter_slot`]):
    /// `slot_machine` on top of the tally's end, the screen blank.
    pub(crate) fn enter_slot(s: &State, data: &FrontData, sdr: u8, frog_only: bool) -> Front {
        let mut f = Front::after_level(
            s,
            sdr,
            Routine::SlotOver(Default::default()),
            frog_only,
        );
        f.chain_below(s, data);
        f.stack.push(Frame {
            routine: Routine::Slot(Default::default()),
            wait: None,
            deferred: None,
        });
        f.cop1lc = title::BLANK_COPPER;
        f
    }

    /// `top_loop` under the tally this stack ends in, after its `jsr level_tally` (T183).
    fn chain_below(&mut self, s: &State, data: &FrontData) {
        if let Some(init) = chain::level_call(s, data) {
            self.stack.insert(
                0,
                Frame {
                    routine: Routine::Chain(chain::Chain::after_tally(init, self.frog_only)),
                    wait: None,
                    deferred: None,
                },
            );
        }
    }

    /// The rest of a frame the level scene began: the main program from `line` on, after the
    /// loop's last pass. Returns its Paula writes (those past the frame's end go with the next
    /// frame's) and the CPU jobs it began.
    pub(crate) fn finish_frame(
        &mut self,
        s: &mut State,
        display: &DisplayData,
        input: FrameInput,
        line: u16,
        into: u32,
    ) -> (Vec<PaulaWrite>, Vec<crate::game::Job>) {
        // The level's own poll has handled this frame's keyboard byte.
        self.keyboard.held = input.keys;
        let mut paula = Vec::new();
        let mut cx = Cx {
            s,
            data: &display.front,
            display,
            input,
            paula: &mut paula,
            keyboard: self.keyboard,
            line,
            cop1lc: self.cop1lc,
            load: None,
            load_line: None,
            load_point: None,
            resync: Vec::new(),
            jobs: Vec::new(),
            ret: self.ret,
            busy_from: None,
            reveal: None,
            reveal_state: None,
            fetch: self.fetch.unwrap_or_default(),
            stall: None,
            unpack: None,
            frame: self.frame,
            seed: None,
            unported: Vec::new(),
            races: Vec::new(),
            slot_outcome: None,
            audio: None,
            schedule: &[],
            early: Vec::new(),
            outcome: None,
            load_runs: Vec::new(),
            load_given: false,
            disk_prompt: None,
            into: None,
        };
        cx.into = Some(into);
        self.run_main(&mut cx, u16::MAX);
        self.stalled = cx.stall;
        self.ret = cx.ret;
        self.cop1lc = cx.cop1lc;
        self.keyboard = cx.keyboard;
        self.unported.append(&mut cx.unported);
        self.slot_outcome = self.slot_outcome.or(cx.slot_outcome.take());
        self.outcome = cx.outcome.take().or(self.outcome);
        let jobs = std::mem::take(&mut cx.jobs);
        for w in &mut paula {
            if w.vpos == 0 {
                w.vpos = line;
            }
        }
        paula.sort_by_key(|w| w.vpos);
        self.note_coper(&paula);
        let split = paula.partition_point(|w| w.vpos < FRAME_LINES);
        self.carry = paula.split_off(split);
        for w in &mut self.carry {
            w.vpos -= FRAME_LINES;
        }
        self.frame += 1;
        (paula, jobs)
    }

    /// A copper interrupt requested, or acknowledged, by these writes in order.
    fn note_coper(&mut self, writes: &[PaulaWrite]) {
        for w in writes.iter().filter(|w| w.reg == 0x9c && w.value & 0x10 != 0) {
            self.coper = w.value & 0x8000 != 0;
        }
    }

    /// How the level is over, once the program after its loop has decided it (the tally's
    /// result, T38).
    pub(crate) fn outcome(&self) -> Option<crate::level::LevelOutcome> {
        self.outcome
    }

    /// The scene this front end's program is in: the slot machine while `slot_machine` is on its
    /// stack (`coins_in` kept from `was` once in it), else the front end.
    pub(crate) fn scene(&self, s: &State, was: crate::scene::Scene) -> crate::scene::Scene {
        use crate::scene::Scene;
        let in_slot = self
            .stack
            .iter()
            .any(|f| matches!(f.routine, Routine::Slot(_)));
        match (in_slot, was) {
            (true, Scene::Slot { coins_in }) => Scene::Slot { coins_in },
            (true, _) => Scene::Slot {
                coins_in: s.w(crate::slot::COINS),
            },
            (false, _) => Scene::Front,
        }
    }

    /// What the slot machine gave, once it has returned (T73).
    pub(crate) fn slot_outcome(&self) -> Option<crate::slot::SlotOutcome> {
        self.slot_outcome
    }

    /// CIA-A's serial register as the front end's keyboard has it: what the level's own poll reads
    /// once the level scene takes over (T33: the power-on byte, never acknowledged, in the demo).
    pub(crate) fn keyboard_sdr(&self) -> u8 {
        self.keyboard.sdr
    }

    /// Whether that byte is one the front end's poll has not handled yet (the serial-port flag).
    pub(crate) fn keyboard_new(&self) -> bool {
        self.keyboard.new
    }

    /// The title waits in its idle, or its menu for a choice: fire in the next frame opens the
    /// menu, or takes its item lit (START GAME unless moved).
    pub(crate) fn title_waits(&self) -> bool {
        match self.stack.last().map(|f| &f.routine) {
            Some(Routine::Title(t)) => t.idle(),
            Some(Routine::Menu(m)) => m.ready(),
            _ => false,
        }
    }

    /// The level's loop has been entered again (a restart, T37): the level scene goes on.
    pub(crate) fn in_level_loop(&self) -> bool {
        matches!(self.stack.last().map(|f| &f.routine), Some(Routine::LevelLoop))
    }

    /// The boot's work, which is all load: done at once, before the first frame.
    pub fn boot(&mut self, s: &mut State, display: &DisplayData) {
        if std::mem::take(&mut self.booting) {
            let mut quiet = Vec::new();
            let mut cx = Cx {
                s: &mut *s,
                data: &display.front,
                display,
                input: FrameInput::NONE,
                paula: &mut quiet,
                keyboard: self.keyboard,
                line: 0,
                cop1lc: self.cop1lc,
                load: None,
                load_line: None,
                load_point: None,
                resync: Vec::new(),
                jobs: Vec::new(),
                ret: 0,
                busy_from: None,
                reveal: None,
                reveal_state: None,
                fetch: self.fetch.unwrap_or_default(),
                stall: None,
                unpack: None,
                frame: self.frame,
                seed: self.seed.take(),
                unported: Vec::new(),
                races: Vec::new(),
                slot_outcome: None,
                audio: None,
                schedule: &[],
                early: Vec::new(),
                outcome: None,
                load_runs: Vec::new(),
                load_given: false,
                disk_prompt: None,
            into: None,
            };
            title::boot(&mut cx);
            self.cop1lc = cx.cop1lc;
        }
    }

    /// The copper list the frame starts with.
    pub fn cop1lc(&self) -> u32 {
        self.cop1lc
    }

    /// One frame.
    pub fn step(&mut self, s: &mut State, display: &DisplayData, input: FrameInput) -> FrontOut {
        self.keyboard.feed(input.keys);
        self.new_frame();
        let mut paula = Vec::new();
        if self.booting {
            self.boot(s, display);
        }
        let window = |s: &State, c: u32| Snapshot {
            at: c,
            bytes: s.read(c, 0x800),
        };
        // The copper list as each part of the frame leaves it.
        let mut history: Vec<(u16, Snapshot)> = vec![(0, window(s, self.cop1lc))];
        let cop1lc = self.cop1lc;
        let data = &display.front;
        let audio = self.audio.clone();
        let schedule = std::mem::take(&mut self.schedule);
        let mut cx = Cx {
            s,
            data,
            display,
            input,
            paula: &mut paula,
            keyboard: self.keyboard,
            line: 0,
            cop1lc: self.cop1lc,
            load: None,
            load_line: self.load_line.take(),
            load_point: None,
            resync: Vec::new(),
            jobs: Vec::new(),
            ret: self.ret,
            busy_from: None,
            reveal: None,
            reveal_state: None,
            fetch: self.fetch.unwrap_or_default(),
            stall: None,
            unpack: None,
            frame: self.frame,
            seed: self.seed,
            unported: Vec::new(),
            races: Vec::new(),
            slot_outcome: None,
            audio: Some(&audio),
            schedule: &schedule,
            early: Vec::new(),
            outcome: None,
            load_runs: Vec::new(),
            load_given: false,
            disk_prompt: None,
            into: None,
        };
        // Work the main program carried over from the last frame with interrupts off.
        let masked = match self.stack.last() {
            Some(Frame {
                wait: Some((Yield::Masked(l), _)),
                ..
            }) => Some(*l),
            _ => None,
        };
        if let Some(l) = masked {
            // Only the masked part: the interrupt is taken as soon as it turns them back on.
            cx.line = l;
            let top = self.stack.last_mut().expect("a frame");
            top.wait = None;
            match top.routine.run(&mut cx) {
                Yield::Return => {
                    self.stack.pop();
                }
                Yield::Call(r) => self.stack.push(Frame {
                    routine: *r,
                    wait: None,
                    deferred: None,
                }),
                w => top.wait = Some((w, cx.line)),
            }
            for w in cx.paula.iter_mut() {
                if w.vpos == 0 {
                    w.vpos = l;
                }
            }
        }
        let load_runs: Vec<(u32, u32)> = self.load_state.iter().map(|(a, b)| (*a, b.len() as u32)).collect();
        for (at, bytes) in std::mem::take(&mut self.load_state) {
            cx.s.poke(at, &bytes);
        }
        let stalled = self.stalled.take();
        let vbl_line = stalled.unwrap_or(cx.line.max(1));
        cx.paula.append(&mut self.carry);
        let from_carry = cx.paula.len();
        let from = cx.paula.len();
        if std::mem::take(&mut self.coper) {
            // h0+$11d1a: the copper interrupt's branch, acknowledged first.
            cx.wr(0x9c, 0x0010);
        }
        // Only on the level's list (T37) and the slot machine's (T93: its fade in, measured on
        // the oracle's frames 775-791): the title's fades are timed as they were.
        let fading = if cop1lc == render::LEVEL_COPPER || cop1lc == crate::slot::SLOT_COPPER {
            fade::due_channels(cx.s)
        } else {
            Vec::new()
        };
        let level_vbl = cx.s.w(gameover::LOGIC_RUNNING) != 0;
        // On the level's list the fades start after `music_tick` and the line wait after it
        // (T100): what the tick and the handler will do, before they do it.
        let level_list = cop1lc == render::LEVEL_COPPER || cop1lc == title::BLANK_COPPER;
        let fade_from = (level_list && !level_vbl)
            .then(|| {
                let d = cx.data.sound.as_ref()?;
                let work = cx.s.audio.tick_work(d);
                let keyboard = vbl::polls_keyboard(&cx);
                Some(vbl::fade_start(vbl_line, &work, cx.s.audio.streaming(), keyboard))
            })
            .flatten();
        // What the slot screen's handler is about to do that lengthens it (T94).
        let slot_handler = crate::slot::Handler {
            fetches_rows: cx.s.audio.fetches_rows(),
            quiet_tick: cx.s.audio.quiet_tick(),
            effects: cx.s.audio.sfx_active(),
            ..Default::default()
        };
        let palette_due = if level_vbl {
            // The level's own handler (game over runs the level's logic, T34).
            cx.line = gameover::level_vbl(&mut cx, self.frog_only, vbl_line);
            false
        } else {
            vbl::run(&mut cx)
        };
        // The interrupt takes about as long wherever it starts; on the slot screen as long as
        // what it did (T94).
        cx.line = if cop1lc == crate::slot::SLOT_COPPER && !level_vbl {
            let handler = &cx.paula[from..];
            let h = crate::slot::Handler {
                keyboard: handler
                    .iter()
                    .any(|w| w.reg == crate::audio::INTENA && w.value == 0x0008),
                dmacon_writes: handler
                    .iter()
                    .filter(|w| w.reg == crate::audio::DMACON)
                    .count() as u32,
                ..slot_handler
            };
            // `vbl_line` is 1 for a handler taken on time.
            cx.line
                .max(vbl_line.saturating_sub(1) + crate::slot::interrupt_lines(h))
        } else {
            match fade_from {
                // No fade due: the dispatch and the rest of the handler (T100).
                Some(start) if fading.is_empty() => {
                    cx.line.max(((start + vbl::NO_FADE_TAIL) / BEAM_CYCLES_PER_LINE) as u16)
                }
                _ => cx.line.max(VBL_END_LINE).max(vbl_line + VBL_END_LINE - 9),
            }
        };
        let after = window(cx.s, cop1lc);
        if fading.is_empty() {
            history.push((VBL_END_LINE, after));
        } else {
            // A fade steps its colours a channel at a time, the handler longer by what it looks at
            // and what it changes; the copper reads each colour as it is on its line (T37:
            // `level_start`'s fade in, the play field's colours read on line `$34` half stepped).
            let mut partial = history[0].1.clone();
            for &(word, _, _, _) in &fading {
                partial.copy_word(&history[0].1, word);
            }
            history.push((VBL_END_LINE, partial.clone()));
            if let Some(start) = fade_from {
                let top = cx.s.b(render::LEVEL_COPPER + 6) as u16;
                let (writes, end) = fade::level_step_times(&fading, start, top);
                for (&(word, mask, _, _), at) in fading.iter().filter(|c| c.2).zip(writes) {
                    partial.copy_bits(&after, word, mask);
                    let line = (at / BEAM_CYCLES_PER_LINE) as u16;
                    history.push((line.max(VBL_END_LINE + 1), partial.clone()));
                }
                cx.line = cx.line.max((end / BEAM_CYCLES_PER_LINE) as u16);
            } else {
                let mut t = (vbl_line + FADE_FROM) as u32 * BEAM_CYCLES_PER_LINE;
                for &(word, mask, changes, fade_in) in &fading {
                    t += if fade_in { FADE_IN_LOOK } else { FADE_OUT_LOOK };
                    if changes {
                        t += if fade_in { FADE_IN_CHANGE } else { FADE_OUT_CHANGE };
                        partial.copy_bits(&after, word, mask);
                        let line = (t / BEAM_CYCLES_PER_LINE) as u16;
                        history.push((line.max(VBL_END_LINE + 1), partial.clone()));
                    }
                }
                cx.line = cx.line.max((t / BEAM_CYCLES_PER_LINE) as u16 + FADE_TAIL);
            }
        }
        if palette_due {
            cx.line = vbl::PALETTE_LINE;
            vbl::palette_copy(&mut cx);
            history.push((vbl::PALETTE_LINE, window(cx.s, cop1lc)));
        }
        // The interrupt's writes, on the lines it runs: all near the top but its acknowledge,
        // after the palette copy when that ran.
        for w in cx.paula[from.max(from_carry)..].iter_mut() {
            w.vpos = vbl_line;
        }
        // (The level's handler made its acknowledge itself.)
        if !level_vbl {
            let ack = cx.paula.len();
            vbl::end(&mut cx);
            for w in &mut cx.paula[ack..] {
                w.vpos = cx.line;
            }
        }
        cx.load_runs = load_runs
            .into_iter()
            .map(|(a, n)| (a, cx.s.read(a, n)))
            .collect();
        // The main program, part by part, until it waits beyond the picture.
        let mut before = Some(Vec::new());
        self.run_main_split(&mut cx, PICTURE_END, &mut before);
        let mut before = before.unwrap_or_default();
        if let Some(u) = cx.unpack.take() {
            self.unpack = Some(u);
        }
        // An unpacking the beam sees half done: the part not yet written shows what was there.
        if let Some(u) = &self.unpack {
            let n = u.pending(self.frame);
            if n == 0 {
                self.unpack = None;
            } else {
                let mut st = cx.s.clone();
                st.write(u.at, &u.old[..n]);
                before.push((FRAME_LINES, st));
            }
        }
        let main_line = match cx.busy_from {
            Some(l) if cx.line >= FRAME_LINES => l,
            _ => cx.line,
        };
        history.push((main_line.max(VBL_END_LINE), window(cx.s, cop1lc)));
        // A frame that loaded stands for the original's frame the load ended on, long after the
        // load began: its copper list is the one set before the load, as the load left it.
        let cop1lc = match cx.load_point.take() {
            Some((list, snap)) => {
                history.retain(|(l, _)| *l > snap.0);
                history.insert(0, (0, snap.1));
                list
            }
            None => cop1lc,
        };
        let copper_word = |addr: u32, line: u16| -> u16 {
            let snap = history
                .iter()
                .rev()
                .find(|(l, _)| *l <= line)
                .map(|(_, s)| s)
                .unwrap_or(&history[0].1);
            snap.word(addr).unwrap_or_else(|| cx.s.w(addr))
        };
        let carry = self.fetch;
        // The slot machine's list turns sprite DMA off (its pointer hidden until a round, T93).
        let dmacon = |paula: &[PaulaWrite]| {
            let mut w: Vec<(u16, u16)> = paula
                .iter()
                .filter(|w| w.reg == crate::audio::DMACON && w.vpos < FRAME_LINES)
                .map(|w| (w.vpos, w.value))
                .collect();
            w.sort_by_key(|w| w.0);
            w
        };
        // The progress map's list blinks its marker the same way (T183).
        let follows_dma = cop1lc == crate::slot::SLOT_COPPER || cop1lc == h2(0x69f0);
        let dma = follows_dma.then(|| render::SpriteDma {
            on: self.spren,
            cpu: dmacon(cx.paula),
        });
        let (mut picture, copper) =
            render::screen(cx.s, display, cop1lc, &copper_word, carry, dma.as_ref());
        if cop1lc == render::LEVEL_COPPER {
            // `level_exit_fade` moves the window's top down, `level_start`'s slide back up: the
            // picture is fetched as the copper had it and framed by where the top is as the next
            // field starts, nothing fetched above the old top (T37).
            let top = cx.s.b(render::LEVEL_COPPER + 6) as u16;
            let bottom = picture.top + picture.height;
            if top != picture.top && top < bottom {
                picture = picture.rewindow(picture.left, top, picture.width, bottom - top);
            }
        }
        let mut end = carry.unwrap_or_default();
        for w in &copper {
            end.write(w.reg, w.value);
        }
        self.fetch = Some(end);
        // Lines the beam drew before a part of the main program ran show the memory before it.
        for (k, (line, st)) in before.iter().enumerate().rev() {
            let (older, _) = render::screen(st, display, cop1lc, &copper_word, carry, dma.as_ref());
            let from = before.get(k.wrapping_sub(1)).map_or(0, |b| b.0);
            if (older.width, older.height, older.top)
                != (picture.width, picture.height, picture.top)
            {
                continue;
            }
            let w = picture.width as usize;
            for y in 0..picture.height as usize {
                let raster = picture.top + y as u16;
                if raster >= from && raster < *line {
                    picture.pixels[y * w..(y + 1) * w]
                        .copy_from_slice(&older.pixels[y * w..(y + 1) * w]);
                    picture.palettes[y] = older.palettes[y];
                }
            }
        }
        // The copper's own writes to Paula's registers (the list's DMACON) come first: they are
        // made on line 0, before the interrupt's.
        let early: Vec<PaulaWrite> = copper
            .iter()
            .filter(|w| is_paula(w.reg) && stalled.is_none_or(|s| w.line >= s))
            .map(|w| PaulaWrite {
                reg: w.reg,
                value: w.value,
                hpos: 0,
                vpos: w.line,
            })
            .collect();
        cx.early = early.clone();
        self.run_main(&mut cx, u16::MAX);
        let cpu = dmacon(cx.paula);
        self.spren = render::SpriteDma {
            on: self.spren,
            cpu,
        }
        .at_end(&copper);
        let mut next_copper = Vec::new();
        if let Some(t) = cx.stall {
            // The CPU held off the bus into the next frame: the frame ends for conformance when it
            // takes the interrupt, after the next frame's copper has begun.
            let writes = render::copper_writes(cx.s, display, cx.cop1lc);
            let mut fetch = render::copper::Fetch::default();
            for w in writes.iter().filter(|w| w.line < t) {
                fetch.write(w.reg, w.value);
                if is_paula(w.reg) {
                    next_copper.push(PaulaWrite {
                        reg: w.reg,
                        value: w.value,
                        hpos: 0,
                        vpos: 0,
                    });
                }
            }
            if fetch.diwstrt != 0 || fetch.diwstop != 0 {
                let (h0, h1) = (fetch.diwstrt & 0xff, fetch.diwstop & 0xff | 0x100);
                let lines = fetch.lines();
                picture = picture.rewindow(
                    h0 as i32,
                    lines.start,
                    h1.saturating_sub(h0),
                    lines.end.saturating_sub(lines.start),
                );
            }
        }
        self.stalled = cx.stall;
        let load = cx.load;
        let disk_prompt = cx.disk_prompt.take();
        let resync = std::mem::take(&mut cx.resync);
        let jobs = std::mem::take(&mut cx.jobs);
        let beam_races = std::mem::take(&mut cx.races);
        self.ret = cx.ret;
        self.frame += 1;
        self.cop1lc = cx.cop1lc;
        self.keyboard = cx.keyboard;
        self.seed = cx.seed;
        self.unported.append(&mut cx.unported);
        self.slot_outcome = self.slot_outcome.or(cx.slot_outcome.take());
        self.outcome = cx.outcome.take().or(self.outcome);
        // In beam order: the copper's writes among the CPU's (lines are approximate for the CPU's:
        // the interrupt's at the top, the main program's where it woke). Positions are not
        // compared (the engine is not cycle-timed), so they are cleared.
        // A frame that loads stands for the original's frame its load ended on: what the CPU
        // wrote before this frame's interrupt (the end of work carried over from the last frame)
        // was written in the frame the load began, which conformance skips.
        if load.is_some() {
            paula.drain(..from_carry);
        }
        let mut all = early;
        all.extend(paula);
        all.sort_by_key(|w| w.vpos);
        self.note_coper(&all);
        // What the CPU wrote after the frame's end goes with the next frame.
        let split = all.partition_point(|w| w.vpos < FRAME_LINES);
        self.carry = all.split_off(split);
        for w in &mut self.carry {
            w.vpos -= FRAME_LINES;
        }
        all.extend(next_copper);
        self.audio
            .frame(&anchored(&all, &schedule), crate::slot::FRAME_LINES as u32);
        // The slot screen's drawing is timed on the beam (T93, T94): its lines are kept.
        if cop1lc != crate::slot::SLOT_COPPER {
            for w in &mut all {
                w.vpos = 0;
            }
        }
        FrontOut {
            picture,
            paula: all,
            load,
            resync,
            jobs,
            beam_races,
            disk_prompt,
        }
    }

    /// For conformance: where the original made the next frame's Paula writes, `(line, slot)` in
    /// order ([`crate::Game::anchor_writes`]), which the audio DMA the slot machine's lamps run
    /// beside is timed by (T129).
    pub fn anchor_writes(&mut self, writes: Vec<(u16, u16)>) {
        self.schedule = writes;
    }

    /// Run the main program's parts that wake before line `until` this frame.
    fn run_main(&mut self, cx: &mut Cx, until: u16) {
        self.run_main_split(cx, until, &mut None);
    }

    /// [`Front::run_main`], keeping in `before` the state as it was before each part that runs
    /// while the beam draws the picture (the lines above see it, the lines below what it did).
    fn run_main_split(&mut self, cx: &mut Cx, until: u16, before: &mut Option<Vec<(u16, State)>>) {
        loop {
            let Some(top) = self.stack.last_mut() else {
                return;
            };
            match &top.wait {
                None => {}
                Some((w, since)) => {
                    let wake = match w {
                        Yield::Raster => {
                            if *since < 0xff && cx.line <= RASTER_LINE {
                                Some(RASTER_LINE)
                            } else {
                                None
                            }
                        }
                        // The original's `cmpi.b #n,VPOS; bne` passes at once anywhere on line
                        // n: a wait begun there is over where the beam is known to the cycle
                        // (T203: `level_exit_fade`'s wait for $d0 after W6-3's loop, begun on
                        // 208); with whole lines, `n` may stand for the line before.
                        Yield::Line(n) => ((*since < *n || *since == *n && cx.into.is_some())
                            && cx.line <= *n)
                            .then_some(*n),
                        Yield::Fade(rec) => fade::done(cx.s, *rec).then_some(cx.line),
                        Yield::Poll => (*since == 0).then_some(cx.line),
                        Yield::Masked(_) => None,
                        Yield::Interrupt => Some(cx.line),
                        Yield::Busy(l) => (*l < FRAME_LINES as u32).then_some(*l as u16),
                        _ => Some(cx.line),
                    };
                    match wake {
                        Some(l) if l < until => {
                            if let Some(b) = before.as_mut()
                                && l > VBL_END_LINE
                            {
                                b.push((l, cx.s.clone()));
                            }
                            if l > cx.line {
                                cx.into = cx.into.map(|_| 0);
                            }
                            cx.line = cx.line.max(l);
                            top.wait = None;
                        }
                        _ => return,
                    }
                }
            }
            let (first, line) = (cx.paula.len(), cx.line);
            let pending = before.as_ref().map_or(0, |b| b.len()).wrapping_sub(1);
            let y = match top.deferred.take() {
                Some(y) => y,
                None => top.routine.run(cx),
            };
            // What the part wrote shows below where the beam is once it is done.
            if let Some(b) = before.as_mut()
                && let Some(e) = b.get_mut(pending)
                && e.0 == line
            {
                e.0 = cx.reveal.take().unwrap_or(cx.line.max(e.0));
                if let Some(st) = cx.reveal_state.take() {
                    e.1 = st;
                }
            }
            for w in &mut cx.paula[first..] {
                if w.vpos == 0 {
                    w.vpos = line;
                }
            }
            if cx.line >= FRAME_LINES && !matches!(y, Yield::Masked(_)) {
                // Busy past the frame's end: the rest waits for the CPU.
                top.deferred = Some(y);
                top.wait = Some((Yield::Busy(cx.line as u32), 0));
                return;
            }
            match y {
                Yield::Call(r) => {
                    top.wait = None;
                    self.stack.push(Frame {
                        routine: *r,
                        wait: None,
                        deferred: None,
                    });
                }
                Yield::Return => {
                    self.stack.pop();
                    if self.stack.is_empty() {
                        // A level entered with no `top_loop` known below it stays over, said
                        // as what the engine does not follow yet.
                        if let Some(o) = cx.outcome.or(self.outcome) {
                            cx.unported(match o {
                                crate::level::LevelOutcome::Completed { .. }
                                    if cx.s.l(crate::level::exit::WORLD)
                                        == crate::level::exit::BONUS_WORLD =>
                                {
                                    "the bonus level's end picture (T21)"
                                }
                                crate::level::LevelOutcome::Completed { .. } => {
                                    "the next level after the tally (T21)"
                                }
                                _ => "a level's end the engine does not follow",
                            });
                            self.stack.push(Frame {
                                routine: Routine::Outcome(o),
                                wait: Some((Yield::Poll, 1)),
                                deferred: None,
                            });
                            return;
                        }
                        cx.unported("the front end's program ran off its end");
                        return;
                    }
                }
                Yield::Jump(r) => {
                    self.stack.clear();
                    self.stack.push(Frame {
                        routine: *r,
                        wait: None,
                        deferred: None,
                    });
                }
                Yield::Fade(rec) if fade::done(cx.s, rec) => {}
                w => top.wait = Some((w, cx.line)),
            }
        }
    }

    /// The frame is the start of a new frame for every wait: a wait begun on an earlier frame
    /// counts as begun at line 0 of this one.
    pub(crate) fn new_frame(&mut self) {
        for f in &mut self.stack {
            if let Some((w, since)) = &mut f.wait {
                *since = 0;
                if let Yield::Busy(l) = w {
                    *l = l.saturating_sub(FRAME_LINES as u32);
                }
            }
        }
    }
}

fn is_paula(reg: u16) -> bool {
    matches!(reg, 0x96 | 0x9a | 0x9c | 0x9e) || (0xa0..0xe0).contains(&reg)
}

/// The copper list's bytes at one moment.
#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    at: u32,
    bytes: Vec<u8>,
}

impl Snapshot {
    /// The word at `addr` as `from` has it.
    fn copy_word(&mut self, from: &Snapshot, addr: u32) {
        self.copy_bits(from, addr, 0xffff);
    }

    /// The `mask` bits of the word at `addr` as `from` has them.
    fn copy_bits(&mut self, from: &Snapshot, addr: u32, mask: u16) {
        let (Some(a), Some(b)) = (self.word(addr), from.word(addr)) else {
            return;
        };
        let v = a & !mask | b & mask;
        let o = (addr - self.at) as usize;
        self.bytes[o..o + 2].copy_from_slice(&v.to_be_bytes());
    }

    fn word(&self, addr: u32) -> Option<u16> {
        let o = addr.checked_sub(self.at)? as usize;
        Some(u16::from_be_bytes([
            *self.bytes.get(o)?,
            *self.bytes.get(o + 1)?,
        ]))
    }
}

/// Where the original's main program was at the end of a frame: its PC, then each long on its
/// stack that is a return address into the game's code (after a `jsr abs.l` or a `bsr.w`),
/// innermost first. At a frame's end the CPU stands at the vertical blank's handler, with the
/// interrupted PC on the supervisor stack.
pub fn main_chain(mem: &crate::layout::AmigaMemory, cpu: &crate::game::Cpu) -> Vec<u32> {
    use crate::layout::Ram;
    let long = |a: u32| {
        (mem.byte(a) as u32) << 24
            | (mem.byte(a + 1) as u32) << 16
            | (mem.byte(a + 2) as u32) << 8
            | mem.byte(a + 3) as u32
    };
    let word = |a: u32| (mem.byte(a) as u32) << 8 | mem.byte(a + 1) as u32;
    let (pc, mut sp) = if cpu.pc == h0(0x11bf8) {
        (long(cpu.ssp + 2), cpu.ssp + 6)
    } else {
        (cpu.pc, cpu.a[7])
    };
    let code = h0(0)..h0(0x4834c);
    let mut chain = vec![pc];
    let top = (sp & !0xFFFF) + 0x1_0000;
    while sp + 4 <= top.min(sp + 0x400) && chain.len() < 8 {
        let a = long(sp);
        let call = code.contains(&a)
            && (word(a - 6) == 0x4eb9 && long(a - 4) >= h0(0) || word(a - 4) == 0x6100);
        if call {
            chain.push(a);
        }
        sp += 2;
    }
    chain
}

/// `h0+$off` in the original's memory.
pub(crate) const fn at(off: u32) -> u32 {
    h0(off)
}

/// What runs on by itself while the original loads, taken from the original as it was just
/// before the frame the engine's instant load is compared on (DESIGN §2.1): `kbd_toggle`, flipped
/// every frame, and the keys it polls, and the sound driver, which plays on in the vertical blank.
pub const RUNS_DURING_LOAD: &[(u32, u32)] = &[
    (h0(0x16076), 4),
    (
        h0(crate::audio::vars::SFX.0),
        crate::audio::vars::SFX.1 as u32,
    ),
    (
        h0(crate::audio::vars::MUSIC_ON.0),
        crate::audio::vars::MUSIC_ON.1 as u32,
    ),
    (
        h0(crate::audio::vars::P41.0),
        crate::audio::vars::P41.1 as u32,
    ),
    (
        h0(crate::audio::vars::SHADOW.0),
        crate::audio::vars::SHADOW.1 as u32,
    ),
    (
        h0(crate::audio::vars::STREAM.0),
        crate::audio::vars::STREAM.1 as u32,
    ),
    // The screens the original loads under (T183; owner, 2026-10-15): the loading picture's
    // walk and clouds, the progress map's blink and fade, the end-of-world picture's time, each
    // run by the vertical blank and waited for by the main program after the load.
    (h0(0x123d4), 2),                    // display_timer
    (h0(0x9f8), 2),                      // frog_exit_sprites
    (h0(0x4112), 4),                     // map_shown
    (h0(0x11e92), 4),                    // exit_walk_state
    (h0(0x11fd8), 6),                    // exit_walk_count
    (h0(0x122f0), 0x4a),                 // cloud_bob
    (h0(0x1b02c), 0x38 + 0xa8),          // exit_sprite_left, _right, cloud_sprites
    (h0(0x802), 0xe),                    // frog_anim .. frog_sprite_dy
    (h0(0x3f50), 2),                     // map_blink
    (h0(0x157d8), 4 * 0x1f2),            // fades
    (crate::layout::h2(0x6750), 0x480),  // intro_copper, exit_sprite_copper, map_copper
];

/// What a load leaves that the engine does not model, taken from the original once the load is
/// over: the trackloader's sector buffer (h0+$b35a points it at h3+$35860, below the overlay).
pub const LOAD_LEAVES: &[(u32, u32)] = &[(
    crate::layout::h3(0x35860),
    data::OVERLAY_AT - crate::layout::h3(0x35860),
)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KeySet;

    #[test]
    fn writes_take_the_originals_positions_when_given_for_all_else_their_lines() {
        let w = |vpos| PaulaWrite {
            reg: 0x96,
            value: 0x8001,
            hpos: 7,
            vpos,
        };
        let ours = [w(1), w(299)];
        let given = anchored(&ours, &[(15, 145), (299, 88), (300, 4)]);
        assert_eq!(
            given.iter().map(|w| (w.vpos, w.hpos)).collect::<Vec<_>>(),
            [(15, 145), (299, 88)]
        );
        // Fewer positions than writes (normal play: none): each at its own line's start.
        let own = anchored(&ours, &[(15, 145)]);
        assert_eq!(
            own.iter().map(|w| (w.vpos, w.hpos)).collect::<Vec<_>>(),
            [(1, 0), (299, 0)]
        );
    }

    #[test]
    fn the_keyboard_sends_a_byte_per_change_and_reads_ff_once_handled() {
        let mut k = Keyboard::default();
        // Power-on: $7e released, never acknowledged.
        assert_eq!((!k.sdr >> 1 & 0x7f, !k.sdr & 1), (0x7e, 1));
        assert!(!k.new);
        k.feed(KeySet::key(0x19));
        assert_eq!((!k.sdr >> 1 & 0x7f, !k.sdr & 1, k.new), (0x19, 0, true));
        k.feed(KeySet::key(0x19));
        assert!(k.new, "holding a key sends nothing more");
        k.feed(KeySet::NONE);
        assert_eq!((!k.sdr >> 1 & 0x7f, !k.sdr & 1), (0x19, 1));
    }

    #[test]
    fn a_fade_out_reaches_black_and_says_so() {
        let mut s = State::default();
        let rec = fade::RECORDS[0];
        let list = h0(0x3ba0);
        for k in 0..4 {
            s.set_w(list + 4 * k + 2, [0xfff, 0x842, 0x00f, 0x000][k as usize]);
        }
        s.set_w(rec.at + fade::SPEED, 1);
        fade::fade_out(&mut s, rec, list, 4);
        let mut frames = 0;
        while !fade::done(&s, rec) {
            fade::step_out(&mut s);
            frames += 1;
            assert!(frames < 40);
        }
        assert!((0..4).all(|k| s.w(list + 4 * k + 2) == 0));
        assert!((15..=17).contains(&frames), "{frames}");
    }

    #[test]
    fn a_blit_copies_a_shifted_word_run_with_its_masks() {
        let mut s = State::default();
        let src = crate::layout::h3(0x10600);
        let dst = src + 0x100;
        s.write(src, &[0x12, 0x34, 0x56, 0x78]);
        blit::Blit {
            con0: 0x09f0 | 4 << 12,
            afwm: 0xffff,
            alwm: 0xffff,
            pt: [src, 0, 0, dst],
            size: 1 << 6 | 2,
            ..blit::Blit::default()
        }
        .run(&mut s);
        assert_eq!(s.read(dst, 4), [0x01, 0x23, 0x45, 0x67]);
        assert!(s.scratch.is_empty(), "a display buffer is modelled memory");
    }
}
