//! Shell, test tooling: end-to-end tests driving a game's built app.
//!
//! [`App::launch`] makes a temporary home the game's [`AppSpec::provision`] fills (its disks, an
//! imported cache). Then either layer drives the real binary:
//!
//! - the script layer, [`App::run_script`]: the app runs a script of host events (they enter where
//!   the window system's are translated) and writes every frame, the mixed audio and the inputs it
//!   saw, under the names in `amiga_app::capture`;
//! - the OS layer ([`LaunchOpts::os`], `os.rs`): real key events through CGEvent, real menu clicks
//!   through the Accessibility API and a capture of the real window. It needs Accessibility and
//!   Screen Recording for the terminal.
//!
//! Every launch passes the spec's home, frame-cap and timeout flags, and puts the app in its own
//! process group, so a run always ends and leaves nothing behind.

mod app;
mod os;

pub use app::{App, Captured, LaunchOpts, Shot, binary, group_alive, read_wav, run_waiting_for_lock};
pub use os::{Key, MenuItemState, Mods, os_permissions};

use std::io;
use std::path::{Path, PathBuf};

/// The app under test, as the game names it. The harness passes every flag by these names and
/// never hard-codes one.
pub struct AppSpec {
    /// The cargo package that builds the app.
    pub package: &'static str,
    /// The binary's name, built with `cargo build -p <package> --bin <binary>`.
    pub binary: &'static str,
    /// The cargo workspace the binary is built in.
    pub workspace: PathBuf,
    /// The flags the harness passes on every launch.
    pub flags: Flags,
    /// Environment variables removed from the app's environment, so a person's own setting (a
    /// home folder, say) cannot reach a test run.
    pub clear_env: Vec<&'static str>,
    /// Set to `1` to keep the temporary homes and their captures.
    pub keep_env: &'static str,
    /// The temporary homes' name prefix, under the system temp folder.
    pub scratch: &'static str,
    /// Fills a fresh temporary home before a [`App::launch`]: disks, an imported cache.
    pub provision: fn(home: &Path) -> io::Result<()>,
    /// What the app prints when another copy holds its home's lock; a launch that says it is
    /// retried ([`run_waiting_for_lock`]).
    pub already_running: &'static str,
}

/// The app's flag names, each one given its value by the harness.
pub struct Flags {
    /// Takes the temporary home folder.
    pub home: &'static str,
    /// Takes the capture folder.
    pub capture: &'static str,
    /// Takes the number of steps after which the app ends itself with status 0.
    pub max_frames: &'static str,
    /// Takes the seconds after which the app stops its whole process group.
    pub timeout: &'static str,
    /// Takes the window's integer scale.
    pub scale: &'static str,
    /// Takes the script file.
    pub script: &'static str,
}

/// An RGBA picture: a captured frame, or the window as the screen showed it.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// Row-major RGBA, 8 bits a channel.
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image({}×{})", self.width, self.height)
    }
}

impl Image {
    pub fn read_png(path: &Path) -> Result<Image, String> {
        let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut dec = png::Decoder::new(std::io::BufReader::new(file));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut r = dec
            .read_info()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let mut buf = vec![0; r.output_buffer_size().ok_or("PNG too large")?];
        let info = r
            .next_frame(&mut buf)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        buf.truncate(info.buffer_size());
        let rgba = match info.color_type {
            png::ColorType::Rgba => buf,
            png::ColorType::Rgb => buf.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
            png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::GrayscaleAlpha => {
                buf.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect()
            }
            png::ColorType::Indexed => return Err(format!("{}: indexed PNG", path.display())),
        };
        Ok(Image {
            width: info.width,
            height: info.height,
            rgba,
        })
    }

    /// The RGB at (`x`, `y`).
    pub fn rgb(&self, x: u32, y: u32) -> [u8; 3] {
        let i = (y * self.width + x) as usize * 4;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]]
    }

    /// An image of what an engine's picture shows, as the app presents it (12-bit colour × 17).
    pub fn of_picture(p: &amiga_chips::Picture) -> Image {
        let rgba = amiga_chips::to_rgb12(p)
            .into_iter()
            .flat_map(|c| {
                let ch = |s: u16| ((c >> s) & 0xF) as u8 * 17;
                [ch(8), ch(4), ch(0), 255]
            })
            .collect();
        Image {
            width: p.width as u32,
            height: p.height as u32,
            rgba,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("amiga-e2e-lib-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn an_rgb_png_reads_back_as_opaque_rgba() {
        let path = temp("rgb.png");
        let mut enc = png::Encoder::new(std::fs::File::create(&path).unwrap(), 2, 1);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&[1, 2, 3, 4, 5, 6]).unwrap();
        let img = Image::read_png(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(img.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
        assert_eq!(img.rgb(1, 0), [4, 5, 6]);
    }

    #[test]
    fn a_missing_png_names_its_path() {
        let path = temp("missing.png");
        let e = Image::read_png(&path).unwrap_err();
        assert!(e.starts_with(&path.display().to_string()), "{e}");
    }
}
