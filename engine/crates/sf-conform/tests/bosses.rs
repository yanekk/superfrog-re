//! The final level and its witch (T18) against the original, faithful and alone (DESIGN §2.1):
//! `engine/conform/cases/final-*.toml`, all run by `sf-conform -- all`. The test block runs the
//! win as far as the level's end; the whole cases (2 500 to 3 000 frames each) are ignored as
//! slow.

fn case(name: &str) -> sf_conform::Case {
    sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"))
}

fn green(case: &sf_conform::Case) {
    if let Err(d) = sf_conform::run(case) {
        panic!("{}: {d}", case.name);
    }
}

/// The witch beaten (1267): the white flash stepping back, `level_state` $64 counting down and
/// the level's loop ending (1469), into `top_loop`'s exit fade.
#[test]
fn final_win_to_the_levels_end() {
    let mut c = case("final-win");
    c.frames = 1500;
    green(&c);
}

/// The final level won, into the end sequence.
#[test]
#[ignore = "slow: 2 600 frames"]
fn final_win() {
    green(&case("final-win"));
}

/// The frog dies to the witch and the final level starts again.
#[test]
#[ignore = "slow: 2 500 frames"]
fn final_lose() {
    green(&case("final-lose"));
}

/// Every pattern of hers, nothing held, for 3 000 frames.
#[test]
#[ignore = "slow: 3 000 frames"]
fn final_idle() {
    green(&case("final-idle"));
}
