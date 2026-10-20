//! T213's probe: every run of more than `min` frames of play on which a channel differs from the
//! original's under the check's tolerance, `frames` set to `min`; a load within 12 frames before
//! the run is marked. `channel_runs <case.toml> <min>`
use sf_conform::{Case, channels};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).expect("case");
    let min: u32 = a[1].parse().unwrap();
    let (feed, theirs, ours) = channels::levels(&case).unwrap_or_else(|e| panic!("{e}"));
    let v = channels::judge(&theirs, &ours, channels::Tolerance { frames: min, ..channels::OWNER });
    println!("compared {} shifted {} furthest {} differing {:?}", v.compared, v.shifted, v.furthest, v.differing);
    for r in &v.runs {
        let first = r.first as usize - 1;
        let load = feed.frames[first.saturating_sub(12)..=first].iter().any(|f| f.load.is_some());
        println!("{r}{}", if load { " (a load just before)" } else { "" });
    }
}
