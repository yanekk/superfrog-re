//! T30's probe: replay a pacing feed headless through two of the app's mixers, one given the
//! engine's Paula writes as they are (all at the frame's first beam position), one with them a
//! line apart, as the original's driver spaces them; count the frames of play whose channels'
//! output differs, and list the first. Where they differ a channel turned off and on again at
//! one instant missed its restart. `retrigger_lost <file.feed>`
use sf_core::feed::Feed;
use sf_core::{Game, Start};
use sf_paula::{Paula, PaulaWrite};

fn main() {
    let path = std::env::args().nth(1).expect("a feed");
    let feed = Feed::parse(&std::fs::read_to_string(path).expect("feed")).expect("a feed");
    let mut g = Game::with_display(sf_conform::display_data(), Start::PowerOn { seed: feed.seed });
    let (mut at_once, mut spread) = (Paula::booted(48_000), Paula::booted(48_000));
    let spaced = |w: &[PaulaWrite]| -> Vec<PaulaWrite> {
        w.iter().enumerate().map(|(i, w)| PaulaWrite { vpos: i as u16, hpos: 0, ..*w }).collect()
    };
    let (mut differ, mut first) = (0, Vec::new());
    for (i, fr) in feed.frames.iter().enumerate() {
        let outs = fr.play(&mut g);
        for blank in g.take_load_paula() {
            at_once.frame(&blank, &g, &mut Vec::new());
            spread.frame(&spaced(&blank), &g, &mut Vec::new());
        }
        let mut d = false;
        for out in &outs {
            at_once.frame(&out.paula, &g, &mut Vec::new());
            spread.frame(&spaced(&out.paula), &g, &mut Vec::new());
            let range = |p: &Paula| -> Vec<(i16, i16)> {
                p.channel_taps()
                    .iter()
                    .map(|t| (*t.iter().min().unwrap_or(&0), *t.iter().max().unwrap_or(&0)))
                    .collect()
            };
            let (a, b) = (range(&at_once), range(&spread));
            if a.iter().zip(&b).any(|(x, y)| (x.1 - x.0 - (y.1 - y.0)).abs() > 256) {
                d = true;
                if first.len() < 15 {
                    first.push(format!("{}: at once {a:?}, spread {b:?}", i + 1));
                }
            }
        }
        differ += d as u32;
    }
    println!("{differ} of {} frames of play differ", feed.frames.len());
    for l in first {
        println!("{l}");
    }
}
