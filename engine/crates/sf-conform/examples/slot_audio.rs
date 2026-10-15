//! Scratch (T129): the audio DMA slots on lines 296.. of chosen frames of a slot case: observed on
//! the original (DMA owners), and `AudioDma` run on its Paula writes from frame 773 (every channel
//! stopped), with their exact positions and with each write's hpos 0 (as the engine has them).
//! `slot_audio <case.toml> <original frame>...`
use sf_conform::{Case, sync};
use sf_paula::{AudioDma, PaulaWrite};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let frames: Vec<u32> = a[1..].iter().map(|x| x.parse().unwrap()).collect();
    let to = *frames.iter().max().unwrap();
    let s = sync::reach(&case.sync, &case.script, to + 100).unwrap();
    let mut am = s.amiga;
    let mut exact = AudioDma::booted();
    let mut line = AudioDma::booted();
    for f in 0..=to {
        let shown = frames.contains(&f);
        if shown {
            am.set_dma_profile();
        } else {
            am.set_profile(false);
        }
        let cap = sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
        if f < 773 {
            continue;
        }
        let w0: Vec<PaulaWrite> = cap
            .paula
            .iter()
            .map(|w| PaulaWrite { hpos: 0, ..*w })
            .collect();
        let lines = am.frame_lines();
        let e = exact.frame(&cap.paula, lines).to_vec();
        let l = line.frame(&w0, lines).to_vec();
        if !shown {
            continue;
        }
        println!("== {f} ({lines} lines)");
        let mut seen = std::collections::BTreeMap::<u32, Vec<(u16, u8)>>::new();
        for s in am.line_samples() {
            if s.vpos == 0 {
                continue;
            }
            for (h, &o) in s.owners.iter().enumerate() {
                if (4..=7).contains(&o) {
                    seen.entry(s.vpos as u32 - 1)
                        .or_default()
                        .push((h as u16, o - 4));
                }
            }
        }
        let by = |v: &[(u32, u8)], ln: u32| -> String {
            v.iter()
                .filter(|s| s.0 / 227 == ln)
                .map(|s| format!("{}:{}", s.1, s.0 % 227))
                .collect::<Vec<_>>()
                .join(",")
        };
        for ln in 290..lines {
            let o = seen
                .get(&ln)
                .map(|v| {
                    v.iter()
                        .map(|(h, c)| format!("{c}:{h}"))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_else(|| "?".into());
            println!(
                "{ln:3} seen {o:24} exact {:24} line {}",
                by(&e, ln),
                by(&l, ln)
            );
        }
    }
}
