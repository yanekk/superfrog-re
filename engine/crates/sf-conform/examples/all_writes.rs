//! T213's probe: both sides' Paula writes (line:hpos reg=value) over a case's frames `from..=to`
//! (the runner's engine, anchored to the original's positions), and the engine run alone's (A). `all_writes <case.toml> <from> <to>`
use sf_conform::{Alone, Case};

fn show(ws: &[sf_paula::PaulaWrite]) -> String {
    ws.iter().map(|w| format!("{}:{} {:02x}={:04x}", w.vpos, w.hpos, w.reg, w.value)).collect::<Vec<_>>().join("  ")
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut case = Case::load(std::path::Path::new(&a[0])).expect("case");
    let (from, to): (u32, u32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    {
        let mut short = case.clone();
        short.frames = to;
        let (feed, _, start) = sf_conform::channels::recorded(&short).unwrap_or_else(|e| panic!("{e}"));
        let mut g = sf_core::Game::with_display(sf_conform::display_data(), start);
        for (i, fr) in feed.frames.iter().enumerate().take(to as usize) {
            let outs = fr.play(&mut g);
            let n = i as u32 + 1;
            for b in g.take_load_paula() { if n >= from { println!("{n} A blank: {}", show(&b)); } }
            if n >= from { for o in outs { println!("{n} A: {}", show(&o.paula)); } }
        }
    }
    case.frames = to + 1;
    case.alone = Alone::Off;
    let _ = sf_conform::run_observing_lag(&case, |f, o, t| {
        if f + 1 >= from && f < to {
            println!("{} O: {}", f + 1, show(t));
            println!("{} E: {}", f + 1, show(o));
        }
    });
}
