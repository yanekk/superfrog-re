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

/// W6-4's second map: every type W6-4 has, the strider among them, the flier killed, a hit.
/// (`enemies-w6-4`, the same run on the first map, waits for T102's pad beam race.)
#[test]
fn enemies_w6_4_alt() {
    green("enemies-w6-4-alt");
}
