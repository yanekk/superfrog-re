//! Scratch (T164): the original's chip RAM over the W1 module (at `h0+$9ec`) against the module
//! file, at chosen original frames. `t164_module <case.toml> <frame>...`
use sf_conform::{Case, sync};
use sf_core::layout::h0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let frames: Vec<u32> = a[1..].iter().map(|x| x.parse().unwrap()).collect();
    let to = *frames.iter().max().unwrap();
    let s = sync::reach(&case.sync, &case.script, to + 100).unwrap();
    let mut am = s.amiga;
    let data = sf_conform::display_data();
    let tune = data.sound().tune(sf_core::audio::TuneId::Title).to_vec();
    for f in 0..=to {
        am.step(case.script.input_at(f)).unwrap();
        if !frames.contains(&f) {
            continue;
        }
        let m = am.peek(h0(0x9ec), 4);
        let at = u32::from_be_bytes([m[0], m[1], m[2], m[3]]);
        for t in [] as [sf_core::audio::TuneId; 0] {
            let tb = data.sound().tune(t);
            let r = am.peek(at, tb.len());
            let same = (0..tb.len()).filter(|&i| r[i] == tb[i]).count();
            println!("  {t:?}: {same}/{} same", tb.len());
        }
        let ram = am.peek(at, tune.len());
        let mut runs = Vec::new();
        let mut k = 0x200;
        while k < tune.len() {
            if ram[k] != tune[k] {
                let s = k;
                while k < tune.len() && ram[k] != tune[k] {
                    k += 1;
                }
                runs.push((s, k));
            }
            k += 1;
        }
        let n: usize = runs.iter().map(|r| r.1 - r.0).sum();
        println!(
            "O {f}: module at ${at:06x} len ${:x}: {n} bytes differ in {} runs: {:x?}",
            tune.len(),
            runs.len(),
            &runs[..runs.len().min(20)]
        );
    }
}
