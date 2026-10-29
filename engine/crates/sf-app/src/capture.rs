//! `--capture DIR`: what the run showed and played, for the end-to-end tests and the drills
//! (DESIGN §5, End to end). The game-free files (frames, `audio.wav`, `audio-frames.txt`,
//! `inputs.txt`, `window-NNNNN.png` and `windows.txt`, `prompts.txt`, `pacing.txt`, `clock.txt`,
//! `memory.txt`) and their names are `amiga_app::capture`'s; this adds Superfrog's own:
//!
//! - `hashes.txt` in place of the PNGs under `--capture-hashes` (T211): each frame's picture and
//!   state hashes, `sf_core::feed`'s format, as `sf-conform` writes them for its engine run alone; a
//!   long run's PNGs would be gigabytes.
//! - `seed.txt`: the power-on RNG seed, `A B`, so a run seeded from the clock can be replayed.
//! - `unported.txt`: what the game met and does not follow (`Game::unported`), one a line; empty
//!   when nothing.

use std::io::Write;
use std::path::Path;

pub use amiga_app::capture::png;

pub struct Capture {
    base: amiga_app::capture::Capture,
    /// `--capture-hashes`: `hashes.txt`, written as the run goes, in place of the PNGs.
    hashes: Option<std::io::BufWriter<std::fs::File>>,
}

/// The game-free files are written through the toolkit's capture.
impl std::ops::Deref for Capture {
    type Target = amiga_app::capture::Capture;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for Capture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Capture {
    pub fn new(dir: &Path, rate: u32, seed: (u32, u32), hashes: bool) -> std::io::Result<Capture> {
        let base = amiga_app::capture::Capture::new(dir, rate)?;
        std::fs::write(dir.join("seed.txt"), format!("{} {}\n", seed.0, seed.1))?;
        let hashes = match hashes {
            false => None,
            true => {
                let mut w = std::io::BufWriter::new(std::fs::File::create(dir.join("hashes.txt"))?);
                writeln!(w, "{}", sf_core::feed::HASHES_HEADER)?;
                Some(w)
            }
        };
        Ok(Capture { base, hashes })
    }

    /// Frame `n`: its PNG, or under `--capture-hashes` its hashes, of `game` and the picture it
    /// showed.
    pub fn frame(
        &mut self,
        n: u64,
        image: (u32, u32, &[u8]),
        game: &sf_core::Game,
        picture: &sf_core::Picture,
    ) -> std::io::Result<()> {
        if let Some(w) = self.hashes.as_mut() {
            w.write_all(sf_core::feed::hash_line(n, sf_core::feed::frame_hashes(game, picture)).as_bytes())?;
            // Flushed now and then, so a run stopped short still says how far it matched.
            if n.is_multiple_of(500) {
                w.flush()?;
            }
            return Ok(());
        }
        self.base.frame(n, image)
    }

    /// Write `unported.txt`.
    pub fn unported(&self, unported: &[&str]) -> std::io::Result<()> {
        let lines: String = unported.iter().map(|u| format!("{u}\n")).collect();
        std::fs::write(self.base.dir().join("unported.txt"), lines)
    }

    /// Write the audio and the inputs. The frames are already on disk.
    pub fn finish(&mut self) -> std::io::Result<()> {
        if let Some(w) = self.hashes.as_mut() {
            w.flush()?;
        }
        self.base.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_is_written_beside_the_toolkit_s_files() {
        let dir = std::env::temp_dir().join(format!("sf-app-capture-{}", std::process::id()));
        let mut c = Capture::new(&dir, 48_000, (1, 2), false).unwrap();
        c.input(2, sf_core::input::FrameInput::key(0x45));
        c.unported(&["a", "b"]).unwrap();
        c.finish().unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("seed.txt")).unwrap(), "1 2\n");
        assert_eq!(std::fs::read_to_string(dir.join("inputs.txt")).unwrap(), "2 00 45\n");
        assert_eq!(std::fs::read_to_string(dir.join("unported.txt")).unwrap(), "a\nb\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
