//! The level scene (docs/08 "Main loop and interrupt", docs/09): what runs while a level plays,
//! and the object dispatch that items, enemies and bosses register into (T15–T18).
//!
//! The original splits a level's work in two. Every frame the vertical-blank interrupt runs the
//! frog, the camera, the clock, scrolling and sound ([`vbl`]); every second frame the main loop
//! runs one 25 Hz pass over the objects and their drawing ([`pass`]). [`LevelCtx`] is what a
//! ported routine works on: the original's memory as the engine models it, the hardware the frame
//! reads, and the sound driver.

pub mod camera;
pub mod clock;
pub mod collision;
pub mod death;
pub mod fade;
pub mod frame;
pub mod load;
pub mod objects;
pub mod panel;
pub mod pass;
pub mod scroll;
pub mod spawn;

use sf_paula::PaulaWrite;

use crate::audio::SoundData;
use crate::input::{FrameInput, Joy};
use crate::layout::h0;
use crate::mem::Bus;

/// `attract_mode` (h0+$18): -1 while the attract demo runs; sound is silent then.
pub const ATTRACT_MODE: u32 = h0(0x18);

/// Whether a 68000 result with these 16 bits leaves the N flag clear: what `bpl` after a `cmp`
/// or `tst` tests. It is not a signed comparison (overflow is ignored), so the ported code uses it
/// wherever the original branches on N alone.
pub fn n_clear(result: u16) -> bool {
    (result as i16) >= 0
}

/// What the custom chips and CIAs answer this frame: the original reads the joystick and the fire
/// button itself, wherever it needs them (docs/08 "Input"); within a frame they hold still.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hw {
    /// `JOY1DAT` (`$DFF00C`): the joystick in port 2, as Denise encodes it.
    pub joy1dat: u16,
    /// Port 2's fire button: CIA-A PRA (`$BFE001`) bit 7 reads 0 while it is held.
    pub fire: bool,
    /// CIA-A's serial data register (`$BFEC01`): the keyboard's last byte (the code shifted up
    /// one, bit 0 set for a key going up, all inverted).
    pub sdr: u8,
}

impl Hw {
    pub fn of(input: FrameInput) -> Hw {
        Hw {
            joy1dat: joy1dat(input.joy),
            fire: input.joy.contains(Joy::FIRE),
            sdr: 0xff,
        }
    }

    /// CIA-A PRA as the game tests it: bit 7 low while port 2's fire is held, bit 6 low for
    /// port 1's (the mouse button).
    pub fn ciaa_pra(&self) -> u8 {
        if self.fire { 0x7f } else { 0xff }
    }
}

/// `JOY1DAT` for a joystick held as `j`: right sets bit 1, left bit 9; bit 0 is bit 1 XOR down and
/// bit 8 is bit 9 XOR up (the quadrature counters a joystick's switches drive).
pub fn joy1dat(j: Joy) -> u16 {
    let (r, l) = (j.contains(Joy::RIGHT) as u16, j.contains(Joy::LEFT) as u16);
    let (d, u) = (j.contains(Joy::DOWN) as u16, j.contains(Joy::UP) as u16);
    r << 1 | (r ^ d) | l << 9 | (l ^ u) << 8
}

/// A slot routine of the original's (docs/09 "The slot record"): `slot` is the slot record (`a0`,
/// or `a5` for a touch handler), `d7` what touched it (0 the frog, 1 the spud; ignored by init and
/// update routines).
pub type SlotFn = fn(c: &mut LevelCtx, slot: u32, d7: u16);

/// One kind of object: the original's template (`obj_templates` h0+$1c6e2, `$ae` bytes each) and
/// every routine a slot of the kind may hold, by the original's address. A slot names its routines
/// by those addresses (`+$00` init, `+$04` update, `+$10` touch), as the original's memory has
/// them, so a transplanted slot runs on as it would there.
#[derive(Debug)]
pub struct ObjectKind {
    pub name: &'static str,
    pub routines: &'static [(u32, SlotFn)],
}

/// What a map marker spawns (docs/09 "Placement"): the stub `obj_flag_jumptab` (h0+$178ec)
/// runs for `(world, marker)`, with the cell's address.
pub type MarkerSpawn = fn(lvl: &mut LevelCtx, cell: u32);

/// The object dispatch: object kinds by template index and their routines by address, marker
/// spawners by world and marker, and the frog's reactions that make objects. Items (T15),
/// enemies and bosses (T16–T18) register theirs.
#[derive(Default)]
pub struct Dispatch {
    kinds: Vec<(u16, &'static ObjectKind)>,
    routines: Vec<(u32, SlotFn)>,
    markers: Vec<((u8, u8), MarkerSpawn)>,
    /// A switch the frog touched (markers 48–51, h0+$f6be), with the cell's address: it opens
    /// its gate and turns into its lever. `None`: nothing happens.
    pub bump_block: Option<MarkerSpawn>,
    /// A pickup drawn in the map's tiles that the frog touched (marker 55, h0+$ed8a): fruit,
    /// clocks, lives, power-ups. `None`: nothing happens.
    pub tile_pickup: Option<MarkerSpawn>,
    /// The object of a bounce pad (marker 4 or 5, the cell and the marker) the frog bounced off
    /// (h0+$e9ea). `None`: none spawns.
    pub bounce_pad: Option<fn(&mut LevelCtx, u32, u16)>,
    /// The spring of a launcher (marker 7) that threw the frog: the cell, and whether it threw it
    /// left (h0+$e94a). `None`: none spawns.
    pub launcher: Option<fn(&mut LevelCtx, u32, bool)>,
}

impl std::fmt::Debug for Dispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dispatch")
            .field(
                "kinds",
                &self
                    .kinds
                    .iter()
                    .map(|k| (k.0, k.1.name))
                    .collect::<Vec<_>>(),
            )
            .field(
                "markers",
                &self.markers.iter().map(|m| m.0).collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// The dispatch a game runs with: every kind and spawner ported so far.
pub fn standard_dispatch() -> &'static Dispatch {
    static D: std::sync::OnceLock<Dispatch> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        let mut d = Dispatch::default();
        crate::items::register_all(&mut d);
        d
    })
}

/// No object kinds at all: the frog plays the level alone (conformance against T14's frog-only
/// cases, whose original has every object frozen out).
pub fn frog_only_dispatch() -> &'static Dispatch {
    static D: std::sync::OnceLock<Dispatch> = std::sync::OnceLock::new();
    D.get_or_init(Dispatch::default)
}

/// Add an object kind under the original's template index.
pub fn register(dispatch: &mut Dispatch, type_id: u16, kind: &'static ObjectKind) {
    dispatch.kinds.retain(|k| k.0 != type_id);
    dispatch.kinds.push((type_id, kind));
    for &(addr, f) in kind.routines {
        dispatch.routines.retain(|r| r.0 != addr);
        dispatch.routines.push((addr, f));
    }
}

impl Dispatch {
    /// The kind registered under `type_id`.
    pub fn kind(&self, type_id: u16) -> Option<&'static ObjectKind> {
        self.kinds.iter().find(|k| k.0 == type_id).map(|k| k.1)
    }

    /// The ported routine for the original's routine at `addr` (an absolute address).
    pub fn routine(&self, addr: u32) -> Option<SlotFn> {
        self.routines.iter().find(|r| r.0 == addr).map(|r| r.1)
    }

    /// Add the spawner for `marker` (1..64) in world table `world` (`obj_world_offset / $100`:
    /// 0..5 for W1–W6 and the final level, 6 the bonus level).
    pub fn register_marker(&mut self, world: u8, marker: u8, spawn: MarkerSpawn) {
        self.markers.retain(|m| m.0 != (world, marker));
        self.markers.push(((world, marker), spawn));
    }

    /// The spawner for `marker` in world table `world`; `None` while nothing models it.
    pub fn marker(&self, world: u8, marker: u8) -> Option<MarkerSpawn> {
        self.markers
            .iter()
            .find(|m| m.0 == (world, marker))
            .map(|m| m.1)
    }

    /// Every `(world, marker)` with a spawner.
    pub fn markers(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.markers.iter().map(|m| m.0)
    }
}

/// Run the original's slot routine at `addr` on `slot`: the ported one, or, for a routine nothing
/// ports yet, nothing (a debug build stops: every object a game meets has its kind registered).
pub fn call(c: &mut LevelCtx, addr: u32, slot: u32, d7: u16) {
    match c.dispatch.routine(addr) {
        Some(f) => f(c, slot, d7),
        None => debug_assert!(
            false,
            "slot routine ${:x} is not ported",
            addr.wrapping_sub(h0(0))
        ),
    }
}

/// What a ported level routine works on.
pub struct LevelCtx<'a> {
    pub bus: Bus<'a>,
    /// The object kinds and marker spawners registered.
    pub dispatch: &'a Dispatch,
    pub hw: Hw,
    pub sound: &'a SoundData,
    /// The frame's Paula writes, in order.
    pub paula: &'a mut Vec<PaulaWrite>,
}

impl LevelCtx<'_> {
    /// `sfx_play` (h0+$12a72): effect `effect` on `channel`, unless the attract demo runs.
    pub fn sfx(&mut self, effect: u8, channel: u8) {
        if self.bus.l(ATTRACT_MODE) == 0 {
            self.bus
                .st
                .audio
                .play_sfx(self.sound, effect, channel, self.paula);
        }
    }

    /// `sample_play` (h0+$148b8): stream sample `k` on channel 3, unless the attract demo runs.
    pub fn sample(&mut self, k: usize) {
        if self.bus.l(ATTRACT_MODE) == 0 {
            self.bus.st.audio.play_sample(self.sound, k, self.paula);
        }
    }

    /// A write to a custom register the frame comparison records (DMACON, INTENA, INTREQ and the
    /// audio registers): the level's own interrupt and DMA writes go out in order with sound's.
    pub fn custom(&mut self, reg: u16, value: u16) {
        self.paula.push(PaulaWrite {
            reg,
            value,
            hpos: 0,
            vpos: 0,
        });
    }

    /// `rng_range` (h0+$160c0): a number in `0..=n`.
    pub fn rng_range(&mut self, n: u16) -> u16 {
        let mut r = self.bus.st.rng();
        let v = r.range(n);
        self.bus.st.set_rng(r);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joy1dat_encodes_directions_as_denise_does() {
        assert_eq!(joy1dat(Joy::NONE), 0);
        assert_eq!(joy1dat(Joy::RIGHT), 0x0003);
        assert_eq!(joy1dat(Joy::LEFT), 0x0300);
        assert_eq!(joy1dat(Joy::DOWN), 0x0001);
        assert_eq!(joy1dat(Joy::UP), 0x0100);
        assert_eq!(joy1dat(Joy::RIGHT | Joy::DOWN), 0x0002);
        assert_eq!(joy1dat(Joy::LEFT | Joy::UP), 0x0200);
        assert_eq!(Hw::of(FrameInput::joy(Joy::FIRE)).ciaa_pra(), 0x7f);
    }
}
