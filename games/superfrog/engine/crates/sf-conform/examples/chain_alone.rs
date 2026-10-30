//! Scratch (T183): a case's engine alone (normal play, the script's inputs), its screens' and
//! walk's state each time they change. `chain_alone <case.toml> [frames]`
use sf_core::layout::h0;
use sf_core::{Game, Start};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = sf_conform::Case::load(std::path::Path::new(&a[0])).unwrap();
    let n: u32 = a.get(1).map_or(case.frames, |s| s.parse().unwrap());
    let synced = sf_conform::sync::reach(&case.sync, &case.script, case.frames).unwrap();
    let mut g = Game::with_display(sf_conform::display_data(), Start::Transplant(synced.memory));
    let mut last = String::new();
    for f in 0..n {
        g.step(case.script.input_at(f));
        let s = g.state();
        let now = format!(
            "cop {:x?} walk {:x} timer {:x} exit {} un {:?}",
            g.cop1lc(),
            s.l(h0(0x11e92)).wrapping_sub(h0(0)),
            s.w(h0(0x123d4)),
            s.w(h0(0x9f8)),
            g.unported()
        );
        let key = now.replace(&format!("timer {:x} ", s.w(h0(0x123d4))), "");
        if key != last {
            println!("{f}: {now}");
            last = key;
        }
    }
}
