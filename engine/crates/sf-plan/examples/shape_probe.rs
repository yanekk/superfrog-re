//! Diagnostic: walk a level's map; from the first seeds in a box, play every default move and
//! print where each ends (or why not). `shape_probe <level> c0 c1 r0 r1 [max seeds]`
use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::map::{Builder, replay};
use sf_plan::core::{default_moves, facts, landed};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: LevelId = a[0].parse().unwrap();
    let n = |k: usize| a[k].parse::<u16>().unwrap();
    let (c0, c1, r0, r1) = (n(1), n(2), n(3), n(4));
    let max: usize = a.get(5).map_or(3, |s| s.parse().unwrap());
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    let ms = default_moves(true);
    let mut b = Builder::new(&g, &fa, &ms);
    let mut done = 0;
    loop {
        for (k, s) in b.frontier() {
            let (c, r) = k.node.cell;
            if done >= max || !(c0..=c1).contains(&c) || !(r0..=r1).contains(&r) {
                continue;
            }
            done += 1;
            println!("seed {k:?}");
            for wait in [0u16, 12, 25, 50, 100, 150, 200, 300] {
            for m in &ms.families {
                if std::env::var("WAITS").is_err() && wait > 0 { continue; }
                let mac = m.expand();
                let mut inputs: Vec<(u8, u16)> = vec![(0, wait)];
                inputs.extend(mac.holds.iter().map(|&(j, n)| (j.0, n)));
                inputs.push((mac.tail.0, mac.land_cap));
                let t = replay(s, &fa, &inputs);
                if std::env::var("WAITS").is_err() || t.end.cell.1 < r0 {
                    println!("  wait {wait} {m:?}: end {:?} landed {} reject {:?} frames {}", t.end.cell, t.landed, t.reject, t.frames);
                }
            }
            }
        }
        let parts = b.parts(40);
        if parts.is_empty() || done >= max { break; }
        let r = std::thread::scope(|sc| {
            let hs: Vec<_> = parts.iter().map(|p| { let b = &b; sc.spawn(move || b.build_part(p)) }).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        b.absorb(r);
    }
}
