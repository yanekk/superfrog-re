//! The end sequence (T35, docs/13 "The end sequence"), after the final level: `end_seq_load`
//! (h0+$a65c) loads `END1` and `END2` from disk 3 and the title's music, and `end_sequence`
//! (h0+$a6e4) unpacks each into hunk 3 and plays it as an IFF ANIM ([`super::anim`]) with fades and
//! timed holds, then clears hunks 1 and 3, reloads the frog's sprites and reseeds the RNG before
//! `top_loop` goes back to the title.
//!
//! Unpacking and drawing an ANIM frame are CPU work the original spreads over frames: the engine
//! does each at once and spends its 68000 cycles on the beam ([`CpuWork`]), which decides on which
//! frame the next picture shows.

use super::anim::{self, BUFFER_A, BUFFER_B, BUFFER_LEN, FILE, RECORD, RECORD_LEN};
use super::common::{FadeKind, MusicFadeWait, MusicStart, call, fade, frames};
use super::fade::{self, RECORDS};
use super::title::{BLANK_COPPER, OVERLAY_PALETTE, OverlayEnd};
use super::{BEAM_CYCLES_PER_LINE, Cx, FRAME_LINES, Routine, Yield, at, h2};
use crate::game::{Job, LoadEnd};
use crate::layout::{VarId, def, h1, h3};

/// `logic_running` (h0+$9d6), cleared as the end sequence starts.
const LOGIC_RUNNING: u32 = at(0x9d6);
/// `loader_mode` (h0+$8): 0 with no trackloader, when no overlay is shown.
const LOADER_MODE: u32 = at(0x8);
/// The end sequence's copper list, and the first of its 32 colour MOVEs.
pub const END_COPPER: u32 = h2(0x5efc);
const END_PALETTE: u32 = h2(0x5fa4);
/// The palette the pictures fade to (h0+$aa80) and the white the last one flashes from ($aa40).
const END_TARGET: u32 = at(0xaa80);
const END_WHITE: u32 = at(0xaa40);
/// `END2` is loaded packed to h1+$13880 (`file_table` entry 1); its tail lies over the level's map.
const END2_PACKED: u32 = h1(0x13880);
/// `password_buf` (h0+$242a), six spaces once a game is over.
const PASSWORD_BUF: u32 = at(0x242a);
/// Hunk 1 and hunk 3 as `end_sequence` clears them: `$d4a8` and `$192dc` longs.
const HUNK1_CLEARED: u32 = 0xd4a8 * 4;
const HUNK3_CLEARED: u32 = 0x192dc * 4;

/// 68000 cycles of `atn_decompress` on `END1` and on `END2` (measured on the oracle: see
/// docs/13 "The end sequence").
const END1_UNPACK_CYCLES: u32 = 24_080_000;
const END2_UNPACK_CYCLES: u32 = 21_046_000;
/// A clear loop's long: `clr.l (a0)+`, `subq.l #1,d0`, `bne.b`.
const CLEAR_LONG_CYCLES: u32 = 20 + 8 + 10;
/// Calibrated on the oracle against all 129 of the two files' frame decodes (docs/13 "The end
/// sequence"): the line from which the main program has the CPU again after the vertical blank
/// (the sound driver's work), and the CPU cycles a lowres fetch past the fourth plane takes from
/// it, in tenths of a cycle a word fetched.
const MAIN_FROM: u16 = 22;
const STOLEN_TENTHS_PER_WORD: u32 = 36;
/// What comes before a frame's decode after a wait: `wait_frame_raster`'s end, `$aac0`/`$aaca`
/// showing the last frame and the call (calibrated with the above).
const SHOW_CYCLES: u32 = 600;

/// CPU work the original's main program does across frames: the engine has done it at once; the
/// beam moves on by its cycles, which the main program gets from the end of each vertical blank,
/// fewer on lines the display fetches more than four planes. Its memory is compared once the
/// original is out of it (`job`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct CpuWork {
    pub cycles: u32,
    job: Option<Job>,
}

impl CpuWork {
    pub fn new(cycles: u32, inside: &[u32], writes: Vec<(u32, u32)>) -> CpuWork {
        CpuWork {
            cycles,
            job: Some(Job {
                inside: inside.to_vec(),
                writes,
            }),
        }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        let resumed = self.job.is_none();
        if let Some(j) = self.job.take() {
            cx.jobs.push(j);
        }
        if resumed {
            cx.line = cx.line.max(MAIN_FROM);
        }
        let stolen = match cx.fetch.planes() {
            p if p > 4 && !cx.fetch.hires() => {
                (p as u32 - 4) * cx.fetch.words() as u32 * STOLEN_TENTHS_PER_WORD / 10
            }
            _ => 0,
        };
        let shown = cx.fetch.lines();
        while self.cycles > 0 {
            if cx.line >= FRAME_LINES - 1 {
                // Past the frame's end: on after the next vertical blank.
                cx.line = FRAME_LINES - 1;
                return Yield::Poll;
            }
            let free = BEAM_CYCLES_PER_LINE - if shown.contains(&cx.line) { stolen } else { 0 };
            self.cycles = self.cycles.saturating_sub(free);
            cx.line += 1;
        }
        Yield::Return
    }
}

pub(crate) fn work(cycles: u32, inside: &[u32], writes: Vec<(u32, u32)>) -> Yield {
    call(Routine::CpuWork(CpuWork::new(cycles, inside, writes)))
}

/// `end_seq_load` (h0+$a65c): the display off, the Accessing-disk overlay up, `END1` and `END2`
/// loaded packed to hunk 1, the title's music loaded and started, the overlay faded out. The loads
/// are instant (owner, 2026-10-12); the original's frames until its `overlays_end` are skipped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct EndSeqLoad {
    pc: u8,
}

impl EndSeqLoad {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        self.pc += 1;
        match self.pc {
            1 => {
                cx.s.set_w(LOGIC_RUNNING, 0);
                cx.cop1lc = BLANK_COPPER;
                if cx.s.l(at(0x445a)) == 0
                    && cx.s.l(OVERLAY_PALETTE) == 0
                    && cx.s.w(LOADER_MODE) != 0
                {
                    super::title::overlay_show(cx);
                }
                // END1, END2 (disk 3), then $1d6c's title music, MUSC (disk 1).
                cx.read_entries(0xa65c, 0xa6e4);
                cx.read_entries(0x1d6c, 0x1d90);
                // The files are kept packed in hunk 1, which the engine does not model; `END2`'s
                // tail over the level's map is taken from the original.
                let sectors = cx.data.h0_w(0xcbfe + 6 + 2) as u32;
                cx.resync.push((END2_PACKED, sectors * 512));
                // music_fade_wait: the music is off already (top_loop faded it out).
                if let Some(d) = cx.data.sound.as_ref() {
                    cx.s.audio.fade_out();
                    let mut quiet = Vec::new();
                    cx.s.audio.stop_music(d, &mut quiet);
                    // $1d6c: the title's music to h3+$64c70.
                    cx.s.audio.file_loaded(
                        d,
                        sf_assets::sound::BANK_ADDR,
                        crate::audio::File::Tune(sf_assets::sound::TuneId::Title),
                    );
                }
                // Over once the original waits on the overlay's fade-out (`overlays_end`); the
                // line its load ended on decides where the music's start and the fade fall.
                cx.loaded(LoadEnd::Reached(at(0xb7e2)));
                call(Routine::MusicStart(MusicStart::returning_to(at(0xa6d2))))
            }
            2 => call(Routine::Overlay(OverlayEnd::default())),
            _ => {
                cx.cop1lc = BLANK_COPPER;
                Yield::Return
            }
        }
    }
}

/// Which file a play of [`EndSequence`] is in, with what differs between `$a890` and `$a95e`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Play {
    /// The calls' return addresses: `bsr $a890`/`$a95e`'s open, `$ab82` in each half.
    open: u32,
    halves: [u32; 2],
    fade_in: u32,
    /// Frames the first picture holds, passes of the loop, frames each picture shows.
    hold: u16,
    passes: u16,
    rasters: u16,
    /// `END2`'s loop ends after the first half of its last pass.
    short_last: bool,
}

const END1: Play = Play {
    open: at(0xa8b4),
    halves: [at(0xa926), at(0xa944)],
    fade_in: at(0xa8ee),
    hold: 0xc8,
    passes: 0x36,
    rasters: 2,
    short_last: false,
};

const END2: Play = Play {
    open: at(0xa982),
    halves: [at(0xa9f4), at(0xaa20)],
    fade_in: at(0xa9bc),
    hold: 0x64,
    passes: 0xb,
    rasters: 3,
    short_last: true,
};

/// `end_sequence` (h0+$a6e4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct EndSequence {
    pc: u16,
    /// `d7`: passes left; and rasters left in a wait between pictures.
    d7: u16,
    waits: u16,
    /// 0 while `END1` plays, 1 for `END2`.
    file: u8,
}

/// The fade record the end sequence uses, and setting its speed to 1 before waiting on it.
fn wait_fade(cx: &mut Cx) -> Yield {
    cx.s.set_w(RECORDS[0].at + fade::SPEED, 1);
    Yield::Fade(RECORDS[0])
}

impl EndSequence {
    fn play(&self) -> Play {
        if self.file == 0 { END1 } else { END2 }
    }

    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        loop {
            let pc = self.pc;
            self.pc += 1;
            match pc {
                // atn_decompress of the file into hunk 3.
                0 | 40 => {
                    let (name, cycles, inside) = if pc == 0 {
                        ("END1", END1_UNPACK_CYCLES, at(0xa6f6))
                    } else {
                        ("END2", END2_UNPACK_CYCLES, at(0xa75a))
                    };
                    let bytes = cx.data.file(name).to_vec();
                    cx.s.write(FILE, &bytes);
                    return work(cycles, &[inside], vec![(FILE, bytes.len() as u32)]);
                }
                // $a890 / $a95e: open it, show buffer A, fade it in.
                1 | 41 => {
                    self.file = (pc == 41) as u8;
                    let cycles = anim::open(cx.s);
                    return work(
                        cycles,
                        &[self.play().open],
                        vec![
                            (BUFFER_A, BUFFER_LEN),
                            (BUFFER_B, BUFFER_LEN),
                            (RECORD, RECORD_LEN),
                        ],
                    );
                }
                2 | 42 => {
                    show(cx, BUFFER_A);
                    return Yield::Raster;
                }
                3 | 43 => return wait_fade(cx),
                4 | 44 => {
                    return fade(
                        FadeKind::In { target: END_TARGET },
                        0,
                        END_PALETTE,
                        0x20,
                        self.play().fade_in,
                    );
                }
                5 | 45 => return Yield::Raster,
                6 | 46 => {
                    cx.cop1lc = END_COPPER;
                    return Yield::Fade(RECORDS[0]);
                }
                7 | 47 => return frames(self.play().hold),
                8 | 48 => self.d7 = self.play().passes,
                // A pass: the next frame into B, shown after the waits, the next into A.
                9 | 49 | 13 | 53 => {
                    let half = (pc % 40 == 13) as usize;
                    let (end, cycles) = anim::next(cx.s);
                    if end {
                        self.pc = if self.file == 0 { 20 } else { 60 };
                        continue;
                    }
                    let hidden = if half == 0 { BUFFER_B } else { BUFFER_A };
                    self.waits = self.play().rasters;
                    // The loop's first decode follows the hold, every other one a picture shown.
                    let first = half == 0 && self.d7 == self.play().passes;
                    return work(
                        cycles + if first { 0 } else { SHOW_CYCLES },
                        &[self.play().halves[half]],
                        vec![(hidden, BUFFER_LEN), (RECORD, RECORD_LEN)],
                    );
                }
                10 | 50 | 14 | 54 => {
                    if self.waits > 0 {
                        self.waits -= 1;
                        self.pc = pc;
                        return Yield::Raster;
                    }
                }
                11 | 51 => {
                    show(cx, BUFFER_B);
                    if self.play().short_last && self.d7 == 1 {
                        self.pc = if self.file == 0 { 20 } else { 60 };
                    }
                }
                12 | 52 => {}
                15 | 55 => {
                    show(cx, BUFFER_A);
                    self.d7 -= 1;
                    if self.d7 != 0 {
                        self.pc = pc - 6;
                    } else {
                        self.pc = if self.file == 0 { 20 } else { 60 };
                    }
                }
                // After END1: hold, fade out.
                20 => return frames(0x64),
                21 => return Yield::Raster,
                22 => return wait_fade(cx),
                23 => return fade(FadeKind::Out, 0, END_PALETTE, 0x20, at(0xa73c)),
                24 => {
                    self.pc = 40;
                    return Yield::Fade(RECORDS[0]);
                }
                // After END2: hold, a white flash fading to the last frame, hold, fade out.
                60 => return frames(0x3c),
                61 => {
                    for k in 0..0x20 {
                        cx.s.set_w(END_PALETTE + 2 + 4 * k, 0xfff);
                    }
                    let (_, cycles) = anim::next(cx.s);
                    return work(
                        cycles,
                        &[at(0xa78a)],
                        vec![
                            (BUFFER_A, BUFFER_LEN),
                            (BUFFER_B, BUFFER_LEN),
                            (RECORD, RECORD_LEN),
                        ],
                    );
                }
                62 => return wait_fade(cx),
                63 => {
                    return fade(
                        FadeKind::Step {
                            from: END_WHITE,
                            target: END_TARGET,
                        },
                        0,
                        END_PALETTE,
                        0x20,
                        at(0xa7c0),
                    );
                }
                64 => {
                    show(cx, BUFFER_A);
                    return wait_fade(cx);
                }
                65 => return frames(0x1c2),
                66 => return Yield::Raster,
                67 => return wait_fade(cx),
                68 => return fade(FadeKind::Out, 0, END_PALETTE, 0x20, at(0xa818)),
                69 => return Yield::Fade(RECORDS[0]),
                70 => {
                    cx.cop1lc = BLANK_COPPER;
                    return Yield::Raster;
                }
                // Hunks 1 and 3 cleared, a long at a time (of hunk 1 the engine holds the
                // variables in it: the map, the level's palette); the loops are inline, so the
                // original is inside one while its PC is.
                71 => {
                    let cleared = h1(0)..h1(HUNK1_CLEARED);
                    let mut runs = Vec::new();
                    for d in crate::layout::LAYOUT.iter().filter(|d| cleared.contains(&d.addr)) {
                        cx.s.var_mut(d.id).fill(0);
                        runs.push((d.addr, d.width.len()));
                    }
                    return work(
                        (HUNK1_CLEARED / 4) * CLEAR_LONG_CYCLES,
                        &[at(0xa840), at(0xa842), at(0xa844)],
                        runs,
                    );
                }
                72 => {
                    cx.s.write(h3(0), &vec![0; HUNK3_CLEARED as usize]);
                    return work(
                        (HUNK3_CLEARED / 4) * CLEAR_LONG_CYCLES,
                        &[at(0xa852), at(0xa854), at(0xa856)],
                        vec![(h3(0), HUNK3_CLEARED)],
                    );
                }
                73 => return call(Routine::MusicFadeWait(MusicFadeWait::default())),
                // load_hero (a load: instant), then rng_seed_from_cia and the level code's entry
                // emptied.
                74 => {
                    super::hero::load_hero(cx);
                    cx.loaded(LoadEnd::Left(at(0xa86a)));
                    super::title::level_tables(cx);
                    // The seed is the CIA's time-of-day clock, which the engine does not keep:
                    // normal play goes on with the RNG as it is; conformance takes the original's.
                    let rng = def(VarId::RngState);
                    cx.resync.push((rng.addr, rng.width.len()));
                    cx.s.write(PASSWORD_BUF, b"      ");
                    return Yield::Return;
                }
                _ => unreachable!("EndSequence pc {pc}"),
            }
        }
    }
}

/// `$aac0` / `$aaca`: the end sequence's copper list shows `buffer` from the next frame.
fn show(cx: &mut Cx, buffer: u32) {
    anim::show_pointers(cx.s, buffer);
    cx.cop1lc = END_COPPER;
}

/// The end sequence's routines on the original's call stack, when its main program is in
/// `end_seq_load` (top_loop's call of it, h0+$7b6 on the stack): the end sequence's start.
pub(super) fn resume(chain: &[u32]) -> Option<Vec<(Routine, Option<Yield>)>> {
    chain.contains(&at(0x7b6)).then(|| {
        vec![
            (
                Routine::TopLoop(super::attract::TopLoop::after_end_seq_load()),
                None,
            ),
            (Routine::EndSeqLoad(EndSeqLoad::default()), None),
        ]
    })
}
