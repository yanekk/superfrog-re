//! A frog trail, for guide paths and hand-overs: writes, per step, `k X Y camx camy dying lives`
//! to stdout (frog and camera in level pixels).
//!
//! - From a bug zip: replays the recording as the app played it (`bug::replay`), so an owner's
//!   recorded play can be spliced into a guide (W6-4: the climb over the gap).
//! - From an app-timed route (`sf-plan --app`'s `# timing: app` output): Game ▸ Start Level… with
//!   the dialog's defaults and power-on seed `N N` from its `# seed:` line, then its inputs, so
//!   `tools/handover.py` finds the step where the route reaches a spot. With `--widescreen`
//!   (real-widescreen T12) the level is started with View ▸ Widescreen on, as `sf-plan
//!   --widescreen` starts it; such a route says `# widescreen: on`, and the flag must match it.
//!
//! `cargo run --release --example bug_trail -- <bug.zip | route.txt LEVEL [--widescreen]> > trail.tsv`
use sf_core::frog::{FROG, f};
use sf_core::level::camera::{CAMERA_X, CAMERA_Y};
use sf_core::Game;
use std::io::Write;

fn line(out: &mut impl Write, k: u32, g: &Game) {
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
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let out = std::io::stdout();
    let mut out = std::io::BufWriter::new(out.lock());
    if a[0].ends_with(".zip") {
        let (rec, _) = sf_conform::bug::read_zip(&std::fs::read(&a[0]).unwrap()).unwrap();
        sf_conform::bug::replay(&rec, |k, g, _| line(&mut out, k, g));
        return;
    }
    let text = std::fs::read_to_string(&a[0]).unwrap();
    let wide = a.iter().any(|x| x == "--widescreen");
    // A route replays only from the start it was planned from: a wide route from a 4:3 start parts
    // at once, and the trail would be another game's.
    let wide_route = text.lines().take_while(|l| l.starts_with('#')).any(|l| l.trim() == "# widescreen: on");
    assert_eq!(wide, wide_route, "{}: `# widescreen: on` and --widescreen must agree", a[0]);
    assert!(text.lines().any(|l| l.trim() == "# timing: app"), "{}: not an app-timed route", a[0]);
    let seed: u32 = text
        .lines()
        .find_map(|l| l.strip_prefix("# seed:"))
        .and_then(|s| s.trim().parse().ok())
        .expect("a `# seed: N` line");
    // The route's length: its closing `N: -` line, the last frame number in it.
    let frames = text
        .lines()
        .filter_map(|l| l.split('#').next()?.split(':').next()?.trim().parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    let script = sf_conform::InputScript::parse(&text).unwrap();
    let id: sf_core::LevelId = a[1].parse().unwrap();
    let mut g = Game::start_level(
        sf_conform::display_data(),
        &sf_core::recording::LevelSetup { level: id, ..Default::default() },
        sf_core::Rng::seeded(seed, seed),
    );
    if wide {
        g.set_widescreen(true);
    }
    for k in 0..frames {
        g.step_unseen(script.input_at(k));
        line(&mut out, k, &g);
    }
}
