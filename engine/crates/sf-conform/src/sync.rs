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
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, h0};
use sf_core::recording::LevelSetup;
use sf_core::{Game, HandlerEnd, Pacing, Rng};
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
    /// [`SyncPoint::PowerOn`] with an empty df1 connected (T201): W6-2..4's level codes then load
    /// their first map set, as the engine plays them (docs/10). The boot reaches the title
    /// [`DF1_LEAD`] frames sooner.
    PowerOnDf1,
    /// Run the named script from power-on for `frame` frames; with `df1`, an empty df1 connected
    /// as [`SyncPoint::PowerOnDf1`] (T208: a stretch of the whole game).
    AtFrame { script: String, frame: u32, df1: bool },
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
    /// [`SyncPoint::EndSequence`] with `score` (h0+$834) then set to `score` (T200: the game's end
    /// with a new high score).
    EndSequenceScore { score: u32 },
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
    /// A level's first frame as [`SyncPoint::AtLevel`] reaches it, its exit then forced
    /// (`level_state` = 1, as [`SyncPoint::Slot`] forces it; T38: the tally without playing the
    /// level to its exit).
    Exit(LevelId),
    /// Game over after the last life (T34): W1-1 reached as [`SyncPoint::AtLevel`] reaches it,
    /// `frog_lives` (h0+$82e) set to 0 on its first frame, `script` played from there until the
    /// frog's death has ended the level's loop and the original's main program is in
    /// `game_over_seq` (h0+$7600 on its stack). The sync point is the end of the frame before:
    /// the level is transplanted, and the case's first frame is the one game over begins in.
    /// With `score`, `score` (h0+$834) is set to it with the lives (T200: a new high score).
    GameOver { script: String, score: Option<u32> },
    /// A level's first frame as [`SyncPoint::AtLevel`] reaches it, `frog_lives` (h0+$82e) then
    /// set to `lives` (T79: the bonus level played to its end through the deaths on the way).
    Lives { level: LevelId, lives: u16 },
    /// A level started with the Start Level dialog's values (bug-recorder T03, its DESIGN §2.8, D2
    /// as T00 measured it): the original switched on (an empty df1 connected for W6-2..4, whose
    /// codes then load the first map set the engine plays), every disk it asks for put in; the
    /// title menu's OPTIONS, LIVES and GAME MODE moved to the setup's, the level's code typed,
    /// the options left and START GAME (the final level by W6-4's code, its exit forced, as
    /// [`SyncPoint::AtLevel`]). At the level's first frame the setup's points, energy, spud and
    /// wings are written in, and [`SETUP_FROM_ENGINE`] is copied from the engine's
    /// `Game::start_level` with the same setup and `seed`: the random state and what the title's
    /// time and the loads' length leave. The engine side is that `start_level` itself, not a
    /// transplant ([`setup_start`]).
    Setup { setup: LevelSetup, seed: Rng },
}

/// What the original takes from the engine's `Game::start_level` at a [`SyncPoint::Setup`]'s
/// frame 0 (T00): the random state (the engine's seed, as the app's start has it); what the
/// title's time decides, which the engine's own menu timing differs in (`flow.rs` `TITLE_TIME`);
/// and what the loads' length leaves, the engine loading at once (the music's place, the progress
/// map's frog sprites, read on WB-1; the music's fade and the map's blink count, which differ only
/// on the final level, reached through W6-4's tally and load).
pub const SETUP_FROM_ENGINE: [&str; 13] = [
    "rng_state",
    "opt_flash",
    "password_buf",
    "sfx_chan0",
    "kbd_toggle",
    "title_blink",
    "title_menu",
    "p41_channels",
    "exit_sprite_left",
    "exit_sprite_right",
    "exit_sprite_copper",
    "music_fade",
    "map_blink",
];

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
/// `vbl_count` (h0+$9d8): the vertical blanks since the level's pass began.
const VBL_COUNT: u32 = h0(0x9d8);
/// `level_frame_loop`'s wait for `vbl_count` (docs/08).
const LEVEL_WAIT: Range<u32> = h0(0xaae)..h0(0xab8);
const LOGIC_RUNNING: u32 = h0(0x9d6);
const LEVEL_STATE: u32 = h0(0xa06);
/// `level_clock` (h0+$876): four digits, minutes first.
const LEVEL_CLOCK: u32 = h0(0x876);
/// `frog_lives` (h0+$82e).
const FROG_LIVES: u32 = h0(0x82e);
/// `score` (h0+$834).
const SCORE: u32 = h0(0x834);
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

/// Whether the main program ended the frame parked in `level_frame_loop`'s wait for its next
/// pass (DESIGN §2.1 check 1: a level's pass ends there; anywhere else a pass is a frame).
pub fn in_level_wait(cpu: &CpuRegs, m: &AmigaMemory) -> bool {
    let main_pc = if cpu.pc == VBL_HANDLER {
        long(m, cpu.ssp + 2)
    } else {
        cpu.pc
    };
    LEVEL_WAIT.contains(&main_pc)
}

/// A level is being played: its logic running (`logic_running`, h0+$9d6) and `level_state`
/// (h0+$a06) negative, as it is from the level's first frame until its exit or the frog's death
/// ends it.
pub fn in_level(m: &AmigaMemory) -> bool {
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
        point: if step == sf_core::level::beam::NO_STEP { 0 } else { pass_point(cpu, m) },
        blanks: word(m, VBL_COUNT) as u16,
    };
    if long(m, 0x6c) == INTRO_VBL {
        return if sf_core::front::story::intro::busy(&main_chain(cpu, m)) {
            late(0, sf_core::level::beam::NO_STEP)
        } else {
            Pacing::OnTime
        };
    }
    if in_level(m)
        && word(m, VBL_COUNT) >= 2
        && irq_frame(cpu, m).is_some_and(|(ret, pc)| ret == COPPER_IRQ_RETURN && LEVEL_WAIT.contains(&pc))
    {
        // The pass was due but never began: the copper interrupt's blits held the CPU through
        // the frame's end, the main loop still in its wait (T195: W3-3 4026, 5280). The engine
        // ends the frame as its pass begins, before `vbl_count` is cleared.
        return late(0, 0);
    }
    // Waiting with the pass already due: it begins after the next blank (T196).
    if in_level(m)
        && in_level_wait(cpu, m)
        && sf_core::level::n_clear((word(m, VBL_COUNT) as u16).wrapping_sub(2))
    {
        return Pacing::PassDue;
    }
    // In the level's interrupt with the main loop in its wait (T203: W1-2 in the whole game, the
    // scroller redrawing the screen through five frames, its copper blits five more): the engine
    // ends the frame there and holds the next vertical blank as the original does.
    if in_level(m) && irq_frame(cpu, m).is_some_and(|(_, pc)| LEVEL_WAIT.contains(&pc)) {
        return late(sf_core::level::beam::IRQ_PHASE, sf_core::level::beam::NO_STEP);
    }
    if mid_pass(cpu, m) {
        let phase = pass_phase(cpu, m);
        let step = if phase == FLIP_PHASE {
            flip_step(cpu, m)
        } else if phase == PANEL_PHASE {
            panel_step(cpu, m)
        } else {
            pass_step(cpu, m)
        };
        late(phase, step)
    } else {
        Pacing::OnTime
    }
}

/// `vbl_handler`'s call of `copper_irq_blits` returns here (h0+$11d1a).
const COPPER_IRQ_RETURN: u32 = h0(0x11d1e);

/// `vbl_handler`'s body (h0+$11bf8..$11d2c): its calls return into it, its `rte`s are at
/// h0+$11d18 and h0+$11d2a.
const VBL_BODY: Range<u32> = h0(0x11bf8)..h0(0x11d2c);
const VBL_RTES: [u32; 2] = [h0(0x11d18), h0(0x11d2a)];

/// When the frame ended inside the level's interrupt (`vbl_handler`'s VBL work or its copper
/// half, `copper_irq_blits`, the next vertical blank held back behind it at the same level): the
/// handler's call it was in (its return address, or the handler's own PC outside a call) and
/// where the main program was, the PC in the interrupt's frame above the 15 registers
/// `vbl_handler` saved and the status register. Not in the paused path's nested save
/// (`logic_running` clear there, so never in a level).
fn irq_frame(cpu: &CpuRegs, m: &AmigaMemory) -> Option<(u32, u32)> {
    if cpu.pc == VBL_HANDLER || long(m, 0x6c) != VBL_HANDLER {
        return None;
    }
    let sp = cpu.a[7];
    if VBL_RTES.contains(&cpu.pc) {
        return Some((cpu.pc, long(m, sp + 2)));
    }
    if VBL_BODY.contains(&cpu.pc) {
        return Some((cpu.pc, long(m, sp + 15 * 4 + 2)));
    }
    (sp..sp + 0x100)
        .step_by(2)
        .find(|&a| {
            let r = long(m, a);
            VBL_BODY.contains(&r) && (word(m, r - 4) == 0x6100 || word(m, r - 6) == 0x4eb9)
        })
        .map(|a| (long(m, a), long(m, a + 4 + 15 * 4 + 2)))
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

/// The pass's call to the buffer flip (h0+$172b4, from h0+$ae0).
const FLIP_PHASE: u8 = 6;

/// How far the buffer flip had got at the frame's end, as the engine counts its steps
/// (`sf_core::level::pass::buffer_flip`): 0 before its first write, then one more after each of
/// `vbl_pass_flag` cleared, `shown_buffer` set to 1, and the flip done (`shown_buffer` 2 set or
/// its test failed: at the `rts`, or the shared one at h0+$16478 the test branches to).
pub fn flip_step(cpu: &CpuRegs, m: &AmigaMemory) -> u16 {
    let pc = if cpu.pc == VBL_HANDLER {
        long(m, cpu.ssp + 2)
    } else {
        cpu.pc
    };
    match pc.wrapping_sub(h0(0)) {
        0x172ba..=0x172c1 => 1,
        0x172c2..=0x172d3 => 2,
        0x172d4 | 0x16478 => 3,
        _ => 0,
    }
}

/// The pass's call to `panel_update` (h0+$8620, from h0+$aec).
const PANEL_PHASE: u8 = 8;

/// How far `panel_update` had got at the frame's end, as the engine counts its steps
/// (`sf_core::level::panel::panel_update`): its calls of the lives icon, the score digit and the
/// clock digit, the one running counted done (T212: W6-1 2128 and W2-3 6728 end in the score
/// digit; the clock digit after it reads the clock as the vertical blank leaves it).
pub fn panel_step(cpu: &CpuRegs, m: &AmigaMemory) -> u16 {
    let chain = main_chain(cpu, m);
    let Some(k) = chain.iter().position(|&a| a == h0(0xaf0)) else {
        return 0;
    };
    // The PC in `panel_update` itself, or the return address of the call it is in.
    match k.checked_sub(1).map(|j| chain[j].wrapping_sub(h0(0))) {
        Some(0x8624..=0x8627) => 1,
        Some(0x8628..=0x862b) => 2,
        Some(0x862c..=0x862f) => 3,
        _ => 0,
    }
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
    // work, whose memory the engine reports as jobs. With `level_state` not negative the frame
    // can still end in a pass: the exit's 1 is set in the vertical blank, and the pass that counts
    // it down can run past the frame's end (T106: W1-1's exit, the original in `bob_draw`); once
    // the loop has returned, `logic_running` is still set in `level_exit_fade`'s set-up.
    let in_pass = || main_chain(cpu, m).iter().any(|a| (h0(0xab8)..h0(0xb66)).contains(a));
    (in_level(m) || word(m, LOGIC_RUNNING) != 0 && in_pass())
        && !LEVEL_WAIT.contains(&main_pc) && !main_chain(cpu, m).contains(&IN_GAME_OVER)
}

/// How far the loop of the pass's call the main program is in had got at the frame's end, as
/// the engine counts its steps ([`sf_core::level::LevelCtx::step_done`]): the slots
/// `obj_run_all` has run (one whose update was running counted done), the slots
/// `obj_build_hitboxes` has looked at, the entries of `build_bob_draw_list`'s three loops, the
/// halves of `collide_all` (the spud's, the frog's) and the hitbox entries the frog's has looked
/// at (one taken but not yet decided on not counted). [`NO_STEP`] in any other call.
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
        // `bob_draw` not begun (at its call or its first instruction), or with only the camera's
        // x latched: the camera it latches then is the one the vertical blank leaves (T212: W6-1
        // 4386, 4402, 5268).
        0xada | 0x19120 => 0,
        0x1912a => 1,
        0x19410..=0x19958 => bob_draw_step(at, cpu, m),
        0xd0cc => 0,
        0xd0d0 | 0xd0e0..=0xd0ff => 1,
        // The frog's box is taken once its x and y are read (h0+$d0f8, $d0fc): from there the
        // walk tests the frog as it was before the vertical blank (T196: W6-4's coins at 153,
        // the frame ending at h0+$d120).
        0xd100..=0xd125 => 2,
        // The entry just taken (a6 past it) is not looked at until its stomp, touch or hurt is
        // decided: those read the frog as the vertical blank leaves it (T185: W6-4's stomp at
        // 1090, the frame ending at h0+$d14a).
        0xd132..=0xd17b | 0xd19c..=0xd1ad => frog_walk_step(cpu.a[6]) - 1,
        0xd126..=0xd1cd => frog_walk_step(cpu.a[6]),
        0xd1ce | 0xd0d4..=0xd0df => frog_walk_step(hitbox_end(m)),
        _ => {
            // Inside a routine a loop called: the slot whose update runs (`obj_run_all` saved
            // d0 under the return address), the hitbox entry whose touch handler runs (the frog's
            // walk saved d0-d7/a0-a6 under the return address; `frog_hurt` a0/a5 under that), or
            // the half of `collide_all`.
            if let Some(a) = ret(0x1b33e) {
                return slot(word(m, a + 6) as u16) + 1;
            }
            if let Some(a) = ret(0xd19a).or_else(|| ret(0xd1c6)) {
                return frog_walk_step(long(m, a + 4 + 56));
            }
            if let Some(a) = ret(0xd1b8) {
                return frog_walk_step(long(m, a + 12 + 56));
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

/// `bob_draw_vars+$16` (h0+$18bde): set while the drawer draws the seam's part.
const BD_SEAM: u32 = h0(0x18bde);
/// `bob_draw_list` (h0+$2970e): the bobs to draw, ended by a 0.
const BOB_DRAW_LIST: u32 = h0(0x2970e);

/// The step of `bob_draw`'s draw loop (h0+$19410..$19958, at `pc` from h0) outside the seam's part, as the engine
/// counts it (`sf_core::level::objects::draw_part`): 2, then one more for each bob drawn going
/// back up the list (`a0` at the bob taken), the one being drawn counted done once its animation
/// is started (past h0+$1943c). [`NO_STEP`] in the seam's part.
fn bob_draw_step(pc: u32, cpu: &CpuRegs, m: &AmigaMemory) -> u16 {
    use sf_core::level::beam::NO_STEP;
    if word(m, BD_SEAM) != 0 {
        return NO_STEP;
    }
    let mut end = BOB_DRAW_LIST;
    while long(m, end) != 0 {
        end += 4;
    }
    let left = |a0: u32| (end.wrapping_sub(a0) / 4) as u16;
    let drawn = match pc {
        // Out of the loop: every bob drawn.
        0x196fe..=0x19706 | 0x19950..=0x19958 => left(BOB_DRAW_LIST),
        // `a0` not yet moved to the next bob.
        0x19410..=0x19413 => left(cpu.a[0]),
        // The bob taken, its animation not yet started.
        0x19414..=0x1943c => left(cpu.a[0]).saturating_sub(1),
        _ => left(cpu.a[0]),
    };
    2 + drawn
}

/// Where in the running step the main program was at the frame's end, as the engine marks it
/// (`sf_core::level::LevelCtx::point`), 0 elsewhere: in `obj_despawn_if_offscreen` (h0+$1b95e)
/// under a slot's update, how many of its reads of `scroll_dirs` and the camera it had made (1:
/// none, 5: all but the last; T212: W2-2 1228 ends at h0+$1b9ce, its last test of `camera_y` made
/// after the vertical blank).
pub fn pass_point(cpu: &CpuRegs, m: &AmigaMemory) -> u8 {
    let (pc, sp) = if cpu.pc == VBL_HANDLER {
        (long(m, cpu.ssp + 2), cpu.ssp + 6)
    } else {
        (cpu.pc, cpu.a[7])
    };
    let in_update = (0..0x100).step_by(2).any(|k| long(m, sp + k) == h0(0x1b33e));
    if !in_update {
        return 0;
    }
    match pc.wrapping_sub(h0(0)) {
        0x1b95e..=0x1b966 => 1,
        0x1b96c..=0x1b990 => 2,
        0x1b996..=0x1b9ac => 3,
        0x1b9b2..=0x1b9c4 => 4,
        0x1b9ca..=0x1b9e2 => 5,
        _ => 0,
    }
}

/// `hitboxes` (h0+$d350): the pass's hitbox list, 12 bytes an entry, ended by a negative word.
const HITBOXES: u32 = h0(0xd350);

/// The step of `collide_all`'s frog half with the walk's pointer at `a6` (h0+$d126 on): 2 once
/// the frog's box is taken, then one more for each entry looked at (the one whose touch handler
/// is running counted done), as `sf_core::level::objects::frog_touches` counts them.
fn frog_walk_step(a6: u32) -> u16 {
    2 + (a6.wrapping_sub(HITBOXES) / 12) as u16
}

/// Where the hitbox list ends (its negative word): the frog's walk all looked at.
fn hitbox_end(m: &AmigaMemory) -> u32 {
    let mut at = HITBOXES;
    while word(m, at) & 0x8000 == 0 {
        at += 12;
    }
    at
}

/// `frog_tiles`'s wall test (h0+$e3fe..$e412), which loops for ever with both wall flags set
/// (DESIGN §2.11).
pub const FROG_TILES_WALL_LOOP: std::ops::RangeInclusive<u32> = h0(0xe3fe)..=h0(0xe412);

/// Whether the original's main program ended the frame inside [`FROG_TILES_WALL_LOOP`]: hung, the
/// frog lifted into a pillar between two `$3d` edges (T205: the planners' ways met it on W2-2 and
/// W2-3 at x 1414). Reads only the interrupted PC, not the whole memory.
pub fn in_wall_loop(a: &Amiga) -> bool {
    let cpu = a.cpu();
    let l = |addr: u32| {
        let b = a.peek(addr, 4);
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    };
    let pc = if cpu.pc == l(0x6c) { l(cpu.ssp + 2) } else { cpu.pc };
    FROG_TILES_WALL_LOOP.contains(&pc)
}

/// Where the main program was at the end of a frame: its PC, then each long on its stack that is
/// a return address into the game's code (a long just after a `jsr`/`bsr`), innermost first. For
/// tracing which loop of a scene a frame belongs to.
pub fn main_chain(cpu: &CpuRegs, m: &AmigaMemory) -> Vec<u32> {
    main_chain_deep(cpu, m, 8)
}

/// [`main_chain`] up to `depth` entries: a load's call can sit deeper than eight (T203: W2's
/// end picture's load a call deeper in the trackloader, h0+$436a ninth at the original's 81215).
pub fn main_chain_deep(cpu: &CpuRegs, m: &AmigaMemory, depth: usize) -> Vec<u32> {
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
    while sp + 4 <= top.min(sp + 0x400) && chain.len() < depth {
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
        SyncPoint::PowerOn | SyncPoint::PowerOnDisk3 | SyncPoint::PowerOnDf1 => {
            let boot = boot_disk(sync);
            let drives = if *sync == SyncPoint::PowerOnDf1 { 2 } else { 1 };
            let seed = power_on_seed(boot, drives, script, frames)?;
            let amiga = power_on(boot, drives, frames)?;
            let _ = frozen;
            let memory = core_memory(amiga.memory());
            Ok(Synced {
                amiga,
                memory,
                seed,
            })
        }
        SyncPoint::AtFrame { script, frame, df1 } => {
            let s = InputScript::load(&script_path(script))?;
            let key = format!("{tag}frame {frame} script\n{}", s.to_oracle().canonical());
            at_snapshot(&key, 1 + *df1 as u8, frames, *frame, |a| {
                // With df1, as a power-on case runs: each disk the original asks for goes in.
                let mut disks = df1.then(|| Disks::at(&core_memory(a.memory())));
                for f in 0..*frame {
                    match disks.as_mut() {
                        None => step(a, s.input_at(f), frozen)?,
                        Some(d) => {
                            frozen.apply(a, &crate::display_data());
                            step_inserting(a, s.input_at(f))?;
                            d.serve(a);
                        }
                    }
                }
                Ok(())
            })
        }
        SyncPoint::AtLevel(id) if (id.world, id.level, id.alt) != (World::W1, 1, false) => {
            by_password(*id, frames, frozen)
        }
        SyncPoint::EndSequence => end_sequence(frames),
        SyncPoint::EndSequenceScore { score } => {
            let mut synced = end_sequence(frames)?;
            synced.amiga.write_memory(SCORE, &score.to_be_bytes());
            synced.memory = core_memory(synced.amiga.memory());
            Ok(synced)
        }
        SyncPoint::Slot { script, start } => slot(script, start.as_deref(), frames),
        SyncPoint::Clock { level, seconds } => {
            let mut synced = reach_frozen(&SyncPoint::AtLevel(*level), script, frames, frozen)?;
            let (m, s) = (seconds / 60, seconds % 60);
            let digits = [(m / 10) as u8, (m % 10) as u8, (s / 10) as u8, (s % 10) as u8];
            synced.amiga.write_memory(LEVEL_CLOCK, &digits);
            synced.memory = core_memory(synced.amiga.memory());
            Ok(synced)
        }
        SyncPoint::Exit(level) => {
            let mut synced = reach_frozen(&SyncPoint::AtLevel(*level), script, frames, frozen)?;
            synced.amiga.write_memory(LEVEL_STATE, &1u32.to_be_bytes());
            synced.memory = core_memory(synced.amiga.memory());
            Ok(synced)
        }
        SyncPoint::GameOver { script, score } => {
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
                if let Some(score) = score {
                    synced.amiga.write_memory(SCORE, &score.to_be_bytes());
                }
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
        SyncPoint::Lives { level, lives } => {
            let mut synced = reach_frozen(&SyncPoint::AtLevel(*level), script, frames, frozen)?;
            synced.amiga.write_memory(FROG_LIVES, &lives.to_be_bytes());
            synced.memory = core_memory(synced.amiga.memory());
            Ok(synced)
        }
        SyncPoint::Setup { setup, seed } => {
            let _ = frozen;
            setup_start(setup, *seed, frames).map(|(synced, _)| synced)
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

/// `score`, `frog_next_life`, `frog_hearts`, `frog_spud_on`, `frog_flight`: where
/// `Game::start_level` writes the setup's values.
const FROG_NEXT_LIFE: u32 = h0(0x880);
const FROG_HEARTS: u32 = h0(0x830);
const FROG_SPUD_ON: u32 = h0(0x852);
const FROG_FLIGHT: u32 = h0(0x866);

/// The options screen's LIVES and GAME MODE at power-on (`opt_lives` 0, `opt_game_mode` 1), where
/// a [`SyncPoint::Setup`]'s presses start from.
const OPTIONS_AT_POWER_ON: (u8, u8) = (0, 1);

/// [`SyncPoint::Setup`]'s menu script: [`MENU`] to the options screen, then from frame 4 800 a
/// press every 40 frames (held 10): right or left on LIVES to the setup's, down, the same on GAME
/// MODE, down twice to LEVEL CODE; the code's digits 100 frames after the last down, one every 16;
/// then [`drive_to`]'s leave and START GAME. The presses from the menu on come [`DF1_LEAD`] sooner
/// with df1 connected. As T00 measured it (6 912 to 7 589 frames to a level's first frame).
fn setup_script(setup: &LevelSetup, r: &Route) -> Result<String, String> {
    let lead = if r.drives == 1 { 0 } else { DF1_LEAD };
    let at = |t: u32| if t >= 4600 { t - lead } else { t };
    let mut text: String = MENU[..8]
        .iter()
        .map(|(t, k)| format!("{}: {k}\n", at(*t)))
        .collect();
    fn press(text: &mut String, t: &mut u32, at: &dyn Fn(u32) -> u32, k: &str) {
        *text += &format!("{}: {k}\n{}: -\n", at(*t), at(*t + 10));
        *t += 40;
    }
    fn change(text: &mut String, t: &mut u32, at: &dyn Fn(u32) -> u32, from: u8, to: u8) {
        for _ in 0..from.abs_diff(to) {
            press(text, t, at, if to > from { "R" } else { "L" });
        }
    }
    let mut t = 4800;
    let (lives, mode) = OPTIONS_AT_POWER_ON;
    change(&mut text, &mut t, &at, lives, setup.lives.opt_lives());
    press(&mut text, &mut t, &at, "D");
    change(&mut text, &mut t, &at, mode, setup.difficulty.opt_game_mode());
    press(&mut text, &mut t, &at, "D");
    press(&mut text, &mut t, &at, "D");
    t += 60;
    // The level code's digits, from the original's own `password_table` as the disks hold it.
    let front = &crate::display_data().front;
    for k in 0..6 {
        let digit = front.h0_b(PASSWORD_TABLE - h0(0) + 10 * r.entry + k);
        let code = match digit {
            b'0' => 0x0a,
            b'1'..=b'9' => digit - b'0',
            _ => return Err(format!("password_table entry {} is not digits", r.entry)),
        };
        text += &format!("{}: K{code:02X}\n{}: -\n", at(t), at(t + 6));
        t += 16;
    }
    for d in [40, 140, 200] {
        text += &format!("{}: F\n{}: -\n", at(t + d), at(t + d + 5));
    }
    Ok(text)
}

/// [`SyncPoint::Setup`]: the original at the setup's level's first frame, the setup's values and
/// [`SETUP_FROM_ENGINE`] written in, and the engine's `Game::start_level` with the same setup and
/// `seed`, which is the case's engine. The original's run there is cached by its menu script and
/// drives; the writes are made after, so one snapshot serves every seed, points, energy, spud and
/// wings.
pub fn setup_start(setup: &LevelSetup, seed: Rng, frames: u32) -> Result<(Synced, Game), String> {
    setup.validate()?;
    let id = setup.level;
    let r = route(id).ok_or_else(|| format!("Setup({id}): no such level"))?;
    let script = setup_script(setup, &r)?;
    let mut key = format!("setup start v1\nlevel {id}\n{script}");
    if r.finish_w6_4 {
        key += "W6-4, then level_state = 1\n";
    }
    let search = search(&r);
    let menu = InputScript::parse(&script).map_err(|e| e.to_string())?;
    let mut synced = at_snapshot(&key, r.drives, frames, search, |a| {
        let mut disks = Disks::at(&core_memory(a.memory()));
        let mut forced = false;
        for f in 0..search {
            a.step(menu.input_at(f)).map_err(|e| e.to_string())?;
            disks.serve(a);
            let m = core_memory(a.memory());
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
        Err(format!("Setup({id}): the level not reached in {search} frames"))
    })?;
    let game = Game::start_level(crate::display_data(), setup, seed);
    let a = &mut synced.amiga;
    a.write_memory(SCORE, &setup.points.to_be_bytes());
    a.write_memory(FROG_NEXT_LIFE, &setup.next_life().to_be_bytes());
    a.write_memory(FROG_HEARTS, &(setup.energy as u16).to_be_bytes());
    a.write_memory(FROG_SPUD_ON, &(setup.spud as u16).to_be_bytes());
    a.write_memory(FROG_FLIGHT, &(setup.wings as u16).to_be_bytes());
    for name in SETUP_FROM_ENGINE {
        let d = LAYOUT
            .iter()
            .find(|d| d.name == name)
            .ok_or_else(|| format!("{name} is not a LAYOUT variable"))?;
        a.write_memory(d.addr, &game.peek(d.addr, d.width.len()));
    }
    synced.memory = core_memory(a.memory());
    Ok((synced, game))
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
/// that is not in df0, that disk goes in. Every case runs with one (T192: a level code from
/// power-on loads its world from disk 2); it acts only when the wanted disk changes, so a case
/// whose original never asks for another disk runs as it did without.
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

    /// After a frame of the original's. Only the game's own wanted disk counts: before the game
    /// owns the vertical blank (the boot, disk 3's story intro) h0+$af54 is not its yet.
    pub fn serve(&mut self, a: &mut Amiga) {
        let m = core_memory(a.memory());
        if long(&m, 0x6c) != VBL_HANDLER {
            return;
        }
        let want = long(&m, WANTED_DISK);
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

/// A snapshot of the original written to a file (`examples/whole_game` saves one at each leg's
/// start in the chain, T202): its frame, its input's joystick, the core's bytes.
pub fn save_snapshot_file(path: &std::path::Path, s: &Snapshot) -> std::io::Result<()> {
    let mut b = s.frame.to_le_bytes().to_vec();
    b.push(s.input.joy.bits());
    b.extend_from_slice(&s.bytes);
    fs::write(path, b)
}

/// [`save_snapshot_file`]'s file, read back.
pub fn load_snapshot_file(path: &std::path::Path) -> Result<Snapshot, String> {
    let b = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if b.len() < 5 {
        return Err(format!("{}: not a snapshot", path.display()));
    }
    Ok(Snapshot {
        frame: u32::from_le_bytes(b[0..4].try_into().unwrap()),
        input: FrameInput::joy(Joy(b[4])),
        bytes: b[5..].to_vec(),
    })
}

/// The original powered on as the whole game runs it (`power-on-df1`: disk 1 booting, an empty
/// df1) and returned to the snapshot in `path`, with room for `frames` more frames (T205: the
/// planners start a leg from its state in the chain).
pub fn at_snapshot_file(path: &std::path::Path, frames: u32) -> Result<Amiga, String> {
    let s = load_snapshot_file(path)?;
    let mut a = power_on(DiskSlot::D1, 2, s.frame + frames)?;
    a.restore(&s);
    Ok(a)
}

fn power_on(boot: DiskSlot, drives: u8, frames: u32) -> Result<Amiga, String> {
    let cfg = OracleConfig::locate(frames)
        .map_err(|e| e.to_string())?
        .with_boot(boot)
        .with_drives(drives);
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
/// the step) when the step began at its entry. The acknowledge alone times the set-up's jobs
/// (T56).
pub fn handler_end(paula: &[sf_paula::PaulaWrite], resumes: Option<u32>) -> Option<HandlerEnd> {
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

/// Whether the original is in the story intro (its vertical blank's vector is the intro's), whose
/// memory map is not the game's.
pub fn in_story(m: &AmigaMemory) -> bool {
    long(m, 0x6c) == INTRO_VBL
}

/// Whether the game program (`d1_c03`) owns the vertical blank: its handler is in the vector.
pub fn game_owns_vbl(m: &AmigaMemory) -> bool {
    long(m, 0x6c) == VBL_HANDLER
}

/// COP1LC as the original's CPU last wrote it (`COP1LCH` `$080`, `COP1LCL` `$082`), from `was`
/// and its last step's register writes: the copper list each frame starts with.
pub fn cop1lc(was: Option<u32>, writes: &[sf_oracle::CpuWrite]) -> Option<u32> {
    let mut c = was;
    for w in writes {
        let v = c.unwrap_or(0);
        c = match w.reg {
            0x80 => Some(v & 0xffff | (w.value as u32) << 16),
            0x82 => Some(v & 0xffff_0000 | w.value as u32),
            _ => c,
        };
    }
    c
}
/// The intro's drive check (d3_c04 h0+$c520) after its two head steps: `moveq #0, d0; rts`, no disk.
const INTRO_NO_DISK: u32 = h0(0xc540);
/// What the runner puts there once disk 1 is in: `bra.s h0+$c530`, the disk-change line read again.
const INTRO_READ_AGAIN: [u8; 2] = [0x60, 0xee];

/// One frame of the original, putting disk 1 in df0 once the story intro asks for it (a
/// disk-3 boot; nothing happens on a disk-1 boot).
///
/// The intro's read of disk 1's id (`intro_wait_disk1`) first looks at the disk-change line, finds
/// it low (a disk was swapped), steps the head in and out and looks again 91 lines later: on the
/// hardware the steps have set it high by then (HRM). The reference emulator's drive has (a read at
/// once after that one sees it high) but its CIA's first read after the steps returns the line as it
/// was, so the intro would take disk 1 for no disk and ask again (T47, FINDINGS 2026-10-15). Its
/// "no disk" branch is turned into that read again, so one press of fire goes on as on the hardware.
pub fn step_inserting(a: &mut Amiga, input: FrameInput) -> Result<sf_oracle::FrameCapture, String> {
    let cap = a.step(input).map_err(|e| e.to_string())?;
    let m = core_memory(a.memory());
    if long(&m, 0x6c) == INTRO_VBL
        && !a.disk1_in()
        && main_chain(&a.cpu(), &m).contains(&INTRO_ASKS_DISK1)
    {
        a.insert_disk(0, DiskSlot::D1);
        assert_eq!(
            a.peek(INTRO_NO_DISK, 4),
            [0x70, 0x00, 0x4e, 0x75],
            "the intro's drive check is not where it was measured"
        );
        a.poke(INTRO_NO_DISK, &INTRO_READ_AGAIN);
    }
    Ok(cap)
}

/// The RNG state on the frame the original first seeds it (docs/08 "RNG"): the first frame, once
/// the game owns the vertical blank, on which `rng_state` changes.
fn power_on_seed(
    boot: DiskSlot,
    drives: u8,
    script: &InputScript,
    frames: u32,
) -> Result<Option<Rng>, String> {
    let mut a = power_on(boot, drives, frames)?;
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
    cache_root().join("sync")
}

/// `~/.cache/superfrog-re`, or `SUPERFROG_CACHE` (DESIGN §3.5).
pub(crate) fn cache_root() -> PathBuf {
    std::env::var_os("SUPERFROG_CACHE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/superfrog-re")
        })
}

/// SHA-1 over the format, the vAmiga build, the Kickstart and disk files and the way there.
pub(crate) fn cache_key(cfg: &OracleConfig, what: &str) -> Result<String, String> {
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

    #[test]
    fn a_snapshot_file_reads_back_as_written() {
        let s = Snapshot {
            frame: 14_798,
            input: FrameInput::joy(Joy(Joy::UP.0 | Joy::FIRE.0)),
            bytes: vec![1, 2, 3, 0xff],
        };
        let p = std::env::temp_dir().join(format!("sf-snap-test-{}", std::process::id()));
        save_snapshot_file(&p, &s).unwrap();
        let back = load_snapshot_file(&p).unwrap();
        std::fs::remove_file(&p).unwrap();
        assert_eq!((back.frame, back.input, back.bytes), (s.frame, s.input, s.bytes));
        assert!(load_snapshot_file(&p).is_err());
    }

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

    /// `collide_all`'s frog half counts 2 once the box is taken, then each hitbox entry looked
    /// at (the walk's a6), in the walk, in a touch handler it called (a6 saved under the return
    /// address) and in `frog_hurt` (a0/a5 saved under that); past the walk, all of them. An entry
    /// taken (a6 past it) whose stomp, touch or hurt is not yet decided is not counted (T185).
    #[test]
    fn the_frog_walks_step_counts_the_hitbox_entries_looked_at() {
        let mut m = AmigaMemory {
            chip: vec![0; 0x1000],
            slow: vec![0; 0x80000],
        };
        let mut poke_l = |a: u32, v: u32| {
            let at = (a - sf_core::layout::SLOW_BASE) as usize;
            m.slow[at..at + 4].copy_from_slice(&v.to_be_bytes());
        };
        // Three entries, then the list's end.
        poke_l(HITBOXES + 36, 0x8000_0000);
        let sp = h0(0x30000);
        // A touch handler's return into the walk, the walk's registers saved above it (a6 last),
        // looking at the second entry; `frog_hurt`'s, a0/a5 between, at the third.
        poke_l(sp, h0(0xd1c6));
        poke_l(sp + 4 + 56, HITBOXES + 24);
        let hurt = sp + 0x100;
        poke_l(hurt, h0(0xd1b8));
        poke_l(hurt + 12 + 56, HITBOXES + 36);
        let step = |pc: u32, a6: u32, a7: u32| {
            let mut a = [0; 8];
            a[6] = a6;
            a[7] = a7;
            let cpu = CpuRegs {
                d: [0; 8],
                a,
                pc: h0(pc),
                sr: 0,
                usp: 0,
                ssp: 0,
            };
            pass_step(&cpu, &m)
        };
        assert_eq!(step(0xd0cc, 0, sp), 0);
        assert_eq!(step(0xd0d0, 0, sp), 1);
        assert_eq!(step(0xd0f0, 0, sp), 1);
        // The frog's x and y read: its box taken (T196).
        assert_eq!(step(0xd110, 0, sp), 2);
        assert_eq!(step(0xd126, HITBOXES, sp), 2);
        assert_eq!(step(0xd12c, HITBOXES + 12, sp), 3);
        // The first entry taken, the frog not yet tested against it: W6-4's frame 1090 (T185).
        assert_eq!(step(0xd132, HITBOXES + 12, sp), 2);
        assert_eq!(step(0xd14a, HITBOXES + 12, sp), 2);
        assert_eq!(step(0xd17a, HITBOXES + 12, sp), 2);
        assert_eq!(step(0xd1a8, HITBOXES + 12, sp), 2);
        // Decided: the bounce made, or the call to `frog_hurt` begun.
        assert_eq!(step(0xd17c, HITBOXES + 12, sp), 3);
        assert_eq!(step(0xd1ae, HITBOXES + 12, sp), 3);
        assert_eq!(step(0xd1c6, HITBOXES + 12, sp), 3);
        assert_eq!(step(0x23f86, 0, sp), 4);
        assert_eq!(step(0x1c5bc, 0, hurt), 5);
        assert_eq!(step(0xd1ce, HITBOXES + 36, sp), 5);
        assert_eq!(step(0xd0d4, 0, sp + 0x200), 5);
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
