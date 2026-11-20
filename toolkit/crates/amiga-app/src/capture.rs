//! What a run showed and played, for an app's end-to-end tests and drills: the files a capture
//! folder holds. Their names are the constants and functions below, which a test driver reads, so
//! the two cannot drift apart. A game adds its own files beside these (its state hashes, its RNG
//! seed, what it met and does not follow).
//!
//! - `frame-NNNNN.png` ([`frame_file`]): frame N exactly as it went to the GPU at 1× (frame 0 is
//!   the start, before any step; frame N the picture after step N), RGB, 8 bits a channel (12-bit
//!   colour × 17).
//! - `audio.wav`: everything the mixer produced, 16-bit stereo PCM at the run's host rate.
//! - `audio-frames.txt`: `N c…` for each step N, the samples in `audio.wav` of each game frame it
//!   ran (one, or more when the steps are paced from a recorded run), in order, so a test can line
//!   the sound up with the frames.
//! - `inputs.txt`: `N joy keys` for each step N whose input was not empty (joy bits in hex, keys
//!   as the held Amiga raw codes in hex), so a test can see what reached the game.
//! - `window-NNNNN.png` ([`window_file`]) for each shot at step N: the whole surface as presented,
//!   in its own pixels; `windows.txt` a line for each, `N frame width height scale_factor x y w h`
//!   (the frame it showed, the surface, its pixels a point, and the picture's place on it).
//! - `prompts.txt`: every question the launch asked a scripted run (written by the app).
//! - `pacing.txt`: `stepped N`, `presented N` (the last frame presented), `dropped N` (frames
//!   stepped after the first presented that never reached the window) and `clock N` (the host
//!   samples the pacing clock had counted when the run ended), one a line.
//! - `clock.txt`: `N samples` for each step N, the host samples the pacing clock had counted
//!   when it ran, so a test can see whether the steps drift from the clock.
//! - `memory.txt`: `footprint N` and `peak N`, the process's physical footprint in bytes at the
//!   end and at its largest (GPU memory included).

use std::io::Write;
use std::path::{Path, PathBuf};

use amiga_input::FrameInput;

use crate::video::Shown;

pub const AUDIO: &str = "audio.wav";
pub const AUDIO_FRAMES: &str = "audio-frames.txt";
pub const INPUTS: &str = "inputs.txt";
pub const WINDOWS: &str = "windows.txt";
pub const PROMPTS: &str = "prompts.txt";
pub const PACING: &str = "pacing.txt";
pub const CLOCK: &str = "clock.txt";
pub const MEMORY: &str = "memory.txt";

/// Frame `n`'s picture: `frame-NNNNN.png`.
pub fn frame_file(n: u64) -> String {
    format!("frame-{n:05}.png")
}

/// The shot at step `n`: `window-NNNNN.png`.
pub fn window_file(n: u64) -> String {
    format!("window-{n:05}.png")
}

pub struct Capture {
    dir: PathBuf,
    rate: u32,
    audio: Vec<[i16; 2]>,
    audio_frames: String,
    clock: String,
    inputs: String,
}

impl Capture {
    /// A capture into `dir`, made if missing; the audio is at `rate` samples a second.
    pub fn new(dir: &Path, rate: u32) -> std::io::Result<Capture> {
        std::fs::create_dir_all(dir)?;
        Ok(Capture {
            dir: dir.to_path_buf(),
            rate,
            audio: Vec::new(),
            audio_frames: String::new(),
            clock: String::new(),
            inputs: String::new(),
        })
    }

    /// The capture folder, where a game writes its own files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Frame `n`'s PNG, of the picture `(width, height, rgba)`.
    pub fn frame(&mut self, n: u64, (width, height, rgba): (u32, u32, &[u8])) -> std::io::Result<()> {
        std::fs::write(self.dir.join(frame_file(n)), png(width, height, rgba))
    }

    pub fn window_path(&self, n: u64) -> PathBuf {
        self.dir.join(window_file(n))
    }

    /// Note what the shot asked for at step `n` showed: frame `frame`.
    pub fn window_shown(&mut self, n: u64, frame: u64, s: &Shown) -> std::io::Result<()> {
        let (x, y, w, h) = s.viewport;
        let line = format!(
            "{n} {frame} {} {} {} {x} {y} {w} {h}\n",
            s.surface.0, s.surface.1, s.scale_factor
        );
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(WINDOWS))
            .and_then(|mut f| f.write_all(line.as_bytes()))
    }

    /// Step `n`'s sound: `samples`, the game frames it ran ending at each of `ends` in them.
    pub fn audio(&mut self, n: u64, samples: &[[i16; 2]], ends: &[usize]) {
        self.audio.extend_from_slice(samples);
        self.audio_frames.push_str(&n.to_string());
        let mut from = 0;
        for &e in ends {
            self.audio_frames.push_str(&format!(" {}", e - from));
            from = e;
        }
        self.audio_frames.push('\n');
    }

    /// Step `n` ran with the pacing clock at `samples`.
    pub fn clock(&mut self, n: u64, samples: u64) {
        self.clock.push_str(&format!("{n} {samples}\n"));
    }

    pub fn input(&mut self, n: u64, f: FrameInput) {
        if f != FrameInput::NONE {
            let keys: Vec<String> = f.keys.codes().map(|c| format!("{c:02x}")).collect();
            self.inputs
                .push_str(&format!("{n} {:02x} {}\n", f.joy.bits(), keys.join(",")));
        }
    }

    /// Write `pacing.txt`.
    pub fn pacing(&self, stepped: u64, presented: u64, dropped: u64, clock: u64) -> std::io::Result<()> {
        std::fs::write(
            self.dir.join(PACING),
            format!("stepped {stepped}\npresented {presented}\ndropped {dropped}\nclock {clock}\n"),
        )
    }

    /// Write `memory.txt`: `footprint N` and `peak N`, the process's physical footprint now and
    /// at its largest, in bytes (the GPU's memory counts).
    pub fn memory(&self) -> std::io::Result<()> {
        // SAFETY: proc_pid_rusage fills the struct it is given for the flavour named.
        let mut ri: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
        let got = unsafe {
            libc::proc_pid_rusage(
                std::process::id() as i32,
                libc::RUSAGE_INFO_V4,
                (&mut ri as *mut libc::rusage_info_v4).cast(),
            )
        };
        if got != 0 {
            return Err(std::io::Error::last_os_error());
        }
        std::fs::write(
            self.dir.join(MEMORY),
            format!("footprint {}\npeak {}\n", ri.ri_phys_footprint, ri.ri_lifetime_max_phys_footprint),
        )
    }

    /// Write the audio and the inputs. The frames are already on disk.
    pub fn finish(&mut self) -> std::io::Result<()> {
        std::fs::write(self.dir.join(AUDIO), wav(self.rate, &self.audio))?;
        std::fs::write(self.dir.join(AUDIO_FRAMES), &self.audio_frames)?;
        std::fs::write(self.dir.join(CLOCK), &self.clock)?;
        std::fs::write(self.dir.join(INPUTS), &self.inputs)
    }
}

/// An RGB PNG of `rgba` (alpha dropped: the game has none).
pub fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, width, height);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        e.set_compression(png::Compression::Fast);
        let mut w = e.write_header().expect("PNG header");
        let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        w.write_image_data(&rgb).expect("PNG data");
    }
    out
}

/// A 16-bit stereo PCM WAV file.
pub fn wav(rate: u32, samples: &[[i16; 2]]) -> Vec<u8> {
    let data = samples.len() as u32 * 4;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.write_all(b"RIFF").unwrap();
    out.write_all(&(36 + data).to_le_bytes()).unwrap();
    out.write_all(b"WAVEfmt ").unwrap();
    out.write_all(&16u32.to_le_bytes()).unwrap();
    out.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    out.write_all(&2u16.to_le_bytes()).unwrap(); // channels
    out.write_all(&rate.to_le_bytes()).unwrap();
    out.write_all(&(rate * 4).to_le_bytes()).unwrap(); // bytes a second
    out.write_all(&4u16.to_le_bytes()).unwrap(); // bytes a sample frame
    out.write_all(&16u16.to_le_bytes()).unwrap();
    out.write_all(b"data").unwrap();
    out.write_all(&data.to_le_bytes()).unwrap();
    for [l, r] in samples {
        out.write_all(&l.to_le_bytes()).unwrap();
        out.write_all(&r.to_le_bytes()).unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_input::{Joy, KeySet};

    #[test]
    fn a_wav_has_the_canonical_header_and_the_samples() {
        let w = wav(48_000, &[[1, -1], [2, -2]]);
        assert_eq!(w.len(), 44 + 8);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), 48_000);
        assert_eq!(&w[44..48], &[1, 0, 0xff, 0xff]);
    }

    #[test]
    fn a_png_decodes_to_the_same_pixels() {
        let rgba = [255, 0, 17, 255, 0, 34, 0, 255];
        let bytes = png(2, 1, &rgba);
        let mut r = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
        let mut buf = vec![0; r.output_buffer_size().unwrap()];
        r.next_frame(&mut buf).unwrap();
        assert_eq!(&buf[..6], &[255, 0, 17, 0, 34, 0]);
    }

    #[test]
    fn inputs_list_only_frames_with_input() {
        let dir = std::env::temp_dir().join(format!("amiga-app-capture-{}", std::process::id()));
        let mut c = Capture::new(&dir, 48_000).unwrap();
        c.input(1, FrameInput::NONE);
        c.input(2, FrameInput {
            joy: Joy::LEFT | Joy::FIRE,
            keys: KeySet::key(0x19).with(0x45),
        });
        c.finish().unwrap();
        let text = std::fs::read_to_string(dir.join("inputs.txt")).unwrap();
        assert_eq!(text, "2 14 19,45\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_file_names_are_the_ones_a_driver_reads() {
        assert_eq!(frame_file(7), "frame-00007.png");
        assert_eq!(window_file(12345), "window-12345.png");
        assert_eq!([AUDIO, INPUTS, WINDOWS, PROMPTS], ["audio.wav", "inputs.txt", "windows.txt", "prompts.txt"]);
    }
}
