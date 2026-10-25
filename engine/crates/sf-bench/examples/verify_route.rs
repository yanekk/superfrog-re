//! Set C's check on one app-timed route file (experiment/ghost-guide): replayed on the engine
//! from Game ▸ Start Level… with power-on seed `N N`, then played in the real app (a window, 1×,
//! real time) with every frame's state hash compared, as `sf-bench --set C` does.
//!
//! `cargo run --release -p sf-bench --example verify_route -- <level> <route.txt> <seed> [--engine]`
use sf_bench::verify::{AppCheck, verify_app};
use sf_conform::InputScript;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let level: sf_core::LevelId = a[0].parse().unwrap();
    let script = InputScript::load(std::path::Path::new(&a[1])).unwrap();
    let seed: u32 = a[2].parse().unwrap();
    let check = if a.iter().any(|x| x == "--engine") { AppCheck::Engine } else { AppCheck::RealApp };
    let v = verify_app(level, seed, &script, check);
    println!(
        "VERIFY {level} completed={} route_frames={:?} hearts_lost={} reason={:?}",
        v.completed, v.route_frames, v.hearts_lost, v.reason
    );
}
