//! Save states (T26, DESIGN §2.8): loading a state and running on gives the frames the game would
//! have given had it never been saved, in every kind of scene; a state from another build or on
//! other game data, or a damaged one, is refused with the matching error.

use std::sync::OnceLock;

use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, GameData, LevelId, Rng, Scene, Start, StateError};

const BUILD: &str = "superfrog test-build";

fn data() -> &'static GameData {
    sf_conform::game_data()
}

fn hash() -> &'static [u8; 32] {
    static HASH: OnceLock<[u8; 32]> = OnceLock::new();
    HASH.get_or_init(|| sf_assets::cache_hash(data()))
}

/// Frame `f`'s input: a walk right with jumps and fire, varied so the play after a save is not
/// still.
fn input(f: u32) -> FrameInput {
    let mut joy = Joy::NONE;
    if f % 90 < 60 {
        joy = joy | Joy::RIGHT;
    } else if f % 90 < 75 {
        joy = joy | Joy::LEFT;
    }
    if f % 37 < 4 {
        joy = joy | Joy::UP;
    }
    if f % 53 < 2 {
        joy = joy | Joy::FIRE;
    }
    FrameInput::joy(joy)
}

/// Everything a frame gives that a player or conformance sees: the frame, its Paula writes and the
/// probe after it.
fn run(g: &mut Game, from: u32, frames: u32) -> Vec<(sf_core::FrameOut, Vec<(sf_core::VarId, u32)>)> {
    (from..from + frames)
        .map(|f| {
            let out = g.step(input(f));
            (out, g.probe())
        })
        .collect()
}

/// `g` run `n` frames, saved, run `m` more; then loaded from the state and run the same `m`:
/// the two runs are the same frame for frame, and the loaded game equals the saved one.
fn check(name: &str, mut g: Game, n: u32, m: u32) {
    run(&mut g, 0, n);
    let state = g.save(BUILD, hash());
    let mut loaded = Game::load(data(), &state, BUILD, hash()).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(loaded == g, "{name}: the loaded game differs from the saved one");
    let scene = g.scene();
    let a = run(&mut g, n, m);
    let b = run(&mut loaded, n, m);
    for (k, (x, y)) in a.iter().zip(&b).enumerate() {
        assert!(x == y, "{name} ({scene:?}): frame {} after the load differs", k);
    }
    assert_eq!(a.len(), m as usize);
}

fn level(id: &str) -> Start {
    Start::Level(id.parse::<LevelId>().unwrap())
}

#[test]
fn a_loaded_story_intro_runs_on_as_the_saved_one() {
    let g = Game::new(data(), Start::Disk3 { seed: Rng::default() });
    check("story", g, 300, 120);
}

#[test]
fn a_loaded_front_end_runs_on_as_the_saved_one() {
    let g = Game::new(data(), Start::PowerOn { seed: Rng::default() });
    check("front, early", g.clone(), 50, 100);
    check("front, title", g, 700, 150);
}

#[test]
fn a_loaded_level_runs_on_as_the_saved_one() {
    check("W1-1 start", Game::new(data(), level("W1-1")), 1, 200);
    check("W1-1 mid", Game::new(data(), level("W1-1")), 400, 200);
    check("W4-2 mid", Game::new(data(), level("W4-2")), 300, 200);
    check("W6-3 mid", Game::new(data(), level("W6-3")), 300, 200);
}

#[test]
fn a_loaded_bonus_level_runs_on_as_the_saved_one() {
    let g = Game::new(data(), level("WB-1"));
    assert!(matches!(g.scene(), Scene::Bonus(_)));
    check("bonus", g, 200, 200);
}

#[test]
fn a_loaded_slot_machine_runs_on_as_the_saved_one() {
    let mut g = Game::new(data(), level("W1-1"));
    run(&mut g, 0, 50);
    g.enter_slot(40);
    assert!(matches!(g.scene(), Scene::Slot { .. }));
    check("slot", g, 150, 200);
}

#[test]
fn a_state_from_another_build_or_cache_is_refused() {
    let mut g = Game::new(data(), level("W1-1"));
    run(&mut g, 0, 20);
    let state = g.save("superfrog other-build", hash());
    assert_eq!(
        Game::load(data(), &state, BUILD, hash()),
        Err(StateError::OtherBuild { theirs: "superfrog other-build".into() })
    );
    let state = g.save(BUILD, &[7; 32]);
    assert_eq!(Game::load(data(), &state, BUILD, hash()), Err(StateError::OtherCache));
}

#[test]
fn a_damaged_or_foreign_file_is_refused() {
    let mut g = Game::new(data(), level("W1-1"));
    run(&mut g, 0, 20);
    let state = g.save(BUILD, hash());
    let load = |b: &[u8]| Game::load(data(), b, BUILD, hash()).map(|_| ());
    // Cut short anywhere: in the magic, the header, the hash or the body.
    for cut in [2, 6, 30, 60, 80, state.len() / 2, state.len() - 1] {
        assert_eq!(load(&state[..cut]), Err(StateError::Corrupt), "cut at {cut}");
    }
    // One byte of the body changed.
    let mut bad = state.clone();
    let last = bad.len() - 1;
    bad[last] ^= 1;
    assert_eq!(load(&bad), Err(StateError::Corrupt));
    // A byte too many.
    let mut long = state.clone();
    long.push(0);
    assert_eq!(load(&long), Err(StateError::Corrupt));
    // Not a state: empty, random bytes, another file.
    assert_eq!(load(&[]), Err(StateError::NotAState));
    let mut x = 0x1234_5678u32;
    let noise: Vec<u8> = (0..4096)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect();
    assert_eq!(load(&noise), Err(StateError::NotAState));
    assert_eq!(load("[settings]\nscale = 2\n".as_bytes()), Err(StateError::NotAState));
}

#[test]
fn the_encoding_is_deterministic() {
    let mut g = Game::new(data(), level("W2-1"));
    run(&mut g, 0, 100);
    let a = g.save(BUILD, hash());
    assert_eq!(a, g.clone().save(BUILD, hash()));
    let loaded = Game::load(data(), &a, BUILD, hash()).unwrap();
    assert_eq!(loaded.save(BUILD, hash()), a, "a loaded game saves to the same bytes");
    let h = Game::state_header(&a).unwrap();
    assert_eq!((h.magic, h.engine_build.as_str(), h.frame), (*b"SFST", BUILD, 100));
}
