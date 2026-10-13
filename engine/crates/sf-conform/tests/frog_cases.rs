//! The test block's share of the frog cases (T14), run against the original. All
//! of them, W1-1's 1 500-frame run and each world's first level among them, are in
//! `engine/conform/cases/frog-*.toml`, run by `sf-conform -- all`.

fn green(name: &str) {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{name}: {d}");
    }
}

/// The frog standing in W1-1 for 200 frames.
#[test]
fn frog_idle_w1_1() {
    green("frog-idle-w1-1");
}

/// Into W1-1's first pit, and the death.
#[test]
fn frog_pit_w1_1() {
    green("frog-pit-w1-1");
}

/// Against the tunnel's pillar, with head bumps.
#[test]
fn frog_wall_w1_1() {
    green("frog-wall-w1-1");
}

/// W1-1 run right for 1 500 frames: both gaps jumped, the hills, the hazards jumped, and the fake
/// wall at W1-1:94,51 opened into the hidden cave.
#[test]
fn frog_run_w1_1() {
    green("frog-run-w1-1");
}

/// W1-1 with its items live, 700 frames: coins, the exit lock, the panel and score (T15), the
/// picture as the beam drew it (T36), with no re-sync after a lag (T82).
#[test]
fn items_run_w1_1() {
    green("items-run-w1-1");
}

/// W2-2 with its items live, 650 frames: five kinds of fruit and the spud taken, the spud thrown
/// and caught, with no re-sync after a lag (T82).
#[test]
fn items_fruit_w2_2() {
    green("items-fruit-w2-2");
}
