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
