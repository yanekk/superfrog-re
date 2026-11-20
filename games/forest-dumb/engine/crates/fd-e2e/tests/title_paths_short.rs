//! Title plan T10, the short run (DESIGN §2.2, §2.6): the title's rows and one code in the real app
//! by key presses, the app's state the headless run's every step (`tests/title/mod.rs`). Down and
//! Up through START, KODY and KONIEC with the drawn screen equal to headless every step; then KODY
//! with 2424 typed on the digit keys and START: world 3's load, its SCENA screen and play. About
//! 1 000 steps. `title_paths.rs` has the other codes, Esc and game over.
//!
//! Needs the owner's disks and the generated tree (`translate.py --build`), and fails naming the
//! command without it.

mod common;
mod title;

use amiga_input::{FrameInput, Joy};
use title::{Run, TITLE};

/// Down and Up as `title-rows.txt` presses them on the original (docs/16, "The rows"): a row moves
/// in the step the stick is let go, one row a release however long it was held, and wraps both
/// ways. The picture is the headless run's every step, and the cursor's row is drawn.
#[test]
fn the_rows_move_on_each_release_and_are_drawn_as_headless() {
    let mut r = Run::to_the_title();
    r.keep_pictures(true);
    let (down, up) = (FrameInput::joy(Joy::DOWN), FrameInput::joy(Joy::UP));
    // (input, steps held, steps after, the row once let go)
    let moves = [
        (down, 10, 20, 1),
        (down, 30, 10, 2),
        (down, 2, 18, 0),
        (up, 10, 10, 2),
        (up, 10, 10, 1),
        (up, 10, 10, 0),
    ];
    let mut released = Vec::new();
    for (input, held, after, row) in moves {
        let from = r.steps();
        r.hold(input, held);
        assert!(
            r.changes("title_cursor", from + 1).is_empty(),
            "held from step {}: the row moves only on the release",
            from + 1
        );
        r.hold(FrameInput::NONE, after);
        released.push((r.steps() - after + 1, row));
    }
    assert_eq!(
        r.changes("title_cursor", TITLE),
        released,
        "(step, row) the cursor moved to"
    );

    let (_app, c) = r.in_the_app(true, 120);
    title::drawn_alike(&c, &r, TITLE + 1, r.steps());
    // The rows are drawn: START, KODY and KONIEC highlighted differ on screen.
    let shown: Vec<_> = released.iter().map(|&(n, _)| c.read_frame(n)).collect();
    assert!(shown[0] != shown[1] && shown[1] != shown[2] && shown[0] != shown[2]);
}

/// KODY with world 3's code (docs/16, "The codes": 2424 sets `world_index` 2), typed on the digit
/// keys, then START: the load, the SCENA screen taking fire and 100 steps of world 3's play.
#[test]
fn world_3s_code_typed_on_kody_then_start_plays_world_3_as_headless() {
    let mut r = Run::to_the_title();
    assert_eq!(
        r.type_code([2, 4, 2, 4]),
        0x0204_0204,
        "code_digits, first digit highest"
    );
    let play = r.start_and_play();
    assert_eq!(
        (r.at(play, "world_index"), r.at(play, "cheat_flag")),
        (2, 0),
        "world 3, no cheat, at play's first pass (step {play})"
    );
    r.hold(FrameInput::NONE, 100);
    assert_eq!(r.now("world_index"), 2, "still in world 3");
    let (_app, c) = r.in_the_app(false, 120);
    assert!(!c.stderr.contains("level 1 begins"), "{}", c.stderr);
}
