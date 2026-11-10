//! The game: data read from the disks, a start, and a step a pass (DESIGN §3.3). `Game::step` is a
//! function of the data, the start and the inputs; `Game: Clone` is the planner's snapshot.

use std::sync::Arc;

use amiga_input::{FrameInput, Joy};
use fd_assets::{AssetError, Files, Level, read_level};

use crate::character::{AnimTables, Character};
use crate::game_over::{self, GameOver};
use crate::level::collision::{self, CollisionPlane};
use crate::level::redraw::{Blit, LevelImage, PlayIo};
use crate::level::{self, Dispatch, DrawList, Event, LevelCtx, Object};
use crate::level_end::{self, Complete, LevelEnd};
use crate::program::{HUNK_BASES, PROGRAM_FILE, Program, Scratch};
use crate::render::status::Shown;
use crate::render::{self, Display, DisplayData};
use crate::{Mem, Scene, State, Transplant, Var};
use amiga_chips::Picture;

/// What the game reads from the disks, at run time: the copyrighted data never enters the repo
/// (DESIGN §3.1). A task needing a table of the main program reads it from [`GameData::program`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameData {
    /// World 1's level files.
    pub level: Level,
    /// The main program's hunk 0, relocated as the oracle loads it ([`HUNK_BASES`]), so a pointer
    /// copied from a table equals the original's (T08).
    pub program: Program,
    /// The animation frame tables in it (T51).
    pub anim: AnimTables,
    /// [`TITLE_SONG`], the title's music, which `load_list_title` reads to `h3+$f000`: at a level's
    /// start the collision plane's slack still holds it (T65, `level::start::lay_title_song`). Empty
    /// from [`GameData::new`].
    pub title_song: Vec<u8>,
    /// What the display reads from the disks (T10); empty for data built without the program's
    /// other hunks, which then draws black.
    pub display: DisplayData,
}

/// The title's music module on disk 2, read unpacked (`load_file_raw`).
pub const TITLE_SONG: &str = "_BALLA";

impl GameData {
    /// Read everything from `files`, which hands over a file by its name, unpacked: the level
    /// files of disk 2, and the main program [`PROGRAM_FILE`] of disk 1. A file missing or
    /// malformed is an error naming it.
    pub fn read(files: &Files) -> Result<GameData, AssetError> {
        let level = read_level(1, files)?;
        let exe = files(PROGRAM_FILE)?;
        let program = Program::hunk0(fd_assets::loaded_hunk(&exe, PROGRAM_FILE, 0, &HUNK_BASES)?);
        let title_song = files(TITLE_SONG)?;
        let display = DisplayData::read(files, &level)?;
        Ok(GameData {
            title_song,
            display,
            ..GameData::new(level, program)?
        })
    }

    /// From the level and hunk 0 as loaded; an error if hunk 0 is too short for the tables.
    pub fn new(level: Level, program: Program) -> Result<GameData, AssetError> {
        let anim = AnimTables::from_hunk0(program.bytes()).map_err(|e| {
            AssetError::new(PROGRAM_FILE, fd_assets::Why::Malformed { at: 0, what: e })
        })?;
        Ok(GameData {
            level,
            program,
            anim,
            title_song: Vec::new(),
            display: DisplayData::default(),
        })
    }
}

/// Where a game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Start {
    /// `W1-1` at the level sync point (DESIGN §2.3).
    Level1,
}

/// A BCD word's value.
fn bcd(w: u16) -> u16 {
    (0..4).rev().fold(0, |v, k| v * 10 + (w >> (4 * k) & 0xf))
}

/// The whole game.
#[derive(Debug, Clone)]
pub struct Game {
    data: Arc<GameData>,
    scene: Scene,
    state: State,
    objects: Vec<Object>,
    dispatch: Dispatch,
    events: Vec<Event>,
    /// The collision plane: `level_init`'s first view (T65) at a new game, then the edge redraws
    /// (T55) as the view scrolls; a case takes the original's ([`Transplant`]).
    plane: CollisionPlane,
    /// The level buffers the edge redraws read, built once from `data`.
    image: Arc<LevelImage>,
    /// The blits the last step's edge redraws started (T55), applied to the play screen; at a new
    /// game, the first view's 368 tiles (T65), already applied.
    blits: Vec<Blit>,
    /// What the display shows (T10): picture state, left out of equality.
    display: Display,
    /// The numbers pass `32k + 1` left, from that pass until the next redraw: what `main_loop`'s
    /// redraw after pass `32k` reads when that pass cuts it short ([`Game::cut_redraw`], T71).
    /// Picture state, left out of equality.
    late: Option<Shown>,
    /// `Some` once a conformance case hands the engine where the original's next pass cut
    /// `main_loop`'s redraw half ([`Game::cut_foreground`], T74): the cut for the next step's
    /// foreground. `None` alone, where the engine's own rule decides (`level::Cut::Rows`). Left out
    /// of equality.
    foreground_cut: Option<Option<level::EdgesCut>>,
}

impl Game {
    pub fn new(data: Arc<GameData>, start: Start) -> Game {
        match start {
            Start::Level1 => {
                let mut state = State::level_1();
                state.object_map = data.level.objects.cells.clone();
                let mut plane = CollisionPlane::new(&data.program);
                let image = Arc::new(LevelImage::new(&data.level, &data.program));
                let mut blits = Vec::new();
                level::start::lay_title_song(&mut plane, &data.title_song);
                // `level_init` (T65): the first view's tiles and shapes, before the first pass.
                level::start::first_view(&mut PlayIo {
                    image: &image,
                    objects: &mut state.object_map,
                    collision: &mut plane,
                    blits: &mut blits,
                    defer_from: None,
                    late_from: None,
                    written: 0,
                    late: Vec::new(),
                });
                // `view_tile` copies by CPU what `h0_774` would blit (docs/12), so the play screen
                // takes the first view's tiles as it takes an edge redraw's.
                let mut display = Display::level_1(&data.display, &data.program);
                for b in &blits {
                    display.screen.apply(b, &|a| data.display.byte(a));
                }
                // The sync point's frame ends inside pass 1, at `char_update`'s start: it has
                // written `char_update_t = 3` and run none of its four sub-steps (measured, T69;
                // docs/12). `char_update` is `game_frame`'s last call, so running it here leaves the
                // state as pass 1 leaves it, which is what a step's state is: the original at the
                // first pass end, where a case transplants.
                Character {
                    state: &mut state,
                    program: &data.program,
                    anim: &data.anim,
                    plane: &plane,
                    scratch: &mut Scratch::default(),
                }
                .update();
                Game {
                    display,
                    plane,
                    image,
                    data,
                    scene: Scene::Level,
                    state,
                    objects: Vec::new(),
                    dispatch: Dispatch::level_1(),
                    events: Vec::new(),
                    blits,
                    late: None,
                    foreground_cut: None,
                }
            }
        }
    }

    /// One pass: first the foreground's redraws for the edges the last pass latched
    /// ([`Game::foreground`]), then the original's level-3 handler counts and runs `game_frame`.
    /// A state is the original's as a pass leaves it, before the foreground has run (measured, T55:
    /// `h0+$1e66` lags the latches by one pass), so the foreground of the frame before comes first.
    /// Events it raises are kept until [`Game::take_events`].
    ///
    /// When `main_loop`'s check finds the game over (`game_over::game_is_over`), that step enters
    /// the game over screen instead and runs no pass ([`Event::GameOver`]); each later step is a
    /// frame of the screen, and the one that ends it restarts the level as [`Game::new`] starts it,
    /// skipping the original's high scores (DESIGN §2.4, [`Event::Restart`]).
    ///
    /// When the check finds the level's end instead (`level_end::level_is_over`), that step runs
    /// `next_world` and no pass ([`Event::LevelEnd`], [`Scene::LevelEnd`]); from the next, "Level 1
    /// complete" shows until fire restarts the level as the game over screen's end does (T26,
    /// DESIGN §2.5).
    pub fn step(&mut self, i: FrameInput) {
        self.blits.clear();
        // A step that cuts no creature leaves none for `creatures_uncut`.
        self.display.cuts.clear();
        // `level2_handler` takes key events whatever the scene.
        self.state.keys.feed(i.keys);
        match self.scene {
            Scene::Level if game_over::game_is_over(&self.state) => {
                // `main_loop`'s check comes before its status bar and its edge redraws.
                self.scene = Scene::GameOver(GameOver::default());
                self.events.push(Event::GameOver);
            }
            Scene::Level if level_end::level_is_over(&self.state) => {
                // `next_world`, then `h0+$500`: the level copper list's colours cleared.
                level_end::next_world(&mut self.state);
                self.display.fade.now = [0; 32];
                self.scene = Scene::LevelEnd(LevelEnd {
                    world: self.state.world_index,
                });
                self.events.push(Event::LevelEnd);
            }
            Scene::Level => {
                // CLXDAT gathers the frame shown since the last pass, which is the picture the last
                // step left: taken before the foreground, whose edge redraws land outside the view.
                let clxdat = self.contact();
                self.foreground();
                // `level3_handler`: `addi.w #1, irq_count`, then `game_frame` while the game runs.
                self.state.irq_count = self.state.irq_count.wrapping_add(1);
                self.game_frame_on(i, clxdat, &mut level::InPlay);
                // `level3_handler` after `game_frame`: `move.w #$ffff, frame_flag`.
                self.state.frame_flag = 0xffff;
                match self.state.irq_count & 0x1f {
                    1 => self.late = Some(self.numbers()),
                    0 => self.late = None,
                    _ => {}
                }
            }
            Scene::GameOver(mut screen) => {
                if screen.step(i.joy.contains(Joy::FIRE)) {
                    self.restart();
                } else {
                    self.scene = Scene::GameOver(screen);
                }
            }
            Scene::LevelEnd(_) | Scene::Complete(_) => {
                let mut message = match self.scene {
                    Scene::Complete(c) => c,
                    _ => Complete::default(),
                };
                if message.step(i.joy.contains(Joy::FIRE)) {
                    self.restart();
                } else {
                    self.scene = Scene::Complete(message);
                }
            }
        }
    }

    /// `game_frame` alone on this game's state, with `clxdat` as `char_hit_check` reads it and what
    /// lies outside the typed state through `pass` (`level::game_frame_with`): no foreground, no
    /// level-3 handler count. [`Game::step`] runs it with `level::InPlay`; the routine rig runs it
    /// against the original's `game_frame` (T63).
    pub fn game_frame_on(&mut self, i: FrameInput, clxdat: u16, pass: &mut dyn level::Pass) {
        let mut ctx = LevelCtx {
            state: &mut self.state,
            level: &self.data.level,
            program: &self.data.program,
            anim: &self.data.anim,
            plane: &mut self.plane,
            image: &self.image,
            display: &mut self.display,
            display_data: &self.data.display,
            clxdat,
        };
        level::game_frame_with(
            &mut ctx,
            &mut self.objects,
            &self.dispatch,
            &mut self.events,
            i,
            pass,
        );
    }

    /// Whether the level's end has been reached: `next_world` has run and "Level 1 complete" shows
    /// or is about to, until fire restarts the level.
    pub fn finished(&self) -> bool {
        matches!(self.scene, Scene::LevelEnd(_) | Scene::Complete(_))
    }

    /// `cheat_flag` on or off: what the title's cheat code sets (`title_code_world`), which the
    /// engine has no title to type in. With it on, HELP touches the exit with no items left
    /// (`help_key`), the one way to the level's end until items and the exit are ported (T22). A
    /// script and a test turn it on; a restart turns it off, as a new game starts.
    pub fn set_cheat(&mut self, on: bool) {
        self.state.keys.cheat = on as u16;
    }

    /// Where the original enters its high scores, or fire ends "Level 1 complete": the level again
    /// from its start, lives full and score zero (DESIGN §2.4, §2.5). Events not yet taken are kept.
    /// For a conformance case (`fd-conform`'s `Engine::foreground_cut`, and nothing else): where the
    /// original's pass that the next [`Game::step`] runs cut `main_loop`'s redraw half, measured on
    /// the original (`level::Cut::At`); `None` when it did not. From the first call on, the engine's
    /// own rule (`level::ROW_BOBS_CUT`) is off: the original's cuts depend on the cycles each pass
    /// took, which the engine does not model (DESIGN §2.1; T74, docs/09).
    pub fn cut_foreground(&mut self, at: Option<level::EdgesCut>) {
        self.foreground_cut = Some(at);
    }

    /// Back to the engine's own rule after [`Game::cut_foreground`]: for a forecast, on the engine
    /// alone, from the state a conformance case's engine holds (T48: the route planned from the
    /// original's states, docs/13).
    pub fn own_cuts(&mut self) {
        self.foreground_cut = None;
    }

    fn restart(&mut self) {
        let (events, cut) = (std::mem::take(&mut self.events), self.foreground_cut);
        *self = Game::new(self.data.clone(), Start::Level1);
        self.events = events;
        self.foreground_cut = cut;
        self.events.push(Event::Restart);
    }

    /// The lives left, as a number (`lives` is BCD).
    pub fn lives(&self) -> i16 {
        bcd(self.state.lives) as i16
    }

    /// The countdown, as a number: the energy the status bar shows, which ends the game at 0
    /// (`countdown`'s first word, BCD).
    pub fn energy(&self) -> u16 {
        bcd((self.state.countdown >> 16) as u16)
    }

    /// What the foreground does between passes: `main_loop`'s status bar every 32 interrupts (T10),
    /// then its edge redraws (T55), whose blits are added to [`Game::blits`] and drawn into the play
    /// screen. Nothing on the game over screen.
    pub fn foreground(&mut self) {
        match self.scene {
            Scene::Level => {
                let shown = self.display.status.shown;
                let mut status = std::mem::replace(
                    &mut self.display.status,
                    render::status::Status {
                        bytes: Vec::new(),
                        shown,
                    },
                );
                self.status_redraw(&mut status);
                self.display.status = status;
                let from = self.blits.len();
                let cut = match self.foreground_cut {
                    Some(at) => level::Cut::At(at),
                    None => level::Cut::Rows,
                };
                level::main_loop_redraws(
                    &mut self.state,
                    &mut self.plane,
                    &self.image,
                    &mut self.blits,
                    cut,
                );
                let data = &self.data.display;
                for b in &self.blits[from..] {
                    self.display.screen.apply(b, &|a| data.byte(a));
                }
            }
            Scene::GameOver(_) | Scene::LevelEnd(_) | Scene::Complete(_) => {}
        }
    }

    /// `main_loop`'s status bar redraw (`h0_6a96`) as the foreground runs it after a pass: every
    /// 32 interrupts, the numbers that changed.
    fn status_redraw(&self, status: &mut render::status::Status) {
        let s = &self.state;
        if s.irq_count & 0x1f == 0 {
            let data = &self.data.display;
            let h4 = |o: u32| data.h4(o);
            let src = render::status::Sources {
                h4: &h4,
                program: &self.data.program,
            };
            let countdown = (s.countdown >> 16) as u16;
            status.update(countdown, s.lives, s.score, s.items_left, &src);
        }
    }

    /// The four numbers `h0_6a96` reads, as they are now.
    fn numbers(&self) -> Shown {
        let s = &self.state;
        Shown {
            countdown: (s.countdown >> 16) as u16,
            lives: s.lives,
            items: s.items_left,
            score: s.score,
        }
    }

    /// [`Game::picture`] as the original shows it when pass `32k + 1` cut `main_loop`'s redraw after
    /// pass `32k` short, `cut` of `h0_6a96`'s four reads (countdown, lives, score, items, in that
    /// order) made before it: the rest read what pass `32k + 1` left, and the bar keeps that drawing
    /// until the next redraw. `None` outside passes `32k + 1` to `32k + 31`, before a pass `32k + 1`
    /// has run, or when it draws what [`Game::picture`] draws.
    ///
    /// The engine cannot know `cut`: `main_loop` calls `h0_6a96` over and over while `irq_count & 31`
    /// is 0, and where in that loop the interrupt lands depends on the cycles every pass took, which
    /// nothing here counts (measured, T71: docs/09, "The status bar's redraw and the next pass"). A
    /// case reads it off the original and accepts this picture for the bar (`fd-conform`).
    pub fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        let late = self.late?;
        if self.scene != Scene::Level || self.state.irq_count & 0x1f == 0 {
            return None;
        }
        let old = self.display.status.shown;
        let pick = |i: u8| i >= cut;
        let mut status = self.display.status.clone();
        let data = &self.data.display;
        let h4 = |o: u32| data.h4(o);
        let src = render::status::Sources {
            h4: &h4,
            program: &self.data.program,
        };
        status.update(
            if pick(0) {
                late.countdown
            } else {
                old.countdown
            },
            if pick(1) { late.lives } else { old.lives },
            if pick(2) { late.score } else { old.score },
            if pick(3) { late.items } else { old.items },
            &src,
        );
        if status.bytes == self.display.status.bytes {
            return None;
        }
        let c = &self.state.camera;
        Some(render::picture(
            data,
            &Display {
                status,
                ..self.display.clone()
            },
            (c.x, c.y),
            &self.data.program,
        ))
    }

    /// The frame the original shows after this step, in the play area and status bar's window
    /// (`render::WINDOW`), drawn as Denise would from the display's memory (T10).
    ///
    /// The status bar is drawn as `main_loop`'s redraw after this pass leaves it: the original's
    /// foreground redraws it once the pass has ended, before the beam reaches the bar at line 241,
    /// so the frame showing pass `32k` already shows its numbers (measured, T44:
    /// `picture_cases.rs`, idle and scrolling alike). The engine runs that redraw at its next step.
    ///
    /// On the game over screen, that screen ([`render::game_over_picture`]); at the level's end,
    /// play with `h0+$500`'s black colours, then "Level 1 complete" in play's window
    /// ([`render::message::level_complete`]).
    pub fn picture(&self) -> Picture {
        if let Scene::GameOver(_) = self.scene {
            return render::game_over_picture(
                &self.data.display,
                &self.display,
                &self.data.program,
            );
        }
        let c = &self.state.camera;
        let draw =
            |d: &Display| render::picture(&self.data.display, d, (c.x, c.y), &self.data.program);
        if let Scene::Complete(_) = self.scene {
            return render::message::level_complete(&draw(&self.display));
        }
        // `main_loop` skips the status bar when its check finds the game over or the level's end.
        if self.state.irq_count & 0x1f != 0
            || game_over::game_is_over(&self.state)
            || level_end::level_is_over(&self.state)
        {
            return draw(&self.display);
        }
        let mut status = self.display.status.clone();
        self.status_redraw(&mut status);
        draw(&Display {
            status,
            ..self.display.clone()
        })
    }

    /// For a conformance case (`fd-conform`'s `Engine::creatures_uncut`, and nothing else):
    /// [`Game::picture`] as the original shows it when the beam had passed the first `rows[k]` blit
    /// rows of the `k`th creature the last pass cut before the blitter wrote them (a row is one
    /// plane of one line, from the creature's top): there the frame still shows what the cut drew
    /// over. `None` out of play, or unless `rows` has one count for each creature cut.
    ///
    /// The engine cannot know `rows`: a pass with several creatures runs past the play area's first
    /// lines, and how far depends on the cycles it took, which nothing here counts (measured, T80:
    /// docs/10, "A creature cut behind the beam"). A case reads the cuts' beam positions off the
    /// original and accepts this picture for that frame.
    pub fn creatures_uncut(&self, rows: &[u16]) -> Option<Picture> {
        if self.scene != Scene::Level {
            return None;
        }
        let data = &self.data.display;
        let screen = self
            .display
            .cuts
            .uncut(&self.display.screen, &|a| data.byte(a), rows)?;
        let mut shown = self.clone();
        shown.display.screen = screen;
        Some(shown.picture())
    }

    /// CLXDAT as the frame [`Game::picture`] draws in play sets it: the character's sprites against
    /// the playfield (`render::contact`, T70).
    pub fn contact(&self) -> u16 {
        let c = &self.state.camera;
        let (data, program) = (&self.data.display, &self.data.program);
        let clx = |d: &Display| render::contact::clxdat(data, d, (c.x, c.y), program);
        // As `picture`: the status bar `main_loop` redraws after a pass `32k` shows in its frame.
        if self.state.irq_count & 0x1f != 0 || game_over::game_is_over(&self.state) {
            return clx(&self.display);
        }
        let mut status = self.display.status.clone();
        self.status_redraw(&mut status);
        clx(&Display {
            status,
            ..self.display.clone()
        })
    }

    /// [`Game::picture`] as RGBA, 8 bits a channel, row-major: `out` holds width × height × 4 bytes.
    pub fn picture_rgba(&self, out: &mut [u8]) {
        render::rgba(&self.picture(), out);
    }

    /// What the display shows that the engine keeps.
    pub fn display(&self) -> &Display {
        &self.display
    }

    /// The blits the last step's edge redraws started, in order: tiles and bobs onto the play screen.
    /// Before the first step, `level_init`'s first view: its 23 by 16 tiles, column by column (T65),
    /// already drawn into the play screen by [`Game::new`].
    pub fn blits(&self) -> &[Blit] {
        &self.blits
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// The collision plane the character probes.
    pub fn plane(&self) -> &CollisionPlane {
        &self.plane
    }

    pub fn data(&self) -> &Arc<GameData> {
        &self.data
    }

    /// The objects in play.
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }

    /// Put an object in play; its first pass is the next step's. T22's scan calls it.
    pub fn spawn(&mut self, o: Object) {
        self.objects.push(o);
    }

    /// The events raised since the last call, oldest first. The app writes an
    /// [`Event::Unported`] to stderr; a conformance case fails on it.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// What the objects show now.
    pub fn draw_objects(&self) -> DrawList {
        let mut out = DrawList::default();
        level::draw_objects(&self.objects, &self.dispatch, &mut out);
        out
    }
}

/// Equal state: the same scene, variables, objects and pending events, on equal data. The dispatch
/// is the same code for every game, so it is not compared; nor is the display, picture state that
/// nothing in play logic reads (T00, T10).
impl PartialEq for Game {
    fn eq(&self, o: &Game) -> bool {
        (Arc::ptr_eq(&self.data, &o.data) || self.data == o.data)
            && self.scene == o.scene
            && self.state == o.state
            && self.objects == o.objects
            && self.events == o.events
            && self.plane == o.plane
    }
}

impl Eq for Game {}

impl Game {
    /// What play keeps beyond `LAYOUT`'s variables (T55): `frame_flag`, the collision plane, the object
    /// map, the creature slots and the animated cells, from the original's memory.
    fn transplant_play(&mut self, mem: &dyn Mem) {
        use crate::level::collision::BEFORE;
        use crate::level::redraw::{AnimCell, CreatureSlot, OBJECT_MAP};
        let word = |b: &[u8], i: usize| u16::from_be_bytes([b[i], b[i + 1]]);
        let s = &mut self.state;
        s.frame_flag = word(&mem.read(0, 0x519e, 2), 0);
        s.edges_resume = None;
        s.edges.deferred.clear();
        // `h3+$fa50` less BEFORE: the map, the tail the probes reach and the slack the redraws write.
        let kept = self.plane.kept_mut();
        kept.copy_from_slice(&mem.read(3, collision::MAP - BEFORE, kept.len()));
        let len = s.object_map.len().max(self.data.level.objects.cells.len());
        s.object_map = mem.read(0, OBJECT_MAP, len);
        let c = &mut s.edges.creatures;
        let active = mem.read(0, 0x67da, 5);
        let slots = mem.read(0, 0x67ec, 40);
        for i in 0..5 {
            c.active[i] = active[i];
            let b = &slots[8 * i..8 * i + 8];
            c.slots[i] = CreatureSlot {
                x: word(b, 0),
                y: word(b, 2),
                kind: word(b, 4),
                facing: b[6],
                phase: b[7],
            };
        }
        c.current = word(&mem.read(0, 0x581c, 2), 0);
        // The creature drawing's (T52): `h0+$2c7a`, `$2c7c`, `$5818`..`$581f`, the flag sets.
        let d = &mut s.creature_draw;
        d.restore_index = word(&mem.read(0, 0x2c7a, 2), 0);
        d.cut_index = word(&mem.read(0, 0x2c7c, 2), 0);
        let at = mem.read(0, 0x5818, 8);
        (d.x, d.y, d.restoring) = (word(&at, 0), word(&at, 2), word(&at, 6));
        d.saved.copy_from_slice(&mem.read(0, 0x67e0, 5));
        d.drawn.copy_from_slice(&mem.read(0, 0x67e6, 5));
        let last = mem.read(0, 0x687e, 4);
        c.last = u32::from_be_bytes([last[0], last[1], last[2], last[3]]);
        // What `game_frame`'s callees keep outside `LAYOUT` (T63): each is written in a pass before
        // it is read, so a case runs the same without them, but the routine rig compares every byte.
        let long = |o: u32| {
            let b = mem.read(0, o, 4);
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        };
        // Its low word is `LAYOUT`'s `char_screen_x`, already taken.
        s.camera.screen_x = long(0x4d76) & 0xffff_0000 | s.camera.screen_x & 0xffff;
        s.camera.screen_y = long(0x4d7a);
        let crossed = mem.read(0, 0x51f8, 4);
        s.camera.edges.crossed = std::array::from_fn(|i| crossed[i] != 0);
        s.collect = crate::objects::Collect {
            score_add: long(0x2a6a),
            row: long(0x2692),
            taken: long(0x29c6),
        };
        s.moving = crate::objects::creature::Moving {
            slot: long(0x2fe6),
            stomped: word(&mem.read(0, 0x306e, 2), 0),
        };
        s.edges.creatures.index = long(0x2faa);
        let used = mem.read(2, 0x1680, 48);
        let cells = mem.read(2, 0x16b0, 384);
        for n in 0..48 {
            let b = &cells[8 * n..8 * n + 8];
            s.edges.cells.used[n / 12][n % 12] = used[n];
            s.edges.cells.cells[n / 12][n % 12] = AnimCell {
                x: word(b, 0),
                y: word(b, 2),
                code: b[4],
                rest: [b[5], b[6], b[7]],
            };
        }
    }
}

impl Game {
    /// The display's memory from the original's (T10), so a picture case starts from its screen: the
    /// play screen, the status bar and what it last drew, the fade and the copper list's colours, the
    /// sprites' control words. The sprites' frame is the one `char_anim_ptr` names unless their data
    /// is clear; the next pass copies it again in any case.
    fn transplant_display(&mut self, mem: &dyn Mem) {
        use crate::render::{COLOURS, objects, playfield, status};
        let long = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
        let word = |b: &[u8], i: usize| u16::from_be_bytes([b[i], b[i + 1]]);
        let d = &mut self.display;
        d.screen.bytes = mem.read(3, playfield::SCREEN, playfield::SCREEN_BYTES as usize);
        d.status.bytes = mem.read(4, status::STATUS, status::STATUS_BYTES as usize);
        let shown = mem.read(0, 0x6a8c, 10);
        d.status.shown = status::Shown {
            countdown: word(&shown, 0),
            lives: word(&shown, 2),
            items: word(&shown, 4),
            score: long(&shown[6..]),
        };
        d.fade.on = mem.read(0, 0x94b4, 1)[0];
        d.fade.pass = long(&mem.read(0, 0x94b6, 4));
        let scratch = mem.read(0, 0x961e, 6);
        d.fade.scratch = std::array::from_fn(|i| word(&scratch, 2 * i));
        let colours = mem.read(1, COLOURS, 128);
        d.fade.now = std::array::from_fn(|i| word(&colours, 4 * i));
        d.sprite.ctl = long(&mem.read(1, objects::SPRITES, 4));
        let data = mem.read(1, objects::SPRITES + 4, objects::SPRITE_DATA as usize);
        d.sprite.frame = data.iter().any(|&b| b != 0).then_some(self.state.anim.ptr);
    }
}

impl Transplant for Game {
    /// Every `LAYOUT` variable from the original, and the collision plane (`h3+$fa50` and the
    /// tail a probe reaches past it). Objects are not transplanted until T21 measures where the
    /// original keeps them.
    fn transplant(&mut self, mem: &dyn Mem) {
        for v in crate::LAYOUT {
            let known = self.state.set(v.name, mem.value(v));
            debug_assert!(known, "{} is in LAYOUT but not in State", v.name);
        }
        self.transplant_play(mem);
        self.transplant_display(mem);
        self.late = None;
    }

    fn probe(&self, v: &Var) -> u32 {
        self.state
            .get(v.name)
            .unwrap_or_else(|| panic!("{} is in LAYOUT but not in State", v.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::{ObjectKind, register};
    use crate::program::Program;
    use crate::{LAYOUT, Width};
    use amiga_input::Joy;
    use fd_assets::{CollisionMap, LEVEL_ROWS, ObjectMap, TileMap};

    /// A one-column level: enough for the state and the dispatch, which read no tiles yet.
    fn data() -> Arc<GameData> {
        let cells = LEVEL_ROWS as usize;
        Arc::new(
            GameData::new(
                Level {
                    tiles: TileMap {
                        columns: 1,
                        cells: vec![0; cells],
                    },
                    gens: vec![[0; fd_assets::TILE_BYTES]],
                    collision: CollisionMap {
                        columns: 1,
                        cells: vec![0; cells],
                    },
                    shapes: vec![[0; 16]],
                    objects: ObjectMap {
                        columns: 1,
                        cells: vec![0; cells],
                    },
                },
                // Every table 0: the character stands on nothing and falls.
                Program::hunk0(vec![0; 0x3_7f08]),
            )
            .unwrap(),
        )
    }

    fn inputs(n: usize) -> impl Iterator<Item = FrameInput> {
        let pattern = [
            FrameInput::NONE,
            FrameInput::joy(Joy::RIGHT),
            FrameInput::joy(Joy::UP | Joy::FIRE),
            FrameInput::joy(Joy::LEFT),
        ];
        (0..n).map(move |k| pattern[k * 7 % pattern.len()])
    }

    /// Moves right one pixel a pass, and leaves after `x` reaches 40.
    struct Walker;

    impl ObjectKind for Walker {
        fn update(&self, o: &mut Object, l: &mut LevelCtx) {
            o.x += 1;
            o.removed = o.x >= 40;
            l.state.score += 1;
        }
        fn draw(&self, o: &Object, out: &mut DrawList) {
            out.bobs.push(level::Bob {
                x: o.x,
                y: o.y,
                image: 7,
            });
        }
    }

    #[test]
    fn two_new_games_are_equal_and_start_at_pass_1s_end() {
        let d = data();
        let (a, b) = (
            Game::new(d.clone(), Start::Level1),
            Game::new(d, Start::Level1),
        );
        assert_eq!(a, b);
        assert_eq!(*a.scene(), Scene::Level);
        // The sync point's state with pass 1's `char_update` run (T69): nothing below the
        // character in this data, so its four sub-steps start the fall and step it.
        let mut want = State {
            object_map: a.data().level.objects.cells.clone(),
            ..State::level_1()
        };
        want.motion.falling = 1;
        want.motion.jump_t = 4;
        want.motion.update_t = 0;
        want.anim = a.state().anim;
        assert_eq!(*a.state(), want);
        // Equal data read twice compares equal too.
        assert_eq!(a, Game::new(data(), Start::Level1));
    }

    #[test]
    fn a_new_game_lists_the_first_views_tiles_until_its_first_step() {
        let mut g = Game::new(data(), Start::Level1);
        assert_eq!(g.blits().len(), 23 * 16);
        assert!(g.blits().iter().all(|b| matches!(b, Blit::Tile { .. })));
        g.step(FrameInput::NONE);
        assert!(g.blits().len() < 23 * 16);
    }

    #[test]
    fn clones_step_identically() {
        let mut a = Game::new(data(), Start::Level1);
        a.dispatch = Dispatch::empty();
        register(&mut a.dispatch, 3, &Walker);
        a.spawn(Object::at_cell(3, 0, 2));
        for i in inputs(10) {
            a.step(i);
        }
        let mut b = a.clone();
        assert_eq!(a, b);
        for i in inputs(500) {
            a.step(i);
            b.step(i);
            assert_eq!(a, b);
        }
        assert_eq!(a.state().irq_count, 511);
    }

    #[test]
    fn a_step_counts_the_pass_runs_the_camera_reads_the_joystick_and_runs_the_character() {
        let mut g = Game::new(data(), Start::Level1);
        g.step(FrameInput::joy(Joy::RIGHT));
        let s = g.state();
        assert_eq!(s.irq_count, 2);
        // main_loop's redraws (T55) run first, for the two edges the first pass latched: down (the
        // row below the view, 23 tiles) and left (x 0: nothing); their latches clear.
        assert_eq!(s.edges.pos, (0, 0x17));
        assert_eq!(g.blits().len(), 23);
        let s = g.state();
        assert_eq!(s.irq_count, 2);
        // camera_update (T09): the character is at rest in the dead zone and the camera stays, but at
        // x 0 its left scroll crosses the cell edge again and latches left anew, as the original does
        // every pass there (measured, T55); it notes the character's distance, before the character
        // moves.
        let want = State::level_1().camera;
        assert_eq!((s.camera.x, s.camera.y), (want.x, want.y));
        assert_eq!(s.camera.edges.crossed, [false, true, false, false]);
        assert_eq!(s.camera.edges.latch, [0, 0xffff, 0, 0]);
        assert_eq!(s.camera.screen_y, 0x70 - 0x17);
        assert_eq!(s.frame_flag, 0xffff);
        // read_joystick (T51): right held, one more byte in the log.
        assert_eq!((s.joy.right, s.input_log.index), (1, 2));
        // char_update (T08): the plane holds only that row below the view, which the probes at the
        // character's feet do not reach, so `char_probe_1` is 0, so four sub-steps fall; the fall
        // started in pass 1 (`Game::new`, T69), so each steps it down `h0+$44d4` (all 0 here) and
        // steers right a pixel.
        assert_eq!(s.motion.falling, 1);
        assert_eq!((s.motion.update_t, s.motion.jump_t), (0, 8));
        assert_eq!((s.char_x, s.char_y), (0x70 + 4, 0x70));
        // The count wraps as the original's word does.
        g.state.irq_count = 0xffff;
        g.step(FrameInput::NONE);
        assert_eq!(g.state().irq_count, 0);
    }

    #[test]
    fn paused_runs_the_camera_and_nothing_after_it() {
        // game_frame runs char_sprite_copy, the fade, camera_update and char_sprite_position before
        // its `paused` test (h0+$200a); the joystick, the objects and the character wait.
        let mut g = Game::new(data(), Start::Level1);
        g.state.paused = 1;
        let start = g.state().clone();
        g.step(FrameInput::joy(Joy::RIGHT));
        let mut want = State {
            irq_count: 2,
            ..start
        };
        want.camera.edges.crossed = [false, true, false, false];
        want.camera.edges.latch = [0, 0xffff, 0, 0];
        want.camera.screen_y = 0x70 - 0x17;
        // char_sprite_copy runs before the test too: the blink counter steps.
        want.motion.unhurt_t = 0x9d;
        want.edges = g.state().edges.clone();
        want.object_map = g.state().object_map.clone();
        assert_eq!(*g.state(), want);
    }

    #[test]
    fn the_game_over_check_shows_the_screen_and_fire_restarts_the_level() {
        let mut g = Game::new(data(), Start::Level1);
        // Esc pressed, then let go: `play_keys` gives up at the pass's end (T23).
        g.step(FrameInput::key(0x45));
        assert_eq!(g.state().keys.esc_down, 1);
        g.step(FrameInput::NONE);
        assert_eq!(g.state().give_up, 1);
        // Passes run on until `main_loop`'s check after pass 32; the step after shows the screen
        // and runs no pass.
        while g.state().irq_count != 32 {
            assert_eq!(*g.scene(), Scene::Level);
            g.step(FrameInput::NONE);
        }
        assert!(g.take_events().is_empty());
        g.state.score = 0x1234;
        let before = g.state().clone();
        g.step(FrameInput::NONE);
        assert_eq!(g.take_events(), [Event::GameOver]);
        assert!(matches!(g.scene(), Scene::GameOver(_)));
        assert_eq!(*g.state(), before, "no pass ran");
        // Fire before the wait's end is ignored; pressed after it, its release restarts.
        for _ in 0..20 {
            g.step(FrameInput::joy(Joy::FIRE));
        }
        g.step(FrameInput::NONE);
        assert!(matches!(g.scene(), Scene::GameOver(_)));
        g.step(FrameInput::joy(Joy::FIRE));
        assert!(g.take_events().is_empty());
        g.step(FrameInput::NONE);
        assert_eq!(g.take_events(), [Event::Restart]);
        let fresh = Game::new(data(), Start::Level1);
        assert_eq!(
            g, fresh,
            "the level as at its start: lives full, score zero"
        );
        assert_eq!((g.lives(), g.energy(), g.state().score), (32, 999, 0));
    }

    #[test]
    fn help_with_the_cheat_ends_the_level_and_fire_on_the_message_restarts_it() {
        let mut g = Game::new(data(), Start::Level1);
        g.set_cheat(true);
        while g.state().irq_count != 31 {
            assert_eq!(*g.scene(), Scene::Level);
            assert!(!g.finished());
            g.step(FrameInput::NONE);
        }
        // HELP read by pass 32: `help_key` at the pass's end touches the exit, no items left (T23's
        // port). Read by any earlier pass, the next one's `char_object_cells` clears the touch (T22).
        g.step(FrameInput::key(0x5f));
        assert_eq!((g.state().level_exit_touched, g.state().items_left), (1, 0));
        assert_eq!(g.state().irq_count, 32);
        assert!(g.take_events().is_empty());
        g.state.score = 0x1234;
        let before = g.state().clone();
        let play = g.picture();
        // `main_loop`'s check after pass 32: `next_world`, no pass. HELP stays down so the keyboard
        // state, which a step takes before the check, is unchanged.
        g.step(FrameInput {
            joy: Joy::FIRE,
            ..FrameInput::key(0x5f)
        });
        assert_eq!(g.take_events(), [Event::LevelEnd]);
        assert_eq!(*g.scene(), Scene::LevelEnd(LevelEnd { world: 1 }));
        assert!(g.finished());
        assert_eq!(
            *g.state(),
            State {
                world_index: 1,
                ..before
            },
            "no pass ran"
        );
        assert_eq!(g.display().fade.now, [0; 32], "h0+$500's colours");
        // The message: every input but a fresh fire press is ignored, fire held since the end too.
        let others = [
            FrameInput::joy(Joy::FIRE),
            FrameInput::joy(Joy::RIGHT | Joy::UP),
            FrameInput::key(0x45),
            FrameInput::key(0x5f),
            FrameInput::NONE,
            FrameInput::joy(Joy::LEFT),
        ];
        for i in others {
            g.step(i);
            assert!(matches!(g.scene(), Scene::Complete(_)), "{i:?}");
            assert_eq!(g.state().world_index, 1);
        }
        assert!(g.take_events().is_empty());
        let message = g.picture();
        assert_eq!(
            (message.width, message.height, message.left, message.top),
            (play.width, play.height, play.left, play.top)
        );
        g.step(FrameInput::joy(Joy::FIRE | Joy::LEFT));
        assert_eq!(g.take_events(), [Event::Restart]);
        assert_eq!(
            g,
            Game::new(data(), Start::Level1),
            "lives full, score zero"
        );
        assert!(!g.finished());
        assert_eq!(g.state().keys.cheat, 0, "a new game, as at the start");
    }

    #[test]
    fn the_exit_with_items_left_or_off_the_check_does_not_end_the_level() {
        let mut g = Game::new(data(), Start::Level1);
        g.state.level_exit_touched = 1;
        for _ in 0..100 {
            g.step(FrameInput::NONE);
        }
        assert_eq!(*g.scene(), Scene::Level, "items left");
        g.state.items_left = 0;
        g.state.irq_count = 0x40;
        g.state.give_up = 1;
        g.step(FrameInput::NONE);
        assert!(matches!(g.scene(), Scene::GameOver(_)), "game over first");
    }

    #[test]
    fn the_game_over_screen_draws_its_own_picture() {
        let mut g = Game::new(data(), Start::Level1);
        let play = g.picture();
        g.scene = Scene::GameOver(GameOver::default());
        let over = g.picture();
        assert_eq!((over.width, over.height), (play.width, play.height));
        // Equal to the oracle's frame on every pixel: `fd-conform/tests/death.rs`.
    }

    #[test]
    fn an_unregistered_value_raises_unported_once_and_is_left_out() {
        let mut g = Game::new(data(), Start::Level1);
        g.spawn(Object::at_cell(0x42, 0, 5));
        g.step(FrameInput::NONE);
        assert_eq!(g.take_events(), vec![Event::Unported(0x42)]);
        assert!(g.objects().is_empty());
        g.step(FrameInput::NONE);
        assert_eq!(g.take_events(), vec![]);
    }

    #[test]
    fn a_registered_kind_updates_draws_and_removes_its_object() {
        let mut g = Game::new(data(), Start::Level1);
        register(&mut g.dispatch, 3, &Walker);
        g.spawn(Object::at_cell(3, 2, 1));
        g.step(FrameInput::NONE);
        assert_eq!(g.objects()[0].x, 33);
        assert_eq!(g.state().score, 1);
        assert_eq!(
            g.draw_objects().bobs,
            vec![level::Bob {
                x: 33,
                y: 16,
                image: 7
            }]
        );
        for _ in 0..7 {
            g.step(FrameInput::NONE);
        }
        assert!(g.objects().is_empty(), "removed at x 40");
        assert!(g.take_events().is_empty());
    }

    #[test]
    #[should_panic(expected = "registered twice")]
    fn one_value_cannot_have_two_kinds() {
        let mut d = Dispatch::empty();
        register(&mut d, 9, &Walker);
        register(&mut d, 9, &Walker);
    }

    #[test]
    fn level_1_registers_no_kind_yet() {
        // T22, T24, T25 change this as they register.
        assert_eq!(Dispatch::level_1().values(), Vec::<u8>::new());
    }

    struct Bytes;

    impl Mem for Bytes {
        /// Byte `k` of every variable is `k + 1`, whatever its address.
        fn read(&self, _hunk: u8, _offset: u32, len: usize) -> Vec<u8> {
            (1..=len).map(|k| k as u8).collect()
        }
    }

    #[test]
    fn transplant_takes_every_layout_variable_and_probe_gives_it_back() {
        let mut g = Game::new(data(), Start::Level1);
        g.transplant(&Bytes);
        for v in LAYOUT {
            let want = match v.width {
                Width::Byte => 0x01,
                Width::Word => 0x0102,
                Width::Long => 0x0102_0304,
            };
            // A creature slot's field (T46) comes from the slots' own read, the whole symbol at
            // once, so its bytes are those at its offset into it.
            let want = match v.name.split_once("+$") {
                Some((_, off)) => {
                    let off = u32::from_str_radix(off, 16).unwrap();
                    (1..=v.width.bytes() as u32).fold(0, |a, k| a << 8 | (off + k))
                }
                None => want,
            };
            assert_eq!(g.probe(v), want, "{}", v.name);
        }
        // The plane is read from `collision::BEFORE` bytes before the map: its first long is bytes
        // $41..$44.
        assert_eq!(g.plane().window(0, 0), 0x4142);
    }

    #[test]
    fn missing_or_malformed_data_is_an_error_naming_the_file() {
        let none = |name: &str| Err(AssetError::missing(name, "no such file"));
        let e = GameData::read(&none).unwrap_err();
        assert!(e.to_string().starts_with("_Gen1-1: no such file"), "{e}");
        let short = |_: &str| Ok(vec![0u8; 3]);
        let e = GameData::read(&short).unwrap_err();
        assert!(e.to_string().contains("_Gen1-1"), "{e}");
    }
}
