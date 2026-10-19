//! T30's probe: replay a pacing feed headless and list every channel start (DMACON setting its
//! bit) whose sample, as `AUDxLC`/`AUDxLEN` latch it, reads all zero in the engine's chip RAM
//! (`Game::chip_word`): where the mixer would play silence. One line per distinct start: channel,
//! address, length, how many times, the first frame of play. `silent_starts <file.feed>`
use std::collections::BTreeMap;

use sf_core::feed::Feed;
use sf_core::{Game, Start};

fn main() {
    let path = std::env::args().nth(1).expect("a feed");
    let feed = Feed::parse(&std::fs::read_to_string(path).expect("feed")).expect("a feed");
    let mut g = Game::with_display(sf_conform::display_data(), Start::PowerOn { seed: feed.seed });
    let mut at = [(0u32, 0u16); 4];
    let mut silent: BTreeMap<(usize, u32, u16), (u32, usize)> = BTreeMap::new();
    let mut starts = 0;
    for (i, fr) in feed.frames.iter().enumerate() {
        for out in fr.play(&mut g) {
            for w in &out.paula {
                match w.reg {
                    0xa0..=0xdf => {
                        let c = ((w.reg - 0xa0) / 0x10) as usize;
                        match (w.reg - 0xa0) % 0x10 {
                            0 => at[c].0 = (w.value as u32) << 16 | (at[c].0 & 0xffff),
                            2 => at[c].0 = (at[c].0 & 0xffff_0000) | w.value as u32,
                            4 => at[c].1 = w.value,
                            _ => {}
                        }
                    }
                    0x96 if w.value & 0x8000 != 0 => {
                        for c in 0..4 {
                            if w.value & 1 << c == 0 {
                                continue;
                            }
                            starts += 1;
                            let (a, len) = at[c];
                            let n = (len as u32).max(1).min(0x8000);
                            if (0..n).all(|k| g.chip_word(a + 2 * k) == 0) {
                                let e = silent.entry((c, a, len)).or_insert((i as u32 + 1, 0));
                                e.1 += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    println!("{starts} channel starts; all-zero samples:");
    for ((c, a, len), (first, n)) in silent {
        println!("ch {c} ${a:06x} len {len}: {n} times, first at frame {first}");
    }
}
