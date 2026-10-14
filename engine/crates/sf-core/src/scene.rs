//! Which part of the game is running. Each scene is a mechanic task's: the level (T14), the slot
//! machine and bonus level (T19), the front end (T20), the game flow between them (T21).

use sf_paula::PaulaWrite;

use crate::game::Pacing;
use crate::layout::{Ram, h0};
use crate::level::{Hw, LevelCtx};
use crate::mem::Bus;
use crate::render::{self, DisplayData, Picture, View};
use crate::state::State;

/// What a scene's frame works on: the state, what the disks decide, the sound driver and the
/// hardware the frame reads.
pub struct Machine<'a> {
    pub state: &'a mut State,
    pub display: &'a DisplayData,
    pub hw: Hw,
    /// The object kinds the level runs with.
    pub dispatch: &'a crate::level::Dispatch,
    /// What the level's work costs on the beam, and the clock's steps when kept
    /// ([`crate::level::beam`]).
    pub costs: &'a [u32; crate::level::beam::Cost::COUNT],
    pub trace: Option<&'a mut Vec<crate::level::beam::Tick>>,
    /// In conformance: where the original made the frame's Paula writes ([`LevelCtx::schedule`]).
    pub schedule: Vec<(u16, u16)>,
    pub blits: Vec<(u16, u16, u16)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Scene {
    /// Nothing ported yet: changes no state, writes nothing to Paula and renders black. Until the
    /// mechanic tasks land, every start is here.
    #[default]
    Placeholder,
    /// In a level: the frog, the camera, scrolling, the clock and sound (T14); objects come with
    /// T15–T18.
    Level,
    /// The front end: boot, title, menus, attract screens (T20; the program is `Game`'s
    /// [`crate::front::Front`]).
    Front,
    /// The slot machine (`slot_machine`, T92/T95), run by `Game`'s [`crate::front::Front`] as the
    /// front end is: `coins_in` is the coins the tally put in (h0+$49c6) as it was entered.
    Slot { coins_in: u16 },
    /// The story intro disk 3 boots (T32; `Game`'s [`crate::front::Story`]).
    Story,
    /// The bonus level (T79): a level the same main program runs, in shooter mode, where a death
    /// with lives left puts the frog back on the screen and the loop runs on
    /// ([`crate::bonus::frame`]). How its loop ended is [`crate::Game::level_outcome`].
    Bonus(sf_assets::LevelId),
}

/// `logic_running` (h0+$9d6): set by `level_frame_loop` while a level plays.
const LOGIC_RUNNING: u32 = h0(0x9d6);

/// `world` (h0+$40da) and `level` (h0+$40de), as the original numbers them (the bonus level is
/// world 7).
const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);

impl Scene {
    /// The level scene for the original's `world` and `level` numbers.
    pub fn level(world: u32, level: u32) -> Scene {
        if world == 7 {
            Scene::Bonus(sf_assets::LevelId {
                world: sf_assets::World::Bonus,
                level: level as u8,
                alt: false,
            })
        } else {
            Scene::Level
        }
    }

    /// The level scene for `id`.
    pub fn of_level(id: sf_assets::LevelId) -> Scene {
        if id.world == sf_assets::World::Bonus {
            Scene::Bonus(id)
        } else {
            Scene::Level
        }
    }

    /// In a level (the bonus level among them).
    pub fn is_level(&self) -> bool {
        matches!(self, Scene::Level | Scene::Bonus(_))
    }

    /// The scene the original's memory is in, for a transplant: a level while
    /// `level_frame_loop` runs it (`level_state` counting down to the loop's end once the frog
    /// dies or the exit is reached, T38), otherwise none ported yet.
    pub fn of(ram: &dyn Ram) -> Scene {
        let word = |a: u32| (ram.byte(a) as u16) << 8 | ram.byte(a + 1) as u16;
        if word(LOGIC_RUNNING) != 0 {
            Scene::level(word(WORLD + 2) as u32, word(LEVEL + 2) as u32)
        } else {
            Scene::Placeholder
        }
    }

    /// Run one frame of this scene; returns the Paula writes it made, in order, and the picture
    /// the frame shows when it is not the state's at the frame's end. A level's frame is its main
    /// program on the beam ([`crate::level::frame::main_program`]): its picture is drawn as the
    /// beam saw it, a write the pass made after the beam had passed a line not showing there
    /// (docs/12 "Late redraws"); a pass the original's frame ends in the middle of (`pacing`) is
    /// `pending` into the next frame. When `level_frame_loop` returns in the frame (T37), `after`
    /// runs the frame's rest from the line the beam is on, before the picture is drawn.
    pub fn step(
        &mut self,
        m: &mut Machine,
        pending: &mut Option<crate::level::beam::Pending>,
        pacing: Pacing,
        view: View,
        after: &mut dyn FnMut(&mut State, u16),
    ) -> (Vec<PaulaWrite>, Option<Picture>) {
        let mut out = Vec::new();
        let mut picture = None;
        match self {
            Scene::Placeholder | Scene::Front | Scene::Slot { .. } | Scene::Story => {}
            Scene::Level | Scene::Bonus(_) => {
                let mut c = LevelCtx::new(
                    Bus::new(m.state, m.display.rom()),
                    m.dispatch,
                    m.hw,
                    m.display.sound(),
                    &mut out,
                );
                c.costs = *m.costs;
                c.schedule = std::mem::take(&mut m.schedule);
                c.blits = std::mem::take(&mut m.blits);
                if m.trace.is_some() {
                    c.trace = Some(Vec::new());
                }
                crate::level::frame::main_program(&mut c, pending.take(), pacing);
                if let (Some(t), Some(out)) = (c.trace.take(), m.trace.as_mut()) {
                    **out = t;
                }
                let f = c.frames.take().expect("the main program's frames");
                if let Some(mid) = f.mid {
                    *c.bus.st = *mid;
                }
                *pending = f.pending;
                let mut log = f.log.unwrap_or_default();
                let mut window = None;
                if let Some(line) = f.ended {
                    // The program after the loop moves the window's top down (`level_exit_fade`):
                    // the beam fetched this field from where the copper had it, the picture is
                    // cut to where it is as the next field starts.
                    let diw = render::LEVEL_COPPER + 6;
                    let old = c.bus.st.byte(diw).unwrap_or(0);
                    after(c.bus.st, line);
                    let new = c.bus.st.byte(diw).unwrap_or(0);
                    if new != old {
                        let at = crate::level::beam::Clock { line, into: 0 }.nominal();
                        log.writes.push((diw, old, at));
                        window = Some(new.wrapping_sub(old) as u16);
                    }
                }
                let mut p = render::level_as_drawn(c.bus.st, m.display, view, &log);
                if let Some(down) = window {
                    p = p.rewindow(p.left, p.top + down, p.width, p.height.saturating_sub(down));
                }
                picture = Some(p);
                drop(c);
                out.truncate(f.out_to.unwrap_or(out.len()));
                out.drain(..f.out_from);
            }
        }
        (out, picture)
    }

    pub fn render(&self, state: &State, display: &DisplayData, view: View) -> Picture {
        match self {
            Scene::Placeholder | Scene::Front | Scene::Slot { .. } | Scene::Story => {
                Picture::black(view)
            }
            Scene::Level | Scene::Bonus(_) => render::level(state, display, view),
        }
    }
}
