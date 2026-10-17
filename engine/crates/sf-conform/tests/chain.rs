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

/// A case's run, faithful and alone, ending in world `world`'s level `level` (the bonus level is
/// world 7) with nothing unported on the way.
fn run_to(name: &str, world: u32, level: u32) {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let (mut unported, mut at) = (Vec::new(), (0, 0));
    if let Err(d) = sf_conform::run_with(&case, |_, g| {
        unported = g.unported().to_vec();
        at = (g.state().world(), g.state().l(sf_core::layout::h0(0x40de)));
    }) {
        panic!("{name}: {d}");
    }
    assert_eq!(unported, Vec::<&str>::new(), "{name}");
    assert_eq!(at, (world, level), "{name}: the run did not end there");
}

/// Game over and continue (T201): game over in W2-2 with 42 700 points (its clock run out), the
/// title with the high-score name entry, W2-1's level code typed on the options screen, W2-1
/// played: faithful and alone. Superfrog has no continue screen; continuing is the level code.
#[test]
fn game_over_then_w2_1_s_code_continues_in_w2_1() {
    run_to("continue-w2", 2, 1);
}

/// Every level code from power-on (T201): OPTIONS, the code typed into LEVEL CODE, START GAME,
/// the world's loads (each disk the original asks for put in), its loading picture and progress
/// map, and the level's first frames, faithful and alone. W3-1's is `chain-level-code`'s; W6-1's
/// runs in `flow.rs` against `Game::start_at`. W6-2..4 run with an empty df1 connected, as the
/// original loads the first map set then, which the engine plays. About 90 s each, so ignored
/// (`conform/README.md`); `cargo test -p sf-conform --test chain -- --ignored`.
macro_rules! code_cases {
    ($($test:ident: $name:literal => ($world:literal, $level:literal),)*) => {$(
        #[test]
        #[ignore = "about 90 s; conform/README.md"]
        fn $test() {
            run_to($name, $world, $level);
        }
    )*};
}

code_cases! {
    code_w1_1: "code-w1-1" => (1, 1),
    code_w1_2: "code-w1-2" => (1, 2),
    code_w1_3: "code-w1-3" => (1, 3),
    code_w1_4: "code-w1-4" => (1, 4),
    code_w2_1: "code-w2-1" => (2, 1),
    code_w2_2: "code-w2-2" => (2, 2),
    code_w2_3: "code-w2-3" => (2, 3),
    code_w2_4: "code-w2-4" => (2, 4),
    code_w3_2: "code-w3-2" => (3, 2),
    code_w3_3: "code-w3-3" => (3, 3),
    code_w3_4: "code-w3-4" => (3, 4),
    code_w4_1: "code-w4-1" => (4, 1),
    code_w4_2: "code-w4-2" => (4, 2),
    code_w4_3: "code-w4-3" => (4, 3),
    code_w4_4: "code-w4-4" => (4, 4),
    code_w5_1: "code-w5-1" => (5, 1),
    code_w5_2: "code-w5-2" => (5, 2),
    code_w5_3: "code-w5-3" => (5, 3),
    code_w5_4: "code-w5-4" => (5, 4),
    code_w6_2: "code-w6-2" => (6, 2),
    code_w6_3: "code-w6-3" => (6, 3),
    code_w6_4: "code-w6-4" => (6, 4),
    code_wb_1: "code-wb-1" => (7, 1),
}

/// The whole game as far as its script replays (T202; T205–T207 extend it): power-on with df1,
/// START GAME, W1-1 completed, its tally's slot machine, W1-2 to W4-4 completed on their chain
/// routes (T205, T206), into W5-1: faithful and alone. The original must have played it, not sat on a
/// screen: a script that never left the title would be just as faithful.
#[test]
#[ignore = "slow: about 25 min"]
fn the_whole_game_plays_as_the_original_does() {
    use sf_conform::milestones::{Milestone, SceneKind};
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("game-whole.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("game-whole: {d}"));
    let met: Vec<_> = o
        .milestones
        .expect("check 4 ran")
        .into_iter()
        .filter(|(_, m)| !matches!(m, Milestone::CoinTaken { .. } | Milestone::ItemTaken { .. }))
        .collect();
    let has = |want: Milestone| met.iter().any(|(_, m)| *m == want);
    let exits: Vec<_> = met
        .iter()
        .filter(|(_, m)| matches!(m, Milestone::LevelExit { .. }))
        .map(|x| x.1)
        .collect();
    let every: Vec<_> = (1..=4)
        .flat_map(|world| (1..=4).map(move |level| Milestone::LevelExit { world, level }))
        .collect();
    assert_eq!(exits, every);
    assert!(has(Milestone::Tally { score: 17800 }), "{met:?}");
    assert!(has(Milestone::Scene(SceneKind::SlotMachine)), "{met:?}");
    assert!(
        has(Milestone::Scene(SceneKind::Level { world: 5, level: 1 })),
        "{met:?}"
    );
    assert!(!has(Milestone::Scene(SceneKind::GameOver)), "{met:?}");
}
