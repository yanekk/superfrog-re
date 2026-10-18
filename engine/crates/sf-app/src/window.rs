//! The window's size and the picture's place in it (DESIGN §2.7): integer scaling only, 1× to the
//! largest that fits, centred with black around it, in a resizable window that cannot be made
//! smaller than 1×; or fullscreen with the largest integer scale and black borders.
//!
//! Sizes are in the surface's pixels. On a Retina display a point is two of them, so `Scale::N(n)`
//! is `2n` pixels a game pixel there, and `Auto` may land on an odd number of pixels: still a whole
//! number, so every game pixel is the same solid square and nothing is smeared.

use winit::dpi::LogicalSize;
use winit::window::{Fullscreen, Window};

use crate::settings::Scale;

/// The game's picture, in its own pixels (4:3).
pub const GAME: (u32, u32) = (320, 256);

/// The window's inner size when it opens: `n` × the game for `Scale::N(n)`, 2× for `Auto`.
pub fn initial_size(scale: Scale) -> LogicalSize<u32> {
    let n = match scale {
        Scale::Auto => 2,
        Scale::N(n) => n,
    };
    LogicalSize::new(GAME.0 * n, GAME.1 * n)
}

/// The smallest the window may be made: 1×.
pub fn min_size() -> LogicalSize<u32> {
    LogicalSize::new(GAME.0, GAME.1)
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

    #[test]
    fn a_fixed_scale_is_its_multiple_centred_and_shrinks_to_fit() {
        let s = |surface, n| picture_scale(surface, GAME, Scale::N(n), 1.0);
        assert_eq!(s((320, 256), 1), 1);
        assert_eq!(viewport((320, 256), GAME, 1), (0, 0, 320, 256));
        assert_eq!(s((1000, 600), 2), 2);
        assert_eq!(viewport((1000, 600), GAME, 2), (180, 44, 640, 512));
        assert_eq!(s((700, 600), 3), 2, "3× does not fit 700 wide");
        assert_eq!(viewport((700, 600), GAME, 2), (30, 44, 640, 512));
        assert_eq!(s((100, 100), 2), 1, "never below 1×");
    }

    #[test]
    fn auto_is_the_largest_whole_multiple_that_fits() {
        let a = |surface| picture_scale(surface, GAME, Scale::Auto, 1.0);
        assert_eq!(a((320, 256)), 1);
        assert_eq!(a((959, 767)), 2);
        assert_eq!(a((960, 768)), 3);
        assert_eq!(a((3000, 800)), 3, "the height decides");
        assert_eq!(a((3456, 2234)), 8, "a 16-inch Retina display, fullscreen");
    }

    #[test]
    fn on_retina_a_point_is_two_pixels() {
        assert_eq!(picture_scale((1280, 1024), GAME, Scale::N(2), 2.0), 4);
        assert_eq!(picture_scale((1280, 1024), GAME, Scale::N(3), 2.0), 4, "shrunk to fit");
        assert_eq!(picture_scale((960, 768), GAME, Scale::Auto, 2.0), 3, "odd pixels, still whole");
    }

    #[test]
    fn the_window_opens_at_its_scale_and_never_goes_below_1x() {
        assert_eq!(initial_size(Scale::N(3)), LogicalSize::new(960, 768));
        assert_eq!(initial_size(Scale::Auto), LogicalSize::new(640, 512));
        assert_eq!(min_size(), LogicalSize::new(320, 256));
    }
}
