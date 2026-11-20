//! Game-free: `Amiga::picture_in` on Kickstart 1.3's insert-disk screen. Needs the Kickstart
//! (`AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`) and `toolkit/tools/setup/oracle.sh`'s build.

use amiga_oracle::*;

/// The first frame showing the insert-disk hand (tests/boot.rs).
const HAND: u32 = 207;

/// The DIWSTRT/DIWSTOP pair that crops to `f`'s window (OCS rules: stop H8 = 1, V8 = NOT V7).
fn window_of(f: &Rgb12Frame) -> (u16, u16) {
    let (right, bottom) = (f.left + f.width, f.top + f.height);
    (f.top << 8 | f.left, (bottom & 0xFF) << 8 | (right & 0xFF))
}

fn at_hand() -> (Amiga, Rgb12Frame) {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let mut a = Amiga::power_on(&OracleConfig::new(rom, Vec::new(), HAND + 1)).unwrap();
    let mut last = Rgb12Frame::default();
    for _ in 0..=HAND {
        last = a.step(FrameInput::NONE).unwrap().picture;
    }
    (a, last)
}

#[test]
fn with_the_frame_end_window_it_equals_the_captured_picture() {
    let (a, pic) = at_hand();
    assert_eq!(pic.width, 320);
    let (strt, stop) = window_of(&pic);
    assert!(
        a.picture_in(strt, stop) == pic,
        "picture_in({strt:#06x}, {stop:#06x}) differs"
    );
}

#[test]
fn another_window_shows_the_same_pixels_where_the_two_overlap() {
    let (a, pic) = at_hand();
    // Sixteen lores pixels wider on each side, eight lines shorter at the top and the bottom.
    let (strt, stop) = window_of(&pic);
    let other = a.picture_in(strt + 0x0800 - 0x10, stop - 0x0800 + 0x10);
    assert_eq!((other.left, other.top), (pic.left - 16, pic.top + 8));
    assert_eq!(
        (other.width, other.height),
        (pic.width + 32, pic.height - 16)
    );
    for y in 0..other.height {
        for x in 0..pic.width {
            assert_eq!(other.pixel(x + 16, y), pic.pixel(x, y + 8), "at ({x}, {y})");
        }
    }
}

#[test]
fn odd_or_empty_windows_do_not_panic() {
    let (a, _) = at_hand();
    for (strt, stop) in [
        (0, 0),
        (0xFFFF, 0xFFFF),
        (0xFFFF, 0xFF00),
        (0xF081, 0x80C1),
        (0x0101, 0xFFFF),
    ] {
        let f = a.picture_in(strt, stop);
        assert_eq!(f.pixels.len(), f.width as usize * f.height as usize);
    }
    assert!(a.picture_in(0xFFFF, 0xFF00).pixels.is_empty());
}
