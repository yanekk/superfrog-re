//! The test block's share of W6's enemy cases (T40), run against the original with every object
//! live on both sides and nothing left out. All of them are in
//! `engine/conform/cases/enemies-w6-*.toml`, run by `sf-conform -- all`; a case not here is held
//! out by an engine fault its case file names.

fn green(name: &str) {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{name}: {d}");
    }
}

/// W6-1: every type the map has met, the flier stomped dead, a hit, the first switch's gate.
#[test]
fn enemies_w6_1() {
    green("enemies-w6-1");
}

/// W6-2: walker, right dart, mace, cannon, the charger attacking, a hit.
#[test]
fn enemies_w6_2() {
    green("enemies-w6-2");
}

/// The same run on W6-2's second map.
#[test]
fn enemies_w6_2_alt() {
    green("enemies-w6-2-alt");
}

/// W6-3: seven of its eight types, the charger stomped and taken, a hit from the thrower.
#[test]
fn enemies_w6_3() {
    green("enemies-w6-3");
}

/// The same run on W6-3's second map.
#[test]
fn enemies_w6_3_alt() {
    green("enemies-w6-3-alt");
}

/// W6-4: every type the map has met, the strider among them, the flier killed, a hit, through
/// the lagging stretch 1052–1119 (T185).
#[test]
fn enemies_w6_4() {
    green("enemies-w6-4");
}

/// T185: W6-4's frame 1090 ends with the original's frog walk taking hitbox entry 11 but not yet
/// testing the frog against it (h0+$d14a); the vertical blank moves the frog down to $169, then
/// the walk stomps the object. The engine stomped before the blank, so its frog bounced from
/// $168, a frame early, and stayed 3 pixels high (its landing at 1120 a frame late).
#[test]
fn enemies_w6_4_stomps_after_the_blank_its_frame_ended_before() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("enemies-w6-4.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut case = case;
    case.frames = 1094;
    let mut seen = Vec::new();
    let _ = sf_conform::run_with(&case, |f, g| {
        if (1091..=1093).contains(&f) {
            let pieces = g.layout().iter().flat_map(|d| d.pieces());
            let y = pieces
                .zip(g.probe())
                .find(|(p, _)| p.name == "frog_y")
                .map(|(_, (_, v))| v);
            seen.push((f - 1, y));
        }
    });
    // The original's frog_y at the end of each frame (`sf-conform record`).
    assert_eq!(
        seen,
        [(1090, Some(0x168)), (1091, Some(0x169)), (1092, Some(0x168))]
    );
}

/// W6-4's second map: every type W6-4 has, the strider among them, the flier killed, a hit.
#[test]
fn enemies_w6_4_alt() {
    green("enemies-w6-4-alt");
}

/// W6-3's second map completed (T104, T181): all 91 coins and its switches to the exit, green
/// faithful and alone (193 milestones), re-synced after the lags at 6129 and 12379 (the panel's
/// clock mid-update), as DESIGN §2.1 lets checks 1–3 do.
#[test]
fn enemies_w6_3_alt_complete() {
    let name = "enemies-w6-3-alt-complete";
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, [6129, 12379], "{:?}", o.resynced);
}

/// W6-3's first map completed (T149): the 98 coins the exit needs and switch 49 to the exit
/// (40 of the map's 138 coins left, owner 2026-10-15), green faithful and alone (254 milestones),
/// re-synced after the lags at 5375 and 5979, as DESIGN §2.1 lets checks 1–3 do.
#[test]
fn enemies_w6_3_complete() {
    let name = "enemies-w6-3-complete";
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, [5375, 5979], "{:?}", o.resynced);
    // The run reaches the exit, the coins it needs all taken: a script cut short stays green
    // otherwise, both sides agreeing on a level not finished.
    let met = o.milestones.expect("check 4 ran");
    let exits: Vec<u64> = met
        .iter()
        .filter(|(_, m)| *m == sf_conform::Milestone::LevelExit { world: 6, level: 3 })
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(exits, [7891]);
    assert!(met.iter().any(|(_, m)| *m == sf_conform::Milestone::CoinTaken { left: 0 }));
}

/// W6-2's second map completed (T148): all 97 coins and both switches (50 on the ledge, then 48,
/// whose gate is the way round to the exit room) to the exit, green faithful and alone with no
/// re-sync. T147's W6-2 script is hit at 1461 here (the map's other enemies fill the object slots
/// otherwise), so from 1420 it was planned again, keeping every heart.
#[test]
fn enemies_w6_2_alt_complete() {
    let name = "enemies-w6-2-alt-complete";
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("{name}: {d}"));
    assert!(o.resynced.is_empty(), "{:?}", o.resynced);
    // The run reaches the exit, every coin taken: a script cut short stays green otherwise.
    let met = o.milestones.expect("check 4 ran");
    let exits: Vec<u64> = met
        .iter()
        .filter(|(_, m)| *m == sf_conform::Milestone::LevelExit { world: 6, level: 2 })
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(exits, [7336]);
    assert!(met.iter().any(|(_, m)| *m == sf_conform::Milestone::CoinTaken { left: 0 }));
}

/// W6-2 completed (T147, T194): all 97 coins and both switches to the exit, green faithful and
/// alone (334 milestones), re-synced after the lags at 1380, 1977, 6620 and 6665, as DESIGN §2.1
/// lets checks 1–3 do. Frame 6657 ends in `sfx_play`'s raster wait on channel 3; the vertical
/// blank it holds off is taken after the wait's next instruction zeroes the old effect's step
/// indices, so its `sfx_tick` moves the period index to 1 and the new effect starts there: its
/// period goes `$be` to `$80` at 6661, never `$98` (T194).
#[test]
fn enemies_w6_2_complete() {
    let name = "enemies-w6-2-complete";
    let mut case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    // One frame on, so the hook (run before each step) sees what the case's last frame did.
    case.frames += 1;
    let (mut exit, mut per_step) = (Vec::new(), None);
    let o = sf_conform::run_with(&case, |f, g| {
        // `level_state` as the frame before left it, while not playing (not negative).
        let state = u32::from_be_bytes(g.peek(sf_core::layout::h0(0xa06), 4).try_into().unwrap());
        if (state as i32) >= 0 && f > 8000 {
            exit.push((f - 1, state));
        }
        // Before frame 6659: channel 3's period step index (`sfx_chan3+$20`) as 6658 left it.
        if f == 6659 {
            per_step = Some(g.peek(sf_core::layout::h0(0x12ff0 + 0x20), 2));
        }
    })
    .unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, [1380, 1977, 6620, 6665], "{:?}", o.resynced);
    assert_eq!(per_step, Some(vec![0, 1]));
    // The run reaches the exit (`level_state` 1 on frame 8429, seen before 8430's step), every
    // coin taken: a script cut short stays green otherwise.
    assert_eq!(exit, [(8429, 1)]);
    let met = o.milestones.expect("check 4 ran");
    let exits: Vec<u64> = met
        .iter()
        .filter(|(_, m)| *m == sf_conform::Milestone::LevelExit { world: 6, level: 2 })
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(exits, [8429]);
    assert!(met.iter().any(|(_, m)| *m == sf_conform::Milestone::CoinTaken { left: 0 }));
}

/// W6-4's second map completed (T151): `enemies-w6-4-complete`'s first 4900 frames, the last 15
/// coins and switch 50 replanned, then the wings, the flight over the 24 spikes at row 10 and the
/// shaft to the exit; green faithful and alone (230 milestones), re-synced after the lag at 5688,
/// as DESIGN §2.1 lets checks 1–3 do.
#[test]
fn enemies_w6_4_alt_complete() {
    let name = "enemies-w6-4-alt-complete";
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let o = sf_conform::run(&case).unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, [5688], "{:?}", o.resynced);
    // The run reaches the exit, the coins it needs all taken.
    let met = o.milestones.expect("check 4 ran");
    let exits: Vec<u64> = met
        .iter()
        .filter(|(_, m)| *m == sf_conform::Milestone::LevelExit { world: 6, level: 4 })
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(exits, [7089]);
    assert!(met.iter().any(|(_, m)| *m == sf_conform::Milestone::CoinTaken { left: 0 }));
}

/// W6-4 completed (T150, T196): the 99 coins the exit counts and the three switches, the wings
/// over the spikes at row 10 and the shaft to the exit (`level_state` 1 on frame 7307), green
/// faithful and alone, re-synced after the lag at 5741 (`bob_records`), as DESIGN §2.1 lets
/// checks 1–3 do.
#[test]
fn enemies_w6_4_complete() {
    let name = "enemies-w6-4-complete";
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut exit_state = None;
    let o = sf_conform::run_with(&case, |f, g| {
        if f == 7308 {
            exit_state = Some(word(g, LEVEL_STATE + 2));
        }
    })
    .unwrap_or_else(|d| panic!("{name}: {d}"));
    let resynced: Vec<u32> = o.resynced.iter().map(|r| r.0).collect();
    assert_eq!(resynced, [5741], "{:?}", o.resynced);
    // The run reaches the exit, the coins it needs all taken: a script cut short stays green
    // otherwise, both sides agreeing on a level not finished.
    assert_eq!(exit_state, Some(1), "level_state at the end of frame 7307");
    let met = o.milestones.expect("check 4 ran");
    let exits: Vec<u64> = met
        .iter()
        .filter(|(_, m)| *m == sf_conform::Milestone::LevelExit { world: 6, level: 4 })
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(exits, [0]);
    assert!(met.iter().any(|(_, m)| *m == sf_conform::Milestone::CoinTaken { left: 0 }));
}

/// `level_state` (h0+$a06, a long), `coins_left_for_exit` (h0+$832), `vbl_count` (h0+$9d8).
const LEVEL_STATE: u32 = sf_core::layout::h0(0xa06);
const COINS_LEFT: u32 = sf_core::layout::h0(0x832);
const VBL_COUNT: u32 = sf_core::layout::h0(0x9d8);

fn word(g: &sf_core::Game, a: u32) -> u16 {
    let b = g.state().read(a, 2);
    (b[0] as u16) << 8 | b[1] as u16
}

/// `name` cut to `frames`, run alone (check 4), `read(game)` of the engine alone at the end of
/// each frame in `at`, in order.
fn alone_at<T>(
    name: &str,
    frames: u32,
    at: std::ops::RangeInclusive<u32>,
    read: impl Fn(&sf_core::Game) -> T,
) -> Vec<(u32, T)> {
    let mut case = sf_conform::Case::load(&sf_conform::cases_dir().join(format!("{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    case.frames = frames;
    case.alone = sf_conform::Alone::Only;
    let mut seen = Vec::new();
    let _ = sf_conform::run_with(&case, |f, g| {
        // Before frame `f` steps: the end of frame `f - 1`.
        if f > 0 && at.contains(&(f - 1)) {
            seen.push((f - 1, read(g)));
        }
    });
    seen
}

/// T196: W6-4's frame 153 ends with the original's frog walk past its reads of the frog's x and
/// y (h0+$d120), so after the blank it tests the frog as it was and misses the three coins the
/// moved frog would touch; it takes them a pass later, at 155. The engine counted the walk not
/// begun (step 1), took the box after the blank and the coins at 154.
#[test]
fn enemies_w6_4_takes_its_coins_with_the_box_from_before_the_blank() {
    let seen = alone_at("enemies-w6-4-complete", 157, 153..=155, |g| word(g, COINS_LEFT));
    // The original's coins_left_for_exit (`sf-conform record`).
    assert_eq!(seen, [(153, 99), (154, 99), (155, 96)]);
}

/// T196: W6-4's pass from 5228, held over the late frames 5228–5229, returns late in frame
/// 5230: the original ends that frame in `level_frame_loop`'s wait with the next pass due
/// (`vbl_count` 2, `Pacing::PassDue`) and begins it at 5231. The engine alone began it at once,
/// its objects a frame ahead from 5231 for the rest of the level.
#[test]
fn enemies_w6_4_alone_waits_out_a_pass_due_at_the_frames_end() {
    let seen = alone_at("enemies-w6-4-complete", 5233, 5229..=5231, |g| word(g, VBL_COUNT));
    // The original's vbl_count (`sf-conform record`).
    assert_eq!(seen, [(5229, 1), (5230, 2), (5231, 0)]);
}
