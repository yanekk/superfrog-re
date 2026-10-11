//! The sound driver (`sf-core::audio`, T11) against the original's own routines.
//!
//! Routine conformance: the driver's entry points are run on Moira (`Cpu68k`, hardware mode, so
//! their Paula writes are recorded and raster waits end) from the game's memory on the title
//! screen, and the Rust driver, transplanted from the same memory, is given the same commands. After
//! every command and every tick the Paula writes must be the same, in order, and so must every
//! driver variable (`AudioDriver::vars`). Full-system conformance: the title music on vAmiga for
//! 2 000 frames, the driver ticking alongside from a transplant, the same Paula writes each frame.
//!
//! Needs what sf-oracle's tests need: `tools/setup/oracle.sh` run, the four disks and Kickstart 1.3.
//! It lives here, not in `sf-core/tests/`, because the purity test keeps the emulator and the disk
//! files out of the pure crates' tests.

use std::sync::OnceLock;

use sf_core::audio::{AudioDriver, BANK_ADDR, BankId, File, H0, PaulaWrite, SoundData, TuneId};
use sf_disk::{DiskSet, Offered, identify};
use sf_oracle::*;

// The driver's entry points (docs/11-sound.md), as hunk-0 offsets.
const SFX_BANK_INIT: u32 = 0x12a26;
const SFX_PLAY: u32 = 0x12a72;
const SFX_TICK: u32 = 0x12c60;
const MUSIC_START: u32 = 0x13126;
const MUSIC_STOP: u32 = 0x132a0;
const MUSIC_TICK: u32 = 0x13424;
const MUSIC_DMA: u32 = 0x13cc4;
const STREAM_TICK: u32 = 0x147e4;
const SAMPLE_PLAY: u32 = 0x148b8;
const START_FADE: u32 = 0x146ae;
const FADE: u32 = H0 + 0x146e0;
const STREAM_TABLE: u32 = 0x1491e;
const ATTRACT_MODE: u32 = H0 + 0x18;
const MUSIC_ON: u32 = H0 + 0x13420;

/// The stack the calls run on: below the game's own, above hunk 1's end (`$C7E58C`).
const STACK: u32 = 0xC7_F800;
const STACK_AREA: std::ops::Range<u32> = 0xC7_E600..0xC7_F800;

/// The crack intro skips on a port-1 click (FINDINGS 2026-10-11).
const CLICK: std::ops::Range<u32> = 1_000..1_005;
const LAST_TITLE: u32 = 7_000;
const FULL_SYSTEM_FRAMES: u32 = 2_000;

fn sound() -> &'static (DiskSet, SoundData) {
    static S: OnceLock<(DiskSet, SoundData)> = OnceLock::new();
    S.get_or_init(|| {
        let files = sf_conform::disks::read_disk_files().unwrap_or_else(|e| panic!("{e}"));
        let offered: Vec<Offered> = files
            .into_iter()
            .map(|(name, bytes)| Offered { name, bytes })
            .collect();
        let disks = identify(&offered).unwrap_or_else(|e| panic!("{e:?}"));
        let data = SoundData::from_disks(&disks).unwrap_or_else(|e| panic!("{e}"));
        (disks, data)
    })
}

fn data() -> &'static SoundData {
    &sound().1
}

/// The title run: memory at the first frame with the title music on, then the oracle's Paula
/// writes for each of the next [`FULL_SYSTEM_FRAMES`] frames, and memory after the last of them.
struct Title {
    start: u32,
    memory: AmigaMemory,
    frames: Vec<Vec<PaulaWrite>>,
    end_memory: AmigaMemory,
}

fn title() -> &'static Title {
    static T: OnceLock<Title> = OnceLock::new();
    T.get_or_init(|| {
        let cfg = OracleConfig::locate(LAST_TITLE + FULL_SYSTEM_FRAMES + 1)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut a = Amiga::power_on(&cfg).unwrap();
        let mut f = 0;
        loop {
            let input = if CLICK.contains(&f) {
                FrameInput::joy(Joy::PORT1_BUTTON)
            } else {
                FrameInput::NONE
            };
            a.step(input).unwrap();
            f += 1;
            let m = a.memory();
            if f > CLICK.end && m.word(MUSIC_ON) == Some(1) {
                break;
            }
            assert!(f < LAST_TITLE, "no title music by frame {LAST_TITLE}");
        }
        let start = f;
        let memory = a.memory();
        let mut frames = Vec::new();
        for _ in 0..FULL_SYSTEM_FRAMES {
            frames.push(a.step(FrameInput::NONE).unwrap().paula);
        }
        Title {
            start,
            memory,
            frames,
            end_memory: a.memory(),
        }
    })
}

/// The original's routines on Moira, over a memory that keeps what each call wrote.
struct Original {
    cpu: Cpu68k,
}

impl Original {
    fn new(mem: &AmigaMemory) -> Original {
        let mut cpu = Cpu68k::new(mem.clone());
        cpu.set_hardware(true);
        cpu.write(ATTRACT_MODE, &[0; 4]);
        Original { cpu }
    }

    fn call(&mut self, entry: u32, d0: u32, d1: u32, a0: u32) -> Vec<(u16, u16)> {
        let mut regs = Regs::default();
        regs.d[0] = d0;
        regs.d[1] = d1;
        regs.a[0] = a0;
        regs.a[7] = STACK;
        let r = self
            .cpu
            .call(H0 + entry, regs, 5_000_000)
            .unwrap_or_else(|e| panic!("h0+${entry:x}: {e}"));
        self.cpu.apply(&r, |a| !STACK_AREA.contains(&a));
        r.hw_writes
    }

    fn byte(&self, a: u32) -> u8 {
        self.cpu.read(a, 1)[0]
    }

    fn load(&mut self, at: u32, bytes: &[u8]) {
        self.cpu.write(at, bytes);
    }

    fn long(&self, a: u32) -> u32 {
        u32::from_be_bytes(self.cpu.read(a, 4).try_into().unwrap())
    }

    /// One VBL's sound work, in `vbl_handler`'s order outside a level.
    fn tick(&mut self) -> Vec<(u16, u16)> {
        let mut w = self.call(STREAM_TICK, 0, 0, 0);
        w.extend(self.call(MUSIC_TICK, 0, 0, 0));
        w.extend(self.call(MUSIC_DMA, 0, 0, 0));
        w.extend(self.call(SFX_TICK, 0, 0, 0));
        w
    }
}

fn rv(w: &[PaulaWrite]) -> Vec<(u16, u16)> {
    w.iter().map(PaulaWrite::reg_value).collect()
}

/// Both sides, run in step; every step compares writes and variables.
struct Pair {
    orig: Original,
    ours: AudioDriver,
    files: Vec<(u32, File)>,
    what: String,
    steps: u32,
    /// Paula writes compared so far.
    writes: usize,
}

impl Pair {
    fn new(mem: &AmigaMemory, files: Vec<(u32, File)>) -> Pair {
        let orig = Original::new(mem);
        let ours = AudioDriver::transplant(data(), &|a| orig.byte(a), &files);
        let p = Pair {
            orig,
            ours,
            files,
            what: String::from("transplant"),
            steps: 0,
            writes: 0,
        };
        p.check_vars();
        p
    }

    /// Replace our driver with a fresh transplant of the original's current state.
    fn retransplant(&mut self) {
        let orig = &self.orig;
        self.ours = AudioDriver::transplant(data(), &|a| orig.byte(a), &self.files);
        self.check_vars();
    }

    fn check(&mut self, what: &str, theirs: Vec<(u16, u16)>, ours: Vec<PaulaWrite>) {
        self.what = what.to_string();
        let ours = rv(&ours);
        if theirs != ours {
            let i = theirs.iter().zip(&ours).take_while(|(a, b)| a == b).count();
            panic!(
                "{} (step {}): Paula writes differ at write {i}\n theirs: {:x?}\n ours:   {:x?}",
                self.what,
                self.steps,
                &theirs[i.saturating_sub(2)..theirs.len().min(i + 6)],
                &ours[i.saturating_sub(2)..ours.len().min(i + 6)],
            );
        }
        self.check_vars();
        self.steps += 1;
        self.writes += ours.len();
    }

    fn check_vars(&self) {
        for (at, bytes) in self.ours.vars(data()) {
            for (i, &b) in bytes.iter().enumerate() {
                let a = at + i as u32;
                let theirs = self.orig.byte(a);
                assert_eq!(
                    b, theirs,
                    "{} (step {}): driver variable at ${a:06x} (block ${at:06x}+{i:#x}) is {b:#04x}, the original's {theirs:#04x}",
                    self.what, self.steps
                );
            }
        }
    }

    fn tick(&mut self) {
        let theirs = self.orig.tick();
        let mut ours = Vec::new();
        self.ours.tick(data(), &mut ours);
        self.check("tick", theirs, ours);
    }

    fn music_stop(&mut self) {
        let theirs = self.orig.call(MUSIC_STOP, 0, 0, 0);
        let mut ours = Vec::new();
        self.ours.stop_music(data(), &mut ours);
        self.check("music_stop", theirs, ours);
    }

    fn load_bank(&mut self, b: BankId) {
        self.orig.load(BANK_ADDR, data().bank(b));
        self.files.push((BANK_ADDR, File::Bank(b)));
        self.ours.file_loaded(data(), BANK_ADDR, File::Bank(b));
        let theirs = self.orig.call(SFX_BANK_INIT, 0, 0, 0);
        self.ours.sfx_bank_init(data());
        self.check(&format!("sfx_bank_init {b:?}"), theirs, Vec::new());
    }

    fn play_music(&mut self, t: TuneId) {
        let at = self.orig.long(H0 + 0x9ec);
        assert_eq!(at, self.ours.module_addr());
        self.orig.load(at, data().tune(t));
        self.files.push((at, File::Tune(t)));
        let theirs = self.orig.call(MUSIC_START, 0, 0, 0);
        let mut ours = Vec::new();
        self.ours.play_music(data(), t, &mut ours);
        self.check(&format!("play_music {t:?}"), theirs, ours);
    }

    fn play_sfx(&mut self, id: u8, chan: u8) {
        let theirs = self.orig.call(SFX_PLAY, id as u32, chan as u32, 0);
        let mut ours = Vec::new();
        self.ours.play_sfx(data(), id, chan, &mut ours);
        self.check(&format!("play_sfx {id} on {chan}"), theirs, ours);
    }

    fn play_sample(&mut self, k: usize) {
        let theirs = self
            .orig
            .call(SAMPLE_PLAY, 0, 0, H0 + STREAM_TABLE + k as u32 * 16);
        let mut ours = Vec::new();
        self.ours.play_sample(data(), k, &mut ours);
        self.check(&format!("play_sample {k}"), theirs, ours);
    }

    fn start_fade(&mut self, word: u16) {
        let theirs = self.orig.call(START_FADE, word as u32, 0, 0);
        self.ours.start_fade(word);
        self.check(&format!("start_fade {word:#x}"), theirs, Vec::new());
    }

    /// `music_fade_wait` (`$1327a`) without its frame waits: the fade word, the frames (up to 100)
    /// until the music stops, then the `music_stop` it falls into.
    fn music_fade_wait(&mut self) {
        self.orig.load(FADE, &[0x20, 0x01]);
        self.ours.fade_out();
        self.check("music_fade_wait", Vec::new(), Vec::new());
        for _ in 0..100 {
            if self.orig.byte(MUSIC_ON + 1) == 0 {
                break;
            }
            self.tick();
        }
        self.music_stop();
    }

    fn sfx_busy(&self, chan: u8) -> bool {
        let rec = [0x12f62, 0x12f88, 0x12fbc, 0x12ff0][chan as usize];
        self.orig.byte(H0 + rec) != 0 || self.orig.byte(H0 + rec + 1) != 0
    }
}

/// The bank each tune is heard with.
fn bank_for(t: TuneId) -> Option<BankId> {
    match t {
        TuneId::Title => None,
        TuneId::World(1 | 2) => Some(BankId::Worlds12),
        TuneId::World(_) => Some(BankId::Worlds36),
        TuneId::Bonus => Some(BankId::Bonus),
        TuneId::Slot => Some(BankId::Slot),
    }
}

/// From the title screen: the title music stopped, `t`'s bank and module loaded as the game loads
/// them, `t` started.
fn start_tune(t: TuneId) -> Pair {
    let mut p = Pair::new(
        &title().memory,
        vec![(BANK_ADDR, File::Tune(TuneId::Title))],
    );
    p.music_stop();
    match bank_for(t) {
        Some(b) => p.load_bank(b),
        None => {
            // The title points the module at the bank's place itself.
            p.orig.load(H0 + 0x9ec, &BANK_ADDR.to_be_bytes());
            p.ours.set_module_addr(BANK_ADDR);
        }
    }
    p.play_music(t);
    p
}

/// A small deterministic generator for the test's choices.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) % n as u64) as u32
    }
}

#[test]
fn every_tune_and_effect_decodes() {
    let d = data();
    for t in TuneId::ALL {
        let m = d.tune(t);
        assert_eq!(&m[..4], b"P41A", "{t:?}");
        let samples = m[6];
        assert!((1..=31).contains(&samples), "{t:?}: {samples} instruments");
    }
    for b in BankId::ALL {
        let bank = d.bank(b);
        // 256 effect sizes, then the samples up to $ffffffff, then the effects they size.
        let sizes: u32 = (0..256)
            .map(|i| u32::from_be_bytes(bank[i * 4..i * 4 + 4].try_into().unwrap()))
            .sum();
        let mut at = 0x400usize;
        let mut n = 0;
        loop {
            let len = u32::from_be_bytes(bank[at..at + 4].try_into().unwrap()) as usize;
            at += 4 + 2 * len;
            n += 1;
            if bank[at..at + 4] == [0xff; 4] {
                break;
            }
        }
        assert!(n > 0 && at + 4 + sizes as usize <= bank.len(), "{b:?}");
    }
    assert_eq!(d.stream.len(), 8);
    for s in &d.stream {
        assert!(s.len > 0 && s.volume <= 64, "{s:?}");
    }
}

/// Every tune from its start for 3 000 ticks; 50 times along the way our driver is replaced by a
/// fresh transplant of the original's state, which must carry on identically.
#[test]
fn every_tune_matches_the_original_replayer() {
    let mut rng = Lcg(11);
    let mut transplants = 0;
    for (k, t) in TuneId::ALL.into_iter().enumerate() {
        let mut p = start_tune(t);
        assert!(p.ours.music_on(), "{t:?} did not start");
        // 50 transplants over 9 tunes: 6 for the first five, 5 for the rest.
        let n = if k < 5 { 6 } else { 5 };
        let mut at: Vec<u32> = (0..n).map(|_| rng.next(3_000)).collect();
        at.sort_unstable();
        for i in 0..3_000 {
            if at.contains(&i) {
                p.retransplant();
                transplants += 1;
            }
            p.tick();
        }
        // Five registers a channel a tick, once the notes start.
        assert!(p.writes > 3_000 * 15, "{t:?}: only {} writes", p.writes);
    }
    assert_eq!(transplants, 50);
}

/// Every effect of every bank alone, with the music stopped, on a channel chosen by its number.
#[test]
fn every_effect_alone_matches_the_original() {
    for b in BankId::ALL {
        let mut p = Pair::new(
            &title().memory,
            vec![(BANK_ADDR, File::Tune(TuneId::Title))],
        );
        p.music_stop();
        p.load_bank(b);
        let bank = data().bank(b);
        let mut played = 0;
        for id in 0..=255u8 {
            let size = u32::from_be_bytes(
                bank[id as usize * 4..id as usize * 4 + 4]
                    .try_into()
                    .unwrap(),
            );
            if size == 0 {
                continue;
            }
            let chan = id % 4;
            p.play_sfx(id, chan);
            let mut ticks = 0;
            while p.sfx_busy(chan) && ticks < 600 {
                p.tick();
                ticks += 1;
            }
            p.tick();
            played += 1;
        }
        assert!(played > 0, "{b:?} has no effects");
    }
}

/// Two effects on one channel with different priorities, both orders; an effect on channel 3
/// while a sample streams; each streamed sample.
#[test]
fn overlapping_effects_and_streams_match_the_original() {
    let mut p = Pair::new(
        &title().memory,
        vec![(BANK_ADDR, File::Tune(TuneId::Title))],
    );
    p.music_stop();
    p.load_bank(BankId::Worlds12);
    let bank = data().bank(BankId::Worlds12);
    let effects: Vec<u8> = (0..=255u8)
        .filter(|&i| bank[i as usize * 4..i as usize * 4 + 4] != [0; 4])
        .collect();
    let prio = |p: &Pair, id: u8| {
        let e = p.ours.vars(data())[5].1[0x400 + id as usize * 4..][..4].to_vec();
        let e = u32::from_be_bytes(e.try_into().unwrap());
        u16::from_be_bytes([p.orig.byte(e + 2), p.orig.byte(e + 3)])
    };
    let mut pairs = 0;
    for &a in &effects {
        for &b in &effects {
            if pairs >= 12 || prio(&p, a) == prio(&p, b) {
                continue;
            }
            for (x, y) in [(a, b), (b, a)] {
                p.play_sfx(x, 1);
                for _ in 0..3 {
                    p.tick();
                }
                p.play_sfx(y, 1);
                for _ in 0..200 {
                    p.tick();
                }
            }
            pairs += 1;
        }
    }
    assert!(pairs > 0, "no two effects with different priorities");
    for k in 0..8 {
        p.play_sample(k);
        p.tick();
        p.play_sfx(effects[k % effects.len()], 3);
        for _ in 0..400 {
            p.tick();
        }
    }
}

/// Effects on every channel over a world's music.
#[test]
fn effects_during_music_match_the_original() {
    let mut rng = Lcg(7);
    for t in [TuneId::World(1), TuneId::World(4), TuneId::Bonus] {
        let mut p = start_tune(t);
        let b = bank_for(t).unwrap();
        let bank = data().bank(b);
        let effects: Vec<u8> = (0..=255u8)
            .filter(|&i| bank[i as usize * 4..i as usize * 4 + 4] != [0; 4])
            .collect();
        for i in 0..2_000 {
            if rng.next(20) == 0 {
                let e = effects[rng.next(effects.len() as u32) as usize];
                p.play_sfx(e, rng.next(4) as u8);
            }
            if i % 500 == 250 {
                p.play_sample(rng.next(8) as usize);
            }
            p.tick();
        }
    }
}

/// The fades: `music_fade_wait`'s over a world's music, and `$146ae`'s with slower speeds and
/// higher ceilings, with an effect holding a channel when it starts.
#[test]
fn fades_match_the_original() {
    let mut fades = 0;
    for (t, word) in [
        (TuneId::World(2), None),
        (TuneId::World(5), Some(0x4002)),
        (TuneId::Bonus, Some(0x7f00)),
        (TuneId::Title, Some(0x1003)),
    ] {
        let mut p = start_tune(t);
        for _ in 0..300 {
            p.tick();
        }
        if let Some(b) = bank_for(t) {
            let bank = data().bank(b);
            let e = (0..=255u8)
                .find(|&i| bank[i as usize * 4..i as usize * 4 + 4] != [0; 4])
                .unwrap();
            p.play_sfx(e, 2);
            p.tick();
        }
        match word {
            None => p.music_fade_wait(),
            Some(w) => {
                p.start_fade(w);
                let mut ticks = 0;
                while p.ours.music_on() {
                    p.tick();
                    ticks += 1;
                    assert!(ticks < 20_000, "{t:?}: the fade {w:#x} never ended");
                }
            }
        }
        assert!(!p.ours.music_on(), "{t:?}");
        for _ in 0..10 {
            p.tick();
        }
        fades += 1;
    }
    assert_eq!(fades, 4);
}

/// The title music on the full-system reference: the driver, transplanted at the first frame of
/// the music, makes the same Paula writes in each of the next 2 000 frames.
#[test]
fn title_music_matches_the_full_system_oracle() {
    let t = title();
    let mem = &t.memory;
    let read = |a: u32| mem.byte(a).unwrap_or(0);
    let files = [(BANK_ADDR, File::Tune(TuneId::Title))];
    let mut ours = AudioDriver::transplant(data(), &read, &files);
    // The game's other code writes DMACON too (sprites, blitter priority); the driver's DMACON
    // writes touch the audio bits only. INTENA is left out: the main loop's own $4000/$c000 pairs
    // (around disk loading) fall between the driver's, and the routine tests above check the
    // driver's INTENA writes exactly.
    let sound = |(r, v): (u16, u16)| (r == 0x96 && v & 0x7ff0 == 0) || (0xa0..=0xde).contains(&r);
    let mut compared = 0;
    for (i, theirs) in t.frames.iter().enumerate() {
        let theirs: Vec<(u16, u16)> = theirs
            .iter()
            .map(PaulaWrite::reg_value)
            .filter(|&w| sound(w))
            .collect();
        let mut w = Vec::new();
        ours.tick(data(), &mut w);
        let w: Vec<PaulaWrite> = w.into_iter().filter(|w| sound(w.reg_value())).collect();
        assert_eq!(
            rv(&w),
            theirs,
            "frame {} (title music frame {i}): Paula writes differ",
            t.start + 1 + i as u32
        );
        compared += w.len();
    }
    assert!(
        compared > FULL_SYSTEM_FRAMES as usize * 15,
        "only {compared} writes compared"
    );
    let end = &t.end_memory;
    for (at, bytes) in ours.vars(data()) {
        for (i, &b) in bytes.iter().enumerate() {
            let a = at + i as u32;
            assert_eq!(
                Some(b),
                end.byte(a),
                "after {FULL_SYSTEM_FRAMES} frames: driver variable ${a:06x}"
            );
        }
    }
}
