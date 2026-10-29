//! Each step of the dry fit's frame, its expected values worked out here from the hardware's rules,
//! not read back from the run.

use amiga_chips::copper::reg;
use amiga_clock::{Clock, FRAME_LINES, LINE_CYCLES, LINE_SLOTS};
use amiga_paula::{CLOCKS_PER_FRAME, PAULA_CLOCK_PAL};
use dry_fit::*;

/// The screen's word at pixel row `y`, word `i` of the row.
fn screen_word(f: &Frame, y: u32, i: u32) -> u16 {
    f.machine.word(SCREEN + y * ROW_BYTES + 2 * i)
}

/// Whether the blitted pattern sets screen pixel (`x`, `y`): row `y - BLIT_Y` of the pattern,
/// moved right by `BLIT_X`.
fn pattern_at(x: u32, y: u32) -> bool {
    let Some(row) = y
        .checked_sub(BLIT_Y)
        .and_then(|r| PATTERN_ROWS.get(r as usize))
    else {
        return false;
    };
    (BLIT_X..BLIT_X + 16).contains(&x) && row >> (15 - (x - BLIT_X)) & 1 == 1
}

#[test]
fn the_blit_writes_the_shifted_pattern_and_nothing_else() {
    let f = run();
    let (shift, word) = (BLIT_X % 16, BLIT_X / 16);
    for (r, &p) in PATTERN_ROWS.iter().enumerate() {
        let y = BLIT_Y + r as u32;
        assert_eq!(screen_word(&f, y, word), p >> shift, "row {r}, first word");
        assert_eq!(
            screen_word(&f, y, word + 1),
            p << (16 - shift),
            "row {r}, the word the shift spills into"
        );
    }
    let rows = BLIT_Y..BLIT_Y + PATTERN_ROWS.len() as u32;
    for w in 0..256 * ROW_BYTES / 2 {
        let (y, i) = (w * 2 / ROW_BYTES, w % (ROW_BYTES / 2));
        if !(rows.contains(&y) && (i == word || i == word + 1)) {
            assert_eq!(
                screen_word(&f, y, i),
                0,
                "row {y} word {i}: outside the blit"
            );
        }
    }
    // The pattern and the copper list are untouched.
    for (i, &p) in PATTERN_ROWS.iter().enumerate() {
        assert_eq!(f.machine.word(PATTERN + 2 * i as u32), p);
    }
}

#[test]
fn the_clock_times_the_blits_writes_in_order_on_the_first_line() {
    let f = run();
    // Two words a row, eight rows, each written once.
    assert_eq!(f.blit_writes.len(), 2 * PATTERN_ROWS.len());
    assert!(
        f.blit_writes.windows(2).all(|w| w[0] < w[1]),
        "{:?}",
        f.blit_writes
    );
    // Line 0 fetches no plane: the blitter has every slot but refresh's, two a word (A, then D).
    assert!(*f.blit_writes.last().unwrap() < LINE_CYCLES);
}

#[test]
fn the_coppers_writes_land_where_the_bus_lets_them() {
    let f = run();
    let list = copper_list();
    // The vertical blank's restart takes slot 0 and the first fetch is at slot 2; each MOVE then
    // takes two free slots for its two words and writes at the second, so the set-up's MOVEs write
    // at 4, 8, 12, ... on line 0 (no plane is fetched there to take a slot).
    let setup: Vec<(u16, u16)> = list[..22].chunks(2).map(|m| (m[0], m[1])).collect();
    for (i, &(r, v)) in setup.iter().enumerate() {
        let w = f.writes[i];
        assert_eq!(
            (w.line, w.h, w.reg, w.value),
            (0, 4 + 4 * i as u16, r, v),
            "MOVE {i}"
        );
    }
    // The waits: the copper wakes two slots before the WAIT's position and the MOVE after it
    // writes two slots after it. Inside the fetch window the plane takes only odd slots, so an
    // even write slot is free.
    let rest: Vec<_> = f.writes[setup.len()..]
        .iter()
        .map(|w| (w.line, w.h, w.reg, w.value))
        .collect();
    assert_eq!(
        rest,
        [
            (SPLIT_LINE, 0x06 + 2, reg::COLOR00 + 2, BELOW),
            (MID_LINE, MID_H, reg::COLOR00, MID_COLOUR),
        ]
    );
}

#[test]
fn the_picture_shows_the_pattern_in_the_coppers_colours() {
    let f = run();
    let p = &f.picture;
    assert_eq!((p.left, p.top, p.width, p.height), (LEFT, TOP, 320, HEIGHT));
    // The mid-line write is a split at its lores position in the window.
    let split_x = 2 * MID_H as i32 - LEFT;
    assert_eq!(p.splits.len(), 1);
    let s = p.splits[0];
    assert_eq!(
        (s.y, s.x as i32, s.index, s.colour),
        (MID_LINE - TOP, split_x, 0, MID_COLOUR)
    );
    for y in 0..HEIGHT as u32 {
        let line = TOP + y as u16;
        for x in 0..320u32 {
            let want = if pattern_at(x, y) {
                if line < SPLIT_LINE { ABOVE } else { BELOW }
            } else if line > MID_LINE || line == MID_LINE && x as i32 >= split_x {
                MID_COLOUR
            } else {
                BACKGROUND
            };
            assert_eq!(p.rgb12(x as u16, y as u16), want, "pixel ({x}, {y})");
        }
    }
    // The colour change fell inside the pattern: both colours are on screen.
    let rows = |c: u16| {
        (0..HEIGHT)
            .filter(|&y| (0..320).any(|x| p.rgb12(x, y) == c))
            .count()
    };
    assert_eq!((rows(ABOVE), rows(BELOW)), (4, 4));
}

#[test]
fn the_clock_steps_exactly_one_frame() {
    let f = run();
    // The CPU's frame of work brings the beam to the next frame's start, whatever the blit took.
    assert_eq!((f.clock.line, f.clock.into), (FRAME_LINES, 0));
    assert_eq!(f.clock.next_frame(), Clock::START);
    assert_eq!(f.clock.nominal(), FRAME_LINES as u32 * LINE_CYCLES);
    // One frame is the same length to the clock and to Paula: a CPU cycle is half a DMA slot and a
    // Paula clock is a slot.
    assert_eq!(LINE_CYCLES, 2 * LINE_SLOTS as u32);
    assert_eq!(CLOCKS_PER_FRAME, FRAME_LINES as u32 * LINE_SLOTS as u32);
    assert_eq!(f.tap.len() as u32, CLOCKS_PER_FRAME);
}

#[test]
fn paulas_note_is_heard_at_its_period() {
    let f = run();
    let full = 0x40 * VOLUME as i16;
    assert!(f.tap.contains(&full) && f.tap.contains(&-full));
    // Each sample byte holds for PERIOD clocks, so the eight-byte square wave rises once every
    // 8 * PERIOD clocks.
    let rises: Vec<usize> = (1..f.tap.len())
        .filter(|&i| f.tap[i - 1] < 0 && f.tap[i] > 0)
        .collect();
    assert!(rises.len() > 40, "{} rises", rises.len());
    assert!(
        rises.windows(2).all(|w| w[1] - w[0] == 8 * PERIOD as usize),
        "{rises:?}"
    );
    // Mixed to the host: a frame's worth of samples, not silent, at the same pitch.
    let want = (HOST_RATE as u64 * CLOCKS_PER_FRAME as u64 / PAULA_CLOCK_PAL as u64) as usize;
    assert!(
        f.audio.len().abs_diff(want) <= 1,
        "{} samples",
        f.audio.len()
    );
    let left: Vec<i16> = f.audio.iter().map(|s| s[0]).collect();
    assert!(left.iter().any(|&s| s.unsigned_abs() > 4000));
    assert!(
        f.audio.iter().all(|s| s[1] == 0),
        "channel 0 is on the left"
    );
    // A rise is a swing from below -2000 to above +2000, so the band-limited edges' ringing near
    // zero is not counted.
    let mut host_rises = Vec::new();
    let mut low = false;
    for (i, &s) in left.iter().enumerate() {
        if s < -2000 {
            low = true;
        } else if s > 2000 && std::mem::take(&mut low) {
            host_rises.push(i);
        }
    }
    let (first, last) = (host_rises[0], *host_rises.last().unwrap());
    let mean = (last - first) as f64 / (host_rises.len() - 1) as f64;
    let expected = 8.0 * PERIOD as f64 * HOST_RATE as f64 / PAULA_CLOCK_PAL as f64;
    assert!(
        (mean - expected).abs() < 0.1,
        "{mean} samples a cycle, want {expected}"
    );
}
