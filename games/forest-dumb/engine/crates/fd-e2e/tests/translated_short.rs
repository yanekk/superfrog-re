//! Translator T40, the short run (DESIGN §2.10): `translated_whole.rs`'s start, launch to the title
//! with its sound, Start by key, level 1 begun on the program's own step, and the one route's first
//! [`ROUTE`] steps by key presses, the app's state the headless run's every step. About 1 300 steps.
//!
//! Full: needs the owner's disks and the generated tree (`translate.py --build`).

mod common;
mod whole;

/// The route's steps played.
const ROUTE: usize = 600;

#[test]
fn the_title_start_and_the_routes_first_600_steps_play_in_the_app_as_headless() {
    let p = whole::plan(ROUTE, false);
    assert_eq!(p.end, None, "the level does not end in {ROUTE} steps");
    assert!(p.lives.iter().all(|&l| l == p.lives[0]), "no life lost");
    let sound = whole::play_in_the_app(&p, 300);
    assert!(
        whole::heard(&sound, 1, p.begins),
        "the title is silent in the app"
    );
}
