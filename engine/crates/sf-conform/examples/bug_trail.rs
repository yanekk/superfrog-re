//! A bug zip's frog trail, for the video-trail trial: replays the recording as the app played it
//! (`bug::replay`) and writes, per step, `k X Y camx camy dying lives` to stdout, the format of
//! `video_truth`'s tsv, so an owner's recorded play can become a guide path like a video's.
//!
//! `cargo run --release --example bug_trail -- <zip> > trail.tsv`
use sf_core::frog::{FROG, f};
use sf_core::level::camera::{CAMERA_X, CAMERA_Y};
use std::io::Write;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (rec, _) = sf_conform::bug::read_zip(&std::fs::read(&a[0]).unwrap()).unwrap();
    let out = std::io::stdout();
    let mut out = std::io::BufWriter::new(out.lock());
    sf_conform::bug::replay(&rec, |k, g, _| {
        let st = g.state();
        let w = |addr| st.w(addr) as i16 as i32;
        writeln!(
            out,
            "{k} {} {} {} {} {} {}",
            w(FROG + f::X),
            w(FROG + f::Y),
            w(CAMERA_X + 2),
            w(CAMERA_Y + 2),
            w(FROG + f::DYING),
            w(FROG + f::LIVES)
        )
        .unwrap();
    });
}
