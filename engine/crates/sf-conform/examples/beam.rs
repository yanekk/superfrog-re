//! Scratch (T36): the original's custom writes per frame with their beam lines.
use sf_conform::{Case, sync};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&args[0])).unwrap();
    let n: u32 = args.get(1).map_or(case.frames, |s| s.parse().unwrap());
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, n + 10, frozen).unwrap();
    let mut a = synced.amiga;
    let display = sf_conform::display_data();
    for f in 0..n {
        frozen.apply(&mut a, &display);
        let cap = a.step(case.script.input_at(f)).unwrap();
        let mem = sync::core_memory(a.memory());
        let p = sync::pacing(&a.cpu(), &mem, &cap.paula);
        let mid = sync::mid_pass(&a.cpu(), &mem);
        let ws: Vec<String> = cap.paula.iter().filter(|w| w.reg < 0xa0)
            .map(|w| format!("{:02x}={:04x}@{}.{}", w.reg, w.value, w.vpos, w.hpos)).collect();
        println!("{f} {:?}{} {}", p, if mid { " MID" } else { "" }, ws.join(" "));
    }
}
