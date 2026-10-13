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
