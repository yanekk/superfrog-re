//! The level sync point: the original at the first frame of play in W1-1, reached by
//! `boot-to-level1` from power-on (docs/09-play-measured.md) and cached outside git, so a level case
//! starts there in seconds rather than after the boot (plans/forest-dumb-level-1 DESIGN §2.3,
//! §3.5). The snapshot is the toolkit's `amiga_conform::sync_cache`, keyed by everything that
//! decides the state there: a changed script, disk, Kickstart or emulator build rebuilds it.

use std::path::{Path, PathBuf};

use amiga_conform::{script, sync_cache};
use amiga_oracle::{Amiga, OracleConfig, default_cache, kickstart_path, read_only};

use crate::{conform_dir, disks};

/// Frames run before the first frame of play: SCENA 1's fire is held in step 6300, whose frame
/// already runs the first level-3 interrupt (docs/09-play-measured.md).
pub const LEVEL_1_SYNC: u32 = 6301;
/// Disk 2 goes into df0 before this frame runs (`boot-to-level1` shares boot-to-title's swap).
pub const DISK_2_SWAP: u32 = 3600;
/// The script that reaches it, under `engine/conform/scripts/`.
pub const LEVEL_1_SCRIPT: &str = "boot-to-level1";

/// Bump when the way there or the cache file layout changes.
const SYNC_FORMAT: u32 = 1;
const MAGIC: &[u8; 8] = b"FDSYNC\0\0";

/// The toolkit's cache root (`AMIGA_TOOLKIT_CACHE`, else `~/.cache/amiga-toolkit`), `sync/`.
pub fn cache_dir() -> PathBuf {
    default_cache().join("sync")
}

/// The original at [`LEVEL_1_SYNC`] on the owner's Kickstart and disks, from the cache when it
/// holds it, with room for `frames` more frames.
pub fn reach_level_1(frames: u32) -> Result<Amiga, String> {
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disk_dir = fd_tools::disk_dir()?;
    reach_level_1_in(&kickstart, &disk_dir, &cache_dir(), frames)
}

/// [`reach_level_1`] on the Kickstart at `kickstart`, the zips in `disk_dir` and the cache under
/// `cache`. A missing input is an error naming its path, before any emulation.
pub fn reach_level_1_in(
    kickstart: &Path,
    disk_dir: &Path,
    cache: &Path,
    frames: u32,
) -> Result<Amiga, String> {
    let rom = read_only(kickstart).map_err(|e| e.to_string())?;
    let disks = disks::read(disk_dir)?;
    let (text, sc) = level_1_script()?;
    let key = level_1_key(&rom, &disks, &text);
    let cap = LEVEL_1_SYNC
        .checked_add(frames)
        .ok_or_else(|| format!("{frames} frames after the sync point is past any frame cap"))?;
    let cfg = OracleConfig::new(kickstart.to_path_buf(), disks, cap);
    sync_cache::at_snapshot(
        cache,
        MAGIC,
        key,
        || Amiga::power_on(&cfg).map_err(|e| e.to_string()),
        |am| {
            for frame in 0..LEVEL_1_SYNC {
                if frame == DISK_2_SWAP {
                    am.insert_disk(0, 1);
                }
                am.step(script::input_at(&sc.lines, frame))
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        },
    )
}

/// The cache key of the level sync point on this Kickstart, these disks and this script text, one
/// drive connected.
pub fn level_1_key(rom: &[u8], disks: &[Vec<u8>], script_text: &str) -> [u8; 20] {
    let disks: Vec<&[u8]> = disks.iter().map(Vec::as_slice).collect();
    sync_cache::cache_key(SYNC_FORMAT, rom, &disks, 1, &level_1_what(script_text))
}

/// The `what` of the level-1 key: the way there, the script as written included.
fn level_1_what(script_text: &str) -> String {
    format!(
        "level W1-1 sync {LEVEL_1_SYNC} swap disk 2 at {DISK_2_SWAP}\nscript {LEVEL_1_SCRIPT}\n{script_text}"
    )
}

/// `boot-to-level1` as written, and parsed.
pub fn level_1_script() -> Result<(String, script::InputScript), String> {
    let path = conform_dir()
        .join("scripts")
        .join(format!("{LEVEL_1_SCRIPT}.txt"));
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("script {}: {e}", path.display()))?;
    let sc = script::parse(&text).map_err(|e| format!("script {}: {e}", path.display()))?;
    Ok((text, sc))
}
