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
