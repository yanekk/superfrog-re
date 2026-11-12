//! The seam (translator DESIGN §2.9, T35): a hand-written routine of `fd-core` run in place of the
//! translated one at the same address.
//!
//! Every generated function asks the bus on entry whether something is mounted at its address
//! (`pcode_rt::Bus::mounted`, emitted by `tools/pcode_rust.py`), so a mount catches every way the
//! program reaches the routine: a `bsr`, a `jsr` through a table, a jump into its entry. [`Seam`] is
//! the table [`crate::TranslatedGame`] answers from. A mounted routine has a translated function's
//! shape ([`Mounted`]); its adapter reads the variables the port reads from the machine's memory
//! into `fd-core`'s typed state (`LAYOUT` and the few values beside it), runs the port, writes what
//! it changed back ([`fd_core::WriteBack`]) and leaves as the original's `rts` does.
//!
//! An adapter is written once per mounted address ([`adapter`]); `fd_core::PORTED`'s `mounted`
//! says which run. Before one is mounted, `fd-judge --seam` holds it to the translated routine it
//! replaces on the same states (fd-conform's `judge::seam`). Granularity (T00, 2026-11-11): both a
//! single routine and level 1's whole `game_frame` pass mounted; level 1 is mounted as the frame
//! (`h0+$1fec`, T36), the two routine adapters here are the seam's proof and the pattern.
//!
//! What a mounted routine does not do: count the original's cycles (the frame bar, DESIGN §2.7,
//! decides whether that shows), count itself in `Regs::depth`, or leave the original's scratch
//! registers; every caller of the routines here reloads or restores them.

use std::cell::RefCell;

use fd_core::level::camera::{Camera, EdgeFlags};
use fd_core::render::objects::{SPRITE_STRIDE, SPRITES, control};
use fd_core::{Mem, MemMut, Ported, WriteBack, layout_var};
use pcode_rt::{Bus, Image, Regs, Stop, rts};

pub use pcode_rt::Mounted;

/// What is mounted where: hunk, offset to the routine run instead of the translated one.
#[derive(Clone, Default)]
pub struct Seam {
    /// Sorted by address, one entry an address.
    mounts: Vec<((u32, u32), Mounted)>,
}

impl Seam {
    /// Nothing mounted: the translated program alone.
    pub fn new() -> Seam {
        Seam::default()
    }

    /// Every entry of `ported` marked `mounted`, each with its [`adapter`]. A mounted entry with no
    /// adapter is an error naming it, so a register that says mounted never runs translated code.
    pub fn from_ported(ported: &[Ported]) -> Result<Seam, String> {
        let mut s = Seam::new();
        for p in ported.iter().filter(|p| p.mounted) {
            let f = adapter((p.hunk, p.offset)).ok_or_else(|| {
                format!(
                    "seam: {} (h{}+${:x}) is mounted in PORTED but has no adapter in fd-translated's seam.rs",
                    p.name, p.hunk, p.offset
                )
            })?;
            s.mount((p.hunk, p.offset), f);
        }
        Ok(s)
    }

    /// Run `f` in place of the translated routine entered at `addr` (hunk, offset); a second mount
    /// at the same address replaces the first.
    pub fn mount(&mut self, addr: (u8, u32), f: Mounted) {
        let k = (addr.0 as u32, addr.1);
        match self.mounts.binary_search_by_key(&k, |m| m.0) {
            Ok(i) => self.mounts[i].1 = f,
            Err(i) => self.mounts.insert(i, (k, f)),
        }
    }

    /// The routine mounted at `k`, as `Bus::mounted` answers.
    pub fn get(&self, k: (u32, u32)) -> Option<Mounted> {
        self.mounts
            .binary_search_by_key(&k, |m| m.0)
            .ok()
            .map(|i| self.mounts[i].1)
    }

    /// The mounted addresses, ascending.
    pub fn addresses(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.mounts.iter().map(|m| m.0)
    }

    pub fn is_empty(&self) -> bool {
        self.mounts.is_empty()
    }
}

/// The adapter written for the routine at `addr` (hunk, offset), whether or not it is mounted: what
/// `fd-judge --seam` judges and [`Seam::from_ported`] mounts.
pub fn adapter(addr: (u8, u32)) -> Option<Mounted> {
    ADAPTERS.iter().find(|a| a.0 == addr).map(|a| a.1)
}

/// Every adapter, by the address of the routine it replaces. Each is `fd_core::PORTED`'s port at
/// that address.
pub const ADAPTERS: &[((u8, u32), Mounted)] = &[
    ((0, 0x4d7e), camera_update),
    ((0, 0x5462), char_sprite_position),
];

/// The machine's memory as `fd-core` reads and writes it: by hunk and offset, resolved through the
/// run's hunk bases. `Mem::read` takes `&self` and the bus reads through `&mut`, hence the cell.
pub struct BusMem<'a, 'b> {
    bus: RefCell<&'a mut (dyn Bus + 'b)>,
    img: &'a Image<'a>,
}

impl<'a, 'b> BusMem<'a, 'b> {
    pub fn new(bus: &'a mut (dyn Bus + 'b), img: &'a Image<'a>) -> Self {
        BusMem {
            bus: RefCell::new(bus),
            img,
        }
    }
}

impl Mem for BusMem<'_, '_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        let mut bus = self.bus.borrow_mut();
        let at = self.img.h(hunk as u32, offset);
        (0..len as u32).map(|k| bus.rb(at.wrapping_add(k))).collect()
    }
}

impl MemMut for BusMem<'_, '_> {
    fn write(&mut self, hunk: u8, offset: u32, bytes: &[u8]) {
        let bus = self.bus.get_mut();
        let at = self.img.h(hunk as u32, offset);
        for (k, &b) in bytes.iter().enumerate() {
            bus.wb(at.wrapping_add(k as u32), b);
        }
    }
}

/// A `LAYOUT` variable's value in `mem`. The adapters name only variables `LAYOUT` holds.
fn var(mem: &dyn Mem, name: &str) -> u32 {
    mem.value(layout_var(name).expect("a LAYOUT variable"))
}

fn put(mem: &mut dyn MemMut, name: &str, value: u32) {
    mem.put_var(layout_var(name).expect("a LAYOUT variable"), value)
}

fn long(mem: &dyn Mem, offset: u32) -> u32 {
    let b = mem.read(0, offset, 4);
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

// --- char_sprite_position (`h0+$5462`) -------------------------------------------------------------

/// What `char_sprite_position` changes: sprites 0 to 3's control words in hunk 1.
struct SpriteControl(u32);

impl WriteBack for SpriteControl {
    fn write_back(&self, mem: &mut dyn MemMut) {
        for n in 0..4u32 {
            // Sprites 2 and 3 sit 16 pixels right: `addi.l #$80000` in the original.
            let ctl = if n < 2 { self.0 } else { self.0.wrapping_add(0x8_0000) };
            mem.put(1, SPRITES + n * SPRITE_STRIDE, &ctl.to_be_bytes());
        }
    }
}

/// `char_sprite_position` mounted: `fd_core::render::objects::CharSprite::position`'s control words.
pub fn char_sprite_position(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let mut mem = BusMem::new(bus, img);
    let ctl = control(
        var(&mem, "char_x"),
        var(&mem, "char_y"),
        var(&mem, "camera_x") as u16,
        var(&mem, "camera_y") as u16,
    );
    SpriteControl(ctl).write_back(&mut mem);
    rts(r, bus);
    Ok(())
}

// --- camera_update (`h0+$4d7e`) --------------------------------------------------------------------

/// Beside `LAYOUT`, what the camera keeps (translator T00): the character's screen place as longs,
/// the crossed flags and `frame_flag`.
const SCREEN_X: u32 = 0x4d76;
const SCREEN_Y: u32 = 0x4d7a;
const CROSSED: u32 = 0x51f8;
const FRAME_FLAG: u32 = 0x519e;

const LATCH: [&str; 4] = [
    "edge_latch_right",
    "edge_latch_left",
    "edge_latch_up",
    "edge_latch_down",
];
const AT: [(&str, &str); 4] = [
    ("edge_at_right_x", "edge_at_right_y"),
    ("edge_at_left_x", "edge_at_left_y"),
    ("edge_at_up_x", "edge_at_up_y"),
    ("edge_at_down_x", "edge_at_down_y"),
];

/// The camera and `frame_flag` as `camera_update` reads and leaves them.
struct CameraState {
    camera: Camera,
    frame_flag: u16,
}

impl CameraState {
    fn read(mem: &dyn Mem) -> CameraState {
        let mut edges = EdgeFlags::default();
        let crossed = mem.read(0, CROSSED, 4);
        for i in 0..4 {
            edges.crossed[i] = crossed[i] != 0;
            edges.latch[i] = var(mem, LATCH[i]) as u16;
            edges.at[i] = (var(mem, AT[i].0) as u16, var(mem, AT[i].1) as u16);
        }
        let ff = mem.read(0, FRAME_FLAG, 2);
        CameraState {
            camera: Camera {
                x: var(mem, "camera_x") as u16,
                y: var(mem, "camera_y") as u16,
                screen_x: long(mem, SCREEN_X),
                screen_y: long(mem, SCREEN_Y),
                edges,
            },
            frame_flag: u16::from_be_bytes([ff[0], ff[1]]),
        }
    }
}

impl WriteBack for CameraState {
    fn write_back(&self, mem: &mut dyn MemMut) {
        let c = &self.camera;
        put(mem, "camera_x", c.x as u32);
        put(mem, "camera_y", c.y as u32);
        // `char_screen_x` is the low word of the long at `h0+$4d76`.
        mem.put(0, SCREEN_X, &c.screen_x.to_be_bytes());
        mem.put(0, SCREEN_Y, &c.screen_y.to_be_bytes());
        for i in 0..4 {
            // `st.b` writes `$ff`, `clr.l` 0: `camera_update` clears all four before it sets any, so
            // a flag's byte is one of the two whatever it held before.
            let e = &c.edges;
            mem.put(0, CROSSED + i as u32, &[if e.crossed[i] { 0xff } else { 0 }]);
            put(mem, LATCH[i], e.latch[i] as u32);
            put(mem, AT[i].0, e.at[i].0 as u32);
            put(mem, AT[i].1, e.at[i].1 as u32);
        }
        mem.put(0, FRAME_FLAG, &self.frame_flag.to_be_bytes());
    }
}

/// `camera_update` mounted: `fd_core::level::camera::Camera::update` on the character's place.
pub fn camera_update(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let mut mem = BusMem::new(bus, img);
    let target = (var(&mem, "char_x"), var(&mem, "char_y"));
    let mut s = CameraState::read(&mem);
    s.camera.update(target, &mut s.frame_flag);
    s.write_back(&mut mem);
    rts(r, bus);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_core::PORTED;

    fn nothing(_: &mut Regs, _: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
        Ok(())
    }

    #[test]
    fn a_mount_is_found_by_its_address_and_a_second_replaces_the_first() {
        let mut s = Seam::new();
        assert!(s.is_empty() && s.get((0, 0x5462)).is_none());
        s.mount((0, 0x5462), char_sprite_position);
        s.mount((0, 0x4d7e), camera_update);
        s.mount((0, 0x5462), nothing);
        assert_eq!(
            s.addresses().collect::<Vec<_>>(),
            [(0, 0x4d7e), (0, 0x5462)]
        );
        let at = |k| s.get(k).map(|f| f as usize);
        assert_eq!(at((0, 0x5462)), Some(nothing as Mounted as usize));
        assert_eq!(at((1, 0x5462)), None);
    }

    #[test]
    fn every_adapter_replaces_a_ported_routine() {
        for &(addr, _) in ADAPTERS {
            assert!(
                PORTED.iter().any(|p| (p.hunk, p.offset) == addr),
                "h{}+${:x} has an adapter but is not in PORTED",
                addr.0,
                addr.1
            );
        }
    }

    #[test]
    fn the_seam_mounts_what_ported_marks_and_refuses_a_mount_with_no_adapter() {
        // Today's register mounts nothing (T36 mounts level 1).
        assert!(Seam::from_ported(PORTED).unwrap().is_empty());
        let mut reg: Vec<Ported> = PORTED.to_vec();
        for p in reg.iter_mut().filter(|p| p.offset == 0x5462) {
            p.mounted = true;
        }
        let s = Seam::from_ported(&reg).unwrap();
        assert_eq!(s.addresses().collect::<Vec<_>>(), [(0, 0x5462)]);
        for p in reg.iter_mut().filter(|p| p.offset == 0x3e0c) {
            p.mounted = true;
        }
        let e = Seam::from_ported(&reg).err().unwrap();
        assert!(e.contains("char_update (h0+$3e0c)"), "{e}");
    }
}
