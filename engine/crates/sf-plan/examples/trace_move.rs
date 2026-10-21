//! Diagnostic: the first never-landing move on a level's first layers, frame by frame.
use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::map::Builder;
use sf_plan::core::{default_moves, facts, frog_cell, landed};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: LevelId = a[0].parse().unwrap();
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    let ms = default_moves(true);
    let mut b = Builder::new(&g, &fa, &ms);
    loop {
        for &(i, m) in b.jobs() {
            let mac = ms.families[m as usize].expand();
            let mut x = b.frontier()[i as usize].1.clone();
            let mut log = vec![];
            let mut step = |x: &mut Game, j| {
                x.step_unseen(FrameInput::joy(j));
                let st = x.state();
                let w = |o| st.w(FROG + o) as i16;
                log.push(format!("{:?} y{:x} vy{} air{} og{} gt{} wr{} wl{} hb{} lt{}", frog_cell(st), w(f::Y), w(f::VY), w(f::AIR), w(f::ON_GROUND), w(f::GRAVITY_TICK), w(f::WALL_RIGHT), w(f::WALL_LEFT), w(f::HEAD_BUMP), w(f::LANDED)));
            };
            for &(j, n) in &mac.holds { for _ in 0..n { step(&mut x, j); } }
            let mut k = 0;
            while !landed(x.state()) && k < mac.land_cap { step(&mut x, mac.tail); k += 1; }
            if !landed(x.state()) {
                println!("{:?} {:?}", b.frontier()[i as usize].0, ms.families[m as usize]);
                for (n, l) in log.iter().enumerate() { if n % 3 == 0 { println!("{n} {l}"); } }
                return;
            }
        }
        let parts = b.parts(40);
        let r = std::thread::scope(|sc| {
            let hs: Vec<_> = parts.iter().map(|p| { let b = &b; sc.spawn(move || b.build_part(p)) }).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        b.absorb(r);
    }
}
