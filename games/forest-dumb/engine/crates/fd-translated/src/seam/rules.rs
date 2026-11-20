//! The timing rules (translator T56, DESIGN §2.9): for each routine `game_frame` calls, the path
//! the original takes through it, as the [`Decision`]s a [`super::walk::Walk`] meets, worked out
//! from the level as the routine finds it. Each mirrors the original's branches over the hand
//! engine's typed state, and is held to the translated routine's counted cycles on the routine
//! judge's states (`fd-conform/tests/mount_timing.rs`).
//!
//! A routine with no rule here is timed by its measured cost (`frame.rs`, `COSTS`).

use fd_core::character::Character;
use fd_core::character::physics::POINTS;
use fd_core::level::LevelCtx;
use fd_core::level::camera::{Camera, Dir, MAX_X, MAX_Y};
use fd_core::level::collision;
use fd_core::program::HUNK_BASES;
use fd_core::program::Scratch;

use super::walk::Decision::{self, Branch, Count, Reg};

/// Memory as a routine reads it, by hunk and offset: for what the typed state does not hold or
/// holds only at the pass's end (a cell past the object map's end, the collision map as the
/// foreground left it).
pub type Peek<'a> = &'a dyn Fn(u32, u32) -> u8;

/// The decisions of the routine at hunk 0 `offset` on `l` (and memory, `peek`), or `None` when it
/// has no rule.
pub fn path(offset: u32, l: &LevelCtx, peek: Peek) -> Option<Vec<Decision>> {
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
        // `char_hit_check`'s on CLXDAT: [`after_clxdat`], once it is read.
        0x3b2a => {}
        0x3e0c => char_update(l, peek, &mut d),
        0x20b0 => countdown_step(l, &mut d),
        0x2104 => play_keys(l, &mut d),
        0x21c0 => spring_redraw(l, &mut d),
        0x2c7e => creatures_restore(l, &mut d),
        0x2d4c => creatures_cut(l, &mut d),
        0x3698 => creatures_sort(l, &mut d),
        0x24ce => spring_trigger(l, peek, &mut d),
        0x2dd6 => creatures_write_back(l, peek, &mut d),
        0x2fae => creatures_move(l, peek, &mut d),
        0x62b6 => animated_cells(l, peek, &mut d),
        0x2698 => object_cells(l, peek, &mut d),
        _ => return None,
    }
    Some(d)
}

/// The routines with a rule.
pub const RULED: [u32; 20] = [
    0x2698, 0x62b6, 0x2fae, 0x2dd6, 0x3698, 0x24ce, 0x9b06, 0x94ba, 0x4d7e, 0x513c, 0x5462, 0x9754,
    0x8dfc, 0x20b0, 0x2104, 0x21c0, 0x2c7e, 0x2d4c, 0x3b2a, 0x3e0c,
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

/// `h0_3698`: five passes (`dbra`) of a bubble sort of the five slots' y, each four compares and
/// swaps; the middle two compare bytes (`cmp.b`), the outer two words (`bpl` keeps the order).
fn creatures_sort(l: &LevelCtx, d: &mut Vec<Decision>) {
    let mut y: [u16; 5] = core::array::from_fn(|i| l.state.edges.creatures.slots[i].y);
    for pass in (0..=4u16).rev() {
        for (k, byte) in [(0, false), (1, true), (2, true), (3, false)] {
            let (a, b) = (y[k], y[k + 1]);
            let kept = if byte {
                (b as u8).wrapping_sub(a as u8) & 0x80 == 0
            } else {
                b.wrapping_sub(a) & 0x8000 == 0
            };
            d.push(Branch(kept));
            if !kept {
                y.swap(k, k + 1);
            }
        }
        d.push(Count(pass));
    }
}

/// `h0_24ce`: while falling and not dying, the object map's cell under the feet tested against the
/// four spring values (`beq` each).
fn spring_trigger(l: &LevelCtx, peek: Peek, d: &mut Vec<Decision>) {
    let s = &*l.state;
    d.push(Branch(s.motion.falling != 0));
    if s.motion.falling == 0 {
        return;
    }
    d.push(Branch(s.motion.dying == 0));
    if s.motion.dying != 0 {
        return;
    }
    // `addi.l`, `lsr.l #4`, `mulu.w #$60` of the column's low word, then the row added.
    let col = (s.char_x.wrapping_add(0xe) >> 4) & 0xffff;
    let at = (col * 0x60).wrapping_add(s.char_y.wrapping_add(0x23) >> 4);
    let v = peek(0, fd_core::level::redraw::OBJECT_MAP.wrapping_add(at));
    for k in 0x10..=0x13 {
        d.push(Branch(v == k));
        if v == k {
            return;
        }
    }
}

/// `h0_2dd6`: for each of the five slots (`bne` on), when its flag is set, `h0_2e0e` tests the
/// creature against the view's four sides in turn (`bpl`/`bmi`) and, outside one, writes its kind
/// plus one into the object map at its cell, or the cell beside it the side's way when that holds
/// something, or the one past (`beq` each), and clears its flag. The cells written are seen by the
/// slots after.
fn creatures_write_back(l: &LevelCtx, peek: Peek, d: &mut Vec<Decision>) {
    let s = &*l.state;
    let (cx, cy) = (s.camera.x, s.camera.y);
    let mut written: Vec<(u32, u8)> = Vec::new();
    let map = |w: &Vec<(u32, u8)>, at: u32| {
        w.iter().rev().find(|e| e.0 == at).map_or_else(
            || peek(0, fd_core::level::redraw::OBJECT_MAP.wrapping_add(at)),
            |e| e.1,
        )
    };
    let neg = |v: u16| v & 0x8000 != 0;
    for i in 0..5 {
        let on = s.edges.creatures.active[i] != 0;
        d.push(Branch(!on));
        if on {
            let v = s.edges.creatures.slots[i];
            let (x, y) = (v.x, v.y);
            let left = cx.wrapping_sub(0x10);
            let right = left.wrapping_add(0x140);
            let up = cy.wrapping_sub(0x20);
            let down = up.wrapping_add(0xe5);
            // (the side's test taken, then the first cell and the two steps from it)
            let sides: [(bool, u16, i32, i32); 4] = [
                (!neg(left.wrapping_sub(x)), y, 0, -0x60),
                (neg(right.wrapping_sub(x)), y, 0x60, 0x60),
                (!neg(up.wrapping_sub(y)), y.wrapping_add(1), 0, 0x60),
                (neg(down.wrapping_sub(y)), y.wrapping_add(1), 0, 0x60),
            ];
            let backs = [0xc0i32, -0xc0, -0x60, -0xc0];
            for (k, &(out, row, first, step)) in sides.iter().enumerate() {
                d.push(Branch(out));
                if !out {
                    continue;
                }
                let base = (x as u32 >> 4) * 0x60 + (row as u32 >> 4);
                let mut at = base.wrapping_add(first as u32);
                let empty = map(&written, at) == 0;
                d.push(Branch(empty));
                if !empty {
                    at = at.wrapping_add(step as u32);
                    let empty = map(&written, at) == 0;
                    d.push(Branch(empty));
                    if !empty {
                        at = at.wrapping_add(backs[k] as u32);
                    }
                }
                written.push((at, v.kind.wrapping_add(1) as u8));
                break;
            }
        }
        d.push(Branch(i != 4));
    }
}

/// `h0_2fae`: for each of the five slots (`bne` on) whose flag is set, `h0_2fea` by its kind (`beq`
/// on 0 to 5 and 7, any other returns): 7 counts its phase to `$20` (`bpl`); 0 faces the character
/// (`bpl`) and steps by the phase's tables; 1 to 5 step their phase, test the facing (`bpl`), move
/// by their tables and probe the collision map ahead of their feet and under them (`h0_3530`,
/// `h0_3566`, `h0_35a0`, `h0_35dc`, `beq`/`bne`), turning on a wall or an edge.
fn creatures_move(l: &LevelCtx, peek: Peek, d: &mut Vec<Decision>) {
    let s = &*l.state;
    let table = |at: u32, phase: u8| l.program.byte(at + (phase & 0x1f) as u32);
    let map = |x: u16, y: u16, dx: u16, dy: u32| {
        let row = (y as u32).wrapping_add(dy) & 0xff;
        peek(
            3,
            fd_core::level::collision::MAP + row * 0x2e + (x.wrapping_add(dx) as u32 >> 3),
        )
    };
    for i in 0..5 {
        let on = s.edges.creatures.active[i] != 0;
        d.push(Branch(!on));
        if on {
            let v = s.edges.creatures.slots[i];
            let (mut x, mut y, mut facing) = (v.x, v.y, v.facing);
            let phase = v.phase.wrapping_add(1);
            let kinds = [0u16, 1, 2, 3, 4, 5, 7];
            let k = kinds.iter().position(|&k| k == v.kind);
            for j in 0..=k.unwrap_or(kinds.len() - 1) {
                d.push(Branch(Some(j) == k));
            }
            // A probe: `beq` (a wall ahead turns it) or `bne` (an edge turns it).
            let probe = |d: &mut Vec<Decision>,
                         x: u16,
                         y: u16,
                         facing: &mut u8,
                         dx: u16,
                         dy: u32,
                         wall: bool,
                         to: u8| {
                let m = map(x, y, dx, dy);
                d.push(Branch(if wall { m == 0 } else { m != 0 }));
                if (m != 0) == wall {
                    *facing = to;
                }
            };
            match v.kind {
                7 => d.push(Branch(phase.wrapping_sub(0x20) & 0x80 == 0)),
                0 => {
                    let right = (x.wrapping_add(0x10).wrapping_sub(s.char_x as u16)) & 0x8000 == 0;
                    d.push(Branch(right));
                    d.push(Branch(!right));
                }
                1..=5 => {
                    let back = facing & 0x80 != 0;
                    d.push(Branch(!back));
                    // The move before the probes (kind 5 moves after them).
                    let (left_t, right_t, dy_t): (u32, u32, Option<u32>) = match v.kind {
                        2 => (0x323a, 0x328a, None),
                        3 => (0x334c, 0x334c, Some(0x336c)),
                        4 => (0x33f6, 0x3416, None),
                        _ => (0, 0, None),
                    };
                    match v.kind {
                        1 => {
                            x = if back {
                                x.wrapping_sub(1)
                            } else {
                                x.wrapping_add(1)
                            }
                        }
                        2..=4 => {
                            let t = table(if back { left_t } else { right_t }, phase) as u16;
                            x = if back {
                                x.wrapping_sub(t)
                            } else {
                                x.wrapping_add(t)
                            };
                            if let Some(at) = dy_t {
                                y = y.wrapping_add(table(at, phase) as i8 as i16 as u16);
                            }
                        }
                        _ => {}
                    }
                    // Kind 3 probes only under its feet; the others ahead first.
                    if back {
                        if v.kind != 3 {
                            probe(d, x, y, &mut facing, 0xffff, 0x21, false, 0);
                        }
                        probe(d, x, y, &mut facing, 0, 0x18, true, 0);
                    } else {
                        if v.kind != 3 {
                            probe(d, x, y, &mut facing, 0x21, 0x21, false, 0xff);
                        }
                        probe(d, x, y, &mut facing, 0x20, 0x18, true, 0xff);
                    }
                }
                _ => {}
            }
            let _ = (x, y, facing);
        }
        d.push(Branch(i != 4));
    }
}

/// `h0_62b6`: the animation's phase stepped (`bne` past the tick's step), then the phase's list of
/// twelve cells (`bne` on to the next): a used cell whose object map byte is gone (`bne`) or out of
/// the view (`bpl`, `bmi`, `bpl`, `bmi`) is dropped; else by its value (`bpl`, `bne`, `beq`, `bne`,
/// `bpl`, `bmi`, `beq`) an element frame, water, the second water, the end of the list, a
/// vanishing block (its collision set or cleared by the frame table's word, `beq`) or the flame.
fn animated_cells(l: &LevelCtx, peek: Peek, d: &mut Vec<Decision>) {
    let s = &*l.state;
    let a = s.animation;
    let phase = a.phase.wrapping_add(1) & 3;
    d.push(Branch(phase != 0));
    let tick = if phase == 0 {
        a.tick.wrapping_add(1)
    } else {
        a.tick
    };
    let list = phase as usize;
    let c = &s.camera;
    let below = |p: u16, q: u16| p.wrapping_sub(q) & 0x8000 != 0;
    for i in 0..12 {
        let used = s.edges.cells.used[list][i] != 0;
        d.push(Branch(used));
        if used {
            let cell = s.edges.cells.cells[list][i];
            let (x, y) = (cell.x, cell.y);
            let at = (x as u32 >> 4) * 0x60 + (y as u32 >> 4);
            let there = peek(0, fd_core::level::redraw::OBJECT_MAP.wrapping_add(at)) != 0;
            d.push(Branch(there));
            if there {
                // Out the left, the right, the top, the bottom: each `bpl`/`bmi` taken leaves.
                let tests = [
                    !below(c.x.wrapping_sub(0x10), x),
                    below(c.x.wrapping_add(0x130), x),
                    !below(c.y.wrapping_sub(0x20), y),
                    below(c.y.wrapping_add(0xc0), y),
                ];
                let mut out = false;
                for t in tests {
                    d.push(Branch(t));
                    if t {
                        out = true;
                        break;
                    }
                }
                if !out {
                    let v = cell.code.wrapping_sub(0x20);
                    let under = |imm: u8| v.wrapping_sub(imm) & 0x80 != 0;
                    d.push(Branch(!under(8)));
                    if under(8) {
                        d.push(Decision::Busy);
                    } else {
                        d.push(Branch(v != 8));
                        if v == 8 {
                            d.push(Branch(a.water_cycle == 0));
                            d.push(Decision::Busy);
                        } else {
                            d.push(Branch(v != 0xa));
                            if v == 0xa {
                                d.push(Decision::Busy);
                            } else {
                                d.push(Branch(!under(0x10)));
                                if under(0x10) {
                                    return;
                                }
                                d.push(Branch(under(0x14)));
                                if under(0x14) {
                                    let k = (v - 0x10) as u32;
                                    let idx = ((k << 4) + tick as u32) & 0x3f;
                                    let word = l.program.byte(0x63ec + 4 * idx + 3);
                                    d.push(Decision::Busy);
                                    d.push(Branch(word == 0x14));
                                } else {
                                    d.push(Branch(v == 0x14));
                                    if v != 0x14 {
                                        return;
                                    }
                                    d.extend([Decision::Busy, Decision::Busy]);
                                }
                            }
                        }
                    }
                }
            }
        }
        d.push(Branch(i != 11));
    }
}

/// `char_object_cells` (`h0_2698`): `h0_26ec` on the object map's cell at the feet, then at the head
/// (the first's pick-up seen by the second): the exit (`bne`), nothing below `$20` (`bmi`), an item
/// below `$28` (`bpl`: taken, its cell redrawn, `items_left_minus_one`'s `bne`, the item sound's
/// wait), `$40` to `$5f` (`bmi`, `bpl`) a bonus by its value (`bne` down the list; `$4c` the time
/// sound and `countdown_plus_100`'s `bmi`, the rest the item sound), else nothing.
fn object_cells(l: &LevelCtx, peek: Peek, d: &mut Vec<Decision>) {
    let s = &*l.state;
    let below = |v: u8, imm: u8| v.wrapping_sub(imm) & 0x80 != 0;
    let mut cleared: Vec<u32> = Vec::new();
    let mut items = s.items_left;
    let mut countdown = (s.countdown >> 16) as u16;
    let x = s.char_x.wrapping_add(0xe);
    for row in [0x19u32, 9] {
        let y = s.char_y.wrapping_add(row);
        let at = ((x >> 4) & 0xffff) * 0x60 + (y >> 4);
        let v = if cleared.contains(&at) {
            0
        } else {
            peek(0, fd_core::level::redraw::OBJECT_MAP.wrapping_add(at))
        };
        d.push(Branch(v != 0xff));
        if v == 0xff {
            continue;
        }
        d.push(Branch(below(v, 0x20)));
        if below(v, 0x20) {
            continue;
        }
        d.push(Branch(!below(v, 0x28)));
        if below(v, 0x28) {
            cleared.push(at);
            d.push(Decision::Busy);
            d.push(Branch(items != 0));
            if items != 0 {
                // `sbcd`: only whether it reaches 0 is read again.
                items = if items == 1 {
                    0
                } else {
                    items.wrapping_sub(1).max(1)
                };
            }
            line_wait(4, d);
            continue;
        }
        d.push(Branch(below(v, 0x40)));
        if below(v, 0x40) {
            continue;
        }
        d.push(Branch(!below(v, 0x60)));
        if !below(v, 0x60) {
            continue;
        }
        cleared.push(at);
        d.push(Decision::Busy);
        for k in 0x40..=0x50u8 {
            d.push(Branch(v != k));
            if v != k {
                continue;
            }
            line_wait(4, d);
            if k == 0x4c {
                let under = minus(countdown, 0x899);
                d.push(Branch(under));
                // `abcd` of 1 into its high byte, or `$999`.
                countdown = if under {
                    countdown.wrapping_add(0x100)
                } else {
                    0x999
                };
            }
            break;
        }
    }
}

/// `char_hit_check`'s decisions after its read of CLXDAT, `v`: `beq` with no contact bit (1, 2),
/// else `contact`, the path with contact ([`hit_check`]).
pub fn after_clxdat(v: u16, contact: &[Decision]) -> Vec<Decision> {
    let none = v & 6 == 0;
    let mut d = vec![Branch(none)];
    if !none {
        d.extend_from_slice(contact);
    }
    d
}

/// `char_hit_check` (`h0+$3b2a`) past its contact test, as the level is on entry: `char_dying`
/// (`beq` on), `char_falling` (`bne` to the creatures); falling, each slot (`beq` past it) whose flag
/// is set tested by `h0_3dd8` (four `bpl`/`bmi` outs, then `beq` on `d7`) and its kind 7 (`beq` on),
/// the first stomped (`h0_3adc`'s wait) ending it; then `char_unhurt_t` (`beq`), the third hit
/// (`bpl`), the cheat (`beq`), `lives_minus_one`'s `bne` and the hurt sound's wait.
pub fn hit_check(l: &LevelCtx) -> Vec<Decision> {
    let s = &*l.state;
    let m = &s.motion;
    let mut d = Vec::new();
    d.push(Branch(m.dying == 0));
    if m.dying != 0 {
        return d;
    }
    d.push(Branch(m.falling != 0));
    if m.falling != 0 {
        let px = s.char_x.wrapping_add(0x10) as u16;
        let py = s.char_y.wrapping_add(0x24) as u16;
        let c = &s.edges.creatures;
        for i in 0..5 {
            let on = c.active[i] != 0;
            d.push(Branch(!on));
            if !on {
                continue;
            }
            let v = c.slots[i];
            // `cmp.w d0, d4; bpl`, `cmp.w d0, d6; bmi`, `cmp.w d1, d5; bpl`, `cmp.w d1, d7; bmi`.
            let outs = [
                !minus(v.x, px),
                minus(v.x.wrapping_add(0x30), px),
                !minus(v.y, py),
                minus(v.y.wrapping_add(0x20), py),
            ];
            let mut hit = true;
            for out in outs {
                d.push(Branch(out));
                if out {
                    hit = false;
                    break;
                }
            }
            d.push(Branch(!hit));
            if !hit {
                continue;
            }
            d.push(Branch(v.kind == 7));
            if v.kind != 7 {
                line_wait(4, &mut d);
                return d;
            }
        }
    }
    d.push(Branch(m.unhurt_t == 0));
    if m.unhurt_t != 0 {
        return d;
    }
    d.push(Branch(!minus(s.hits_taken.wrapping_add(1), 3)));
    if minus(s.hits_taken.wrapping_add(1), 3) {
        return d;
    }
    d.push(Branch(s.keys.cheat == 0));
    if s.keys.cheat != 0 {
        return d;
    }
    d.push(Branch(s.lives != 0));
    line_wait(3, &mut d);
    d
}

/// `char_update` (`h0+$3e0c`): four sub-steps (`dbra`), each `char_probe`, `char_probe_flags` and
/// `char_step`. The level is moved on by the hand engine's own sub-steps ([`Character`]) on a copy,
/// so each sub-step's path is taken from the state the one before left, as the original's is; the
/// probe's samples past what the plane keeps come from memory ([`far_samples`]).
fn char_update(l: &LevelCtx, peek: Peek, d: &mut Vec<Decision>) {
    let mut state = l.state.clone();
    let mut scratch = Scratch::default();
    let mut c = Character {
        state: &mut state,
        program: l.program,
        anim: l.anim,
        plane: &*l.plane,
        scratch: &mut scratch,
    };
    for n in (0..=3u16).rev() {
        probe(&c, d);
        c.probe();
        far_samples(&mut c, peek);
        probe_flags(&c, d);
        c.probe_flags();
        char_step(&mut c, d);
        d.push(Count(n));
    }
}

/// `char_probe` (`h0+$5500`): each of the nine points' `lsl.l d2, d0`, `d2` its x's pixel in the
/// plane's word.
fn probe(c: &Character, d: &mut Vec<Decision>) {
    let x0 = c.state.char_x.wrapping_sub(0x10);
    for (dx, _, _) in POINTS {
        d.push(Reg(2, x0.wrapping_add(dx) & 0xf));
    }
}

/// A probe point whose long lies past what a plane in play keeps (an `x` far past the level's) reads
/// whatever memory is there, which the plane holds as 0: its samples are taken again from memory.
/// The chips' and CIAs' range is left at 0, as a read there is not memory and `peek` would make it.
fn far_samples(c: &mut Character, peek: Peek) {
    let x0 = c.state.char_x.wrapping_sub(0x10);
    let y0 = c.state.char_y;
    for (dx, dy, bits) in POINTS {
        let (x, y) = (x0.wrapping_add(dx), y0.wrapping_add(dy));
        let off = c.plane.offset(x, y);
        if off.wrapping_add(collision::BEFORE) < collision::BEFORE + collision::KEPT {
            continue;
        }
        let long = (0..4u32).fold(0u32, |v, k| {
            let o = collision::MAP.wrapping_add(off).wrapping_add(k);
            let a = HUNK_BASES[3].wrapping_add(o) & 0xff_ffff;
            let b = if (0xa0_0000..0xc0_0000).contains(&a) || (0xd8_0000..0xe0_0000).contains(&a) {
                0
            } else {
                peek(3, o)
            };
            v << 8 | b as u32
        });
        let w = (long << (x & 15)) >> 16;
        for &(bit, i) in bits {
            c.state.motion.samples[i] = if w >> bit & 1 != 0 { 0xff } else { 0 };
        }
    }
}

/// `char_probe_flags` (`h0_4f90`): a `beq` past each sample it folds, in its order.
fn probe_flags(c: &Character, d: &mut Vec<Decision>) {
    let s = c.state.motion.samples;
    for i in [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 9, 13, 14, 12] {
        d.push(Branch(s[i] == 0));
    }
}

/// `cmpi.b #1, v; bpl`: on ground.
fn one_up(v: u8) -> bool {
    v.wrapping_sub(1) & 0x80 == 0
}

/// `cmpi.l #k, v; bpl`.
fn not_below(v: u32, k: u32) -> bool {
    v.wrapping_sub(k) & 0x8000_0000 == 0
}

/// `char_step` (`h0+$3e30`): the dying fall, the rise, the fall (`beq` past each), else up, left
/// and right in turn (`beq` past each), and with none held `h0_3ea8`. Each part's path is taken
/// before the hand engine's part moves the copy.
fn char_step(c: &mut Character, d: &mut Vec<Decision>) {
    let m = c.state.motion;
    d.push(Branch(m.dying == 0));
    if m.dying != 0 {
        dying_fall(c, d);
        return c.step();
    }
    d.push(Branch(m.rising == 0));
    if m.rising != 0 {
        rise(c, d);
        return c.step();
    }
    d.push(Branch(m.falling == 0));
    if m.falling != 0 {
        land_or_fall(c, d);
        return c.land_or_fall();
    }
    let j = c.state.joy;
    d.push(Branch(j.up == 0));
    if j.up != 0 {
        up_pressed(c, d);
        c.up_pressed();
    }
    d.push(Branch(j.left == 0));
    if j.left != 0 {
        pressed(c, true, d);
        c.left_pressed();
    }
    d.push(Branch(j.right == 0));
    if j.right != 0 {
        pressed(c, false, d);
        c.right_pressed();
    }
    // `tst.w; bne` on left, right, up in turn.
    d.push(Branch(j.left != 0));
    if j.left != 0 {
        return;
    }
    d.push(Branch(j.right != 0));
    if j.right != 0 {
        return;
    }
    d.push(Branch(j.up != 0));
    if j.up != 0 {
        return;
    }
    // `h0_3ea8`: `h0_410c`, then the slide's tests.
    land_or_fall(c, d);
    c.land_or_fall();
    idle_landed(c, d);
    c.idle_landed();
}

/// `char_step` at `h0+$4b64`: `bmi` while the count's frame is below 8, then the facing (`bmi`) for
/// the frame; from 8, `bne` on ground.
fn dying_fall(c: &Character, d: &mut Vec<Decision>) {
    let t = c.state.motion.dying_t.wrapping_add(1);
    let early = minus(t >> 4, 8);
    d.push(Branch(early));
    if early {
        d.push(Branch(c.state.anim.facing & 0x80 != 0));
    } else {
        d.push(Branch(c.state.anim.probe_1 != 0));
    }
}

/// `char_step` at `h0+$41b8`, the rise: `bne` once taken off; either way the head's two probes
/// (`beq` on the first clear, `bne` on the second set). Before take-off, unblocked, `beq` on
/// `char_jump_41b6`; after, unblocked, `h0_4274`, `h0_4696`, `h0_4d32` and `beq` when the step
/// count wraps.
fn rise(c: &Character, d: &mut Vec<Decision>) {
    let m = c.state.motion;
    d.push(Branch(m.jumped != 0));
    d.push(Branch(m.probe_5 == 0));
    if m.probe_5 != 0 {
        d.push(Branch(m.probe_6 != 0));
    }
    let blocked = m.probe_5 != 0 && m.probe_6 != 0;
    if m.jumped == 0 {
        if !blocked {
            d.push(Branch(m.jump_41b6 == 0));
        }
        return;
    }
    if blocked {
        return;
    }
    let ended = rise_step(c, d);
    let facing = move_x(c, d);
    d.push(Branch(facing & 0x80 != 0));
    let t = if ended { 0xff } else { m.jump_t };
    d.push(Branch(t.wrapping_add(1) == 0));
}

/// `h0_4274`: `bne` on `char_jump_41b4`, else `bne` on `char_jump_3b28`, for the table; `beq` on its
/// `$ff`, `beq` past bit 7 clear, `beq` with bit 7 and up not held. Whether the rise ended.
fn rise_step(c: &Character, d: &mut Vec<Decision>) -> bool {
    let m = c.state.motion;
    d.push(Branch(m.jump_41b4 != 0));
    let table = if m.jump_41b4 != 0 {
        0x43ba
    } else {
        d.push(Branch(m.jump_3b28 != 0));
        if m.jump_3b28 != 0 { 0x42ec } else { 0x431e }
    };
    let v = c.table(table + m.jump_t as u32);
    d.push(Branch(v == 0xff));
    if v == 0xff {
        return true;
    }
    d.push(Branch(v & 0x80 == 0));
    if v & 0x80 == 0 {
        return false;
    }
    let up = c.state.joy.up != 0;
    d.push(Branch(!up));
    !up
}

/// `h0_4696`: `bne` on right (then `bne` on the right wall), else `bne` on left (then the left
/// wall). The facing it leaves.
fn move_x(c: &Character, d: &mut Vec<Decision>) -> u8 {
    let (j, m) = (c.state.joy, c.state.motion);
    let facing = c.state.anim.facing;
    d.push(Branch(j.right != 0));
    if j.right != 0 {
        d.push(Branch(m.probe_4 != 0));
        return if m.probe_4 == 0 { 1 } else { facing };
    }
    d.push(Branch(j.left != 0));
    if j.left != 0 {
        d.push(Branch(m.probe_3 != 0));
        return if m.probe_3 == 0 { 0xff } else { facing };
    }
    facing
}

/// `h0_410c`: `bpl` on ground; in the air `h0_416c` (`beq` on the fall table's `$ff`), `h0_4d54`
/// (`bmi` on the facing) and `h0_4696`.
fn land_or_fall(c: &Character, d: &mut Vec<Decision>) {
    let on = one_up(c.state.anim.probe_1);
    d.push(Branch(on));
    if on {
        return;
    }
    let v = c.table(0x44d4 + c.state.motion.jump_t as u32);
    d.push(Branch(v == 0xff));
    d.push(Branch(c.state.anim.facing & 0x80 != 0));
    move_x(c, d);
}

/// `h0_4c6a`, the standing frame from the probes and the facing (`beq`, `bmi`, `bne` in its order).
fn frame_from_probes(c: &Character, d: &mut Vec<Decision>) {
    let (p1, p2) = (c.state.anim.probe_1, c.state.anim.probe_2);
    let left = c.state.anim.facing & 0x80 != 0;
    d.push(Branch(p1 == 1));
    d.push(Branch(left));
    if p1 == 1 {
        return;
    }
    // Facing right the pair of tests for bit 1 and 2, then bit 0 and 3; left, the other order.
    let pairs = if left {
        [(1u8, 3u8), (2, 2)]
    } else {
        [(2, 2), (1, 3)]
    };
    for (bit, p) in pairs {
        d.push(Branch(p2 & bit != 0));
        if p2 & bit != 0 {
            return;
        }
        d.push(Branch(p1 != p));
        if p1 == p {
            return;
        }
    }
}

/// `h0_4b0e`, up held: `bpl` on ground, then `beq` past each head probe, `h0_4c6a` on one set.
fn up_pressed(c: &Character, d: &mut Vec<Decision>) {
    let on = one_up(c.state.anim.probe_1);
    d.push(Branch(on));
    if !on {
        return;
    }
    let m = c.state.motion;
    for p in [m.probe_6, m.probe_5] {
        d.push(Branch(p == 0));
        if p != 0 {
            return frame_from_probes(c, d);
        }
    }
}

/// `h0_4718` (left) or `h0_48be` (right): `beq` past the wall; a wall, `bmi` off ground (`h0+$4836`)
/// or `h0_4c6a`. No wall: the walk step's `bmi`, `bmi` off ground, then the slope's tests and the run
/// step (`h0_47e2`, `h0_497c`).
fn pressed(c: &Character, left: bool, d: &mut Vec<Decision>) {
    let (m, a) = (c.state.motion, c.state.anim);
    let (p1, p2) = (a.probe_1, a.probe_2);
    let wall = if left { m.probe_3 } else { m.probe_4 };
    d.push(Branch(wall == 0));
    if wall != 0 {
        let off = !one_up(p1);
        d.push(Branch(off));
        if off {
            off_ground(c, left, d);
        } else {
            frame_from_probes(c, d);
        }
        return;
    }
    // `char_step_4be6` / `h0_4c28`: `cmpi.b #$28; bmi` on the walk's count.
    d.push(Branch(
        a.walk_t.wrapping_add(1).wrapping_sub(0x28) & 0x80 != 0,
    ));
    let off = !one_up(p1);
    d.push(Branch(off));
    if off {
        return off_ground(c, left, d);
    }
    // Left tests bit 0 first and compares 2; right bit 1 and 3.
    let (first, second, steep) = if left { (1, 2, 2) } else { (2, 1, 3) };
    d.push(Branch(p2 & first == 0));
    if p2 & first != 0 {
        d.push(Branch(p1 == steep));
        if p1 != steep {
            d.push(Branch(p2 != 3));
        }
    } else {
        d.push(Branch(p2 & second == 0));
        if p2 & second != 0 {
            d.push(Branch(p1 == 1));
        }
    }
    run_step(c, left, d);
}

/// `h0+$4836` / `h0+$487a`: `bne` on the low ground sample, then the run step.
fn off_ground(c: &Character, left: bool, d: &mut Vec<Decision>) {
    let low = c.state.motion.samples[15] != 0;
    d.push(Branch(low));
    if low {
        run_step(c, left, d);
    }
}

/// `h0_47e2` / `h0_497c`: the count stepped, `bne` past `$f0`, `beq` on the table's `$ff` (the
/// count back `$1e` and again).
fn run_step(c: &Character, left: bool, d: &mut Vec<Decision>) {
    let m = c.state.motion;
    let mut t = if left { m.run_left } else { m.run_right };
    // The guard of the hand engine's loop: the original would hang first.
    for _ in 0..1 << 20 {
        t = t.wrapping_add(1);
        d.push(Branch(t != 0xf0));
        let v = c.table(0x49d0u32.wrapping_add(t));
        d.push(Branch(v == 0xff));
        if v != 0xff {
            return;
        }
        t = t.wrapping_sub(0x1e);
    }
}

/// `h0_3ea8` after its `h0_410c`: `bne` on `char_probe_2`, `bne` on `char_probe_1` not 1, then `bpl`
/// on each run of `$30`, to the slide; else `h0_4c6a`.
fn idle_landed(c: &Character, d: &mut Vec<Decision>) {
    let (m, a) = (c.state.motion, c.state.anim);
    d.push(Branch(a.probe_2 != 0));
    if a.probe_2 == 0 {
        d.push(Branch(a.probe_1 != 1));
        if a.probe_1 == 1 {
            for left in [true, false] {
                let run = if left { m.run_left } else { m.run_right };
                d.push(Branch(not_below(run, 0x30)));
                if not_below(run, 0x30) {
                    return slide(c, left, d);
                }
            }
        }
    }
    frame_from_probes(c, d);
}

/// `h0+$3ef6` / `h0+$3fce`, a slide step: `beq` past the wall, `bne` past the sound (its wait) unless
/// the step is 3, `bpl` on a run of `$a0` for the table, `bne` past its `$ff`.
fn slide(c: &Character, left: bool, d: &mut Vec<Decision>) {
    let m = c.state.motion;
    let wall = if left { m.probe_3 } else { m.probe_4 };
    d.push(Branch(wall == 0));
    if wall != 0 {
        return;
    }
    let t = m.slide_t;
    d.push(Branch(t != 3));
    if t == 3 {
        line_wait(4, d);
    }
    let run = if left { m.run_left } else { m.run_right };
    let fast = not_below(run, 0xa0);
    d.push(Branch(fast));
    // Sliding right reads `h0+$40ac` whatever it chose.
    let table = if left && fast { 0x40cb } else { 0x40ac };
    d.push(Branch(c.table(table + t as u32) != 0xff));
}
