//! Level 1's frame mount (translator DESIGN §2.9, T36): `game_frame` (`h0+$1fec`) run by the hand
//! engine, `fd_core::Game::game_frame_on`, in place of the translated routine, while the program
//! plays world 1.
//!
//! Each call transplants a [`Game`] from memory ([`Transplant`]: every `LAYOUT` variable, the
//! creatures, cells and display state, what the callees keep outside `LAYOUT`), runs the pass with a
//! [`Pass`] over the bus, and stores what the engine changed back. The pass's outside world goes to
//! the bus as the original's does: the object ports and the creature drawing read memory in place
//! and turn each blit and sound into the original's custom-chip writes, in order, which the
//! machine's blitter and Paula take; the object map and the collision plane they write are read
//! back into the typed state for the callees after them. This is the routine rig's adapter
//! (fd-conform's `tests/game_frame_routine.rs`, T63) on a bus, where it was held to the original on
//! 10 000 states; `fd-judge --seam 'h0+$1fec'` holds this one to the translated routine.
//!
//! The engine's start (level 1's data, the first view) is built once by the host ([`Level1`]) and
//! handed to the routine through `Bus::mount_data`; a run that mounts the frame without it stops.
//! In another world the translated routine runs: the hand engine is level 1's.
//!
//! The pass takes the original's time on the machine (T55, DESIGN §2.9): each custom-chip access is
//! made by the instruction the original makes it with, ticked from `pcode_rt::cycles` at its place
//! in the pass (CLXDAT, JOY1DAT, the raster copy, every blit's wait on BBUSY and its registers,
//! every sound's line wait and registers); the rest of each callee is a lump of its cycles measured
//! on the translated program ([`COSTS`]), ticked a few accesses at a time through the bus's `poll`,
//! so the machine's contention, the frame's end and any interrupt above level 3 fall inside the
//! pass as they do inside the translated routine.
//!
//! What the mount does not do: leave the original's scratch registers (`level3_handler` restores
//! them all around the call).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use amiga_input::FrameInput;
use fd_core::level::collision::MAP;
use fd_core::level::redraw::{Blit, OBJECT_MAP};
use fd_core::level::{LevelCtx, Pass};
use fd_core::objects::{Io, MapObjects, Sound};
use fd_core::render::COLOURS;
use fd_core::render::Display;
use fd_core::render::objects::{CharSprite, CreatureBlit, Creatures, DrawIo};
use fd_core::render::palette::Fade;
use fd_core::render::playfield::{SCREEN as SCREEN_AT, Scroll};
use fd_core::render::status::STATUS;
use fd_core::{Game, GameData, LAYOUT, Mem, MemMut, Scratch, Start, State, Transplant};
use pcode_rt::cycles::{self as cy, Ea, Op, Size};
use pcode_rt::{Bus, Image, Regs, Stop, block_start, insn_start, rts};

use super::BusMem;
use super::rules;
use super::walk::{Decision, Event, Until, Walk};

/// `game_frame`'s entry.
pub const GAME_FRAME: (u8, u32) = (0, 0x1fec);
const GAME_FRAME_AT: (u32, u32) = (GAME_FRAME.0 as u32, GAME_FRAME.1);

/// `read_joystick`, whose branches hang on JOY1DAT ([`rules::after_joy1dat`]).
const READ_JOYSTICK: u32 = 0x8dfc;

/// `world_index` (`h0+$6dfe`, a long): 0 in world 1, where the hand engine plays.
const WORLD_INDEX: u32 = 0x6dfe;

/// The custom chips the pass reads: the blitter's busy bit, the raster, JOY1DAT and CLXDAT (which a
/// read clears).
const DMACONR: u32 = 0xdf_f002;
const VHPOSR: u32 = 0xdf_f006;
const JOY1DAT: u32 = 0xdf_f00c;
const CLXDAT: u32 = 0xdf_f00e;
/// `input_log`, a byte a pass at `input_log_index`, and where it stops.
const INPUT_LOG: u32 = 0x3_4c28;
const LOG_FULL: u32 = 0x2ee0;
/// Hunk offsets the blits and sounds point at: the tiles, `_ELEMENTY` and its masks in hunk 3 and
/// 4, and the three samples in hunk 4 (the rig's, T22, T25).
const TILES: (u32, u32) = (3, 0x1_3540);
const ELEMENTS: (u32, u32) = (4, 0);
const ELEMENT_MASKS: (u32, u32) = (4, 0x1900);
const ITEM_SAMPLE: (u32, u32) = (4, 0x1_2acc);
const TIME_SAMPLE: (u32, u32) = (4, 0x1_3a62);
const SPRING_SAMPLE: (u32, u32) = (4, 0xb548);

/// Level 1's hand engine at its start, as the host builds it once: each pass clones it and
/// transplants the state from memory.
pub struct Level1 {
    base: Game,
    /// The passes the hand engine has played: what shows the mount ran (a watched return does not).
    passes: AtomicU32,
}

impl Level1 {
    pub fn new(data: Arc<GameData>) -> Level1 {
        Level1 {
            base: Game::new(data, Start::Level1),
            passes: AtomicU32::new(0),
        }
    }

    /// The passes the hand engine has played so far.
    pub fn passes(&self) -> u32 {
        self.passes.load(Ordering::Relaxed)
    }
}

/// Memory as the transplant reads it. The play screen and the status bar are picture state the
/// pass never reads (its blits run on the machine), and reading them is most of a pass's time.
struct NoPicture<'m, 'a, 'b>(&'m BusMem<'a, 'b>);

impl Mem for NoPicture<'_, '_, '_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        if (hunk, offset) == (3, SCREEN_AT) || (hunk, offset) == (4, STATUS) {
            return vec![0; len];
        }
        self.0.read(hunk, offset, len)
    }
}

/// `game_frame` mounted: the hand engine's pass in world 1, the translated routine elsewhere.
pub fn game_frame(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let world = bus.rl(img.h(0, WORLD_INDEX));
    if world != 0 {
        return translated(r, bus, img);
    }
    let Some(level) = bus.mount_data().and_then(|d| d.downcast_ref::<Level1>()) else {
        return Err(Stop::Unsupported {
            op: "seam: game_frame mounted with no level 1 engine (TranslatedGame::mount_level_1)"
                .into(),
            at: img.describe(img.h(0, GAME_FRAME.1)),
        });
    };
    level.passes.fetch_add(1, Ordering::Relaxed);
    let mut before = level.base.clone();
    let mut mem = BusMem::new(bus, img);
    before.transplant(&NoPicture(&mem));
    let mut g = before.clone();
    let stop = {
        let mut p = BusPass {
            mem: &mut mem,
            img,
            at: 0,
            r: &mut *r,
            owed: 0,
            walk: Walk::new(GAME_FRAME_AT, crate::tree::shapes),
            walked: false,
            log_full: false,
            stop: None,
        };
        // CLXDAT is read at `char_hit_check`, through `Pass::clxdat`.
        g.game_frame_on(FrameInput::NONE, 0, &mut p);
        p.finish();
        p.stop
    };
    if let Some(e) = stop {
        return Err(e);
    }
    store(&mut mem, &before, &g);
    rts(r, mem.bus());
    Ok(())
}

/// The routine at hunk 0 `offset` timed alone by its rule (translator T56): the level transplanted
/// from memory as the frame mount does, the rule's path walked with every custom write writing 0,
/// and the routine left as its `rts` leaves. What the rules' check holds to the translated routine's
/// counted cycles (`fd-conform/tests/mount_timing.rs`); it changes no memory.
pub fn time_alone(offset: u32, r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let at = || img.describe(img.h(0, offset));
    let Some(level) = bus.mount_data().and_then(|d| d.downcast_ref::<Level1>()) else {
        return Err(Stop::Unsupported {
            op: "seam: a timing rule run with no level 1 engine".into(),
            at: at(),
        });
    };
    let mut g = level.base.clone();
    g.transplant(&NoPicture(&BusMem::new(bus, img)));
    let mem = BusMem::new(bus, img);
    let peek = |h: u32, o: u32| mem.read_at(img.h(h, o));
    let path = g.level(|l| rules::path(offset, l, &peek));
    drop(mem);
    let Some(path) = path else {
        return Err(Stop::Unsupported {
            op: "seam: no timing rule".into(),
            at: at(),
        });
    };
    let mut w = Walk::new((0, offset), crate::tree::shapes);
    w.decide(path);
    w.free_writes();
    if offset == READ_JOYSTICK {
        let full = g.level(rules::log_full);
        if let Event::Read(_, v) = w.run(bus, r, img, Until::Read(0x00c))? {
            w.decide(rules::after_joy1dat(v, full));
        }
    }
    w.run(bus, r, img, Until::End)?;
    let (d, _) = w.left();
    if d > 0 {
        return Err(Stop::Unsupported {
            op: format!("seam: the rule left {d} decisions the walk did not take"),
            at: at(),
        });
    }
    rts(r, bus);
    Ok(())
}

/// The translated `game_frame`, entered past its mount check.
fn translated(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let k = (GAME_FRAME.0 as u32, GAME_FRAME.1);
    match crate::tree::resume(k) {
        Some(f) => f(k, r, bus, img),
        None => Err(Stop::Untranslated(img.describe(img.h(0, GAME_FRAME.1)))),
    }
}

fn at(img: &Image, (hunk, off): (u32, u32)) -> u32 {
    img.h(hunk, off)
}

/// [`Pass`] over the bus.
struct BusPass<'m, 'a, 'b> {
    mem: &'m mut BusMem<'a, 'b>,
    img: &'m Image<'m>,
    /// The routine running, as the last [`Pass::call`] named it.
    at: u32,
    r: &'m mut Regs,
    /// Of the routine running, the cycles of its [`COSTS`] not yet ticked.
    owed: u32,
    /// `game_frame` through the original's instructions (T56): its own, and those of each routine
    /// with a timing rule ([`rules`]); a routine with none is a lump of its [`COSTS`].
    walk: Walk,
    /// The routine running is walked.
    walked: bool,
    /// `read_joystick` found `input_log` full ([`rules::log_full`]).
    log_full: bool,
    /// What stopped the run inside the pass (a hang, a fault the bus reported): the pass ticks
    /// nothing more and the mount returns it.
    stop: Option<Stop>,
}

impl BusPass<'_, '_, '_> {
    fn h0(&self, a: u32) -> u32 {
        self.img.h(0, 0).wrapping_add(a)
    }
}

// --- the time -------------------------------------------------------------------------------------

/// What each callee of `game_frame` costs, in cycles of the 68000 with no chip holding the bus: its
/// most common time from entry to return on the translated program, the waits for DMA slots, for
/// BBUSY and for the beam taken out (`fd-conform`'s `examples/game_frame_costs.rs` over the level 1
/// cases, 2026-11-19), and what each blit it starts adds. The instructions the mount makes its own
/// accesses with are paid out of it; what is left is ticked as a lump (module comment).
struct Cost {
    at: u32,
    base: u32,
    per_blit: u32,
}

const COSTS: [Cost; 20] = [
    Cost {
        at: 0x9b06,
        base: 3232,
        per_blit: 0,
    },
    // `h0_94ba` with the fade off; [`FADE_ON`] while it runs.
    Cost {
        at: 0x94ba,
        base: 72,
        per_blit: 0,
    },
    Cost {
        at: 0x4d7e,
        base: 1240,
        per_blit: 0,
    },
    Cost {
        at: 0x513c,
        base: 128,
        per_blit: 0,
    },
    Cost {
        at: 0x5462,
        base: 434,
        per_blit: 0,
    },
    Cost {
        at: 0x9754,
        base: 1272,
        per_blit: 0,
    },
    Cost {
        at: 0x2c7e,
        base: 608,
        per_blit: 306,
    },
    Cost {
        at: 0x2fae,
        base: 1414,
        per_blit: 0,
    },
    Cost {
        at: 0x21c0,
        base: 44,
        per_blit: 1060,
    },
    Cost {
        at: 0x62b6,
        base: 1030,
        per_blit: 1070,
    },
    Cost {
        at: 0x2dd6,
        base: 1142,
        per_blit: 0,
    },
    Cost {
        at: 0x3698,
        base: 1546,
        per_blit: 0,
    },
    Cost {
        at: 0x2d4c,
        base: 716,
        per_blit: 840,
    },
    Cost {
        at: 0x8dfc,
        base: 456,
        per_blit: 0,
    },
    Cost {
        at: 0x3b2a,
        base: 98,
        per_blit: 0,
    },
    Cost {
        at: 0x20b0,
        base: 92,
        per_blit: 0,
    },
    Cost {
        at: 0x24ce,
        base: 44,
        per_blit: 0,
    },
    Cost {
        at: 0x3e0c,
        base: 15968,
        per_blit: 0,
    },
    Cost {
        at: 0x2698,
        base: 600,
        per_blit: 1000,
    },
    Cost {
        at: 0x2104,
        base: 220,
        per_blit: 0,
    },
];

/// `h0_94ba` while the fade runs (`h0+$94b4` set as the pass starts).
const FADE_ON: u32 = 13550;
const FADE_FLAG: u32 = 0x94b4;

/// A lump is ticked this many cycles at a time, a block start before each, so the frame's end and
/// an interrupt are taken within a few accesses of where the translated code takes them.
const LUMP: u32 = 40;

/// A wait loop on the bus gives up after this many turns (a bus with no beam or no blitter).
const WAIT_GUARD: u32 = 100_000;

fn cost(at: u32) -> Option<&'static Cost> {
    COSTS.iter().find(|c| c.at == at)
}

/// `move.w #imm, abs.l` and `move.l #imm, abs.l`: how the original writes the chips' registers.
const MOVE_W: (u32, u32) = (
    cy::of(Op::Move, Size::W, Ea::Imm, Ea::AbsL),
    cy::off_of(Op::Move, Size::W, Ea::Imm, Ea::AbsL),
);
const MOVE_L: (u32, u32) = (
    cy::of(Op::Move, Size::L, Ea::Imm, Ea::AbsL),
    cy::off_of(Op::Move, Size::L, Ea::Imm, Ea::AbsL),
);

impl BusPass<'_, '_, '_> {
    fn bus(&mut self) -> &mut dyn Bus {
        self.mem.bus()
    }

    /// A block's start, as the translated code makes one: where a due interrupt or the frame's end
    /// is taken.
    fn block(&mut self) {
        if self.stop.is_none()
            && let Err(e) = block_start(self.mem.bus(), self.r, self.img)
        {
            self.stop = Some(e);
        }
    }

    /// One instruction: `cycles`, `off` of them off the bus (before its accesses for a branch),
    /// ticked and fitted onto the bus before the access the caller then makes. Paid out of the
    /// routine's cost unless it is a wait (`paid` false).
    fn insn(&mut self, (cycles, off): (u32, u32), lead: u32, paid: bool) {
        if self.stop.is_some() {
            return;
        }
        if let Err(e) = insn_start(self.mem.bus(), self.r, self.img) {
            self.stop = Some(e);
            return;
        }
        self.r.tick(cycles);
        self.r.off_bus(off);
        if lead > 0 {
            self.r.idle_first(lead);
        }
        self.mem.bus().sync(self.r);
        if paid {
            self.owed = self.owed.saturating_sub(cycles);
        }
    }

    /// `n` cycles of work with no access the pass sees, as bus accesses of the CPU's.
    fn spend(&mut self, mut n: u32) {
        while n > 0 && self.stop.is_none() {
            let k = n.min(LUMP);
            self.block();
            self.r.tick(k);
            n -= k;
        }
    }

    /// The rest of the routine running.
    fn settle(&mut self) {
        let n = std::mem::take(&mut self.owed);
        self.spend(n);
    }

    /// The pass's end: the last routine's rest and `game_frame`'s own instructions to its `rts`.
    fn finish(&mut self) {
        if !self.walked {
            self.settle();
        }
        if let Some(e) = self.walk_to(Until::End)
            && e != Event::Done
        {
            self.stop = Some(self.walk_stop("the walk ended before game_frame's return"));
        }
    }

    /// The walk on to `until`; `None` once the pass has stopped.
    fn walk_to(&mut self, until: Until) -> Option<Event> {
        if self.stop.is_some() {
            return None;
        }
        match self.walk.run(self.mem.bus(), self.r, self.img, until) {
            Ok(e) => Some(e),
            Err(e) => {
                self.stop = Some(e);
                None
            }
        }
    }

    fn walk_stop(&self, why: &str) -> Stop {
        Stop::Unsupported {
            op: format!("seam: {why}"),
            at: self.img.describe(self.img.h(0, self.at)),
        }
    }

    /// The custom writes of a walked routine's blit or sound, made by the original's instructions
    /// as the walk meets them: on to the next write the engine has not made yet, or the routine's
    /// return.
    fn walk_writes(&mut self, writes: &[(u32, u32, bool)]) {
        for &(a, v, long) in writes {
            self.walk.write(a as u16, v, if long { 4 } else { 2 });
        }
        self.walk_to(Until::Write);
    }

    /// A custom register read by a walked routine's instruction: its value.
    fn walk_read(&mut self, reg: u16) -> u16 {
        match self.walk_to(Until::Read(reg)) {
            Some(Event::Read(_, v)) => v,
            Some(_) => {
                self.stop =
                    Some(self.walk_stop(&format!("the walk made no read of $dff{reg:03x}")));
                0
            }
            None => 0,
        }
    }

    /// `n` of the routine's cycles, before an access the pass makes.
    fn pay(&mut self, n: u32) {
        let n = n.min(self.owed);
        self.owed -= n;
        self.spend(n);
    }

    /// A custom register written by `move.w #imm` or (`long`) `move.l #imm`.
    fn write(&mut self, a: u32, v: u32, long: bool) {
        self.insn(if long { MOVE_L } else { MOVE_W }, 0, true);
        let m = self.bus();
        if long {
            m.wl(a, v);
        } else {
            m.ww(a, v as u16);
        }
    }

    /// A blit: the routine's work towards it, the wait on BBUSY (`btst #6, $dff002` then `bne`,
    /// every blit site's), then its registers in order.
    fn blit(&mut self, writes: &[(u32, u32, bool)]) {
        if self.walked {
            return self.walk_writes(writes);
        }
        let per = cost(self.at).map_or(0, |c| c.per_blit);
        let btst = (
            cy::of(Op::Btst, Size::B, Ea::Imm, Ea::AbsL),
            cy::off_of(Op::Btst, Size::B, Ea::Imm, Ea::AbsL),
        );
        let own: u32 = btst.0
            + cy::bcc(false, Size::W)
            + writes
                .iter()
                .map(|w| if w.2 { MOVE_L.0 } else { MOVE_W.0 })
                .sum::<u32>();
        self.owed += per;
        self.pay(per.saturating_sub(own));
        for turn in 0..WAIT_GUARD {
            if turn > 0 {
                self.block();
            }
            self.insn(btst, 0, turn == 0);
            let busy = self.bus().rb(DMACONR) & 0x40 != 0;
            let b = (cy::bcc(busy, Size::W), cy::bcc_off(busy));
            self.insn(b, b.1, turn == 0);
            if !busy || self.stop.is_some() {
                break;
            }
        }
        for &(a, v, long) in writes {
            self.write(a, v, long);
        }
    }

    /// A sound's wait for `lines` changes of VHPOSR's high byte: `moveq #lines-1, d0`, then
    /// `move.b $dff006, d1`, `cmp.b $dff006, d1` and `beq` until it changes, `dbra d0`. Every sound
    /// routine of the pass waits so between turning its channel off and setting it up (`h0+$2394`,
    /// `item_sound`, `h0+$28fe`, `h0_3adc`, `h0+$3d8e`, `h0+$3f2c`, `h0+$4004`).
    fn line_wait(&mut self, lines: u16) {
        let moveq = cy::of(Op::Moveq, Size::L, Ea::Imm, Ea::Dn);
        self.insn((moveq, 0), 0, false);
        let read = (
            cy::of(Op::Move, Size::B, Ea::AbsL, Ea::Dn),
            cy::off_of(Op::Move, Size::B, Ea::AbsL, Ea::Dn),
        );
        let cmp = (
            cy::of(Op::Cmp, Size::B, Ea::AbsL, Ea::Dn),
            cy::off_of(Op::Cmp, Size::B, Ea::AbsL, Ea::Dn),
        );
        for left in (0..lines).rev() {
            self.block();
            self.insn(read, 0, false);
            let b = self.bus().rb(VHPOSR);
            for _ in 0..WAIT_GUARD {
                self.block();
                self.insn(cmp, 0, false);
                let same = self.bus().rb(VHPOSR) == b;
                let br = (cy::bcc(same, Size::W), cy::bcc_off(same));
                self.insn(br, br.1, false);
                if !same || self.stop.is_some() {
                    break;
                }
            }
            let d = (cy::dbcc(false, left), cy::dbcc_off(false));
            self.insn(d, d.1, false);
            if self.stop.is_some() {
                return;
            }
        }
    }

    /// A sound: its channel off, the wait, its registers (the sample's location a `move.l`), on.
    /// Writes as `(register, word)`; a DMACON write clearing a channel starts the wait, three lines
    /// for channel 1, four for channels 2 and 3 (each routine's `moveq`).
    fn sound_writes(&mut self, hw: &[(u16, u16)]) {
        if self.walked {
            let mut writes = Vec::new();
            let mut i = 0;
            while i < hw.len() {
                let (reg, v) = hw[i];
                let a = 0xdf_f000 + reg as u32;
                if let Some(&(next, lo)) = hw.get(i + 1)
                    && next == reg + 2
                    && matches!(reg & 0xf, 0x0)
                    && (0xa0..0xe0).contains(&reg)
                {
                    writes.push((a, (v as u32) << 16 | lo as u32, true));
                    i += 2;
                } else {
                    writes.push((a, v as u32, false));
                    i += 1;
                }
            }
            return self.walk_writes(&writes);
        }
        let mut i = 0;
        while i < hw.len() {
            let (reg, v) = hw[i];
            let a = 0xdf_f000 + reg as u32;
            // A location's high word then its low word: one `move.l`.
            if let Some(&(next, lo)) = hw.get(i + 1)
                && next == reg + 2
                && matches!(reg & 0xf, 0x0)
                && (0xa0..0xe0).contains(&reg)
            {
                self.write(a, (v as u32) << 16 | lo as u32, true);
                i += 2;
                continue;
            }
            self.write(a, v as u32, false);
            if reg == 0x96 && v & 0x8000 == 0 {
                self.line_wait(if v & 0x2 != 0 { 3 } else { 4 });
            }
            i += 1;
        }
    }
}

/// The object ports' memory, as the routine last called reaches it.
struct ObjIo<'p, 'm, 'a, 'b> {
    p: &'p mut BusPass<'m, 'a, 'b>,
    cells: u32,
    /// The object map bytes and collision longs written, for the typed state after.
    map: Vec<(u32, u8)>,
    plane: Vec<(u32, u32)>,
}

impl Io for ObjIo<'_, '_, '_, '_> {
    fn rb(&self, a: u32) -> u8 {
        let a = self.p.h0(a);
        self.p.mem.read_at(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        let at = self.p.h0(a);
        self.p.mem.bus().wb(at, v);
        self.map.push((a, v));
    }
    fn blit(&mut self, b: Blit) {
        let img = self.p.img;
        let screen = img.h(3, SCREEN_AT);
        let every = matches!(self.p.at, 0x2698 | 0x2dd6);
        let first = self.cells == 0;
        let (w, l) = (|a, v: u32| (a, v, false), |a, v: u32| (a, v, true));
        let writes: Vec<(u32, u32, bool)> = match b {
            Blit::Cell { src, dst } => {
                // `h0_21c0` and `h0_62b6` set the modulos and masks for their first cell only,
                // `h0_29ca` (under `char_object_cells` and `h0_2dd6`) for every one.
                self.cells += 1;
                let mut v = vec![
                    l(0xdf_f054, screen.wrapping_add(dst)),
                    l(0xdf_f050, at(img, TILES).wrapping_add(src)),
                ];
                if every || first {
                    v.extend([
                        w(0xdf_f066, 0x2c),
                        w(0xdf_f064, 0),
                        l(0xdf_f044, 0xffff_ffff),
                        w(0xdf_f040, 0x09f0),
                        w(0xdf_f042, 0),
                    ]);
                }
                v.push(w(0xdf_f058, 0x1401));
                v
            }
            Blit::Picture { src, modulo, dst } => vec![
                l(0xdf_f054, screen.wrapping_add(dst)),
                l(0xdf_f050, img.h(4, 0).wrapping_add(src)),
                w(0xdf_f066, 0x2a),
                w(0xdf_f064, modulo as u32),
                l(0xdf_f044, 0xffff_ffff),
                w(0xdf_f040, 0x09f0),
                w(0xdf_f042, 0),
                w(0xdf_f058, 0x1402),
            ],
            Blit::Element { frame, tile, dst } => vec![
                w(0xdf_f066, 0x2c),
                w(0xdf_f064, 0x26),
                w(0xdf_f062, 0x26),
                w(0xdf_f060, 0),
                l(0xdf_f044, 0xffff_ffff),
                w(0xdf_f040, 0x0fca),
                w(0xdf_f042, 0),
                l(0xdf_f054, screen.wrapping_add(dst)),
                l(0xdf_f050, at(img, ELEMENT_MASKS).wrapping_add(2 * frame)),
                l(0xdf_f04c, at(img, ELEMENTS).wrapping_add(2 * frame)),
                l(0xdf_f048, at(img, TILES).wrapping_add(tile)),
                w(0xdf_f058, 0x1401),
            ],
            Blit::Tile { .. } | Blit::Bob { .. } => {
                unreachable!(
                    "only the edge redraws (main_loop's, translated) start tile and bob blits"
                )
            }
        };
        self.p.blit(&writes);
    }
    fn sound(&mut self, s: Sound) {
        // DMACON off for the channel, the wait, its location, length, period and volume, DMACON on.
        let (ch, sample, len, period) = match s {
            Sound::Item => (2u16, ITEM_SAMPLE, 0x7cb, 0x165),
            Sound::Time => (3, TIME_SAMPLE, 0x1c4f, 0x1ff),
            Sound::Spring => (1, SPRING_SAMPLE, 0x9c4, 0x1bf),
        };
        let sample = at(self.p.img, sample);
        let aud = 0xa0 + 0x10 * ch;
        self.p.sound_writes(&[
            (0x96, 1 << ch),
            (aud, (sample >> 16) as u16),
            (aud + 2, sample as u16),
            (aud + 4, len),
            (aud + 6, period),
            (aud + 8, 0x3f),
            (0x96, 0x8200 | 1 << ch),
        ]);
    }
    fn collision(&mut self, off: u32, v: u32) {
        let a = self.p.img.h(3, MAP).wrapping_add(off);
        self.p.mem.bus().wl(a, v);
        self.plane.push((off, v));
    }
}

/// The creature drawing's memory.
struct DrawBus<'p, 'm, 'a, 'b>(&'p mut BusPass<'m, 'a, 'b>);

impl DrawIo for DrawBus<'_, '_, '_, '_> {
    fn rb(&self, a: u32) -> u8 {
        let a = self.0.h0(a);
        self.0.mem.read_at(a)
    }
    fn blit(&mut self, b: CreatureBlit) {
        let (w, l) = (|a, v: u32| (a, v, false), |a, v: u32| (a, v, true));
        match b {
            // The cut's registers set once for the pass's cuts: no blit starts, so no wait.
            CreatureBlit::CutSetup => {
                let setup = [
                    w(0xdf_f066, 0x28),
                    w(0xdf_f064, 0x22),
                    w(0xdf_f062, 0x22),
                    w(0xdf_f060, 0x28),
                    l(0xdf_f044, 0xffff_0000),
                ];
                if self.0.walked {
                    return self.0.walk_writes(&setup);
                }
                for (a, v, long) in setup {
                    self.0.write(a, v, long);
                }
            }
            CreatureBlit::Restore { src, dst, first } => {
                let mut v = vec![l(0xdf_f054, dst), l(0xdf_f050, src)];
                if first {
                    v.extend([
                        w(0xdf_f066, 0x2c),
                        w(0xdf_f064, 0),
                        l(0xdf_f044, 0xffff_ffff),
                        w(0xdf_f040, 0x09f0),
                        w(0xdf_f042, 0),
                    ]);
                }
                v.push(w(0xdf_f058, 0x1401));
                self.0.blit(&v);
            }
            CreatureBlit::Cut {
                mask,
                picture,
                dst,
                con0,
                con1,
                size,
            } => self.0.blit(&[
                l(0xdf_f054, dst),
                l(0xdf_f050, mask),
                l(0xdf_f04c, picture),
                l(0xdf_f048, dst),
                w(0xdf_f040, con0 as u32),
                w(0xdf_f042, con1 as u32),
                w(0xdf_f058, size as u32),
            ]),
        }
    }
}

impl Pass for BusPass<'_, '_, '_> {
    fn enter(&mut self, offset: u32, l: &LevelCtx) {
        if !self.walked {
            self.settle();
        }
        // `game_frame`'s one branch of its own: `tst.w paused; beq` past `play_keys`.
        if self.at == 0x9754 {
            self.walk.decide([Decision::Branch(l.state.paused == 0)]);
        }
        // The routine before's rest and `game_frame`'s instructions up to the `bsr`.
        match self.walk_to(Until::Call(offset)) {
            Some(Event::Call) => {}
            Some(_) => {
                self.stop = Some(
                    self.walk_stop(&format!("game_frame makes no call of h0+${offset:x} here")),
                );
            }
            None => {}
        }
        self.at = offset;
        if offset == READ_JOYSTICK {
            self.log_full = rules::log_full(l);
        }
        let path = {
            let (mem, img) = (&*self.mem, self.img);
            rules::path(offset, l, &|h, o| mem.read_at(img.h(h, o)))
        };
        match path {
            Some(d) => {
                self.walk.decide(d);
                self.walked = true;
                self.owed = 0;
            }
            None => {
                if self.stop.is_none()
                    && let Err(e) = self.walk.skip_call(self.mem.bus(), self.r, self.img)
                {
                    self.stop = Some(e);
                }
                self.walked = false;
                self.owed = match cost(offset) {
                    Some(_) if offset == 0x94ba && self.mem.read_at(self.h0(FADE_FLAG)) != 0 => {
                        FADE_ON
                    }
                    Some(c) => c.base,
                    None => 0,
                };
            }
        }
    }
    fn objects(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut MapObjects)) {
        let mut io = ObjIo {
            p: self,
            cells: 0,
            map: Vec::new(),
            plane: Vec::new(),
        };
        f(&mut MapObjects {
            state: &mut *l.state,
            io: &mut io,
        });
        // What the callees after read from the typed state, the object map and the plane, as
        // `objects::PlayIo` keeps them.
        let map = &mut l.state.object_map;
        for (a, v) in io.map {
            if let Some(b) = map.get_mut(a.wrapping_sub(OBJECT_MAP) as usize) {
                *b = v;
            }
        }
        for (at, v) in io.plane {
            l.plane.set_word(at, (v >> 16) as u16);
            l.plane.set_word(at.wrapping_add(2), v as u16);
        }
    }
    fn creatures(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut Creatures)) {
        let s = &mut *l.state;
        let camera_y = s.camera.y;
        f(&mut Creatures {
            draw: &mut s.creature_draw,
            slots: &mut s.edges.creatures,
            camera_y,
            io: &mut DrawBus(self),
        });
    }
    fn joy1dat(&mut self, _: &FrameInput) -> u16 {
        if self.walked {
            let v = self.walk_read(0x00c);
            self.walk.decide(rules::after_joy1dat(v, self.log_full));
            return v;
        }
        // `read_joystick` clears its four words (`move.w #0, abs.l`), then reads JOY1DAT.
        self.pay(4 * cy::of(Op::Move, Size::W, Ea::Imm, Ea::AbsL));
        self.insn(
            (
                cy::of(Op::Move, Size::W, Ea::AbsL, Ea::Dn),
                cy::off_of(Op::Move, Size::W, Ea::AbsL, Ea::Dn),
            ),
            0,
            true,
        );
        self.bus().rw(JOY1DAT)
    }
    fn clxdat(&mut self, _: u16) -> u16 {
        if self.walked {
            return self.walk_read(0x00e);
        }
        // `char_hit_check`'s first instruction, `move.w $dff00e, clxdat_copy`.
        self.insn(
            (
                cy::of(Op::Move, Size::W, Ea::AbsL, Ea::AbsL),
                cy::off_of(Op::Move, Size::W, Ea::AbsL, Ea::AbsL),
            ),
            0,
            true,
        );
        self.bus().rw(CLXDAT)
    }
    fn scratch(&mut self) -> Scratch {
        // The beam's value only counts the reads (`Scratch::vhposr` steps it): no routine on it
        // keeps a VHPOSR byte past its own wait loop. Its reads are made on the bus at the end.
        Scratch::default()
    }
    fn scratch_done(&mut self, s: Scratch) {
        // The routine's custom writes are its sounds' (the hurt, stomp and slide sounds), each with
        // its line wait made on the bus. `s.beam` counted the slide's reads on the engine's own
        // beam; the wait here reads the machine's VHPOSR in its place, as often as the original.
        self.sound_writes(&s.hw);
    }
    fn scroll(&mut self, camera_x: u16, camera_y: u16) {
        for (off, bytes) in Scroll::from_camera(camera_x, camera_y).copper() {
            self.mem.write(1, off, &bytes);
        }
    }
    fn raster_copy(&mut self) {
        // After `h0_2d4c`'s `rts`: `game_frame`'s `move.w $dff006, h0+$8dfa`, walked.
        if !self.walked {
            self.settle();
        }
        let v = self.walk_read(0x006);
        let a = self.h0(0x8dfa);
        self.mem.bus().ww(a, v);
    }
    fn shown(&mut self, d: &Display) {
        // The fade and the sprites as they stand (the original writes them every pass), before the
        // frame's end the pass usually runs past, so the copper's next list and the sprite DMA of
        // the lines after the pass's start read them as the original leaves them.
        let Fade {
            on,
            pass,
            now,
            scratch,
        } = d.fade;
        let mem = &mut *self.mem;
        mem.put(0, 0x94b4, &[on]);
        mem.put(0, 0x94b6, &pass.to_be_bytes());
        for (i, c) in now.iter().enumerate() {
            mem.put(1, COLOURS + 4 * i as u32, &c.to_be_bytes());
        }
        for (i, w) in scratch.iter().enumerate() {
            mem.put(0, 0x961e + 2 * i as u32, &w.to_be_bytes());
        }
        let s: &CharSprite = &d.sprite;
        let writes = {
            let m = &*mem;
            let frame = |a: u32, n: u32| m.read_bytes(a, n);
            s.bytes(&frame)
        };
        for (off, b) in writes {
            mem.put(1, off, &b);
        }
    }
}

// --- the store back -------------------------------------------------------------------------------

/// What the pass keeps in the typed state outside `LAYOUT`, as (hunk, offset, bytes).
fn extras(s: &State) -> Vec<(u8, u32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut w = |o: u32, v: &[u8]| out.push((0, o, v.to_vec()));
    let c = &s.camera;
    w(0x4d76, &c.screen_x.to_be_bytes());
    w(0x4d7a, &c.screen_y.to_be_bytes());
    let crossed = c.edges.crossed.map(|x| if x { 0xff } else { 0 });
    w(0x51f8, &crossed);
    w(0x519e, &s.frame_flag.to_be_bytes());
    w(0x2a6a, &s.collect.score_add.to_be_bytes());
    w(0x2692, &s.collect.row.to_be_bytes());
    w(0x29c6, &s.collect.taken.to_be_bytes());
    w(0x2fe6, &s.moving.slot.to_be_bytes());
    w(0x306e, &s.moving.stomped.to_be_bytes());
    let cs = &s.edges.creatures;
    w(0x2faa, &cs.index.to_be_bytes());
    w(0x67da, &cs.active);
    for (i, v) in cs.slots.iter().enumerate() {
        let mut b = Vec::new();
        b.extend(v.x.to_be_bytes());
        b.extend(v.y.to_be_bytes());
        b.extend(v.kind.to_be_bytes());
        b.extend([v.facing, v.phase]);
        w(0x67ec + 8 * i as u32, &b);
    }
    w(0x581c, &cs.current.to_be_bytes());
    w(0x687e, &cs.last.to_be_bytes());
    let d = &s.creature_draw;
    w(0x2c7a, &d.restore_index.to_be_bytes());
    w(0x2c7c, &d.cut_index.to_be_bytes());
    w(0x5818, &d.x.to_be_bytes());
    w(0x581a, &d.y.to_be_bytes());
    w(0x581e, &d.restoring.to_be_bytes());
    w(0x67e0, &d.saved);
    w(0x67e6, &d.drawn);
    for n in 0..48usize {
        let (g, i) = (n / 12, n % 12);
        let e = s.edges.cells.cells[g][i];
        out.push((2, 0x1680 + n as u32, vec![s.edges.cells.used[g][i]]));
        let mut b = Vec::new();
        b.extend(e.x.to_be_bytes());
        b.extend(e.y.to_be_bytes());
        b.push(e.code);
        b.extend(e.rest);
        out.push((2, 0x16b0 + 8 * n as u32, b));
    }
    out
}

/// Store what the engine changed: every `LAYOUT` variable and extra it moved and the input log's
/// byte (the fade and the sprites are written inside the pass, `Pass::shown`).
fn store(mem: &mut BusMem, before: &Game, after: &Game) {
    for v in LAYOUT {
        let (was, now) = (before.probe(v), after.probe(v));
        if was != now {
            mem.put_var(v, now);
        }
    }
    let (s0, s1) = (before.state(), after.state());
    for ((h, o, was), (_, _, now)) in extras(s0).into_iter().zip(extras(s1)) {
        if was != now {
            mem.put(h, o, &now);
        }
    }
    let i = s0.input_log.index;
    if s1.input_log.index != i && i < LOG_FULL {
        mem.put(0, INPUT_LOG + i, &[s1.input_log.byte(i)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plain memory over hunk 0 at its base, no mount data.
    struct Ram(Vec<u8>);

    impl Bus for Ram {
        fn rb(&mut self, a: u32) -> u8 {
            self.0[(a - fd_core::HUNK_BASES[0]) as usize]
        }
        fn rw(&mut self, a: u32) -> u16 {
            u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
        }
        fn rl(&mut self, a: u32) -> u32 {
            (self.rw(a) as u32) << 16 | self.rw(a + 2) as u32
        }
        fn wb(&mut self, _: u32, _: u8) {}
        fn ww(&mut self, _: u32, _: u16) {}
        fn wl(&mut self, _: u32, _: u32) {}
    }

    fn run(world: u32) -> Result<(), Stop> {
        let mut ram = Ram(vec![0; 0x8000]);
        let at = WORLD_INDEX as usize;
        ram.0[at..at + 4].copy_from_slice(&world.to_be_bytes());
        let sizes = [0x8000];
        let img = Image {
            bases: &fd_core::HUNK_BASES,
            sizes: &sizes,
            code: &[],
        };
        game_frame(&mut Regs::new(), &mut ram, &img)
    }

    #[test]
    fn in_world_1_with_no_engine_handed_to_it_the_mount_stops_naming_it() {
        let e = run(0).unwrap_err().to_string();
        assert!(e.contains("no level 1 engine"), "{e}");
    }

    #[test]
    fn in_another_world_the_translated_routine_runs() {
        if crate::TranslatedGame::generated() {
            return; // the real tree's game_frame needs a machine; the full cases run it
        }
        // The stub tree has none: the stop names game_frame's entry, not the mount.
        assert_eq!(run(1), Err(Stop::Untranslated("h0+$1fec".into())));
    }
}
