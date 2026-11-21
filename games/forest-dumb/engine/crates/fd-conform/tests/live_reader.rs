//! `fd_core::live` on headless runs (plans/forest-dumb-map-page T06): the translated program from
//! power-on, level 1's hand engine mounted as the app mounts it, and the hand engine's `Game`. Full:
//! the owner's disks and the tree built in (`tools/translate.py --build`), which
//! `fd-translated` takes from `games/forest-dumb/build/translator/tree/` or `FD_TRANSLATED`.

use std::path::Path;
use std::sync::Arc;

use amiga_oracle::{FrameInput, KeySet};
use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_core::live::{Live, live_from_game, live_from_ram};
use fd_core::recording::{TO_LEVEL_1_FRAMES, in_level_1, location_in_ram, to_level_1_input};
use fd_core::{Game, GameData, HUNK_BASES, Mem, Start, Transplant, layout_var};
use fd_translated::TranslatedGame;

const CHIP: usize = 512 << 10;
/// The route's opening the smoke guard plays (`full_route.rs`).
const STEPS: usize = 600;
/// `items_left` at W1-1's start, decoded.
const W1_ITEMS: u16 = 282;

fn generated() {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
}

/// The level file's object map of `world`, column-major as RAM holds it.
fn level_map(world: u8) -> Option<Vec<u8>> {
    let dir = fd_tools::disk_dir().ok()?;
    let files = |name: &str| {
        fd_tools::game_file(&dir, &format!("disk2/{name}"))
            .map_err(|e| fd_assets::AssetError::missing(name, e))
    };
    fd_assets::read_level(world, &files)
        .ok()
        .map(|l| l.objects.cells)
}

fn ram(e: &TranslatedEngine) -> (&[u8], &[u8]) {
    let m = e.mem();
    m.split_at(CHIP.min(m.len()))
}

fn engine(data: Arc<GameData>) -> TranslatedEngine {
    generated();
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut e = TranslatedEngine::power_on(&disks).unwrap_or_else(|e| panic!("{e}"));
    e.mount_level_1(data);
    e
}

fn frame(e: &mut TranslatedEngine, input: FrameInput, what: &str) {
    e.frame(input);
    if let Some(s) = FrameEngine::stopped(e) {
        panic!("{what}: the translated program stopped: {s}");
    }
}

/// Power-on to W1-1's first frame of play, as the app's `--level 1` start.
fn to_level_1(e: &mut TranslatedEngine) {
    for n in 0..TO_LEVEL_1_FRAMES {
        frame(e, to_level_1_input(n), "to level 1");
        let (chip, slow) = ram(e);
        if in_level_1(chip, slow) {
            return;
        }
    }
    panic!("no level 1 in {TO_LEVEL_1_FRAMES} frames");
}

/// T84's route (`fd-plan/tests/routes/W1-1.txt`), its first `n` steps.
fn route(n: usize) -> Vec<FrameInput> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fd-plan/tests/routes/W1-1.txt");
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let s = amiga_conform::script::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    (0..n as u32).map(|k| s.input_at(k)).collect()
}

/// The coins taken by `l`: changed cells that held an item (`$20` to `$27`, docs/10) and are 0.
fn taken(l: &Live, file: &[u8]) -> usize {
    l.changed
        .iter()
        .filter(|&&(c, r, v)| v == 0 && (0x20..=0x27).contains(&file[c as usize * 96 + r as usize]))
        .count()
}

#[test]
#[ignore = "full: the disks and the generated tree; power-on to W1-1 and 600 steps, about a minute"]
fn the_route_s_opening_reads_the_character_items_and_taken_cells() {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut e = engine(data);
    to_level_1(&mut e);
    let file = level_map(1).expect("W1-1's level file");
    let start = |w: u8| level_map(w);
    let mut prev = None;
    let mut last_items = W1_ITEMS;
    let mut plays = 0;
    for (k, input) in route(STEPS).into_iter().enumerate() {
        frame(&mut e, input, "the route");
        let (chip, slow) = ram(&e);
        let l = live_from_ram(chip, slow, prev, &start);
        prev = Some(l.prev());
        if !l.playing {
            // A frame no pass ended in: `irq_count` stood.
            continue;
        }
        plays += 1;
        assert_eq!(l.world, Some(1), "step {k}");
        let at = location_in_ram(chip, slow).expect("in level 1");
        let (x, y) = l.char_xy.unwrap();
        assert_eq!((x / 16, y / 16), (at.col as u32, at.row as u32), "step {k}");
        let items = l.items_left.unwrap();
        assert!(
            items <= last_items,
            "step {k}: items left rose {last_items} to {items}"
        );
        last_items = items;
        assert_eq!(
            taken(&l, &file),
            (W1_ITEMS - items) as usize,
            "step {k}: {:?}",
            l.changed
        );
    }
    assert!(plays > STEPS * 9 / 10, "{plays} of {STEPS} steps in play");
    // `ONE_ROUTE_OPENING_ITEMS` in full_route.rs, measured on the original.
    assert_eq!(last_items, 272);
}

/// Chip RAM then slow RAM as the original's hunks see it, for [`Game`]'s transplant.
struct Flat<'a>(&'a [u8]);

impl Mem for Flat<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        let (chip, slow) = self.0.split_at(CHIP.min(self.0.len()));
        let at = HUNK_BASES[hunk as usize] + offset;
        (0..len as u32)
            .map(|k| {
                let a = (at + k) as usize;
                match a.checked_sub(0xC0_0000) {
                    Some(i) => slow.get(i),
                    None => chip.get(a),
                }
                .copied()
                .unwrap_or(0)
            })
            .collect()
    }
}

/// The hand engine and the translated run read the same at matched passes: at each pass end of
/// the route's opening the translated program's RAM is taken into a `Game` as the seam's mount
/// takes it (`Transplant`), and both readers give one `Live`. A `Game` stepped on its own is not
/// compared pass by pass: from power-on the translated program reads each step's input on its
/// own frame, so the two runs drift a pass and a pixel apart (measured 2026-11-20); they end the
/// opening with the same items taken.
#[test]
#[ignore = "full: the disks and the generated tree; power-on to W1-1 and 600 steps beside the hand engine"]
fn the_hand_engine_reads_as_the_translated_run_at_matched_passes() {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let file = level_map(1).expect("W1-1's level file");
    let steps = route(STEPS);
    let mut e = engine(data.clone());
    to_level_1(&mut e);
    let start = |w: u8| level_map(w);
    let mut g = Game::new(data.clone(), Start::Level1);
    let (mut prev, mut matched) = (None::<fd_core::live::Prev>, 0);
    let mut last = None;
    for &input in &steps {
        frame(&mut e, input, "the route");
        for ram in e.pass_end_ram().to_vec() {
            let (chip, slow) = ram.split_at(CHIP);
            let l = live_from_ram(chip, slow, prev, &start);
            prev = Some(l.prev());
            if !l.playing {
                continue;
            }
            g.transplant(&Flat(&ram));
            assert_eq!(live_from_game(&g, &file), l, "irq_count {}", l.irq);
            matched += 1;
            last = Some(l);
        }
    }
    assert!(
        matched > STEPS * 9 / 10,
        "{matched} passes matched of {STEPS}"
    );
    // The hand engine on its own, the same steps: the same items taken by the opening's end.
    let mut alone = Game::new(data, Start::Level1);
    for &input in &steps {
        alone.step(input);
        alone.take_events();
    }
    let alone = live_from_game(&alone, &file);
    let last = last.unwrap();
    let coins = |l: &Live| -> Vec<(u16, u16)> {
        let mut v: Vec<_> = l
            .changed
            .iter()
            .filter(|&&(c, r, v)| {
                v == 0 && (0x20..=0x27).contains(&file[c as usize * 96 + r as usize])
            })
            .map(|&(c, r, _)| (c, r))
            .collect();
        v.sort_unstable();
        v
    };
    assert_eq!(
        (alone.items_left, coins(&alone)),
        (last.items_left, coins(&last))
    );
}

#[test]
#[ignore = "full: the disks and the generated tree; power-on through world 2's load, about a minute"]
fn world_2_through_its_way_in_reads_world_2_from_its_first_pass() {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut e = engine(data);
    // `cases/world-2-start.toml`'s way in: `world_index` 1 written as `load_world` is entered.
    let wi = layout_var("world_index").unwrap();
    e.write_at((0, 0x578), HUNK_BASES[0] + wi.offset, &1u32.to_be_bytes())
        .unwrap();
    let start = |w: u8| level_map(w);
    let mut prev = None;
    for n in 0..TO_LEVEL_1_FRAMES * 3 {
        frame(&mut e, to_level_1_input(n), "to world 2");
        let (chip, slow) = ram(&e);
        let l = live_from_ram(chip, slow, prev, &start);
        if prev.is_some_and(|p: fd_core::live::Prev| p.irq != l.irq) {
            // The first pass of play: `irq_count` counts from 0, the title takes no interrupt.
            assert!(l.playing && l.world == Some(2), "frame {n}: {l:?}");
            assert!(l.char_xy.is_some() && l.items_left.is_some(), "{l:?}");
            return;
        }
        assert!(!l.playing, "frame {n}");
        prev = Some(l.prev());
    }
    panic!("no play in world 2 in {} frames", TO_LEVEL_1_FRAMES * 3);
}

#[test]
#[ignore = "full: the disks and the generated tree; power-on to W1-1, Esc and the game over screen"]
fn esc_to_the_game_over_screen_is_not_playing_within_two_samples() {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut e = engine(data);
    to_level_1(&mut e);
    let start = |w: u8| level_map(w);
    // The app's sample: every 12th frame.
    const EVERY: u32 = 12;
    let esc = FrameInput {
        keys: KeySet::key(0x45),
        ..FrameInput::default()
    };
    let mut prev = None;
    let mut samples = Vec::new();
    for n in 0..600u32 {
        // Idle 100 frames, Esc held 3 (`death-esc.txt`), then nothing: the game over screen
        // waits for fire.
        let input = if (100..103).contains(&n) {
            esc
        } else {
            FrameInput::default()
        };
        frame(&mut e, input, "esc");
        if n % EVERY == EVERY - 1 {
            let (chip, slow) = ram(&e);
            let l = live_from_ram(chip, slow, prev, &start);
            prev = Some(l.prev());
            samples.push((n, l));
        }
    }
    let (before, after): (Vec<_>, Vec<_>) = samples.iter().partition(|(n, _)| *n < 100);
    assert!(
        before.iter().skip(1).all(|(_, l)| l.playing),
        "play before Esc"
    );
    // `main_loop` acts on `give_up` within 32 passes of its release (docs/09): the last pass is by
    // frame 103 + 32, and two samples after it nothing plays.
    let stood = after
        .iter()
        .position(|(n, _)| *n >= 103 + 32)
        .expect("samples after Esc");
    let over = &after[stood + 2..];
    assert!(!over.is_empty());
    for (n, l) in over {
        assert!(!l.playing && l.irq != 0, "frame {n}: {l:?}");
        assert!(l.char_xy.is_none() && l.world.is_none(), "frame {n}: {l:?}");
    }
}
