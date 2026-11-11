//! The operating system, answered by the machine instead of run (translator DESIGN §2.8, T26): no
//! Kickstart code is executed or translated. ExecBase, the library bases and their jump tables are
//! made-up addresses in Kickstart's ROM window; a `jsr` through a vector reaches the machine through
//! `Bus::call_outside`, is answered here in Rust and returns as `rts` would.
//!
//! What is answered is what a program was measured to use (the game's OS surface, its T01) and no
//! more: any other vector, any argument the answer cannot honour, stops the run naming it
//! (`Stop::Os`), so nothing is guessed. The crate README lists every answer and why.

use pcode_rt::{A0, D0, Image, PC, Regs, Stop};

use crate::Machine;
use crate::mem::ROM_BASE;

pub mod dos;
pub mod exec;
pub mod gfx;

/// The disks in the drives: `df0` is an ADF, read through `amiga-disk`'s OFS reader on each open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Disks {
    pub df0: Vec<u8>,
}

/// The made-up OS's layout in the ROM window, as offsets from `ROM_BASE`. Each library's jump table
/// takes the `NEG` bytes below its base (170 six-byte vectors); its structure the bytes above.
pub(crate) mod layout {
    /// The OS's handler for interrupt level `n` (1 to 7): `IRQ + 8 * n`.
    pub const IRQ: u32 = 0x100;
    pub const NEG: u32 = 0x3FC;
    pub const EXEC: u32 = 0x1000;
    pub const GFX: u32 = 0x2000;
    pub const DOS: u32 = 0x3000;
    /// graphics' `topaz.font`, 8: a TextFont.
    pub const TOPAZ: u32 = 0x3800;
    /// The font's glyphs (`tf_CharData`): nothing reads them.
    pub const TOPAZ_DATA: u32 = 0x3900;
    /// A copper list that only ends, where GfxBase's `copinit` and `LOFlist` point.
    pub const COPPER: u32 = 0x3A00;
    /// The libraries' and the font's names, NUL-terminated.
    pub const NAMES: u32 = 0x3B00;
    /// Bytes of the window the OS structures fill; reads past them are outside memory.
    pub const SIZE: u32 = 0x3C00;
}

/// The libraries the machine answers, with their names as `OpenLibrary` matches them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Library {
    Exec,
    Graphics,
    Dos,
}

impl Library {
    pub const ALL: [Library; 3] = [Library::Exec, Library::Graphics, Library::Dos];

    pub fn name(self) -> &'static str {
        match self {
            Library::Exec => "exec.library",
            Library::Graphics => "graphics.library",
            Library::Dos => "dos.library",
        }
    }

    /// The library's base, the address `OpenLibrary` returns.
    pub fn base(self) -> u32 {
        ROM_BASE
            + match self {
                Library::Exec => layout::EXEC,
                Library::Graphics => layout::GFX,
                Library::Dos => layout::DOS,
            }
    }

    fn by_name(name: &[u8]) -> Option<Library> {
        Library::ALL
            .into_iter()
            .find(|l| l.name().as_bytes() == name)
    }

    fn by_base(base: u32) -> Option<Library> {
        Library::ALL.into_iter().find(|l| l.base() == base)
    }
}

/// Every vector the machine answers: (library, LVO, the call's name). The README's table lists the
/// same; anything else stops as an unknown LVO.
pub const ANSWERED: &[(Library, i32, &str)] = &[
    (Library::Exec, exec::OLD_OPEN_LIBRARY, "OldOpenLibrary"),
    (Library::Exec, exec::CLOSE_LIBRARY, "CloseLibrary"),
    (Library::Exec, exec::OPEN_LIBRARY, "OpenLibrary"),
    (Library::Graphics, gfx::OPEN_FONT, "OpenFont"),
    (Library::Graphics, gfx::CLOSE_FONT, "CloseFont"),
    (Library::Graphics, gfx::FREE_SPRITE, "FreeSprite"),
    (Library::Dos, dos::OPEN, "Open"),
    (Library::Dos, dos::CLOSE, "Close"),
    (Library::Dos, dos::READ, "Read"),
];

/// The OS's state: the made-up structures in the ROM window, the disk and the open files.
#[derive(Clone, Debug)]
pub struct Os {
    rom: Vec<u8>,
    disks: Disks,
    files: Vec<Option<dos::OpenFile>>,
}

impl Default for Os {
    fn default() -> Self {
        Os::new()
    }
}

/// ExecBase's home, and the autovectors the OS owns until a program takes them.
const EXEC_BASE_PTR: u32 = 4;
const AUTOVECTORS: u32 = 0x64;

impl Os {
    pub fn new() -> Os {
        let mut rom = vec![0u8; layout::SIZE as usize];
        let mut names = layout::NAMES;
        let mut put_name = |rom: &mut [u8], s: &str| {
            let at = names;
            rom[at as usize..at as usize + s.len()].copy_from_slice(s.as_bytes());
            names += s.len() as u32 + 1;
            ROM_BASE + at
        };
        for lib in Library::ALL {
            let name = put_name(&mut rom, lib.name());
            let b = (lib.base() - ROM_BASE) as usize;
            // A Library node as exec.library's `MakeLibrary` leaves it: ln_Type NT_LIBRARY (9),
            // ln_Name, lib_NegSize, lib_PosSize, lib_Version 34 (Kickstart 1.3). Nothing measured
            // reads them; they make the bases look like what they stand for.
            rom[b + 8] = 9;
            put_l(&mut rom, b + 10, name);
            put_w(&mut rom, b + 16, layout::NEG as u16);
            put_w(&mut rom, b + 18, 0x200);
            put_w(&mut rom, b + 20, 34);
        }
        let g = layout::GFX as usize;
        // GfxBase's copinit (+$26) and LOFlist (+$32), saved by a program to put back on exit.
        put_l(&mut rom, g + 0x26, ROM_BASE + layout::COPPER);
        put_l(&mut rom, g + 0x32, ROM_BASE + layout::COPPER);
        put_l(&mut rom, layout::COPPER as usize, 0xFFFF_FFFE);
        let font = put_name(&mut rom, gfx::TOPAZ);
        let t = layout::TOPAZ as usize;
        // A TextFont: ln_Name, tf_YSize, tf_XSize, tf_Baseline, tf_LoChar..tf_HiChar, tf_CharData
        // (+$22), tf_Modulo (+$26). Values of the right kind, not Kickstart's (unmeasured): a
        // program was measured copying +$22 and +$26 and reading them no more (T01).
        put_l(&mut rom, t + 10, font);
        put_w(&mut rom, t + 0x14, 8);
        put_w(&mut rom, t + 0x18, 8);
        put_w(&mut rom, t + 0x1A, 6);
        rom[t + 0x20] = 0x20;
        rom[t + 0x21] = 0xFF;
        put_l(&mut rom, t + 0x22, ROM_BASE + layout::TOPAZ_DATA);
        put_w(&mut rom, t + 0x26, 192);
        // Each OS interrupt handler's address holds `rte`, though calling it stops (`irq_handler`).
        for n in 1..=7 {
            put_w(&mut rom, (layout::IRQ + 8 * n) as usize, 0x4E73);
        }
        Os {
            rom,
            disks: Disks::default(),
            files: Vec::new(),
        }
    }

    /// A byte of the ROM window at `off`, if the OS's structures cover it.
    pub(crate) fn rom_byte(&self, off: u32) -> Option<u8> {
        self.rom.get(off as usize).copied()
    }

    pub(crate) fn insert(&mut self, d: Disks) {
        self.disks = d;
    }

    pub(crate) fn disks(&self) -> &Disks {
        &self.disks
    }
}

/// The address of the OS's handler for interrupt level `n` (1 to 7), as the autovectors hold it
/// until a program installs its own.
pub fn irq_handler(n: u32) -> u32 {
    ROM_BASE + layout::IRQ + 8 * n
}

fn put_w(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_be_bytes());
}
fn put_l(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_be_bytes());
}

impl Machine {
    /// Put the disk in the drive: every `Open` from now reads its files (DESIGN §2.8: both of the
    /// original's disks are present, so the first open succeeds).
    pub fn insert(&mut self, d: Disks) {
        self.os.insert(d);
    }

    /// The disk in the drive.
    pub fn disks(&self) -> &Disks {
        self.os.disks()
    }

    /// What Kickstart leaves in low memory for a program it starts: ExecBase at `$4`, and the
    /// autovectors (`$64..$7C`) on the OS's handlers.
    pub(crate) fn install_os(&mut self) {
        self.set_exec_base();
        for n in 1..=7u32 {
            self.poke_l(AUTOVECTORS + 4 * (n - 1), irq_handler(n));
        }
    }

    /// `$4` to the machine's ExecBase: at a snapshot, the oracle's own ExecBase would lead into a
    /// Kickstart that is not here; the program reads `$4` afresh before each library call it makes.
    pub(crate) fn set_exec_base(&mut self) {
        self.poke_l(EXEC_BASE_PTR, Library::Exec.base());
    }

    fn poke_l(&mut self, a: u32, v: u32) {
        let ram = self.ram_mut(a, 4).expect("low chip RAM");
        ram.copy_from_slice(&v.to_be_bytes());
    }

    /// A byte of RAM or of the OS's structures, read without touching the data bus: what the OS's own
    /// code would read for itself. `None` outside both.
    pub(crate) fn peek(&self, a: u32) -> Option<u8> {
        use crate::mem::Region;
        match self.region(a) {
            Region::Chip(i) => Some(self.chip[i]),
            Region::Slow(i) => Some(self.slow[i]),
            Region::Rom(off) => self.os.rom_byte(off),
            _ => None,
        }
    }

    pub(crate) fn peek_w(&self, a: u32) -> Option<u16> {
        Some(u16::from_be_bytes([
            self.peek(a)?,
            self.peek(a.wrapping_add(1))?,
        ]))
    }

    pub(crate) fn peek_l(&self, a: u32) -> Option<u32> {
        Some((self.peek_w(a)? as u32) << 16 | self.peek_w(a.wrapping_add(2))? as u32)
    }

    /// The NUL-terminated string at `a`, at most 255 bytes, from RAM or the OS's structures.
    pub(crate) fn c_string(&self, a: u32, what: &str, site: &str) -> Result<Vec<u8>, Stop> {
        let mut s = Vec::new();
        for i in 0..256u32 {
            match self.peek(a.wrapping_add(i)) {
                Some(0) => return Ok(s),
                Some(b) => s.push(b),
                None => {
                    return Err(Stop::Os(format!(
                        "{what} at ${:06x} runs outside memory at {site}",
                        a & 0xFF_FFFF
                    )));
                }
            }
        }
        Err(Stop::Os(format!(
            "{what} at ${:06x} is over 255 bytes at {site}",
            a & 0xFF_FFFF
        )))
    }

    /// The calling instruction, from the return address the `jsr` pushed: `jsr d16(An)` (and
    /// `d8(An,Xn)`, `d16(PC)`) is 4 bytes, `jsr (An)` 2, `jsr abs.l` 6. Read from memory, since the
    /// translated code does not keep the PC; the return address itself when none of them matches.
    fn call_site(&self, ret: u32, img: &Image) -> String {
        let is = |back: u32, ok: &dyn Fn(u16) -> bool| {
            self.peek_w(ret.wrapping_sub(back)).is_some_and(ok)
        };
        let at = if is(4, &|w| {
            (0x4EA8..=0x4EBB).contains(&w) && w != 0x4EB8 && w != 0x4EB9
        }) {
            ret.wrapping_sub(4)
        } else if is(2, &|w| (0x4E90..=0x4E97).contains(&w)) {
            ret.wrapping_sub(2)
        } else if is(6, &|w| w == 0x4EB9) {
            ret.wrapping_sub(6)
        } else {
            ret
        };
        img.describe(at)
    }

    /// A `jsr` to `a`, outside every hunk (`Bus::call_outside`): answer the OS vector there and
    /// return as `rts` would, or stop naming what was called and from where.
    pub(crate) fn os_call(&mut self, a: u32, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let a = a & 0xFF_FFFF;
        let sp = r.a(7);
        let Some(ret) = self.peek_l(sp) else {
            return Err(Stop::Os(format!(
                "a call to ${a:06x} with the stack at ${sp:06x}, outside memory"
            )));
        };
        let site = self.call_site(ret, img);
        let in_os = a.wrapping_sub(ROM_BASE) < layout::SIZE;
        let lib = Library::ALL.into_iter().find(|l| {
            let below = l.base().wrapping_sub(a);
            below > 0 && below <= layout::NEG && below % 6 == 0
        });
        let answered = match lib {
            Some(lib) => {
                let lvo = a as i32 - lib.base() as i32;
                let done = match lib {
                    Library::Exec => exec::call(self, lvo, r, &site),
                    Library::Graphics => gfx::call(self, lvo, r, &site),
                    Library::Dos => dos::call(self, lvo, r, img, &site),
                };
                match done {
                    Some(res) => res,
                    None => Err(Stop::Os(format!(
                        "unknown {} LVO {lvo} at {site}",
                        lib.name()
                    ))),
                }
            }
            None => {
                if let Some(n) = (1..=7).find(|&n| irq_handler(n) == a) {
                    // The OS's interrupt code is not modelled: the program was measured to take
                    // every interrupt it needs through its own handlers (T01), and loads, where the
                    // OS's would run, finish at once.
                    Err(Stop::Os(format!(
                        "the OS's level {n} interrupt handler, not modelled, entered at {site}"
                    )))
                } else if in_os {
                    Err(Stop::Os(format!("unknown OS address ${a:06x} at {site}")))
                } else {
                    Err(Stop::Untranslated(img.describe(a)))
                }
            }
        };
        answered?;
        r.set_a(7, sp.wrapping_add(4));
        r.set(PC, 4, ret as u64);
        Ok(())
    }
}

/// A register by number in the order the answers name them.
pub(crate) fn d(r: &Regs, n: usize) -> u32 {
    r.get(D0 + 4 * n, 4) as u32
}
pub(crate) fn a(r: &Regs, n: usize) -> u32 {
    r.get(A0 + 4 * n, 4) as u32
}

pub(crate) fn library_by_base(base: u32) -> Option<Library> {
    Library::by_base(base)
}
pub(crate) fn library_by_name(name: &[u8]) -> Option<Library> {
    Library::by_name(name)
}

#[cfg(test)]
mod tests;
