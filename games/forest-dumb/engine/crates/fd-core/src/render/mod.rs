//! The renderer (plans/forest-dumb-level-1 T10; DESIGN §2.6; docs/06-graphics-and-rendering.md, "The
//! engine's picture"; docs/09-play-measured.md, "Display").
//!
//! It draws the frame the original shows, as Denise would: the engine keeps the memory the display
//! reads in the original's form ([`Display`]: the play screen, the status bar, the character's
//! sprites, the copper list's colours), the level copper list (`h1+$d4`, read from the main program)
//! is run for the frame with the engine's values written into it, and each line is fetched from
//! whatever its bitplane pointers point at, with the sprites over it. Nothing is drawn from the map:
//! the play screen is what the original's blits leave in it (`playfield.rs`).
//!
//! `amiga-chips` gives the copper run, the bitplane fetch and colour registers ([`Planes`],
//! [`Colours`]) and the sprite composition (`amiga_chips::sprite`, moved there from the first game's
//! renderer by this task). Agreement with the original's picture is T44's.

pub mod contact;
pub mod message;
pub mod objects;
pub mod palette;
pub mod playfield;
pub mod status;

use amiga_chips::Picture;
use amiga_chips::copper::{self, Fetch, Write, reg};
use amiga_chips::display::{Colours, Planes};
use amiga_chips::sprite::{self, SpriteLine};
use fd_assets::{AssetError, Files, Level, Sprites};

use crate::program::{HUNK_BASES, PROGRAM_FILE, Program, h};
use objects::CharSprite;
use palette::Fade;
use playfield::{SCREEN, SCREEN_BYTES, Screen, Scroll};
use status::{STATUS, STATUS_BYTES, Shown, Status};

/// The level's copper list, hunk 1.
pub const LEVEL_COPPER: u32 = 0xd4;
/// The level palette the fade reaches, hunk 1: 32 MOVEs' values, 4 bytes apart.
pub const PALETTE: u32 = 0x306;
/// The colours the copper list sets now, hunk 1, as [`PALETTE`].
pub const COLOURS: u32 = 0x13e;
/// The window the play area and the status bar make (T04): DIWSTRT and DIWSTOP.
pub const WINDOW: (u16, u16) = (0x2f81, 0x29c1);
/// Hunk sizes as loaded: hunk 1 (data) and hunk 4 (code and data).
const H1_BYTES: u32 = 0x1280;
const H4_BYTES: u32 = 0x1_c94c;
/// Where the load lists put `_GenN-1`, hunk 3: the tiles the redraws blit.
const TILES: u32 = 0x1_3540;
/// Where world 1's load list puts `_ELEMENTY.RAWM` and `.MASK`, hunk 4.
const ELEMENTS: u32 = 0;
const ELEMENT_MASKS: u32 = 0x1900;
/// Where it puts `_WYBIJACZKA`, `_ZNIKAJKA1` and `_Plomien1`, hunk 4 (docs/07).
const KNOCKER: u32 = crate::objects::spring::PICTURES[0];
const VANISHING: u32 = crate::objects::animated::VANISH_PICTURE;
const FLAME: u32 = crate::objects::animated::FLAME_PICTURE;

/// What the display reads that the disks decide, at the original's addresses: hunk 1 (the copper
/// list, the level palette, the sprites' blocks), hunk 4 (the status bar's picture, the font, the
/// icons) with world 1's `_ELEMENTY` and the mechanics' pictures loaded over it, and the tiles. A
/// test's data without the program reads as 0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DisplayData {
    h1: Vec<u8>,
    h4: Vec<u8>,
    tiles: Vec<u8>,
}

impl DisplayData {
    /// From the main program and world 1's files.
    pub fn read(files: &Files, level: &Level) -> Result<DisplayData, AssetError> {
        let exe = files(PROGRAM_FILE)?;
        let h1 = fd_assets::loaded_hunk(&exe, PROGRAM_FILE, 1, &HUNK_BASES)?;
        let h4 = fd_assets::loaded_hunk(&exe, PROGRAM_FILE, 4, &HUNK_BASES)?;
        let sprites = fd_assets::read_sprites(1, files)?;
        Ok(DisplayData::new(h1, h4, level, &sprites))
    }

    /// From hunks 1 and 4 as loaded, the level and the world's sprites.
    pub fn new(mut h1: Vec<u8>, mut h4: Vec<u8>, level: &Level, sprites: &Sprites) -> DisplayData {
        h1.resize(H1_BYTES as usize, 0);
        h4.resize(H4_BYTES as usize, 0);
        let mut put = |at: u32, b: &[u8]| {
            let at = at as usize;
            let n = b.len().min(h4.len() - at);
            h4[at..at + n].copy_from_slice(&b[..n]);
        };
        put(ELEMENTS, &sprites.elements.picture);
        if let Some(m) = &sprites.elements.mask {
            put(ELEMENT_MASKS, m);
        }
        // The mechanics' pictures (T25), where world 1's load list puts them: the spring's first copy
        // only (its second slot keeps the program's bytes, docs/06), the vanishing block, the flame.
        put(KNOCKER, &sprites.knocker.picture);
        put(VANISHING, &sprites.vanishing.picture);
        put(FLAME, &sprites.flame.picture);
        // `level_init` for worlds 1 and 2: the level palette from `h1+$384` over `h1+$304`
        // (docs/06, "The palette"; measured at the sync point, `drawing_routines.rs`).
        h1.copy_within(0x384..0x404, 0x304);
        for (off, b) in sprite_pointers().into_iter().chain(status_pointers()) {
            h1[off as usize..off as usize + b.len()].copy_from_slice(&b);
        }
        DisplayData {
            h1,
            h4,
            tiles: level.gens.iter().flatten().copied().collect(),
        }
    }

    /// The byte at chip address `a` of what the disks decide; 0 elsewhere.
    pub fn byte(&self, a: u32) -> u8 {
        let a = a & 0xff_ffff;
        let at = |base: u32, b: &[u8]| b.get(a.wrapping_sub(base) as usize).copied();
        at(h(1, 0), &self.h1)
            .or_else(|| at(h(4, 0), &self.h4))
            .or_else(|| at(h(3, TILES), &self.tiles))
            .unwrap_or(0)
    }

    /// Hunk 4's byte at offset `off`.
    pub fn h4(&self, off: u32) -> u8 {
        self.h4.get(off as usize).copied().unwrap_or(0)
    }

    /// The level palette the fade reaches, as `level_init` leaves it.
    pub fn palette(&self) -> [u16; 32] {
        std::array::from_fn(|i| self.h1_word(PALETTE + 4 * i as u32))
    }

    fn h1_word(&self, off: u32) -> u16 {
        let b = |o: u32| self.h1.get(o as usize).copied().unwrap_or(0);
        u16::from_be_bytes([b(off), b(off + 1)])
    }

    /// The status bar's buffer as the program holds it.
    pub fn status(&self) -> Vec<u8> {
        (STATUS..STATUS + STATUS_BYTES)
            .map(|o| self.h4(o))
            .collect()
    }
}

/// A MOVE pair's two value words (`move.w d1, 6(a1); swap d1; move.w d1, 2(a1)`) pointing at `a`,
/// as (hunk 1 offset, bytes), low word first as the original writes them.
fn pointer(list: u32, a: u32) -> [(u32, Vec<u8>); 2] {
    [
        (list + 6, (a as u16).to_be_bytes().to_vec()),
        (list + 2, ((a >> 16) as u16).to_be_bytes().to_vec()),
    ]
}

/// `h0_9a34`, at start-up: the level copper list's sprite pointers (`h1+$d8`), sprites 0 to 3 at the
/// character's blocks, 4 to 7 at one empty sprite (`h1+$fd0`).
pub fn sprite_pointers() -> Vec<(u32, Vec<u8>)> {
    (0..8u32)
        .flat_map(|n| {
            let block = if n < 4 {
                objects::SPRITES + n * objects::SPRITE_STRIDE
            } else {
                0xfd0
            };
            pointer(0xd8 + 8 * n, h(1, block))
        })
        .collect()
}

/// `h0_988a`, at start-up: the status bar's five bitplane pointers (`h1+$248`), a plane's 40 bytes
/// apart in its interleaved buffer.
pub fn status_pointers() -> Vec<(u32, Vec<u8>)> {
    (0..5u32)
        .flat_map(|k| pointer(0x248 + 8 * k, h(4, STATUS + 0x28 * k)))
        .collect()
}

/// What the display shows that the engine keeps: picture state, which nothing in play logic reads,
/// so `Game`'s equality leaves it out. The bitplane pointers and scroll are not kept: `h0_9754`
/// computes them from the camera every pass after the camera has moved ([`Scroll::from_camera`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Display {
    pub fade: Fade,
    pub sprite: CharSprite,
    pub status: Status,
    pub screen: Screen,
}

impl Display {
    /// At the level sync point, as measured there (`fd-conform/tests/drawing_routines.rs`): the fade
    /// has run its first pass from black, the sprites hold the character's first frame, the status
    /// bar is the program's own picture with the level's starting numbers drawn, and the play screen is as `level_init`
    /// drew it, which [`crate::Game::new`] applies after (T65).
    pub fn level_1(data: &DisplayData, program: &Program) -> Display {
        let mut fade = Fade {
            on: 1,
            pass: 0,
            now: [0; 32],
            scratch: [0; 3],
        };
        fade.step(&data.palette(), program);
        let mut status = Status {
            bytes: data.status(),
            shown: Shown {
                countdown: 0xffff,
                lives: 0xffff,
                items: 0xffff,
                score: 0xffff_ffff,
            },
        };
        // `main_loop` has drawn the numbers once before play (its check runs whenever `irq_count`
        // is a multiple of 32, the level's start included): countdown `$999`, lives `$32`, score 0,
        // items `$282` (measured).
        let h4 = |o: u32| data.h4(o);
        let src = status::Sources { h4: &h4, program };
        status.update(0x999, 0x32, 0, 0x282, &src);
        Display {
            fade,
            sprite: CharSprite {
                ctl: objects::control(0x70, 0x70, 0, 0x17),
                frame: Some(0x00c4_91a0),
            },
            status,
            screen: Screen::default(),
        }
    }
}

/// Chip memory as the display reads it: the engine's [`Display`] over the disks' [`DisplayData`].
pub(crate) struct Chip<'a> {
    data: &'a DisplayData,
    display: &'a Display,
    /// Hunk 1 with the engine's copper and sprite values written in.
    h1: Vec<u8>,
}

impl<'a> Chip<'a> {
    pub(crate) fn new(
        data: &'a DisplayData,
        display: &'a Display,
        camera: (u16, u16),
        program: &Program,
    ) -> Chip<'a> {
        let mut h1 = data.h1.clone();
        let mut put = |off: u32, b: &[u8]| {
            for (k, &v) in b.iter().enumerate() {
                if let Some(x) = h1.get_mut(off as usize + k) {
                    *x = v;
                }
            }
        };
        for (i, c) in display.fade.now.iter().enumerate() {
            put(COLOURS + 4 * i as u32, &c.to_be_bytes());
        }
        let frame = |a: u32, n: u32| {
            (0..n)
                .map(|k| program.byte(a.wrapping_add(k).wrapping_sub(HUNK_BASES[0])))
                .collect::<Vec<u8>>()
        };
        for (off, b) in display.sprite.bytes(&frame) {
            put(off, &b);
        }
        for (off, b) in Scroll::from_camera(camera.0, camera.1).copper() {
            put(off, &b);
        }
        Chip { data, display, h1 }
    }

    pub(crate) fn byte(&self, a: u32) -> u8 {
        let a = a & 0xff_ffff;
        let at = |base: u32, len: u32| {
            let i = a.wrapping_sub(base);
            (i < len).then_some(i as usize)
        };
        if let Some(i) = at(h(1, 0), self.h1.len() as u32) {
            return self.h1[i];
        }
        if let Some(i) = at(h(3, SCREEN), SCREEN_BYTES) {
            return self.display.screen.bytes[i];
        }
        if let Some(i) = at(h(4, STATUS), STATUS_BYTES) {
            return self.display.status.bytes[i];
        }
        self.data.byte(a)
    }

    /// `out.len()` bytes from `a`: a slice of the play screen or the status bar at once when they lie
    /// in one, else byte by byte.
    pub(crate) fn read(&self, a: u32, out: &mut [u8]) {
        let a = a & 0xff_ffff;
        let n = out.len() as u32;
        let within = |base: u32, len: u32| {
            let i = a.wrapping_sub(base);
            (i < len && len - i >= n).then_some(i as usize)
        };
        if let Some(i) = within(h(3, SCREEN), SCREEN_BYTES) {
            out.copy_from_slice(&self.display.screen.bytes[i..i + out.len()]);
        } else if let Some(i) = within(h(4, STATUS), STATUS_BYTES) {
            out.copy_from_slice(&self.display.status.bytes[i..i + out.len()]);
        } else {
            for (k, b) in out.iter_mut().enumerate() {
                *b = self.byte(a.wrapping_add(k as u32));
            }
        }
    }

    pub(crate) fn word(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.byte(a), self.byte(a.wrapping_add(1))])
    }
}

/// The line on which every sprite fetches its control words afresh (PAL).
pub(crate) const SPRITE_RESET_LINE: u16 = 25;

/// One sprite channel as Agnus runs it: its pointer, window, and the line's data.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Channel {
    pt: u32,
    pos: u16,
    ctl: u16,
    shown: Option<[u16; 2]>,
}

impl Channel {
    /// The line's sprite DMA: the control words on the reset line and at the stop line, a line of
    /// data inside the window.
    pub(crate) fn dma(&mut self, line: u16, chip: &Chip) {
        let (start, stop) = sprite::lines(self.pos, self.ctl);
        self.shown = None;
        if line == SPRITE_RESET_LINE || (line > SPRITE_RESET_LINE && line == stop) {
            self.pos = chip.word(self.pt);
            self.ctl = chip.word(self.pt + 2);
            self.pt += 4;
        } else if line > SPRITE_RESET_LINE && line >= start && line < stop {
            self.shown = Some([chip.word(self.pt), chip.word(self.pt + 2)]);
            self.pt += 4;
        }
    }

    pub(crate) fn line(&self) -> Option<SpriteLine> {
        self.shown.map(|data| SpriteLine {
            hstart: sprite::hstart(self.pos, self.ctl),
            data,
            attached: self.ctl & 0x80 != 0,
            cut: None,
        })
    }
}

/// The frame as Denise shows it in [`WINDOW`], with the camera at `camera`.
pub fn picture(
    data: &DisplayData,
    display: &Display,
    camera: (u16, u16),
    program: &Program,
) -> Picture {
    let chip = Chip::new(data, display, camera, program);
    run_list(&chip, h(1, LEVEL_COPPER))
}

/// The game over screen's copper list, hunk 1 (`game_over_screen`, T23).
pub const GAME_OVER_COPPER: u32 = 0x810;
/// Its five bitplane pointers' MOVE pairs, hunk 1, which `h0+$993e` points at [`GAME_OVER_PICTURE`].
const GAME_OVER_PLANES: u32 = 0x908;
/// The game over picture, hunk 4: five interleaved planes, a plane's line 40 bytes after the last.
const GAME_OVER_PICTURE: u32 = 0x1_49e8;

/// The game over screen as Denise shows it in [`WINDOW`]: `h0+$993e`'s bitplane pointers written
/// into its copper list, the list run for the frame. The play screen and sprites are left as play
/// left them; nothing in that list points at them.
pub fn game_over_picture(data: &DisplayData, display: &Display, program: &Program) -> Picture {
    let mut chip = Chip::new(data, display, (0, 0), program);
    for k in 0..5 {
        for (off, b) in pointer(GAME_OVER_PLANES + 8 * k, h(4, GAME_OVER_PICTURE + 0x28 * k)) {
            // Data read without the program has no hunk 1: nothing to write into.
            for (x, &v) in chip.h1.iter_mut().skip(off as usize).zip(&b) {
                *x = v;
            }
        }
    }
    run_list(&chip, h(1, GAME_OVER_COPPER))
}

/// The frame copper list `list` draws from `chip`.
fn run_list(chip: &Chip, list: u32) -> Picture {
    let (writes, start) = frame_writes(chip, list);
    draw(chip, &writes, start)
}

/// The copper's writes over one frame of `list`, and the bitplane setup the frame starts with: the
/// one the list leaves at its end.
pub(crate) fn frame_writes(chip: &Chip, list: u32) -> (Vec<Write>, Fetch) {
    let word = |a: u32| chip.word(a);
    let frame = amiga_clock::FRAME_LINES;
    let mut start = Fetch::default();
    for w in copper::run(&word, list, start, frame) {
        start.write(w.reg, w.value);
    }
    (copper::run(&word, list, start, frame), start)
}

fn draw(chip: &Chip, writes: &[Write], start: Fetch) -> Picture {
    let (diwstrt, diwstop) = WINDOW;
    let top = diwstrt >> 8;
    let left = (diwstrt & 0xff) as i32;
    let stop = diwstop >> 8 | (!diwstop >> 7 & 0x100);
    let width = ((diwstop & 0xff) | 0x100) - (diwstrt & 0xff);
    let mut pic = Picture::blank(left, top, width, stop - top);
    let w = width as usize;
    let mut planes = Planes {
        fetch: start,
        ..Planes::default()
    };
    let mut colours = Colours::default();
    let mut channels = [Channel::default(); 8];
    let mut next = 0;
    // Apply the writes before beam position (`line`, `h`).
    let mut apply = |line: u16,
                     hpos: u16,
                     y: Option<usize>,
                     planes: &mut Planes,
                     colours: &mut Colours,
                     channels: &mut [Channel; 8],
                     pic: &mut Picture| {
        while let Some(wr) = writes.get(next) {
            if (wr.line, wr.h) >= (line, hpos) {
                break;
            }
            next += 1;
            if colours.apply(wr, y, pic) {
                continue;
            }
            if (reg::SPR0PTH..reg::SPR0PTH + 32).contains(&wr.reg) {
                let c = &mut channels[((wr.reg - reg::SPR0PTH) / 4) as usize];
                c.pt = if wr.reg & 2 == 0 {
                    (wr.value as u32 & 0x1f) << 16 | c.pt & 0xffff
                } else {
                    c.pt & 0xffff_0000 | wr.value as u32 & 0xfffe
                };
            } else {
                planes.write(wr.reg, wr.value);
            }
        }
    };
    for line in 0..top + pic.height {
        let y = line.checked_sub(top).map(|y| y as usize);
        let first = planes.fetch.first_cycle();
        apply(
            line,
            first,
            y,
            &mut planes,
            &mut colours,
            &mut channels,
            &mut pic,
        );
        for c in &mut channels {
            c.dma(line, chip);
        }
        let fetching = planes.fetching(line);
        if let Some(y) = y {
            let row = &mut pic.pixels[y * w..(y + 1) * w];
            if fetching {
                planes.fetch_line(left, row, |_, addr, len| {
                    (addr..addr + len)
                        .map(|a| chip.byte(a))
                        .collect::<Vec<u8>>()
                });
            }
            let lines: [Option<SpriteLine>; 8] = std::array::from_fn(|n| channels[n].line());
            if lines.iter().any(Option::is_some) {
                let priority = planes.priority() as usize;
                for (x, px) in row.iter_mut().enumerate() {
                    if let Some((c, pair)) = sprite::sprite_at(&lines, left + x as i32, x as i32)
                        && (pair < priority || *px == 0)
                    {
                        *px = c;
                    }
                }
            }
        }
        // Denise shows colour 0 outside the window the copper has set for the line: the play area's
        // is 16 pixels narrower each side than the status bar's.
        if let Some(y) = y {
            let f = &planes.fetch;
            let (hstart, hstop) = ((f.diwstrt & 0xff) as i32, (f.diwstop & 0xff | 0x100) as i32);
            for (x, px) in pic.pixels[y * w..(y + 1) * w].iter_mut().enumerate() {
                let at = left + x as i32;
                if at < hstart || at >= hstop {
                    *px = 0;
                }
            }
        }
        if fetching {
            planes.advance();
        }
        apply(
            line + 1,
            0,
            y,
            &mut planes,
            &mut colours,
            &mut channels,
            &mut pic,
        );
        if let Some(y) = y {
            pic.palettes[y] = colours.end_line();
        }
    }
    pic
}

/// `picture`'s colours as RGBA, 8 bits a channel, row-major.
pub fn rgba(p: &Picture, out: &mut [u8]) {
    for (px, c) in out.chunks_exact_mut(4).zip(amiga_chips::to_rgb12(p)) {
        let ch = |s: u16| (c >> s & 0xf) as u8 * 17;
        px.copy_from_slice(&[ch(8), ch(4), ch(0), 0xff]);
    }
}
