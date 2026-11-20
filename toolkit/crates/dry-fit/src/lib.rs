//! The dry fit: one frame of a made-up game, built from toolkit crates alone (toolkit-extract T11).
//!
//! A game's engine on this toolkit does, each frame, what [`run`] does once: the blitter draws a
//! pattern into a bitplane in chip memory, the copper runs a list that sets up a one-plane lores
//! screen and changes a colour part way down it, the display piece draws the frame Denise would show
//! from that memory and those writes, the beam clock steps across the frame, and Paula plays a note
//! from a sample in the same memory. Every address and value here is made up for the fit; none comes
//! from a game.

use amiga_chips::copper::{self, Fetch, Write, reg};
use amiga_chips::{AmigaMemory, Blit, BlitMem, Picture, Ram, display};
use amiga_clock::{Clock, DisplayTiming, LINE_CYCLES, Timing};
use amiga_paula::{ChipMemory, Paula, PaulaWrite};

/// Chip RAM the fit uses: 64 KB is plenty for a list, a sample, a pattern and one plane.
pub const CHIP_SIZE: usize = 0x1_0000;
/// The sample Paula plays.
pub const SAMPLE: u32 = 0x0100;
/// The copper list.
pub const COPPER: u32 = 0x0200;
/// The pattern the blitter copies: one word a row, [`PATTERN_ROWS`] rows.
pub const PATTERN: u32 = 0x1000;
/// The screen's one bitplane: 320 × 256 lores, 40 bytes a row.
pub const SCREEN: u32 = 0x4000;
/// Bytes in one row of the screen.
pub const ROW_BYTES: u32 = 40;

/// The pattern's rows, a cross 16 pixels wide.
pub const PATTERN_ROWS: [u16; 8] = [
    0x8001, 0x4002, 0x2004, 0x1008, 0x0810, 0x0420, 0x0240, 0x0180,
];
/// Where the blit puts the pattern: row [`BLIT_Y`] of the screen, from pixel [`BLIT_X`], which is
/// not on a word boundary, so the blitter's shifter is used.
pub const BLIT_Y: u32 = 10;
pub const BLIT_X: u32 = 36;

/// The display window's first raster line and left edge, and the screen's height.
pub const TOP: u16 = 0x2c;
pub const LEFT: i32 = 0x81;
pub const HEIGHT: u16 = 256;

/// Colours the copper sets: the background, the pattern above [`SPLIT_LINE`] and from it on.
pub const BACKGROUND: u16 = 0x002;
pub const ABOVE: u16 = 0xF00;
pub const BELOW: u16 = 0x0F0;
/// The raster line the copper waits for before changing colour 1, inside the pattern's rows.
pub const SPLIT_LINE: u16 = TOP + BLIT_Y as u16 + 4;
/// A colour 0 the copper writes in the middle of [`MID_LINE`], from DMA slot [`MID_H`] on.
pub const MID_COLOUR: u16 = 0x00F;
pub const MID_LINE: u16 = TOP + 100;
pub const MID_H: u16 = 0x80;

/// Paula's note: the sample's length in words, its period in Paula clocks and its volume.
pub const SAMPLE_WORDS: u16 = 4;
pub const PERIOD: u16 = 200;
pub const VOLUME: u16 = 64;
/// The host rate the note is mixed to.
pub const HOST_RATE: u32 = 44_100;

/// The machine's memory as each chip reads it.
pub struct Machine {
    pub mem: AmigaMemory,
}

impl Machine {
    pub fn new() -> Machine {
        Machine {
            mem: AmigaMemory {
                chip: vec![0; CHIP_SIZE],
                slow: Vec::new(),
            },
        }
    }

    pub fn put_word(&mut self, addr: u32, w: u16) {
        let a = addr as usize;
        self.mem.chip[a..a + 2].copy_from_slice(&w.to_be_bytes());
    }

    pub fn word(&self, addr: u32) -> u16 {
        self.mem.word(addr).unwrap_or(0)
    }
}

impl Default for Machine {
    fn default() -> Machine {
        Machine::new()
    }
}

impl BlitMem for Machine {
    fn blit_w(&self, addr: u32) -> u16 {
        self.word(addr)
    }

    fn blit_set_w(&mut self, addr: u32, v: u16) {
        self.put_word(addr, v);
    }
}

impl Ram for Machine {
    fn byte(&self, addr: u32) -> u8 {
        self.mem.byte(addr)
    }
}

impl ChipMemory for Machine {
    fn chip_word(&self, addr: u32) -> u16 {
        self.mem.chip_word(addr)
    }
}

/// The frame this stand-in game runs: a long PAL frame, supplied here as any game's is.
pub const FRAME_LINES: u16 = 313;

/// The screen's timing for the beam clock: one lores plane fetched on the window's lines. One to
/// four lores planes take only odd DMA slots, so they take nothing from the CPU and leave the
/// blitter seven slots of each eight (plane 1's is the eighth).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct OnePlane;

impl Timing for OnePlane {
    const TIMING: DisplayTiming = DisplayTiming {
        frame_lines: FRAME_LINES,
        fetch_lines: TOP..TOP + HEIGHT,
        fetch_slots: 0x38..0x38 + 20 * 8,
        blitter_free: 0x7F,
        stolen_cycles: 0,
    };
}

/// The blit: A to D, two words a row (the pattern's word and the word its shift spills into), A
/// shifted right by [`BLIT_X`] mod 16. A's modulo of -2 reads each pattern word once per row and the
/// last-word mask clears the second word A brings, so only the shifted-out bits reach it.
pub fn blit() -> Blit {
    let shift = (BLIT_X % 16) as u16;
    Blit {
        // ASH, USEA | USED, minterm D = A.
        con0: shift << 12 | 0x0900 | 0xF0,
        con1: 0,
        afwm: 0xFFFF,
        alwm: 0x0000,
        pt: [PATTERN, 0, 0, SCREEN + BLIT_Y * ROW_BYTES + BLIT_X / 16 * 2],
        modulo: [-2, 0, 0, ROW_BYTES as i16 - 4],
        size: (PATTERN_ROWS.len() as u16) << 6 | 2,
    }
}

/// The copper list: the screen's set-up and colours in the vertical blank, a wait for
/// [`SPLIT_LINE`] and colour 1's change, a wait inside [`MID_LINE`] and colour 0's change, and the
/// end of the list. The copper wakes when the beam is two slots short of a WAIT's position, so the
/// MOVE after it writes at the position plus two: a wait for `MID_H - 2` writes at [`MID_H`].
pub fn copper_list() -> Vec<u16> {
    let mv = |r: u16, v: u16| [r, v];
    let wait = |line: u16, h: u16| [(line & 0xFF) << 8 | h & 0xFE | 1, 0xFFFE];
    [
        mv(reg::DIWSTRT, (TOP << 8) | LEFT as u16),
        // VSTOP $2c: bit 8 is the inverse of bit 7, so the window ends at $12c, 256 lines on.
        mv(
            reg::DIWSTOP,
            ((TOP + HEIGHT) & 0xFF) << 8 | (LEFT as u16 + 320 - 0x100),
        ),
        mv(reg::DDFSTRT, 0x38),
        mv(reg::DDFSTOP, 0xD0),
        mv(reg::BPLCON0, 0x1200),
        mv(reg::BPLCON1, 0),
        mv(reg::BPL1MOD, 0),
        mv(reg::BPL1PTH, (SCREEN >> 16) as u16),
        mv(reg::BPL1PTH + 2, SCREEN as u16),
        mv(reg::COLOR00, BACKGROUND),
        mv(reg::COLOR00 + 2, ABOVE),
        wait(SPLIT_LINE, 0x06),
        mv(reg::COLOR00 + 2, BELOW),
        wait(MID_LINE, MID_H - 2),
        mv(reg::COLOR00, MID_COLOUR),
        [0xFFFF, 0xFFFE],
    ]
    .concat()
}

/// The note: channel 0 playing the sample at [`SAMPLE`], DMA turned on with it.
pub fn note() -> Vec<PaulaWrite> {
    let w = |reg, value| PaulaWrite {
        reg,
        value,
        hpos: 0,
        vpos: 0,
    };
    vec![
        w(0xA0, (SAMPLE >> 16) as u16),
        w(0xA2, SAMPLE as u16),
        w(0xA4, SAMPLE_WORDS),
        w(0xA6, PERIOD),
        w(0xA8, VOLUME),
        w(0x96, 0x8201),
    ]
}

/// The sample: half a cycle at +64, half at -64, a square wave of eight bytes.
pub const SAMPLE_BYTES: [u8; 8] = [0x40, 0x40, 0x40, 0x40, 0xC0, 0xC0, 0xC0, 0xC0];

/// What one frame of the fit made.
pub struct Frame {
    pub machine: Machine,
    /// The blit's D writes, in CPU cycles from the frame's start.
    pub blit_writes: Vec<u32>,
    /// The copper's register writes, in beam order.
    pub writes: Vec<Write>,
    pub picture: Picture,
    /// The clock after the CPU's frame of work.
    pub clock: Clock<OnePlane>,
    /// Channel 0's DAC value at every Paula clock of the frame.
    pub tap: Vec<i16>,
    /// The mixed host-rate samples.
    pub audio: Vec<[i16; 2]>,
}

/// Run the fit's one frame.
pub fn run() -> Frame {
    let mut m = Machine::new();
    for (i, &w) in copper_list().iter().enumerate() {
        m.put_word(COPPER + 2 * i as u32, w);
    }
    for (i, &w) in PATTERN_ROWS.iter().enumerate() {
        m.put_word(PATTERN + 2 * i as u32, w);
    }
    let s = SAMPLE as usize;
    m.mem.chip[s..s + SAMPLE_BYTES.len()].copy_from_slice(&SAMPLE_BYTES);

    // The CPU starts the blit at the frame's start; the clock says when its words are written.
    let b = blit();
    let mut clock = Clock::<OnePlane>::START;
    let (rows, words) = amiga_chips::bltsize_words(b.size);
    let blit_writes = clock.blit_cycles(rows * words, b.con0);
    b.run(&mut m);
    // The rest of the frame is the CPU's.
    let used = clock.nominal();
    clock.cpu(FRAME_LINES as u32 * LINE_CYCLES - used);

    let writes = copper::run(&|a| m.word(a), COPPER, Fetch::default(), FRAME_LINES);
    let picture = display::draw(&m, &writes, Fetch::default());

    let mut paula = Paula::new(HOST_RATE, FRAME_LINES as u32);
    let mut audio = Vec::new();
    paula.frame(&note(), &m, &mut audio);
    let tap = paula.channel_taps()[0].to_vec();

    Frame {
        machine: m,
        blit_writes,
        writes,
        picture,
        clock,
        tap,
        audio,
    }
}
