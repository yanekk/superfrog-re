//! Boot and the title screen (docs/13): `init_and_script` (h0+$26) up to the title, then
//! `title_screen` (h0+$109e): the title's intro (the eyes in the dark, `title_intro_anim`
//! h0+$3902), its fade-in, and the wait for fire or for attract mode.

use sf_assets::sound::{BANK_ADDR, TuneId};

use super::blit::Blit;
use super::common::{FadeKind, MusicFadeWait, MusicStart, call, fade, palette_wait};
use super::data::OVERLAY_AT;
use super::fade::{self, RECORDS, Rec};
use super::{Cx, Routine, Yield, at, h2, sprite};
use crate::audio::{DMACON, File, INTENA};
use crate::layout::h3;

// The original's variables (docs/08, re/symbols.toml).
pub const START_WORLD: u32 = at(0xa);
pub const START_LEVEL: u32 = at(0xe);
pub const ATTRACT_MODE: u32 = at(0x18);
pub const FINAL_LEVEL_FLAG: u32 = at(0x1e);
pub const TITLE_FADING: u32 = at(0x9e0);
pub const LEVEL_STARTED: u32 = at(0x9f0);
pub const ATTRACT_ABORT: u32 = at(0x22b8);
pub const OVERLAY_PALETTE: u32 = at(0xb806);
pub const OVERLAY_TARGET: u32 = at(0xb80a);
/// The two eye-blink timers and the four eye sprites (h0+$136c).
const BLINK: [u32; 2] = [at(0x1368), at(0x136a)];
const EYES: [u32; 4] = [at(0x1b198), at(0x1b1b4), at(0x1b1d0), at(0x1b1ec)];
/// The title intro's blit source: a pointer to a pointer into the eyes' graphics.
const INTRO_SOURCE: u32 = at(0x3dc8);
/// The eyes' bounce table (h0+$3cca): line numbers, negative at the end.
const BOUNCE: u32 = at(0x3cca);
/// The display buffers.
pub const BUFFER_0: u32 = h3(0x10600);
pub const BUFFER_1: u32 = h3(0x22f30);
/// The copper lists: blank (`h2+$6fd8`) and the title's (`h2+$7158`).
pub const BLANK_COPPER: u32 = h2(0x6fd8);
pub const TITLE_COPPER: u32 = h2(0x7158);
/// One plane of a 320 × 256 screen.
pub(super) const PLANE: u32 = 0x2800;

/// Return address in `top_loop` after `overlays_end`'s fade wait: where the original is once the
/// title's loads are over (h0+$b7e2, inside `overlays_end`).
const AFTER_TITLE_LOAD: u32 = at(0xb7e2);

/// Chip RAM's first twelve bytes as Kickstart 1.3 leaves them on the A500 the original runs on:
/// 0, ExecBase (`$c00276`, in slow RAM) and the bus-error vector (`$fc0818`, in the ROM). Not
/// the game's data: the music replayer reads them through an empty instrument (docs/11), so the
/// title music's silent channels carry them (measured on the oracle).
pub(crate) const KICKSTART_LOW_CHIP: [u8; 12] = [0, 0, 0, 0, 0, 0xc0, 0x02, 0x76, 0, 0xfc, 0x08, 0x18];

/// What the frames before the title leave in memory, built at once (loads are instant, owner
/// 2026-10-12): `init_save_boot_args`, `hw_takeover`, the Accessing-disk overlay, the seeded RNG,
/// and `title_screen` up to its loads. The title routine then goes on from `overlays_end`.
pub(super) fn boot(cx: &mut Cx) {
    if let Some(d) = cx.data.sound.as_ref() {
        cx.s.audio = crate::audio::AudioDriver::new(d);
    }
    cx.s.audio.set_low_chip(KICKSTART_LOW_CHIP);
    // Hunk 0 as loaded, and hunk 2 as the bootstrap left it, under every modelled run.
    let regions: Vec<(u32, u32)> = crate::layout::LAYOUT
        .iter()
        .map(|d| (d.addr, d.width.len()))
        .collect();
    for (addr, len) in regions {
        if (crate::layout::H0..crate::layout::H1).contains(&addr) {
            let bytes = cx.data.h0_bytes(addr - crate::layout::H0, len);
            cx.s.poke(addr, &bytes);
        } else if (crate::layout::H2..crate::layout::H3).contains(&addr) {
            let bytes = cx.display.h2_bytes(addr - crate::layout::H2, len);
            cx.s.poke(addr, &bytes);
        }
    }
    // init_save_boot_args (h0+$766e): every fade done; the bootstrap's arguments; hunks 1 and 3
    // cleared (they are zero already).
    for r in RECORDS {
        cx.s.set_w(r.at + fade::DONE, 1);
    }
    cx.s.set_l(at(0xa0a), 0x400);
    cx.s.set_l(at(0xa0e), crate::layout::H0);
    cx.s.set_l(at(0xa12), 0);
    // hw_takeover (h0+$11b6a): the blank copper list.
    cx.cop1lc = BLANK_COPPER;
    overlay_show(cx);
    super::hero::load_hero(cx);
    // rng_seed_from_cia (h0+$7722): the seed the start was given.
    if let Some(seed) = cx.seed.take() {
        cx.s.set_rng(seed);
    }
    level_tables(cx);
}

/// The rest of `rng_seed_from_cia` (h0+$7722): the level display's tables, of which the engine
/// models the map's row pointer (h0+$1730c), and the frame counters cleared.
pub(super) fn level_tables(cx: &mut Cx) {
    cx.s.set_map_rows(crate::layout::h1(0x21680));
    cx.s.set_w(at(0x9d8), 0);
    cx.s.set_w(at(0x9da), 0);
}

/// `access_overlay_show` (h0+$b6f8) when no overlay is up: the Accessing-disk program is unpacked
/// at [`OVERLAY_AT`] and run: it points its copper list at its picture and fades it in.
pub(crate) fn overlay_show(cx: &mut Cx) {
    let o = &cx.data.overlay;
    cx.s.write(OVERLAY_AT, &o.clone());
    let ov = |off: u32| OVERLAY_AT + off;
    // Its picture: four planes of $1090 from +$81e, into the MOVEs at +$4ac2.
    for k in 0..4 {
        let p = ov(0x81e) + k * 0x1090;
        cx.s.set_w(ov(0x4ac2) + 8 * k + 6, p as u16);
        cx.s.set_w(ov(0x4ac2) + 8 * k + 2, (p >> 16) as u16);
    }
    cx.cop1lc = ov(0x4a5e);
    // Its fade in (+$8e), with its own copy of the fade code: two steps a frame until done.
    let rec = Rec::overlay(ov(0x62a));
    fade::fade_in(cx.s, rec, ov(0x4a82), ov(0x4b6e), 0x10);
    while !fade::done(cx.s, rec) {
        fade::step_in_rec(cx.s, rec);
        fade::step_in_rec(cx.s, rec);
    }
    cx.s.set_l(OVERLAY_PALETTE, ov(0x4a82));
    cx.s.set_l(OVERLAY_TARGET, ov(0x4b6e));
}

/// `$1d6c`, `$1dd2`: the title's files. The title music (`MUSC`) at h3+$64c70; the title picture
/// (`MPIC`, six planes) in display buffer 0 and copied to buffer 1, and the eyes' graphics (packed
/// at h0+$34e92) after it in buffer 1.
fn load_title(cx: &mut Cx) {
    // h0+$1dd2's files, HPIC and MPIC (disk 1).
    cx.read_entries(0x1dd2, 0x1e20);
    if let Some(d) = cx.data.sound.as_ref()
        && !cx.s.audio.music_on()
    {
        cx.s.audio
            .file_loaded(d, BANK_ADDR, File::Tune(TuneId::Title));
    }
    let pic = cx.data.file("MPIC").to_vec();
    cx.s.write(BUFFER_0, &pic[..pic.len().min(0xf000)]);
    let copy = cx.s.read(BUFFER_0, 0xf000);
    cx.s.write(BUFFER_1, &copy);
    // HPIC is loaded packed to h1+$20d00 (for the hall of talent), into what is the level's map
    // in a level; the engine unpacks it from the disks when needed, so those bytes are taken from
    // the original.
    let sectors = cx.data.h0_w(0xcbfe + 13 * 6 + 2) as u32;
    cx.resync.push((crate::layout::h1(0x20d00), sectors * 512));
    let eyes = cx.data.file("h0@34e92").to_vec();
    cx.s.write(BUFFER_1 + 0xf000, &eyes);
}

/// `title_screen` (h0+$109e).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Title {
    pc: u8,
    /// Booting: the first title, whose start is part of the boot's load.
    pub boot: bool,
    count: u16,
}

impl Title {
    /// In the title's idle (h0+$1158's loop): fire next frame opens the menu.
    pub(super) fn idle(&self) -> bool {
        self.pc == 6
    }

    pub fn after_boot() -> Title {
        Title {
            boot: true,
            ..Title::default()
        }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    cx.s.set_w(FINAL_LEVEL_FLAG, 0);
                    if cx.s.w(at(0x12)) != 0 {
                        cx.unported("title_screen with h0+$12 set");
                        return Yield::Return;
                    }
                    self.pc = 1;
                    if cx.s.l(ATTRACT_MODE) == 0 {
                        if self.boot {
                            // music_fade_wait while the boot's overlay is up: its one frame is
                            // part of the load.
                            if let Some(d) = cx.data.sound.as_ref() {
                                cx.s.audio.fade_out();
                                let mut quiet = Vec::new();
                                cx.s.audio.stop_music(d, &mut quiet);
                            }
                        } else {
                            return call(Routine::MusicFadeWait(MusicFadeWait::default()));
                        }
                    }
                }
                1 => {
                    cx.s.set_l(START_WORLD, 1);
                    cx.s.set_l(START_LEVEL, 1);
                    cx.s.set_w(TITLE_FADING, 1);
                    cx.s.set_w(LEVEL_STARTED, 0);
                    if cx.s.w(ATTRACT_ABORT) == 0 {
                        if cx.s.l(OVERLAY_PALETTE) == 0 {
                            overlay_show(cx);
                        }
                        load_title(cx);
                        cx.loaded(if self.boot {
                            crate::game::LoadEnd::Reached(AFTER_TITLE_LOAD)
                        } else {
                            // After a game (T34) the overlay is long faded in, and its fade-out's
                            // set-up begins in the frame the last load (`$1dd2`) ends in.
                            crate::game::LoadEnd::Left(at(0x1102))
                        });
                    } else {
                        cx.unported("title_screen after an aborted attract screen");
                    }
                    cx.s.set_w(ATTRACT_ABORT, 0);
                    self.pc = 2;
                    return Yield::Call(Box::new(Routine::Overlay(OverlayEnd::default())));
                }
                2 => {
                    self.pc = 12;
                    if cx.s.l(ATTRACT_MODE) == 0 {
                        return call(Routine::HiscoreCredits(Default::default()));
                    }
                }
                12 => {
                    eyes_init(cx);
                    title_copper(cx);
                    self.pc = 3;
                }
                3 => {
                    eyes_init(cx);
                    self.pc = 4;
                    return Yield::Call(Box::new(Routine::IntroAnim(IntroAnim::default())));
                }
                4 => {
                    self.pc = 5;
                    return Yield::Call(Box::new(Routine::FadeIn(TitleFadeIn::default())));
                }
                5 => {
                    cx.s.set_l(ATTRACT_MODE, 0);
                    self.count = if cx.s.w(at(0x24)) != 0 { 0x2ee } else { 0x190 };
                    self.pc = 6;
                    return Yield::Raster;
                }
                6 => {
                    if cx.s.w(at(0x24)) != 0 {
                        cx.unported("title idle with h0+$24 set");
                    }
                    if cx.fire() {
                        self.pc = 8;
                        continue;
                    }
                    blink(cx);
                    self.count -= 1;
                    if self.count != 0 {
                        return Yield::Raster;
                    }
                    cx.s.set_l(ATTRACT_MODE, 0xffff_ffff);
                    self.pc = 7;
                }
                // Attract mode, or START GAME: the title's end (h0+$1180).
                7 => {
                    self.pc = 11;
                    return call(Routine::TitleExit(TitleExit::default()));
                }
                // Fire: the menu.
                8 => {
                    self.pc = 9;
                    return call(Routine::Menu(super::menu::Menu::default()));
                }
                9 => {
                    if cx.ret == 0 {
                        self.pc = 7;
                        continue;
                    }
                    self.pc = 10;
                    return call(Routine::Options(super::menu::Options::default()));
                }
                // Back from the options: the title's intro again (h0+$1122).
                10 => {
                    self.pc = 3;
                }
                _ => return Yield::Return,
            }
        }
    }
}

/// `$1180`: the title's end, for attract mode or a game: the title fades out, the music too unless
/// attract mode goes on, and the game's start values are set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct TitleExit {
    pc: u8,
}

impl TitleExit {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            self.pc += 1;
            match self.pc {
                1 => {
                    if cx.s.w(at(0x12)) != 0 {
                        self.pc = 5;
                        continue;
                    }
                    return fade(FadeKind::Out, 0, at(0x3ba0), 0x20, at(0x1cc2));
                }
                2 => return fade(FadeKind::Out, 1, at(0x3c20), 0x20, at(0x1cec)),
                3 => return palette_wait(0),
                4 => return palette_wait(1),
                5 => cx.cop1lc = BLANK_COPPER,
                6 => {
                    if cx.s.w(at(0x24)) == 0 && cx.s.l(ATTRACT_MODE) == 0 && cx.s.audio.music_on() {
                        return call(Routine::MusicFadeWait(MusicFadeWait::default()));
                    }
                }
                _ => {
                    let mode = cx.s.w(at(0x9e6));
                    cx.s.set_w(at(0x9e8), mode);
                    let attract = cx.s.l(ATTRACT_MODE) != 0;
                    if attract {
                        cx.s.set_w(at(0x9e6), 1);
                    }
                    cx.s.set_l(at(0x11a24), 1);
                    cx.s.set_w(at(0x11a28), 0);
                    cx.s.set_w(at(0x11a2a), 0);
                    cx.s.set_w(at(0x11a2c), 2);
                    let v = cx.s.l(at(0x9f2));
                    cx.s.set_l(at(0x11a2e), v);
                    crate::frog::frog_new_game(&mut crate::mem::Bus::new(
                        cx.s,
                        cx.display.rom(),
                    ));
                    cx.s.set_w(at(0x87c0), 0xffff);
                    cx.s.set_w(at(0x9de), 0);
                    cx.s.set_w(TITLE_FADING, 0);
                    cx.s.set_w(at(0x9dc), 0);
                    if attract {
                        // Three demos, all of W1-1.
                        let k = cx.s.w(at(0x9f6));
                        let next = k.wrapping_add(1);
                        cx.s.set_w(at(0x9f6), if (next as i16) < 3 { next } else { 0 });
                        cx.s.set_l(START_WORLD, 1);
                        cx.s.set_l(START_LEVEL, 1);
                    }
                    return Yield::Return;
                }
            }
        }
    }
}

/// `$136c`: the blink timers and the four eye sprites at their places.
pub(super) fn eyes_init(cx: &mut Cx) {
    cx.s.set_w(BLINK[0], 0x4b);
    cx.s.set_w(BLINK[1], 0x23);
    for r in EYES {
        sprite::attach(cx.s, r);
    }
    for (r, y) in EYES.into_iter().zip([0x4f, 0x4f, 0x9f, 0x9f]) {
        cx.s.set_w(r, 0x150);
        cx.s.set_w(r + 2, y);
        sprite::update(cx.s, r);
    }
}

/// `$1d0c`: the title's copper list shows display buffer 0, all its colours black.
fn title_copper(cx: &mut Cx) {
    for k in 0..6 {
        let p = BUFFER_0 + k * PLANE;
        cx.s.set_w(h2(0x7204) + 8 * k + 6, p as u16);
        cx.s.set_w(h2(0x7204) + 8 * k + 2, (p >> 16) as u16);
    }
    for k in 0..0x20 {
        cx.s.set_w(h2(0x7236) + 4 * k, 0);
        cx.s.set_w(h2(0x733e) + 4 * k, 0);
    }
    for o in [0x73e2, 0x73e6, 0x73ea] {
        cx.s.set_w(h2(o), 0);
    }
}

/// `$1282`: one frame of the eyes' blinking, each pair on its own timer.
fn blink(cx: &mut Cx) {
    for (timer, (a, b), (ya, yb)) in [
        (BLINK[0], (EYES[0], EYES[1]), (0x88, 0x98)),
        (BLINK[1], (EYES[2], EYES[3]), (0x50, 0x60)),
    ] {
        cx.s.set_w(a, 0x150);
        cx.s.set_w(b, 0x150);
        let t = cx.s.w(timer).wrapping_sub(1);
        cx.s.set_w(timer, t);
        if (t as i16) < 0 {
            cx.s.set_w(a, ya);
            cx.s.set_w(b, yb);
        }
        sprite::update(cx.s, a);
        sprite::update(cx.s, b);
        if (cx.s.w(timer) as i16) < -2 {
            let mut rng = cx.s.rng();
            let r = rng.range(0x4b);
            cx.s.set_rng(rng);
            cx.s.set_w(timer, r.wrapping_add(0x4b));
        }
    }
}

/// `overlays_end` (h0+$b774) with the Accessing-disk overlay up: fade it out, then the blank list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct OverlayEnd {
    pc: u8,
}

impl OverlayEnd {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        match self.pc {
            0 => {
                // The screens between levels first (T183): each one's own ending, then back.
                use super::chain;
                let end = if cx.s.l(chain::END_PIC_SHOWN) != 0 {
                    Some(Routine::EndPicEnd(Default::default()))
                } else if cx.s.l(chain::MAP_SHOWN) != 0 {
                    Some(Routine::MapEnd(Default::default()))
                } else if cx.s.l(OVERLAY_PALETTE) == 0 && cx.s.l(chain::INTRO_SHOWN) != 0 {
                    Some(Routine::IntroEnd(Default::default()))
                } else {
                    None
                };
                if let Some(r) = end {
                    self.pc = 9;
                    return call(r);
                }
                if cx.s.l(OVERLAY_PALETTE) == 0 {
                    return Yield::Return;
                }
                cx.s.set_w(RECORDS[0].at + fade::SPEED, 1);
                self.pc = 1;
                Yield::Fade(RECORDS[0])
            }
            // The fade-out's set-up with interrupts off (`$155a0`: ~30 lines for 16 colours).
            1 => {
                let list = cx.s.l(OVERLAY_PALETTE);
                self.pc = 2;
                fade(FadeKind::Out, 0, list, 0x10, at(0xb7d6))
            }
            2 => {
                self.pc = 3;
                Yield::Fade(RECORDS[0])
            }
            9 => Yield::Return,
            _ => {
                cx.cop1lc = BLANK_COPPER;
                cx.s.set_l(OVERLAY_PALETTE, 0);
                cx.s.set_l(OVERLAY_TARGET, 0);
                Yield::Return
            }
        }
    }
}

/// The blits the title uses, each between COPER off and on. With the blitter given priority
/// (`nasty`), a blit that runs into the next frame holds the CPU off the bus: the frame's
/// interrupt waits for it (`cx.stall`), and its memory is compared once the original's call
/// (return address `inside`) is over.
fn blit_planes(
    cx: &mut Cx,
    mut b: Blit,
    planes: u32,
    a_step: u32,
    d_step: u32,
    nasty: bool,
    inside: Option<u32>,
) {
    cx.wr(INTENA, 0x0010);
    if nasty {
        cx.wr(DMACON, 0x8400);
    }
    let start = cx.line;
    let first_d = b.pt[3];
    for _ in 0..planes {
        b.run(cx.s);
        cx.blit_wait(&b);
        b.pt[0] = b.pt[0].wrapping_add(a_step);
        b.pt[3] = b.pt[3].wrapping_add(d_step);
    }
    if nasty && let Some(inside) = inside {
        stalled_past(cx, start, (first_d, b.pt[3] - first_d), inside);
    }
    cx.wr(INTENA, 0x8010);
}

/// After a blit with priority begun on line `start`: if it ran past the frame's end, the CPU was
/// held off the bus and the next frame's interrupt waits for it (`cx.stall`), its end later by
/// the interrupt's length, and its memory (`writes`) is compared once the original is out of the
/// call returning to `inside`.
pub(super) fn stalled_past(cx: &mut Cx, start: u16, writes: (u32, u32), inside: u32) {
    if start < super::FRAME_LINES && cx.line >= super::FRAME_LINES {
        cx.stall = Some(super::STALL_VBL_LINE);
        cx.line += 11;
        cx.jobs.push(crate::game::Job {
            inside: vec![inside],
            writes: vec![writes],
        });
    }
}

/// `$1ba6`: clear `lines` lines of buffer 0's six planes from line `from`.
fn blit_clear(cx: &mut Cx, from: u32, lines: u16) {
    let b = Blit {
        con0: 0x0100,
        pt: [0, 0, 0, BUFFER_0 + from * 0x28],
        size: (lines << 6) + 0x14,
        ..Blit::default()
    };
    blit_planes(cx, b, 6, 0, PLANE, false, None);
}

/// `$1b18`: copy `lines` lines of buffer 1 into buffer 0 from line `from`.
pub(super) fn blit_restore(cx: &mut Cx, from: u32, lines: u16, inside: u32) {
    let b = Blit {
        con0: 0x09f0,
        afwm: 0xffff,
        alwm: 0xffff,
        pt: [BUFFER_1 + from * 0x28, 0, 0, BUFFER_0 + from * 0x28],
        size: (lines << 6) + 0x14,
        ..Blit::default()
    };
    blit_planes(cx, b, 6, PLANE, PLANE, true, Some(inside));
}

/// `$3d0a`: the eyes' current frame into buffer 0 at (`x`, `y`), shifted to `x`.
fn blit_eyes(cx: &mut Cx, x: u16, y: u16) {
    if (x as i16) >= 0x120 {
        return;
    }
    let src = cx.l(cx.l(INTRO_SOURCE));
    let b = Blit {
        con0: 0x09f0 | (x & 15) << 12,
        afwm: 0xffff,
        alwm: 0,
        pt: [
            src,
            0,
            0,
            BUFFER_0 + (x as u32 >> 3) + (y as u32 & 0xffff) * 0x28,
        ],
        modulo: [0x18, 0, 0, 0x22],
        size: 0x903,
        ..Blit::default()
    };
    blit_planes(cx, b, 6, 0x8e8, PLANE, true, None);
}

/// `title_intro_anim` (h0+$3902): the eyes bounce in from the right in the dark, blink, and the
/// music starts; fire skips to the end.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct IntroAnim {
    pc: u8,
    x: u16,
    y: u16,
    table: u32,
    frames: u16,
    /// Which of the blink frames (h0+$3dd0..$3de4) is next.
    frame: u8,
}

impl IntroAnim {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            match self.pc {
                0 => {
                    if cx.s.audio.music_on() {
                        // $3aec: the music is on (back from the options): no intro, the menu's
                        // line out of the way, the title's list from the next frame.
                        cx.s.set_w(h2(0x7338), 0x7401);
                        menu_line_off(cx);
                        self.pc = 30;
                        return Yield::Raster;
                    }
                    blit_clear(cx, 0x30, 0x40);
                    self.pc = 1;
                    return Yield::Raster;
                }
                1 => {
                    cx.cop1lc = TITLE_COPPER;
                    cx.s.set_w(h2(0x7338), 0x2001);
                    for (k, v) in [0x6401ff00u32, 0x01820aaa, 0x01a80ddd, 0x01be0fff]
                        .into_iter()
                        .enumerate()
                    {
                        cx.s.set_l(h2(0x73bc) + 4 * k as u32, v);
                    }
                    self.x = 0x128;
                    cx.s.set_l(INTRO_SOURCE, at(0x3dcc));
                    self.table = BOUNCE;
                    self.pc = 2;
                }
                // $3968: in from the right, bouncing.
                2 => {
                    if cx.fire() {
                        self.pc = 20;
                        continue;
                    }
                    self.pc = 3;
                    return Yield::Raster;
                }
                3 => {
                    if (cx.w(self.table) as i16) < 0 {
                        self.table = BOUNCE;
                    }
                    self.y = cx.w(self.table);
                    self.table += 2;
                    blit_eyes(cx, self.x, self.y);
                    self.x = self.x.wrapping_sub(1);
                    self.pc = if (self.x as i16) >= 0xe0 || self.y != 0x40 {
                        2
                    } else {
                        4
                    };
                }
                // The music, and the eyes blink.
                4 => {
                    self.pc = 24;
                    return call(Routine::MusicStart(MusicStart::returning_to(at(0x39ac))));
                }
                24 => {
                    cx.s.set_l(INTRO_SOURCE, at(0x3dd0));
                    self.pc = 5;
                    return Yield::Raster;
                }
                5 => {
                    blit_eyes(cx, self.x, self.y);
                    cx.s.set_l(INTRO_SOURCE, at(0x3dd4));
                    self.pc = 6;
                    return Yield::Raster;
                }
                6 => {
                    blit_eyes(cx, self.x, self.y);
                    self.frames = 0x32;
                    self.frame = 0;
                    self.pc = 7;
                }
                // $3cee: up to 50 frames, fire skips.
                7 | 13 => {
                    self.pc += 1;
                    return Yield::Raster;
                }
                8 | 14 => {
                    if cx.fire() {
                        self.pc = 20;
                        continue;
                    }
                    self.frames -= 1;
                    self.pc = if self.frames != 0 {
                        self.pc - 1
                    } else {
                        self.pc + 1
                    };
                }
                // Four more blink frames, two frames apart.
                9 => {
                    let src = [0x3dd8, 0x3ddc, 0x3de0, 0x3de4][self.frame as usize];
                    cx.s.set_l(INTRO_SOURCE, at(src));
                    self.pc = 10;
                    return Yield::Raster;
                }
                10 => {
                    self.pc = 11;
                    return Yield::Raster;
                }
                11 => {
                    blit_eyes(cx, self.x, self.y);
                    self.frame += 1;
                    if self.frame < 4 {
                        self.pc = 9;
                    } else {
                        self.frames = 0x32;
                        self.pc = 13;
                    }
                }
                // $3a5a: on to the left at twice the speed, bouncing.
                15 => {
                    self.table = BOUNCE;
                    self.pc = 16;
                }
                16 => {
                    if cx.fire() {
                        self.pc = 20;
                        continue;
                    }
                    self.pc = 17;
                    return Yield::Raster;
                }
                17 => {
                    if (cx.w(self.table) as i16) < 0 {
                        self.table = BOUNCE;
                    }
                    self.y = cx.w(self.table);
                    self.table += 4;
                    blit_eyes(cx, self.x, self.y);
                    self.x = self.x.wrapping_sub(2);
                    if (self.x as i16) >= 0x8c || self.y != 0x40 {
                        self.pc = 16;
                    } else {
                        self.frames = 0x64;
                        self.pc = 18;
                    }
                }
                18 => {
                    self.pc = 19;
                    return Yield::Raster;
                }
                19 => {
                    if cx.fire() {
                        self.pc = 20;
                        continue;
                    }
                    self.frames -= 1;
                    self.pc = if self.frames != 0 { 18 } else { 20 };
                }
                // $3a96: music (again), then at line $a0 the picture comes back under the eyes.
                20 => {
                    self.pc = 25;
                    return call(Routine::MusicStart(MusicStart::returning_to(at(0x3aaa))));
                }
                25 => {
                    self.pc = 21;
                    return Yield::Line(0xa0);
                }
                21 => {
                    cx.s.set_w(h2(0x7338), 0x7401);
                    blit_restore(cx, 0x30, 0x40, at(0x3ad0));
                    self.frames = 0x32;
                    self.pc = 22;
                }
                22 => {
                    if cx.fire() {
                        return Yield::Return;
                    }
                    self.pc = 23;
                    return Yield::Raster;
                }
                30 => {
                    cx.cop1lc = TITLE_COPPER;
                    return Yield::Return;
                }
                23 => {
                    self.frames -= 1;
                    if self.frames == 0 {
                        return Yield::Return;
                    }
                    self.pc = 22;
                }
                _ => unreachable!("IntroAnim pc {}", self.pc),
            }
        }
    }
}

/// `$1c0e`: both halves of the title fade in together, then the menu's line is moved below the
/// picture.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct TitleFadeIn {
    pc: u8,
}

impl TitleFadeIn {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        self.pc += 1;
        match self.pc {
            1 => fade(
                FadeKind::In { target: at(0x37e2) },
                0,
                at(0x3ba0),
                0x20,
                at(0x1c38),
            ),
            2 => fade(
                FadeKind::In { target: at(0x3822) },
                1,
                at(0x3c20),
                0x20,
                at(0x1c62),
            ),
            3 => palette_wait(0),
            4 => palette_wait(1),
            _ => {
                menu_line_off(cx);
                Yield::Return
            }
        }
    }
}

/// The menu's line (h2+$73bc: a WAIT and three MOVEs) out of the way, below the picture.
pub(super) fn menu_line_off(cx: &mut Cx) {
    for (k, v) in [0x7401ff00u32, 0x00980000, 0x00980000, 0x00980000]
        .into_iter()
        .enumerate()
    {
        cx.s.set_l(h2(0x73bc) + 4 * k as u32, v);
    }
}

/// The title's routines on the original's call stack, from where its main program is (`chain`:
/// PC, then return addresses) and its registers: outermost first, each with what it waits for.
pub(super) fn resume(
    chain: &[u32],
    cpu: &crate::game::Cpu,
    _mem: &crate::layout::AmigaMemory,
) -> Option<Vec<(Routine, Option<Yield>)>> {
    let in_raster_wait = |pc: u32| (at(0x73d0)..at(0x73e6)).contains(&pc);
    match chain {
        // The idle loop (h0+$114a): d0 frames left before attract mode.
        [pc, r, ..] if in_raster_wait(*pc) && *r == at(0x114e) => Some(vec![
            (
                Routine::TopLoop(super::attract::TopLoop::after_title()),
                None,
            ),
            (
                Routine::Title(Title {
                    pc: 6,
                    boot: false,
                    count: cpu.d[0] as u16,
                }),
                Some(Yield::Raster),
            ),
        ]),
        _ => None,
    }
}

/// The copper list a place on the title shows (COP1LC is not in memory).
pub(super) fn cop1lc_of(chain: &[u32]) -> u32 {
    match chain.get(1) {
        Some(&r) if r == at(0x114e) => TITLE_COPPER,
        _ => BLANK_COPPER,
    }
}
