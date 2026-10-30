//! T213's probe: both sides' channel levels (`sf_conform::channels::levels`) for frames of play
//! `from..=to` of a case. `channel_window <case.toml> <from> <to>`
use sf_conform::{Case, channels};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).expect("case");
    let (from, to): (usize, usize) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let (feed, theirs, ours) = channels::levels(&case).unwrap_or_else(|e| panic!("{e}"));
    for (i, f) in feed.frames.iter().enumerate().filter(|(_, f)| f.twice || f.load.is_some() || !f.then.is_empty()) {
        println!("{} twice {} load {:?} then {}", i + 1, f.twice, f.load, f.then.len());
    }
    for i in from - 1..to.min(ours.len()) {
        let f = &feed.frames[i];
        println!("{} {:?}{}{} original {:?} engine {:?}", i + 1, f.pacing, if f.twice { " twice" } else { "" }, if f.load.is_some() { " load" } else { "" }, theirs[i], ours[i]);
    }
}
