//! When to step the game (DESIGN §2.5): one step per PAL frame of a clock, never a fraction of
//! one. The clock is the audio device's samples played, or the wall clock when there is no device
//! (DESIGN §2.11); either way it is counted in host samples, and the pacer turns it into Paula
//! clocks, which is what a PAL frame is made of (`lines * 227`).

use sf_paula::{CLOCKS_PER_LINE, LINES_PER_FRAME, PAULA_CLOCK_PAL};

#[derive(Debug, Clone)]
pub struct Pacer {
    host_rate: u64,
    /// Paula clocks of every frame stepped so far.
    stepped: u64,
    /// The length of the next frames, in lines (a long PAL frame, or one fewer: FINDINGS, LOF).
    lines: u32,
}

impl Pacer {
    pub fn new(host_rate: u32) -> Pacer {
        assert!(host_rate > 0);
        Pacer {
            host_rate: host_rate as u64,
            stepped: 0,
            lines: LINES_PER_FRAME,
        }
    }

    /// The length of the frames from now on (313 lines, or 312 while the game keeps short frames).
    /// `Game` does not report its frame length yet, so nothing calls this outside the tests.
    #[allow(dead_code)]
    pub fn set_frame_lines(&mut self, lines: u32) {
        self.lines = lines;
    }

    /// How many whole frames to step now that the clock reads `samples` host samples: every frame
    /// that has wholly elapsed and is not yet stepped. They are counted as stepped.
    pub fn due(&mut self, samples: u64) -> u32 {
        let now = samples as u128 * PAULA_CLOCK_PAL as u128 / self.host_rate as u128;
        let frame = (self.lines * CLOCKS_PER_LINE) as u64;
        let mut n = 0;
        while (self.stepped + frame) as u128 <= now {
            self.stepped += frame;
            n += 1;
        }
        n
    }
}

/// What the window shows: the newest completed frame, whole (DESIGN §2.5). A refresh between two
/// steps shows the earlier frame again, never a blend of the two.
#[derive(Debug, Default)]
pub struct Presenter<F> {
    newest: Option<F>,
}

impl<F> Presenter<F> {
    pub fn completed(&mut self, frame: F) {
        self.newest = Some(frame);
    }

    pub fn at_refresh(&self) -> Option<&F> {
        self.newest.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten seconds of a clock advancing `chunk` samples at a time.
    fn steps_over_10s(rate: u32, chunk: u64) -> u32 {
        let mut p = Pacer::new(rate);
        let end = rate as u64 * 10;
        let mut t = 0;
        let mut n = 0;
        while t < end {
            t = (t + chunk).min(end);
            n += p.due(t);
        }
        n
    }

    #[test]
    fn ten_seconds_of_audio_clock_is_499_or_500_pal_frames() {
        for rate in [44_100, 48_000, 96_000] {
            for chunk in [1, 64, 512, 4096, 48_000] {
                let n = steps_over_10s(rate, chunk);
                assert!((499..=500).contains(&n), "{rate} Hz in {chunk}s: {n} steps");
            }
        }
    }

    #[test]
    fn steps_come_whole_and_only_once_their_frame_has_elapsed() {
        let mut p = Pacer::new(48_000);
        // One PAL frame is 961.5… samples at 48 kHz.
        assert_eq!(p.due(961), 0);
        assert_eq!(p.due(962), 1);
        assert_eq!(p.due(962), 0, "a frame is stepped once");
        assert_eq!(p.due(962 * 5), 4);
    }

    #[test]
    fn short_frames_run_a_little_faster() {
        let mut p = Pacer::new(48_000);
        p.set_frame_lines(312);
        assert_eq!(p.due(48_000 * 10), 500, "312-line frames: 50.08 Hz");
    }

    #[test]
    fn the_presenter_shows_the_newest_whole_frame_at_every_refresh() {
        // 60 Hz refreshes against 49.92 Hz steps: each refresh shows a frame that was stepped,
        // the newest one, and never one in between.
        let mut p = Pacer::new(48_000);
        let mut shown = Presenter::default();
        let mut stepped = 0u32;
        let mut last = None;
        for refresh in 1..=600u64 {
            for _ in 0..p.due(refresh * 800) {
                stepped += 1;
                shown.completed(stepped);
            }
            let s = shown.at_refresh().copied();
            assert_eq!(s, (stepped > 0).then_some(stepped));
            assert!(s >= last);
            last = s;
        }
        assert!((499..=500).contains(&stepped));
    }
}
