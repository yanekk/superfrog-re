//! Sync points: where the original and the engine start a case together (DESIGN §2.1).
//!
//! The original is brought to the sync point on the oracle by its own input, from power-on, and
//! the engine starts from the original's memory there (`Start::Transplant`). Reaching a sync point
//! is a long run, so the oracle's snapshot there is cached outside git, keyed by everything that
//! decides it (`~/.cache/superfrog-re/sync/`, `SUPERFROG_CACHE` replacing the root).

use crate::freeze::Freeze;
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sf_assets::{LevelId, World};
use sf_core::input::{FrameInput, Joy, KeySet};
use sf_core::layout::{AmigaMemory, Ram, h0};
use sf_core::{HandlerEnd, Pacing, Rng};
use sf_disk::{DiskSlot, sha1_hex};
use sf_oracle::{Amiga, CpuRegs, OracleConfig, Snapshot};

use crate::case::script_path;
use crate::script::InputScript;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPoint {
    /// Switch on with disk 1 in df0; the engine starts with the oracle's RNG seed (owner,
    /// 2026-10-11).
    PowerOn,
    /// Switch on with disk 3 in df0: the Team 17 logo and the story intro, then the game once
    /// disk 1 is in (it goes in when the intro asks for it). The engine starts with the seed the
    /// game takes after the intro.
    PowerOnDisk3,
    /// Run the named script from power-on for `frame` frames.
    AtFrame { script: String, frame: u32 },
    /// The first frame of a level, reached the way a player would: W1-1 by START GAME from the
    /// title menu (`scripts/start-w1-1.txt`), every other level of W1–W6 and the bonus level by
    /// typing its level code on the options screen, inserting each disk the game asks for. W6-2..4
    /// load their first map set only with a second drive connected, so those are reached with df1
    /// connected and their `alt` maps on one drive. The final level, which has no code, is reached
    /// by forcing W6-4's exit (docs/12 "Reaching every level").
    AtLevel(LevelId),
    /// The end sequence's start (T35): the final level reached as `AtLevel` reaches it, its exit
    /// forced on its first frame the same way (`level_state` = 1), and the original run on, every
    /// disk it asks for inserted, to the end of the first frame on which its main program is in
    /// `end_seq_load` (top_loop's call, return address h0+$7b6).
    EndSequence,
    /// The slot machine's start (T71): W1-1 reached as `AtLevel` reaches it, by the `start` script
    /// (`start-w1-1` when `None`), `script` played in W1-1 to its last line's frame, then the
    /// level's exit forced (`level_state` = 1, as for the final level); in `level_tally`'s choice up
    /// is held until the slot machine is picked, then fire pressed on every 4th frame counted from the
    /// forced exit. The sync point
    /// is the end of the first frame on which the main program is in `slot_machine` (its return
    /// address, h0+$478e, on the stack).
    Slot {
        script: String,
        start: Option<String>,
    },
    /// A level's first frame as [`SyncPoint::AtLevel`] reaches it, its clock (`level_clock`,
    /// h0+$876) then set to `seconds` (T37: a time-up case without waiting four minutes).
    Clock { level: LevelId, seconds: u16 },
    /// Game over after the last life (T34): W1-1 reached as [`SyncPoint::AtLevel`] reaches it,
    /// `frog_lives` (h0+$82e) set to 0 on its first frame, `script` played from there until the
    /// frog's death has ended the level's loop and the original's main program is in
    /// `game_over_seq` (h0+$7600 on its stack). The sync point is the end of the frame before:
    /// the level is transplanted, and the case's first frame is the one game over begins in.
    GameOver { script: String },
}

/// The oracle at a sync point.
pub struct Synced {
    pub amiga: Amiga,
    /// The original's memory there.
    pub memory: AmigaMemory,
    /// For [`SyncPoint::PowerOn`]: the RNG state just after the original seeded it, if it did
    /// within the case.
    pub seed: Option<Rng>,
}

/// Bump when the cache file layout changes.
const SYNC_FORMAT: u32 = 1;
const MAGIC: &[u8; 8] = b"SFSYNC\0\0";

/// The level-3 interrupt vector holds `vbl_handler` once the game owns the machine.
const VBL_HANDLER: u32 = h0(0x11bf8);
/// `level_frame_loop`'s wait for `vbl_count` (docs/08).
const LEVEL_WAIT: Range<u32> = h0(0xaae)..h0(0xab8);
const LOGIC_RUNNING: u32 = h0(0x9d6);
const LEVEL_STATE: u32 = h0(0xa06);
/// `level_clock` (h0+$876): four digits, minutes first.
const LEVEL_CLOCK: u32 = h0(0x876);
/// `frog_lives` (h0+$82e).
const FROG_LIVES: u32 = h0(0x82e);
/// `level_exit_fade`'s return address from `game_over_seq`.
const IN_GAME_OVER: u32 = h0(0x7600);
/// Frames of W1-1 the game-over sync point's script has to lose the last life in.
const GAME_OVER_SEARCH: u32 = 3_000;
const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);
const RNG_STATE: u32 = h0(0x160d4);
/// W1-1 is playable by frame ~7 290 of `start-w1-1` (T07); stop looking well after.
const LEVEL_SEARCH: u32 = 9_000;
/// The password path loads up to two worlds' files across disk swaps: W6-4 is playable by about
/// frame 12 000.
const PASSWORD_SEARCH: u32 = 20_000;
/// `password_table` (h0+$24dc): 28 entries of six ASCII digits and a handler; W1-1..W6-4 in order,
/// then the bonus level's.
const PASSWORD_TABLE: u32 = h0(0x24dc);
/// The disk ID `disk_request` is looking for (h0+$af54): `2222`..`5555` for disks 1–4.
const WANTED_DISK: u32 = h0(0xaf54);
/// Set by `world_load_lw` and `level_init_lw` for the final level.
const FINAL_LEVEL_FLAG: u32 = h0(0x1e);
/// The final level follows W6-4's tally, the slot machine's chance and the LW files' load.
const FINAL_SEARCH: u32 = 30_000;
/// With df1 connected the boot reaches the title 219 frames sooner (measured on the oracle, 2026-10-12);
/// every input from the title on comes that much earlier.
const DF1_LEAD: u32 = 219;
/// The script [`SyncPoint::Slot`] reaches W1-1 by when it names none.
pub const DEFAULT_START: &str = "start-w1-1";
/// `attract_mode` (h0+$18): set while the attract demo plays, which a late START GAME reaches.
const ATTRACT_MODE: u32 = h0(0x18);
/// `level_tally`'s choice between points and the slot machine: its loop polls the joystick from
/// h0+$467e to h0+$46e2, and h0+$4798 is 1 once up picked the slot.
const TALLY_WAIT: Range<u32> = h0(0x467e)..h0(0x46e2);
const TALLY_SLOT: u32 = h0(0x4798);
/// `level_tally`'s call of `slot_machine` returns here.
pub const IN_SLOT_MACHINE: u32 = h0(0x478e);
/// The slot machine is entered 73 frames after the forced exit (measured, T19); stop looking well
/// after.
const SLOT_SEARCH: u32 = 1_000;
/// `top_loop`'s call of `end_seq_load` returns here.
const IN_END_SEQ_LOAD: u32 = h0(0x7b6);
/// The final level's forced exit reaches `end_seq_load` about 80 frames on.
const END_SEARCH: u32 = 1_000;
/// Title menu, OPTIONS, three times down to LEVEL CODE; the first is the crack intro's click.
const MENU: [(u32, &str); 14] = [
    (1000, "M"),
    (1005, "-"),
    (4600, "F"),
    (4605, "-"),
    (4650, "D"),
    (4655, "-"),
    (4700, "F"),
    (4705, "-"),
    (4800, "D"),
    (4810, "-"),
    (4840, "D"),
    (4850, "-"),
    (4880, "D"),
    (4890, "-"),
];
const TYPE_FROM: u32 = 4980;

fn word(m: &AmigaMemory, a: u32) -> u32 {
    (m.byte(a) as u32) << 8 | m.byte(a + 1) as u32
}

fn long(m: &AmigaMemory, a: u32) -> u32 {
    word(m, a) << 16 | word(m, a + 2)
}

/// The oracle's CPU as the engine's type.
pub fn core_cpu(c: &CpuRegs) -> sf_core::Cpu {
    sf_core::Cpu {
        d: c.d,
        a: c.a,
        pc: c.pc,
        sr: c.sr,
        usp: c.usp,
        ssp: c.ssp,
    }
}

/// The oracle's memory as the engine's type.
pub fn core_memory(m: sf_oracle::AmigaMemory) -> AmigaMemory {
    AmigaMemory {
        chip: m.chip,
        slow: m.slow,
    }
}

fn in_level(m: &AmigaMemory) -> bool {
    word(m, LOGIC_RUNNING) != 0 && (long(m, LEVEL_STATE) as i32) < 0
}

/// How the original's main loop kept time on the frame that just ended (owner, 2026-10-11; docs/08
/// "Lag"): late when, in a level, the frame ended with the main loop in the middle of a pass
/// rather than parked in `level_frame_loop`'s wait, so the next vertical blank comes in the
/// middle of it (T36); the frame's Paula writes say where ([`Pacing::Late`]). In the story
/// intro, late while the original is in a long job at the blank (owner, 2026-10-12). Every
/// other screen paces on a raster wait and is never late.
pub fn pacing(cpu: &CpuRegs, m: &AmigaMemory, writes: &[sf_paula::PaulaWrite]) -> Pacing {
    let last = writes.last().copied().unwrap_or_default();
    let late = |phase, step| Pacing::Late {
        writes: writes.len() as u16,
        line: last.vpos,
        hpos: last.hpos,
        phase,
        step,
    };
    if long(m, 0x6c) == INTRO_VBL {
        return if sf_core::front::story::intro::busy(&main_chain(cpu, m)) {
            late(0, sf_core::level::beam::NO_STEP)
        } else {
            Pacing::OnTime
        };
    }
    if mid_pass(cpu, m) {
        late(pass_phase(cpu, m), pass_step(cpu, m))
    } else {
        Pacing::OnTime
    }
}

/// The return addresses of `level_frame_loop`'s calls in its pass, in order (h0+$abe..$b00).
const PASS_CALLS: [u32; 13] = [
    0xac2, 0xac8, 0xace, 0xad4, 0xada, 0xae0, 0xae6, 0xaec, 0xaf0, 0xaf6, 0xafa, 0xb00, 0xb06,
];

/// Which of the pass's calls the main program is in at the frame's end: the call whose return
/// address is on its stack, or, between calls, the next one (13 past the last).
pub fn pass_phase(cpu: &CpuRegs, m: &AmigaMemory) -> u8 {
    let chain = main_chain(cpu, m);
    let body = h0(0xab8)..h0(0xb66);
    for (k, &a) in chain.iter().enumerate() {
        if !body.contains(&a) {
            continue;
        }
        let at = a - h0(0);
        let mut calls = PASS_CALLS.iter();
        return if k > 0 {
            calls.position(|&r| r == at).unwrap_or(13) as u8
        } else {
            calls.position(|&r| r > at).unwrap_or(13) as u8
        };
    }
    13
}

/// Whether the original's frame ended with its main loop in the middle of a level's pass (not
/// parked in `level_frame_loop`'s wait): its memory is then part way through what the engine does
/// in one go, a lag the runner reports and re-syncs (DESIGN §2.1; owner, 2026-10-11).
pub fn mid_pass(cpu: &CpuRegs, m: &AmigaMemory) -> bool {
    let main_pc = if cpu.pc == VBL_HANDLER {
        long(m, cpu.ssp + 2)
    } else {
        cpu.pc
    };
    // `game_over_seq` runs the level's logic outside its loop (T34): its passes are front-end
    // work, whose memory the engine reports as jobs.
    in_level(m) && !LEVEL_WAIT.contains(&main_pc) && !main_chain(cpu, m).contains(&IN_GAME_OVER)
}

/// How far the loop of the pass's call the main program is in had got at the frame's end, as
/// the engine counts its steps ([`sf_core::level::LevelCtx::step_done`]): the slots
/// `obj_run_all` has run (one whose update was running counted done), the slots
/// `obj_build_hitboxes` has looked at, the entries of `build_bob_draw_list`'s three loops, the
/// halves of `collide_all` (the spud's, the frog's). [`NO_STEP`] in any other call.
pub fn pass_step(cpu: &CpuRegs, m: &AmigaMemory) -> u16 {
    use sf_core::level::beam::NO_STEP;
    let (pc, sp) = if cpu.pc == VBL_HANDLER {
        (long(m, cpu.ssp + 2), cpu.ssp + 6)
    } else {
        (cpu.pc, cpu.a[7])
    };
    let at = pc.wrapping_sub(h0(0));
    let d = |r: usize| cpu.d[r] as u16;
    let slot = |down: u16| 20u16.saturating_sub(down);
    // A return address on the stack (and where).
    let ret = |r: u32| (0..0x100).step_by(2).map(|k| sp + k).find(|&a| long(m, a) == h0(r));
    match at {
        0x1b324..=0x1b32d => 0,
        0x1b32e..=0x1b33d => slot(d(0)),
        0x1b33e..=0x1b34d => slot(d(0)) + 1,
        0xd2f6..=0xd303 => 0,
        0xd304..=0xd33f => slot(d(7)),
        0xd340..=0xd34d => slot(d(7)) + 1,
        0x19b42..=0x19b55 => 0,
        0x19b56..=0x19b63 => slot(d(0)),
        0x19b64..=0x19b6d => slot(d(0)) + 1,
        0x19b6e..=0x19b75 => 20,
        0x19b76..=0x19b83 => 20 + slot(d(0)),
        0x19b84..=0x19b8d => 21 + slot(d(0)),
        0x19b8e..=0x19b95 => 40,
        0x19b96..=0x19bc0 => 40 + slot(d(0)).saturating_sub(1),
        0xd0cc => 0,
        0xd0d0 => 1,
        0xd0d4..=0xd0df => 2,
        _ => {
            // Inside a routine a loop called: the slot whose update runs (`obj_run_all` saved
            // d0 under the return address), or the half of `collide_all`.
            if let Some(a) = ret(0x1b33e) {
                return slot(word(m, a + 6) as u16) + 1;
            }
            if ret(0xd0d0).is_some() {
                return 0;
            }
            if ret(0xd0d4).is_some() {
                return 1;
            }
            NO_STEP
        }
    }
}

/// Where the main program was at the end of a frame: its PC, then each long on its stack that is
/// a return address into the game's code (a long just after a `jsr`/`bsr`), innermost first. For
/// tracing which loop of a scene a frame belongs to.
pub fn main_chain(cpu: &CpuRegs, m: &AmigaMemory) -> Vec<u32> {
    // A step ends at the level-3 handler's entry (the game's, the story intro's or Kickstart's
    // while the disk-3 boot runs); the interrupted PC is on the supervisor stack.
    let vector = long(m, 0x6c);
    let (pc, mut sp) = if cpu.pc == vector {
        (long(m, cpu.ssp + 2), cpu.ssp + 6)
    } else {
        (cpu.pc, cpu.a[7])
    };
    // Return addresses are looked for in the running program's code: the game's, else the story
    // intro's and the disk-3 boot's.
    let code: [std::ops::Range<u32>; 2] = if vector == VBL_HANDLER {
        [h0(0)..h0(0x4834c), 0..0]
    } else {
        [
            h0(0)..h0(INTRO_LEN),
            DISK3_BOOT_BASE..DISK3_BOOT_BASE + DISK3_BOOT_LEN,
        ]
    };
    let mut chain = vec![pc];
    let top = (sp & !0xFFFF) + 0x1_0000;
    while sp + 4 <= top.min(sp + 0x400) && chain.len() < 8 {
        let a = long(m, sp);
        // A return address follows a jsr abs.l (6 bytes, $4eb9) or a bsr.w (4 bytes, $6100).
        let call = code.iter().any(|c| c.contains(&a))
            && (word(m, a - 6) == 0x4eb9 && long(m, a - 4) >= h0(0)
                || word(m, a - 6) == 0x4eb9 && code[1].contains(&long(m, a - 4))
                || word(m, a - 4) == 0x6100);
        if call {
            chain.push(a);
        }
        sp += 2;
    }
    chain
}

/// Bring the oracle to `sync`, with room for `frames` more frames of `script` (the case's own,
/// which from power-on decides when the original seeds its RNG).
pub fn reach(sync: &SyncPoint, script: &InputScript, frames: u32) -> Result<Synced, String> {
    reach_frozen(sync, script, frames, crate::freeze::Freeze::None)
}

/// One frame of the original, its objects frozen out first when `frozen` ([`crate::freeze`]).
fn step(a: &mut Amiga, input: FrameInput, frozen: Freeze) -> Result<(), String> {
    frozen.apply(a, &crate::display_data());
    a.step(input).map(|_| ()).map_err(|e| e.to_string())
}

/// [`reach`], with the original's objects frozen out from power-on when `frozen` (a separate
/// snapshot in the cache).
pub fn reach_frozen(
    sync: &SyncPoint,
    script: &InputScript,
    frames: u32,
    frozen: Freeze,
) -> Result<Synced, String> {
    let tag = frozen.tag();
    match sync {
        SyncPoint::PowerOn | SyncPoint::PowerOnDisk3 => {
            let boot = boot_disk(sync);
            let seed = power_on_seed(boot, script, frames)?;
            let amiga = power_on(boot, frames)?;
            let _ = frozen;
            let memory = core_memory(amiga.memory());
            Ok(Synced {
                amiga,
                memory,
                seed,
            })
        }
        SyncPoint::AtFrame { script, frame } => {
            let s = InputScript::load(&script_path(script))?;
            let key = format!("{tag}frame {frame} script\n{}", s.to_oracle().canonical());
            at_snapshot(&key, 1, frames, *frame, |a| {
                for f in 0..*frame {
                    step(a, s.input_at(f), frozen)?;
                }
                Ok(())
            })
        }
        SyncPoint::AtLevel(id) if (id.world, id.level, id.alt) != (World::W1, 1, false) => {
            by_password(*id, frames, frozen)
        }
        SyncPoint::EndSequence => end_sequence(frames),
        SyncPoint::Slot { script, start } => slot(script, start.as_deref(), frames),
        SyncPoint::Clock { level, seconds } => {
            let mut synced = reach_frozen(&SyncPoint::AtLevel(*level), script, frames, frozen)?;
            let (m, s) = (seconds / 60, seconds % 60);
            let digits = [(m / 10) as u8, (m % 10) as u8, (s / 10) as u8, (s % 10) as u8];
            synced.amiga.write_memory(LEVEL_CLOCK, &digits);
            synced.memory = core_memory(synced.amiga.memory());
            Ok(synced)
        }
        SyncPoint::GameOver { script } => {
            let w1_1 = LevelId {
                world: World::W1,
                level: 1,
                alt: false,
            };
            let s = InputScript::load(&script_path(script))?;
            let at_level = || -> Result<Synced, String> {
                let mut synced = reach_frozen(
                    &SyncPoint::AtLevel(w1_1),
                    &InputScript::idle(),
                    frames,
                    frozen,
                )?;
                synced.amiga.write_memory(FROG_LIVES, &[0, 0]);
                Ok(synced)
            };
            // The frame game over begins in, then the original again to the frame before it.
            let mut synced = at_level()?;
            let a = &mut synced.amiga;
            let begins = (0..GAME_OVER_SEARCH)
                .find(|&f| {
                    step(a, s.input_at(f), frozen).is_ok()
                        && main_chain(&a.cpu(), &core_memory(a.memory())).contains(&IN_GAME_OVER)
                })
                .ok_or(format!(
                    "{script}: game_over_seq not reached in {GAME_OVER_SEARCH} frames of W1-1 with no lives left"
                ))?;
            let mut synced = at_level()?;
            for f in 0..begins {
                step(&mut synced.amiga, s.input_at(f), frozen)?;
            }
            synced.memory = core_memory(synced.amiga.memory());
            Ok(synced)
        }
        SyncPoint::AtLevel(_) => {
            let s = InputScript::load(&script_path(DEFAULT_START))?;
            at_snapshot(&w1_1_key(&tag, &s), 1, frames, LEVEL_SEARCH, |a| {
                to_w1_1(a, &s, frozen)
            })
        }
    }
}

/// The snapshot key of W1-1 reached by the menu script `start`: for `start-w1-1` the key
/// `AtLevel(W1-1)` has always had, so its snapshots are found again.
fn w1_1_key(tag: &str, start: &InputScript) -> String {
    format!("{tag}level W1-1 script\n{}", start.to_oracle().canonical())
}

/// From power-on to W1-1's first frame by `start`, a menu script.
fn to_w1_1(a: &mut Amiga, start: &InputScript, frozen: Freeze) -> Result<(), String> {
    for f in 0..LEVEL_SEARCH {
        step(a, start.input_at(f), frozen)?;
        let m = core_memory(a.memory());
        if in_level(&m) && long(&m, WORLD) == 1 && long(&m, LEVEL) == 1 {
            return Ok(());
        }
    }
    Err(format!("W1-1 not reached in {LEVEL_SEARCH} frames"))
}

/// W1-1's first frame by the menu script `start`, failing if it is the attract demo's W1-1 (a
/// START GAME too late lands there). For [`SyncPoint::Slot`] and its tests.
pub fn w1_1_by(start: &str) -> Result<Synced, String> {
    let s = InputScript::load(&script_path(start))?;
    let synced = at_snapshot(&w1_1_key("", &s), 1, 1, LEVEL_SEARCH, |a| {
        to_w1_1(a, &s, Freeze::None)
    })?;
    if long(&synced.memory, ATTRACT_MODE) != 0 {
        return Err(format!("{start}: W1-1 is reached in the attract demo"));
    }
    Ok(synced)
}

/// [`SyncPoint::Slot`].
fn slot(script: &str, start: Option<&str>, frames: u32) -> Result<Synced, String> {
    let start = start.unwrap_or(DEFAULT_START);
    let menu = InputScript::load(&script_path(start))?;
    let s = InputScript::load(&script_path(script))?;
    let play = s.lines.last().map_or(0, |l| l.0);
    let key = format!(
        "slot v2\nW1-1 by\n{}in W1-1\n{}",
        menu.to_oracle().canonical(),
        s.to_oracle().canonical()
    );
    let reach = LEVEL_SEARCH + play + SLOT_SEARCH;
    at_snapshot(&key, 1, frames, reach, |a| {
        to_w1_1(a, &menu, Freeze::None)?;
        if long(&core_memory(a.memory()), ATTRACT_MODE) != 0 {
            return Err(format!("{start}: W1-1 is reached in the attract demo"));
        }
        for f in 0..play {
            step(a, s.input_at(f), Freeze::None)?;
        }
        a.write_memory(LEVEL_STATE, &1u32.to_be_bytes());
        for n in 0..SLOT_SEARCH {
            let m = core_memory(a.memory());
            let chain = main_chain(&a.cpu(), &m);
            let input = if !TALLY_WAIT.contains(&chain[0]) {
                FrameInput::NONE
            } else if word(&m, TALLY_SLOT) != 1 {
                FrameInput {
                    joy: Joy::UP,
                    ..FrameInput::NONE
                }
            } else if n % 4 == 0 {
                FrameInput {
                    joy: Joy::FIRE,
                    ..FrameInput::NONE
                }
            } else {
                FrameInput::NONE
            };
            step(a, input, Freeze::None)?;
            let m = core_memory(a.memory());
            if main_chain(&a.cpu(), &m).contains(&IN_SLOT_MACHINE) {
                return Ok(());
            }
        }
        Err(format!(
            "slot_machine not reached in {SLOT_SEARCH} frames of W1-1's forced exit"
        ))
    })
}

/// How [`by_password`] gets to a level.
struct Route {
    /// `world` as the original numbers it once the level runs (the bonus level is 7).
    world: u32,
    level: u32,
    /// The `password_table` entry typed.
    entry: u32,
    /// Drives connected: 2 for W6-2..4's first map set (docs/10).
    drives: u8,
    /// The final level: W6-4's code, then its exit forced.
    finish_w6_4: bool,
}

fn route(id: LevelId) -> Option<Route> {
    let plain = |world, level, entry| Route {
        world,
        level,
        entry,
        drives: 1,
        finish_w6_4: false,
    };
    match (id.world, id.level, id.alt) {
        (World::Bonus, 1, false) => Some(plain(7, 1, 24)),
        (World::Final, 1, false) => Some(Route {
            finish_w6_4: true,
            ..plain(6, 4, 23)
        }),
        (World::W6, 2..=4, alt) => Some(Route {
            drives: if alt { 1 } else { 2 },
            ..plain(6, id.level as u32, 20 + id.level as u32 - 1)
        }),
        (w, l @ 1..=4, false) if w != World::Bonus && w != World::Final => {
            let n = World::ALL.iter().position(|&x| x == w)? as u32;
            Some(plain(n + 1, l as u32, 4 * n + l as u32 - 1))
        }
        _ => None,
    }
}

/// The menu script for `r`'s drive set-up, and the snapshot's cache key. One drive keeps the key it
/// had before drives could be connected, so T13's snapshots stay valid.
fn password_key(id: LevelId, r: &Route) -> (String, String) {
    let lead = if r.drives == 1 { 0 } else { DF1_LEAD };
    let at = |t: u32| if t >= 4600 { t - lead } else { t };
    let menu: String = MENU
        .iter()
        .map(|(t, k)| format!("{}: {k}\n", at(*t)))
        .collect();
    let mut key = format!("level {id} by level code v1\n{menu}");
    if r.drives != 1 {
        key += &format!("drives {}\n", r.drives);
    }
    if r.finish_w6_4 {
        key += "W6-4, then level_state = 1\n";
    }
    (menu, key)
}

/// The options screen's way into a level (docs/12 "Reaching every level"): from the title menu
/// pick OPTIONS, move to LEVEL CODE (the entry starts as the cursor reaches it), type the level's
/// six digits as raw keys 01..0A, leave the options and START GAME. The digits are read from the
/// original's own `password_table` as it runs, so no code is written down here. Whenever the game
/// asks for a disk, that disk goes in. W6-2..4 without `alt` run with an empty df1 connected; the
/// final level runs W6-4 and then sets `level_state` to 1, the exit's own value, so the original
/// tallies W6-4 and starts the final level by its own code.
fn by_password(id: LevelId, frames: u32, frozen: Freeze) -> Result<Synced, String> {
    let r = route(id).ok_or_else(|| format!("AtLevel({id}): no such level"))?;
    let (_, key) = password_key(id, &r);
    let key = format!("{}{key}", frozen.tag());
    at_snapshot(&key, r.drives, frames, search(&r), |a| {
        drive_to(a, id, &r, frozen)
    })
}

fn search(r: &Route) -> u32 {
    if r.finish_w6_4 {
        FINAL_SEARCH
    } else {
        PASSWORD_SEARCH
    }
}

/// [`by_password`]'s way there, on `a` from power-on, its objects frozen out when `frozen`.
fn drive_to(a: &mut Amiga, id: LevelId, r: &Route, frozen: Freeze) -> Result<(), String> {
    let lead = if r.drives == 1 { 0 } else { DF1_LEAD };
    let type_from = TYPE_FROM - lead;
    let (menu, _) = password_key(id, r);
    let search = search(r);
    let mut script = InputScript::parse(&menu).map_err(|e| e.to_string())?;
    let mut disk = 0;
    let mut forced = false;
    for f in 0..search {
        if f == type_from {
            let m = core_memory(a.memory());
            let mut text = menu.clone();
            let mut t = type_from;
            for k in 0..6 {
                let digit = m.byte(PASSWORD_TABLE + 10 * r.entry + k);
                let code = match digit {
                    b'0' => 0x0a,
                    b'1'..=b'9' => digit - b'0',
                    _ => return Err(format!("password_table entry {} is not digits", r.entry)),
                };
                text += &format!("{t}: K{code:02X}\n{}: -\n", t + 6);
                t += 16;
            }
            // Leave the options (cursor back on LIVES), fire for the menu, START GAME.
            text += &format!("{}: F\n{}: -\n", t + 40, t + 45);
            text += &format!("{}: F\n{}: -\n", t + 140, t + 145);
            text += &format!("{}: F\n{}: -\n", t + 200, t + 205);
            script = InputScript::parse(&text).map_err(|e| e.to_string())?;
        }
        step(a, script.input_at(f), frozen)?;
        let m = core_memory(a.memory());
        if f > type_from {
            let want = long(&m, WANTED_DISK);
            if want != disk {
                disk = want;
                let slot = match want {
                    0x3232_3232 => Some(DiskSlot::D1),
                    0x3333_3333 => Some(DiskSlot::D2),
                    0x3434_3434 => Some(DiskSlot::D3),
                    0x3535_3535 => Some(DiskSlot::D4),
                    _ => None,
                };
                if let Some(s) = slot {
                    a.insert_disk(0, s);
                }
            }
        }
        let at_level = in_level(&m) && long(&m, WORLD) == r.world && long(&m, LEVEL) == r.level;
        if r.finish_w6_4 {
            if at_level && !forced && word(&m, FINAL_LEVEL_FLAG) == 0 {
                a.write_memory(LEVEL_STATE, &1u32.to_be_bytes());
                forced = true;
            } else if forced && in_level(&m) && word(&m, FINAL_LEVEL_FLAG) != 0 {
                return Ok(());
            }
        } else if at_level {
            return Ok(());
        }
    }
    Err(format!("{id} not reached in {search} frames"))
}

/// [`SyncPoint::EndSequence`].
fn end_sequence(frames: u32) -> Result<Synced, String> {
    let id = LevelId {
        world: World::Final,
        level: 1,
        alt: false,
    };
    let r = route(id).ok_or("the final level has a route")?;
    let (_, key) = password_key(id, &r);
    let key = format!("{key}end sequence: level_state = 1, to end_seq_load v1\n");
    at_snapshot(&key, r.drives, frames, search(&r) + END_SEARCH, |a| {
        drive_to(a, id, &r, Freeze::None)?;
        a.write_memory(LEVEL_STATE, &1u32.to_be_bytes());
        let mut disks = Disks::at(&core_memory(a.memory()));
        for _ in 0..END_SEARCH {
            a.step(FrameInput::NONE).map_err(|e| e.to_string())?;
            disks.serve(a);
            let m = core_memory(a.memory());
            if main_chain(&a.cpu(), &m).contains(&IN_END_SEQ_LOAD) {
                return Ok(());
            }
        }
        Err(format!(
            "end_seq_load not reached in {END_SEARCH} frames of the final level"
        ))
    })
}

/// The player at the drive: whenever the game asks for a disk (`disk_request`'s wanted ID, h0+$af54)
/// that is not in df0, that disk goes in.
pub struct Disks {
    wanted: u32,
}

impl Disks {
    /// The drive as it is in the original's memory `m`: holding the disk last asked for, or disk 1
    /// before any was.
    pub fn at(m: &AmigaMemory) -> Disks {
        Disks {
            wanted: match long(m, WANTED_DISK) {
                0 => 0x3232_3232,
                w => w,
            },
        }
    }

    /// After a frame of the original's.
    pub fn serve(&mut self, a: &mut Amiga) {
        let want = long(&core_memory(a.memory()), WANTED_DISK);
        if want == self.wanted || want == 0 {
            return;
        }
        self.wanted = want;
        let slot = match want {
            0x3232_3232 => Some(DiskSlot::D1),
            0x3333_3333 => Some(DiskSlot::D2),
            0x3434_3434 => Some(DiskSlot::D3),
            0x3535_3535 => Some(DiskSlot::D4),
            _ => None,
        };
        if let Some(s) = slot {
            a.insert_disk(0, s);
        }
    }
}

/// The disk a power-on sync point boots.
fn boot_disk(sync: &SyncPoint) -> DiskSlot {
    match sync {
        SyncPoint::PowerOnDisk3 => DiskSlot::D3,
        _ => DiskSlot::D1,
    }
}

fn power_on(boot: DiskSlot, frames: u32) -> Result<Amiga, String> {
    let cfg = OracleConfig::locate(frames)
        .map_err(|e| e.to_string())?
        .with_boot(boot);
    Amiga::power_on(&cfg).map_err(|e| e.to_string())
}

/// The story intro's `intro_wait_disk1` (d3_c04 h0+$c38e) is running: its return address, h0+$310,
/// is on the main program's stack.
const INTRO_ASKS_DISK1: u32 = h0(0x310);
/// Where the story intro's vertical-blank handler is (d3_c04 h0+$b102).
pub const INTRO_VBL: u32 = h0(0xb102);
/// Where the disk-3 boot program (`d3_c02`) is loaded (measured on the oracle).
pub const DISK3_BOOT_BASE: u32 = 0x519d0;
/// Its hunk 0's length.
const DISK3_BOOT_LEN: u32 = 0x2c5a8;
/// Where the original's CPU took the vertical blank's interrupt of the frame a step ran (line, DMA
/// cycle), from the interrupts of the step before it (`before`, one taken past its end belongs to
/// this frame) and of the step itself.
pub fn handler_entry(before: &[sf_oracle::Irq], now: &[sf_oracle::Irq]) -> Option<(u16, u16)> {
    before
        .iter()
        .filter(|i| i.next)
        .chain(now.iter().filter(|i| !i.next))
        .find(|i| i.level == 3)
        .map(|i| (i.vpos, i.hpos))
}

/// The PC the story intro's vertical-blank handler will return to, when a step of `a` is about to
/// begin at its entry (a step ends there): where the main program was interrupted.
pub fn intro_resumes(a: &Amiga) -> Option<u32> {
    let cpu = a.cpu();
    (cpu.pc == INTRO_VBL).then(|| main_chain(&cpu, &core_memory(a.memory()))[0])
}

/// Where the story intro's handler of a step ended (owner, T46; T54): its acknowledge, the step's
/// first `INTREQ = $20` among its Paula writes, and the PC it returned to ([`intro_resumes`] before
/// the step).
pub fn handler_end(paula: &[sf_paula::PaulaWrite], resumes: Option<u32>) -> Option<HandlerEnd> {
    let resumes = resumes?;
    paula
        .iter()
        .find(|w| w.reg == 0x9c && w.value == 0x20)
        .map(|w| HandlerEnd {
            ack: (w.vpos, w.hpos),
            resumes,
        })
}

/// How many vertical-blank handlers the original ran in its last step, and whether the last
/// one's interrupt came so late that its work (its first write) falls in the next step. A
/// handler counts in the step it writes in (its sound, its acknowledgement): `carried` is the
/// last step's late one, `now` this step's interrupts (one past its end counts in the next),
/// `paula` its writes. Fewer than the video frames the step ran: a blit held the CPU through a
/// blank, whose request the next handler served too (T55).
pub fn handlers(
    carried: bool,
    before: &[sf_oracle::Irq],
    now: &[sf_oracle::Irq],
    paula: &[sf_paula::PaulaWrite],
) -> (u32, bool) {
    let taken: Vec<&sf_oracle::Irq> = now.iter().filter(|i| i.level == 3 && !i.next).collect();
    let late = taken.last().is_some_and(|i| {
        !paula
            .iter()
            .any(|w| (w.vpos, w.hpos) >= (i.vpos, i.hpos))
    });
    let n = u32::from(carried)
        + before.iter().filter(|i| i.level == 3 && i.next).count() as u32
        + taken.len() as u32
        - u32::from(late);
    (n, late)
}

/// The story intro's hunk 0 length (d3_c04, at the game's own base).
const INTRO_LEN: u32 = 0x52160;

/// One frame of the original, putting disk 1 in df0 once the story intro asks for it (a
/// disk-3 boot; nothing happens on a disk-1 boot).
pub fn step_inserting(a: &mut Amiga, input: FrameInput) -> Result<sf_oracle::FrameCapture, String> {
    let cap = a.step(input).map_err(|e| e.to_string())?;
    let m = core_memory(a.memory());
    if long(&m, 0x6c) == INTRO_VBL
        && !a.disk1_in()
        && main_chain(&a.cpu(), &m).contains(&INTRO_ASKS_DISK1)
    {
        a.insert_disk(0, DiskSlot::D1);
    }
    Ok(cap)
}

/// The RNG state on the frame the original first seeds it (docs/08 "RNG"): the first frame, once
/// the game owns the vertical blank, on which `rng_state` changes.
fn power_on_seed(boot: DiskSlot, script: &InputScript, frames: u32) -> Result<Option<Rng>, String> {
    let mut a = power_on(boot, frames)?;
    let mut prev = None;
    for f in 0..frames {
        step_inserting(&mut a, script.input_at(f))?;
        let m = core_memory(a.memory());
        let rng = (long(&m, RNG_STATE), long(&m, RNG_STATE + 4));
        if long(&m, 0x6c) == VBL_HANDLER {
            if prev.is_some_and(|p| p != rng) {
                return Ok(Some(Rng {
                    lo: rng.0,
                    hi: rng.1,
                }));
            }
            prev = Some(rng);
        }
    }
    Ok(None)
}

/// The oracle with `drives` drives after `get_there`, from the snapshot cache when it has this
/// key.
fn at_snapshot(
    what: &str,
    drives: u8,
    frames: u32,
    reach_frames: u32,
    get_there: impl FnOnce(&mut Amiga) -> Result<(), String>,
) -> Result<Synced, String> {
    let cfg = OracleConfig::locate(reach_frames + frames)
        .map_err(|e| e.to_string())?
        .with_drives(drives);
    let key = cache_key(&cfg, what)?;
    let path = cache_dir().join(format!("{key}.sync"));
    let mut amiga = Amiga::power_on(&cfg).map_err(|e| e.to_string())?;
    match fs::read(&path).ok().and_then(|b| decode(&b, &key)) {
        Some(snap) => amiga.restore(&snap),
        None => {
            get_there(&mut amiga)?;
            store(&path, &key, &amiga.snapshot());
        }
    }
    let memory = core_memory(amiga.memory());
    Ok(Synced {
        amiga,
        memory,
        seed: None,
    })
}

fn cache_dir() -> PathBuf {
    std::env::var_os("SUPERFROG_CACHE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/superfrog-re")
        })
        .join("sync")
}

/// SHA-1 over the format, the vAmiga build, the Kickstart and disk files and the way there.
fn cache_key(cfg: &OracleConfig, what: &str) -> Result<String, String> {
    let file = |p: &PathBuf| {
        fs::read(p)
            .map(|b| sha1_hex(&b))
            .map_err(|e| format!("{}: {e}", p.display()))
    };
    let mut text = format!(
        "sync format {SYNC_FORMAT}\nvamiga {}\nkickstart {}\n",
        sf_oracle::BUILD,
        file(&cfg.kickstart)?
    );
    for d in &cfg.disks {
        text += &format!("disk {}\n", file(d)?);
    }
    // A one-drive key is what it was before drives could be connected.
    if cfg.drives != 1 {
        text += &format!("drives {}\n", cfg.drives);
    }
    text += what;
    Ok(sha1_hex(text.as_bytes()))
}

/// `MAGIC`, the key, the frame, the held input, the snapshot bytes.
fn store(path: &PathBuf, key: &str, s: &Snapshot) {
    let mut b = MAGIC.to_vec();
    b.extend(key.as_bytes());
    b.extend(s.frame.to_le_bytes());
    b.push(s.input.joy.bits());
    b.extend(s.input.keys.0.to_le_bytes());
    b.extend(&s.bytes);
    // A cache that cannot be written only costs the next run time.
    if let Some(dir) = path.parent()
        && fs::create_dir_all(dir).is_ok()
    {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("tmp{}-{n}", std::process::id()));
        if fs::write(&tmp, &b).is_ok() {
            let _ = fs::rename(&tmp, path);
        }
    }
}

fn decode(b: &[u8], key: &str) -> Option<Snapshot> {
    let rest = b.strip_prefix(MAGIC)?.strip_prefix(key.as_bytes())?;
    if rest.len() < 21 {
        return None;
    }
    Some(Snapshot {
        frame: u32::from_le_bytes(rest[..4].try_into().ok()?),
        input: FrameInput {
            joy: Joy(rest[4]),
            keys: KeySet(u128::from_le_bytes(rest[5..21].try_into().ok()?)),
        },
        bytes: rest[21..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn irq(next: bool, vpos: u16, hpos: u16) -> sf_oracle::Irq {
        sf_oracle::Irq {
            level: 3,
            next,
            vpos,
            hpos,
        }
    }

    fn write(reg: u16, vpos: u16, hpos: u16) -> sf_paula::PaulaWrite {
        sf_paula::PaulaWrite {
            reg,
            value: 0,
            vpos,
            hpos,
        }
    }

    /// The oracle's intro, frames 6449-6453 and 6771-6772 (T55): a two-frame step with one
    /// handler; a handler taken at line 309 whose writes all fall in the next step.
    #[test]
    fn a_handler_counts_in_the_step_it_writes_in() {
        // One interrupt in a two-frame step: one handler, the second blank merged.
        assert_eq!(
            handlers(false, &[], &[irq(false, 281, 26)], &[write(0xa8, 282, 0)]),
            (1, false)
        );
        // One past the last step's end belongs to this one.
        assert_eq!(handlers(false, &[irq(true, 0, 16)], &[], &[]), (1, false));
        // Taken too late to write: none here, one in the next step.
        assert_eq!(
            handlers(false, &[], &[irq(false, 309, 188)], &[write(0x96, 12, 0)]),
            (0, true)
        );
        assert_eq!(handlers(true, &[], &[], &[write(0xa6, 10, 0)]), (1, false));
        // No interrupt at all: the blank merged into the next.
        assert_eq!(handlers(false, &[], &[], &[]), (0, false));
    }

    fn id(s: &str) -> LevelId {
        s.parse().unwrap()
    }

    #[test]
    fn w6_2_to_4_take_df1_for_their_first_maps_and_one_drive_for_alt() {
        for (s, world, level, entry, drives, finish) in [
            ("W1-2", 1, 2, 1, 1, false),
            ("W3-2", 3, 2, 9, 1, false),
            ("W6-1", 6, 1, 20, 1, false),
            ("W6-2", 6, 2, 21, 2, false),
            ("W6-4", 6, 4, 23, 2, false),
            ("W6-2alt", 6, 2, 21, 1, false),
            ("W6-4alt", 6, 4, 23, 1, false),
            ("WB-1", 7, 1, 24, 1, false),
            ("WW-1", 6, 4, 23, 1, true),
        ] {
            let r = route(id(s)).unwrap();
            assert_eq!(
                (r.world, r.level, r.entry, r.drives, r.finish_w6_4),
                (world, level, entry, drives, finish),
                "{s}"
            );
        }
    }

    /// T13's one-drive snapshots are found again: the key text is the one it wrote.
    #[test]
    fn a_one_drive_level_keeps_its_t13_key_and_the_others_differ() {
        const T13_MENU: &str = "1000: M\n1005: -\n4600: F\n4605: -\n4650: D\n4655: -\n4700: F\n4705: -\n\
                                4800: D\n4810: -\n4840: D\n4850: -\n4880: D\n4890: -\n";
        let (menu, key) = password_key(id("W3-2"), &route(id("W3-2")).unwrap());
        assert_eq!(menu, T13_MENU);
        assert_eq!(key, format!("level W3-2 by level code v1\n{T13_MENU}"));

        let (menu2, key2) = password_key(id("W6-2"), &route(id("W6-2")).unwrap());
        assert!(menu2.starts_with("1000: M\n1005: -\n4381: F\n"), "{menu2}");
        assert!(key2.ends_with("drives 2\n"));
        let (_, alt) = password_key(id("W6-2alt"), &route(id("W6-2alt")).unwrap());
        let (_, fin) = password_key(id("WW-1"), &route(id("WW-1")).unwrap());
        assert!(alt != key2 && fin != alt && fin.contains("level_state = 1"));
    }

    #[test]
    fn the_snapshot_cache_never_mixes_drive_set_ups() {
        let one = OracleConfig::locate(10).unwrap_or_else(|e| panic!("{e}"));
        let two = one.clone().with_drives(2);
        let k1 = cache_key(&one, "x").unwrap();
        assert_ne!(k1, cache_key(&two, "x").unwrap());
        // One drive: the text T13 hashed, nothing added.
        let mut text = format!(
            "sync format {SYNC_FORMAT}\nvamiga {}\nkickstart {}\n",
            sf_oracle::BUILD,
            sha1_hex(&fs::read(&one.kickstart).unwrap())
        );
        for d in &one.disks {
            text += &format!("disk {}\n", sha1_hex(&fs::read(d).unwrap()));
        }
        assert_eq!(k1, sha1_hex(format!("{text}x").as_bytes()));
    }

    /// W1-1's snapshots, cached under T13's key, are found again now the start script is a
    /// parameter.
    #[test]
    fn w1_1_keeps_its_key() {
        let s = InputScript::load(&script_path(DEFAULT_START)).unwrap();
        assert_eq!(
            w1_1_key("", &s),
            format!("level {} script\n{}", id("W1-1"), s.to_oracle().canonical())
        );
        assert!(w1_1_key("", &s).starts_with("level W1-1 script\n"));
    }

    #[test]
    fn levels_without_a_route_are_refused() {
        let bad = LevelId {
            world: World::W5,
            level: 2,
            alt: true,
        };
        assert!(route(bad).is_none());
    }
}
