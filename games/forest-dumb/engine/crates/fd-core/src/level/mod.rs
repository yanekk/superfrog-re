//! Play in a level: one pass of the original's `game_frame`, and the object dispatch.
//!
//! # The object dispatch (for T22 collectibles, T24 enemies, T25 mechanics)
//!
//! Every object in play is an [`Object`] carrying its **value**: the byte the original's object map
//! (`_MapaBobowN`, `fd_assets::ObjectMap`) holds at its cell, unchanged. Values are the original's,
//! so a list transplanted from the original's memory maps one to one, and `re/objects-W1-1.toml`
//! (T21) names each value with the task that ports it.
//!
//! A porting task gives each value it ports an [`ObjectKind`]: a `static` unit struct whose
//! `update` is the port of the original's per-pass routine for that object, and whose `draw` adds
//! what it shows to a [`DrawList`]. It registers the kind in [`Dispatch::level_1`], one line a
//! value, through [`register`]; two kinds for one value panic there, so two tasks cannot port one
//! object twice. A kind keeps its own state in the object's fields; a field a kind needs that
//! [`Object`] lacks is added to [`Object`], never kept in the kind (kinds are shared statics, and
//! `Game: Clone` is the planner's snapshot).
//!
//! An object whose value has no kind is not skipped: the pass it is met records
//! [`Event::Unported`] with its value and drops the object, so it is reported once. Every
//! conformance case fails on one (`fd-conform`, DESIGN §4.1), and the app writes it to stderr and
//! plays on without the object. So an object nobody ported is never a silent gap.
//!
//! Objects are spawned with [`crate::Game::spawn`]; the scan that spawns them from the object map as
//! the view passes is T22's. Where in the pass the original updates its objects is T21's to measure:
//! [`game_frame`] runs them after the character and camera until then.

pub mod camera;
pub mod collision;
pub mod redraw;
pub mod start;

use amiga_input::FrameInput;
use fd_assets::Level;

use crate::State;
use crate::character::{AnimTables, Character, Play};
use crate::objects::creature::{Creatures, PlayGround};
use crate::program::{Program, Scratch};

use collision::CollisionPlane;

/// What happened in a step that its caller must see. Later tasks add events (a life lost, the
/// level ended), so callers match it with a fallback.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    /// An object of this value was met with no [`ObjectKind`] registered: it was left out.
    Unported(u8),
    /// `main_loop` found the game over: the game over screen shows from this step (T23).
    GameOver,
    /// The game over screen ended where the original enters its high scores; the level restarted
    /// as at its start (T23, DESIGN §2.4); or fire ended "Level 1 complete" (T26, DESIGN §2.5).
    Restart,
    /// `main_loop` found the level's end and ran `next_world` in this step (T26); "Level 1
    /// complete" shows from the next.
    LevelEnd,
}

/// One object in play.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Object {
    /// The original's object value, as its object map holds it.
    pub value: u8,
    /// The object map cell it came from.
    pub col: u16,
    pub row: u16,
    /// Its place in level pixels.
    pub x: i32,
    pub y: i32,
    /// Set by a kind's `update` to take the object out of play after this pass.
    pub removed: bool,
}

impl Object {
    /// An object of `value` spawned at a cell, placed at the cell's top-left.
    pub fn at_cell(value: u8, col: u16, row: u16) -> Object {
        Object {
            value,
            col,
            row,
            x: i32::from(col) * 16,
            y: i32::from(row) * 16,
            removed: false,
        }
    }
}

/// What a pass may read and change, handed to each object's `update`.
pub struct LevelCtx<'a> {
    pub state: &'a mut State,
    pub level: &'a Level,
    /// The main program's hunk 0 as loaded: the tables the original's routines read (T08).
    pub program: &'a Program,
    pub anim: &'a AnimTables,
    /// The collision plane, play state (T08; T55 draws it, T25's vanishing blocks change it).
    pub plane: &'a mut CollisionPlane,
    /// The level's buffers as the original lays them out in hunk 0 (T55): the tile map a taken
    /// cell is redrawn from (T22).
    pub image: &'a redraw::LevelImage,
    /// What the display shows (T10): picture state, which play logic never reads.
    pub display: &'a mut crate::render::Display,
    /// What the display reads from the disks: the level palette the fade reaches.
    pub display_data: &'a crate::render::DisplayData,
    /// CLXDAT as this pass's `char_hit_check` reads it: the collisions of the frame the display
    /// showed since the last pass (`render::contact`, T70).
    pub clxdat: u16,
}

/// One thing an object shows: an image at a place in level pixels. T10 and the kinds' tasks
/// (T22, T24, T25, T52) say what `image` numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bob {
    pub x: i32,
    pub y: i32,
    pub image: u16,
}

/// What the objects show this pass, in drawing order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DrawList {
    pub bobs: Vec<Bob>,
}

/// The behaviour of one object value. See the module comment.
pub trait ObjectKind: Sync {
    /// One pass of the object, as the original's routine for it.
    fn update(&self, o: &mut Object, l: &mut LevelCtx);
    /// What the object shows, added to `out`.
    fn draw(&self, o: &Object, out: &mut DrawList);
}

/// The kind of each object value, by value.
#[derive(Clone, Copy)]
pub struct Dispatch {
    kinds: [Option<&'static dyn ObjectKind>; 256],
}

impl Dispatch {
    /// No value has a kind.
    pub fn empty() -> Dispatch {
        Dispatch { kinds: [None; 256] }
    }

    /// Every kind ported for world 1. The porting tasks add their `register` lines here.
    pub fn level_1() -> Dispatch {
        // T22 collectibles, T24 enemies, T25 mechanics: `register(&mut d, value, &Kind);`
        Dispatch::empty()
    }

    /// The kind registered for `value`.
    pub fn kind(&self, value: u8) -> Option<&'static dyn ObjectKind> {
        self.kinds[value as usize]
    }

    /// Every value that has a kind, ascending.
    pub fn values(&self) -> Vec<u8> {
        (0..=255u8)
            .filter(|&v| self.kinds[v as usize].is_some())
            .collect()
    }
}

impl std::fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dispatch")
            .field("values", &self.values())
            .finish()
    }
}

/// Give object value `value` the kind `k`. Panics if `value` already has one: two ports of one
/// object are a mistake to fix, not a choice to make at run time.
pub fn register(d: &mut Dispatch, value: u8, k: &'static dyn ObjectKind) {
    assert!(
        d.kinds[value as usize].is_none(),
        "object value {value:#04x} is registered twice"
    );
    d.kinds[value as usize] = Some(k);
}

/// What `game_frame`'s callees meet outside the engine's typed state: the custom chips, the play
/// screen and the raster. In play ([`InPlay`]) blits are drawn into the display and sounds,
/// custom-chip writes and raster reads are dropped; the routine rig (`fd-conform`'s
/// `tests/game_frame_routine.rs`, T63) turns each into the original's writes, in order, so the
/// whole pass is held to `game_frame` byte for byte.
pub trait Pass {
    /// `game_frame` calls the routine at hunk 0 `offset` next: its `bsr`s in the listing's order.
    fn call(&mut self, _offset: u32) {}
    /// The object ports (`objects::MapObjects`) run as the routine last [`Pass::call`]ed.
    fn objects(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut crate::objects::MapObjects));
    /// The creature drawing (`render::objects::Creatures`) run as the routine last called.
    fn creatures(
        &mut self,
        l: &mut LevelCtx,
        f: &mut dyn FnMut(&mut crate::render::objects::Creatures),
    );
    /// JOY1DAT as `read_joystick` reads it.
    fn joy1dat(&mut self, input: &FrameInput) -> u16 {
        crate::character::joy1dat(input)
    }
    /// The registers and raster a routine on [`Scratch`] starts with.
    fn scratch(&mut self) -> Scratch {
        Scratch::default()
    }
    /// That routine's scratch when it returns, with the custom-chip writes it made.
    fn scratch_done(&mut self, _s: Scratch) {}
    /// `h0_9754`: the copper list's play screen pointers from the camera. Nothing in play reads them
    /// before the picture, which computes them then (`render::picture`).
    fn scroll(&mut self, _camera_x: u16, _camera_y: u16) {}
    /// `move.w $dff006, h0+$8dfa`: the raster position stored, which nothing reads. The engine keeps
    /// no beam, so play leaves it out.
    fn raster_copy(&mut self) {}
}

/// [`Pass`] in play: the object ports and the creature drawing on the play screen.
pub struct InPlay;

impl Pass for InPlay {
    fn objects(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut crate::objects::MapObjects)) {
        crate::objects::in_play(l, f);
    }
    fn creatures(
        &mut self,
        l: &mut LevelCtx,
        f: &mut dyn FnMut(&mut crate::render::objects::Creatures),
    ) {
        crate::render::objects::in_play(l, f);
    }
}

/// One pass of play after the level-3 handler's count: the original's `game_frame` (`h0+$1fec`), in
/// play ([`InPlay`]). See [`game_frame_with`].
pub fn game_frame(
    l: &mut LevelCtx,
    objects: &mut Vec<Object>,
    dispatch: &Dispatch,
    events: &mut Vec<Event>,
    input: FrameInput,
) {
    game_frame_with(l, objects, dispatch, events, input, &mut InPlay);
}

/// `game_frame` (`h0+$1fec`) with what lies outside the typed state through `p`. Its calls, in the
/// original's order (docs/09-play-measured.md, docs/11-translation.md): `char_sprite_copy`,
/// `h0+$94ba`, `camera_update`, `h0+$513c`, `char_sprite_position`, `h0+$9754`; if `paused`, only
/// `play_keys`; else `h0+$2c7e`, `h0+$2fae`, `h0+$21c0`, `h0+$62b6`, `h0+$2dd6`, `h0+$3698`,
/// `h0+$2d4c`, `read_joystick`, `char_hit_check`, `countdown_step`, `h0+$24ce`, `char_update`,
/// `char_object_cells`, `play_keys`. Proven whole on the routine rig, and its calls' order against
/// `re/translate.toml` (T63, `fd-conform/tests/game_frame_routine.rs`).
pub fn game_frame_with(
    l: &mut LevelCtx,
    objects: &mut Vec<Object>,
    dispatch: &Dispatch,
    events: &mut Vec<Event>,
    input: FrameInput,
    p: &mut dyn Pass,
) {
    // char_sprite_copy, h0+$94ba (T10).
    p.call(0x9b06);
    let s = &mut *l.state;
    l.display.sprite.copy(&mut s.motion.unhurt_t, s.anim.ptr);
    p.call(0x94ba);
    l.display.fade.step(&l.display_data.palette(), l.program);
    // `camera_update`, then `h0+$513c` (T09).
    p.call(0x4d7e);
    s.camera.update((s.char_x, s.char_y), &mut s.frame_flag);
    p.call(0x513c);
    s.camera.clamp();
    // char_sprite_position, h0+$9754 (T10).
    p.call(0x5462);
    l.display
        .sprite
        .position(s.char_x, s.char_y, s.camera.x, s.camera.y);
    p.call(0x9754);
    p.scroll(s.camera.x, s.camera.y);
    if l.state.paused != 0 {
        p.call(0x2104);
        Play { state: l.state }.play_keys();
        return;
    }
    // h0+$2c7e: the tiles behind the creatures the last pass drew restored, run with their flags
    // (`creature_active+$c`) in place of the live ones, which wait at `creature_active+6` (T52).
    let s = &mut *l.state;
    s.creature_draw.saved = s.edges.creatures.active;
    s.edges.creatures.active = s.creature_draw.drawn;
    p.call(0x2c7e);
    p.creatures(l, &mut |c| c.restore_all());
    let s = &mut *l.state;
    s.edges.creatures.active = s.creature_draw.saved;
    // h0+$2fae: the creatures moved (T24).
    p.call(0x2fae);
    Creatures {
        state: l.state,
        ground: &PlayGround {
            plane: l.plane,
            program: l.program,
        },
    }
    .move_all();
    // Until they are ported, the dispatch's kinds run here.
    update_objects(l, objects, dispatch, events);
    // h0+$21c0: the spring's picture; h0+$62b6: the animated cells (T25).
    p.call(0x21c0);
    p.objects(l, &mut |o| o.spring_redraw());
    p.call(0x62b6);
    p.objects(l, &mut |o| o.animate());
    // h0+$2dd6: creatures out of view written back into the object map (T22).
    p.call(0x2dd6);
    p.objects(l, &mut |o| o.write_back());
    // h0+$3698: the slots sorted by y for drawing (T24).
    p.call(0x3698);
    Creatures {
        state: l.state,
        ground: &PlayGround {
            plane: l.plane,
            program: l.program,
        },
    }
    .sort();
    // h0+$2d4c: the active creatures cut onto the screen, and their flags kept for the next pass's
    // restore (T52).
    p.call(0x2d4c);
    p.creatures(l, &mut |c| c.cut_all());
    p.raster_copy();
    let s = &mut *l.state;
    s.creature_draw.drawn = s.edges.creatures.active;
    p.call(0x8dfc);
    let dat = p.joy1dat(&input);
    s.joy.read_joy1dat(&mut s.input_log, dat);
    // The registers and custom-chip writes the original leaves are the rig's; play drops them.
    p.call(0x3b2a);
    let mut scratch = p.scratch();
    let mut play = Play { state: l.state };
    play.char_hit_check(l.clxdat, &mut scratch);
    p.scratch_done(scratch);
    p.call(0x20b0);
    play.countdown_step();
    // h0+$24ce: a spring under the feet (T25).
    p.call(0x24ce);
    p.objects(l, &mut |o| o.spring_trigger());
    p.call(0x3e0c);
    let mut scratch = p.scratch();
    Character {
        state: l.state,
        program: l.program,
        anim: l.anim,
        plane: &*l.plane,
        scratch: &mut scratch,
    }
    .update();
    p.scratch_done(scratch);
    // char_object_cells (T22).
    p.call(0x2698);
    p.objects(l, &mut |o| o.object_cells());
    // play_keys (T23).
    p.call(0x2104);
    Play { state: l.state }.play_keys();
}

/// A row redraw that cuts this many bobs is still running when the next pass starts (T73,
/// measured on T24's planner route; docs/09, "The edge redraws and the next pass"). The redraws'
/// blits wait for the raster to be on lines 103 to 230 (`h0_3ac2`), and the next pass comes at line
/// 233: a row's 23 tile blits end near line 161, its first bob starts near 197, and each bob takes
/// about 5 lines. Rows with 7 bobs finished, the 7th starting at 227; rows with 9 were cut, the 8th
/// starting at 230 at the earliest, so 8 is taken as cut. With the animated cells running (T74) the
/// original also cuts columns and, rarely, rows of fewer bobs, as the cycles each pass took decide;
/// the engine alone keeps this rule, and a conformance case hands the measured cut ([`Cut::At`]).
pub const ROW_BOBS_CUT: usize = 8;

/// Where the next pass cut `main_loop`'s redraw half, measured on the original (T74;
/// `fd_conform::edges_cut`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgesCut {
    /// The index in the order down, up, left, right of the first latch `main_loop` had not yet
    /// tested, 4 when it had tested all but not yet cleared `frame_flag`.
    pub next: u8,
    /// In the redraw before `next`, the one the pass interrupted: the object map cell (hunk 0
    /// offset) from which its scan's animated cells were added after the pass, 0 when its scan had
    /// not started ([`redraw::PlayIo::defer_from`]); `None` when it had returned.
    pub defer_from: Option<u32>,
    /// How many `collision_map` words the interrupted redraw had copied when the pass came before
    /// its scan, in the order it copies them (16 a shape, a row's 23 shapes or a column's 16): the
    /// rest land after the pass, which still probes the band as it was (T48, measured on the route:
    /// a creature on the row below the view turned at a ledge the row's shapes then filled).
    /// `None` once they were all copied.
    pub shape_words: Option<u16>,
    /// How far `h0_6882` had got with the cell at `defer_from` when the pass came.
    pub add: AddCut,
}

/// Where a pass cut `h0_6882`, which adds a scan's animated cell to the list holding the fewest
/// (T82). It counts the four lists, sorts the counts, then looks for the chosen list's first free
/// entry, and the pass between drops cells from one list (`h0_62b6`), so a choice made before the
/// pass can differ from one made after. Measured on the route: pass 12 569 came at `h0+$693e`, in
/// the sort, and the cell joined the list the counts before the pass chose, which by then held
/// one more than another.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AddCut {
    /// Not in `h0_6882`, or still counting: the cell is added whole after the pass. A pass that
    /// came between two counts is not modelled (never measured).
    #[default]
    Whole,
    /// Past the four counts (`h0+$691e`): the list is the one the counts before the pass choose,
    /// its free entry found after the pass.
    List,
    /// Past the search (`h0+$6984`): the entry is the one free before the pass too.
    Entry,
}

/// Where the next pass cuts `main_loop`'s redraw half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    /// The engine's own rule, alone (the app, the planner's search): after a row redraw (down or up)
    /// cutting [`ROW_BOBS_CUT`] bobs or more.
    Rows,
    /// Measured on the original by a conformance case; `None` when the redraw half had finished
    /// (or was never reached) before the next pass.
    At(Option<EdgesCut>),
}

/// `main_loop`'s redraw half (`h0+$2a0` to `$358`), which runs in the foreground between passes: when
/// `frame_flag` is set, each latched edge in the order down, up, left, right has its position copied to
/// the redraws' (`h0+$1e66`), its latch cleared and its redraw run; then `frame_flag` is cleared.
///
/// The engine runs it before the next pass, except where `cut` says that pass cut it. Then the
/// latches not yet tested are still set during the next pass, which can neither cross them nor have
/// them drawn, and the rest of the order runs after it, from [`State::edges_resume`], testing each
/// latch as it then stands, `frame_flag` set or not, and clearing `frame_flag` at the end. So a latch
/// that pass sets earlier in the order waits for the pass after; cut after the last redraw, before
/// `clr.w frame_flag` (`h0+$352`), every latch that pass sets waits (T74, docs/09). The redraw the
/// pass interrupted is drawn whole here: its last blits land outside the view a pass early.
pub fn main_loop_redraws(
    s: &mut State,
    plane: &mut CollisionPlane,
    image: &redraw::LevelImage,
    blits: &mut Vec<redraw::Blit>,
    cut: Cut,
) {
    use camera::Dir;
    // The pass came again inside the redraw the last one cut, still short of its scan: only the
    // shapes it has copied since land now, and its scan is still to come.
    let again = match cut {
        Cut::At(Some(c)) if Some(c.next) == s.edges_resume => c.shape_words,
        _ => None,
    };
    // The cut redraw's shapes and scan, finished after the pass.
    for d in std::mem::take(&mut s.edges.deferred) {
        match d {
            redraw::Deferred::Collision { n, .. } if again.is_some_and(|m| n >= m) => {
                s.edges.deferred.push(d)
            }
            redraw::Deferred::Collision { at, word, .. } => plane.set_word(at, word),
            _ if again.is_some() => s.edges.deferred.push(d),
            redraw::Deferred::Cell(c) => s.edges.cells.add(c.x, c.y, c.code),
            redraw::Deferred::CellIn { cell, list, entry } => {
                s.edges.cells.add_chosen(cell, list, entry)
            }
            redraw::Deferred::Creature {
                cell,
                x,
                y,
                kind,
                facing,
            } => {
                // `h0_681c`, then the scan's `clr.b (a2)` when a slot took it.
                if !s.edges.creatures.start(x, y, kind, facing) {
                    let i = cell.wrapping_sub(redraw::OBJECT_MAP) as usize;
                    if let Some(b) = s.object_map.get_mut(i) {
                        *b = 0;
                    }
                }
            }
        }
    }
    let from = match s.edges_resume.take() {
        Some(i) => i as usize,
        None if s.frame_flag == 0 => return,
        None => 0,
    };
    // A measured cut before where a resumed half picks up: the pass came while `main_loop` was
    // still finishing the redraw it resumed into, which the engine drew whole before.
    let (stop, defer_from, shape_words, add) = match cut {
        Cut::At(Some(c)) if c.next as usize > from => {
            (Some(c.next as usize), c.defer_from, c.shape_words, c.add)
        }
        Cut::At(Some(_)) => (Some(from), None, None, AddCut::Whole),
        _ => (None, None, None, AddCut::Whole),
    };
    let mut io = redraw::PlayIo {
        image,
        objects: &mut s.object_map,
        collision: plane,
        blits,
        defer_from: None,
        add_cut: add,
        late_from: None,
        written: 0,
        late: Vec::new(),
    };
    let order = [Dir::Down, Dir::Up, Dir::Left, Dir::Right];
    for (k, &dir) in order.iter().enumerate().skip(from) {
        if stop == Some(k) {
            s.edges_resume = Some(k as u8);
            return;
        }
        let i = dir as usize;
        if s.camera.edges.latch[i] == 0 {
            continue;
        }
        io.defer_from = defer_from.filter(|_| stop == Some(k + 1));
        io.late_from = shape_words.filter(|_| stop == Some(k + 1));
        io.written = 0;
        s.edges.pos = s.camera.edges.at[i];
        s.camera.edges.latch[i] = 0;
        let before = io.blits.len();
        match dir {
            Dir::Down => s.edges.redraw_below(&mut io),
            Dir::Up => s.edges.redraw_above(&mut io),
            Dir::Left => s.edges.redraw_left(&mut io),
            Dir::Right => s.edges.redraw_right(&mut io),
        }
        s.edges.deferred.extend(
            io.late
                .drain(..)
                .map(|(n, at, word)| redraw::Deferred::Collision { n, at, word }),
        );
        let bobs = io.blits[before..]
            .iter()
            .filter(|b| matches!(b, redraw::Blit::Bob { .. }))
            .count();
        if cut == Cut::Rows && matches!(dir, Dir::Down | Dir::Up) && bobs >= ROW_BOBS_CUT {
            s.edges_resume = Some(k as u8 + 1);
            return;
        }
    }
    if stop == Some(order.len()) {
        s.edges_resume = Some(order.len() as u8);
        return;
    }
    s.frame_flag = 0;
}

/// Each object's pass through its kind; an object with none is reported and dropped, and one its
/// kind removed is dropped.
pub fn update_objects(
    l: &mut LevelCtx,
    objects: &mut Vec<Object>,
    dispatch: &Dispatch,
    events: &mut Vec<Event>,
) {
    objects.retain_mut(|o| match dispatch.kind(o.value) {
        Some(k) => {
            k.update(o, l);
            !o.removed
        }
        None => {
            events.push(Event::Unported(o.value));
            false
        }
    });
}

/// What every object shows, in the order they are held.
pub fn draw_objects(objects: &[Object], dispatch: &Dispatch, out: &mut DrawList) {
    for o in objects {
        if let Some(k) = dispatch.kind(o.value) {
            k.draw(o, out);
        }
    }
}
