//! What `fd-app` and the tests that drive it share: the state hash its capture's `hashes.txt`
//! holds, so a test computes it on a headless [`Game`] the same way; and for the translated
//! program (translator T37) the window its frames are shown through, its state hash, and
//! `--level 1`'s way through the title, so a test drives a headless run the same way.

pub mod audio;

use std::hash::{Hash, Hasher};

use amiga_input::{FrameInput, Joy};
use pcode_machine::Picture;
use fd_core::{Game, HUNK_BASES, layout_var};

/// The capture's file of state hashes: `N hash` a line, hash in hex, for each frame N (0 the start).
pub const HASHES: &str = "hashes.txt";

/// The game's play state as a number: its scene, variables, objects and collision plane, what
/// [`Game`]'s equality compares (the data aside, which is the same for a run). Stable within one
/// build, which is all a test of that build needs.
pub fn state_hash(g: &Game) -> u64 {
    // DefaultHasher::new() has fixed keys: two processes of one build agree.
    let mut h = std::collections::hash_map::DefaultHasher::new();
    g.scene().hash(&mut h);
    g.state().hash(&mut h);
    g.objects().hash(&mut h);
    g.plane().hash(&mut h);
    h.finish()
}

/// One line of [`HASHES`].
pub fn hash_line(n: u64, g: &Game) -> String {
    format!("{n} {:016x}\n", state_hash(g))
}

/// What the app says, and exits 1 on, when it is built with no translated program (translator
/// DESIGN §2.10, §2.14): the stub tree, every run of which stops at once.
pub const NOT_GENERATED: &str = "The translated program is not built in: generate it with \
`python3 -I games/forest-dumb/tools/translate.py --build` from the repository's root, then build \
the app again. `--engine hand` plays level 1 on the hand engine meanwhile.";

/// The display window the translated program's frames are shown through: left, top, width,
/// height. The machine's picture is the program's own window, which changes from screen to screen
/// (measured 2026-11-19, T37: the disk 2 prompt 320×200 from line 64, the title 320×238 from 42,
/// play's first frame 320×255 from 42, play 320×250 from 47, all from column 129); this is the
/// least window holding every one, so the app's window keeps one size.
pub const CANVAS: (i32, u16, u16, u16) = (129, 42, 320, 256);

/// `p` through [`CANVAS`]: what the app shows of a translated frame.
pub fn canvas(p: &Picture) -> Picture {
    let (left, top, width, height) = CANVAS;
    p.rewindow(left, top, width, height)
}

/// The translated program's state as a number: chip RAM then slow RAM at a frame's end, what a
/// headless run of `fd-translated` holds at the same frame. Stable within one build.
pub fn memory_hash(chip: &[u8], slow: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    h.write(chip);
    h.write(slow);
    h.finish()
}

/// One line of [`HASHES`] for the translated program.
pub fn memory_hash_line(n: u64, chip: &[u8], slow: &[u8]) -> String {
    format!("{n} {:016x}\n", memory_hash(chip, slow))
}

/// `--level 1`'s way through the title (translator DESIGN §2.10): from power-on, fire pressed for
/// ten frames and released for ten, over and over. It passes the disk 2 prompt, presses START on
/// the title and fire on the SCENA 1 screen; frame `n` is the `n`th the program runs, from 0.
pub fn to_level_1_input(n: u64) -> FrameInput {
    let mut i = FrameInput::default();
    if n / 10 % 2 == 1 {
        i.joy = Joy::FIRE;
    }
    i
}

/// `--level 1`'s seatbelt: the frames the program may take to reach play from power-on (it takes
/// about 500 on [`to_level_1_input`], T37).
pub const TO_LEVEL_1_FRAMES: u64 = 3_000;

/// Whether the translated program is playing world 1, from chip RAM and slow RAM at a frame's end:
/// `irq_count` counts the passes of play from 1 at its first frame and is 0 before (docs/09), and
/// `world_index` is 0 in world 1.
pub fn in_level_1(chip: &[u8], slow: &[u8]) -> bool {
    let value = |name: &str| {
        let v = layout_var(name).expect("in LAYOUT");
        let at = HUNK_BASES[v.hunk as usize] + v.offset;
        (0..v.width.bytes() as u32).fold(0u32, |acc, k| {
            let a = ((at + k) & 0xFF_FFFF) as usize;
            let b = match a.checked_sub(SLOW_BASE) {
                Some(i) => slow.get(i),
                None => chip.get(a),
            };
            acc << 8 | b.copied().unwrap_or(0) as u32
        })
    };
    value("irq_count") != 0 && value("world_index") == 0
}

/// Where slow RAM starts in the A500's address space.
const SLOW_BASE: usize = 0xC0_0000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_names_the_command_that_generates_the_tree() {
        assert!(NOT_GENERATED.contains("python3 -I games/forest-dumb/tools/translate.py --build"));
        assert!(NOT_GENERATED.contains("--engine hand"));
    }

    #[test]
    fn the_way_to_level_1_presses_fire_ten_frames_in_twenty() {
        let fire: Vec<bool> = (0..40)
            .map(|n| to_level_1_input(n).joy.contains(Joy::FIRE))
            .collect();
        assert!(fire[..10].iter().all(|f| !f) && fire[10..20].iter().all(|&f| f));
        assert!(fire[20..30].iter().all(|f| !f) && fire[30..].iter().all(|&f| f));
    }

    #[test]
    fn level_1_is_play_counted_in_world_1() {
        let (chip, mut slow) = (vec![0u8; 512 << 10], vec![0u8; 512 << 10]);
        assert!(!in_level_1(&chip, &slow));
        let irq = layout_var("irq_count").unwrap();
        let at = (HUNK_BASES[irq.hunk as usize] + irq.offset) as usize;
        let put = |chip: &mut Vec<u8>, slow: &mut Vec<u8>, a: usize, b: u8| match a.checked_sub(SLOW_BASE) {
            Some(i) => slow[i] = b,
            None => chip[a] = b,
        };
        let mut chip = chip;
        put(&mut chip, &mut slow, at + 1, 1);
        assert!(in_level_1(&chip, &slow));
        let w = layout_var("world_index").unwrap();
        let wat = (HUNK_BASES[w.hunk as usize] + w.offset) as usize;
        put(&mut chip, &mut slow, wat + 3, 1);
        assert!(!in_level_1(&chip, &slow), "world 2 is not level 1");
    }

    #[test]
    fn the_canvas_holds_every_window_the_program_shows() {
        for (left, top, w, h) in [(129, 64, 320, 200), (129, 42, 320, 238), (129, 42, 320, 255), (129, 47, 320, 250)] {
            let (cl, ct, cw, ch) = CANVAS;
            assert!(left >= cl && left + w <= cl + cw as i32, "{left} {w}");
            assert!(top >= ct && top + h <= ct + ch, "{top} {h}");
        }
        let p = canvas(&Picture::blank(129, 47, 320, 250));
        assert_eq!((p.width, p.height), (CANVAS.2, CANVAS.3));
    }
}
