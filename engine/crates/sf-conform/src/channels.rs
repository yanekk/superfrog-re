//! The channels' output, compared (T213): what each of Paula's four channels played, frame of play
//! by frame, the original's (vAmiga's DAC taps, [`sf_oracle::FrameCapture::taps`]) against the
//! app's mixer run on the engine's writes and chip RAM ([`amiga_paula::Paula::channel_taps`]).
//!
//! Check 3 compares sound events (where a sample starts, its length, period and volume), never what
//! a channel then plays: a note whose restart Paula missed (its channel turned off and on again
//! before it went idle) leaves the old sample playing on with every event right. This compares the
//! output itself.
//!
//! A channel's frame is reduced to its [`level`], how far its DAC swings about its mean, so that a
//! write a line or two from the original's moves it little and a lost or wrong note moves it far.
//! The original's levels are recorded with the pacing feed (`<key>.channels`, [`CHANNELS_HEADER`]),
//! one line a frame of play, as the original's mix is (`feed::AUDIO_HEADER`); the engine's come
//! from replaying the feed headless through the app's mixer ([`engine_levels`]). The levels and
//! their judge are `amiga_conform::channels`'; the files, the replay and the cases are Superfrog's.

use sf_core::feed::Feed;
use sf_core::{Game, Start};

use crate::{Alone, Case, SyncPoint, feed};
use amiga_paula::Paula;

// The levels and their judge moved to the toolkit (toolkit-extract T06); kept at this path for
// their callers.
pub use amiga_conform::channels::{
    Levels, OWNER, Original, Run, Tolerance, Verdict, judge, level, line, mixer_levels,
};

/// The first line of a channels file: then one line a frame of play, from frame 1, the four
/// channels' [`level`]s in the original's frame its step ended in, or `-` where that step ran two
/// video frames (the engine's two are not told apart from it).
pub const CHANNELS_HEADER: &str = "superfrog original channel levels 1";

/// A channels file's levels, frame of play 1 first (`None`: not compared).
pub fn parse(text: &str) -> Result<Vec<Option<Levels>>, String> {
    amiga_conform::channels::parse(text, CHANNELS_HEADER)
}

/// The engine's levels each frame of play: `feed` replayed headless from `start` (a power-on
/// seeded with the feed's seed) through the app's mixer (`Paula::booted`, a load's blanks a frame
/// each before the step after them, each frame's [`sf_core::FrameOut::mixed`], as `sf-app` mixes
/// them), the last video frame of each.
pub fn engine_levels(feed: &Feed, start: Start) -> Vec<Levels> {
    let mut g = Game::with_display(crate::display_data(), start);
    let mut paula = Paula::booted(48_000, sf_core::BOOT_DMACON);
    let mut sink = Vec::new();
    let mut out = Vec::with_capacity(feed.frames.len());
    for fr in &feed.frames {
        let outs = fr.play(&mut g);
        for blank in g.take_load_paula() {
            paula.frame(&blank, &g, &mut sink);
        }
        for o in &outs {
            paula.frame(&o.mixed(), &g, &mut sink);
        }
        sink.clear();
        out.push(mixer_levels(&paula));
    }
    out
}

/// `case` judged by [`levels`], its runs inside a known stretch moved to [`Verdict::known`].
pub fn check(case: &Case, tol: Tolerance) -> Result<Verdict, String> {
    let (_, theirs, ours) = levels(case)?;
    Ok(excuse(judge(&theirs, &ours, tol), case))
}

/// `v` with its runs lying wholly inside a stretch `case` lists as known
/// ([`Case::channels_known`]) moved to [`Verdict::known`].
pub fn excuse(mut v: Verdict, case: &Case) -> Verdict {
    for r in std::mem::take(&mut v.runs) {
        match case.channels_known(r.first, r.first + r.frames - 1) {
            Some(task) => v.known.push((r, task.to_string())),
            None => v.runs.push(r),
        }
    }
    v
}

/// `case`'s levels each frame of play, the original's and the engine's: the original's as
/// recorded with its pacing feed (from the trace cache, or by running it), the engine's from
/// replaying the feed. A case from any other start (the story's disk 3, a sync point in the whole
/// game) has no cached feed, which is a disk-1 power-on's: it is run and replayed here, from the
/// start its engine run alone stepped from. With the feed.
pub fn levels(case: &Case) -> Result<(Feed, Vec<Option<Levels>>, Vec<Levels>), String> {
    let (feed, theirs, start) = recorded(case)?;
    let ours = engine_levels(&feed, start);
    Ok((feed, theirs, ours))
}

/// What [`levels`] replays: `case`'s feed, the original's levels and the engine's start.
pub fn recorded(case: &Case) -> Result<(Feed, Vec<Option<Levels>>, Start), String> {
    Ok(match case.sync {
        SyncPoint::PowerOn | SyncPoint::PowerOnDf1 => {
            let f = feed::feed(case)?;
            let read = |p: &std::path::Path| {
                std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
            };
            let feed = Feed::parse(&read(&f.feed)?)?;
            let theirs = parse(&read(&f.channels)?)?;
            let seed = feed.seed;
            (feed, theirs, Start::PowerOn { seed })
        }
        _ => {
            let mut c = case.clone();
            c.alone = Alone::Only;
            let mut rec = feed::Recorded::default();
            let _ = crate::run_inner(&c, |_, _| {}, &mut |_, _, _| {}, None, false, Some(&mut rec), None);
            if rec.panicked {
                return Err(format!("{}: the engine run alone panicked", case.name));
            }
            let theirs = parse(&rec.channels)?;
            let start = rec.start.ok_or_else(|| format!("{}: no engine run alone", case.name))?;
            (Feed { engine: String::new(), seed: rec.seed, frames: rec.frames }, theirs, start)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: Tolerance = Tolerance { floor: 100, percent: 25, frames: 2, shift: 10, window: 25 };

    /// A tune: four channels at levels from 300 to 2 250 that never repeat in a row.
    fn tune(frames: usize) -> Vec<Levels> {
        let mut x = 1u32;
        (0..frames)
            .map(|_| {
                [0; 4].map(|_| {
                    x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    (300 + (x >> 16) % 14 * 150) as u16
                })
            })
            .collect()
    }

    /// A case's `[[exclude]] what = "channels"` stretch: a run wholly inside it is known, not
    /// failed; one reaching past it still fails.
    #[test]
    fn a_run_inside_a_known_stretch_is_reported_not_failed() {
        let base = "sync = \"power-on\"\nframes = 300\n[[exclude]]\nwhat = \"channels\"\n";
        let case = Case::parse("x", &format!("{base}frames = [140, 160]\nnote = \"n\"\ntask = \"T216\"\n"))
            .unwrap();
        let t = tune(300);
        let theirs: Vec<Option<Levels>> = t.iter().copied().map(Some).collect();
        let mut ours = t.clone();
        for l in &mut ours[145..150] {
            l[3] = 0;
        }
        for l in &mut ours[158..164] {
            l[1] = 0;
        }
        let v = excuse(judge(&theirs, &ours, TOL), &case);
        let known: Vec<_> = v.known.iter().map(|(r, t)| (r.channel, r.first, t.as_str())).collect();
        assert_eq!(known, [(3, 146, "T216")], "{v:?}");
        assert_eq!(v.runs.iter().map(|r| (r.channel, r.first)).collect::<Vec<_>>(), [(1, 159)]);
        // A stretch needs its frames.
        assert!(Case::parse("x", &format!("{base}note = \"n\"\ntask = \"T\"\n")).is_err());
    }

    #[test]
    fn channels_files_round_trip() {
        let text = format!("{CHANNELS_HEADER}\n{}{}", line(Some([1, 2, 3, 8192])), line(None));
        assert_eq!(parse(&text).unwrap(), vec![Some([1, 2, 3, 8192]), None]);
        assert!(parse("1 2 3 4\n").is_err());
        assert!(parse(&format!("{CHANNELS_HEADER}\n1 2 3\n")).is_err());
    }
}
