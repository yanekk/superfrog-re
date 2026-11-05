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

use amiga_input::FrameInput;
use fd_assets::Level;

use crate::State;

/// What happened in a step that its caller must see. Later tasks add events (a life lost, the
/// level ended), so callers match it with a fallback.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    /// An object of this value was met with no [`ObjectKind`] registered: it was left out.
    Unported(u8),
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

/// One pass of play after the level-3 handler's count: the original's `game_frame` (`h0+$1fec`).
///
/// Its calls, in the original's order, for the tasks that port them (docs/09-play-measured.md,
/// docs/11-translation.md): `char_sprite_copy`, `h0+$94ba`, `camera_update`, `h0+$513c`,
/// `char_sprite_position`, `h0+$9754`; if `paused`, only `play_keys`; else `h0+$2c7e`,
/// `h0+$2fae`, `h0+$21c0`, `h0+$62b6`, `h0+$2dd6`, `h0+$3698`, `h0+$2d4c`, `read_joystick`,
/// `char_hit_check`, `countdown_step`, `h0+$24ce`, `char_update`, `char_object_cells`, `play_keys`.
/// T08 is the first to fill it in; each later porting task adds its calls in that order.
pub fn game_frame(
    l: &mut LevelCtx,
    objects: &mut Vec<Object>,
    dispatch: &Dispatch,
    events: &mut Vec<Event>,
    _input: FrameInput,
) {
    // `camera_update`, then `h0+$513c` (T09).
    let s = &mut *l.state;
    s.camera.step((s.char_x, s.char_y), &mut s.frame_flag);
    update_objects(l, objects, dispatch, events);
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
