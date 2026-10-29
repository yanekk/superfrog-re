//! The bug zip: the recording's text and the screenshot the app showed when it stopped, stored.

use amiga_disk::zip::{read_entries, write_stored};

use crate::recording::{GameRecording, Recording};

/// The bug zip's entries.
pub const ZIP_RECORDING: &str = "recording.txt";
pub const ZIP_SCREENSHOT: &str = "screenshot.png";

/// The largest entry read out of a bug zip. `recording.txt` at `MAX_FRAMES` with the input
/// changing every frame is ~60 MB; a 1× screenshot is well under 1 MB.
const MAX_ENTRY: usize = 256 << 20;

/// The recording and the screenshot's bytes of a bug zip. Refuses, naming why, anything that is not
/// a zip, lacks either entry, or holds a recording the parser or [`Recording::validate`] refuses.
/// The screenshot is not decoded here: a caller that needs it to be a picture checks it.
pub fn read_zip<G: GameRecording>(bytes: &[u8]) -> Result<(Recording<G>, Vec<u8>), String> {
    let entries = read_entries(
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
    let rec = Recording::<G>::parse(&text).map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    rec.validate()
        .map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    Ok((rec, png))
}

/// The bug zip's bytes: `recording.txt` then `screenshot.png`, stored. A recording that could not
/// be read back is refused, so nothing unreadable is ever written.
pub fn zip_bytes<G: GameRecording>(rec: &Recording<G>, png: &[u8]) -> Result<Vec<u8>, String> {
    rec.validate()?;
    let text = rec.to_text();
    Ok(write_stored(&[
        (ZIP_RECORDING, text.as_bytes()),
        (ZIP_SCREENSHOT, png),
    ]))
}
