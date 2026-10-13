//! The renderer (DESIGN §2.1 picture, §2.7, §3.3 `render`; docs/12).
//!
//! It draws the frame the original displays from the engine's [`State`] and what the disks
//! decide ([`DisplayData`]), as Denise would show it: it runs the level's copper list for the
//! frame ([`copper`]), follows the registers it writes, fetches each line's bitplanes from the
//! panel or the shown display buffer ([`panel`], [`playfield`]), and puts the hardware sprites
//! ([`objects`]) on top in the original's priority order. Nothing is blitted here: the buffers
//! are state, as the original's blits left them. Widescreen ([`View`]) draws more of the same
//! level from the map beside the original's window.

pub mod copper;
mod objects;
mod panel;
mod playfield;
mod view;

pub use view::{Picture, Split, View, to_rgb12};

use sf_assets::{ExeId, GameData, SpriteId, World};

use crate::layout::{H2, h2, h3};
use crate::state::State;
use copper::{Fetch, Write, reg};

/// The level's copper list (`level_copper`).
pub const LEVEL_COPPER: u32 = h2(0x6bd0);
/// The frog's hardware-sprite frames, converted from `HERO` by `load_hero` (h0+$19ce0).
pub const HERO_SPRITES: u32 = h3(0x4c0b0);

/// The parts of the original's memory the display reads that the disks decide, laid out where
/// the original keeps them, built once from [`GameData`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayData {
    /// Hunk 2's contents (`d1_c02`'s data block): the HUD's sprite digits, the panel's parts.
    h2: Vec<u8>,
    /// The frog's sprite frames at [`HERO_SPRITES`].
    hero: Vec<u8>,
    /// Each world's tiles (16 × 16 colour indices), by [`World::ALL`] order: what widescreen's
    /// extra columns are drawn from.
    worlds: Vec<WorldDisplay>,
    /// What the game's memory holds that never changes once set up ([`crate::mem::Rom`]): the
    /// tables the mechanics read.
    pub(crate) rom: crate::mem::Rom,
    /// What the sound driver reads from the disks (T11's), from the decoded data; a test's empty
    /// `GameData` gets an empty one.
    pub(crate) sound: crate::audio::SoundData,
    /// What the front end reads from the disks.
    pub front: crate::front::FrontData,
    /// Each level's `T7MP` file as `load_file` leaves it in memory (decompressed), for
    /// [`crate::level::load`].
    pub(crate) levels: std::collections::BTreeMap<sf_assets::LevelId, Vec<u8>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WorldDisplay {
    tiles: Vec<[u8; 256]>,
    /// `LnBO` as the world's load leaves it in hunk 3: five planes of 320 × 320.
    bobs: Vec<u8>,
}

impl Default for DisplayData {
    fn default() -> DisplayData {
        DisplayData::new(&GameData::default())
    }
}

impl DisplayData {
    /// Everything the display reads from the disks. Content a test's empty `GameData` lacks is
    /// left empty and reads as zero.
    pub fn new(data: &GameData) -> DisplayData {
        let game = data.exes.iter().find(|e| e.id == ExeId::Game);
        let h2 = data
            .raw
            .get("accessing-picture")
            .map(|b| match b.get(..4) {
                // The block keeps its HUNK_DATA header: the id and the length in longs.
                Some([0, 0, 3, 0xea]) => b[8..].to_vec(),
                _ => b.clone(),
            })
            .unwrap_or_default();
        let hero = data
            .sprites
            .iter()
            .find(|s| s.id == SpriteId::Hero)
            .zip(game)
            .map(|(s, g)| hero_sprites(&s.sheet.to_planar(), g))
            .unwrap_or_default();
        let worlds = data
            .worlds
            .iter()
            .map(|w| WorldDisplay {
                tiles: w.tiles.tiles.clone(),
                bobs: w.bobs.sheet.to_planar(),
            })
            .collect();
        DisplayData {
            h2,
            hero,
            worlds,
            front: crate::front::FrontData::new(data),
            levels: data
                .levels
                .iter()
                .map(|(id, l)| (*id, sf_assets::t7mp_encode(&l.t7mp)))
                .collect(),
            rom: crate::mem::Rom::new(data),
            sound: crate::audio::SoundData::from_game(data)
                .unwrap_or_else(|_| crate::audio::SoundData::from_parts(vec![0; 0x50000], vec![], vec![], vec![])),
        }
    }

    /// What the sound driver reads from the disks.
    pub fn sound(&self) -> &crate::audio::SoundData {
        &self.sound
    }

    /// The frog's sprite frames as `load_hero` leaves them at [`HERO_SPRITES`].
    pub fn hero(&self) -> &[u8] {
        &self.hero
    }

    /// What the ported routines read the game's fixed tables from.
    pub fn rom(&self) -> &crate::mem::Rom {
        &self.rom
    }

    /// `len` bytes of hunk 2 from `off` as the bootstrap leaves it (0 past what the disks give).
    pub fn h2_bytes(&self, off: u32, len: u32) -> Vec<u8> {
        (off..off + len)
            .map(|a| self.h2.get(a as usize).copied().unwrap_or(0))
            .collect()
    }

    /// The bob sheet (`LnBO`) of `world` (W1..W6, the bonus and the final level) as its load
    /// leaves it in hunk 3; empty when the disks are not there.
    pub(crate) fn bob_sheet(&self, world: World) -> &[u8] {
        World::ALL
            .iter()
            .position(|&x| x == world)
            .and_then(|i| self.worlds.get(i))
            .map_or(&[], |w| &w.bobs[..])
    }

    /// The world the `world` variable names: 1..6, 7 the bonus level.
    fn world(&self, world: u32) -> Option<&WorldDisplay> {
        let w = match world {
            1..=6 => World::ALL[world as usize - 1],
            7 => World::Bonus,
            _ => return None,
        };
        let i = World::ALL.iter().position(|&x| x == w)?;
        self.worlds.get(i)
    }
}

/// The frog's sprite frames, as `load_hero` builds them (h0+$19ce0): each rectangle of the
/// `hero_rects` table (h0+$1a6f4: x, y, w, h, two offsets) cut from `HERO` and from `HERO`
/// mirrored in 32-pixel steps (h0+$19cb2), as two 16-pixel columns of an attached sprite pair:
/// planes 0 and 1 a row at a time, two zero words, then planes 2 and 3 likewise. The first set of
/// frames takes as many rectangles as the pointer table at h0+$19ddc has entries before its
/// negative end mark, the mirrored set the next ones (the routine does not rewind the table).
fn hero_sprites(hero: &[u8], game: &sf_assets::ExeImage) -> Vec<u8> {
    const PLANE: usize = 0x3c00;
    const ROW: usize = 0x28;
    let long = |at: u32| {
        game.bytes(0, at, 4, "hero sprite tables")
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0xFFFF_FFFF)
    };
    let word = |at: u32| (long(at) >> 16) as usize;
    let count = |table: u32| {
        (0..)
            .take_while(|&k| long(table + 0x18 * k) & 0x8000_0000 == 0)
            .count() as u32
    };
    if hero.len() < 4 * PLANE {
        return Vec::new();
    }
    let mirrored: Vec<u8> = hero
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|l| {
            u32::from_be_bytes([l[0], l[1], l[2], l[3]])
                .reverse_bits()
                .to_be_bytes()
        })
        .collect();
    let mut out = Vec::new();
    let mut rect = 0x1a6f4;
    for (src, table) in [(hero, 0x19ddc), (&mirrored[..], 0x19de8)] {
        for _ in 0..count(table) {
            let (x, y, h) = (word(rect), word(rect + 2), word(rect + 6));
            let base = y * ROW + (x >> 4) * 2;
            for column in [0, 2] {
                for planes in [0, 2 * PLANE] {
                    for r in 0..h {
                        let a = base + column + planes + r * ROW;
                        out.extend(src.get(a..a + 2).unwrap_or(&[0, 0]));
                        out.extend(src.get(a + PLANE..a + PLANE + 2).unwrap_or(&[0, 0]));
                    }
                    out.extend([0; 4]);
                }
            }
            rect += 12;
        }
    }
    out
}

/// What the hardware reads: the state's variables kept in the original's form over the disks'
/// content, at the original's addresses; anything else reads as zero.
pub(crate) struct Mem<'a> {
    state: &'a State,
    data: &'a DisplayData,
    world: Option<&'a WorldDisplay>,
    /// Writes made during the frame ([`crate::mem::WriteLog`]), by address: `(address, the byte
    /// before, the beam's line)`, each address's in the order made. A line drawn before a write
    /// sees the byte before it.
    undo: Vec<(u32, u8, u16)>,
    /// The line the beam is drawing.
    line: std::cell::Cell<u16>,
}

impl<'a> Mem<'a> {
    fn new(state: &'a State, data: &'a DisplayData) -> Mem<'a> {
        Mem {
            state,
            data,
            world: data.world(state.world()),
            undo: Vec::new(),
            line: std::cell::Cell::new(0),
        }
    }

    /// `state` as the beam saw it, with the frame's writes in `log` made as it went.
    fn with_log(state: &'a State, data: &'a DisplayData, log: &crate::mem::WriteLog) -> Mem<'a> {
        let mut undo = log.writes.clone();
        // A stable sort keeps each address's writes in the order made.
        undo.sort_by_key(|w| w.0);
        Mem {
            undo,
            ..Mem::new(state, data)
        }
    }

    /// `len` bytes at `addr` as the beam sees them on its line, from the one place that holds
    /// them all, else byte by byte.
    pub fn bytes(&self, addr: u32, len: u32) -> std::borrow::Cow<'a, [u8]> {
        let b: std::borrow::Cow<'a, [u8]> = match self.whole(addr, len) {
            Some(b) => b.into(),
            None => (addr..addr + len)
                .map(|a| self.whole(a, 1).map_or(0, |b| b[0]))
                .collect::<Vec<u8>>()
                .into(),
        };
        if self.undo.is_empty() {
            return b;
        }
        let line = self.line.get();
        let from = self.undo.partition_point(|w| w.0 < addr);
        let mut out: Option<Vec<u8>> = None;
        let mut done = None;
        for &(a, old, at) in &self.undo[from..] {
            if a >= addr + len {
                break;
            }
            // The first write the beam has not seen holds what it sees.
            if at >= line && done != Some(a) {
                out.get_or_insert_with(|| b.to_vec())[(a - addr) as usize] = old;
                done = Some(a);
            }
        }
        match out {
            Some(v) => v.into(),
            None => b,
        }
    }

    /// The word at `addr` as the beam sees it on `line`.
    fn word_at(&self, addr: u32, line: u16) -> u16 {
        self.line.set(line);
        self.word(addr)
    }

    fn whole(&self, addr: u32, len: u32) -> Option<&'a [u8]> {
        if let Some(b) = self.state.raw(addr, len) {
            return Some(b);
        }
        let regions: [(u32, &'a [u8]); 2] = [(H2, &self.data.h2), (HERO_SPRITES, &self.data.hero)];
        regions.into_iter().find_map(|(base, b)| {
            let at = addr.checked_sub(base)? as usize;
            b.get(at..at + len as usize)
        })
    }

    pub fn word(&self, addr: u32) -> u16 {
        let b = self.bytes(addr, 2);
        u16::from_be_bytes([b[0], b[1]])
    }
}

/// Lores pixels from a line's first fetched bit to its display (Denise's pipeline): the fetch
/// index of window pixel `x` is `left + x - (2 * DDFSTRT + 17) - delay` (docs/12, measured).
const PIPELINE: i32 = 17;
/// The DMA cycle of sprite 0's slot; sprite `n`'s is four cycles later per sprite.
const SPRITE_SLOT: u16 = 0x15;

/// Bitplane registers as the copper leaves them.
#[derive(Debug, Clone, Copy, Default)]
struct Planes {
    fetch: Fetch,
    bplcon1: u16,
    bplcon2: u16,
    modulo: [i16; 2],
    pt: [u32; 6],
}

impl Planes {
    fn write(&mut self, reg: u16, value: u16) {
        self.fetch.write(reg, value);
        match reg {
            reg::BPLCON1 => self.bplcon1 = value,
            reg::BPLCON2 => self.bplcon2 = value,
            reg::BPL1MOD => self.modulo[0] = value as i16,
            reg::BPL2MOD => self.modulo[1] = value as i16,
            r if (reg::BPL1PTH..reg::BPL1PTH + 24).contains(&r) => {
                let p = ((r - reg::BPL1PTH) / 4) as usize;
                self.pt[p] = if r & 2 == 0 {
                    (value as u32 & 0x1F) << 16 | self.pt[p] & 0xFFFF
                } else {
                    self.pt[p] & 0xFFFF_0000 | value as u32 & 0xFFFE
                };
            }
            _ => {}
        }
    }

    /// Plane `p`'s scroll delay in lores pixels: odd planes (index 0, 2, 4) use PF1's.
    fn delay(&self, p: usize) -> i32 {
        (if p.is_multiple_of(2) {
            self.bplcon1
        } else {
            self.bplcon1 >> 4
        } & 15) as i32
    }

    /// The fetch index (lores pixel of the line's fetched data) plane `p` shows at window
    /// position `left + x`.
    fn index(&self, p: usize, left: i32) -> i32 {
        left - (2 * self.fetch.first_cycle() as i32 + PIPELINE) - self.delay(p)
    }

    /// The play field's priority against the sprite pairs: in single-playfield mode Denise
    /// uses PF2P (BPLCON2 bits 3-5); pairs below it are in front.
    fn priority(&self) -> u16 {
        self.bplcon2 >> 3 & 7
    }
}

/// What a picture line shows under the sprites.
#[derive(Debug, Clone, Copy)]
enum Under {
    /// No bitplane fetch: colour 0.
    Nothing,
    /// Fetched from memory; its indices are already in the picture.
    Fetched,
    /// Fetched from a display buffer (indices already in the picture), showing a level row with
    /// level x `x0` at picture pixel 0: widescreen's columns continue it from the map.
    Field { y: i32, x0: i32 },
}

/// One frame of Denise: walks the lines following the copper's writes, then fills the picture.
struct Denise<'a> {
    mem: &'a Mem<'a>,
    writes: &'a [Write],
    next: usize,
    planes: Planes,
    colours: [u16; 32],
    sprites: [objects::Sprite; 8],
    /// A level's screen: lines fetched from a display buffer are play-field rows (widescreen).
    level: bool,
}

/// The level's picture.
pub(crate) fn level(state: &State, data: &DisplayData, view: View) -> Picture {
    level_as_drawn(state, data, view, &crate::mem::WriteLog::default())
}

/// The level's picture as the beam drew it: `state` at the frame's end, less what `log` says
/// was written after the beam had passed.
pub(crate) fn level_as_drawn(
    state: &State,
    data: &DisplayData,
    view: View,
    log: &crate::mem::WriteLog,
) -> Picture {
    let mem = Mem::with_log(state, data, log);
    let word = |a: u32, line: u16| mem.word_at(a, line);
    draw(
        &mem,
        &word,
        LEVEL_COPPER,
        view.extra_columns_each_side,
        true,
        None,
    )
    .0
}

/// A screen outside a level (the front end): the copper list at `cop1lc` run for the frame,
/// `copper_word(addr, line)` giving each word of it as the copper reads it on that line (a list the
/// CPU rewrote during the frame), every other read from `state`. Also returns the copper's
/// register writes, for the ones Paula's conformance sees (DMACON, INTENA).
pub(crate) fn screen(
    state: &State,
    data: &DisplayData,
    cop1lc: u32,
    copper_word: &dyn Fn(u32, u16) -> u16,
    carry: Option<Fetch>,
) -> (Picture, Vec<Write>) {
    let mem = Mem::new(state, data);
    draw(&mem, copper_word, cop1lc, 0, false, carry)
}

/// The copper's writes on a frame of the list at `cop1lc`, as `state` holds it.
pub(crate) fn copper_writes(state: &State, data: &DisplayData, cop1lc: u32) -> Vec<Write> {
    let mem = Mem::new(state, data);
    let word = |a: u32, _: u16| mem.word(a);
    let mut start = Fetch::default();
    for w in copper::run_timed(&word, cop1lc, Fetch::default()) {
        start.write(w.reg, w.value);
    }
    copper::run_timed(&word, cop1lc, start)
}

fn draw(
    mem: &Mem,
    word: &dyn Fn(u32, u16) -> u16,
    cop1lc: u32,
    n: u16,
    level: bool,
    carry: Option<Fetch>,
) -> (Picture, Vec<Write>) {
    // The frame starts with the registers as the last frame left them (`carry`); without them,
    // with the setup the list leaves at its end, as if the last frame ran it too.
    let start = carry.unwrap_or_else(|| {
        let mut f = Fetch::default();
        for w in copper::run_timed(word, cop1lc, Fetch::default()) {
            f.write(w.reg, w.value);
        }
        f
    });
    let writes = copper::run_timed(word, cop1lc, start);
    let mut end = start;
    for w in &writes {
        end.write(w.reg, w.value);
    }
    let (hstart, hstop) = (end.diwstrt & 0xFF, end.diwstop & 0xFF | 0x100);
    let lines = end.lines();
    let mut pic = Picture::blank(
        hstart as i32 - n as i32,
        lines.start,
        hstop.saturating_sub(hstart) + 2 * n,
        lines.end.saturating_sub(lines.start),
    );
    let mut denise = Denise {
        mem,
        writes: &writes,
        next: 0,
        planes: Planes {
            fetch: start,
            ..Planes::default()
        },
        colours: [0; 32],
        sprites: Default::default(),
        level,
    };
    let rows = denise.walk(&mut pic);
    let field = if level {
        playfield::Field::draw(mem.state, mem, &rows, pic.width, n)
    } else {
        playfield::Field::default()
    };
    objects::compose(&mut pic, &rows, &field, n as usize);
    (pic, writes)
}

/// What the walk learns about one picture line.
pub(crate) struct Row {
    under: Under,
    priority: u16,
    sprites: [Option<objects::SpriteLine>; 8],
}

impl Denise<'_> {
    /// Run the frame's lines: palettes and splits into `pic`, fetched lines' indices too, and
    /// what each line shows under the sprites.
    fn walk(&mut self, pic: &mut Picture) -> Vec<Row> {
        let width = pic.width as usize;
        let left = pic.left;
        let mut rows = Vec::with_capacity(pic.height as usize);
        let mut field = playfield::Lines::default();
        for line in 0..pic.top + pic.height {
            self.mem.line.set(line);
            let y = line.checked_sub(pic.top).map(|y| y as usize);
            let mut cuts = [None; 8];
            let mut palette = None;
            self.apply(line, SPRITE_SLOT, y, pic, &mut palette, &mut cuts);
            let fetching =
                self.planes.fetch.lines().contains(&line) && self.planes.fetch.planes() > 0;
            for (n, s) in self.sprites.iter_mut().enumerate() {
                // OCS: a fetch window opening at or before a sprite's slot takes it.
                let slot = SPRITE_SLOT + 4 * n as u16;
                s.dma(
                    line,
                    self.mem,
                    fetching && self.planes.fetch.first_cycle() <= slot + 1,
                );
            }
            let first = self.planes.fetch.first_cycle();
            self.apply(line, first, y, pic, &mut palette, &mut cuts);
            let mut under = Under::Nothing;
            if self.planes.fetch.lines().contains(&line) && self.planes.fetch.planes() > 0 {
                let x0 = self.planes.index(0, left);
                let words = self.planes.fetch.words() as i32;
                let stride = 2 * words + self.planes.modulo[0] as i32;
                let line = if self.level {
                    field.line(self.mem.state, self.planes.pt[0], stride)
                } else {
                    None
                };
                under = match line {
                    Some((y, x)) => Under::Field { y, x0: x + x0 },
                    None => Under::Fetched,
                };
                if let Some(y) = y {
                    let row = &mut pic.pixels[y * width..(y + 1) * width];
                    panel::fetch(self.mem, &self.planes, left, row);
                }
                let words = self.planes.fetch.words() as i32;
                for p in 0..self.planes.fetch.planes() as usize {
                    let m = self.planes.modulo[p % 2] as i32;
                    self.planes.pt[p] = (self.planes.pt[p] as i32 + 2 * words + m) as u32;
                }
            }
            // OCS Denise shows sprites only from a line's first bitplane fetch on (vAmiga
            // `spriteClipBegin`): a line without one shows none.
            let sprites: [Option<objects::SpriteLine>; 8] = std::array::from_fn(|n| {
                self.sprites[n]
                    .shown()
                    .filter(|_| !matches!(under, Under::Nothing))
            });
            let priority = self.planes.priority();
            self.apply(line + 1, 0, y, pic, &mut palette, &mut cuts);
            for s in &mut self.sprites {
                s.end_of_line(line + 1, copper::FRAME_LINES - 1);
            }
            if let Some(y) = y {
                pic.palettes[y] = palette.unwrap_or(self.colours);
                let sprites = std::array::from_fn(|n| {
                    sprites[n].map(|s| objects::SpriteLine { cut: cuts[n], ..s })
                });
                rows.push(Row {
                    under,
                    priority,
                    sprites,
                });
            }
        }
        rows
    }

    /// Apply the writes before beam position (`line`, `h`). A colour write inside picture line
    /// `y`'s pixels becomes a split (the line's palette is taken just before the first); a write
    /// to a sprite's control after its DMA slot turns it off from that pixel on (`cuts`).
    fn apply(
        &mut self,
        line: u16,
        h: u16,
        y: Option<usize>,
        pic: &mut Picture,
        palette: &mut Option<[u16; 32]>,
        cuts: &mut [Option<i32>; 8],
    ) {
        while let Some(w) = self.writes.get(self.next).copied() {
            if (w.line, w.h) >= (line, h) {
                break;
            }
            self.next += 1;
            let x = 2 * w.h as i32 - pic.left;
            let on_line = y.filter(|&y| w.line == pic.top + y as u16);
            match w.reg {
                r if (reg::COLOR00..reg::COLOR00 + 64).contains(&r) => {
                    let index = ((r - reg::COLOR00) / 2) as u8;
                    if let Some(y) = on_line.filter(|_| x > 0) {
                        palette.get_or_insert(self.colours);
                        if x < pic.width as i32 {
                            pic.splits.push(Split {
                                y: y as u16,
                                x: x as u16,
                                index,
                                colour: w.value & 0xFFF,
                            });
                        }
                    }
                    self.colours[index as usize] = w.value & 0xFFF;
                }
                r if (reg::SPR0PTH..reg::SPR0POS + 64).contains(&r) => {
                    let n = objects::sprite_of(r);
                    let disarmed = self.sprites[n].write(r, w.value, w.line);
                    if disarmed && on_line.is_some() && w.h >= SPRITE_SLOT + 4 * n as u16 {
                        cuts[n] = Some(x);
                    }
                }
                r => self.planes.write(r, w.value),
            }
        }
    }
}
