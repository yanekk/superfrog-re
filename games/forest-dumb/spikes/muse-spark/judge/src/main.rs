//! judge <routine> [--cases N]: runs the original routine (Moira, over the loaded program) and the
//! port built in (SPIKE_PORT) from the same generated states, and prints one JSON line: pass or the
//! first failing case's inputs and differences. A case where the original faults or does not return
//! is not counted. Bytes on the call's stack are not compared (return addresses of inner calls).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use amiga_oracle::{AmigaMemory, Cpu68k, Regs};
use fd_spike_judge::{Cpu, HUNK_BASES, Mem, h, port};

const LOAD: u32 = 0x2_0000;
const STACK: u32 = 0x7F_0000;
const STACK_AREA: std::ops::Range<u32> = 0x7E_0000..0x7F_0000;
const MAX_INSTR: u64 = 200_000;

struct Gen(u64);
impl Gen {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn one_in(&mut self, n: u64) -> bool {
        self.below(n) == 0
    }
    fn pick<T: Copy>(&mut self, c: &[T]) -> T {
        c[self.below(c.len() as u64) as usize]
    }
    /// A coordinate: mostly in a level's range, sometimes anything.
    fn coord(&mut self) -> u32 {
        match self.below(10) {
            0..=2 => self.below(0x200) as u32,
            3..=8 => self.below(0x1400) as u32,
            _ => self.next() as u32,
        }
    }
}

#[derive(Default)]
struct Setup {
    pokes: Vec<(u32, Vec<u8>)>,
    d: [u32; 8],
    a: [u32; 8],
}
impl Setup {
    /// Addresses wrap to the 24-bit bus, as the 68000's do.
    fn put(&mut self, a: u32, bytes: Vec<u8>) {
        let a = a & 0xFF_FFFF;
        if a as usize + bytes.len() <= amiga_oracle::MEM_SIZE {
            self.pokes.push((a, bytes));
        }
    }
    fn b(&mut self, a: u32, v: u8) {
        self.put(a, vec![v]);
    }
    fn w(&mut self, a: u32, v: u16) {
        self.put(a, v.to_be_bytes().to_vec());
    }
    fn l(&mut self, a: u32, v: u32) {
        self.put(a, v.to_be_bytes().to_vec());
    }
}

fn random_regs(g: &mut Gen, s: &mut Setup) {
    for i in 0..8 {
        s.d[i] = g.next() as u32;
    }
    for i in 0..7 {
        s.a[i] = g.next() as u32 & !1;
    }
}

const MAP: u32 = 0xcb38; // h0: a byte map, 0x60 cells to a column

fn map_cell(x: u32, y: u32) -> u32 {
    (((x >> 4) & 0xffff) * 0x60).wrapping_add(y >> 4)
}

fn cell_value(g: &mut Gen) -> u8 {
    match g.below(4) {
        0 => 0,
        1 => g.pick(&[0x10, 0x11, 0x12, 0x13]),
        _ => g.next() as u8,
    }
}

/// Five objects of 8 bytes at h0+$67ec, flagged in h0+$67da, around the camera.
fn objects(g: &mut Gen, s: &mut Setup, types: u64) {
    let cx = 0x20 + g.below(0x1000) as u16;
    let cy = 0x30 + g.below(0x800) as u16;
    s.w(h(0, 0x1e62), cx);
    s.w(h(0, 0x1e64), cy);
    s.l(h(0, 0x1e5a), cx as u32 + g.below(0x140) as u32);
    s.l(h(0, 0x1e5e), cy as u32 + g.below(0xe0) as u32);
    for i in 0..5u32 {
        s.b(h(0, 0x67da) + i, if g.one_in(3) { 0 } else { g.pick(&[1, 1, 0xff, 7]) });
        let o = h(0, 0x67ec) + i * 8;
        let x = (cx as i32 + g.below(0x200) as i32 - 0x60) as u16;
        let y = (cy as i32 + g.below(0x180) as i32 - 0x60) as u16;
        s.w(o, x);
        s.w(o + 2, y);
        s.w(o + 4, if g.one_in(10) { g.next() as u16 } else { g.below(types) as u16 });
        s.b(o + 6, g.next() as u8);
        let (any, low) = (g.next() as u8, g.below(0x20) as u8);
        s.b(o + 7, g.pick(&[0, 0x1e, 0x1f, 0x20, any, low]));
        // The cells an object lands on and around.
        let c = map_cell(x as u32, y as u32);
        for d in [0u32, 1, 0x60, 0x61, 0xc0, 0xc1, 0x5f, 0xbf] {
            s.b(h(0, MAP).wrapping_add(c).wrapping_add(d).wrapping_sub(0x60), cell_value(g));
        }
    }
}

fn generate(routine: &str, g: &mut Gen) -> Setup {
    let mut s = Setup::default();
    random_regs(g, &mut s);
    match routine {
        "5462" => {
            s.l(h(0, 0x1e5a), g.coord());
            s.l(h(0, 0x1e5e), g.coord());
            s.w(h(0, 0x1e62), g.coord() as u16);
            s.w(h(0, 0x1e64), g.coord() as u16);
        }
        "9754" => {
            s.w(h(0, 0x1e62), g.coord() as u16);
            s.w(h(0, 0x1e64), if g.one_in(3) { g.next() as u16 } else { g.below(0x120) as u16 });
        }
        "24ce" => {
            s.b(h(0, 0x4ed0), if g.one_in(5) { 0 } else { g.next() as u8 | 1 });
            s.w(h(0, 0x4b62), if g.one_in(5) { g.next() as u16 | 1 } else { 0 });
            let (x, y) = (g.coord(), g.coord());
            s.l(h(0, 0x1e5a), x);
            s.l(h(0, 0x1e5e), y);
            let c = map_cell(x.wrapping_add(0xe), y.wrapping_add(0x23));
            s.b(h(0, MAP).wrapping_add(c), cell_value(g));
            for a in [0x4ecf, 0x41b6, 0x21b4, 0x21b8, 0x21bc, 0x21be] {
                s.b(h(0, a), g.next() as u8);
            }
        }
        "2dd6" => objects(g, &mut s, 8),
        "2fae" => {
            objects(g, &mut s, 9);
            // The collision bytes the objects read, from h3+$fa50.
            for _ in 0..64 {
                let off = g.below(0x2000) as u32;
                s.b(h(3, 0xfa50) + off, if g.one_in(2) { 0 } else { g.next() as u8 });
            }
        }
        other => panic!("unknown routine {other}"),
    }
    s
}

fn entry(routine: &str) -> u32 {
    h(0, u32::from_str_radix(routine, 16).expect("routine is a hunk 0 offset in hex"))
}

/// `hN+$off` for an address inside the program, else the address.
fn name(a: u32) -> String {
    let bases = HUNK_BASES.get().unwrap();
    for (i, &b) in bases.iter().enumerate().rev() {
        if a >= b && a < b + 0x6_0000 && (i + 1 == bases.len() || a < bases[i + 1]) {
            return format!("h{i}+${:x}", a - b);
        }
    }
    format!("${a:06x}")
}

fn describe_inputs(s: &Setup) -> String {
    let mut t = String::new();
    t += &format!("entry registers: d={:08x?} a0-a6={:08x?}\n", s.d, &s.a[..7]);
    t += "memory set before the call:\n";
    for (a, bytes) in &s.pokes {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        t += &format!("  {} = ${hex}\n", name(*a));
    }
    t
}

fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '\n' => o += "\\n",
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let routine = args.get(1).expect("usage: judge <routine> [--cases N]").clone();
    let cases: usize = args
        .iter()
        .position(|a| a == "--cases")
        .map(|i| args[i + 1].parse().unwrap())
        .unwrap_or(2000);

    let exe_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../build/files/disk1/FOREST_DUMB_FOREVER");
    let exe = std::fs::read(exe_path).unwrap_or_else(|e| panic!("{exe_path}: {e} (run fd-tools extract)"));
    let mut cpu = Cpu68k::new(AmigaMemory { chip: vec![], slow: vec![] });
    let map = cpu.load_exe(&exe, LOAD).expect("loading the program");
    HUNK_BASES.set(map.hunks.iter().map(|h| h.base).collect()).unwrap();
    let base = Arc::new(cpu.memory().to_vec());

    let mut g = Gen(0x9E37_79B9_7F4A_7C15 ^ u64::from_str_radix(&routine, 16).unwrap());
    let (mut valid, mut faulted) = (0usize, 0usize);
    let mut paths = std::collections::BTreeSet::new();
    let mut failure: Option<String> = None;
    let mut failures = 0usize;
    for case in 0..cases {
        let s = generate(&routine, &mut g);
        // The original.
        let saved: Vec<(u32, Vec<u8>)> =
            s.pokes.iter().map(|(a, b)| (*a, cpu.read(*a, b.len()).to_vec())).collect();
        for (a, b) in &s.pokes {
            cpu.write(*a, b);
        }
        let mut regs = Regs { d: s.d, a: s.a, ..Regs::default() };
        regs.a[7] = STACK;
        let res = cpu.call(entry(&routine), regs, MAX_INSTR);
        for (a, b) in saved.iter().rev() {
            cpu.write(*a, b);
        }
        let Ok(res) = res else {
            faulted += 1;
            continue;
        };
        valid += 1;
        paths.insert(res.instr);
        let want: Vec<(u32, u8)> = res
            .writes
            .iter()
            .filter(|w| !STACK_AREA.contains(&w.0))
            .map(|w| (w.0, w.2))
            .collect();

        // The port.
        let before = Mem::new(base.clone(), &s.pokes);
        let mut a = s.a;
        a[7] = STACK;
        let mut pc = Cpu { d: s.d, a, mem: before.clone() };
        let ran = catch_unwind(AssertUnwindSafe(|| port::port(&mut pc)));
        let mut diffs = Vec::new();
        match ran {
            Err(e) => {
                let m = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                diffs.push(format!("the port panicked: {m}"));
            }
            Ok(()) => {
                for i in 0..8 {
                    if pc.d[i] != res.regs.d[i] {
                        diffs.push(format!("d{i}: original ${:08x}, port ${:08x}", res.regs.d[i], pc.d[i]));
                    }
                }
                for i in 0..7 {
                    if pc.a[i] != res.regs.a[i] {
                        diffs.push(format!("a{i}: original ${:08x}, port ${:08x}", res.regs.a[i], pc.a[i]));
                    }
                }
                let got: Vec<(u32, u8)> = pc
                    .mem
                    .changed_since(&before)
                    .into_iter()
                    .filter(|w| !STACK_AREA.contains(&w.0))
                    .collect();
                let (mut i, mut j) = (0, 0);
                while i < want.len() || j < got.len() {
                    let wa = want.get(i).map(|w| w.0).unwrap_or(u32::MAX);
                    let ga = got.get(j).map(|w| w.0).unwrap_or(u32::MAX);
                    if wa == ga {
                        if want[i].1 != got[j].1 {
                            diffs.push(format!("byte {}: original wrote ${:02x}, port wrote ${:02x}", name(wa), want[i].1, got[j].1));
                        }
                        i += 1;
                        j += 1;
                    } else if wa < ga {
                        diffs.push(format!("byte {}: original wrote ${:02x}, port left it ${:02x}", name(wa), want[i].1, before.rb(wa)));
                        i += 1;
                    } else {
                        diffs.push(format!("byte {}: original left it ${:02x}, port wrote ${:02x}", name(ga), before.rb(ga), got[j].1));
                        j += 1;
                    }
                }
            }
        }
        if !diffs.is_empty() {
            failures += 1;
            if failure.is_none() {
                let shown = diffs.len().min(24);
                failure = Some(format!(
                    "case {case}:\n{}differences ({} in all):\n  {}",
                    describe_inputs(&s),
                    diffs.len(),
                    diffs[..shown].join("\n  ")
                ));
            }
        }
    }
    println!(
        "{{\"routine\":\"{routine}\",\"cases\":{cases},\"valid\":{valid},\"faulted\":{faulted},\"paths\":{},\"failures\":{failures},\"pass\":{},\"first_failure\":\"{}\"}}",
        paths.len(),
        failures == 0 && valid > 0,
        esc(failure.as_deref().unwrap_or(""))
    );
}
