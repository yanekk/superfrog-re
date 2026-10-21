//! Diagnostic: walk W6-3's map; from seeds near 149-153,13 try jump shapes and print which land
//! at col >= 160, row <= 7.
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, LevelId, Start};
use sf_plan::core::map::{Builder, replay};
use sf_plan::core::{default_moves, facts, landed};

fn main() {
    let id: LevelId = "W6-3".parse().unwrap();
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    let mut b = Builder::new(&g, &fa, &default_moves(true));
    let dirs = [("-", Joy::NONE), ("R", Joy::RIGHT), ("L", Joy::LEFT)];
    let mut tried = 0;
    loop {
        for (k, s) in b.frontier() {
            let (c, r) = k.node.cell;
            if !(140..=163).contains(&c) || !(10..=15).contains(&r) {
                continue;
            }
            for (n1, d1) in dirs {
                for h in [4u16, 8, 12, 16, 20, 30] {
                    for (n2, d2) in dirs {
                        for up in [false, true] {
                            let t = if up { d2 | Joy::UP } else { d2 };
                            let mut x = s.clone();
                            let mut f = 0;
                            for _ in 0..h { x.step_unseen(FrameInput::joy(d1 | Joy::UP)); f += 1; }
                            while !landed(x.state()) && f < 200 { x.step_unseen(FrameInput::joy(t)); f += 1; }
                            tried += 1;
                            let e = sf_plan::core::frog_cell(x.state());
                            if e.0 >= 158 && e.1 <= 8 {
                                println!("{k:?}: U{n1} {h} then {n2}{} -> {e:?} in {f}", if up { "+U" } else { "" });
                            }
                        }
                    }
                }
            }
        }
        let parts = b.parts(40);
        if parts.is_empty() { break; }
        let r = std::thread::scope(|sc| {
            let hs: Vec<_> = parts.iter().map(|p| { let b = &b; sc.spawn(move || b.build_part(p)) }).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        b.absorb(r);
    }
    let _ = replay;
    println!("tried {tried}");
}
