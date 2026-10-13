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

/// Into W1-1's first pit, the death, the level's exit fade and its restart (T37).
#[test]
fn frog_pit_w1_1() {
    green("frog-pit-w1-1");
}

/// The clock run out in W1-1 (set to 0:03): green to the frame before the level's loop ends, and
/// on that frame the engine is at game over (`game_over_seq`, T34's) after the time ran out.
#[test]
fn frog_timeup_w1_1() {
    green("frog-timeup-w1-1");
    let mut case =
        sf_conform::Case::load(&sf_conform::cases_dir().join("frog-timeup-w1-1.toml")).unwrap();
    case.frames += 1;
    let d = sf_conform::run(&case).expect_err("game over is T34's");
    assert_eq!(d.frame, 269);
    assert_eq!(d.unported, ["game over, the time up (T34)"]);
}

/// ESC in W1-1 (T37 review): green through the exit fade to the frame before the ESC path's
/// music fade, which is game over's (T34).
#[test]
fn frog_esc_w1_1() {
    green("frog-esc-w1-1");
    let mut case =
        sf_conform::Case::load(&sf_conform::cases_dir().join("frog-esc-w1-1.toml")).unwrap();
    case.frames += 1;
    let d = sf_conform::run(&case).expect_err("the ESC path is T34's");
    assert_eq!(d.frame, 77);
    assert_eq!(d.unported, ["game over (T34)"]);
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

/// W1-1's exit forced on its first frame (T38): the exit fade, the tally with its two pages for
/// the time left, fire with up never held taking the points, the hundred frames' pause and the
/// fade out; green to the frame before the tally returns into `top_loop`, and on that frame the
/// level is over with the pages and the points taken (the next level is T21's).
#[test]
fn tally_w1_1() {
    green("tally-w1-1");
    let mut case = sf_conform::Case::load(&sf_conform::cases_dir().join("tally-w1-1.toml")).unwrap();
    case.frames += 1;
    let d = sf_conform::run(&case).expect_err("the next level is T21's");
    assert_eq!(d.frame, 185);
    assert_eq!(d.unported, ["the next level after the tally (T21)"]);
    assert_eq!(
        d.level_outcome,
        Some(sf_core::level::LevelOutcome::Completed {
            tally: sf_core::level::Tally {
                pages: 2,
                slot_machine: false
            }
        })
    );
}

/// The tally's other choice (T38): up, then fire, the picture faded out and the slot machine
/// entered with the pages as its coins; green to the slot machine's loads (T92, T95).
#[test]
fn tally_slot_w1_1() {
    green("tally-slot-w1-1");
}
