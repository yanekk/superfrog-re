//! The translated program on `pcode-machine` with the OS answered (translator T26), for the two full
//! checks of its OS answers: power-on to the title, and `load_file` loading `_NAPIS` as the original
//! does. Both need the translated tree built in (`FD_TRANSLATED`, `judge::translated::tree`); without
//! it every run stops `untranslated` at once. `fd-judge --os-run` prints their results.

use std::thread;

use amiga_disk::ofs::OfsImage;
use pcode_machine::{Config, Disks, Input, Machine};
use pcode_rt::{Bus, Image, Regs, Stop};

use crate::HUNK_BASES;
use crate::judge::translated::{RUN_STACK, tree};

/// The program's file on disk 1.
pub const PROGRAM: &str = "FOREST_DUMB_FOREVER";

/// Where the stack starts at power-on: slow RAM below hunk 0 (`$C077D8`), which nothing the program
/// loads reaches. Kickstart's own task stack is elsewhere; the program sets none of its own (T26).
pub const POWER_ON_STACK: u32 = 0xC0_7000;

/// CIA-A's PRA, polled for fire (`docs/15-os-surface.md`: `h0+$70c8`, `$7b96`, `$7c7e`). On the
/// power-on path the first poll is the disk 2 prompt's (`h0+$7b70`, which the entry always calls,
/// `docs/03-boot-chain.md`), and it returns at once on fire held; the next is the title's, whose
/// loop starts the game on fire, so fire is let go after the prompt's.
const CIAA_PRA: u32 = 0xBF_E001;

/// The host's own end of a run, not the program's: the title's first fire poll.
pub const AT_TITLE: &str = "host: the title's first fire poll";

/// How a run ended.
#[derive(Clone, Debug)]
pub struct Ended {
    /// `Ok` when the program returned; the stop otherwise (`AT_TITLE` for the title).
    pub result: Result<(), Stop>,
    pub blocks: u64,
    pub cycles: u64,
    /// The files `Open` was asked for, in order, and whether each opened.
    pub opened: Vec<(String, bool)>,
}

/// The machine with fire held for the disk 2 prompt's poll and let go after it, ending the run at
/// the next block once CIA-A's PRA has been read a second time.
struct UntilTitle {
    m: Machine,
    polls: u32,
}

impl Bus for UntilTitle {
    fn poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        if self.polls >= 2 {
            return Err(Stop::Unsupported {
                op: AT_TITLE.into(),
                at: format!("{} cycles", r.cycles()),
            });
        }
        self.m.poll(r, img)
    }
    fn call_outside(&mut self, a: u32, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.m.call_outside(a, r, img)
    }
    fn sync(&mut self, r: &Regs) {
        self.m.sync(r)
    }
    fn rb(&mut self, a: u32) -> u8 {
        let v = self.m.rb(a);
        if a & 0xFF_FFFF == CIAA_PRA {
            self.polls += 1;
            self.m.set_input(&Input::default());
        }
        v
    }
    fn rw(&mut self, a: u32) -> u16 {
        self.m.rw(a)
    }
    fn rl(&mut self, a: u32) -> u32 {
        self.m.rl(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.m.wb(a, v)
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.m.ww(a, v)
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.m.wl(a, v)
    }
}

/// The program's bytes from disk 1's OFS volume.
pub fn program(disk1: &[u8]) -> Result<Vec<u8>, String> {
    OfsImage::parse(disk1)
        .and_then(|v| v.read_file(PROGRAM))
        .map_err(|e| format!("disk 1's {PROGRAM}: {e}"))
}

fn image(sizes: &[u32]) -> Image<'_> {
    Image {
        bases: &HUNK_BASES,
        sizes,
        code: tree::CODE,
    }
}

/// Run `entry` of the tree on `bus` with `r`, on a thread with room for the translated frames.
fn run<B: Bus + Send>(entry: &str, bus: &mut B, r: &mut Regs, sizes: &[u32]) -> Result<(), Stop> {
    let Some(f) = tree::entry(entry) else {
        return Err(Stop::Untranslated(entry.into()));
    };
    let img = image(sizes);
    thread::scope(|s| {
        thread::Builder::new()
            .stack_size(RUN_STACK)
            .spawn_scoped(s, || f(r, bus, &img))
            .expect("a thread for the run")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

/// Power-on: the program loaded at `HUNK_BASES` as Kickstart would, disk 2 in df0, entered at
/// `h0+$0`, fire pressed at the disk 2 prompt, until the title's first fire poll or a stop.
pub fn power_on_to_title(program: &[u8], disk2: Vec<u8>) -> Result<Ended, String> {
    let mut m = Machine::new(Config::a500(HUNK_BASES.to_vec()), program)
        .map_err(|e| format!("{PROGRAM}: {e}"))?;
    m.insert(Disks { df0: disk2 });
    let sizes = m.hunk_sizes().to_vec();
    m.set_input(&Input {
        fire: true,
        ..Input::default()
    });
    let mut bus = UntilTitle { m, polls: 0 };
    let mut r = Regs::new();
    r.set_a(7, POWER_ON_STACK);
    let result = run("h0+$0", &mut bus, &mut r, &sizes);
    Ok(Ended {
        result,
        blocks: r.steps,
        cycles: r.cycles(),
        opened: bus.m.opened().to_vec(),
    })
}

/// `load_file` (`h0+$8ec4`): opens `load_name`, reads it to `load_buffer`, unpacks an `ESL!` file to
/// `load_target` (or copies it there).
pub const LOAD_FILE: &str = "h0+$8ec4";
/// `load_name` (`h0+$90a2`) and `load_target` (`h0+$9086`), which a load list sets before the call.
const LOAD_NAME: u32 = 0x90a2;
const LOAD_TARGET: u32 = 0x9086;
/// `_NAPIS`, the title's first file: its load list's target is `h3+$0`, 18 720 bytes unpacked
/// (`docs/07-executable-map.md`, `docs/05-file-formats.md`).
pub const NAPIS: &str = "_NAPIS";
pub const NAPIS_LEN: usize = 18_720;
/// The file the title loads next: once the original's `load_name` holds it, `_NAPIS` is in.
const NEXT: &[u8] = b"_CZOLOWKA";
/// The original's title is at frame 4921 (`docs/03-boot-chain.md`); a run that has not moved on by
/// this frame is a failure, not a wait.
const ORACLE_CAP: u32 = 5_200;
/// Returned to by a top-level `rts`, outside every hunk and RAM.
const SENTINEL: u32 = 0x00F0_0000;

/// `_NAPIS` loaded on both sides.
#[derive(Clone, Debug)]
pub struct Loaded {
    pub result: Result<(), Stop>,
    /// The bytes at `h3+$0` after the load: the machine's, the original's.
    pub machine: Vec<u8>,
    pub original: Vec<u8>,
    /// The original's frame at which its `load_name` held the next file.
    pub frame: u32,
    pub opened: Vec<(String, bool)>,
}

/// Translated `load_file` alone on the machine (power-on memory, disk 2 in df0, `load_name` and
/// `load_target` set as the title's load list sets them) against the original booted to its title
/// by `boot-to-title`, read when its `load_name` has moved on to the next file.
pub fn load_napis(program: &[u8], disks: Vec<Vec<u8>>) -> Result<Loaded, String> {
    use amiga_oracle::{Amiga, OracleConfig, kickstart_path};

    let h = |hunk: usize, off: u32| HUNK_BASES[hunk] + off;
    let mut m = Machine::new(Config::a500(HUNK_BASES.to_vec()), program)
        .map_err(|e| format!("{PROGRAM}: {e}"))?;
    m.insert(Disks {
        df0: disks[1].clone(),
    });
    for (i, &b) in NAPIS.as_bytes().iter().chain(&[0]).enumerate() {
        m.wb(h(0, LOAD_NAME) + i as u32, b);
    }
    m.wl(h(0, LOAD_TARGET), h(3, 0));
    let sp = POWER_ON_STACK - 4;
    m.wl(sp, SENTINEL);
    let sizes = m.hunk_sizes().to_vec();
    let mut r = Regs::new();
    r.set_a(7, sp);
    let mut result = run(LOAD_FILE, &mut m, &mut r, &sizes);
    if result.is_ok() && r.get(pcode_rt::PC, 4) as u32 != SENTINEL {
        result = Err(Stop::Untranslated(format!(
            "returned to ${:06x}",
            r.get(pcode_rt::PC, 4)
        )));
    }
    let base = h(3, 0) as usize;
    let machine = m.chip()[base..base + NAPIS_LEN].to_vec();

    let text = std::fs::read_to_string(crate::conform_dir().join("scripts/boot-to-title.txt"))
        .map_err(|e| format!("boot-to-title: {e}"))?;
    let sc = amiga_conform::script::parse(&text).map_err(|e| format!("boot-to-title: {e}"))?;
    let rom = kickstart_path().map_err(|e| e.to_string())?;
    let mut am = Amiga::power_on(&OracleConfig::new(rom, disks, ORACLE_CAP + 1))
        .map_err(|e| e.to_string())?;
    let mut frame = 0;
    loop {
        if frame == crate::sync::DISK_2_SWAP {
            am.insert_disk(0, 1);
        }
        if am.peek(h(0, LOAD_NAME), NEXT.len()) == NEXT {
            break;
        }
        if frame == ORACLE_CAP {
            return Err(format!(
                "the original's load_name never held {} by frame {ORACLE_CAP}",
                String::from_utf8_lossy(NEXT)
            ));
        }
        am.step(amiga_conform::script::input_at(&sc.lines, frame))
            .map_err(|e| e.to_string())?;
        frame += 1;
    }
    Ok(Loaded {
        result,
        machine,
        original: am.peek(h(3, 0), NAPIS_LEN),
        frame,
        opened: m.opened().to_vec(),
    })
}
