//! A bug recording's screenshot held against the engine's picture: how many pixels differ. Here,
//! not beside the recording (`amiga-record`, pure), because it decodes a PNG.

use amiga_chips::{Picture, to_rgb12};

/// `pic` as 8-bit RGB, each 4-bit channel times 17, as the app shows it.
pub fn picture_rgb(pic: &Picture) -> Vec<u8> {
    to_rgb12(pic)
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
    let (w, h, rgb) = decode_png(png).map_err(|e| format!("screenshot.png: {e}"))?;
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

/// A PNG as (width, height, 8-bit RGB). An app writes RGB; any other colour type a PNG may hold is
/// taken too, alpha dropped.
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
