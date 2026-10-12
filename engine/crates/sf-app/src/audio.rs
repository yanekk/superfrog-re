//! Sound to the speakers (DESIGN §2.5, §2.11): the mixer's samples go into a queue the audio
//! device drains, and the samples the device has asked for are the clock that paces the game.
//! No device, or a device that fails, leaves the game running silently on the wall clock.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Where the pacer reads the time, in host samples.
pub enum Clock {
    /// The audio device's samples played.
    Device(Arc<AtomicU64>),
    /// The wall clock since the start, at `rate` samples a second.
    Wall { start: Instant, rate: u32 },
}

impl Clock {
    pub fn samples(&self) -> u64 {
        match self {
            Clock::Device(played) => played.load(Ordering::Acquire),
            Clock::Wall { start, rate } => {
                (start.elapsed().as_nanos() * *rate as u128 / 1_000_000_000) as u64
            }
        }
    }
}

/// The open audio device.
pub struct Audio {
    pub rate: u32,
    queue: Arc<Mutex<VecDeque<[i16; 2]>>>,
    played: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
    _stream: cpal::Stream,
}

impl Audio {
    /// The default output device, or `Err` saying why there is none.
    pub fn open() -> Result<Audio, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no audio output device")?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("audio device: {e}"))?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let channels = config.channels as usize;
        let rate = config.sample_rate.0;
        let queue = Arc::new(Mutex::new(VecDeque::<[i16; 2]>::new()));
        let played = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let (q, p, f) = (queue.clone(), played.clone(), failed.clone());
        let on_error = move |e| {
            eprintln!("superfrog: audio device: {e}; carrying on silently");
            f.store(true, Ordering::Release);
        };
        // The device asks for `n` sample frames; each one is a frame of the clock, whether the
        // queue had a sample for it or not.
        macro_rules! stream {
            ($t:ty, $conv:expr) => {
                device.build_output_stream(
                    &config,
                    move |out: &mut [$t], _| {
                        let mut q = q.lock().unwrap();
                        for frame in out.chunks_mut(channels) {
                            let [l, r] = q.pop_front().unwrap_or([0, 0]);
                            for (c, s) in frame.iter_mut().enumerate() {
                                *s = $conv(if c % 2 == 0 { l } else { r });
                            }
                        }
                        p.fetch_add((out.len() / channels) as u64, Ordering::Release);
                    },
                    on_error,
                    None,
                )
            };
        }
        let stream = match format {
            cpal::SampleFormat::F32 => stream!(f32, |s: i16| s as f32 / 32768.0),
            cpal::SampleFormat::I16 => stream!(i16, |s: i16| s),
            other => return Err(format!("audio device wants {other} samples")),
        }
        .map_err(|e| format!("audio device: {e}"))?;
        stream.play().map_err(|e| format!("audio device: {e}"))?;
        Ok(Audio {
            rate,
            queue,
            played,
            failed,
            _stream: stream,
        })
    }

    pub fn clock(&self) -> Clock {
        Clock::Device(self.played.clone())
    }

    /// The device stopped working since it was opened.
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    pub fn push(&self, samples: &[[i16; 2]]) {
        self.queue.lock().unwrap().extend(samples);
    }
}

/// Chip RAM as the mixer's DMA reads it. `Game` does not run the sound driver yet (the first
/// mechanic to step it is T14), so there is no sample data to read and no write that starts DMA;
/// this reads zeros until `Game` exposes its chip view.
pub struct NoChip;

impl sf_paula::ChipMemory for NoChip {
    fn chip_word(&self, _addr: u32) -> u16 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wall_clock_counts_samples_at_its_rate() {
        let c = Clock::Wall {
            start: Instant::now() - std::time::Duration::from_secs(2),
            rate: 48_000,
        };
        let n = c.samples();
        assert!((96_000..96_000 + 48_000).contains(&n), "{n}");
    }
}
