//! CPU time on the beam (T36): where in the frame a level's work happens.
//!
//! The original's picture is drawn while its CPU works: the vertical blank's interrupt from line 0,
//! then the main loop's pass, which in a level's first frame draws its bobs into the buffer being
//! shown, redraws a taken pickup's tile into both buffers while the beam may be below it, and in
//! busy stretches runs on past the frame's end, the next interrupt landing in the middle of it
//! (docs/08 "Lag"). So the ported routines say what they cost ([`super::LevelCtx::spend`] for the
//! CPU, [`super::LevelCtx::blit_words`] for a blit the CPU waits on), and a [`Clock`] turns that
//! into the beam's line: a line gives the CPU fewer cycles while the display fetches bitplanes,
//! and the blitter fewer DMA slots. The costs are fitted to the original's beam positions on the
//! oracle (its DMACON, INTENA and INTREQ writes, and where its CPU is at each line's end).

/// Lines in a PAL frame.
pub const FRAME_LINES: u16 = 313;
/// CPU cycles in a raster line (227 DMA slots of two).
pub const LINE_CYCLES: u32 = 454;
/// DMA slots in a raster line.
pub const LINE_SLOTS: u32 = 227;

/// The level's display fetches bitplanes on these lines (the panel from `$27`, the play field to
/// the window's end): a line there gives the CPU and the blitter less.
const FETCH_LINES: std::ops::Range<u16> = 0x27..0x12c;
/// CPU cycles a fetching line takes from the CPU: the fifth plane's fetches use slots the CPU
/// would (fitted).
const FETCH_STOLEN_CYCLES: u32 = 40;
/// DMA slots a line leaves the blitter: the whole line but for refresh, audio and sprites, and on
/// a fetching line the bitplanes' 5 × 20 too (fitted).
const FREE_SLOTS: u32 = 203;
const FREE_SLOTS_FETCHING: u32 = 103;

/// What the level's work costs, in CPU cycles, fitted to the original's beam positions on the
/// oracle (the `items-*` and `frog-*` cases' frames): [`COSTS`] by [`Cost`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cost {
    // --- The vertical blank (h0+$11bf8) ---
    /// The interrupt taken, registers saved, and `scroll_latch` (h0+$1700a).
    VblEntry,
    /// The frog's routines: the drop-in's (`frog_level_entry` h0+$fa02), the demo's recorder and
    /// player, the map's markers under it (`frog_tiles` h0+$e2e2), the air animation, the
    /// animation tick, the joystick, the input animation, the sprites, the clock, the move and
    /// the camera.
    FrogEntry,
    Demo,
    FrogTiles,
    FrogAir,
    FrogAnim,
    FrogWalk,
    FrogInput,
    FrogSprites,
    ClockTick,
    FrogMove,
    Camera,
    /// The music player's tick (h0+$147e4).
    Music,
    /// After a Paula write by the sound driver: a channel stopped (DMACON without its set bit: it
    /// waits for the DMA to let go), interrupts off (INTENA `$4000`: a sample set up), any other.
    AudioStop,
    AudioMasked,
    AudioWrite,
    /// An effect or a sample started (`sfx_play` h0+$12a72, `sample_play` h0+$148b8).
    SfxPlay,
    /// The scroller's checks of the rows or columns in view (h0+$1668c), a tile it draws into the
    /// clean buffer with the CPU (80 words), and a row or column queued for the copper interrupt.
    Scroller,
    ScrollTile,
    ScrollQueue,
    /// `map_spawn_scan_edges` (h0+$17810): its set-up, a cell looked at, a marker's spawner
    /// called (its stub, registers saved and restored), an object put in a slot (the search, the
    /// template copied, the bob set up).
    Scan,
    ScanCell,
    Spawner,
    Spawn,
    /// The rest of the handler: the effects, the keyboard, the timers, the fades.
    VblTail,
    /// The copper interrupt's set-up of a queued copy (h0+$1693e), each plane's two blits.
    IrqBlits,
    IrqPlane,
    // --- The pass (h0+$ab8) ---
    /// The main loop waking (`vbl_count` reached 2) and the pass's start.
    PassEntry,
    /// `spud_update` (h0+$80f0).
    Spud,
    /// `obj_run_all` (h0+$1b324): a slot looked at, and a live slot's update called.
    ObjSlot,
    ObjUpdate,
    /// The slot routines ([`routine_cost`]): the coin's update, the falling coin's, the hidden
    /// spring's, the exit lock's, a fixture's (pad, launcher, lever), an init, a touch handler,
    /// any other.
    CoinUpdate,
    FallingCoinUpdate,
    SpringUpdate,
    LockUpdate,
    FixtureUpdate,
    Init,
    Touch,
    Routine,
    /// `obj_build_hitboxes` (h0+$d2f6): a slot looked at, and a hitbox entered.
    HitboxSlot,
    Hitbox,
    /// `collide_all` (h0+$d0cc): its set-up, and a hitbox entry looked at by the spud's and the
    /// frog's walks.
    Collide,
    CollideEntry,
    /// `build_bob_draw_list` (h0+$19b42): its set-up, and a slot looked at (twice).
    DrawList,
    DrawListSlot,
    /// `bob_draw` (h0+$19120): its set-up, and a bob's screen position.
    BobDraw,
    BobPosition,
    /// A part's start and end (DMACON), a bob's erase looked at, and an erase's blits' set-up.
    BobPart,
    BobEraseLook,
    BobErase,
    /// A bob drawn (its animation stepped and clipped), and when it shows, its blits' set-up and
    /// each plane's.
    Bob,
    BobShown,
    BobPlane,
    /// The flip, the tile animation queue and a cell it redraws (80 words into the clean
    /// buffer by the CPU, then copied by the blitter).
    AfterDraw,
    Redraw,
    /// The panel's update (a digit drawn by the CPU) and a panel blit's set-up.
    Panel,
    PanelBlit,
    /// The rest: the pause, the passage, the extra life, the time.
    PassTail,
}

impl Cost {
    pub const COUNT: usize = Cost::PassTail as usize + 1;
    pub const ALL: [Cost; Cost::COUNT] = [
        Cost::VblEntry,
        Cost::FrogEntry,
        Cost::Demo,
        Cost::FrogTiles,
        Cost::FrogAir,
        Cost::FrogAnim,
        Cost::FrogWalk,
        Cost::FrogInput,
        Cost::FrogSprites,
        Cost::ClockTick,
        Cost::FrogMove,
        Cost::Camera,
        Cost::Music,
        Cost::AudioStop,
        Cost::AudioMasked,
        Cost::AudioWrite,
        Cost::SfxPlay,
        Cost::Scroller,
        Cost::ScrollTile,
        Cost::ScrollQueue,
        Cost::Scan,
        Cost::ScanCell,
        Cost::Spawner,
        Cost::Spawn,
        Cost::VblTail,
        Cost::IrqBlits,
        Cost::IrqPlane,
        Cost::PassEntry,
        Cost::Spud,
        Cost::ObjSlot,
        Cost::ObjUpdate,
        Cost::CoinUpdate,
        Cost::FallingCoinUpdate,
        Cost::SpringUpdate,
        Cost::LockUpdate,
        Cost::FixtureUpdate,
        Cost::Init,
        Cost::Touch,
        Cost::Routine,
        Cost::HitboxSlot,
        Cost::Hitbox,
        Cost::Collide,
        Cost::CollideEntry,
        Cost::DrawList,
        Cost::DrawListSlot,
        Cost::BobDraw,
        Cost::BobPosition,
        Cost::BobPart,
        Cost::BobEraseLook,
        Cost::BobErase,
        Cost::Bob,
        Cost::BobShown,
        Cost::BobPlane,
        Cost::AfterDraw,
        Cost::Redraw,
        Cost::Panel,
        Cost::PanelBlit,
        Cost::PassTail,
    ];
}

impl Cost {
    /// What it takes, in CPU cycles.
    pub const fn cycles(self) -> u32 {
        match self {
            Cost::VblEntry => 2981,
            Cost::FrogEntry => 327,
            Cost::Demo => 120,
            Cost::FrogTiles => 2122,
            Cost::FrogAir => 570,
            Cost::FrogAnim => 513,
            Cost::FrogWalk => 845,
            Cost::FrogInput => 546,
            Cost::FrogSprites => 1340,
            Cost::ClockTick => 224,
            Cost::FrogMove => 417,
            Cost::Camera => 664,
            Cost::Music => 1042,
            Cost::AudioStop => 500,
            Cost::AudioMasked => 500,
            Cost::AudioWrite => 88,
            Cost::SfxPlay => 500,
            Cost::Scroller => 3322,
            Cost::ScrollTile => 1468,
            Cost::ScrollQueue => 1969,
            Cost::Scan => 159,
            Cost::ScanCell => 68,
            Cost::Spawner => 195,
            Cost::Spawn => 1908,
            Cost::VblTail => 1798,
            Cost::IrqBlits => 1772,
            Cost::IrqPlane => 360,
            Cost::PassEntry => 687,
            Cost::Spud => 760,
            Cost::ObjSlot => 59,
            Cost::ObjUpdate => 294,
            Cost::CoinUpdate => 498,
            Cost::FallingCoinUpdate => 307,
            Cost::SpringUpdate => 346,
            Cost::LockUpdate => 300,
            Cost::FixtureUpdate => 500,
            Cost::Init => 1008,
            Cost::Touch => 2453,
            Cost::Routine => 500,
            Cost::HitboxSlot => 76,
            Cost::Hitbox => 131,
            Cost::Collide => 909,
            Cost::CollideEntry => 45,
            Cost::DrawList => 1058,
            Cost::DrawListSlot => 163,
            Cost::BobDraw => 1091,
            Cost::BobPosition => 106,
            Cost::BobPart => 640,
            Cost::BobEraseLook => 55,
            Cost::BobErase => 105,
            Cost::Bob => 831,
            Cost::BobShown => 223,
            Cost::BobPlane => 142,
            Cost::AfterDraw => 493,
            Cost::Redraw => 8815,
            Cost::Panel => 1327,
            Cost::PanelBlit => 647,
            Cost::PassTail => 1144,
        }
    }
}

/// What each [`Cost`] takes, in CPU cycles, by its number.
pub const COSTS: [u32; Cost::COUNT] = {
    let mut a = [0; Cost::COUNT];
    let mut k = 0;
    while k < Cost::COUNT {
        a[k] = Cost::ALL[k].cycles();
        k += 1;
    }
    a
};

/// The cost of the slot routine at `addr` (the original's address), by what it is.
pub fn routine_cost(addr: u32) -> Cost {
    use crate::layout::h0;
    match addr.wrapping_sub(h0(0)) {
        0x23f86 => Cost::CoinUpdate,
        0x23fae => Cost::FallingCoinUpdate,
        0x2408a => Cost::SpringUpdate,
        0x2404c => Cost::LockUpdate,
        0x241b2 | 0x240ea | 0x255f6 => Cost::FixtureUpdate,
        0x23f60 | 0x2406a | 0x24036 | 0x24106 | 0x2414a | 0x240ca | 0x240aa | 0x255ca..=0x256a0 => {
            Cost::Init
        }
        0x239f6 | 0x23a8e | 0x23970 => Cost::Touch,
        _ => Cost::Routine,
    }
}

/// One step of the clock, recorded for fitting the costs ([`super::LevelCtx::trace`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tick {
    Cpu(Cost, u32),
    Blit(u32),
    Wait(u16),
    /// The frame's Paula write with this index was made here.
    Write(u32),
}

/// The beam's position as the CPU sees it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Clock {
    /// The line, counted from the frame's vertical blank; past [`FRAME_LINES`] when work runs on
    /// past the frame's end.
    pub line: u16,
    /// CPU cycles of the line already used.
    pub into: u32,
}

impl Clock {
    /// The frame's start, where the vertical blank's interrupt is taken.
    pub const START: Clock = Clock { line: 0, into: 0 };

    fn fetching(line: u16) -> bool {
        FETCH_LINES.contains(&(line % FRAME_LINES))
    }

    /// CPU cycles a line gives the CPU.
    fn cpu_cycles(line: u16) -> u32 {
        if Clock::fetching(line) {
            LINE_CYCLES - FETCH_STOLEN_CYCLES
        } else {
            LINE_CYCLES
        }
    }

    /// The CPU works `cycles`.
    pub fn cpu(&mut self, mut cycles: u32) {
        loop {
            let per = Clock::cpu_cycles(self.line);
            if self.into + cycles < per {
                self.into += cycles;
                return;
            }
            cycles -= per - self.into;
            self.into = 0;
            self.line = self.line.saturating_add(1);
        }
    }

    /// The blitter has the bus for `slots` DMA slots, the CPU waiting.
    pub fn blit(&mut self, mut slots: u32) {
        loop {
            let per = if Clock::fetching(self.line) {
                FREE_SLOTS_FETCHING
            } else {
                FREE_SLOTS
            };
            // What is left of the line, in the blitter's slots.
            let left = (LINE_CYCLES - self.into.min(LINE_CYCLES)) * per / LINE_CYCLES;
            if slots < left {
                self.into += slots * LINE_CYCLES / per;
                return;
            }
            slots -= left;
            self.into = 0;
            self.line = self.line.saturating_add(1);
        }
    }

    /// The horizontal position in DMA slots, as the beam counts it.
    pub fn hpos(&self) -> u16 {
        (self.into / 2).min(LINE_SLOTS - 1) as u16
    }

    /// Past the frame's end: the same moment counted from the next frame's start.
    pub fn next_frame(&self) -> Clock {
        Clock {
            line: self.line.saturating_sub(FRAME_LINES),
            into: self.into,
        }
    }
}

/// Where a frame ended in the middle of a pass ([`crate::Pacing::Late`]): after its `writes`th
/// Paula write, the last at `line`, slot `hpos`, in the pass's `phase`th call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cut {
    pub writes: u16,
    pub line: u16,
    pub hpos: u16,
    pub phase: u8,
}

/// A frame the main program runs through: its vertical blank's hardware, and, when it ends in
/// the middle of a pass, where.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameSpec {
    pub hw: super::Hw,
    pub cut: Option<Cut>,
}

/// A pass the frame ended in the middle of (a late pass, DESIGN §2.1): the state as the pass
/// began, where the beam was, the hardware then and the Paula writes its frame had made by then,
/// and each frame it has run through (the one it began in first). The next frame runs it again
/// from its start, each of those frames ending where it did, then runs its own ([`Frames`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pending {
    pub start: Box<crate::state::State>,
    pub clock: Clock,
    pub hw: super::Hw,
    pub pre: u16,
    pub frames: Vec<FrameSpec>,
}

/// The level's main program on the beam for one step ([`super::frame::main_program`]): the
/// frames it runs through (the last is the step's own; any before it are a pending pass's, run
/// again), where it is, and what the step gives once its frame ends.
#[derive(Debug, Default)]
pub struct Frames {
    pub specs: Vec<FrameSpec>,
    /// The frame being run, once its vertical blank has come; the Paula write its writes start
    /// at, the writes it had made before (a pass run again from its middle), and whether its
    /// clock has been set by its cut's last write.
    pub at: Option<usize>,
    pub from: usize,
    pub pre: u16,
    pub anchored: bool,
    pub in_vbl: bool,
    pub in_pass: bool,
    /// A pass not interrupted at its frame's end (the frame ended on time), running on past it.
    pub past_end: bool,
    /// The pass being run: the state, beam and hardware as it began and the writes its frame had
    /// made by then (kept when its frame may end in the middle of it), and the frames it has run
    /// through.
    pub pass_start: Option<(Box<crate::state::State>, Clock, super::Hw, u16)>,
    pub pass_frames: Vec<FrameSpec>,
    /// The step's own frame's Paula writes: from its vertical blank to its end.
    pub out_from: usize,
    pub out_to: Option<usize>,
    /// The frame has ended: the state then, if a pass was in the middle, and that pass.
    pub done: bool,
    pub mid: Option<Box<crate::state::State>>,
    pub pending: Option<Pending>,
    /// What the step's own frame wrote to the display's memory.
    pub log: Option<crate::mem::WriteLog>,
}

impl Frames {
    /// The frame being run: whether it is the step's own, and where it ends.
    pub fn current(&self) -> Option<(bool, Option<Cut>)> {
        let at = self.at?;
        Some((at + 1 == self.specs.len(), self.specs[at].cut))
    }
}

/// DMA slots one word of a blit takes, by the channels BLTCON0 enables (A, B, C, D): the
/// hardware reference's table.
pub fn slots_per_word(con0: u16) -> u32 {
    let (a, b, c, d) = (con0 & 0x800 != 0, con0 & 0x400 != 0, con0 & 0x200 != 0, con0 & 0x100 != 0);
    match (a, b, c, d) {
        (_, true, true, _) => 4,
        (_, true, false, _) | (_, false, true, true) => 3,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cpu_gets_less_of_a_fetching_line() {
        let mut c = Clock::START;
        c.cpu(LINE_CYCLES * 10);
        assert_eq!((c.line, c.into), (10, 0));
        let mut c = Clock { line: 0x30, into: 0 };
        c.cpu(LINE_CYCLES);
        assert_eq!(c.line, 0x31);
        assert_eq!(c.into, FETCH_STOLEN_CYCLES);
    }

    #[test]
    fn a_blit_takes_longer_under_the_display() {
        let mut blank = Clock::START;
        blank.blit(FREE_SLOTS * 2);
        assert_eq!(blank.line, 2);
        let mut shown = Clock { line: 0x40, into: 0 };
        shown.blit(FREE_SLOTS * 2);
        assert!(shown.line > 0x43);
        assert_eq!(slots_per_word(0x0fca), 4);
        assert_eq!(slots_per_word(0x09f0), 2);
    }
}
