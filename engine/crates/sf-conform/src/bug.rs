//! A bug zip replayed headlessly (bug-recorder DESIGN §2.5, §2.7): the recording read out of the
//! zip, the engine started as it says (`Game::start_level`), stepped through its inputs unpaced as
//! the app plays them (`Game::step`, one a frame), and its last picture and place held against the
//! owner's screenshot and `stopped-at`. A mismatch is a result, not an error: on a fixed engine it
//! is the point. `sf-conform bug replay` (main.rs) is the CLI.

use sf_core::recording::Location;
use sf_core::recording::{Recording, ZIP_RECORDING, ZIP_SCREENSHOT};
use sf_core::{Game, Picture, View};

/// The largest entry read out of a bug zip. `recording.txt` at `MAX_FRAMES` with the input
/// changing every frame is ~60 MB; a 1× screenshot is well under 1 MB.
const MAX_ENTRY: usize = 256 << 20;

/// The recording and the screenshot's PNG bytes of a bug zip. Refuses, naming why, anything that
/// is not a zip, lacks either entry, holds a screenshot that is not a PNG (refused here, not
/// after a replay that may run minutes), or holds a recording the parser or
/// [`Recording::validate`] refuses (a level no game starts at included, which `Game::start_level`
/// could not reach).
pub fn read_zip(bytes: &[u8]) -> Result<(Recording, Vec<u8>), String> {
    let entries = sf_disk::zip::read_entries(
        bytes,
        |n| n == ZIP_RECORDING || n == ZIP_SCREENSHOT,
        MAX_ENTRY,
    )
    .map_err(|e| format!("not a bug zip: {e}"))?;
    let take = |name: &str| {
        entries
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.bytes.clone())
            .ok_or_else(|| format!("not a bug zip: it has no {name}"))
    };
    let text = take(ZIP_RECORDING)?;
    let png = take(ZIP_SCREENSHOT)?;
    let text =
        String::from_utf8(text).map_err(|_| format!("{ZIP_RECORDING}: it is not UTF-8 text"))?;
    let rec = Recording::parse(&text).map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    rec.validate()
        .map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    decode_png(&png).map_err(|e| format!("{ZIP_SCREENSHOT}: it is not a PNG: {e}"))?;
    Ok((rec, png))
}

/// A recording played to its end.
pub struct Replayed {
    pub game: Game,
    /// [`Game::location`] after the last step.
    pub location: Location,
    /// The last step's picture (the level's first frame for a recording of no frames): what the app
    /// showed when STOP was pressed.
    pub last: Picture,
}

/// `rec` played as the app played it: [`Game::start_level`] with its setup and seed, then one
/// [`Game::step`] per recorded input. `each` sees every step's frame number (0 the first input's),
/// the game after it and the picture it drew, which is not always `Game::render`'s (a level frame
/// whose pass ends mid-frame shows the picture drawn then). The 4:3 view: the screenshot is the
/// core's picture at 1× (DESIGN §2.5).
pub fn replay(rec: &Recording, mut each: impl FnMut(u32, &Game, &Picture)) -> Replayed {
    let mut game = Game::start_level(crate::display_data(), &rec.setup, rec.seed);
    let mut last = game.render(View::default());
    for (k, &input) in rec.inputs.iter().enumerate() {
        let out = game.step(input);
        each(k as u32, &game, &out.picture);
        last = out.picture;
    }
    Replayed {
        location: game.location(),
        game,
        last,
    }
}

/// `pic` as 8-bit RGB, each 4-bit channel times 17, as the app shows it (`sf_app::video`).
pub fn picture_rgb(pic: &Picture) -> Vec<u8> {
    sf_core::to_rgb12(pic)
        .into_iter()
        .flat_map(|c| {
            [
                (c >> 8 & 15) as u8 * 17,
                (c >> 4 & 15) as u8 * 17,
                (c & 15) as u8 * 17,
            ]
        })
        .collect()
}

/// `pic` as a PNG.
pub fn picture_png(pic: &Picture) -> Vec<u8> {
    crate::report::png(pic.width as u32, pic.height as u32, &picture_rgb(pic))
}

/// How many pixels of the screenshot `png` differ from `pic`: 0 is equal. A screenshot of another
/// size differs in every pixel of the larger of the two. `Err` when `png` does not decode.
pub fn screenshot_differs(png: &[u8], pic: &Picture) -> Result<u64, String> {
    let (w, h, rgb) = decode_png(png).map_err(|e| format!("{ZIP_SCREENSHOT}: {e}"))?;
    let (pw, ph) = (pic.width as u32, pic.height as u32);
    if (w, h) != (pw, ph) {
        return Ok(u64::from(w * h).max(u64::from(pw * ph)));
    }
    let ours = picture_rgb(pic);
    Ok(rgb
        .chunks_exact(3)
        .zip(ours.chunks_exact(3))
        .filter(|(a, b)| a != b)
        .count() as u64)
}

/// A PNG as (width, height, 8-bit RGB). The app writes RGB (`sf_app::capture::png`); any other
/// colour type a PNG may hold is taken too, alpha dropped.
fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; r.output_buffer_size().ok_or("the PNG is too large")?];
    let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgb = match info.color_type {
        png::ColorType::Rgb => buf,
        png::ColorType::Rgba => buf
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g]).collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0]])
            .collect(),
        png::ColorType::Indexed => {
            return Err("an indexed PNG, which EXPAND should have widened".into());
        }
    };
    Ok((info.width, info.height, rgb))
}

/// The frames `bug replay` writes: `from..to` (to excluded), every `every`th from `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePick {
    pub from: u32,
    pub to: u32,
    pub every: u32,
}

/// How many frames `bug replay` writes when `--frames` is not given: the last this many.
pub const DEFAULT_LAST: u32 = 250;

impl FramePick {
    /// `--frames A..B` (`None`: the last [`DEFAULT_LAST`] of `frames`) and `--every N` (`None`:
    /// 1), refused when not numbers, `A > B` or `N` is 0. A range past the recording's end writes
    /// only the frames it has.
    pub fn parse(
        range: Option<&str>,
        every: Option<&str>,
        frames: u32,
    ) -> Result<FramePick, String> {
        let (from, to) = match range {
            None => (frames.saturating_sub(DEFAULT_LAST), frames),
            Some(r) => {
                let bad =
                    || format!("--frames {r:?}: expected A..B, frames from A up to B, B left out");
                let (a, b) = r.split_once("..").ok_or_else(bad)?;
                let (a, b): (u32, u32) =
                    (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
                if a > b {
                    return Err(bad());
                }
                (a, b)
            }
        };
        let every = match every {
            None => 1,
            Some(n) => n
                .parse()
                .ok()
                .filter(|&n: &u32| n > 0)
                .ok_or_else(|| format!("--every {n:?}: expected a whole number above 0"))?,
        };
        Ok(FramePick { from, to, every })
    }

    pub fn wants(&self, frame: u32) -> bool {
        (self.from..self.to).contains(&frame) && (frame - self.from) % self.every == 0
    }
}

/// `bug replay`'s default folder for `zip`'s frames: `~/.cache/superfrog-re/bugs/<zip stem>/`,
/// outside the repo (the pictures are disk-derived, RE §2.3).
pub fn default_out(zip: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set; give --out DIR")?;
    let stem = zip.file_stem().ok_or_else(|| {
        format!(
            "{}: no file name to name the folder by; give --out DIR",
            zip.display()
        )
    })?;
    Ok(std::path::PathBuf::from(home)
        .join(".cache/superfrog-re/bugs")
        .join(stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_picks() {
        let p = FramePick::parse(None, None, 1000).unwrap();
        assert_eq!(
            p,
            FramePick {
                from: 750,
                to: 1000,
                every: 1
            }
        );
        assert_eq!(FramePick::parse(None, None, 10).unwrap().from, 0);
        let p = FramePick::parse(Some("10..20"), Some("5"), 1000).unwrap();
        let got: Vec<u32> = (0..1000).filter(|&f| p.wants(f)).collect();
        assert_eq!(got, [10, 15]);
        for (r, e) in [
            (Some("20..10"), None),
            (Some("x..2"), None),
            (Some("5"), None),
            (None, Some("0")),
        ] {
            assert!(FramePick::parse(r, e, 100).is_err(), "{r:?} {e:?}");
        }
    }

    #[test]
    fn a_screenshot_of_another_size_differs_everywhere() {
        let pic = Picture::blank(0, 0, 4, 2);
        let same = picture_png(&pic);
        assert_eq!(screenshot_differs(&same, &pic).unwrap(), 0);
        let other = picture_png(&Picture::blank(0, 0, 5, 3));
        assert_eq!(screenshot_differs(&other, &pic).unwrap(), 15);
        let mut lit = pic.clone();
        lit.palettes[1][0] = 0xFFF;
        assert_eq!(screenshot_differs(&picture_png(&lit), &pic).unwrap(), 4);
        assert!(screenshot_differs(b"not a png", &pic).is_err());
    }

    #[test]
    fn refusals_name_why() {
        let e = read_zip(b"hello").err().unwrap();
        assert!(e.starts_with("not a bug zip"), "{e}");
        let z = sf_disk::zip::write_stored(&[(ZIP_SCREENSHOT, b"x")]);
        assert!(read_zip(&z).err().unwrap().contains("no recording.txt"));
        let z = sf_disk::zip::write_stored(&[
            (ZIP_RECORDING, b"superfrog recording 2\n"),
            (ZIP_SCREENSHOT, b"x"),
        ]);
        let e = read_zip(&z).err().unwrap();
        assert!(e.starts_with("recording.txt: line 1"), "{e}");
    }
}
