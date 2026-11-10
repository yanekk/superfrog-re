//! The character's hardware sprites (plans/forest-dumb-level-1 T10; docs/09-play-measured.md, "Sprites,
//! bobs, playfields").
//!
//! The character is sprites 0 to 3: two attached pairs side by side, 32 pixels wide, 40 lines, 15
//! colours. Their control words and data sit in hunk 1 from [`SPRITES`], [`SPRITE_STRIDE`] bytes a
//! sprite, and the copper list points sprite DMA at them. Each pass `char_sprite_copy` copies the
//! frame at `char_anim_ptr` into their data, or clears it while the character blinks after a hit, and
//! `char_sprite_position` writes their control words from the character's place in the view.
//! Ported from `char_sprite_copy` (`h0+$9b06`) and `char_sprite_position` (`h0+$5462`), each proven
//! on the routine rig. Enemies and items are bobs blitted into the play screen (`playfield.rs`).
//!
//! Interruptible points (porting rule 1): none; both run inside the level-3 interrupt.
//!
//! What the sprites show is picture state; `char_unhurt_t`, which the copy counts, is game state
//! (`character::Motion`).
//!
//! # The creatures (T52)
//!
//! The five creature slots (`creature_slots`, `level::redraw::CreatureSlots`) are bobs cut into the
//! play screen each pass and restored from the map the next: `game_frame` runs `h0_2c7e` with the
//! flags of the slots the last pass drew swapped in, which copies back the map's tiles behind each
//! (`h0_5e44`), and, once the creatures have moved, `h0_2d4c`, which cuts each active one from
//! `StworkiNew` (or kind 7's sheet in hunk 4) over the screen (`h0_5820`), C and D the screen.
//! [`Creatures`] holds the ports, proven on the routine rig
//! (`fd-conform/tests/creature_drawing_routines.rs`); [`in_play`] runs them on the play screen.
//!
//! Interruptible points (porting rule 1), listed before porting: all four run inside the level-3
//! interrupt, which nothing interrupts. Each blit is preceded by a blitter-busy wait (6 in
//! `h0_5e44`, up to 2 in `h0_5820`), which the engine's blits never leave busy. The slot loops keep
//! their index in memory (`h0+$2c7a`, `$2c7c`) but finish within the pass. The beam is not waited
//! for: a creature cut after the beam has passed its lines shows on the next frame (T46's).

use crate::level::LevelCtx;
use crate::level::redraw::{Blit, CreatureSlots, IMAGE_END, LevelImage, SCREEN_LINE, SHAPES};
use crate::program::{Program, h};
use crate::render::DisplayData;
use crate::render::playfield::{SCREEN, SCREEN_BYTES, Screen};

/// Hunk 1 offset of sprite 0's control words; its data follows.
pub const SPRITES: u32 = 0xd30;
/// Bytes from one sprite to the next: control words, 40 lines of two words, end words.
pub const SPRITE_STRIDE: u32 = 0xa8;
/// A sprite's data bytes: 40 lines of two words.
pub const SPRITE_DATA: u32 = 0xa0;

/// What the character's sprites hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharSprite {
    /// Sprite 0's and 1's control words as a long (`h1+$d30`, `$dd8`); sprites 2 and 3 have it plus
    /// `$80000`, 16 pixels right.
    pub ctl: u32,
    /// The frame whose data the sprites hold: the address `char_anim_ptr` held when it was copied,
    /// or `None` when the blink cleared them.
    pub frame: Option<u32>,
}

impl CharSprite {
    /// `char_sprite_copy`: copy the frame at `anim_ptr`, unless the character blinks. While
    /// `char_unhurt_t` is not 0 it counts up a pass, wrapping to 0, and the sprites are cleared on
    /// the passes it reaches 2 and 3 modulo 4.
    pub fn copy(&mut self, unhurt_t: &mut u8, anim_ptr: u32) {
        if *unhurt_t != 0 {
            *unhurt_t = unhurt_t.wrapping_add(1);
            if *unhurt_t & 3 >= 2 {
                self.frame = None;
                return;
            }
        }
        self.frame = Some(anim_ptr);
    }

    /// `char_sprite_position`: the control words from the character's place less the camera's.
    pub fn position(&mut self, char_x: u32, char_y: u32, camera_x: u16, camera_y: u16) {
        self.ctl = control(char_x, char_y, camera_x, camera_y);
    }

    /// The sprites' bytes in hunk 1 as the copy and position leave them, as (hunk 1 offset, bytes),
    /// the frame's read through `frame(address, len)`. Writes the original makes, in its order.
    pub fn bytes(&self, frame: &impl Fn(u32, u32) -> Vec<u8>) -> Vec<(u32, Vec<u8>)> {
        let mut out = Vec::new();
        for n in 0..4u32 {
            let at = SPRITES + 4 + n * SPRITE_STRIDE;
            let data = match self.frame {
                Some(p) => frame(p.wrapping_add(n * SPRITE_DATA), SPRITE_DATA),
                None => vec![0; SPRITE_DATA as usize],
            };
            out.push((at, data));
        }
        for n in 0..4u32 {
            let ctl = if n < 2 {
                self.ctl
            } else {
                self.ctl.wrapping_add(0x8_0000)
            };
            out.push((SPRITES + n * SPRITE_STRIDE, ctl.to_be_bytes().to_vec()));
        }
        out
    }
}

/// The control words (SPRxPOS in the high word, SPRxCTL in the low) the original computes for a
/// character at (`char_x`, `char_y`) in a view at (`camera_x`, `camera_y`), with its widths: the
/// differences are words in longs, so what the high words hold reaches the result as it does there.
pub fn control(char_x: u32, char_y: u32, camera_x: u16, camera_y: u16) -> u32 {
    let low = |v: u32, w: u16| v & 0xffff_0000 | (v as u16).wrapping_sub(w) as u32;
    let x = low(char_x, camera_x);
    let hbit = x & 1;
    let x = (x >> 1).wrapping_add(0x3c).rotate_left(16);
    let y = low(char_y, camera_y).wrapping_add(0x31);
    let mut d0 = x;
    if y & 0x100 != 0 {
        d0 |= 4;
    }
    let start = y.rotate_left(16) << 8;
    let stop = y.wrapping_add(0x28);
    if stop & 0x100 != 0 {
        d0 |= 2;
    }
    d0 | start | (stop & 0xff) << 8 | 0x80 | hbit
}

// ---------------------------------------------------------------------------------------------
// The creatures (T52)
// ---------------------------------------------------------------------------------------------

/// Hunk 0 offset of `creature_slots`: five slots of 8 bytes (x, y, kind words; facing, phase bytes).
pub const CREATURE_SLOTS: u32 = 0x67ec;
/// A slot's bytes.
const SLOT_BYTES: u32 = 8;
/// A slot's kind, facing and phase bytes from its start.
const SLOT_KIND: u32 = 4;
const SLOT_FACING: u32 = 6;
const SLOT_PHASE: u32 = 7;
/// Hunk 0 offset of the frame tables' pointers, one long a kind: each table holds a frame's byte
/// offset into the sheet, 4 bytes a phase, the right-facing frames `$20` on.
pub const FRAME_TABLES: u32 = 0x3a18;
/// The phase byte's bits that pick the frame, and how far on the right-facing frames are.
const PHASE_FRAME: u8 = 0x1c;
const FACING_RIGHT: u32 = 0x20;
/// Hunk 0 offsets of the tile map (words, a column [`TILE_COLUMN`] bytes) and the tile table (a
/// tile's byte offset into `_GenN-1`, a long a tile).
const TILE_MAP: u32 = 0x1_6318;
const TILE_TABLE: u32 = 0x3_b988;
const TILE_COLUMN: u32 = 0xc0;
/// Chip addresses the drawing blits from and to: `_GenN-1`'s tiles, `StworkiNew`'s pictures and
/// masks (hunk 3), kind 7's pictures and masks (hunk 4), and the play screen.
pub const TILES: u32 = h(3, 0x1_3540);
pub const CREATURE_PICTURES: u32 = h(3, 0x2_0c18);
pub const CREATURE_MASKS: u32 = h(3, 0x2_bb18);
pub const KIND_7_PICTURES: u32 = h(4, 0x3200);
pub const KIND_7_MASKS: u32 = h(4, 0x4b00);
/// The creature kind cut from hunk 4's sheet.
const KIND_7: u16 = 7;
/// The kinds `h0_5e44` restores a third row of tiles for.
const TALL_KINDS: [u16; 3] = [0, 3, 7];
/// The play screen's start and end (256 lines), and a 16-line band of it.
const SCREEN_START: u32 = h(3, SCREEN);
const SCREEN_END: u32 = SCREEN_START + 0x100 * SCREEN_LINE;
const BAND: u32 = 0x10 * SCREEN_LINE;
/// A creature's height in lines, and a line of its sheet: five planes of 40 bytes.
const CREATURE_LINES: u32 = 0x20;
const SHEET_LINE: u32 = 0xc8;
/// BLTSIZE's height step for one line of five planes, and a creature's width in words: two and the
/// word its shift spills into.
const SIZE_LINE: u32 = 0x140;
const CUT_WORDS: u32 = 3;
/// BLTCON0's channels and minterm for a cut: A, B, C, D; D = A·B + ¬A·C.
const CUT_MINTERM: u16 = 0xfca;
/// How far below `camera_y` `h0_5e44` counts a creature as in view.
const VIEW_LINES: u16 = 0xd0;

/// What the creature drawing keeps between its calls, at the original's addresses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CreatureDraw {
    /// `h0+$2c7a`: the slot `h0_2c7e` is at; 5 once it has run.
    pub restore_index: u16,
    /// `h0+$2c7c`: the slot `h0_2d4c` is at; 5 once it has run.
    pub cut_index: u16,
    /// `h0+$5818`, `$581a`: the place of the creature being drawn, copied from its slot.
    pub x: u16,
    pub y: u16,
    /// `h0+$581e`: cleared by `h0_2c7e` before each restore; nothing reached in W1-1 reads it.
    pub restoring: u16,
    /// `creature_active+6` (`h0+$67e0`): the flags as the pass found them, kept while `h0_2c7e` runs
    /// with [`CreatureDraw::drawn`] in their place. The byte after each set of five is padding the
    /// original copies along, always 0.
    pub saved: [u8; 5],
    /// `creature_active+$c` (`h0+$67e6`): the flags of the slots `h0_2d4c` last drew.
    pub drawn: [u8; 5],
}

/// One blit the creature drawing starts, in the original's order. Addresses are chip addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreatureBlit {
    /// `h0_2d4c`'s set-up for the cuts: modulos D `$28`, A and B `$22`, C `$28`; A's first word
    /// mask `$ffff`, its last `$0000`.
    CutSetup,
    /// `h0_5e44`: a tile from `src` copied over 16 lines of five planes at `dst` (A to D,
    /// `BLTCON0=$09f0`). Its `first` sets the copy's modulos, masks and BLTCON; the rest reuse them.
    Restore { src: u32, dst: u32, first: bool },
    /// `h0_5820`: the creature's mask (A) and picture (B) cut over the screen at `dst` (C and D).
    Cut {
        mask: u32,
        picture: u32,
        dst: u32,
        con0: u16,
        con1: u16,
        size: u16,
    },
}

/// What the creature drawing reads and starts outside its typed state.
pub trait DrawIo {
    /// The byte at hunk 0 offset `a`, wrapping: the tile map, the tile table, the frame tables, and
    /// a slot past the fifth.
    fn rb(&self, a: u32) -> u8;
    /// A blit started.
    fn blit(&mut self, b: CreatureBlit);

    fn rw(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a.wrapping_add(1))])
    }

    fn rl(&self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a.wrapping_add(2)) as u32
    }
}

/// The creature drawing over its state and memory.
pub struct Creatures<'a> {
    pub draw: &'a mut CreatureDraw,
    pub slots: &'a mut CreatureSlots,
    /// `camera_y`.
    pub camera_y: u16,
    pub io: &'a mut dyn DrawIo,
}

impl Creatures<'_> {
    /// Slot `i`'s byte at `off`: the typed slots for the five, memory past them, as the original's
    /// `slot * 8` indexing reaches.
    fn slot_byte(&self, i: u16, off: u32) -> u8 {
        match self.slots.slots.get(i as usize) {
            Some(s) => {
                let b = [
                    s.x.to_be_bytes(),
                    s.y.to_be_bytes(),
                    s.kind.to_be_bytes(),
                    [s.facing, s.phase],
                ];
                b[off as usize / 2][off as usize % 2]
            }
            None => self
                .io
                .rb(CREATURE_SLOTS.wrapping_add(i as u32 * SLOT_BYTES + off)),
        }
    }

    /// The current slot's (`h0+$581c`) kind.
    fn kind(&self) -> u16 {
        let i = self.slots.current;
        u16::from_be_bytes([
            self.slot_byte(i, SLOT_KIND),
            self.slot_byte(i, SLOT_KIND + 1),
        ])
    }

    /// The copy of slot `i`'s place the drawing works from, with `h0+$581c` set to it.
    fn take_slot(&mut self, i: u16) {
        self.slots.current = i;
        let s = self.slots.slots[i as usize];
        (self.draw.x, self.draw.y) = (s.x, s.y);
    }

    /// `h0_2c7e` (`h0+$2c7e`): for each slot, by `h0+$2c7a` from 0 to 4, whose flag is set, its place
    /// taken ([`Creatures::restore`]'s input, `h0+$581e` cleared) and the tiles behind it restored.
    pub fn restore_all(&mut self) {
        self.draw.restore_index = 0;
        while self.draw.restore_index != 5 {
            let i = self.draw.restore_index;
            if self.slots.active[i as usize] != 0 {
                self.take_slot(i);
                self.draw.restoring = 0;
                self.restore();
            }
            self.draw.restore_index += 1;
        }
    }

    /// `h0_5e44` (`h0+$5e44`): the map's tiles copied back over the creature at (`x`, `y`), three
    /// across from its cell, on the cell's row and the next. The next row is the map's 16 rows higher
    /// when `y + $10` is past `camera_y + $d0`. Kinds 0, 3 and 7 not on a cell's top line get a third
    /// row while `y + $20` is above `camera_y + $d0`. The screen wraps at its 256th line.
    pub fn restore(&mut self) {
        let (x, y) = (self.draw.x as u32, self.draw.y as u32);
        let column = (x >> 3) & 0xfffe;
        let mut dst = SCREEN_START + column + (y & 0xf0) * SCREEN_LINE;
        let mut cell = TILE_MAP + column * (TILE_COLUMN / 2) + ((y >> 3) & 0xfffe);
        let mut first = true;
        let mut row = |c: &mut Self, cell: &mut u32, dst: &mut u32| {
            for k in 0..3 {
                if k > 0 {
                    *cell = cell.wrapping_add(TILE_COLUMN);
                    *dst = dst.wrapping_add(2);
                }
                let tile = c.io.rw(*cell) as u32;
                let src = TILES.wrapping_add(c.io.rl(TILE_TABLE.wrapping_add(tile << 2)));
                c.io.blit(CreatureBlit::Restore {
                    src,
                    dst: *dst,
                    first,
                });
                first = false;
            }
        };
        // Back two columns, down a row and a band, wrapping at the screen's end (a signed compare).
        let next_row = |cell: &mut u32, dst: &mut u32| {
            *cell = cell.wrapping_sub(2 * TILE_COLUMN).wrapping_add(2);
            *dst = dst.wrapping_sub(4).wrapping_add(BAND);
            if (dst.wrapping_sub(SCREEN_END) as i32) >= 0 {
                *dst = dst.wrapping_sub(0x100 * SCREEN_LINE);
            }
        };
        row(self, &mut cell, &mut dst);
        let view_end = self.camera_y.wrapping_add(VIEW_LINES);
        if (view_end.wrapping_sub(self.draw.y.wrapping_add(0x10)) as i16) < 0 {
            cell = cell.wrapping_sub(0x20);
        }
        next_row(&mut cell, &mut dst);
        row(self, &mut cell, &mut dst);
        if !TALL_KINDS.contains(&self.kind()) || y & 0xf == 0 {
            return;
        }
        if y + CREATURE_LINES >= self.camera_y as u32 + VIEW_LINES as u32 {
            return;
        }
        next_row(&mut cell, &mut dst);
        row(self, &mut cell, &mut dst);
    }

    /// `h0_2d4c` (`h0+$2d4c`): the cuts' modulos and masks set, then for each slot, by `h0+$2c7c` from
    /// 0 to 4, whose flag is set, its place taken and the creature cut ([`Creatures::cut`]).
    pub fn cut_all(&mut self) {
        self.io.blit(CreatureBlit::CutSetup);
        self.draw.cut_index = 0;
        while self.draw.cut_index != 5 {
            let i = self.draw.cut_index;
            if self.slots.active[i as usize] != 0 {
                self.take_slot(i);
                self.cut();
            }
            self.draw.cut_index += 1;
        }
    }

    /// `h0_5820` (`h0+$5820`): cut the current slot's creature over the play
    /// screen, wrapping to its top when the `$20` lines run past line `$ff`.
    pub fn cut(&mut self) {
        let kind = self.kind();
        let frame = self.cut_frame(kind);
        let (mask_base, picture_base) = if kind == KIND_7 {
            (KIND_7_MASKS, KIND_7_PICTURES)
        } else {
            (CREATURE_MASKS, CREATURE_PICTURES)
        };
        let mask = mask_base.wrapping_add(frame);
        let picture = picture_base.wrapping_add(frame);
        let y_low = (self.draw.y & 0xff) as u32;
        let column = (self.draw.x.wrapping_shr(3) & !1) as u32;
        let dst = SCREEN_START
            .wrapping_add(y_low.wrapping_mul(SCREEN_LINE))
            .wrapping_add(column);
        if y_low.wrapping_add(CREATURE_LINES) & 0x100 == 0 {
            self.emit_cut(mask, picture, dst, CREATURE_LINES);
            return;
        }
        let mut top = y_low ^ 0xff;
        if top != CREATURE_LINES - 1 {
            top = top.wrapping_add(1);
        }
        self.emit_cut(mask, picture, dst, top);
        let bottom = y_low.wrapping_add(CREATURE_LINES) & 0xff;
        if bottom == 0 {
            return;
        }
        let advance = top.wrapping_mul(SHEET_LINE);
        self.emit_cut(
            mask.wrapping_add(advance),
            picture.wrapping_add(advance),
            SCREEN_START.wrapping_add(column),
            bottom,
        );
    }

    /// The current slot's frame offset into its sheet, from its kind's table
    /// by phase and facing; part of `h0_5820` (`h0+$5820`).
    fn cut_frame(&self, kind: u16) -> u32 {
        let table = self
            .io
            .rl(FRAME_TABLES.wrapping_add(kind.wrapping_mul(4) as u32));
        // The table's pointer is a relocated address; `io` reads by hunk 0 offset.
        let base = table.wrapping_sub(h(0, 0));
        let phase = self.slot_byte(self.slots.current, SLOT_PHASE);
        let facing = self.slot_byte(self.slots.current, SLOT_FACING);
        let mut entry = (phase & PHASE_FRAME) as u32;
        if facing & 0x80 == 0 {
            entry = entry.wrapping_add(FACING_RIGHT);
        }
        self.io.rl(base.wrapping_add(entry))
    }

    /// Cut `lines` of the sheet at `mask` and `picture` over the screen at
    /// `dst`, shifted by the drawn place's low nibble; part of `h0_5820`
    /// (`h0+$5820`).
    fn emit_cut(&mut self, mask: u32, picture: u32, dst: u32, lines: u32) {
        let shift = (self.draw.x & 0xf).wrapping_shl(12);
        self.io.blit(CreatureBlit::Cut {
            mask,
            picture,
            dst,
            con0: shift.wrapping_add(CUT_MINTERM),
            con1: shift,
            size: lines.wrapping_mul(SIZE_LINE).wrapping_add(CUT_WORDS) as u16,
        });
    }
}

/// The creature drawing's [`DrawIo`] in play: the level's buffers from their image, the program's
/// tables from hunk 0, and the blits drawn into the play screen at once, from the display's data.
pub struct PlayDraw<'a> {
    pub image: &'a LevelImage,
    pub program: &'a Program,
    pub screen: &'a mut Screen,
    pub data: &'a DisplayData,
    /// The pass's cuts and what each drew over (T80).
    pub cuts: &'a mut CutsDrawn,
}

/// A creature's cut in blit rows: `$20` lines of five planes.
pub const CUT_ROWS: u16 = (CREATURE_LINES * 5) as u16;

/// The cuts the last pass drew, in order, and the screen's words each drew over: what
/// [`CutsDrawn::uncut`] needs to draw a frame the original showed with a cut not yet made (T80).
/// Picture state that no drawing reads back, so it compares equal whatever it holds.
#[derive(Debug, Clone, Default)]
pub struct CutsDrawn {
    /// Each cut's mask, picture and destination, BLTCON0, BLTCON1 and BLTSIZE, and where its
    /// words start in `under`.
    blits: Vec<([u32; 3], [u16; 3], usize)>,
    under: Vec<u8>,
}

impl PartialEq for CutsDrawn {
    fn eq(&self, _: &CutsDrawn) -> bool {
        true
    }
}

impl Eq for CutsDrawn {}

impl std::hash::Hash for CutsDrawn {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

impl CutsDrawn {
    /// Forget the last pass's cuts: a pass that cuts nothing leaves none.
    pub fn clear(&mut self) {
        self.blits.clear();
        self.under.clear();
    }

    /// Make a cut on `screen` and keep it: `at` its mask, picture and destination, `con` its
    /// BLTCON0, BLTCON1 and BLTSIZE.
    fn draw(
        &mut self,
        screen: &mut Screen,
        chip: &impl Fn(u32) -> u8,
        at: [u32; 3],
        con: [u16; 3],
    ) {
        self.blits.push((at, con, self.under.len()));
        let under = Some(&mut self.under);
        cut_screen(screen, chip, at, con[0], con[1], con[2], 0, under);
    }

    /// How many creatures the pass cut. One wrapping past the screen's last line is two blits.
    pub fn creatures(&self) -> usize {
        let rows: u32 = self.blits.iter().map(|b| (b.1[2] >> 6) as u32).sum();
        (rows / CUT_ROWS as u32) as usize
    }

    /// `screen` as it was with the first `rows[k]` blit rows of the `k`th creature's cut not yet
    /// written: a row is one plane of one line, a line's five planes in turn from the creature's
    /// top. Every cut is taken back, last first, and made again without those rows, so a creature
    /// cut over another keeps what it covers. `None` unless `rows` has one count a creature.
    pub fn uncut(
        &self,
        screen: &Screen,
        chip: &impl Fn(u32) -> u8,
        rows: &[u16],
    ) -> Option<Screen> {
        if rows.len() != self.creatures() {
            return None;
        }
        let mut screen = screen.clone();
        for (at, con, from) in self.blits.iter().rev() {
            let mut under = self.under[*from..].chunks_exact(2);
            cut_words(at[2], con[2], |_, i| {
                if let (Some(i), Some(w)) = (i, under.next()) {
                    screen.bytes[i..i + 2].copy_from_slice(w);
                }
            });
        }
        // The rows of the creature's `CUT_ROWS` made so far, and those still to leave out.
        let (mut creature, mut made, mut left) = (0, 0u32, rows.first().copied()? as u32);
        for (at, con, _) in &self.blits {
            let n = (con[2] >> 6) as u32;
            let skip = left.min(n);
            cut_screen(&mut screen, chip, *at, con[0], con[1], con[2], skip, None);
            (made, left) = (made + n, left - skip);
            if made >= CUT_ROWS as u32 {
                creature += 1;
                (made, left) = (0, rows.get(creature).copied().unwrap_or(0) as u32);
            }
        }
        Some(screen)
    }
}

impl DrawIo for PlayDraw<'_> {
    fn rb(&self, a: u32) -> u8 {
        if (SHAPES..IMAGE_END).contains(&a) {
            self.image.byte(a)
        } else {
            self.program.byte(a)
        }
    }

    fn blit(&mut self, b: CreatureBlit) {
        let chip = |a: u32| self.data.byte(a);
        match b {
            // The modulos and masks `cut_screen` assumes. It comes once a pass, before the cuts.
            CreatureBlit::CutSetup => self.cuts.clear(),
            // `h0_774`'s blit, which `Blit::Cell` draws.
            CreatureBlit::Restore { src, dst, .. } => self.screen.apply(
                &Blit::Cell {
                    src: src.wrapping_sub(TILES),
                    dst: dst.wrapping_sub(SCREEN_START),
                },
                &chip,
            ),
            CreatureBlit::Cut {
                mask,
                picture,
                dst,
                con0,
                con1,
                size,
            } => self
                .cuts
                .draw(self.screen, &chip, [mask, picture, dst], [con0, con1, size]),
        }
    }
}

/// A cut's size as the blitter counts it: rows, and words a row.
fn cut_size(size: u16) -> (u32, u32) {
    let rows = match size >> 6 {
        0 => 1024,
        r => r as u32,
    };
    let words = match size & 63 {
        0 => 64,
        w => w as u32,
    };
    (rows, words)
}

/// Each word a cut of `size` at `dst` writes, in the blitter's order: its row and its index in the
/// kept screen, `None` outside it.
fn cut_words(mut d: u32, size: u16, mut f: impl FnMut(u32, Option<usize>)) {
    const CHIP: u32 = 0x7_fffe;
    let (rows, words) = cut_size(size);
    for row in 0..rows {
        for _ in 0..words {
            let i = (d & CHIP).wrapping_sub(SCREEN_START);
            f(row, (i + 1 < SCREEN_BYTES).then_some(i as usize));
            d = d.wrapping_add(2);
        }
        d = d.wrapping_add(0x28);
    }
}

/// A cut as the blitter runs it after `h0_2d4c`'s set-up: ascending, A (`mask`) and B (`picture`)
/// shifted by BLTCON0's and BLTCON1's top nibbles, each carrying the bits it shifts out into its next
/// word, rows and lines included; A's last word masked to 0; D = A·B + ¬A·C, C and D the screen. A
/// word outside the kept screen reads 0 and is dropped, as `Screen::apply`'s. The first `skip` rows
/// are left unwritten, and `under` takes each kept word as it was before the cut (T80).
#[allow(clippy::too_many_arguments)]
fn cut_screen(
    screen: &mut Screen,
    chip: &impl Fn(u32) -> u8,
    [mut a, mut b, d]: [u32; 3],
    con0: u16,
    con1: u16,
    size: u16,
    skip: u32,
    mut under: Option<&mut Vec<u8>>,
) {
    const CHIP: u32 = 0x7_fffe;
    let word = |a: u32| u16::from_be_bytes([chip(a & CHIP), chip((a & CHIP) + 1)]);
    let (_, words) = cut_size(size);
    let (ash, bsh) = (con0 >> 12, con1 >> 12);
    let (mut a_old, mut b_old) = (0u16, 0u16);
    let mut x = 0;
    cut_words(d, size, |row, i| {
        let aw = if x == words - 1 { 0 } else { word(a) };
        let bw = word(b);
        let sa = ((a_old as u32) << 16 | aw as u32) >> ash;
        let sb = ((b_old as u32) << 16 | bw as u32) >> bsh;
        (a_old, b_old) = (aw, bw);
        if let Some(i) = i {
            if let Some(u) = under.as_mut() {
                u.extend_from_slice(&screen.bytes[i..i + 2]);
            }
            if row >= skip {
                let c = u16::from_be_bytes([screen.bytes[i], screen.bytes[i + 1]]);
                let v = (sa as u16 & sb as u16) | (!sa as u16 & c);
                screen.bytes[i..i + 2].copy_from_slice(&v.to_be_bytes());
            }
        }
        (a, b) = (a.wrapping_add(2), b.wrapping_add(2));
        x += 1;
        if x == words {
            x = 0;
            a = a.wrapping_add(0x22);
            b = b.wrapping_add(0x22);
        }
    });
}

/// Run `f` on the creature drawing in play: the play screen drawn into as it goes.
pub fn in_play(l: &mut LevelCtx, f: impl FnOnce(&mut Creatures)) {
    let s = &mut *l.state;
    let mut io = PlayDraw {
        image: l.image,
        program: l.program,
        screen: &mut l.display.screen,
        data: l.display_data,
        cuts: &mut l.display.cuts,
    };
    f(&mut Creatures {
        draw: &mut s.creature_draw,
        slots: &mut s.edges.creatures,
        camera_y: s.camera.y,
        io: &mut io,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_words_place_the_sprite_and_attach_it() {
        // At the view's top-left: VSTART $31, HSTART $3c << 1, VSTOP $59, attached.
        let c = control(0x10, 0x10, 0x10, 0x10);
        assert_eq!(c, 0x313c_5980);
        assert_eq!(
            amiga_chips::sprite::lines((c >> 16) as u16, c as u16),
            (0x31, 0x59)
        );
        // An odd x sets HSTART's low bit; a low view sets the ninth bits.
        assert_eq!(control(0x11, 0xe0, 0x10, 0x10) & 0x7, 0x7);
    }

    /// Two creatures cut one over the other, then drawn as a frame that caught some rows uncut.
    #[test]
    fn an_uncut_row_shows_what_the_cut_drew_over_and_a_later_cut_stays() {
        // Masks and pictures all ones: a cut writes ones over two words a row.
        let chip = |a: u32| if a < 0x1_0000 { 0xff } else { 0 };
        let mut before = Screen::default();
        before.bytes.fill(0x11);
        let dst = SCREEN_START + 0x10 * SCREEN_LINE;
        let con = |lines: u32| [CUT_MINTERM, 0, (lines * SIZE_LINE + CUT_WORDS) as u16];
        let (mut after, mut cuts) = (before.clone(), CutsDrawn::default());
        // The first in two blits, as one wrapping past the screen's end; the second 8 lines lower,
        // over the first's last 24 lines and 8 lines more.
        let lower = dst + 0x8 * SCREEN_LINE;
        cuts.draw(&mut after, &chip, [0, 0, dst], con(0x8));
        cuts.draw(&mut after, &chip, [0, 0, lower], con(0x18));
        cuts.draw(&mut after, &chip, [0, 0, lower], con(0x20));
        assert_eq!(cuts.creatures(), 2);
        assert_ne!(after, before);
        // One count a creature, or nothing.
        assert_eq!(cuts.uncut(&after, &chip, &[0]), None);
        assert_eq!(cuts.uncut(&after, &chip, &[0, 0]), Some(after.clone()));
        let all = [CUT_ROWS, CUT_ROWS];
        assert_eq!(cuts.uncut(&after, &chip, &all), Some(before.clone()));
        // The first creature's first 9 lines and 2 planes uncut, its first blit whole and 7 rows
        // of its second: the second creature, cut over it from line 8 on, hides all but line 0 to 7.
        let part = cuts.uncut(&after, &chip, &[47, 0]).unwrap();
        let (mut second, mut alone) = (before.clone(), CutsDrawn::default());
        alone.draw(&mut second, &chip, [0, 0, lower], con(0x20));
        assert_eq!(part, second);
        // The second's first line uncut shows the first creature there, not the screen before it.
        let part = cuts.uncut(&after, &chip, &[0, 5]).unwrap();
        let (mut first, mut alone) = (before.clone(), CutsDrawn::default());
        alone.draw(&mut first, &chip, [0, 0, dst], con(0x20));
        let line = |s: &Screen, l: u32| {
            let at = ((0x10 + l) * SCREEN_LINE) as usize;
            s.bytes[at..at + SCREEN_LINE as usize].to_vec()
        };
        assert_eq!(line(&part, 8), line(&first, 8));
        assert_eq!(line(&part, 9), line(&after, 9));
        assert_eq!(line(&part, 0x27), line(&after, 0x27));
        // A pass that cuts nothing leaves no creature to draw uncut.
        cuts.clear();
        assert_eq!(
            (cuts.creatures(), cuts.uncut(&after, &chip, &[1])),
            (0, None)
        );
    }

    #[test]
    fn the_blink_clears_two_passes_in_four_and_wraps_to_steady() {
        let mut s = CharSprite {
            ctl: 0,
            frame: None,
        };
        let mut t = 0xfcu8;
        let mut shown = Vec::new();
        for _ in 0..6 {
            s.copy(&mut t, 0x1234);
            shown.push(s.frame.is_some());
        }
        // $fd, $fe, $ff clear on $fe and $ff; 0 shows, and then it stays 0.
        assert_eq!(shown, [true, false, false, true, true, true]);
        assert_eq!(t, 0);
    }
}
