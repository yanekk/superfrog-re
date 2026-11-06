//! What ends a life or the game (plans/forest-dumb-level-1 T23, DESIGN §2.4): the countdown, the
//! keys play acts on (P pauses, Esc gives up, HELP ends the level with the cheat on) and the lives.
//! Ported from `countdown_step` (`h0+$20b0`), `play_keys` (`h0+$2104`) with `help_key` and
//! `esc_key`, and `lives_minus_one` (`h0+$69d6`); `fd-conform/tests/death_routines.rs` holds each
//! against the original on the routine rig. `main_loop` turns `give_up`, `lives` 0 and `countdown`
//! 0 into the game over screen (`crate::game_over`).
//!
//! A life is lost only in `char_hit_check` (`h0+$3b2a`, T70): three passes in a row with contact,
//! the CLXDAT bits the frame the display showed sets (`crate::render::contact`). Its creature half,
//! the stomp of `h0_3dd8` while falling, is T24's ([`Play::stomp`]).
//!
//! None of these can be stopped partway: they run inside `game_frame`, which nothing interrupts
//! (docs/09-play-measured.md, "The character's update"). What they read that is set elsewhere is
//! `last_key_byte`, which the keyboard interrupt writes between passes ([`Keys::feed`]).
//!
//! The counters are BCD and step by `sbcd`, whose X flag the routines never set before the low
//! byte: they treat it as clear, as the rig enters them, except `countdown_step`, whose
//! `addi.l #1, countdown_tick` leaves X as its carry.

use amiga_input::KeySet;

use crate::objects::collectible::bcd_add;
use crate::objects::creature;
use crate::program::{h, Scratch};
use crate::render::contact::CONTACT;
use crate::State;

/// `countdown_tick` counts passes to this, then the countdown loses one (`cmpi.l #$3f`).
pub const TICKS_PER_COUNT: u32 = 0x3f;

/// Passes in a row with contact that cost a life (`cmpi.w #3, hits_taken`).
pub const HITS_PER_LIFE: u16 = 3;

/// The hurt sound `char_hit_check` starts on audio channel 1: its sample's hunk 4 offset, length in
/// words, period and volume (`crate::program::h`). Sound is out of scope (DESIGN §8); the writes are
/// kept for the rig.
pub const HURT_SAMPLE: u32 = 0x00c8d0;
pub const HURT_LENGTH: u16 = 0x15a4;
pub const HURT_PERIOD: u16 = 0x00ff;
pub const HURT_VOLUME: u16 = 0x003f;
/// The stomp sound `h0_3adc` starts on audio channel 2, as [`HURT_SAMPLE`]'s.
pub const STOMP_SAMPLE: u32 = 0x010df4;
pub const STOMP_LENGTH: u16 = 0x0e6c;
pub const STOMP_PERIOD: u16 = 0x0165;
pub const STOMP_VOLUME: u16 = 0x003f;
const AUD2_OFF: u16 = 0x0004;
const AUD2_ON: u16 = 0x8204;
/// What a stomp scores, BCD.
pub const STOMP_SCORE: u32 = 0x250;
/// The point a stomp tests against a creature's box: below the character's feet.
pub const FEET_X: u32 = 0x10;
pub const FEET_Y: u32 = 0x24;
/// DMACON, and the writes that stop and start audio channel 1's DMA.
const DMACON: u16 = 0x96;
const AUD1_OFF: u16 = 0x0002;
const AUD1_ON: u16 = 0x8202;

/// `last_key_byte` as the keyboard sends a key: the raw code shifted left over its up bit, all
/// inverted (the CIA's serial byte, which `level2_handler` stores unchanged).
pub const fn key_byte(code: u8, up: bool) -> u8 {
    !((code << 1) | up as u8)
}

/// Raw key codes play acts on.
pub const KEY_P: u8 = 0x19;
pub const KEY_ESC: u8 = 0x45;
pub const KEY_HELP: u8 = 0x5f;

/// The keyboard as play sees it, and the keys' latches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Keys {
    /// `last_key_byte` (`h0+$1f78`): the last key event's serial byte ([`key_byte`]).
    pub last: u8,
    /// `pause_key_down` (`h0+$2102`), word: P is down; its release toggles `paused`.
    pub pause_down: u16,
    /// `esc_down` (`h0+$21b0`), word: Esc is down; its release sets `give_up`.
    pub esc_down: u16,
    /// `cheat_flag` (`h0+$780c`), word: HELP ends the level. Set only by the title's cheat code.
    pub cheat: u16,
    /// The keys held at the last pass: not the original's memory, but what tells the engine which
    /// keys changed since. A case starts it empty.
    pub held: KeySet,
}

impl Keys {
    /// `level2_handler` for the keys that changed since the last pass: each event puts its byte in
    /// `last_key_byte`, so the last one stays. Releases are taken before presses, each lowest code
    /// first; which the keyboard sends first when two change in one frame is not measured, and
    /// play never reads more than the last.
    pub fn feed(&mut self, now: KeySet) {
        let (up, down) = (self.held.0 & !now.0, now.0 & !self.held.0);
        for code in KeySet(up).codes() {
            self.last = key_byte(code, true);
        }
        for code in KeySet(down).codes() {
            self.last = key_byte(code, false);
        }
        self.held = now;
    }
}

/// `sbcd`: `dst - src - x` in BCD, digit by digit as the rig's 68000 computes it on any byte,
/// valid digits or not (a digit borrowing loses 6 more, masked to four bits); the borrow out of the
/// high digit is the new X. A carry-chain formula differs on bad digits (`$be - 1`).
pub fn sbcd(dst: u8, src: u8, x: bool) -> (u8, bool) {
    let digit = |d: u8, s: u8, borrow: bool| {
        let r = i16::from(d) - i16::from(s) - i16::from(borrow);
        let borrow = r < 0;
        (((r - 6 * i16::from(borrow)) & 0xf) as u8, borrow)
    };
    let (lo, borrow) = digit(dst & 0xf, src & 0xf, x);
    let (hi, borrow) = digit(dst >> 4, src >> 4, borrow);
    (hi << 4 | lo, borrow)
}

/// A BCD word less one, with the X flag the first `sbcd` enters with: two `sbcd`s, low byte first.
fn bcd_word_minus_one(w: u16, x: bool) -> u16 {
    let [hi, lo] = w.to_be_bytes();
    let (lo, x) = sbcd(lo, 1, x);
    let (hi, _) = sbcd(hi, 0, x);
    u16::from_be_bytes([hi, lo])
}

/// The rules of a level's end of life and game, on the game state.
pub struct Play<'a> {
    pub state: &'a mut State,
}

impl Play<'_> {
    /// `countdown_step` (`h0+$20b0`), once a pass: `countdown_tick` counts to
    /// [`TICKS_PER_COUNT`], then clears and the countdown loses one, unless it is already 0.
    pub fn countdown_step(&mut self) {
        let s = &mut *self.state;
        let (tick, carry) = s.countdown_tick.overflowing_add(1);
        s.countdown_tick = tick;
        // `cmpi.l #$3f` then `bpl`: the sign of the difference, so a tick past `$8000003e` counts too.
        if (tick.wrapping_sub(TICKS_PER_COUNT) as i32) < 0 {
            return;
        }
        s.countdown_tick = 0;
        let count = (s.countdown >> 16) as u16;
        if count == 0 {
            return;
        }
        // `addi.l` left its carry in X, which the first `sbcd` subtracts too.
        let count = bcd_word_minus_one(count, carry);
        s.countdown = u32::from(count) << 16 | s.countdown & 0xffff;
    }

    /// `play_keys` (`h0+$2104`), at a pass's end and in a paused pass: `help_key`, `esc_key`, then P
    /// down latches `pause_key_down` and its release toggles `paused`.
    pub fn play_keys(&mut self) {
        self.help_key();
        self.esc_key();
        let k = &mut self.state.keys;
        if k.last == key_byte(KEY_P, false) {
            k.pause_down = 1;
        } else if k.last == key_byte(KEY_P, true) && k.pause_down != 0 {
            self.state.paused = !self.state.paused;
            k.pause_down = 0;
        }
    }

    /// `help_key` (`h0+$2146`): HELP with the cheat on touches the exit with no items left, and
    /// forgets the key.
    pub fn help_key(&mut self) {
        let s = &mut *self.state;
        if s.keys.last == key_byte(KEY_HELP, false) && s.keys.cheat != 0 {
            s.level_exit_touched = 1;
            s.items_left = 0;
            s.keys.last = 0;
        }
    }

    /// `esc_key` (`h0+$2176`): Esc down latches `esc_down`; its release with the latch set gives
    /// up and clears it.
    pub fn esc_key(&mut self) {
        let k = &mut self.state.keys;
        if k.last == key_byte(KEY_ESC, false) {
            k.esc_down = 1;
        } else if k.last == key_byte(KEY_ESC, true) && k.esc_down != 0 {
            self.state.give_up = 1;
            k.esc_down = 0;
        }
    }

    /// `char_hit_check` (`h0+$3b2a`), once a pass after `read_joystick`, with CLXDAT as the hardware
    /// reads it: copies it to `clxdat_copy`; no [`CONTACT`] bit clears `hits_taken`. Contact while
    /// dying does nothing; while falling, a stomped creature ([`Play::stomp`]) spares the pass;
    /// otherwise, once `char_unhurt_t` is 0, `hits_taken` counts it, and the third in a row (with the
    /// cheat off) starts the dying fall, takes a life, stops the run and jump, and starts the hurt
    /// sound, its custom-chip writes pushed on `scratch.hw`.
    pub fn char_hit_check(&mut self, clxdat: u16, scratch: &mut Scratch) {
        let s = &mut *self.state;
        s.clxdat_copy = clxdat;
        if clxdat & CONTACT == 0 {
            s.hits_taken = 0;
            return;
        }
        if s.motion.dying != 0 {
            return;
        }
        if s.motion.falling != 0 && self.stomp(scratch) {
            return;
        }
        let s = &mut *self.state;
        if s.motion.unhurt_t != 0 {
            return;
        }
        s.hits_taken = s.hits_taken.wrapping_add(1);
        // `cmpi.w #3` then `bmi`: the sign of the difference, so a count past `$8002` counts too.
        if (s.hits_taken.wrapping_sub(HITS_PER_LIFE) as i16) < 0 || s.keys.cheat != 0 {
            return;
        }
        s.motion.dying = 1;
        self.lives_minus_one();
        let m = &mut self.state.motion;
        (m.run_left, m.run_right) = (0, 0);
        (m.rising, m.jumped, m.jump_t) = (0, 0, 0);
        // The hurt sound: channel 1 off, a wait of three beam lines (reads only), its sample, on.
        let sample = h(4, HURT_SAMPLE);
        scratch.hw.extend([
            (DMACON, AUD1_OFF),
            (0xb0, (sample >> 16) as u16),
            (0xb2, sample as u16),
            (0xb4, HURT_LENGTH),
            (0xb6, HURT_PERIOD),
            (0xb8, HURT_VOLUME),
            (DMACON, AUD1_ON),
        ]);
    }

    /// The creature half of `char_hit_check` (`h0+$3b66` to `$3d34`, T24), while falling: the first
    /// active slot, 0 to 4, whose box ([`creature::overlaps`], `h0_3dd8`) holds the point
    /// (`char_x + $10`, `char_y + $24`) and is not already stomped is stomped: kind
    /// [`creature::STOMPED`], phase 0, `hits_taken` cleared, the bounce (`char_jump_3b28` and
    /// `char_rising` set), [`STOMP_SCORE`] added through `score_add` and the stomp sound (`h0_3adc`)
    /// on audio channel 2, its writes pushed on `scratch.hw`. `true` when one was: the pass counts
    /// no hit.
    pub fn stomp(&mut self, scratch: &mut Scratch) -> bool {
        let s = &mut *self.state;
        let px = s.char_x.wrapping_add(FEET_X) as u16;
        let py = s.char_y.wrapping_add(FEET_Y) as u16;
        let c = &mut s.edges.creatures;
        let Some(i) = (0..creature::SLOTS).find(|&i| {
            c.active[i] != 0
                && creature::overlaps(&c.slots[i], px, py)
                && c.slots[i].kind != creature::STOMPED
        }) else {
            return false;
        };
        (c.slots[i].kind, c.slots[i].phase) = (creature::STOMPED, 0);
        s.hits_taken = 0;
        s.motion.jump_3b28 = 1;
        s.motion.rising = 1;
        // `score_add_routine`: added, then `score_add` cleared.
        s.score = bcd_add(s.score, STOMP_SCORE);
        s.collect.score_add = 0;
        // `h0_3adc`: channel 2 off, a wait of four beam lines (reads only), its sample, on.
        let sample = h(4, STOMP_SAMPLE);
        scratch.hw.extend([
            (DMACON, AUD2_OFF),
            (0xc0, (sample >> 16) as u16),
            (0xc2, sample as u16),
            (0xc4, STOMP_LENGTH),
            (0xc6, STOMP_PERIOD),
            (0xc8, STOMP_VOLUME),
            (DMACON, AUD2_ON),
        ]);
        true
    }

    /// `lives_minus_one` (`h0+$69d6`): one life less in BCD, unless none is left.
    pub fn lives_minus_one(&mut self) {
        let s = &mut *self.state;
        if s.lives != 0 {
            s.lives = bcd_word_minus_one(s.lives, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_play_reads_have_the_bytes_the_original_compares() {
        // `esc_key`'s `$75`/`$74`, `play_keys`' `$cd`/`$cc`, `help_key`'s `$41`.
        assert_eq!(key_byte(KEY_ESC, false), 0x75);
        assert_eq!(key_byte(KEY_ESC, true), 0x74);
        assert_eq!(key_byte(KEY_P, false), 0xcd);
        assert_eq!(key_byte(KEY_P, true), 0xcc);
        assert_eq!(key_byte(KEY_HELP, false), 0x41);
    }

    #[test]
    fn a_held_key_is_one_event_and_its_release_another() {
        let mut k = Keys::default();
        k.feed(KeySet::key(KEY_ESC));
        assert_eq!(k.last, 0x75);
        k.last = 0;
        k.feed(KeySet::key(KEY_ESC));
        assert_eq!(k.last, 0, "held, nothing new");
        k.feed(KeySet::NONE);
        assert_eq!(k.last, 0x74);
    }

    #[test]
    fn bcd_steps_down_as_the_68000s() {
        assert_eq!(bcd_word_minus_one(0x32, false), 0x31);
        assert_eq!(bcd_word_minus_one(0x100, false), 0x99);
        assert_eq!(bcd_word_minus_one(0x999, false), 0x998);
        assert_eq!(bcd_word_minus_one(0x01, false), 0x00);
        assert_eq!(sbcd(0, 1, false), (0x99, true));
    }
}
