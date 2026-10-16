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

/// Power-on, OPTIONS, W3-1's level code typed into LEVEL CODE (its digits read from the disks),
/// the options left, START GAME: `game_start_at` takes the code and `top_loop` jumps to world 3's
/// entry, W3's loads from disk 2 (each disk the original asks for put in), W3's loading picture,
/// the progress map, W3-1 played: faithful and alone, nothing unported on the way. Disk 2 is
/// asked for once, on the same frame of play as the original asks (T21: a milestone).
#[test]
fn a_level_code_starts_the_game_at_w3_1_as_the_original_does() {
    use sf_conform::milestones::Milestone;
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("chain-level-code.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut unported, mut world) = (Vec::new(), 0);
    let o = sf_conform::run_with(&case, |_, g| {
        unported = g.unported().to_vec();
        world = g.state().world();
    })
    .unwrap_or_else(|d| panic!("chain-level-code: {d}"));
    let asked: Vec<_> = o
        .milestones
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, m)| matches!(m, Milestone::DiskAsked { .. }))
        .collect();
    assert_eq!(asked, [(1494, Milestone::DiskAsked { disk: 2 })]);
    assert_eq!(unported, Vec::<&str>::new());
    // A code typed wrong starts at W1-1 on both sides, faithful too: the code must have been taken.
    assert_eq!(world, 3, "the run did not end in world 3");
}

/// W1-1's exit (forced, as `tally-w1-1`), the tally, then W1-2 through its progress map and
/// 500 frames of play, faithful and alone.
#[test]
fn w1_1_goes_on_to_w1_2() {
    assert_eq!(run("chain-w1-1-to-w1-2"), Vec::<&str>::new());
}

/// W1-4's exit (forced), the tally, W1's end picture (600 frames), the music faded, W2's effects
/// and music loaded under the picture, W2's loading picture with the frog's walk in from the left,
/// the progress map, W2-1 played: faithful and alone.
#[test]
fn w1_4_goes_on_to_w2_1() {
    assert_eq!(run("chain-w1-4-to-w2-1"), Vec::<&str>::new());
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

/// W5-4's exit (forced), the tally, W5's end picture, the bonus level's effects and music loaded
/// under it, its loading picture held as long as the original's loads (owner, T21), the
/// progress map, the bonus level played: faithful and alone, and the bonus level reached.
#[test]
fn w5_4_goes_on_to_the_bonus_level() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("chain-w5-4-to-bonus.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut unported, mut world) = (Vec::new(), 0);
    if let Err(d) = sf_conform::run_with(&case, |_, g| {
        unported = g.unported().to_vec();
        world = g.state().world();
    }) {
        panic!("chain-w5-4-to-bonus: {d}");
    }
    assert_eq!(unported, Vec::<&str>::new());
    assert_eq!(world, 7, "the run did not end in the bonus level");
}

/// The bonus level played to its end, its end picture, W6's loading picture and the frog's walk,
/// the progress map, W6-1 played: faithful and alone, and W6-1 reached.
#[test]
fn the_bonus_level_goes_on_to_w6_1() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("chain-bonus-to-w6-1.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut unported, mut at) = (Vec::new(), (0, 0));
    if let Err(d) = sf_conform::run_with(&case, |_, g| {
        unported = g.unported().to_vec();
        at = (g.state().world(), g.state().l(sf_core::layout::h0(0x40de)));
    }) {
        panic!("chain-bonus-to-w6-1: {d}");
    }
    assert_eq!(unported, Vec::<&str>::new());
    assert_eq!(at, (6, 1), "the run did not end in W6-1");
}
