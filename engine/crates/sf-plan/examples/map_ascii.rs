//! Diagnostic: a level's live markers as text with the map's nodes over them.
//! `map_ascii <level> <c0> <c1> <r0> <r1> [--no-map]`
use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::{default_moves, facts, landed};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: LevelId = a[0].parse().unwrap();
    let n = |k: usize| a[k].parse::<u16>().unwrap();
    let (c0, c1, r0, r1) = (n(1), n(2), n(3), n(4));
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    let nodes: std::collections::BTreeSet<(u16, u16)> = if a.iter().any(|x| x == "--no-map") {
        Default::default()
    } else {
        let (m, _) = sf_plan::threads::build_threaded(&g, &fa, &default_moves(true), 10);
        m.nodes.iter().map(|n| n.cell).collect()
    };
    let st = g.state();
    print!("    ");
    for c in c0..=c1 {
        print!("{}", c % 10);
    }
    println!();
    for r in r0..=r1 {
        print!("{r:3} ");
        for c in c0..=c1 {
            let m = st.w(sf_core::layout::h1(0x218e0 + 0x4c0) + r as u32 * 0x260 + 2 * c as u32) & 0x3f;
            let ch = if nodes.contains(&(c, r)) {
                'N'
            } else if fa.exits.contains(&(c, r)) {
                'E'
            } else if fa.hazards.contains(&(c, r)) {
                '^'
            } else if fa.coins.iter().any(|k| k.cell == (c, r)) {
                'o'
            } else if m >= 0x3a {
                '#'
            } else if m == 0 {
                '.'
            } else {
                (b'a' + (m % 26) as u8) as char
            };
            print!("{ch}");
        }
        println!();
    }
}
