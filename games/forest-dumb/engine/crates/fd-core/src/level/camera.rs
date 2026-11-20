//! The camera: where the view's top-left is in the level, and the edge-redraw flags that tell the
//! foreground which new row or column of the map to draw (plans/forest-dumb-level-1 T09;
//! games/forest-dumb/docs/09-play-measured.md, "The camera").
//!
//! Ported from `camera_update` (`h0+$4d7e`), its four flag steps (`h0_4e9a`..`h0_4ec2`), the
//! one-pixel scrolls they loop on (`h0_51fc`..`h0_5346`) and the clamp `h0_513c`. Values are words as
//! the original keeps them, compared and wrapped at 16 bits: the original's `cmpi.w` overflows for
//! `camera_y` in `$8000..$853e`, and the clamp then gives `$53f`, not 0. Each routine is proven on the
//! routine rig (`fd-conform/tests/camera_routines.rs`).

/// A direction, indexing [`EdgeFlags`]'s arrays in the original's memory order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Right = 0,
    Left = 1,
    Up = 2,
    Down = 3,
}

/// The edge-redraw flags, one entry a direction, indexed by [`Dir`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EdgeFlags {
    /// `h0+$51f8..$51fb`: this pass crossed a 16-pixel boundary. `camera_update` clears them first
    /// and reads them last; the original stores `$ff` (`st.b`) and 0.
    pub crossed: [bool; 4],
    /// `h0+$5196`, `$5198`, `$519a`, `$519c`: a redraw is pending. Set to `$ffff` on a crossing;
    /// `main_loop` clears it once it has drawn the edge (T55). Any non-zero word blocks the next
    /// crossing that way.
    pub latch: [u16; 4],
    /// `h0+$5186`.. one (x, y) word pair a direction: the camera when that edge was last crossed,
    /// where `main_loop` draws it from.
    pub at: [(u16, u16); 4],
}

/// The camera and what it hands the foreground.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Camera {
    /// `camera_x` (`h0+$1e62`), the view's left edge in level pixels.
    pub x: u16,
    /// `camera_y` (`h0+$1e64`).
    pub y: u16,
    /// `h0+$4d76`: `char_x` with its low word less `camera_x`, as `camera_update` found it before any
    /// scroll. Its low word is `char_screen_x`.
    pub screen_x: u32,
    /// `h0+$4d7a`: the same for `char_y` and `camera_y`.
    pub screen_y: u32,
    pub edges: EdgeFlags,
}

/// The camera's largest x and y (`h0_513c` and the scroll steps).
pub const MAX_X: u16 = 0x17cf;
pub const MAX_Y: u16 = 0x53f;

/// N after `cmpi.w #imm, v`: the 16-bit difference is negative.
fn minus(v: u16, imm: u16) -> bool {
    v.wrapping_sub(imm) & 0x8000 != 0
}

/// The dead zone: the character's distance from the camera's edge past which each flag step runs.
/// Right and down run at or past it; left and up below it.
const EDGE: [u16; 4] = [0x94, 0x78, 0x46, 0x5a];

impl Camera {
    /// What `game_frame` runs: `camera_update`, then `h0_513c`. `target` is (`char_x`, `char_y`).
    pub fn step(&mut self, target: (u32, u32), frame_flag: &mut u16) {
        self.update(target, frame_flag);
        self.clamp();
    }

    /// The position's low word less the camera's, the rest kept: `move.l char_x, d0;
    /// sub.w camera_x, d0`.
    fn screen(&self, (cx, cy): (u32, u32)) -> (u32, u32) {
        let low = |v: u32, c: u16| (v & 0xffff_0000) | (v as u16).wrapping_sub(c) as u32;
        (low(cx, self.x), low(cy, self.y))
    }

    /// `camera_update` (`h0+$4d7e`): follow the character out of the dead zone, right, left, down,
    /// up in that order, each from the camera the one before left; then note where the camera was
    /// for each edge crossed.
    pub fn update(&mut self, target: (u32, u32), frame_flag: &mut u16) {
        self.edges.crossed = [false; 4];
        (self.screen_x, self.screen_y) = self.screen(target);
        for dir in [Dir::Right, Dir::Left, Dir::Down, Dir::Up] {
            let (sx, sy) = self.screen(target);
            let d = match dir {
                Dir::Right | Dir::Left => sx as u16,
                Dir::Up | Dir::Down => sy as u16,
            };
            // `bmi` skips right and down below the edge; `bpl` skips left and up at or past it.
            let below = minus(d, EDGE[dir as usize]);
            let runs = match dir {
                Dir::Right | Dir::Down => !below,
                Dir::Left | Dir::Up => below,
            };
            if runs {
                self.follow(dir, d, frame_flag);
            }
        }
        for i in 0..4 {
            if self.edges.crossed[i] {
                self.edges.at[i] = (self.x, self.y);
            }
        }
    }

    /// A flag step (`h0_4e9a` right, `h0_4ea6` left, `h0_4eb4` up, `h0_4ec2` down): from the
    /// character's distance `d` (low word of `d0` or `d1`), scroll one pixel at a time until a scroll
    /// refuses or the count runs out: up to `d - $93` pixels right, `$78 - d` left, `$46 - d` up,
    /// `d - $59` down, at 16 bits, as the original's `subi.l`, `not.l`, `subi.l #1` and `dbra` make it
    /// (at least one; a wrapped count runs 65 536). Returns whether the last scroll refused (the
    /// original's `d0`, -1 or 0).
    pub fn follow(&mut self, dir: Dir, d: u16, frame_flag: &mut u16) -> bool {
        // `subi.l #bias, d7`, `not.l d7` for left and up, then `subi.l #1, d7`; `dbra` on the low word.
        let mut n = match dir {
            Dir::Right => d.wrapping_sub(0x93),
            Dir::Left => !d.wrapping_sub(0x79),
            Dir::Up => !d.wrapping_sub(0x47),
            Dir::Down => d.wrapping_sub(0x59),
        }
        .wrapping_sub(1);
        loop {
            if self.scroll(dir, frame_flag) {
                return true;
            }
            n = n.wrapping_sub(1);
            if n == 0xffff {
                return false;
            }
        }
    }

    /// A one-pixel scroll (`h0_51fc` right, `h0_526a` left, `h0_52d8` up, `h0_5346` down). Within a
    /// 16-pixel cell it just moves. Crossing into the next it moves only if that direction's latch is
    /// clear, then clamps, and sets the crossed flag, the latch and `frame_flag`. Returns whether it
    /// refused (the original's `d0 = -1`).
    pub fn scroll(&mut self, dir: Dir, frame_flag: &mut u16) -> bool {
        let i = dir as usize;
        let horizontal = matches!(dir, Dir::Right | Dir::Left);
        let forward = matches!(dir, Dir::Right | Dir::Down);
        let c = if horizontal { self.x } else { self.y };
        let n = if forward {
            c.wrapping_add(1)
        } else {
            c.wrapping_sub(1)
        };
        if n & 0xfff0 == c & 0xfff0 {
            self.set(horizontal, n);
            return false;
        }
        if self.edges.latch[i] != 0 {
            return true;
        }
        // Forward: `cmpi.w #max; bmi` keeps it. Back: `tst.w; bpl` keeps it.
        let max = if horizontal { MAX_X } else { MAX_Y };
        let kept = if forward { minus(n, max) } else { !minus(n, 0) };
        let limit = if forward { max } else { 0 };
        self.set(horizontal, if kept { n } else { limit });
        self.edges.crossed[i] = true;
        self.edges.latch[i] = 0xffff;
        *frame_flag = 0xffff;
        false
    }

    fn set(&mut self, horizontal: bool, v: u16) {
        if horizontal {
            self.x = v;
        } else {
            self.y = v;
        }
    }

    /// `h0_513c`: `camera_y` to `0..=$53f`, then `camera_x` to `0..=$17cf`, each by a signed
    /// 16-bit compare that can overflow (a `y` of `$8000..$853e` becomes `$53f`).
    pub fn clamp(&mut self) {
        if !minus(self.y, MAX_Y) {
            self.y = MAX_Y;
        }
        if minus(self.y, 0) {
            self.y = 0;
        }
        if !minus(self.x, MAX_X) {
            self.x = MAX_X;
        }
        if minus(self.x, 0) {
            self.x = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: u16, y: u16) -> Camera {
        Camera {
            x,
            y,
            ..Camera::default()
        }
    }

    #[test]
    fn a_scroll_inside_a_cell_moves_and_sets_nothing() {
        let (mut c, mut ff) = (at(0x20, 0x20), 0);
        assert!(!c.scroll(Dir::Right, &mut ff));
        assert_eq!((c.x, c.edges, ff), (0x21, EdgeFlags::default(), 0));
    }

    #[test]
    fn crossing_a_cell_sets_flag_latch_and_frame_flag_once() {
        let (mut c, mut ff) = (at(0x2f, 0x20), 0);
        assert!(!c.scroll(Dir::Right, &mut ff));
        assert_eq!(c.x, 0x30);
        assert_eq!(
            (c.edges.crossed, c.edges.latch, ff),
            ([true, false, false, false], [0xffff, 0, 0, 0], 0xffff)
        );
        // The latch blocks the next crossing that way: the camera stays at the cell's edge.
        c.x = 0x3f;
        assert!(c.scroll(Dir::Right, &mut ff));
        assert_eq!(c.x, 0x3f);
    }

    #[test]
    fn crossing_past_a_limit_clamps() {
        let mut ff = 0;
        let mut c = at(0, 0);
        assert!(!c.scroll(Dir::Left, &mut ff));
        assert_eq!((c.x, c.edges.latch[Dir::Left as usize]), (0, 0xffff));
        let mut c = at(MAX_X, 0);
        assert!(!c.scroll(Dir::Right, &mut ff));
        assert_eq!(c.x, MAX_X);
    }

    #[test]
    fn the_clamp_overflows_as_cmpi_w_does() {
        let mut c = at(0x8000, 0x8000);
        c.clamp();
        // y: `$8000 - $53f` is positive at 16 bits, so `$53f`; x: `$8000 - $17cf` likewise.
        assert_eq!((c.x, c.y), (MAX_X, MAX_Y));
        let mut c = at(0xffff, 0xffff);
        c.clamp();
        assert_eq!((c.x, c.y), (0, 0));
    }

    #[test]
    fn update_follows_right_to_the_next_boundary_and_records_where() {
        let (mut c, mut ff) = (at(0x20, 0), 0);
        // The character `$a0` right of the camera: `$c` past the edge, so up to 13 pixels, but the
        // boundary at `$30` is crossed after 16 and the second one is blocked by the latch.
        c.update((0x20 + 0xa0, 0x50), &mut ff);
        assert_eq!(c.x, 0x2d);
        assert_eq!(c.screen_x, 0xa0);
        assert_eq!(c.edges.crossed, [false; 4]);
        c.update((0x20 + 0xc0, 0x50), &mut ff);
        assert_eq!(c.x, 0x30 + 0xf);
        assert_eq!(c.edges.crossed, [true, false, false, false]);
        assert_eq!(c.edges.at[0], (0x30 + 0xf, 0));
    }
}
