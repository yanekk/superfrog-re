//! The game: data read from the disks, a start, and a step a pass (DESIGN §3.3). `Game::step` is a
//! function of the data, the start and the inputs; `Game: Clone` is the planner's snapshot.

use std::sync::Arc;

use amiga_input::FrameInput;
use fd_assets::{AssetError, Files, Level, read_level};

use crate::character::AnimTables;
use crate::level::collision::{self, CollisionPlane};
use crate::level::redraw::{Blit, LevelImage};
use crate::level::{self, Dispatch, DrawList, Event, LevelCtx, Object};
use crate::program::{HUNK_BASES, PROGRAM_FILE, Program};
use crate::{Mem, Scene, State, Transplant, Var};

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
}

impl GameData {
    /// Read everything from `files`, which hands over a file by its name, unpacked: the level
    /// files of disk 2, and the main program [`PROGRAM_FILE`] of disk 1. A file missing or
    /// malformed is an error naming it.
    pub fn read(files: &Files) -> Result<GameData, AssetError> {
        let level = read_level(1, files)?;
        let exe = files(PROGRAM_FILE)?;
        let program = Program::hunk0(fd_assets::loaded_hunk(&exe, PROGRAM_FILE, 0, &HUNK_BASES)?);
        GameData::new(level, program)
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
        })
    }
}

/// Where a game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Start {
    /// `W1-1` at the level sync point (DESIGN §2.3).
    Level1,
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
    /// The collision plane: the edge redraws (T55) draw it as the view scrolls; empty at a new game
    /// until T65 ports level_init's first view; a case takes the original's ([`Transplant`]).
    plane: CollisionPlane,
    /// The level buffers the edge redraws read, built once from `data`.
    image: Arc<LevelImage>,
    /// The blits the last step's edge redraws started, for the picture (T10).
    blits: Vec<Blit>,
}

impl Game {
    pub fn new(data: Arc<GameData>, start: Start) -> Game {
        match start {
            Start::Level1 => {
                let mut state = State::level_1();
                state.object_map = data.level.objects.cells.clone();
                Game {
                    plane: CollisionPlane::new(&data.program),
                    image: Arc::new(LevelImage::new(&data.level)),
                    data,
                    scene: Scene::Level,
                    state,
                    objects: Vec::new(),
                    dispatch: Dispatch::level_1(),
                    events: Vec::new(),
                    blits: Vec::new(),
                }
            }
        }
    }

    /// One pass: first the foreground's redraws for the edges the last pass latched
    /// ([`Game::foreground`]), then the original's level-3 handler counts and runs `game_frame`.
    /// A state is the original's as a pass leaves it, before the foreground has run (measured, T55:
    /// `h0+$1e66` lags the latches by one pass), so the foreground of the frame before comes first.
    /// Events it raises are kept until [`Game::take_events`].
    pub fn step(&mut self, i: FrameInput) {
        self.blits.clear();
        self.foreground();
        match self.scene {
            Scene::Level => {
                // `level3_handler`: `addi.w #1, irq_count`, then `game_frame` while the game runs.
                self.state.irq_count = self.state.irq_count.wrapping_add(1);
                let mut ctx = LevelCtx {
                    state: &mut self.state,
                    level: &self.data.level,
                    program: &self.data.program,
                    anim: &self.data.anim,
                    plane: &self.plane,
                };
                level::game_frame(
                    &mut ctx,
                    &mut self.objects,
                    &self.dispatch,
                    &mut self.events,
                    i,
                );
                // `level3_handler` after `game_frame`: `move.w #$ffff, frame_flag`.
                self.state.frame_flag = 0xffff;
            }
        }
    }

    /// `main_loop`'s edge redraws (T55): what the foreground does between passes. Their blits are
    /// added to [`Game::blits`].
    pub fn foreground(&mut self) {
        match self.scene {
            Scene::Level => level::main_loop_redraws(
                &mut self.state,
                &mut self.plane,
                &self.image,
                &mut self.blits,
            ),
        }
    }

    /// The blits the last step's edge redraws started, in order: tiles and bobs onto the play screen,
    /// for the picture (T10).
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
/// is the same code for every game, so it is not compared.
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
        let last = mem.read(0, 0x687e, 4);
        c.last = u32::from_be_bytes([last[0], last[1], last[2], last[3]]);
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
    fn two_new_games_are_equal_and_start_at_the_sync_point() {
        let d = data();
        let (a, b) = (
            Game::new(d.clone(), Start::Level1),
            Game::new(d, Start::Level1),
        );
        assert_eq!(a, b);
        assert_eq!(*a.scene(), Scene::Level);
        let want = State {
            object_map: a.data().level.objects.cells.clone(),
            ..State::level_1()
        };
        assert_eq!(*a.state(), want);
        // Equal data read twice compares equal too.
        assert_eq!(a, Game::new(data(), Start::Level1));
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
        // character's feet do not reach, so `char_probe_1` is 0, so four sub-steps fall; the
        // first starts the fall, the next three step it down `h0+$44d4` (all 0 here) and steer
        // right a pixel each.
        assert_eq!(s.motion.falling, 1);
        assert_eq!(s.motion.update_t, 0);
        assert_eq!((s.char_x, s.char_y), (0x70 + 3, 0x70));
        // The count wraps as the original's word does.
        g.state.irq_count = 0xffff;
        g.step(FrameInput::NONE);
        assert_eq!(g.state().irq_count, 0);
    }

    #[test]
    fn paused_runs_the_camera_and_nothing_after_it() {
        // game_frame runs camera_update before its `paused` test (h0+$200a); the joystick, the
        // objects and the character wait.
        let mut g = Game::new(data(), Start::Level1);
        g.state.paused = 1;
        g.step(FrameInput::joy(Joy::RIGHT));
        let mut want = State {
            irq_count: 2,
            paused: 1,
            frame_flag: 0xffff,
            ..State::level_1()
        };
        want.camera.edges.crossed = [false, true, false, false];
        want.camera.edges.latch = [0, 0xffff, 0, 0];
        want.camera.screen_y = 0x70 - 0x17;
        want.edges = g.state().edges.clone();
        want.object_map = g.state().object_map.clone();
        assert_eq!(*g.state(), want);
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
