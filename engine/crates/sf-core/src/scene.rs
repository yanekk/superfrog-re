//! Which part of the game is running. Each scene is a mechanic task's: the level (T14), the slot
//! machine and bonus level (T19), the front end (T20), the game flow between them (T21).
//!
//! [`SceneKind`] is the screen a frame shows, told by its copper list: one classifier for
//! sf-conform's milestones and the app's bug recorder (`Game::location`; bug-recorder T02).

use sf_paula::PaulaWrite;

use crate::game::Pacing;
use crate::layout::{Ram, h0, h2};
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
    /// What the level met and does not follow ([`LevelCtx::unported`]), added to.
    pub unported: &'a mut Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

/// A screen of the game, as its copper list (and, in a level, what plays there) says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneKind {
    /// The story intro on disk 3 (its own program and memory map).
    Story,
    /// The title and its menus (`TITLE_COPPER`, h2+$7158).
    Title,
    /// The attract screens' award pages (h2+$6110) and hall of talent (h2+$6ff0).
    Awards,
    HallOfTalent,
    /// The attract demo: a level played by its recording (`attract_mode` set).
    AttractDemo,
    /// A level started, by the original's `world` and `level` numbers (the bonus level is world
    /// 7, [`SceneKind::BonusLevel`]).
    Level {
        world: u16,
        level: u16,
    },
    BonusLevel {
        level: u16,
    },
    /// The last life lost: `game_over_seq` runs in the level it was lost in.
    GameOver,
    /// The level-complete tally (h2+$6268).
    Tally,
    /// The slot machine (h2+$65e4).
    SlotMachine,
    /// The end sequence (h2+$5efc).
    EndSequence,
    /// The world's loading picture with its text and the frog's walk (`world_intro_show`,
    /// h2+$6750).
    Intro,
    /// The progress map (`progress_map_show`, h2+$69f0).
    WorldMap,
    /// The end-of-world picture after a world's fourth level (`end_world_pic_show`, h2+$6494).
    WorldEnd,
    /// A screen with a copper list not named above: its address.
    Other(u32),
}

/// The copper lists that name a screen (docs/12, docs/13; `front::chain` for the screens between
/// levels).
pub const TITLE_COPPER: u32 = h2(0x7158);
pub const AWARD_COPPER: u32 = h2(0x6110);
pub const HALL_COPPER: u32 = h2(0x6ff0);
pub const TALLY_COPPER: u32 = h2(0x6268);
pub const SLOT_COPPER: u32 = h2(0x65e4);
pub const END_COPPER: u32 = h2(0x5efc);
pub const INTRO_COPPER: u32 = h2(0x6750);
pub const MAP_COPPER: u32 = h2(0x69f0);
pub const WORLD_END_COPPER: u32 = h2(0x6494);
/// The black list screens fade through.
pub const BLANK_COPPER: u32 = h2(0x6fd8);

impl SceneKind {
    /// The screen a frame is on, from the copper list it starts with (`None` when the side does
    /// not say) and, on a level's list, `attract_mode`, `frog_lives`, `world` and `level`. `None`
    /// for the blank list a screen fades through, or no list: the screen before it stays the
    /// scene. `story`: in the story intro, whose memory is not the game's.
    pub fn classify(
        story: bool,
        cop1lc: Option<u32>,
        attract: bool,
        lives: u16,
        world: u16,
        level: u16,
    ) -> Option<SceneKind> {
        if story {
            return Some(SceneKind::Story);
        }
        Some(match cop1lc? {
            render::LEVEL_COPPER if attract => SceneKind::AttractDemo,
            render::LEVEL_COPPER if (lives as i16) < 0 => SceneKind::GameOver,
            render::LEVEL_COPPER if world == 7 => SceneKind::BonusLevel { level },
            render::LEVEL_COPPER => SceneKind::Level { world, level },
            TITLE_COPPER => SceneKind::Title,
            AWARD_COPPER => SceneKind::Awards,
            HALL_COPPER => SceneKind::HallOfTalent,
            TALLY_COPPER => SceneKind::Tally,
            SLOT_COPPER => SceneKind::SlotMachine,
            END_COPPER => SceneKind::EndSequence,
            INTRO_COPPER => SceneKind::Intro,
            MAP_COPPER => SceneKind::WorldMap,
            WORLD_END_COPPER => SceneKind::WorldEnd,
            BLANK_COPPER => return None,
            other => SceneKind::Other(other),
        })
    }

    /// The name a bug recording gives this screen ([`crate::recording::SCREENS`]); `None` in a
    /// level, which is named by the frog's cell, and for a list no name covers. The attract
    /// screens and demo are the title's (owner, 2026-10-20).
    pub fn screen_name(self) -> Option<&'static str> {
        Some(match self {
            SceneKind::Story => "story",
            SceneKind::Title | SceneKind::Awards | SceneKind::HallOfTalent | SceneKind::AttractDemo => {
                "title"
            }
            SceneKind::GameOver => "game over",
            SceneKind::Tally => "tally",
            SceneKind::SlotMachine => "slot machine",
            SceneKind::EndSequence => "end sequence",
            SceneKind::Intro => "intro",
            SceneKind::WorldMap => "world map",
            SceneKind::WorldEnd => "world end",
            SceneKind::Level { .. } | SceneKind::BonusLevel { .. } | SceneKind::Other(_) => {
                return None;
            }
        })
    }
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
        after: &mut dyn FnMut(&mut State, u16, u32),
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
                for u in c.unported.drain(..) {
                    if !m.unported.contains(&u) {
                        m.unported.push(u);
                    }
                }
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
                if let Some((line, into)) = f.ended {
                    // The program after the loop moves the window's top down (`level_exit_fade`):
                    // the beam fetched this field from where the copper had it, the picture is
                    // cut to where it is as the next field starts.
                    let diw = render::LEVEL_COPPER + 6;
                    let old = c.bus.st.byte(diw).unwrap_or(0);
                    after(c.bus.st, line, into);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_list_names_its_screen_and_the_blank_names_none() {
        let on = |cop| SceneKind::classify(false, Some(cop), false, 2, 1, 3);
        for (cop, kind, name) in [
            (TITLE_COPPER, SceneKind::Title, Some("title")),
            (AWARD_COPPER, SceneKind::Awards, Some("title")),
            (HALL_COPPER, SceneKind::HallOfTalent, Some("title")),
            (TALLY_COPPER, SceneKind::Tally, Some("tally")),
            (SLOT_COPPER, SceneKind::SlotMachine, Some("slot machine")),
            (END_COPPER, SceneKind::EndSequence, Some("end sequence")),
            (INTRO_COPPER, SceneKind::Intro, Some("intro")),
            (MAP_COPPER, SceneKind::WorldMap, Some("world map")),
            (WORLD_END_COPPER, SceneKind::WorldEnd, Some("world end")),
            (render::LEVEL_COPPER, SceneKind::Level { world: 1, level: 3 }, None),
            (h2(0x1000), SceneKind::Other(h2(0x1000)), None),
        ] {
            assert_eq!(on(cop), Some(kind), "{cop:x}");
            assert_eq!(kind.screen_name(), name, "{kind:?}");
            if let Some(n) = name {
                assert!(crate::recording::SCREENS.contains(&n), "{n}");
            }
        }
        assert_eq!(on(BLANK_COPPER), None);
        assert_eq!(SceneKind::classify(false, None, false, 2, 1, 3), None);
        let level = |attract, lives, world| {
            SceneKind::classify(false, Some(render::LEVEL_COPPER), attract, lives, world, 1)
        };
        assert_eq!(level(true, 2, 1), Some(SceneKind::AttractDemo));
        assert_eq!(SceneKind::AttractDemo.screen_name(), Some("title"));
        assert_eq!(level(false, 0xffff, 1), Some(SceneKind::GameOver));
        assert_eq!(SceneKind::GameOver.screen_name(), Some("game over"));
        assert_eq!(level(false, 2, 7), Some(SceneKind::BonusLevel { level: 1 }));
        assert_eq!(SceneKind::classify(true, None, false, 0, 0, 0), Some(SceneKind::Story));
        assert_eq!(SceneKind::Story.screen_name(), Some("story"));
    }
}
