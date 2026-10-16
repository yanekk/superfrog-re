//! Plays a script on the original and prints, every `step` frames, the frog's map cell and the
//! coins still needed (`frog+$5c`), and (T168) the hearts, the lives, the wings (`frog+$90`), the spud
//! (`+$7c`) and
//! `level` (h0+$40de, the frame a tally ends):
//! where a completion's route goes (T154).
//!
//! `cargo run --release --example frog_track -- <level> <script.txt> <frames> [step]`
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::layout::h0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = a[0].parse().unwrap();
    let p = InputScript::load(std::path::Path::new(&a[1])).unwrap();
    let n: u32 = a[2].parse().unwrap();
    let step: u32 = a.get(3).map_or(50, |s| s.parse().unwrap());
    let mut s = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), n + 100).unwrap();
    for k in 0..n {
        sync::step_inserting(&mut s.amiga, p.input_at(k)).unwrap();
        if (k + 1) % step == 0 {
            let st = sf_core::State::transplant(&sync::core_memory(s.amiga.memory()));
            let w = |o| st.w(FROG + o) as i16 as i32;
            // The frog's middle a row up, as `plan_exit` places it.
            let (x, y) = (w(f::X) + 0x17, w(f::Y) - 0x10);
            println!(
                "frame {}: cell {},{} coins needed {} hearts {} lives {} wings {} spud {} level {}",
                k + 1,
                x / 16,
                y / 16,
                st.w(h0(0x832)),
                st.w(h0(0x830)),
                st.w(FROG + f::LIVES),
                st.w(FROG + f::FLIGHT),
                st.w(FROG + f::SPUD_ON),
                st.l(h0(0x40de))
            );
        }
    }
}
