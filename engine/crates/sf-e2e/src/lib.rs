//! Shell, test tooling: end-to-end tests driving the built app (DESIGN §5, End to end).
//!
//! [`App::launch`] makes a temporary home with the owner's disks imported into it. Then either
//! layer drives the real `superfrog` binary:
//!
//! - the script layer, [`App::run_script`]: the app runs a script of host events (they enter where
//!   winit's are translated) and writes every frame, the mixed audio and the inputs it saw;
//! - the OS layer ([`LaunchOpts::os`], `os.rs`): real key events through CGEvent, real menu clicks
//!   through the Accessibility API and a capture of the real window. It needs Accessibility and Screen
//!   Recording for the terminal, and its tests run only under `SF_E2E_OS=1` (README.md).
//!
//! Every launch passes `--home`, `--max-frames` and `--timeout`, and puts the app in its own
//! process group, so a run always ends and leaves nothing behind (DESIGN §5.2).

mod app;
mod os;

pub use app::{App, Captured, LaunchOpts, binary, group_alive, read_wav};
pub use os::{Key, os_permissions};

use std::path::Path;

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

    /// An image of what the engine's picture shows, as the app presents it (12-bit colour × 17).
    pub fn of_picture(p: &sf_core::Picture) -> Image {
        let rgba = sf_core::to_rgb12(p)
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
