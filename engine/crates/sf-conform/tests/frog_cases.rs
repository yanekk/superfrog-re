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

/// The original's late frames are counted, its slowdown measured (DESIGN §2.1, T181): in
/// `frog-run-w1-1` its pass is late at the vertical blank twice, the two lags (559, 561); a
/// title-side case never is.
#[test]
fn the_originals_late_frames_are_counted() {
    let run = |name: &str| {
        let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
            .unwrap_or_else(|e| panic!("{e}"));
        sf_conform::run(&case).unwrap_or_else(|d| panic!("{name}: {d}"))
    };
    let o = run("frog-run-w1-1");
    assert_eq!((o.late, o.straddled.as_slice()), (2, &[559, 561][..]));
    assert_eq!(run("title-rng").late, 0);
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
/// taken, the spud thrown and caught. No re-sync (`resync = false`, T121): its one at 311 was
/// the bob drawer's wait for the vertical blank's flag (T102).
#[test]
fn items_fruit_w2_2() {
    green("items-fruit-w2-2");
}

/// W1-1's exit forced on its first frame (T38): the exit fade, the tally with its two pages for
/// the time left, fire with up never held taking the points, the hundred frames' pause and the
/// fade out; green, and on the frame after, the tally back in `top_loop` (W1-2's load, T183), the
/// level is over with the pages and the points taken.
#[test]
fn tally_w1_1() {
    green("tally-w1-1");
    let mut case = sf_conform::Case::load(&sf_conform::cases_dir().join("tally-w1-1.toml")).unwrap();
    // Two frames on: the hook runs before each step, so it sees frame 185's end.
    case.frames += 2;
    let mut outcome = None;
    if let Err(d) = sf_conform::run_with(&case, |_, g| outcome = g.level_outcome()) {
        panic!("{d}");
    }
    assert_eq!(
        outcome,
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
/// and the loop's end in frame 2448, after a lag: game over. Green to frame 2448, the first of
/// `game_over_seq`, since a loop ending in a late pass ends in the next step (T106); frame 2449
/// diverges (FINDINGS, T79 review), pinned here with its outcome.
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
    assert_eq!(d.frame, 2449);
    assert_eq!(d.level_outcome, Some(sf_core::level::LevelOutcome::GameOver));
}

/// The bonus level completed (T79): 16 deaths on the way, marker 55, the loop's end, the exit
/// fade; the engine's outcome is the exit, with no re-sync (frames 4283 and 7813 end inside
/// phase 6, before `buffer_flip`'s writes: T101). Frame 8127, the first after the fade, is the end
/// picture's: the last fade frame's `shooter_off` (h0+$7494) is seen only there, and the first
/// variable apart is `end_pic_shown`, the original's picture up (T183 names it).
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
    assert!(
        d.first_var.as_ref().is_some_and(|v| v.0.name == "end_pic_shown"),
        "{d}"
    );
}

/// W4-4 with its enemies live (T41): swoopers, scenery and a ledge freed as the camera leaves
/// them and spawned again as it comes back, on the original's frames.
#[test]
fn enemies_w4_4_activation() {
    green("enemies-w4-4-activation");
}

/// W4-3 played from its start to its exit and through the tally (T103): the switch opening the
/// start area, a marker-22 floater and an upward crusher hurting the frog, the 99 coins, the exit
/// at 8485 and fire taking the points; green, and on the frames after, the tally back in
/// `top_loop` (W4-4's load, T183), the level completed with the points taken.
///
/// Frame 4542 ends while the copper interrupt still blits: its DMACON `$0400` and INTREQ `$10`
/// are 4543's first writes, and 4543's vertical blank is taken straight after, before the main
/// program's next blit sets BLTPRI (DMACON `$8400`, line 252 of 4543 on the original). With the
/// interrupt's tail in 4542 and that `$8400` before the blank, 4543's 40th write was `sfx_play`'s
/// and channel 0's effect stepped a ramp step on (red at 4546, T197). Both lag frames, which the
/// comparison leaves out: their DMACON and INTREQ writes are pinned here.
#[test]
fn complete_w4_3() {
    let name = "complete-w4-3";
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let dma = |w: &[sf_paula::PaulaWrite]| -> Vec<(u16, u16)> {
        w.iter().filter(|w| matches!(w.reg, 0x96 | 0x9c)).map(|w| (w.reg, w.value)).collect()
    };
    let mut seen = Vec::new();
    if let Err(d) = sf_conform::run_observing_lag(&case, |f, ours, theirs| {
        if matches!(f, 4542 | 4543) {
            seen.push((f, ours.len(), dma(ours), theirs.len(), dma(theirs)));
        }
    }) {
        panic!("{name}: {d}");
    }
    let [(4542, n2, ours2, t2, theirs2), (4543, n3, ours3, t3, theirs3)] = &seen[..] else {
        panic!("{name}: frames seen {seen:?}");
    };
    assert_eq!((n2, ours2), (t2, theirs2));
    assert_eq!((n3, ours3), (t3, theirs3));
    assert_eq!((*t2, *t3), (20, 40));
    assert_eq!(theirs2.last(), Some(&(0x96, 0x8400)));
    assert_eq!(theirs3[..2], [(0x96, 0x0400), (0x9c, 0x0010)]);
    // After the copper interrupt's acknowledge, the main program's BLTPRI and its end.
    let ack = theirs3.iter().rposition(|w| *w == (0x9c, 0x0010)).expect("the acknowledge");
    assert_eq!(theirs3[ack + 1..], [(0x96, 0x8400), (0x96, 0x0400), (0x96, 0x8400)]);
    goes_on(name);
}

/// `name` run two frames past its end (the hook runs before each step, so it sees its last
/// frame's end): green, nothing unported, the level completed with the points taken and no slot
/// machine, `top_loop` going on to what follows (T183).
fn goes_on(name: &str) {
    let mut case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    case.frames += 2;
    let (mut outcome, mut unported) = (None, Vec::new());
    if let Err(d) = sf_conform::run_with(&case, |_, g| {
        outcome = g.level_outcome();
        unported = g.unported().to_vec();
    }) {
        panic!("{name}: {d}");
    }
    assert_eq!(unported, Vec::<&str>::new(), "{name}");
    assert!(
        matches!(
            outcome,
            Some(sf_core::level::LevelOutcome::Completed { tally }) if !tally.slot_machine
        ),
        "{name}: {outcome:?}"
    );
}

/// `name` run to its end with re-syncs at exactly `pins`, then on (T106): the level completed with
/// the points taken and no slot machine, and `top_loop` going on to the next level ([`goes_on`],
/// T183). Returns the run's outcome.
fn completes(name: &str, pins: &[u32]) -> sf_conform::Outcome {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, pins, "{name}: {:?}", o.resynced);
    goes_on(name);
    o
}

/// W1-2 played from its start to its exit and through the tally (T106): the 68 coins, the exit at
/// 5182 and fire taking the points, with no re-sync.
#[test]
fn complete_w1_2() {
    completes("complete-w1-2", &[]);
}

/// W1-3 played from its start to its exit and through the tally (T106): the 75 coins, the spring
/// at 105,75 out of the basin, the exit at 7728 and fire taking the points. One re-sync, at 5079:
/// a lag cut inside `panel_update` leaves the panel's digits differing (as W5-1's at 979, pinned
/// in `enemies_w5_cases`).
#[test]
fn complete_w1_3() {
    completes("complete-w1-3", &[5079]);
}

/// W1-4 played from its start to its exit and through the tally (T154): T106's route to 1800, the
/// lower left's coins, the hill's pad at 28,30 with the wings' flaps across to the block at
/// 100,19, its coins and the cave's, down the shaft at 6-12,54 and the right half's coins; the exit
/// at 7377 with the 96 coins taken, fire taking the points. No re-sync.
#[test]
fn complete_w1_4() {
    let met = completes("complete-w1-4", &[]).milestones.expect("check 4 ran");
    let exits: Vec<u64> = met
        .iter()
        .filter(|(_, m)| *m == sf_conform::Milestone::LevelExit { world: 1, level: 4 })
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(exits, [7377]);
}

/// W4-4 played from its start to its exit and through the tally (T132, T145): the pacer's hurt at
/// 2515, marker 20's dart's at 4717, a crusher's at 22458, marker 21's dart's at 23577, the exit
/// at 24200 and fire taking the points. Three re-syncs, each after a lag: the panel at 8030 and
/// 23129 (as W1-3's 5079) and the hitboxes at 15914; the play field buffer's at 9003 was T184's.
/// Frame 5252's picture (the frog thrown by a pad) and 24201's (the exit fade's palette step) are
/// within the faithful picture tolerance (T181).
#[test]
fn complete_w4_4() {
    completes("complete-w4-4", &[8030, 15914, 23129]);
}

/// W1-1 played to its exit (T106): green through the exit at 4473, which `level_state` 1 reaches
/// in a late pass (`sync::mid_pass`), to frame 4476. Frame 4475 shows the exit-fade palette one
/// step off (T131's): faithful tolerates and counts it (T180); `--strict` diverges there.
#[test]
fn complete_w1_1_to_exit_fade() {
    let mut case =
        sf_conform::Case::load(&sf_conform::cases_dir().join("complete-w1-1.toml")).unwrap();
    case.frames = 4476;
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("complete-w1-1: {d}"));
    assert_eq!(o.picture_colour_step, [4475], "{o:?}");
    case.mode = sf_conform::Mode::Strict;
    let d = sf_conform::run(&case).expect_err("the exit fade's palette step, frame-exact");
    assert_eq!(d.frame, 4475);
    assert!(d.first_var.is_none() && d.picture_diff.is_some(), "{d}");
}

/// W1-1 played from its start to its exit and through the tally (T106, T181): the 90 coins, the
/// exit at 4473 after a late pass, fire taking the points; the exit fade's two frames a step off
/// (4475, 4477) within the faithful picture tolerance. No re-sync.
#[test]
fn complete_w1_1() {
    completes("complete-w1-1", &[]);
}

/// W4-1 played from its start to its exit and through the tally (T103, T181): the 41 coins the
/// exit needs, the exit at 2145, fire taking the points; the exit fade's palette step a frame
/// early at 2149 within the faithful picture tolerance. No re-sync.
#[test]
fn complete_w4_1() {
    completes("complete-w4-1", &[]);
}

/// A map cell's marker in the engine's memory (`map_rows[1]` h1+$218e0, rows `$260` bytes).
fn marker(g: &sf_core::Game, col: u32, row: u32) -> u16 {
    let at = sf_core::layout::h1(0x218e0 + 0x4c0) + row * 0x260 + 2 * col;
    g.state().w(at) & 0x3f
}

/// A green run of `name`, seen through `run_with` (T105): for each item cell in `cells`, the frame
/// that took it (its marker changed; the engine before the next frame), with the lives and score
/// it took them from and to, and the frames the original ended mid-pass.
fn takes(name: &str, cells: &[(u32, u32)]) -> (Vec<(u32, (u16, u16), (u32, u32))>, Vec<u32>) {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut seen: Vec<(u16, u32, Vec<u16>)> = Vec::new();
    let outcome = sf_conform::run_with(&case, |_, g| {
        let st = g.state();
        let cells = cells.iter().map(|&(c, r)| marker(g, c, r)).collect();
        seen.push((st.w(sf_core::frog::FROG + sf_core::frog::f::LIVES), st.l(sf_core::items::SCORE), cells));
    })
    .unwrap_or_else(|d| panic!("{name}: {d}"));
    assert!(outcome.resynced.is_empty(), "{name}: {:?}", outcome.resynced);
    let found = (0..cells.len())
        .map(|k| {
            let f = (1..seen.len())
                .find(|&f| seen[f].2[k] != seen[0].2[k])
                .unwrap_or_else(|| panic!("{name}: {:?} not taken", cells[k]));
            // `seen[f]` is the engine before frame `f`: the take was frame `f - 1`.
            let (a, b) = (&seen[f - 1], &seen[f]);
            ((f - 1) as u32, (a.0, b.0), (a.1, b.1))
        })
        .collect();
    (found, outcome.straddled)
}

/// The first frame from `frame` on whose variables the run compared: `frame` itself, or the one
/// after the original's lag ends (`run` compares it with no re-sync allowed).
fn compared_from(frame: u32, straddled: &[u32]) -> u32 {
    (frame..).find(|f| !straddled.contains(f)).unwrap()
}

/// W2-1 with its items live, 940 frames: items-upper-w2-1's route to the pillar, then the lift up
/// its column 63 onto the extra life at 63,35 (T105). The take is frame 895, which the original
/// lags into 896 on every route found: its lives are compared on 897 (owner, 2026-10-14).
#[test]
fn items_life_w2_1() {
    let (found, straddled) = takes("items-life-w2-1", &[(63, 35)]);
    let (frame, lives, _) = found[0];
    assert_eq!(lives.1, lives.0 + 1, "the extra life at frame {frame}");
    assert_eq!(frame, 895);
    assert_eq!(compared_from(frame, &straddled), 897, "{straddled:?}");
}

/// W2-1 with its items live, 1 175 frames: the switch bumped, then past the exit while coins are
/// left onto the 20 000-point fruit at 260,25 (h0+$f008, T133). Its twin 48,22 is in a room play
/// never reaches (docs/07).
#[test]
fn items_fruit_w2_1() {
    let (found, straddled) = takes("items-fruit-w2-1", &[(260, 25)]);
    let (frame, _, score) = found[0];
    assert_eq!(score.1, score.0 + 20_000, "the fruit at frame {frame}");
    assert_eq!(frame, 1160);
    assert_eq!(compared_from(frame, &straddled), 1160, "{straddled:?}");
}
