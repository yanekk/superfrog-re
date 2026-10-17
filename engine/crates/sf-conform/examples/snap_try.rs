//! Plays scripts on the original from a snapshot `whole_game` saved at a leg's start, each from
//! the same state, and prints for each the frame `level_state` leaves its running -2 (or that it
//! does not within the script and `--frames`), the lives then and the score (T207: which way of
//! fighting the witch wins in the chain, the frog carrying W1's wings).
//!
//! `cargo run --release --example snap_try -- --snap <leg-NN.snap> [--frames N] <script.txt>...`
use sf_conform::{InputScript, sync};
use sf_core::layout::h0;

const LEVEL_STATE: u32 = h0(0xa06);
const LIVES: u32 = h0(0x82e);
const SCORE: u32 = h0(0x834);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut snap, mut frames, mut scripts) = (None, 4000u32, Vec::new());
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--snap" => {
                snap = Some(args[i + 1].clone());
                i += 1;
            }
            "--frames" => {
                frames = args[i + 1].parse().unwrap();
                i += 1;
            }
            s => scripts.push(s.to_string()),
        }
        i += 1;
    }
    let path = std::path::PathBuf::from(snap.expect("--snap <leg-NN.snap>"));
    let mut a = sync::at_snapshot_file(&path, frames + 100).unwrap_or_else(|e| panic!("{e}"));
    let start = a.snapshot();
    let long = |a: &sf_oracle::Amiga, addr: u32| {
        let b = a.peek(addr, 4);
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    };
    for s in &scripts {
        let script = InputScript::load(std::path::Path::new(s)).unwrap_or_else(|e| panic!("{e}"));
        a.restore(&start);
        let lives0 = (long(&a, LIVES) >> 16) as i16;
        let mut seen = format!("still running at {frames}");
        for f in 0..frames {
            sync::step_inserting(&mut a, script.input_at(f)).unwrap();
            if f > 10 && long(&a, LEVEL_STATE) != 0xffff_fffe {
                seen = format!("level_state {:#x} at frame {f}", long(&a, LEVEL_STATE));
                break;
            }
        }
        let lives = (long(&a, LIVES) >> 16) as i16;
        println!("{s}: {seen}, lives {lives0} -> {lives}, score {}", long(&a, SCORE));
    }
}
