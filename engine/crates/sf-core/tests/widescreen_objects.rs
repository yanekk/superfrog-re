//! real-widescreen T08 (DESIGN §2.9, D6): every bob a pass drew shows in the side strips,
//! pixel-identical to the same bob in the centre, from the shown buffer's wide draw list. The
//! constructed-state cases are `render::bobs`' unit tests; these play W1-1.

use std::sync::Arc;

use sf_core::input::{FrameInput, Joy};
use sf_core::level::objects::{SLOTS, WIDE_SLOTS, slot_at};
use sf_core::render::{StripLayers, strip_layers};
use sf_core::{DisplayData, Game, LevelId, Start, View};

const N: i32 = 65;
const WIDE: View = View {
    extra_columns_each_side: N as u16,
};
const BUILD: &str = "widescreen-objects-test";

fn display() -> Arc<DisplayData> {
    static D: std::sync::OnceLock<Arc<DisplayData>> = std::sync::OnceLock::new();
    D.get_or_init(|| Arc::new(DisplayData::new(sf_conform::game_data())))
        .clone()
}

/// Frame `f`'s input: walking right, a jump now and then, so the camera scrolls and the strips
/// fill.
fn input(f: u32) -> FrameInput {
    let mut joy = if f % 90 < 70 { Joy::RIGHT } else { Joy::NONE };
    if f % 40 < 3 {
        joy = joy | Joy::UP;
    }
    FrameInput::joy(joy)
}

/// `level` in widescreen from the start, drawn wide.
fn wide(level: &str) -> Game {
    let mut g = Game::with_display(display(), Start::Level(level.parse::<LevelId>().unwrap()));
    g.set_widescreen(true);
    g.set_view(WIDE);
    g
}

/// W1-1's run, every `every`th frame in the level: `look(frame, game, layers)`.
fn run(frames: u32, every: u32, look: impl FnMut(u32, &Game, &StripLayers)) {
    run_level("W1-1", frames, every, look)
}

fn run_level(level: &str, frames: u32, every: u32, mut look: impl FnMut(u32, &Game, &StripLayers)) {
    let mut g = wide(level);
    for f in 0..frames {
        g.step(input(f));
        if f % every == 0 && g.scene().is_level() {
            look(f, &g, &strip_layers(g.state(), &display(), WIDE));
        }
    }
}

fn in_strip(x: i32, width: i32) -> bool {
    x < N || x >= width - N
}

/// In W1-1, the busy W4-2 and the bonus level: every bob pixel the strips' drawing puts in the centre is the buffer's pixel there, and
/// every strip pixel a bob covers is that drawing, on every frame looked at; on some frame a bob
/// straddles the centre's right edge, so its two halves come from the two sides of the seam.
/// Where the scroller drew map rows over a pass's bobs (the camera moving up or down, or
/// jumping at a restart: frames 207 and 357 here) and where the drawer's two parts left rows
/// out, the strips leave them out too.
#[test]
fn a_strip_bob_continues_the_centre_pixel_for_pixel() {
    let mut checked = (0, 0);
    let mut straddled = 0;
    let mut right_strip = 0;
    for level in ["W1-1", "W4-2", "WB-1"] {
        run_level(level, 1500, 3, |f, _, l| {
            let (w, h) = (l.picture.width as i32, l.picture.height as i32);
            for y in 0..h {
                for x in 0..w {
                    let at = (y * w + x) as usize;
                    if let Some(v) = l.bobs[at] {
                        assert_eq!(
                            l.picture.pixels[at], v,
                            "{level} frame {f}: picture ({x}, {y})"
                        );
                        if in_strip(x, w) {
                            checked.1 += 1;
                        } else {
                            checked.0 += 1;
                        }
                        right_strip += (x >= w - N) as u32;
                    }
                }
            }
            let seam = w - N;
            straddled += l
                .placed
                .iter()
                .filter(|(_, x, y, bw, bh)| {
                    *x < seam - 2 && x + bw > seam + 2 && *y >= 0 && y + bh <= h
                })
                .filter(|(_, x, y, bw, bh)| {
                    let row = |side: std::ops::Range<i32>| {
                        side.clone().any(|px| {
                            (*y..y + bh).any(|py| l.bobs[(py * w + px) as usize].is_some())
                        })
                    };
                    row(*x..seam) && row(seam..x + bw)
                })
                .count();
        });
    }
    assert!(
        checked.0 > 10_000 && checked.1 > 1_000,
        "bob pixels checked (centre, strips): {checked:?}"
    );
    assert!(right_strip > 0, "no bob in the right strip");
    assert!(straddled > 0, "no bob straddled the centre's right edge");
}

/// A bob with one frame record, wholly in the centre on one frame and wholly in a strip on
/// another, shows the same pixels at both places.
#[test]
fn the_same_bob_scrolled_into_a_strip_draws_the_same_pixels() {
    // Each frame record's pixels seen wholly in the centre, then compared in the strips.
    type Block = Vec<Option<u8>>;
    let mut centre: std::collections::BTreeMap<u32, Block> = Default::default();
    let mut matched = 0;
    let block = |l: &StripLayers, x: i32, y: i32, bw: i32, bh: i32| -> Block {
        let w = l.picture.width as i32;
        (y..y + bh)
            .flat_map(|py| (x..x + bw).map(move |px| (px, py)))
            .map(|(px, py)| {
                let at = (py * w + px) as usize;
                l.bobs[at].map(|_| l.picture.pixels[at])
            })
            .collect()
    };
    run(1500, 2, |f, _, l| {
        let w = l.picture.width as i32;
        // Only a bob no other overlaps, so its pixels are its own.
        let alone = |k: usize| {
            let (_, x, y, bw, bh) = l.placed[k];
            l.placed
                .iter()
                .enumerate()
                .all(|(j, &(_, ox, oy, ow, oh))| {
                    j == k || ox >= x + bw || ox + ow <= x || oy >= y + bh || oy + oh <= y
                })
        };
        for (k, &(b, x, y, bw, bh)) in l.placed.iter().enumerate() {
            let rows = &l.field_rows;
            if y < rows.start || y + bh > rows.end || x < 0 || x + bw > w || !alone(k) {
                continue;
            }
            let pixels = block(l, x, y, bw, bh);
            if pixels.iter().all(Option::is_none) {
                continue;
            }
            if x >= N && x + bw <= w - N {
                centre.insert(b.frame, pixels);
            } else if (x + bw <= N || x >= w - N)
                && let Some(seen) = centre.get(&b.frame)
            {
                assert_eq!(
                    &pixels, seen,
                    "frame {f}: frame record ${:x} at ({x}, {y})",
                    b.frame
                );
                matched += 1;
            }
        }
    });
    assert!(
        matched > 0,
        "no bob was seen wholly in the centre and then wholly in a strip"
    );
}

/// A bob in an extra slot (T03) is in the list and shows in a strip: W4-2, whose walk right has
/// extra slots live from early on (W1-1's never needs one).
#[test]
fn a_bob_in_an_extra_slot_shows_in_the_strip() {
    let mut shown = 0;
    run_level("W4-2", 1500, 3, |_, g, l| {
        let w = l.picture.width as i32;
        let peek_l = |a: u32| u32::from_be_bytes(g.peek(a, 4).try_into().unwrap());
        for k in SLOTS..SLOTS + WIDE_SLOTS {
            let slot = slot_at(k);
            let bob = peek_l(slot + 0x50);
            if peek_l(slot + 0x40) == 0 || bob == 0 {
                continue;
            }
            let pos = peek_l(bob - 4);
            for &(b, x, y, bw, bh) in &l.placed {
                if ((b.x as u16 as u32) << 16 | b.y as u16 as u32) != pos {
                    continue;
                }
                let strip_pixels = (y.max(0)..(y + bh).min(l.picture.height as i32))
                    .flat_map(|py| (x.max(0)..(x + bw).min(w)).map(move |px| (px, py)))
                    .filter(|&(px, py)| in_strip(px, w) && l.bobs[(py * w + px) as usize].is_some())
                    .count();
                shown += (strip_pixels > 0) as u32;
            }
        }
    });
    assert!(shown > 0, "no extra slot's bob showed in a strip");
}

/// A save state carries the wide draw lists: the loaded game draws the same strips at once.
#[test]
fn a_loaded_save_state_draws_its_strips_at_once() {
    let hash = sf_assets::cache_hash(sf_conform::game_data());
    let mut g = wide("W1-1");
    let mut f = 0;
    // On to a frame with a bob in the right strip.
    loop {
        g.step(input(f));
        f += 1;
        let l = strip_layers(g.state(), &display(), WIDE);
        let w = l.picture.width as usize;
        if g.scene().is_level()
            && l.bobs
                .iter()
                .enumerate()
                .any(|(at, v)| v.is_some() && at % w >= w - N as usize)
        {
            break;
        }
        assert!(f < 3000, "no bob reached the right strip");
    }
    let saved = g.save(BUILD, &hash);
    let mut loaded = Game::load(sf_conform::game_data(), &saved, BUILD, &hash).unwrap();
    loaded.set_view(WIDE);
    assert_eq!(loaded.render(WIDE), g.render(WIDE));
}
