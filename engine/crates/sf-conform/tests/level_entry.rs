//! `Start::Level` (T14) against the original: each of the 29 levels, entered by its level code on
//! the reference emulator (the final level by W6-4's, then finishing it), against the engine's
//! entry, at frame 0 (the frame `level_frame_loop` starts on).
//!
//! The frog's position and the camera must be equal, and so must the picture and every modelled
//! variable, but for what the original's way there leaves and the engine's entry does not go
//! through, listed below with the task that models it.

use sf_assets::LevelId;
use sf_conform::{InputScript, SyncPoint, display_data, sync};
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarDef};
use sf_core::{Game, Start, View};

/// The 29 levels a game plays (W6-1's map is on the disks twice; no route loads the copy).
fn ids() -> Vec<LevelId> {
    let ids: Vec<LevelId> = sf_conform::game_data()
        .levels
        .keys()
        .copied()
        .filter(|id| id.to_string().parse::<LevelId>().is_ok())
        .collect();
    assert_eq!(ids.len(), 29);
    ids
}

/// Variables the engine's entry leaves as it does not go the original's way: (names, why, task).
const NOT_MODELLED: &[(&[&str], &str, &str)] = &[
    (
        &[
            "front_a0a",
            "title_blink",
            "title_menu",
            "intro_blit",
            "mouse",
            "display_timer",
            "opt_flash",
            "password_buf",
            "text_ptr",
            "kbd_toggle",
            "sprite_recs",
            "front_copper",
            "fades",
            "rng_state",
        ],
        "what the front end leaves: the title, the level code typed, its fades and sprites, the RNG \
         the boot seeds from the clock (a Start::Level game has no seed)",
        "T21",
    ),
    (
        &[
            "exit_sprite_copper",
            "exit_sprite_left",
            "exit_sprite_right",
            "playfield_buffer_0",
            "playfield_buffer_1",
            "playfield_clean",
            "sfx_chan0",
            "p41_channels",
            "music_fade",
        ],
        "what the screens on the way leave: the progress map's frog sprites, the display buffers \
         outside the picture, the music's place (it plays from the world's intro screen on)",
        "T21",
    ),
    (
        &[
            "line_offsets",
            "bob_records",
            "bob_frames",
            "bob_free_list",
            "obj_count",
            "obj_slots",
        ],
        "the objects the level starts with, and their bobs",
        "T15–T18",
    ),
];

/// The final level is reached by finishing W6-4: what that level and its exit leave.
const FINAL_LEVEL: &[&str] = &[
    "level_copper",
    "frog_climb",
    "frog_18",
    "frog_1a",
    "frog_22",
    "frog_40",
    "frog_50",
    "frog_52",
    "frog_7c",
    "hitboxes",
    "shown_buffer",
    "bob_draw_vars",
    "frog_sprite_left",
    "frog_sprite_right",
    "bob_draw_list",
];

fn original(id: LevelId) -> AmigaMemory {
    sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), 1)
        .unwrap_or_else(|e| panic!("{id}: {e}"))
        .memory
}

/// Every variable with a piece that differs: (name, pieces differing, the first one's original
/// and engine values).
fn diffs(theirs: &AmigaMemory, game: &Game) -> Vec<(&'static str, usize, u32, u32)> {
    let pieces: Vec<VarDef> = LAYOUT.iter().flat_map(|d| d.pieces()).collect();
    let mut out: Vec<(&'static str, usize, u32, u32)> = Vec::new();
    for (p, (_, ours)) in pieces.iter().zip(game.probe()) {
        let t = p.read(theirs as &dyn Ram);
        if t != ours {
            match out.last_mut() {
                Some(l) if l.0 == p.name => l.1 += 1,
                _ => out.push((p.name, 1, t, ours)),
            }
        }
    }
    out
}

#[test]
fn every_level_is_entered_as_the_original_enters_it() {
    let display = display_data();
    let mut failures = Vec::new();
    for id in ids() {
        let theirs = original(id);
        let game = Game::with_display(display.clone(), Start::Level(id));
        let d = diffs(&theirs, &game);
        for must in ["frog_x", "frog_y", "camera_x", "camera_y"] {
            if let Some(v) = d.iter().find(|v| v.0 == must) {
                failures.push(format!("{id}: {} original ${:x}, engine ${:x}", v.0, v.2, v.3));
            }
        }
        let excused = |name: &str| {
            NOT_MODELLED.iter().any(|g| g.0.contains(&name))
                || (id.to_string() == "WW-1" && FINAL_LEVEL.contains(&name))
        };
        for v in d.iter().filter(|v| !excused(v.0)) {
            failures.push(format!(
                "{id}: {} ({} pieces) original ${:x}, engine ${:x}",
                v.0, v.1, v.2, v.3
            ));
        }
        // The picture: the original's memory rendered against the engine's. W6-4 shows a line
        // of the progress map's frog sprites (row 15), which the engine's entry has not drawn.
        let render = |g: &Game| sf_core::to_rgb12(&g.render(View::default()));
        let original_picture = render(&Game::with_display(
            display.clone(),
            Start::Transplant(theirs),
        ));
        let ours = render(&game);
        let width = game.render(View::default()).width as usize;
        let rows: std::collections::BTreeSet<usize> = original_picture
            .iter()
            .zip(&ours)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i / width)
            .collect();
        let allowed: &[usize] = if id.world == sf_assets::World::W6 && id.level == 4 {
            &[15]
        } else {
            &[]
        };
        if rows.iter().any(|r| !allowed.contains(r)) {
            failures.push(format!("{id}: the picture differs on rows {rows:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every variable that differs at frame 0, each level: for finding what an entry still lacks.
#[test]
#[ignore]
fn report() {
    for id in ids() {
        let game = Game::with_display(display_data(), Start::Level(id));
        let d = diffs(&original(id), &game);
        println!("{id}: {} variables differ", d.len());
        for (n, k, t, o) in d {
            println!("  {n} ({k}): original {t:x}, engine {o:x}");
        }
    }
}

/// A game started at W1-1 plays as the original does from there: the `frog-run-right` script
/// (running, jumping two pits, the big hill) for 500 frames, the original's objects frozen out as
/// in the frog cases; the frog, the camera and the picture equal every frame. The picture shows
/// that the display buffers' leftovers outside it never scroll into view.
#[test]
fn a_game_started_at_a_level_plays_as_the_original() {
    use sf_core::layout::{VarId, def};
    let display = display_data();
    let id: LevelId = "W1-1".parse().unwrap();
    let script = InputScript::load(&sf_conform::case::script_path("frog-run-right")).unwrap();
    let frames = 500;
    let synced = sync::reach_frozen(&SyncPoint::AtLevel(id), &script, frames, true).unwrap();
    let mut amiga = synced.amiga;
    let mut game = Game::with_display(display.clone(), Start::Level(id));
    let watched = [VarId::FrogX, VarId::FrogY, VarId::CameraX, VarId::CameraY];
    for f in 0..frames {
        sf_conform::freeze::freeze_objects(&mut amiga, &display);
        let input = script.input_at(f);
        let theirs = amiga.step(input).unwrap();
        let mem = sync::core_memory(amiga.memory());
        let ours = game.step_paced(input, sync::pacing(&amiga.cpu(), &mem));
        let probe = game.probe();
        for v in watched {
            let d = def(v);
            let at = LAYOUT
                .iter()
                .take_while(|e| e.id != v)
                .flat_map(|e| e.pieces())
                .count();
            assert_eq!(
                probe[at].1,
                d.read(&mem as &dyn Ram),
                "frame {f}: {}",
                d.name
            );
        }
        assert!(
            sf_conform::compare::picture(&ours.picture, &theirs.picture).is_none(),
            "frame {f}: the picture differs"
        );
    }
}
