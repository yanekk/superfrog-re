//! The level's game state: the variables a case compares, typed at the original's widths
//! (`layout.rs`, docs/09-play-measured.md "Variables"). A porting task that models more of the
//! original adds its fields here and, if a case compares them, a `LAYOUT` entry and an arm in
//! [`State::get`] and [`State::set`].

use crate::character::{Anim, InputLog, JoyFlags, Motion};

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
    /// The character's animation frame and what picks it (T51).
    pub anim: Anim,
    /// The joystick flags `read_joystick` sets each pass (T51).
    pub joy: JoyFlags,
    /// `input_log` and `input_log_index` (T51).
    pub input_log: InputLog,
    /// The character's movement state (T08).
    pub motion: Motion,
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
            // The program's own initial frame, `h0+$419c8`, at the oracle's hunk 0 base: no picker
            // has run (`char_anim_ptr`'s note, `character::anim`). The rest are 0 as loaded.
            anim: Anim {
                ptr: 0x00c4_91a0,
                ..Anim::default()
            },
            // Idle: play's first pass read no direction and appended its one byte, 0.
            joy: JoyFlags::default(),
            input_log: InputLog::at(1),
            // Measured at the sync point (`level_entry.rs`). `char_update_t` is 3 there, not the 0
            // a whole `char_update` leaves: why is not known. `char_unhurt_t` is counted by a routine
            // another task ports (T59: contact hurts once it wraps to 0). The rest are 0.
            motion: Motion {
                update_t: 3,
                unhurt_t: 0x9c,
                ..Motion::default()
            },
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
            "char_jump_3b28" => self.motion.jump_3b28.into(),
            "char_jump_41b4" => self.motion.jump_41b4.into(),
            "char_jump_41b6" => self.motion.jump_41b6.into(),
            "char_dying_t" => self.motion.dying_t.into(),
            "char_dying" => self.motion.dying.into(),
            "char_rising" => self.motion.rising.into(),
            "char_falling" => self.motion.falling.into(),
            "char_probe_3" => self.motion.probe_3.into(),
            "char_probe_4" => self.motion.probe_4.into(),
            "char_probe_5" => self.motion.probe_5.into(),
            "char_probe_6" => self.motion.probe_6.into(),
            "char_slide_t" => self.motion.slide_t.into(),
            "char_jumped" => self.motion.jumped.into(),
            "char_jump_t" => self.motion.jump_t.into(),
            "char_run_left" => self.motion.run_left,
            "char_run_right" => self.motion.run_right,
            "char_samples_0" => u32::from_be_bytes(self.motion.samples[0..4].try_into().unwrap()),
            "char_samples_4" => u32::from_be_bytes(self.motion.samples[4..8].try_into().unwrap()),
            "char_samples_8" => u32::from_be_bytes(self.motion.samples[8..12].try_into().unwrap()),
            "char_samples_c" => u32::from_be_bytes(self.motion.samples[12..16].try_into().unwrap()),
            "char_update_t" => self.motion.update_t,
            "char_unhurt_t" => self.motion.unhurt_t.into(),
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
            "char_jump_3b28" => self.motion.jump_3b28 = w,
            "char_jump_41b4" => self.motion.jump_41b4 = w,
            "char_jump_41b6" => self.motion.jump_41b6 = w,
            "char_dying_t" => self.motion.dying_t = w,
            "char_dying" => self.motion.dying = w,
            "char_rising" => self.motion.rising = b,
            "char_falling" => self.motion.falling = b,
            "char_probe_3" => self.motion.probe_3 = b,
            "char_probe_4" => self.motion.probe_4 = b,
            "char_probe_5" => self.motion.probe_5 = b,
            "char_probe_6" => self.motion.probe_6 = b,
            "char_slide_t" => self.motion.slide_t = b,
            "char_jumped" => self.motion.jumped = b,
            "char_jump_t" => self.motion.jump_t = b,
            "char_run_left" => self.motion.run_left = value,
            "char_run_right" => self.motion.run_right = value,
            "char_samples_0" => self.motion.samples[0..4].copy_from_slice(&value.to_be_bytes()),
            "char_samples_4" => self.motion.samples[4..8].copy_from_slice(&value.to_be_bytes()),
            "char_samples_8" => self.motion.samples[8..12].copy_from_slice(&value.to_be_bytes()),
            "char_samples_c" => self.motion.samples[12..16].copy_from_slice(&value.to_be_bytes()),
            "char_update_t" => self.motion.update_t = value,
            "char_unhurt_t" => self.motion.unhurt_t = b,
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
