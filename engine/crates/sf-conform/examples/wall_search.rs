//! T168: whether play reaches `frog_tiles`'s wall loop (h0+$e3fe..$e412, DESIGN §2.11) from a
//! place. A script is played on the original; from its end, random input runs on the engine, each
//! frame tested as `walls` (`level/collision.rs`) tests it: facing left with `$3d` in both cells
//! it probes, which the original never leaves. Prints the first run that gets there, as holds, and
//! each new lowest frog x seen (raw, `+$00`) within rows `y0..y1` (raw y).
//!
//! `cargo run --release --example wall_search -- <level> <script.txt> <runs> <len> <y0> <y1>`
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, Start};

/// `map_rows` (h0+$17334): each map row's address in memory.
const MAP_ROWS: u32 = h0(0x17334);

/// Whether `walls` would find both wall flags in `st`: facing left (`+$12` nonzero), the cells it
/// probes from `map_rows[y >> 4] + (x >> 4) * 2 + 2 + $260` (y moved by a falling vy), at
/// `+$4c0` and `+$4c2`, both `$3d`.
fn both_flags(st: &sf_core::State) -> bool {
    let (x, y, vy) = (st.w(FROG + f::X), st.w(FROG + f::Y), st.w(FROG + f::VY));
    let y = if (vy as i16) >= 0 {
        y.wrapping_add(vy)
    } else {
        y
    };
    let row = st.l(MAP_ROWS.wrapping_add(((y.wrapping_sub(0x10) >> 4) as u32) * 4));
    let a2 = row + ((x >> 4) as u32) * 2 + 2 + 0x260;
    st.w(FROG + f::FACING) != 0
        && st.w(a2 + 0x4c0) & 0x3f == 0x3d
        && st.w(a2 + 0x4c2) & 0x3f == 0x3d
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = a[0].parse().unwrap();
    let text = std::fs::read_to_string(&a[1]).unwrap();
    let p = InputScript::parse(&text).unwrap();
    let n = |k: usize| a[k].parse::<i32>().unwrap();
    let (runs, len, y0, y1) = (n(2), n(3), n(4), n(5));
    let at: u32 = text
        .lines()
        .filter_map(|l| l.split(':').next()?.trim().parse().ok())
        .max()
        .unwrap_or(0);
    let mut s = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), at + 100).unwrap();
    for k in 0..at {
        sync::step_inserting(&mut s.amiga, p.input_at(k)).unwrap();
    }
    let start = Game::with_display(
        sf_conform::display_data(),
        Start::Transplant(sync::core_memory(s.amiga.memory())),
    );
    let acts = [
        Joy::NONE,
        Joy::LEFT,
        Joy::RIGHT,
        Joy::UP,
        Joy::UP | Joy::LEFT,
        Joy::UP | Joy::RIGHT,
        Joy::DOWN,
    ];
    let mut seed: u64 = 12345;
    let mut best = i32::MAX;
    for run in 0..runs {
        let mut g = start.clone();
        let mut seq = Vec::new();
        let mut fr = 0;
        while fr < len {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let act = acts[((seed >> 33) % acts.len() as u64) as usize];
            let hold = 2 + ((seed >> 20) % 14) as i32;
            seq.push((act, hold));
            for _ in 0..hold {
                g.step_unseen(FrameInput::joy(act));
                fr += 1;
                let st = g.state();
                let (x, y) = (
                    st.w(FROG + f::X) as i16 as i32,
                    st.w(FROG + f::Y) as i16 as i32,
                );
                let holds = || {
                    let t: Vec<String> = seq
                        .iter()
                        .map(|(j, h)| {
                            format!("{}:{h}", InputScript::format_input(FrameInput::joy(*j)))
                        })
                        .collect();
                    t.join(",")
                };
                if both_flags(st) {
                    println!(
                        "both flags: run {run} frame {fr}, x {x} y {y} | {}",
                        holds()
                    );
                    return;
                }
                if x < best && (y0..y1).contains(&y) {
                    best = x;
                    println!("run {run} frame {fr}: x {x} y {y} | {}", holds());
                }
            }
        }
    }
    println!("both flags never, in {runs} runs of {len} frames from frame {at}");
}
