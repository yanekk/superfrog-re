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

/// W6-3's first map completed (T149): every coin and switch 49 to the exit, green faithful and
/// alone (254 milestones), re-synced after the lags at 5375 and 5979, as DESIGN §2.1 lets checks
/// 1–3 do.
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
