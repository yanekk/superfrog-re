//! The pacing feed (T211): what check 4's engine, run alone, is fed, and the hash of each frame it
//! played, written into the trace cache (DESIGN §3.5) so the app can be fed the same and its frames
//! compared with the headless run's (`sf-e2e`'s `whole_game.rs`). The formats are
//! `sf_core::feed`'s. Beside them, `<key>.audio` holds how loud the original was each frame of
//! play ([`AUDIO_HEADER`]), so the app's sound can be held to the original's (T30).
//!
//! `~/.cache/superfrog-re/traces/<key>.feed` and `<key>.hashes` (`SUPERFROG_CACHE` replacing the
//! root), the key a SHA-1, as a trace's, over the feed format, the vAmiga build, the Kickstart and
//! disk files, the case's sync point, frames and script, and the engine build: a feed depends on
//! where the engine loads, so another build's is another key.

use std::path::{Path, PathBuf};

use sf_core::feed::{Feed, FeedFrame, HASHES_HEADER, frame_hashes, hash_line};
use sf_core::{Game, Picture, Rng, View};

use crate::{Alone, Case, SyncPoint};

/// Bump when what a feed or its hashes hold changes.
const FEED_FORMAT: u32 = 5;

/// The engine build this crate's engine is (sf-app `build.rs`, shared): what a feed is tagged with.
pub const ENGINE_BUILD: &str = env!("SF_ENGINE_BUILD");

/// What the engine run alone was fed and the hash of each frame it played, as it ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recorded {
    pub seed: Rng,
    pub frames: Vec<FeedFrame>,
    /// `sf_core::feed`'s hash file, frame 0 (the start) on.
    pub hashes: String,
    /// [`AUDIO_HEADER`]'s file: the original's loudness each frame of play.
    pub audio: String,
    /// [`crate::channels::CHANNELS_HEADER`]'s file: each of the original's channels' level each
    /// frame of play (T213).
    pub channels: String,
    /// The engine run alone panicked: what it was fed after is no feed.
    pub panicked: bool,
}

/// The first line of an audio file: then one line a frame of play, from frame 1, the [`swing`]
/// of the original's own mix (vAmiga's, `Amiga::mixed_audio`) in the frame its step ended in.
pub const AUDIO_HEADER: &str = "superfrog original audio swing 1";

/// How far a frame of the original's mix swings: half the range its samples span on the wider
/// side, in 16-bit units (0 to 32767). A level held still is no sound however far from zero: a
/// stopped channel leaves its last sample on the DAC, which vAmiga's mix keeps (T30: W2-1's
/// stops sat at 367 and 786 for frames).
pub fn swing(mix: &[[f32; 2]]) -> u16 {
    let side = |c: usize| {
        let (lo, hi) = mix
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), s| (lo.min(s[c]), hi.max(s[c])));
        if mix.is_empty() { 0.0 } else { (hi - lo) / 2.0 }
    };
    (side(0).max(side(1)).min(1.0) * 32767.0).round() as u16
}

/// An audio file's swings, frame of play 1 first.
pub fn parse_audio(text: &str) -> Result<Vec<u16>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(AUDIO_HEADER) {
        return Err(format!("not an audio-peak file (no `{AUDIO_HEADER}` line)"));
    }
    lines
        .enumerate()
        .map(|(n, l)| l.parse().map_err(|_| format!("line {}: {l:?}", n + 2)))
        .collect()
}

impl Recorded {
    /// Recording from `game` at its start, seeded `seed`: frame 0's hashes.
    pub(crate) fn start(game: &Game, seed: Rng) -> Recorded {
        let pic = game.render(View::default());
        Recorded {
            seed,
            frames: Vec::new(),
            hashes: format!("{HASHES_HEADER}\n{}", hash_line(0, frame_hashes(game, &pic))),
            audio: format!("{AUDIO_HEADER}\n"),
            channels: format!("{}\n", crate::channels::CHANNELS_HEADER),
            panicked: false,
        }
    }

    /// A frame of play: what was fed, the game and its last picture after it, and the
    /// original's [`swing`] and channel levels that frame.
    pub(crate) fn frame(
        &mut self,
        fed: FeedFrame,
        game: &Game,
        picture: &Picture,
        swing: u16,
        channels: Option<crate::channels::Levels>,
    ) {
        self.frames.push(fed);
        self.hashes
            .push_str(&hash_line(self.frames.len() as u64, frame_hashes(game, picture)));
        self.audio.push_str(&format!("{swing}\n"));
        self.channels.push_str(&crate::channels::line(channels));
    }
}

/// A case's feed and hashes in the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedFiles {
    pub feed: PathBuf,
    pub hashes: PathBuf,
    /// The original's loudness each frame of play ([`AUDIO_HEADER`]).
    pub audio: PathBuf,
    /// The original's channel levels each frame of play ([`crate::channels::CHANNELS_HEADER`]).
    pub channels: PathBuf,
    /// Frames of play: records in the feed.
    pub frames: usize,
    /// Whether the files were already in the cache.
    pub cached: bool,
    /// Check 4's verdict when the run was made now: `Err` with the divergence when the engine
    /// run alone did not meet the original's milestones (the feed is still what it was fed).
    pub alone: Result<(), String>,
}

/// Where `case`'s feed, hashes, audio and channels go: `<traces>/<key>.feed`, `.hashes`,
/// `.audio`, `.channels`.
pub fn paths(case: &Case) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), String> {
    let cfg = sf_oracle::OracleConfig::locate(1).map_err(|e| e.to_string())?;
    let script: String = case
        .script
        .lines
        .iter()
        .map(|(f, i)| format!("{f} {:02x} {:032x}\n", i.joy.bits(), i.keys.0))
        .collect();
    let what = format!(
        "pacing feed {FEED_FORMAT}\nengine {ENGINE_BUILD}\nsync {:?}\nframes {}\nscript\n{script}",
        case.sync, case.frames
    );
    let key = crate::sync::cache_key(&cfg, &what)?;
    let dir = crate::sync::cache_root().join("traces");
    Ok((
        dir.join(format!("{key}.feed")),
        dir.join(format!("{key}.hashes")),
        dir.join(format!("{key}.audio")),
        dir.join(format!("{key}.channels")),
    ))
}

/// `case`'s feed and hashes: from the cache, or made by running it (check 4 only, as
/// `run --alone`) and stored. Only a case from power-on has one: the app starts there.
pub fn feed(case: &Case) -> Result<FeedFiles, String> {
    if !matches!(case.sync, SyncPoint::PowerOn | SyncPoint::PowerOnDf1) {
        return Err(format!("{}: a pacing feed starts at power-on, not {:?}", case.name, case.sync));
    }
    // A frozen case's objects are held still on both sides, which the app never does (and the
    // feed's key does not say so, so it would find the unfrozen case's feed).
    if case.freeze() != crate::freeze::Freeze::None {
        return Err(format!("{}: a frozen case ({:?}) has no pacing feed", case.name, case.freeze()));
    }
    let (feed_path, hashes_path, audio_path, channels_path) = paths(case)?;
    if let (Ok(text), true, true, true) = (
        std::fs::read_to_string(&feed_path),
        hashes_path.is_file(),
        audio_path.is_file(),
        channels_path.is_file(),
    )
        && let Ok(f) = Feed::parse(&text)
        && f.engine == ENGINE_BUILD
    {
        return Ok(FeedFiles {
            feed: feed_path,
            hashes: hashes_path,
            audio: audio_path,
            channels: channels_path,
            frames: f.frames.len(),
            cached: true,
            alone: Ok(()),
        });
    }
    let mut c = case.clone();
    c.alone = Alone::Only;
    let mut rec = Recorded::default();
    let verdict = crate::run_inner(&c, |_, _| {}, &mut |_, _, _| {}, None, false, Some(&mut rec));
    if rec.panicked {
        let why = verdict.err().map(|d| d.to_string()).unwrap_or_default();
        return Err(format!("{}: the engine run alone panicked, so there is no feed: {why}", case.name));
    }
    if rec.frames.len() != case.frames as usize {
        return Err(format!(
            "{}: the engine run alone was fed {} frames of the case's {}",
            case.name,
            rec.frames.len(),
            case.frames
        ));
    }
    let feed = Feed {
        engine: ENGINE_BUILD.to_string(),
        seed: rec.seed,
        frames: rec.frames,
    };
    write(&hashes_path, rec.hashes.as_bytes())?;
    write(&audio_path, rec.audio.as_bytes())?;
    write(&channels_path, rec.channels.as_bytes())?;
    write(&feed_path, feed.to_text().as_bytes())?;
    Ok(FeedFiles {
        feed: feed_path,
        hashes: hashes_path,
        audio: audio_path,
        channels: channels_path,
        frames: feed.frames.len(),
        cached: false,
        alone: verdict.map(|_| ()).map_err(|d| d.to_string()),
    })
}

/// Temp file, then rename, in the same folder.
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(err)?;
    std::fs::rename(&tmp, path).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_swings_round_trip_and_a_held_level_is_none() {
        assert_eq!(swing(&[[0.0, -0.5], [0.25, 0.0]]), 8192);
        assert_eq!(swing(&[[0.0, -0.5], [0.0, 0.5]]), 16384);
        // A stopped channel's last sample, held: no sound.
        assert_eq!(swing(&[[0.02, 0.0]; 960]), 0);
        assert_eq!(swing(&[[-1.5, 0.0], [1.5, 0.0]]), 32767);
        assert_eq!(swing(&[]), 0);
        let text = format!("{AUDIO_HEADER}\n0\n12\n32767\n");
        assert_eq!(parse_audio(&text).unwrap(), vec![0, 12, 32767]);
        assert!(parse_audio("0\n").is_err());
        assert!(parse_audio(&format!("{AUDIO_HEADER}\n-1\n")).is_err());
    }
    use crate::case::{Excluded, Exclusion};

    /// A frozen case's original and engine run with their objects held still (`freeze.rs`,
    /// `Game::frog_only`), which the app never does: its feed would replay another game.
    #[test]
    fn a_frozen_case_has_no_feed() {
        let mut case = Case::load(&crate::cases_dir().join("game-whole.toml")).unwrap();
        for what in [Excluded::Objects, Excluded::Enemies] {
            case.exclude = vec![Exclusion { what, note: String::new(), task: String::new() }];
            let e = feed(&case).unwrap_err();
            assert!(e.contains("frozen"), "{e}");
        }
    }
}
