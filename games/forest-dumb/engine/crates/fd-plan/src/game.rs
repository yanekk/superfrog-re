//! The engine as the toolkit's planner plays it: [`PlanState`], fd-core's [`Game`] behind a newtype
//! (the orphan rule bars implementing `amiga_plan::PlanGame` on `Game` here), and what the planner
//! reads off its state.
//!
//! Each reading is the state's own variable (docs/09-play-measured.md, "Variables"). Where the engine
//! does not model a rule yet the reading still comes from the variable the original keeps, so it
//! starts to count the moment a later task ports the rule: lives and `hits_taken` (T23, T24), the
//! dying fall (T24), the items (T22) and the level's end (T26).

use amiga_input::{FrameInput, Joy};
use amiga_plan::{Cell, Facts, PlanGame};
use fd_core::Game;
use fd_core::recording::{LevelId, Location};

use crate::facts::{COLS, ROWS, is_item, solid};

/// Contact passes in a row that cost a life (`char_hit_check`, docs/10 "Contact damage").
const HITS_PER_LIFE: u16 = 3;

/// The burst's joystick states: still, the four directions but up alone, and a jump straight or
/// either way. Forest Dumb reads no fire button in play (`read_joystick` sets `joy_left`..`joy_down`
/// only, docs/09 "Input"), so no state holds it.
pub const ACTS: [Joy; 7] = [
    Joy::NONE,
    Joy::RIGHT,
    Joy::LEFT,
    Joy::UP,
    Joy(Joy::RIGHT.0 | Joy::UP.0),
    Joy(Joy::LEFT.0 | Joy::UP.0),
    Joy::DOWN,
];

/// fd-core's game as the planner steps it. `Clone` is the planner's snapshot, as `Game`'s is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanState(pub Game);

/// A BCD word's value.
pub fn bcd(w: u16) -> u16 {
    (0..4).rev().fold(0, |n, i| n * 10 + (w >> (4 * i) & 0xf))
}

impl PlanState {
    pub fn game(&self) -> &Game {
        &self.0
    }

    /// The character's cell on the map: its spot (`Location::of_character`), clamped to the map.
    pub fn location(&self) -> Location {
        let s = self.0.state();
        Location::of_character(LevelId::W1_1, s.char_x, s.char_y)
    }
}

impl PlanGame for PlanState {
    fn step(&mut self, i: FrameInput) {
        self.0.step(i);
        // Nothing reads them; the app logs them, a case fails on them. Left, they would grow with
        // every frame of a burst.
        self.0.take_events();
    }

    /// What a clone holds: the struct, and its own copies of the object map, the play screen and
    /// the status bar. The collision plane is shared until written, and nearly every step writes it,
    /// so it counts too. The level's data is shared by every state (an `Arc`).
    fn state_bytes(&self) -> u64 {
        let g = &self.0;
        let d = g.display();
        (std::mem::size_of::<Game>()
            + g.state().object_map.len()
            + d.screen.bytes.len()
            + d.status.bytes.len()
            + fd_core::level::collision::KEPT as usize
            + fd_core::level::collision::BEFORE as usize) as u64
    }

    fn cell(&self) -> Cell {
        let l = self.location();
        (l.col.min(COLS - 1), l.row.min(ROWS - 1))
    }

    fn lives(&self) -> i16 {
        bcd(self.0.state().lives) as i16
    }

    fn dying(&self) -> bool {
        self.0.state().motion.dying != 0
    }

    fn health(&self) -> u16 {
        HITS_PER_LIFE.saturating_sub(self.0.state().hits_taken)
    }

    /// `world_index` is 0 while W1-1 is played; `next_world` sets it to 1 at the level's end (the
    /// comparison's end, docs/09 "Level end").
    fn playing(&self) -> bool {
        self.0.state().world_index == 0
    }

    fn collectibles_left(&self) -> u16 {
        bcd(self.0.state().items_left)
    }

    fn held_flag(&self) -> bool {
        false
    }

    /// Neither rising nor falling: standing, walking or sliding on ground.
    fn landed(&self) -> bool {
        let m = &self.0.state().motion;
        m.rising == 0 && m.falling == 0
    }

    /// A cell whose collision shape is not wholly solid. Read from the level's file: the vanishing
    /// blocks are the only cells play changes (T25), and none is on a wall.
    fn open(&self, (c, r): Cell) -> bool {
        !solid(&self.0.data().level, c, r)
    }

    /// An item (`$20` to `$23`) still in the object map as play has it.
    fn collectible_live(&self, (c, r): Cell) -> bool {
        let at = c as usize * ROWS as usize + r as usize;
        self.0
            .state()
            .object_map
            .get(at)
            .is_some_and(|&v| is_item(v))
    }

    /// W1-1 has no switches.
    fn pressed(&self, _: &Facts) -> u32 {
        0
    }

    const ACTS: &'static [Joy] = &ACTS;
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_core::level::redraw::OBJECT_MAP;
    use fd_core::{LAYOUT, Mem, Start, Transplant};

    /// The game's own state as the original's memory, with the character moved to (`x`, `y`). Bytes
    /// no variable or the object map covers read 0: an empty collision plane and a blank screen.
    struct Moved<'a> {
        game: &'a Game,
        at: (u32, u32),
    }

    impl Mem for Moved<'_> {
        fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
            let s = self.game.state();
            if (hunk, offset) == (0, OBJECT_MAP) {
                return s.object_map[..len].to_vec();
            }
            let Some(v) = LAYOUT
                .iter()
                .find(|v| (v.hunk, v.offset, v.width.bytes()) == (hunk, offset, len))
            else {
                return vec![0; len];
            };
            let value = match v.name {
                "char_x" => self.at.0,
                "char_y" => self.at.1,
                _ => self.game.probe(v),
            };
            value.to_be_bytes()[4 - len..].to_vec()
        }
    }

    /// A coin taken by the engine is gone from the planner's readings: `collectibles_left` counts it
    /// down from `items_left` and `collectible_live` reads the object map play has (T22).
    #[test]
    fn a_coin_the_engine_takes_leaves_the_planners_readings() {
        let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
        let game = Game::new(data.clone(), Start::Level1);
        // W1-1:2,73, a coin (docs/10). Its column under `char_x + $e`, its row under the feet
        // (`char_y + $19`), four pixels down the cell, so the pass's fall stays in it.
        let coin = (2u16, 73u16);
        assert!(crate::facts::is_item(data.level.object(coin.0, coin.1)));
        let at = (coin.0 as u32 * 16 + 2 - 0xe, coin.1 as u32 * 16 + 4 - 0x19);
        let mut g = game.clone();
        g.transplant(&Moved { game: &game, at });
        let mut p = PlanState(g);
        assert!(p.collectible_live(coin));
        let before = p.collectibles_left();
        assert_eq!(before, 282);
        p.step(FrameInput::NONE);
        assert!(!p.collectible_live(coin), "the coin is still in the map");
        assert_eq!(p.collectibles_left(), before - 1);
        assert_eq!(p.game().state().score, 0x100);
    }

    /// A bonus scores what its branch of the program says (`objects::collectible::BONUS_SCORES`, read
    /// from hunk 0 in play) and leaves the items alone: W1-1:2,37's orange, `$40`, +20.
    #[test]
    fn a_bonus_the_engine_takes_scores_from_the_program() {
        let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
        let game = Game::new(data.clone(), Start::Level1);
        let bonus = (2u16, 37u16);
        assert_eq!(data.level.object(bonus.0, bonus.1), 0x40);
        let at = (
            bonus.0 as u32 * 16 + 2 - 0xe,
            bonus.1 as u32 * 16 + 4 - 0x19,
        );
        let mut g = game.clone();
        g.transplant(&Moved { game: &game, at });
        let mut p = PlanState(g);
        p.step(FrameInput::NONE);
        let s = p.game().state();
        let cell = bonus.0 as usize * ROWS as usize + bonus.1 as usize;
        assert_eq!(
            (s.score, s.items_left, s.object_map[cell]),
            (0x20, 0x282, 0)
        );
    }
}
