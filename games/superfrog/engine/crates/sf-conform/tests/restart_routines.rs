//! What follows a level's loop (T37, `sf_core::level::exit`) against the original's routines on
//! the routine oracle: `level_start` with its waits ended at once (the rig's raster waits end;
//! its fade is set up, not stepped, as no vertical blank runs), and `game_over_check`'s choice.
//! `level_exit_fade` waits for the vertical blank's fade, which the rig does not run: its frames
//! are proven by `frog-pit-w1-1` (frames 277–292).

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::front::fade;
use sf_core::layout::{h0, h2};
use sf_core::level::{LevelCtx, LevelOutcome, exit, load, panel};

/// `frog_lives` (h0+$82e), `abort_flag` (h0+$9de), `level_state` (h0+$a06), `attract_mode`
/// (h0+$18).
const LIVES: u32 = h0(0x82e);
const ABORT_FLAG: u32 = h0(0x9de);
const LEVEL_STATE: u32 = h0(0xa06);
/// The frog's coins (`+$5c`) and score (`+$5e`), the clock's digits (h0+$876), the game mode
/// (h0+$9e6).
const COINS: u32 = h0(0x7d6 + 0x5c);
const SCORE: u32 = h0(0x7d6 + 0x5e);
const CLOCK: u32 = h0(0x876);
const GAME_MODE: u32 = h0(0x9e6);

/// `level_start` as the engine runs it on the beam (`exit::LevelStart`), its waits taken out: the
/// frog's step, the level's copper list, the fade in's set-up (interrupts off, `$153a4`), the
/// sprites on, the slide's 24 steps, the panel's contents, the drop-in.
fn level_start(c: &mut LevelCtx) {
    load::start_frog(c);
    let list = h2(0x6bd0);
    c.custom(0x80, (list >> 16) as u16);
    c.custom(0x82, list as u16);
    let rec = fade::RECORDS[0];
    c.bus.set_w(rec.at + fade::SPEED, 1);
    c.custom(0x9a, fade::INTENA_OFF);
    c.bus.set_w(h0(0x3ca0), 0);
    let target = c.bus.l(h0(0x9fe));
    fade::fade_in(c.bus.st, rec, list + 0x2d4, target, 0x20);
    c.custom(0x9a, fade::INTENA_ON);
    c.custom(0x96, 0x8020);
    exit::start_slide(c);
    for k in 0..panel::SLIDE_STEPS {
        panel::slide_step(&mut c.bus, k);
    }
    exit::start_after_slide(c);
}

#[test]
fn level_start_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(37);
    let mut ran = 0;
    for case in 0..40 {
        let mut s = Setup::default();
        s.poke_w(LIVES, g.below(10) as u16);
        s.poke_w(COINS, g.pick(&[0, 1, 9, 10, 42, 99]));
        s.poke_l(SCORE, g.below(10_000_000) as u32);
        for k in 0..4 {
            s.poke_b(CLOCK + k, g.below(if k == 2 { 6 } else { 10 }) as u8);
        }
        s.poke_w(GAME_MODE, g.below(3) as u16);
        if rig.check("level_start", case, 0x6952, &s, level_start) {
            ran += 1;
        }
    }
    assert_eq!(ran, 40, "level_start returns from every set-up");
}

/// `game_over_check` (h0+$749c) returns, for the level again or the tally, exactly when the
/// engine's outcome is a death with lives left or the exit; on ESC or the last life it goes to
/// the title (and does not return), which is game over.
#[test]
fn game_over_check_chooses_as_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(38);
    for case in 0..200 {
        let mut s = Setup::default();
        let lives = g.pick(&[0xffff, 0, 1, 2]);
        let abort = g.pick(&[0, 0, 1]);
        let state = g.pick(&[0xffff_ffffu32, 0xffff_fffe, 0, 1]);
        s.poke_w(LIVES, lives);
        s.poke_w(ABORT_FLAG, abort);
        s.poke_l(LEVEL_STATE, state);
        let mut st = rig.state().clone();
        st.set_w(LIVES, lives);
        st.set_w(ABORT_FLAG, abort);
        st.set_l(LEVEL_STATE, state);
        let outcome = exit::outcome_of_loop(&st);
        let returned = rig.check("game_over_check", case, 0x749c, &s, |_| {});
        let goes_on = matches!(outcome, LevelOutcome::Died | LevelOutcome::Completed { .. });
        assert_eq!(returned, goes_on, "case {case}: {outcome:?}, lives {lives:x}, abort {abort}");
    }
}
