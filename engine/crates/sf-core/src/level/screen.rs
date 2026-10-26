//! The level's horizontal screen edges, in one place (real-widescreen DESIGN §2.2, §3.2, D7).
//!
//! The original's screen is 334 window pixels from `camera_x`; widescreen's is 65 more each side.
//! Every margin the original measures from an edge keeps its distance from that edge, so every
//! site that tests an object or a sprite against the screen asks [`Screen`] for its edge instead
//! of writing the original's literal. With [`Screen::wide`] false each method is the original's
//! constant, which is what keeps the 4:3 game the original (DESIGN Stance). Vertical edges are not
//! here: the picture's height does not change.

/// The extra lores columns each side in widescreen (DESIGN §2.1): the app's
/// `window::extra_columns(261)`, which a test in `sf-app` pins to this.
pub const WIDE_EXTRA: u16 = 65;

/// [`WIDE_EXTRA`] in whole 16-pixel cells, rounded up: spawn scans are cell-based and move by
/// this, a scan column a little more than 65 pixels further out so an object is live before any of
/// it shows, as in the original (DESIGN §2.2).
pub const WIDE_CELLS: u16 = WIDE_EXTRA.div_ceil(16);

/// The screen a level step plays against: the original's, or widescreen's. Copied into
/// [`super::LevelCtx::screen`] from `Game`'s mode each step (DESIGN §3.1, §3.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Screen {
    pub wide: bool,
}

impl Screen {
    /// The original's screen.
    pub const NARROW: Screen = Screen { wide: false };
    /// Widescreen's.
    pub const WIDE: Screen = Screen { wide: true };

    /// The pixels each edge moves out by: 0 or [`WIDE_EXTRA`].
    pub fn extra(self) -> u16 {
        if self.wide { WIDE_EXTRA } else { 0 }
    }

    /// The cells a spawn scan's column moves out by: 0 or [`WIDE_CELLS`].
    pub fn extra_cells(self) -> u16 {
        if self.wide { WIDE_CELLS } else { 0 }
    }

    /// `obj_despawn_if_offscreen`'s right test: an object at `x ≥ cx + this` is gone, `$170` (36
    /// past the right edge), plus [`Screen::extra`].
    pub fn right_despawn(self) -> u16 {
        0x170 + self.extra()
    }

    /// `obj_despawn_if_offscreen`'s left test: an object whose right side (`x` + the frame width)
    /// is below `cx − this` is gone, `$30`, plus [`Screen::extra`]. The site adds it to the frame
    /// width, as the original adds `$30`.
    pub fn left_despawn(self) -> u16 {
        0x30 + self.extra()
    }

    /// Where a hardware sprite is hidden past the right of the screen, and parked to hide it
    /// (`frog/anim.rs`'s `OFF_SCREEN`): `$150`, plus [`Screen::extra`] (DESIGN §2.5).
    pub fn sprite_hide_right(self) -> u16 {
        0x150 + self.extra()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_is_the_originals_constants() {
        let s = Screen::default();
        assert_eq!(s, Screen::NARROW);
        assert_eq!(s.extra(), 0);
        assert_eq!(s.extra_cells(), 0);
        assert_eq!(s.right_despawn(), 0x170);
        assert_eq!(s.left_despawn(), 0x30);
        assert_eq!(s.sprite_hide_right(), 0x150);
    }

    #[test]
    fn wide_moves_every_edge_out_by_65_or_5_cells() {
        let s = Screen::WIDE;
        assert_eq!(WIDE_CELLS, 5);
        assert_eq!(s.extra(), 65);
        assert_eq!(s.extra_cells(), 5);
        assert_eq!(s.right_despawn(), 0x170 + 65);
        assert_eq!(s.left_despawn(), 0x30 + 65);
        assert_eq!(s.sprite_hide_right(), 0x150 + 65);
    }

    #[test]
    fn the_cell_move_covers_the_pixel_move() {
        assert!(WIDE_CELLS * 16 >= WIDE_EXTRA && (WIDE_CELLS - 1) * 16 < WIDE_EXTRA);
    }
}
