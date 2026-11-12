//! The runtime translated 68000 code calls: Ghidra's register space, memory through a host-provided
//! `Bus`, where the program sits, P-code's operations (`ops`) and the stops that end a run instead of
//! guessing (translator-trial DESIGN §2.5, §3.3).
//!
//! Pure: no clock, I/O, floats or hash maps. Every fact about a program (its hunk bases and sizes,
//! which bytes are code) arrives as input, so the same generated code runs wherever it is loaded.

use std::fmt;

pub mod bcd;
pub mod cycles;
pub mod ops;

/// Blocks a translated run may enter, counted in `Regs::steps` across every call, before it stops as
/// `Stop::Hang { why: Steps }` (DESIGN §2.4, §5.2). A per-call count let endless recursion run until
/// the Rust stack overflowed; one count for the run cannot be reset by a call.
///
/// Measured (translator T14, 2026-11-11, the trial's 85 judgeable routines over 2 000 states each):
/// the longest run was 3 968 blocks (`h0+$b20`), `game_frame` 737. The trial's 10 000 000 is kept,
/// 2 500 times that and more than the blocks a 68000 can run in 280 frames (at most 35 450 a
/// frame: 141 800 cycles, 4 a instruction), until T16 measures whole frames of the whole tree and
/// sets it to 10 times the longest.
pub const STEP_CAP_GLOBAL: u64 = 10_000_000;

/// Translated functions a run may have entered and not yet left, counted in `Regs::depth`, before it
/// stops as `Stop::Hang { why: Depth }`. Each is a Rust frame, so the cap bounds the stack a run
/// needs; a host runs translated code on a thread with room for this many (fd-conform's judge).
///
/// Measured (translator T14, 2026-11-11): the deepest call chain in the whole tree's call graph is 8
/// functions (from the program entry, no recursion), the level 3 handler's 7; the trial's routines
/// nested 6 at most (`game_frame`). Four times the deepest chain, which also covers an interrupt
/// handler's chain taken on top of the deepest one (8 + 7 + 1).
pub const DEPTH_CAP: u32 = 32;

/// The size of Ghidra's 68000 register space: its last register, `FP7`, is 10 bytes at `0x746`
/// (`Processors/68000/data/languages/68000.sinc`, Ghidra 12.0).
pub const REG_SPACE: usize = 0x750;
/// `D0`; `Dn` is 4 bytes at `D0 + 4n`, big-endian, so its low byte is at `D0 + 4n + 3`.
pub const D0: usize = 0x00;
/// `A0`; `An` is at `A0 + 4n`, and `A7` (Ghidra's `SP`) at `A0 + 28`.
pub const A0: usize = 0x20;
/// The condition flags, one byte each, holding 0 or 1.
pub const XF: usize = 0x43;
pub const NF: usize = 0x44;
pub const ZF: usize = 0x45;
pub const VF: usize = 0x46;
pub const CF: usize = 0x47;
pub const PC: usize = 0x50;
pub const SR: usize = 0x200;

/// A routine with a translated function's shape, mounted in its place through [`Bus::mounted`].
pub type Mounted = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// The original's `rts`, for a mounted routine to leave by: pop the return address off `A7` into
/// `PC`, where the generated caller checks it against the address after its call.
pub fn rts<B: Bus + ?Sized>(r: &mut Regs, bus: &mut B) {
    let sp = r.a(7);
    let ret = bus.rl(sp);
    r.set_a(7, sp.wrapping_add(4));
    r.set(PC, 4, ret as u64);
}

/// Memory as the host provides it. Word and long accesses are big-endian.
pub trait Bus {
    /// Called at the start of every translated basic block, before its first instruction: where a
    /// machine takes a pending interrupt (DESIGN §2.8), by calling the translated handler with the
    /// exception frame pushed. A stop it returns ends the run. Plain memory has nothing to take.
    #[inline]
    fn poll(&mut self, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let _ = (r, img);
        Ok(())
    }
    /// A `jsr`/`bsr` to `a`, outside every hunk, made with the return address already pushed: where a
    /// machine answers the operating system's library vectors (translator T26). The answer returns
    /// as `rts` would, popping that address into `PC`, which the generated caller then checks.
    /// Plain memory has no OS, so control has reached an address with no translation.
    fn call_outside(&mut self, a: u32, r: &mut Regs, img: &Image) -> Result<(), Stop> {
        let _ = r;
        Err(Stop::Untranslated(img.describe(a)))
    }
    /// The hand-written routine mounted in place of the translated function entered at `k` (hunk,
    /// offset), if any: the seam (translator DESIGN §2.9). Every generated function asks on entry,
    /// before it counts itself in, and runs the mounted one instead, which must leave as the
    /// original's `rts` does (pop the return address into `PC`, see [`rts`]). Plain memory mounts
    /// nothing.
    #[inline]
    fn mounted(&self, k: (u32, u32)) -> Option<Mounted> {
        let _ = k;
        None
    }
    fn rb(&mut self, a: u32) -> u8;
    fn rw(&mut self, a: u32) -> u16;
    fn rl(&mut self, a: u32) -> u32;
    fn wb(&mut self, a: u32, v: u8);
    fn ww(&mut self, a: u32, v: u16);
    fn wl(&mut self, a: u32, v: u32);
}

/// Ghidra's 68000 register space, byte for byte, big-endian, the run's two counters and its cycles.
#[derive(Clone, PartialEq, Eq)]
pub struct Regs {
    pub space: [u8; REG_SPACE],
    /// Blocks entered since the host last set it, across every call; against `STEP_CAP_GLOBAL`. A
    /// host that runs frame by frame resets it each frame.
    pub steps: u64,
    /// Translated functions entered and not yet returned from; against `DEPTH_CAP`. Left as it was
    /// when a run stops, since a stop ends the run.
    pub depth: u32,
    /// The 68000 cycles the translated instructions have taken (`cycles`, DESIGN §2.6): the
    /// machine's beam position comes from it. Outside `space`, which is Ghidra's alone.
    cycles: u64,
    /// Of `cycles`, those the instructions spent off the bus (their internal cycles,
    /// [`cycles::off_bus`]): a machine that models DMA contention gives the chips those slots.
    off_bus: u64,
    /// Each instruction's (cycles, of them off the bus) since the machine last cleared it
    /// ([`Regs::clear_trail`]), oldest first: where in a block the bus is free matters to DMA
    /// contention, not only how much (translator T49). Past [`TRAIL`] entries the last one takes
    /// the rest.
    trail: [(u32, u32); TRAIL],
    trail_len: u8,
}

/// Instructions a block's trail holds one by one ([`Regs::trail`]).
pub const TRAIL: usize = 48;

impl Default for Regs {
    fn default() -> Self {
        Regs {
            space: [0; REG_SPACE],
            steps: 0,
            depth: 0,
            cycles: 0,
            off_bus: 0,
            trail: [(0, 0); TRAIL],
            trail_len: 0,
        }
    }
}

impl fmt::Debug for Regs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d: Vec<String> = (0..8).map(|n| format!("{:08x}", self.d(n))).collect();
        let a: Vec<String> = (0..8).map(|n| format!("{:08x}", self.a(n))).collect();
        let flags = [XF, NF, ZF, VF, CF].map(|o| self.space[o]);
        write!(f, "Regs {{ d: {d:?}, a: {a:?}, xnzvc: {flags:?} }}")
    }
}

impl Regs {
    pub fn new() -> Self {
        Self::default()
    }

    /// The cycles counted so far.
    #[inline]
    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    /// Count `n` more cycles: generated code calls it once per instruction (per path of a branch).
    #[inline]
    pub fn tick(&mut self, n: u32) {
        self.cycles = self.cycles.wrapping_add(n as u64);
        let len = self.trail_len as usize;
        if len < TRAIL {
            self.trail[len] = (n, 0);
            self.trail_len += 1;
        } else {
            self.trail[TRAIL - 1].0 = self.trail[TRAIL - 1].0.saturating_add(n);
        }
    }

    /// Of the cycles counted so far, those spent off the bus.
    #[inline]
    pub fn off_bus_cycles(&self) -> u64 {
        self.off_bus
    }

    /// `n` of the cycles just ticked were off the bus: generated code calls it after an
    /// instruction's tick when the instruction has internal cycles ([`cycles::off_bus`]).
    #[inline]
    pub fn off_bus(&mut self, n: u32) {
        self.off_bus = self.off_bus.wrapping_add(n as u64);
        match self.trail_len as usize {
            0 => {
                self.trail[0] = (0, n);
                self.trail_len = 1;
            }
            len => self.trail[len - 1].1 = self.trail[len - 1].1.saturating_add(n),
        }
    }

    /// The instructions ticked since [`Regs::clear_trail`]: (cycles, of them off the bus) each.
    #[inline]
    pub fn trail(&self) -> &[(u32, u32)] {
        &self.trail[..self.trail_len as usize]
    }

    /// Forget the trail: the machine has fitted those instructions onto the bus.
    #[inline]
    pub fn clear_trail(&mut self) {
        self.trail_len = 0;
    }

    /// The `size` bytes at `off`, big-endian.
    #[inline]
    pub fn get(&self, off: usize, size: usize) -> u64 {
        self.space[off..off + size]
            .iter()
            .fold(0u64, |v, &b| (v << 8) | b as u64)
    }

    /// The low `size` bytes of `v` into the `size` bytes at `off`, big-endian; the rest untouched.
    #[inline]
    pub fn set(&mut self, off: usize, size: usize, v: u64) {
        for (i, b) in self.space[off..off + size].iter_mut().enumerate() {
            *b = (v >> (8 * (size - 1 - i))) as u8;
        }
    }

    pub fn d(&self, n: usize) -> u32 {
        self.get(D0 + 4 * n, 4) as u32
    }
    pub fn set_d(&mut self, n: usize, v: u32) {
        self.set(D0 + 4 * n, 4, v as u64)
    }
    pub fn a(&self, n: usize) -> u32 {
        self.get(A0 + 4 * n, 4) as u32
    }
    pub fn set_a(&mut self, n: usize, v: u32) {
        self.set(A0 + 4 * n, 4, v as u64)
    }
}

/// Where the program sits, given at run time, and which bytes are translated instructions.
///
/// `bases[h]` and `sizes[h]` are hunk `h`'s load address and length. `code` holds
/// `(hunk, start, end)` ranges, `end` exclusive, sorted by hunk then start and not overlapping:
/// `is_code` searches them by halves, since every store asks it.
#[derive(Clone, Copy, Debug)]
pub struct Image<'a> {
    pub bases: &'a [u32],
    pub sizes: &'a [u32],
    pub code: &'a [(u32, u32, u32)],
}

impl Image<'_> {
    /// The absolute address of byte `off` of hunk `hunk`.
    #[inline]
    pub fn h(&self, hunk: u32, off: u32) -> u32 {
        self.bases[hunk as usize].wrapping_add(off)
    }

    /// `a` as (hunk, offset), or `None` outside every hunk.
    #[inline]
    pub fn hunk_of(&self, a: u32) -> Option<(u32, u32)> {
        self.bases
            .iter()
            .zip(self.sizes)
            .position(|(&b, &s)| a.wrapping_sub(b) < s)
            .map(|h| (h as u32, a.wrapping_sub(self.bases[h])))
    }

    /// `a` as the stops name it: `hN+$off`, or `$xxxxxx` outside every hunk.
    pub fn describe(&self, a: u32) -> String {
        match self.hunk_of(a) {
            Some((h, off)) => format!("h{h}+${off:x}"),
            None => format!("${a:06x}"),
        }
    }

    /// The first byte of `a .. a + size` that is a translated instruction's, if any.
    pub fn first_code_byte(&self, a: u32, size: u32) -> Option<u32> {
        (0..size)
            .map(|i| a.wrapping_add(i))
            .find(|&x| self.byte_is_code(x))
    }

    /// Whether any byte of `a .. a + size` is a translated instruction's.
    #[inline]
    pub fn is_code(&self, a: u32, size: u32) -> bool {
        // An access within one hunk (all but a straddling one) is answered by a single search.
        if let Some((h, off)) = self.hunk_of(a)
            && off
                .checked_add(size)
                .is_some_and(|e| e <= self.sizes[h as usize])
        {
            return self.range_is_code(h, off, off + size);
        }
        self.first_code_byte(a, size).is_some()
    }

    fn byte_is_code(&self, a: u32) -> bool {
        self.hunk_of(a)
            .is_some_and(|(h, off)| self.range_is_code(h, off, off + 1))
    }

    /// Whether `[lo, hi)` of hunk `h` meets a code range.
    fn range_is_code(&self, h: u32, lo: u32, hi: u32) -> bool {
        // The first range of hunk h (or a later hunk) ending after lo; it meets [lo, hi) if it
        // is hunk h's and starts before hi.
        let i = self
            .code
            .partition_point(|&(ch, _, end)| (ch, end) <= (h, lo));
        self.code
            .get(i)
            .is_some_and(|&(ch, start, _)| ch == h && start < hi)
    }
}

/// Why a translated run stopped instead of going on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Control reached an address with no translation: `hN+$off`, or `$xxxxxx` outside every hunk.
    Untranslated(String),
    /// A store into a byte of a translated instruction: `hN+$off` of the first such byte.
    WriteIntoCode(String),
    /// An instruction or P-code op the translator does not carry out.
    Unsupported { op: String, at: String },
    /// A cap reached (DESIGN §2.4, §5.2).
    Hang { why: Hang },
    /// The instruction at `at` took the 68000 exception `vector` (5: a division by zero). The
    /// translated code cannot follow it into the handler, so it ends the run; the machine decides
    /// what the original's exception would have done.
    Trap { vector: u32, at: String },
    /// A read or write of `$xxxxxx`, an address the machine maps to no memory or chip (ROM,
    /// autoconfig, the clock chip, nothing). The bus cannot fail an access, so the machine records
    /// it and returns this at the next block start (pcode-machine).
    Outside(String),
    /// A call into the operating system the machine does not answer (an unknown library vector, an
    /// argument it cannot honour), its reason and the calling instruction's address (pcode-machine).
    Os(String),
}

/// Which cap a `Stop::Hang` reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hang {
    /// `STEP_CAP_GLOBAL`: an endless loop, however many calls it spans.
    Steps,
    /// `DEPTH_CAP`: endless recursion, by calls or by jumps into other functions' entries.
    Depth,
}

impl fmt::Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stop::Untranslated(at) => write!(f, "untranslated: {at}"),
            Stop::WriteIntoCode(at) => write!(f, "write-into-code: {at}"),
            Stop::Unsupported { op, at } => write!(f, "unsupported: {op} at {at}"),
            Stop::Hang { why: Hang::Steps } => write!(f, "hang (steps)"),
            Stop::Hang { why: Hang::Depth } => write!(f, "hang (depth)"),
            Stop::Trap { vector, at } => write!(f, "trap: vector {vector} at {at}"),
            Stop::Outside(a) => write!(f, "outside-memory: {a}"),
            Stop::Os(why) => write!(f, "os: {why}"),
        }
    }
}

/// What a translated function does on entry: one more frame, or `Hang { why: Depth }` past the cap.
/// It undoes this itself (`r.depth -= 1`) when it returns, or when a function it jumped into does.
#[inline]
pub fn fn_entry(r: &mut Regs) -> Result<(), Stop> {
    r.depth += 1;
    if r.depth > DEPTH_CAP {
        return Err(Stop::Hang { why: Hang::Depth });
    }
    Ok(())
}

/// What every translated basic block does first: one more step, or `Hang { why: Steps }` past the
/// cap, then the bus's `poll`.
#[inline]
pub fn block_start<B: Bus + ?Sized>(bus: &mut B, r: &mut Regs, img: &Image) -> Result<(), Stop> {
    r.steps += 1;
    if r.steps > STEP_CAP_GLOBAL {
        return Err(Stop::Hang { why: Hang::Steps });
    }
    bus.poll(r, img)
}

/// A load of `size` bytes at `a`, big-endian. An 8-byte load is two longs; any other size than
/// 1, 2, 4 or 8 is read byte by byte.
#[inline]
pub fn load<B: Bus + ?Sized>(bus: &mut B, a: u32, size: usize) -> u64 {
    match size {
        1 => bus.rb(a) as u64,
        2 => bus.rw(a) as u64,
        4 => bus.rl(a) as u64,
        8 => ((bus.rl(a) as u64) << 32) | bus.rl(a.wrapping_add(4)) as u64,
        _ => {
            assert!(size <= 8, "load of {size} bytes");
            (0..size).fold(0u64, |v, i| {
                (v << 8) | bus.rb(a.wrapping_add(i as u32)) as u64
            })
        }
    }
}

/// A store of the low `size` bytes of `v` at `a`, big-endian, split as `load` is. A store touching
/// any byte of a translated instruction writes nothing and returns `Stop::WriteIntoCode`.
#[inline]
pub fn store<B: Bus + ?Sized>(
    bus: &mut B,
    img: &Image,
    a: u32,
    size: usize,
    v: u64,
) -> Result<(), Stop> {
    if img.is_code(a, size as u32) {
        let at = img.first_code_byte(a, size as u32).unwrap_or(a);
        return Err(Stop::WriteIntoCode(img.describe(at)));
    }
    match size {
        1 => bus.wb(a, v as u8),
        2 => bus.ww(a, v as u16),
        4 => bus.wl(a, v as u32),
        8 => {
            bus.wl(a, (v >> 32) as u32);
            bus.wl(a.wrapping_add(4), v as u32);
        }
        _ => {
            assert!(size <= 8, "store of {size} bytes");
            for i in 0..size {
                bus.wb(a.wrapping_add(i as u32), (v >> (8 * (size - 1 - i))) as u8);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat 64 KB memory at address 0 that logs every write as (address, size).
    struct Ram {
        bytes: Vec<u8>,
        writes: Vec<(u32, usize)>,
    }

    impl Ram {
        fn new() -> Self {
            Ram {
                bytes: vec![0; 0x10000],
                writes: Vec::new(),
            }
        }
    }

    impl Bus for Ram {
        fn rb(&mut self, a: u32) -> u8 {
            self.bytes[a as usize]
        }
        fn rw(&mut self, a: u32) -> u16 {
            u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
        }
        fn rl(&mut self, a: u32) -> u32 {
            ((self.rw(a) as u32) << 16) | self.rw(a + 2) as u32
        }
        fn wb(&mut self, a: u32, v: u8) {
            self.writes.push((a, 1));
            self.bytes[a as usize] = v;
        }
        fn ww(&mut self, a: u32, v: u16) {
            self.writes.push((a, 2));
            self.bytes[a as usize..a as usize + 2].copy_from_slice(&v.to_be_bytes());
        }
        fn wl(&mut self, a: u32, v: u32) {
            self.writes.push((a, 4));
            self.bytes[a as usize..a as usize + 4].copy_from_slice(&v.to_be_bytes());
        }
    }

    // Two hunks, deliberately out of address order: hunk 0 at $3000 (0x100 bytes), hunk 1 at
    // $1000 (0x80 bytes). Code: hunk 0 bytes $10..$20 and $40..$42, hunk 1 bytes $0..$4.
    const BASES: [u32; 2] = [0x3000, 0x1000];
    const SIZES: [u32; 2] = [0x100, 0x80];
    const CODE: [(u32, u32, u32); 3] = [(0, 0x10, 0x20), (0, 0x40, 0x42), (1, 0, 4)];

    fn img() -> Image<'static> {
        Image {
            bases: &BASES,
            sizes: &SIZES,
            code: &CODE,
        }
    }

    #[test]
    fn d_and_a_agree_with_get_at_ghidras_offsets() {
        let mut r = Regs::new();
        for n in 0..8 {
            r.set_d(n, 0x1000_0000 * n as u32 + 0x0102_0304);
            r.set_a(n, 0x8000_0000 | n as u32);
        }
        for n in 0..8 {
            assert_eq!(r.get(4 * n, 4), r.d(n) as u64, "D{n}");
            assert_eq!(r.get(0x20 + 4 * n, 4), r.a(n) as u64, "A{n}");
            // Ghidra's DnW is the low word at 4n+2, Dnb the low byte at 4n+3.
            assert_eq!(r.get(4 * n + 2, 2), 0x0304, "D{n}w");
            assert_eq!(r.get(4 * n + 3, 1), 0x04, "D{n}b");
        }
        assert_eq!(r.d(7), 0x7102_0304);
        assert_eq!(r.a(7), 0x8000_0007);
        assert_eq!(r.get(A0 + 28, 4), 0x8000_0007, "SP is A7");
        assert_eq!(r.space[0x20], 0x80, "big-endian");
    }

    #[test]
    fn a_byte_write_to_d0s_low_byte_leaves_the_rest() {
        let mut r = Regs::new();
        r.set_d(0, 0x1234_5678);
        r.set_d(1, 0xdead_beef);
        r.set(3, 1, 0xab);
        assert_eq!(r.d(0), 0x1234_56ab);
        assert_eq!(r.d(1), 0xdead_beef);
        // A word write to D0's low word, and junk above the size is not written.
        r.set(2, 2, 0xffff_0000_cafe);
        assert_eq!(r.d(0), 0x1234_cafe);
        r.set(CF, 1, 1);
        assert_eq!(r.get(CF, 1), 1);
        assert_eq!(r.get(NF, 4), 0x0000_0001, "NZVC are 0x44..0x48");
    }

    #[test]
    fn the_trail_keeps_each_instructions_cycles_off_the_bus_until_cleared() {
        let mut r = Regs::new();
        r.tick(24);
        r.off_bus(20);
        r.tick(12);
        assert_eq!(r.trail(), &[(24, 20), (12, 0)]);
        assert_eq!((r.cycles(), r.off_bus_cycles()), (36, 20));
        r.clear_trail();
        assert!(r.trail().is_empty());
        assert_eq!((r.cycles(), r.off_bus_cycles()), (36, 20));
        // Past TRAIL instructions the last entry takes the rest.
        for _ in 0..TRAIL + 3 {
            r.tick(4);
            r.off_bus(2);
        }
        assert_eq!(r.trail().len(), TRAIL);
        assert_eq!(r.trail()[TRAIL - 1], (16, 8));
    }

    #[test]
    fn tick_adds_to_the_counter_and_new_starts_at_zero() {
        let mut r = Regs::new();
        assert_eq!(r.cycles(), 0);
        r.tick(4);
        r.tick(
            const {
                cycles::of(
                    cycles::Op::Move,
                    cycles::Size::L,
                    cycles::Ea::Dn,
                    cycles::Ea::AbsL,
                )
            },
        );
        assert_eq!(r.cycles(), 24);
    }

    #[test]
    fn store_into_code_at_its_first_byte_last_byte_and_one_before() {
        let i = img();
        let mut ram = Ram::new();
        let first = i.h(0, 0x10);
        let last = i.h(0, 0x1f);
        let before = i.h(0, 0x0f);
        assert_eq!(
            store(&mut ram, &i, first, 1, 0x55),
            Err(Stop::WriteIntoCode("h0+$10".into()))
        );
        assert_eq!(
            store(&mut ram, &i, last, 1, 0x55),
            Err(Stop::WriteIntoCode("h0+$1f".into()))
        );
        assert_eq!(store(&mut ram, &i, before, 1, 0x55), Ok(()));
        assert_eq!(ram.writes, vec![(before, 1)], "two stops, one write");
        assert_eq!(ram.bytes[first as usize], 0);
        assert_eq!(ram.bytes[before as usize], 0x55);
    }

    #[test]
    fn a_wider_store_reaching_into_code_writes_nothing_and_names_the_code_byte() {
        let i = img();
        let mut ram = Ram::new();
        // A long at $0e covers $0e..$12, reaching two bytes into the range at $10.
        let r = store(&mut ram, &i, i.h(0, 0x0e), 4, 0x1122_3344);
        assert_eq!(r, Err(Stop::WriteIntoCode("h0+$10".into())));
        // One past the end of a range, and in the gap between two, are data.
        assert_eq!(store(&mut ram, &i, i.h(0, 0x20), 4, 1), Ok(()));
        assert_eq!(store(&mut ram, &i, i.h(0, 0x3e), 2, 1), Ok(()));
        assert_eq!(
            store(&mut ram, &i, i.h(0, 0x3f), 2, 1),
            Err(Stop::WriteIntoCode("h0+$40".into()))
        );
        // The other hunk's range, and a word straddling the end of hunk 1 into unloaded memory.
        assert_eq!(
            store(&mut ram, &i, i.h(1, 2), 2, 1),
            Err(Stop::WriteIntoCode("h1+$2".into()))
        );
        assert_eq!(store(&mut ram, &i, i.h(1, 0x7f), 2, 1), Ok(()));
        assert_eq!(
            store(&mut ram, &i, 0x0ffe, 4, 1),
            Err(Stop::WriteIntoCode("h1+$0".into()))
        );
        assert_eq!(ram.writes.len(), 3);
    }

    #[test]
    fn h_and_hunk_of_round_trip() {
        let i = img();
        for h in 0..2u32 {
            for off in [0, 1, SIZES[h as usize] - 1] {
                assert_eq!(i.hunk_of(i.h(h, off)), Some((h, off)));
            }
        }
        assert_eq!(i.h(0, 0x10), 0x3010);
        assert_eq!(i.h(1, 0x10), 0x1010);
        for outside in [0, 0x0fff, 0x1080, 0x2fff, 0x3100, u32::MAX] {
            assert_eq!(i.hunk_of(outside), None, "${outside:x}");
        }
        assert_eq!(i.describe(0x3010), "h0+$10");
        assert_eq!(i.describe(0x1080), "$001080");
        assert_eq!(i.describe(0xc0_77d8), "$c077d8");
    }

    #[test]
    fn loads_and_stores_are_big_endian_and_split_eight_bytes() {
        let i = img();
        let mut ram = Ram::new();
        store(&mut ram, &i, 0x100, 8, 0x0102_0304_0506_0708).unwrap();
        assert_eq!(ram.writes, vec![(0x100, 4), (0x104, 4)]);
        assert_eq!(ram.bytes[0x100], 1);
        assert_eq!(ram.bytes[0x107], 8);
        assert_eq!(load(&mut ram, 0x100, 8), 0x0102_0304_0506_0708);
        assert_eq!(load(&mut ram, 0x100, 4), 0x0102_0304);
        assert_eq!(load(&mut ram, 0x102, 2), 0x0304);
        assert_eq!(load(&mut ram, 0x107, 1), 0x08);
        assert_eq!(load(&mut ram, 0x101, 3), 0x02_0304);
        store(&mut ram, &i, 0x200, 2, 0xffff_beef).unwrap();
        assert_eq!(load(&mut ram, 0x200, 4), 0xbeef_0000);
        store(&mut ram, &i, 0x300, 3, 0xaa_bbcc).unwrap();
        assert_eq!(load(&mut ram, 0x300, 4), 0xaabb_cc00);
    }

    #[test]
    fn a_bus_trait_object_works_too() {
        let i = img();
        let mut ram = Ram::new();
        let bus: &mut dyn Bus = &mut ram;
        store(bus, &i, 0x400, 2, 0x1234).unwrap();
        assert_eq!(load(bus, 0x400, 2), 0x1234);
    }

    #[test]
    fn stops_read_as_the_design_names_them() {
        assert_eq!(
            Stop::Untranslated("h0+$5462".into()).to_string(),
            "untranslated: h0+$5462"
        );
        assert_eq!(
            Stop::WriteIntoCode("h2+$4".into()).to_string(),
            "write-into-code: h2+$4"
        );
        let u = Stop::Unsupported {
            op: "CALLOTHER".into(),
            at: "h0+$10".into(),
        };
        assert_eq!(u.to_string(), "unsupported: CALLOTHER at h0+$10");
        assert_eq!(Stop::Hang { why: Hang::Steps }.to_string(), "hang (steps)");
        assert_eq!(Stop::Hang { why: Hang::Depth }.to_string(), "hang (depth)");
        let t = Stop::Trap {
            vector: 5,
            at: "h0+$6c9c".into(),
        };
        assert_eq!(t.to_string(), "trap: vector 5 at h0+$6c9c");
        assert_eq!(
            Stop::Outside("$f80000".into()).to_string(),
            "outside-memory: $f80000"
        );
        assert_eq!(
            Stop::Os("unknown dos.library LVO -48 at h0+$10".into()).to_string(),
            "os: unknown dos.library LVO -48 at h0+$10"
        );
    }
}
