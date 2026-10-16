//! T168: puts the frog where `frog_tiles`'s wall test (h0+$e3fe..$e412) would find both wall
//! flags, and runs the original and the engine on from there: whether the original loops for ever
//! (its clock and frog frozen, the main program in the loop) and what the engine does instead
//! (DESIGN §2.11). The script is played on the original to `at`; then the frog's x, y and facing
//! (`+$12`, nonzero: left) are written and both run `frames` more frames, idle.
//!
//! `cargo run --release --example wall_place -- <level> <script.txt> <at> <x> <y> <facing> <frames>`
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_core::layout::h0;
use sf_core::{Game, Start};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = a[0].parse().unwrap();
    let p = InputScript::load(std::path::Path::new(&a[1])).unwrap();
    let n = |k: usize| a[k].parse::<u32>().unwrap();
    let (at, x, y, facing, frames) = (n(2), n(3) as u16, n(4) as u16, n(5) as u16, n(6));
    let mut s = sync::reach(
        &SyncPoint::AtLevel(id),
        &InputScript::idle(),
        at + frames + 100,
    )
    .unwrap();
    for k in 0..at {
        sync::step_inserting(&mut s.amiga, p.input_at(k)).unwrap();
    }
    for (o, v) in [(f::X, x), (f::Y, y), (f::FACING, facing), (f::VX, 0)] {
        s.amiga.poke(FROG + o, &v.to_be_bytes());
    }
    let mut g = Game::with_display(
        sf_conform::display_data(),
        Start::Transplant(sync::core_memory(s.amiga.memory())),
    );
    for k in 0..frames {
        sync::step_inserting(&mut s.amiga, FrameInput::default()).unwrap();
        g.step_unseen(FrameInput::default());
        let o = sf_core::State::transplant(&sync::core_memory(s.amiga.memory()));
        let pc = s.amiga.cpu().pc.wrapping_sub(h0(0));
        let st = g.state();
        if k % 10 == 0 || k < 5 {
            println!(
                "+{k}: original clock {:x} x {} y {} pc {pc:x} | engine clock {:x} x {} y {}",
                o.l(h0(0x876)),
                o.w(FROG + f::X) as i16,
                o.w(FROG + f::Y) as i16,
                st.l(h0(0x876)),
                st.w(FROG + f::X) as i16,
                st.w(FROG + f::Y) as i16
            );
        }
    }
}
