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

/// The slot machine's loads (T213 review): `music_fade_wait`'s `music_stop`, made inside the load
/// the engine does at once, reaches the mixer (`FrameOut::unheard`). Thrown away, channels 0 and 2
/// played the level's music on for 15 frames after the load where the original is silent.
#[test]
fn the_music_stopped_inside_the_slot_machines_load_is_silent_after_it() {
    let case = Case::load(&cases_dir().join("slot-load.toml")).unwrap();
    let v = channels::check(&case, channels::OWNER).unwrap_or_else(|e| panic!("{e}"));
    assert!(v.runs.is_empty(), "{:?}", v.runs);
    assert_eq!(v.differing, [0; 4], "{v:?}");
}

/// The slot machine's effects (T213 review): `sfx_play`'s writes from its raster wait's end are
/// four lines after its channel's DMACON off, so Paula lets the channel go idle and restarts the
/// effect. Made at one instant, the reels' effects were lost on channel 2: runs of 6 and 7 frames.
/// The runs of 3 left then (a slow effect a few lines off) pass since T215 judges each frame against
/// the original's range over it and its neighbours.
#[test]
fn the_slot_machines_effects_restart_as_on_the_original() {
    let case = Case::load(&cases_dir().join("slot-gamble.toml")).unwrap();
    let v = channels::check(&case, channels::OWNER).unwrap_or_else(|e| panic!("{e}"));
    assert!(v.runs.is_empty(), "{:?}", v.runs);
}

/// The story's loads (T214): the intro starts a scene's music just before it loads the scene, and
/// the original's handler goes on playing it on every blank of the load. The engine run alone is
/// fed those blanks and runs their sound at its instant load, the frame's writes before it being
/// the frame the load began in. Before T214 none ran (the runner counted the game's handler's
/// only): the music came out of each load behind the original's, a note lost (`story-scene4-sound`
/// after 2286 and 5221). Here every channel's every frame is the original's, unshifted. About 75 s.
#[test]
#[ignore = "slow: about 75 s"]
fn the_storys_music_plays_on_through_its_loads_as_the_original_does() {
    let case = Case::load(&cases_dir().join("story-scene1-out.toml")).unwrap();
    let v = channels::check(&case, channels::OWNER).unwrap_or_else(|e| panic!("{e}"));
    assert!(v.runs.is_empty(), "{:?}", v.runs);
    assert_eq!(v.differing, [0; 4], "{v:?}");
    assert_eq!(v.shifted, 0, "{v:?}");
}

/// The whole game (T215), power-on to `end_seq_load`, judged with the owner's tolerance: each frame
/// against the original's range over it and its neighbours, so a slow sound a few lines off passes.
/// Only the stretch `game-whole.toml` lists as known (WB-1's channel 3, T216) may differ past it.
/// The feed from the trace cache: 7 min (434 s), 40 uncached.
#[test]
#[ignore]
fn the_whole_games_channels_sound_as_the_original_does() {
    let case = Case::load(&cases_dir().join("game-whole.toml")).unwrap();
    let v = channels::check(&case, channels::OWNER).unwrap_or_else(|e| panic!("{e}"));
    println!("{v:?}");
    assert!(v.runs.is_empty(), "{:?}", v.runs);
    assert!(v.known.iter().all(|(_, task)| task == "T216"), "{:?}", v.known);
}
