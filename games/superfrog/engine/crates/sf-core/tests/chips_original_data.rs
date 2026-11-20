//! The toolkit's copper and blitter on this game's own data: the copper list and display set-ups
//! the original uses, which may not enter the toolkit (toolkit-extract DESIGN §2.3). The toolkit's
//! own tests run the same checks on synthetic data.

use amiga_chips::copper::{Fetch, run_bus};
use amiga_chips::word_times;

fn list(words: &[u16]) -> impl Fn(u32) -> u16 + '_ {
    move |a| words.get(a as usize / 2).copied().unwrap_or(0xFFFF)
}

#[test]
fn the_intro_s_scene_1_list_takes_the_vblank_cycle_and_its_fetches() {
    // The intro's scene-1 list (T89): three moves and the end. Nine cycles: 0 and the eight
    // fetches; WAIT1 and WAIT2 ($12, $14) only wait for the bus.
    let w = [
        0x0180, 0x0000, 0x0100, 0x0200, 0x0096, 0x0020, 0xFFFF, 0xFFFE,
    ];
    let mut bus = Vec::new();
    run_bus(&|a, _, _| list(&w)(a), 0, Fetch::default(), 313, &mut bus);
    let h: Vec<u16> = bus.iter().map(|b| b.slot()).collect();
    assert!(bus.iter().all(|b| b.line == 0));
    assert_eq!(h, [0, 2, 4, 6, 8, 10, 12, 14, 16]);
}

/// `word_times` as it was before it moved to `amiga-clock` (one-clock T02), with `&Fetch`: the
/// reference the moved one, given `|l, h| fetch.takes(l, h)`, is held to.
fn old_word_times(words: u32, line: u16, h: u16, fetch: &Fetch) -> Vec<(u16, u16)> {
    let mut out = Vec::with_capacity(words as usize);
    let (mut l, mut c) = (line, h);
    let mut half = false;
    while (out.len() as u32) < words {
        let fixed = c % 2 == 1 && c <= 0x0d;
        if !fixed && !fetch.takes(l % 313, c) {
            if half {
                out.push((l, c));
            }
            half = !half;
        }
        c += 1;
        if c >= 227 {
            c = 0;
            l += 1;
        }
    }
    out
}

#[test]
fn word_times_on_the_game_s_display_set_ups_are_the_old_ones() {
    let f = |bplcon0, diwstrt, diwstop, ddfstrt, ddfstop| Fetch {
        bplcon0,
        diwstrt,
        diwstop,
        ddfstrt,
        ddfstop,
    };
    // The set-ups the engine draws with: no display, the front end's and story's lores
    // screens of four to six planes, the level's scrolled fetch, hires, and a window past
    // line $ff.
    let fetches = [
        Fetch::default(),
        f(0x4200, 0x2c81, 0x2cc1, 0x38, 0xd0),
        f(0x5200, 0x2c81, 0x2cc1, 0x38, 0xd0),
        f(0x6200, 0x2c81, 0x2cc1, 0x38, 0xd0),
        f(0x5200, 0x2781, 0x2cc1, 0x30, 0xd8),
        f(0xc200, 0x2c81, 0x2cc1, 0x3c, 0xd4),
        f(0x3200, 0x1081, 0xf4c1, 0x28, 0xd8),
    ];
    for fetch in &fetches {
        for line in (0..313).step_by(5).chain([255, 256, 311, 312, 313, 400]) {
            for h in (0..227).step_by(3).chain([0x0d, 0x0e, 0xe2]) {
                for words in [0, 1, 2, 3, 40, 321, 2000] {
                    assert_eq!(
                        word_times(words, line, h, 313, |l, h| fetch.takes(l, h)),
                        old_word_times(words, line, h, fetch),
                        "{fetch:?} {line} {h} {words}"
                    );
                }
            }
        }
    }
}
