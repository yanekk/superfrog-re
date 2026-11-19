//! The timing rules (translator T56, DESIGN §2.9): for each routine `game_frame` calls, the path
//! the original takes through it, as the [`Decision`]s a [`super::walk::Walk`] meets, worked out
//! from the level as the routine finds it. Each mirrors the original's branches over the hand
//! engine's typed state, and is held to the translated routine's counted cycles on the routine
//! judge's states (`fd-conform/tests/mount_timing.rs`).
//!
//! A routine with no rule here is timed by its measured cost (`frame.rs`, `COSTS`).

use fd_core::level::LevelCtx;
use fd_core::level::camera::{Camera, Dir, MAX_X, MAX_Y};

use super::walk::Decision::{self, Branch, Count};

/// The decisions of the routine at hunk 0 `offset` on `l`, or `None` when it has no rule.
pub fn path(offset: u32, l: &LevelCtx) -> Option<Vec<Decision>> {
    let mut d = Vec::new();
    match offset {
        0x4d7e => camera_update(l, &mut d),
        0x513c => clamp(&l.state.camera, &mut d),
        _ => return None,
    }
    Some(d)
}

/// The routines with a rule.
pub const RULED: [u32; 2] = [0x4d7e, 0x513c];

/// N after `cmpi.w #imm, v`.
fn minus(v: u16, imm: u16) -> bool {
    v.wrapping_sub(imm) & 0x8000 != 0
}

/// `camera_update` (`h0+$4d7e`): the four dead-zone tests, each flag step's loop of one-pixel
/// scrolls (`h0+$51a0`..`$51f6`, `h0_51fc`..`h0_5346`), then the four crossed flags' tests.
fn camera_update(l: &LevelCtx, d: &mut Vec<Decision>) {
    let s = &*l.state;
    let mut c = s.camera.clone();
    let mut flag = s.frame_flag;
    let target = (s.char_x, s.char_y);
    let low = |v: u32, k: u16| (v as u16).wrapping_sub(k);
    c.edges.crossed = [false; 4];
    // Right (`bmi` skips), left (`bpl`), down (`bmi`), up (`bpl`), in the original's order.
    for (dir, edge) in [
        (Dir::Right, 0x94),
        (Dir::Left, 0x78),
        (Dir::Down, 0x5a),
        (Dir::Up, 0x46),
    ] {
        let v = match dir {
            Dir::Right | Dir::Left => low(target.0, c.x),
            Dir::Up | Dir::Down => low(target.1, c.y),
        };
        let below = minus(v, edge);
        let skip = match dir {
            Dir::Right | Dir::Down => below,
            Dir::Left | Dir::Up => !below,
        };
        d.push(Branch(skip));
        if !skip {
            follow(&mut c, dir, v, &mut flag, d);
        }
    }
    // `tst.b` each crossed flag, `beq` past its copy of the camera.
    for i in [0, 1, 2, 3] {
        d.push(Branch(!c.edges.crossed[i]));
    }
}

/// A flag step's loop: `subi.l #1, d7`, then `bsr` a scroll, `tst.l d0; bne` out, `dbra d7`.
fn follow(c: &mut Camera, dir: Dir, v: u16, flag: &mut u16, d: &mut Vec<Decision>) {
    let mut n = match dir {
        Dir::Right => v.wrapping_sub(0x93),
        Dir::Left => !v.wrapping_sub(0x79),
        Dir::Up => !v.wrapping_sub(0x47),
        Dir::Down => v.wrapping_sub(0x59),
    }
    .wrapping_sub(1);
    loop {
        let refused = scroll(c, dir, flag, d);
        d.push(Branch(refused));
        if refused {
            return;
        }
        d.push(Count(n));
        if n == 0 {
            return;
        }
        n = n.wrapping_sub(1);
    }
}

/// A one-pixel scroll's branches: `bne` past the in-cell move, `bne` on the latch, then the
/// limit's test (`bmi` forward, `bpl` back). Moves `c` as the hand engine's [`Camera::scroll`].
fn scroll(c: &mut Camera, dir: Dir, flag: &mut u16, d: &mut Vec<Decision>) -> bool {
    let horizontal = matches!(dir, Dir::Right | Dir::Left);
    let forward = matches!(dir, Dir::Right | Dir::Down);
    let v = if horizontal { c.x } else { c.y };
    let n = if forward {
        v.wrapping_add(1)
    } else {
        v.wrapping_sub(1)
    };
    let crossing = n & 0xfff0 != v & 0xfff0;
    d.push(Branch(crossing));
    if crossing {
        let latched = c.edges.latch[dir as usize] != 0;
        d.push(Branch(latched));
        if !latched {
            let max = if horizontal { MAX_X } else { MAX_Y };
            d.push(Branch(if forward { minus(n, max) } else { !minus(n, 0) }));
        }
    }
    c.scroll(dir, flag)
}

/// `h0_513c`: `cmpi.w #$53f; bmi`, `tst.w; bpl` for y, then the same for x.
fn clamp(c: &Camera, d: &mut Vec<Decision>) {
    let mut y = c.y;
    d.push(Branch(minus(y, MAX_Y)));
    if !minus(y, MAX_Y) {
        y = MAX_Y;
    }
    d.push(Branch(!minus(y, 0)));
    let x = c.x;
    d.push(Branch(minus(x, MAX_X)));
    let x = if minus(x, MAX_X) { x } else { MAX_X };
    d.push(Branch(!minus(x, 0)));
}
