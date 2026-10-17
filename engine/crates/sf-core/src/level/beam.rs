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
/// A level line's bitplane fetch: its slots, from DDFSTRT (`$28`, a word early for the
/// scroll) to DDFSTOP's last unit (`$d8` + 8).
const FETCH_FROM: u32 = 0x28;
const FETCH_TO: u32 = 0xe0;

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
    /// `frog_anim_tick`'s work past its sprite wait (h0+$1027a): the pose's height and data
    /// pointers into the sprite records and the copper list. Its head ([`Cost::FrogAnim`]) is
    /// the part before the wait (T127): of the routine's 652 cycles on a new pose, 300 to the
    /// wait's read of VPOS, 352 after it; the fitted 513 is split in that ratio. Charged after
    /// the routine on every path, as the fitted total was.
    FrogAnimTail,
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
    /// World 1's hover and watcher updates (h0+$243b6, h0+$26904; T42), about twice any other
    /// enemy's: measured on Moira over `enemies_w1_routines`' states (medians).
    HoverUpdate,
    WatcherUpdate,
    /// `obj_build_hitboxes` (h0+$d2f6): a slot looked at, and a hitbox entered.
    HitboxSlot,
    Hitbox,
    /// `collide_all` (h0+$d0cc): its set-up, and a hitbox entry looked at by the spud's and the
    /// frog's walks.
    Collide,
    CollideEntry,
    /// `build_bob_draw_list` (h0+$19b42): its set-up, and a slot looked at (in each of its
    /// three loops).
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
    /// buffer by the CPU, then copied by the blitter, each plane's copy into each buffer set up).
    AfterDraw,
    Redraw,
    RedrawPlane,
    /// The panel's update (a digit drawn by the CPU) and a panel blit's set-up.
    Panel,
    PanelBlit,
    /// The rest: the pause, the passage, the extra life, the time.
    PassTail,
    // --- Between levels: `level_init_wN` (T37, the restart after a death) ---
    /// `obj_free_all` (h0+$1b9ee), the frog's reset and the start of h0+$1995a; and a slot in
    /// use that `obj_free_all` frees (only [`RESTART_COSTS`] has it).
    InitStart,
    InitFreed,
    /// h0+$1995a's parts: a frame record set (h0+$18e8a), a frame record's blit parameters
    /// (h0+$18f6c), a bob record's sub-records (h0+$18ef8), a byte of the mask cleared by the CPU,
    /// ten words of the mask's clear blit and of a plane's OR into it (the blitter's priority on),
    /// and the sprites' set-up after.
    FrameRecord,
    BlitParams,
    SubRecords,
    MaskClearByte,
    MaskClear10,
    MaskOr10,
    BobSprites,
    /// A byte of the panel cleared (h0+$8bf6), `level_start_point`'s camera and scroller reset,
    /// and a cell `map_spawn_scan_screen` (h0+$1777c) looks at (its handler called for every
    /// cell, all registers saved and restored).
    PanelClearByte,
    StartPoint,
    ScreenScanCell,
    /// A marker's spawner `map_spawn_scan_screen` runs, over what its spawn costs (only
    /// [`RESTART_COSTS`] has it).
    ScreenSpawner,
}

impl Cost {
    pub const COUNT: usize = Cost::ScreenSpawner as usize + 1;
    pub const ALL: [Cost; Cost::COUNT] = [
        Cost::VblEntry,
        Cost::FrogEntry,
        Cost::Demo,
        Cost::FrogTiles,
        Cost::FrogAir,
        Cost::FrogAnim,
        Cost::FrogAnimTail,
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
        Cost::HoverUpdate,
        Cost::WatcherUpdate,
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
        Cost::RedrawPlane,
        Cost::Panel,
        Cost::PanelBlit,
        Cost::PassTail,
        Cost::InitStart,
        Cost::InitFreed,
        Cost::FrameRecord,
        Cost::BlitParams,
        Cost::SubRecords,
        Cost::MaskClearByte,
        Cost::MaskClear10,
        Cost::MaskOr10,
        Cost::BobSprites,
        Cost::PanelClearByte,
        Cost::StartPoint,
        Cost::ScreenScanCell,
        Cost::ScreenSpawner,
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
            Cost::FrogAnim => 236,
            Cost::FrogAnimTail => 277,
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
            Cost::HoverUpdate => 946,
            Cost::WatcherUpdate => 1070,
            Cost::HitboxSlot => 76,
            Cost::Hitbox => 131,
            Cost::Collide => 909,
            Cost::CollideEntry => 45,
            Cost::DrawList => 1058,
            Cost::DrawListSlot => 54,
            Cost::BobDraw => 1091,
            Cost::BobPosition => 106,
            Cost::BobPart => 640,
            Cost::BobEraseLook => 55,
            Cost::BobErase => 105,
            Cost::Bob => 831,
            Cost::BobShown => 223,
            Cost::BobPlane => 142,
            Cost::AfterDraw => 493,
            Cost::Redraw => 4000,
            Cost::RedrawPlane => 40,
            Cost::Panel => 1327,
            Cost::PanelBlit => 647,
            Cost::PassTail => 1144,
            Cost::InitStart => 15620,
            Cost::InitFreed => 0,
            Cost::FrameRecord => 337,
            Cost::BlitParams => 418,
            Cost::SubRecords => 1000,
            Cost::MaskClearByte => 30,
            Cost::MaskClear10 => 41,
            Cost::MaskOr10 => 62,
            Cost::BobSprites => 3000,
            Cost::PanelClearByte => 29,
            Cost::StartPoint => 2000,
            Cost::ScreenScanCell => 386,
            Cost::ScreenSpawner => 0,
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

/// [`COSTS`] for work done with the screen blank between levels (`level_init_wN`): no plane
/// fetch takes the CPU's or the blitter's cycles, and the first screen's copies are called
/// directly, not from the copper interrupt. T100 refitted two once the front end's handler took
/// its own length (T37's were fitted to a handler always ending on line 20, about a line and a
/// half long): the sheet's masks (the restart's long blit ends on line 270, the original's 271)
/// and the screen scan (the blit chain after it starts on line 82 as the original's, 75 for 73
/// on the second restart).
pub const BLANK_COSTS: [u32; Cost::COUNT] = {
    let mut a = COSTS;
    a[Cost::ScrollTile as usize] = 1313;
    a[Cost::IrqBlits as usize] = 450;
    a[Cost::IrqPlane as usize] = 270;
    a[Cost::MaskOr10 as usize] = 63;
    a[Cost::ScreenScanCell as usize] = 400;
    a
};

/// [`BLANK_COSTS`] for the restart after a death (T199). Its set-up runs with the level's five
/// planes still fetched on most lines, a level's entry after a load with none, and the parts slow
/// unevenly (the frame records a fifth, the mask's byte clear not at all), so the restart has its
/// own fit; [`BLANK_COSTS`] stays the level entries'. Measured on the oracle (`restart_probe`)
/// across `frog-pit-w1-1`'s two restarts, its objects live and frozen (13 slots in use, 14 bob
/// records against none and 1), each of the set-up's writes against the original's: every one
/// within 3 lines. `obj_free_all` is 5 lines longer live, h0+$18ef8 86 (3003 a record; T37's
/// 1000 was fitted to one, its 2003 more come off `InitStart`), the 8 spawns of the screen scan
/// 10 lines more than the engine's spawns; the masks' blits began 6 lines early, their clear was 4
/// long, the scan 9 long and the redraw after it 8 short.
pub const RESTART_COSTS: [u32; Cost::COUNT] = {
    let mut a = BLANK_COSTS;
    a[Cost::InitStart as usize] = 15620 - 2003 + 2724;
    a[Cost::InitFreed as usize] = 175;
    a[Cost::SubRecords as usize] = 3003;
    a[Cost::MaskClear10 as usize] = 38;
    a[Cost::ScreenScanCell as usize] = 390;
    a[Cost::ScreenSpawner as usize] = 560;
    a[Cost::ScrollTile as usize] = 1322;
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
        0x243b6 => Cost::HoverUpdate,
        0x26904 => Cost::WatcherUpdate,
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
    /// How far into the line, in [`LINE_CYCLES`] (the beam's own pace, two a DMA slot).
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
            // What is left of the line, in the CPU's cycles.
            let left = (LINE_CYCLES - self.into.min(LINE_CYCLES)) * per / LINE_CYCLES;
            if cycles < left {
                self.into += cycles * LINE_CYCLES / per;
                return;
            }
            cycles -= left;
            self.into = 0;
            self.line = self.line.saturating_add(1);
        }
    }

    /// Whether the blitter may have DMA slot `slot` of line `line`: not the memory refresh's
    /// (`$01`, `$03`, `$05`, `$e2`), and on a fetching line not the five planes' (lores: five of
    /// each 8 slots from DDFSTRT `$28` to DDFSTOP `$d8`'s end). Audio and sprite fetches, which
    /// come and go, are not counted (the oracle's DMA debugger, measured).
    fn blitter_slot(line: u16, slot: u32) -> bool {
        if matches!(slot, 1 | 3 | 5 | 0xe2) {
            return false;
        }
        if Clock::fetching(line) && (FETCH_FROM..FETCH_TO).contains(&slot) {
            return matches!((slot - FETCH_FROM) % 8, 0 | 2 | 4);
        }
        true
    }

    /// The blitter has the bus for `slots` DMA slots, the CPU waiting.
    pub fn blit(&mut self, mut slots: u32) {
        let mut slot = self.into / 2;
        while slots > 0 {
            if slot >= LINE_SLOTS {
                slot = 0;
                self.line = self.line.saturating_add(1);
            }
            if Clock::blitter_slot(self.line, slot) {
                slots -= 1;
            }
            slot += 1;
        }
        self.into = 2 * slot;
        if self.into >= LINE_CYCLES {
            self.into = 0;
            self.line = self.line.saturating_add(1);
        }
    }

    /// A blit of `words` words with BLTCON0 `con0`, started (BLTSIZE written) here, cycle by
    /// cycle as the reference emulator's blitter runs it (vAmiga's `SlowBlitter`): two start-up
    /// cycles, then one micro-instruction a cycle, each that uses the bus waiting for a DMA slot
    /// free to it: per word a read for each of A, B and C it uses, then the pipelined D write of
    /// the word before (the first word's only waits for a free slot); after the last word one
    /// cycle needing no bus, then the last word's D. Returns where each word's D write was made;
    /// the clock is left where the blit ended.
    pub fn blit_cycles(&mut self, words: u32, con0: u16) -> Vec<u32> {
        let reads = (con0 >> 9 & 7).count_ones();
        let mut slot = self.into / 2;
        let mut line = self.line;
        // The next cycle that may run an instruction: with `bus`, the next free one.
        let mut next = |bus: bool| loop {
            slot += 1;
            if slot >= LINE_SLOTS {
                slot -= LINE_SLOTS;
                line = line.saturating_add(1);
            }
            if !bus || Clock::blitter_slot(line, slot) {
                return line as u32 * LINE_CYCLES + 2 * slot;
            }
        };
        // BLTSIZE takes effect a cycle after its write; the blit starts on the next two cycles
        // that find the bus free.
        next(false);
        next(true);
        next(true);
        let mut out = Vec::with_capacity(words as usize);
        for w in 0..words {
            for _ in 0..reads {
                next(true);
            }
            let at = next(true);
            if w > 0 {
                out.push(at);
            }
        }
        next(false);
        out.push(next(true));
        let end = line as u32 * LINE_CYCLES + 2 * (slot + 1);
        self.line = (end / LINE_CYCLES) as u16;
        self.into = end % LINE_CYCLES;
        if con0 & 0x100 == 0 {
            out.clear();
        }
        out
    }

    /// The position in [`LINE_CYCLES`] from the frame's start.
    pub fn nominal(&self) -> u32 {
        self.line as u32 * LINE_CYCLES + self.into
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
    /// The steps of that call's loop done, or [`NO_STEP`].
    pub step: u16,
    /// The vertical blanks the pass had met ([`crate::Pacing::Late`]'s `blanks`).
    pub blanks: u16,
}

/// A [`Cut`] whose call has no loop steps counted: the clock decides within it.
pub const NO_STEP: u16 = 0xffff;

/// A [`Cut`]'s `phase` when the original's frame ended in the level's interrupt (`vbl_handler`'s
/// VBL work or its copper half) with the main loop in its wait, not in a pass (T203): the engine
/// ends the frame there, and the next vertical blank is held until the handler returns
/// ([`super::LevelCtx::irq_due`]).
pub const IRQ_PHASE: u8 = 14;

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
    /// Not a pass but a vertical blank's handler the frame ended in ([`IRQ_PHASE`]): `start` is
    /// the state before the first of `frames`' vertical blank, which runs again from there.
    pub vbl: bool,
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
    /// `level_frame_loop` returned in the step's own frame, the beam on this line (T37).
    pub ended: Option<u16>,
    /// Before a vertical blank whose handler a frame may end in ([`IRQ_PHASE`]): the state, beam
    /// and hardware then, and the frame it is the blank of.
    pub vbl_start: Option<(Box<crate::state::State>, Clock, super::Hw, usize)>,
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
        let per = LINE_CYCLES - FETCH_STOLEN_CYCLES;
        assert_eq!(c.into, FETCH_STOLEN_CYCLES * LINE_CYCLES / per);
    }

    #[test]
    fn a_blit_takes_longer_under_the_display() {
        let mut blank = Clock::START;
        // A line with no fetch leaves the blitter all but the four refresh slots: 223.
        blank.blit(400);
        assert_eq!(blank.line, 1);
        let mut shown = Clock { line: 0x40, into: 0 };
        shown.blit(400);
        assert!(shown.line > 0x42);
        assert_eq!(slots_per_word(0x0fca), 4);
        assert_eq!(slots_per_word(0x09f0), 2);
    }
}
