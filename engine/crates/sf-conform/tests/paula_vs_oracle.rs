//! The mixer (`sf-paula`) against the reference emulator on the title music (T10).
//!
//! One run from power-on: the crack intro (music from frame ~345), a port-1 click at frames
//! 1 000-1 004 to skip it, then the title, whose music starts near frame 3 996. Every frame's
//! Paula writes (with their beam positions), its length in lines and the chip RAM after it are fed
//! to [`Paula`], from power-on to [`TITLE_FRAMES`] frames into the title music, and compared with
//! the reference's own DAC taps (core patch 0004) and its own 48 kHz mix.
//!
//! Needs what sf-oracle's tests need: `tools/setup/oracle.sh` run, the four disks and Kickstart 1.3.
//! It lives here, not in `sf-paula/tests/`, because the purity test scans the pure crates' tests
//! too, and this one runs the emulator and measures in floating point.

use std::ops::Range;
use std::sync::OnceLock;

use sf_oracle::*;
use sf_paula::{CLOCKS_PER_LINE, Paula, RESAMPLER_DELAY};

/// The crack intro skips on a port-1 click (FINDINGS 2026-10-11).
const CLICK: Range<u32> = 1_000..1_005;
/// How far into the title music the run goes.
const TITLE_FRAMES: u32 = 500;
/// The title music is the first DAC change after this many frames without one (the crack intro's
/// music stops when it is skipped; the title loads in silence).
const QUIET: u32 = 500;
/// Give up looking for the title music after this frame.
const LAST_START: u32 = 6_000;

/// The reference's stream starts with half its 4 096-sample buffer of silence (`AudioStream`
/// aligning its write pointer), ours runs the resampler's delay late: our sample i + PREFILL_LAG
/// is its sample i. Measured: the best lag in a ±64 search is exactly this one.
const PREFILL_LAG: i64 = RESAMPLER_DELAY as i64 - 2_048;

/// Our full scale over the reference's: we put two channels at sample -128, volume 64 at -32 768
/// (`2 * value / 32 768` per channel value); vAmiga multiplies a channel value by 0.000 025 at
/// channel volume 100 and master volume 50 (`AudioPort::setOption`).
const NOMINAL_GAIN: f64 = 2.0 / 32_768.0 / 0.000_025;

/// The mix's tolerance: the relative RMS difference between our output and the reference's, after
/// the least-squares gain per side, may be at most this. Why it is not near 0: vAmiga mixes by
/// point-sampling each channel's DAC steps (no anti-aliasing), on a sample grid it restarts at
/// every frame's mixing call, while ours is a band-limited resample. Everything above the cutoff
/// the reference folds back as aliasing is a difference, and on this music it is large: measured
/// 0.181 on 2026-10-11, falling to 0.05 when both are smoothed over 16 samples, so the difference
/// is high-frequency, not timing or level. 0.25 leaves room for that and fails on any real fault:
/// a swapped side, a missing channel or a frame's misplaced audio each push it towards 1.
const TOLERANCE: f64 = 0.25;
/// The least-squares gain per side may be this far (relative) from [`NOMINAL_GAIN`]: aliasing adds
/// energy to the reference that the fit cannot match, which pulls the gain low (measured 2 %).
const GAIN_TOLERANCE: f64 = 0.05;

/// Where a channel's taps first differ from the reference's.
#[derive(Debug)]
#[allow(dead_code)] // read through Debug
struct Mismatch {
    frame: u32,
    channel: usize,
    vpos: u32,
    hpos: u32,
    got: i16,
    want: i16,
}

struct Run {
    /// The first frame of the title music.
    start: u32,
    frames: u32,
    first_mismatch: Option<Mismatch>,
    /// DAC changes the reference made in the title-music frames.
    title_taps: usize,
    /// The mixer's output and the reference's own mix, both at 48 kHz from power-on.
    ours: Vec<[i16; 2]>,
    theirs: Vec<[f32; 2]>,
    /// The title music's samples in `theirs`.
    window: Range<usize>,
}

fn run() -> &'static Run {
    static RUN: OnceLock<Run> = OnceLock::new();
    RUN.get_or_init(|| {
        let cfg = OracleConfig::locate(DEFAULT_MAX_FRAMES).unwrap_or_else(|e| panic!("{e}"));
        let mut amiga = Amiga::power_on(&cfg).unwrap();
        let mut paula = Paula::new(MIX_RATE);
        let mut reference = Reference::default();
        let mut run = Run {
            start: 0,
            frames: 0,
            first_mismatch: None,
            title_taps: 0,
            ours: Vec::new(),
            theirs: Vec::new(),
            window: 0..0,
        };
        let mut end = LAST_START + TITLE_FRAMES;
        let mut quiet = 0;
        let mut f = 0;
        while f < end {
            let input = if CLICK.contains(&f) {
                FrameInput::joy(Joy::PORT1_BUTTON)
            } else {
                FrameInput::NONE
            };
            let c = amiga.step(input).unwrap();
            let lines = amiga.frame_lines();
            paula.set_frame_lines(lines);
            paula.frame(&c.paula, &amiga.memory(), &mut run.ours);
            let want = reference.frame(&c, lines);
            let sound = c.taps().next().is_some();
            if run.start == 0 && sound && quiet >= QUIET {
                run.start = f;
                end = f + TITLE_FRAMES;
                run.window.start = run.theirs.len();
            }
            quiet = if sound { 0 } else { quiet + 1 };
            run.theirs.extend(amiga.mixed_audio());
            if run.start != 0 {
                run.title_taps += c.taps().count();
            }
            if run.first_mismatch.is_none() {
                run.first_mismatch = first_difference(f, &paula, &want);
            }
            f += 1;
            run.frames = f;
        }
        assert_ne!(run.start, 0, "no title music by frame {LAST_START}");
        run.window.end = run.theirs.len();
        run
    })
}

fn first_difference(frame: u32, paula: &Paula, want: &[Vec<i16>; 4]) -> Option<Mismatch> {
    let got = paula.channel_taps();
    (0..4).find_map(|n| {
        assert_eq!(got[n].len(), want[n].len(), "frame {frame}: frame length");
        let i = (0..got[n].len()).find(|&i| got[n][i] != want[n][i])?;
        Some(Mismatch {
            frame,
            channel: n,
            vpos: i as u32 / CLOCKS_PER_LINE,
            hpos: i as u32 % CLOCKS_PER_LINE,
            got: got[n][i],
            want: want[n][i],
        })
    })
}

/// The reference's taps as the DAC value at every clock. A step's list runs to just after the next
/// frame's start, so a tap whose position goes back belongs, with those after it, to the next frame.
#[derive(Default)]
struct Reference {
    dac: [i16; 4],
    carried: Vec<AudioTap>,
}

impl Reference {
    fn frame(&mut self, c: &FrameCapture, lines: u32) -> [Vec<i16>; 4] {
        let pos = |t: &AudioTap| t.vpos as u32 * CLOCKS_PER_LINE + t.hpos as u32;
        let all: Vec<AudioTap> = c.taps().collect();
        let wrap = (1..all.len())
            .find(|&i| pos(&all[i]) < pos(&all[i - 1]))
            .unwrap_or(all.len());
        let mut taps = std::mem::take(&mut self.carried);
        taps.extend_from_slice(&all[..wrap]);
        self.carried = all[wrap..].to_vec();
        let mut out: [Vec<i16>; 4] = Default::default();
        let mut k = 0;
        for clock in 0..lines * CLOCKS_PER_LINE {
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
        out
    }
}

#[test]
fn channel_taps_are_the_references_at_every_clock_through_the_title_music() {
    let r = run();
    if let Some(m) = &r.first_mismatch {
        panic!(
            "{m:?} (title music starts at frame {}; run of {} frames)",
            r.start, r.frames
        );
    }
    // The comparison covered real music: thousands of DAC changes on the title.
    assert!(r.title_taps > 100_000, "only {} taps", r.title_taps);
    assert_eq!(r.frames, r.start + TITLE_FRAMES);
}

#[test]
fn the_mix_is_within_tolerance_of_the_references_own() {
    let r = run();
    let ours: Vec<[f64; 2]> = r
        .ours
        .iter()
        .map(|s| [s[0] as f64 / 32_768.0, s[1] as f64 / 32_768.0])
        .collect();
    let theirs: Vec<[f64; 2]> = r
        .theirs
        .iter()
        .map(|s| [s[0] as f64, s[1] as f64])
        .collect();
    let w = &r.window;
    let (lag, err) = (-64..=64)
        .map(|d| PREFILL_LAG + d)
        .map(|lag| (lag, residual(&ours, &theirs, w, lag).0))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    let (_, gain) = residual(&ours, &theirs, w, lag);
    let what = format!("lag {lag} (expected {PREFILL_LAG}), gain {gain:?}, error {err:.4}");
    assert_eq!(lag, PREFILL_LAG, "{what}");
    assert!(err <= TOLERANCE, "{what}");
    for g in gain {
        assert!(
            (g / NOMINAL_GAIN - 1.0).abs() <= GAIN_TOLERANCE,
            "{what}; nominal gain {NOMINAL_GAIN}"
        );
    }
}

/// The relative RMS of `ours - gain * theirs` over the window with `ours` shifted by `lag`, for the
/// least-squares gain per side, and those gains.
fn residual(ours: &[[f64; 2]], theirs: &[[f64; 2]], w: &Range<usize>, lag: i64) -> (f64, [f64; 2]) {
    let mut gain = [0.0; 2];
    let (mut num, mut den) = (0.0, 0.0);
    for side in 0..2 {
        let pairs = || {
            w.clone().filter_map(move |i| {
                let j = i as i64 + lag;
                (j >= 0 && (j as usize) < ours.len())
                    .then(|| (ours[j as usize][side], theirs[i][side]))
            })
        };
        let xy: f64 = pairs().map(|(o, t)| o * t).sum();
        let yy: f64 = pairs().map(|(_, t)| t * t).sum();
        let g = if yy > 0.0 { xy / yy } else { 0.0 };
        gain[side] = g;
        num += pairs().map(|(o, t)| (o - g * t) * (o - g * t)).sum::<f64>();
        den += pairs().map(|(o, _)| o * o).sum::<f64>();
    }
    ((num / den).sqrt(), gain)
}
