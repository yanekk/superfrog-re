//! Experiment: play a set-A route on the original (AtLevel) and on the engine (Start::Level) side
//! by side; print the first frame their frog cells differ by more than one, and how each ends.
//! `cargo run --release -p sf-bench --example part_ways -- <level> <route.txt>`
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::{Game, LevelId, Start};

fn cell(st: &sf_core::State) -> (i32, i32) {
    let w = |o| st.w(FROG + o) as i16 as i32;
    ((w(f::X) + 0x17).max(0) / 16, (w(f::Y) - 0x10).max(0) / 16)
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: LevelId = a[0].parse().unwrap();
    let script = InputScript::load(std::path::Path::new(&a[1])).unwrap();
    let n = script.lines.last().unwrap().0;
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), n + 200).unwrap().amiga;
    let mut g = Game::with_display(sf_conform::display_data(), Start::Level(id));
    let mut first = None;
    for k in 0..n {
        let x = script.input_at(k);
        sync::step_inserting(&mut amiga, x).unwrap();
        g.step_unseen(x);
        let o = sf_core::State::transplant(&sync::core_memory(amiga.memory()));
        let (co, ce) = (cell(&o), cell(g.state()));
        if first.is_none() && ((co.0 - ce.0).abs() > 1 || (co.1 - ce.1).abs() > 1) {
            first = Some(k);
            println!("parted at frame {k}: original {co:?} engine {ce:?}");
        }
        let dying = o.w(FROG + f::DYING) != 0;
        if k > 20 && (dying || o.l(sf_core::layout::h0(0xa06)) != 0xffff_fffe) {
            println!("original: level_state left -2 or dying at frame {k} (dying {dying}), cell {co:?}");
            break;
        }
    }
    if first.is_none() {
        println!("never parted");
    }
}
