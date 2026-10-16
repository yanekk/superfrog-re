//! What follows a level's loop (`top_loop` after `level_frame_loop`, docs/08 "Scene flow"): the
//! exit fade (`level_exit_fade`, h0+$75f2: `game_over_seq` first, then the palette faded out
//! while the display window closes from the top a line a frame), `game_over_check` (h0+$749c),
//! then the level again after a death or, the exit reached, the tally (`level_tally`,
//! [`crate::level::tally`]).
//!
//! These run on the front end's program (`crate::front`): the main program waits on the beam
//! between them, and the vertical blank runs with the level's logic stopped.

use sf_assets::{LevelId, World};

use crate::front::common::{FadeKind, call, fade};
use crate::front::{Cx, Routine, Yield};
use crate::layout::h0;
use crate::level::timeline::Timeline;
use crate::level::{Hw, LevelCtx, LevelOutcome};
use crate::render::LEVEL_COPPER;
use crate::state::State;

/// `level_copper+6`: DIWSTRT's value in the level's copper list.
const DIWSTRT: u32 = LEVEL_COPPER + 6;
/// The level's palette in its copper list (`level_copper+$2d4`).
const PALETTE: u32 = LEVEL_COPPER + 0x2d4;
/// The blank copper list the screens between levels show (h2+$6fd8).
const BLANK_COPPER: u32 = crate::layout::h2(0x6fd8);
/// `attract_mode` (h0+$18), `abort_flag` (h0+$9de), `level_state` (h0+$a06).
const ATTRACT_MODE: u32 = h0(0x18);
const ABORT_FLAG: u32 = h0(0x9de);
/// `opt_game_mode` (h0+$9e6), and the player's kept at h0+$9e8 while the demo plays.
const OPT_GAME_MODE: u32 = h0(0x9e6);
const GAME_MODE_KEPT: u32 = h0(0x9e8);
const LEVEL_STATE: u32 = h0(0xa06);
/// `frog_lives` (h0+$82e), `level_clock` (h0+$876).
const LIVES: u32 = h0(0x82e);
const LEVEL_CLOCK: u32 = h0(0x876);
/// `world` (h0+$40da): the bonus level is world 7. `shooter_mode` (h0+$9dc).
pub(crate) const WORLD: u32 = h0(0x40da);
pub(crate) const BONUS_WORLD: u32 = 7;
const SHOOTER_MODE: u32 = h0(0x9dc);
/// Where `level_exit_fade` is when its fade set-up returns (h0+$762a).
const AFTER_FADE_SETUP: u32 = h0(0x762a);
/// Lines the window closes by (`moveq #$e`).
const CLOSE_LINES: u16 = 0xe;

/// How the level's loop that has just ended ended: the last life gone (with the clock at 0:00,
/// the time up), ESC, a death with lives left, or the exit.
pub fn outcome_of_loop(s: &State) -> LevelOutcome {
    if s.l(ATTRACT_MODE) == 0 && (s.w(LIVES) as i16) < 0 {
        return if s.l(LEVEL_CLOCK) == 0 {
            LevelOutcome::TimeUp
        } else {
            LevelOutcome::GameOver
        };
    }
    if s.w(ABORT_FLAG) != 0 {
        LevelOutcome::GameOver
    } else if (s.l(LEVEL_STATE) as i32) < 0 {
        LevelOutcome::Died
    } else {
        LevelOutcome::Completed {
            tally: super::Tally::default(),
        }
    }
}

/// `level_exit_fade` and `game_over_check`, then what comes next.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct LevelOver {
    pc: u8,
    /// Run from `top_loop`'s own code ([`crate::front::chain`], T183): over once the screen is
    /// blank, `game_over_check` and what follows being the caller's.
    chained: bool,
    /// Lines the window has still to close.
    left: u16,
    /// The level runs with no object kinds (`crate::Game::frog_only`).
    frog_only: bool,
}

impl LevelOver {
    pub(crate) fn new(frog_only: bool) -> LevelOver {
        LevelOver {
            frog_only,
            ..LevelOver::default()
        }
    }

    /// `level_exit_fade` as `top_loop` calls it ([`crate::front::chain::Chain`]).
    pub(crate) fn chained(frog_only: bool) -> LevelOver {
        LevelOver {
            frog_only,
            chained: true,
            ..LevelOver::default()
        }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        match self.pc {
            0 => {
                // `game_over_seq` first, outside the attract demo (it is over at once unless the
                // last life is gone). After the demo `top_loop` puts the game mode back first
                // (h0+$d2).
                self.pc = 10;
                if cx.s.l(ATTRACT_MODE) != 0 {
                    cx.s.set_w(OPT_GAME_MODE, cx.s.w(GAME_MODE_KEPT));
                } else {
                    return call(Routine::GameOverSeq(crate::front::GameOverSeq::new(
                        self.frog_only,
                    )));
                }
                Yield::Interrupt
            }
            10 => {
                self.pc = 1;
                fade(FadeKind::Out, 0, PALETTE, 0x20, AFTER_FADE_SETUP)
            }
            1 => {
                cx.s.set_w(DIWSTRT, 0x2779);
                self.left = CLOSE_LINES;
                self.pc = 2;
                Yield::Line(0xd0)
            }
            2 => {
                self.pc = 3;
                Yield::Line(0xd1)
            }
            3 => {
                // The window's top a line lower (DIWSTRT's high byte).
                let v = cx.s.b(DIWSTRT).wrapping_add(1);
                cx.s.set_b(DIWSTRT, v);
                self.left -= 1;
                self.pc = if self.left != 0 { 2 } else { 4 };
                if self.left != 0 {
                    Yield::Line(0xd0)
                } else {
                    Yield::Fade(crate::front::fade::RECORDS[0])
                }
            }
            4 => {
                cx.cop1lc = BLANK_COPPER;
                self.pc = 9;
                if cx.s.l(WORLD) == BONUS_WORLD {
                    // h0+$7494, the bonus level's: shooter mode off.
                    cx.s.set_w(SHOOTER_MODE, 0);
                }
                if cx.s.l(ATTRACT_MODE) != 0 {
                    // The attract demo's end: `bra top_loop`, the title again (h0+$e2).
                    return Yield::Jump(Box::new(Routine::TopLoop(Default::default())));
                }
                if self.chained {
                    return Yield::Return;
                }
                // `game_over_check`: ESC (`abort_flag`) or no lives leave the game, a death goes
                // round `top_loop` again, the exit to the tally (T38).
                if crate::front::GameOverCheck::ends(cx) {
                    return call(Routine::GameOverCheck(Default::default()));
                }
                match outcome_of_loop(cx.s) {
                    LevelOutcome::Died => call(Routine::Restart(Restart {
                        pc: 0,
                        frog_only: self.frog_only,
                    })),
                    // The bonus level's end goes to its end picture (h0+$4326, T21), not the tally.
                    LevelOutcome::Completed { .. } if cx.s.l(WORLD) != BONUS_WORLD => {
                        call(Routine::Tally(crate::level::tally::LevelTally::default()))
                    }
                    o => call(Routine::Outcome(o)),
                }
            }
            _ => Yield::Return,
        }
    }
}

/// `level_init_wN`'s calls' return addresses (`level_init_lb`'s for the bonus level), where the
/// original is while it runs.
pub(crate) fn init_inside(id: LevelId) -> Vec<u32> {
    let (base, offsets): (u32, &[u32]) = match id.world {
        World::Bonus => (0x8c3a, &[6, 0x2c, 0x32, 0x36, 0x80, 0x9a]),
        w => (
            match w {
                World::W1 => 0x8cd8,
                World::W2 => 0x8d2e,
                World::W3 => 0x8d84,
                World::W4 => 0x8dda,
                World::W5 => 0x8e30,
                World::W6 => 0x8e86,
                _ => 0x8edc,
            },
            &[6, 0x2e, 0x34, 0x38, 0x4a],
        ),
    };
    offsets.iter().map(|o| h0(base + o)).collect()
}

/// `demo_init`'s calls' return addresses (h0+$8f3a: `level_init_w1`'s code again).
pub(crate) fn demo_init_inside() -> Vec<u32> {
    [6, 0x2e, 0x34, 0x38, 0x4a].iter().map(|o| h0(0x8f3a + o)).collect()
}

/// A level routine run on the front end's program, its Paula writes made on the line the beam is
/// on.
pub(crate) fn with_level(cx: &mut Cx, frog_only: bool, f: impl FnOnce(&mut LevelCtx)) {
    let (_, writes) = level_ctx(cx, frog_only, false, f);
    for w in writes {
        cx.wr(w.reg, w.value);
    }
}

/// A level routine run over the front end's state: what it cost (its trace, when `traced`) and
/// its Paula writes.
fn level_ctx(
    cx: &mut Cx,
    frog_only: bool,
    traced: bool,
    f: impl FnOnce(&mut LevelCtx),
) -> (Vec<crate::level::beam::Tick>, Vec<sf_paula::PaulaWrite>) {
    let mut writes = Vec::new();
    let mut trace = Vec::new();
    {
        let dispatch = if frog_only {
            crate::level::frog_only_dispatch()
        } else {
            crate::level::standard_dispatch()
        };
        let mut c = LevelCtx::new(
            crate::mem::Bus::new(cx.s, cx.display.rom()),
            dispatch,
            Hw::of(cx.input),
            cx.display.sound(),
            &mut writes,
        );
        if traced {
            c.trace = Some(Vec::new());
        }
        f(&mut c);
        if let Some(t) = c.trace.take() {
            trace = t;
        }
        for u in std::mem::take(&mut c.unported) {
            cx.unported(u);
        }
    }
    (trace, writes)
}

/// After a death with lives left (`top_loop`'s `bmi` back to `level_init_wN`): the level set up
/// again, which takes the CPU frames (the engine does it at once; its memory is compared once the
/// original is out of it), then `level_start`, then the level's loop.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Restart {
    pc: u8,
    frog_only: bool,
}

impl Restart {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        match self.pc {
            0 => {
                self.pc = 1;
                let Some(id) = crate::level::load::current(&crate::mem::Bus::new(cx.s, cx.display.rom()))
                else {
                    cx.unported("a restart in a level with no world and level numbers");
                    return Yield::Poll;
                };
                init_on_beam(cx, self.frog_only, id, init_inside(id))
            }
            1 => {
                self.pc = 2;
                call(Routine::LevelStart(LevelStart {
                    frog_only: self.frog_only,
                    ..LevelStart::default()
                }))
            }
            _ => Yield::Return,
        }
    }
}

/// `level_init_wN` for `id` done at once, then replayed on the beam: what it cost and its Paula
/// writes, its memory compared once the original is out of the calls `inside` (T37).
pub(crate) fn init_on_beam(cx: &mut Cx, frog_only: bool, id: LevelId, inside: Vec<u32>) -> Yield {
    let mut written = Vec::new();
    let (trace, ws) = level_ctx(cx, frog_only, true, |c| {
        c.bus.written = Some(vec![false; crate::layout::LAYOUT.len()]);
        crate::level::load::level_init(c, id);
        written = c.bus.written.take().unwrap_or_default();
    });
    let writes = crate::layout::LAYOUT
        .iter()
        .zip(written)
        .filter(|(_, w)| *w)
        .map(|(d, _)| (d.addr, d.width.len()))
        .collect();
    let ws: Vec<(u16, u16)> = ws.iter().map(|w| (w.reg, w.value)).collect();
    let job = crate::game::Job { inside, writes };
    let t = Timeline::of(&trace, &crate::level::beam::BLANK_COSTS, &ws, job);
    call(Routine::Timeline(t))
}

/// Where `level_start` is when its fade set-up returns (h0+$69f8).
const AFTER_FADE_IN_SETUP: u32 = h0(0x69f8);
/// The level's fade target (h0+$9fe).
const FADE_TARGET: u32 = h0(0x9fe);
/// What a step of the panel's slide takes: 70 one-line blits, each set up and waited for.
const SLIDE_STEP_CYCLES: u32 = 70 * 200;

/// `level_start` after its fade's set-up (h0+$6a00): the coins' digits, then the panel's slide
/// begun (h0+$8510): the copper interrupt off for its blits, the window opened.
pub fn start_slide(c: &mut LevelCtx) {
    crate::level::panel::sprite_digits(&mut c.bus);
    c.custom(0x9a, 0x0010);
    crate::level::panel::slide_begin(&mut c.bus);
}

/// `level_start` once the panel is in (h0+$8610): the copper interrupt on, the panel's contents,
/// the drop-in and the music.
pub fn start_after_slide(c: &mut LevelCtx) {
    c.custom(0x9a, 0x8010);
    crate::level::panel::after_slide(c);
    crate::level::load::start_end(c);
}

/// `level_start` (h0+$6952) on the beam: the frog's first step, `wait_frame_raster`, the level's
/// copper list and its colours fading in, the sprites on, the panel sliding in a step a frame (on
/// line `$40`), its contents, the drop-in; then the level's loop.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct LevelStart {
    pc: u8,
    step: u32,
    frog_only: bool,
}

impl LevelStart {
    pub(crate) fn new(frog_only: bool) -> LevelStart {
        LevelStart {
            frog_only,
            ..LevelStart::default()
        }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        use crate::level::panel;
        loop {
            match self.pc {
                0 => {
                    with_level(cx, self.frog_only, crate::level::load::start_frog);
                    self.pc = 1;
                    return Yield::Raster;
                }
                1 => {
                    cx.cop1lc = LEVEL_COPPER;
                    self.pc = 2;
                    let target = cx.s.l(FADE_TARGET);
                    return fade(FadeKind::In { target }, 0, PALETTE, 0x20, AFTER_FADE_IN_SETUP);
                }
                2 => {
                    cx.wr(0x96, 0x8020);
                    with_level(cx, self.frog_only, start_slide);
                    self.pc = 3;
                    return Yield::Line(0x40);
                }
                3 => {
                    panel::slide_step(&mut crate::mem::Bus::new(cx.s, cx.display.rom()), self.step);
                    cx.busy(SLIDE_STEP_CYCLES);
                    self.step += 1;
                    if self.step < panel::SLIDE_STEPS {
                        return Yield::Line(0x40);
                    }
                    self.pc = 4;
                }
                4 => {
                    with_level(cx, self.frog_only, start_after_slide);
                    self.pc = 5;
                    return call(Routine::LevelLoop);
                }
                _ => return Yield::Return,
            }
        }
    }
}
