//! T29, the app drill: what a player would try that no feature task listed, driven through the
//! app's script layer and checked against the app's own frames or the engine run headless.

mod common;

use common::{cache_data, not_black};
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, Start};
use sf_e2e::{App, Captured, Image, LaunchOpts, read_wav};

fn opts(max_frames: u64) -> LaunchOpts {
    LaunchOpts {
        max_frames,
        timeout_s: 300,
        ..Default::default()
    }
}

/// The steps after which the app took the command `id`, in order.
fn after_steps(c: &Captured, id: &str) -> Vec<u64> {
    let tag = format!("superfrog: menu {id} after step ");
    c.stderr.lines().filter_map(|l| l.strip_prefix(&tag)?.trim().parse().ok()).collect()
}

/// The frames move: some frame in `range` differs from the one before it.
fn moves(frames: &[Image], range: std::ops::Range<usize>) -> bool {
    range.into_iter().any(|n| frames[n] != frames[n - 1])
}

/// This build's name, as the app gives it when it refuses a state from another build.
fn our_build(app: &mut App, data: &sf_core::GameData) -> String {
    let hash = sf_assets::cache_hash(data);
    let other = Game::new(data, Start::Level("W1-1".parse().unwrap()));
    std::fs::create_dir_all(app.home.join("states")).unwrap();
    std::fs::write(app.home.join("states/slot-5.sfstate"), other.save("not this build", &hash)).unwrap();
    let c = app.run_script("2: menu load state 5\n");
    std::fs::remove_file(app.home.join("states/slot-5.sfstate")).unwrap();
    let line = c.prompts.lines().next().unwrap_or_else(|| panic!("no refusal: {}", c.stderr));
    let ours = line.split("; this is ").nth(1).and_then(|s| s.strip_suffix(". It was not loaded."));
    ours.unwrap_or_else(|| panic!("{line}")).to_string()
}

/// W1-1 from the title (fire at 100, 200 and 250), saved at step 700 and played on to 1000, then
/// WW-1 played from a level file exported from the disks, and the W1-1 state loaded at step 1300,
/// the witch on screen. The game goes on from W1-1's step 700, as it did the first time.
#[test]
fn a_state_loaded_during_the_witch_fight_goes_on_from_the_state() {
    let mut app = App::launch(opts(1600));
    let file = app.home.join("WW-1.sflevel");
    let script = format!(
        "save {f}\nopen {f}\n100: press Space\n200: press Space\n250: press Space\n\
         700: menu save state 1\n990: menu export WW-1\n1000: menu play-level-file\n\
         1300: menu load state 1\n",
        f = file.display()
    );
    let c = app.run_script(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(after_steps(&c, "load-state-1"), [1299]);
    assert!(c.stderr.contains("playing WW-1 from"), "{}", c.stderr);
    // The witch fight was on: WW-1's frames move, and are not W1-1's.
    assert!(moves(&c.frames, 1150..1300));
    assert!(c.frames[1150..1300].iter().all(|f| *f != c.frames[700]));
    for n in 1300..1600 {
        assert_eq!(c.frames[n], c.frames[n - 600], "frame {n} is not W1-1's step {} again", n - 600);
    }
    assert!(moves(&c.frames, 1300..1600) && c.frames[1300..].iter().any(not_black));
}

/// The slot machine (entered as W1-1's tally enters it, saved headless as a state with this
/// build), loaded, its reels spun with fire, the game paused mid-spin through the menu and resumed:
/// the picture and sound hold through the pause, and the spin then goes on where it was, as the
/// same run without the pause.
#[test]
fn pause_during_a_slot_spin_holds_the_reels_and_the_sound_then_spins_on() {
    let mut app = App::launch(opts(300));
    let data = cache_data(&app.home);
    let build = our_build(&mut app, &data);
    let mut g = Game::new(&data, Start::Level("W1-1".parse().unwrap()));
    g.enter_slot(3);
    for _ in 0..30 {
        g.step(FrameInput::NONE);
    }
    std::fs::write(app.home.join("states/slot-1.sfstate"), g.save(&build, &sf_assets::cache_hash(&data))).unwrap();
    // Headless, fire on the state's step 50 spins: a spin, and how it goes on.
    let mut h = g.clone();
    let spun: Vec<_> = (1..=100u64)
        .map(|n| h.step(if n == 50 { FrameInput::joy(Joy::FIRE) } else { FrameInput::NONE }).picture)
        .collect();
    assert!(spun[55..95].windows(2).any(|w| w[0] != w[1]), "the reels do not move after fire");

    let spin = "10: menu load state 1\n59: press Space\n";
    let c = app.run_script(&format!("{spin}70: menu pause\n120: menu pause\n"));
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(after_steps(&c, "pause"), [69, 119]);
    let mut plain = App::launch(opts(250));
    std::fs::create_dir_all(plain.home.join("states")).unwrap();
    std::fs::copy(app.home.join("states/slot-1.sfstate"), plain.home.join("states/slot-1.sfstate")).unwrap();
    let b = plain.run_script(spin);
    assert!(b.exit.success(), "{}", b.stderr);
    // The app's steps from the load are the headless state's: step 10 + k is its step k + 1.
    for n in 10..70 {
        assert_eq!(c.frames[n], Image::of_picture(&spun[n - 10]), "frame {n} is not the state's");
        assert_eq!(c.frames[n], b.frames[n], "frame {n}, before the pause");
    }
    for n in 70..120 {
        assert_eq!(c.frames[n], b.frames[69], "frame {n}: paused mid-spin, the reels stay");
    }
    for n in 120..=300 {
        assert_eq!(c.frames[n], b.frames[n - 50], "frame {n}: the spin goes on where it was");
    }
    let (ra, sa) = read_wav(&c.audio_wav);
    let (rb, sb) = read_wav(&b.audio_wav);
    assert_eq!(ra, rb);
    assert!(sa == sb, "the paused run's sound is not the unpaused run's ({} vs {} samples)", sa.len(), sb.len());
    assert!(sa.iter().any(|s| s[0] != 0 || s[1] != 0), "the slot machine is silent");
}

/// A window macOS will not draw (covered, minimised, the screen locked) stopped `present` before
/// its submit, so every frame's upload stayed staged: 14 GB of GPU memory over the long-run
/// drill's 50 000 frames (T29). Measured over 1 500 frames: peak 604 MB without the fix, 150 MB
/// with it, as an unoccluded run.
#[test]
fn a_window_that_cannot_be_drawn_does_not_gather_gpu_memory() {
    let mut app = App::launch(opts(1500));
    let c = app.run_script("1: occlude on\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let text = std::fs::read_to_string(c.dir.join("memory.txt")).expect("memory.txt");
    let peak: u64 = text
        .lines()
        .find_map(|l| l.strip_prefix("peak ")?.parse().ok())
        .unwrap_or_else(|| panic!("{text}"));
    assert!(peak < 300 << 20, "peak footprint {} MB over 1 500 occluded frames", peak >> 20);
}
