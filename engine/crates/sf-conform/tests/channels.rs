//! Each of Paula's channels against the original's (T213, `sf_conform::channels`): the original's
//! levels recorded with a case's pacing feed, the engine's from the feed replayed through the app's
//! mixer. Needs what the cases need: the oracle, the four disks and Kickstart 1.3.

use sf_conform::{Case, cases_dir, channels};

/// The title's first 600 frames of play. On frame 516 the music starts a note on channels 0 and 1
/// while each is partway through a long sample (at volume 0). Before T213 the engine's DMACON off
/// and on for them landed at one instant, Paula never let the channels go idle, and they played
/// the old sample on at the new volume: levels 2 399 and 1 586 against the original's 686, for 11
/// frames. Now the DMA stage is where the original's is, eight lines after the replayer.
#[test]
fn the_titles_retriggered_notes_restart_as_on_the_original() {
    let mut case = Case::load(&cases_dir().join("front-attract.toml")).unwrap();
    case.frames = 600;
    let (_, theirs, ours) = channels::levels(&case).unwrap_or_else(|e| panic!("{e}"));
    for f in 516..=526 {
        let (t, o) = (theirs[f - 1].expect("a frame of one video frame"), ours[f - 1]);
        for c in 0..2 {
            assert!(t[c] > 0, "frame {f}: the original's channel {c} sounds");
            assert!(
                t[c].abs_diff(o[c]) * 10 <= t[c],
                "frame {f} channel {c}: original {}, engine {}",
                t[c],
                o[c]
            );
        }
    }
    let v = channels::judge(&theirs, &ours, channels::OWNER);
    assert!(v.runs.is_empty(), "{:?}", v.runs);
    assert_eq!(v.shifted, 0, "the title's sound is the original's, not late");
}
