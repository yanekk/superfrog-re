//! The level's game state: the variables a case compares, typed at the original's widths
//! (`layout.rs`, docs/09-play-measured.md "Variables"). A porting task that models more of the
//! original adds its fields here and, if a case compares them, a `LAYOUT` entry and an arm in
//! [`State::get`] and [`State::set`].

/// The game state of a level in play. BCD variables are held as stored.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    /// The title song's tick (`h4+$195c9`). The song stops when the title returns, so in play it
    /// holds the value of the title's last frame and nothing changes it (measured, T07).
    pub mt_counter: u8,
    pub char_x: u32,
    pub char_y: u32,
    pub camera_x: u16,
    pub camera_y: u16,
    pub char_screen_x: u16,
    /// BCD.
    pub lives: u16,
    pub hits_taken: u16,
    /// BCD in its first word; the original's long, as stored.
    pub countdown: u32,
    /// BCD.
    pub items_left: u16,
    /// BCD.
    pub score: u32,
    pub world_index: u32,
    /// The level-3 handler's count, one a pass: it numbers the passes.
    pub irq_count: u16,
    pub paused: u16,
    pub level_exit_touched: u16,
    pub give_up: u16,
}

impl State {
    /// W1-1 at the level sync point (DESIGN §2.3): oracle frame 6301 of `boot-to-level1`, the
    /// state after play's first pass. Every value is measured there (`fd-conform/tests/level_entry.rs`
    /// holds them against the original); where it comes from is read from the listing.
    pub fn level_1() -> State {
        State {
            // `(5001 - 4922) % 6`: the boot script's fire at the title stops the song on frame 5001,
            // so this is that script's, and the app's start takes it to stay comparable.
            mt_counter: 1,
            // Set by world 1's load list.
            char_x: 0x70,
            char_y: 0x70,
            camera_x: 0,
            // `camera_update` in play's first pass has already followed the character.
            camera_y: 0x17,
            char_screen_x: 0x70,
            // `lives_reset`, `move.w #$32`.
            lives: 0x32,
            hits_taken: 0,
            // `level_init`, `move.w #$999, countdown`; the long's second word is 0.
            countdown: 0x0999_0000,
            items_left: 0x282,
            score: 0,
            world_index: 0,
            irq_count: 1,
            paused: 0,
            level_exit_touched: 0,
            give_up: 0,
        }
    }

    /// The value of the variable called `name`, zero-extended, or `None` if the state has no such
    /// variable.
    pub fn get(&self, name: &str) -> Option<u32> {
        Some(match name {
            "mt_counter" => self.mt_counter.into(),
            "char_x" => self.char_x,
            "char_y" => self.char_y,
            "camera_x" => self.camera_x.into(),
            "camera_y" => self.camera_y.into(),
            "char_screen_x" => self.char_screen_x.into(),
            "lives" => self.lives.into(),
            "hits_taken" => self.hits_taken.into(),
            "countdown" => self.countdown,
            "items_left" => self.items_left.into(),
            "score" => self.score,
            "world_index" => self.world_index,
            "irq_count" => self.irq_count.into(),
            "paused" => self.paused.into(),
            "level_exit_touched" => self.level_exit_touched.into(),
            "give_up" => self.give_up.into(),
            _ => return None,
        })
    }

    /// Set the variable called `name` to `value`, truncated to its width; `false` if the state has
    /// no such variable.
    pub fn set(&mut self, name: &str, value: u32) -> bool {
        let (b, w) = (value as u8, value as u16);
        match name {
            "mt_counter" => self.mt_counter = b,
            "char_x" => self.char_x = value,
            "char_y" => self.char_y = value,
            "camera_x" => self.camera_x = w,
            "camera_y" => self.camera_y = w,
            "char_screen_x" => self.char_screen_x = w,
            "lives" => self.lives = w,
            "hits_taken" => self.hits_taken = w,
            "countdown" => self.countdown = value,
            "items_left" => self.items_left = w,
            "score" => self.score = value,
            "world_index" => self.world_index = value,
            "irq_count" => self.irq_count = w,
            "paused" => self.paused = w,
            "level_exit_touched" => self.level_exit_touched = w,
            "give_up" => self.give_up = w,
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LAYOUT;

    #[test]
    fn every_layout_variable_is_state_and_round_trips_at_its_width() {
        for v in LAYOUT {
            let mut s = State::level_1();
            let all_ones = u32::MAX >> (32 - 8 * v.width.bytes());
            assert!(s.set(v.name, 0xdead_beef), "{} is not in State", v.name);
            assert_eq!(s.get(v.name), Some(0xdead_beef & all_ones), "{}", v.name);
            // Setting one variable leaves every other alone.
            for w in LAYOUT.iter().filter(|w| w.name != v.name) {
                assert_eq!(
                    s.get(w.name),
                    State::level_1().get(w.name),
                    "{} moved {}",
                    v.name,
                    w.name
                );
            }
        }
    }

    #[test]
    fn an_unknown_name_is_not_state() {
        let mut s = State::level_1();
        assert_eq!(s.get("no_such"), None);
        assert!(!s.set("no_such", 1));
        assert_eq!(s, State::level_1());
    }
}
