//! The renderer against the original (T13, DESIGN §2.1 (1), §2.7): the original's memory at a
//! frame, transplanted into a fresh engine with no stepping, renders the picture the reference
//! emulator showed. Here rather than in `sf-core/tests` because it runs the emulator (the purity
//! test keeps processes and files out of the pure crates' tests).
//!
//! The test block's share: W1-1, whose sync point is the cheapest to reach. Every level of W1–W6
//! (W6-2..4 in both map sets), the bonus level and the final level, 400 frames each, is in
//! `engine/conform/cases/render-*.toml`, run by `sf-conform -- all`.

use sf_conform::{Case, InputScript, LATE_REDRAW, NOT_A_LEVEL, display_data, sync};
use sf_core::layout::{Ram, h0};
use sf_core::{Game, Picture, Start, View, to_rgb12};

const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);

fn long(m: &dyn Ram, a: u32) -> u32 {
    (0..4).fold(0, |v, k| v << 8 | m.byte(a + k) as u32)
}

/// Each frame of the `render-walk` script through W1-1: transplanted renders against the
/// original's pictures, frames whose late redraws a transplant cannot see left out and listed.
/// The walk scrolls right, jumps (the camera rises and falls) and drops into the first pit.
#[test]
fn w1_1_renders_as_the_original_while_it_scrolls_both_ways() {
    let case = Case::parse(
        "render-w1-1-test",
        "sync = { level = \"W1-1\" }\nscript = \"render-walk\"\nframes = 400\ncompare = [\"render\"]\n",
    )
    .unwrap();
    let outcome = sf_conform::run(&case).unwrap_or_else(|d| panic!("{d}"));
    for (f, why) in &outcome.skipped {
        println!("frame {f} left out: {why}");
        assert!([LATE_REDRAW, NOT_A_LEVEL].contains(why));
    }
    assert!(
        outcome.rendered >= 300,
        "only {} of 400 frames compared",
        outcome.rendered
    );

    // The same walk scrolls both ways.
    let synced = sync::reach(&case.sync, &case.script, case.frames).unwrap();
    let mut amiga = synced.amiga;
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for f in 0..case.frames {
        amiga.step(case.script.input_at(f)).unwrap();
        let m = sync::core_memory(amiga.memory());
        xs.push(long(&m, CAMERA_X));
        ys.push(long(&m, CAMERA_Y));
    }
    let span = |v: &[u32]| v.iter().max().unwrap() - v.iter().min().unwrap();
    assert!(span(&xs) >= 160, "camera x moved only {}", span(&xs));
    assert!(span(&ys) >= 32, "camera y moved only {}", span(&ys));
}

/// The copper's palette work in a level, on 100 frames of W1-1: the panel's palette switched in
/// the middle of line $28 (splits), every frame equal. The level palette itself changes only in
/// the fades around a level and the boss's flash (h0+$153a4, $155a0, $151b2), outside the level
/// loop; there is no colour cycling (docs/12 "Palettes").
#[test]
fn copper_palette_effects_match_for_100_frames() {
    let script = InputScript::load(&sf_conform::case::script_path("render-walk")).unwrap();
    let synced = sync::reach(
        &sf_conform::SyncPoint::AtLevel("W1-1".parse().unwrap()),
        &script,
        100,
    )
    .unwrap();
    let mut amiga = synced.amiga;
    let display = display_data();
    let (mut compared, mut with_splits) = (0, 0);
    for f in 0..100 {
        let theirs = amiga.step(script.input_at(f)).unwrap().picture;
        let mem = sync::core_memory(amiga.memory());
        let p = Game::with_display(display.clone(), Start::Transplant(mem)).render(View::default());
        if to_rgb12(&p) != theirs.pixels {
            continue; // a late redraw; the first test checks what each such frame is
        }
        compared += 1;
        // Picture line 1 is raster line $28.
        with_splits += p.splits.iter().any(|s| s.y == 1) as u32;
    }
    assert!(compared >= 90, "only {compared} of 100 frames equal");
    assert_eq!(
        with_splits, compared,
        "every frame switches the panel palette mid-line"
    );
}

fn transplanted(frames: &[u32]) -> Vec<Game> {
    let script = InputScript::load(&sf_conform::case::script_path("render-walk")).unwrap();
    let last = *frames.iter().max().unwrap() + 1;
    let synced = sync::reach(
        &sf_conform::SyncPoint::AtLevel("W1-1".parse().unwrap()),
        &script,
        last,
    )
    .unwrap();
    let mut amiga = synced.amiga;
    let display = display_data();
    let mut games = Vec::new();
    for f in 0..last {
        amiga.step(script.input_at(f)).unwrap();
        if frames.contains(&f) {
            let mem = sync::core_memory(amiga.memory());
            games.push(Game::with_display(display.clone(), Start::Transplant(mem)));
        }
    }
    games
}

fn colours(p: &Picture, x0: usize, w: usize) -> Vec<u16> {
    let all = to_rgb12(p);
    all.chunks(p.width as usize)
        .flat_map(|r| r[x0..x0 + w].to_vec())
        .collect()
}

/// Widescreen draws the same frame in its centre (DESIGN §2.7), and beyond the map's left edge
/// the original's out-of-map colour 0, at the start of W1-1 where the camera is at x 16.
#[test]
fn widescreen_keeps_the_centre_and_shows_colour_0_off_the_map() {
    for g in transplanted(&[0, 120, 240]) {
        let narrow = g.render(View::default());
        let w = narrow.width as usize;
        // 150: a 21:9 window, wider than the hardware's own left edge allows.
        for n in [32u16, 53, 80, 150] {
            let wide = g.render(View {
                extra_columns_each_side: n,
            });
            assert_eq!(
                (wide.width as usize, wide.left),
                (w + 2 * n as usize, narrow.left - n as i32)
            );
            assert_eq!(
                colours(&wide, n as usize, w),
                colours(&narrow, 0, w),
                "n = {n}"
            );
        }
    }
    let start = &transplanted(&[0])[0];
    let wide = start.render(View {
        extra_columns_each_side: 80,
    });
    let x_shown = start.state().camera.x as usize;
    // Picture column 80 shows level x ~16 (the camera); 64 columns left of the level's x 0 are off
    // the map. Rows of the play field only: below the panel and its black lines.
    for y in 20..wide.height as usize {
        for x in 0..(80 - x_shown - 2) {
            assert_eq!(wide.pixels[y * wide.width as usize + x], 0, "({x}, {y})");
        }
    }
}
