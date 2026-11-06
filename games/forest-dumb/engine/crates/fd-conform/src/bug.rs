//! A bug zip replayed headlessly (DESIGN §2.8, §2.12): the recording read out of the zip, refused
//! when another build made it, the engine started at the level's start as the app starts it
//! (`Game::new(data, Start::Level1)`), stepped through its inputs unpaced as the app plays them
//! (one `Game::step` a frame), and its last picture held against the owner's screenshot. A
//! mismatch is a result, not an error: on a fixed engine it is the point. `fd-conform bug replay`
//! (main.rs) is the CLI; comparing with the original is T15's.

use std::sync::Arc;

use amiga_chips::Picture;
use fd_core::recording::{ForestDumb, Location, Recording, ZIP_SCREENSHOT, check_build};
use fd_core::{Game, GameData, Start};

pub use amiga_conform::screenshot::{picture_png, screenshot_differs};

/// The recording and the screenshot's PNG bytes of a bug zip (`amiga_record::read_zip`), refused
/// unless `build` made it and the screenshot is a PNG: here, before a replay that may run minutes.
pub fn open_zip(bytes: &[u8], build: &str) -> Result<(Recording, Vec<u8>), String> {
    let (rec, png) = amiga_record::read_zip::<ForestDumb>(bytes)?;
    amiga_conform::screenshot::decode_png(&png)
        .map_err(|e| format!("{ZIP_SCREENSHOT}: it is not a PNG: {e}"))?;
    check_build(&rec, build)?;
    Ok((rec, png))
}

/// A recording played to its end.
pub struct Replayed {
    pub game: Game,
    /// [`Game::location`] after the last step.
    pub location: Location,
    /// The last step's picture (the level's first frame for a recording of no frames): what the
    /// app showed when Stop was pressed.
    pub last: Picture,
}

/// `rec` played as the app played it, from the level's start. `each` sees every step's frame
/// number (0 the first input's), the game after it and its picture.
pub fn replay(
    data: Arc<GameData>,
    rec: &Recording,
    each: impl FnMut(u32, &Game, &Picture),
) -> Replayed {
    // The only level a recording names today; the reader refuses any other (`LevelId`).
    let mut game = Game::new(data, Start::Level1);
    let last = amiga_record::replay(&mut game, &rec.inputs, each);
    Replayed {
        location: game.location(),
        game,
        last,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_core::recording::{LevelId, recorder};

    fn zip(build: &str, png: &[u8]) -> Vec<u8> {
        let rec = recorder(LevelId::W1_1, build)
            .finish(
                Location {
                    level: LevelId::W1_1,
                    col: 7,
                    row: 7,
                },
                String::new(),
            )
            .unwrap();
        amiga_record::zip_bytes(&rec, png).unwrap()
    }

    #[test]
    fn a_zip_is_refused_naming_why() {
        let png = amiga_app_png();
        assert!(open_zip(&zip("b", &png), "b").is_ok());
        let e = open_zip(&zip("a", &png), "b").unwrap_err();
        assert!(e.contains("`a`") && e.contains("`b`"), "{e}");
        let e = open_zip(&zip("b", b"not a png"), "b").unwrap_err();
        assert!(e.starts_with("screenshot.png: it is not a PNG"), "{e}");
        assert!(open_zip(b"hello", "b").unwrap_err().starts_with("not a bug zip"));
    }

    /// A 1x1 PNG, as the conformance harness writes one.
    fn amiga_app_png() -> Vec<u8> {
        amiga_conform::report::png(1, 1, &[0, 0, 0])
    }
}
