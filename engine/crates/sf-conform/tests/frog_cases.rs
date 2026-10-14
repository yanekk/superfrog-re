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

/// The clock run out in W1-1 (set to 0:03), to the frame before the level's loop ends into game
/// over (`game-over-timeup-w1-1` goes on to the title, T34).
#[test]
fn frog_timeup_w1_1() {
    green("frog-timeup-w1-1");
}

/// ESC in W1-1 (T37 review): green through the exit fade (`game-over-esc-w1-1` goes on to the
/// title, T34).
#[test]
fn frog_esc_w1_1() {
    green("frog-esc-w1-1");
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
/// picture as the beam drew it (T36). World 1's enemies are live too (T42): a pass across four
/// frames ends at frame 321 on the wait's last check, the next waiting for the next frame (T101).
#[test]
fn items_run_w1_1() {
    green("items-run-w1-1");
}

/// W1-1 with its items live, 860 frames: the fake wall, the bounce pad at 16,67 and the coins up
/// the shaft (T83). The pad bounce lags every frame: passes across three frames, and an effect
/// started in `sfx_play`'s raster wait as the frame ends, which holds its vertical blank off
/// until interrupts are back on (T101).
#[test]
fn items_pad_w1_1() {
    green("items-pad-w1-1");
}

/// W2-2 with its items and (T43) its enemies live, 650 frames: five kinds of fruit and the spud
/// taken, the spud thrown and caught. No re-sync: its one at 311 was the bob drawer's wait for
/// the vertical blank's flag (T102).
#[test]
fn items_fruit_w2_2() {
    pinned_bob_draw("items-fruit-w2-2", &[]);
}

/// A case green but for re-syncs at `frames`, each after the original's pass was cut inside the
/// bob drawer and differing first in a play-field buffer (T121's, FINDINGS "T43: pinned
/// re-syncs").
fn pinned_bob_draw(name: &str, frames: &[u32]) {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(case.resync, "{name}: a pinned case re-syncs");
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, frames, "{name}: {:?}", o.resynced);
    assert!(
        o.resynced.iter().all(|r| r.1.starts_with("playfield_buffer_")),
        "{name}: {:?}",
        o.resynced
    );
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

/// W1-2 with its items live, 650 frames: the wings and their flight, the spud taken and thrown,
/// with no re-sync (T83).
#[test]
fn items_wings_w1_2() {
    green("items-wings-w1-2");
}

/// W1-1 with its items live, 800 frames: the checkpoint and the speed-up until it ends, with no
/// re-sync (T83).
#[test]
fn items_checkpoint_w1_1() {
    green("items-checkpoint-w1-1");
}

/// W2-1 with its items live, 760 frames: both hidden coin springs knocked empty, the switch at
/// 116,39 opening its gate, and the wings, with no re-sync (T85).
#[test]
fn items_switch_w2_1() {
    green("items-switch-w2-1");
}

/// W2-1 with its items live, 1 010 frames: the coin springs, the lift into the pillar and the
/// speed-up, with no re-sync (T85; the fruit and the extra life are T105's).
#[test]
fn items_upper_w2_1() {
    green("items-upper-w2-1");
}

/// W2-2 with its items live, 720 frames: the launchers at 176,6-7 and 182,8-9 throwing the frog
/// right, left and right again, the throw's frame showing the frog's pose before it (T86).
#[test]
fn items_launcher_w2_2() {
    green("items-launcher-w2-2");
}

/// W4-1 with its items live, 2 360 frames: enemies-w4-1's route on to the 50 000-point fruit at
/// 290,34 (T86), with no re-sync (T102).
#[test]
fn items_fruit_w4_1() {
    green("items-fruit-w4-1");
}

/// W3-1 with its items live, 1 130 frames: the bottle, both pads (the second over the spiked
/// pillar), the slow-down, the speed-up, the wings and the shield, then idle until the speed-up
/// and the shield run out, with no re-sync (T102): long lags in the pad bounce, the frog's
/// collision walk cut by the vertical blank and the bob drawer waiting for its flag.
#[test]
fn items_shield_w3_1() {
    green("items-shield-w3-1");
}

/// The bonus level entered (T79): 600 frames from its first, the frog flown about and firing,
/// with no re-sync.
#[test]
fn bonus_enter_wb_1() {
    green("bonus-enter-wb-1");
}

/// The bonus level failed (T79): two deaths with a life left, each putting the frog back on the
/// screen without leaving the loop (no outcome reaches the game flow), then the last life lost
/// and the loop's end in frame 2448: game over. Green to frame 2447; frame 2448, the first of
/// `game_over_seq`, diverges after a lag (FINDINGS, T79 review), pinned here with its outcome.
#[test]
fn bonus_death_wb_1() {
    let mut case =
        sf_conform::Case::load(&sf_conform::cases_dir().join("bonus-death-wb-1.toml")).unwrap();
    let mut first = None;
    if let Err(d) = sf_conform::run_with(&case, |f, g| {
        if first.is_none() {
            first = g.level_outcome().map(|o| (f, o));
        }
    }) {
        panic!("bonus-death-wb-1: {d}");
    }
    // Neither earlier death ends the loop.
    assert_eq!(first, None);
    case.frames += 1;
    let d = sf_conform::run(&case).expect_err("game over in the bonus level, open");
    assert_eq!(d.frame, 2448);
    assert_eq!(d.level_outcome, Some(sf_core::level::LevelOutcome::GameOver));
}

/// The bonus level completed (T79): 16 deaths on the way, marker 55, the loop's end, the exit
/// fade; the engine's outcome is the exit, with no re-sync (frames 4283 and 7813 end inside
/// phase 6, before `buffer_flip`'s writes: T101). Frame 8127, the first after the fade, is the end picture's, every variable equal: the last
/// fade frame's `shooter_off` (h0+$7494) is seen only there.
#[test]
fn bonus_end_wb_1() {
    let mut case =
        sf_conform::Case::load(&sf_conform::cases_dir().join("bonus-end-wb-1.toml")).unwrap();
    let mut last = None;
    sf_conform::run_with(&case, |_, g| last = g.level_outcome())
        .unwrap_or_else(|d| panic!("bonus-end-wb-1: {d}"));
    assert!(
        matches!(last, Some(sf_core::level::LevelOutcome::Completed { .. })),
        "{last:?}"
    );
    case.frames += 1;
    let d = sf_conform::run(&case).expect_err("the end picture is T21's");
    assert_eq!(d.frame, 8127);
    assert_eq!(d.unported, ["the bonus level's end picture (T21)"]);
    assert!(d.first_var.is_none(), "{d}");
}

/// W4-4 with its enemies live (T41): swoopers, scenery and a ledge freed as the camera leaves
/// them and spawned again as it comes back, on the original's frames.
#[test]
fn enemies_w4_4_activation() {
    green("enemies-w4-4-activation");
}
