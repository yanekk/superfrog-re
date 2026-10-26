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

    /// How far left of the camera the spawn scans start, in pixels (`scan_origin`, h0+$177da):
    /// `$20`, plus [`Screen::extra_cells`] whole cells, so the scans stay cell-aligned
    /// (DESIGN §2.2). The site clamps the result at 0 as the original does.
    pub fn scan_left(self) -> u16 {
        0x20 + 16 * self.extra_cells()
    }

    /// The byte offset from the scan origin to `map_spawn_scan_edges`' right-hand column (and the
    /// bonus level's right column, h0+$1788a): `$2e` (23 cells), plus twice
    /// [`Screen::extra_cells`] cells, since the origin moved left by as many.
    pub fn scan_right_column(self) -> u32 {
        0x2e + 2 * 2 * self.extra_cells() as u32
    }

    /// The cells `map_spawn_scan_edges`' row scans: `$19`, plus [`Screen::extra_cells`] each side.
    pub fn scan_row_cells(self) -> u32 {
        0x19 + 2 * self.extra_cells() as u32
    }

    /// The cells of each row `map_spawn_scan_screen` (h0+$1777c) scans: `$18`, plus
    /// [`Screen::extra_cells`] each side.
    pub fn scan_screen_cells(self) -> u32 {
        0x18 + 2 * self.extra_cells() as u32
    }

    /// `obj_free_if_left_behind`'s edge (h0+$1b940): an object whose right side is left of
    /// `cx − this` is freed, 0 (the camera), plus [`Screen::extra`].
    pub fn left_behind(self) -> u16 {
        self.extra()
    }

    /// Where a hardware sprite is hidden past the right of the screen, and parked to hide it
    /// (`frog/anim.rs`'s `place_frog_sprites`): `$150`, plus [`Screen::extra`] (DESIGN §2.5).
    pub fn sprite_hide_right(self) -> u16 {
        0x150 + self.extra()
    }

    /// Where a hardware sprite is hidden past the left of the screen: at `x + this < 0`, `$20`,
    /// plus [`Screen::extra`] (DESIGN §2.5).
    pub fn sprite_hide_left(self) -> u16 {
        0x20 + self.extra()
    }

    /// The camera's least x (`keep_inside`, `start_point`): `$10`, the view from level x 16, plus
    /// [`Screen::extra`] so the wide view starts there too (DESIGN §2.5, D3).
    pub fn camera_left(self) -> u16 {
        0x10 + self.extra()
    }

    /// The camera's greatest x is the level's width less this: `$168`, the view to level x
    /// `level_width − 26`, plus [`Screen::extra`] so the wide view ends there too (DESIGN §2.5).
    pub fn camera_right(self) -> u16 {
        0x168 + self.extra()
    }

    /// The bonus frog's least x is `cx` less this: `8`, plus [`Screen::extra`] (DESIGN §2.5).
    pub fn shooter_frog_left(self) -> u16 {
        8 + self.extra()
    }

    /// The bonus frog's greatest x is `cx` plus this: `$12a`, plus [`Screen::extra`] (DESIGN
    /// §2.5).
    pub fn shooter_frog_right(self) -> u16 {
        0x12a + self.extra()
    }

    /// Objects with their own edge tests (DESIGN §2.6). Spud's park bounds (`spud_update`): it is
    /// parked when `x ≤ cx − this`, 0, plus [`Screen::extra`] (the left edge, as a distance left
    /// of `cx`).
    pub fn spud_left(self) -> u16 {
        self.extra()
    }

    /// Spud is parked when `x > cx + this`: `$150`, plus [`Screen::extra`].
    pub fn spud_right(self) -> u16 {
        0x150 + self.extra()
    }

    /// Where `spawn_shots` puts the bonus level's first shot: `cx + $160`, plus
    /// [`Screen::extra`]; the others follow `$10` apart.
    pub fn shot_spawn(self) -> u16 {
        0x160 + self.extra()
    }

    /// A bonus shot is gone, ready to fire again (`shooter_spud`), once `x > cx + this`: `$150`,
    /// plus [`Screen::extra`].
    pub fn shot_gone(self) -> u16 {
        0x150 + self.extra()
    }

    /// A bonus shot moves while `x < cx + this`, and is parked there off the top or bottom
    /// (`shot_update`): `$200`, plus [`Screen::extra`].
    pub fn shot_far(self) -> u16 {
        0x200 + self.extra()
    }

    /// `direction`'s clamp of a screen x (`x − cx`, signed): `−extra ..= $153 + extra`, the
    /// original's `0 ..= $153` widened by [`Screen::extra`] each side.
    pub fn direction_x(self) -> (i32, i32) {
        let e = self.extra() as i32;
        (-e, 0x154 + e - 1)
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
        assert_eq!(s.sprite_hide_left(), 0x20);
        assert_eq!(s.camera_left(), 0x10);
        assert_eq!(s.camera_right(), 0x168);
        assert_eq!(s.shooter_frog_left(), 8);
        assert_eq!(s.shooter_frog_right(), 0x12a);
        assert_eq!(s.scan_left(), 0x20);
        assert_eq!(s.scan_right_column(), 0x2e);
        assert_eq!(s.scan_row_cells(), 0x19);
        assert_eq!(s.scan_screen_cells(), 0x18);
        assert_eq!(s.left_behind(), 0);
        assert_eq!((s.spud_left(), s.spud_right()), (0, 0x150));
        assert_eq!((s.shot_spawn(), s.shot_gone(), s.shot_far()), (0x160, 0x150, 0x200));
        assert_eq!(s.direction_x(), (0, 0x153));
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
        assert_eq!(s.sprite_hide_left(), 0x20 + 65);
        assert_eq!(s.camera_left(), 0x10 + 65);
        assert_eq!(s.camera_right(), 0x168 + 65);
        assert_eq!(s.shooter_frog_left(), 8 + 65);
        assert_eq!(s.shooter_frog_right(), 0x12a + 65);
        assert_eq!(s.scan_left(), 0x20 + 80);
        assert_eq!(s.scan_right_column(), 0x2e + 20);
        assert_eq!(s.scan_row_cells(), 0x19 + 10);
        assert_eq!(s.scan_screen_cells(), 0x18 + 10);
        assert_eq!(s.left_behind(), 65);
        assert_eq!((s.spud_left(), s.spud_right()), (65, 0x150 + 65));
        assert_eq!((s.shot_spawn(), s.shot_gone(), s.shot_far()), (0x160 + 65, 0x150 + 65, 0x200 + 65));
        assert_eq!(s.direction_x(), (-65, 0x153 + 65));
    }

    #[test]
    fn the_cell_move_covers_the_pixel_move() {
        assert!(WIDE_CELLS * 16 >= WIDE_EXTRA && (WIDE_CELLS - 1) * 16 < WIDE_EXTRA);
    }
}
