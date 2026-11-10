//! Shell, test tooling: Forest Dumb Forever's conformance runner. A case brings the original on the
//! oracle to a sync point, starts the engine there, feeds both the same input and compares the
//! modelled variables and the picture (plans/forest-dumb-level-1 DESIGN §2.1, §4, §4.1). The format
//! is in `engine/conform/README.md`.
//!
//! Two runners share the case format. A title case (forest-dumb-start's first case) steps the
//! title scene once a frame from power-on. A level case starts at the level sync point
//! (`sync.rs`) and compares at the end of each pass of `game_frame`: in play a pass usually ends
//! after the frame does, so the state at a frame's end is mid-pass (docs/09-play-measured.md).

pub mod bug;
pub mod case;
pub mod coverage;
pub mod disks;
// `gen` is a reserved word in edition 2024; the files keep the task's folder name.
#[path = "gen/mod.rs"]
pub mod generators;
pub mod judge;
pub mod routine;
pub mod sync;

use std::fmt;
use std::ops::{Range, RangeInclusive};
use std::path::{Path, PathBuf};

use amiga_chips::{Picture, to_rgb12};
use amiga_conform::compare::{PictureClass, PictureJudge, classify, picture_except};
use amiga_conform::{Mode, Tolerance, report, script};
use amiga_oracle::{
    Amiga, AmigaMemory, BlitEventKind, FrameInput, LineSample, OracleConfig, Rgb12Frame, SLOW_BASE,
    kickstart_path,
};
use std::sync::Arc;

use fd_core::level::redraw::{OBJECT_MAP, SCREEN_LINE, TILE_MAP};
use fd_core::level::{AddCut, EdgesCut};
use fd_core::render::objects::CUT_ROWS;
use fd_core::render::playfield::{SCREEN, Scroll};
use fd_core::{
    Event, Game, GameData, Mem, PORTED, Ported, Start, TitleScene, Transplant, Var, Width,
    layout_var,
};

pub use case::{Case, Compare, Exclusion, Goal, Swap};
pub use coverage::Coverage;

/// The main program's hunks where the boot of `boot-to-title` loads them, and `boot-to-level1`'s too
/// (it shares the boot up to the title)
/// (`docs/07-executable-map.md`). AmigaDOS picks them when the program loads, so another boot may
/// differ; there a wrong base shows as a comparison failing on its first change.
pub const HUNK_BASES: [u32; 5] = fd_core::HUNK_BASES;

/// The display window a level case compares, in DIWSTRT/DIWSTOP's encoding: the play area's top
/// (line 47), the status bar's left, right and bottom (lores 129..449, to line 297). The copper
/// reloads DIW between the two, so the frame-end window holds only the bar
/// (docs/09-play-measured.md "Display"; `tests/play_picture.rs`). Row 0 is line 47; the bar starts
/// at row 194.
pub const PLAY_WINDOW: (u16, u16) = (0x2F81, 0x29C1);

/// The end of a pass: `level3_handler` writes `INTREQ=$4010` at `h0+$1f02`, just after
/// `game_frame` returns (custom register `$9c`, value).
pub const PASS_END: (u16, u16) = (0x9c, 0x4010);

/// `game_over_screen` (`h0+$4c4`): `main_loop` has found the game over; no pass runs after it.
pub const GAME_OVER_SCREEN: (u8, u32) = (0, 0x4c4);
/// The high scores (`h0+$89d0`), after the game over screen: where the engine restarts the level
/// instead (DESIGN §2.4), so where a comparison ends.
pub const HIGH_SCORES: (u8, u32) = (0, 0x89d0);
/// `next_world` (`h0+$1ba`): `main_loop` has found the level's end; its first instruction adds one
/// to `world_index` (T26).
pub const NEXT_WORLD: (u8, u32) = (0, 0x1ba);

/// `main_loop`'s `bsr.w h0_6a96` (`h0+$29c`), its status bar redraw: reached only on a turn that
/// read `irq_count` as a multiple of 32, so only between the starts of passes `32k` and `32k + 1`.
/// A period with no hit never redrew its bar (T82).
pub const STATUS_REDRAW: (u8, u32) = (0, 0x29c);

/// The rows of [`PLAY_WINDOW`] the status bar fills (lines 241 to 296).
pub const STATUS_ROWS: Range<u16> = 194..250;

/// `level3_handler`'s `bsr.w game_frame` returns here (`h0+$1efa`): a line sample with it on the stack
/// is inside a pass, the handler's frame above it.
const AFTER_GAME_FRAME: u32 = 0x1efa;

/// The foreground's PC (a hunk 0 offset) where the level-3 interrupt of the last step took it: the
/// return address of the exception frame below `level3_handler`'s `movem.l d0-d7/a0-a6` (15 longs,
/// then SR and PC), read off the first line sample from line 233, where the interrupt comes, that
/// holds [`AFTER_GAME_FRAME`]. Needs [`Amiga::set_profile`] on for the step; `None` without samples.
pub fn interrupted_pc(am: &Amiga) -> Option<u32> {
    interrupt(&am.line_samples()).map(|(_, pc, _)| pc)
}

/// The first line sample from line 233 inside a pass ([`interrupted_pc`]): its index in `samples`,
/// the foreground's PC the interrupt took (a hunk 0 offset) and its registers d0-d7/a0-a6 as
/// `level3_handler`'s `movem.l` saved them.
fn interrupt(samples: &[LineSample]) -> Option<(usize, u32, [u32; 15])> {
    let ret = HUNK_BASES[0] + AFTER_GAME_FRAME;
    samples
        .iter()
        .enumerate()
        .filter(|(_, s)| s.vpos >= 233)
        .find_map(|(k, s)| {
            let i = s.stack.iter().position(|&l| l == ret)?;
            let word = |b: usize| -> Option<u32> {
                let l = *s.stack.get(b / 4)?;
                Some(if b % 4 == 0 { l >> 16 } else { l & 0xffff })
            };
            let at = 4 * (i + 1) + 15 * 4 + 2;
            let pc = (word(at)? << 16) | word(at + 2)?;
            let regs = s.stack.get(i + 1..i + 16)?.try_into().ok()?;
            Some((k, pc.wrapping_sub(HUNK_BASES[0]), regs))
        })
}

/// The edge latches right, left, up, down and `frame_flag` (`h0+$5196` to `$519f`), where the
/// original runs.
const FLAG_AND_LATCHES: u32 = HUNK_BASES[0] + 0x5196;

/// `camera_update` (`h0+$4d7e`), `game_frame`'s third call: the one that sets the edge latches.
const CAMERA_UPDATE: u32 = 0x4d7e;

/// Whether the runner reads, on the original, where each pass cut `main_loop`'s edge redraws and
/// hands it to the engine (T74): in every level case. Reading it samples
/// the lines of a step ([`Amiga::set_profile`]), which leaves the original's run unchanged
/// (`amiga-oracle`'s `tests/line_samples.rs`; here `tests/edge_latch.rs`, every case the runner
/// samples, every frame). Before T78 a sampled step could end early where a blit held the CPU across
/// the frame's end (a pass ended a step later on `enemy-route`).
pub fn records_edge_cuts(case: &Case) -> bool {
    case.is_level()
}

/// Which steps the runner samples line by line, as [`run_with`] decides it, shared with the check
/// that the sampling leaves the original unchanged. A step is sampled where it starts pass
/// `32k + 1`, whose cut decides the status bar (T71), and, when [`records_edge_cuts`], where its
/// pass may find the foreground in the edge redraws (T74): `frame_flag` or a latch set at the
/// step's start (a crossing scroll sets both), or the last pass still running short of
/// `camera_update`, which latches; else, at most, the last pass's end leaves four clear latches to
/// test, a few dozen cycles, before it spins. The oracle must count
/// [`Sampler::CAMERA_UPDATE`] and watch [`PASS_END`].
#[derive(Debug, Clone, Default)]
pub struct Sampler {
    camera_ran: bool,
    last_end: Option<u32>,
}

impl Sampler {
    /// `camera_update`'s address, for [`Amiga::watch_pcs`].
    pub const CAMERA_UPDATE: u32 = HUNK_BASES[0] + CAMERA_UPDATE;

    /// Whether to sample the next step of `case` on `am`.
    pub fn samples(&self, am: &Amiga, case: &Case) -> bool {
        let irq = layout_var("irq_count").expect("irq_count in LAYOUT");
        let started = Live(am).value(irq);
        let running = self.last_end.is_none_or(|last| started > last);
        let busy = records_edge_cuts(case)
            && ((running && !self.camera_ran)
                || am.peek(FLAG_AND_LATCHES, 10).iter().any(|&b| b != 0));
        busy || (case.compare.picture && started.wrapping_add(1) & 31 == 1)
    }

    /// After each step: whether `camera_update` ran in it, and the last pass that ended.
    pub fn stepped(&mut self, am: &Amiga) {
        self.camera_ran = am
            .pc_hits()
            .iter()
            .any(|&(p, n)| p == Self::CAMERA_UPDATE && n > 0);
        if let Some(mem) = am.watched_memory() {
            let irq = layout_var("irq_count").expect("irq_count in LAYOUT");
            let ended = OracleMem(&mem).value(irq);
            self.last_end = Some(self.last_end.map_or(ended, |l| l.max(ended)));
        }
    }
}

/// `main_loop`'s `bsr.w redraw_edge_a` to `_d` return here (`h0+$2da`, `$302`, `$32a`, `$352`): one
/// on the foreground's stack means that redraw was running, and the latches up to it were tested.
const AFTER_REDRAW: [u32; 4] = [0x2da, 0x302, 0x32a, 0x352];

/// Each redraw's object scan (`redraw_edge_a` to `_d`): the loop's top (`lea.l h0+$cb38, a2`, then
/// `d5` and `d6` added), where a found cell's compares start, and the `bra.w` after its
/// `bsr.w h0_6882`. From the compares to that `bra.w` the cell's add has not been made.
const SCANS: [(u32, u32, u32); 4] = [
    (0x98a, 0x9a8, 0xa02),
    (0xc6c, 0xc88, 0xce0),
    (0xfd8, 0xff4, 0x1048),
    (0x1318, 0x1334, 0x138c),
];

/// Each redraw's bob routine (`h0_a06`, `h0_ce4`, `h0_104c`, `h0_1390`): its first instruction and
/// its `move.w #$1401, $dff058.l`, which starts the blit. A PC from one to the other has not.
const BOBS: [(u32, u32); 4] = [
    (0xa06, 0xab2),
    (0xce4, 0xd8c),
    (0x104c, 0x10f2),
    (0x1390, 0x143a),
];

/// `h0_6882`, and its `movem.l` after the `st.b` that marks the new cell used.
const ADD_CELL: Range<u32> = 0x6882..0x69a0;
/// In `h0_6882`: the first instruction after its four counts, where it has found the chosen
/// list's free entry, and its `st.b` (T82, `fd_core::level::AddCut`). A PC at one has not run it.
const ADD_COUNTED: u32 = 0x691e;
const ADD_FOUND: u32 = 0x6984;
const ADD_MARK: u32 = 0x699e;

/// Each scan's creature branch, from its first instruction to its `bsr.w h0_681c`: a PC in it has
/// not started the cell's creature (T75).
const CREATURE_BRANCH: [RangeInclusive<u32>; 4] = [
    0xabc..=0xb10,
    0xd98..=0xde6,
    0x10fc..=0x113a,
    0x1444..=0x1486,
];

/// `h0_681c` and `h0_6852`, up to the `st.b` that marks the slot used: until it runs the pass sees
/// the slot free, so the creature is not started yet. Not `h0_681c`'s two instructions after its
/// `bsr.w h0_6852` (`h0+$6836`, `$6838`), where it is.
const START_CREATURE: RangeInclusive<u32> = 0x681c..=0x687a;
const STARTED: RangeInclusive<u32> = 0x6836..=0x6838;

/// Each redraw's own code (`redraw_edge_a` to `_d`, with its bob routine and creature branch), and
/// where its last tile blit returns: up to there it draws tiles, then shapes, then scans from
/// [`SCANS`]' top to its end.
const REDRAWS: [(Range<u32>, u32); 4] = [
    (0x844..0xb20, 0x904),
    (0xb20..0xdf6, 0xbe0),
    (0xdf6..0x114a, 0xee2),
    (0x114a..0x1496, 0x1226),
];

/// `h0_774` and `h0_7cc`, the tile blit and the shape copy: only the redraws' tile and shape loops
/// call them in play, so a PC in them is before the scan.
const TILE_AND_SHAPE: Range<u32> = 0x774..0x83c;

/// `h0_3ac2`, the raster and blitter wait: a tile's blit calls it before the scan and a bob's in it.
const WAIT: RangeInclusive<u32> = 0x3ac2..=0x3ada;

/// Where the last step's pass cut `main_loop`'s redraw half (`h0+$2b2` to `$358`), as
/// [`fd_core::level::Cut::At`] takes it; `None` when the interrupt found the foreground elsewhere
/// (T74). Needs [`Amiga::set_profile`] on for the step.
///
/// The interrupted PC decides inside `main_loop` and inside a redraw's own code. Elsewhere it is in
/// a routine many callers share, and the exception frame is on the supervisor stack while the
/// foreground runs on the user stack, so the redraw is named by `main_loop`'s return address on the
/// user stack of the last line sample before the interrupt taken in user mode, at most a line old.
/// Its scan's position is the foreground's `a2`, once the scan has started ([`edges_cut_at`]).
pub fn edges_cut(am: &Amiga) -> Option<EdgesCut> {
    let samples = am.line_samples();
    let (k, pc, regs) = interrupt(&samples)?;
    let (seen, returns): (u32, Vec<u32>) = samples[..k]
        .iter()
        .rev()
        .find(|s| s.sr & 0x2000 == 0)
        .map(|s| {
            let returns = s
                .stack
                .iter()
                .map(|l| l.wrapping_sub(HUNK_BASES[0]))
                .collect();
            (s.pc.wrapping_sub(HUNK_BASES[0]), returns)
        })
        .unwrap_or_default();
    edges_cut_at(pc, seen, &returns, &regs)
}

/// [`edges_cut`] from the interrupted PC, the foreground's PC and stack (longs, as hunk 0 offsets)
/// at the line sample before it, and its registers d0-d7/a0-a6 at the interrupt. `main_loop`'s
/// `clr.w` of each latch (`h0+$2d0`, `$2f8`, `$320`, `$348`) and of `frame_flag` (`$352`) mark the
/// bounds: a PC at one has not run it yet. Below `$2b2` the loop may be spinning with `frame_flag`
/// clear, or in `h0_6a96`'s status redraw before the redraw half, which this does not tell apart:
/// not a cut (the redraw half then runs whole after the pass, never measured to change a latch).
///
/// In a redraw, whether its scan has started is the PC's place, never `a2`: the row redraws' tile
/// and shape loops leave `a2` alone, so until the scan's first `lea.l` it still points into the
/// object map where the scan before it stopped (T81: `mechanic-vanish`, pass 331, cut in
/// `redraw_edge_a`'s shapes with the last column scan's `a2`). The scan has not started in the
/// redraw's own code below its loop's top, nor in the tile blit or the shape copy; in the raster
/// wait, which a tile's blit and a bob's both call, it has unless the sample a line before was
/// still at the tiles (the shapes after them take 23 lines). Started, at the loop's top the cell
/// is `h0+$cb38 + d5 + d6`, not yet looked at; elsewhere it is `a2`, its add or creature start made
/// unless the PC is between its compares and the return from `h0_6882`'s `st.b`, or in its
/// creature branch short of `h0_6852`'s `st.b` (T75).
pub fn edges_cut_at(pc: u32, seen: u32, returns: &[u32], regs: &[u32; 15]) -> Option<EdgesCut> {
    let next = match pc {
        0x2b2..=0x2d0 => 0,
        0x2d2..=0x2f8 => 1,
        0x2fa..=0x320 => 2,
        0x322..=0x348 => 3,
        0x34a..=0x352 => 4,
        // The rest of `main_loop`, and the status bar's redraw and draw routines (T71).
        0x252..0x35c | 0x6a96..0x6da0 => return None,
        _ => {
            // In a redraw's own code the PC names it, whatever the sample a line before held.
            let r = REDRAWS
                .iter()
                .position(|(own, _)| own.contains(&pc))
                .or_else(|| {
                    returns
                        .iter()
                        .find_map(|r| AFTER_REDRAW.iter().position(|a| a == r))
                })?;
            let (top, compares, added) = SCANS[r];
            let (own, tiles_end) = &REDRAWS[r];
            let scanning = if own.contains(&pc) {
                pc >= top
            } else if WAIT.contains(&pc) {
                // Where the redraw's own code was a line before: in it, or called from it.
                let was = Some(seen)
                    .filter(|p| own.contains(p))
                    .or_else(|| returns.iter().copied().find(|p| own.contains(p)));
                was.is_some_and(|p| p > *tiles_end)
            } else {
                !TILE_AND_SHAPE.contains(&pc)
            };
            let (d5, d6, a2) = (regs[5], regs[6], regs[10].wrapping_sub(HUNK_BASES[0]));
            let defer_from = if !scanning {
                0
            } else if (top..top + 0xc).contains(&pc) {
                OBJECT_MAP.wrapping_add(d5).wrapping_add(d6)
            } else if (OBJECT_MAP..TILE_MAP).contains(&a2) {
                // A bob not yet started: in its routine up to its BLTSIZE write, or in the
                // blitter wait, which only a bob calls once the scan runs (T82).
                let (bob, size) = BOBS[r];
                let pending = (compares..added).contains(&pc)
                    || (bob..=size).contains(&pc)
                    || WAIT.contains(&pc)
                    || ADD_CELL.contains(&pc)
                    || CREATURE_BRANCH[r].contains(&pc)
                    || (START_CREATURE.contains(&pc) && !STARTED.contains(&pc));
                a2 + !pending as u32
            } else {
                0
            };
            // Inside `h0_6882` for the scan's cell: what it had settled before the pass (T82).
            let add = match pc {
                _ if !scanning || defer_from != a2 => AddCut::Whole,
                ADD_COUNTED..ADD_FOUND => AddCut::List,
                ADD_FOUND..=ADD_MARK => AddCut::Entry,
                _ => AddCut::Whole,
            };
            return Some(EdgesCut {
                next: r as u8 + 1,
                defer_from: Some(defer_from),
                shape_words: (!scanning).then(|| shapes_copied(r, pc, regs)),
                add,
            });
        }
    };
    Some(EdgesCut {
        next,
        defer_from: None,
        shape_words: None,
        add: AddCut::Whole,
    })
}

/// `h0_7cc`, the shape copy, and its first `move.w`: a PC at a `move.w` has not run it.
const SHAPE_COPY: Range<u32> = 0x7cc..0x83c;
const SHAPE_COPY_FIRST: u32 = 0x7da;
/// The row redraws' shape loops (`redraw_edge_a`, `_b`): the loop's top, its `bsr.w h0_7cc`, its
/// `dbra d0` (22 shapes, `d0` from `$15`), and the `bsr.w` of the 23rd after it.
const ROW_SHAPES: [(u32, u32, u32, u32); 2] =
    [(0x938, 0x940, 0x950, 0x968), (0xc12, 0xc1a, 0xc2a, 0xc42)];
/// The column redraws' (`_c`, `_d`): where `d2` takes the band block, then each `bsr.w h0_7cc` with
/// the `addi.l #4, d2` after it for the loop of 14 and the 15th shape, and the 16th's `bsr.w`.
/// `d2` is the block (a multiple of `$40`) plus 4 a shape, so its low six bits name the shape.
const COLUMN_SHAPES: [(u32, [(u32, u32); 2], u32); 2] = [
    (0xf0a, [(0xf3c, 0xf46), (0xf7c, 0xf86)], 0xfb2),
    (0x124e, [(0x1280, 0x128a), (0x12c0, 0x12ca)], 0x12f6),
];

/// How many `collision_map` words redraw `r` (0 to 3, `redraw_edge_a` to `_d`) had copied when a
/// pass interrupted it at `pc` before its scan, with `regs` at the interrupt
/// ([`EdgesCut::shape_words`], T48): 16 a shape already copied, by the PC's place in the redraw's
/// own code and its loop's counter, and inside `h0_7cc` the words its `move.w`s before the PC made.
/// In the tile blit or its raster wait none is copied yet.
fn shapes_copied(r: usize, pc: u32, regs: &[u32; 15]) -> u16 {
    let copying = SHAPE_COPY.contains(&pc);
    let words = if !copying || pc <= SHAPE_COPY_FIRST {
        0
    } else {
        ((pc - SHAPE_COPY_FIRST - 2) / 6 + 1).min(16)
    };
    let own = REDRAWS[r].0.contains(&pc);
    let whole = if r < 2 {
        let (top, call, dbra, last) = ROW_SHAPES[r];
        let d0 = regs[0] & 0xffff;
        let k = 0x15u32.wrapping_sub(d0);
        match pc {
            _ if copying => {
                if d0 == 0xffff {
                    22
                } else {
                    k
                }
            }
            _ if !own || pc < top => 0,
            _ if pc <= call => k,
            _ if pc <= dbra => k + 1,
            _ if pc <= last => 22,
            _ => 23,
        }
    } else {
        let (load, calls, last) = COLUMN_SHAPES[r - 2];
        let j = (regs[2] & 0x3f) >> 2;
        match pc {
            _ if copying => j,
            _ if !own || pc <= load => 0,
            _ if pc > last => 16,
            _ if calls.iter().any(|&(call, addi)| pc > call && pc <= addi) => j + 1,
            _ => j,
        }
    };
    (whole * 16 + words) as u16
}

/// One blit of a creature's cut (`h0_5820`) as the original made it (T80).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutSeen {
    /// D's first word, as a byte offset from the play screen.
    pub dst: u32,
    /// Its rows, each one plane of one line: a creature is [`CUT_ROWS`], in two blits where it
    /// wraps past the screen's last line.
    pub rows: u16,
    /// The beam as the blit began and as it ended, in DMA cycles from the frame's start.
    pub begin: u32,
    pub end: u32,
}

/// A raster line in DMA cycles (PAL), and the level-3 interrupt's line, where a pass starts.
const LINE_CYCLES: u32 = 227;
const PASS_LINE: u16 = 233;

/// The cycle in a line at which the play area's first word is fetched (`DDFSTRT=$30`, docs/09
/// "Display"), the cycles a word of its planes takes, and its lines (47 to 225, `DIWSTOP=$e2a1`).
const FETCH_START: u32 = 0x30;
const FETCH_WORD: u32 = 8;
const PLAY_LINES: u32 = 0xe2 - 0x2f;

/// The creature cuts the last step's frame saw before its pass came at line 233, in order: the cuts
/// of the pass that started in the step before and ran on into this frame, the one this frame shows
/// (T80). A short pass cuts in the step it starts in, below the play area, and leaves none here.
/// Read off what the oracle keeps of every step ([`Amiga::cpu_writes`], [`Amiga::blit_events`]),
/// so nothing is switched on to take it. A cut is a BLTSIZE write three words wide under
/// `h0_5820`'s minterm (`BLTCON0 & $fff = $fca`); the bobs' and the animated cells' are one word.
/// `None` when a cut's begin or end is missing from the step's blits.
pub fn creature_cuts(am: &Amiga) -> Option<Vec<CutSeen>> {
    let cycles = |v: u16, h: u16| v as u32 * LINE_CYCLES + h as u32;
    let events = am.blit_events();
    let (mut con0, mut dpt) = (0u16, 0u32);
    let mut cuts = Vec::new();
    for w in am.cpu_writes() {
        if w.vpos >= PASS_LINE {
            break;
        }
        match w.reg {
            0x40 => con0 = w.value,
            0x54 => dpt = (dpt & 0xffff) | (w.value as u32) << 16,
            0x56 => dpt = (dpt & 0xffff_0000) | w.value as u32,
            0x58 if con0 & 0xfff == 0xfca && w.value & 63 == 3 => {
                let at = cycles(w.vpos, w.hpos);
                let mut after = events
                    .iter()
                    .filter(|e| !e.next)
                    .map(|e| (e.kind, cycles(e.vpos, e.hpos)))
                    .skip_while(|&(k, t)| k != BlitEventKind::Begin || t < at);
                let begin = after.next()?.1;
                let end = after.find(|&(k, _)| k == BlitEventKind::End)?.1;
                cuts.push(CutSeen {
                    dst: (dpt & 0x7_fffe).wrapping_sub(HUNK_BASES[3] + SCREEN),
                    rows: w.value >> 6,
                    begin,
                    end,
                });
            }
            _ => {}
        }
    }
    Some(cuts)
}

/// For each creature of `cuts`, how many of its cut's first blit rows the beam had passed before
/// the blitter wrote them, in the frame a camera at `camera` shows: the frame shows those rows as
/// they were before the cut (T80). A row is one plane of one line. Its words are taken as written
/// evenly between the blit's begin and its end, and fetched where the display reaches its line
/// ([`PLAY_WINDOW`]'s top is display row 0) and its place in it, so the count can be a row or two
/// off and the row at the edge half shown: measured over the three cases that meet it, at most 48
/// pixels of a frame, inside the tolerance's few (docs/10). Rows outside the play area's lines are
/// never fetched.
pub fn rows_behind_beam(cuts: &[CutSeen], camera: (u16, u16)) -> Vec<u16> {
    let buffer = 0x100 * SCREEN_LINE;
    let top = Scroll::from_camera(camera.0, camera.1).top % buffer;
    let first_line = (PLAY_WINDOW.0 >> 8) as u32;
    let (mut out, mut made, mut behind) = (Vec::new(), 0u32, 0u32);
    for c in cuts {
        // From the window's first fetched word; a cut starts at most a word left of it.
        let from = (c.dst % buffer + buffer + 4 - top) % buffer;
        let (line, word) = (
            from / SCREEN_LINE,
            (from % SCREEN_LINE).saturating_sub(4) / 2,
        );
        for r in 0..c.rows as u32 {
            let shown_line = (line + r / 5) % 0x100;
            // The middle one of the row's three words.
            let written = c.begin + (c.end - c.begin) * (2 * r + 1) / (2 * c.rows as u32);
            let fetched =
                (first_line + shown_line) * LINE_CYCLES + FETCH_START + FETCH_WORD * (word + 1);
            if shown_line < PLAY_LINES && written > fetched {
                behind = made + r + 1;
            }
        }
        made += c.rows as u32;
        if made >= CUT_ROWS as u32 {
            out.push(behind as u16);
            (made, behind) = (0, 0);
        }
    }
    out
}

/// How many of `h0_6a96`'s four reads (countdown at `h0+$6a96`, lives `$6aaa`, score `$6abe`, items
/// `$6ad2`) `main_loop`'s redraw made before the interrupt that took the foreground at hunk 0 offset
/// `pc` (T71). `None` when the redraw was not cut: `main_loop` had not read `irq_count` yet
/// (`h0+$252`), or `h0_6a96` had read all four. A cut between a compare and its draw routine's own
/// read may show either value of that number, so both counts on its sides are given; one inside a
/// draw routine (`h0_6ae8` to `h0_6c80`'s end) gives every count, never measured (docs/09).
pub fn redraw_cut(pc: u32) -> Option<Range<u8>> {
    const READS: [u32; 4] = [0x6a96, 0x6aaa, 0x6abe, 0x6ad2];
    match pc {
        // After `move.w irq_count, d0`, up to the `bsr.w h0_6a96`: nothing read yet.
        0x258..=0x29c => Some(0..1),
        // Up to `bsr.w h0_6c20`, the items' draw: `rts` at `h0+$6ae6` has read all four.
        0x6a96..=0x6ae2 => {
            let done = READS.iter().filter(|&&r| r < pc).count() as u8;
            // Exactly at a read: nothing between it and the last; else the number just compared
            // may be drawn with the value its routine reads after.
            if READS.contains(&pc) {
                Some(done..done + 1)
            } else {
                Some(done - 1..done + 1)
            }
        }
        // The draw routines, up to the lives' icon table.
        0x6ae8..0x6da0 => Some(0..5),
        _ => None,
    }
}

/// What a level case never compares, by name: the original after `next_world`. A level case's
/// comparison ends in the step the original runs it, holding `world_index` (DESIGN §2.5); world 2
/// is no task of this plan, so the exclusion stays until world 2 is built.
pub const AFTER_NEXT_WORLD: &str = "world 2's load and its SCENA screen after `next_world`, where \
     the engine shows \"Level 1 complete\" until world 2 is built (DESIGN §2.5)";

/// `engine/conform/`, wherever this is run from.
pub fn conform_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conform")
}

/// Where variable `name` is in the original's memory, on the boot the case runs.
pub fn address(name: &str) -> Result<u32, String> {
    let v = layout_var(name).ok_or_else(|| format!("`{name}` is not in fd-core's LAYOUT"))?;
    Ok(HUNK_BASES[v.hunk as usize] + v.offset)
}

/// The original's RAM as the main program's hunks see it.
pub struct OracleMem<'a>(pub &'a AmigaMemory);

impl Mem for OracleMem<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        let m = self.0;
        let slow = SLOW_BASE as usize;
        (0..len as u32)
            .map(|k| {
                let a = (HUNK_BASES[hunk as usize] + offset + k) as usize & 0xFF_FFFF;
                if a < m.chip.len() {
                    m.chip[a]
                } else if a >= slow && a - slow < m.slow.len() {
                    m.slow[a - slow]
                } else {
                    0
                }
            })
            .collect()
    }
}

/// The original as it runs, read through [`Mem`] without copying its RAM.
pub struct Live<'a>(pub &'a Amiga);

impl Mem for Live<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        self.0.peek(HUNK_BASES[hunk as usize] + offset, len)
    }
}

/// What a level case runs on the engine's side: started from the original's memory at a pass's
/// end ([`Transplant`]), then one pass of play per call.
pub trait Engine: Transplant {
    /// One pass of play on the input the original's pass read.
    fn pass(&mut self, input: FrameInput);
    /// The picture after the last pass, in [`PLAY_WINDOW`]; `None` if the engine draws none.
    fn picture(&self) -> Option<Picture>;
    /// What happened in the last pass that a case must see. An [`Event::Unported`] fails every
    /// case (DESIGN §4.1); an `idle` goal fails on any event.
    fn events(&mut self) -> Vec<Event> {
        Vec::new()
    }
    /// A step the original spends on the game over screen, where no pass runs: the engine steps on
    /// the step's input (T23). An engine without the screen does nothing.
    fn off_play(&mut self, _input: FrameInput) {}
    /// Whether the engine shows the game over screen.
    fn game_over(&self) -> bool {
        false
    }
    /// Whether the engine has found the level's end and run `next_world` (T26).
    fn level_end(&self) -> bool {
        false
    }
    /// The picture after the last pass with `main_loop`'s last status bar redraw cut short by pass
    /// `32k + 1` after `cut` of its four reads ([`redraw_cut`], T71); `None` if it draws the same.
    fn cut_redraw(&self, _cut: u8) -> Option<Picture> {
        None
    }
    /// Before each [`Engine::pass`]: where the original's same pass cut `main_loop`'s redraw half
    /// ([`edges_cut`]; `None`: it did not, or no line sample showed it), for the engine to stop
    /// its own there (`Game::cut_foreground`, T74). A wrapper of [`Game`] forwards it; one that
    /// does not keeps the engine's own rule.
    fn foreground_cut(&mut self, _at: Option<EdgesCut>) {}
    /// The picture after the last pass as the original shows it where the beam had passed the first
    /// `rows[k]` blit rows of the `k`th creature the pass cut before they were written
    /// ([`rows_behind_beam`], `Game::creatures_uncut`, T80); `None` if the engine cannot draw it. A
    /// wrapper of [`Game`] forwards it; one that does not fails at the first run of such frames
    /// past the tolerance.
    fn creatures_uncut(&self, _rows: &[u16]) -> Option<Picture> {
        None
    }
    /// The picture after the last pass as the original shows it through a status period whose
    /// redraw `main_loop` never reached: the bar as it stood before that redraw
    /// (`Game::redraw_skipped`, T82), the play area with `uncut`'s creature rows left out when the
    /// frame also saw [`Engine::creatures_uncut`]'s cut. `None` if the bar is the same or the
    /// engine cannot draw it. A wrapper of [`Game`] forwards it; one that does not fails a case
    /// that meets such a period.
    fn redraw_skipped(&self, _uncut: Option<&[u16]>) -> Option<Picture> {
        None
    }
}

/// The level engine, fd-core's [`Game`], as a case runs it: one [`Game::step`] a pass, and its
/// picture ([`Game::picture`], T10) as the pass leaves the display.
impl Engine for Game {
    fn pass(&mut self, input: FrameInput) {
        self.step(input);
    }

    fn picture(&self) -> Option<Picture> {
        Some(Game::picture(self))
    }

    fn events(&mut self) -> Vec<Event> {
        self.take_events()
    }

    fn off_play(&mut self, input: FrameInput) {
        self.step(input);
    }

    fn game_over(&self) -> bool {
        matches!(self.scene(), fd_core::Scene::GameOver(_))
    }

    fn level_end(&self) -> bool {
        self.finished()
    }

    fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        Game::cut_redraw(self, cut)
    }

    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        self.cut_foreground(at);
    }

    fn creatures_uncut(&self, rows: &[u16]) -> Option<Picture> {
        Game::creatures_uncut(self, rows)
    }

    fn redraw_skipped(&self, uncut: Option<&[u16]>) -> Option<Picture> {
        Game::redraw_skipped(self, uncut)
    }
}

/// The game's data from the owner's disks, which [`fd_tools::disk_dir`] finds; a missing disk or
/// file is an error naming it.
pub fn game_data() -> Result<Arc<GameData>, String> {
    let dir = fd_tools::disk_dir()?;
    game_data_in(&dir)
}

/// [`game_data`] from the zips in `dir`.
pub fn game_data_in(dir: &Path) -> Result<Arc<GameData>, String> {
    let files = |name: &str| {
        // The main program is disk 1's; every level file is disk 2's.
        let disk = if name == fd_core::program::PROGRAM_FILE {
            1
        } else {
            2
        };
        fd_tools::game_file(dir, &format!("disk{disk}/{name}"))
            .map_err(|e| fd_assets::AssetError::missing(name, e))
    };
    GameData::read(&files)
        .map(Arc::new)
        .map_err(|e| format!("game data from {}: {e}", dir.display()))
}

/// How a run is made: what a case file cannot choose.
#[derive(Debug, Clone)]
pub struct Options<'a> {
    /// Faithful unless a diagnosis asks for strict: a case cannot loosen its own bar.
    pub mode: Mode,
    /// The routines whose hits the coverage report counts: fd-core's `PORTED`.
    pub register: &'a [Ported],
    /// Where a picture difference's PNG goes (it shows the original, so outside git).
    pub diff_dir: Option<PathBuf>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            mode: Mode::Faithful,
            register: PORTED,
            diff_dir: None,
        }
    }
}

/// How a case came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every comparison held and the goal was reached.
    Same,
    /// The first variable that differs. `pass` counts from the engine's start (a title case's sync
    /// point is pass 0); `frame` is the oracle's frames run when it was read.
    Differs {
        pass: u32,
        frame: u32,
        var: String,
        engine: u32,
        oracle: u32,
    },
    /// A picture past the tolerance.
    PictureDiffers { pass: u32, frame: u32, why: String },
    /// The case compared nothing of what it says (porting-routines §4).
    ComparedNothing { what: String },
    /// Both sides agreed and missed the goal.
    GoalMissed { goal: String },
    /// An `idle` goal failed: an event, or a pass not compared.
    NotIdle { why: String },
    /// The engine met an object value nobody has ported (DESIGN §4.1): every case fails on it.
    Unported { pass: u32, frame: u32, value: u8 },
    /// An exclusion hid no difference over the whole run: it is no longer needed.
    ExclusionUnused { what: String, task: String },
    /// One side went to the game over screen, or left it, or found the level's end, and the other
    /// did not (T23, T26).
    SceneDiffers { frame: u32, why: String },
}

/// A case's result and its report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub outcome: Outcome,
    /// Passes whose variables were compared (a title case: frames, the sync point included).
    pub passes: u32,
    /// Frames whose picture was compared.
    pub pictures: u32,
    /// Passes the original ended unseen (two ends in one step), so not compared.
    pub unseen: u32,
    /// Frames tolerated as a colour step, as a few pixels, as a short glitch.
    pub tolerated: [u32; 3],
    /// Frames whose status bar matched the engine's [`Engine::cut_redraw`] at the cut measured on
    /// the original, not its own redraw (T71).
    pub cut_redraws: u32,
    /// Frames compared with the status bar as it stood before a redraw the original never
    /// reached, its calls counted on the original ([`Engine::redraw_skipped`], T82).
    pub skipped_redraws: u32,
    /// Passes that came while `main_loop`'s edge redraws were still running on the original, so
    /// handed to the engine as cut there ([`Engine::foreground_cut`], T74).
    pub edge_cuts: u32,
    /// Frames compared as the engine draws them with the creature cuts the beam had passed left
    /// out, as measured on the original ([`Engine::creatures_uncut`], T80).
    pub creature_cuts: u32,
    /// Each exclusion and the frames on which its rows differed, or the passes on which its
    /// variable did.
    pub excluded: Vec<(Exclusion, u32)>,
    /// `(pass, event)` as the engine raised them.
    pub events: Vec<(u32, Event)>,
    /// The oracle frame at whose pass end the goal held.
    pub goal_met: Option<u32>,
    /// The oracle frames after whose steps both sides showed the game over screen and both left it
    /// for the high scores (T23); the comparison ends at the second.
    pub game_over: Option<(u32, Option<u32>)>,
    /// The oracle frame of the step both sides found the level's end in (T26): the original ran
    /// `next_world`, the engine entered its level end, `world_index` equal. The comparison ends
    /// there; [`AFTER_NEXT_WORLD`] is not compared.
    pub level_end: Option<u32>,
    pub coverage: Coverage,
}

impl Verdict {
    fn new(register: &[Ported]) -> Verdict {
        Verdict {
            outcome: Outcome::Same,
            passes: 0,
            pictures: 0,
            unseen: 0,
            tolerated: [0; 3],
            cut_redraws: 0,
            skipped_redraws: 0,
            edge_cuts: 0,
            creature_cuts: 0,
            excluded: Vec::new(),
            events: Vec::new(),
            goal_met: None,
            game_over: None,
            level_end: None,
            coverage: Coverage::new(register),
        }
    }

    pub fn passed(&self) -> bool {
        self.outcome == Outcome::Same
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Same => write!(f, "same"),
            Outcome::Differs {
                pass,
                frame,
                var,
                engine,
                oracle,
            } => write!(
                f,
                "differs at pass {pass} (oracle frame {frame}): {var} is {engine} ({engine:#x}) in \
                 the engine, {oracle} ({oracle:#x}) in the original"
            ),
            Outcome::PictureDiffers { pass, frame, why } => write!(
                f,
                "picture differs at pass {pass} (oracle frame {frame}): {why}"
            ),
            Outcome::ComparedNothing { what } => write!(f, "compared nothing: {what}"),
            Outcome::GoalMissed { goal } => {
                write!(f, "both sides agree but miss the goal: {goal}")
            }
            Outcome::NotIdle { why } => write!(f, "not idle: {why}"),
            Outcome::Unported { pass, frame, value } => write!(
                f,
                "unported object value {value:#04x} met at pass {pass} (oracle frame {frame}): \
                 no ObjectKind is registered for it"
            ),
            Outcome::ExclusionUnused { what, task } => write!(
                f,
                "exclusion `{what}` ({task}) hid no difference: remove it"
            ),
            Outcome::SceneDiffers { frame, why } => {
                write!(f, "scene differs at oracle frame {frame}: {why}")
            }
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.outcome {
            Outcome::Same => write!(
                f,
                "same on all {} passes compared, {} pictures",
                self.passes, self.pictures
            )?,
            ref other => write!(f, "{other}")?,
        }
        if self.unseen > 0 {
            write!(f, "\n{} pass ends unseen, not compared", self.unseen)?;
        }
        let [c, p, g] = self.tolerated;
        if c + p + g > 0 {
            write!(
                f,
                "\ntolerated: {c} frames a colour step off, {p} a few pixels, {g} glitches"
            )?;
        }
        if self.cut_redraws > 0 {
            write!(
                f,
                "\n{} frames' status bar as main_loop's redraw cut short by the next pass shows it",
                self.cut_redraws
            )?;
        }
        if self.skipped_redraws > 0 {
            write!(
                f,
                "\n{} frames' status bar as it stood before a redraw main_loop never reached",
                self.skipped_redraws
            )?;
        }
        if self.edge_cuts > 0 {
            write!(
                f,
                "\n{} passes came during main_loop's edge redraws, handed to the engine",
                self.edge_cuts
            )?;
        }
        if self.creature_cuts > 0 {
            write!(
                f,
                "\n{} frames show a creature the original cut behind the beam, drawn so by the engine",
                self.creature_cuts
            )?;
        }
        for (e, n) in &self.excluded {
            write!(
                f,
                "\nexcluded {} ({}): differed on {n} {}",
                e.what,
                e.task,
                if e.var.is_some() { "passes" } else { "frames" }
            )?;
        }
        for (pass, e) in &self.events {
            write!(f, "\nevent at pass {pass}: {e:?}")?;
        }
        if let Some((from, to)) = self.game_over {
            write!(f, "\ngame over screen from oracle frame {from}")?;
            if let Some(to) = to {
                write!(
                    f,
                    "; the high scores start at {to}, where the engine restarts: compared up to there"
                )?;
            }
        }
        if let Some(fr) = self.level_end {
            write!(
                f,
                "\nlevel end: both ran `next_world` at oracle frame {fr}, world_index equal; \
                 compared up to there, excluded after: {AFTER_NEXT_WORLD}"
            )?;
        }
        if let Some(fr) = self.goal_met {
            write!(f, "\ngoal reached at oracle frame {fr}")?;
        }
        write!(f, "\n{}", self.coverage)
    }
}

/// Run `case` on the real disks and Kickstart. Every missing input is an error naming its path, and
/// the cheap checks come first so a wrong setup fails before any emulation.
pub fn run(case: &Case) -> Result<Verdict, String> {
    if case.is_level() {
        let mut game = Game::new(game_data()?, Start::Level1);
        return run_with(case, &mut game, &Options::default());
    }
    run_title(case, &Options::default())
}

/// A script under `engine/conform/scripts/`.
fn load_script(name: &str) -> Result<script::InputScript, String> {
    let path = conform_dir().join("scripts").join(format!("{name}.txt"));
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("script {}: {e}", path.display()))?;
    script::parse(&text).map_err(|e| format!("script {}: {e}", path.display()))
}

/// The input of the step that starts at oracle frame `frame`: the sync script's before the sync
/// point, and after it the case's `input` (counted from the sync frame) or the sync script's.
pub struct Inputs {
    sync: script::InputScript,
    after: Option<script::InputScript>,
    sync_frame: u32,
}

impl Inputs {
    pub fn load(case: &Case) -> Result<Inputs, String> {
        Ok(Inputs {
            sync: load_script(&case.script)?,
            after: case.input.as_deref().map(load_script).transpose()?,
            sync_frame: case.sync_frame,
        })
    }

    /// The case's sync script, and `after` from its sync frame on: inputs held in memory.
    pub fn with_after(case: &Case, after: script::InputScript) -> Result<Inputs, String> {
        Ok(Inputs {
            sync: load_script(&case.script)?,
            after: Some(after),
            sync_frame: case.sync_frame,
        })
    }

    pub fn at(&self, frame: u32) -> FrameInput {
        match &self.after {
            Some(after) if frame >= self.sync_frame => {
                script::input_at(&after.lines, frame - self.sync_frame)
            }
            _ => script::input_at(&self.sync.lines, frame),
        }
    }
}

/// The title scene's value of `var`. Each title variable needs its arm here.
fn title_value(scene: &TitleScene, var: &Var) -> u32 {
    match var.name {
        "mt_counter" => scene.variable() as u32,
        other => panic!("{other} is not modelled by the title scene"),
    }
}

fn layout_vars(names: &[String]) -> Result<Vec<&'static Var>, String> {
    names
        .iter()
        .map(|v| layout_var(v).ok_or_else(|| format!("`{v}` is not in fd-core's LAYOUT")))
        .collect()
}

/// forest-dumb-start's runner: power-on to the sync frame, then the title scene and the original a
/// frame at a time, every variable each frame, the sync point included.
fn run_title(case: &Case, opts: &Options) -> Result<Verdict, String> {
    let inputs = Inputs::load(case)?;
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disks = disks::load()?;
    let vars = layout_vars(&case.vars)?;

    let cap = case.sync_frame + case.frames;
    let cfg = OracleConfig::new(kickstart, disks, cap);
    let mut amiga = Amiga::power_on(&cfg).map_err(|e| e.to_string())?;
    let step = |amiga: &mut Amiga, frame: u32| {
        let input = inputs.at(frame);
        amiga.step(input).map_err(|e| e.to_string())?;
        Ok::<_, String>(input)
    };
    for frame in 0..case.sync_frame {
        if let Some(swap) = case.swap.filter(|s| s.frame == frame) {
            amiga.insert_disk(0, swap.disk as usize - 1);
        }
        step(&mut amiga, frame)?;
    }
    let pcs: Vec<u32> = opts.register.iter().map(coverage::address).collect();
    amiga.watch_pcs(&pcs);
    let mut verdict = Verdict::new(opts.register);
    let mut scene = TitleScene::new();
    for pass in 0..=case.frames {
        let frame = case.sync_frame + pass;
        if pass > 0 {
            let input = step(&mut amiga, frame - 1)?;
            verdict.coverage.add(opts.register, amiga.pc_hits());
            scene.frame(input);
        }
        for v in &vars {
            let (engine, oracle) = (title_value(&scene, v), Live(&amiga).value(v));
            if engine != oracle {
                verdict.outcome = Outcome::Differs {
                    pass,
                    frame,
                    var: v.name.to_string(),
                    engine,
                    oracle,
                };
                return Ok(verdict);
            }
        }
        verdict.passes += 1;
    }
    Ok(verdict)
}

/// The original at a level case's sync frame, `set` applied, with room for its frames
/// and `slack` more. From the cached level sync point, then the sync script up to the case's frame.
pub fn reach_case(case: &Case, slack: u32) -> Result<Amiga, String> {
    reach_case_with(case, slack, |_, _| Ok(()))
}

/// [`reach_case`], calling `each` once at the level sync point with `None` and after every step
/// up to the case's frame with the input that step ran on.
fn reach_case_with(
    case: &Case,
    slack: u32,
    mut each: impl FnMut(&mut Amiga, Option<FrameInput>) -> Result<(), String>,
) -> Result<Amiga, String> {
    if !case.is_level() {
        return Err(format!("{}: not a level case", case.name));
    }
    let inputs = Inputs::load(case)?;
    let lead = case.sync_frame - sync::LEVEL_1_SYNC;
    let mut am = sync::reach_level_1(lead + case.frames + slack)?;
    each(&mut am, None)?;
    for frame in sync::LEVEL_1_SYNC..case.sync_frame {
        let input = inputs.at(frame);
        am.step(input).map_err(|e| e.to_string())?;
        each(&mut am, Some(input))?;
    }
    for (name, value) in &case.set {
        let v = layout_var(name).ok_or_else(|| format!("`{name}` is not in fd-core's LAYOUT"))?;
        let bytes = match v.width {
            Width::Byte => vec![*value as u8],
            Width::Word => (*value as u16).to_be_bytes().to_vec(),
            Width::Long => value.to_be_bytes().to_vec(),
        };
        am.poke(address(name)?, &bytes);
    }
    Ok(am)
}

/// `read_joystick`'s `move.w $dff00c.l, d0` (`h0+$8e1c`): `game_frame` reads JOY1DAT here once a
/// pass while not paused. Not the routine's entry, `h0+$8dfc`: a frame can end between the two, and
/// the pass then reads the next step's joystick (T48, measured on the route: step 12 287 entered
/// and step 12 288 read, and so on for six passes, the last across an input change).
pub const READ_JOYSTICK: (u8, u32) = (0, 0x8e1c);
/// `game_frame`'s paused branch, `h0+$200e` (`bsr.w play_keys` after `tst.w paused`): a paused
/// pass runs this and no `read_joystick`.
pub const PAUSED_BRANCH: (u8, u32) = (0, 0x200e);
/// `read_joystick` stops counting `input_log_index` here (`cmpi.l #$2ee0`).
const INPUT_LOG_FULL: u32 = 12000;

/// The input each pass of the original read, pass by pass, from T31's counter (T67).
///
/// Every pass of `game_frame` runs exactly one of [`READ_JOYSTICK`] and [`PAUSED_BRANCH`], and
/// passes run one after another (`level3_handler` is not re-entered), so the hits of the two, in
/// step order, are the passes' in pass order: the `k`th hit after the anchor is pass `anchor + k`'s,
/// whatever step that pass started in. While the camera scrolls a pass usually reads in the step
/// after its start (57 of 76 slide passes, T41), so the step a pass started in is not the one whose
/// joystick it saw.
///
/// A pass's keys are the keys of the step it started in (measured, T67: P held from step `N`, idle
/// or sliding, reached `pause_key_down` in the pass started in step `N`, whose `play_keys` ran in
/// step `N + 1` when sliding; `level2_handler` puts the byte in `last_key_byte` before that step's
/// level-3 interrupt). A paused pass reads no joystick: it gets the joystick of the step its paused
/// branch ran in, which nothing reads.
///
/// `Clone` keeps the count beside an oracle snapshot, for a tool that goes back to one.
#[derive(Debug, Clone)]
pub struct PassInputs {
    /// `irq_count` at the last step's end: passes up to it have started.
    started: u32,
    /// The last pass whose `game_frame` read the joystick or took the paused branch.
    decided: u32,
    /// `input_log_index` as the counter says it must be.
    log: u32,
    /// (pass, the input of the step it started in) for passes started and not yet decided.
    starts: Vec<(u32, FrameInput)>,
    /// (pass, its input) for passes decided and not yet taken.
    ready: Vec<(u32, FrameInput)>,
    pcs: [u32; 2],
}

impl PassInputs {
    /// Anchored at the original as it is now. It holds only where every pass so far read the
    /// joystick, the last started one included: `input_log_index` equals `irq_count` and nothing
    /// is paused. W1-1's sync point is such a place (measured: both 1).
    pub fn anchor(am: &Amiga) -> Result<PassInputs, String> {
        let var = |n: &str| layout_var(n).ok_or_else(|| format!("no {n} in LAYOUT"));
        let started = Live(am).value(var("irq_count")?);
        let log = Live(am).value(var("input_log_index")?);
        let paused = Live(am).value(var("paused")?);
        if log != started || paused != 0 {
            return Err(format!(
                "cannot anchor the input pairing: irq_count {started}, input_log_index {log}, \
                 paused {paused}; it needs every pass so far to have read the joystick"
            ));
        }
        Ok(PassInputs {
            started,
            decided: started,
            log,
            starts: Vec::new(),
            ready: Vec::new(),
            pcs: [READ_JOYSTICK, PAUSED_BRANCH].map(|(h, o)| HUNK_BASES[h as usize] + o),
        })
    }

    /// The addresses [`Amiga::watch_pcs`] must count for [`PassInputs::step`].
    pub fn pcs(&self) -> [u32; 2] {
        self.pcs
    }

    /// Account for the step just run on `input`. The counter must be watching [`PassInputs::pcs`].
    /// `input_log_index` checks the count: a read the counter missed, or one it saw twice, is an
    /// error, never a silent mispairing.
    pub fn step(&mut self, am: &Amiga, input: FrameInput) -> Result<(), String> {
        let var = |n: &str| layout_var(n).ok_or_else(|| format!("no {n} in LAYOUT"));
        // irq_count wraps at $10000, far past any level case.
        let now = Live(am).value(var("irq_count")?);
        self.starts
            .extend((self.started + 1..=now).map(|n| (n, input)));
        self.started = now;
        let hits = |pc: u32| {
            am.pc_hits()
                .iter()
                .find(|(p, _)| *p == pc)
                .map_or(0, |(_, n)| *n)
        };
        let (reads, paused) = (hits(self.pcs[0]), hits(self.pcs[1]));
        for _ in 0..reads + paused {
            self.decided += 1;
            let n = self.decided;
            let i = self
                .starts
                .iter()
                .position(|(p, _)| *p == n)
                .ok_or_else(|| format!("pass {n} read its input before it started"))?;
            let (_, start) = self.starts.remove(i);
            self.ready.push((
                n,
                FrameInput {
                    joy: input.joy,
                    keys: start.keys,
                },
            ));
        }
        self.log = (self.log + reads).min(INPUT_LOG_FULL);
        let log = Live(am).value(var("input_log_index")?);
        // A read made just before the step's end counts at `h0+$8e1c` in this step but adds
        // to `input_log_index` (`h0+$8eb4`) in the next: one behind is allowed only in a step that
        // counted a read, and the next step's check holds it to the count (T15, measured at frame
        // 7715 of a long varied recording, the character dying).
        let reading = reads > 0 && log + 1 == self.log && self.log < INPUT_LOG_FULL;
        if log != self.log && !reading {
            return Err(format!(
                "input_log_index is {log} on the original, the counter says {}: a joystick read \
                 was missed or counted twice",
                self.log
            ));
        }
        Ok(())
    }

    /// Forget passes up to `n`: the engine starts after them.
    pub fn drop_through(&mut self, n: u32) {
        self.ready.retain(|(p, _)| *p > n);
        self.starts.retain(|(p, _)| *p > n);
    }

    /// The pass whose `game_frame` reads the joystick next: the input fed in the next step is
    /// its joystick, if it reads in that step.
    pub fn next_read(&self) -> u32 {
        self.decided + 1
    }

    /// The pass the next step starts, if it starts one: its keys are that step's.
    pub fn next_start(&self) -> u32 {
        self.started + 1
    }

    /// Pass `n`'s input, once its `game_frame` has read it.
    pub fn take(&mut self, n: u32) -> Option<FrameInput> {
        let i = self.ready.iter().position(|(p, _)| *p == n)?;
        Some(self.ready.remove(i).1)
    }
}

/// [`reach_case`] with its passes' inputs counted from the level sync point, and the counter
/// watching `extra` as well (the coverage register), counts reset at the case's sync point.
pub fn reach_case_counting(
    case: &Case,
    slack: u32,
    extra: &[u32],
) -> Result<(Amiga, PassInputs), String> {
    let mut tracker: Option<PassInputs> = None;
    let am = reach_case_with(case, slack, |am, input| {
        match (input, &mut tracker) {
            (None, _) => {
                let t = PassInputs::anchor(am)?;
                let mut pcs = extra.to_vec();
                pcs.extend(t.pcs());
                am.watch_pcs(&pcs);
                tracker = Some(t);
            }
            (Some(input), Some(t)) => t.step(am, input)?,
            (Some(_), None) => unreachable!("anchored before the first step"),
        }
        Ok(())
    })?;
    Ok((am, tracker.expect("anchored at the sync point")))
}

/// Whether any pixel in exclusion `e`'s rows (and columns, for a box), clipped to both pictures,
/// differs. `ours` is row-major colours of a picture `width` wide.
fn differs_in(ours: &[u16], width: u16, theirs: &Rgb12Frame, e: &Exclusion) -> bool {
    let height = (ours.len() / width.max(1) as usize) as u16;
    let (w, h) = (width.min(theirs.width), height.min(theirs.height));
    let cols = e.cols.clone().unwrap_or(0..w);
    (e.rows.start..e.rows.end.min(h)).any(|y| {
        (cols.start..cols.end.min(w))
            .any(|x| ours[y as usize * width as usize + x as usize] != theirs.pixel(x, y))
    })
}

/// The original's frame with the engine's colours in every excluded box, so a box's differences
/// are left out of the judgement and the rest of its rows is still compared.
fn with_boxes_ours(
    theirs: &Rgb12Frame,
    ours: &[u16],
    width: u16,
    exclude: &[Exclusion],
) -> Rgb12Frame {
    let mut f = theirs.clone();
    let height = (ours.len() / width.max(1) as usize) as u16;
    let (w, h) = (width.min(f.width), height.min(f.height));
    for e in exclude.iter().filter(|e| e.var.is_none()) {
        let Some(cols) = &e.cols else { continue };
        for y in e.rows.start..e.rows.end.min(h) {
            for x in cols.start..cols.end.min(w) {
                f.pixels[y as usize * f.width as usize + x as usize] =
                    ours[y as usize * width as usize + x as usize];
            }
        }
    }
    f
}

const NO_PICTURE: &str = "the case compares the picture but the engine draws none";

/// Run a level case with `engine` on the engine's side.
///
/// The original's passes are numbered by its own `irq_count`, which `level3_handler` raises as a
/// pass starts: pass `n`'s joystick is the input of the step in which its `read_joystick` ran, and
/// its keys that of the step in which the count reached `n` ([`PassInputs`], T67); its
/// state is RAM at the pass's end ([`PASS_END`], `Amiga::watch_write`), captured in whichever step
/// the end falls. The engine is transplanted at the first pass end after the sync point and then
/// runs each later pass on that pass's input, so it is compared with the original pass for pass
/// whatever step a pass ends in. A step that ends two passes shows only the first; the other is
/// counted in [`Verdict::unseen`]. The picture of pass `n` is the oracle frame of the step its end
/// was seen in. `amiga_conform::compare::PassMatch` is not used: it pairs ends step by step, and
/// here the original numbers its own passes.
pub fn run_with(case: &Case, engine: &mut dyn Engine, opts: &Options) -> Result<Verdict, String> {
    run_inputs(case, &Inputs::load(case)?, engine, opts)
}

/// [`run_with`] on `inputs` rather than the scripts the case names: a bug zip's retimed inputs
/// (`bug::compare`), which are no file. The way to the sync point is still the case's.
pub fn run_inputs(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn Engine,
    opts: &Options,
) -> Result<Verdict, String> {
    if !case.is_level() {
        return Err(format!(
            "{}: run_with runs level cases; a title case runs with `run`",
            case.name
        ));
    }
    let vars = layout_vars(&case.vars)?;
    let goal: Vec<(&Var, u32)> = match &case.goal {
        Some(Goal::All { all, .. }) => all
            .iter()
            .map(|(v, value)| Ok((layout_vars(std::slice::from_ref(v))?[0], *value)))
            .collect::<Result<_, String>>()?,
        _ => Vec::new(),
    };
    let irq = layout_var("irq_count").ok_or("no irq_count in LAYOUT")?;
    let mut pcs: Vec<u32> = opts.register.iter().map(coverage::address).collect();
    let scene_pcs =
        [GAME_OVER_SCREEN, HIGH_SCORES, NEXT_WORLD].map(|(h, o)| HUNK_BASES[h as usize] + o);
    let world = layout_var("world_index").ok_or("no world_index in LAYOUT")?;
    pcs.extend(scene_pcs);
    if !pcs.contains(&Sampler::CAMERA_UPDATE) {
        pcs.push(Sampler::CAMERA_UPDATE);
    }
    let status_redraw = HUNK_BASES[STATUS_REDRAW.0 as usize] + STATUS_REDRAW.1;
    if !pcs.contains(&status_redraw) {
        pcs.push(status_redraw);
    }
    let records = records_edge_cuts(case);
    let (mut am, mut read) = reach_case_counting(case, 0, &pcs)?;
    am.watch_write(Some(PASS_END));

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    // Whole excluded rows; a box is left out by [`with_boxes_ours`].
    let rows: Vec<Range<u16>> = case
        .exclude
        .iter()
        .filter(|e| e.var.is_none() && e.cols.is_none())
        .map(|e| e.rows.clone())
        .collect();
    // (index in `excluded`, the variable) for each excluded variable.
    let excluded_vars: Vec<(usize, &Var)> = case
        .exclude
        .iter()
        .enumerate()
        .filter_map(|(i, e)| Some((i, layout_var(e.var.as_deref()?)?)))
        .collect();
    let tol = Tolerance::OWNER;
    let mut judge = PictureJudge::new(tol, case.frames);
    // (the pass the engine was transplanted at, the last pass it ran).
    let mut at: Option<(u32, u32)> = None;
    let mut passes_run = 0u32;

    // The engine's picture after each pass it has run, until the frame showing it is drawn, and its
    // pictures with the status bar as a cut redraw leaves it, if the original's was cut (T71).
    let mut drawn: Vec<(u32, Picture)> = Vec::new();
    let mut cut_drawn: Vec<(u32, Vec<Picture>)> = Vec::new();
    // Pass `32k + 1` and how many of `h0_6a96`'s reads the redraw it cut made before it.
    let mut cut: Option<(u32, Range<u8>)> = None;
    // `main_loop`'s calls of its status redraw since the last pass `32k` started, and the pass
    // `32k` after which it made none before pass `32k + 1` took it elsewhere (T82): that period's
    // frames show the bar as it stood, which the engine draws for them.
    let mut redraws = 0u32;
    let mut skipped: Option<u32> = None;
    let mut skip_drawn: Vec<(u32, Picture)> = Vec::new();
    // Each pass and where it cut `main_loop`'s edge redraws on the original, by the pass number,
    // from the step it started in to the step it ends in (T74).
    let mut edges_cuts: Vec<(u32, Option<EdgesCut>)> = Vec::new();
    // Which steps are sampled line by line ([`Sampler`]). Sampling costs about half a step again;
    // it is switched before a step, and switching leaves the frame just drawn as it was (T78).
    let mut sampler = Sampler::default();
    let mut profiling = false;
    // The last pass whose picture was compared.
    let mut shown_last: Option<u32> = None;
    // A pass and the engine's picture of it with the creature cuts the beam had passed on the
    // original left out (T80).
    let mut uncut_drawn: Option<(u32, Picture)> = None;
    let camera = ["camera_x", "camera_y"].map(|n| layout_var(n).expect("the camera in LAYOUT"));

    for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        // A step starts one pass, at line 233.
        let started = Live(&am).value(irq);
        let sample = sampler.samples(&am, case);
        if sample != profiling {
            am.set_profile(sample);
            profiling = sample;
        }
        am.step(input).map_err(|e| e.to_string())?;
        let now = Live(&am).value(irq);
        if now != started && now & 31 == 0 {
            redraws = 0;
        }
        redraws += am
            .pc_hits()
            .iter()
            .find(|h| h.0 == status_redraw)
            .map_or(0, |h| h.1);
        if sample && now == started.wrapping_add(1) {
            if case.compare.picture && now & 31 == 1 {
                let pc = interrupted_pc(&am);
                cut = pc.and_then(redraw_cut).map(|r| (now, r));
                // Never called, and the pass did not come between the turn's read of the count
                // and the call (`redraw_cut`'s first arm, which redraws after the pass).
                let never = pc.is_some() && cut.is_none() && redraws == 0;
                skipped = never.then_some(now - 1);
            }
            if records {
                edges_cuts.push((now, edges_cut(&am)));
            }
        }
        let frame = am.frame();
        // The cuts this frame saw of the pass that started in the step before (T80).
        let late_cuts = if case.compare.picture {
            creature_cuts(&am).unwrap_or_default()
        } else {
            Vec::new()
        };
        sampler.stepped(&am);
        verdict.coverage.add(opts.register, am.pc_hits());
        read.step(&am, input)?;
        if let Some(mem) = am.watched_memory() {
            let theirs = OracleMem(&mem);
            let ended = theirs.value(irq);
            match at {
                None => {
                    engine.transplant(&theirs);
                    at = Some((ended, ended));
                    read.drop_through(ended);
                    if case.compare.picture {
                        drawn.push((ended, engine.picture().ok_or(NO_PICTURE)?));
                    }
                }
                Some((first, last)) if ended > last => {
                    for n in last + 1..=ended {
                        let pass_input = read.take(n).ok_or_else(|| {
                            format!("pass {n} ended on the original but never read its input")
                        })?;
                        // A pass started in a step with another (none sampled) is taken as not cut.
                        // Not recorded, nothing is handed: the engine keeps its own rule.
                        if records {
                            let edges = edges_cuts.iter().find(|(p, _)| *p == n).and_then(|c| c.1);
                            edges_cuts.retain(|(p, _)| *p > n);
                            verdict.edge_cuts += edges.is_some() as u32;
                            engine.foreground_cut(edges);
                        }
                        engine.pass(pass_input);
                        passes_run += 1;
                        if engine.level_end() {
                            return Ok(scene_differs(
                                verdict,
                                frame,
                                "the engine found the level's end; the original ran a pass",
                            ));
                        }
                        if case.compare.picture {
                            drawn.push((n, engine.picture().ok_or(NO_PICTURE)?));
                            // This step's frame shows pass `n` where it started in the step before
                            // and ends in this one; the engine draws it as the beam found it.
                            uncut_drawn = None;
                            let mut behind = None;
                            if n == started && n == ended && !late_cuts.is_empty() {
                                let at = camera.map(|v| theirs.value(v) as u16);
                                let rows = rows_behind_beam(&late_cuts, (at[0], at[1]));
                                if rows.iter().any(|&b| b > 0) {
                                    uncut_drawn = engine.creatures_uncut(&rows).map(|p| (n, p));
                                    behind = Some(rows);
                                }
                            }
                            // Pass `32k` is drawn before the step that shows whether its redraw
                            // was reached; the frame's comparison decides whether to take it.
                            if (n & 31 == 0 || skipped.is_some_and(|k| (k..k + 32).contains(&n)))
                                && let Some(p) = engine.redraw_skipped(behind.as_deref())
                            {
                                skip_drawn.push((n, p));
                            }
                            if let Some((from, r)) = cut.as_ref()
                                && (*from..from + 31).contains(&n)
                            {
                                let pics: Vec<Picture> =
                                    r.clone().filter_map(|c| engine.cut_redraw(c)).collect();
                                if !pics.is_empty() {
                                    cut_drawn.push((n, pics));
                                }
                            }
                        }
                        for e in engine.events() {
                            verdict.events.push((n - first, e));
                            if let Event::Unported(value) = e {
                                // Every case fails on an object nobody ported (DESIGN §4.1).
                                verdict.outcome = Outcome::Unported {
                                    pass: n - first,
                                    frame,
                                    value,
                                };
                                return Ok(verdict);
                            }
                        }
                    }
                    verdict.unseen += ended - last - 1;
                    at = Some((first, ended));
                    let pass = ended - first;

                    if case.compare.vars {
                        for v in &vars {
                            let (ours, theirs) = (engine.probe(v), theirs.value(v));
                            if ours != theirs {
                                verdict.outcome = Outcome::Differs {
                                    pass,
                                    frame,
                                    var: v.name.to_string(),
                                    engine: ours,
                                    oracle: theirs,
                                };
                                return Ok(verdict);
                            }
                        }
                        for &(i, v) in &excluded_vars {
                            if engine.probe(v) != theirs.value(v) {
                                verdict.excluded[i].1 += 1;
                            }
                        }
                        verdict.passes += 1;
                    }
                    if let Some(Goal::All { by_frame, .. }) = &case.goal {
                        let reached = |v: &Var, value: u32| {
                            engine.probe(v) == value && theirs.value(v) == value
                        };
                        if verdict.goal_met.is_none()
                            && frame <= *by_frame
                            && goal.iter().all(|(v, value)| reached(v, *value))
                        {
                            verdict.goal_met = Some(frame);
                        }
                    }
                }
                Some(_) => {}
            }
        }

        // After the pass ends: pass 32k can end in the step `main_loop` then finds the game over in.
        let ran = |pc: u32| am.pc_hits().iter().any(|&(p, n)| p == pc && n > 0);
        let (entered, scores) = (ran(scene_pcs[0]), ran(scene_pcs[1]));
        if ran(scene_pcs[2]) {
            // `next_world` (T26): no pass runs after it; the engine steps on the step's input, must
            // find the level's end in it with `world_index` equal, and the comparison ends.
            engine.off_play(input);
            for e in engine.events() {
                verdict.events.push((at.map_or(0, |(f, l)| l - f), e));
            }
            if !engine.level_end() {
                return Ok(scene_differs(
                    verdict,
                    frame,
                    "the original ran `next_world`; the engine did not find the level's end",
                ));
            }
            let (ours, theirs) = (engine.probe(world), Live(&am).value(world));
            if ours != theirs {
                verdict.outcome = Outcome::Differs {
                    pass: at.map_or(0, |(f, l)| l - f),
                    frame,
                    var: world.name.to_string(),
                    engine: ours,
                    oracle: theirs,
                };
                return Ok(verdict);
            }
            verdict.level_end = Some(frame);
            break;
        }
        if verdict.game_over.is_some() || entered {
            // The game over screen (T23): no pass runs; the engine steps on the step's input and
            // must enter it, stay on it and leave it in the steps the original does.
            let first = verdict.game_over.is_none();
            if first && engine.game_over() {
                return Ok(scene_differs(
                    verdict,
                    frame,
                    "the engine went to the game over screen a step before the original",
                ));
            }
            engine.off_play(input);
            for e in engine.events() {
                verdict.events.push((at.map_or(0, |(f, l)| l - f), e));
            }
            if first {
                verdict.game_over = Some((frame, None));
            }
            if engine.game_over() == scores {
                let why = if scores {
                    "the original left the game over screen for the high scores; the engine did not"
                } else if first {
                    "the original went to the game over screen; the engine did not"
                } else {
                    "the engine left the game over screen; the original did not"
                };
                return Ok(scene_differs(verdict, frame, why));
            }
            if scores {
                verdict.game_over = verdict.game_over.map(|(from, _)| (from, Some(frame)));
                break;
            }
            // The step that entered the screen draws play above the copper's switch and the
            // screen below it: its frame is not compared.
            if case.compare.picture && !first {
                let ours = engine.picture().ok_or(NO_PICTURE)?;
                let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
                verdict.pictures += 1;
                let class = classify(&ours, &pic, &[], &tol);
                if class != PictureClass::Same {
                    let mut why = format!("the game over screen: {class:?}");
                    if let (Some(dir), Some(m)) = (&opts.diff_dir, picture_except(&ours, &pic, &[]))
                    {
                        let d = report::write_diff_png(dir, &case.name, frame, &ours, &pic, m);
                        why += &format!("; {}", d.path.display());
                    }
                    verdict.outcome = Outcome::PictureDiffers {
                        pass: at.map_or(0, |(f, l)| l - f),
                        frame,
                        why,
                    };
                    return Ok(verdict);
                }
            }
            continue;
        } else if scores {
            return Ok(scene_differs(
                verdict,
                frame,
                "the original entered its high scores without its game over screen",
            ));
        }
        // The frame this step drew shows the pass that started at its vertical blank: the last
        // pass started by the step's end less one (measured, T44: every pixel of scrolled frames
        // agrees so, and the frame of the step a pass ended in is a pass behind once passes run
        // past the frame's end). A pass no frame shows (a step starting two) is not compared.
        if !case.compare.picture {
            continue;
        }
        let Some((first, _)) = at else { continue };
        let shown = Live(&am).value(irq).wrapping_sub(1);
        if shown_last.is_some_and(|s| shown <= s) {
            continue;
        }
        let Some(k) = drawn.iter().position(|(n, _)| *n == shown) else {
            continue;
        };
        shown_last = Some(shown);
        let whole = drawn.swap_remove(k).1;
        drawn.retain(|(n, _)| *n > shown);
        // Where the original cut a creature behind the beam, the engine's picture is the one it
        // draws with the measured rows left out, and nothing else stands in for it.
        let uncut = uncut_drawn.take_if(|(n, _)| *n == shown).map(|(_, p)| p);
        verdict.creature_cuts += uncut.is_some() as u32;
        // Through a status period whose redraw the original never reached, the engine's picture is
        // the one with the bar as it stood (T82), on that period's frames and no other.
        let unredrawn = skip_drawn
            .iter()
            .position(|(n, _)| *n == shown)
            .map(|k| skip_drawn.swap_remove(k).1)
            .filter(|_| skipped.is_some_and(|k| (k..k + 32).contains(&shown)));
        skip_drawn.retain(|(n, _)| *n > shown);
        verdict.skipped_redraws += unredrawn.is_some() as u32;
        let ours = unredrawn.or(uncut).unwrap_or(whole);
        let cut_ours = cut_drawn
            .iter()
            .position(|(n, _)| *n == shown)
            .map(|k| cut_drawn.swap_remove(k).1)
            .unwrap_or_default();
        cut_drawn.retain(|(n, _)| *n > shown);
        let pass = shown - first;
        let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
        let colours = to_rgb12(&ours);
        // A box naming frames is left out on those frames alone (T48).
        let on = |e: &Exclusion| e.frames.as_ref().is_none_or(|f| f.contains(&frame));
        for (i, e) in case.exclude.iter().enumerate() {
            if e.var.is_none() && on(e) && differs_in(&colours, ours.width, &pic, e) {
                verdict.excluded[i].1 += 1;
            }
        }
        let boxes: Vec<Exclusion> = case.exclude.iter().filter(|e| on(e)).cloned().collect();
        let theirs = with_boxes_ours(&pic, &colours, ours.width, &boxes);
        let class = classify(&ours, &theirs, &rows, &tol);
        // The original's redraw was cut short: its bar may show what the engine's cut redraw draws,
        // and only the bar may differ (owner, 2026-11-06, T71).
        let bar_only = |alt: &Picture| {
            let n = STATUS_ROWS.start as usize * ours.width as usize;
            to_rgb12(alt)[..n] == colours[..n]
        };
        let cut_shows = class != PictureClass::Same
            && cut_ours.iter().any(|alt| {
                bar_only(alt) && classify(alt, &theirs, &rows, &tol) == PictureClass::Same
            });
        if cut_shows {
            verdict.cut_redraws += 1;
        }
        // The judge hears of every compared frame, as the toolkit counts (T82, owner 2026-11-10):
        // an identical frame, or one accepted as the measured cut redraw, ends a run of glitches.
        // Told only of differing frames it counted glitches thousands of frames apart as a run.
        let accepted = class == PictureClass::Same || cut_shows;
        let failed = match opts.mode {
            _ if accepted => judge.frame(frame, PictureClass::Same).err(),
            Mode::Strict => Some(format!("{class:?}, strict")),
            Mode::Faithful => judge.frame(frame, class).err(),
        };
        verdict.pictures += 1;
        verdict.tolerated = [
            judge.colour_step.len() as u32,
            judge.few_pixels.len() as u32,
            judge.glitches.len() as u32,
        ];
        if let Some(mut why) = failed {
            let shown = picture_except(&ours, &theirs, &rows);
            if let Some((x, y, o, e)) = shown.as_ref().and_then(|m| m.first) {
                why += &format!(
                    "; first at ({x}, {y}): {o:03x} in the original, {e:03x} in the engine"
                );
            }
            if let (Some(dir), Some(m)) = (&opts.diff_dir, shown) {
                let d = report::write_diff_png(dir, &case.name, frame, &ours, &pic, m);
                why += &format!("; {}", d.path.display());
            }
            verdict.outcome = Outcome::PictureDiffers { pass, frame, why };
            return Ok(verdict);
        }
    }

    let nothing = |what: &str| {
        Some(Outcome::ComparedNothing {
            what: what.to_string(),
        })
    };
    let failed = if at.is_none() {
        nothing("no pass of the original ended after the sync point")
    } else if case.compare.vars && verdict.passes == 0 {
        nothing("no pass ended on both sides, so no variable was compared")
    } else if case.compare.picture && verdict.pictures == 0 {
        nothing("no frame's picture was compared")
    } else {
        match &case.goal {
            Some(Goal::All { all, by_frame }) if verdict.goal_met.is_none() => {
                let want: Vec<String> = all.iter().map(|(v, n)| format!("{v} = {n}")).collect();
                Some(Outcome::GoalMissed {
                    goal: format!("{} by oracle frame {by_frame}", want.join(", ")),
                })
            }
            Some(Goal::GameOver { by_frame }) => match verdict.game_over {
                Some((_, Some(left))) if left <= *by_frame => {
                    verdict.goal_met = Some(left);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!(
                        "the game over screen left for the high scores by oracle frame {by_frame}"
                    ),
                }),
            },
            Some(Goal::LevelEnd { by_frame }) => match verdict.level_end {
                Some(end) if end <= *by_frame => {
                    verdict.goal_met = Some(end);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!("the level's end (`next_world`) by oracle frame {by_frame}"),
                }),
            },
            Some(Goal::Idle) => if let Some((pass, e)) = verdict.events.first() {
                Some(format!("the engine raised `{e:?}` at pass {pass}"))
            } else if verdict.unseen > 0 || verdict.passes != passes_run {
                Some(format!(
                    "{} of {passes_run} passes compared: every pass must be",
                    verdict.passes
                ))
            } else {
                None
            }
            .map(|why| Outcome::NotIdle { why }),
            _ => None,
        }
    };
    // Every exclusion names the task that removes it, and fails once it hides nothing
    // (porting-routines §4).
    let unused = || {
        verdict
            .excluded
            .iter()
            .find(|(_, n)| *n == 0)
            .map(|(e, _)| Outcome::ExclusionUnused {
                what: e.what.clone(),
                task: e.task.clone(),
            })
    };
    verdict.outcome = failed.or_else(unused).unwrap_or(Outcome::Same);
    Ok(verdict)
}

/// A scene difference ends the run.
fn scene_differs(mut verdict: Verdict, frame: u32, why: &str) -> Verdict {
    verdict.outcome = Outcome::SceneDiffers {
        frame,
        why: why.to_string(),
    };
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variable_resolves_to_the_address_the_map_records() {
        // docs/07-executable-map.md: "one byte, `$66c31` on the oracle".
        assert_eq!(address("mt_counter"), Ok(0x66c31));
        assert_eq!(address("char_x"), Ok(0xc0_77d8 + 0x1e5a));
    }

    #[test]
    fn a_difference_names_the_pass_the_frame_and_the_two_values() {
        let text = Outcome::Differs {
            pass: 3,
            frame: 4925,
            var: "char_x".into(),
            engine: 0x71,
            oracle: 0x70,
        }
        .to_string();
        for part in [
            "pass 3",
            "4925",
            "char_x",
            "113 (0x71) in the engine",
            "112 (0x70) in the original",
        ] {
            assert!(text.contains(part), "{text}");
        }
    }

    #[test]
    fn a_cut_counts_the_reads_h0_6a96_made_before_the_pass() {
        // `main_loop` before its `irq_count` read, or after `h0_6a96` returned: not cut.
        for pc in [0x252, 0x2a0, 0x2aa, 0x6ae6, 0x1eda] {
            assert_eq!(redraw_cut(pc), None, "h0+${pc:x}");
        }
        // Between the `irq_count` read and the `bsr`, or at `h0_6a96`'s first read: none made.
        for pc in [0x258, 0x282, 0x29c, 0x6a96] {
            assert_eq!(redraw_cut(pc), Some(0..1), "h0+${pc:x}");
        }
        // At a read: those before it; after a read, its number may be drawn either way.
        assert_eq!(redraw_cut(0x6aaa), Some(1..2));
        assert_eq!(redraw_cut(0x6ad2), Some(3..4));
        assert_eq!(redraw_cut(0x6a9c), Some(0..2));
        assert_eq!(redraw_cut(0x6ac4), Some(2..4));
        assert_eq!(redraw_cut(0x6ae2), Some(3..5));
        // Inside a draw routine: any.
        assert_eq!(redraw_cut(0x6b82), Some(0..5));
    }

    /// Frame 6779 of `mechanic-spring` as measured (docs/10): five cuts from line 31 to 110 under a
    /// camera at `$dc9,$3be`. The two creatures at the view's top are cut as the beam passes them.
    #[test]
    fn the_rows_behind_the_beam_are_those_written_after_their_line_was_fetched() {
        let line = |v: u32, h: u32| v * LINE_CYCLES + h;
        let top = Scroll::from_camera(0xdc9, 0x3be).top;
        // A creature whose first line is display row `row`, cut from `begin` to `end`.
        let cut = |row: u32, begin: u32, end: u32| CutSeen {
            dst: (top + row * SCREEN_LINE + 20) % (0x100 * SCREEN_LINE),
            rows: CUT_ROWS,
            begin,
            end,
        };
        let camera = (0xdc9, 0x3be);
        // Cut before the play area's first line: all of it shows.
        let early = cut(100, line(31, 47), line(39, 192));
        // Cut after the beam left its 32 lines (rows 3 to 34, lines 50 to 81): none of it shows.
        let late = cut(3, line(94, 76), line(110, 0));
        // Begun on its ninth line (row 1, line 48, at line 56): the blitter, two lines of the
        // creature to the beam's one, overtakes the beam a few lines on.
        let part = cut(1, line(56, 76), line(72, 196));
        // Below the play area's last line (row 179 on): never fetched.
        let below = cut(180, line(200, 0), line(216, 0));
        let got = rows_behind_beam(&[early, late, part, below], camera);
        assert_eq!((got[0], got[1], got[3]), (0, CUT_ROWS, 0));
        assert!((60..=100).contains(&got[2]), "{got:?}");
        // A creature wrapping past the screen's last line is two blits and one count.
        let first = CutSeen { rows: 50, ..late };
        let rest = CutSeen {
            dst: late.dst % SCREEN_LINE,
            rows: CUT_ROWS - 50,
            ..late
        };
        assert_eq!(rows_behind_beam(&[first, rest], camera).len(), 1);
    }

    #[test]
    fn oracle_memory_reads_chip_and_slow_ram_by_hunk() {
        let mut m = AmigaMemory {
            chip: vec![0; 0x8_0000],
            slow: vec![0; 0x8_0000],
        };
        m.slow[0x77d8 + 0x1e5a..][..4].copy_from_slice(&[0, 0, 0x12, 0x34]);
        m.chip[0x4_d668 + 0x195c9] = 5;
        let mem = OracleMem(&m);
        assert_eq!(mem.value(layout_var("char_x").unwrap()), 0x1234);
        assert_eq!(mem.value(layout_var("mt_counter").unwrap()), 5);
    }

    /// Where `main_loop`'s redraw half was when the pass came, read as T74 measured it: the PC
    /// within `main_loop` itself, else the redraw named by the PC or by its return address on the
    /// foreground's stack and, once its scan has started (T81), the scan's cell from `a2` (or
    /// `d5 + d6` at the loop's top).
    #[test]
    fn a_pass_inside_the_edge_redraws_names_the_next_latch_and_the_scans_cell() {
        let h = |o: u32| HUNK_BASES[0] + o;
        let mut regs = [0u32; 15];
        // The sample a line before: the same place, unless a check says otherwise.
        let at = |pc, returns: &[u32], regs: &[u32; 15]| edges_cut_at(pc, pc, returns, regs);
        // Before the scan and in none of the shapes' code: no shape copied yet.
        let cut = |next, defer_from| {
            Some(EdgesCut {
                next,
                defer_from,
                shape_words: (defer_from == Some(0)).then_some(0),
                add: AddCut::Whole,
            })
        };
        // Spinning, or in the status bar's redraw before the redraw half: no cut.
        assert_eq!(at(0x2a6, &[0x352], &regs), None);
        assert_eq!(at(0x6a9c, &[0x352], &regs), None);
        assert_eq!(at(0x358, &[], &regs), None);
        // In `main_loop` between the tests: a PC at a `clr.w` has not run it.
        assert_eq!(at(0x2b2, &[], &regs), cut(0, None));
        assert_eq!(at(0x2d0, &[], &regs), cut(0, None));
        assert_eq!(at(0x2d6, &[], &regs), cut(1, None));
        assert_eq!(at(0x348, &[], &regs), cut(3, None));
        assert_eq!(at(0x352, &[], &regs), cut(4, None));
        // In a redraw's tiles: `a2` holds a band table, so its scan has not started.
        regs[10] = h(0x1496);
        assert_eq!(at(0x774, &[0x7c6, 0x352], &regs), cut(4, Some(0)));
        // Pass 495 of `mechanic-spring`: back from `h0_6882`'s `st.b`, the cell's add made.
        regs[10] = h(OBJECT_MAP + 0x3005);
        assert_eq!(
            at(0x69a4, &[0x138c, 0x352], &regs),
            cut(4, Some(OBJECT_MAP + 0x3006))
        );
        // Inside `h0_6882` before it, or at the cell's compares: the cell's add comes after.
        assert_eq!(
            at(0x693e, &[0x352], &regs),
            cut(4, Some(OBJECT_MAP + 0x3005))
        );
        assert_eq!(
            at(0x1356, &[0x352], &regs),
            cut(4, Some(OBJECT_MAP + 0x3005))
        );
        // In the shapes (T48): 16 words a shape copied, and the words of the one being copied.
        let shapes = |next, n| {
            Some(EdgesCut {
                next,
                defer_from: Some(0),
                shape_words: Some(n),
                add: AddCut::Whole,
            })
        };
        let mut r = [0u32; 15];
        // The down row: before its loop, at the fourth shape's `bsr.w`, after it, in its copy with
        // three words made, after the loop, in the 23rd's copy, and after it.
        r[0] = 0x15 - 3;
        assert_eq!(at(0x920, &[0x2da], &r), shapes(1, 0));
        assert_eq!(at(0x940, &[0x2da], &r), shapes(1, 3 * 16));
        assert_eq!(at(0x944, &[0x2da], &r), shapes(1, 4 * 16));
        assert_eq!(at(0x7da, &[0x944, 0x2da], &r), shapes(1, 3 * 16));
        assert_eq!(at(0x7e8, &[0x944, 0x2da], &r), shapes(1, 3 * 16 + 3));
        assert_eq!(at(0x836, &[0x944, 0x2da], &r), shapes(1, 4 * 16));
        r[0] = 0xffff;
        assert_eq!(at(0x960, &[0x2da], &r), shapes(1, 22 * 16));
        assert_eq!(at(0x7dc, &[0x96c, 0x2da], &r), shapes(1, 22 * 16 + 1));
        assert_eq!(at(0x96c, &[0x2da], &r), shapes(1, 23 * 16));
        // The left column: `d2`'s low six bits name the shape; past its `bsr.w` and short of the
        // `addi.l` one more is copied.
        r[2] = 0x2c0 + 4 * 5;
        assert_eq!(at(0xf0a, &[0x32a], &r), shapes(3, 0));
        assert_eq!(at(0xf3c, &[0x32a], &r), shapes(3, 5 * 16));
        assert_eq!(at(0xf40, &[0x32a], &r), shapes(3, 6 * 16));
        assert_eq!(at(0x7e2, &[0xf40, 0x32a], &r), shapes(3, 5 * 16 + 2));
        r[2] = 0x2c0 + 4 * 15;
        assert_eq!(at(0xfb2, &[0x32a], &r), shapes(3, 15 * 16));
        assert_eq!(at(0xfb6, &[0x32a], &r), shapes(3, 16 * 16));
        // A bob's blit in the down row: the row's cell is done.
        assert_eq!(
            at(0x3aca, &[0xa5e, 0x9c6, 0x2da], &regs),
            cut(1, Some(OBJECT_MAP + 0x3006))
        );
        // In the down row's creature branch, or in `h0_681c` and `h0_6852` short of the slot's
        // `st.b`: the creature starts after the pass (T75). Back from it, it has started.
        for pc in [0xabc, 0xb10, 0x681c, 0x6862, 0x687a] {
            assert_eq!(
                at(pc, &[0x2da], &regs),
                cut(1, Some(OBJECT_MAP + 0x3005)),
                "{pc:#x}"
            );
        }
        for pc in [0x687c, 0x6836, 0xb14, 0xb1a] {
            assert_eq!(
                at(pc, &[0x2da], &regs),
                cut(1, Some(OBJECT_MAP + 0x3006)),
                "{pc:#x}"
            );
        }
        // Each scan has its own creature branch, and a PC in a redraw's own code names the redraw
        // whatever the stack a line before held.
        assert_eq!(
            at(0x113a, &[0x32a], &regs),
            cut(3, Some(OBJECT_MAP + 0x3005))
        );
        assert_eq!(
            at(0x113a, &[0x2da], &regs),
            cut(3, Some(OBJECT_MAP + 0x3005))
        );
        assert_eq!(edges_cut_at(0x852, 0x2d6, &[], &regs), cut(1, Some(0)));
        // Before the scan `a2` is the scan's before it (T81). Pass 331 of `mechanic-vanish`: in
        // the down row's shape copy, `a2` where a column scan stopped; and anywhere in the row's
        // own code below its loop's top.
        // `d0` is 0 here: the loop's 22nd shape, 21 copied; past the loop 22, past the last 23.
        assert_eq!(
            edges_cut_at(0x7da, 0x7d8, &[0x944, 0x2da], &regs),
            shapes(1, 21 * 16)
        );
        for (pc, n) in [(0x844, 0), (0x904, 0), (0x968, 22 * 16), (0x988, 23 * 16)] {
            assert_eq!(at(pc, &[0x2da], &regs), shapes(1, n), "{pc:#x}");
        }
        assert_eq!(
            at(0x98a, &[0x2da], &regs),
            cut(1, Some(OBJECT_MAP + regs[5] + regs[6]))
        );
        // The raster wait is a tile's, before the scan, while the sample a line before was at the
        // tiles; from the shapes on it is a bob's, in the scan.
        assert_eq!(at(0x3aca, &[0x788, 0x8e2, 0x2da], &regs), cut(1, Some(0)));
        assert_eq!(
            edges_cut_at(0x3aca, 0x8e8, &[0x2da], &regs),
            cut(1, Some(0))
        );
        assert_eq!(
            edges_cut_at(0x3aca, 0x7dc, &[0x96c, 0x2da], &regs),
            cut(1, Some(OBJECT_MAP + 0x3006))
        );
        // At the up row's loop top `a2` is being rebuilt: the next cell is `h0+$cb38 + d5 + d6`.
        (regs[5], regs[6]) = (0x60 * 7, 0x21);
        assert_eq!(
            at(0xc72, &[0x302], &regs),
            cut(2, Some(OBJECT_MAP + 0x60 * 7 + 0x21))
        );
    }
}
