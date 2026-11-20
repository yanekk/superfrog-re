//! T13: the app plays level 1 (DESIGN §2.3, §2.7). The real `fd-app`, launched on the owner's
//! disks, scripted through its real input path (keys enter where the window's are translated),
//! is compared with the headless `Game` fed the same input: its first frame, and its state hash
//! after every step. The default keys are the owner's: Up on Space.

mod common;

use common::{RIGHT, UP};
use fd_e2e::{App, LaunchOpts};

#[test]
fn the_first_frame_is_the_headless_game_s() {
    let mut app = App::launch(LaunchOpts {
        max_frames: 1,
        ..Default::default()
    });
    let c = app.run_script("");
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(c.stderr.contains("fd-app: playing W1-1"), "{}", c.stderr);
    let g = common::headless();
    assert_eq!(
        c.frames[0],
        common::picture(&g),
        "frame 0 is the level's start"
    );
    assert!(
        c.frames[0].rgba.chunks(4).any(|p| p[..3] != [0, 0, 0]),
        "the start is not black"
    );
    assert_eq!(common::hashes(&c)[0], common::hash(&g));
}

#[test]
fn up_and_right_by_key_play_as_the_headless_game_does_every_frame() {
    const FRAMES: u64 = 240;
    let mut app = App::launch(LaunchOpts {
        max_frames: FRAMES,
        ..Default::default()
    });
    let c = app.run_script_unread(
        "20: down ArrowRight\n120: down Space\n130: up Space\n200: up ArrowRight\n210: press Space\n",
    );
    assert!(c.exit.success(), "{}", c.stderr);
    // The keys reached the game as the owner's joystick.
    assert_eq!(common::joy(&c, 19, 21), [0, RIGHT, RIGHT]);
    assert_eq!(
        common::joy(&c, 119, 131),
        [&[RIGHT][..], &[RIGHT | UP; 10], &[RIGHT, RIGHT]].concat()
    );
    assert_eq!(
        common::joy(&c, 199, 211),
        [&[RIGHT][..], &[0; 10], &[UP], &[0]].concat()
    );

    let hashes = common::hashes(&c);
    assert_eq!(
        hashes.len() as u64,
        FRAMES + 1,
        "a hash for the start and every step"
    );
    let fed = common::inputs(&c);
    let mut g = common::headless();
    let x0 = g.state().char_x;
    assert_eq!(hashes[0], common::hash(&g), "the start");
    for n in 1..=FRAMES {
        g.step(
            fed.get(&n)
                .copied()
                .unwrap_or(amiga_input::FrameInput::NONE),
        );
        g.take_events();
        assert_eq!(
            hashes[n as usize],
            common::hash(&g),
            "the state after step {n}"
        );
    }
    assert_ne!(g.state().char_x, x0, "the character moved right");
    assert_eq!(c.read_frame(FRAMES), common::picture(&g), "the last frame");
}
