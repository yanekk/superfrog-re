//! The window's size and the picture's place in it (DESIGN §2.7): integer scaling only, 1× to the
//! largest that fits, centred with black around it, in a resizable window that cannot be made
//! smaller than 1×; or fullscreen with the largest integer scale and black borders.
//!
//! Sizes are in the surface's pixels. On a Retina display a point is two of them, so `Scale::N(n)`
//! is `2n` pixels a game pixel there, and `Auto` may land on an odd number of pixels: still a whole
//! number, so every game pixel is the same solid square and nothing is smeared.

use sf_core::View;
use winit::dpi::LogicalSize;
use winit::window::{Fullscreen, Window};

use crate::settings::Scale;

/// The largest display window the game opens, in its own pixels: a level's, the progress map's
/// and the tally's, DIWSTRT `$2779` (T210, `sf-conform`'s `examples/windows`). Every other screen
/// (title, story intro, slot machine, end sequence) is 320×256 or smaller and is centred in it.
/// The window is sized for it, so `Scale::N(n)` shows every screen at `n`.
pub const GAME: (u32, u32) = (334, 261);

/// Widescreen's extra columns on each side of a display window `height` lines high and as wide as
/// [`GAME`]: as many as fill 16:9, `(16/9 × height − 334) / 2` rounded down (65 for 261 lines).
/// DESIGN §2.7.
pub fn extra_columns(height: u32) -> u16 {
    ((16 * height).saturating_sub(9 * GAME.0) / 18) as u16
}

/// What the game draws for the widescreen setting: 4:3, or 16:9's extra columns.
pub fn view(widescreen: bool) -> View {
    View {
        extra_columns_each_side: if widescreen { extra_columns(GAME.1) } else { 0 },
    }
}

/// The largest picture the game draws for the widescreen setting, in its own pixels.
pub fn frame(widescreen: bool) -> (u32, u32) {
    (GAME.0 + 2 * view(widescreen).extra_columns_each_side as u32, GAME.1)
}

/// The window's inner size at `n` × the game: `Scale::N(n)`, 2× for `Auto`.
pub fn initial_size(scale: Scale, widescreen: bool) -> LogicalSize<u32> {
    let n = match scale {
        Scale::Auto => 2,
        Scale::N(n) => n,
    };
    let (w, h) = frame(widescreen);
    LogicalSize::new(w * n, h * n)
}

/// The smallest the window may be made: 1×.
pub fn min_size(widescreen: bool) -> LogicalSize<u32> {
    let (w, h) = frame(widescreen);
    LogicalSize::new(w, h)
}

/// Surface pixels a game pixel, for a `frame` drawn on a `surface` of that many pixels at
/// `scale_factor` pixels a point: the setting's multiple, or the largest that fits when the
/// surface is smaller; never below 1.
pub fn picture_scale(surface: (u32, u32), frame: (u32, u32), scale: Scale, scale_factor: f64) -> u32 {
    let fit = (surface.0 / frame.0.max(1)).min(surface.1 / frame.1.max(1)).max(1);
    match scale {
        Scale::Auto => fit,
        Scale::N(n) => (n * scale_factor.round().max(1.0) as u32).min(fit),
    }
}

/// The scale the picture is drawn at: the setting in a window, the largest that fits fullscreen.
pub fn effective(scale: Scale, fullscreen: bool) -> Scale {
    if fullscreen { Scale::Auto } else { scale }
}

/// The picture's place on the surface at `s` pixels a game pixel, centred: `(x, y, w, h)`.
pub fn viewport(surface: (u32, u32), frame: (u32, u32), s: u32) -> (u32, u32, u32, u32) {
    let (w, h) = (frame.0 * s, frame.1 * s);
    (surface.0.saturating_sub(w) / 2, surface.1.saturating_sub(h) / 2, w, h)
}

/// Fullscreen on the window's display, or back to the window.
pub fn set_fullscreen(window: &Window, on: bool) {
    window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The title's display window, the smallest full screen.
    const TITLE: (u32, u32) = (320, 256);

    #[test]
    fn a_fixed_scale_is_its_multiple_centred_and_shrinks_to_fit() {
        let s = |surface, n| picture_scale(surface, GAME, Scale::N(n), 1.0);
        assert_eq!(s((334, 261), 1), 1);
        assert_eq!(viewport((334, 261), GAME, 1), (0, 0, 334, 261));
        assert_eq!(s((1000, 600), 2), 2);
        assert_eq!(viewport((1000, 600), GAME, 2), (166, 39, 668, 522));
        assert_eq!(s((700, 600), 3), 2, "3× does not fit 700 wide");
        assert_eq!(viewport((700, 600), GAME, 2), (16, 39, 668, 522));
        assert_eq!(s((100, 100), 2), 1, "never below 1×");
    }

    #[test]
    fn auto_is_the_largest_whole_multiple_that_fits() {
        let a = |surface| picture_scale(surface, GAME, Scale::Auto, 1.0);
        assert_eq!(a((334, 261)), 1);
        assert_eq!(a((1001, 782)), 2);
        assert_eq!(a((1002, 783)), 3);
        assert_eq!(a((3000, 800)), 3, "the height decides");
        assert_eq!(a((3456, 2234)), 8, "a 16-inch Retina display, fullscreen");
    }

    #[test]
    fn on_retina_a_point_is_two_pixels() {
        assert_eq!(picture_scale((1336, 1044), GAME, Scale::N(2), 2.0), 4);
        assert_eq!(picture_scale((1336, 1044), GAME, Scale::N(3), 2.0), 4, "shrunk to fit");
        assert_eq!(picture_scale((1002, 783), GAME, Scale::Auto, 2.0), 3, "odd pixels, still whole");
    }

    #[test]
    fn fullscreen_is_always_the_largest_whole_scale() {
        assert_eq!(effective(Scale::N(1), true), Scale::Auto);
        assert_eq!(effective(Scale::N(2), false), Scale::N(2));
        assert_eq!(effective(Scale::Auto, false), Scale::Auto);
    }

    #[test]
    fn the_window_opens_at_its_scale_and_never_goes_below_1x() {
        assert_eq!(initial_size(Scale::N(3), false), LogicalSize::new(1002, 783));
        assert_eq!(initial_size(Scale::Auto, false), LogicalSize::new(668, 522));
        assert_eq!(min_size(false), LogicalSize::new(334, 261));
    }

    /// T210: a window sized for the title showed a level at `n − 1`.
    #[test]
    fn every_screen_shows_at_the_scale_its_window_opened_at() {
        for wide in [false, true] {
            let n_cols = view(wide).extra_columns_each_side as u32;
            for n in 1..=4 {
                for sf in [1.0, 2.0] {
                    let size = initial_size(Scale::N(n), wide);
                    let surface = ((size.width as f64 * sf) as u32, (size.height as f64 * sf) as u32);
                    for (w, h) in [GAME, TITLE, (320, 106), (320, 200)] {
                        let frame = (w + 2 * n_cols, h);
                        let s = picture_scale(surface, frame, Scale::N(n), sf);
                        assert_eq!(s, n * sf as u32, "{w}×{h} at {n}× (wide {wide}, {sf} px/pt)");
                    }
                }
            }
        }
    }

    #[test]
    fn widescreen_adds_the_columns_that_fill_16_9() {
        assert_eq!(extra_columns(261), 65, "(464 - 334) / 2");
        assert_eq!(extra_columns(180), 0, "a window already wider than 16:9 adds none");
        assert_eq!(view(false), View::default());
        assert_eq!(view(true).extra_columns_each_side, 65);
        assert_eq!(frame(true), (464, 261));
        assert_eq!(initial_size(Scale::N(2), true), LogicalSize::new(928, 522));
        assert_eq!(min_size(true), LogicalSize::new(464, 261));
        // 16:9 to within a column at every whole scale.
        let (w, h) = frame(true);
        assert!((w as f64 / h as f64 - 16.0 / 9.0).abs() < 1.0 / h as f64 * 2.0);
    }
}
