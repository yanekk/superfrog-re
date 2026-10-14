//! The slot machine's rounds (T73): from the attract loop's fire to `slot_machine`'s return. Each
//! round (h0+$4d4a) is the hold menu ([`HoldMenu`]) and the spin ([`Spin`]) unless a free spin
//! skips them, the payout check ([`payout`], its line's lamp flashed), the award (a line's prize
//! collected; the ladder's gamble, the hi-lo game and the jackpot are T74's), the win counted into
//! the score ([`Count`]) and the free spin's chance; rounds go on while coins are left. Then 100
//! frames or fire, the picture faded out, the score taken, the music faded, and the level's sound
//! put back.
//!
//! The drawing runs on the beam as the attract's does ([`super::draw`]): the lamps from the
//! raster wait into the next frame, the spin's digits (CPU byte copies, [`super::draw::Work`]) and
//! reels through the interrupt and down the picture.

use super::draw::{Draw, Part, Work};
use super::{
    COINS, COUNT_CYCLES, COUNT_OVER, COUNT_SHOWN, COUNT_TEXT, DIGITS_CYCLES, HILO, LAMP_COUNT,
    LAMPS, LAMPS_OFF_CYCLES, MENU, POINTER_X, POINTER_Y, REELS, ROW, SCORE, SCORE_SHOWN,
    SLOT_COLOURS, STATE_49D0, STATE_49D2, STATE_49DA, WIN,
};
use crate::front::common::{FadeKind, call, fade, frames};
use crate::front::{Cx, Routine as FrontRoutine, Yield, at};
use crate::state::State;

/// The holds (h0+$6154, $6156, $6158), holds allowed (h0+$615a), the gamble offered (h0+$615c)
/// and taken (h0+$615e), the flashing lamps' list (h0+$6160) and the lit ones' (h0+$61cc), both
/// lamp records' addresses ended by -1.
const HOLDS: u32 = at(0x6154);
const HOLDS_ALLOWED: u32 = at(0x615a);
const OFFER: u32 = at(0x615c);
const GAMBLE: u32 = at(0x615e);
const FLASHING: u32 = at(0x6160);
const LIT: u32 = at(0x61cc);
const LIST_END: u32 = 0xffff_ffff;
/// The menu's frame count (h0+$603e), its pointer stop (h0+$6064) and the stops' x (h0+$606a,
/// -1 before and after).
const FLASH_COUNT: u32 = at(0x603e);
const CURSOR: u32 = at(0x6064);
const STOPS: u32 = at(0x606a);
/// The hi-lo game's pick (h0+$598a: 1 higher, 2 lower) and the spin's (h0+$598c).
const HILO_PICK: u32 = at(0x598a);
const SPIN_PICKED: u32 = at(0x598c);
/// `slot_spin` (h0+$5dc6): the hi-lo number's toggle, the three-bars flag, the holds as the spin
/// began (h0+$5dca, $5dce, $5dd2) and the reels' counters (longs, h0+$5dd6..).
const SPIN_TOGGLE: u32 = at(0x5dc6);
const BARS: u32 = at(0x5dc8);
const SAVED_HOLDS: u32 = at(0x5dca);
const COUNTERS: u32 = at(0x5dd6);
/// Lamp records: the spin's button, the hold buttons (reels 1, 2, 3) and the free spin's lamp.
const SPIN_LAMP: u32 = at(0x68d2);
const HOLD_LAMPS: [u32; 3] = [at(0x68ca), at(0x68c2), at(0x68ba)];
const FREE_SPIN_LAMP: u32 = at(0x690a);
/// `world`, `level` (longs) and the frog's lives (`frog_lives`).
const WORLD: u32 = at(0x40da);
const LEVEL: u32 = at(0x40de);
pub(super) const LIVES: u32 = at(0x82e);
/// `fades`' first record and the blank copper list `slot_machine` ends on.
const BLANK_COPPER: u32 = crate::front::h2(0x6fd8);

/// 68000 cycles, measured on the oracle (T73): from a `bsr slot_draw_lamps` to its INTENA
/// `$0010` (the blitter wait, the move); the menu's frame before it (the count, the lists); the
/// spin's work before its reels (the counters and stops); `slot_digits` on the spin's frame.
const LAMPS_CALL: u32 = 60;
const MENU_FRAME: u32 = 240;
const MENU_ENTRY: u32 = 400;
const SPIN_LEAD: u32 = 150;
const REELS_CALL: u32 = 150;
/// `slot_digits` while the picture shows: fitted to the spin's reels' INTENA lines over `slot-fire`
/// (1 000 fewer than [`DIGITS_CYCLES`]; the byte copies lose less to the display than
/// [`super::draw`]'s CPU rate says).
const DIGITS_ON_BEAM: u32 = DIGITS_CYCLES - 1_000;
/// The bank's copy back over the level's (h0+$15fb0: $3e80 longs at about 39 cycles).
const SOUND_COPY_CYCLES: u32 = 0x3e80 * 39;

/// `sfx_play`, unless the game has no sound.
fn sfx(cx: &mut Cx, id: u8, chan: u8) {
    let Some(d) = cx.data.sound.as_ref() else {
        return;
    };
    let mut out = Vec::new();
    cx.s.audio.play_sfx(d, id, chan, &mut out);
    for w in out {
        cx.wr(w.reg, w.value);
    }
}

/// `rng_range` (h0+$160c0): a number in `0..=n`.
fn rng(s: &mut State, n: u16) -> u16 {
    let mut r = s.rng();
    let v = r.range(n);
    s.set_rng(r);
    v
}

/// `slot_lamps_off` (h0+$6238): every lamp's `+0` and `+6` cleared.
pub(crate) fn lamps_off(s: &mut State) {
    for k in 0..LAMP_COUNT {
        s.set_w(LAMPS + 8 * k, 0);
        s.set_w(LAMPS + 8 * k + 6, 0);
    }
}

/// `slot_lamps_on` (h0+$6258): every lamp's `+0` set.
fn lamps_on(s: &mut State) {
    for k in 0..LAMP_COUNT {
        s.set_w(LAMPS + 8 * k, 0xffff);
    }
}

/// The lamps drawn on the beam, `lead` cycles of the caller's work before the call.
fn draw_lamps(lead: u32) -> Yield {
    call(FrontRoutine::SlotDraw(Draw::new(Part::Lamps {
        lead: lead + LAMPS_CALL,
    })))
}

/// `slot_digits` on the beam: its memory at once (compared once the original is back at `inside`),
/// its byte copies' time spent.
fn digits(cx: &mut Cx, inside: u32) -> Yield {
    super::slot_digits(cx);
    // The digits' rows, in each plane.
    let mut writes = Vec::new();
    for p in 0..6 {
        for at in [0x1ae0, 0xf00] {
            writes.push((super::BUFFER_0 + p * super::PLANE + at, 0x18 * ROW));
        }
    }
    cx.jobs.push(crate::game::Job {
        inside: vec![inside],
        writes,
    });
    call(FrontRoutine::SlotWork(Work {
        left: DIGITS_ON_BEAM,
    }))
}

/// The lamp records a list holds, up to its -1.
fn list(s: &State, at: u32) -> Vec<u32> {
    (0..)
        .map(|k| s.l(at + 4 * k))
        .take_while(|&a| a != LIST_END)
        .collect()
}

/// `slot_payout` (h0+$4eae) up to its flash (h0+$504e): the reels' line scored, its lamp named in
/// h0+$49d2 and its sound played; returns the prize (`d7`: -1 none, $a the hi-lo game, else the
/// ladder's rung). `sfx` plays an effect on a channel.
pub fn payout(s: &mut State, sfx: &mut dyn FnMut(&mut State, u8, u8)) -> u16 {
    s.set_w(STATE_49D0, 0);
    let reels = [s.w(REELS), s.w(REELS + 2), s.w(REELS + 4)];
    let mut d7 = 0xffff;
    // In the original's order: each line that matches overrides the one before.
    for (line, prize, lamp, effect) in [
        ([8, 0x18, 0], 0xa, None, 0xa),
        ([0x18, 0x18, 0x18], 0, None, 8),
        ([8, 8, 8], 4, Some(0x689a), 4),
        ([0, 0, 0], 5, Some(0x68a2), 3),
        ([8, 8, 0x18], 7, Some(0x68aa), 1),
        ([0, 0, 0x18], 8, Some(0x68b2), 0),
    ] {
        if reels == line {
            d7 = prize;
            if let Some(l) = lamp {
                s.set_l(STATE_49D2, at(l));
            }
            sfx(s, effect, 3);
        }
    }
    if d7 != 0xffff {
        s.set_w(STATE_49DA, 1);
        sfx(s, 0xc, 2);
    }
    d7
}

/// `slot_reel_stop` (h0+$5d48): reel `reel` (0..3) at `d0` stops on 0, 9 or $18 (a bar, a
/// symbol's middle): its position's low bit set, its hold set, its sound played on channel 3.
pub fn reel_stop(s: &mut State, reel: u32, d0: u16, sfx: &mut dyn FnMut(&mut State, u8, u8)) {
    for stop in [0, 9, 0x18] {
        if d0 == stop {
            let r = REELS + 2 * reel;
            let v = s.w(r) | 1;
            s.set_w(r, v);
            s.set_w(HOLDS + 2 * reel, 1);
            sfx(s, 0x11 + reel as u8, 3);
        }
    }
}

/// The round loop (h0+$4d4a) and `slot_machine`'s end (h0+$4d82..$4e26).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Rounds {
    pc: u8,
    /// The prize the payout named (`d7`).
    prize: u16,
    /// The end's frames left (`d7`).
    left: u16,
}

impl Rounds {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        use crate::audio::DMACON;
        loop {
            match self.pc {
                0 => {
                    cx.wr(DMACON, 0x8020);
                    if cx.s.w(STATE_49D0) != 0 {
                        self.pc = 3;
                        continue;
                    }
                    self.pc = 1;
                    return call(FrontRoutine::SlotHold(HoldMenu::default()));
                }
                1 => {
                    self.pc = 3;
                    return call(FrontRoutine::SlotSpin(Spin::default()));
                }
                3 => {
                    let mut played = Vec::new();
                    self.prize = payout(cx.s, &mut |_, id, ch| played.push((id, ch)));
                    for (id, ch) in played {
                        sfx(cx, id, ch);
                    }
                    self.pc = 4;
                    if self.prize != 0xffff {
                        return call(FrontRoutine::SlotFlash(Flash::payout()));
                    }
                }
                4 => {
                    self.pc = 5;
                    cx.s.set_l(WIN, 0);
                    if self.prize != 0xffff {
                        return call(FrontRoutine::SlotAward(Award::new(self.prize)));
                    }
                }
                5 => {
                    self.pc = 6;
                    return call(FrontRoutine::SlotCount(Count::on_beam()));
                }
                6 => {
                    cx.s.set_l(STATE_49D2, 0);
                    self.pc = 7;
                    // `slot_free_spin` (h0+$4e28).
                    if cx.s.l(WIN) != 0 {
                        cx.s.set_w(STATE_49D0, 0);
                        if cx.s.w(COINS) != 0 && (rng(cx.s, 0x64).wrapping_sub(6) as i16) < 0 {
                            cx.s.set_w(STATE_49D0, 1);
                            return call(FrontRoutine::SlotFlash(Flash::free_spin()));
                        }
                    }
                }
                7 => {
                    if (cx.s.w(COINS).wrapping_sub(1) as i16) >= 0 {
                        self.pc = 0;
                        continue;
                    }
                    self.left = 0x64;
                    self.pc = 8;
                }
                8 => {
                    self.pc = 9;
                    return Yield::Raster;
                }
                9 => {
                    self.left = self.left.wrapping_sub(1);
                    self.pc = if cx.fire() || cx.mouse_button() || self.left == 0 {
                        10
                    } else {
                        8
                    };
                }
                10 => {
                    let rec = crate::front::fade::RECORDS[0];
                    cx.s.set_w(rec.at + crate::front::fade::SPEED, 1);
                    self.pc = 11;
                    return fade(FadeKind::Out, 0, SLOT_COLOURS, 0x20, at(0x4dd0));
                }
                11 => {
                    self.pc = 12;
                    return Yield::Fade(crate::front::fade::RECORDS[0]);
                }
                12 => {
                    cx.cop1lc = BLANK_COPPER;
                    let shown = cx.s.l(SCORE_SHOWN);
                    cx.s.set_l(SCORE, shown);
                    self.pc = 13;
                    return call(FrontRoutine::MusicFadeWait(Default::default()));
                }
                13 => {
                    if (cx.s.l(LEVEL).wrapping_sub(4) as i32) >= 0 {
                        return Yield::Return;
                    }
                    self.pc = 14;
                    sound_back(cx);
                    return crate::front::ending::work(
                        SOUND_COPY_CYCLES,
                        &[at(0x4e1a)],
                        Vec::new(),
                    );
                }
                _ => {
                    if let Some(d) = cx.data.sound.as_ref() {
                        cx.s.audio.sfx_bank_init(d);
                        let mut out = Vec::new();
                        cx.s.audio.music_start(d, &mut out);
                        for w in out {
                            cx.wr(w.reg, w.value);
                        }
                    }
                    return Yield::Return;
                }
            }
        }
    }
}

/// h0+$4e02: the level's effect bank and module copied back from where `slot_machine` kept them
/// (h1+$20d00) to their place in chip RAM, as the sound driver sees it.
fn sound_back(cx: &mut Cx) {
    let Some(d) = cx.data.sound.as_ref() else {
        return;
    };
    let module = cx.s.audio.module_addr();
    for (addr, f) in crate::audio::files_for(cx.s.l(WORLD), module) {
        cx.s.audio.file_loaded(d, addr, f);
    }
    for i in 0..super::SOUND_KEPT_LEN {
        let v = cx.s.b(super::SOUND_KEPT_AT + i);
        let a = super::SOUND_KEPT + i;
        if cx.s.audio.mem.b(d, a) != v {
            cx.s.audio.mem.set_b(a, v);
        }
    }
}

/// `slot_hold_menu` (h0+$5de2): the lamps off, the holds cleared and maybe allowed, then the menu.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct HoldMenu {
    pc: u8,
}

impl HoldMenu {
    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        if self.pc == 1 {
            return Yield::Return;
        }
        self.pc = 1;
        lamps_off(cx.s);
        for k in 0..3 {
            cx.s.set_w(HOLDS + 2 * k, 0);
        }
        cx.s.set_w(HOLDS_ALLOWED, 0);
        if cx.s.l(WIN) == 0 && cx.s.w(STATE_49DA) == 0 && rng(cx.s, 8) == 6 {
            cx.s.set_w(HOLDS_ALLOWED, 1);
        }
        cx.s.set_w(STATE_49DA, 0);
        hold_lists(cx.s);
        call(FrontRoutine::SlotMenu(Menu::new(LAMPS_OFF_CYCLES)))
    }
}

/// h0+$5e2e: the spin's lamp flashing, with each unheld reel's hold lamp when holds are allowed
/// (the held ones lit); no gamble offered.
fn hold_lists(s: &mut State) {
    let (mut a0, mut a1) = (FLASHING, LIT);
    s.set_l(a0, SPIN_LAMP);
    a0 += 4;
    if s.w(HOLDS_ALLOWED) != 0 {
        for (k, lamp) in HOLD_LAMPS.into_iter().enumerate() {
            if s.w(HOLDS + 2 * k as u32) == 0 {
                s.set_l(a0, lamp);
                a0 += 4;
            } else {
                s.set_l(a1, lamp);
                a1 += 4;
            }
        }
    }
    s.set_l(a0, LIST_END);
    s.set_l(a1, LIST_END);
    s.set_w(OFFER, 0);
}

/// `slot_run_menu` (h0+$5eb4): the flashing lamps flash every 6th frame and the lit ones stay lit
/// while the pointer moves between its stops, until fire or the mouse's button picks the stop.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Menu {
    pc: u8,
    /// The caller's cycles before the menu's first draw.
    lead: u32,
}

impl Menu {
    fn new(lead: u32) -> Menu {
        Menu { pc: 0, lead }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    cx.s.set_w(GAMBLE, 0);
                    cx.s.set_l(CURSOR, STOPS);
                    for lamp in list(cx.s, FLASHING) {
                        cx.s.set_w(lamp, 0);
                    }
                    let lamp = cx.s.l(STATE_49D2);
                    if lamp != 0 {
                        cx.s.set_w(lamp, 1);
                    }
                    self.pc = 1;
                    return draw_lamps(std::mem::take(&mut self.lead) + MENU_ENTRY);
                }
                1 => {
                    self.pc = 2;
                    return frames(8);
                }
                2 => {
                    cx.s.set_w(FLASH_COUNT, 0);
                    self.pc = 3;
                }
                3 => {
                    self.pc = 4;
                    return Yield::Raster;
                }
                4 => {
                    let n = cx.s.w(FLASH_COUNT).wrapping_add(1);
                    cx.s.set_w(FLASH_COUNT, n);
                    if (n.wrapping_sub(6) as i16) >= 0 {
                        cx.s.set_w(FLASH_COUNT, 0);
                        for lamp in list(cx.s, FLASHING) {
                            let v = !cx.s.w(lamp);
                            cx.s.set_w(lamp, v);
                        }
                    }
                    for lamp in list(cx.s, LIT) {
                        cx.s.set_w(lamp, 0xffff);
                    }
                    self.pc = 5;
                    return draw_lamps(MENU_FRAME);
                }
                5 => {
                    self.pc = 6;
                    if cx.joy1dat() & 0x303 == 3 {
                        let mut c = cx.s.l(CURSOR) + 2;
                        if cx.w(c) == 0xffff {
                            c = STOPS + 8;
                        }
                        self.point(cx, c);
                        return frames(6);
                    }
                }
                6 => {
                    self.pc = 7;
                    if cx.joy1dat() & 0x303 == 0x300 {
                        let mut c = cx.s.l(CURSOR) - 2;
                        if cx.w(c) == 0xffff {
                            c = STOPS;
                        }
                        self.point(cx, c);
                        return frames(6);
                    }
                }
                7 => {
                    if !cx.fire() && !cx.mouse_button() {
                        self.pc = 3;
                        continue;
                    }
                    lamps_off(cx.s);
                    self.pc = 8;
                    return draw_lamps(LAMPS_OFF_CYCLES);
                }
                8 => {
                    sfx(cx, 0xf, 2);
                    let x = cx.s.w(POINTER_X) as i16;
                    let hilo = cx.s.w(MENU) != 0;
                    if x < 0x46 {
                        cx.s.set_w(SPIN_PICKED, 1);
                        let coins = cx.s.w(COINS);
                        if cx.s.w(OFFER) == 0 && coins != 0 {
                            cx.s.set_w(COINS, coins - 1);
                        }
                        self.pc = 9;
                        return digits(cx, at(0x6062));
                    }
                    // h0+$6076..$6150: which stop, then what it does in the hi-lo game
                    // (`slot_menu` set) or out of it.
                    let offer = cx.s.w(OFFER) != 0;
                    let allowed = cx.s.w(HOLDS_ALLOWED) != 0;
                    let (hold, pick) = if x < 0x77 {
                        (Some(0), 1)
                    } else if x < 0xa8 {
                        (Some(1), 0)
                    } else if x < 0xea {
                        (Some(2), 2)
                    } else {
                        (None, 0)
                    };
                    match hold {
                        Some(_) if hilo && pick != 0 => {
                            cx.s.set_w(HILO_PICK, pick);
                            return Yield::Return;
                        }
                        // A hold toggled: the lists again and the menu from its start (the
                        // original pops the menu's return and branches to h0+$5e2e).
                        Some(k) if !hilo && allowed => {
                            let h = HOLDS + 2 * k;
                            let v = !cx.s.w(h);
                            cx.s.set_w(h, v);
                            sfx(cx, 0xe, 3);
                            hold_lists(cx.s);
                        }
                        // The gamble, when offered.
                        None if !hilo && offer => {
                            cx.s.set_w(GAMBLE, 1);
                            return Yield::Return;
                        }
                        // A press that does nothing: the menu again.
                        _ => {}
                    }
                    *self = Menu::new(0);
                }
                _ => return Yield::Return,
            }
        }
    }

    /// The pointer to stop `c`.
    fn point(&self, cx: &mut Cx, c: u32) {
        cx.s.set_l(CURSOR, c);
        let x = cx.w(c);
        cx.s.set_w(POINTER_X, x);
        cx.s.set_w(POINTER_Y, 0xe2);
    }
}

/// `slot_spin_reels` (h0+$5a06).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Spin {
    pc: u8,
}

impl Spin {
    fn holds(s: &State) -> [bool; 3] {
        [0, 1, 2].map(|k| s.w(HOLDS + 2 * k) != 0)
    }

    fn counter(s: &State, k: u32) -> i32 {
        s.l(COUNTERS + 4 * k) as i32
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    lamps_off(cx.s);
                    for (k, lamp) in HOLD_LAMPS.into_iter().enumerate() {
                        let h = cx.s.w(HOLDS + 2 * k as u32);
                        cx.s.set_w(lamp, h);
                    }
                    self.pc = 1;
                    return draw_lamps(LAMPS_OFF_CYCLES);
                }
                1 => {
                    cx.s.set_w(BARS, 0);
                    let r = rng(cx.s, 0x64);
                    if (r.wrapping_sub(0xa) as i16) >= 0 && Spin::holds(cx.s) == [false; 3] {
                        cx.s.set_w(BARS, 1);
                    }
                    self.pc = 2;
                    return frames(10);
                }
                2 => {
                    let a = rng(cx.s, 0x40) as u32 + 0x40;
                    cx.s.set_l(COUNTERS, a);
                    let b = rng(cx.s, 0x40) as u32 + 0x20 + a;
                    cx.s.set_l(COUNTERS + 4, b);
                    let c = rng(cx.s, 0x40) as u32 + 0x20 + b;
                    cx.s.set_l(COUNTERS + 8, c);
                    for k in 0..3 {
                        let h = cx.s.w(HOLDS + 2 * k);
                        cx.s.set_w(SAVED_HOLDS + 4 * k, h);
                    }
                    self.pc = if Spin::holds(cx.s) == [true; 3] { 7 } else { 3 };
                }
                3 => {
                    self.pc = 4;
                    return Yield::Raster;
                }
                4 => {
                    if Spin::holds(cx.s) != [true; 3] {
                        let t = !cx.s.w(SPIN_TOGGLE);
                        cx.s.set_w(SPIN_TOGGLE, t);
                        if t != 0 {
                            let mut h = cx.s.w(HILO).wrapping_add(1);
                            if (h.wrapping_sub(0xb) as i16) >= 0 {
                                h = 1;
                            }
                            cx.s.set_w(HILO, h);
                        }
                    }
                    self.pc = 5;
                    return digits(cx, at(0x5b48));
                }
                5 => {
                    let holds = Spin::holds(cx.s);
                    for k in 0..3 {
                        if !holds[k as usize] && Spin::counter(cx.s, k) >= 0 {
                            let r = REELS + 2 * k;
                            let mut v = cx.s.w(r).wrapping_add(1);
                            if (v.wrapping_sub(0x20) as i16) >= 0 {
                                v = 0;
                            }
                            cx.s.set_w(r, v);
                        }
                    }
                    let mut played = Vec::new();
                    for k in 0..3 {
                        if cx.s.w(HOLDS + 2 * k) == 0 && Spin::counter(cx.s, k) < 0x20 {
                            let d0 = cx.s.w(REELS + 2 * k);
                            reel_stop(cx.s, k, d0, &mut |_, id, ch| played.push((id, ch)));
                        }
                    }
                    for (id, ch) in played {
                        sfx(cx, id, ch);
                    }
                    self.pc = 6;
                    return call(FrontRoutine::SlotDraw(Draw::new(Part::Reels {
                        lead: SPIN_LEAD + REELS_CALL,
                    })));
                }
                6 => {
                    for k in 0..3 {
                        if cx.s.w(SAVED_HOLDS + 4 * k) == 0 && cx.s.w(HOLDS + 2 * k) != 0 {
                            let r = REELS + 2 * k;
                            let v = cx.s.w(r) & !1;
                            cx.s.set_w(r, v);
                        }
                    }
                    let bars = (0..3).all(|k| cx.s.w(REELS + 2 * k) == 0x18);
                    if cx.s.w(BARS) != 0 && bars {
                        cx.s.set_w(SAVED_HOLDS + 8, 0);
                        cx.s.set_l(COUNTERS + 8, 0x10);
                        cx.s.set_w(BARS, 0);
                        let b = Spin::counter(cx.s, 1);
                        if b >= 0 {
                            let c = (b as u32).wrapping_add(0x10);
                            cx.s.set_l(COUNTERS + 8, c);
                        }
                    }
                    for k in 0..3 {
                        let c = cx.s.l(COUNTERS + 4 * k).wrapping_sub(1);
                        cx.s.set_l(COUNTERS + 4 * k, c);
                    }
                    self.pc = if (0..3).any(|k| Spin::counter(cx.s, k) >= 0) {
                        3
                    } else {
                        7
                    };
                }
                7 => {
                    for k in 0..3 {
                        let r = REELS + 2 * k;
                        let v = cx.s.w(r) & !1;
                        cx.s.set_w(r, v);
                    }
                    self.pc = 8;
                    return frames(3);
                }
                _ => {
                    for k in 0..3 {
                        let h = cx.s.w(SAVED_HOLDS + 4 * k);
                        cx.s.set_w(HOLDS + 2 * k, h);
                    }
                    return Yield::Return;
                }
            }
        }
    }
}

/// A lamp flashed: the payout's line (h0+$504e: 10 times its lamp, if any, 4 frames, then every
/// lamp 4 frames) and the free spin's (h0+$4e5e: 20 times nothing 3 frames, its lamp 3 frames).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Flash {
    pc: u8,
    left: u16,
    free_spin: bool,
}

impl Flash {
    fn payout() -> Flash {
        Flash {
            pc: 0,
            left: 10,
            free_spin: false,
        }
    }

    fn free_spin() -> Flash {
        Flash {
            pc: 0,
            left: 20,
            free_spin: true,
        }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        let n = if self.free_spin { 3 } else { 4 };
        self.pc += 1;
        match self.pc {
            1 => {
                lamps_off(cx.s);
                let lamp = cx.s.l(STATE_49D2);
                if !self.free_spin && lamp != 0 {
                    cx.s.set_w(lamp, 1);
                }
                frames(n)
            }
            2 => draw_lamps(0),
            3 => {
                if self.free_spin {
                    cx.s.set_w(FREE_SPIN_LAMP, 1);
                } else {
                    lamps_on(cx.s);
                }
                frames(n)
            }
            4 => draw_lamps(0),
            _ => {
                self.left -= 1;
                if self.left == 0 {
                    return Yield::Return;
                }
                self.pc = 0;
                self.run(cx)
            }
        }
    }
}

/// `slot_award` (h0+$51c4) for a line's prize: its rung lit, then the menu offering the gamble,
/// collected when it is not taken. The gamble, the hi-lo game ($a), the jackpot (0) and the
/// ladder's ends are T74's.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Award {
    pc: u8,
    rung: u16,
}

/// The prize ladder (h0+$57b6): ten lamp records, the best first, then 0.
const LADDER: u32 = at(0x57b6);

impl Award {
    fn new(rung: u16) -> Award {
        Award { pc: 0, rung }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        if self.pc == 1 {
            if cx.s.w(GAMBLE) != 0 {
                cx.unported("the slot machine's gamble (T74)");
                return Yield::Poll;
            }
            let mut played = Vec::new();
            collect(cx.s, self.rung, &mut |_, id, ch| played.push((id, ch)));
            for (id, ch) in played {
                sfx(cx, id, ch);
            }
            return Yield::Return;
        }
        self.pc = 1;
        let ladder: Vec<u32> = (0..)
            .map(|k| cx.l(LADDER + 4 * k))
            .take_while(|&a| a != 0)
            .collect();
        let mut played = Vec::new();
        let r = award_rung(cx.s, &ladder, self.rung, &mut |_, id, ch| {
            played.push((id, ch))
        });
        for (id, ch) in played {
            sfx(cx, id, ch);
        }
        if let Err(what) = r {
            cx.unported(what);
            return Yield::Poll;
        }
        call(FrontRoutine::SlotMenu(Menu::new(LAMPS_OFF_CYCLES)))
    }
}

/// `slot_award` (h0+$51c4) for a line's prize up to its menu (h0+$5460): the lamps and the
/// ladder's (`ladder`, h0+$57b6's records) out, rung `d7` lit, its win and sound, then the rungs
/// either side and the buttons flashing, the holds cleared and the gamble offered. `Err` names
/// what is T74's: the hi-lo game ($a), the jackpot (0), the ladder's ends.
pub fn award_rung(
    s: &mut State,
    ladder: &[u32],
    d7: u16,
    sfx: &mut dyn FnMut(&mut State, u8, u8),
) -> Result<(), &'static str> {
    s.set_l(WIN, 0);
    lamps_off(s);
    for &lamp in ladder {
        s.set_w(lamp + 6, 0);
        s.set_w(lamp, 0);
    }
    if matches!(d7, 0 | 0xa) || d7 > 8 {
        return Err("the slot machine's hi-lo game and jackpot (T74)");
    }
    let (win, lamp, effect) = rung(s, d7);
    s.set_l(WIN, win);
    s.set_w(at(lamp), 1);
    sfx(s, effect, 3);
    // h0+$53ce: the rung lit (its `+6`), the rungs either side flashing with the buttons.
    let Some(k) = ladder.iter().position(|&l| s.w(l + 6) != 0) else {
        return Err("the slot machine's ladder with no rung lit (T74)");
    };
    if k == 0 || k == 9 {
        return Err("the slot machine's ladder's ends (T74)");
    }
    let (mut a2, a3) = (FLASHING, LIT);
    s.set_l(a2, ladder[k - 1]);
    a2 += 4;
    s.set_l(a2, ladder[(k + 3).min(9)]);
    a2 += 4;
    s.set_l(a3, ladder[k]);
    for lamp in [0x6892, 0x68d2, 0x68da] {
        s.set_l(a2, at(lamp));
        a2 += 4;
    }
    s.set_l(a2, LIST_END);
    s.set_l(a3 + 4, LIST_END);
    s.set_w(HOLDS_ALLOWED, 0);
    for k in 0..3 {
        s.set_w(HOLDS + 2 * k, 0);
    }
    s.set_w(OFFER, 1);
    Ok(())
}

/// Rung `d7`'s win (given the win so far), its lamp record's `+6` (h0+$6850 is $684a's) and its
/// sound (h0+$520a..$53ce; rungs
/// 1..=8).
fn rung(s: &mut State, d7: u16) -> (u32, u32, u8) {
    let win = s.l(WIN);
    match d7 {
        1 => (s.l(SCORE_SHOWN), 0x6850, 7),
        2 => {
            let lives = s.w(LIVES).wrapping_add(1);
            s.set_w(LIVES, lives);
            (0, 0x6858, 6)
        }
        3 => (win.wrapping_add(75_000), 0x6860, 5),
        4 => (win.wrapping_add(50_000), 0x6868, 4),
        5 => (win.wrapping_add(40_000), 0x6870, 3),
        6 => (win.wrapping_add(30_000), 0x6878, 2),
        7 => (win.wrapping_add(20_000), 0x6880, 1),
        _ => (win.wrapping_add(10_000), 0x6888, 0),
    }
}

/// `slot_collect` (h0+$5528) for rungs 1..=8: its sound, and the rung's win (h0+$5560..$5626, no
/// sound).
pub fn collect(s: &mut State, d7: u16, sfx: &mut dyn FnMut(&mut State, u8, u8)) {
    sfx(s, 0xb, 3);
    s.set_l(WIN, 0);
    let (win, lamp, _) = rung(s, d7);
    s.set_l(WIN, win);
    s.set_w(at(lamp), 1);
}

/// `slot_count_win` (h0+$50b6): the lamps off and drawn, the win counted into the score shown,
/// 1 000 a frame (all at once from 160 000), printed each frame. At the slot machine's start the
/// screen is blank and the lamps' drawing is not on the beam (`on_beam` false).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Count {
    pc: u8,
    on_beam: bool,
}

impl Count {
    pub(crate) fn at_start() -> Count {
        Count {
            pc: 0,
            on_beam: false,
        }
    }

    fn on_beam() -> Count {
        Count {
            pc: 0,
            on_beam: true,
        }
    }

    pub(crate) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 if !self.on_beam => {
                    super::count_win_lamps(cx);
                    self.pc = 2;
                    if cx.s.w(COUNT_OVER) != 0 {
                        self.pc = 3;
                        continue;
                    }
                    return Yield::Raster;
                }
                0 => {
                    lamps_off(cx.s);
                    self.pc = 1;
                    return draw_lamps(LAMPS_OFF_CYCLES);
                }
                1 => {
                    let win = cx.s.l(WIN);
                    let shown = cx.s.l(SCORE_SHOWN);
                    if (win.wrapping_sub(0x27100) as i32) >= 0 {
                        cx.s.set_w(COUNT_OVER, 1);
                        cx.s.set_l(SCORE_SHOWN, shown.wrapping_add(win));
                        cx.s.set_l(COUNT_SHOWN, shown.wrapping_add(win));
                        self.pc = 3;
                        continue;
                    }
                    cx.s.set_l(COUNT_SHOWN, shown);
                    cx.s.set_l(SCORE_SHOWN, shown.wrapping_add(win));
                    cx.s.set_w(COUNT_OVER, 0);
                    self.pc = 2;
                    return Yield::Raster;
                }
                // h0+$5108, after the raster wait.
                2 => {
                    let shown = cx.s.l(COUNT_SHOWN).wrapping_add(0x3e8);
                    cx.s.set_l(COUNT_SHOWN, shown);
                    let score = cx.s.l(SCORE_SHOWN);
                    if (score.wrapping_sub(shown) as i32) < 0 {
                        cx.s.set_l(COUNT_SHOWN, score);
                        cx.s.set_w(COUNT_OVER, 1);
                    }
                    self.pc = 3;
                }
                // h0+$512e: the print.
                3 => {
                    super::count_text(cx);
                    let to = super::BUFFER_0 + 0x7683;
                    let cycles = COUNT_CYCLES + super::print_text(cx, COUNT_TEXT, to);
                    cx.jobs.push(crate::game::Job {
                        inside: vec![at(0x5150)],
                        writes: vec![(to, 8 * ROW)],
                    });
                    cx.busy(cycles);
                    if cx.s.w(COUNT_OVER) == 0 {
                        self.pc = 2;
                        return Yield::Raster;
                    }
                    self.pc = 4;
                    // The caller goes on at once, once the CPU is through the print.
                    return Yield::Interrupt;
                }
                _ => return Yield::Return,
            }
        }
    }
}
