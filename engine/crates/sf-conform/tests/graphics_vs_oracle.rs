//! `sf-assets`' decoded graphics against the original (DESIGN §4), here rather than in
//! `sf-assets/tests` because it runs the emulator on two threads (the purity test keeps threads,
//! files and processes out of the pure crates' tests): the pictures against the screens the
//! reference emulator shows, the sheets, tiles and font against the copies the game keeps in RAM,
//! and both ANIMs, frame by frame, against the game's own player run on the routine oracle.
//!
//! Two runs from power-on (input as in `sf-oracle/tests/exe_map_claims.rs`), on two threads:
//! - `attract`: left on the title: the title (MPIC), the tables (AWA1, AWA2), the hall (HPIC).
//! - `play`: into W1-1: the progress map, the loading picture (L1LP), the level's RAM.
//!
//! Not reached by either, and decoded by the same code as one that is: W2–W6 and the bonus
//! level's pictures, the end-of-world pictures (AWA's format), FRUT/FRUC/FRUM (MPIC's and HERO's).

use std::sync::OnceLock;

use sf_assets::*;
use sf_oracle::*;

fn chunks() -> &'static [(ChunkId, Vec<u8>)] {
    static CHUNKS: OnceLock<Vec<(ChunkId, Vec<u8>)>> = OnceLock::new();
    CHUNKS.get_or_init(|| {
        let files: Vec<sf_disk::Offered> = sf_conform::disks::read_disk_files()
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .map(|(name, bytes)| sf_disk::Offered { name, bytes })
            .collect();
        let set = sf_disk::identify(&files).unwrap_or_else(|e| panic!("{e:?}"));
        unpack(&set).unwrap_or_else(|e| panic!("{e}"))
    })
}

fn data() -> &'static GameData {
    static DATA: OnceLock<GameData> = OnceLock::new();
    DATA.get_or_init(|| decode(chunks()).unwrap_or_else(|e| panic!("{e}")))
}

/// The first chunk the catalogue calls `name`, decompressed.
fn named_chunk(name: &str) -> &'static [u8] {
    let id = catalogue(chunks())
        .unwrap()
        .into_iter()
        .find(|c| c.0 == name)
        .unwrap()
        .1;
    &chunks().iter().find(|c| c.0 == id).unwrap().1
}

const H0: u32 = 0xC0_0FA0;
const H1: u32 = 0xC4_92EC;
const H3: u32 = 0x77FC;
const LOGIC_FLAG: u32 = H0 + 0x9d6;
const LEVEL_STATE: u32 = H0 + 0xa06;

fn joy(bits: u8) -> FrameInput {
    FrameInput::joy(Joy(bits))
}

/// A picture to look for on screen: in frames `from..to`, the screen equals rows `row..row + h` of
/// the picture (`palette`: its own; `None`: only check that each (line, colour index) pair shows
/// one colour, for a picture whose palette the copper sets).
struct Watch {
    id: PictureId,
    frames: std::ops::Range<u32>,
}

#[derive(Debug, Default, Clone)]
struct Seen {
    /// Best fraction of the screen's pixels the picture explains, and the frame.
    best: (u64, u64, u32),
}

/// How much of `screen` the picture explains, as (pixels explained, pixels).
fn explained(p: &Picture, screen: &Rgb12Frame) -> (u64, u64) {
    let (w, h) = (
        screen.width.min(p.bitmap.width),
        screen.height.min(p.bitmap.height),
    );
    let total = w as u64 * h as u64;
    match p.palette {
        Some(pal) => {
            let mut same = 0;
            for y in 0..h {
                for x in 0..w {
                    same += (pal.0[p.bitmap.pixel(x, y) as usize] == screen.pixel(x, y)) as u64;
                }
            }
            (same, total)
        }
        None => {
            // Per line, each colour index's most common colour.
            let mut same = 0;
            for y in 0..h {
                let mut count = std::collections::BTreeMap::<(u8, u16), u64>::new();
                for x in 0..w {
                    *count
                        .entry((p.bitmap.pixel(x, y), screen.pixel(x, y)))
                        .or_default() += 1;
                }
                let mut best = std::collections::BTreeMap::<u8, u64>::new();
                for ((i, _), n) in count {
                    let b = best.entry(i).or_default();
                    *b = (*b).max(n);
                }
                same += best.values().sum::<u64>();
            }
            (same, total)
        }
    }
}

struct Run {
    seen: Vec<(PictureId, Seen)>,
    /// RAM on the title (attract) or at the first frame of W1-1 (play).
    memory: AmigaMemory,
}

fn run(
    script: InputScript,
    frames: u32,
    watches: &[Watch],
    keep: impl Fn(&AmigaMemory, u32) -> bool,
) -> Run {
    let d = data();
    let cfg = OracleConfig::locate(frames).unwrap_or_else(|e| panic!("{e}"));
    let mut a = Amiga::power_on(&cfg).unwrap();
    let mut seen: Vec<(PictureId, Seen)> =
        watches.iter().map(|w| (w.id, Seen::default())).collect();
    let mut memory = None;
    for f in 0..frames {
        let cap = a.step(script.input_at(f)).unwrap();
        for (w, s) in watches.iter().zip(seen.iter_mut()) {
            if w.frames.contains(&f) && f % 5 == 0 {
                let (same, total) = explained(&d.pictures[&w.id], &cap.picture);
                let (b_same, b_total, _) = s.1.best;
                if b_total == 0 || same as u128 * b_total as u128 > b_same as u128 * total as u128 {
                    s.1.best = (same, total, f);
                }
            }
        }
        if memory.is_none() {
            let m = a.memory();
            if keep(&m, f) {
                memory = Some(m);
                if watches.iter().all(|w| w.frames.end <= f) {
                    break;
                }
            }
        }
    }
    Run {
        seen,
        memory: memory.expect("the run never reached the moment its memory is kept at"),
    }
}

fn attract() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| {
        let s = InputScript::new(vec![(1_000, joy(0x20)), (1_005, FrameInput::NONE)]);
        let w = [
            Watch {
                id: PictureId::Title,
                frames: 4_450..4_550,
            },
            Watch {
                id: PictureId::Awards1,
                frames: 5_100..5_300,
            },
            Watch {
                id: PictureId::Awards2,
                frames: 5_550..5_750,
            },
            Watch {
                id: PictureId::HallOfTalent,
                frames: 7_300..7_420,
            },
        ];
        // Seen: the title is up and HERO is at h1 from ~4 360.
        run(s, 7_420, &w, |_, f| f == 4_500)
    })
}

fn play() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| {
        let s = InputScript::new(vec![
            (1_000, joy(0x20)),
            (1_005, FrameInput::NONE),
            (4_600, joy(0x10)),
            (4_605, FrameInput::NONE),
            (4_700, joy(0x10)),
            (4_705, FrameInput::NONE),
        ]);
        let w = [
            Watch {
                id: PictureId::Loading(World::W1),
                frames: 5_250..5_700,
            },
            Watch {
                id: PictureId::ProgressMap,
                frames: 6_000..6_800,
            },
        ];
        run(s, 7_400, &w, |m, _| {
            m.word(LOGIC_FLAG) != Some(0) && (m.long(LEVEL_STATE).unwrap() as i32) < 0
        })
    })
}

/// Both runs at once: each is minutes of emulation on one core.
fn runs() -> (&'static Run, &'static Run) {
    std::thread::scope(|s| {
        let a = s.spawn(attract);
        let p = s.spawn(play);
        (a.join().unwrap(), p.join().unwrap())
    })
}

#[test]
fn pictures_are_the_screens_the_original_shows() {
    let (a, p) = runs();
    for (id, s) in a.seen.iter().chain(&p.seen) {
        let (same, total, frame) = s.best;
        println!("{id:?}: {same}/{total} pixels at frame {frame}");
        assert_eq!(same, total, "{id:?}: best frame {frame}");
    }
}

fn ram(m: &AmigaMemory, at: u32, len: usize) -> Vec<u8> {
    (0..len as u32).map(|k| m.byte(at + k).unwrap()).collect()
}

#[test]
fn sheets_tiles_and_the_font_are_the_copies_in_ram() {
    let (a, p) = runs();
    let d = data();
    let hero = d.sprites.iter().find(|s| s.id == SpriteId::Hero).unwrap();
    let planar = hero.sheet.to_planar();
    assert_eq!(
        ram(&a.memory, H1, planar.len()),
        planar,
        "HERO on the title, at h1"
    );
    let font = &d.fonts[0];
    let strip = ram(&a.memory, H0 + 0x378ca, 320);
    for (k, g) in font.glyphs.iter().enumerate() {
        for y in 0..8 {
            assert_eq!(strip[y * 40 + k], g[y], "glyph {}", font.charset[k] as char);
        }
    }
    let w1 = d.world(World::W1);
    let tiles = w1.tiles.encode();
    assert_eq!(
        ram(&p.memory, H1, tiles.len()),
        tiles,
        "L1BM at W1-1's start"
    );
    let bobs = w1.bobs.sheet.to_planar();
    assert_eq!(ram(&p.memory, H3, bobs.len()), bobs, "L1BO at W1-1's start");
}

/// The level copper list holds the level palette: COLOR01..31 as `move` pairs, four bytes apart.
/// COLOR06 is the one the game changes while it plays (colour cycling, inferred).
#[test]
fn w1_1_shows_the_level_palette() {
    let (_, p) = runs();
    let pal = data().world(World::W1).palette;
    let chip = &p.memory.chip;
    let moves: Vec<u8> = (1..32u16)
        .flat_map(|k| {
            [
                (0x180 + 2 * k).to_be_bytes(),
                pal.0[k as usize].to_be_bytes(),
            ]
            .concat()
        })
        .collect();
    let at = chip
        .windows(8)
        .position(|w| w == &moves[..8])
        .expect("no copper move of W1's COLOR01 and COLOR02");
    let differ: Vec<usize> = (0..31)
        .filter(|&k| chip[at + 4 * k..at + 4 * k + 4] != moves[4 * k..4 * k + 4])
        .map(|k| k + 1)
        .collect();
    println!("W1 palette copper moves at chip {at:#x}; differ: {differ:?}");
    assert_eq!(differ, [6]);
}

/// The end sequence's player (h0+$ab02 opens an ANIM into two buffers, h0+$ab82 draws the next
/// frame into the other one), run on Moira over the game loaded at 0x100000 with the file at h3.
/// Every frame it draws equals ours.
#[test]
fn anims_are_what_the_game_s_own_player_draws() {
    let d = data();
    let game = named_chunk("game");
    for (id, name) in [(AnimId::End2, "END2"), (AnimId::End1, "END1")] {
        let mut cpu = Cpu68k::new(AmigaMemory {
            chip: vec![0; 512 * 1024],
            slow: vec![0; 512 * 1024],
        });
        let map = cpu.load_exe(game, 0x10_0000).unwrap();
        let h0 = |off| map.addr(0, off);
        let h3 = |off| map.addr(3, off);
        cpu.write(h3(0), named_chunk(name));
        let bufs = [h3(0x3d090), h3(0x49890)];
        call(&mut cpu, h0(0xab02), |r| {
            r.a[0] = bufs[0];
            r.a[1] = bufs[1];
            r.a[2] = h3(0);
            r.a[3] = h0(0x3b18c);
            r.d[0] = 0x28;
            r.d[1] = 0x100;
        });
        let anim = &d.anims[&id];
        let mut drawn = 0;
        loop {
            let want = anim.frames[drawn].bitmap.to_planar();
            let got = cpu.read(bufs[drawn % 2], want.len()).to_vec();
            assert!(got == want, "{name} frame {drawn} differs from the game's");
            drawn += 1;
            if drawn == anim.frames.len()
                || call(&mut cpu, h0(0xab82), |r| r.a[0] = h0(0x3b18c)) != 0
            {
                break;
            }
        }
        println!(
            "{name}: the game drew {drawn} of {} frames, all equal",
            anim.frames.len()
        );
        assert!(
            drawn + 2 >= anim.frames.len(),
            "{name}: the player stopped at {drawn}"
        );
    }
}

/// Call a routine with the stack at 0x80000 and keep what it wrote (a call's writes are undone
/// when it returns). Its `d0`.
fn call(cpu: &mut Cpu68k, entry: u32, set: impl Fn(&mut Regs)) -> u32 {
    let mut r = Regs::default();
    r.a[7] = 0x8_0000;
    set(&mut r);
    let res = cpu
        .call(entry, r, 50_000_000)
        .unwrap_or_else(|e| panic!("{entry:#x}: {e:?}"));
    for &(addr, _, new) in &res.writes {
        cpu.write(addr, &[new]);
    }
    res.regs.d[0]
}
