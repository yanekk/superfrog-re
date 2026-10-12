//! Sync points: where the original and the engine start a case together (DESIGN §2.1).
//!
//! The original is brought to the sync point on the oracle by its own input, from power-on, and
//! the engine starts from the original's memory there (`Start::Transplant`). Reaching a sync point
//! is a long run, so the oracle's snapshot there is cached outside git, keyed by everything that
//! decides it (`~/.cache/superfrog-re/sync/`, `SUPERFROG_CACHE` replacing the root).

use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use sf_assets::{LevelId, World};
use sf_core::input::{FrameInput, Joy, KeySet};
use sf_core::layout::{AmigaMemory, Ram, h0};
use sf_core::{Pacing, Rng};
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
const VBL_COUNT: u32 = h0(0x9d8);
const LEVEL_STATE: u32 = h0(0xa06);
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
/// "Lag"): late when, in a level, the vertical blank came while the main loop was not parked in
/// `level_frame_loop`'s wait and the pass was due. Every other screen paces on a raster wait and
/// is never late.
pub fn pacing(cpu: &CpuRegs, m: &AmigaMemory) -> Pacing {
    // A step ends at the level-3 handler's entry; the interrupted PC is on the stack (FINDINGS).
    let main_pc = if cpu.pc == VBL_HANDLER {
        long(m, cpu.ssp + 2)
    } else {
        cpu.pc
    };
    if in_level(m) && !LEVEL_WAIT.contains(&main_pc) && word(m, VBL_COUNT) >= 1 {
        Pacing::Late
    } else {
        Pacing::OnTime
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
    match sync {
        SyncPoint::PowerOn | SyncPoint::PowerOnDisk3 => {
            let boot = boot_disk(sync);
            let seed = power_on_seed(boot, script, frames)?;
            let amiga = power_on(boot, frames)?;
            let memory = core_memory(amiga.memory());
            Ok(Synced {
                amiga,
                memory,
                seed,
            })
        }
        SyncPoint::AtFrame { script, frame } => {
            let s = InputScript::load(&script_path(script))?;
            let key = format!("frame {frame} script\n{}", s.to_oracle().canonical());
            at_snapshot(&key, 1, frames, *frame, |a| {
                for f in 0..*frame {
                    a.step(s.input_at(f)).map_err(|e| e.to_string())?;
                }
                Ok(())
            })
        }
        SyncPoint::AtLevel(id) if (id.world, id.level, id.alt) != (World::W1, 1, false) => {
            by_password(*id, frames)
        }
        SyncPoint::AtLevel(id) => {
            let s = InputScript::load(&script_path("start-w1-1"))?;
            let key = format!("level {id} script\n{}", s.to_oracle().canonical());
            at_snapshot(&key, 1, frames, LEVEL_SEARCH, |a| {
                for f in 0..LEVEL_SEARCH {
                    a.step(s.input_at(f)).map_err(|e| e.to_string())?;
                    let m = core_memory(a.memory());
                    if in_level(&m) && long(&m, WORLD) == 1 && long(&m, LEVEL) == 1 {
                        return Ok(());
                    }
                }
                Err(format!("W1-1 not reached in {LEVEL_SEARCH} frames"))
            })
        }
    }
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
fn by_password(id: LevelId, frames: u32) -> Result<Synced, String> {
    let r = route(id).ok_or_else(|| format!("AtLevel({id}): no such level"))?;
    let lead = if r.drives == 1 { 0 } else { DF1_LEAD };
    let type_from = TYPE_FROM - lead;
    let (menu, key) = password_key(id, &r);
    let search = if r.finish_w6_4 {
        FINAL_SEARCH
    } else {
        PASSWORD_SEARCH
    };
    at_snapshot(&key, r.drives, frames, search, |a| {
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
            a.step(script.input_at(f)).map_err(|e| e.to_string())?;
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
    })
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
