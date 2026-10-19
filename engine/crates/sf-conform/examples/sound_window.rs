//! T30's probe: run a case to frame `to` and print, for frames `from..=to`, both sides' sound
//! events (the original's and the runner's engine's), lag frames included.
//! `sound_window <case.toml> <from> <to>`
use sf_conform::{Alone, Case, sound_events};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut case = Case::load(std::path::Path::new(&args[0])).expect("case");
    let (from, to): (u32, u32) = (args[1].parse().unwrap(), args[2].parse().unwrap());
    case.frames = to + 1;
    case.alone = Alone::Off;
    let mut theirs = sound_events::Paula::default();
    let mut ours = sound_events::Paula::default();
    let r = sf_conform::run_observing_lag(&case, |f, o, t| {
        let (eo, et) = (ours.frame(o), theirs.frame(t));
        if f >= from {
            let s = |e: Vec<sf_conform::SoundEvent>| e.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
            let w = |ws: &[sf_paula::PaulaWrite]| {
                ws.iter()
                    .filter(|w| w.reg == 0x96 || (0xa0..0xe0).contains(&w.reg))
                    .map(|w| format!("{:02x}={:04x}@{}", w.reg, w.value, w.vpos))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            println!("{f} O: {}\n{f} E: {}\n{f} Ow: {}\n{f} Ew: {}", s(et), s(eo), w(t), w(o));
        }
    });
    println!("{:?}", r.map(|_| "ok"));
}
