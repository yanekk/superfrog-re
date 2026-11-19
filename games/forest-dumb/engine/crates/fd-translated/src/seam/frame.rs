//! Level 1's frame mount (translator DESIGN §2.9, T36): `game_frame` (`h0+$1fec`) run by the hand
//! engine, `fd_core::Game::game_frame_on`, in place of the translated routine, while the program
//! plays world 1.
//!
//! Each call transplants a [`Game`] from memory ([`Transplant`]: every `LAYOUT` variable, the
//! creatures, cells and display state, what the callees keep outside `LAYOUT`), runs the pass with a
//! [`Pass`] over the bus, and stores what the engine changed back. The pass's outside world goes to
//! the bus as the original's does: the object ports and the creature drawing read memory in place
//! and turn each blit and sound into the original's custom-chip writes, in order, which the
//! machine's blitter and Paula take; the object map and the collision plane they write are read
//! back into the typed state for the callees after them. This is the routine rig's adapter
//! (fd-conform's `tests/game_frame_routine.rs`, T63) on a bus, where it was held to the original on
//! 10 000 states; `fd-judge --seam 'h0+$1fec'` holds this one to the translated routine.
//!
//! The engine's start (level 1's data, the first view) is built once by the host ([`Level1`]) and
//! handed to the routine through `Bus::mount_data`; a run that mounts the frame without it stops.
//! In another world the translated routine runs: the hand engine is level 1's.
//!
//! What the mount does not do: count the original's cycles (`Regs` is not ticked, so the pass
//! takes no machine time), or leave the original's scratch registers (`level3_handler` restores
//! them all around the call).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use amiga_input::FrameInput;
use fd_core::level::collision::MAP;
use fd_core::level::redraw::{Blit, OBJECT_MAP};
use fd_core::level::{LevelCtx, Pass};
use fd_core::objects::{Io, MapObjects, Sound};
use fd_core::render::COLOURS;
use fd_core::render::objects::{CharSprite, CreatureBlit, Creatures, DrawIo};
use fd_core::render::palette::Fade;
use fd_core::render::playfield::{SCREEN as SCREEN_AT, Scroll};
use fd_core::render::status::STATUS;
use fd_core::{Game, GameData, LAYOUT, Mem, MemMut, Scratch, Start, State, Transplant};
use pcode_rt::{Bus, Image, Regs, Stop, rts};

use super::BusMem;

/// `game_frame`'s entry.
pub const GAME_FRAME: (u8, u32) = (0, 0x1fec);

/// `world_index` (`h0+$6dfe`, a long): 0 in world 1, where the hand engine plays.
const WORLD_INDEX: u32 = 0x6dfe;

/// The custom chips the pass reads: the raster, JOY1DAT and CLXDAT (which a read clears).
const VHPOSR: u32 = 0xdf_f006;
const JOY1DAT: u32 = 0xdf_f00c;
const CLXDAT: u32 = 0xdf_f00e;
/// `input_log`, a byte a pass at `input_log_index`, and where it stops.
const INPUT_LOG: u32 = 0x3_4c28;
const LOG_FULL: u32 = 0x2ee0;
/// Hunk offsets the blits and sounds point at: the tiles, `_ELEMENTY` and its masks in hunk 3 and
/// 4, and the three samples in hunk 4 (the rig's, T22, T25).
const TILES: (u32, u32) = (3, 0x1_3540);
const ELEMENTS: (u32, u32) = (4, 0);
const ELEMENT_MASKS: (u32, u32) = (4, 0x1900);
const ITEM_SAMPLE: (u32, u32) = (4, 0x1_2acc);
const TIME_SAMPLE: (u32, u32) = (4, 0x1_3a62);
const SPRING_SAMPLE: (u32, u32) = (4, 0xb548);

/// Level 1's hand engine at its start, as the host builds it once: each pass clones it and
/// transplants the state from memory.
pub struct Level1 {
    base: Game,
    /// The passes the hand engine has played: what shows the mount ran (a watched return does not).
    passes: AtomicU32,
}

impl Level1 {
    pub fn new(data: Arc<GameData>) -> Level1 {
        Level1 {
            base: Game::new(data, Start::Level1),
            passes: AtomicU32::new(0),
        }
    }

    /// The passes the hand engine has played so far.
    pub fn passes(&self) -> u32 {
        self.passes.load(Ordering::Relaxed)
    }
}

/// Memory as the transplant reads it. The play screen and the status bar are picture state the
/// pass never reads (its blits run on the machine), and reading them is most of a pass's time.
struct NoPicture<'m, 'a, 'b>(&'m BusMem<'a, 'b>);

impl Mem for NoPicture<'_, '_, '_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        if (hunk, offset) == (3, SCREEN_AT) || (hunk, offset) == (4, STATUS) {
            return vec![0; len];
        }
        self.0.read(hunk, offset, len)
    }
}

/// `game_frame` mounted: the hand engine's pass in world 1, the translated routine elsewhere.
pub fn game_frame(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let world = bus.rl(img.h(0, WORLD_INDEX));
    if world != 0 {
        return translated(r, bus, img);
    }
    let Some(level) = bus.mount_data().and_then(|d| d.downcast_ref::<Level1>()) else {
        return Err(Stop::Unsupported {
            op: "seam: game_frame mounted with no level 1 engine (TranslatedGame::mount_level_1)"
                .into(),
            at: img.describe(img.h(0, GAME_FRAME.1)),
        });
    };
    level.passes.fetch_add(1, Ordering::Relaxed);
    let mut before = level.base.clone();
    let mut mem = BusMem::new(bus, img);
    before.transplant(&NoPicture(&mem));
    let mut g = before.clone();
    // CLXDAT gathers collisions until read; `char_hit_check` reads it on every running pass and on
    // no paused one.
    let clxdat = if before.state().paused == 0 {
        mem.bus().rw(CLXDAT)
    } else {
        0
    };
    let mut p = BusPass {
        mem: &mut mem,
        img,
        at: 0,
    };
    g.game_frame_on(FrameInput::NONE, clxdat, &mut p);
    store(&mut mem, &before, &g);
    rts(r, mem.bus());
    Ok(())
}

/// The translated `game_frame`, entered past its mount check.
fn translated(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let k = (GAME_FRAME.0 as u32, GAME_FRAME.1);
    match crate::tree::resume(k) {
        Some(f) => f(k, r, bus, img),
        None => Err(Stop::Untranslated(img.describe(img.h(0, GAME_FRAME.1)))),
    }
}

fn at(img: &Image, (hunk, off): (u32, u32)) -> u32 {
    img.h(hunk, off)
}

/// [`Pass`] over the bus.
struct BusPass<'m, 'a, 'b> {
    mem: &'m mut BusMem<'a, 'b>,
    img: &'m Image<'m>,
    /// The routine running, as the last [`Pass::call`] named it.
    at: u32,
}

impl BusPass<'_, '_, '_> {
    fn h0(&self, a: u32) -> u32 {
        self.img.h(0, 0).wrapping_add(a)
    }
}

/// The object ports' memory, as the routine last called reaches it.
struct ObjIo<'p, 'm, 'a, 'b> {
    p: &'p mut BusPass<'m, 'a, 'b>,
    cells: u32,
    /// The object map bytes and collision longs written, for the typed state after.
    map: Vec<(u32, u8)>,
    plane: Vec<(u32, u32)>,
}

impl Io for ObjIo<'_, '_, '_, '_> {
    fn rb(&self, a: u32) -> u8 {
        let a = self.p.h0(a);
        self.p.mem.read_at(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        let at = self.p.h0(a);
        self.p.mem.bus().wb(at, v);
        self.map.push((a, v));
    }
    fn blit(&mut self, b: Blit) {
        let img = self.p.img;
        let screen = img.h(3, SCREEN_AT);
        let every = matches!(self.p.at, 0x2698 | 0x2dd6);
        let first = self.cells == 0;
        let m = self.p.mem.bus();
        match b {
            Blit::Cell { src, dst } => {
                // `h0_21c0` and `h0_62b6` set the modulos and masks for their first cell only,
                // `h0_29ca` (under `char_object_cells` and `h0_2dd6`) for every one.
                m.wl(0xdf_f054, screen.wrapping_add(dst));
                m.wl(0xdf_f050, at(img, TILES).wrapping_add(src));
                if every || first {
                    m.ww(0xdf_f066, 0x2c);
                    m.ww(0xdf_f064, 0);
                    m.wl(0xdf_f044, 0xffff_ffff);
                    m.ww(0xdf_f040, 0x09f0);
                    m.ww(0xdf_f042, 0);
                }
                m.ww(0xdf_f058, 0x1401);
                self.cells += 1;
            }
            Blit::Picture { src, modulo, dst } => {
                m.wl(0xdf_f054, screen.wrapping_add(dst));
                m.wl(0xdf_f050, img.h(4, 0).wrapping_add(src));
                m.ww(0xdf_f066, 0x2a);
                m.ww(0xdf_f064, modulo);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x09f0);
                m.ww(0xdf_f042, 0);
                m.ww(0xdf_f058, 0x1402);
            }
            Blit::Element { frame, tile, dst } => {
                m.ww(0xdf_f066, 0x2c);
                m.ww(0xdf_f064, 0x26);
                m.ww(0xdf_f062, 0x26);
                m.ww(0xdf_f060, 0);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x0fca);
                m.ww(0xdf_f042, 0);
                m.wl(0xdf_f054, screen.wrapping_add(dst));
                m.wl(0xdf_f050, at(img, ELEMENT_MASKS).wrapping_add(2 * frame));
                m.wl(0xdf_f04c, at(img, ELEMENTS).wrapping_add(2 * frame));
                m.wl(0xdf_f048, at(img, TILES).wrapping_add(tile));
                m.ww(0xdf_f058, 0x1401);
            }
            Blit::Tile { .. } | Blit::Bob { .. } => {
                unreachable!(
                    "only the edge redraws (main_loop's, translated) start tile and bob blits"
                )
            }
        }
    }
    fn sound(&mut self, s: Sound) {
        // DMACON off for the channel, its location, length, period and volume, DMACON on.
        let (ch, sample, len, period) = match s {
            Sound::Item => (2u32, ITEM_SAMPLE, 0x7cb, 0x165),
            Sound::Time => (3, TIME_SAMPLE, 0x1c4f, 0x1ff),
            Sound::Spring => (1, SPRING_SAMPLE, 0x9c4, 0x1bf),
        };
        let sample = at(self.p.img, sample);
        let m = self.p.mem.bus();
        let aud = 0xdf_f0a0 + 0x10 * ch;
        m.ww(0xdf_f096, 1 << ch);
        m.wl(aud, sample);
        m.ww(aud + 4, len);
        m.ww(aud + 6, period);
        m.ww(aud + 8, 0x3f);
        m.ww(0xdf_f096, 0x8200 | 1 << ch);
    }
    fn collision(&mut self, off: u32, v: u32) {
        let a = self.p.img.h(3, MAP).wrapping_add(off);
        self.p.mem.bus().wl(a, v);
        self.plane.push((off, v));
    }
}

/// The creature drawing's memory.
struct DrawBus<'p, 'm, 'a, 'b>(&'p mut BusPass<'m, 'a, 'b>);

impl DrawIo for DrawBus<'_, '_, '_, '_> {
    fn rb(&self, a: u32) -> u8 {
        let a = self.0.h0(a);
        self.0.mem.read_at(a)
    }
    fn blit(&mut self, b: CreatureBlit) {
        let m = self.0.mem.bus();
        match b {
            CreatureBlit::CutSetup => {
                m.ww(0xdf_f066, 0x28);
                m.ww(0xdf_f064, 0x22);
                m.ww(0xdf_f062, 0x22);
                m.ww(0xdf_f060, 0x28);
                m.wl(0xdf_f044, 0xffff_0000);
            }
            CreatureBlit::Restore { src, dst, first } => {
                m.wl(0xdf_f054, dst);
                m.wl(0xdf_f050, src);
                if first {
                    m.ww(0xdf_f066, 0x2c);
                    m.ww(0xdf_f064, 0);
                    m.wl(0xdf_f044, 0xffff_ffff);
                    m.ww(0xdf_f040, 0x09f0);
                    m.ww(0xdf_f042, 0);
                }
                m.ww(0xdf_f058, 0x1401);
            }
            CreatureBlit::Cut {
                mask,
                picture,
                dst,
                con0,
                con1,
                size,
            } => {
                m.wl(0xdf_f054, dst);
                m.wl(0xdf_f050, mask);
                m.wl(0xdf_f04c, picture);
                m.wl(0xdf_f048, dst);
                m.ww(0xdf_f040, con0);
                m.ww(0xdf_f042, con1);
                m.ww(0xdf_f058, size);
            }
        }
    }
}

impl Pass for BusPass<'_, '_, '_> {
    fn call(&mut self, offset: u32) {
        self.at = offset;
    }
    fn objects(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut MapObjects)) {
        let mut io = ObjIo {
            p: self,
            cells: 0,
            map: Vec::new(),
            plane: Vec::new(),
        };
        f(&mut MapObjects {
            state: &mut *l.state,
            io: &mut io,
        });
        // What the callees after read from the typed state, the object map and the plane, as
        // `objects::PlayIo` keeps them.
        let map = &mut l.state.object_map;
        for (a, v) in io.map {
            if let Some(b) = map.get_mut(a.wrapping_sub(OBJECT_MAP) as usize) {
                *b = v;
            }
        }
        for (at, v) in io.plane {
            l.plane.set_word(at, (v >> 16) as u16);
            l.plane.set_word(at.wrapping_add(2), v as u16);
        }
    }
    fn creatures(&mut self, l: &mut LevelCtx, f: &mut dyn FnMut(&mut Creatures)) {
        let s = &mut *l.state;
        let camera_y = s.camera.y;
        f(&mut Creatures {
            draw: &mut s.creature_draw,
            slots: &mut s.edges.creatures,
            camera_y,
            io: &mut DrawBus(self),
        });
    }
    fn joy1dat(&mut self, _: &FrameInput) -> u16 {
        self.mem.bus().rw(JOY1DAT)
    }
    fn scratch(&mut self) -> Scratch {
        // The beam's value only counts the reads (`Scratch::vhposr` steps it): no routine on it
        // keeps a VHPOSR byte past its own wait loop. Its reads are made on the bus at the end.
        Scratch::default()
    }
    fn scratch_done(&mut self, s: Scratch) {
        let m = self.mem.bus();
        // VHPOSR's high byte read as often as the routine read it (its wait loop), so a bus that
        // counts reads, as the routine oracle's does, steps as far as under the translated code.
        // On the machine the reads take no time: the wait's lines are not spent (module comment).
        for _ in 0..s.beam {
            m.rb(VHPOSR);
        }
        for (reg, v) in s.hw {
            m.ww(0xdf_f000 + reg as u32, v);
        }
    }
    fn scroll(&mut self, camera_x: u16, camera_y: u16) {
        for (off, bytes) in Scroll::from_camera(camera_x, camera_y).copper() {
            self.mem.write(1, off, &bytes);
        }
    }
    fn raster_copy(&mut self) {
        let a = self.h0(0x8dfa);
        let m = self.mem.bus();
        let v = m.rw(VHPOSR);
        m.ww(a, v);
    }
}

// --- the store back -------------------------------------------------------------------------------

/// What the pass keeps in the typed state outside `LAYOUT`, as (hunk, offset, bytes).
fn extras(s: &State) -> Vec<(u8, u32, Vec<u8>)> {
    let mut out = Vec::new();
    let mut w = |o: u32, v: &[u8]| out.push((0, o, v.to_vec()));
    let c = &s.camera;
    w(0x4d76, &c.screen_x.to_be_bytes());
    w(0x4d7a, &c.screen_y.to_be_bytes());
    let crossed = c.edges.crossed.map(|x| if x { 0xff } else { 0 });
    w(0x51f8, &crossed);
    w(0x519e, &s.frame_flag.to_be_bytes());
    w(0x2a6a, &s.collect.score_add.to_be_bytes());
    w(0x2692, &s.collect.row.to_be_bytes());
    w(0x29c6, &s.collect.taken.to_be_bytes());
    w(0x2fe6, &s.moving.slot.to_be_bytes());
    w(0x306e, &s.moving.stomped.to_be_bytes());
    let cs = &s.edges.creatures;
    w(0x2faa, &cs.index.to_be_bytes());
    w(0x67da, &cs.active);
    for (i, v) in cs.slots.iter().enumerate() {
        let mut b = Vec::new();
        b.extend(v.x.to_be_bytes());
        b.extend(v.y.to_be_bytes());
        b.extend(v.kind.to_be_bytes());
        b.extend([v.facing, v.phase]);
        w(0x67ec + 8 * i as u32, &b);
    }
    w(0x581c, &cs.current.to_be_bytes());
    w(0x687e, &cs.last.to_be_bytes());
    let d = &s.creature_draw;
    w(0x2c7a, &d.restore_index.to_be_bytes());
    w(0x2c7c, &d.cut_index.to_be_bytes());
    w(0x5818, &d.x.to_be_bytes());
    w(0x581a, &d.y.to_be_bytes());
    w(0x581e, &d.restoring.to_be_bytes());
    w(0x67e0, &d.saved);
    w(0x67e6, &d.drawn);
    for n in 0..48usize {
        let (g, i) = (n / 12, n % 12);
        let e = s.edges.cells.cells[g][i];
        out.push((2, 0x1680 + n as u32, vec![s.edges.cells.used[g][i]]));
        let mut b = Vec::new();
        b.extend(e.x.to_be_bytes());
        b.extend(e.y.to_be_bytes());
        b.push(e.code);
        b.extend(e.rest);
        out.push((2, 0x16b0 + 8 * n as u32, b));
    }
    out
}

/// Store what the engine changed: every `LAYOUT` variable and extra it moved, the input log's
/// byte, and the fade and sprites as they stand (the original writes them every pass).
fn store(mem: &mut BusMem, before: &Game, after: &Game) {
    for v in LAYOUT {
        let (was, now) = (before.probe(v), after.probe(v));
        if was != now {
            mem.put_var(v, now);
        }
    }
    let (s0, s1) = (before.state(), after.state());
    for ((h, o, was), (_, _, now)) in extras(s0).into_iter().zip(extras(s1)) {
        if was != now {
            mem.put(h, o, &now);
        }
    }
    let i = s0.input_log.index;
    if s1.input_log.index != i && i < LOG_FULL {
        mem.put(0, INPUT_LOG + i, &[s1.input_log.byte(i)]);
    }
    let Fade {
        on,
        pass,
        now,
        scratch,
    } = after.display().fade;
    mem.put(0, 0x94b4, &[on]);
    mem.put(0, 0x94b6, &pass.to_be_bytes());
    for (i, c) in now.iter().enumerate() {
        mem.put(1, COLOURS + 4 * i as u32, &c.to_be_bytes());
    }
    for (i, w) in scratch.iter().enumerate() {
        mem.put(0, 0x961e + 2 * i as u32, &w.to_be_bytes());
    }
    let s: &CharSprite = &after.display().sprite;
    let writes = {
        let m = &*mem;
        let frame = |a: u32, n: u32| m.read_bytes(a, n);
        s.bytes(&frame)
    };
    for (off, b) in writes {
        mem.put(1, off, &b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plain memory over hunk 0 at its base, no mount data.
    struct Ram(Vec<u8>);

    impl Bus for Ram {
        fn rb(&mut self, a: u32) -> u8 {
            self.0[(a - fd_core::HUNK_BASES[0]) as usize]
        }
        fn rw(&mut self, a: u32) -> u16 {
            u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
        }
        fn rl(&mut self, a: u32) -> u32 {
            (self.rw(a) as u32) << 16 | self.rw(a + 2) as u32
        }
        fn wb(&mut self, _: u32, _: u8) {}
        fn ww(&mut self, _: u32, _: u16) {}
        fn wl(&mut self, _: u32, _: u32) {}
    }

    fn run(world: u32) -> Result<(), Stop> {
        let mut ram = Ram(vec![0; 0x8000]);
        let at = WORLD_INDEX as usize;
        ram.0[at..at + 4].copy_from_slice(&world.to_be_bytes());
        let sizes = [0x8000];
        let img = Image {
            bases: &fd_core::HUNK_BASES,
            sizes: &sizes,
            code: &[],
        };
        game_frame(&mut Regs::new(), &mut ram, &img)
    }

    #[test]
    fn in_world_1_with_no_engine_handed_to_it_the_mount_stops_naming_it() {
        let e = run(0).unwrap_err().to_string();
        assert!(e.contains("no level 1 engine"), "{e}");
    }

    #[test]
    fn in_another_world_the_translated_routine_runs() {
        if crate::TranslatedGame::generated() {
            return; // the real tree's game_frame needs a machine; the full cases run it
        }
        // The stub tree has none: the stop names game_frame's entry, not the mount.
        assert_eq!(run(1), Err(Stop::Untranslated("h0+$1fec".into())));
    }
}
