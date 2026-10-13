//! The test block's share of the game-over cases (T34): game over after the last life in W1-1,
//! from the frame `game_over_seq` begins, untouched to the title. The others (fire pressed in it,
//! ESC, the time up) are in `engine/conform/cases/game-over-*.toml`, run by `sf-conform -- all`.

#[test]
fn game_over_w1_1() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("game-over-w1-1.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("game-over-w1-1: {d}");
    }
}
