//! Diagnostic: where moves that never land end, on the first layers of a level's walk.
use std::collections::BTreeMap;
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, LevelId, Start};
use sf_plan::core::map::Builder;
use sf_plan::core::moves::LAND_CAP;
use sf_plan::core::{default_moves, facts, frog_cell, landed};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: LevelId = a[0].parse().unwrap();
    let layers: u32 = a[1].parse().unwrap();
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    let ms = default_moves(true);
    let mut b = Builder::new(&g, &fa, &ms);
    let mut ends: BTreeMap<(u16, u16), (u32, String)> = BTreeMap::new();
    for _ in 0..layers {
        for &(i, m) in b.jobs() {
            let mac = ms.families[m as usize].expand();
            let mut x = b.frontier()[i as usize].1.clone();
            for &(j, n) in &mac.holds { for _ in 0..n { x.step_unseen(FrameInput::joy(j)); } }
            let mut k = 0;
            while !landed(x.state()) && k < mac.land_cap { x.step_unseen(FrameInput::joy(mac.tail)); k += 1; }
            if !landed(x.state()) {
                let st = x.state();
                let w = |o| st.w(FROG + o) as i16;
                let e = ends.entry(frog_cell(st)).or_default();
                e.0 += 1;
                e.1 = format!("vy {} air {} og {} plat {} frozen {} passage {}", w(f::VY), w(f::AIR), w(f::ON_GROUND), w(f::ON_PLATFORM), w(f::FROZEN), w(f::IN_PASSAGE));
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
    let _ = (Joy::NONE, LAND_CAP);
    for (c, (n, s)) in ends { println!("{c:?} x{n}: {s}"); }
}
