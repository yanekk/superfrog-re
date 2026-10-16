//! Game over (docs/13 "Game over"): `game_over_seq` (h0+$74d0), which `level_exit_fade` runs
//! first, and `game_over_check` (h0+$749c), after the exit fade, which leaves the game for
//! `top_loop` and the title.
//!
//! `game_over_seq` is not a screen of its own: it puts a GAME OVER object in the level the frog
//! died in and runs the level's objects, bobs and scroll for two passes over it, with the level's
//! vertical blank on (`logic_running`), then waits for fire, at most 350 frames.

use super::common::call;
use super::{Cx, Routine, Yield, at};
use crate::level::beam::Clock;
use crate::level::{LevelCtx, ObjectKind, objects, pass, scroll};

/// `frog_lives` (h0+$82e), `frog_launched` (h0+$800), `frog_spring` (h0+$85a),
/// `frog_camera_held` (h0+$81e).
const LIVES: u32 = at(0x82e);
const LAUNCHED: u32 = at(0x800);
const SPRING: u32 = at(0x85a);
const CAMERA_HELD: u32 = at(0x81e);
/// `game_over_seq`'s own two variables: the spawn gave up (h0+$75ec), and the GAME OVER bob
/// (h0+$75ee).
const GAVE_UP: u32 = at(0x75ec);
const BOB: u32 = at(0x75ee);
/// `logic_running` (h0+$9d6).
pub(crate) const LOGIC_RUNNING: u32 = at(0x9d6);
/// `camera_x`, `camera_y` (h0+$16664, h0+$16660).
const CAMERA_X: u32 = at(0x16664);
const CAMERA_Y: u32 = at(0x16660);
/// `abort_flag` (h0+$9de), `game_full_mode` (h0+$6).
const ABORT_FLAG: u32 = at(0x9de);
const GAME_FULL_MODE: u32 = at(0x6);
/// Frames `game_over_seq` waits for fire.
const FIRE_FRAMES: u16 = 0x15e;
/// `obj_run_all`'s and `build_bob_draw_list`'s return addresses in each of `game_over_seq`'s
/// passes: the original's frame can end in either (the engine runs them at once).
const IN_PASS: [[u32; 2]; 2] = [[at(0x756a), at(0x7570)], [at(0x759e), at(0x75a4)]];
/// What they write: `bob_free_list`, `obj_count` and `obj_slots`; `bob_draw_list`; the bobs.
const PASS_WRITES: [(u32, u32); 3] = [
    (at(0x1ba16), 0x54 + 2 + 0x9b0),
    (at(0x2970a), 0xa8),
    (at(0x297b2), 0x1730),
];

/// Template 1: the GAME OVER sign (h0+$2cdf8's frame), where `game_over_seq` puts it.
pub static GAME_OVER: ObjectKind = ObjectKind {
    name: "game over",
    routines: &[
        (at(0x23f3a), game_over_init),
        (at(0x23f50), game_over_update),
    ],
};

/// h0+$23f3a: the sign's frame.
fn game_over_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    objects::bob_anim(&mut c.bus, slot, at(0x2cdf8));
}

/// h0+$23f50: the bob where the slot is; gone once off the screen.
fn game_over_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    objects::bob_follow(&mut c.bus, slot);
    objects::obj_despawn_if_offscreen(c, slot, 0);
}

/// `game_over_seq` (h0+$74d0).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct GameOverSeq {
    pc: u8,
    /// The pass being run (0, 1).
    pass: u8,
    count: u16,
    /// The GAME OVER bob (h0+$75ee).
    bob: u32,
    /// Lines of the pass's first half left when the frame ended: they run after the next
    /// vertical blank (the interrupt stops the main program meanwhile).
    owed: u16,
    frog_only: bool,
}

impl GameOverSeq {
    pub(crate) fn new(frog_only: bool) -> GameOverSeq {
        GameOverSeq {
            frog_only,
            ..GameOverSeq::default()
        }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    if (cx.s.w(LIVES) as i16) >= 0 {
                        return Yield::Return;
                    }
                    cx.s.set_w(LAUNCHED, 0x1f4);
                    cx.s.set_w(SPRING, 1);
                    cx.s.set_w(CAMERA_HELD, 1);
                    cx.s.set_w(GAVE_UP, 0);
                    // The GAME OVER object (template 1), `$82`, `$80` from the camera.
                    let x = cx.s.l(CAMERA_X).wrapping_add(0x82) as u16;
                    let y = cx.s.l(CAMERA_Y).wrapping_add(0x80) as u16;
                    let mut spawned = None;
                    in_level(cx, self.frog_only, |c| {
                        let t = crate::items::template(1);
                        spawned = Some(objects::obj_spawn_at_xy_nodedup(c, t, x, y));
                    });
                    let s = spawned.expect("spawned");
                    if s.a0 == 0 {
                        return Yield::Return;
                    }
                    if s.d0 != 0 {
                        // (Never: `obj_spawn_at_xy_nodedup` gives a slot with `d0` 0, or none.)
                        // The original frees the first slot here, not the one it was given.
                        in_level(cx, self.frog_only, |c| {
                            objects::obj_free(c, objects::OBJ_SLOTS, 0)
                        });
                        cx.s.set_w(GAVE_UP, 1);
                        return Yield::Return;
                    }
                    self.bob = cx.s.l(s.a0 + 0x50);
                    cx.s.set_l(BOB, self.bob);
                    cx.s.set_w(LOGIC_RUNNING, 1);
                    self.pc = 1;
                    return Yield::Raster;
                }
                // A pass: the objects and the draw list, ...
                1 => {
                    let line = cx.line;
                    let at_end = in_level_from(cx, self.frog_only, line, |c| {
                        objects::obj_run_all(c);
                        objects::build_bob_draw_list(c);
                    });
                    cx.jobs.push(crate::game::Job {
                        inside: IN_PASS[self.pass as usize].to_vec(),
                        writes: PASS_WRITES.to_vec(),
                    });
                    self.pc = 2;
                    if at_end >= super::FRAME_LINES {
                        // On after the next vertical blank, for what is left.
                        self.owed = at_end - super::FRAME_LINES;
                        cx.line = super::FRAME_LINES - 1;
                        return Yield::Poll;
                    }
                    cx.line = at_end;
                }
                // ... the GAME OVER bob first in it, the bobs drawn, the buffers swapped, the
                // scroll latched.
                2 => {
                    let line = cx.line + std::mem::take(&mut self.owed);
                    let bob = self.bob;
                    cx.line = in_level_from(cx, self.frog_only, line, |c| {
                        c.bus.set_l(objects::BOB_DRAW_LIST, bob);
                        objects::bob_draw(c);
                        pass::buffer_flip(c);
                        scroll::scroll_latch(c);
                    });
                    if self.pass == 0 {
                        self.pass = 1;
                        self.pc = 3;
                        return Yield::Raster;
                    }
                    cx.s.set_w(LOGIC_RUNNING, 0);
                    self.pc = 5;
                }
                3 => {
                    self.pc = 1;
                    return Yield::Raster;
                }
                // Fire held: wait for it to be let go.
                5 => {
                    if cx.fire() {
                        return Yield::Poll;
                    }
                    self.count = FIRE_FRAMES;
                    self.pc = 6;
                    return Yield::Raster;
                }
                _ => {
                    if cx.fire() {
                        return Yield::Return;
                    }
                    self.count -= 1;
                    if self.count == 0 {
                        return Yield::Return;
                    }
                    return Yield::Raster;
                }
            }
        }
    }
}

/// `game_over_check` (h0+$749c) when it ends the game (ESC, or no lives): the music faded out, and
/// `top_loop` again from its start, the title (in a game started from the disk; the one run from
/// a loader goes back to it, h0+$7c6, not ported).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct GameOverCheck {
    pc: u8,
}

impl GameOverCheck {
    /// Whether `game_over_check` ends the game: ESC, or the lives gone.
    pub(crate) fn ends(cx: &Cx) -> bool {
        cx.s.w(ABORT_FLAG) != 0 || (cx.s.w(LIVES) as i16) < 0
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        let full = cx.s.w(GAME_FULL_MODE) != 0;
        match self.pc {
            0 => {
                self.pc = 1;
                if full {
                    return call(Routine::MusicFadeWait(Default::default()));
                }
                Yield::Interrupt
            }
            _ => {
                if !full {
                    cx.unported("the game's end after game over (h0+$7c6)");
                    return Yield::Raster;
                }
                Yield::Jump(Box::new(Routine::TopLoop(Default::default())))
            }
        }
    }
}

/// A level routine run on the front end's program from where the beam is.
fn in_level(cx: &mut Cx, frog_only: bool, f: impl FnOnce(&mut LevelCtx)) {
    let line = cx.line;
    in_level_from(cx, frog_only, line, f);
}

/// A level routine run on the front end's program from line `line`: its Paula writes made where
/// its cost puts them. Returns the line it ends on (past the frame's end when it runs on).
fn in_level_from(cx: &mut Cx, frog_only: bool, line: u16, f: impl FnOnce(&mut LevelCtx)) -> u16 {
    let mut end = line;
    let mut writes = Vec::new();
    {
        let dispatch = if frog_only {
            crate::level::frog_only_dispatch()
        } else {
            crate::level::standard_dispatch()
        };
        let mut hw = crate::level::Hw::of(cx.input);
        hw.sdr = cx.keyboard.sdr;
        let mut c = LevelCtx::new(
            crate::mem::Bus::new(cx.s, cx.display.rom()),
            dispatch,
            hw,
            cx.display.sound(),
            &mut writes,
        );
        c.clock = Clock { line, into: 0 };
        f(&mut c);
        end = end.max(c.clock.line);
        for u in std::mem::take(&mut c.unported) {
            cx.unported(u);
        }
    }
    cx.paula.extend(writes);
    end
}

/// The vertical blank while `logic_running` is set outside the level's own loop (in
/// `game_over_seq`): the level's handler, from `line`. Returns the line it ends on.
pub(crate) fn level_vbl(cx: &mut Cx, frog_only: bool, line: u16) -> u16 {
    let polled = !cx.s.b(at(0x16076)) != 0;
    let end = in_level_from(cx, frog_only, line, crate::level::frame::vbl);
    // The poll has handled a new byte: the register reads `$ff` from then on.
    if polled && std::mem::take(&mut cx.keyboard.new) {
        cx.keyboard.sdr = 0xff;
    }
    end
}
