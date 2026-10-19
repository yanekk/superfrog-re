//! The channels' output, compared (T213): what each of Paula's four channels played, frame of play
//! by frame, the original's (vAmiga's DAC taps, [`sf_oracle::FrameCapture::taps`]) against the
//! app's mixer run on the engine's writes and chip RAM ([`sf_paula::Paula::channel_taps`]).
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
//! from replaying the feed headless through the app's mixer ([`engine_levels`]).

use sf_core::feed::Feed;
use sf_core::{Game, Start};

use crate::{Alone, Case, SyncPoint, feed};
use sf_oracle::{AudioTap, FrameCapture};
use sf_paula::{CLOCKS_PER_LINE, Paula};

/// The first line of a channels file: then one line a frame of play, from frame 1, the four
/// channels' [`level`]s in the original's frame its step ended in, or `-` where that step ran two
/// video frames (the engine's two are not told apart from it).
pub const CHANNELS_HEADER: &str = "superfrog original channel levels 1";

/// Four channels' levels.
pub type Levels = [u16; 4];

/// How far a channel's DAC (sample byte × volume, ±8 192) swings about its mean over a frame: the
/// mean distance from the mean. A held value, however far from zero, is 0: a stopped channel keeps
/// its last sample on the DAC (T30).
pub fn level(dac: &[i16]) -> u16 {
    if dac.is_empty() {
        return 0;
    }
    let n = dac.len() as i64;
    let sum: i64 = dac.iter().map(|&v| v as i64).sum();
    // Twice the distance from the mean, in units of 1/n, to stay in integers.
    let dev: i64 = dac.iter().map(|&v| (2 * n * v as i64 - 2 * sum).abs()).sum();
    (dev / (2 * n * n)) as u16
}

/// The four channels' levels over the mixer's last frame.
pub fn mixer_levels(p: &Paula) -> Levels {
    p.channel_taps().map(level)
}

/// The original's DAC at every clock, from the taps vAmiga reports at each change (as
/// `tests/paula_vs_oracle.rs` rebuilds it). A step's list runs to just after the next frame's
/// start, so a tap whose position goes back belongs, with those after it, to the next frame.
#[derive(Debug, Clone, Default)]
pub struct Original {
    dac: [i16; 4],
    carried: Vec<AudioTap>,
}

impl Original {
    /// The levels of the last video frame of a step that ran `frames` of them (1 or 2), the last
    /// `lines` long. Every step of the original must come here, or the DAC is stale.
    pub fn step(&mut self, c: &FrameCapture, frames: u32, lines: u32) -> Levels {
        let pos = |t: &AudioTap| t.vpos as u32 * CLOCKS_PER_LINE + t.hpos as u32;
        let all: Vec<AudioTap> = c.taps().collect();
        let mut rest = &all[..];
        let mut levels = [0; 4];
        for _ in 0..frames.max(1) {
            let wrap = (1..rest.len())
                .find(|&i| pos(&rest[i]) < pos(&rest[i - 1]))
                .unwrap_or(rest.len());
            let mut taps = std::mem::take(&mut self.carried);
            taps.extend_from_slice(&rest[..wrap]);
            rest = &rest[wrap..];
            levels = self.frame(&taps, lines);
        }
        self.carried = rest.to_vec();
        levels
    }

    fn frame(&mut self, taps: &[AudioTap], lines: u32) -> Levels {
        let pos = |t: &AudioTap| t.vpos as u32 * CLOCKS_PER_LINE + t.hpos as u32;
        let clocks = lines * CLOCKS_PER_LINE;
        let mut out: [Vec<i16>; 4] = Default::default();
        for o in &mut out {
            o.reserve(clocks as usize);
        }
        let mut k = 0;
        for clock in 0..clocks {
            while k < taps.len() && pos(&taps[k]) <= clock {
                self.dac[taps[k].channel as usize] = taps[k].value;
                k += 1;
            }
            for (n, o) in out.iter_mut().enumerate() {
                o.push(self.dac[n]);
            }
        }
        for t in &taps[k..] {
            self.dac[t.channel as usize] = t.value;
        }
        [level(&out[0]), level(&out[1]), level(&out[2]), level(&out[3])]
    }
}

/// One frame of play's line in a channels file.
pub fn line(levels: Option<Levels>) -> String {
    match levels {
        Some([a, b, c, d]) => format!("{a} {b} {c} {d}\n"),
        None => "-\n".to_string(),
    }
}

/// A channels file's levels, frame of play 1 first (`None`: not compared).
pub fn parse(text: &str) -> Result<Vec<Option<Levels>>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(CHANNELS_HEADER) {
        return Err(format!("not a channels file (no `{CHANNELS_HEADER}` line)"));
    }
    lines
        .enumerate()
        .map(|(n, l)| {
            if l == "-" {
                return Ok(None);
            }
            let v: Vec<u16> = l
                .split(' ')
                .map(|x| x.parse().map_err(|_| format!("line {}: {l:?}", n + 2)))
                .collect::<Result<_, _>>()?;
            <Levels>::try_from(v).map(Some).map_err(|_| format!("line {}: {l:?}", n + 2))
        })
        .collect()
}

/// The engine's levels each frame of play: `feed` replayed headless from `start` (a power-on
/// seeded with the feed's seed) through the app's mixer (`Paula::booted`, a load's blanks a frame
/// each before the step after them, as `sf-app` mixes them), the last video frame of each.
pub fn engine_levels(feed: &Feed, start: Start) -> Vec<Levels> {
    let mut g = Game::with_display(crate::display_data(), start);
    let mut paula = Paula::booted(48_000);
    let mut sink = Vec::new();
    let mut out = Vec::with_capacity(feed.frames.len());
    for fr in &feed.frames {
        let outs = fr.play(&mut g);
        for blank in g.take_load_paula() {
            paula.frame(&blank, &g, &mut sink);
        }
        for o in &outs {
            paula.frame(&o.paula, &g, &mut sink);
        }
        sink.clear();
        out.push(mixer_levels(&paula));
    }
    out
}

/// How far the engine's channels may be from the original's (DESIGN §2.1: a sound no player
/// could tell apart). A channel's frame differs when its level is further from the original's than
/// `floor` and than `percent` of the louder of the two.
///
/// The engine run alone may be some frames from the original (check 4 allows its milestones
/// [`crate::milestones::MILESTONE_FRAMES`]; a load the engine does at once starts the music after it
/// that much late). So each frame of play is judged with the engine's sound shifted by the frames,
/// at most `shift`, that make the most of the `window` frames either side of it match on all four
/// channels: a sound late as a whole lines up, a note the engine never plays differs at every
/// shift. The case fails where a channel differs on more than `frames` frames of play in a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tolerance {
    pub floor: u16,
    pub percent: u16,
    pub frames: u32,
    pub shift: u32,
    pub window: u32,
}

/// The owner's (2026-10-19, T213): a channel's frame differs past 256 (about 3 % of full scale)
/// and past 25 % of the louder side; a run of up to 2 frames is let through (a note started a few
/// lines off); the engine's sound may be up to 10 frames off over 25 frames either side.
pub const OWNER: Tolerance = Tolerance {
    floor: 256,
    percent: 25,
    frames: 2,
    shift: 10,
    window: 25,
};

impl Tolerance {
    fn differs(&self, theirs: u16, ours: u16) -> bool {
        let d = theirs.abs_diff(ours) as u32;
        d > self.floor as u32 && d * 100 > self.percent as u32 * theirs.max(ours) as u32
    }
}

/// A channel differing from the original's on frames of play `first..first + frames`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Run {
    pub channel: usize,
    pub first: u32,
    pub frames: u32,
    /// On its first frame: the original's level, the engine's, and the engine's frames of play
    /// it was compared from (positive: the engine late).
    pub theirs: u16,
    pub ours: u16,
    pub shift: i32,
}

impl std::fmt::Display for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "channel {} from frame of play {} for {} frames (original {}, engine {} {} frames on)",
            self.channel, self.first, self.frames, self.theirs, self.ours, self.shift
        )
    }
}

/// What [`judge`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Verdict {
    /// Frames of play compared (a step of two video frames is not).
    pub compared: u32,
    /// Of them, those judged with the engine's sound shifted, and the furthest shift.
    pub shifted: u32,
    pub furthest: i32,
    /// Of them, those whose shift put the engine's frame before the case's start or past its end:
    /// not judged (as check 3's events cut off).
    pub cut_off: u32,
    /// Each channel's frames that differed.
    pub differing: [u32; 4],
    /// The longest run of frames a channel differed on.
    pub longest: Run,
    /// Every run longer than the tolerance's `frames`, by where it starts: the case fails.
    pub runs: Vec<Run>,
}

impl Verdict {
    /// The first run past the tolerance.
    pub fn failed(&self) -> Option<Run> {
        self.runs.first().copied()
    }
}

/// Each channel's levels, the original's (`None`: not compared) against the engine's, frame of
/// play 1 first.
pub fn judge(theirs: &[Option<Levels>], ours: &[Levels], tol: Tolerance) -> Verdict {
    let len = theirs.len().min(ours.len());
    let k = tol.shift as i64;
    // For each shift, the channels that differ on each frame (four past the engine's frames), and
    // how many differ up to each frame.
    let differ = |n: usize, s: i64| -> u8 {
        let Some(t) = theirs[n] else { return 0 };
        let m = n as i64 + s;
        if m < 0 || m >= len as i64 {
            // Counted against the shift when choosing it, never judged.
            return 0xf;
        }
        let o = ours[m as usize];
        (0..4).filter(|&c| tol.differs(t[c], o[c])).fold(0, |b, c| b | 1 << c)
    };
    let masks: Vec<Vec<u8>> = (-k..=k).map(|s| (0..len).map(|n| differ(n, s)).collect()).collect();
    let sums: Vec<Vec<u32>> = masks
        .iter()
        .map(|m| {
            let mut acc = 0;
            std::iter::once(0)
                .chain(m.iter().map(|b| {
                    acc += b.count_ones();
                    acc
                }))
                .collect()
        })
        .collect();
    let mut v = Verdict::default();
    let mut runs: [Option<Run>; 4] = [None; 4];
    let w = tol.window as usize;
    for n in 0..len {
        if theirs[n].is_none() {
            continue;
        }
        v.compared += 1;
        let (lo, hi) = (n.saturating_sub(w), (n + w + 1).min(len));
        // The fewest differences in the window, the smallest shift among equals.
        let best = (0..masks.len())
            .min_by_key(|&i| (sums[i][hi] - sums[i][lo], (i as i64 - k).abs()))
            .unwrap_or(k as usize);
        let s = best as i64 - k;
        if s != 0 {
            v.shifted += 1;
            if s.abs() > (v.furthest as i64).abs() {
                v.furthest = s as i32;
            }
        }
        let m = n as i64 + s;
        if m < 0 || m >= len as i64 {
            // The engine's sound for this frame comes before the case's start or after its end.
            v.cut_off += 1;
            continue;
        }
        let mask = masks[best][n];
        for c in 0..4 {
            if mask & 1 << c == 0 {
                if let Some(r) = runs[c].take().filter(|r| r.frames > tol.frames) {
                    v.runs.push(r);
                }
                continue;
            }
            v.differing[c] += 1;
            let at = m as usize;
            let r = runs[c].get_or_insert(Run {
                channel: c,
                first: n as u32 + 1,
                frames: 0,
                theirs: theirs[n].map_or(0, |t| t[c]),
                ours: ours[at][c],
                shift: s as i32,
            });
            r.frames += 1;
            if r.frames > v.longest.frames {
                v.longest = *r;
            }
        }
    }
    v.runs.extend(runs.into_iter().flatten().filter(|r| r.frames > tol.frames));
    v.runs.sort_by_key(|r| (r.first, r.channel));
    v
}

/// `case` judged by [`levels`].
pub fn check(case: &Case, tol: Tolerance) -> Result<Verdict, String> {
    let (_, theirs, ours) = levels(case)?;
    Ok(judge(&theirs, &ours, tol))
}

/// `case`'s levels each frame of play, the original's and the engine's: the original's as
/// recorded with its pacing feed (from the trace cache, or by running it), the engine's from
/// replaying the feed. A disk-3 power-on (the story) has no cached feed, which the app starts from
/// disk 1: it is run and replayed here. With the feed.
pub fn levels(case: &Case) -> Result<(Feed, Vec<Option<Levels>>, Vec<Levels>), String> {
    let (feed, theirs, start) = match case.sync {
        SyncPoint::PowerOnDisk3 => {
            let mut c = case.clone();
            c.alone = Alone::Only;
            let mut rec = feed::Recorded::default();
            let _ = crate::run_inner(&c, |_, _| {}, &mut |_, _, _| {}, None, false, Some(&mut rec));
            if rec.panicked {
                return Err(format!("{}: the engine run alone panicked", case.name));
            }
            let theirs = parse(&rec.channels)?;
            let seed = rec.seed;
            (Feed { engine: String::new(), seed, frames: rec.frames }, theirs, Start::Disk3 { seed })
        }
        _ => {
            let f = feed::feed(case)?;
            let read = |p: &std::path::Path| {
                std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
            };
            let feed = Feed::parse(&read(&f.feed)?)?;
            let theirs = parse(&read(&f.channels)?)?;
            let seed = feed.seed;
            (feed, theirs, Start::PowerOn { seed })
        }
    };
    let ours = engine_levels(&feed, start);
    Ok((feed, theirs, ours))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_value_has_no_level_and_a_square_wave_its_amplitude() {
        assert_eq!(level(&[]), 0);
        assert_eq!(level(&[5000; 300]), 0);
        let square: Vec<i16> = (0..400).map(|i| if i % 4 < 2 { 1000 } else { -1000 }).collect();
        assert_eq!(level(&square), 1000);
        // Offset by a held DC, the same.
        let up: Vec<i16> = square.iter().map(|v| v + 3000).collect();
        assert_eq!(level(&up), 1000);
    }

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

    #[test]
    fn the_same_sound_matches_and_a_glitch_of_two_frames_is_let_through() {
        let theirs: Vec<Option<Levels>> = tune(200).into_iter().map(Some).collect();
        let mut ours = tune(200);
        let v = judge(&theirs, &ours, TOL);
        assert_eq!((v.compared, v.shifted, v.runs.len()), (200, 0, 0), "{v:?}");
        ours[100][2] = 0;
        ours[101][2] = 0;
        let v = judge(&theirs, &ours, TOL);
        assert_eq!((v.differing, v.longest.frames, v.runs.len()), ([0, 0, 2, 0], 2, 0), "{v:?}");
    }

    /// The engine late as a whole (a load done at once) lines up; a note it never plays (silent
    /// where the original sounds, T30's W5 note) fails at any shift.
    #[test]
    fn a_late_sound_lines_up_and_a_lost_note_fails() {
        let t = tune(300);
        let theirs: Vec<Option<Levels>> = t.iter().copied().map(Some).collect();
        let late: Vec<Levels> = std::iter::repeat_n([0; 4], 6).chain(t.iter().copied()).take(300).collect();
        let v = judge(&theirs, &late, TOL);
        assert!(v.runs.is_empty(), "{v:?}");
        assert_eq!(v.furthest, 6);
        let mut lost = t.clone();
        for l in &mut lost[150..154] {
            l[1] = 0;
        }
        let v = judge(&theirs, &lost, TOL);
        assert_eq!(v.failed().map(|r| (r.channel, r.first, r.frames)), Some((1, 151, 4)), "{v:?}");
        // Late by more than the shift allows: every channel differs nearly throughout.
        let later: Vec<Levels> = std::iter::repeat_n([0; 4], 40).chain(t.iter().copied()).take(300).collect();
        let v = judge(&theirs, &later, TOL);
        assert!(v.differing.iter().all(|&d| d > 150), "{v:?}");
        assert!(v.longest.frames > 2, "{v:?}");
    }

    #[test]
    fn channels_files_round_trip() {
        let text = format!("{CHANNELS_HEADER}\n{}{}", line(Some([1, 2, 3, 8192])), line(None));
        assert_eq!(parse(&text).unwrap(), vec![Some([1, 2, 3, 8192]), None]);
        assert!(parse("1 2 3 4\n").is_err());
        assert!(parse(&format!("{CHANNELS_HEADER}\n1 2 3\n")).is_err());
    }
}
