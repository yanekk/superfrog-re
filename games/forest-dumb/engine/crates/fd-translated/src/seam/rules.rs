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
    let s = &*l.state;
    match offset {
        0x9b06 => sprite_copy(s.motion.unhurt_t, &mut d),
        0x94ba => fade(l, &mut d),
        0x4d7e => camera_update(l, &mut d),
        0x513c => clamp(&s.camera, &mut d),
        0x5462 => sprite_position(l, &mut d),
        0x9754 => copper_pointers(s.camera.y, &mut d),
        // `read_joystick`'s branches hang on JOY1DAT: [`after_joy1dat`], once it is read.
        0x8dfc => {}
        0x20b0 => countdown_step(l, &mut d),
        0x2104 => play_keys(l, &mut d),
        0x21c0 => spring_redraw(l, &mut d),
        0x2c7e => creatures_restore(l, &mut d),
        0x2d4c => creatures_cut(l, &mut d),
        _ => return None,
    }
    Some(d)
}

/// The routines with a rule.
pub const RULED: [u32; 12] = [
    0x9b06, 0x94ba, 0x4d7e, 0x513c, 0x5462, 0x9754, 0x8dfc, 0x20b0, 0x2104, 0x21c0, 0x2c7e, 0x2d4c,
];

/// `read_joystick`'s decisions after its read of JOY1DAT, `v`: the four direction bits (`btst #9`
/// and `#1` of `v`, `#8` and `#0` of `v ^ v >> 1`), then, unless `input_log` is full, a test of each
/// flag it set for the log's byte. `full`: [`log_full`] as the routine found it.
pub fn after_joy1dat(v: u16, full: bool) -> Vec<Decision> {
    let x = v ^ (v >> 1);
    let (left, right, up, down) = (v & 0x200 != 0, v & 2 != 0, x & 0x100 != 0, x & 1 != 0);
    let mut d = vec![Branch(!left), Branch(!right), Branch(!up), Branch(!down)];
    d.push(Branch(full));
    if !full {
        d.extend([Branch(!left), Branch(!right), Branch(!up), Branch(!down)]);
    }
    d
}

/// `input_log_index` at its end (`cmpi.l #$2ee0; beq`): `read_joystick` logs nothing more.
pub fn log_full(l: &LevelCtx) -> bool {
    l.state.input_log.index == 0x2ee0
}

/// `char_sprite_copy` (`h0_9b06`): with `char_unhurt_t` 0 the frame is copied; else it steps, and
/// on two steps of four the sprite is copied, on the others cleared (`h0+$9c30`, four rows of ten
/// `clr.l` runs).
fn sprite_copy(unhurt: u8, d: &mut Vec<Decision>) {
    d.push(Branch(unhurt != 0));
    if unhurt == 0 {
        return;
    }
    let k = unhurt.wrapping_add(1) & 3;
    d.push(Branch(k == 0));
    if k == 0 {
        return;
    }
    d.push(Branch(k == 1));
    if k == 1 {
        return;
    }
    for row in (0..=3u16).rev() {
        for run in (0..=9u16).rev() {
            d.push(Count(run));
        }
        d.push(Count(row));
    }
}

/// `h0_94ba`, the fade: off, nothing; on, `h0_94fe` steps each of the 32 colours by `h0_9550`'s
/// three components (`beq` when the component is reached, else `beq` when the fade table says
/// wait), then the pass count's test for the end.
fn fade(l: &LevelCtx, d: &mut Vec<Decision>) {
    let f = &l.display.fade;
    d.push(Branch(f.on != 0));
    if f.on == 0 {
        return;
    }
    let to = l.display_data.palette();
    let d6 = (f.pass >> 1) as u16;
    for (i, (&now, &to)) in f.now.iter().zip(to.iter()).enumerate() {
        for (mask, shift) in [(0xfu16, 4i32), (0xf0, 0), (0xf00, -4)] {
            let (t, n) = (to & mask, now & mask);
            d.push(Branch(t == n));
            if t != n {
                let row = if shift >= 0 { t << shift } else { t >> -shift };
                let at = fd_core::render::palette::FADE_TABLE + row.wrapping_add(d6) as u32;
                d.push(Branch(l.program.byte(at) == 0));
            }
        }
        d.push(Count(31 - i as u16));
    }
    d.push(Branch(f.pass.wrapping_add(1) != 0x20));
}

/// `char_sprite_position` (`h0_5462`): bit 0 of the screen x, then bit 8 of the screen y plus $31,
/// then of that plus $28.
fn sprite_position(l: &LevelCtx, d: &mut Vec<Decision>) {
    let s = &*l.state;
    let x = (s.char_x as u16).wrapping_sub(s.camera.x);
    let y = (s.char_y & 0xffff_0000 | (s.char_y as u16).wrapping_sub(s.camera.y) as u32)
        .wrapping_add(0x31);
    d.push(Branch(x & 1 == 0));
    d.push(Branch(y & 0x100 == 0));
    d.push(Branch(y.wrapping_add(0x28) & 0x100 == 0));
}

/// `h0_9754`: two loops of five copper pointers, then `cmpi.l #$f0; bmi` on the split line.
fn copper_pointers(camera_y: u16, d: &mut Vec<Decision>) {
    for _ in 0..2 {
        d.extend((0..=4u16).rev().map(Count));
    }
    let line = ((camera_y as u32 & 0xff) ^ 0xff) + 0x2f;
    d.push(Branch(line < 0xf0));
}

/// `countdown_step` (`h0+$20b0`): `bpl` at 63 passes, then `bne` while the countdown's word is not 0.
fn countdown_step(l: &LevelCtx, d: &mut Vec<Decision>) {
    let s = &*l.state;
    let tick = s.countdown_tick.wrapping_add(1);
    let due = (tick.wrapping_sub(0x3f) as i32) >= 0;
    d.push(Branch(due));
    if due {
        d.push(Branch(s.countdown >> 16 != 0));
    }
}

/// `play_keys` (`h0_2104`): `help_key` (HELP with the cheat clears the key byte), `esc_key`, then
/// the pause key's press and release.
fn play_keys(l: &LevelCtx, d: &mut Vec<Decision>) {
    let k = &l.state.keys;
    let mut key = k.last;
    d.push(Branch(key == 0x41));
    if key == 0x41 {
        d.push(Branch(k.cheat != 0));
        if k.cheat != 0 {
            key = 0;
        }
    }
    d.push(Branch(key != 0x75));
    if key != 0x75 {
        d.push(Branch(key != 0x74));
        if key == 0x74 {
            d.push(Branch(k.esc_down == 0));
        }
    }
    d.push(Branch(key != 0xcd));
    if key != 0xcd {
        d.push(Branch(key != 0xcc));
        if key == 0xcc {
            d.push(Branch(k.pause_down == 0));
        }
    }
}

/// A sound's wait of `lines` changes of VHPOSR's high byte: `moveq #lines-1, d0`, each turn a
/// `cmp.b; beq` loop and `dbra d0`.
fn line_wait(lines: u16, d: &mut Vec<Decision>) {
    for c in (0..lines).rev() {
        d.push(Decision::Same);
        d.push(Count(c));
    }
}

/// `h0_21c0`, the spring's picture: nothing while its timer is 0; from 6 on, four cell blits (the
/// third's row wrapped past the play screen's end, `cmpa.l; bmi`) and the timer cleared; at 1 the
/// spring sound and two picture blits (`h0_3ab4` waits), by its kind; else the timer steps.
fn spring_redraw(l: &LevelCtx, d: &mut Vec<Decision>) {
    let sp = &l.state.spring;
    d.push(Branch(sp.timer != 0));
    if sp.timer == 0 {
        return;
    }
    let early = minus(sp.timer, 6);
    d.push(Branch(early));
    if !early {
        // The third cell's address less the play screen's end, `cmpa.l #h3+$e998; bmi`: `lsr.w`
        // and `andi.b` leave `spring_x`'s high word in the long `adda.l` adds.
        let col = sp.x & 0xffff_0000 | (((sp.x as u16) >> 3) & !1) as u32;
        let row = (sp.y as u16 & 0xf0) as u32 * 0xe6;
        let third = (0x398u32 + 0xe60).wrapping_add(col).wrapping_add(row);
        d.extend([Decision::Busy, Decision::Busy]);
        d.push(Branch(third.wrapping_sub(0xe998) & 0x8000_0000 != 0));
        d.extend([Decision::Busy, Decision::Busy]);
        return;
    }
    d.push(Branch(sp.timer == 1));
    if sp.timer == 1 {
        line_wait(3, d);
        d.push(Branch(sp.kind != 0));
        d.extend([Decision::Busy, Decision::Busy]);
    }
}

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

/// The play screen's line, its first and last lines' offsets in hunk 3 (`h3+$398`, `h3+$e998`) and
/// a band of 16 lines.
const LINE: u32 = 0xe6;
const SCREEN_AT: u32 = 0x398;
const SCREEN_END: u32 = SCREEN_AT + 0x100 * LINE;
const BAND: u32 = 0x10 * LINE;
const VIEW_LINES: u16 = 0xd0;

/// `cmpa.l #h3+$e998, a0; bmi` past a wrap: the screen's last line passed.
fn wrapped(at: u32) -> bool {
    at.wrapping_sub(SCREEN_END) & 0x8000_0000 == 0
}

/// `h0_2c7e`: for each of the five slots (`bne` on to the next), when its drawn flag is set, the
/// tiles behind it restored by `h0_5e44`: three cell blits on its row and the next, the next a map
/// row up past the view's foot (`bpl`), the screen wrapped (`bmi`); kinds 0, 3 and 7 off a cell's
/// top line and above the view's foot get a third row.
fn creatures_restore(l: &LevelCtx, d: &mut Vec<Decision>) {
    let c = &l.state.edges.creatures;
    let cam = l.state.camera.y;
    for i in 0..5 {
        let on = c.active[i] != 0;
        d.push(Branch(!on));
        if on {
            let v = c.slots[i];
            let (x, y) = (v.x as u32, v.y as u32);
            let mut at = SCREEN_AT + ((x >> 3) & 0xfffe) + (y & 0xf0) * LINE;
            let mut next_row = |d: &mut Vec<Decision>| {
                at = at.wrapping_add(BAND);
                d.push(Branch(!wrapped(at)));
                if wrapped(at) {
                    at = at.wrapping_sub(0x100 * LINE);
                }
                d.extend([Decision::Busy; 3]);
            };
            d.extend([Decision::Busy; 3]);
            let foot = cam.wrapping_add(VIEW_LINES);
            d.push(Branch(
                foot.wrapping_sub(v.y.wrapping_add(0x10)) & 0x8000 == 0,
            ));
            next_row(d);
            let tall = [0, 3, 7].iter().position(|&k| k == v.kind);
            for k in 0..=tall.unwrap_or(2) {
                d.push(Branch(Some(k) == tall));
            }
            if tall.is_some() {
                d.push(Branch(y & 0xf != 0));
                if y & 0xf != 0 {
                    let above = (y + 0x20).wrapping_sub(cam as u32 + VIEW_LINES as u32);
                    d.push(Branch(above & 0x8000_0000 != 0));
                    if above & 0x8000_0000 != 0 {
                        next_row(d);
                    }
                }
            }
        }
        d.push(Branch(i != 4));
    }
}

/// `h0_2d4c`: the cuts' registers, then for each of the five slots whose flag is set `h0_5820`'s
/// cut: whole when its 32 lines stay on the screen (`btst #8; beq`), else in two at the screen's
/// last line (the second left out when it would be empty); each part tests the facing (`bmi`) and
/// kind 7 (`bne`) for its frame.
fn creatures_cut(l: &LevelCtx, d: &mut Vec<Decision>) {
    let c = &l.state.edges.creatures;
    for i in 0..5 {
        let on = c.active[i] != 0;
        d.push(Branch(!on));
        if on {
            let v = c.slots[i];
            let frame = [Branch(v.facing & 0x80 != 0), Branch(v.kind != 7)];
            let low = v.y as u32 & 0xff;
            let split = (low + 0x20) & 0x100 != 0;
            d.push(Branch(!split));
            d.extend(frame);
            if !split {
                d.push(Decision::Busy);
            } else {
                d.push(Branch(low ^ 0xff == 0x1f));
                d.push(Decision::Busy);
                d.extend(frame);
                let rest = v.y.wrapping_add(0x20) & 0xff;
                d.push(Branch(rest != 0));
                if rest != 0 {
                    d.push(Decision::Busy);
                }
            }
        }
        d.push(Branch(i != 4));
    }
}
