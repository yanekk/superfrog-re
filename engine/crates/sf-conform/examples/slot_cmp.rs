//! Scratch (T74): words of the slot machine's memory on both sides, frame by frame, the engine run
//! alone from the case's sync point on the case's script (frame f the original's 758 + f).
//!
//! `cargo run --release --example slot_cmp -- <case.toml> <from> <to> <h0 offset[:L]>...`
use sf_conform::{Case, sync};
use sf_core::Ram;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let case = Case::load(std::path::Path::new(&args[1])).unwrap();
    let from: u32 = args[2].parse().unwrap();
    let to: u32 = args[3].parse().unwrap();
    let watch: Vec<(u32, bool)> = args[4..]
        .iter()
        .map(|a| {
            let (o, l) = a.split_once(':').map_or((a.as_str(), false), |(o, w)| (o, w == "L"));
            (sf_core::layout::h0(u32::from_str_radix(o, 16).unwrap()), l)
        })
        .collect();
    let mut first = None;
    let one = Case {
        frames: 1,
        ..case.clone()
    };
    let _ = sf_conform::run_with(&one, |f, g| {
        if f == 0 {
            first = Some(g.clone());
        }
    });
    let mut g = first.unwrap();
    let synced = sync::reach(&case.sync, &case.script, to + 900).unwrap();
    let mut a = synced.amiga;
    for of in 0..to + 758 {
        sync::step_inserting(&mut a, case.script.input_at(of)).unwrap();
        if of < 758 {
            continue;
        }
        let f = of - 758;
        g.step(case.script.input_at(of));
        if f < from {
            continue;
        }
        let m = sync::core_memory(a.memory());
        let s = g.state();
        let mut line = format!("E{f} O{of}");
        for &(at, l) in &watch {
            let theirs = (0..if l { 4 } else { 2 })
                .fold(0u32, |v, k| v << 8 | m.byte(at + k) as u32);
            let ours = if l { s.l(at) } else { s.w(at) as u32 };
            let mark = if ours == theirs { " " } else { "*" };
            line += &format!(" {:x}={theirs:x}/{ours:x}{mark}", at - sf_core::layout::h0(0));
        }
        println!("{line}");
    }
}
