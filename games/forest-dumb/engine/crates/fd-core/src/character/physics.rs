//! The character's movement (plans/forest-dumb-level-1 T08): running, the slide, jumping, falling,
//! the dying fall, and the probe of the collision plane they all turn on. Ported from `char_update`
//! (`h0+$3e0c`) and what it calls: `char_probe`, `char_probe_flags`, `char_step` and its helpers
//! `h0_3ea8` to `h0_4b0e`. docs/09-play-measured.md ("The character's movement (T08)") lists what
//! each reads and why none can be stopped partway; `fd-conform/tests/character_routines.rs` holds
//! each against the original on the routine rig.
//!
//! The original has no velocities. A pass runs four sub-steps; each samples the plane at sixteen
//! points around the character (`char_probe`), folds them into six flags (`char_probe_flags`), and
//! moves the character a pixel or a table step by those flags and the joystick (`char_step`).
//! Every step size comes from a byte table in hunk 0, read through [`Program`] as loaded.

use crate::State;
use crate::character::AnimTables;
use crate::character::anim::{WALK_LEFT, WALK_RIGHT};
use crate::level::collision::{self, CollisionPlane};
use crate::program::{Program, Scratch, h};

/// The movement state, at the original's widths. Readings of what each is for are from the
/// listing; the rig proves the routines, not the names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Motion {
    /// `char_jump_3b28` (`h0+$3b28`), word: when set (and `char_jump_41b4` clear) the rise reads
    /// `h0+$42ec`. Cleared by every landing and fall.
    pub jump_3b28: u16,
    /// `char_jump_41b4` (`h0+$41b4`), word: the rise reads `h0+$43ba`.
    pub jump_41b4: u16,
    /// `char_jump_41b6` (`h0+$41b6`), word: a take-off with it set sets `char_jump_41b4`.
    pub jump_41b6: u16,
    /// `char_dying_t` (`h0+$4b60`), word: the dying fall's counter.
    pub dying_t: u16,
    /// `char_dying` (`h0+$4b62`), word: the dying fall runs instead of everything else.
    pub dying: u16,
    /// `char_rising` (`h0+$4ecf`): the jump's rise runs.
    pub rising: u8,
    /// `char_falling` (`h0+$4ed0`): the fall runs.
    pub falling: u8,
    /// `char_probe_3`..`char_probe_6` (`h0+$4ed3`..`$4ed6`): wall left, wall right, and two
    /// above, 0 or 1 (`char_probe_flags`).
    pub probe_3: u8,
    pub probe_4: u8,
    pub probe_5: u8,
    pub probe_6: u8,
    /// `char_slide_t` (`h0+$4108`): the slide's step.
    pub slide_t: u8,
    /// `char_jumped` (`h0+$4109`): the rise has left the ground.
    pub jumped: u8,
    /// `char_jump_t` (`h0+$410a`): the rise's or fall's step.
    pub jump_t: u8,
    /// `char_run_left`, `char_run_right` (`h0+$4710`, `$4714`): how long the run has gone on.
    pub run_left: u32,
    pub run_right: u32,
    /// `char_samples` (`h0+$54f0`..`$54ff`): the probe's points, `$ff` solid or 0.
    pub samples: [u8; 16],
    /// `char_update_t` (`h0+$3e2c`): `char_update`'s sub-step count, 0 after a pass.
    pub update_t: u32,
    /// `char_unhurt_t` (`h0+$9b04`): set to `$9b` when the dying fall lands; contact hurts only once
    /// it wraps to 0 (T59). `char_hit_check` (T24) counts it.
    pub unhurt_t: u8,
}

/// Hunk 0 tables the movement reads.
const PROBE_TABLE: u32 = 0x4f88;
const FALL_TABLE: u32 = 0x44d4;
const RISE_TABLE: u32 = 0x431e;
const RISE_3B28: u32 = 0x42ec;
const RISE_41B4: u32 = 0x43ba;
const RUN_TABLE: u32 = 0x49d0;
const SLIDE_SLOW: u32 = 0x40ac;
const SLIDE_FAST: u32 = 0x40cb;

/// `char_probe`'s points: (x from `char_x - $10`, y from `char_y`, (window bit, sample) pairs), in
/// the listing's order.
const POINTS: [(u32, u32, &[(u32, usize)]); 9] = [
    (0x18, 0x28, &[(15, 0), (13, 1), (11, 2)]),
    (0x18, 0x2c, &[(13, 15)]),
    (0x18, 0x27, &[(14, 3), (12, 4)]),
    (0x14, 0x0f, &[(15, 5)]),
    (0x14, 0x16, &[(15, 6)]),
    (0x24, 0x0f, &[(15, 7)]),
    (0x24, 0x16, &[(15, 8)]),
    (0x18, 0xffff_ffff, &[(15, 9), (13, 10), (11, 11)]),
    (0x18, 0x08, &[(15, 12), (13, 13), (11, 14)]),
];

/// A loop of the original that ends only on a table byte other than `$ff` stops here in the
/// engine. Play never gets near it; on the rig the original would have hung first.
const LOOP_GUARD: u32 = 1 << 20;

/// `cmpi.b #1, v; bpl`: the byte difference is not negative, so `v` is 1 to `$80`.
fn one_up(v: u8) -> bool {
    v.wrapping_sub(1) & 0x80 == 0
}

/// `cmpi.l #k, v; bpl`: the long difference's sign, overflow ignored as `bpl` ignores it.
fn not_below(v: u32, k: u32) -> bool {
    v.wrapping_sub(k) & 0x8000_0000 == 0
}

/// The character's routines over the level state.
pub struct Character<'a> {
    pub state: &'a mut State,
    pub program: &'a Program,
    pub anim: &'a AnimTables,
    pub plane: &'a CollisionPlane,
    pub scratch: &'a mut Scratch,
}

impl Character<'_> {
    /// A table byte at `h0+off`. Past a table's end a read can reach the game's own variables
    /// (`h0+$40cb + $3d` is `char_slide_t`); the original reads their value now, so a byte of a
    /// variable the state models comes from the state, the rest from the program.
    fn table(&self, off: u32) -> u8 {
        for v in crate::LAYOUT.iter().filter(|v| v.hunk == 0) {
            let k = off.wrapping_sub(v.offset);
            let n = v.width.bytes() as u32;
            if k < n {
                if let Some(x) = self.state.get(v.name) {
                    return (x >> (8 * (n - 1 - k))) as u8;
                }
            }
        }
        self.program.byte(off)
    }

    fn m(&mut self) -> &mut Motion {
        &mut self.state.motion
    }

    /// `char_update`: four sub-steps, each probe, flags, step. The count is kept in memory.
    pub fn update(&mut self) {
        let mut d7: u32 = 3;
        loop {
            self.m().update_t = d7;
            self.probe();
            self.probe_flags();
            self.step();
            d7 = self.m().update_t;
            let w = (d7 as u16).wrapping_sub(1);
            d7 = d7 & 0xffff_0000 | w as u32;
            if w == 0xffff {
                break;
            }
        }
        self.scratch.d[7] = d7;
    }

    /// `char_probe`: the sixteen points of the plane around the character.
    pub fn probe(&mut self) {
        let d3 = self.state.char_x.wrapping_sub(0x10);
        let d4 = self.state.char_y;
        let (mut x, mut y, mut w) = (0, 0, 0);
        for (dx, dy, bits) in POINTS {
            (x, y) = (d3.wrapping_add(dx), d4.wrapping_add(dy));
            w = self.plane.window(x, y);
            for &(bit, i) in bits {
                self.state.motion.samples[i] = if w >> bit & 1 != 0 { 0xff } else { 0 };
            }
        }
        let r = &mut *self.scratch;
        r.d[0] = w as u32;
        r.d[1] = self.plane.offset(x, y);
        r.d[2] = x & 15;
        r.d[3] = d3;
        r.d[4] = d4;
        r.a[0] = h(3, collision::MAP);
        r.a[5] = h(0, collision::ROWS);
    }

    /// `char_probe_flags`: the samples to `char_probe_1`..`char_probe_6`.
    pub fn probe_flags(&mut self) {
        let s = self.state.motion.samples;
        let on = |i: usize| (s[i] != 0) as u8;
        let index = (on(0) << 2 | on(1) << 1 | on(2)) as u32;
        self.state.anim.probe_1 = self.table(PROBE_TABLE + index);
        self.state.anim.probe_2 = on(3) | on(4) << 1;
        let m = self.m();
        m.probe_3 = on(5) | on(6);
        m.probe_4 = on(7) | on(8);
        m.probe_5 = on(10) | on(11) | on(9);
        m.probe_6 = on(13) | on(14) | on(12);
        let p6 = m.probe_6;
        self.scratch.d[0] = p6 as u32;
        self.scratch.a[0] = h(0, PROBE_TABLE);
    }

    /// `char_step`: one sub-step's movement.
    pub fn step(&mut self) {
        let m = self.state.motion;
        if m.dying != 0 {
            return self.dying_fall();
        }
        if m.rising != 0 {
            return self.rise();
        }
        if m.falling != 0 {
            return self.land_or_fall();
        }
        if self.state.joy.up != 0 {
            self.up_pressed();
        }
        if self.state.joy.left != 0 {
            self.left_pressed();
        }
        if self.state.joy.right != 0 {
            self.right_pressed();
        }
        let j = self.state.joy;
        if j.left == 0 && j.right == 0 && j.up == 0 {
            self.idle();
        }
    }

    /// `char_step` at `h0+$4b64`: the dying fall's frames, then a pixel a sub-step down to ground.
    fn dying_fall(&mut self) {
        self.scratch.d[0] = 0;
        let t = self.m().dying_t.wrapping_add(1);
        self.m().dying_t = t;
        let d0 = (t >> 4) as u32;
        self.scratch.d[0] = d0;
        if d0 >= 8 {
            if self.state.anim.probe_1 != 0 {
                let m = self.m();
                m.unhurt_t = 0x9b;
                m.dying = 0;
                m.dying_t = 0;
                m.rising = 0;
            } else {
                self.m().dying_t = t.wrapping_sub(1);
                self.state.char_y = self.state.char_y.wrapping_add(1);
            }
            return;
        }
        let d0 = d0 << 2;
        let table = if self.state.anim.facing & 0x80 != 0 {
            0x4f64
        } else {
            0x4f10
        };
        self.scratch.d[0] = d0;
        self.scratch.a[0] = h(0, table) + d0;
        self.state.anim.ptr = self.anim.long(table + d0);
    }

    /// `char_step` at `h0+$41b8`: the rise. A head against something above stops it before take-off
    /// and ends it after.
    fn rise(&mut self) {
        let m = self.state.motion;
        let blocked = m.probe_5 != 0 && m.probe_6 != 0;
        if m.jumped == 0 {
            let m = self.m();
            if blocked {
                m.rising = 0;
                return;
            }
            m.jumped = 0xff;
            m.jump_t = 0;
            m.run_left = 0;
            m.run_right = 0;
            m.slide_t = 0;
            if m.jump_41b6 != 0 {
                m.jump_41b4 = 1;
            }
            return;
        }
        if !blocked {
            self.rise_step();
            self.move_x();
            self.state.anim.frame_4d32(self.anim);
            let t = self.m().jump_t.wrapping_add(1);
            self.m().jump_t = t;
            if t != 0 {
                return;
            }
        }
        let m = self.m();
        m.falling = 0xff;
        m.rising = 0;
        m.jumped = 0;
        m.jump_t = 0;
        m.jump_41b4 = 0;
        m.jump_3b28 = 0;
        m.jump_41b6 = 0;
    }

    /// `h0_410c`: on ground, the jump and fall end; in the air, a fall step.
    pub fn land_or_fall(&mut self) {
        let m = self.m();
        m.jump_3b28 = 0;
        m.jump_41b4 = 0;
        m.jump_41b6 = 0;
        if one_up(self.state.anim.probe_1) {
            let m = self.m();
            m.falling = 0;
            m.rising = 0;
            m.jumped = 0;
            m.jump_t = 0;
            return;
        }
        self.fall_step();
        self.state.anim.frame_4d54(self.anim);
        let m = self.m();
        m.jump_t = m.jump_t.wrapping_add(1);
        self.move_x();
    }

    /// `h0_416c`: one fall step down by `h0+$44d4`; at its `$ff` end, 2 a step and the step held.
    pub fn fall_step(&mut self) {
        let m = self.m();
        m.rising = 0;
        m.falling = 1;
        let v = self.table(FALL_TABLE + self.state.motion.jump_t as u32);
        self.scratch.a[0] = h(0, FALL_TABLE);
        self.scratch.d[0] = v as u32;
        if v == 0xff {
            let m = self.m();
            m.jump_t = m.jump_t.wrapping_sub(1);
            self.state.char_y = self.state.char_y.wrapping_add(2);
        } else {
            self.state.char_y = self.state.char_y.wrapping_add(v as u32);
        }
    }

    /// `h0_4274`: one rise step up. An entry with bit 7 set needs up held; `$ff` ends the rise.
    pub fn rise_step(&mut self) {
        let m = self.state.motion;
        let table = if m.jump_41b4 != 0 {
            RISE_41B4
        } else if m.jump_3b28 != 0 {
            RISE_3B28
        } else {
            RISE_TABLE
        };
        let mut v = self.table(table + m.jump_t as u32);
        self.scratch.a[0] = h(0, table);
        self.scratch.d[0] = v as u32;
        if v == 0xff || (v & 0x80 != 0 && self.state.joy.up == 0) {
            self.m().jump_t = 0xff;
            return;
        }
        if v & 0x80 != 0 {
            v &= 0x7f;
            self.scratch.d[0] = v as u32;
        }
        self.state.char_y = self.state.char_y.wrapping_sub(v as u32);
    }

    /// `h0_4696`: a pixel left or right in the air, unless a wall is there; right wins. Either
    /// way the run and the slide end.
    pub fn move_x(&mut self) {
        let j = self.state.joy;
        if j.right != 0 {
            if self.state.motion.probe_4 == 0 {
                self.state.anim.facing = 1;
                self.state.char_x = self.state.char_x.wrapping_add(1);
            }
        } else if j.left != 0 {
            if self.state.motion.probe_3 == 0 {
                self.state.anim.facing = 0xff;
                self.state.char_x = self.state.char_x.wrapping_sub(1);
            }
        } else {
            return;
        }
        let m = self.m();
        m.slide_t = 0;
        m.run_left = 0;
        m.run_right = 0;
    }

    /// `h0_47e2`: the left run's next step from `h0+$49d0`, moved and returned; at a `$ff` the
    /// counter steps back `$1e` and tries again.
    pub fn run_left_step(&mut self) -> u32 {
        let v = self.run_step(true);
        self.state.char_x = self.state.char_x.wrapping_sub(v);
        v
    }

    /// `h0_497c`: the right run's, as [`Character::run_left_step`].
    pub fn run_right_step(&mut self) -> u32 {
        let v = self.run_step(false);
        self.state.char_x = self.state.char_x.wrapping_add(v);
        v
    }

    fn run_step(&mut self, left: bool) -> u32 {
        let mut v = 0;
        for _ in 0..LOOP_GUARD {
            let m = self.m();
            let t = if left {
                m.run_right = 0;
                m.run_left = m.run_left.wrapping_add(1);
                m.run_left
            } else {
                m.run_left = 0;
                m.run_right = m.run_right.wrapping_add(1);
                m.run_right
            };
            v = self.table(RUN_TABLE.wrapping_add(t));
            if v != 0xff {
                break;
            }
            let m = self.m();
            let c = if left {
                &mut m.run_left
            } else {
                &mut m.run_right
            };
            *c = c.wrapping_sub(0x1e);
        }
        self.scratch.a[0] = h(0, RUN_TABLE);
        self.scratch.d[0] = v as u32;
        v as u32
    }

    /// `h0_4b0e`: up pressed. On ground and clear above, the jump starts; off ground, the fall.
    pub fn up_pressed(&mut self) {
        if !one_up(self.state.anim.probe_1) {
            self.m().falling = 0xff;
            return;
        }
        let m = self.state.motion;
        if m.probe_6 != 0 || m.probe_5 != 0 {
            self.state.anim.frame_from_probes(self.anim);
            return;
        }
        let m = self.m();
        m.slide_t = 0;
        m.run_right = 0;
        m.run_left = 0;
        m.rising = 0xff;
    }

    /// `h0_4718`: left pressed. A walk step and a run step, following a slope up or down.
    pub fn left_pressed(&mut self) {
        self.m().slide_t = 0;
        if self.state.motion.probe_3 != 0 {
            if !one_up(self.state.anim.probe_1) {
                return self.off_ground(true);
            }
            return self.state.anim.frame_from_probes(self.anim);
        }
        self.walk(true);
        let (p1, p2) = (self.state.anim.probe_1, self.state.anim.probe_2);
        if !one_up(p1) {
            return self.off_ground(true);
        }
        if p2 & 1 != 0 {
            if p1 == 2 || p2 != 3 {
                let d = self.run_left_step();
                self.state.char_y = self.state.char_y.wrapping_sub(d);
            } else {
                self.run_left_step();
                self.state.char_y = self.state.char_y.wrapping_sub(1);
            }
        } else if p2 & 2 == 0 {
            self.run_left_step();
        } else if p1 == 1 {
            self.run_left_step();
            self.state.char_y = self.state.char_y.wrapping_sub(1);
        } else {
            let d = self.run_left_step();
            self.state.char_y = self.state.char_y.wrapping_add(d);
        }
    }

    /// `h0_48be`: right pressed, as [`Character::left_pressed`] with the slope bits swapped.
    pub fn right_pressed(&mut self) {
        self.m().slide_t = 0;
        if self.state.motion.probe_4 != 0 {
            if !one_up(self.state.anim.probe_1) {
                return self.off_ground(false);
            }
            return self.state.anim.frame_from_probes(self.anim);
        }
        self.walk(false);
        let (p1, p2) = (self.state.anim.probe_1, self.state.anim.probe_2);
        if !one_up(p1) {
            return self.off_ground(false);
        }
        if p2 & 2 != 0 {
            if p1 == 3 || p2 != 3 {
                let d = self.run_right_step();
                self.state.char_y = self.state.char_y.wrapping_sub(d);
            } else {
                self.run_right_step();
                self.state.char_y = self.state.char_y.wrapping_sub(1);
            }
        } else if p2 & 1 == 0 {
            self.run_right_step();
        } else if p1 == 1 {
            self.run_right_step();
            self.state.char_y = self.state.char_y.wrapping_sub(1);
        } else {
            let d = self.run_right_step();
            self.state.char_y = self.state.char_y.wrapping_add(d);
        }
    }

    /// `h0+$4836` and `h0+$487a`: no ground under the walk. With the low ground sample solid the
    /// run steps on down; without it the fall starts.
    fn off_ground(&mut self, left: bool) {
        if self.state.motion.samples[15] != 0 {
            let d = if left {
                self.run_left_step()
            } else {
                self.run_right_step()
            };
            self.state.char_y = self.state.char_y.wrapping_add(d);
            return;
        }
        let m = self.m();
        m.falling = 0xff;
        m.rising = 0;
        m.jumped = 0;
        m.jump_t = 0;
        m.slide_t = 0;
        m.run_left = 0;
        m.run_right = 0;
    }

    /// T51's walk step, with the registers it leaves.
    fn walk(&mut self, left: bool) {
        let table = if left {
            self.state.anim.walk_left(self.anim);
            WALK_LEFT
        } else {
            self.state.anim.walk_right(self.anim);
            WALK_RIGHT
        };
        let d0 = 4 * (self.state.anim.walk_t >> 3) as u32;
        self.scratch.d[0] = d0;
        self.scratch.a[0] = h(0, table).wrapping_add(d0);
    }

    /// `h0_3ea8`: nothing pressed. Land or fall; on flat ground after a run of `$30` steps or more,
    /// the slide; else the run ends.
    pub fn idle(&mut self) {
        self.land_or_fall();
        let (p1, p2) = (self.state.anim.probe_1, self.state.anim.probe_2);
        let m = self.state.motion;
        if p2 == 0 && p1 == 1 {
            if not_below(m.run_left, 0x30) {
                return self.slide(true);
            }
            if not_below(m.run_right, 0x30) {
                return self.slide(false);
            }
        }
        let m = self.m();
        m.slide_t = 0;
        m.run_left = 0;
        m.run_right = 0;
        self.state.anim.frame_from_probes(self.anim);
    }

    /// `h0+$3ef6` and `h0+$3fce`: one slide step from `h0+$40ac` (`h0+$40cb` after a run of `$a0`),
    /// starting a sound on its fourth. Sliding right always reads `h0+$40ac`: the original loads
    /// `$40cb` and then `$40ac` over it.
    fn slide(&mut self, left: bool) {
        let m = self.state.motion;
        if (if left { m.probe_3 } else { m.probe_4 }) != 0 {
            let m = self.m();
            m.slide_t = 0;
            m.run_left = 0;
            m.run_right = 0;
            return;
        }
        let t = m.slide_t as u32;
        self.scratch.d[0] = t;
        if t == 3 {
            self.slide_sound();
        }
        let run = if left { m.run_left } else { m.run_right };
        let table = if left && not_below(run, 0xa0) {
            SLIDE_FAST
        } else {
            SLIDE_SLOW
        };
        let v = self.table(table + t);
        self.scratch.a[0] = h(0, table);
        self.scratch.d[0] = v as u32;
        if v == 0xff {
            let m = self.m();
            m.run_left = 0;
            m.run_right = 0;
            m.slide_t = 0;
            return;
        }
        let m = self.m();
        m.slide_t = m.slide_t.wrapping_add(1);
        let (x, frame) = if left {
            (self.state.char_x.wrapping_sub(v as u32), 0x4f54)
        } else {
            (self.state.char_x.wrapping_add(v as u32), 0x4f00)
        };
        self.state.char_x = x;
        self.state.anim.ptr = self.anim.long(frame);
    }

    /// `h0+$3f2a`: the slide's sound on audio channel 2, after a wait of four beam lines. Sound is
    /// out of scope (DESIGN §8); the writes are kept in [`Scratch`] for the rig.
    fn slide_sound(&mut self) {
        let r = &mut *self.scratch;
        r.d[7] = r.d[0];
        r.hw.push((0x96, 0x0004));
        r.d[0] = 3;
        loop {
            let b = r.vhposr();
            r.set_b(1, b);
            for _ in 0..LOOP_GUARD {
                if r.vhposr() != b {
                    break;
                }
            }
            let w = (r.d[0] as u16).wrapping_sub(1);
            r.set_w(0, w);
            if w == 0xffff {
                break;
            }
        }
        let sample = h(4, 0x1_3a62);
        r.hw.extend([
            (0xc0, (sample >> 16) as u16),
            (0xc2, sample as u16),
            (0xc4, 0x0723),
            (0xc6, 0x012a),
            (0xc8, 0x003f),
            (0x96, 0x8204),
        ]);
        r.d[0] = r.d[7];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_branch_conditions_are_the_listings_flags() {
        assert!(one_up(1) && one_up(0x80) && one_up(3));
        assert!(!one_up(0) && !one_up(0x81) && !one_up(0xff));
        assert!(not_below(0x30, 0x30) && !not_below(0x2f, 0x30));
        // The long difference's sign only: a huge counter reads as below.
        assert!(!not_below(0x8000_0030, 0x30));
        assert!(not_below(0x8000_0000, 0x30));
    }

    #[test]
    fn a_table_read_past_its_end_reads_the_variable_there_as_it_is_now() {
        // The fast slide table `h0+$40cb` read at step `$3d` reaches `char_slide_t` itself
        // (`h0+$4108`): the original moves by the step count, not by the byte the program loaded
        // there (0 here). Found on the rig (T08).
        let program = Program::hunk0(vec![0; 0x5000]);
        let anim = AnimTables::from_hunk0(program.bytes()).unwrap();
        let plane = CollisionPlane::new(&Program::hunk0(vec![0; 0x3_7f08]));
        let mut state = State::level_1();
        state.char_x = 0x1000;
        state.anim.probe_1 = 1;
        state.anim.probe_2 = 0;
        state.motion.run_left = 0xb0;
        state.motion.slide_t = 0x3d;
        let mut scratch = Scratch::default();
        Character {
            state: &mut state,
            program: &program,
            anim: &anim,
            plane: &plane,
            scratch: &mut scratch,
        }
        .idle();
        assert_eq!(state.char_x, 0x1000 - 0x3d);
        assert_eq!(state.motion.slide_t, 0x3e);
    }
}
