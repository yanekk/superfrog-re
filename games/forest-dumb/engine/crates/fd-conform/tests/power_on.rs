//! Power-on cases (`start = "entry"`, `align = "loads"`, translator T32). Full: the owner's disks
//! and Kickstart; the translated tests also the tree built in (`tools/translate.py --build`).
//!
//! The runner, `fd_conform::frames::run_entry_frames`, against a stand-in: a second copy of the
//! original from power-on, started at the same entry step, that runs each of its loads 50 frames
//! short (it steps its own copy 50 frames on inside the load, in one engine frame). The runner's
//! original then has 50 frames more inside the load, which it must skip and line up at the
//! load's return. The OS keeps the machine after it until the entry installs the program's
//! handlers (frame 3968), a stretch lined up as part of the load (owner, T51); a byte changed in
//! the first frame after that must fail there.

mod common;

use amiga_oracle::{Amiga, FrameInput, PaulaWrite, Rgb12Frame};
use common::picture_of;
use fd_conform::frames::{FrameEngine, SCREEN_WINDOW, run_entry_frames};
use fd_conform::sync::{self, LOAD_RETURNS, LOAD_ROUTINES};
use fd_conform::translated_engine::run_translated;
use fd_conform::{Case, HUNK_BASES, Inputs, Options, Outcome, Verdict};
use fd_core::{Mem, Transplant, Var};
use fd_translated::TranslatedGame;

/// boot-to-title from power-on to just past `read_marker`'s load (its return at frame 3889 on the
/// original) and the program taking the machine (3968), so one load is lined up.
fn first_load() -> Case {
    Case::parse(
        "power-on-made-up",
        "sync = { script = \"boot-to-title\", frame = 0 }\nstart = \"entry\"\nalign = \"loads\"\n\
         swap = { frame = 3600, disk = 2 }\nframes = 4000\n\
         compare = [\"memory\", \"picture\", \"writes\"]\n",
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// Frames a stand-in load is shorter than the original's.
const SHORT: u32 = 50;

struct StandIn {
    am: Amiga,
    inputs: Inputs,
    case: Case,
    /// No step yet: the stand-in's copy is at the end of the entry's step, as the runner's is.
    first: bool,
    entered: u32,
    returned: u32,
    /// Loads already cut short.
    cut: u32,
    loads: (u32, bool),
    ram: Vec<u8>,
    pic: Option<Rgb12Frame>,
    paula: Vec<PaulaWrite>,
    custom: Vec<(u16, u16)>,
    /// Change one byte of `h0+$7cde` in the reported memory on the first frame after this many
    /// loads returned that ends with the program's handler in the level 3 vector.
    meddle_after: Option<u32>,
    meddled: bool,
}

fn pcs() -> (Vec<u32>, Vec<u32>) {
    let h = |(hunk, off): (u8, u32)| HUNK_BASES[hunk as usize] + off;
    (
        LOAD_ROUTINES.iter().copied().map(h).collect(),
        LOAD_RETURNS.iter().copied().map(h).collect(),
    )
}

impl StandIn {
    fn new(case: &Case, meddle_after: Option<u32>) -> StandIn {
        let inputs = Inputs::load(case).unwrap();
        let (entries, returns) = pcs();
        let watch: Vec<u32> = entries.iter().chain(&returns).copied().collect();
        // Its loads end `SHORT` frames early, so it runs that far past the case's last frame.
        let mut longer = case.clone();
        longer.frames += SHORT;
        let (am, _) = sync::reach_entry(&longer, &inputs, &watch).unwrap_or_else(|e| panic!("{e}"));
        let mut s = StandIn {
            am,
            inputs,
            case: case.clone(),
            first: true,
            entered: 0,
            returned: 0,
            cut: 0,
            loads: (0, false),
            ram: Vec::new(),
            pic: None,
            paula: Vec::new(),
            custom: Vec::new(),
            meddle_after,
            meddled: false,
        };
        s.read(Vec::new());
        s
    }

    /// One step of the copy on `input`, its swap made; the loads it returned from.
    fn step(&mut self, input: FrameInput) -> (u32, Vec<PaulaWrite>) {
        let f = self.am.frame();
        if let Some(s) = self.case.swap.filter(|s| s.frame == f) {
            self.am.insert_disk(0, s.disk as usize - 1);
        }
        let c = self.am.step(input).unwrap();
        let (entries, returns) = pcs();
        let hits = |l: &[u32]| -> u32 {
            self.am
                .pc_hits()
                .iter()
                .filter(|(pc, _)| l.contains(pc))
                .map(|&(_, n)| n)
                .sum()
        };
        let r = hits(&returns);
        self.entered += hits(&entries);
        self.returned += r;
        (r, c.paula)
    }

    fn read(&mut self, paula: Vec<PaulaWrite>) {
        let m = self.am.memory();
        self.ram = m.chip.clone();
        self.ram.extend_from_slice(&m.slow);
        self.pic = Some(self.am.picture_in(SCREEN_WINDOW.0, SCREEN_WINDOW.1));
        self.paula = paula;
        self.custom = self
            .am
            .cpu_writes()
            .iter()
            .map(|w| (w.reg & 0x1FE, w.value))
            .collect();
    }
}

impl Transplant for StandIn {
    fn transplant(&mut self, _: &dyn Mem) {
        unreachable!("a power-on case transplants nothing")
    }
    fn probe(&self, _: &Var) -> u32 {
        unreachable!("the case compares memory, not variables")
    }
}

impl FrameEngine for StandIn {
    fn frame(&mut self, input: FrameInput) {
        if std::mem::take(&mut self.first) {
            self.loads = (0, false);
            return;
        }
        let (mut returned, paula) = self.step(input);
        // Inside a load not yet cut: run its next `SHORT` frames now, on the copy's own inputs.
        if self.entered > self.returned && self.cut < self.entered {
            self.cut = self.entered;
            for _ in 0..SHORT {
                let f = self.am.frame();
                returned += self.step(self.inputs.at(f)).0;
            }
        }
        self.loads = (returned, self.entered > self.returned);
        self.read(paula);
        if let Some(n) = self.meddle_after
            && !self.meddled
            && self.returned >= n
            && !self.loads.1
            && returned == 0
            && self.am.peek(sync::LEVEL3_VECTOR, 4)
                == (HUNK_BASES[0] + sync::LEVEL3_HANDLER.1).to_be_bytes()
        {
            self.meddled = true;
            let a = (HUNK_BASES[0] + 0x7cde - amiga_oracle::SLOW_BASE) as usize + 512 * 1024;
            self.ram[a] ^= 0x01;
        }
    }
    fn picture(&self) -> Option<amiga_chips::Picture> {
        self.pic.as_ref().map(picture_of)
    }
    fn paula(&self) -> &[PaulaWrite] {
        &self.paula
    }
    fn custom_writes(&self) -> &[(u16, u16)] {
        &self.custom
    }
    fn pass_ends(&self, _: &[&Var]) -> Vec<Vec<u32>> {
        unreachable!("a power-on case is lined up by loads, not pass ends")
    }
    fn stopped(&self) -> Option<String> {
        None
    }
    fn ram(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }
    fn loads(&self) -> (u32, bool) {
        self.loads
    }
}

fn run_stand_in(meddle_after: Option<u32>) -> Verdict {
    run_stand_in_with(meddle_after, &Options::default())
}

fn run_stand_in_with(meddle_after: Option<u32>, opts: &Options) -> Verdict {
    let case = first_load();
    let inputs = Inputs::load(&case).unwrap();
    let mut s = StandIn::new(&case, meddle_after);
    run_entry_frames(&case, &inputs, &mut s, opts).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn a_load_fifty_frames_longer_on_the_original_aligns_and_passes() {
    let v = run_stand_in(None);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(v.loads, 1, "read_marker's load lined up: {v}");
    assert_eq!(v.load_frames, SHORT, "the original's extra frames skipped: {v}");
    assert_eq!(v.entry.map(|e| e.0), Some(3461), "{v}");
    // Every frame outside the load: the prompt's 240 up to fire at 3700, then those after the
    // program takes the machine at 3968 (260 measured): the OS's stretch from the return at
    // 3889 is the load's (owner, T51).
    assert!(v.aligned_frames >= 255 && v.memory_frames == v.aligned_frames, "{v}");
    assert!(v.tail_frames > 0, "the OS's stretch after the load: {v}");
    assert!(v.os_frames < v.aligned_frames, "the program's own frames' writes compared: {v}");
}

#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn a_difference_just_after_the_program_takes_the_machine_fails_there() {
    let v = run_stand_in(Some(1));
    match &v.outcome {
        Outcome::Differs { var, frame, .. } => {
            assert!(var.starts_with("h0+$7cde (1 bytes"), "{v}");
            assert_eq!(*frame, 3968, "the first frame the program has the machine: {v}");
        }
        other => panic!("expected a memory difference, got {other}"),
    }
}

/// With `every_channel` (what `run --engine translated` sets) a channel's first difference drops
/// that channel and the run goes on to the case's end with the others (T32 review: it stopped at the
/// next frame).
#[test]
#[ignore = "full: the disks and Kickstart; the original from power-on twice"]
fn every_channel_goes_on_past_the_first_difference() {
    let v = run_stand_in_with(
        Some(1),
        &Options {
            every_channel: true,
            ..Options::default()
        },
    );
    assert!(
        matches!(&v.outcome, Outcome::Differs { frame: 3968, .. }),
        "the memory difference first: {v}"
    );
    assert_eq!(v.firsts.len(), 1, "only memory differs: {v}");
    assert_eq!(v.loads, 1, "{v}");
    assert!(v.aligned_frames >= 255, "every frame outside the load compared: {v}");
    assert!(v.pictures >= 255, "the picture judged on to the end: {v}");
}

fn generated() {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again (fd-translated rebuilds with it)"
    );
}

fn translated(name: &str) -> Verdict {
    generated();
    let case = Case::load(std::path::Path::new(&format!("cases/{name}.toml")))
        .unwrap_or_else(|e| panic!("{e}"));
    run_translated(&case, &Options::default()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; 15 078 title frames, about 75 s"]
fn first_case_passes_on_the_translated_program() {
    let v = translated("first-case");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; power-on to the title"]
fn power_on_title_passes_on_the_translated_program() {
    let v = translated("power-on-title");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}

#[test]
#[ignore = "full: the disks, Kickstart and the generated tree; power-on into level 1"]
fn power_on_level_1_passes_on_the_translated_program() {
    let v = translated("power-on-level-1");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
}
