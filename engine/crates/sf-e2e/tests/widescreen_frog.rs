//! real-widescreen T09 (DESIGN §2.5, §2.9): the frog in the app's right strip. W1-1 played
//! headless in widescreen to its right end, the frog against the last wall in the strip, saved as a
//! state with this build and loaded into the app, paused, at 2× with widescreen on: every frame
//! the app shows is the engine's widescreen picture of that state headless, the window is that
//! frame at a whole scale, and the frog's pixels are drawn in the right strip.
//!
//! The game is put there headless, which is quicker than walking it there through the app. A state
//! does not hold the mode; the app sets it on the loaded game from its setting (T10), and the
//! game is shown, held by the pause. `widescreen_app.rs` plays such a state on.

mod common;

use common::{assert_integer_scaled, cache_data};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::level::camera::LEVEL_WIDTH;
use sf_core::{DisplayData, Game, Start, View};
use sf_e2e::{App, Image, LaunchOpts};

const N: u32 = 65;
const WIDE: View = View {
    extra_columns_each_side: N as u16,
};
const LOAD: u64 = 10;
const SHOT: u64 = 40;
const FRAMES: u64 = 60;

/// This build's name, from the app's refusal of a state from another (as `drill.rs`).
fn our_build(app: &mut App, data: &sf_core::GameData) -> String {
    let other = Game::new(data, Start::Level("W1-1".parse().unwrap()));
    std::fs::create_dir_all(app.home.join("states")).unwrap();
    std::fs::write(app.home.join("states/slot-5.sfstate"), other.save("not this build", &sf_assets::cache_hash(data))).unwrap();
    let c = app.run_script("2: menu load state 5\n");
    std::fs::remove_file(app.home.join("states/slot-5.sfstate")).unwrap();
    let line = c.prompts.lines().next().unwrap_or_else(|| panic!("no refusal: {}", c.stderr));
    let ours = line.split("; this is ").nth(1).and_then(|s| s.strip_suffix(". It was not loaded."));
    ours.unwrap_or_else(|| panic!("{line}")).to_string()
}

fn w(g: &Game, a: u32) -> u16 {
    let b = g.peek(a, 2);
    u16::from_be_bytes([b[0], b[1]])
}

/// W1-1 in widescreen, the frog put near the end and walked into the right strip against the last
/// wall (sf-core `widescreen_frog.rs`), then left standing.
fn at_right_end(data: &sf_core::GameData) -> Game {
    let mut g = Game::new(data, Start::Level("W1-1".parse().unwrap()));
    g.set_widescreen(true);
    for _ in 0..3 {
        g.step(FrameInput::NONE);
    }
    let x = w(&g, LEVEL_WIDTH) - 0x200;
    g.poke(FROG + f::X, &x.to_be_bytes());
    for n in 0..740 {
        g.step(FrameInput::joy(if n < 700 { Joy::RIGHT } else { Joy::NONE }));
    }
    g
}

#[test]
fn the_frog_at_w1_1_s_right_end_is_drawn_in_the_app_s_right_strip() {
    let mut app = App::launch(LaunchOpts {
        max_frames: FRAMES,
        timeout_s: 120,
        scale: Some(2),
        ..Default::default()
    });
    let data = cache_data(&app.home);
    let build = our_build(&mut app, &data);
    let g = at_right_end(&data);
    assert!(g.scene().is_level());
    std::fs::write(app.home.join("states/slot-1.sfstate"), g.save(&build, &sf_assets::cache_hash(&data))).unwrap();
    std::fs::write(app.home.join("settings.toml"), "widescreen = true\n").unwrap();
    let c = app.run_script(&format!("5: menu pause\n{LOAD}: menu load-state-1\n{SHOT}: shot\n"));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("superfrog: loaded state 1 at game frame "), "{}", c.stderr);

    let want = Image::of_picture(&g.render(WIDE));
    for n in LOAD..=FRAMES {
        assert_eq!(c.frames[n as usize], want, "frame {n}: not the state's widescreen picture");
    }
    // Paused, the window's frame number stays the last stepped, before the load; what it presents
    // is the loaded picture.
    let (shot, info) = c.read_shot(SHOT);
    assert_eq!(assert_integer_scaled(&shot, &info, &want), 2 * info.scale_factor as u32);

    // The frog's sprite pixels in the right strip, drawn there: the frame differs from the same
    // picture without the sprites at each.
    let display = DisplayData::new(&data);
    let l = sf_core::render::strip_layers(g.state(), &display, WIDE);
    let bare = Image::of_picture(&l.picture);
    let width = l.picture.width as usize;
    assert_eq!((want.width, want.height), (l.picture.width as u32, l.picture.height as u32));
    let frog: Vec<(u32, u32)> = l
        .sprites
        .iter()
        .enumerate()
        .filter(|&(at, v)| v.is_some() && at % width >= width - N as usize)
        .map(|(at, _)| ((at % width) as u32, (at / width) as u32))
        .collect();
    assert!(frog.len() > 20, "{} frog pixels in the right strip", frog.len());
    let shown = frog.iter().filter(|&&(x, y)| want.rgb(x, y) != bare.rgb(x, y)).count();
    assert!(shown * 2 > frog.len(), "{shown} of {} frog pixels drawn over the strip", frog.len());
}
