//! Experiment: judge one set-A route file the way the benchmark does (`verify::verify`).
//! `cargo run --release -p sf-bench --example verify_route -- <level> <route.txt>`
use sf_bench::sets::{Set, Start, StartKind};
use sf_bench::verify::{ref_frames, verify};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let s = Start { level: a[0].parse().unwrap(), kind: StartKind::AtLevel };
    let script = sf_conform::InputScript::load(std::path::Path::new(&a[1])).unwrap();
    let v = verify(Set::A, &s, &script);
    println!("{} completed {} route_frames {:?} committed {:?} hearts_lost {} {}",
        a[0], v.completed, v.route_frames, ref_frames(Set::A, &s), v.hearts_lost, v.reason);
}
