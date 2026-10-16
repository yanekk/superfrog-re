//! The game's flow (T183, docs/08 "Scene flow"): `top_loop`'s game path from h0+$e6, a fixed
//! sequence of calls per world and level (`access_overlay_show`, `world_load_wN`,
//! `level_resume_jump`, `level_load_wN_lM`, `overlays_end`, `level_init_wN`, `level_start`,
//! `level_frame_loop`, `level_exit_fade`, `game_over_check`, a `bmi` back after a death,
//! `level_tally`, and after a world's fourth level `end_world_pic_wN` and `music_fade_wait`).
//!
//! [`Chain`] runs that code as the original has it in hunk 0: it reads each instruction and calls
//! the engine's routine for each call's target, so the order, the branches and the resume points
//! a level code jumps to (`start_resume_table`) are the original's own. The screens between the
//! levels are here too: the world's loading picture with the frog's walk across it
//! (`world_intro_show`, `frog_exit_walk`), the progress map (`progress_map_show`,
//! `progress_map_vbl`) and the end-of-world picture (`end_world_pic_show`), each taken down by
//! `overlays_end`.

use sf_assets::{LevelId, World};

use super::common::{FadeKind, MusicFadeWait, MusicStart, call, fade};
use super::fade::RECORDS;
use super::title::{BLANK_COPPER, BUFFER_0, BUFFER_1, OVERLAY_PALETTE};
use super::{Cx, Routine, Yield, at, h2, sprite};
use crate::audio::DMACON;
use crate::game::LoadEnd;
use crate::layout::H0;
use crate::level::exit;

/// `attract_mode` (h0+$18), `game_full_mode` (h0+$6).
const ATTRACT_MODE: u32 = at(0x18);
const GAME_FULL_MODE: u32 = at(0x6);
/// `start_world`, `start_level` (h0+$a, $e), `level_started` (h0+$9f0), `resume_target` (h0+$bda).
const START_WORLD: u32 = at(0xa);
const START_LEVEL: u32 = at(0xe);
const LEVEL_STARTED: u32 = at(0x9f0);
const RESUME_TARGET: u32 = at(0xbda);
/// `world`, `level` (h0+$40da, $40de), `level_clock` (h0+$876).
const WORLD: u32 = at(0x40da);
const LEVEL: u32 = at(0x40de);
const LEVEL_CLOCK: u32 = at(0x876);
/// `display_timer` (h0+$123d4), counted down by the vertical blank.
const DISPLAY_TIMER: u32 = at(0x123d4);
/// `frog_exit_sprites` (h0+$9f8): the frog shows in the second pair of sprite records.
const EXIT_SPRITES: u32 = at(0x9f8);
/// The screens' flags: the progress map up (h0+$4112), the loading picture shown (h0+$42a6), the
/// end-of-world picture up (h0+$445a).
pub(crate) const MAP_SHOWN: u32 = at(0x4112);
pub(crate) const INTRO_SHOWN: u32 = at(0x42a6);
pub(crate) const END_PIC_SHOWN: u32 = at(0x445a);
/// `shooter_mode` (h0+$9dc).
const SHOOTER_MODE: u32 = at(0x9dc);
/// `top_loop` (h0+$5c) and its game path's start (h0+$e6).
const TOP_LOOP: u32 = 0x5c;
pub(crate) const GAME_PATH: u32 = 0xe6;

/// The loading picture's copper list, palette and plane pointers (h2+$6750, $67f4, $6874, $69bc).
const INTRO_COPPER: u32 = h2(0x6750);
const INTRO_PALETTE: u32 = h2(0x67f4);
const INTRO_PLANES: u32 = h2(0x6874);
const INTRO_PLANES_LOW: u32 = h2(0x69bc);
/// The loading picture's split: a wait for line `$ff`, then the line its lower part starts on.
const INTRO_SPLIT: u32 = h2(0x69b4);
const INTRO_SPLIT_LINE: u32 = h2(0x69b8);
/// The progress map's copper list, its sprite DMA move, palette and planes.
const MAP_COPPER: u32 = h2(0x69f0);
const MAP_DMACON: u32 = h2(0x6a96);
const MAP_PALETTE: u32 = h2(0x6a98);
const MAP_PLANES: u32 = h2(0x6b18);
/// The end-of-world picture's copper list, palette and planes.
const END_COPPER: u32 = h2(0x6494);
const END_PALETTE: u32 = h2(0x6538);
const END_PLANES: u32 = h2(0x65b8);

/// What a call in `top_loop` goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    AccessOverlay,
    WorldLoad(u32),
    ResumeJump,
    LevelLoad(u32, u8),
    OverlaysEnd,
    LevelInit(u32),
    LevelStart,
    FrameLoop,
    ExitFade,
    GameOverCheck,
    Tally,
    EndPic(u32),
    MusicFadeWait,
    /// `level_init_lw` (h0+$8edc): the final level's init (T18).
    FinalInit,
    /// `end_seq_load` (h0+$a65c) and `end_sequence` (h0+$a6e4), after the final level (T18).
    EndSeqLoad,
    EndSequence,
    /// `$adca`, `$add6`: the cracked disk requests, a bare `rts`.
    Nothing,
    ShooterOff,
    Other,
}

/// `world_load_wN` for worlds 1–6.
const WORLD_LOADS: [u32; 6] = [0x94a0, 0x9722, 0x9976, 0x9bca, 0x9e1e, 0xa072];
/// `level_load_wN_lM`.
const LEVEL_LOADS: [[u32; 4]; 6] = [
    [0x9552, 0x95c6, 0x963a, 0x96ae],
    [0x97c6, 0x9832, 0x989e, 0x990a],
    [0x9a1a, 0x9a86, 0x9af2, 0x9b5e],
    [0x9c6e, 0x9cda, 0x9d46, 0x9db2],
    [0x9ec2, 0x9f2e, 0x9f9a, 0xa006],
    [0xa116, 0xa1b8, 0xa25a, 0xa2fc],
];
/// `level_init_wN`.
const LEVEL_INITS: [u32; 6] = [0x8cd8, 0x8d2e, 0x8d84, 0x8dda, 0x8e30, 0x8e86];
/// `level_init_lw`: the final level's init.
const FINAL_INIT: u32 = 0x8edc;
/// `end_world_pic_wN` (h0+$42aa..$4326): the world before the next, and its picture's file.
const END_PICS: [(u32, u32); 6] = [
    (0x42aa, 1),
    (0x42be, 2),
    (0x42ea, 3),
    (0x42fe, 4),
    (0x4312, 5),
    (0x4326, 6),
];

fn target(off: u32) -> Target {
    if let Some(w) = WORLD_LOADS.iter().position(|&a| a == off) {
        return Target::WorldLoad(w as u32 + 1);
    }
    for (w, loads) in LEVEL_LOADS.iter().enumerate() {
        if let Some(l) = loads.iter().position(|&a| a == off) {
            return Target::LevelLoad(w as u32 + 1, l as u8 + 1);
        }
    }
    if let Some(w) = LEVEL_INITS.iter().position(|&a| a == off) {
        return Target::LevelInit(w as u32 + 1);
    }
    if let Some(&(_, w)) = END_PICS.iter().find(|p| p.0 == off) {
        return Target::EndPic(w);
    }
    match off {
        0xb6f8 => Target::AccessOverlay,
        0xbb2 => Target::ResumeJump,
        0xb774 => Target::OverlaysEnd,
        0x6952 => Target::LevelStart,
        0xaa6 => Target::FrameLoop,
        0x75f2 => Target::ExitFade,
        0x749c => Target::GameOverCheck,
        0x4466 => Target::Tally,
        0x1327a => Target::MusicFadeWait,
        FINAL_INIT => Target::FinalInit,
        0xa65c => Target::EndSeqLoad,
        0xa6e4 => Target::EndSequence,
        0xadca | 0xadd6 => Target::Nothing,
        0x7494 => Target::ShooterOff,
        _ => Target::Other,
    }
}

fn level_id(world: u32, level: u8) -> LevelId {
    let world = match world {
        1 => World::W1,
        2 => World::W2,
        3 => World::W3,
        4 => World::W4,
        5 => World::W5,
        _ => World::W6,
    };
    LevelId { world, level, alt: false }
}

/// Where in `top_loop` the level `world`.`level` (1–6, 1–4) has its `jsr level_init_wN`: the
/// `level`th call of its world's init in the loop.
pub(crate) fn init_call(data: &super::FrontData, world: u32, level: u32) -> Option<u32> {
    let init = *LEVEL_INITS.get(world.checked_sub(1)? as usize)?;
    let mut seen = 0;
    let mut pc = GAME_PATH;
    while pc < 0x7d4 {
        if data.h0_w(pc) == 0x4eb9 && data.h0_l(pc + 2).wrapping_sub(H0) == init {
            seen += 1;
            if seen == level {
                return Some(pc);
            }
        }
        pc += 2;
    }
    None
}

/// Where in `top_loop` the level the state is in has its `jsr level_init_wN` (worlds 1–6 and
/// the final level's `jsr level_init_lw`, by `final_level_flag`; not in the attract demo, which
/// `top_loop` plays apart).
pub(crate) fn level_call(s: &crate::state::State, data: &super::FrontData) -> Option<u32> {
    if s.l(ATTRACT_MODE) != 0 {
        return None;
    }
    if s.w(at(0x1e)) != 0 {
        return (GAME_PATH..0x7d4)
            .step_by(2)
            .find(|&pc| data.h0_w(pc) == 0x4eb9 && data.h0_l(pc + 2).wrapping_sub(H0) == FINAL_INIT);
    }
    init_call(data, s.l(WORLD), s.l(LEVEL))
}

/// `top_loop` from `pc` (a hunk 0 offset), run as the original's code reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Chain {
    pc: u32,
    /// The condition codes the last `tst` left: negative, zero.
    n: bool,
    z: bool,
    frog_only: bool,
}

impl Chain {
    pub(crate) fn at(pc: u32, frog_only: bool) -> Chain {
        Chain {
            pc,
            n: false,
            z: false,
            frog_only,
        }
    }

    /// After the level whose `jsr level_init_wN` is at `init`: at its `jsr level_exit_fade`.
    pub(crate) fn after_loop(init: u32, frog_only: bool) -> Chain {
        Chain::at(init + 18, frog_only)
    }

    /// After the level whose `jsr level_init_wN` is at `init`: after its `jsr level_tally`.
    pub(crate) fn after_tally(init: u32, frog_only: bool) -> Chain {
        Chain::at(init + 0x2c, frog_only)
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        for _ in 0..64 {
            let pc = self.pc;
            let op = cx.data.h0_w(pc);
            match op {
                // jsr abs.l
                0x4eb9 => {
                    let to = cx.data.h0_l(pc + 2).wrapping_sub(H0);
                    self.pc = pc + 6;
                    if let Some(y) = self.dispatch(cx, to) {
                        return y;
                    }
                }
                // tst.l abs.l, tst.w abs.l
                0x4ab9 | 0x4a79 => {
                    let a = cx.data.h0_l(pc + 2);
                    let v = if op == 0x4ab9 {
                        cx.s.l(a)
                    } else {
                        cx.s.w(a) as i16 as i32 as u32
                    };
                    (self.n, self.z) = ((v as i32) < 0, v == 0);
                    self.pc = pc + 6;
                }
                // move.w abs.l, abs.l
                0x33f9 => {
                    let v = cx.s.w(cx.data.h0_l(pc + 2));
                    cx.s.set_w(cx.data.h0_l(pc + 6), v);
                    self.pc = pc + 10;
                }
                // Bcc
                _ if op & 0xf000 == 0x6000 => {
                    let (to, next) = match op & 0xff {
                        // Displacements back are negative: the sum wraps (a `bmi` after a death).
                        0 => ((pc + 2).wrapping_add(cx.data.h0_w(pc + 2) as i16 as i32 as u32), pc + 4),
                        d => ((pc + 2).wrapping_add(d as i8 as i32 as u32), pc + 2),
                    };
                    let taken = match op >> 8 & 0xf {
                        0x0 => true,
                        0x6 => !self.z,
                        0x7 => self.z,
                        0xa => !self.n,
                        0xb => self.n,
                        _ => {
                            cx.unported("a branch in top_loop the engine does not read");
                            return Yield::Poll;
                        }
                    };
                    self.pc = if taken { to } else { next };
                    if self.pc == TOP_LOOP {
                        return Yield::Jump(Box::new(Routine::TopLoop(Default::default())));
                    }
                }
                _ => {
                    cx.unported("top_loop past the game's levels (the bonus level, the final level, the end)");
                    return Yield::Poll;
                }
            }
        }
        cx.unported("top_loop ran 64 instructions without waiting");
        Yield::Poll
    }

    fn dispatch(&mut self, cx: &mut Cx, to: u32) -> Option<Yield> {
        let ret = at(self.pc);
        let frog_only = self.frog_only;
        Some(match target(to) {
            Target::AccessOverlay => {
                access_overlay_show(cx);
                return None;
            }
            Target::WorldLoad(w) => call(Routine::WorldLoad(WorldLoad::new(w))),
            Target::ResumeJump => {
                if cx.s.l(START_WORLD) != 0
                    && cx.s.l(START_LEVEL) != 0
                    && cx.s.w(LEVEL_STARTED) == 0
                {
                    self.pc = cx.s.l(RESUME_TARGET).wrapping_sub(H0);
                }
                return None;
            }
            Target::LevelLoad(w, l) => call(Routine::LevelLoad(LevelLoad::new(level_id(w, l), ret))),
            Target::OverlaysEnd => call(Routine::Overlay(Default::default())),
            Target::LevelInit(w) => {
                let level = cx.s.l(LEVEL) as u8;
                let id = level_id(w, level);
                exit::init_on_beam(cx, frog_only, id, exit::init_inside(id))
            }
            Target::FinalInit => {
                let id = LevelId {
                    world: World::Final,
                    level: 1,
                    alt: false,
                };
                exit::init_on_beam(cx, frog_only, id, exit::init_inside(id))
            }
            Target::EndSeqLoad => call(Routine::EndSeqLoad(Default::default())),
            Target::EndSequence => call(Routine::EndSequence(Default::default())),
            Target::LevelStart => call(Routine::LevelStart(exit::LevelStart::new(frog_only))),
            Target::FrameLoop => call(Routine::LevelLoop),
            Target::ExitFade => call(Routine::LevelOver(exit::LevelOver::chained(frog_only))),
            Target::GameOverCheck => {
                if super::GameOverCheck::ends(cx) {
                    call(Routine::GameOverCheck(Default::default()))
                } else {
                    return None;
                }
            }
            Target::Tally => call(Routine::Tally(Default::default())),
            Target::EndPic(w) => call(Routine::EndPic(EndPic::new(w, to == 0x42be))),
            Target::MusicFadeWait => call(Routine::MusicFadeWait(MusicFadeWait::default())),
            Target::Nothing => return None,
            Target::ShooterOff => {
                cx.s.set_w(SHOOTER_MODE, 0);
                return None;
            }
            Target::Other => {
                cx.unported("a call in top_loop the engine does not follow (the bonus or final level)");
                Yield::Poll
            }
        })
    }
}

/// `game_start_at` (h0+$b66), entered from `top_loop` (h0+$9e): the level code checked, then a
/// `jmp` to the start world's entry in `top_loop` (h0+$e6 for world 1, also with no code) with
/// the start level's call kept in `resume_target` for `level_resume_jump`.
pub(crate) fn game_start_at(cx: &mut Cx) -> u32 {
    super::password::check(cx);
    let world = cx.s.l(START_WORLD);
    if world == 0 || world >= 8 {
        return GAME_PATH;
    }
    let row = 0xbde + (world - 1) * 0x14;
    let level = cx.s.l(START_LEVEL);
    let target = cx.l(at(row + (level.wrapping_sub(1) * 4 + 4) & 0xffff));
    cx.s.set_l(RESUME_TARGET, target);
    cx.l(at(row)).wrapping_sub(H0)
}

/// `access_overlay_show` (h0+$b6f8): the Accessing-disk overlay, unless one is up or no
/// trackloader runs.
pub(crate) fn access_overlay_show(cx: &mut Cx) {
    if cx.s.l(END_PIC_SHOWN) == 0 && cx.s.l(OVERLAY_PALETTE) == 0 && cx.s.w(at(0x8)) != 0 {
        super::title::overlay_show(cx);
    }
}

/// The world's files: its effects bank and music, then its graphics.
fn world_sound(cx: &mut Cx, world: u32) {
    use crate::audio::{BANK_ADDR, BankId, File, TuneId};
    let Some(d) = cx.data.sound.as_ref() else {
        return;
    };
    let bank = if world <= 2 { BankId::Worlds12 } else { BankId::Worlds36 };
    cx.s.audio.file_loaded(d, BANK_ADDR, File::Bank(bank));
    cx.s.audio.sfx_bank_init(d);
    let at = cx.s.audio.module_addr();
    cx.s.audio.file_loaded(d, at, File::Tune(TuneId::World(world as u8)));
}

/// `world_load_wN` (h0+$94a0 …): `world` set, the effects and the music loaded, the loading
/// picture shown (`world_intro_show`), the map area cleared (h0+$172d6), the tiles, bobs and
/// masks loaded. Loads are instant; the picture's frog walks meanwhile in the vertical blank.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WorldLoad {
    world: u32,
    pc: u8,
}

impl WorldLoad {
    fn new(world: u32) -> WorldLoad {
        WorldLoad { world, pc: 0 }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        self.pc += 1;
        match self.pc {
            1 => {
                cx.s.set_l(WORLD, self.world);
                world_sound(cx, self.world);
                if cx.s.l(END_PIC_SHOWN) != 0 {
                    // Under the end-of-world picture: the original is past these loads and in
                    // h0+$43e0's wait, called from `world_intro_show`.
                    cx.loaded(LoadEnd::Reached(at(0x411c)));
                }
                call(Routine::WorldIntro(WorldIntro::new(self.world)))
            }
            2 => {
                let display = cx.display;
                let world = level_id(self.world, 1).world;
                exit::with_level(cx, false, |c| {
                    crate::level::load::clear_map_area(&mut c.bus);
                    c.bus.st.poke(crate::layout::H3, display.bob_sheet(world));
                });
                // The original waits for the tiles in `world_intro_end`, called from
                // `progress_map_show`'s `overlays_end` (under the trackloader the runner's stack
                // never reaches `top_loop`'s return address).
                cx.loaded(LoadEnd::Reached(at(0x3df6)));
                Yield::Return
            }
            _ => Yield::Return,
        }
    }
}

/// `world_intro_show` (h0+$4116) with `LnLP` loaded: the Accessing-disk overlay taken down, the
/// picture's planes (an upper and a lower part), the frog and the clouds set up for their walk
/// (h0+$12408), the picture faded in, its copper list, the world's music, the walk started.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WorldIntro {
    world: u32,
    pc: u8,
}

impl WorldIntro {
    fn new(world: u32) -> WorldIntro {
        WorldIntro { world, pc: 0 }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            self.pc += 1;
            match self.pc {
                1 => {
                    if cx.s.l(END_PIC_SHOWN) != 0 {
                        return call(Routine::EndPicEnd(Default::default()));
                    }
                }
                2 => {
                    if cx.s.w(GAME_FULL_MODE) == 0 {
                        return Yield::Return;
                    }
                    let pic = cx.data.file(&format!("L{}LP", self.world)).to_vec();
                    cx.s.write(BUFFER_0, &pic);
                    cx.loaded(LoadEnd::Left(at(0x4132)));
                    return call(Routine::Overlay(Default::default()));
                }
                3 => {
                    set_planes(cx, INTRO_PLANES, BUFFER_0, 0x3700);
                    set_planes(cx, INTRO_PLANES_LOW, BUFFER_0 + 0x23a0, 0x3700);
                    cx.s.set_l(INTRO_SPLIT, 0xffe1_fffe);
                    cx.s.set_l(INTRO_SPLIT_LINE, 0x1001_ff00);
                    if cx.s.l(WORLD) == 7 {
                        self.pc = 4;
                        continue;
                    }
                    walk_setup(cx);
                    return Yield::Raster;
                }
                4 => cx.wr(DMACON, 0x8020),
                5 => return Yield::Fade(RECORDS[0]),
                6 => {
                    return fade(
                        FadeKind::In { target: BUFFER_0 + 0x11300 },
                        0,
                        INTRO_PALETTE,
                        0x20,
                        at(0x41e6),
                    );
                }
                7 => return Yield::Raster,
                8 => {
                    cx.cop1lc = INTRO_COPPER;
                    cx.s.set_l(INTRO_SHOWN, 1);
                    if cx.s.l(ATTRACT_MODE) == 0 && !cx.s.audio.music_on() {
                        return call(Routine::MusicStart(MusicStart::keeping_module(
                            at(0x4216),
                            MUSIC_INIT_LINES,
                        )));
                    }
                }
                _ => {
                    cx.s.set_w(EXIT_SPRITES, 1);
                    return Yield::Return;
                }
            }
        }
    }
}

/// `p41_init`'s lines on a world's module (taken as the title module's).
const MUSIC_INIT_LINES: u16 = 20;

/// Five plane pointers from `first`, `step` apart, into the copper MOVEs at `moves`.
fn set_planes(cx: &mut Cx, moves: u32, first: u32, step: u32) {
    for k in 0..5 {
        let p = first + k * step;
        cx.s.set_w(moves + 8 * k + 6, p as u16);
        cx.s.set_w(moves + 8 * k + 2, (p >> 16) as u16);
    }
}

/// The frog's two sprite records (h0+$1b02c, $1b048) and the six clouds' (h0+$1b064 ...).
const EXIT_LEFT: u32 = crate::frog::anim::EXIT_SPRITE_LEFT;
const EXIT_RIGHT: u32 = crate::frog::anim::EXIT_SPRITE_RIGHT;
const CLOUDS: [u32; 6] = [
    at(0x1b064),
    at(0x1b080),
    at(0x1b09c),
    at(0x1b0b8),
    at(0x1b0d4),
    at(0x1b0f0),
];
/// `frog_exit_walk`'s state (the code it calls), wait, speed and step counters.
const WALK: u32 = at(0x11e92);
const WAIT: u32 = at(0x11fd8);
const SPEED: u32 = at(0x11fda);
const STEP: u32 = at(0x11fdc);
/// The clouds' bob: the every-other-frame toggle, three table pointers (and each table's start),
/// three base heights.
const BOB_TOGGLE: u32 = at(0x122f0);
const BOB_PTRS: u32 = at(0x122f2);
const BOB_BASE: u32 = at(0x12334);
/// `frog_anim_step` (h0+$806).
const FROG_ANIM_STEP: u32 = at(0x806);

/// h0+$12408: the frog's sprites at the right edge, the clouds', the walk's first state for the
/// world.
fn walk_setup(cx: &mut Cx) {
    cx.s.set_w(EXIT_LEFT, 0x150);
    sprite::attach(cx.s, EXIT_LEFT);
    cx.s.set_w(EXIT_RIGHT, 0x160);
    sprite::attach(cx.s, EXIT_RIGHT);
    for c in CLOUDS {
        sprite::attach(cx.s, c);
    }
    for (k, t) in [0x1230a, 0x12310, 0x12316].into_iter().enumerate() {
        cx.s.set_l(BOB_PTRS + 8 * k as u32, at(t));
    }
    cx.s.set_l(WALK, at(0x7d4));
    // h0+$12526 (worlds 1, 3, 6, the bonus level), $1268c (2), $12860 (4, 5): the picture's
    // colour splits, the walk's first state and the frog's pose, the clouds' animations.
    let world = cx.s.l(WORLD);
    let (walk, anim, tables) = match world {
        2 => (0x11e96, 0x10d8e, (0x127d8, 0x1281c)),
        4 | 5 => (0x11f44, 0x105b6, (0x1298c, 0x129d0)),
        7 => (0x11f44, 0x103a6, (0x12664, 0x12678)),
        _ => (0x11f44, 0x105b6, (0x12664, 0x12678)),
    };
    cx.s.set_l(h2(0x68e8), 0x6401_ff00);
    cx.s.set_l(h2(0x692c), 0x7f01_ff00);
    cx.s.set_l(h2(0x6970), 0x9a01_ff00);
    if world == 2 {
        // In from the left.
        cx.s.set_w(EXIT_LEFT, 0xffe0);
        sprite::attach(cx.s, EXIT_LEFT);
        cx.s.set_w(EXIT_RIGHT, 0xfff0);
        sprite::attach(cx.s, EXIT_RIGHT);
    }
    cx.s.set_l(WALK, at(walk));
    cx.s.set_w(WAIT, 0x28);
    cx.s.set_w(EXIT_RIGHT, if world == 2 { 0xfff0 } else { 0x160 });
    exit::with_level(cx, false, |c| crate::frog::anim::frog_set_anim(c, at(anim)));
    for (k, &c) in CLOUDS.iter().enumerate() {
        let table = at(if k % 2 == 0 { tables.0 } else { tables.1 });
        cx.s.set_l(c + 0x10, table);
        cx.s.set_l(c + 0x14, table);
        cx.s.set_w(c + 0x18, 0);
    }
    cx.s.set_w(BOB_BASE, 0x38);
    cx.s.set_w(BOB_BASE + 2, 0x53);
    cx.s.set_w(BOB_BASE + 4, 0x6e);
    for (&c, x) in CLOUDS.iter().zip([0xffc8, 0xffd8, 0xffd0, 0xffe0, 0xffb8, 0xffc8]) {
        cx.s.set_w(c, x);
        sprite::update(cx.s, c);
    }
    // h0+$124fa: the frog's sprites into the list.
    sprite::update(cx.s, EXIT_LEFT);
    sprite::update(cx.s, EXIT_RIGHT);
}

/// `frog_exit_walk` (h0+$11e62) in the vertical blank while `frog_exit_sprites` is set: the
/// walk's state, the frog's animation, its sprites.
pub(crate) fn exit_walk_vbl(cx: &mut Cx) {
    exit::with_level(cx, false, |c| {
        walk_step(c);
        crate::frog::anim::frog_anim_tick(c);
        sprite::update(c.bus.st, EXIT_LEFT);
        sprite::update(c.bus.st, EXIT_RIGHT);
    });
}

/// One vertical blank of the walk's state machine (h0+$11e96 .. $123d6), by the code address in
/// [`WALK`].
fn walk_step(c: &mut crate::level::LevelCtx) {
    use crate::frog::anim::frog_set_anim;
    let state = c.bus.l(WALK).wrapping_sub(H0);
    let world7 = c.bus.l(WORLD) == 7;
    let x = |b: &crate::mem::Bus| b.w(EXIT_LEFT) as i16;
    let shift = |b: &mut crate::mem::Bus, d: i16| {
        for r in [EXIT_LEFT, EXIT_RIGHT] {
            b.set_w(r, b.w(r).wrapping_add(d as u16));
        }
    };
    // `subq.w #1; bpl` on a counter: true while it has not run out.
    let waiting = |b: &mut crate::mem::Bus, a: u32| {
        let v = b.w(a).wrapping_sub(1);
        b.set_w(a, v);
        (v as i16) >= 0
    };
    let go = |b: &mut crate::mem::Bus, to: u32| b.set_l(WALK, at(to));
    let slow = |b: &mut crate::mem::Bus| {
        if b.w(SPEED) != 1 {
            b.set_w(SPEED, b.w(SPEED).wrapping_sub(1));
        }
    };
    match state {
        0x11e96 => {
            let b = &mut c.bus;
            if waiting(b, WAIT) {
                return;
            }
            shift(b, 6);
            if x(b) >= 0x60 {
                b.set_l(FROG_ANIM_STEP, at(0x10cd6));
            }
            if x(b) < 0x74 {
                return;
            }
            go(b, 0x11eea);
            b.set_w(SPEED, 5);
            frog_set_anim(c, at(0x10cd6));
        }
        0x11eea => {
            let b = &mut c.bus;
            slow(b);
            let d = b.w(SPEED) as i16;
            shift(b, d);
            if x(b) < 0x90 {
                return;
            }
            go(b, 0x120c2);
            b.set_w(WAIT, 5);
            if !world7 {
                frog_set_anim(c, at(0x10c9a));
            }
        }
        0x11f44 => {
            let b = &mut c.bus;
            b.set_w(DISPLAY_TIMER, 0x1388);
            if waiting(b, WAIT) {
                return;
            }
            shift(b, -6);
            if x(b) < 0xc8 && !world7 {
                b.set_l(FROG_ANIM_STEP, at(0x104fe));
            }
            if x(b) >= 0xb4 {
                return;
            }
            go(b, 0x12068);
            if b.l(WORLD) == 5 {
                go(b, 0x11fde);
                b.set_w(STEP, 0xb);
            }
            b.set_w(SPEED, 5);
            if !world7 {
                frog_set_anim(c, at(0x104fe));
            }
        }
        0x11fde => {
            frog_set_anim(c, at(0x1049e));
            if x(&c.bus) < 0x30 {
                frog_set_anim(c, at(0x104aa));
            }
            let b = &mut c.bus;
            if b.w(SPEED) != 1 {
                let s = b.w(STEP).wrapping_sub(1);
                b.set_w(STEP, s);
                if s == 0 {
                    b.set_w(STEP, 0xb);
                    b.set_w(SPEED, b.w(SPEED).wrapping_sub(1));
                }
            }
            let d = b.w(SPEED) as i16;
            shift(b, -d);
            if x(b) >= -0x26 {
                return;
            }
            b.set_w(EXIT_LEFT, 0xffe8);
            b.set_w(EXIT_RIGHT, 0xfff8);
            go(b, 0x1213e);
            b.set_w(WAIT, 1);
            frog_set_anim(c, at(0x10d8e));
        }
        0x12068 => {
            let b = &mut c.bus;
            slow(b);
            let d = b.w(SPEED) as i16;
            shift(b, -d);
            if x(b) >= 0x90 {
                return;
            }
            go(b, 0x120c2);
            b.set_w(WAIT, 5);
            if !world7 {
                frog_set_anim(c, at(0x10492));
            }
        }
        0x120c2 => {
            let b = &mut c.bus;
            if waiting(b, WAIT) {
                return;
            }
            frog_set_anim(c, at(0x1049e));
            if world7 {
                frog_set_anim(c, at(0x104b6));
            }
            go(&mut c.bus, 0x12100);
            c.bus.set_w(WAIT, 0x50);
        }
        0x12100 => {
            let b = &mut c.bus;
            if waiting(b, WAIT) {
                return;
            }
            frog_set_anim(c, at(0x104aa));
            if world7 {
                frog_set_anim(c, at(0x104c2));
            }
            go(&mut c.bus, 0x1213e);
            c.bus.set_w(WAIT, 0x46);
        }
        0x1213e => {
            let b = &mut c.bus;
            b.set_w(DISPLAY_TIMER, 0x32);
            clouds(b);
            if waiting(b, WAIT) {
                return;
            }
            frog_set_anim(c, at(0x10d8e));
            if world7 {
                frog_set_anim(c, at(0x10b7e));
            }
            let b = &mut c.bus;
            go(b, 0x1218c);
            b.set_w(WAIT, 0);
            b.set_w(SPEED, 0);
        }
        0x1218c => {
            let b = &mut c.bus;
            b.set_w(DISPLAY_TIMER, 0x32);
            clouds(b);
            b.set_w(SPEED, b.w(SPEED).wrapping_add(1));
            if b.w(SPEED) as i16 >= 6 && (b.w(WAIT) as i16) < 7 {
                b.set_w(WAIT, b.w(WAIT).wrapping_add(1));
                b.set_w(SPEED, 0);
            }
            let d = b.w(WAIT) as i16;
            shift(b, d);
            if x(b) < 0x144 {
                return;
            }
            go(b, 0x1233a);
            b.set_w(WAIT, 0x5a);
        }
        0x1233a => {
            let b = &mut c.bus;
            if world7 {
                go(b, 0x7d4);
                return;
            }
            b.set_w(DISPLAY_TIMER, 0x32);
            clouds(b);
            if waiting(b, WAIT) {
                return;
            }
            // The lower part of the picture rises a line a frame.
            b.set_w(DISPLAY_TIMER, 0x28a);
            let line = b.l(INTRO_SPLIT_LINE).wrapping_sub(0x0100_0000);
            b.set_l(INTRO_SPLIT_LINE, line);
            if line == 0xff01_ff00 {
                b.set_l(INTRO_SPLIT, 0x0098_0000);
            }
            if line != 0xc001_ff00 {
                return;
            }
            go(b, 0x7d4);
            go(b, 0x123d6);
            frog_set_anim(c, at(0x105b6));
            c.bus.set_w(WAIT, 0x8c);
        }
        0x123d6 => {
            let b = &mut c.bus;
            if waiting(b, WAIT) {
                return;
            }
            shift(b, -10);
            if x(b) >= -0x26 {
                return;
            }
            go(b, 0x7d4);
        }
        _ => {}
    }
}

/// h0+$121f2: every other frame the clouds bob a step (each pair from its table), and each cloud
/// left of x `$150` drifts two pixels right.
fn clouds(b: &mut crate::mem::Bus) {
    let t = !b.w(BOB_TOGGLE);
    b.set_w(BOB_TOGGLE, t);
    if t != 0 {
        for k in 0..3u32 {
            let ptr = BOB_PTRS + 8 * k;
            b.set_l(ptr, b.l(ptr).wrapping_add(2));
            let mut a5 = b.l(ptr);
            if (b.w(a5) as i16) < 0 {
                a5 = b.l(ptr + 4);
                b.set_l(ptr, a5);
            }
            let y = b.w(a5).wrapping_add(b.w(BOB_BASE + 2 * k));
            b.set_w(CLOUDS[2 * k as usize] + 2, y);
            b.set_w(CLOUDS[2 * k as usize + 1] + 2, y);
        }
    }
    for c in CLOUDS {
        if (b.w(c) as i16) < 0x150 {
            b.set_w(c, b.w(c).wrapping_add(2));
            sprite::update(b.st, c);
        }
    }
}

/// `progress_map_vbl` (h0+$3f22) while the progress map is up: the marker shown 32 frames in 64
/// (sprite DMA on and off in its copper list).
pub(crate) fn progress_map_vbl(cx: &mut Cx) {
    let n = cx.s.w(at(0x3f50)).wrapping_add(1);
    cx.s.set_w(at(0x3f50), n);
    cx.s.set_w(MAP_DMACON, if n & 0x20 != 0 { 0x20 } else { 0x8020 });
}

/// `level_load_wN_lM` (h0+$9552 …): `level` and the clock set, the progress map shown, then the
/// map loaded and read (instant).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LevelLoad {
    id: LevelId,
    ret: u32,
    pc: u8,
}

impl LevelLoad {
    fn new(id: LevelId, ret: u32) -> LevelLoad {
        LevelLoad { id, ret, pc: 0 }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        self.pc += 1;
        match self.pc {
            1 => {
                cx.s.set_l(LEVEL, self.id.level as u32);
                for (k, d) in [0u8, 4, 0, 0].into_iter().enumerate() {
                    cx.s.set_b(LEVEL_CLOCK + k as u32, d);
                }
                call(Routine::MapShow(Default::default()))
            }
            2 => {
                let (display, id) = (cx.display, self.id);
                exit::with_level(cx, false, |c| crate::level::load::level_load(c, display, id));
                cx.loaded(LoadEnd::Left(self.ret));
                Yield::Return
            }
            _ => Yield::Return,
        }
    }
}

/// How long unpacking the progress map's picture takes (fitted to the oracle: T183).
const MAP_UNPACK_CYCLES: u32 = (30 * 313 + 150) * super::BEAM_CYCLES_PER_LINE;

/// `progress_map_show` (h0+$3de8): the screen before taken down (`overlays_end`), the map's
/// picture (packed in hunk 0) unpacked, its marker on the level's place, faded in, shown for 300
/// frames (`display_timer`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct MapShow {
    pc: u8,
}

impl MapShow {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        self.pc += 1;
        match self.pc {
            1 => {
                if cx.s.w(GAME_FULL_MODE) == 0 {
                    self.pc = 9;
                    return Yield::Return;
                }
                call(Routine::Overlay(Default::default()))
            }
            2 => {
                if cx.s.l(MAP_SHOWN) != 0 {
                    return Yield::Return;
                }
                let packed: Vec<u8> = (0..0x3e80).map(|k| cx.b(at(0x37a0a) + k)).collect();
                cx.s.write(BUFFER_0, &packed);
                let pic = cx.data.file("h0@37a0a").to_vec();
                cx.unpack(BUFFER_0, &pic, MAP_UNPACK_CYCLES, &[at(0x3e2a)]);
                Yield::Interrupt
            }
            3 => {
                set_planes(cx, MAP_PLANES, BUFFER_0, 0x2800);
                // h0+$3f52: the marker's sprite data, then its place for the world and level.
                let data: Vec<u8> = (0..0x300).map(|k| cx.b(at(0x3aa8c) + k)).collect();
                cx.s.write(BUFFER_0 + 0xc840, &data);
                let (world, level) = (cx.s.l(WORLD), cx.s.l(LEVEL));
                let e = at(0x3fea)
                    + (world.wrapping_sub(1) << 4 & 0xfffe)
                    + (level.wrapping_sub(1) * 4 & 0xfffe);
                let x = cx.w(e).wrapping_sub(0x10);
                let y = cx.w(e + 2).wrapping_sub(0x2e);
                let (a, b) = (at(0x40e2), at(0x40fa));
                cx.s.set_w(a, x);
                cx.s.set_w(a + 2, y);
                cx.s.set_w(b, x.wrapping_add(0x10));
                cx.s.set_w(b + 2, y);
                for r in [a, b] {
                    sprite::attach(cx.s, r);
                }
                for r in [a, b] {
                    sprite::update(cx.s, r);
                }
                Yield::Fade(RECORDS[0])
            }
            4 => fade(
                FadeKind::In { target: BUFFER_0 + 0xc800 },
                0,
                MAP_PALETTE,
                0x20,
                at(0x3e86),
            ),
            5 => Yield::Raster,
            _ => {
                cx.s.set_l(MAP_SHOWN, 1);
                cx.cop1lc = MAP_COPPER;
                cx.s.set_w(DISPLAY_TIMER, 0x12c);
                Yield::Return
            }
        }
    }
}

/// Fire held, or `display_timer` run out (`btst #7,$bfe001; beq` / `cmpi.w #1; bpl`): the wait
/// the screens between levels end on.
fn screen_over(cx: &Cx) -> bool {
    cx.fire() || (cx.s.w(DISPLAY_TIMER) as i16) < 1
}

/// `progress_map_end` (h0+$3eaa), from `overlays_end`: the map's time out (or fire), faded out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct MapEnd {
    pc: u8,
}

impl MapEnd {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        match self.pc {
            0 => {
                if !screen_over(cx) {
                    return Yield::Poll;
                }
                cx.s.set_l(MAP_SHOWN, 0);
                self.pc = 1;
                Yield::Fade(RECORDS[0])
            }
            1 => {
                self.pc = 2;
                fade(FadeKind::Out, 0, MAP_PALETTE, 0x20, at(0x3f04))
            }
            2 => {
                self.pc = 3;
                Yield::Fade(RECORDS[0])
            }
            3 => {
                self.pc = 4;
                Yield::Raster
            }
            _ => {
                cx.cop1lc = BLANK_COPPER;
                Yield::Return
            }
        }
    }
}

/// `world_intro_end` (h0+$4220), from `overlays_end` once the loading picture has been shown: the
/// picture's time out (or fire), the walk stopped, faded out, the sprites off.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct IntroEnd {
    pc: u8,
}

impl IntroEnd {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        match self.pc {
            0 => {
                // The original's loads run under the picture for many frames, so its walk has
                // begun (holding `display_timer` up) before this looks; after the engine's
                // instant load it waits until the walk first sets the timer (W2's walk in from
                // the left sets none until h0+$1213e).
                let unstarted =
                    cx.s.l(WALK) != at(0x7d4) && (cx.s.w(DISPLAY_TIMER) as i16) < 1;
                if !screen_over(cx) || (unstarted && !cx.fire()) {
                    return Yield::Poll;
                }
                cx.s.set_w(EXIT_SPRITES, 0);
                self.pc = 1;
                Yield::Raster
            }
            1 => {
                self.pc = 2;
                Yield::Fade(RECORDS[0])
            }
            2 => {
                self.pc = 3;
                fade(FadeKind::Out, 0, INTRO_PALETTE, 0x20, at(0x4280))
            }
            3 => {
                self.pc = 4;
                Yield::Fade(RECORDS[0])
            }
            4 => {
                self.pc = 5;
                Yield::Raster
            }
            _ => {
                cx.cop1lc = BLANK_COPPER;
                cx.wr(DMACON, 0x20);
                Yield::Return
            }
        }
    }
}

/// `end_world_pic_wN` (h0+$42aa …): `LnET` loaded and faded in, shown for 600 frames
/// (`display_timer`); taken down by h0+$43e0 ([`EndPicEnd`]) at the next `overlays_end` or the
/// next world's loading picture. World 2's waits for it here.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct EndPic {
    world: u32,
    wait: bool,
    pc: u8,
}

impl EndPic {
    fn new(world: u32, wait: bool) -> EndPic {
        EndPic { world, wait, pc: 0 }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        self.pc += 1;
        match self.pc {
            1 => {
                if cx.s.w(GAME_FULL_MODE) == 0 || cx.s.l(END_PIC_SHOWN) != 0 {
                    self.pc = 5;
                    return Yield::Return;
                }
                cx.s.set_l(END_PIC_SHOWN, 1);
                let pic = cx.data.file(&format!("L{}ET", self.world)).to_vec();
                cx.s.write(BUFFER_1, &pic);
                cx.loaded(LoadEnd::Left(at(0x436a)));
                set_planes(cx, END_PLANES, BUFFER_1, 0x2800);
                Yield::Fade(RECORDS[0])
            }
            2 => fade(
                FadeKind::In { target: BUFFER_1 + 0xc800 },
                0,
                END_PALETTE,
                0x20,
                at(0x43c2),
            ),
            3 => Yield::Raster,
            4 => {
                cx.cop1lc = END_COPPER;
                cx.s.set_w(DISPLAY_TIMER, 0x258);
                if self.wait && !screen_over(cx) {
                    return Yield::Poll;
                }
                Yield::Return
            }
            _ => {
                if self.wait && !screen_over(cx) {
                    self.pc = 4;
                    return Yield::Poll;
                }
                Yield::Return
            }
        }
    }
}

/// h0+$43e0, from `overlays_end` or the next world's `world_intro_show`: the end-of-world
/// picture's time out (or fire), faded out.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct EndPicEnd {
    pc: u8,
}

impl EndPicEnd {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        match self.pc {
            0 => {
                if !screen_over(cx) {
                    return Yield::Poll;
                }
                cx.s.set_l(END_PIC_SHOWN, 0);
                self.pc = 1;
                Yield::Fade(RECORDS[0])
            }
            1 => {
                self.pc = 2;
                fade(FadeKind::Out, 0, END_PALETTE, 0x20, at(0x4438))
            }
            2 => {
                self.pc = 3;
                Yield::Fade(RECORDS[0])
            }
            3 => {
                self.pc = 4;
                Yield::Raster
            }
            _ => {
                cx.cop1lc = BLANK_COPPER;
                Yield::Return
            }
        }
    }
}
