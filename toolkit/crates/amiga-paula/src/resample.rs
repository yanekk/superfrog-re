//! Paula-rate stereo to host-rate stereo, band-limited. The one pure file allowed floating point
//! and filter tables (DESIGN §3.1, owner 2026-10-10): what it makes feeds no game state.
//!
//! The channels' output is a step function that changes at whole Paula clocks. Each change of a
//! side's level is placed on the host timeline as a band-limited impulse (a Kaiser-windowed sinc,
//! looked up in a table of [`PHASES`] fractional positions and interpolated between them), and the
//! impulses are summed into a running level: a band-limited step, so nothing above the host's
//! Nyquist rate folds back as aliasing. Every table row sums to exactly 1, so a step settles at
//! exactly its height. An output sample is final once no later change can reach it, so the output
//! runs [`HALF_WIDTH`] host samples late: it starts with that many samples of silence, and after N
//! Paula clocks exactly `N * host_rate / PAULA_CLOCK_PAL` samples (rounded down) have been made.

use std::collections::VecDeque;

use crate::PAULA_CLOCK_PAL;

/// Host samples each side of an impulse's centre.
pub const HALF_WIDTH: usize = 24;
/// Fractional positions in the impulse table.
const PHASES: usize = 128;
const TAPS: usize = 2 * HALF_WIDTH;
/// Cutoff as a fraction of the host rate (Nyquist is 0.5).
const CUTOFF: f64 = 0.42;
/// Kaiser window shape: about 80 dB of stop-band attenuation.
const KAISER_BETA: f64 = 8.0;

pub struct Resampler {
    host_rate: u64,
    /// `PHASES + 1` rows of `TAPS`: row p is the impulse centred `p / PHASES` of a sample after a
    /// whole sample. Row `PHASES` is row 0 shifted by one, for interpolating.
    table: Vec<f64>,
    /// Paula clocks run since the start.
    clock: u64,
    /// Impulses not yet summed, from host sample `emitted` on: [left, right].
    pending: VecDeque<[f64; 2]>,
    /// Host samples emitted so far.
    emitted: u64,
    /// The running (summed) level of each side.
    level: [f64; 2],
    /// The level of each side at the last Paula clock seen, as an integer.
    last: [i32; 2],
}

impl Resampler {
    pub fn new(host_rate: u32) -> Resampler {
        assert!(host_rate > 0, "a host rate of 0");
        Resampler {
            host_rate: host_rate as u64,
            table: table(),
            clock: 0,
            pending: VecDeque::new(),
            emitted: 0,
            level: [0.0; 2],
            last: [0; 2],
        }
    }

    /// Take one frame of Paula-rate levels, `[left, right]` per Paula clock, and append every
    /// host sample now final to `out`.
    pub fn push(&mut self, levels: impl Iterator<Item = [i32; 2]>, out: &mut Vec<[i16; 2]>) {
        for lv in levels {
            for side in 0..2 {
                let d = lv[side] - self.last[side];
                if d != 0 {
                    self.impulse(side, d as f64);
                    self.last[side] = lv[side];
                }
            }
            self.clock += 1;
        }
        // Every sample a change at or after `clock` cannot reach is final.
        let ready = self.clock * self.host_rate / PAULA_CLOCK_PAL as u64;
        while self.emitted < ready {
            let p = self.pending.pop_front().unwrap_or([0.0; 2]);
            self.emitted += 1;
            let mut s = [0i16; 2];
            for side in 0..2 {
                self.level[side] += p[side];
                s[side] = (self.level[side] * 2.0).round().clamp(-32768.0, 32767.0) as i16;
            }
            out.push(s);
        }
    }

    /// A step of `d` on `side` at Paula clock `self.clock`.
    fn impulse(&mut self, side: usize, d: f64) {
        // Its position in output samples, HALF_WIDTH late: whole part `n`, fraction `frac`.
        let num = self.clock * self.host_rate;
        let den = PAULA_CLOCK_PAL as u64;
        let n = num / den + HALF_WIDTH as u64;
        let frac = (num % den) as f64 / den as f64;
        let pos = frac * PHASES as f64;
        let p = (pos as usize).min(PHASES - 1);
        let w = pos - p as f64;
        // Tap j lands on host sample n - HALF_WIDTH + 1 + j.
        let first = n + 1 - HALF_WIDTH as u64;
        debug_assert!(first >= self.emitted, "a change reached an emitted sample");
        let offset = (first - self.emitted) as usize;
        if self.pending.len() < offset + TAPS {
            self.pending.resize(offset + TAPS, [0.0; 2]);
        }
        let (a, b) = (
            &self.table[p * TAPS..][..TAPS],
            &self.table[(p + 1) * TAPS..][..TAPS],
        );
        for j in 0..TAPS {
            self.pending[offset + j][side] += d * (a[j] + (b[j] - a[j]) * w);
        }
    }
}

/// The impulse table: row p, tap j is h(j + 1 - HALF_WIDTH - p / PHASES), each row scaled to sum 1.
fn table() -> Vec<f64> {
    let mut t = Vec::with_capacity((PHASES + 1) * TAPS);
    for p in 0..=PHASES {
        let frac = p as f64 / PHASES as f64;
        let row: Vec<f64> = (0..TAPS)
            .map(|j| kernel(j as f64 + 1.0 - HALF_WIDTH as f64 - frac))
            .collect();
        let sum: f64 = row.iter().sum();
        t.extend(row.iter().map(|v| v / sum));
    }
    t
}

/// Kaiser-windowed sinc with cutoff [`CUTOFF`], at `x` host samples from its centre.
fn kernel(x: f64) -> f64 {
    let r = x / HALF_WIDTH as f64;
    if r.abs() >= 1.0 {
        return 0.0;
    }
    let a = 2.0 * CUTOFF * x;
    let sinc = if a == 0.0 {
        1.0
    } else {
        (std::f64::consts::PI * a).sin() / (std::f64::consts::PI * a)
    };
    sinc * bessel_i0(KAISER_BETA * (1.0 - r * r).sqrt()) / bessel_i0(KAISER_BETA)
}

/// The modified Bessel function of the first kind, order 0, by its power series.
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term, mut k) = (1.0, 1.0, 1.0);
    while term > 1e-12 * sum {
        term *= (x / (2.0 * k)) * (x / (2.0 * k));
        sum += term;
        k += 1.0;
    }
    sum
}
