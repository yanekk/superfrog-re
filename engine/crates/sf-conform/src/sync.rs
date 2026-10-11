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
    /// Run the named script from power-on for `frame` frames.
    AtFrame { script: String, frame: u32 },
    /// The first frame of a level, reached the way a player would: W1-1 by START GAME from the
    /// title menu (`scripts/start-w1-1.txt`), every other level of W1–W6 and the bonus level by
    /// typing its level code on the options screen, inserting each disk the game asks for.
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

fn word(m: &AmigaMemory, a: u32) -> u32 {
    (m.byte(a) as u32) << 8 | m.byte(a + 1) as u32
}

fn long(m: &AmigaMemory, a: u32) -> u32 {
    word(m, a) << 16 | word(m, a + 2)
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

/// Bring the oracle to `sync`, with room for `frames` more frames of `script` (the case's own,
/// which from power-on decides when the original seeds its RNG).
pub fn reach(sync: &SyncPoint, script: &InputScript, frames: u32) -> Result<Synced, String> {
    match sync {
        SyncPoint::PowerOn => {
            let seed = power_on_seed(script, frames)?;
            let amiga = power_on(frames)?;
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
            at_snapshot(&key, frames, *frame, |a| {
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
            at_snapshot(&key, frames, LEVEL_SEARCH, |a| {
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

/// The options screen's way into a level (docs/12 "Reaching every level"): from the title menu
/// pick OPTIONS, move to LEVEL CODE (the entry starts as the cursor reaches it), type the level's
/// six digits as raw keys 01..0A, leave the options and START GAME. The digits are read from the
/// original's own `password_table` as it runs, so no code is written down here. Whenever the game
/// asks for a disk, that disk goes in.
fn by_password(id: LevelId, frames: u32) -> Result<Synced, String> {
    let (world, entry) = match id.world {
        World::Bonus if id.level == 1 && !id.alt => (7, 24),
        w @ (World::W2 | World::W3 | World::W4 | World::W5 | World::W6 | World::W1)
            if !id.alt && (1..=4).contains(&id.level) =>
        {
            let n = World::ALL.iter().position(|&x| x == w).unwrap_or(0) as u32;
            (n + 1, 4 * n + id.level as u32 - 1)
        }
        _ => {
            return Err(format!(
                "AtLevel({id}): not reachable by a level code on one drive"
            ));
        }
    };
    // Title menu, OPTIONS, three times down to LEVEL CODE.
    const MENU: &str = "1000: M\n1005: -\n4600: F\n4605: -\n4650: D\n4655: -\n4700: F\n4705: -\n\
                        4800: D\n4810: -\n4840: D\n4850: -\n4880: D\n4890: -\n";
    const TYPE_FROM: u32 = 4980;
    let key = format!("level {id} by level code v1\n{MENU}");
    at_snapshot(&key, frames, PASSWORD_SEARCH, |a| {
        let mut script = InputScript::parse(MENU).map_err(|e| e.to_string())?;
        let mut disk = 0;
        for f in 0..PASSWORD_SEARCH {
            if f == TYPE_FROM {
                let m = core_memory(a.memory());
                let mut text = String::from(MENU);
                let mut t = TYPE_FROM;
                for k in 0..6 {
                    let digit = m.byte(PASSWORD_TABLE + 10 * entry + k);
                    let code = match digit {
                        b'0' => 0x0a,
                        b'1'..=b'9' => digit - b'0',
                        _ => return Err(format!("password_table entry {entry} is not digits")),
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
            if f > TYPE_FROM {
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
            if in_level(&m) && long(&m, WORLD) == world && long(&m, LEVEL) == id.level as u32 {
                return Ok(());
            }
        }
        Err(format!("{id} not reached in {PASSWORD_SEARCH} frames"))
    })
}

fn power_on(frames: u32) -> Result<Amiga, String> {
    let cfg = OracleConfig::locate(frames).map_err(|e| e.to_string())?;
    Amiga::power_on(&cfg).map_err(|e| e.to_string())
}

/// The RNG state on the frame the original first seeds it (docs/08 "RNG"): the first frame, once
/// the game owns the vertical blank, on which `rng_state` changes.
fn power_on_seed(script: &InputScript, frames: u32) -> Result<Option<Rng>, String> {
    let mut a = power_on(frames)?;
    let mut prev = None;
    for f in 0..frames {
        a.step(script.input_at(f)).map_err(|e| e.to_string())?;
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

/// The oracle after `get_there`, from the snapshot cache when it has this key.
fn at_snapshot(
    what: &str,
    frames: u32,
    reach_frames: u32,
    get_there: impl FnOnce(&mut Amiga) -> Result<(), String>,
) -> Result<Synced, String> {
    let cfg = OracleConfig::locate(reach_frames + frames).map_err(|e| e.to_string())?;
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
