//! T26: the level's end through the real app (DESIGN §2.5). HELP with the title's cheat code on
//! gets there without collecting W1-1's items: the script's `cheat` line stands
//! in for the code, and H is bound to the Amiga's HELP in `settings.toml`, as Game ▸ Controls…
//! saves it. `main_loop`'s check after pass 64 ends the level, "LEVEL 1 COMPLETE" shows, fire held
//! from before and other keys are ignored, and a fresh fire press restarts the level with lives full
//! and score zero. The app's state hash equals the headless game's after every step, the frames are
//! the headless game's pictures, and the message is whole inside the picture and the picture whole
//! inside the window. At the default scale and at scale 1, with a `shot` of the window.

mod common;

use amiga_input::FrameInput;
use fd_core::render::message;
use fd_core::{Event, Scene};
use fd_e2e::{App, Captured, Image, LaunchOpts};

/// The cheat from step 1, fire on Left Option held over the level's end (steps 60 to 66), H (HELP)
/// tapped in step 63, read by pass 64 (any earlier and the next pass's `char_object_cells` clears
/// `level_exit_touched`, T22), Up and an arrow on the message, a shot of it, fire pressed in step 90, then
/// right held.
const SCRIPT: &str = "1: cheat\n60: down AltLeft\n63: press KeyH\n66: up AltLeft\n\
                      70: press Space\n72: press ArrowLeft\n75: shot\n90: down AltLeft\n\
                      92: up AltLeft\n100: down ArrowRight\n";
const FRAMES: u64 = 140;

/// The box of the white pixels of `img`: (left, top, right, bottom), exclusive at the end.
fn white_box(img: &Image) -> Option<(u32, u32, u32, u32)> {
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for (i, p) in img.rgba.chunks(4).enumerate() {
        if p[..3] != [255, 255, 255] {
            continue;
        }
        let (x, y) = (i as u32 % img.width, i as u32 / img.width);
        b = Some(match b {
            None => (x, y, x + 1, y + 1),
            Some((l, t, r, bt)) => (l.min(x), t.min(y), r.max(x + 1), bt.max(y + 1)),
        });
    }
    b
}

/// The app's capture is removed with `app`, so everything is read before it goes.
fn run(scale: Option<u32>) {
    let mut app = App::launch(LaunchOpts {
        max_frames: FRAMES,
        scale,
        ..Default::default()
    });
    std::fs::write(
        app.home.join("settings.toml"),
        "[keys]\n\"key $5f\" = [\"KeyH\"]\n",
    )
    .unwrap();
    let c = app.run_script(SCRIPT);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr.contains("fd-app: level 1 complete\n"),
        "{}",
        c.stderr
    );
    assert!(
        c.stderr
            .contains("fd-app: fire on the message; W1-1 restarts, lives full, score zero\n"),
        "{}",
        c.stderr
    );

    let fed = common::inputs(&c);
    let hashes = common::hashes(&c);
    let mut g = common::headless();
    let (mut ended, mut restarted) = (None, None);
    for n in 1..=FRAMES {
        if n == 1 {
            g.set_cheat(true);
        }
        g.step(fed.get(&n).copied().unwrap_or(FrameInput::NONE));
        for e in g.take_events() {
            match e {
                Event::LevelEnd => ended = Some(n),
                Event::Restart => restarted = Some(n),
                _ => {}
            }
        }
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "the state after step {n}"
        );
        if n == 75 {
            assert!(matches!(g.scene(), Scene::Complete(_)));
            assert_eq!(c.read_frame(n), common::picture(&g), "the message");
        }
    }
    // Step n leaves pass n + 1: the check after pass 64 (step 63) ends the level in step 64; fire
    // held over it does not count, the press in step 90 restarts.
    assert_eq!((ended, restarted), (Some(64), Some(90)), "{}", c.stderr);
    assert_eq!(*g.scene(), Scene::Level);
    assert_eq!(
        (g.lives(), g.state().score, g.state().world_index),
        (32, 0, 0)
    );
    assert_eq!(c.read_frame(FRAMES), common::picture(&g), "the last frame");
    assert_fits(&c);
}

/// The message whole in the picture with room around it, and the picture whole in the window.
fn assert_fits(c: &Captured) {
    let pic = c.read_frame(75);
    let (l, t, r, b) = white_box(&pic).expect("the message is drawn");
    let (tw, th) = message::text_size();
    assert_eq!(
        (r - l, b - t),
        (tw as u32, th as u32),
        "every letter drawn whole"
    );
    assert!(
        l >= 8 && t >= 8 && r + 8 <= pic.width && b + 8 <= pic.height,
        "the message {:?} is inside the {}×{} picture with a margin",
        (l, t, r, b),
        pic.width,
        pic.height
    );
    let (img, shot) = c.read_shot(75);
    let (x, y, w, h) = shot.viewport;
    assert!(
        w % pic.width == 0 && w / pic.width == h / pic.height,
        "{shot:?}: a whole multiple of the picture"
    );
    assert!(
        x + w <= shot.surface.0 && y + h <= shot.surface.1,
        "{shot:?}: the picture is cut off"
    );
    // The window shows the message: its white box, scaled, inside the viewport.
    let k = w / pic.width;
    let (sl, st, sr, sb) = white_box(&img).expect("the window shows the message");
    assert_eq!(
        (sl, st, sr - sl, sb - st),
        (x + l * k, y + t * k, tw as u32 * k, th as u32 * k),
        "{shot:?}"
    );
}

#[test]
fn the_level_end_in_the_app_shows_level_1_complete_and_fire_restarts_at_the_default_scale() {
    run(None);
}

#[test]
fn the_level_end_in_the_app_shows_level_1_complete_and_fire_restarts_at_scale_1() {
    run(Some(1));
}
