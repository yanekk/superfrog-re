//! T213's probe: replay a pacing feed and print the engine's scene and COP1LC at each frame of play
//! where it changes, within `from..=to`. `scene_at <file.feed> <from> <to>`
use sf_core::feed::Feed;
use sf_core::{Game, Start};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let feed = Feed::parse(&std::fs::read_to_string(&a[0]).expect("feed")).expect("a feed");
    let (from, to): (usize, usize) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let mut g = Game::with_display(sf_conform::display_data(), Start::PowerOn { seed: feed.seed });
    let mut last = String::new();
    for (i, fr) in feed.frames.iter().enumerate().take(to) {
        fr.play(&mut g);
        let now = format!("{:?} cop {:x?}", g.scene(), g.cop1lc());
        if i + 1 >= from && now != last {
            println!("{} {now}", i + 1);
        }
        last = now;
    }
}
