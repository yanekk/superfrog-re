//! The disk-3 intro against the original booted from disk 3 (T32): the cases `story-*` in
//! `engine/conform/cases/`.

/// Power-on with disk 3 through the logo, the intro's load, the first text page and the first
/// scene with its music and effects, frame for frame (the case `story-intro`), in the test block.
#[test]
fn the_intro_from_disk_3_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("story-intro.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
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
/// the CPU through a whole frame, so the original's step runs two. Nothing differs before the
/// first draw ends (frame 3191, its fourth write), whose timing is T52–T54's. About 80 s: out of
/// the test block, run with `cargo test -p sf-conform -- --ignored`.
#[test]
#[ignore = "slow: about 80 s"]
fn scene_4s_set_up_under_the_display_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("story-scene4-init.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let found = sf_conform::scan(&case);
    let first = found
        .first()
        .expect("the first draw's end differs until T52-T54");
    assert_eq!(first.frame, 3191, "{first}");
    assert!(
        first
            .paula_diff
            .as_deref()
            .is_some_and(|d| d.starts_with("write #3:")),
        "{first}"
    );
}

/// Power-on with disk 3 through scenes 4 to 6 (the case `story-scene4-sound`, T55): the effects'
/// ramps and the vertical blank's counters keep in step through the draws that hold the CPU
/// through a whole frame (one handler for two blanks, fed by conformance). Every Paula write left
/// differing is a draw's end or what follows it (DMACON, INTENA: T53, T54). About 80 s: out of
/// the test block, run with `cargo test -p sf-conform -- --ignored`.
#[test]
#[ignore = "slow: about 80 s"]
fn the_effects_keep_in_step_through_scenes_4_to_6() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("story-scene4-sound.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
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
