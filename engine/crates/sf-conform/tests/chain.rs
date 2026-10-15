//! The game's flow (T183): from the title to a level, and from one level to the next, as
//! `top_loop` runs it (`sf_core::front::chain`), against the original on the oracle.

fn run(name: &str) -> Vec<&'static str> {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut unported = Vec::new();
    if let Err(d) = sf_conform::run_with(&case, |_, g| unported = g.unported().to_vec()) {
        panic!("{name}: {d}");
    }
    unported
}

/// Power-on, START GAME, W1's loading picture (the frog's walk, its loads beneath it), the
/// progress map, W1-1 played: faithful and alone, nothing unported on the way.
#[test]
fn start_game_reaches_w1_1_as_the_original_does() {
    assert_eq!(run("chain-start-w1-1"), Vec::<&str>::new());
}

/// W1-1's exit (forced, as `tally-w1-1`), the tally, then W1-2 through its progress map and
/// 500 frames of play, faithful and alone.
#[test]
fn w1_1_goes_on_to_w1_2() {
    assert_eq!(run("chain-w1-1-to-w1-2"), Vec::<&str>::new());
}

/// P in W1-1 pauses (T191: `pause`, h0+$7442, the pass stopped at its pause check, the vertical
/// blank's paused path), P again resumes the pass where it stopped, then play on: faithful and
/// alone.
#[test]
fn p_pauses_and_p_resumes_as_the_original_does() {
    assert_eq!(run("chain-pause"), Vec::<&str>::new());
}

/// P with fire held pauses; the pause waits for fire to be let go, then fire ends it as P does.
#[test]
fn fire_ends_the_pause_as_the_original_does() {
    assert_eq!(run("chain-pause-fire"), Vec::<&str>::new());
}
