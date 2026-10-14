//! The level scene (docs/08 "Main loop and interrupt", docs/09): what runs while a level plays,
//! and the object dispatch that items, enemies and bosses register into (T15–T18).
//!
//! The original splits a level's work in two. Every frame the vertical-blank interrupt runs the
//! frog, the camera, the clock, scrolling and sound ([`vbl`]); every second frame the main loop
//! runs one 25 Hz pass over the objects and their drawing ([`pass`]). [`LevelCtx`] is what a
//! ported routine works on: the original's memory as the engine models it, the hardware the frame
//! reads, and the sound driver.

pub mod beam;
pub mod bobs;
pub mod camera;
pub mod clock;
pub mod death;
pub mod exit;
pub mod collision;
pub mod fade;
pub mod frame;
pub mod load;
pub mod objects;
pub mod panel;
pub mod pass;
pub mod scroll;
pub mod spawn;
pub mod tally;
pub mod timeline;

use sf_paula::PaulaWrite;

use crate::audio::SoundData;
use crate::input::{FrameInput, Joy};
use crate::layout::h0;
use crate::mem::Bus;

/// `attract_mode` (h0+$18): -1 while the attract demo runs; sound is silent then.
pub const ATTRACT_MODE: u32 = h0(0x18);

/// Whether a 68000 result with these 16 bits leaves the N flag clear: what `bpl` after a `cmp`
/// or `tst` tests. It is not a signed comparison (overflow is ignored), so the ported code uses it
/// wherever the original branches on N alone.
pub fn n_clear(result: u16) -> bool {
    (result as i16) >= 0
}

/// What the custom chips and CIAs answer this frame: the original reads the joystick and the fire
/// button itself, wherever it needs them (docs/08 "Input"); within a frame they hold still.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Hw {
    /// `JOY1DAT` (`$DFF00C`): the joystick in port 2, as Denise encodes it.
    pub joy1dat: u16,
    /// Port 2's fire button: CIA-A PRA (`$BFE001`) bit 7 reads 0 while it is held.
    pub fire: bool,
    /// CIA-A's serial data register (`$BFEC01`): the keyboard's last byte (the code shifted up
    /// one, bit 0 set for a key going up, all inverted).
    pub sdr: u8,
}

impl Hw {
    pub fn of(input: FrameInput) -> Hw {
        Hw {
            joy1dat: joy1dat(input.joy),
            fire: input.joy.contains(Joy::FIRE),
            sdr: 0xff,
        }
    }

    /// CIA-A PRA as the game tests it: bit 7 low while port 2's fire is held, bit 6 low for
    /// port 1's (the mouse button).
    pub fn ciaa_pra(&self) -> u8 {
        if self.fire { 0x7f } else { 0xff }
    }
}

/// `JOY1DAT` for a joystick held as `j`: right sets bit 1, left bit 9; bit 0 is bit 1 XOR down and
/// bit 8 is bit 9 XOR up (the quadrature counters a joystick's switches drive).
pub fn joy1dat(j: Joy) -> u16 {
    let (r, l) = (j.contains(Joy::RIGHT) as u16, j.contains(Joy::LEFT) as u16);
    let (d, u) = (j.contains(Joy::DOWN) as u16, j.contains(Joy::UP) as u16);
    r << 1 | (r ^ d) | l << 9 | (l ^ u) << 8
}

/// A slot routine of the original's (docs/09 "The slot record"): `slot` is the slot record (`a0`,
/// or `a5` for a touch handler), `d7` what touched it (0 the frog, 1 the spud; ignored by init and
/// update routines).
pub type SlotFn = fn(c: &mut LevelCtx, slot: u32, d7: u16);

/// One kind of object: the original's template (`obj_templates` h0+$1c6e2, `$ae` bytes each) and
/// every routine a slot of the kind may hold, by the original's address. A slot names its routines
/// by those addresses (`+$00` init, `+$04` update, `+$10` touch), as the original's memory has
/// them, so a transplanted slot runs on as it would there.
#[derive(Debug)]
pub struct ObjectKind {
    pub name: &'static str,
    pub routines: &'static [(u32, SlotFn)],
}

/// What a map marker spawns (docs/09 "Placement"): the stub `obj_flag_jumptab` (h0+$178ec)
/// runs for `(world, marker)`, with the cell's address.
pub type MarkerSpawn = fn(lvl: &mut LevelCtx, cell: u32);

/// The object dispatch: object kinds by template index and their routines by address, marker
/// spawners by world and marker, and the frog's reactions that make objects. Items (T15),
/// enemies and bosses (T16–T18) register theirs.
#[derive(Default)]
pub struct Dispatch {
    kinds: Vec<(u16, &'static ObjectKind)>,
    routines: Vec<(u32, SlotFn)>,
    markers: Vec<((u8, u8), MarkerSpawn)>,
    /// A switch the frog touched (markers 48–51, h0+$f6be), with the cell's address: it opens
    /// its gate and turns into its lever. `None`: nothing happens.
    pub bump_block: Option<MarkerSpawn>,
    /// A pickup drawn in the map's tiles that the frog touched (marker 55, h0+$ed8a): fruit,
    /// clocks, lives, power-ups. `None`: nothing happens.
    pub tile_pickup: Option<MarkerSpawn>,
    /// The object of a bounce pad (marker 4 or 5, the cell and the marker) the frog bounced off
    /// (h0+$e9ea). `None`: none spawns.
    pub bounce_pad: Option<fn(&mut LevelCtx, u32, u16)>,
    /// The spring of a launcher (marker 7) that threw the frog: the cell, and whether it threw it
    /// left (h0+$e94a). `None`: none spawns.
    pub launcher: Option<fn(&mut LevelCtx, u32, bool)>,
}

impl std::fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dispatch")
            .field("kinds", &self.kinds.iter().map(|k| (k.0, k.1.name)).collect::<Vec<_>>())
            .field("markers", &self.markers.iter().map(|m| m.0).collect::<Vec<_>>())
            .finish()
    }
}

/// The dispatch a game runs with: every kind and spawner ported so far.
pub fn standard_dispatch() -> &'static Dispatch {
    static D: std::sync::OnceLock<Dispatch> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        let mut d = Dispatch::default();
        crate::items::register_all(&mut d);
        crate::enemies::register_all(&mut d);
        crate::bonus::register_all(&mut d);
        crate::enemies::register_all(&mut d);
        // `game_over_seq`'s sign (T34).
        register(&mut d, 1, &crate::front::GAME_OVER);
        d
    })
}

/// No object kinds at all: the frog plays the level alone (conformance against T14's frog-only
/// cases, whose original has every object frozen out).
pub fn frog_only_dispatch() -> &'static Dispatch {
    static D: std::sync::OnceLock<Dispatch> = std::sync::OnceLock::new();
    D.get_or_init(Dispatch::default)
}

/// Add an object kind under the original's template index.
pub fn register(dispatch: &mut Dispatch, type_id: u16, kind: &'static ObjectKind) {
    dispatch.kinds.retain(|k| k.0 != type_id);
    dispatch.kinds.push((type_id, kind));
    register_routines(dispatch, kind.routines);
}

/// Add slot routines by the original's address alone: ones a slot turns to (a killed enemy's,
/// a reward's) that no template names.
pub fn register_routines(dispatch: &mut Dispatch, routines: &[(u32, SlotFn)]) {
    for &(addr, f) in routines {
        dispatch.routines.retain(|r| r.0 != addr);
        dispatch.routines.push((addr, f));
    }
}

impl Dispatch {
    /// The kind registered under `type_id`.
    pub fn kind(&self, type_id: u16) -> Option<&'static ObjectKind> {
        self.kinds.iter().find(|k| k.0 == type_id).map(|k| k.1)
    }

    /// The ported routine for the original's routine at `addr` (an absolute address).
    pub fn routine(&self, addr: u32) -> Option<SlotFn> {
        self.routines.iter().find(|r| r.0 == addr).map(|r| r.1)
    }

    /// Add the spawner for `marker` (1..64) in world table `world` (`obj_world_offset / $100`:
    /// 0..5 for W1–W6 and the final level, 6 the bonus level).
    pub fn register_marker(&mut self, world: u8, marker: u8, spawn: MarkerSpawn) {
        self.markers.retain(|m| m.0 != (world, marker));
        self.markers.push(((world, marker), spawn));
    }

    /// The spawner for `marker` in world table `world`; `None` while nothing models it.
    pub fn marker(&self, world: u8, marker: u8) -> Option<MarkerSpawn> {
        self.markers
            .iter()
            .find(|m| m.0 == (world, marker))
            .map(|m| m.1)
    }

    /// Every `(world, marker)` with a spawner.
    pub fn markers(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.markers.iter().map(|m| m.0)
    }
}

/// Run the original's slot routine at `addr` on `slot`: the ported one, or, for a routine nothing
/// ports yet, nothing (a debug build stops: every object a game meets has its kind registered).
pub fn call(c: &mut LevelCtx, addr: u32, slot: u32, d7: u16) {
    c.spend(beam::routine_cost(addr));
    match c.dispatch.routine(addr) {
        Some(f) => f(c, slot, d7),
        None => debug_assert!(false, "slot routine ${:x} is not ported", addr.wrapping_sub(h0(0))),
    }
}

/// How a level's loop ended (`level_frame_loop`, then `level_exit_fade` and `game_over_check`),
/// as the game flow is told ([`crate::Game::level_outcome`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelOutcome {
    /// The exit was reached; the tally's result (T38).
    Completed { tally: Tally },
    /// A life was lost: the level starts again from its start point.
    Died,
    /// The clock ran out, which ends the game (`frog_die` takes every life).
    TimeUp,
    /// The last life was lost, or ESC left the game.
    GameOver,
}

/// What the level-complete tally gave (`level_tally`, h0+$4466; T38).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Tally {
    /// Pages of 5 000 points earned (h0+$4796).
    pub pages: u16,
    /// The player took the slot machine instead of the points (h0+$4798).
    pub slot_machine: bool,
}

/// What a ported level routine works on.
pub struct LevelCtx<'a> {
    pub bus: Bus<'a>,
    /// The object kinds and marker spawners registered.
    pub dispatch: &'a Dispatch,
    pub hw: Hw,
    pub sound: &'a SoundData,
    /// The frame's Paula writes, in order.
    pub paula: &'a mut Vec<PaulaWrite>,
    /// Where the beam is as the CPU works ([`beam`]), and what the work costs.
    pub clock: beam::Clock,
    pub costs: [u32; beam::Cost::COUNT],
    /// When kept: the clock's steps, for fitting the costs.
    pub trace: Option<Vec<beam::Tick>>,
    /// While the main program runs on the beam: its frame boundaries ([`frame::main_program`]).
    pub frames: Option<Box<beam::Frames>>,
    /// The Paula writes given the beam's position so far.
    stamped: usize,
    /// In conformance: where the original made each of the step's own frame's Paula writes, as
    /// (line, slot); the clock is set there as the engine makes each ([`crate::Game::anchor_writes`]).
    pub schedule: Vec<(u16, u16)>,
    /// In conformance: where the original started each of the frame's blits (BLTSIZE, line,
    /// slot), and the next to match ([`crate::Game::anchor_blits`]).
    pub blits: Vec<(u16, u16, u16)>,
    blit_next: usize,
    /// A blit ran since the last anchored point.
    blitted: bool,
    /// The last anchored point: where the beam was, and the display writes logged by then.
    anchored_at: Option<(u32, usize)>,
    /// Which of the pass's calls is running (`level_frame_loop`'s, in order: [`frame::pass`]),
    /// and the steps of its loop done (slots looked at; [`LevelCtx::step_done`]).
    pub phase: u8,
    pub pass_step: u16,
    /// The copper interrupt is masked (INTENA `$0010`: the pass's blits mask it), and one has been
    /// requested meanwhile: it is taken once the pass unmasks it.
    coper_masked: bool,
    coper_pending: bool,
    /// `sfx_play`'s INTENA `$c000` after its raster wait with interrupts off, held while a
    /// frame's end in the wait is looked for, and the line its INTENA `$4000` was made on.
    held: Option<(PaulaWrite, u16)>,
}

impl<'a> LevelCtx<'a> {
    /// A routine's context, the beam at the frame's start.
    pub fn new(
        bus: Bus<'a>,
        dispatch: &'a Dispatch,
        hw: Hw,
        sound: &'a SoundData,
        paula: &'a mut Vec<PaulaWrite>,
    ) -> LevelCtx<'a> {
        let stamped = paula.len();
        LevelCtx {
            bus,
            dispatch,
            hw,
            sound,
            paula,
            clock: beam::Clock::START,
            costs: beam::COSTS,
            trace: None,
            frames: None,
            stamped,
            schedule: Vec::new(),
            anchored_at: None,
            blits: Vec::new(),
            blit_next: 0,
            blitted: false,
            phase: 0,
            pass_step: 0,
            coper_masked: false,
            coper_pending: false,
            held: None,
        }
    }
}

impl LevelCtx<'_> {
    /// The Paula writes made since the beam last moved happened where it is.
    fn stamp(&mut self) {
        let fresh = self.stamped < self.paula.len();
        for k in self.stamped..self.paula.len() {
            self.scheduled(k);
            self.paula[k].vpos = self.clock.line;
            self.paula[k].hpos = self.clock.hpos();
            if let Some(t) = self.trace.as_mut() {
                t.push(beam::Tick::Write(k as u32));
            }
        }
        self.stamped = self.paula.len();
        if fresh {
            self.anchor();
        }
    }

    fn tick(&mut self, t: beam::Tick) {
        if let Some(trace) = self.trace.as_mut() {
            trace.push(t);
        }
    }

    /// The pass enters its `phase`th call.
    pub fn enter_phase(&mut self, phase: u8) {
        self.phase = phase;
        self.pass_step = 0;
    }

    /// A step of the pass's loop is done (a slot looked at): the original's frame may end here
    /// ([`beam::Cut::step`]).
    pub fn step_done(&mut self) {
        self.pass_step = self.pass_step.saturating_add(1);
        self.interrupt_due();
    }

    /// The CPU does work of kind `k` here (see [`beam`]).
    pub fn spend(&mut self, k: beam::Cost) {
        self.spend_n(k, 1);
    }

    /// The CPU does `n` pieces of work of kind `k` here.
    pub fn spend_n(&mut self, k: beam::Cost, n: u32) {
        self.interrupt_due();
        self.stamp();
        if n == 0 {
            return;
        }
        self.tick(beam::Tick::Cpu(k, n));
        self.clock.cpu(n * self.costs[k as usize]);
        self.bus.at = self.clock.nominal();
    }

    /// The Paula write with index `k` is being made: in the step's own frame, with a schedule,
    /// the clock is where the original made it.
    fn scheduled(&mut self, k: usize) {
        let Some(f) = self.frames.as_ref() else {
            return;
        };
        if f.at.is_none_or(|a| a + 1 != f.specs.len()) || k < f.out_from {
            return;
        }
        if let Some(&(line, hpos)) = self.schedule.get(k - f.out_from) {
            self.anchor_to(beam::Clock {
                line,
                into: 2 * hpos as u32,
            });
        }
    }

    /// The clock is set to `to`, where the original was at this point: what was written to the
    /// display since the last such point is put between the two as the costs spread it,
    /// stretched to fit.
    fn anchor_to(&mut self, to: beam::Clock) {
        let (now, target) = (self.clock.nominal(), to.nominal());
        // A blit's writes are placed by its own cycles from its start, not stretched.
        let blitted = std::mem::take(&mut self.blitted);
        if let (Some((from, at)), Some(log)) = (self.anchored_at, self.bus.log.as_mut())
            && now > from
            && !blitted
        {
            for w in &mut log.writes[at..] {
                if w.2 >= from {
                    let t = (w.2 - from) as u64 * target.saturating_sub(from) as u64;
                    w.2 = from + (t / (now - from) as u64) as u32;
                }
            }
        }
        self.clock = to;
        self.bus.at = target;
        self.anchored_at = Some((target, self.bus.log.as_ref().map_or(0, |l| l.writes.len())));
    }

    /// In the step's own frame, with a schedule: a blit of `words` words starts where the
    /// original started its next blit of that size.
    fn scheduled_blit(&mut self, words: u32) {
        let Some(f) = self.frames.as_ref() else {
            return;
        };
        if f.at.is_none_or(|a| a + 1 != f.specs.len()) {
            return;
        }
        let size_words = |s: u16| {
            (match s >> 6 { 0 => 1024, h => h as u32 }) * (match s & 0x3f { 0 => 64, w => w as u32 })
        };
        let from = self.blit_next;
        if let Some(j) = (from..self.blits.len().min(from + 3)).find(|&j| size_words(self.blits[j].0) == words) {
            self.blit_next = j + 1;
            let (_, line, hpos) = self.blits[j];
            self.anchor_to(beam::Clock {
                line,
                into: 2 * hpos as u32,
            });
        }
    }

    /// The Paula writes the frame being run has made so far (0 outside the main program).
    pub fn frame_writes_now(&self) -> u16 {
        self.frame_writes().unwrap_or(0)
    }

    /// The Paula writes the frame being run has made.
    fn frame_writes(&self) -> Option<u16> {
        let f = self.frames.as_ref()?;
        f.at?;
        Some(f.pre + (self.paula.len() - f.from) as u16)
    }

    /// After a write: when it is the last the frame's original made before its end
    /// ([`beam::Cut`]), the clock goes from where that one was made.
    fn anchor(&mut self) {
        let Some(n) = self.frame_writes() else {
            return;
        };
        let f = self.frames.as_mut().expect("frames");
        if let Some((_, Some(cut))) = f.current()
            && !f.anchored
            && n >= cut.writes
        {
            f.anchored = true;
            self.clock = beam::Clock {
                line: cut.line,
                into: 2 * cut.hpos as u32,
            };
            self.bus.at = self.clock.nominal();
        }
    }

    /// In the main program's pass: whether the frame ends here, the next vertical blank coming
    /// in the middle of the pass. A frame with a cut ends once it has made the cut's writes and
    /// its clock is past the frame's end, or, `writing`, before it makes one more; one without
    /// ends when its clock is (and the pass then runs on uninterrupted: the frame was on time).
    fn interrupt_due_at(&mut self, writing: bool) {
        let Some(n) = self.frame_writes() else {
            return;
        };
        let f = self.frames.as_ref().expect("frames");
        if !f.in_pass || f.in_vbl || f.done || f.past_end {
            return;
        }
        let past = self.clock.line >= beam::FRAME_LINES;
        let due = match f.current() {
            // In the call the original was in, once the clock says; past it, at once; before
            // it, not yet (unless a write it had not made comes first).
            Some((_, Some(cut))) => {
                let here = self.phase == cut.phase;
                n >= cut.writes
                    && (writing
                        || self.phase > cut.phase
                        || (here && cut.step != beam::NO_STEP && self.pass_step >= cut.step)
                        || (here && cut.step == beam::NO_STEP && past))
            }
            _ => past,
        };
        if due {
            if self.clock.line < beam::FRAME_LINES {
                self.clock = beam::Clock {
                    line: beam::FRAME_LINES,
                    into: 0,
                };
            }
            self.frame_end();
        }
    }

    fn interrupt_due(&mut self) {
        self.interrupt_due_at(false);
    }

    /// The beam reaches the frame's end ([`frame::main_program`]): the next frame's vertical
    /// blank runs (the step's own, or one a pending pass met before), or, after the step's own
    /// frame, the step is over: in the middle of a pass its frame ends in, the state and the pass
    /// are kept for the next step; in the middle of a pass its frame ends without (on time), the
    /// pass runs on uninterrupted.
    pub fn frame_end(&mut self) {
        let f = self.frames.as_mut().expect("the main program's frames");
        let next = f.at.map_or(0, |a| a + 1);
        if let Some((true, None)) = f.current()
            && f.in_pass
        {
            f.past_end = true;
            return;
        }
        if next < f.specs.len() {
            let f = self.frames.as_mut().expect("frames");
            let spec = f.specs[next];
            let own = next + 1 == f.specs.len();
            if f.in_pass {
                f.pass_frames.push(spec);
            }
            f.at = Some(next);
            f.pre = 0;
            f.anchored = false;
            f.in_vbl = true;
            self.stamp();
            let f = self.frames.as_mut().expect("frames");
            f.from = self.paula.len();
            self.clock = self.clock.next_frame();
            self.bus.at = self.clock.nominal();
            if own {
                f.out_from = self.paula.len();
                self.bus.log = Some(crate::mem::WriteLog::default());
            }
            self.hw = spec.hw;
            if let Some((on, off)) = self.held.take() {
                self.interrupts_back(on, off);
            }
            frame::vbl(self);
            let f = self.frames.as_mut().expect("frames");
            f.in_vbl = false;
            return;
        }
        self.stamp();
        let f = self.frames.as_mut().expect("frames");
        f.done = true;
        f.out_to = Some(self.paula.len());
        f.log = self.bus.log.take();
        if let Some((on, _)) = self.held.take() {
            // Past the step's end: the next step runs the pass again.
            self.paula.push(on);
            self.stamped = self.paula.len();
        }
        let f = self.frames.as_mut().expect("frames");
        if f.in_pass {
            f.mid = Some(Box::new(self.bus.st.clone()));
            if let Some((start, clock, hw, pre)) = f.pass_start.clone() {
                f.pending = Some(beam::Pending {
                    start,
                    clock,
                    hw,
                    pre,
                    frames: f.pass_frames.clone(),
                });
            }
        }
    }

    /// A frame ended in `sfx_play`'s raster wait (h0+$12b28), interrupts off since INTENA `$4000`
    /// on line `off`: its vertical blank waits for the wait's end, the first line of the new frame
    /// whose VPOS low byte is three past the one the wait read, and INTENA `$c000` (`on`).
    fn interrupts_back(&mut self, on: PaulaWrite, off: u16) {
        let end = ((off % beam::FRAME_LINES) as u8).wrapping_add(3) as u16;
        if self.clock.line < end {
            self.clock = beam::Clock { line: end, into: 0 };
        }
        self.bus.at = self.clock.nominal();
        self.scheduled(self.paula.len());
        self.paula.push(PaulaWrite {
            hpos: self.clock.hpos(),
            vpos: self.clock.line,
            ..on
        });
        self.stamped = self.paula.len();
        self.tick(beam::Tick::Write(self.stamped as u32 - 1));
        self.tick(beam::Tick::Cpu(beam::Cost::AudioWrite, 1));
        self.clock.cpu(self.costs[beam::Cost::AudioWrite as usize]);
        self.bus.at = self.clock.nominal();
    }

    /// `level_frame_loop` has returned in the step's own frame (T37): the step is over where the
    /// beam is, its frame's rest is the program after the loop's ([`exit`]).
    pub fn loop_over(&mut self) {
        self.stamp();
        let line = self.clock.line;
        let f = self.frames.as_mut().expect("the main program's frames");
        debug_assert!(matches!(f.current(), Some((true, _))), "the loop ends in the step's own frame");
        f.done = true;
        f.ended = Some(line);
        f.out_to = Some(self.paula.len());
        f.log = self.bus.log.take();
    }

    /// The sound driver's writes from `from` on were made one after another, each costing what
    /// it does: a channel stopped (DMACON without its set bit) `stop`, any other `each`.
    pub fn audio_spent(&mut self, from: usize, stop: beam::Cost, each: beam::Cost) {
        self.stamp();
        for k in from..self.paula.len() {
            let w = self.paula[k];
            self.scheduled(k);
            self.paula[k].vpos = self.clock.line;
            self.paula[k].hpos = self.clock.hpos();
            self.tick(beam::Tick::Write(k as u32));
            let c = if w.reg == 0x96 && w.value & 0x8000 == 0 {
                stop
            } else if w.reg == 0x9a && w.value == 0x4000 {
                beam::Cost::AudioMasked
            } else {
                each
            };
            self.tick(beam::Tick::Cpu(c, 1));
            self.clock.cpu(self.costs[c as usize]);
        }
        self.bus.at = self.clock.nominal();
        self.anchor();
    }

    /// h0+$19c9c: with the beam on lines `$1d`..`$21` (VPOS's low byte, compared signed; past
    /// the frame's end, the next frame's lines), the CPU waits for line `$22` before it writes a
    /// sprite's copper words.
    pub fn sprite_wait(&mut self) {
        self.stamp();
        let v = (self.clock.line % beam::FRAME_LINES) as u8;
        if v.wrapping_sub(0x1d) as i8 >= 0 && (v.wrapping_sub(0x22) as i8) < 0 {
            self.wait_lines(0x22 - v as u16);
        }
    }

    /// The CPU waits until the beam has passed `n` more lines (a raster wait).
    pub fn wait_lines(&mut self, n: u16) {
        self.stamp();
        self.tick(beam::Tick::Wait(n));
        self.clock.line = self.clock.line.saturating_add(n);
        self.clock.into = 0;
        self.bus.at = self.clock.nominal();
    }

    /// The CPU waits while the blitter runs `words` words of a blit with BLTCON0 `con0`.
    pub fn blit_words(&mut self, words: u32, con0: u16) {
        self.interrupt_due();
        self.stamp();
        self.scheduled_blit(words);
        let slots = words * beam::slots_per_word(con0);
        self.tick(beam::Tick::Blit(slots));
        let times = self.clock.blit_cycles(words, con0);
        self.blitted = true;
        self.bus.at = self.clock.nominal();
        if !times.is_empty() {
            self.bus.span = Some((times, 0));
        }
    }

    /// `sfx_play` (h0+$12a72): effect `effect` on `channel`, unless the attract demo runs.
    pub fn sfx(&mut self, effect: u8, channel: u8) {
        if self.bus.l(ATTRACT_MODE) == 0 {
            self.spend(beam::Cost::SfxPlay);
            let at = self.paula.len();
            let rest = self.bus.st.audio
                .play_sfx_begin(self.sound, effect, channel, self.paula);
            let Some(rest) = rest else {
                self.audio_spent(at, beam::Cost::AudioStop, beam::Cost::AudioWrite);
                return;
            };
            // The raster wait runs with interrupts off: a frame's end in it holds the vertical
            // blank until INTENA `$c000` ([`LevelCtx::interrupts_back`]).
            let on = self.paula.pop().expect("sfx_play's INTENA $c000");
            self.audio_spent(at, beam::Cost::AudioStop, beam::Cost::AudioWrite);
            let off = self.paula.last().expect("sfx_play's INTENA $4000").vpos;
            self.held = Some((on, off));
            self.interrupt_due_at(true);
            if let Some((on, _)) = self.held.take() {
                let at = self.paula.len();
                self.paula.push(on);
                self.audio_spent(at, beam::Cost::AudioStop, beam::Cost::AudioWrite);
            }
            let at = self.paula.len();
            self.bus.st.audio.play_sfx_rest(self.sound, rest, self.paula);
            self.audio_spent(at, beam::Cost::AudioStop, beam::Cost::AudioWrite);
        }
    }

    /// `sample_play` (h0+$148b8): stream sample `k` on channel 3, unless the attract demo runs.
    pub fn sample(&mut self, k: usize) {
        if self.bus.l(ATTRACT_MODE) == 0 {
            self.spend(beam::Cost::SfxPlay);
            let at = self.paula.len();
            self.bus.st.audio.play_sample(self.sound, k, self.paula);
            self.audio_spent(at, beam::Cost::AudioStop, beam::Cost::AudioWrite);
        }
    }

    /// A write to a custom register the frame comparison records (DMACON, INTENA, INTREQ and the
    /// audio registers): the level's own interrupt and DMA writes go out in order with sound's.
    pub fn custom(&mut self, reg: u16, value: u16) {
        self.stamp();
        self.interrupt_due_at(true);
        self.tick(beam::Tick::Write(self.paula.len() as u32));
        self.scheduled(self.paula.len());
        self.paula.push(PaulaWrite {
            reg,
            value,
            hpos: self.clock.hpos(),
            vpos: self.clock.line,
        });
        self.stamped = self.paula.len();
        self.anchor();
        if reg == 0x9a && value & 0x10 != 0 {
            self.coper_masked = value & 0x8000 == 0;
            if !self.coper_masked && std::mem::take(&mut self.coper_pending) {
                frame::copper_irq(self);
            }
        }
    }

    /// The copper interrupt the scroller requests: taken at once, or once unmasked.
    pub fn request_copper(&mut self) {
        if self.coper_masked {
            self.coper_pending = true;
        } else {
            frame::copper_irq(self);
        }
    }

    /// `rng_range` (h0+$160c0): a number in `0..=n`.
    pub fn rng_range(&mut self, n: u16) -> u16 {
        let mut r = self.bus.st.rng();
        let v = r.range(n);
        self.bus.st.set_rng(r);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joy1dat_encodes_directions_as_denise_does() {
        assert_eq!(joy1dat(Joy::NONE), 0);
        assert_eq!(joy1dat(Joy::RIGHT), 0x0003);
        assert_eq!(joy1dat(Joy::LEFT), 0x0300);
        assert_eq!(joy1dat(Joy::DOWN), 0x0001);
        assert_eq!(joy1dat(Joy::UP), 0x0100);
        assert_eq!(joy1dat(Joy::RIGHT | Joy::DOWN), 0x0002);
        assert_eq!(joy1dat(Joy::LEFT | Joy::UP), 0x0200);
        assert_eq!(Hw::of(FrameInput::joy(Joy::FIRE)).ciaa_pra(), 0x7f);
    }
}
