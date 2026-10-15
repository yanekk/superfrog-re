//! Plays a script on the original, then tries input runs on the engine from there, printing the
//! frog's cell (as `plan_exit` reads it: the lower half) every 2 frames, `!` once a life is gone
//! (T122: how high a jump or a pad's throw reaches).
//!
//! `cargo run --release --example try_inputs -- <level> <script.txt> <trial>...`, a trial
//! `R:8,UR:12,-:4` (held inputs and their frames; keys L R U D F, `-` none).
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, Start};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = a[0].parse().unwrap();
    let text = std::fs::read_to_string(&a[1]).unwrap();
    let p = InputScript::parse(&text).unwrap();
    let n: u32 = text
        .lines()
        .filter_map(|l| l.split(':').next()?.trim().parse().ok())
        .max()
        .unwrap_or(0);
    let mut s = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), n + 100).unwrap();
    for k in 0..n {
        sync::step_inserting(&mut s.amiga, p.input_at(k)).unwrap();
    }
    let mem = sync::core_memory(s.amiga.memory());
    let display = sf_conform::display_data();
    let start = Game::with_display(display, Start::Transplant(mem));
    let cell = |g: &Game| {
        let w = |o| g.state().w(FROG + o) as i16 as i32;
        (
            (w(f::X) + 0x17) / 16,
            (w(f::Y) - 0x10) / 16,
            w(f::VY),
            g.state().w(FROG + f::LIVES),
        )
    };
    println!("start {:?}", cell(&start));
    for t in &a[2..] {
        let mut g = start.clone();
        let mut out = Vec::new();
        let mut fr = 0;
        for step in t.split(',') {
            let (k, c) = step.split_once(':').unwrap();
            let mut j = Joy::NONE;
            let mut fire = false;
            for ch in k.chars() {
                match ch {
                    'L' => j = j | Joy::LEFT,
                    'R' => j = j | Joy::RIGHT,
                    'U' => j = j | Joy::UP,
                    'D' => j = j | Joy::DOWN,
                    'F' => fire = true,
                    _ => {}
                }
            }
            let x = if fire {
                FrameInput::joy(j | Joy::FIRE)
            } else {
                FrameInput::joy(j)
            };
            for _ in 0..c.parse::<u32>().unwrap() {
                g.step(x);
                fr += 1;
                if fr % 2 == 0 {
                    let (cx, cy, _, l) = cell(&g);
                    out.push(format!(
                        "{cx},{cy}{}",
                        if l != cell(&start).3 { "!" } else { "" }
                    ));
                }
            }
        }
        println!("{t}: {}", out.join(" "));
    }
}
