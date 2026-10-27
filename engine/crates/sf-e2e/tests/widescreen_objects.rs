//! real-widescreen T08 (DESIGN §2.9): the app's widescreen strips show the bobs, drawn from the
//! shown buffer's wide draw list. W1-1 by Game ▸ Start Level… (its defaults, no recording) with
//! widescreen on from the start, walking right for 1 500 frames; every frame the app shows is the
//! engine's widescreen picture headless, strips included, and on some frame a bob is drawn in the
//! right strip.
//!
//! Since T10 the app's widescreen is the widescreen game, mode and view together, so the headless
//! game has both set too, after `Game::start_level` as the app does it.

mod common;

use common::cache_data;
use sf_core::input::{FrameInput, Joy};
use sf_core::recording::LevelSetup;
use sf_core::{DisplayData, Game, Rng, View};
use sf_e2e::{App, Image, LaunchOpts};

const N: u32 = 65;
const WIDE: View = View {
    extra_columns_each_side: N as u16,
};
/// The step Start Level starts W1-1 after, and the right arrow held for 1 500 frames from just
/// after its loading picture and fade-in (about 807 frames).
const START: u64 = 4;
const WALK: std::ops::Range<u64> = 820..2320;
const FRAMES: u64 = 2400;

fn input(n: u64) -> FrameInput {
    if WALK.contains(&n) { FrameInput::joy(Joy::RIGHT) } else { FrameInput::NONE }
}

#[test]
fn w1_1_s_right_strip_shows_bobs_and_every_frame_is_the_engine_s() {
    let mut app = App::launch(LaunchOpts {
        max_frames: START + FRAMES,
        timeout_s: 600,
        ..Default::default()
    });
    let script = format!(
        "2: menu widescreen\n5: menu start-level\nform record off\nform start\n{}: down ArrowRight\n{}: up ArrowRight\n",
        START + WALK.start,
        START + WALK.end
    );
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains(&format!("superfrog: started W1-1 after step {START}\n")), "{}", c.stderr);
    let data = cache_data(&app.home);
    let display = std::sync::Arc::new(DisplayData::new(&data));
    let mut g = Game::start_level(display.clone(), &LevelSetup::default(), Rng::seeded(0, 0));
    g.set_widescreen(true);
    g.set_view(WIDE);
    let mut in_level = 0;
    let mut bob_frames = 0;
    for n in 1..=FRAMES {
        let p = g.step(input(n)).picture;
        let f = START + n;
        assert_eq!(c.read_frame(f), Image::of_picture(&p), "frame {f} (W1-1's {n}) is not the engine's");
        if !g.scene().is_level() {
            continue;
        }
        in_level += 1;
        let l = sf_core::render::strip_layers(g.state(), &display, WIDE);
        let w = l.picture.width as usize;
        let right = l.bobs.iter().enumerate().any(|(at, v)| v.is_some() && at % w >= w - N as usize);
        bob_frames += right as u32;
    }
    assert!(in_level > 1500, "{in_level} frames in W1-1");
    assert!(bob_frames > 0, "no frame showed a bob in the right strip");
}
