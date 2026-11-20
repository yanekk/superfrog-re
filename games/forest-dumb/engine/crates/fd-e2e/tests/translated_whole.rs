//! Translator T40 (DESIGN §1 success criteria, §2.10): the whole translated app as a person plays
//! it. Launch, the disk 2 prompt and the title with its sound, Start and the SCENA 1 screen by
//! key, level 1 on its hand engine through the seam, the one route
//! (`fd-plan/tests/routes/W1-1.txt`) by key presses to the level's end with no life lost, world
//! 2's load and SCENA screen, fire, and [`whole::WORLD_2_PLAY`] steps of world 2's play: the app's
//! state equal to the headless run's after every step, its window to the headless picture on
//! world 2's play. How the inputs are chosen: `tests/whole/mod.rs`.
//!
//! Full and `#[ignore]`d, like `finish_level.rs`: the owner's disks, the generated tree
//! (`translate.py --build`) and about 23 500 steps run twice, headless and in the app.

mod common;
mod whole;

#[test]
#[ignore = "full: the whole translated app, title to world 2, minutes (debug); cargo test -q -p fd-e2e --test translated_whole -- --ignored"]
fn the_translated_app_plays_from_launch_through_level_1_into_world_2_as_headless() {
    let route = common::route();
    let p = whole::plan(route.len(), true);
    // `next_world` sets `world_index` in the route's last step, where `fd-plan`'s replay ends it.
    let end = p.end.expect("the route ends level 1");
    assert_eq!(
        end,
        p.begins + route.len() as u64,
        "the level ends in the route's last step"
    );
    assert!(
        p.lives.iter().all(|&l| l == p.lives[0]),
        "no life lost on the way"
    );
    let world_2 = p.world_2.expect("world 2's play begins");
    assert_eq!(p.steps(), world_2 + whole::WORLD_2_PLAY - 1);
    let sound = whole::play_in_the_app(&p, 1_800);
    assert!(
        whole::heard(&sound, 1, p.begins),
        "the title is silent in the app"
    );
    assert!(
        whole::heard(&sound, p.begins + 1, end),
        "level 1's effects are silent in the app"
    );
    eprintln!(
        "T40: level 1 begins at step {}, ends at {end}, world 2's play from {world_2} to {}",
        p.begins,
        p.steps()
    );
}
