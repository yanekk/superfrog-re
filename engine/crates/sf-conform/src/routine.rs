//! Routine conformance (DESIGN §4): a ported level routine against the original's, on many
//! generated states.
//!
//! A [`Rig`] holds the original's memory at a sync point twice over: on Moira ([`Cpu68k`], in
//! hardware mode, so the routine's Paula writes are recorded and its raster waits end) and
//! transplanted into the engine's [`State`] and sound driver. Each [`Setup`] changes some bytes and
//! sets what the joystick and fire button read; [`Rig::check`] runs the original routine and the
//! port from there and requires the same bytes written, the same sound driver state and the same
//! Paula writes. A byte the original writes that no [`LAYOUT`] variable and no sound-driver
//! variable covers fails the check: the port would be writing state the engine does not keep.

use std::collections::BTreeMap;
use std::ops::Range;

use sf_core::audio::{AudioDriver, BANK_ADDR, BankId, File, SoundData, TuneId};
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, h0, index_of};
use sf_core::level::{Hw, LevelCtx};
use sf_core::mem::Bus;
use sf_core::{DisplayData, PaulaWrite, State};
use sf_oracle::{Cpu68k, Regs};

/// The stack the calls run on: below the game's own, above hunk 1's end (`$C7E58C`).
pub const STACK: u32 = 0xC7_F800;
const STACK_AREA: Range<u32> = 0xC7_E600..0xC7_F800;

/// A generated starting point: bytes to change and what the hardware reads.
#[derive(Debug, Clone, Default)]
pub struct Setup {
    pub pokes: Vec<(u32, Vec<u8>)>,
    pub hw: Hw,
    /// Registers the routine is entered with (`a7` is the rig's stack).
    pub d: [u32; 8],
    pub a: [u32; 8],
}

impl Setup {
    pub fn poke_w(&mut self, addr: u32, v: u16) {
        self.pokes.push((addr, v.to_be_bytes().to_vec()));
    }

    pub fn poke_l(&mut self, addr: u32, v: u32) {
        self.pokes.push((addr, v.to_be_bytes().to_vec()));
    }

    pub fn poke_b(&mut self, addr: u32, v: u8) {
        self.pokes.push((addr, vec![v]));
    }
}

/// A small deterministic generator for setups.
#[derive(Debug, Clone)]
pub struct Gen(u64);

impl Gen {
    pub fn new(seed: u64) -> Gen {
        Gen(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A number in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }

    /// One of `choices`.
    pub fn pick<T: Copy>(&mut self, choices: &[T]) -> T {
        choices[self.below(choices.len() as u64) as usize]
    }

    /// True one time in `n`.
    pub fn one_in(&mut self, n: u64) -> bool {
        self.below(n) == 0
    }
}

/// The world's sound files where the game loads them, for the sound driver's transplant.
pub fn sound_files(world: u32, module_addr: u32) -> Vec<(u32, File)> {
    let bank = if world <= 2 {
        BankId::Worlds12
    } else {
        BankId::Worlds36
    };
    let tune = match world {
        1..=6 => TuneId::World(world as u8),
        _ => TuneId::Bonus,
    };
    vec![(BANK_ADDR, File::Bank(bank)), (module_addr, File::Tune(tune))]
}

/// Both sides of a routine comparison, from one memory.
pub struct Rig<'d> {
    cpu: Cpu68k,
    state: State,
    audio: AudioDriver,
    sound: &'d SoundData,
    display: &'d DisplayData,
}

/// What differed, for the panic message.
fn show(addr: u32) -> String {
    match index_of(addr) {
        Some(i) => format!("${addr:06x} ({}+{})", LAYOUT[i].name, addr - LAYOUT[i].addr),
        None => format!("${addr:06x}"),
    }
}

impl<'d> Rig<'d> {
    pub fn new(mem: &AmigaMemory, sound: &'d SoundData, display: &'d DisplayData) -> Rig<'d> {
        let mut cpu = Cpu68k::new(sf_oracle::AmigaMemory {
            chip: mem.chip.clone(),
            slow: mem.slow.clone(),
        });
        cpu.set_hardware(true);
        let state = State::transplant(mem);
        let word = |a: u32| (mem.byte(a) as u32) << 8 | mem.byte(a + 1) as u32;
        let long = |a: u32| word(a) << 16 | word(a + 2);
        let files = sound_files(long(h0(0x40da)), long(h0(0x9ec)));
        let audio = AudioDriver::transplant(sound, &|a| mem.byte(a), &files);
        Rig {
            cpu,
            state,
            audio,
            sound,
            display,
        }
    }

    /// The engine's state the rig starts each check from.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Run the original routine at `h0+$entry` and `port` from `setup`; panic naming `name`, the
    /// case number and the first difference.
    pub fn check(
        &mut self,
        name: &str,
        case: usize,
        entry: u32,
        setup: &Setup,
        port: impl FnOnce(&mut LevelCtx),
    ) {
        // The original.
        let mut saved = Vec::new();
        for (a, b) in &setup.pokes {
            saved.push((*a, self.cpu.read(*a, b.len()).to_vec()));
            self.cpu.write(*a, b);
        }
        let [hi, lo] = setup.hw.joy1dat.to_be_bytes();
        self.cpu.set_hardware_read(0xDF_F00C, hi);
        self.cpu.set_hardware_read(0xDF_F00D, lo);
        self.cpu.set_hardware_read(0xBF_E001, setup.hw.ciaa_pra());
        let regs = Regs {
            d: setup.d,
            a: {
                let mut a = setup.a;
                a[7] = STACK;
                a
            },
            sr: 0x2700,
        };
        let result = self.cpu.call(h0(entry), regs, 2_000_000);
        let theirs_audio = {
            let cpu = &self.cpu;
            let after: BTreeMap<u32, u8> = match &result {
                Ok(r) => r.writes.iter().map(|w| (w.0, w.2)).collect(),
                Err(_) => BTreeMap::new(),
            };
            AudioDriver::transplant(
                self.sound,
                &|a| after.get(&a).copied().unwrap_or(cpu.read(a, 1)[0]),
                &[],
            )
        };
        for (a, b) in saved.iter().rev() {
            self.cpu.write(*a, b);
        }
        let r = result.unwrap_or_else(|e| panic!("{name} case {case}: the original: {e}"));

        // The port.
        let mut st = self.state.clone();
        for (a, b) in &setup.pokes {
            st.poke(*a, b);
        }
        let before = st.clone();
        let mut audio = self.audio.clone();
        let audio_before = audio.vars(self.sound);
        let mut paula: Vec<PaulaWrite> = Vec::new();
        {
            let mut ctx = LevelCtx {
                bus: Bus::new(&mut st, self.display.h0()),
                hw: setup.hw,
                audio: &mut audio,
                sound: self.sound,
                paula: &mut paula,
            };
            port(&mut ctx);
        }

        // Bytes: every byte the original changed, the port changed to the same value, and the
        // port changed nothing else.
        let audio_after = audio.vars(self.sound);
        let in_audio = |a: u32| {
            audio_before
                .iter()
                .any(|(at, b)| (*at..*at + b.len() as u32).contains(&a))
        };
        let mut theirs: BTreeMap<u32, u8> = BTreeMap::new();
        for &(a, _, new) in &r.writes {
            if STACK_AREA.contains(&a) {
                continue;
            }
            if index_of(a).is_none() && !in_audio(a) {
                panic!(
                    "{name} case {case}: the original writes {}, which the engine does not keep \
                     (add it to LAYOUT)",
                    show(a)
                );
            }
            theirs.insert(a, new);
        }
        for (d, (b0, b1)) in LAYOUT.iter().zip(
            sf_core::layout::VarId::ALL
                .iter()
                .map(|&id| (before.var(id), st.var(id))),
        ) {
            if b0 == b1 {
                for k in 0..d.width.len() {
                    if let Some(&v) = theirs.get(&(d.addr + k)) {
                        panic!(
                            "{name} case {case}: the original writes {} = ${v:02x}, the port \
                             leaves it ${:02x}",
                            show(d.addr + k),
                            b0[k as usize]
                        );
                    }
                }
                continue;
            }
            for k in 0..d.width.len() {
                let a = d.addr + k;
                let (old, new) = (b0[k as usize], b1[k as usize]);
                let want = theirs.get(&a).copied().unwrap_or(old);
                if new != want {
                    panic!(
                        "{name} case {case}: {} is ${want:02x} in the original, ${new:02x} in \
                         the port (was ${old:02x})",
                        show(a)
                    );
                }
            }
        }
        let theirs_vars = theirs_audio.vars(self.sound);
        for ((at, ours), (_, t)) in audio_after.iter().zip(&theirs_vars) {
            if let Some(k) = ours.iter().zip(t).position(|(a, b)| a != b) {
                panic!(
                    "{name} case {case}: sound driver ${:06x} is ${:02x} in the original, \
                     ${:02x} in the port",
                    at + k as u32,
                    t[k],
                    ours[k]
                );
            }
        }
        let ours: Vec<(u16, u16)> = paula.iter().map(PaulaWrite::reg_value).collect();
        if ours != r.hw_writes {
            panic!(
                "{name} case {case}: Paula writes differ: original {:x?}, port {:x?}",
                r.hw_writes, ours
            );
        }
    }
}
