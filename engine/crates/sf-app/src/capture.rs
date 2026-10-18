//! `--capture DIR`: what the run showed and played, for the end-to-end tests and the drills
//! (DESIGN §5, End to end).
//!
//! - `frame-NNNNN.png`: frame N exactly as it went to the GPU at 1× (frame 0 is the start, before
//!   any step; frame N the picture after step N), RGB, 8 bits a channel (12-bit colour × 17).
//! - `audio.wav`: everything the mixer produced, 16-bit stereo PCM at the run's host rate.
//! - `inputs.txt`: `N joy keys` for each step N whose input was not empty (joy bits in hex, keys
//!   as the held Amiga raw codes in hex), so a test can see what reached the game.
//! - `seed.txt`: the power-on RNG seed, `A B`, so a run seeded from the clock can be replayed.
//! - `window-NNNNN.png` for each script `shot` at step N: the whole surface as presented, in its
//!   own pixels; `windows.txt` a line for each, `N frame width height scale_factor x y w h` (the
//!   frame it showed, the surface, its pixels a point, and the picture's place on it).
//! - `prompts.txt`: every question the launch asked a scripted run (`import.rs`).

use std::io::Write;
use std::path::{Path, PathBuf};

use sf_core::input::FrameInput;

pub struct Capture {
    dir: PathBuf,
    rate: u32,
    audio: Vec<[i16; 2]>,
    inputs: String,
}

impl Capture {
    pub fn new(dir: &Path, rate: u32, seed: (u32, u32)) -> std::io::Result<Capture> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("seed.txt"), format!("{} {}\n", seed.0, seed.1))?;
        Ok(Capture {
            dir: dir.to_path_buf(),
            rate,
            audio: Vec::new(),
            inputs: String::new(),
        })
    }

    pub fn frame(&mut self, n: u64, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
        let path = self.dir.join(format!("frame-{n:05}.png"));
        std::fs::write(path, png(width, height, rgba))
    }

    pub fn window_path(&self, n: u64) -> PathBuf {
        self.dir.join(format!("window-{n:05}.png"))
    }

    /// Note what the shot asked for at step `n` showed: frame `frame`.
    pub fn window_shown(&mut self, n: u64, frame: u64, s: &crate::video::Shown) -> std::io::Result<()> {
        let (x, y, w, h) = s.viewport;
        let line = format!(
            "{n} {frame} {} {} {} {x} {y} {w} {h}\n",
            s.surface.0, s.surface.1, s.scale_factor
        );
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("windows.txt"))
            .and_then(|mut f| f.write_all(line.as_bytes()))
    }

    pub fn audio(&mut self, samples: &[[i16; 2]]) {
        self.audio.extend_from_slice(samples);
    }

    pub fn input(&mut self, n: u64, f: FrameInput) {
        if f != FrameInput::NONE {
            let keys: Vec<String> = f.keys.codes().map(|c| format!("{c:02x}")).collect();
            self.inputs
                .push_str(&format!("{n} {:02x} {}\n", f.joy.bits(), keys.join(",")));
        }
    }

    /// Write the audio and the inputs. The frames are already on disk.
    pub fn finish(&self) -> std::io::Result<()> {
        std::fs::write(self.dir.join("audio.wav"), wav(self.rate, &self.audio))?;
        std::fs::write(self.dir.join("inputs.txt"), &self.inputs)
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
    use sf_core::input::{Joy, KeySet};

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
        let dir = std::env::temp_dir().join(format!("sf-app-capture-{}", std::process::id()));
        let mut c = Capture::new(&dir, 48_000, (1, 2)).unwrap();
        c.input(1, FrameInput::NONE);
        c.input(2, FrameInput {
            joy: Joy::LEFT | Joy::FIRE,
            keys: KeySet::key(0x19).with(0x45),
        });
        c.finish().unwrap();
        let text = std::fs::read_to_string(dir.join("inputs.txt")).unwrap();
        assert_eq!(text, "2 14 19,45\n");
        assert_eq!(std::fs::read_to_string(dir.join("seed.txt")).unwrap(), "1 2\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
