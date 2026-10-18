//! T211: the real app plays the whole game (`conform/cases/game-whole.toml`) fed the original's
//! pacing exactly as `sf-conform`'s engine run alone is fed it (its pacing feed, `sf_core::feed`),
//! and its frames are that run's, frame for frame: each frame's picture and state hashes, the
//! app's `--capture-hashes` against the ones `sf-conform` wrote as it ran alone. Since that run
//! meets the original's milestones (DESIGN §2.1 check 4, T209), so does the app's.
//!
//! The feed comes from the trace cache, or is made there first by running the case on the oracle
//! (`sf_conform::feed::feed`, as `sf-conform feed <case.toml>` does): about 25 s for the short
//! stretch, about 25 min for the whole game. The app then runs at the PAL rate: 30 s for the
//! stretch, about 75 min for the whole game, which is ignored unless asked (engine/crates/sf-e2e/README.md).

use std::process::{Command, Stdio};

use sf_conform::Case;
use sf_conform::feed::FeedFiles;
use sf_core::feed::{Feed, parse_hashes};
use sf_core::{Game, Start, View};
use sf_e2e::{App, LaunchOpts, binary};

/// `game-whole`'s first `frames` frames of play (all of them for `None`), and its feed.
fn case_and_feed(frames: Option<u32>) -> (Case, FeedFiles) {
    let mut case = Case::load(&sf_conform::cases_dir().join("game-whole.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Some(n) = frames {
        case.frames = n;
    }
    let f = sf_conform::feed::feed(&case).unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = &f.alone {
        // The feed is what the engine was fed all the same; the milestones are T209's to keep.
        eprintln!("note: {}'s engine run alone did not meet the original's milestones: {d}", case.name);
    }
    (case, f)
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The first frame whose hashes differ between `ours` and `theirs`, and which differ.
fn first_difference(ours: &[(u64, u64)], theirs: &[(u64, u64)]) -> Option<String> {
    let at = ours.iter().zip(theirs).position(|(a, b)| a != b);
    match at {
        Some(n) => {
            let what = match (ours[n].0 == theirs[n].0, ours[n].1 == theirs[n].1) {
                (false, false) => "picture and state",
                (false, true) => "picture",
                _ => "state",
            };
            Some(format!("frame {n}: the {what} differ"))
        }
        None if ours.len() != theirs.len() => Some(format!(
            "the app has {} frames' hashes, the headless run {}",
            ours.len(),
            theirs.len()
        )),
        None => None,
    }
}

/// The app, paced by `game-whole`'s feed for `frames` frames of play, gives the headless run's
/// hashes on every frame.
fn paced_app_is_the_run_alone(frames: Option<u32>, timeout_s: u64) {
    let (case, f) = case_and_feed(frames);
    let theirs = parse_hashes(&read(&f.hashes)).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(theirs.len(), f.frames + 1, "frame 0 and one a frame of play");
    let started = std::time::Instant::now();
    let mut app = App::launch(LaunchOpts {
        max_frames: f.frames as u64,
        timeout_s,
        scale: Some(1),
        extra_args: vec![
            "--pacing".into(),
            f.feed.to_string_lossy().into_owned(),
            "--capture-hashes".into(),
        ],
        ..Default::default()
    });
    let c = app.run_script_unread("# Every step is the pacing feed's (--pacing).\n");
    assert!(c.exit.success(), "{}: the app exited {:?}: {}", case.name, c.exit, c.stderr);
    let ours = parse_hashes(&read(&c.dir.join("hashes.txt"))).unwrap_or_else(|e| panic!("{e}"));
    if let Some(d) = first_difference(&ours, &theirs) {
        panic!("{}: the app, paced as the engine run alone, parts from it at {d}", case.name);
    }
    eprintln!(
        "{}: {} frames of play, the app's equal the headless run's; the app ran {:.0} s",
        case.name,
        f.frames,
        started.elapsed().as_secs_f64()
    );
}

/// The feed alone replays the run: the engine headless, stepped by `sf_core::feed` from the feed
/// and nothing else, gives the hashes `sf-conform` wrote as it ran alone. So what the app is fed
/// is all the run alone was fed.
#[test]
fn the_feed_replays_the_run_alone_headless() {
    let (_, f) = case_and_feed(Some(STRETCH));
    let feed = Feed::parse(&read(&f.feed)).unwrap_or_else(|e| panic!("{e}"));
    let theirs = parse_hashes(&read(&f.hashes)).unwrap_or_else(|e| panic!("{e}"));
    let mut g = Game::with_display(sf_conform::display_data(), Start::PowerOn { seed: feed.seed });
    let mut ours = vec![sf_core::feed::frame_hashes(&g, &g.render(View::default()))];
    for fr in &feed.frames {
        let pic = fr.play(&mut g).pop().expect("a frame steps").picture;
        ours.push(sf_core::feed::frame_hashes(&g, &pic));
    }
    assert_eq!(first_difference(&ours, &theirs), None);
    // The stretch has what a feed carries: loads, and frames under a loading screen.
    assert!(feed.frames.iter().filter(|f| f.load.is_some()).count() >= 3);
    assert!(feed.frames.iter().any(|f| f.then.len() > 100));
}

/// Frames of play in the test block's stretch: power-on, the boot's loads and the loading
/// screens the original times itself (757 and 193 frames under them), into the title: ~30 s of
/// the app at the PAL rate.
const STRETCH: u32 = 1500;

#[test]
fn the_app_paced_from_power_on_through_the_boot_loads_is_the_run_alone() {
    paced_app_is_the_run_alone(Some(STRETCH), 120);
}

/// The whole game, 224 082 frames of play: about 75 min at the PAL rate (plus about 25 min for
/// the feed when it is not in the cache).
#[test]
#[ignore = "slow: about 75 min (the app at the PAL rate), plus 25 min for an uncached feed"]
fn the_app_paced_through_the_whole_game_is_the_run_alone() {
    paced_app_is_the_run_alone(None, 5 * 3600);
}

/// Normal play never reads a feed: without `--script` the app refuses `--pacing` before it opens
/// anything (DESIGN §2.5).
#[test]
fn without_a_script_the_app_reads_no_feed() {
    let home = std::env::temp_dir().join(format!("sf-e2e-nofeed-{}", std::process::id()));
    let out = Command::new(binary())
        .args(["--home"])
        .arg(&home)
        .args(["--pacing", "/nonexistent/feed", "--max-frames", "1", "--timeout", "10"])
        .stdin(Stdio::null())
        .output()
        .expect("launching the app");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("--pacing") && stderr.contains("--script"), "{stderr}");
    assert!(!stderr.contains("/nonexistent/feed"), "the feed was looked for: {stderr}");
    assert!(!home.exists(), "the app made its home: {}", home.display());
}
