//! The pacing feed: what `sf-conform`'s engine run alone (DESIGN §2.1 check 4) is fed besides its
//! start, one record per frame of play, so the app can be fed it exactly (T211). Test tooling only:
//! normal play never reads one (DESIGN §2.5), the app only under `--script --pacing FILE`.
//!
//! Both sides step through the same functions here ([`FeedFrame::step`], [`poke_seed`],
//! [`step_after`], in that order, as [`FeedFrame::play`] does), so the two cannot drift.
//!
//! The file, written by `sf-conform` into the trace cache (DESIGN §3.5), is text:
//!
//! ```text
//! superfrog pacing feed 1           the format and its version
//! engine superfrog 0.1.0 (…)        the engine build that ran alone (sf-app build.rs)
//! seed 1a2b3c4d 00000021            the start's RNG state, lo and hi (Start::PowerOn's)
//! frames 224082                     how many records follow
//! 00.0 o                            one a frame of play: the input, the pacing, then options
//! 08.0 l3,120,64,1,7,3,2 2 load:12,40,118 rng:00ff00ff00ff00ff then:08.0*2,0.0
//! ```
//!
//! A record is the input (`joy.keys`, both hex: [`FrameInput`]'s bits and key set), the
//! [`Pacing`] (`-` unpaced, `o` on time, `d` pass due, `l<writes>,<line>,<hpos>,<phase>,<step>,<point>,<blanks>`
//! late), then, each only when it applies: `2` the original's step ran two video frames, so the
//! engine steps twice; `load:<blanks>,<screen frames>,<line>` the original's load ended this frame
//! ([`Game::load_blanks`], [`Game::set_screen_frames`], [`Game::set_load_line`]; `-` for none);
//! `rng:<hex>` a seed read from the original's clock, poked into `rng_state` after the step (one
//! token a seed, in order); `then:<input>,…` the frames the original loaded under a screen it
//! times itself, stepped on time on their inputs after this frame's step (`input*n` for n in a
//! row).
//!
//! The frame hashes both sides write ([`frame_hashes`]) are text too: `superfrog frame hashes 1`,
//! then `N picture state` (16 hex digits each) for frame 0 (the start) and after each record N.

use std::fmt::Write as _;

use crate::game::{FrameOut, Game, Pacing};
use crate::input::{FrameInput, Joy, KeySet};
use crate::layout::{VarId, def};
use crate::render::{Picture, to_rgb12};
use crate::rng::Rng;

/// The first line of a feed file.
pub const FEED_HEADER: &str = "superfrog pacing feed 1";
/// The first line of a frame-hash file.
pub const HASHES_HEADER: &str = "superfrog frame hashes 1";

/// A whole feed: the run's start and every frame of play.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Feed {
    /// The engine build whose run alone this is (sf-app `build.rs`'s `SF_ENGINE_BUILD`): a feed
    /// depends on where the engine loads, so another build's is refused.
    pub engine: String,
    /// `Start::PowerOn`'s seed: the RNG state the original read from its clock at power-on.
    pub seed: Rng,
    pub frames: Vec<FeedFrame>,
}

/// One frame of play as the engine run alone is fed it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedFrame {
    pub input: FrameInput,
    pub pacing: Pacing,
    /// The original's step ran two video frames: the engine steps twice.
    pub twice: bool,
    /// The original's load ended this frame.
    pub load: Option<FeedLoad>,
    /// Seeds read from the original's clock, poked into `rng_state` after the step, in order.
    pub seeds: Vec<Vec<u8>>,
    /// The frames the original loaded under a screen it times itself: stepped on time on these
    /// inputs after the seeds.
    pub then: Vec<FrameInput>,
}

/// A load's timing, which is the disk's, as an input (owner, T209).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeedLoad {
    /// The vertical blanks the game's handler (in the story, the intro's: T214) ran in it, not
    /// under a timed screen.
    pub blanks: u32,
    /// The frames it held a loading picture for.
    pub screen_frames: Option<u16>,
    /// The beam line it ended on.
    pub line: Option<u16>,
}

impl FeedFrame {
    /// The frame's own step: the load's timing first, then one step, or two when the original's
    /// step ran two video frames. Returns each step's output, in order.
    pub fn step(&self, game: &mut Game) -> Vec<FrameOut> {
        if let Some(l) = self.load {
            game.load_blanks(l.blanks);
            game.set_screen_frames(l.screen_frames);
            game.set_load_line(l.line);
        }
        let mut outs = Vec::with_capacity(2);
        if self.twice {
            outs.push(game.step_paced(self.input, self.pacing));
        }
        outs.push(game.step_paced(self.input, self.pacing));
        outs
    }

    /// The whole record: [`FeedFrame::step`], the seeds ([`poke_seed`]), the frames under a
    /// timed screen ([`step_after`]). Returns every step's output, in order.
    pub fn play(&self, game: &mut Game) -> Vec<FrameOut> {
        let mut outs = self.step(game);
        for s in &self.seeds {
            poke_seed(game, s);
        }
        for &i in &self.then {
            outs.push(step_after(game, i));
        }
        outs
    }
}

/// A seed the original read from its clock, given to the engine where it asked for one.
pub fn poke_seed(game: &mut Game, seed: &[u8]) {
    game.poke(def(VarId::RngState).addr, seed);
}

/// A frame the original loaded under a screen it times itself: the engine shows the screen
/// through it, on time.
pub fn step_after(game: &mut Game, input: FrameInput) -> FrameOut {
    game.step_paced(input, Pacing::OnTime)
}

/// A frame's hashes: the picture as it goes to the GPU (its 12-bit colours and size) and the
/// whole game's state (its save-state encoding). Stable across builds and platforms.
pub fn frame_hashes(game: &Game, picture: &Picture) -> (u64, u64) {
    let mut p = Fnv::default();
    p.word(picture.width as u64);
    p.word(picture.height as u64);
    let rgb = to_rgb12(picture);
    for c in rgb.chunks(4) {
        let mut w = 0u64;
        for (k, &x) in c.iter().enumerate() {
            w |= (x as u64) << (16 * k);
        }
        p.word(w);
    }
    let mut s = Fnv::default();
    s.bytes(&game.encoded());
    (p.0, s.0)
}

/// A hash-file line for frame `n`.
pub fn hash_line(n: u64, (picture, state): (u64, u64)) -> String {
    format!("{n} {picture:016x} {state:016x}\n")
}

/// The `(picture, state)` of each frame of a hash file, from frame 0 on.
pub fn parse_hashes(text: &str) -> Result<Vec<(u64, u64)>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(HASHES_HEADER) {
        return Err(format!("not a frame-hash file (no `{HASHES_HEADER}` line)"));
    }
    lines
        .enumerate()
        .map(|(n, l)| {
            let v: Vec<&str> = l.split(' ').collect();
            let hex = |s: &str| u64::from_str_radix(s, 16).map_err(|_| format!("line {}: {l:?}", n + 2));
            match v[..] {
                [f, p, s] if f == n.to_string() => Ok((hex(p)?, hex(s)?)),
                _ => Err(format!("line {}: expected `{n} picture state`, got {l:?}", n + 2)),
            }
        })
        .collect()
}

/// FNV-1a over 64-bit words, folded: stable everywhere, unlike the standard library's hasher.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn word(&mut self, w: u64) {
        self.0 = (self.0 ^ w).wrapping_mul(0x100_0000_01b3);
        self.0 ^= self.0 >> 29;
    }

    fn bytes(&mut self, b: &[u8]) {
        self.word(b.len() as u64);
        let mut chunks = b.chunks_exact(8);
        for c in &mut chunks {
            self.word(u64::from_le_bytes(c.try_into().unwrap()));
        }
        let mut last = [0u8; 8];
        last[..chunks.remainder().len()].copy_from_slice(chunks.remainder());
        self.word(u64::from_le_bytes(last));
    }
}

fn input_text(i: FrameInput) -> String {
    format!("{:x}.{:x}", i.joy.bits(), i.keys.0)
}

fn parse_input(s: &str) -> Option<FrameInput> {
    let (j, k) = s.split_once('.')?;
    Some(FrameInput {
        joy: Joy(u8::from_str_radix(j, 16).ok()?),
        keys: KeySet(u128::from_str_radix(k, 16).ok()?),
    })
}

fn pacing_text(p: Pacing) -> String {
    match p {
        Pacing::Unpaced => "-".into(),
        Pacing::OnTime => "o".into(),
        Pacing::PassDue => "d".into(),
        Pacing::Late { writes, line, hpos, phase, step, point, blanks } => {
            format!("l{writes},{line},{hpos},{phase},{step},{point},{blanks}")
        }
    }
}

fn parse_pacing(s: &str) -> Option<Pacing> {
    Some(match s {
        "-" => Pacing::Unpaced,
        "o" => Pacing::OnTime,
        "d" => Pacing::PassDue,
        _ => {
            let v: Vec<&str> = s.strip_prefix('l')?.split(',').collect();
            let [w, l, h, p, st, pt, b] = v[..] else { return None };
            Pacing::Late {
                writes: w.parse().ok()?,
                line: l.parse().ok()?,
                hpos: h.parse().ok()?,
                phase: p.parse().ok()?,
                step: st.parse().ok()?,
                point: pt.parse().ok()?,
                blanks: b.parse().ok()?,
            }
        }
    })
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or("-".into(), |v| v.to_string())
}

fn parse_opt<T: std::str::FromStr>(s: &str) -> Option<Option<T>> {
    if s == "-" { Some(None) } else { s.parse().ok().map(Some) }
}

impl FeedFrame {
    /// The record's line, without its newline.
    pub fn to_line(&self) -> String {
        let mut s = format!("{} {}", input_text(self.input), pacing_text(self.pacing));
        if self.twice {
            s.push_str(" 2");
        }
        if let Some(l) = self.load {
            let _ = write!(s, " load:{},{},{}", l.blanks, opt(l.screen_frames), opt(l.line));
        }
        for seed in &self.seeds {
            s.push_str(" rng:");
            for b in seed {
                let _ = write!(s, "{b:02x}");
            }
        }
        if !self.then.is_empty() {
            // Runs of one input as `input*n`.
            let mut runs: Vec<(FrameInput, usize)> = Vec::new();
            for &i in &self.then {
                match runs.last_mut() {
                    Some((j, n)) if *j == i => *n += 1,
                    _ => runs.push((i, 1)),
                }
            }
            let then: Vec<String> = runs
                .into_iter()
                .map(|(i, n)| match n {
                    1 => input_text(i),
                    _ => format!("{}*{n}", input_text(i)),
                })
                .collect();
            let _ = write!(s, " then:{}", then.join(","));
        }
        s
    }

    pub fn parse_line(line: &str) -> Option<FeedFrame> {
        let mut t = line.split(' ');
        let mut f = FeedFrame {
            input: parse_input(t.next()?)?,
            pacing: parse_pacing(t.next()?)?,
            ..FeedFrame::default()
        };
        for tok in t {
            if tok == "2" {
                f.twice = true;
            } else if let Some(v) = tok.strip_prefix("load:") {
                let v: Vec<&str> = v.split(',').collect();
                let [b, s, l] = v[..] else { return None };
                f.load = Some(FeedLoad {
                    blanks: b.parse().ok()?,
                    screen_frames: parse_opt(s)?,
                    line: parse_opt(l)?,
                });
            } else if let Some(v) = tok.strip_prefix("rng:") {
                if v.len() % 2 != 0 {
                    return None;
                }
                let bytes = (0..v.len())
                    .step_by(2)
                    .map(|k| u8::from_str_radix(&v[k..k + 2], 16).ok())
                    .collect::<Option<Vec<u8>>>()?;
                f.seeds.push(bytes);
            } else if let Some(v) = tok.strip_prefix("then:") {
                for run in v.split(',') {
                    let (i, n) = match run.split_once('*') {
                        Some((i, n)) => (i, n.parse().ok().filter(|&n: &usize| n > 1)?),
                        None => (run, 1),
                    };
                    let i = parse_input(i)?;
                    f.then.extend(std::iter::repeat_n(i, n));
                }
            } else {
                return None;
            }
        }
        Some(f)
    }
}

impl Feed {
    /// The header: everything before the records.
    pub fn header(engine: &str, seed: Rng, frames: usize) -> String {
        format!(
            "{FEED_HEADER}\nengine {engine}\nseed {:08x} {:08x}\nframes {frames}\n",
            seed.lo, seed.hi
        )
    }

    pub fn to_text(&self) -> String {
        let mut s = Feed::header(&self.engine, self.seed, self.frames.len());
        for f in &self.frames {
            s.push_str(&f.to_line());
            s.push('\n');
        }
        s
    }

    pub fn parse(text: &str) -> Result<Feed, String> {
        let mut lines = text.lines();
        if lines.next() != Some(FEED_HEADER) {
            return Err(format!("not a pacing feed (no `{FEED_HEADER}` line)"));
        }
        let mut field = |name: &str| -> Result<String, String> {
            lines
                .next()
                .and_then(|l| l.strip_prefix(name)?.strip_prefix(' ').map(str::to_string))
                .ok_or_else(|| format!("the feed's `{name}` line is missing"))
        };
        let engine = field("engine")?;
        let seed = field("seed")?;
        let seed = seed
            .split_once(' ')
            .and_then(|(lo, hi)| {
                Some(Rng {
                    lo: u32::from_str_radix(lo, 16).ok()?,
                    hi: u32::from_str_radix(hi, 16).ok()?,
                })
            })
            .ok_or_else(|| format!("the feed's seed {seed:?}: expected two hex longs"))?;
        let n: usize = field("frames")?
            .parse()
            .map_err(|_| "the feed's frame count is not a number".to_string())?;
        let frames = lines
            .enumerate()
            .map(|(k, l)| FeedFrame::parse_line(l).ok_or_else(|| format!("feed line {}: {l:?}", k + 5)))
            .collect::<Result<Vec<_>, _>>()?;
        if frames.len() != n {
            return Err(format!("the feed says {n} frames and holds {}: cut short?", frames.len()));
        }
        Ok(Feed { engine, seed, frames })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_feed_reads_back_as_written() {
        let feed = Feed {
            engine: "superfrog 0.1.0 (0123456789abcdef)".into(),
            seed: Rng { lo: 0x1a2b_3c4d, hi: 0x21 },
            frames: vec![
                FeedFrame::default(),
                FeedFrame {
                    input: FrameInput { joy: Joy::RIGHT | Joy::FIRE, keys: KeySet::key(0x45) },
                    pacing: Pacing::Late { writes: 3, line: 120, hpos: 64, phase: 1, step: 7, point: 3, blanks: 2 },
                    twice: true,
                    load: Some(FeedLoad { blanks: 12, screen_frames: None, line: Some(118) }),
                    seeds: vec![vec![0, 0xff, 1, 2, 3, 4, 5, 6], vec![9; 8]],
                    then: vec![FrameInput::joy(Joy::LEFT), FrameInput::NONE, FrameInput::NONE, FrameInput::NONE],
                },
                FeedFrame { pacing: Pacing::OnTime, ..FeedFrame::default() },
                FeedFrame { pacing: Pacing::PassDue, load: Some(FeedLoad::default()), ..FeedFrame::default() },
            ],
        };
        let text = feed.to_text();
        assert!(text.starts_with("superfrog pacing feed 1\nengine superfrog 0.1.0"), "{text}");
        assert!(text.contains(" then:4.0,0.0*3\n"), "{text}");
        assert_eq!(Feed::parse(&text).unwrap(), feed);
    }

    #[test]
    fn a_feed_cut_short_or_damaged_is_refused() {
        let feed = Feed { frames: vec![FeedFrame::default(); 3], ..Feed::default() };
        let text = feed.to_text();
        let short: String = text.lines().take(6).map(|l| format!("{l}\n")).collect();
        assert!(Feed::parse(&short).unwrap_err().contains("cut short"));
        assert!(Feed::parse(&text.replace("0.0 -", "0.0 q")).is_err());
        assert!(Feed::parse("hello\n").unwrap_err().contains("not a pacing feed"));
    }

    #[test]
    fn hash_files_read_back_and_refuse_a_skipped_frame() {
        let text = format!("{HASHES_HEADER}\n{}{}", hash_line(0, (1, 2)), hash_line(1, (u64::MAX, 0)));
        assert_eq!(parse_hashes(&text).unwrap(), vec![(1, 2), (u64::MAX, 0)]);
        let gap = format!("{HASHES_HEADER}\n{}{}", hash_line(0, (1, 2)), hash_line(2, (3, 4)));
        assert!(parse_hashes(&gap).is_err());
    }

    #[test]
    fn the_word_hash_sees_one_bit_anywhere() {
        let h = |b: &[u8]| {
            let mut f = Fnv::default();
            f.bytes(b);
            f.0
        };
        let base = vec![0u8; 100];
        let mut seen = std::collections::BTreeSet::new();
        assert!(seen.insert(h(&base)));
        for i in 0..100 {
            for bit in 0..8 {
                let mut b = base.clone();
                b[i] ^= 1 << bit;
                assert!(seen.insert(h(&b)), "byte {i} bit {bit}");
            }
        }
        assert_ne!(h(&[0; 7]), h(&[0; 8]), "the length counts");
    }
}
