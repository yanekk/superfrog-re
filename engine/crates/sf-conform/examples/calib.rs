//! Scratch (T36): the engine's beam lines for its custom writes against the original's, per frame.
//! `calib <case> [from] [to] [all]` — prints frames from..to; always prints the error summary.
use sf_conform::{Case, sync};
use sf_core::{Game, Start};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&args[0])).unwrap();
    let from: u32 = args.get(1).map_or(0, |s| s.parse().unwrap());
    let to: u32 = args.get(2).map_or(case.frames, |s| s.parse().unwrap());
    let all = args.get(3).is_some();
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, case.frames + 10, frozen).unwrap();
    let mut a = synced.amiga;
    let display = sf_conform::display_data();
    let mut game = Game::with_display(display.clone(), Start::Transplant(synced.memory));
    if frozen == sf_conform::freeze::Freeze::Objects { game.frog_only(); }
    let mut errs: Vec<i32> = Vec::new();
    let mut worst: Vec<(i32, u32, String)> = Vec::new();
    let mut after_mid = false;
    for f in 0..case.frames.min(to) {
        frozen.apply(&mut a, &display);
        let input = case.script.input_at(f);
        let theirs = a.step(input).unwrap();
        let mem = sync::core_memory(a.memory());
        let pacing = sync::pacing(&a.cpu(), &mem, &theirs.paula);
        let ours = game.step_paced(input, pacing);
        let mid = sync::mid_pass(&a.cpu(), &mem);
        let pick = |w: &&sf_paula::PaulaWrite| all || w.reg < 0xa0;
        let t: Vec<_> = theirs.paula.iter().filter(pick).collect();
        let o: Vec<_> = ours.paula.iter().filter(pick).collect();
        let same = t.len() == o.len() && t.iter().zip(&o).all(|(x, y)| x.reg_value() == y.reg_value());
        if same && !mid && !after_mid {
            for (x, y) in t.iter().zip(&o) {
                let e = (y.vpos as i32 * 227 + y.hpos as i32) - (x.vpos as i32 * 227 + x.hpos as i32);
                let e = e / 227;
                errs.push(e);
                worst.push((e.abs(), f, format!("{:02x}={:04x} o{} e{}", x.reg, x.value, x.vpos, y.vpos)));
            }
        }
        if same && !mid && !after_mid && std::env::var("SEG").is_ok() {
            let mut prev: Option<(i32, i32, String)> = None;
            for (x, y) in t.iter().zip(&o) {
                let (tx, ty) = (x.vpos as i32 * 227 + x.hpos as i32, y.vpos as i32 * 227 + y.hpos as i32);
                let lab = format!("{:02x}={:04x}@{}", x.reg, x.value, x.vpos);
                if let Some((px, py, pl)) = &prev {
                    let e = ((ty - py) - (tx - px)) / 227;
                    if e.abs() >= 2 { println!("SEG {f} {pl} -> {lab}: original {} lines, engine {} ({e:+})", (tx - px) / 227, (ty - py) / 227); }
                } else {
                    let e = (ty - tx) / 227;
                    if e.abs() >= 2 { println!("SEG {f} start -> {lab}: original {}, engine {} ({e:+})", tx / 227, ty / 227); }
                }
                prev = Some((tx, ty, lab));
            }
        }
        if f >= from {
            let line: Vec<String> = if same {
                t.iter().zip(&o).map(|(x, y)| format!("{:02x}={:04x} {}/{}", x.reg, x.value, x.vpos, y.vpos as i32 - x.vpos as i32)).collect()
            } else {
                vec![format!("DIFFER o:{} e:{}", t.iter().map(|w| format!("{:02x}={:04x}@{}", w.reg, w.value, w.vpos)).collect::<Vec<_>>().join(" "),
                    o.iter().map(|w| format!("{:02x}={:04x}@{}", w.reg, w.value, w.vpos)).collect::<Vec<_>>().join(" "))]
            };
            println!("{f} {:?}{} {}", pacing, if mid { " MID" } else { "" }, line.join(" "));
        }
        after_mid = mid;
        if mid {
            // keep the engine on the original's memory after a lag, as the runner does
        }
        if after_mid { continue; }
        if sf_conform::compare::vars(game.layout(), &game.probe(), &mem, &[]).is_some() && !mid {
            game = Game::with_display(display.clone(), Start::Transplant(mem.clone()));
            if frozen == sf_conform::freeze::Freeze::Objects { game.frog_only(); }
        }
    }
    errs.sort();
    let n = errs.len().max(1);
    let within = |k: i32| errs.iter().filter(|e| e.abs() <= k).count() * 100 / n;
    println!("writes {}: median {} p5 {} p95 {}; |e|<=1 {}% <=3 {}% <=10 {}%", errs.len(),
        errs.get(n / 2).copied().unwrap_or(0), errs.get(n / 20).copied().unwrap_or(0), errs.get(n * 19 / 20).copied().unwrap_or(0),
        within(1), within(3), within(10));
    worst.sort_by(|a, b| b.0.cmp(&a.0));
    for w in worst.iter().take(8) { println!("  worst {} at frame {}: {}", w.0, w.1, w.2); }
}
