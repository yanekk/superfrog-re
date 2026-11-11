//! The instruction check (translator T06, DESIGN §2.6): one instruction, translated alone by
//! `tools/pcode_rust.py --insn`, against Moira's one-instruction step ([`Cpu68k::step`]) on generated
//! states, compared in every column:
//!
//! - `regs`: d0-d7, a0-a7, the next PC, and SR's system byte (trace, supervisor, interrupt mask);
//! - `ccr`: X N Z V C;
//! - `mem`: every byte written, and every custom-chip write in order;
//! - `cycles`: the translated cycle counter against Moira's ([`translated_cycles`]; `not-counted`
//!   for a kind the generated code does not count, T12's);
//!
//! plus `stop`, a state the translation stopped on (unsupported, write into code, a panic).
//!
//! Both sides run over the same memory: the program loaded at its hunk bases, so extension words
//! and absolute operands are the program's own, with the area address registers point into
//! ([`AREG_AREA`], [`STACK_AREA`]) filled with random bytes once. Each state then has random d0-d7,
//! a0-a6 inside [`AREG_AREA`], a7 inside [`STACK_AREA`], a random X N Z V C in supervisor mode with
//! interrupts masked, and fresh random bytes around every address register and every absolute
//! operand the instruction names (its `seeds`), never on the instruction's own bytes.
//!
//! A state the original cannot take is skipped, counted and named, never judged: an exception
//! (an odd address, a division by zero, `trap`), or a write into the instruction itself.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use amiga_oracle::{AmigaMemory, Cpu68k, MEM_SIZE, Regs, Stepped};
use pcode_rt::{Bus, CF, Image, NF, PC, SR, Stop, VF, XF, ZF};

use super::Mem;
use crate::routine::{Gen, seed};

/// Where a0-a6 point: 128 KB of random bytes, clear of the program and of a7's area.
pub const AREG_AREA: std::ops::Range<u32> = 0x7A_0000..0x7C_0000;
/// Where a7 points.
pub const STACK_AREA: std::ops::Range<u32> = 0x7E_4000..0x7E_C000;
/// Bytes re-randomised each state on either side of every address register and absolute operand.
const AROUND: u32 = 0x20;
/// The background's extent: both areas with room for a displacement either side.
const FILL: std::ops::Range<u32> = 0x78_0000..0x7F_0000;
/// The VHPOSR probe, as the routine rig's: `move.b $dff006,d0; rts`, called before each state so
/// the translated side's beam counter starts where the oracle's does.
const PROBE: u32 = 0x70_0000;
const PROBE_CODE: [u8; 8] = [0x10, 0x39, 0x00, 0xdf, 0xf0, 0x06, 0x4e, 0x75];

/// A translated instruction, as `pcode_rust.py --insn` emits each.
pub type Entry = fn(&mut pcode_rt::Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The `--insn` module built in from `FD_INSN` (`build/translator/insn.rs`, never committed), or
/// the stub with no sites. `SITES` holds (encoding, `hN+$off`, hunk, offset, length, seeds, entry).
#[allow(
    unused_imports,
    dead_code,
    unused_variables,
    unused_mut,
    unreachable_code,
    clippy::all
)]
pub mod tree {
    include!(concat!(env!("OUT_DIR"), "/insn.rs"));
}

/// The translated cycle count after one instruction, from 0, or `None` when it counted none: every
/// 68000 instruction takes 4 cycles or more, so 0 is a kind the generated code does not count yet
/// (`pcode_fix.cycles`' data-dependent kinds, T12), reported `not-counted`, never failed, unless
/// another state of the same site counted: then it is a path that lost its tick, and fails.
pub fn translated_cycles(r: &pcode_rt::Regs) -> Option<u64> {
    Some(r.cycles()).filter(|&c| c != 0)
}

/// One instruction to check: a row of [`tree::SITES`].
#[derive(Clone, Copy)]
pub struct Site<'a> {
    pub encoding: &'a str,
    pub at: &'a str,
    pub hunk: u32,
    pub off: u32,
    pub len: u32,
    /// Hunk-relative addresses the instruction names as data (its absolute operands).
    pub seeds: &'a [(u32, u32)],
    pub run: Entry,
}

impl<'a> Site<'a> {
    /// A row of the generated `SITES` table.
    #[allow(clippy::type_complexity)]
    pub fn of(row: &'a (&'a str, &'a str, u32, u32, u32, &'a [(u32, u32)], Entry)) -> Site<'a> {
        let &(encoding, at, hunk, off, len, seeds, run) = row;
        Site {
            encoding,
            at,
            hunk,
            off,
            len,
            seeds,
            run,
        }
    }
}

/// Which columns are compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    pub regs: bool,
    pub ccr: bool,
    pub mem: bool,
    pub cycles: bool,
}

impl Columns {
    pub const ALL: Columns = Columns {
        regs: true,
        ccr: true,
        mem: true,
        cycles: true,
    };

    /// `regs,ccr,mem,cycles`, any of them, comma-separated.
    pub fn parse(s: &str) -> Result<Columns, String> {
        let mut c = Columns {
            regs: false,
            ccr: false,
            mem: false,
            cycles: false,
        };
        for w in s.split(',').map(str::trim).filter(|w| !w.is_empty()) {
            match w {
                "regs" => c.regs = true,
                "ccr" => c.ccr = true,
                "mem" => c.mem = true,
                "cycles" => c.cycles = true,
                _ => return Err(format!("`{w}` is no column: regs, ccr, mem, cycles")),
            }
        }
        if c == (Columns {
            regs: false,
            ccr: false,
            mem: false,
            cycles: false,
        }) {
            return Err("--columns names none of regs, ccr, mem, cycles".into());
        }
        Ok(c)
    }
}

/// The first difference found: the state, its column, what differs and both values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub state: usize,
    pub column: &'static str,
    pub what: String,
    pub moira: String,
    pub translated: String,
}

/// One site's verdict over its states.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InsnReport {
    pub states: usize,
    pub pass: usize,
    pub fail: usize,
    /// States the original could not take (an exception, a write into the instruction).
    pub skipped: usize,
    pub first_skip: String,
    /// Failing states by column; a state may fail in several.
    pub regs: usize,
    pub ccr: usize,
    pub mem: usize,
    pub cycles: usize,
    pub stop: usize,
    /// Whether the translated side counted cycles on any state ([`translated_cycles`]).
    pub cycles_counted: bool,
    pub first: Option<Mismatch>,
}

/// The oracle and the memory both sides start from, shared by every site.
pub struct InsnRig {
    cpu: Cpu68k,
    base: Arc<Vec<u8>>,
    bases: Vec<u32>,
    sizes: Vec<u32>,
}

impl InsnRig {
    /// `mem` (a program loaded at `bases`, padded to 16 MB) with the address registers' areas filled
    /// with random bytes from a fixed seed, and the beam probe.
    pub fn new(mut mem: Vec<u8>, bases: &[u32], sizes: &[u32]) -> InsnRig {
        mem.resize(MEM_SIZE, 0);
        let mut g = Gen::new(seed("insn background"));
        for a in FILL.step_by(8) {
            mem[a as usize..a as usize + 8].copy_from_slice(&g.next().to_be_bytes());
        }
        mem[PROBE as usize..PROBE as usize + PROBE_CODE.len()].copy_from_slice(&PROBE_CODE);
        let mut cpu = Cpu68k::new(AmigaMemory {
            chip: vec![],
            slow: vec![],
        });
        cpu.memory_mut().copy_from_slice(&mem);
        cpu.set_hardware(true);
        InsnRig {
            cpu,
            base: Arc::new(mem),
            bases: bases.to_vec(),
            sizes: sizes.to_vec(),
        }
    }

    /// `hN+$off` of `a`, or `$xxxxxx`.
    fn name(&self, a: u32) -> String {
        Image {
            bases: &self.bases,
            sizes: &self.sizes,
            code: &[],
        }
        .describe(a)
    }

    /// Run `site` from `states` generated states, seeded by `seed_text` and the site, and compare
    /// the columns `cols`.
    pub fn check(
        &mut self,
        site: &Site,
        states: usize,
        seed_text: &str,
        cols: Columns,
    ) -> InsnReport {
        let at = self.bases[site.hunk as usize].wrapping_add(site.off);
        let own = at..at + site.len;
        let code = [(site.hunk, site.off, site.off + site.len)];
        let img = Image {
            bases: &self.bases,
            sizes: &self.sizes,
            code: &code,
        };
        let seeds: Vec<u32> = site
            .seeds
            .iter()
            .map(|&(h, o)| self.bases[h as usize].wrapping_add(o))
            .collect();
        let mut g = Gen::new(seed(&format!("insn {} {seed_text}", site.at)));
        let mut rep = InsnReport {
            states,
            ..InsnReport::default()
        };
        // Each judged state's mismatches, and Moira's cycles where the translation counted none:
        // tallied after the loop, since a state counting none fails only when another counted
        // (a path that lost its tick, not a kind left to T12).
        let mut judged: Vec<(Vec<Mismatch>, Option<(usize, u64)>)> = Vec::new();
        for state in 0..states {
            let s = State::random(&mut g, &seeds, &own);
            for (a, b) in &s.pokes {
                self.cpu.write(*a, b);
            }
            let beam = self
                .cpu
                .call(
                    PROBE,
                    Regs {
                        a: [0, 0, 0, 0, 0, 0, 0, STACK_AREA.end],
                        ..Regs::default()
                    },
                    10,
                )
                .expect("the VHPOSR probe")
                .regs
                .d[0] as u8;
            let moira = self.cpu.step(at, s.regs);
            for (a, b) in &s.pokes {
                let a = *a as usize;
                self.cpu.write(a as u32, &self.base[a..a + b.len()]);
            }
            let moira = match moira {
                Ok(m) => m,
                Err(e) => {
                    rep.skip(format!("state {state}: {e}"));
                    continue;
                }
            };
            if let Some(&(w, _, _)) = moira.writes.iter().find(|w| own.contains(&w.0)) {
                rep.skip(format!(
                    "state {state}: the original writes ${w:06x}, a byte of the instruction"
                ));
                continue;
            }
            let before = Mem::new(self.base.clone(), &s.pokes);
            before.beam.set(beam.wrapping_add(1) as u16);
            let mut mem = before.clone();
            let mut r = pcode_rt::Regs::new();
            for n in 0..8 {
                r.set_d(n, s.regs.d[n]);
                r.set_a(n, s.regs.a[n]);
            }
            for (bit, off) in [(4, XF), (3, NF), (2, ZF), (1, VF), (0, CF)] {
                r.set(off, 1, (s.regs.sr >> bit) as u64 & 1);
            }
            r.set(SR, 2, s.regs.sr as u64);
            r.set(PC, 4, at as u64);
            let ran = catch_unwind(AssertUnwindSafe(|| (site.run)(&mut r, &mut mem, &img)));
            let stopped = match ran {
                Ok(Ok(())) => None,
                Ok(Err(stop)) => Some(stop.to_string()),
                Err(_) => Some("panic".to_string()),
            };
            let mut diffs: Vec<Mismatch> = Vec::new();
            let mut uncounted = None;
            match stopped {
                Some(why) => diffs.push(Mismatch {
                    state,
                    column: "stop",
                    what: why,
                    moira: "ran".into(),
                    translated: "stopped".into(),
                }),
                None => {
                    let cyc = translated_cycles(&r);
                    rep.cycles_counted |= cyc.is_some();
                    if cyc.is_none() {
                        uncounted = Some((state, moira.cycles as u64));
                    }
                    let t = Translated {
                        r: &r,
                        writes: mem.changed_since(&before),
                        hw: &mem.hw_writes,
                        cycles: cyc,
                    };
                    compare(state, &moira, &t, cols, |a| self.name(a), &mut diffs);
                }
            }
            judged.push((diffs, uncounted));
        }
        for (mut diffs, uncounted) in judged {
            if let Some((state, c)) = uncounted.filter(|_| cols.cycles && rep.cycles_counted) {
                diffs.push(Mismatch {
                    state,
                    column: "cycles",
                    what: "cycles".into(),
                    moira: c.to_string(),
                    translated: "0".into(),
                });
            }
            rep.tally(&diffs);
        }
        rep
    }
}

impl InsnReport {
    fn skip(&mut self, why: String) {
        self.skipped += 1;
        if self.first_skip.is_empty() {
            self.first_skip = why;
        }
    }

    fn tally(&mut self, diffs: &[Mismatch]) {
        if diffs.is_empty() {
            self.pass += 1;
            return;
        }
        self.fail += 1;
        let mut seen = [false; 5];
        for d in diffs {
            let k = ["regs", "ccr", "mem", "cycles", "stop"]
                .iter()
                .position(|c| *c == d.column)
                .unwrap();
            if !seen[k] {
                seen[k] = true;
                *[
                    &mut self.regs,
                    &mut self.ccr,
                    &mut self.mem,
                    &mut self.cycles,
                    &mut self.stop,
                ][k] += 1;
            }
        }
        if self.first.is_none() {
            self.first = Some(diffs[0].clone());
        }
    }

    /// The cycles column as reported: a count, or `"not-counted"` when the translation counts none.
    pub fn cycles_cell(&self, cols: Columns) -> String {
        if !cols.cycles {
            "null".into()
        } else if !self.cycles_counted {
            "\"not-counted\"".into()
        } else {
            self.cycles.to_string()
        }
    }

    /// The JSON line `fd-judge --insn` prints for `site`.
    pub fn json(&self, site: &Site, cols: Columns) -> String {
        let col = |on: bool, n: usize| if on { n.to_string() } else { "null".into() };
        let first = match &self.first {
            None => "null".to_string(),
            Some(m) => format!(
                "{{\"state\":{},\"column\":\"{}\",\"what\":\"{}\",\"moira\":\"{}\",\"translated\":\"{}\"}}",
                m.state,
                m.column,
                esc(&m.what),
                esc(&m.moira),
                esc(&m.translated)
            ),
        };
        format!(
            "{{\"encoding\":\"{}\",\"at\":\"{}\",\"states\":{},\"pass\":{},\"fail\":{},\"skipped\":{},\
             \"columns\":{{\"regs\":{},\"ccr\":{},\"mem\":{},\"cycles\":{},\"stop\":{}}},\"first\":{first},\"first_skip\":\"{}\"}}",
            esc(site.encoding),
            esc(site.at),
            self.states,
            self.pass,
            self.fail,
            self.skipped,
            col(cols.regs, self.regs),
            col(cols.ccr, self.ccr),
            col(cols.mem, self.mem),
            self.cycles_cell(cols),
            self.stop,
            esc(&self.first_skip)
        )
    }
}

fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o
}

/// One generated state: the registers and the bytes laid over the shared memory.
struct State {
    regs: Regs,
    pokes: Vec<(u32, Vec<u8>)>,
}

impl State {
    fn random(g: &mut Gen, seeds: &[u32], own: &std::ops::Range<u32>) -> State {
        let d = std::array::from_fn(|_| data_value(g));
        let mut a: [u32; 8] = std::array::from_fn(|_| {
            AREG_AREA.start + (g.below((AREG_AREA.end - AREG_AREA.start) as u64) as u32 & !1)
        });
        a[7] = STACK_AREA.start + (g.below((STACK_AREA.end - STACK_AREA.start) as u64) as u32 & !1);
        // Supervisor, interrupts masked, as the game runs; X N Z V C random.
        let sr = 0x2700 | g.below(32) as u16;
        let mut pokes = Vec::new();
        for &c in a.iter().chain(seeds) {
            let lo = c.wrapping_sub(AROUND) & 0xFF_FFFF;
            let bytes: Vec<u8> = (0..2 * AROUND).map(|_| g.next() as u8).collect();
            // Never over the instruction itself: an absolute operand may sit beside it.
            let mut run_start = lo;
            let mut run = Vec::new();
            for (i, b) in bytes.into_iter().enumerate() {
                let x = lo.wrapping_add(i as u32);
                if own.contains(&x) || (x as usize) >= MEM_SIZE || is_hardware(x) {
                    if !run.is_empty() {
                        pokes.push((run_start, std::mem::take(&mut run)));
                    }
                    run_start = x.wrapping_add(1);
                } else {
                    run.push(b);
                }
            }
            if !run.is_empty() {
                pokes.push((run_start, run));
            }
        }
        State {
            regs: Regs { d, a, sr },
            pokes,
        }
    }
}

fn is_hardware(a: u32) -> bool {
    (a & 0xFF_FE00) == 0xDF_F000 || (a & 0xFF_0000) == 0xBF_0000
}

/// A data register's value: the edges flags turn on half the time, any value the other half.
fn data_value(g: &mut Gen) -> u32 {
    match g.below(8) {
        0 => g.below(16) as u32,
        1 => (g.below(32) as i32 - 16) as u32,
        2 => g.pick(&[
            0,
            1,
            0x7f,
            0x80,
            0xff,
            0x7fff,
            0x8000,
            0xffff,
            0x7fff_ffff,
            0x8000_0000,
            0xffff_ffff,
        ]),
        3 => g.next() as u32 & 0xff,
        4 => g.next() as u32 & 0xffff,
        _ => g.next() as u32,
    }
}

/// What the translation left.
struct Translated<'a> {
    r: &'a pcode_rt::Regs,
    writes: Vec<(u32, u8)>,
    hw: &'a [(u16, u16)],
    cycles: Option<u64>,
}

fn compare(
    state: usize,
    m: &Stepped,
    t: &Translated,
    cols: Columns,
    name: impl Fn(u32) -> String,
    out: &mut Vec<Mismatch>,
) {
    let mut diff = |column: &'static str, what: String, moira: String, translated: String| {
        out.push(Mismatch {
            state,
            column,
            what,
            moira,
            translated,
        })
    };
    if cols.regs {
        for n in 0..8 {
            if m.regs.d[n] != t.r.d(n) {
                diff("regs", format!("d{n}"), hex(m.regs.d[n]), hex(t.r.d(n)));
            }
        }
        for n in 0..8 {
            if m.regs.a[n] != t.r.a(n) {
                diff("regs", format!("a{n}"), hex(m.regs.a[n]), hex(t.r.a(n)));
            }
        }
        let tpc = t.r.get(PC, 4) as u32 & 0xFF_FFFF;
        if m.pc & 0xFF_FFFF != tpc {
            diff("regs", "pc".into(), name(m.pc & 0xFF_FFFF), name(tpc));
        }
        // T, S and the interrupt mask; the low byte is the ccr column's.
        let (ms, ts) = (m.regs.sr >> 8 & 0xa7, (t.r.get(SR, 2) >> 8) as u16 & 0xa7);
        if ms != ts {
            diff(
                "regs",
                "sr".into(),
                format!("{ms:02x}xx"),
                format!("{ts:02x}xx"),
            );
        }
    }
    if cols.ccr {
        for (bit, off, f) in [
            (4, XF, "X"),
            (3, NF, "N"),
            (2, ZF, "Z"),
            (1, VF, "V"),
            (0, CF, "C"),
        ] {
            let (mf, tf) = ((m.regs.sr >> bit) & 1, t.r.get(off, 1) as u16);
            if mf != tf {
                diff("ccr", f.into(), mf.to_string(), tf.to_string());
            }
        }
    }
    if cols.mem {
        let mw: Vec<(u32, u8)> = m.writes.iter().map(|&(a, _, new)| (a, new)).collect();
        if mw != t.writes {
            let (mut i, mut j) = (0, 0);
            while i < mw.len() && j < t.writes.len() && mw[i] == t.writes[j] {
                i += 1;
                j += 1;
            }
            let at = match (mw.get(i), t.writes.get(j)) {
                (Some(x), Some(y)) => x.0.min(y.0),
                (Some(x), None) => x.0,
                (None, Some(y)) => y.0,
                (None, None) => unreachable!(),
            };
            let byte = |w: &[(u32, u8)]| {
                w.iter()
                    .find(|x| x.0 == at)
                    .map_or("unwritten".to_string(), |x| format!("{:02x}", x.1))
            };
            diff("mem", format!("${at:06x}"), byte(&mw), byte(&t.writes));
        }
        if m.hw_writes != t.hw {
            diff(
                "mem",
                "custom writes".into(),
                format!("{:x?}", m.hw_writes),
                format!("{:x?}", t.hw),
            );
        }
    }
    if cols.cycles
        && let Some(c) = t.cycles
        && c != m.cycles as u64
    {
        diff(
            "cycles",
            "cycles".into(),
            m.cycles.to_string(),
            c.to_string(),
        );
    }
}

fn hex(v: u32) -> String {
    format!("{v:08x}")
}
