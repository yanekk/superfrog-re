//! What a case leaves out of the original because the engine does not model it yet (every
//! exclusion listed in the case file with the task that will model it).
//!
//! `enemies`: the objects the engine has no kind for never come into play in the original. Each
//! world's marker stub in `obj_flag_jumptab` that the engine's dispatch has no spawner for
//! (enemies and each world's scenery, T16–T18) returns at once, and so does each spawn the frog's
//! own code makes that no task has ported yet (the bubble rising from a pushing surface, T17),
//! patched into the original's code from power-on, as soon as the game is in memory. Items,
//! pickups, switches, pads and launchers (T15) play as they do.
//! `panel`: the status panel's bitmap is not compared, as a variable or in the picture.

use sf_core::DisplayData;
use sf_core::layout::h0;
use sf_core::level::standard_dispatch;
use sf_oracle::Amiga;

/// `obj_flag_jumptab` (h0+$178ec): 7 worlds × 64 `bra.w` stubs.
const JUMPTAB: u32 = 0x178ec;
const WORLDS: u32 = 7;
const MARKERS: u32 = 64;
/// Frog-side spawns not modelled, as `(jsr's address, task)`: `frog_air_anim`'s bubble
/// (template 86, h0+$1130a).
const UNPORTED_SPAWNS: [(u32, &str); 1] = [(0x1130a, "T17")];

fn word(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

/// The `(world, marker)` stubs frozen: those the engine's dispatch has no spawner for.
pub fn frozen_markers() -> Vec<(u8, u8)> {
    let d = standard_dispatch();
    (0..WORLDS as u8)
        .flat_map(|w| (0..MARKERS as u8).map(move |m| (w, m)))
        .filter(|&(w, m)| d.marker(w, m).is_none())
        .collect()
}

/// What the freeze leaves out, as a line for the sync point's cache key: a snapshot taken with
/// a different set frozen is a different snapshot.
pub fn tag() -> String {
    let markers: Vec<String> = frozen_markers()
        .iter()
        .map(|(w, m)| format!("{w}:{m}"))
        .collect();
    let spawns: Vec<String> = UNPORTED_SPAWNS.iter().map(|s| format!("{:x}", s.0)).collect();
    format!("frozen v2 {} / {}\n", markers.join(","), spawns.join(","))
}

/// The frog-side spawns not modelled, as `(address, bytes)` patches over the original's code:
/// for a routine test, whose original is Moira's memory rather than the oracle's.
pub fn unported_spawn_patches() -> Vec<(u32, Vec<u8>)> {
    UNPORTED_SPAWNS
        .iter()
        .map(|&(at, _)| (h0(at), vec![0x4e, 0x71, 0x4e, 0x71, 0x4e, 0x71]))
        .collect()
}

/// Patch the original's unmodelled objects out, once the game's hunk 0 is in memory (each patch
/// checks its code is the executable's before writing; already patched code is left alone).
pub fn freeze_objects(a: &mut Amiga, display: &DisplayData) {
    let rom = &display.rom().h0;
    if rom.len() < (JUMPTAB + 4 * WORLDS * MARKERS) as usize {
        return;
    }
    let mem = a.peek(h0(0), rom.len().min(0x30000));
    for (w, m) in frozen_markers() {
        let o = (JUMPTAB + 4 * (w as u32 * MARKERS + m as u32)) as usize;
        if mem[o..o + 4] == rom[o..o + 4] {
            a.poke(h0(o as u32), &[0x4e, 0x75, 0x4e, 0x71]);
        }
    }
    for &(at, _) in &UNPORTED_SPAWNS {
        let o = at as usize;
        if word(&mem, o) == 0x4eb9 && mem[o..o + 6] == rom[o..o + 6] {
            a.poke(h0(at), &[0x4e, 0x71, 0x4e, 0x71, 0x4e, 0x71]);
        }
    }
}
