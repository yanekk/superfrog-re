//! T213's probe: replay a pacing feed and count, per scene, frames whose Paula writes all sit at
//! (0, 0) and frames whose DMACON off and on for one channel land at one clock.
//! `write_positions <file.feed> [print_from print_to]`
use sf_core::feed::Feed;
use sf_core::{Game, Start};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let feed = Feed::parse(&std::fs::read_to_string(&a[0]).expect("feed")).expect("a feed");
    let (pf, pt): (usize, usize) = (a.get(1).map_or(0, |s| s.parse().unwrap()), a.get(2).map_or(0, |s| s.parse().unwrap()));
    let mut g = Game::with_display(sf_conform::display_data(), Start::PowerOn { seed: feed.seed });
    let mut stats = std::collections::BTreeMap::<String, [u32; 3]>::new();
    for (i, fr) in feed.frames.iter().enumerate() {
        let scene = format!("{:?}", g.scene()).chars().take(12).collect::<String>();
        let outs = fr.play(&mut g);
        let blanks = g.take_load_paula();
        let mut all: Vec<(String, Vec<sf_paula::PaulaWrite>)> = blanks.into_iter().map(|b| ("blank".to_string(), b)).collect();
        all.extend(outs.into_iter().map(|o| (scene.clone(), o.paula)));
        for (s, w) in &all {
            if w.is_empty() { continue; }
            let e = stats.entry(s.clone()).or_default();
            e[0] += 1;
            if w.iter().all(|w| w.vpos == 0 && w.hpos == 0) { e[1] += 1; }
            // a channel off then on at one clock
            let mut same = false;
            for (k, x) in w.iter().enumerate() {
                if x.reg == 0x96 && x.value & 0x8000 == 0 {
                    for y in &w[k + 1..] {
                        if y.reg == 0x96 && y.value & 0x8000 != 0 && y.value & x.value & 0xf != 0 {
                            let d = (y.vpos as i32 * 227 + y.hpos as i32) - (x.vpos as i32 * 227 + x.hpos as i32);
                            if d < 227 * 4 { same = true; }
                            break;
                        }
                    }
                }
            }
            if same { e[2] += 1; }
            if i + 1 >= pf && i + 1 <= pt {
                let l: Vec<String> = w.iter().filter(|w| w.reg == 0x96).map(|w| format!("{:04x}@{}.{}", w.value, w.vpos, w.hpos)).collect();
                println!("{} {s}: {}", i + 1, l.join(" "));
            }
        }
    }
    for (s, e) in stats { println!("{s}: {} frames with writes, {} all at 0, {} off-on under 4 lines", e[0], e[1], e[2]); }
}
