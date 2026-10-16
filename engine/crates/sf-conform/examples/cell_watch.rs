//! Plays a script on the original and prints, on each frame from `from` where any watched cell's
//! marker (its low 6 bits in `map_rows[1]`) changes, the markers, the frog's cell, its raw x, y
//! and the clock (h0+$876): when a switch's gate opens or closes under the frog (T168).
//!
//! `cargo run --release --example cell_watch -- <level> <script.txt> <frames> <from> col,row...`
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::layout::{h0, h1};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = a[0].parse().unwrap();
    let p = InputScript::load(std::path::Path::new(&a[1])).unwrap();
    let n: u32 = a[2].parse().unwrap();
    let from: u32 = a[3].parse().unwrap();
    let cells: Vec<(u32, u32)> = a[4..]
        .iter()
        .map(|c| {
            let (x, y) = c.split_once(',').unwrap();
            (x.parse().unwrap(), y.parse().unwrap())
        })
        .collect();
    let mut s = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), n + 100).unwrap();
    let row0 = h1(0x218e0 + 0x4c0);
    let mut last: Option<Vec<u16>> = None;
    for k in 0..n {
        sync::step_inserting(&mut s.amiga, p.input_at(k)).unwrap();
        if k < from {
            continue;
        }
        let st = sf_core::State::transplant(&sync::core_memory(s.amiga.memory()));
        let m: Vec<u16> = cells
            .iter()
            .map(|&(c, r)| st.w(row0 + r * 0x260 + 2 * c) & 0x3f)
            .collect();
        let w = |o| st.w(FROG + o) as i16 as i32;
        if last.as_ref() != Some(&m) || (k - from) % 50 == 0 {
            println!(
                "{k}: {:?} frog cell {},{} raw {},{} wr {} wl {} clock {:x}",
                m,
                (w(f::X) + 0x17) / 16,
                (w(f::Y) - 0x10) / 16,
                w(f::X),
                w(f::Y),
                w(f::WALL_RIGHT),
                w(f::WALL_LEFT),
                st.l(h0(0x876))
            );
        }
        last = Some(m);
    }
}
