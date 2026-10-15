//! The disk-3 intro against the original booted from disk 3 (T32): the cases `story-*` in
//! `engine/conform/cases/`.

/// The engine frame scene 1's mask blits start in (`objects_init`, h0+$8782).
const SCENE_1_MASKS: u32 = 532;

/// Power-on with disk 3 through the logo, the intro's load, the first text page and the first
/// scene with its music and effects, frame for frame (the case `story-intro`), in the test block.
/// Its comparison of Paula writes leaves their beam out; a draw's DMACON `$8400` is checked on its
/// line here: from engine frame 538 on, scene 1's draws the handler lets go of start from the
/// original's fed handler end (T54). The earlier ones (521, 536, 537) start elsewhere.
/// Scene 1's mask blits' DMACON `$8400` is checked on its line too (T56).
#[test]
fn the_intro_from_disk_3_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("story-intro.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut draws = Vec::new();
    let mut masks = None;
    let result = sf_conform::run_observing(&case, |f, ours, theirs| {
        let starts = |ws: &[sf_paula::PaulaWrite]| {
            ws.iter()
                .filter(|w| w.reg == 0x96 && w.value == 0x8400)
                .map(|w| w.vpos)
                .collect::<Vec<_>>()
        };
        if f >= 538 && !starts(theirs).is_empty() {
            draws.push((f, starts(ours), starts(theirs)));
        }
        if f == SCENE_1_MASKS {
            masks = Some((starts(ours), starts(theirs)));
        }
    });
    if let Err(d) = result {
        panic!("{d}");
    }
    // Scene 1's set-up job (T56) starts in the frame the text page's list is shown, under its
    // three planes, though the engine has already switched to the scene's list: its mask blits'
    // DMACON $8400 lands on the original's line only when that frame is timed under the old list.
    let (ours, theirs) = masks.expect("engine frame 532 is reached");
    assert!(!theirs.is_empty(), "the original's mask blits start in engine frame 532");
    assert_eq!(ours, theirs, "engine frame 532: scene 1's mask blits' DMACON $8400 lines");
    assert!(draws.len() >= 5, "draws checked: {draws:?}");
    for (f, ours, theirs) in &draws {
        assert_eq!(ours, theirs, "engine frame {f}: the draw's DMACON $8400 lines");
    }
}

/// Power-on with disk 3 through scene 1's fade out, whose set-up runs with interrupts off across
/// frame 1242's end (T49), text page 2 and into scene 2's set-up (the case `story-scene1-out`).
/// About 90 s: out of the test block, run with `cargo test -p sf-conform -- --ignored`.
#[test]
#[ignore = "slow: about 90 s"]
fn the_intro_through_scene_1s_fade_out_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("story-scene1-out.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
    }
}

/// Power-on with disk 3 into scene 4 (the case `story-scene4-init`): its set-up runs under the
/// scene's display (T50), the masks' clearing slowed by the fetch, and the second mask blit holds
/// the CPU through a whole frame, so the original's step runs two; the first draw after the masks
/// is timed by its CPU program (T53). About 80 s: out of the test block, run with
/// `cargo test -p sf-conform -- --ignored`.
#[test]
#[ignore = "slow: about 80 s"]
fn scene_4s_set_up_under_the_display_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("story-scene4-init.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
    }
}

/// Power-on with disk 3 through scenes 4 to 6 (the case `story-scene4-sound`, T55): the effects'
/// ramps and the vertical blank's counters keep in step through the draws that hold the CPU
/// through a whole frame (one handler for two blanks, fed by conformance). Every Paula write left
/// differing is a draw's end or what follows it (DMACON, INTENA: T53, T54). Past engine frame
/// 6682 a handler's wait runs on into the next frame, its effects' step and acknowledge with it
/// (T97). About 2 min: out of the test block, run with `cargo test -p sf-conform -- --ignored`.
#[test]
#[ignore = "slow: about 2 min"]
fn the_effects_keep_in_step_through_scenes_4_to_6() {
    // The Paula writes themselves: `--strict`.
    let case = sf_conform::Case {
        mode: sf_conform::Mode::Strict,
        ..sf_conform::Case::load(&sf_conform::cases_dir().join("story-scene4-sound.toml"))
            .unwrap_or_else(|e| panic!("{e}"))
    };
    for d in sf_conform::scan(&case) {
        if let Some((v, _, _)) = &d.first_var {
            assert_ne!(v.name, "sfx_records", "{d}");
        } else if let Some(p) = &d.paula_diff {
            let first = p.lines().next().unwrap_or("");
            assert!(
                first.contains("DMACON") || first.contains("INTENA"),
                "{d}"
            );
        }
    }
}

/// Power-on with disk 3 through the whole intro, untouched, to the "insert disk 1" page (the case
/// `story-intro-full`, T56): every scene's set-up timed on the bus with the original's handlers,
/// its play, its fades and the text pages. About 5 min: out of the test block, run with
/// `cargo test -p sf-conform -- --ignored`.
#[test]
#[ignore = "slow: about 5 min"]
fn the_whole_intro_to_the_insert_disk_page_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("story-intro-full.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
    }
}
