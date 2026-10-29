//! real-widescreen T11, the widescreen drill (DESIGN §2.7, §2.8): what the drill drove in the app
//! that no feature task's test drives. The drill found no defect with one right answer; this file
//! pins the switches it made where a person would make them, and the paused picture the owner
//! chose to keep (2026-10-27, FINDINGS).

mod common;

use common::cache_data;
use sf_core::feed::{frame_hashes, parse_hashes};
use sf_core::input::{FrameInput, Joy};
use sf_core::recording::LevelSetup;
use sf_core::{DisplayData, Game, Rng, View};
use sf_e2e::{App, LaunchOpts};

const WIDE: View = View {
    extra_columns_each_side: 65,
};

/// W3-1 by Game ▸ Start Level… after step 4 (seed 0 0), recording: widescreen switched twice in
/// the level's fade-in (over by its step 81), once more before the right arrow scrolls the camera
/// and off mid-scroll; paused over app steps 400..470 with three switches inside the pause (net
/// on); then off, on, off again. Unpaused, every step is the headless game with the net mode
/// applied before it; paused, the window keeps the last picture, centred, whatever the window's
/// new shape (owner, 2026-10-27). The recording holds one toggle for the pause's three, and
/// `bug replay` plays the game the app played.
#[test]
fn switches_in_the_fade_in_mid_scroll_and_while_paused_play_as_the_headless_game() {
    const START: u64 = 4;
    const STOP: u64 = 1000;
    let pause = 400..470u64;
    let switches = [30u64, 60, 70, 200, 420, 440, 450, 600, 650, 700];
    let walk = 104..304u64;
    let bugs = std::env::temp_dir().join(format!("sf-e2e-wide-drill-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bugs);
    let mut app = App::launch(LaunchOpts {
        max_frames: STOP + 20,
        timeout_s: 600,
        scale: Some(2),
        extra_args: vec!["--capture-hashes".into(), "--bugs-dir".into(), bugs.display().to_string()],
        ..Default::default()
    });
    let mut script = format!(
        "note w\n{}: start-level W3-1 record\n{}: down ArrowRight\n{}: up ArrowRight\n{}: menu pause\n{}: menu pause\n",
        START + 1,
        walk.start,
        walk.end,
        pause.start,
        pause.end
    );
    for s in switches {
        script += &format!("{s}: menu widescreen\n");
    }
    script += &format!("415: shot\n425: shot\n{STOP}: menu stop-recording\n");
    let c = app.run_script_unread(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains(&format!("superfrog: started W3-1 after step {START}, recording\n")), "{}", c.stderr);

    // Paused: switched on at 420, the window is wider and shows the picture of step 399, centred.
    let (_, before) = c.read_shot(415);
    let (_, after) = c.read_shot(425);
    assert_eq!((before.frame, after.frame), (399, 399), "paused, no step is drawn");
    assert!(after.surface.0 > before.surface.0, "the window widens: {before:?} {after:?}");
    assert_eq!(after.viewport.2, before.viewport.2, "the paused picture keeps its size: {after:?}");
    assert_eq!(2 * after.viewport.0 + after.viewport.2, after.surface.0, "centred: {after:?}");

    let data = cache_data(&app.home);
    let display = std::sync::Arc::new(DisplayData::new(&data));
    let mut g = Game::start_level(
        display,
        &LevelSetup {
            level: "W3-1".parse().unwrap(),
            ..LevelSetup::default()
        },
        Rng::seeded(0, 0),
    );
    let app_hashes = parse_hashes(&std::fs::read_to_string(c.dir.join("hashes.txt")).unwrap()).unwrap();
    let (mut wide, mut played, mut fade_wide) = (false, Vec::new(), 0);
    for n in START + 1..STOP {
        wide ^= switches.contains(&n);
        if pause.contains(&n) {
            continue;
        }
        if g.widescreen() != wide {
            g.set_widescreen(wide);
            g.set_view(if wide { WIDE } else { View::default() });
        }
        let input = if walk.contains(&n) { FrameInput::joy(Joy::RIGHT) } else { FrameInput::NONE };
        let p = g.step(input).picture;
        let h = frame_hashes(&g, &p);
        assert_eq!(app_hashes.get(n as usize), Some(&h), "app step {n} is not the headless game's");
        played.push(h);
        fade_wide += (wide && n < START + 81) as u32;
    }
    assert!(fade_wide > 0 && g.scene().is_level(), "a switch landed in the fade-in, and W3-1 still plays");

    let zip: Vec<_> = std::fs::read_dir(&bugs).unwrap().flatten().map(|e| e.path()).collect();
    assert_eq!(zip.len(), 1, "{}", c.prompts);
    let (rec, png) = sf_conform::bug::open_zip(&std::fs::read(&zip[0]).unwrap()).unwrap();
    // A toggle before each unpaused switch, one for the pause's three at its first step after.
    assert_eq!(rec.toggles, [25, 55, 65, 195, 395, 525, 575, 625]);
    let mut replayed = Vec::new();
    let r = sf_conform::bug::replay(&rec, |_, game, pic| replayed.push(frame_hashes(game, pic)));
    assert_eq!(replayed, played, "`bug replay` plays the game the app played");
    assert_eq!(sf_conform::bug::screenshot_differs(&png, &r.last), Ok(0));
    let _ = std::fs::remove_dir_all(&bugs);
}
