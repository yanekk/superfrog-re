//! The level's game state: the variables a case compares, typed at the original's widths
//! (`layout.rs`, docs/09-play-measured.md "Variables"). A porting task that models more of the
//! original adds its fields here and, if a case compares them, a `LAYOUT` entry and an arm in
//! [`State::get`] and [`State::set`].

use crate::character::{Anim, InputLog, JoyFlags};
use crate::level::camera::{Camera, EdgeFlags};

/// The game state of a level in play. BCD variables are held as stored.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    /// The title song's tick (`h4+$195c9`). The song stops when the title returns, so in play it
    /// holds the value of the title's last frame and nothing changes it (measured, T07).
    pub mt_counter: u8,
    pub char_x: u32,
    pub char_y: u32,
    /// `camera_x`, `camera_y`, `char_screen_x` (the low word of `camera.screen_x`) and the edge
    /// latches and positions (T09).
    pub camera: Camera,
    /// `h0+$519e`: set by a scroll that crosses a cell and by the level-3 handler after every pass;
    /// cleared by `main_loop`, which the engine does not run yet.
    pub frame_flag: u16,
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
    /// The character's animation frame and what picks it (T51).
    pub anim: Anim,
    /// The joystick flags `read_joystick` sets each pass (T51).
    pub joy: JoyFlags,
    /// `input_log` and `input_log_index` (T51).
    pub input_log: InputLog,
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
            // `camera_update` in play's first pass has already followed the character, from (0, 0):
            // down `$17` pixels across the cell edge at `$10`, and left across 0, clamped back. Both
            // latched at (0, `$17`); `main_loop` has not cleared them yet (measured, T09).
            camera: Camera {
                x: 0,
                y: 0x17,
                screen_x: 0x70,
                screen_y: 0x70,
                edges: EdgeFlags {
                    crossed: [false, true, false, true],
                    latch: [0, 0xffff, 0, 0xffff],
                    at: [(0, 0), (0, 0x17), (0, 0), (0, 0x17)],
                },
            },
            frame_flag: 0,
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
            // The program's own initial frame, `h0+$419c8`, at the oracle's hunk 0 base: no picker
            // has run (`char_anim_ptr`'s note, `character::anim`). The rest are 0 as loaded.
            anim: Anim {
                ptr: 0x00c4_91a0,
                ..Anim::default()
            },
            // Idle: play's first pass read no direction and appended its one byte, 0.
            joy: JoyFlags::default(),
            input_log: InputLog::at(1),
        }
    }

    /// The value of the variable called `name`, zero-extended, or `None` if the state has no such
    /// variable.
    pub fn get(&self, name: &str) -> Option<u32> {
        Some(match name {
            "mt_counter" => self.mt_counter.into(),
            "char_x" => self.char_x,
            "char_y" => self.char_y,
            "camera_x" => self.camera.x.into(),
            "camera_y" => self.camera.y.into(),
            "char_screen_x" => self.camera.screen_x & 0xffff,
            "edge_latch_right" => self.camera.edges.latch[0].into(),
            "edge_latch_left" => self.camera.edges.latch[1].into(),
            "edge_latch_up" => self.camera.edges.latch[2].into(),
            "edge_latch_down" => self.camera.edges.latch[3].into(),
            "edge_at_right_x" => self.camera.edges.at[0].0.into(),
            "edge_at_right_y" => self.camera.edges.at[0].1.into(),
            "edge_at_left_x" => self.camera.edges.at[1].0.into(),
            "edge_at_left_y" => self.camera.edges.at[1].1.into(),
            "edge_at_up_x" => self.camera.edges.at[2].0.into(),
            "edge_at_up_y" => self.camera.edges.at[2].1.into(),
            "edge_at_down_x" => self.camera.edges.at[3].0.into(),
            "edge_at_down_y" => self.camera.edges.at[3].1.into(),
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
            "char_walk_t" => self.anim.walk_t.into(),
            "char_facing" => self.anim.facing.into(),
            "char_probe_1" => self.anim.probe_1.into(),
            "char_probe_2" => self.anim.probe_2.into(),
            "char_anim_ptr" => self.anim.ptr,
            "joy_left" => self.joy.left.into(),
            "joy_right" => self.joy.right.into(),
            "joy_up" => self.joy.up.into(),
            "joy_down" => self.joy.down.into(),
            "input_log_index" => self.input_log.index,
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
            "camera_x" => self.camera.x = w,
            "camera_y" => self.camera.y = w,
            "char_screen_x" => {
                self.camera.screen_x = (self.camera.screen_x & 0xffff_0000) | w as u32
            }
            "edge_latch_right" => self.camera.edges.latch[0] = w,
            "edge_latch_left" => self.camera.edges.latch[1] = w,
            "edge_latch_up" => self.camera.edges.latch[2] = w,
            "edge_latch_down" => self.camera.edges.latch[3] = w,
            "edge_at_right_x" => self.camera.edges.at[0].0 = w,
            "edge_at_right_y" => self.camera.edges.at[0].1 = w,
            "edge_at_left_x" => self.camera.edges.at[1].0 = w,
            "edge_at_left_y" => self.camera.edges.at[1].1 = w,
            "edge_at_up_x" => self.camera.edges.at[2].0 = w,
            "edge_at_up_y" => self.camera.edges.at[2].1 = w,
            "edge_at_down_x" => self.camera.edges.at[3].0 = w,
            "edge_at_down_y" => self.camera.edges.at[3].1 = w,
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
            "char_walk_t" => self.anim.walk_t = b,
            "char_facing" => self.anim.facing = b,
            "char_probe_1" => self.anim.probe_1 = b,
            "char_probe_2" => self.anim.probe_2 = b,
            "char_anim_ptr" => self.anim.ptr = value,
            "joy_left" => self.joy.left = w,
            "joy_right" => self.joy.right = w,
            "joy_up" => self.joy.up = w,
            "joy_down" => self.joy.down = w,
            "input_log_index" => self.input_log.index = value,
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
