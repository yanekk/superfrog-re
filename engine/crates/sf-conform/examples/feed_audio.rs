//! T30's probe: replay a pacing feed headless through the app's mixer (`Paula::booted` at
//! 48 kHz) and, for frames of play `from..=to`, print the mix's swing, each channel's DAC range
//! and the frame's sound events. `feed_audio <file.feed> <from> <to>`
use sf_conform::sound_events;
use sf_core::feed::Feed;
use sf_core::{Game, Start};
use sf_paula::Paula;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let feed = Feed::parse(&std::fs::read_to_string(&args[0]).expect("feed")).expect("a feed");
    let (from, to): (usize, usize) = (args[1].parse().unwrap(), args[2].parse().unwrap());
    let mut g = Game::with_display(sf_conform::display_data(), Start::PowerOn { seed: feed.seed });
    let mut paula = Paula::booted(48_000);
    let mut events = sound_events::Paula::default();
    for (i, fr) in feed.frames.iter().enumerate().take(to) {
        let n = i + 1;
        let outs = fr.play(&mut g);
        for blank in g.take_load_paula() {
            paula.frame(&blank, &g, &mut Vec::new());
            events.frame(&blank);
        }
        for (k, out) in outs.iter().enumerate() {
            let mut s = Vec::new();
            paula.frame(&out.paula, &g, &mut s);
            let ev = events.frame(&out.paula);
            if n < from {
                continue;
            }
            let swing = |c: usize| {
                let lo = s.iter().map(|x| x[c]).min().unwrap_or(0) as i32;
                let hi = s.iter().map(|x| x[c]).max().unwrap_or(0) as i32;
                (hi - lo) / 2
            };
            let ch: Vec<String> = paula
                .channel_taps()
                .iter()
                .map(|t| format!("{}..{}", t.iter().min().unwrap_or(&0), t.iter().max().unwrap_or(&0)))
                .collect();
            let ev: Vec<String> = ev.iter().map(|e| e.to_string()).collect();
            println!(
                "{n}.{k}{} swing {} ch [{}] {}",
                if fr.load.is_some() { " load" } else { "" },
                swing(0).max(swing(1)),
                ch.join(" "),
                ev.join("; ")
            );
        }
    }
}
