//! The story intro (T32, docs/13 "The story intro"): what booting disk 3 plays before the game.
//! Disk 3's boot program shows the Team 17 logo while it loads the intro (`boot`); the intro by
//! Eric Schwartz (`d3_c04`) plays its scenes and asks for disk 1 (`intro`); the game then starts
//! as a disk-1 power-on does ([`crate::Start::PowerOn`]).
//!
//! The intro is a program of its own with its own memory map, so its state is the whole of RAM as
//! the original's bytes ([`StoryMem`]) and the variables conformance compares are its own
//! ([`Story::layout`]). Loads are instant (owner, 2026-10-12): a frame that loads reports where
//! the original is once its load is over, as the front end's do.

pub mod bobs;
pub mod boot;
pub mod cpu;
pub mod data;
pub mod fade;
pub mod intro;
pub mod mem;
pub mod ops;
pub mod scenes;
pub mod sim;
pub mod sound;
pub mod vbl;

pub use data::StoryData;
pub use mem::StoryMem;

use sf_paula::PaulaWrite;

use crate::game::{Job, LoadEnd};
use crate::layout::{VarDef, VarId, Width};
use crate::render::copper::Fetch;
use crate::render::{DisplayData, Picture};
use crate::state::State;

/// `hN+$off` of the intro program (`d3_c04`), as `d3_c02` places its hunks.
pub const fn h0(off: u32) -> u32 {
    data::INTRO_H0 + off
}
pub const fn h1(off: u32) -> u32 {
    data::INTRO_H1 + off
}
pub const fn h2(off: u32) -> u32 {
    data::INTRO_H2 + off
}
pub const fn h3(off: u32) -> u32 {
    data::INTRO_H3 + off
}

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

/// What Kickstart 1.3 leaves in chip RAM's first longs (its reset stack word, ExecBase at $C00276,
/// the bus and address error vectors), which the music replayer reads through a zero instrument
/// pointer (measured on the oracle; the owner's ROM).
const KICKSTART_LOW: [u8; 12] = [0, 0, 0, 0, 0x00, 0xc0, 0x02, 0x76, 0x00, 0xfc, 0x08, 0x18];

/// The bus the handler finds: refresh and the copper's cycles (sorted).
struct HandlerBus(Vec<u32>);

impl cpu::Bus for HandlerBus {
    fn taken(&self, t: u32) -> bool {
        cpu::refresh(t) || self.0.binary_search(&t).is_ok()
    }
}

/// Lines into the handler from which the copper's reads see its writes.
const COPPER_SEES_HANDLER: u16 = 2;

/// The DMA cycle of line 0 by which the CPU has taken the vertical blank's interrupt: copper
/// writes before it go with the frame before (oracle: the black list's DMACON on cycle 12).
const BEFORE_IRQ: u16 = 0x14;

fn is_paula(reg: u16) -> bool {
    matches!(reg, 0x96 | 0x9a | 0x9c | 0x9e) || (0xa0..0xe0).contains(&reg)
}

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
    /// The boot program is done; the intro takes over from the next frame (a load).
    BootDone,
    /// The intro, and its frame count.
    Intro(Box<intro::Intro>, u64),
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
    /// For conformance: what runs on by itself during the next load, to put in place at it.
    pub(crate) load_state: Vec<(u32, Vec<u8>)>,
    /// For conformance: where the original took the next frame's interrupt (line, DMA cycle).
    pub(crate) handler_entry: Option<(u16, u16)>,
    /// For conformance: where the original's handler of the next frame ended (T54).
    pub(crate) handler_end: Option<crate::game::HandlerEnd>,
    /// For conformance: whether the next frame's blank goes without a handler (merged into the
    /// next one's), as the original's did.
    pub(crate) blank_merged: Option<bool>,
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

/// The modelled variables while the intro runs.
pub static INTRO_LAYOUT: &[VarDef] = &[
    VarDef {
        id: VarId::StoryVars,
        name: "intro_vars",
        addr: h0(0x16),
        width: Width::Bytes(10),
    },
    VarDef {
        id: VarId::StoryRows,
        name: "buffer_toggle_rows",
        addr: h0(0x84e0),
        width: Width::Bytes(4 + 400),
    },
    VarDef {
        id: VarId::StoryObjects,
        name: "objects",
        addr: h0(0x8b68),
        width: Width::Bytes(0x8bb0 - 0x8b68 + 16 * 0xb0),
    },
    VarDef {
        id: VarId::StoryFrames,
        name: "frame_descriptors",
        addr: h0(0x97be),
        width: Width::Bytes(0xafd6 - 0x97be),
    },
    VarDef {
        id: VarId::StoryShown,
        name: "draw_shown_frozen",
        addr: h0(0xb42e),
        width: Width::Bytes(10),
    },
    VarDef {
        id: VarId::StoryFades,
        name: "fade_records",
        addr: h0(0xbb70),
        width: Width::Bytes(4 * 0x1f2),
    },
    VarDef {
        id: VarId::StoryCopper,
        name: "intro_copper",
        addr: h2(0xda5a),
        width: Width::Bytes(0xdc9a - 0xda5a),
    },
    VarDef {
        id: VarId::StoryText,
        name: "text_page",
        addr: h3(0),
        width: Width::Bytes(0x5e00),
    },
    VarDef {
        id: VarId::StoryBuffers,
        name: "display_buffers",
        addr: bobs::BUFFER_A,
        width: Width::Bytes(2 * 5 * bobs::PLANE),
    },
    VarDef {
        id: VarId::StoryClean,
        name: "clean_background",
        addr: bobs::CLEAN,
        width: Width::Bytes(5 * bobs::PLANE + 0x40),
    },
    VarDef {
        id: VarId::StorySfx,
        name: "sfx_records",
        addr: h0(0x38efc),
        width: Width::Bytes(0xfe),
    },
    VarDef {
        id: VarId::StoryMusicOn,
        name: "music_on",
        addr: h0(0x39296),
        width: Width::Bytes(4),
    },
    VarDef {
        id: VarId::StoryReplayer,
        name: "p41_vars",
        addr: h0(0x3a352),
        width: Width::Bytes(0xda),
    },
    VarDef {
        id: VarId::StoryShadow,
        name: "music_fade_shadow",
        addr: h0(0x3a556),
        width: Width::Bytes(0x44),
    },
    VarDef {
        id: VarId::StoryStream,
        name: "stream",
        addr: h0(0x3a726),
        width: Width::Bytes(8),
    },
    VarDef {
        id: VarId::StoryTimeline,
        name: "timeline",
        addr: h0(0xb1f4),
        width: Width::Bytes(6),
    },
];

impl Story {
    /// Switch on with disk 3 in df0: the boot program starts.
    pub fn power_on(data: &StoryData) -> Story {
        let mut mem = StoryMem::default();
        mem.write(0, &KICKSTART_LOW);
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
            load_state: Vec::new(),
            handler_entry: None,
            handler_end: None,
            blank_merged: None,
            frame: 0,
        }
    }

    /// The variables conformance compares now.
    pub fn layout(&self) -> &'static [VarDef] {
        match self.stage {
            Stage::Boot(_) | Stage::BootDone => BOOT_LAYOUT,
            Stage::Intro(..) => INTRO_LAYOUT,
            Stage::Over => &[],
        }
    }

    /// What runs on by itself while the original loads (the vertical blank's work).
    pub fn runs_during_load(&self) -> &'static [(u32, u32)] {
        match self.stage {
            Stage::Intro(..) => intro::RUNS_DURING_LOAD,
            _ => &[],
        }
    }

    /// The intro's main-program clock (lines since its start), while it runs (for tooling).
    pub fn intro_clock(&self) -> Option<u64> {
        match &self.stage {
            Stage::Intro(i, _) => Some(i.t),
            _ => None,
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
        pacing: crate::game::Pacing,
    ) -> StoryOut {
        // Agnus flips the field at each frame's start while interlace is on.
        if self.lace {
            self.lof = !self.lof;
        }
        if self.stage == Stage::BootDone {
            // The boot program unpacks and relocates the intro and starts it (a load).
            self.cop1lc = scenes::BLACK_LIST;
            self.stage = Stage::Intro(
                Box::new(intro::Intro::start(&mut self.mem, &display.story)),
                0,
            );
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
        // Conformance's feed is for this frame only, whatever stage it is in.
        let merged = self.blank_merged.take();
        let mut resync = Vec::new();
        let mut after_handler: Option<(u16, Vec<u8>)> = None;
        let wait = match &mut self.stage {
            Stage::Boot(b) => b.run(&mut cx),
            Stage::Intro(i, n) => {
                let load_line = cx.load_line.take();
                drop(cx);
                let load_state = std::mem::take(&mut self.load_state);
                let entry = self.handler_entry.take();
                let end = self.handler_end.take();
                let f = i.frame(
                    &mut self.mem,
                    data,
                    input,
                    *n,
                    load_line,
                    list,
                    pacing,
                    load_state,
                    entry,
                    end,
                    merged,
                );
                *n += 1;
                // A load's frame shows the list set before the load, as the load left memory.
                let (list, copper, at_handler) = match f.list_at_load {
                    Some(l) => (l, self.mem.read(l, 0x400), None),
                    // The handler's writes (bitplane pointers, a fade's colours) are in place by the
                    // time the copper reads the list past its wait for line $20.
                    None => (
                        list,
                        copper,
                        Some((f.handler.0 + COPPER_SEES_HANDLER, f.copper)),
                    ),
                };
                paula = f.paula;
                if let Some(c) = f.cop1lc {
                    self.cop1lc = c;
                }
                resync = f.resync;
                after_handler = at_handler;
                let w = if i.ended { Wait::Over } else { Wait::Frame };
                let walk = match (f.list_at_load, f.walk) {
                    (None, Some(w)) => Some((f.entry, w)),
                    _ => None,
                };
                return self.finish(
                    empty,
                    display,
                    list,
                    copper,
                    after_handler,
                    walk,
                    f.load,
                    resync,
                    f.jobs,
                    None,
                    paula,
                    w,
                );
            }
            Stage::BootDone | Stage::Over => Wait::Over,
        };
        let (load, jobs, bplcon0) = (cx.load, cx.jobs, cx.bplcon0);
        self.cop1lc = cx.cop1lc;
        self.finish(
            empty,
            display,
            list,
            copper,
            after_handler,
            None,
            load,
            resync,
            jobs,
            bplcon0,
            paula,
            wait,
        )
    }

    /// The frame's picture and the hand-over between stages.
    #[allow(clippy::too_many_arguments)]
    fn finish(
        &mut self,
        empty: &State,
        display: &DisplayData,
        list: u32,
        copper: Vec<u8>,
        after_handler: Option<(u16, Vec<u8>)>,
        walk: Option<(u32, vbl::Handler)>,
        load: Option<LoadEnd>,
        resync: Vec<(u32, u32)>,
        jobs: Vec<Job>,
        bplcon0: Option<u16>,
        mut paula: Vec<PaulaWrite>,
        wait: Wait,
    ) -> StoryOut {
        let picture = match self.fetch {
            None => Picture::blank(0x81, 0x2c, 320, 200),
            Some(carry) => {
                let snap = |a: u32, line: u16| {
                    let o = a.wrapping_sub(list) as usize;
                    let snap = match &after_handler {
                        Some((l, after)) if line >= *l => after,
                        _ => &copper,
                    };
                    match snap.get(o..o + 2) {
                        Some(w) => u16::from_be_bytes([w[0], w[1]]),
                        None => self.mem.w(a),
                    }
                };
                // The handler's writes to the fades' colour words, timed on the bus the copper
                // leaves the CPU (its own cycles, run first: the CPU never holds it up).
                let fade_writes = match &walk {
                    Some((entry, h)) => {
                        let mut taken = Vec::new();
                        crate::render::copper::run_bus(
                            &|a, line, _| snap(a, line),
                            list,
                            carry,
                            &mut taken,
                        );
                        let bus = HandlerBus(
                            taken
                                .iter()
                                .map(|b| b.line as u32 * cpu::LINE + b.h as u32)
                                .collect(),
                        );
                        vbl::colour_writes(h, *entry, &bus)
                    }
                    None => Vec::new(),
                };
                let word = |a: u32, line: u16, h: u16| {
                    // A word the handler's fade step rewrites: as its last write before the
                    // copper's read left it.
                    let t = line as u32 * cpu::LINE + h as u32;
                    let mut rewritten = fade_writes.iter().filter(|w| w.0 == a).peekable();
                    if rewritten.peek().is_some() {
                        let o = a.wrapping_sub(list) as usize;
                        let last = rewritten.rfind(|w| w.1 < t).map(|w| w.2);
                        return last.unwrap_or_else(|| match copper.get(o..o + 2) {
                            Some(w) => u16::from_be_bytes([w[0], w[1]]),
                            None => self.mem.w(a),
                        });
                    }
                    snap(a, line)
                };
                let (pic, writes) = crate::render::story_screen(
                    &self.mem,
                    empty,
                    display,
                    list,
                    &word,
                    Some(carry),
                );
                let mut end = carry;
                for w in &writes {
                    // The copper's writes to Paula's registers, but those it makes before the CPU
                    // takes the vertical blank's interrupt: the last frame had them.
                    if is_paula(w.reg) && (w.line, w.h) >= (0, BEFORE_IRQ) {
                        paula.push(PaulaWrite {
                            reg: w.reg,
                            value: w.value,
                            hpos: 0,
                            vpos: w.line,
                        });
                    }
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
        // The next frame's copper, up to the interrupt, ends this frame on the original.
        if let Some(carry) = self.fetch
            && !matches!(self.stage, Stage::Boot(_) | Stage::BootDone)
        {
            let mem = &self.mem;
            let next = crate::render::copper::run_timed(&|a, _| mem.w(a), self.cop1lc, carry);
            for w in next
                .iter()
                .take_while(|w| (w.line, w.h) < (0, BEFORE_IRQ))
                .filter(|w| is_paula(w.reg))
            {
                paula.push(PaulaWrite {
                    reg: w.reg,
                    value: w.value,
                    hpos: 0,
                    vpos: crate::render::copper::FRAME_LINES,
                });
            }
        }
        paula.sort_by_key(|w| w.vpos);
        if wait == Wait::Over {
            self.stage = match self.stage {
                Stage::Boot(_) => Stage::BootDone,
                _ => Stage::Over,
            };
        }
        self.frame += 1;
        StoryOut {
            picture,
            paula,
            load,
            resync,
            jobs,
            over: self.stage == Stage::Over,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With no disks the boot program's data is empty, but the stages still follow each other.
    #[test]
    fn the_boot_hands_over_to_the_intro_with_a_load() {
        let display = DisplayData::new(&sf_assets::GameData::default());
        let empty = State::default();
        let mut s = Story::power_on(&display.story);
        let mut loads = Vec::new();
        for f in 0..200 {
            let out = s.step(
                &empty,
                &display,
                crate::input::FrameInput::NONE,
                crate::game::Pacing::Unpaced,
            );
            if out.load.is_some() {
                loads.push((f, out.load));
            }
        }
        assert!(loads.len() >= 2, "{loads:?}");
    }
}
