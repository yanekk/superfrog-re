//! The whole play picture of W1-1 through `Amiga::picture_in` (DESIGN §7). The frame-end window is
//! the status bar's, so the captured picture holds only that bar; a window spanning the play area
//! and the bar holds both. Needs the disks and the Kickstart: absent, it fails naming where it
//! looked. With `FD_SAVE_PICTURE=<path.ppm>` it also writes the play frame there, for a look; the
//! picture comes from the disks, so the path belongs under `build/`.

use amiga_conform::script::{self, InputScript};
use amiga_oracle::{Amiga, OracleConfig, Rgb12Frame, kickstart_path};
use fd_conform::{conform_dir, disks};

/// boot-to-title's disk change, which `boot-to-level1` shares.
const SWAP: u32 = 3600;
/// The idle title, before START is fired at 5000 (boot-to-level1.txt).
const TITLE: u32 = 4990;
/// Two hundred frames into play (play begins at 6301).
const PLAY: u32 = 6500;

/// docs/09-play-measured.md: the play area's window (lores 145..417, lines 47..226) and the status
/// bar's (lores 129..449, lines 241..297), both set by the copper.
const PLAY_STRT: u16 = 0x2F91;
const PLAY_STOP: u16 = 0xE2A1;
const BAR_STRT: u16 = 0xF181;
const BAR_STOP: u16 = 0x29C1;
/// One window spanning both: the play area's top, the bar's left, right and bottom.
const WHOLE_STRT: u16 = PLAY_STRT & 0xFF00 | BAR_STRT & 0x00FF;
const WHOLE_STOP: u16 = BAR_STOP;

/// The DIWSTRT/DIWSTOP pair that crops to `f`'s window (OCS rules: stop H8 = 1, V8 = NOT V7).
fn window_of(f: &Rgb12Frame) -> (u16, u16) {
    let (right, bottom) = (f.left + f.width, f.top + f.height);
    (f.top << 8 | f.left, (bottom & 0xFF) << 8 | (right & 0xFF))
}

fn colours(f: &Rgb12Frame, x0: u16, y0: u16, w: u16, h: u16) -> usize {
    let mut c: Vec<u16> = (y0..y0 + h)
        .flat_map(|y| (x0..x0 + w).map(move |x| (x, y)))
        .map(|(x, y)| f.pixel(x, y))
        .collect();
    c.sort_unstable();
    c.dedup();
    c.len()
}

fn save_ppm(f: &Rgb12Frame, path: &str) {
    let mut out = format!("P6\n{} {}\n15\n", f.width, f.height).into_bytes();
    for p in &f.pixels {
        out.extend([(p >> 8 & 0xF) as u8, (p >> 4 & 0xF) as u8, (p & 0xF) as u8]);
    }
    std::fs::write(path, out).unwrap_or_else(|e| panic!("{path}: {e}"));
}

#[test]
fn in_play_the_spanning_window_holds_the_play_area_and_the_status_bar() {
    let path = conform_dir().join("scripts/boot-to-level1.txt");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("script {}: {e}", path.display()));
    let sc: InputScript = script::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    let kickstart = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let disks = disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut am = Amiga::power_on(&OracleConfig::new(kickstart, disks, PLAY))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut step = |am: &mut Amiga, frame: u32| {
        if frame == SWAP {
            am.insert_disk(0, 1);
        }
        am.step(script::input_at(&sc.lines, frame))
            .unwrap_or_else(|e| panic!("{e}"))
            .picture
    };

    // On the title the frame-end window is the whole picture, and picture_in through it agrees.
    let mut title = Rgb12Frame::default();
    for frame in 0..TITLE {
        title = step(&mut am, frame);
    }
    assert_eq!(
        (title.width, title.height),
        (320, 238),
        "the title's window, measured 2026-11-05"
    );
    let (strt, stop) = window_of(&title);
    assert!(
        am.picture_in(strt, stop) == title,
        "picture_in on the title differs"
    );

    let mut bar = Rgb12Frame::default();
    for frame in TITLE..PLAY {
        bar = step(&mut am, frame);
    }
    // The captured picture is the status bar alone (forest-dumb-start's finding).
    assert_eq!(
        (bar.left, bar.top, bar.width, bar.height),
        (0x81, 0xF1, 320, 56)
    );
    assert!(am.picture_in(BAR_STRT, BAR_STOP) == bar);

    let whole = am.picture_in(WHOLE_STRT, WHOLE_STOP);
    if let Ok(p) = std::env::var("FD_SAVE_PICTURE") {
        save_ppm(&whole, &p);
    }
    assert_eq!(
        (whole.left, whole.top, whole.width, whole.height),
        (0x81, 0x2F, 320, 0x129 - 0x2F),
        "lines 47 to 296, lores 129 to 448"
    );
    // Its last 56 lines are the status bar, pixel for pixel.
    let bar_y = 0xF1 - 0x2F;
    for y in 0..bar.height {
        for x in 0..bar.width {
            assert_eq!(
                whole.pixel(x, bar_y + y),
                bar.pixel(x, y),
                "bar at ({x}, {y})"
            );
        }
    }
    // The play area (lores 145..417, lines 47..226) is drawn: a level, not a blank window.
    let play = am.picture_in(PLAY_STRT, PLAY_STOP);
    assert_eq!((play.width, play.height), (272, 0xE2 - 0x2F));
    let n = colours(&whole, 145 - 129, 0, 272, 0xE2 - 0x2F);
    assert!(n >= 8, "the play area shows {n} colours");
    for y in 0..play.height {
        for x in 0..play.width {
            assert_eq!(whole.pixel(x + 16, y), play.pixel(x, y));
        }
    }
}
