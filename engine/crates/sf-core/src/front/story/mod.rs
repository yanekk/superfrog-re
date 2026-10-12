//! The story intro (T32, docs/13 "The story intro"): what booting disk 3 plays before the game.
//! Disk 3's boot program shows the Team 17 logo while it loads the intro (`boot`); the intro by
//! Eric Schwartz (`d3_c04`) plays its scenes and asks for disk 1 (`intro`); the game then starts
//! as a disk-1 power-on does ([`crate::Start::PowerOn`]).
//!
//! The intro is a program of its own with its own memory map, so its state is the whole of RAM as
//! the original's bytes ([`StoryMem`]) and the variables conformance compares are its own
//! ([`Story::layout`]). Loads are instant (owner, 2026-10-12): a frame that loads reports where
//! the original is once its load is over, as the front end's do.

pub mod boot;
pub mod data;
pub mod fade;
pub mod mem;

pub use data::StoryData;
pub use mem::StoryMem;

use sf_paula::PaulaWrite;

use crate::game::{Job, LoadEnd};
use crate::layout::{VarDef, VarId, Width};
use crate::render::copper::Fetch;
use crate::render::{DisplayData, Picture};
use crate::state::State;

/// What a story routine gives back at the end of its work in a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wait {
    /// It waits on into the next frame.
    Frame,
    /// The part is over.
    Over,
}

/// The display window Kickstart's screen leaves, which the boot program never sets until its
/// picture's list runs: 320 × 200 from (`$81`, `$2c`) (measured).
const KICKSTART_DIWSTRT: u16 = 0x2c81;
const KICKSTART_DIWSTOP: u16 = 0xf4c1;

/// The line of its first frame the boot program starts on: after line `$120`, since its first
/// `wait_raster` ends in the next frame (oracle).
const BOOT_START_LINE: u16 = 0x130;

/// Lines in a long frame; a short one has one fewer.
const LONG_FRAME: u16 = 313;

/// What a routine sees while it runs.
pub struct Cx<'a> {
    pub m: &'a mut StoryMem,
    pub data: &'a StoryData,
    pub input: crate::input::FrameInput,
    pub paula: &'a mut Vec<PaulaWrite>,
    /// The beam's line while this code runs.
    pub line: u16,
    /// This frame is a long one (LOF): with interlace on, frames alternate.
    pub lof: bool,
    /// COP1LC: the copper list the next frame starts.
    pub cop1lc: u32,
    /// Byte waits still outstanding (`wait_raster`'s `$ff` then `$20`), across frames.
    pub pending: &'a mut Vec<u8>,
    /// BPLCON0 as the CPU last wrote it (carried into the next frame's display registers).
    pub bplcon0: Option<u16>,
    pub load: Option<LoadEnd>,
    pub load_line: Option<u16>,
    pub jobs: Vec<Job>,
}

impl Cx<'_> {
    fn frame_lines(&self) -> u16 {
        if self.lof { LONG_FRAME } else { LONG_FRAME - 1 }
    }

    /// Busy-wait for the beam to reach a line whose low byte is each of `bytes` in turn
    /// (`cmpi.b #b, VHPOSR`): true once all have passed this frame, false when the frame ends
    /// first (the rest are waited for from the next frame's top).
    pub fn wait_bytes(&mut self, bytes: &[u8]) -> bool {
        if self.pending.is_empty() {
            self.pending.extend(bytes.iter().rev());
        }
        while let Some(&b) = self.pending.last() {
            let lines = self.frame_lines();
            match (self.line..lines).find(|l| *l as u8 == b) {
                Some(l) => {
                    self.line = l;
                    self.pending.pop();
                }
                None => {
                    self.line = lines;
                    return false;
                }
            }
        }
        true
    }

    /// Busy-wait for line `n` (`n` < 256).
    pub fn wait_line(&mut self, n: u16) -> bool {
        self.wait_bytes(&[n as u8])
    }

    /// The CPU writes BPLCON0: interlace on or off from here.
    pub fn cpu_bplcon0(&mut self, v: u16) {
        self.bplcon0 = Some(v);
    }

    /// A CPU job of the original's that may run past the frame's end: its memory, `writes`
    /// (address, length), is compared once the original is out of the call returning to `inside`.
    pub fn job(&mut self, inside: u32, writes: &[(u32, u32)]) {
        self.jobs.push(Job {
            inside: vec![inside],
            writes: writes.to_vec(),
        });
    }

    /// An instant load is over: conformance finds the original past it as `end` says; the program
    /// goes on from the line the runner gave, if it gave one. The original's load ends frames
    /// after it began, so the line can be above the one the load began on.
    pub fn loaded(&mut self, end: LoadEnd) {
        self.load = Some(end);
        if let Some(l) = self.load_line.take() {
            self.line = l;
        }
    }
}

/// Which program runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Stage {
    Boot(boot::Boot),
    /// The story is over: the game starts.
    Over,
}

/// One frame of the story.
pub struct StoryOut {
    pub picture: Picture,
    pub paula: Vec<PaulaWrite>,
    pub load: Option<LoadEnd>,
    pub resync: Vec<(u32, u32)>,
    pub jobs: Vec<Job>,
    /// The story handed over to the game this frame.
    pub over: bool,
}

/// The story's program and the hardware state the engine tracks outside memory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Story {
    pub mem: StoryMem,
    stage: Stage,
    /// COP1LC: the copper list the next frame starts with.
    cop1lc: u32,
    /// The display registers as the last frame left them; `None` on the first frame, which
    /// shows Kickstart's screen.
    fetch: Option<Fetch>,
    /// Interlace on (BPLCON0's LACE as the copper or CPU last wrote it) and this frame's parity.
    lace: bool,
    lof: bool,
    pending: Vec<u8>,
    /// For conformance: the line the next load ends on in the original.
    pub(crate) load_line: Option<u16>,
    frame: u32,
}

/// The modelled variables while the boot program runs: its copper lists and its fade record.
pub static BOOT_LAYOUT: &[VarDef] = &[
    VarDef {
        id: VarId::StoryBootFade,
        name: "boot_fade",
        addr: boot::at(0x2be18),
        width: Width::Bytes(0x2c00c - 0x2be18),
    },
    VarDef {
        id: VarId::StoryBootCopper,
        name: "boot_copper",
        addr: boot::LIST_A,
        width: Width::Bytes(0x2c2b4 - 0x2c00c),
    },
];

impl Story {
    /// Switch on with disk 3 in df0: the boot program starts.
    pub fn power_on(data: &StoryData) -> Story {
        let mut mem = StoryMem::default();
        mem.write(data::BOOT_BASE, &data.boot);
        let cop1lc = boot::start(&mut mem);
        Story {
            mem,
            stage: Stage::Boot(boot::Boot::Started),
            cop1lc,
            fetch: None,
            lace: false,
            // So that the first interlaced frame, the boot program's third, is a short one, as
            // on the original.
            lof: true,
            pending: Vec::new(),
            load_line: None,
            frame: 0,
        }
    }

    /// The variables conformance compares now.
    pub fn layout(&self) -> &'static [VarDef] {
        match self.stage {
            Stage::Boot(_) => BOOT_LAYOUT,
            Stage::Over => &[],
        }
    }

    /// Whether the story has handed over to the game.
    pub fn over(&self) -> bool {
        self.stage == Stage::Over
    }

    /// One frame.
    pub fn step(
        &mut self,
        empty: &State,
        display: &DisplayData,
        input: crate::input::FrameInput,
    ) -> StoryOut {
        // Agnus flips the field at each frame's start while interlace is on.
        if self.lace {
            self.lof = !self.lof;
        }
        let list = self.cop1lc;
        // The copper reads its list at the frame's top, before any of this frame's work.
        let copper = self.mem.read(list, 0x400);
        let mut paula = Vec::new();
        let data = &display.story;
        let mut cx = Cx {
            m: &mut self.mem,
            data,
            input,
            paula: &mut paula,
            line: 0,
            lof: self.lof,
            cop1lc: self.cop1lc,
            pending: &mut self.pending,
            bplcon0: None,
            load: None,
            load_line: self.load_line.take(),
            jobs: Vec::new(),
        };
        // The engine starts where the boot program does: the original's frames up to its first
        // wait (Kickstart, the bootblock's load) are a load.
        // It starts late in that frame: its first wait for line $ff ends in the next one.
        if self.frame == 0 {
            cx.load = Some(LoadEnd::Reached(boot::RET_FIRST_WAIT));
            cx.line = BOOT_START_LINE;
        }
        let wait = match &mut self.stage {
            Stage::Boot(b) => b.run(&mut cx),
            Stage::Over => Wait::Over,
        };
        let (load, jobs, bplcon0) = (cx.load, cx.jobs, cx.bplcon0);
        self.cop1lc = cx.cop1lc;
        let picture = match self.fetch {
            None => Picture::blank(0x81, 0x2c, 320, 200),
            Some(carry) => {
                let word = |a: u32, _line: u16| {
                    let o = a.wrapping_sub(list) as usize;
                    match copper.get(o..o + 2) {
                        Some(w) => u16::from_be_bytes([w[0], w[1]]),
                        None => self.mem.w(a),
                    }
                };
                let (pic, writes) =
                    crate::render::story_screen(&self.mem, empty, display, list, &word, Some(carry));
                let mut end = carry;
                for w in &writes {
                    end.write(w.reg, w.value);
                    match w.reg {
                        0x80 => self.cop1lc = (w.value as u32) << 16 | self.cop1lc & 0xffff,
                        0x82 => self.cop1lc = self.cop1lc & 0xffff_0000 | w.value as u32,
                        _ => {}
                    }
                }
                self.fetch = Some(end);
                pic
            }
        };
        if self.fetch.is_none() {
            self.fetch = Some(Fetch {
                diwstrt: KICKSTART_DIWSTRT,
                diwstop: KICKSTART_DIWSTOP,
                ..Fetch::default()
            });
        }
        if let (Some(v), Some(f)) = (bplcon0, self.fetch.as_mut()) {
            f.bplcon0 = v;
        }
        if let Some(f) = self.fetch {
            self.lace = f.bplcon0 & 4 != 0;
        }
        if wait == Wait::Over {
            self.stage = Stage::Over;
        }
        self.frame += 1;
        StoryOut {
            picture,
            paula,
            load,
            resync: Vec::new(),
            jobs,
            over: self.stage == Stage::Over,
        }
    }
}
