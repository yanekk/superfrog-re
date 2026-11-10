//! The foreground's edge redraws (plans/forest-dumb-level-1 T55; games/forest-dumb/docs/09-play-measured.md,
//! "The edge redraws"). When the camera crosses a 16-pixel cell it latches that direction (T09,
//! `level/camera.rs`); `main_loop` then redraws the new edge: one row (`redraw_edge_a` below the view,
//! `redraw_edge_b` above) or column (`redraw_edge_c` left, `redraw_edge_d` right) of tiles onto the
//! play screen, the same cells' collision shapes into `collision_map`, and a scan of the object map
//! just outside that edge that starts creatures, adds animated cells and cuts bobs over the tiles.
//!
//! Ported from `redraw_edge_a..d` (`h0+$844`, `$b20`, `$df6`, `$114a`), the tile blit `h0_774`, the
//! shape copy `h0_7cc`, the four bob blits `h0_a06`, `h0_ce4`, `h0_104c`, `h0_1390`, the blitter wait
//! `h0_3ac2`, the creature start `h0_681c` with its slot fill `h0_6852`, and `h0_6882`, which adds an
//! animated cell. Each is proven on the routine rig (`fd-conform/tests/edge_redraw_routines.rs`).
//!
//! The arithmetic is the original's, at its widths: positions are words, cell and buffer offsets longs
//! that wrap, so a position past the level reads and writes where the original's would. The level
//! buffers are read through [`Io`] at their hunk 0 offsets for that reason. One register is carried:
//! `redraw_edge_a` and `_b` load only the low word of `d0` before a bob, so its high word is whatever
//! the last cell left (0, or `$ffff` after a creature found no free slot), and the bob lands there.
//!
//! What the engine models of the picture is the list of blits, in order ([`Blit`]): the play screen is
//! picture state, not game state (docs/09), and T10 draws it. `collision_map` is game state
//! (`level/collision.rs`, `CollisionPlane`).

/// Hunk 0 offsets of the level buffers the redraws read: `_GenN-1.Mapa` (32-byte shapes), the object
/// map (`_MapaBobowN`), the tile map (`_MapaN-1`, words), the shape map (`_MapaN-1.Mapa`, bytes). The
/// maps run 96 cells to a column.
pub const SHAPES: u32 = 0xbda0;
pub const OBJECT_MAP: u32 = 0xcb38;
pub const TILE_MAP: u32 = 0x1_6318;
pub const SHAPE_MAP: u32 = 0x2_8f18;
/// Bytes a map column, tile map (words) and the byte maps.
const TILE_COLUMN: u32 = 0xc0;
const BYTE_COLUMN: u32 = 0x60;
/// Bytes a `collision_map` row.
const ROW_BYTES: u32 = 0x2e;
/// A play screen line: five interleaved planes of `$2e` bytes.
pub const SCREEN_LINE: u32 = 0xe6;

/// The band, 0 to 15, entry `k` of the program's two pointer tables holds: `h0+$1496` (the play
/// screen's 16-line bands, `h3+$398 + band * $e60`) and `h0+$18d6` (`collision_map`'s 16-row bands,
/// `h3+$fa50 + band * $2e0`). Sixteen blocks of sixteen; entry `j` of block `b` is band
/// `(j - 1 - b) & 15`, so a tile row `r` always lands in band `r & 15`. The tables are the program's
/// own data; `fd-conform/tests/edge_redraw_routines.rs` holds this against them.
pub fn band(k: u32) -> u32 {
    (k & 15).wrapping_sub(1).wrapping_sub(k >> 4) & 15
}

/// Entry `k`'s play screen offset from `h3+$398`.
pub fn screen_band(k: u32) -> u32 {
    band(k) * 16 * SCREEN_LINE
}

/// Entry `k`'s `collision_map` offset from `h3+$fa50`.
pub fn collision_band(k: u32) -> u32 {
    band(k) * 16 * ROW_BYTES
}

/// One blit a redraw starts, in the original's order. `dst` is a byte offset from the play screen
/// (`h3+$398`), wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blit {
    /// `h0_774`: tile `tile` of `_GenN-1` (at `h3+$13540`, `$a0` bytes each) copied over 16 lines of
    /// five planes (A to D, `BLTCON0=$09f0`).
    Tile { tile: u16, dst: u32 },
    /// `h0_a06` and its twins: the `_ELEMENTY` bob at byte `image` of its pictures (`h4+$c80`, B) and
    /// masks (`h4+$2580`, A) cut over the screen (C and D, `BLTCON0=$0fca`).
    Bob { image: u32, dst: u32 },
    /// `h0_29ca` (T22, `objects::collectible`): a taken cell's tile redrawn as `h0_774` draws one, its
    /// source `src` bytes past `h3+$13540`, the offset the tile table at `h0+$3b988` holds for it.
    Cell { src: u32, dst: u32 },
    /// `h0_62b6` (T25, `objects::animated`): an `_ELEMENTY` frame cut over its cell's tile in one blit
    /// (`BLTCON0=$0fca`, D = A·B + ¬A·C): A the mask at `h4+$1900 + 2 * frame`, B the picture at
    /// `h4 + 2 * frame` (modulos `$26`), C the tile `tile` bytes past `h3+$13540` (modulo 0).
    Element { frame: u32, tile: u32, dst: u32 },
    /// `h0_21c0` and `h0_62b6` (T25): a 32 x 16 picture copied over the screen (A to D,
    /// `BLTCON0=$09f0`, two words by 80 rows, D's modulo `$2a`), from `src` bytes into hunk 4, its rows
    /// `4 + modulo` bytes apart.
    Picture { src: u32, modulo: u16, dst: u32 },
}

/// What the redraws read and write outside their typed state.
pub trait Io {
    /// The byte at hunk 0 offset `a`: the level buffers.
    fn rb(&self, a: u32) -> u8;
    /// `clr.b (a2)`: the object map cell at hunk 0 offset `a` is cleared.
    fn clear_object(&mut self, a: u32);
    /// A blit started.
    fn blit(&mut self, b: Blit);
    /// A word written to `collision_map` at `at` bytes from `h3+$fa50`, wrapping.
    fn collision_word(&mut self, at: u32, v: u16);
    /// Whether the animated cell or creature the scan found at object map cell `cell` (hunk 0
    /// offset) is added or started after the next pass: that pass cut the scan before it (T74, T75,
    /// [`PlayIo::defer_from`]).
    fn defers(&self, _cell: u32) -> bool {
        false
    }

    fn rw(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a.wrapping_add(1))])
    }
}

/// A long the redraws leave at `h0+$840` or `h0+$83c`, which nothing else reads: a pointer into the
/// play screen or `collision_map` (kept as an offset from its start) or a plain value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ptr {
    Raw(u32),
    Screen(u32),
    Collision(u32),
}

impl Default for Ptr {
    fn default() -> Ptr {
        Ptr::Raw(0)
    }
}

/// One creature slot (`creature_slots`, `h0+$67ec`, 8 bytes).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CreatureSlot {
    pub x: u16,
    pub y: u16,
    /// The object value less one.
    pub kind: u16,
    /// `$ff` or 1.
    pub facing: u8,
    pub phase: u8,
}

/// The five creature slots and their flags. The redraws start creatures here; the creature routines
/// (T24) run them. `h0_681c` is also called from `h0+$3a38`'s list of fixed starts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CreatureSlots {
    /// `creature_active` (`h0+$67da`): a byte a slot, 0 free, `$ff` once started.
    pub active: [u8; 5],
    pub slots: [CreatureSlot; 5],
    /// `h0+$581c`, a word: the slot last started, which the creature routines also use.
    pub current: u16,
    /// `h0+$687e`, a long: the same, `h0_6852`'s own copy.
    pub last: u32,
    /// `creature_slot_index` (`h0+$2faa`), a long: the slot the per-pass loops over the five are at;
    /// `h0_2dd6` leaves it at 5 (T22, `objects::collectible`).
    pub index: u32,
}

impl CreatureSlots {
    /// `h0_681c`: the first free slot of five takes the creature. Returns whether none was free (the
    /// original's `d0 = -1`), when nothing changes.
    pub fn start(&mut self, x: u16, y: u16, kind: u16, facing: u8) -> bool {
        match self.active.iter().position(|&f| f == 0) {
            Some(i) => {
                self.current = i as u16;
                self.fill(i, x, y, kind, facing);
                false
            }
            None => true,
        }
    }

    /// `h0_6852`: slot `i` takes the creature, its phase 0, its flag `$ff`. Only `h0_681c` calls it,
    /// with a free slot.
    pub fn fill(&mut self, i: usize, x: u16, y: u16, kind: u16, facing: u8) {
        self.last = i as u32;
        self.slots[i] = CreatureSlot {
            x,
            y,
            kind,
            facing,
            phase: 0,
        };
        self.active[i] = 0xff;
    }
}

/// One animated cell (`anim_cells`, `h2+$16b0`, 8 bytes).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AnimCell {
    pub x: u16,
    pub y: u16,
    /// The object value.
    pub code: u8,
    /// Three bytes `h0_62b6` keeps (T25); a redraw leaves them.
    pub rest: [u8; 3],
}

/// `h0_62b6`'s four lists of 12 animated cells (T25 runs them).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct AnimatedCells {
    /// `anim_cell_used` (`h2+$1680`): a byte a cell, 0 free.
    pub used: [[u8; 12]; 4],
    pub cells: [[AnimCell; 12]; 4],
}

impl AnimatedCells {
    /// `h0_6882`: the cell at (`x`, `y`) with object value `code` joins the list with the fewest
    /// cells in use (the first such), at its first free entry; a full list takes nothing.
    pub fn add(&mut self, x: u16, y: u16, code: u8) {
        let count = |g: usize| self.used[g].iter().filter(|&&f| f != 0).count();
        // The original sorts the four counts with a stable bubble sort and takes the first.
        let g = (1..4).fold(0, |best, g| if count(g) < count(best) { g } else { best });
        if let Some(i) = self.used[g].iter().position(|&f| f == 0) {
            let c = &mut self.cells[g][i];
            (c.x, c.y, c.code) = (x, y, code);
            self.used[g][i] = 0xff;
        }
    }
}

/// What the redraws keep between them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Edges {
    /// `h0+$1e66`, `$1e68`: where the redraw draws from, which `main_loop` copies from the latched
    /// direction's `edge_at` before the call.
    pub pos: (u16, u16),
    /// `h0+$840`: the shape column's `collision_map` pointer (a, b) or the band block (c, d).
    pub p840: Ptr,
    /// `h0+$83c`: the tile row's screen pointer (a, b).
    pub p83c: Ptr,
    pub creatures: CreatureSlots,
    pub cells: AnimatedCells,
    /// The animated cells and creature cells a redraw's scan found after the pass that cut it, in
    /// the order found: `main_loop` adds them (`h0_6882`) and starts them (`h0_681c`) once that
    /// pass has ended, before it tests the next latch (T74, T75, `level::main_loop_redraws`). Empty
    /// but for the pass after such a cut.
    pub deferred: Vec<Deferred>,
}

/// What a cut scan does after the pass that cut it ([`Edges::deferred`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Deferred {
    /// `h0_6882` for this cell.
    Cell(AnimCell),
    /// `h0_681c` for the creature at object map cell `cell` (hunk 0 offset), which is cleared if a
    /// slot takes it. Which slot is free is decided then: the pass between can free one.
    Creature {
        cell: u32,
        x: u16,
        y: u16,
        kind: u16,
        facing: u8,
    },
    /// The cut redraw's `n`th `collision_map` word, `word` at `at` bytes from the map's first: the
    /// pass came before `h0_7cc` copied it (T48, `level::EdgesCut::shape_words`).
    Collision { n: u16, at: u32, word: u16 },
}

/// N after `cmpi.w #imm, v`.
fn minus(v: u16, imm: u16) -> bool {
    v.wrapping_sub(imm) & 0x8000 != 0
}

/// What an object map byte asks the scan for, by the original's signed byte compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Code {
    Skip,
    /// `$ff`: the scan ends.
    End,
    /// 1 to 6 and `$87` to `$fe`: `cmpi.b #7` is negative.
    Creature,
    /// `$40` to `$86`.
    Bob,
    /// `$20` to `$3f`.
    Animated,
}

fn code(b: u8) -> Code {
    let neg = |imm: u8| b.wrapping_sub(imm) & 0x80 != 0;
    match b {
        0 => Code::Skip,
        0xff => Code::End,
        _ if neg(7) => Code::Creature,
        _ if !neg(0x40) => Code::Bob,
        _ if !neg(0x20) => Code::Animated,
        _ => Code::Skip,
    }
}

/// `h0_3ac2`: wait for the raster to leave lines `$e7` to `$66` and for the blitter. It changes no
/// state; the engine has no beam and no blitter to wait for.
pub fn blitter_wait() {}

/// `h0_774`: tile `tile` blitted to `dst`.
pub fn tile_blit(io: &mut impl Io, tile: u16, dst: u32) {
    blitter_wait();
    io.blit(Blit::Tile { tile, dst });
}

/// `h0_7cc`: shape `n` (`d5`, a long the original shifts by 5) copied as 16 words down a
/// `collision_map` column from `at`. A shape past the 108 before the object map is read from it.
pub fn shape_copy(io: &mut impl Io, n: u32, at: u32) {
    let src = SHAPES.wrapping_add(n.wrapping_shl(5));
    for k in 0..16u32 {
        let w = io.rw(src.wrapping_add(2 * k));
        io.collision_word(at.wrapping_add(k * ROW_BYTES), w);
    }
}

/// A bob's byte offset into `_ELEMENTY`: the value less `$40` as a long, its low word doubled
/// (`subi.l #$40, d2; add.w d2, d2`).
fn bob_image(b: u8) -> u32 {
    let d2 = (b as u32).wrapping_sub(0x40);
    (d2 & 0xffff_0000) | (d2 as u16).wrapping_add(d2 as u16) as u32
}

/// `(~d4 + bias) << 4`: the scanned cell's distance in pixels, which `d4` counts down to.
fn step(d4: u32, bias: u32) -> u32 {
    (!d4).wrapping_add(bias).wrapping_shl(4)
}

fn bob(io: &mut impl Io, b: u8, dst: u32) {
    blitter_wait();
    io.blit(Blit::Bob {
        image: bob_image(b),
        dst,
    });
}

/// `h0_a06`: bob `b` on the row below the view, the scan's counter `d4`; `d0` is the register as the
/// scan left it, whose high word the column takes. Returns `d0` as the original leaves it.
pub fn bob_below(io: &mut impl Io, b: u8, (x, y): (u16, u16), d4: u32, d0: u32) -> u32 {
    let d0 = ((d0 & 0xffff_0000) | x as u32).wrapping_add(step(d4, 0x14)) >> 4 << 1;
    let d1 = (y.wrapping_add(0xc1) & 0xf0) as u32 * SCREEN_LINE;
    bob(io, b, d0.wrapping_add(d1));
    d0
}

/// `h0_ce4`: bob `b` on the row above the view, as [`bob_below`].
pub fn bob_above(io: &mut impl Io, b: u8, (x, y): (u16, u16), d4: u32, d0: u32) -> u32 {
    let d0 = ((d0 & 0xffff_0000) | x as u32).wrapping_add(step(d4, 0x14)) >> 4 << 1;
    let d1 = (y.wrapping_sub(8) & 0xf0) as u32 * SCREEN_LINE;
    bob(io, b, d0.wrapping_add(d1));
    d0
}

/// The line a column's bob lands on: `(y + (~d4 + $d) << 4) & $f0` lines.
fn column_line(y: u16, d4: u32) -> u32 {
    ((y as u32).wrapping_add(step(d4, 0xd)) & 0xf0) * SCREEN_LINE
}

/// `h0_104c`: bob `b` in the column at x.
pub fn bob_left(io: &mut impl Io, b: u8, (x, y): (u16, u16), d4: u32) {
    let d0 = (x as u32 >> 4) << 1;
    bob(io, b, d0.wrapping_add(column_line(y, d4)));
}

/// `h0_1390`: bob `b` in the column at x + `$130`.
pub fn bob_right(io: &mut impl Io, b: u8, (x, y): (u16, u16), d4: u32) {
    let d0 = (x.wrapping_add(0x130) as u32 >> 4) << 1;
    bob(io, b, d0.wrapping_add(column_line(y, d4)));
}

/// The block offset into the pointer tables for a y: `(((y & $ff) ^ $ff) >> 4) << 6`.
fn block(y: u16) -> u32 {
    (((y as u32 & 0xff) ^ 0xff) >> 4) << 6
}

/// A row's 23 tiles: 22 from `tiles` rightwards to the screen at `dst`, then the column before.
fn tile_row(io: &mut impl Io, tiles: u32, dst: u32) {
    for k in 0..22 {
        let t = io.rw(tiles.wrapping_add(k * TILE_COLUMN));
        tile_blit(io, t, dst.wrapping_add(2 * k));
    }
    let t = io.rw(tiles.wrapping_sub(TILE_COLUMN));
    tile_blit(io, t, dst.wrapping_sub(2));
}

/// The same row's 23 shapes into `collision_map` at `at`.
fn shape_row(io: &mut impl Io, shapes: u32, at: u32) {
    for k in 0..22 {
        let n = io.rb(shapes.wrapping_add(k * BYTE_COLUMN)) as u32;
        shape_copy(io, n, at.wrapping_add(2 * k));
    }
    let n = io.rb(shapes.wrapping_sub(BYTE_COLUMN)) as u32;
    shape_copy(io, n, at.wrapping_sub(2));
}

/// Where a column's 16 cells come from: rows `r` to `r + 13`, then `r - 2` and `r - 1`, as entry
/// `j` of the block (the original runs 14 then backs `a4` up by 16 cells).
fn column_cell(j: u32) -> u32 {
    if j < 14 { j } else { j.wrapping_sub(16) }
}

/// A column's 16 tiles from `tiles` (one word a row), each at its band plus `dx`.
fn tile_column(io: &mut impl Io, tiles: u32, blk: u32, dx: u32) {
    for j in 0..16u32 {
        let t = io.rw(tiles.wrapping_add(column_cell(j).wrapping_mul(2)));
        let k = (blk >> 2).wrapping_add(j);
        tile_blit(io, t, screen_band(k).wrapping_add(dx));
    }
}

/// The same column's 16 shapes.
fn shape_column(io: &mut impl Io, shapes: u32, blk: u32, dx: u32) {
    for j in 0..16u32 {
        let n = io.rb(shapes.wrapping_add(column_cell(j))) as u32;
        let k = (blk >> 2).wrapping_add(j);
        shape_copy(io, n, collision_band(k).wrapping_add(dx));
    }
}

/// A creature's kind, from its cell's byte, and its y with the original's `+ 2` for kind 0.
fn creature(b: u8, y: u32) -> (u16, u32) {
    let kind = (b as u32).wrapping_sub(1);
    let y = if kind as u8 == 0 {
        y.wrapping_add(2)
    } else {
        y
    };
    (kind as u16, y)
}

impl Edges {
    /// `bsr.w h0_6882` for the scan's cell `cell`, now or, when the next pass cut the scan before
    /// it, after that pass ([`Edges::deferred`]). The list a cell joins depends on how many cells
    /// each holds, which that pass's `h0_62b6` changes.
    fn add_cell(&mut self, io: &impl Io, cell: u32, x: u16, y: u16, code: u8) {
        if io.defers(cell) {
            self.deferred.push(Deferred::Cell(AnimCell {
                x,
                y,
                code,
                rest: [0; 3],
            }));
        } else {
            self.cells.add(x, y, code);
        }
    }

    /// Start a creature from cell `cell` (hunk 0 offset), clearing it if a slot took it. `d0` is the
    /// long the scan passes in d0, x in its low word. Returns the original's `d0`: 0 taken, -1 not.
    fn start(
        &mut self,
        io: &mut impl Io,
        cell: u32,
        d0: u32,
        y: u16,
        kind: u16,
        facing: u8,
    ) -> u32 {
        let d0 = if io.defers(cell) {
            // The next pass cut the scan before this cell (T75): the original starts the creature
            // after that pass, so it misses the pass's move and is not drawn in it. The scan's later
            // bobs are listed as after a start that found a slot; one finding none (`d0 = -1`) only
            // moves a bob's blit off the screen.
            self.deferred.push(Deferred::Creature {
                cell,
                x: d0 as u16,
                y,
                kind,
                facing,
            });
            return 0;
        } else if self.creatures.start(d0 as u16, y, kind, facing) {
            0xffff_ffff
        } else {
            0
        };
        if d0 == 0 {
            io.clear_object(cell);
        }
        d0
    }

    /// `redraw_edge_a` (`h0+$844`): the row below the view, `y + $d0`, while y is under `$53c`.
    pub fn redraw_below(&mut self, io: &mut impl Io) {
        let (x, y) = self.pos;
        if !minus(y, 0x53c) {
            return;
        }
        let (col, row) = ((x >> 4) as u32, (y >> 4) as u32);
        let k = (block(y) + 0x34) >> 2;
        let at = collision_band(k).wrapping_add(col * 2);
        self.p840 = Ptr::Collision(at);
        let dst = screen_band(k).wrapping_add(col * 2);
        self.p83c = Ptr::Screen(dst);
        let tiles = TILE_MAP.wrapping_add((row << 1) + 0x1a + col * TILE_COLUMN);
        tile_row(io, tiles, dst);
        shape_row(
            io,
            SHAPE_MAP.wrapping_add(row + 0xd + col * BYTE_COLUMN),
            at,
        );

        let row = (y.wrapping_add(0xc0) >> 4) as u32;
        // `d0` as the shape loop's `dbra` left it.
        let mut d0: u32 = 0xffff;
        for (i, d4) in (0..=0x13u32).rev().enumerate() {
            let cell = OBJECT_MAP.wrapping_add((col + i as u32) * BYTE_COLUMN + row);
            let b = io.rb(cell);
            match code(b) {
                Code::End => return,
                Code::Skip => {}
                Code::Bob => d0 = bob_below(io, b, (x, y), d4, d0),
                Code::Animated => {
                    d0 = ((d0 & 0xffff_0000) | x as u32).wrapping_add(step(d4, 0x14)) & 0xfff0;
                    self.add_cell(io, cell, d0 as u16, y.wrapping_add(0xc1) & 0xfff0, b);
                }
                Code::Creature => {
                    let cx = (x as u32).wrapping_add(step(d4, 0x14));
                    let (kind, cy) = creature(b, (y.wrapping_add(0xc1) & 0xfff0) as u32);
                    let facing = if minus(d4 as u16, 0xa) { 0xff } else { 1 };
                    d0 = self.start(io, cell, cx, cy as u16, kind, facing);
                }
            }
        }
    }

    /// `redraw_edge_b` (`h0+$b20`): the row above the view, `y - $10`, while y is at least 1; its scan
    /// at `y - 8`, while that is not negative.
    pub fn redraw_above(&mut self, io: &mut impl Io) {
        let (x, y) = self.pos;
        if minus(y, 1) {
            return;
        }
        let (col, row) = ((x >> 4) as u32, (y >> 4) as u32);
        let k = (block(y) + 0x38) >> 2;
        let at = collision_band(k).wrapping_add(col * 2);
        self.p840 = Ptr::Collision(at);
        let dst = screen_band(k).wrapping_add(col * 2);
        self.p83c = Ptr::Screen(dst);
        let tiles =
            TILE_MAP.wrapping_add(((row << 1).wrapping_sub(4)).wrapping_add(col * TILE_COLUMN));
        tile_row(io, tiles, dst);
        // `subi.w #2` on the row: a word, so row 0 is `$fffe`.
        let shapes = (row as u16).wrapping_sub(2) as u32 + col * BYTE_COLUMN;
        shape_row(io, SHAPE_MAP.wrapping_add(shapes), at);

        let y8 = y.wrapping_sub(8);
        if minus(y8, 0) {
            return;
        }
        let row = (y8 >> 4) as u32;
        let mut d0: u32 = 0xffff;
        for (i, d4) in (0..=0x13u32).rev().enumerate() {
            let cell = OBJECT_MAP.wrapping_add((col + i as u32) * BYTE_COLUMN + row);
            let b = io.rb(cell);
            match code(b) {
                Code::End => return,
                Code::Skip => {}
                Code::Bob => d0 = bob_above(io, b, (x, y), d4, d0),
                Code::Animated => {
                    // `andi.w`: the high word stays.
                    let sum = ((d0 & 0xffff_0000) | x as u32).wrapping_add(step(d4, 0x14));
                    d0 = (sum & 0xffff_0000) | (sum & 0xfff0);
                    self.add_cell(io, cell, d0 as u16, y8 & 0xfff0, b);
                }
                Code::Creature => {
                    let cx = ((d0 & 0xffff_0000) | x as u32).wrapping_add(step(d4, 0x14));
                    let (kind, cy) = creature(b, (y.wrapping_sub(1) & 0xfff0) as u32);
                    let facing = if minus(d4 as u16, 0xa) { 0xff } else { 1 };
                    d0 = self.start(io, cell, cx, cy as u16, kind, facing);
                }
            }
        }
    }

    /// `redraw_edge_c` (`h0+$df6`): the column left of the view, `x - $10`, while x is at least 1; its
    /// scan in the column at x, while x is under `$8000`. Ported by hand (181 instructions of its own
    /// flow; docs/11-translation.md).
    pub fn redraw_left(&mut self, io: &mut impl Io) {
        let (x, y) = self.pos;
        if minus(x, 1) {
            return;
        }
        let (col, row) = ((x >> 4) as u32, (y >> 4) as u32);
        let blk = block(y);
        self.p840 = Ptr::Raw(blk);
        let tiles = ((row << 1).wrapping_sub(TILE_COLUMN)).wrapping_add(col * TILE_COLUMN);
        tile_column(
            io,
            TILE_MAP.wrapping_add(tiles),
            blk,
            (col * 2).wrapping_sub(2),
        );
        let shapes = row
            .wrapping_sub(BYTE_COLUMN)
            .wrapping_add(col * BYTE_COLUMN);
        shape_column(
            io,
            SHAPE_MAP.wrapping_add(shapes),
            blk,
            (col * 2).wrapping_sub(2),
        );

        if minus(x, 0) {
            return;
        }
        for (i, d4) in (0..=0xcu32).rev().enumerate() {
            let cell = OBJECT_MAP.wrapping_add(col * BYTE_COLUMN + row + i as u32);
            let b = io.rb(cell);
            match code(b) {
                Code::End => return,
                Code::Skip => {}
                Code::Bob => bob_left(io, b, (x, y), d4),
                Code::Animated => {
                    let cy = (y as u32).wrapping_add(step(d4, 0xd)) as u16 & 0xfff0;
                    self.add_cell(io, cell, x & 0xfff0, cy, b);
                }
                Code::Creature => {
                    let cy = ((y & 0xfff0) as u32).wrapping_add(step(d4, 0xd));
                    let (kind, cy) = creature(b, cy);
                    self.start(io, cell, x.wrapping_sub(7) as u32, cy as u16, kind, 1);
                }
            }
        }
    }

    /// `redraw_edge_d` (`h0+$114a`): the column right of the view, `x + $150`; its scan at x + `$130`.
    pub fn redraw_right(&mut self, io: &mut impl Io) {
        let (x, y) = self.pos;
        let (col, row) = ((x >> 4) as u32, (y >> 4) as u32);
        let blk = block(y);
        self.p840 = Ptr::Raw(blk);
        let tiles = (row << 1)
            .wrapping_add(0xfc0)
            .wrapping_add(col * TILE_COLUMN);
        tile_column(
            io,
            TILE_MAP.wrapping_add(tiles),
            blk,
            (col * 2).wrapping_add(0x2a),
        );
        let shapes = row.wrapping_add(0x7e0).wrapping_add(col * BYTE_COLUMN);
        shape_column(
            io,
            SHAPE_MAP.wrapping_add(shapes),
            blk,
            (col * 2).wrapping_add(0x2a),
        );

        let xr = x.wrapping_add(0x130);
        let col = (xr >> 4) as u32;
        for (i, d4) in (0..=0xcu32).rev().enumerate() {
            let cell = OBJECT_MAP.wrapping_add(col * BYTE_COLUMN + row + i as u32);
            let b = io.rb(cell);
            match code(b) {
                Code::End => return,
                Code::Skip => {}
                Code::Bob => bob_right(io, b, (x, y), d4),
                Code::Animated => {
                    let cy = (y as u32).wrapping_add(step(d4, 0xd)) as u16 & 0xfff0;
                    self.add_cell(io, cell, xr & 0xfff0, cy, b);
                }
                Code::Creature => {
                    let cy = ((y & 0xfff0) as u32).wrapping_add(step(d4, 0xd));
                    let (kind, cy) = creature(b, cy);
                    self.start(io, cell, xr.wrapping_sub(7) as u32, cy as u16, kind, 0xff);
                }
            }
        }
    }
}

/// Where the level buffers end: world 6's shape map, the largest, from [`SHAPE_MAP`].
pub const IMAGE_END: u32 = SHAPE_MAP + 0x7800;

/// The level buffers of hunk 0 as the original lays them out, `h0+$bda0` to [`IMAGE_END`]: what a
/// redraw reads of the level in play. What lies between the buffers (program data) reads 0, which only
/// positions past the camera's clamps reach. The object map is play state, kept apart ([`PlayIo`]).
///
/// Past [`IMAGE_END`] a redraw reads the program itself: with the camera in the map's top two rows
/// `redraw_edge_b`'s `subi.w #2` takes the shape row to `$ffff` or `$fffe`, 64 KB past the shape
/// map, in hunk 0's tables (`h0+$38eb7` on), and copies the shapes those bytes name into the band
/// the row below the view probes (T48, measured on the route at pass 13 942: a creature turned at
/// the ledge they left).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelImage {
    bytes: Vec<u8>,
    program: crate::program::Program,
}

impl LevelImage {
    pub fn new(level: &fd_assets::Level, program: &crate::program::Program) -> LevelImage {
        let mut bytes = vec![0; (IMAGE_END - SHAPES) as usize];
        let mut put = |a: u32, v: &[u8]| {
            let at = (a - SHAPES) as usize;
            let n = v.len().min(bytes.len() - at);
            bytes[at..at + n].copy_from_slice(&v[..n]);
        };
        // The shapes buffer holds 108 and a bit before the object map; the loader's later files win.
        let shapes: Vec<u8> = level
            .shapes
            .iter()
            .flat_map(|s| s.iter().flat_map(|w| w.to_be_bytes()))
            .take((OBJECT_MAP - SHAPES) as usize)
            .collect();
        put(SHAPES, &shapes);
        put(OBJECT_MAP, &level.objects.cells);
        let tiles: Vec<u8> = level
            .tiles
            .cells
            .iter()
            .flat_map(|t| t.to_be_bytes())
            .collect();
        put(TILE_MAP, &tiles);
        put(SHAPE_MAP, &level.collision.cells);
        LevelImage {
            bytes,
            program: program.clone(),
        }
    }

    /// The byte at hunk 0 offset `a`: the level's from [`SHAPES`] to [`IMAGE_END`], the program's
    /// outside them.
    pub fn byte(&self, a: u32) -> u8 {
        let i = a.wrapping_sub(SHAPES) as usize;
        match self.bytes.get(i) {
            Some(b) => *b,
            None => self.program.byte(a),
        }
    }
}

/// The redraws' [`Io`] in play: the level read from its image and the object map, the cleared cells
/// written to the object map, `collision_map` kept, and the blits listed for the picture.
pub struct PlayIo<'a> {
    pub image: &'a LevelImage,
    /// The object map from `h0+$cb38`.
    pub objects: &'a mut [u8],
    pub collision: &'a mut super::collision::CollisionPlane,
    pub blits: &'a mut Vec<Blit>,
    /// The object map cell (hunk 0 offset) from which the scan's animated cells are added and its
    /// creatures started after the next pass, which cut the scan there (T74, `level::EdgesCut`); `None` adds each at once.
    /// The scans walk the map upward, rows `$60` bytes a cell and columns 1, so a later cell is a
    /// higher one.
    pub defer_from: Option<u32>,
    /// The redraw's `collision_map` words from this one on are kept in `late` for after the next
    /// pass, which came before `h0_7cc` copied them (T48, `level::EdgesCut::shape_words`); `None`
    /// writes each at once.
    pub late_from: Option<u16>,
    /// The words the redraw running has written or kept so far.
    pub written: u16,
    /// (its place in the redraw's order, bytes from the map's first, the word) of each word kept.
    pub late: Vec<(u16, u32, u16)>,
}

impl PlayIo<'_> {
    fn object(&self, a: u32) -> Option<usize> {
        let i = a.wrapping_sub(OBJECT_MAP) as usize;
        (i < self.objects.len()).then_some(i)
    }
}

impl Io for PlayIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        match self.object(a) {
            Some(i) => self.objects[i],
            None => self.image.byte(a),
        }
    }
    fn clear_object(&mut self, a: u32) {
        if let Some(i) = self.object(a) {
            self.objects[i] = 0;
        }
    }
    fn blit(&mut self, b: Blit) {
        self.blits.push(b);
    }
    fn collision_word(&mut self, at: u32, v: u16) {
        let n = self.written;
        self.written = n.saturating_add(1);
        if self.late_from.is_some_and(|f| n >= f) {
            self.late.push((n, at, v));
        } else {
            self.collision.set_word(at, v);
        }
    }
    fn defers(&self, cell: u32) -> bool {
        self.defer_from.is_some_and(|f| cell >= f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T48: the up row's shape row is `$ffff` with the camera in the map's second row, far past
    /// the shape map: the byte there is the program's, not 0.
    #[test]
    fn past_the_level_buffers_a_redraw_reads_the_program() {
        let mut program = vec![0u8; (IMAGE_END + 0x1_0000) as usize];
        program[(SHAPE_MAP + 0xffff) as usize] = 0x32;
        program[(SHAPES + 1) as usize] = 0x77;
        let image = LevelImage {
            bytes: vec![1, 2, 3],
            program: crate::program::Program::hunk0(program),
        };
        // Inside the buffers the level's bytes win over whatever the program's file holds there.
        assert_eq!(image.byte(SHAPES + 1), 2);
        assert_eq!(image.byte(SHAPE_MAP + 0xffff), 0x32);
        assert_eq!(image.byte(SHAPE_MAP + 0xfffe), 0);
    }

    /// T48: a redraw the next pass cut after `late_from` collision words writes those and keeps
    /// the rest, in order, for after the pass.
    #[test]
    fn the_words_a_cut_redraw_had_not_copied_are_kept_for_after_the_pass() {
        let program = crate::program::Program::hunk0(vec![0; 0x3_8000]);
        let image = LevelImage {
            bytes: Vec::new(),
            program: program.clone(),
        };
        let mut plane = super::super::collision::CollisionPlane::new(&program);
        let (mut objects, mut blits) = (Vec::new(), Vec::new());
        let mut io = PlayIo {
            image: &image,
            objects: &mut objects,
            collision: &mut plane,
            blits: &mut blits,
            defer_from: None,
            late_from: Some(2),
            written: 0,
            late: Vec::new(),
        };
        for (k, w) in [0x1111u16, 0x2222, 0x3333, 0x4444].into_iter().enumerate() {
            io.collision_word(2 * k as u32, w);
        }
        assert_eq!(io.late, [(2, 4, 0x3333), (3, 6, 0x4444)]);
        assert_eq!(io.written, 4);
        let got: Vec<u8> = (0..8).map(|o| plane.byte(o)).collect();
        assert_eq!(got, [0x11, 0x11, 0x22, 0x22, 0, 0, 0, 0]);
    }

    #[test]
    fn a_tile_row_lands_in_the_band_of_its_row() {
        // Entry j of block b holds band (j - 1 - b) & 15; edge a's entry 13 of block 15 - (r & 15)
        // is band (r + 13) & 15: the row it draws, y / 16 + 13.
        for r in 0..16u32 {
            let k = (15 - r) * 16 + 13;
            assert_eq!(band(k), (r + 13) & 15);
        }
    }

    #[test]
    fn codes_split_as_the_signed_compares_do() {
        assert_eq!(code(0), Code::Skip);
        assert_eq!(code(0xff), Code::End);
        for b in [1, 6, 0x87, 0xfe] {
            assert_eq!(code(b), Code::Creature, "{b:#x}");
        }
        for b in [7, 0x1f] {
            assert_eq!(code(b), Code::Skip, "{b:#x}");
        }
        assert_eq!((code(0x20), code(0x3f)), (Code::Animated, Code::Animated));
        assert_eq!((code(0x40), code(0x86)), (Code::Bob, Code::Bob));
    }

    #[test]
    fn the_emptiest_list_takes_the_cell_the_first_on_a_tie() {
        let mut c = AnimatedCells::default();
        c.used[0] = [0xff; 12];
        c.used[1][0] = 1;
        c.used[3][5] = 1;
        c.add(0x10, 0x20, 0x21);
        assert_eq!(c.used[2][0], 0xff);
        assert_eq!(
            (c.cells[2][0].x, c.cells[2][0].y, c.cells[2][0].code),
            (0x10, 0x20, 0x21)
        );
        let mut full = AnimatedCells {
            used: [[0xff; 12]; 4],
            ..AnimatedCells::default()
        };
        let before = full.clone();
        full.add(1, 2, 0x20);
        assert_eq!(full, before);
    }

    #[test]
    fn a_creature_takes_the_first_free_slot_or_none() {
        let mut s = CreatureSlots {
            active: [0xff, 0, 0, 0xff, 0],
            ..CreatureSlots::default()
        };
        assert!(!s.start(1, 2, 3, 0xff));
        assert_eq!((s.current, s.last, s.active[1]), (1, 1, 0xff));
        assert_eq!(
            s.slots[1],
            CreatureSlot {
                x: 1,
                y: 2,
                kind: 3,
                facing: 0xff,
                phase: 0
            }
        );
        let mut full = CreatureSlots {
            active: [1; 5],
            ..CreatureSlots::default()
        };
        assert!(full.start(1, 2, 3, 1));
        assert_eq!(full.current, 0);
    }
}
