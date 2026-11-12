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

// Power-on cases (`start = "entry"`, translator T32): the original from power-on to the step that
// first runs the program's entry, where the translated program starts with the machine's OS
// answers; and the loads both sides are lined up at (`align = "loads"`).

/// The program's entry, `h0+$0` (docs/07-executable-map.md).
pub const ENTRY: (u8, u32) = (0, 0x0);

/// Each hunk's length in memory, BSS included (docs/07-executable-map.md, "hunks"), at
/// `HUNK_BASES`: the bytes a power-on case's `memory` compares.
pub const HUNK_SIZES: [u32; 5] = [0x5be68, 0x1280, 0x1830, 0x3aa28, 0x1c94c];

/// The file routines of docs/07 ("dos.library and the file table"), by entry: `load_file`,
/// `load_file_raw`, `read_marker`. A load is one call of one of them; the translated program
/// loads at once (DESIGN §2.8), the original in seconds. `save_scores_unreferenced` is never called.
pub const LOAD_ROUTINES: [(u8, u32); 3] = [(0, 0x8ec4), (0, 0x8f58), (0, 0x8fd8)];

/// Their `rts` instructions, where the original is seen returning: `load_file` has two (after
/// `unpack_esl` at `h0+$8f56`, and `h0+$905e` after the copy of a file that is not `ESL!`, which
/// only `load_file`'s `bne.w` reaches), `load_file_raw` `h0+$8fd6`, `read_marker` `h0+$9056`.
pub const LOAD_RETURNS: [(u8, u32); 4] = [(0, 0x8f56), (0, 0x905e), (0, 0x8fd6), (0, 0x9056)];

/// The most frames the original may take over one load the translated program did at once:
/// measured, `_BALLA`'s load with `_OPCJE`'s unpack took 697 (T32).
pub const LOAD_CAP: u32 = 3000;

/// Bytes of the program's hunks that hold what the OS handed it, (hunk, offset, length): the
/// machine answers the OS with values of its own (T26: dos base, file handle, the font's fields,
/// copinit and LOFlist, the saved level 2 and 3 vectors), and the INTENAR, DMACONR and ADKCONR the
/// entry saves at `h0+$1e3a` (T32, measured: the OS's `$602c`, `$03f0`, `$11..` against the
/// machine's 0). A power-on case does not compare them; everything else in the hunks it does.
pub const OS_VALUES: [(u8, u32, u32); 9] = [
    (0, 0x1e26, 4),
    (0, 0x1e2e, 4),
    (0, 0x1e32, 4),
    (0, 0x1e36, 4),
    (0, 0x1e3a, 6),
    (0, 0x1e54, 4),
    (0, 0x1e58, 2),
    (0, 0x908e, 4),
    (0, 0x909e, 4),
];

/// The level 3 autovector and the program's handler there (`level3_handler`): while it is
/// installed the program has the machine, and every custom write is its own (T32; docs/07). The
/// OS's interrupts write INTREQ and the rest while it runs, which the machine does not model.
pub const LEVEL3_VECTOR: u32 = 0x6C;
pub const LEVEL3_HANDLER: (u8, u32) = (0, 0x1eda);

/// The original from power-on, `case`'s script and swap, up to the end of the step that first runs
/// the program's entry, with room for the rest of the case's frames, and the raster line the
/// entry ran on. `watch` are the PCs whose hits the run counts from then on (the entry's own is
/// added first); the entry step's hits are the caller's to read.
///
/// The line: Kickstart enters the program partway through a frame (line 179 of frame 3461 on
/// boot-to-title's input, T32), and the program draws the disk 2 prompt at once, so a translated
/// run started at line 0 has drawn more of it by the frame's end. The line is the first whose end
/// finds the CPU in the program's code, sampled (`Amiga::set_profile`, which leaves the run
/// unchanged) on every step up to the entry's, as which step that is is known only after it.
pub fn reach_entry(
    case: &crate::Case,
    inputs: &crate::Inputs,
    watch: &[u32],
) -> Result<(Amiga, u16), String> {
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disks = disks::load()?;
    let cfg = OracleConfig::new(kickstart, disks, case.frames + 1);
    let mut am = Amiga::power_on(&cfg).map_err(|e| e.to_string())?;
    let entry = crate::HUNK_BASES[ENTRY.0 as usize] + ENTRY.1;
    let mut pcs = vec![entry];
    pcs.extend_from_slice(watch);
    am.watch_pcs(&pcs);
    am.set_profile(true);
    let in_code = |pc: u32| {
        [0usize, 4]
            .iter()
            .any(|&h| (crate::HUNK_BASES[h]..crate::HUNK_BASES[h] + HUNK_SIZES[h]).contains(&pc))
    };
    while am.frame() < case.frames {
        let frame = am.frame();
        if let Some(s) = case.swap.filter(|s| s.frame == frame) {
            am.insert_disk(0, s.disk as usize - 1);
        }
        am.step(inputs.at(frame)).map_err(|e| e.to_string())?;
        if am.pc_hits().iter().any(|&(pc, n)| pc == entry && n > 0) {
            let line = am
                .line_samples()
                .iter()
                .find(|l| in_code(l.pc))
                .map(|l| l.vpos)
                .ok_or("the entry's step: no line ended in the program's code")?;
            am.set_profile(false);
            return Ok((am, line));
        }
    }
    Err(format!(
        "{}: the program's entry h0+$0 did not run within the case's {} frames",
        case.name, case.frames
    ))
}
