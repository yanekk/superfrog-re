//! `game_frame`'s time on the translated program, callee by callee (translator T55): what level 1's
//! frame mount spends in their place (`fd-translated/src/seam/frame.rs`).
//!
//! `cargo run --release -p fd-conform --example game_frame_costs -- CASE [CASE ..] > costs.tsv`
//!
//! Each case runs on the translated program alone, from its start, with a timing wrapper mounted
//! at `game_frame` and at each routine it calls: the wrapper notes the cycle count at entry and at
//! return and runs the translated routine. Inside `game_frame` the bus is wrapped too, so the
//! reads of DMACONR (the blitter waits), VHPOSR (the raster waits), CLXDAT and JOY1DAT and the
//! writes of BLTSIZE are noted with the cycle of the instruction making them. One TSV row a callee
//! a pass: `case pass callee from dur dmaconr vhposr bltsize blit_wait beam_wait dma_wait clx joy`, `dma_wait` the
//! cycles the machine's poll ticked (the CPU's waits for slots the chips held), `from` and the reads'
//! cycles counted from `game_frame`'s entry, `clx`/`joy` `-` when not read.
//!
//! `--passes` (first argument) prints a row a pass instead, `case pass entry dur clx_at clx joy_at
//! joy`: the cycle count at `game_frame`'s entry, its length, and when (from the entry) CLXDAT and
//! JOY1DAT were read and what they read. `--passes --mounted` runs level 1's frame mount in place
//! of the translated `game_frame`, so the two can be laid side by side.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use fd_conform::translated_engine::{case_start, frame_bar};
use fd_conform::{Case, Inputs};
use fd_translated::{Control, RUN_STACK, Start, TranslatedGame};
use pcode_machine::{Disks, Input};
use pcode_rt::{Bus, Image, Mounted, Regs, Stop};

/// `game_frame` and its callees, hunk 0 offsets.
const GAME_FRAME: u32 = 0x1fec;
const CALLEES: [u32; 21] = [
    0x9b06, 0x94ba, 0x4d7e, 0x513c, 0x5462, 0x9754, 0x2104, 0x2c7e, 0x2fae, 0x21c0, 0x62b6, 0x2dd6,
    0x3698, 0x2d4c, 0x8dfc, 0x3b2a, 0x20b0, 0x24ce, 0x3e0c, 0x2698, 0x3ab4,
];

#[derive(Default)]
struct Log {
    /// Inside `game_frame`: its entry cycle and the pass's number.
    pass: Option<(u64, u32)>,
    passes: u32,
    /// The callee running (offset, entry cycle, counts so far).
    open: Vec<(u32, u64, [u64; 6], Option<u64>, Option<u64>)>,
    rows: Vec<String>,
    case: String,
    last: u64,
    /// The last access, when it was a read of DMACONR (0) or VHPOSR (1): the cycle it was made.
    prev: Option<(u8, u64)>,
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);

/// `--passes`, `--mounted`.
static PASSES: AtomicBool = AtomicBool::new(false);
static MOUNTED: AtomicBool = AtomicBool::new(false);

/// This pass's reads of CLXDAT and JOY1DAT: (cycle from the entry, value).
static READS: Mutex<[Option<(u64, u16)>; 2]> = Mutex::new([None, None]);

/// `--blits`: this pass's BLTSIZE writes, (cycle from the entry, size).
static BLITS: Mutex<Vec<(u64, u16)>> = Mutex::new(Vec::new());
static SHOW_BLITS: AtomicBool = AtomicBool::new(false);

fn with<R>(f: impl FnOnce(&mut Log) -> R) -> R {
    f(LOG.lock().unwrap().as_mut().unwrap())
}

/// The bus inside `game_frame`, noting the reads and writes above.
struct Watch<'a>(&'a mut dyn Bus);

impl Watch<'_> {
    fn note(&mut self, a: u32, write: bool) {
        let custom = Self::custom(a);
        let a = a & 0x1fe;
        with(|l| {
            let at = l.last;
            let prev = l.prev.take();
            if !custom || !matches!(a, 0x002 | 0x006 | 0x00c | 0x00e | 0x058) {
                return;
            }
            let Some((p0, _)) = l.pass else { return };
            let Some(o) = l.open.last_mut() else { return };
            // A read of the same register again with no access between is its wait loop: the
            // cycles since the last read were spent waiting.
            let mut wait = |k: u8, o: &mut (u32, u64, [u64; 6], Option<u64>, Option<u64>)| {
                if let Some((pk, pt)) = prev
                    && pk == k
                {
                    o.2[3 + k as usize] += at - pt;
                }
                l.prev = Some((k, at));
            };
            match (a, write) {
                (0x002, false) => {
                    o.2[0] += 1;
                    wait(0, o);
                }
                (0x006, false) => {
                    o.2[1] += 1;
                    wait(1, o);
                }
                (0x058, true) => o.2[2] += 1,
                (0x00e, false) => o.3 = Some(at - p0),
                (0x00c, false) => o.4 = Some(at - p0),
                _ => {}
            }
        });
    }
    /// What the machine's poll ticked: the CPU's waits for bus slots the chips held.
    fn waited(c0: u64, r: &Regs) {
        with(|l| {
            l.last = r.cycles();
            if let Some(o) = l.open.last_mut() {
                o.2[5] += r.cycles() - c0;
            }
        });
    }
    fn custom(a: u32) -> bool {
        a & 0xff_f000 == 0xdf_f000
    }
}

impl Bus for Watch<'_> {
    fn poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let c0 = r.cycles();
        let res = self.0.poll(r, img);
        Self::waited(c0, r);
        res
    }
    fn insn(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let c0 = r.cycles();
        let res = self.0.insn(r, img);
        Self::waited(c0, r);
        res
    }
    fn call_outside(&mut self, a: u32, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        self.0.call_outside(a, r, img)
    }
    fn sync(&mut self, r: &Regs) {
        with(|l| l.last = r.cycles());
        self.0.sync(r)
    }
    fn mounted(&self, k: (u32, u32)) -> Option<Mounted> {
        self.0.mounted(k)
    }
    fn mount_data(&self) -> Option<&dyn std::any::Any> {
        self.0.mount_data()
    }
    fn rb(&mut self, a: u32) -> u8 {
        self.note(a, false);
        self.0.rb(a)
    }
    fn rw(&mut self, a: u32) -> u16 {
        self.note(a, false);
        let v = self.0.rw(a);
        if Self::custom(a) && matches!(a & 0x1fe, 0x00e | 0x00c) {
            let at = with(|l| l.pass.map(|(p0, _)| l.last - p0));
            if let Some(at) = at {
                let k = (a & 0x1fe == 0x00c) as usize;
                let mut r = READS.lock().unwrap();
                r[k] = r[k].or(Some((at, v)));
            }
        }
        v
    }
    fn rl(&mut self, a: u32) -> u32 {
        self.note(a, false);
        self.0.rl(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.note(a, true);
        self.0.wb(a, v)
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.note(a, true);
        if Self::custom(a) && a & 0x1fe == 0x058 {
            let at = with(|l| l.pass.map(|(p0, _)| l.last - p0));
            if let Some(at) = at {
                BLITS.lock().unwrap().push((at, v));
            }
        }
        self.0.ww(a, v)
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.note(a, true);
        self.0.wl(a, v)
    }
    fn ww_leftover(&mut self, a: u32, v: u16) {
        self.0.ww_leftover(a, v)
    }
}

fn translated(off: u32, r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let f = fd_translated::tree::resume((0, off)).expect("translated");
    f((0, off), r, bus, img)
}

fn game_frame(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    with(|l| {
        l.passes += 1;
        l.pass = Some((r.cycles(), l.passes));
    });
    *READS.lock().unwrap() = [None, None];
    BLITS.lock().unwrap().clear();
    let res = if MOUNTED.load(Ordering::Relaxed) {
        fd_translated::seam::frame::game_frame(r, &mut Watch(bus), img)
    } else {
        translated(GAME_FRAME, r, &mut Watch(bus), img)
    };
    with(|l| {
        let (p0, n) = l.pass.take().unwrap();
        let case = l.case.clone();
        if PASSES.load(Ordering::Relaxed) {
            let s = |v: Option<(u64, u16)>| {
                v.map_or("-\t-".into(), |(at, v)| format!("{at}\t{v:04x}"))
            };
            let [clx, joy] = *READS.lock().unwrap();
            let mut row = format!("{case}\t{n}\t{p0}\t{}\t{}\t{}", r.cycles() - p0, s(clx), s(joy));
            if SHOW_BLITS.load(Ordering::Relaxed) {
                for (at, v) in BLITS.lock().unwrap().iter() {
                    row += &format!("\t{at}:{v:04x}");
                }
            }
            l.rows.push(row);
        } else {
            l.rows.push(format!(
                "{case}\t{n}\t{GAME_FRAME:x}\t0\t{}\t-\t-\t-\t-\t-\t-\t-\t-",
                r.cycles() - p0
            ));
        }
    });
    res
}

fn callee(off: u32, r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let inside = with(|l| {
        if l.pass.is_some() && !PASSES.load(Ordering::Relaxed) {
            l.open.push((off, r.cycles(), [0; 6], None, None));
            true
        } else {
            false
        }
    });
    let res = translated(off, r, bus, img);
    if inside {
        with(|l| {
            let (o, t0, c, clx, joy) = l.open.pop().unwrap();
            let (p0, n) = l.pass.unwrap();
            let s = |v: Option<u64>| v.map_or("-".into(), |v| v.to_string());
            let row = format!(
                "{}\t{n}\t{o:x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                l.case,
                t0 - p0,
                r.cycles() - t0,
                c[0],
                c[1],
                c[2],
                c[3],
                c[4],
                c[5],
                s(clx),
                s(joy)
            );
            l.rows.push(row);
            // A nested callee's reads belong to its caller too.
            if let Some(up) = l.open.last_mut() {
                for i in 0..6 {
                    up.2[i] += c[i];
                }
                up.3 = up.3.or(clx);
                up.4 = up.4.or(joy);
            }
        });
    }
    res
}

macro_rules! wrappers {
    ($($n:ident = $off:literal),* $(,)?) => {
        $(fn $n(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
            callee($off, r, bus, img)
        })*
        const WRAPPERS: &[(u32, Mounted)] = &[$(($off, $n as Mounted)),*];
    };
}

wrappers!(
    w9b06 = 0x9b06, w94ba = 0x94ba, w4d7e = 0x4d7e, w513c = 0x513c, w5462 = 0x5462,
    w9754 = 0x9754, w2104 = 0x2104, w2c7e = 0x2c7e, w2fae = 0x2fae, w21c0 = 0x21c0,
    w62b6 = 0x62b6, w2dd6 = 0x2dd6, w3698 = 0x3698, w2d4c = 0x2d4c, w8dfc = 0x8dfc,
    w3b2a = 0x3b2a, w20b0 = 0x20b0, w24ce = 0x24ce, w3e0c = 0x3e0c, w2698 = 0x2698,
    w3ab4 = 0x3ab4,
);

fn run_case(name: &str) -> Result<Vec<String>, String> {
    let case = Case::load(&fd_conform::conform_dir().join(format!("cases/{name}.toml")))?;
    let bar = frame_bar(&case);
    let start = case_start(&bar)?;
    let inputs = Inputs::load(&bar)?;
    *LOG.lock().unwrap() = Some(Log {
        case: name.into(),
        ..Log::default()
    });
    let frames = case.frames;
    let data = MOUNTED
        .load(Ordering::Relaxed)
        .then(|| fd_conform::game_data())
        .transpose()?;
    std::thread::Builder::new()
        .stack_size(RUN_STACK)
        .spawn(move || {
            let mut g = TranslatedGame::new(Start::Snapshot {
                mem: &start.mem,
                regs: start.regs,
                custom: &start.custom,
                disks: Disks {
                    df0: start.disk2.clone(),
                },
            });
            g.held_at_start(&Input::from(start.input));
            if let Some(d) = data {
                // The engine the frame mount reads (`Bus::mount_data`); the mount is the wrapper's.
                g.mount_level_1(d);
            }
            g.mount((0, GAME_FRAME), game_frame);
            for &(off, f) in WRAPPERS {
                assert!(CALLEES.contains(&off));
                g.mount((0, off), f);
            }
            let mut k = 1u32;
            g.set_input(Input::from(inputs.at(start.frame + k)));
            let stop = g.run(&mut |_| {
                k += 1;
                if k < frames {
                    Control::Next(Input::from(inputs.at(start.frame + k)))
                } else {
                    Control::Stop
                }
            });
            if let Some(s) = stop {
                eprintln!("{}: stopped: {s}", start.frame);
            }
        })
        .unwrap()
        .join()
        .map_err(|_| "the run panicked".to_string())?;
    Ok(LOG.lock().unwrap().take().unwrap().rows)
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    for (flag, on) in [("--passes", &PASSES), ("--mounted", &MOUNTED), ("--blits", &SHOW_BLITS)] {
        if let Some(i) = args.iter().position(|a| a == flag) {
            args.remove(i);
            on.store(true, Ordering::Relaxed);
        }
    }
    if PASSES.load(Ordering::Relaxed) {
        println!("case\tpass\tentry\tdur\tclx_at\tclx\tjoy_at\tjoy");
    } else {
        println!("case\tpass\tcallee\tfrom\tdur\tdmaconr\tvhposr\tbltsize\tblit_wait\tbeam_wait\tdma_wait\tclx\tjoy");
    }
    for name in args {
        match run_case(&name) {
            Ok(rows) => rows.iter().for_each(|r| println!("{r}")),
            Err(e) => eprintln!("{name}: {e}"),
        }
    }
}
