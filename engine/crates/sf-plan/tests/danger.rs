//! `core::danger` on real enemies: states reached by replaying committed conformance scripts from
//! `Start::Level`, unpaced, as the planner plays (the scripts were fitted to the paced original, but
//! the frames below are the engine's own, measured at T07).

use sf_core::frog::{FROG, f};
use sf_core::{Game, Start};
use sf_plan::core::*;

/// `level` entered as the original enters it, then `frames` frames of the script `name`.
fn after(level: &str, name: &str, frames: u32) -> (Game, sf_conform::InputScript) {
    let text =
        std::fs::read_to_string(sf_conform::scripts_dir().join(format!("{name}.txt"))).unwrap();
    let script = sf_conform::InputScript::parse(&text).unwrap();
    let mut g = Game::with_display(
        sf_conform::display_data(),
        Start::Level(level.parse().unwrap()),
    );
    for i in 0..frames {
        g.step_unseen(script.input_at(i));
    }
    (g, script)
}

/// The script's frames `a..b` as held joystick states and their lengths.
fn runs(script: &sf_conform::InputScript, a: u32, b: u32) -> Vec<(u8, u16)> {
    let mut v: Vec<(u8, u16)> = Vec::new();
    for i in a..b {
        let j = script.input_at(i).joy.0;
        match v.last_mut() {
            Some((k, n)) if *k == j => *n += 1,
            _ => v.push((j, 1)),
        }
    }
    v
}

fn vx(g: &Game) -> i16 {
    g.state().w(FROG + f::VX) as i16
}

/// W1-1 along `enemies-w1-1`: at frame 395 the frog is on the ground at (58,33), moving right at
/// 5; walking left 40 frames meets an enemy (heart lost at about frame 431 of the script).
#[test]
fn a_walk_into_an_enemy_is_safe_after_a_wait() {
    let (g, _) = after("W1-1", "enemies-w1-1", 395);
    assert_ne!(g.state().w(FROG + f::ON_GROUND), 0);
    let walk = [(4u8, 40u16)];

    let hit = play(&g, &walk);
    assert_eq!(hit.hearts_lost, 1);
    assert!(!hit.died);
    assert_eq!(hit.wait, 0);

    let (o, wait) = play_safe(&g, &walk);
    let w = wait.expect("a wait clears the enemy");
    assert!(WAITS.contains(&w), "{w}");
    assert_eq!(o.wait, w);
    assert!(o.safe());
    // The earliest: no shorter wait in `WAITS` was safe.
    for &shorter in WAITS.iter().take_while(|&&s| s < w) {
        let tried = play(&g, &[(0, shorter), (4, 40)]);
        assert!(!tried.safe(), "{shorter}");
    }
    // The variant is the idle frames and then the walk, nothing else.
    let by_hand = play(&g, &[(0, w), (4, 40)]);
    assert_eq!(by_hand.end.state(), o.end.state());
}

/// W1-1's first frames: the frog walks right from its start with nothing near enough to hurt it.
/// The move as given is returned, with no wait.
#[test]
fn a_move_with_no_enemy_near_is_played_once() {
    let (g, _) = after("W1-1", "enemies-w1-1", 0);
    let walk = [(8u8, 40u16)];
    let once = play(&g, &walk);
    assert!(once.safe());
    let (o, wait) = play_safe(&g, &walk);
    assert_eq!(wait, Some(0));
    assert_eq!(o.wait, 0);
    assert_eq!(o.end.state(), once.end.state());
}

/// W2-1 along `items-life-w2-1`: the lift into column 63 onto the extra life at 63,35 (lives 2 to
/// 3 at about frame 895). A life gained is neither a heart lost nor a death.
#[test]
fn a_one_up_is_not_a_loss() {
    let (g, script) = after("W2-1", "items-life-w2-1", 860);
    let mv = runs(&script, 860, 900);
    let o = play(&g, &mv);
    let lives = |g: &Game| g.state().w(FROG + f::LIVES);
    assert_eq!(lives(&o.end), lives(&g) + 1);
    assert_eq!(o.hearts_lost, 0);
    assert!(!o.died);
    assert_eq!(play_safe(&g, &mv).1, Some(0));
}

#[test]
fn deterministic_for_the_same_start() {
    let (g, _) = after("W1-1", "enemies-w1-1", 395);
    let walk = [(4u8, 40u16)];
    let (a, wa) = play_safe(&g, &walk);
    let (b, wb) = play_safe(&g, &walk);
    assert_eq!(wa, wb);
    assert_eq!(
        (a.hearts_lost, a.died, a.wait),
        (b.hearts_lost, b.died, b.wait)
    );
    assert_eq!(a.end.state(), b.end.state());
}

/// W1-1 along `enemies-w1-1`: at frame 392 the frog is on the ground at full speed (6, its
/// `MAX_SPEED`); a running jump to the left meets an enemy. The wait variant starts stopped (the
/// idle frames bleed the speed), so it lands somewhere else: a different move (DESIGN §2.5).
#[test]
fn a_wait_variant_of_a_run_jump_lands_elsewhere() {
    let (g, _) = after("W1-1", "enemies-w1-1", 392);
    assert_ne!(g.state().w(FROG + f::ON_GROUND), 0);
    assert_eq!(vx(&g), g.state().w(FROG + f::MAX_SPEED) as i16);
    let jump = [(5u8, 12u16), (4, 28)];

    let at_speed = play(&g, &jump);
    assert!(!at_speed.safe());
    let (o, wait) = play_safe(&g, &jump);
    let w = wait.expect("a wait clears the enemy");
    assert!(w > 0 && o.wait == w);
    assert!(o.safe());
    // The idle frames stop the frog: the variant is a standing-start jump.
    assert_eq!(vx(&play(&g, &[(0, w)]).end), 0);
    assert_ne!(frog_cell(o.end.state()), frog_cell(at_speed.end.state()));
}

#[test]
fn a_lost_heart_costs_more_than_any_detour() {
    for max in [0u32, 1, 48, 30 * 60 * 50, u32::MAX] {
        assert!(heart_cost(max) > max as u64);
    }
}

/// W1-1 along `frog-pit`: running right from the start into the first pit (columns 27–30) kills
/// the frog with `frog_hearts` unchanged, so a death is unsafe without a heart lost. No wait helps:
/// the run still ends in the pit, and the original outcome comes back with `None`.
#[test]
fn a_pit_death_without_a_heart_lost_is_unsafe() {
    let (g, _) = after("W1-1", "frog-pit", 0);
    let run = [(8u8, 260u16)];
    let o = play(&g, &run);
    assert!(o.died);
    assert_eq!(o.hearts_lost, 0);
    assert!(!o.safe());
    // `play` stops at the death frame: a longer run ends in the same state.
    assert_eq!(play(&g, &[(8, 400)]).end.state(), o.end.state());
    let (back, wait) = play_safe(&g, &run);
    assert_eq!(wait, None);
    assert_eq!((back.died, back.wait), (true, 0));
    assert_eq!(back.end.state(), o.end.state());
}
