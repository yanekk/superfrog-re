//! The frame bar's runner (translator DESIGN §2.7, T29): a level case on an engine that is a whole
//! machine, stepped a frame at a time and judged on what the bar names: the case's state and the
//! custom-chip writes, Paula's in order and the CPU's others as a multiset (T45), at every pass end;
//! the picture in [`PLAY_WINDOW`] at every frame's end.
//!
//! **At the pass end, not the frame's** (owner, 2026-11-12, T30). The original's `game_frame`
//! returns after the frame's end on almost every moving frame (docs/09, 2 977 of 3 000), so its
//! state at a frame's end is mid-pass, and a machine with no DMA contention ends the same work some
//! lines earlier. Both sides' state is therefore taken where the CPU writes [`PASS_END`]
//! (`Amiga::watch_write`, `FrameEngine::pass_ends`) and paired by `irq_count`, and the writes are
//! cut into passes at that same write in each side's own stream and paired in order. A pass may end
//! up to [`PASS_LAG`] frames apart on the two sides; one further apart fails
//! (`Outcome::PassEndsApart`). The picture is what a frame showed, so it stays per frame.
//!
//! **Custom writes.** §2.7 asks order of Paula's writes only, and the bar does not chase sub-frame
//! timing, so every other register's writes are compared as (register, value) counts: the same
//! writes, each as many times (owner, 2026-11-12). Paula's registers are left out of that channel,
//! being compared in order already. Both sides give the CPU's writes alone, tagged where made: the
//! oracle keeps no Copper write (`Amiga::cpu_writes`), and the engine hands `pcode-machine`'s
//! `cpu_writes`, not its log with the copper's MOVEs in it.
//!
//! **Why not [`crate::run_with`].** The pass runner pairs the original's pass ends with an engine
//! that knows only passes: it hands each pass the input that pass read ([`crate::PassInputs`]),
//! compares at [`crate::PASS_END`], and shows each pass in the frame after it. A machine running the
//! original's program has frames, not passes: it reads the joystick when its own code does, so it
//! takes the input of each frame as the oracle got it, and its state, picture and writes are those
//! of the frame's end, which is where an oracle step ends. So the two step together, one engine
//! frame an oracle step, and nothing of the hand engine's measured cuts (T71, T74, T80, T82) applies.
//!
//! **Every channel.** A run ends at its first difference unless [`Options::every_channel`] is set:
//! then a channel that differed is no longer compared, the others go on, and
//! [`Verdict::firsts`] holds each channel's first difference in the order met (a stop always ends
//! the run). A whole-machine engine can differ in one channel from the first frame (its timing
//! moves a write across a frame's end) and still be right in the others, which a first reading
//! needs to see.
//!
//! **The start** of a level case is the pass runner's: the first step after the sync point in which
//! a pass ends. The engine is transplanted at that step's end (`translated_engine::case_start` takes
//! its start there), and its first frame is the next step's. A title case's engine starts at the end
//! of the sync frame's step, its first frame the next step's.
//!
//! **From power-on** (`start = "entry"`, T32, [`run_entry_frames`]): the engine starts at the
//! program's entry and its first frame is the oracle step that first ran it. Its loads finish at
//! once (DESIGN §2.8), so the frames are lined up by loads (`align = "loads"`,
//! `amiga_conform::loads::ByReturns`): the original runs on, its frames not compared, until it has
//! returned from as many loads as the engine, and a frame with a load running at its end on either
//! side is not compared. The state is the hunks' memory (`compare = ["memory"]`) less the OS's
//! values ([`crate::sync::OS_VALUES`]), and the custom writes are compared only on frames the
//! program had the machine for on the original ([`crate::sync::LEVEL3_HANDLER`] in place at the
//! step's start and end): while the OS has it, its interrupts write registers the machine does not.
//!
//! **The OS's stretch after a load is part of the load** (owner, T51, [`OsTail`]): from a load's
//! return until the program takes the machine back (its handler in the level 3 vector), a frame is
//! not compared, and the taking back is counted as a return, so the original runs on until it has
//! taken it back too. The original's OS runs its own interrupts there, about 5% of the CPU (world
//! 2's byte scan, measured with the OS's handlers stubbed: `system_off` 3 frames sooner), which
//! the machine never runs (DESIGN §2.8).
//!
//! **In play, at the pass end** (owner's §2.7 rule, T53): once the program has the machine and
//! either side has ended a pass, memory is judged at each pass end
//! ([`FrameEngine::pass_end_ram`], the oracle's `watched_memory`) paired by `irq_count` as a level
//! case's state is, and the other custom writes are cut into passes with [`custom_carried`]'s pass
//! of slack. The picture and Paula's writes stay per frame; before play (the title, SCENA) memory
//! stays judged at frame ends with T51's allowances.

use std::collections::{BTreeMap, VecDeque};
use std::ops::Range;

use amiga_chips::{Picture, is_paula, to_rgb12};
use amiga_conform::compare::{PictureClass, PictureJudge, classify, paula, picture_except};
use amiga_conform::loads::ByReturns;
use amiga_conform::{Mode, Tolerance, report};
use amiga_oracle::{Amiga, AmigaMemory, FrameCapture, FrameInput, PaulaWrite, SLOW_BASE};
use fd_core::{Mem, Transplant, Var, layout_var};

use crate::sync::{
    self, BEAM_VALUES, HUNK_SIZES, LOAD_CAP, LOAD_RETURNS, LOAD_ROUTINES, OS_VALUES,
};
use crate::{
    Case, GAME_OVER_SCREEN, Goal, HIGH_SCORES, HUNK_BASES, Inputs, Live, NEXT_WORLD, Options,
    OracleMem, Outcome, PASS_END, PLAY_WINDOW, Verdict, layout_vars, reach_case,
};

/// How many frames apart one pass may end on the two sides: the original's ends near line 310 or
/// in the next frame's first lines (docs/09), so a frame either way is the sub-frame timing the bar
/// does not chase (DESIGN §2.7); two is a different pace.
pub const PASS_LAG: u32 = 1;

/// The picture a title or power-on case compares: the PAL display window most screens use
/// (DIWSTRT `$2C81`, DIWSTOP `$2CC1`, 320 by 256 from line 44).
pub const SCREEN_WINDOW: (u16, u16) = (0x2C81, 0x2CC1);

/// An engine stepped a whole frame at a time: the translated program on its machine, or a stand-in.
pub trait FrameEngine: Transplant {
    /// Run one frame on `input`, the input the original was given in the same frame.
    fn frame(&mut self, input: FrameInput);
    /// The whole picture of the last frame (it is cut to [`PLAY_WINDOW`] here); `None` if none.
    fn picture(&self) -> Option<Picture>;
    /// Paula's writes in the last frame, in order; their beam positions are not compared. A
    /// write made with the bus's leftover word has [`LEFTOVER`] set in its register.
    fn paula(&self) -> &[PaulaWrite];
    /// Every custom-register write the CPU made in the last frame, `(register offset from
    /// $DFF000, value)`, Paula's included; never the copper's, which the oracle does not keep.
    /// A write made with the bus's leftover word has [`LEFTOVER`] set in its register.
    fn custom_writes(&self) -> &[(u16, u16)];
    /// `vars`' values at each pass end of the last frame (each CPU write of [`PASS_END`]), in
    /// order; empty if no pass ended in it.
    fn pass_ends(&self, vars: &[&Var]) -> Vec<Vec<u32>>;
    /// Chip RAM then slow RAM at each pass end of the last frame (each CPU write of [`PASS_END`]),
    /// in order: memory in play for a case lined up by loads (T53). Empty if no pass ended in it.
    fn pass_end_ram(&self) -> &[Vec<u8>] {
        &[]
    }
    /// How the engine stopped (translator DESIGN §2.4: `untranslated: hN+$off` and the rest), once
    /// it has; no frame runs after it.
    fn stopped(&self) -> Option<String>;
    /// Chip RAM then slow RAM at the last frame's end, for `compare = ["memory"]`.
    fn ram(&self) -> Option<&[u8]> {
        None
    }
    /// The loads ([`crate::sync::LOAD_ROUTINES`]) that returned in the last frame, and whether one
    /// runs at its end: for `align = "loads"`.
    fn loads(&self) -> (u32, bool) {
        (0, false)
    }
    /// Before a power-on case's first frame: the raster line the original's entry ran on, where
    /// the engine starts its program ([`crate::sync::reach_entry`]).
    fn start_line(&mut self, _line: u16) {}
    /// Before the first frame: write `bytes` at `addr` as the program enters the routine at `at`
    /// (hunk, offset), once; a case's `set` with `when` (T33), which the runner makes on the
    /// original at the same routine (`Amiga::poke_at`). An engine that cannot refuses.
    fn write_at(&mut self, _at: (u8, u32), _addr: u32, _bytes: &[u8]) -> Result<(), String> {
        Err("this engine makes no write as a routine is entered".into())
    }
    /// How many [`FrameEngine::write_at`] writes the engine has not made by the last frame's end.
    fn writes_at_pending(&self) -> usize {
        0
    }
}

/// The channels a frame is judged on, and how many: state, Paula's writes, the picture, the other
/// custom writes, the hunks' memory.
const CHANNELS: usize = 5;

/// What one frame is judged on and against, and the run's judgement so far.
struct Judge<'a> {
    case: &'a Case,
    opts: &'a Options<'a>,
    vars: Vec<&'static Var>,
    rows: Vec<Range<u16>>,
    window: (u16, u16),
    tol: Tolerance,
    pictures: PictureJudge,
    /// The channels still compared. With `Options::every_channel` a channel that differed is
    /// dropped and the run goes on with the rest.
    live: [bool; CHANNELS],
    /// The last judged frame's memory differences, held for the next frame ([`one_frame_on`]).
    held: Option<Held>,
    /// Each side's RAM at its previous frame's end, set by the runner before [`Judge::frame`];
    /// without it no difference is held: every byte must be equal.
    before: Option<(Vec<u8>, AmigaMemory)>,
    /// In play (T53): memory is judged at pass ends ([`Judge::play`]), not at the frame's end.
    in_play: bool,
    /// Each side's RAM at its pass ends in play, not yet paired.
    theirs_ends: PassEnds<AmigaMemory>,
    ours_ends: PassEnds<Vec<u8>>,
    /// The other custom writes in play, cut into passes at each side's own [`PASS_END`] write and
    /// carried a pass ([`custom_carried`]), as a level case's are.
    custom_passes: [Passes; 2],
    carried: BTreeMap<(u16, u16), Carried>,
}

/// A frame's memory differences waiting for the next oracle frame (owner, T51): its pass and
/// frame, and each differing byte.
#[derive(Debug, Clone)]
struct Held {
    pass: u32,
    frame: u32,
    bytes: Vec<Byte>,
}

/// A byte differing at a frame's end: its address, the engine's and the original's value there,
/// and each side's at its previous frame's end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Byte {
    pub at: u32,
    pub engine: u8,
    pub original: u8,
    pub engine_before: u8,
    pub original_before: u8,
}

impl Held {
    /// Its bytes not one frame on by `next` (the next judged frame: number, engine's RAM,
    /// original's), all of them when that is not the very next frame or there is none.
    fn left_after(&self, next: Option<(u32, &[u8], &AmigaMemory)>) -> Vec<Byte> {
        match next {
            Some((f, ram, theirs)) if f == self.frame + 1 => one_frame_on(&self.bytes, ram, theirs),
            _ => self.bytes.clone(),
        }
    }
}

impl<'a> Judge<'a> {
    fn new(case: &'a Case, opts: &'a Options<'a>, window: (u16, u16)) -> Result<Self, String> {
        let rows = case
            .exclude
            .iter()
            .filter(|e| e.var.is_none() && e.cols.is_none())
            .map(|e| e.rows.clone())
            .collect();
        if case
            .exclude
            .iter()
            .any(|e| e.var.is_some() || e.cols.is_some())
        {
            return Err(format!(
                "{}: the frame runner leaves out whole rows only; a box or a variable exclusion \
                 is the pass runner's",
                case.name
            ));
        }
        let tol = Tolerance::OWNER;
        let w = case.compare.writes;
        Ok(Judge {
            case,
            opts,
            vars: layout_vars(&case.vars)?,
            rows,
            window,
            tol,
            pictures: PictureJudge::new(tol, case.frames),
            live: [
                case.compare.vars,
                w,
                case.compare.picture,
                w,
                case.compare.memory,
            ],
            held: None,
            before: None,
            in_play: false,
            theirs_ends: PassEnds::default(),
            ours_ends: PassEnds::default(),
            custom_passes: [Passes::default(), Passes::default()],
            carried: BTreeMap::new(),
        })
    }

    /// The held frame's memory differences judged by the frame after (`frame`, `ram` the engine's,
    /// `theirs` the original's): `Some` naming the held frame when a byte is not one frame on.
    fn settle(
        &mut self,
        frame: Option<(u32, &[u8], &AmigaMemory)>,
        verdict: &mut Verdict,
    ) -> Option<Outcome> {
        let held = self.held.take()?;
        let left = held.left_after(frame);
        match left.first() {
            None => {
                verdict.memory_frames += 1;
                verdict.memory_one_on += 1;
                None
            }
            Some(&Byte {
                at,
                engine,
                original,
                ..
            }) => Some(Outcome::Differs {
                pass: held.pass,
                frame: held.frame,
                var: format!(
                    "{} ({} bytes differ, {} not one frame on)",
                    hunk_name(at),
                    held.bytes.len(),
                    left.len()
                ),
                engine: engine as u32,
                oracle: original as u32,
            }),
        }
    }

    /// At the run's end: a frame still held has no frame after it to be judged by, and fails.
    fn finish(&mut self, verdict: &mut Verdict) {
        if self.live[4]
            && let Some(o) = self.settle(None, verdict)
        {
            self.live[4] = false;
            verdict.firsts.push(o);
        }
    }

    /// Judge the frame the engine just ran against the oracle's last step (`capture`, numbered
    /// `frame`, the engine's `pass`-th). `writes` is false on a frame whose writes are not the
    /// program's alone on the original. Every channel's first difference is returned, by channel.
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        engine: &dyn FrameEngine,
        am: &Amiga,
        capture: &FrameCapture,
        pass: u32,
        frame: u32,
        writes: bool,
        verdict: &mut Verdict,
    ) -> Result<Vec<(usize, Outcome)>, String> {
        let (case, opts, tol) = (self.case, self.opts, self.tol);
        let theirs = Live(am);
        let mut differs: Vec<(usize, Outcome)> = Vec::new();
        if self.live[0] {
            let first = self.vars.iter().find_map(|v| {
                let (ours, theirs) = (engine.probe(v), theirs.value(v));
                (ours != theirs).then(|| Outcome::Differs {
                    pass,
                    frame,
                    var: v.name.to_string(),
                    engine: ours,
                    oracle: theirs,
                })
            });
            match first {
                Some(o) => differs.push((0, o)),
                None => verdict.passes += 1,
            }
        }
        if self.live[4] {
            let ram = engine
                .ram()
                .ok_or("the case compares memory but the engine gives none")?;
            let theirs = am.memory();
            // A frame's end may fall a few instructions apart on the two sides while the CPU
            // writes (owner, T51): its differences pass if each byte is one frame on.
            if let Some(o) = self.settle(Some((frame, ram, &theirs)), verdict) {
                differs.push((4, o));
            } else if self.in_play {
                // Judged at the pass ends instead (`Judge::play`).
            } else {
                let d = memory_differences(ram, &theirs);
                match (&self.before, d.first()) {
                    (_, None) => verdict.memory_frames += 1,
                    (Some((ours_before, theirs_before)), _) => {
                        let chip = theirs_before.chip.len();
                        let bytes = d
                            .iter()
                            .map(|&(at, engine, original)| Byte {
                                at,
                                engine,
                                original,
                                engine_before: byte_ours(ours_before, chip, at),
                                original_before: theirs_before.byte_at(at).unwrap_or(0),
                            })
                            .collect();
                        self.held = Some(Held { pass, frame, bytes });
                    }
                    (None, Some(&(at, engine, original))) => differs.push((
                        4,
                        Outcome::Differs {
                            pass,
                            frame,
                            var: format!("{} ({} bytes differ)", hunk_name(at), d.len()),
                            engine: engine as u32,
                            oracle: original as u32,
                        },
                    )),
                }
            }
        }
        if self.live[1] && writes {
            let theirs: Vec<(u16, u16)> = capture.paula.iter().map(|w| (w.reg, w.value)).collect();
            let ours: Vec<(u16, u16)> = engine.paula().iter().map(|w| (w.reg, w.value)).collect();
            match paula(
                &as_paula(&leftover_in_order(&ours, &theirs)),
                &capture.paula,
            ) {
                Some(why) => differs.push((1, Outcome::WritesDiffer { pass, frame, why })),
                None => {
                    verdict.writes += capture.paula.len() as u32;
                    verdict.write_frames += 1;
                }
            }
        }
        if self.live[2] {
            let pic = am.picture_in(self.window.0, self.window.1);
            let whole = engine
                .picture()
                .ok_or("the case compares the picture but the engine draws none")?;
            let ours = whole.rewindow(pic.left as i32, pic.top, pic.width, pic.height);
            let colours = to_rgb12(&ours);
            for (i, e) in case.exclude.iter().enumerate() {
                let differs = (e.rows.start..e.rows.end.min(pic.height)).any(|y| {
                    (0..pic.width).any(|x| colours[(y * pic.width + x) as usize] != pic.pixel(x, y))
                });
                verdict.excluded[i].1 += differs as u32;
            }
            let class = classify(&ours, &pic, &self.rows, &tol);
            let failed = match opts.mode {
                _ if class == PictureClass::Same => self.pictures.frame(frame, class).err(),
                Mode::Strict => Some(format!("{class:?}, strict")),
                Mode::Faithful => self.pictures.frame(frame, class).err(),
            };
            verdict.pictures += 1;
            verdict.tolerated = [
                self.pictures.colour_step.len() as u32,
                self.pictures.few_pixels.len() as u32,
                self.pictures.glitches.len() as u32,
            ];
            if let Some(mut why) = failed {
                let shown = picture_except(&ours, &pic, &self.rows);
                if let Some((x, y, o, e)) = shown.as_ref().and_then(|m| m.first) {
                    why += &format!(
                        "; first at ({x}, {y}): {o:03x} in the original, {e:03x} in the engine"
                    );
                }
                if let (Some(dir), Some(m)) = (&opts.diff_dir, shown) {
                    let d = report::write_diff_png(dir, &case.name, frame, &ours, &pic, m);
                    why += &format!("; {}", d.path.display());
                }
                differs.push((2, Outcome::PictureDiffers { pass, frame, why }));
            }
        }
        if self.live[3] && writes && self.in_play {
            let theirs: Vec<(u16, u16)> = am
                .cpu_writes()
                .iter()
                .map(|w| (w.reg & 0x1FE, w.value))
                .collect();
            if let Some(o) = self.custom_in_play(frame, &theirs, engine.custom_writes(), verdict) {
                differs.push((3, o));
            }
        } else if self.live[3] && writes {
            let theirs: Vec<(u16, u16)> = am
                .cpu_writes()
                .iter()
                .map(|w| (w.reg & 0x1FE, w.value))
                .collect();
            match custom_difference(&leftover_counted(engine.custom_writes(), &theirs), &theirs) {
                Some(d) => differs.push((
                    3,
                    Outcome::CustomWritesDiffer {
                        pass,
                        frame,
                        reg: d.reg,
                        value: d.value,
                        original: d.original,
                        engine: d.engine,
                    },
                )),
                None => {
                    verdict.custom_writes +=
                        theirs.iter().filter(|(r, _)| !is_paula(*r)).count() as u32;
                    verdict.custom_frames += 1;
                }
            }
        }
        Ok(differs)
    }

    /// Whether memory is judged at pass ends from this compared frame on (owner, T30's rule in play,
    /// T53): once either side has ended a pass with the program holding the machine (`owned`, the
    /// level 3 vector its handler at the step's start and end on the original). The original's
    /// `game_frame` returns after the frame's end on almost every moving frame (docs/09), so a frame's
    /// end in play is mid-pass on both sides, at points a machine of another pace does not share.
    /// Before play (the title, SCENA) a frame's end stays the judge. Leaving the machine (a load)
    /// ends play and drops the pass ends not yet paired.
    fn enter_play(&mut self, owned: bool, ended: bool) {
        if !owned {
            self.leave_play();
        } else if ended && self.live[4] {
            self.in_play = true;
        }
    }

    fn leave_play(&mut self) {
        self.in_play = false;
        self.theirs_ends = PassEnds::default();
        self.ours_ends = PassEnds::default();
        self.custom_passes = [Passes::default(), Passes::default()];
        self.carried.clear();
    }

    /// Memory in play (T53): each side's RAM at its pass ends in this frame (`theirs` the
    /// original's, the step's first only; `ours` the engine's), paired by `irq_count` as
    /// [`run_frames_inputs`] pairs a level case's state, a pass allowed to end [`PASS_LAG`] frames
    /// apart. The first pass whose hunks differ, or that ended too far apart, is returned.
    fn play(
        &mut self,
        theirs: Option<AmigaMemory>,
        ours: &[Vec<u8>],
        frame: u32,
        verdict: &mut Verdict,
    ) -> Option<Outcome> {
        if !self.in_play || !self.live[4] {
            return None;
        }
        let irq = layout_var("irq_count").expect("irq_count in LAYOUT");
        if let Some(m) = theirs {
            let k = OracleMem(&m).value(irq);
            self.theirs_ends.add(frame, k, m);
        }
        for ram in ours {
            self.ours_ends.add(frame, ram_value(ram, irq), ram.clone());
        }
        while let Some((k, (at, theirs), (ours_at, ours))) = pair_ends(
            &mut self.theirs_ends,
            &mut self.ours_ends,
            &mut verdict.unseen,
        ) {
            if let Some(o) = too_far(k, at, ours_at, &format!("memory at irq_count {k}")) {
                return Some(o);
            }
            let d = memory_differences(&ours, &theirs);
            if let Some(&(a, engine, original)) = d.first() {
                return Some(Outcome::Differs {
                    pass: k,
                    frame: at,
                    var: format!(
                        "{} ({} bytes differ at the pass end, irq_count {k})",
                        hunk_name(a),
                        d.len()
                    ),
                    engine: engine as u32,
                    oracle: original as u32,
                });
            }
            verdict.memory_passes += 1;
        }
        ends_apart(&self.theirs_ends, &self.ours_ends, frame)
    }

    /// The other custom writes in play (T53): cut into passes at each side's [`PASS_END`] write,
    /// paired in order, a pass allowed to end [`PASS_LAG`] frames apart and a write to fall a pass
    /// early or late ([`custom_carried`]), as [`run_frames_inputs`] judges a level case's.
    fn custom_in_play(
        &mut self,
        frame: u32,
        theirs: &[(u16, u16)],
        ours: &[(u16, u16)],
        verdict: &mut Verdict,
    ) -> Option<Outcome> {
        let p = &mut self.custom_passes;
        p[0].add(frame, theirs);
        p[1].add(frame, ours);
        while let Some((n, at, ours_at, theirs, ours)) = Passes::pair(p) {
            if let Some(o) = too_far(n, at, ours_at, "custom writes") {
                return Some(o);
            }
            match custom_carried(
                &mut self.carried,
                &leftover_counted(&ours, &theirs),
                &theirs,
            ) {
                Some(d) => {
                    return Some(Outcome::CustomWritesDiffer {
                        pass: n,
                        frame: at,
                        reg: d.reg,
                        value: d.value,
                        original: d.original,
                        engine: d.engine,
                    });
                }
                None => {
                    verdict.custom_writes +=
                        theirs.iter().filter(|(r, _)| !is_paula(*r)).count() as u32;
                    verdict.custom_frames += 1;
                }
            }
        }
        Passes::apart(p, frame, "custom writes")
    }

    /// Record `differs`: each channel's first goes into `verdict.firsts` and the channel is no
    /// longer compared. Whether the run ends here.
    fn record(&mut self, differs: Vec<(usize, Outcome)>, verdict: &mut Verdict) -> bool {
        for (k, o) in differs {
            self.live[k] = false;
            verdict.firsts.push(o);
            if !self.opts.every_channel || !self.live.contains(&true) {
                return true;
            }
        }
        false
    }
}

/// Run `case` on `engine`: a level case judged at its pass ends ([`run_frames_inputs`]), a title
/// case frame by frame ([`run_title_frames`]), a power-on case lined up by loads
/// ([`run_entry_frames`]).
pub fn run_frames(
    case: &Case,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    let inputs = Inputs::load(case)?;
    if case.is_entry() {
        return run_entry_frames(case, &inputs, engine, opts);
    }
    run_frames_inputs(case, &inputs, engine, opts)
}

/// [`run_frames`] on `inputs` rather than the scripts the case names, for a level or title case.
pub fn run_frames_inputs(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    if case.is_entry() {
        return Err(format!(
            "{}: a power-on case runs with `run_entry_frames`",
            case.name
        ));
    }
    if !case.is_level() {
        return run_title_frames(case, inputs, engine, opts);
    }
    let vars = layout_vars(&case.vars)?;
    let goal: Vec<(&Var, u32)> = match &case.goal {
        Some(Goal::All { all, .. }) => all
            .iter()
            .map(|(v, value)| Ok((layout_vars(std::slice::from_ref(v))?[0], *value)))
            .collect::<Result<_, String>>()?,
        _ => Vec::new(),
    };
    let rows: Vec<std::ops::Range<u16>> = case
        .exclude
        .iter()
        .filter(|e| e.var.is_none() && e.cols.is_none())
        .map(|e| e.rows.clone())
        .collect();
    if case
        .exclude
        .iter()
        .any(|e| e.var.is_some() || e.cols.is_some())
    {
        return Err(format!(
            "{}: the frame runner leaves out whole rows only; a box or a variable exclusion is the \
             pass runner's",
            case.name
        ));
    }
    let scene_pcs =
        [GAME_OVER_SCREEN, HIGH_SCORES, NEXT_WORLD].map(|(h, o)| HUNK_BASES[h as usize] + o);
    let mut pcs: Vec<u32> = opts.register.iter().map(crate::coverage::address).collect();
    pcs.extend(scene_pcs);
    let mut am = reach_case(case, 0)?;
    am.watch_pcs(&pcs);
    am.watch_write(Some(PASS_END));

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    let tol = Tolerance::OWNER;
    let mut judge = PictureJudge::new(tol, case.frames);
    // Transplanted yet, and the engine frames run since.
    let mut started = false;
    let mut run = 0u32;
    // The channels still compared: state, Paula's writes, the picture, the other custom writes.
    // With `Options::every_channel` a channel that differed is dropped and the run goes on with
    // the rest.
    let w = case.compare.writes;
    let mut live = [case.compare.vars, w, case.compare.picture, w];
    // What is read at a pass end: the case's vars, the goal's, and `irq_count`, which names it.
    let irq = layout_var("irq_count").ok_or("irq_count is not in LAYOUT")?;
    let mut probe: Vec<&Var> = vars.clone();
    probe.extend(goal.iter().map(|(v, _)| *v));
    probe.push(irq);
    let at_irq = probe.len() - 1;
    let (mut theirs_ends, mut ours_ends) = (PassEnds::default(), PassEnds::default());
    let mut start_irq = 0u32;
    let mut paula_passes = [Passes::default(), Passes::default()];
    let mut custom_passes = [Passes::default(), Passes::default()];
    let mut carried: BTreeMap<(u16, u16), Carried> = BTreeMap::new();

    'steps: for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        let capture = am.step(input).map_err(|e| e.to_string())?;
        let frame = am.frame();
        verdict.coverage.add(opts.register, am.pc_hits());
        if !started {
            if let Some(mem) = am.watched_memory() {
                engine.transplant(&OracleMem(&mem));
                start_irq = OracleMem(&mem).value(irq);
                started = true;
            }
            continue;
        }
        engine.frame(input);
        run += 1;
        if let Some(stop) = engine.stopped() {
            verdict.firsts.push(Outcome::Stopped {
                pass: run,
                frame,
                stop,
            });
            break;
        }
        let mut differs: Vec<(usize, Outcome)> = Vec::new();

        // State, at each pass end both sides have reached.
        if let Some(mem) = am.watched_memory() {
            let m = OracleMem(&mem);
            let values: Vec<u32> = probe.iter().map(|v| m.value(v)).collect();
            theirs_ends.add(frame, values[at_irq], values);
        }
        for values in engine.pass_ends(&probe) {
            ours_ends.add(frame, values[at_irq], values);
        }
        while let Some((k, theirs, ours)) =
            pair_ends(&mut theirs_ends, &mut ours_ends, &mut verdict.unseen)
        {
            let pass = k.wrapping_sub(start_irq);
            let at = theirs.0;
            if live[0]
                && let Some(o) = too_far(pass, theirs.0, ours.0, &format!("irq_count {k}"))
            {
                differs.push((0, o));
                break;
            }
            if live[0] {
                let first = (0..vars.len())
                    .find(|&i| ours.1[i] != theirs.1[i])
                    .map(|i| Outcome::Differs {
                        pass,
                        frame: at,
                        var: vars[i].name.to_string(),
                        engine: ours.1[i],
                        oracle: theirs.1[i],
                    });
                match first {
                    Some(o) => differs.push((0, o)),
                    None => verdict.passes += 1,
                }
            }
            if let Some(Goal::All { by_frame, .. }) = &case.goal
                && verdict.goal_met.is_none()
                && at <= *by_frame
                && goal.iter().enumerate().all(|(i, (_, value))| {
                    let i = vars.len() + i;
                    ours.1[i] == *value && theirs.1[i] == *value
                })
            {
                verdict.goal_met = Some(at);
            }
        }
        if live[0]
            && let Some(o) = ends_apart(&theirs_ends, &ours_ends, frame)
        {
            differs.push((0, o));
        }

        // The writes, cut into passes at each side's own PASS_END write.
        if live[1] {
            let theirs: Vec<(u16, u16)> = capture.paula.iter().map(|w| (w.reg, w.value)).collect();
            let ours: Vec<(u16, u16)> = engine.paula().iter().map(|w| (w.reg, w.value)).collect();
            paula_passes[0].add(frame, &theirs);
            paula_passes[1].add(frame, &ours);
            while let Some((n, at, ours_at, theirs, ours)) = Passes::pair(&mut paula_passes) {
                if let Some(o) = too_far(n, at, ours_at, "Paula's writes") {
                    differs.push((1, o));
                    break;
                }
                match paula(
                    &as_paula(&leftover_in_order(&ours, &theirs)),
                    &as_paula(&theirs),
                ) {
                    Some(why) => {
                        differs.push((
                            1,
                            Outcome::WritesDiffer {
                                pass: n,
                                frame: at,
                                why,
                            },
                        ));
                        break;
                    }
                    None => {
                        verdict.writes += theirs.len() as u32;
                        verdict.write_frames += 1;
                    }
                }
            }
            if !differs.iter().any(|(k, _)| *k == 1)
                && let Some(o) = Passes::apart(&paula_passes, frame, "Paula's writes")
            {
                differs.push((1, o));
            }
        }
        if live[3] {
            let theirs: Vec<(u16, u16)> = am
                .cpu_writes()
                .iter()
                .map(|w| (w.reg & 0x1FE, w.value))
                .collect();
            custom_passes[0].add(frame, &theirs);
            custom_passes[1].add(frame, engine.custom_writes());
            while let Some((n, at, ours_at, theirs, ours)) = Passes::pair(&mut custom_passes) {
                if let Some(o) = too_far(n, at, ours_at, "custom writes") {
                    differs.push((3, o));
                    break;
                }
                match custom_carried(&mut carried, &leftover_counted(&ours, &theirs), &theirs) {
                    Some(d) => {
                        differs.push((
                            3,
                            Outcome::CustomWritesDiffer {
                                pass: n,
                                frame: at,
                                reg: d.reg,
                                value: d.value,
                                original: d.original,
                                engine: d.engine,
                            },
                        ));
                        break;
                    }
                    None => {
                        verdict.custom_writes +=
                            theirs.iter().filter(|(r, _)| !is_paula(*r)).count() as u32;
                        verdict.custom_frames += 1;
                    }
                }
            }
            if !differs.iter().any(|(k, _)| *k == 3)
                && let Some(o) = Passes::apart(&custom_passes, frame, "custom writes")
            {
                differs.push((3, o));
            }
        }

        // The picture, at the frame's end: what the frame showed.
        if live[2] {
            let pass = run;
            let pic = am.picture_in(PLAY_WINDOW.0, PLAY_WINDOW.1);
            let whole = engine
                .picture()
                .ok_or("the case compares the picture but the engine draws none")?;
            let ours = whole.rewindow(pic.left as i32, pic.top, pic.width, pic.height);
            let colours = to_rgb12(&ours);
            for (i, e) in case.exclude.iter().enumerate() {
                let differs = (e.rows.start..e.rows.end.min(pic.height)).any(|y| {
                    (0..pic.width).any(|x| colours[(y * pic.width + x) as usize] != pic.pixel(x, y))
                });
                verdict.excluded[i].1 += differs as u32;
            }
            let class = classify(&ours, &pic, &rows, &tol);
            let failed = match opts.mode {
                _ if class == PictureClass::Same => judge.frame(frame, class).err(),
                Mode::Strict => Some(format!("{class:?}, strict")),
                Mode::Faithful => judge.frame(frame, class).err(),
            };
            verdict.pictures += 1;
            verdict.tolerated = [
                judge.colour_step.len() as u32,
                judge.few_pixels.len() as u32,
                judge.glitches.len() as u32,
            ];
            if let Some(mut why) = failed {
                let shown = picture_except(&ours, &pic, &rows);
                if let Some((x, y, o, e)) = shown.as_ref().and_then(|m| m.first) {
                    why += &format!(
                        "; first at ({x}, {y}): {o:03x} in the original, {e:03x} in the engine"
                    );
                }
                if let (Some(dir), Some(m)) = (&opts.diff_dir, shown) {
                    let d = report::write_diff_png(dir, &case.name, frame, &ours, &pic, m);
                    why += &format!("; {}", d.path.display());
                }
                differs.push((2, Outcome::PictureDiffers { pass, frame, why }));
            }
        }
        differs.sort_by_key(|(k, _)| *k);
        for (k, o) in differs {
            live[k] = false;
            verdict.firsts.push(o);
            if !opts.every_channel || !live.contains(&true) {
                break 'steps;
            }
        }
        // The scenes are the program's own on both sides, so they are judged by the frame's state
        // and picture above; here they only end the comparison where the pass runner ends it.
        let ran = |pc: u32| am.pc_hits().iter().any(|&(p, n)| p == pc && n > 0);
        if ran(scene_pcs[2]) {
            verdict.level_end = Some(frame);
            break;
        }
        if ran(scene_pcs[0]) && verdict.game_over.is_none() {
            verdict.game_over = Some((frame, None));
        }
        if ran(scene_pcs[1]) {
            verdict.game_over = Some((verdict.game_over.map_or(frame, |g| g.0), Some(frame)));
            break;
        }
    }
    let nothing = if !started {
        Some("no pass of the original ended after the sync point")
    } else if run == 0 {
        Some("the engine ran no frame after the transplant")
    } else {
        None
    };
    Ok(conclude(case, verdict, nothing))
}

/// A title case (no `start`, not the level script) judged at every frame's end, from the end of
/// its sync frame's step; the level parts serve a level case too, though [`run_frames_inputs`]
/// judges those at their pass ends (T30).
pub fn run_title_frames(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    if case.is_entry() {
        return Err(format!(
            "{}: a power-on case runs with `run_entry_frames`",
            case.name
        ));
    }
    let level = case.is_level();
    let goal = goal_vars(case)?;
    let window = if level { PLAY_WINDOW } else { SCREEN_WINDOW };
    let mut judge = Judge::new(case, opts, window)?;
    let scene_pcs =
        [GAME_OVER_SCREEN, HIGH_SCORES, NEXT_WORLD].map(|(h, o)| HUNK_BASES[h as usize] + o);
    let mut pcs: Vec<u32> = opts.register.iter().map(crate::coverage::address).collect();
    pcs.extend(scene_pcs);
    let mut am = if level {
        reach_case(case, 0)?
    } else {
        reach_title(case, inputs)?
    };
    am.watch_pcs(&pcs);
    if level {
        am.watch_write(Some(PASS_END));
    }

    let mut verdict = Verdict::new(opts.register);
    verdict.excluded = case.exclude.iter().map(|e| (e.clone(), 0)).collect();
    // Transplanted yet (a title case's engine starts at the sync frame), and the frames run since.
    let mut started = !level;
    let mut run = 0u32;

    for step in 0..case.frames {
        let input = inputs.at(case.sync_frame + step);
        let capture = am.step(input).map_err(|e| e.to_string())?;
        let frame = am.frame();
        verdict.coverage.add(opts.register, am.pc_hits());
        if !started {
            if let Some(mem) = am.watched_memory() {
                engine.transplant(&OracleMem(&mem));
                started = true;
                am.watch_write(None);
            }
            continue;
        }
        engine.frame(input);
        run += 1;
        // `pass` in an outcome counts the engine's frames from the transplant.
        let pass = run;
        if let Some(stop) = engine.stopped() {
            verdict.firsts.push(Outcome::Stopped { pass, frame, stop });
            break;
        }
        let differs = judge.frame(&*engine, &am, &capture, pass, frame, true, &mut verdict)?;
        if judge.record(differs, &mut verdict) {
            break;
        }
        goal_check(case, &goal, &*engine, &am, frame, &mut verdict);
        // The scenes are the program's own on both sides, so they are judged by the frame's state
        // and picture above; here they only end the comparison where the pass runner ends it.
        let ran = |pc: u32| am.pc_hits().iter().any(|&(p, n)| p == pc && n > 0);
        if ran(scene_pcs[2]) {
            verdict.level_end = Some(frame);
            break;
        }
        if ran(scene_pcs[0]) && verdict.game_over.is_none() {
            verdict.game_over = Some((frame, None));
        }
        if ran(scene_pcs[1]) {
            verdict.game_over = Some((verdict.game_over.map_or(frame, |g| g.0), Some(frame)));
            break;
        }
    }
    let nothing = if !started {
        Some("no pass of the original ended after the sync point")
    } else if run == 0 {
        Some("the engine ran no frame after the transplant")
    } else {
        None
    };
    Ok(conclude(case, verdict, nothing))
}

/// One side's stretch after a load while the OS still has the machine (owner, T51): fed each frame
/// the loads returned in it and whether the program has the machine at its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OsTail {
    owned: bool,
    after_load: bool,
}

impl OsTail {
    /// `owned`: whether the program has the machine where this side starts.
    pub fn new(owned: bool) -> OsTail {
        OsTail {
            owned,
            after_load: false,
        }
    }

    /// The frame's loads returned and ownership at its end: (1 if the program took the machine
    /// back after a load in it, else 0; whether the frame ends in the OS's stretch after a load).
    /// A taking back with no load before it (the entry installing its handlers) is not counted.
    pub fn frame(&mut self, returned: u32, owned: bool) -> (u32, bool) {
        let after_load = self.after_load || returned > 0;
        let back = (!self.owned && owned && after_load) as u32;
        self.owned = owned;
        self.after_load = after_load && !owned;
        (back, self.after_load)
    }
}

/// Whether `ram` (chip RAM then slow) holds the program's handler in the level 3 vector; no RAM
/// counts as the program's.
fn owns(ram: Option<&[u8]>) -> bool {
    let v = sync::LEVEL3_VECTOR as usize;
    ram.and_then(|m| m.get(v..v + 4)).is_none_or(|b| {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
            == HUNK_BASES[sync::LEVEL3_HANDLER.0 as usize] + sync::LEVEL3_HANDLER.1
    })
}

/// Case `case`, lined up by loads (`align = "loads"`), on `engine`. A power-on case's
/// (`start = "entry"`) engine starts at the program's entry, paired with the oracle step that
/// first runs it; any other's starts from a snapshot at the end of its sync frame's step, paired
/// with the next (T33), its `set` made on both sides as the program enters each row's routine
/// (`Amiga::poke_at`, [`FrameEngine::write_at`]). A write either side never made fails the case.
pub fn run_entry_frames(
    case: &Case,
    inputs: &Inputs,
    engine: &mut dyn FrameEngine,
    opts: &Options,
) -> Result<Verdict, String> {
    if !case.by_loads() {
        return Err(format!("{}: not lined up by loads", case.name));
    }
    let end = case.sync_frame + case.frames;
    let goal = goal_vars(case)?;
    let mut judge = Judge::new(case, opts, SCREEN_WINDOW)?;
    let h = |(hunk, off): (u8, u32)| HUNK_BASES[hunk as usize] + off;
    let entries: Vec<u32> = LOAD_ROUTINES.iter().copied().map(h).collect();
    let returns: Vec<u32> = LOAD_RETURNS.iter().copied().map(h).collect();
    let mut pcs: Vec<u32> = opts.register.iter().map(crate::coverage::address).collect();
    pcs.extend(&entries);
    pcs.extend(&returns);
    let (mut am, line) = if case.is_entry() {
        sync::reach_entry(case, inputs, &pcs)?
    } else {
        let mut am = reach_title(case, inputs)?;
        am.watch_pcs(&pcs);
        (am, 0)
    };
    if case.is_entry() {
        engine.start_line(line);
    }
    // Memory in play is judged at pass ends (T53).
    am.watch_write(Some(PASS_END));
    for s in &case.set {
        let w = s
            .when
            .as_ref()
            .ok_or("a `set` lined up by loads has a `when`")?;
        let (bytes, addr) = (s.bytes(), s.address()?);
        am.poke_at(h(w.at), addr, &bytes);
        engine.write_at(w.at, addr, &bytes)?;
    }
    let handler = h(sync::LEVEL3_HANDLER);
    let program_owns = |am: &Amiga| {
        let v = am.peek(sync::LEVEL3_VECTOR, 4);
        u32::from_be_bytes([v[0], v[1], v[2], v[3]]) == handler
    };
    // Loads entered and returned on the original since the entry.
    let (mut entered, mut returned) = (0u32, 0u32);
    let mut count = |am: &Amiga| {
        let hits = |list: &[u32]| -> u32 {
            am.pc_hits()
                .iter()
                .filter(|(pc, _)| list.contains(pc))
                .map(|&(_, n)| n)
                .sum()
        };
        let r = hits(&returns);
        entered += hits(&entries);
        returned += r;
        (r, entered > returned)
    };
    let mut verdict = Verdict::new(opts.register);
    if case.is_entry() {
        verdict.entry = Some((am.frame(), line));
    }
    let mut align = ByReturns::new(LOAD_CAP);
    // Both sides start from the same state: the snapshot's, or before the entry the OS's.
    let mut os_tail = OsTail::new(program_owns(&am));
    let mut engine_tail = os_tail;
    // The original's loads returned, without the takings back `align` also counts.
    let mut load_count = 0u32;
    // The step just run: its input, its capture, its first pass end's RAM, whether the program
    // had the machine before it.
    let mut input = inputs.at(am.frame() - 1);
    let mut capture = FrameCapture::default();
    let mut orig_end: Option<AmigaMemory> = None;
    let mut owned_before = false;
    // Each side's RAM at its previous frame's end, for a memory difference one frame on (T51).
    let keep = case.compare.memory;
    let mut orig_before: Option<AmigaMemory> = None;
    let mut run = 0u32;
    let step = |am: &mut Amiga| -> Result<(FrameInput, FrameCapture), String> {
        let frame = am.frame();
        if let Some(s) = case.swap.filter(|s| s.frame == frame) {
            am.insert_disk(0, s.disk as usize - 1);
        }
        let input = inputs.at(frame);
        let c = am.step(input).map_err(|e| e.to_string())?;
        Ok((input, c))
    };
    if !case.is_entry() {
        // The engine starts where the snapshot was taken, the sync frame's step's end: its first
        // frame is the next step's.
        owned_before = program_owns(&am);
        orig_before = keep.then(|| am.memory());
        (input, capture) = step(&mut am)?;
        orig_end = am.watched_memory();
        verdict.coverage.add(opts.register, am.pc_hits());
    }
    // The first step's own hits: nothing loads before the entry.
    let (r, l) = count(&am);
    let (back, tail) = os_tail.frame(r, program_owns(&am));
    load_count += r;
    align.original(r + back, l || tail);
    loop {
        let eng_before = keep.then(|| engine.ram().map(<[u8]>::to_vec)).flatten();
        engine.frame(input);
        run += 1;
        let pass = run;
        let frame = am.frame();
        if let Some(stop) = engine.stopped() {
            verdict.firsts.push(Outcome::Stopped { pass, frame, stop });
            break;
        }
        let (r, l) = engine.loads();
        let (back, tail) = engine_tail.frame(r, owns(engine.ram()));
        if tail {
            verdict.tail_frames += 1;
        }
        align.engine(r + back, l || tail);
        // Loads apart ends the run whatever `every_channel` says: nothing after it lines up. A
        // channel's earlier difference does not (`firsts` holds those with `every_channel`).
        let mut apart = false;
        while align.behind() {
            if let Err(e) = align.skip() {
                apart = true;
                verdict.firsts.push(Outcome::LoadApart {
                    pass,
                    frame: am.frame(),
                    why: format!("load {}: {e}", align.returned().0),
                });
                break;
            }
            if am.frame() >= end {
                break;
            }
            owned_before = program_owns(&am);
            // The engine has run past this step's input: its next frame takes the next step's.
            orig_before = keep.then(|| am.memory());
            (_, capture) = step(&mut am)?;
            orig_end = am.watched_memory();
            verdict.coverage.add(opts.register, am.pc_hits());
            let (r, l) = count(&am);
            let (back, tail) = os_tail.frame(r, program_owns(&am));
            load_count += r;
            align.original(r + back, l || tail);
        }
        if apart || align.behind() {
            break;
        }
        let frame = am.frame();
        if !align.compared() {
            judge.leave_play();
        } else {
            verdict.aligned_frames += 1;
            let writes = owned_before && program_owns(&am);
            if !writes {
                verdict.os_frames += 1;
            }
            judge.before = eng_before.zip(orig_before.take());
            let ended = orig_end.is_some() || !engine.pass_end_ram().is_empty();
            judge.enter_play(writes, ended);
            let mut differs =
                judge.frame(&*engine, &am, &capture, pass, frame, writes, &mut verdict)?;
            if !differs.iter().any(|(k, _)| *k == 4)
                && let Some(o) =
                    judge.play(orig_end.take(), engine.pass_end_ram(), frame, &mut verdict)
            {
                differs.push((4, o));
                differs.sort_by_key(|(k, _)| *k);
            }
            if judge.record(differs, &mut verdict) {
                break;
            }
            goal_check(case, &goal, &*engine, &am, frame, &mut verdict);
        }
        if am.frame() >= end {
            break;
        }
        owned_before = program_owns(&am);
        orig_before = keep.then(|| am.memory());
        (input, capture) = step(&mut am)?;
        orig_end = am.watched_memory();
        verdict.coverage.add(opts.register, am.pc_hits());
        let (r, l) = count(&am);
        let (back, tail) = os_tail.frame(r, program_owns(&am));
        load_count += r;
        align.original(r + back, l || tail);
    }
    if verdict.firsts.is_empty() || opts.every_channel {
        judge.finish(&mut verdict);
    }
    verdict.loads = load_count;
    verdict.load_frames = align.skipped();
    // A `set` is what the case says it compares under: both sides must have made it.
    let pending = [am.pokes_at_pending(), engine.writes_at_pending()];
    if verdict.firsts.is_empty() && pending != [0, 0] {
        let sets: Vec<String> = case
            .set
            .iter()
            .map(|s| {
                format!(
                    "{} at {}",
                    s.var,
                    s.when.as_ref().map_or("?", |w| &w.routine)
                )
            })
            .collect();
        verdict.firsts.push(Outcome::ComparedNothing {
            what: format!(
                "the set {}: {} writes never made on the original, {} on the engine",
                sets.join(", "),
                pending[0],
                pending[1]
            ),
        });
    }
    if verdict.firsts.is_empty() && align.returned().0 != align.returned().1 {
        let (e, o) = align.returned();
        verdict.firsts.push(Outcome::LoadApart {
            pass: run,
            frame: am.frame(),
            why: format!(
                "the engine returned from {e} loads, the original from {o}, by the case's end"
            ),
        });
    }
    let nothing = (verdict.aligned_frames == 0).then_some("no frame outside a load on both sides");
    Ok(conclude(case, verdict, nothing))
}

/// The original from power-on to the end of title case `case`'s sync frame, its swap made on the way.
fn reach_title(case: &Case, inputs: &Inputs) -> Result<Amiga, String> {
    use amiga_oracle::{OracleConfig, kickstart_path};
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let cfg = OracleConfig::new(
        kickstart,
        crate::disks::load()?,
        case.sync_frame + case.frames + 1,
    );
    let mut am = Amiga::power_on(&cfg).map_err(|e| e.to_string())?;
    for frame in 0..case.sync_frame {
        if let Some(s) = case.swap.filter(|s| s.frame == frame) {
            am.insert_disk(0, s.disk as usize - 1);
        }
        am.step(inputs.at(frame)).map_err(|e| e.to_string())?;
    }
    Ok(am)
}

fn goal_vars(case: &Case) -> Result<Vec<(&'static Var, u32)>, String> {
    match &case.goal {
        Some(Goal::All { all, .. }) => all
            .iter()
            .map(|(v, value)| Ok((layout_vars(std::slice::from_ref(v))?[0], *value)))
            .collect(),
        _ => Ok(Vec::new()),
    }
}

/// Whether an `all` goal holds on both sides at this compared frame.
fn goal_check(
    case: &Case,
    goal: &[(&Var, u32)],
    engine: &dyn FrameEngine,
    am: &Amiga,
    frame: u32,
    verdict: &mut Verdict,
) {
    if let Some(Goal::All { by_frame, .. }) = &case.goal
        && verdict.goal_met.is_none()
        && frame <= *by_frame
        && goal
            .iter()
            .all(|(v, value)| engine.probe(v) == *value && Live(am).value(v) == *value)
    {
        verdict.goal_met = Some(frame);
    }
}

/// The outcome of a run that ended: its first difference, or what it compared nothing of
/// (`nothing`), or the goal it missed, or an exclusion it did not need.
fn conclude(case: &Case, mut verdict: Verdict, nothing: Option<&str>) -> Verdict {
    if let Some(first) = verdict.firsts.first() {
        verdict.outcome = first.clone();
        return verdict;
    }
    let none = |what: &str| {
        Some(Outcome::ComparedNothing {
            what: what.to_string(),
        })
    };
    let only_writes =
        case.compare.writes && !case.compare.vars && !case.compare.picture && !case.compare.memory;
    let failed = if let Some(what) = nothing {
        none(what)
    } else if only_writes && verdict.writes == 0 {
        none("neither side wrote to Paula in any compared frame")
    } else {
        match &case.goal {
            Some(Goal::All { all, by_frame }) if verdict.goal_met.is_none() => {
                let want: Vec<String> = all.iter().map(|(v, n)| format!("{v} = {n}")).collect();
                Some(Outcome::GoalMissed {
                    goal: format!("{} by oracle frame {by_frame}", want.join(", ")),
                })
            }
            Some(Goal::GameOver { by_frame }) => match verdict.game_over {
                Some((_, Some(left))) if left <= *by_frame => {
                    verdict.goal_met = Some(left);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!(
                        "the game over screen left for the high scores by oracle frame {by_frame}"
                    ),
                }),
            },
            Some(Goal::LevelEnd { by_frame }) => match verdict.level_end {
                Some(end) if end <= *by_frame => {
                    verdict.goal_met = Some(end);
                    None
                }
                _ => Some(Outcome::GoalMissed {
                    goal: format!("the level's end (`next_world`) by oracle frame {by_frame}"),
                }),
            },
            _ => None,
        }
    };
    let unused = || {
        verdict
            .excluded
            .iter()
            .find(|(_, n)| *n == 0)
            .map(|(e, _)| Outcome::ExclusionUnused {
                what: e.what.clone(),
                task: e.task.clone(),
            })
    };
    verdict.outcome = failed.or_else(unused).unwrap_or(Outcome::Same);
    verdict
}

/// The first byte of the program's hunks that differs between the engine's RAM (chip then slow)
/// and the original's, the OS's values left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryDifference {
    /// `hN+$off`.
    pub at: String,
    pub engine: u8,
    pub original: u8,
    /// How many compared bytes differ.
    pub count: u32,
}

/// [`MemoryDifference`] of `ours` against `theirs`; `None` if every compared byte is equal.
pub fn memory_difference(ours: &[u8], theirs: &AmigaMemory) -> Option<MemoryDifference> {
    let d = memory_differences(ours, theirs);
    d.first().map(|&(a, engine, original)| MemoryDifference {
        at: hunk_name(a),
        engine,
        original,
        count: d.len() as u32,
    })
}

/// Whether `(hunk, off)` is left out of the memory comparison: the OS's values and the beam's.
fn left_out(hunk: usize, off: u32) -> bool {
    OS_VALUES
        .iter()
        .chain(BEAM_VALUES.iter())
        .any(|&(h, o, n)| h as usize == hunk && (o..o + n).contains(&off))
}

/// `v`'s value in `ram` (chip RAM then slow RAM, as the original's is laid out), zero-extended.
fn ram_value(ram: &[u8], v: &Var) -> u32 {
    let at = HUNK_BASES[v.hunk as usize] + v.offset;
    (0..v.width.bytes() as u32).fold(0, |acc, k| acc << 8 | byte_ours(ram, CHIP, at + k) as u32)
}

/// The A500's chip RAM, which an engine's RAM holds before slow RAM.
const CHIP: usize = 0x8_0000;

/// `a` as `hN+$off`.
fn hunk_name(a: u32) -> String {
    (0..HUNK_BASES.len())
        .find(|&h| (HUNK_BASES[h]..HUNK_BASES[h] + HUNK_SIZES[h]).contains(&a))
        .map_or(format!("${a:06x}"), |h| {
            format!("h{h}+${:x}", a - HUNK_BASES[h])
        })
}

/// The engine's byte at `a` in `ours` (chip RAM then slow RAM, as the original's is laid out).
fn byte_ours(ours: &[u8], chip: usize, a: u32) -> u8 {
    let i = if a >= SLOW_BASE {
        chip + (a - SLOW_BASE) as usize
    } else {
        a as usize
    };
    ours.get(i).copied().unwrap_or(0)
}

/// Every compared byte of the hunks that differs, as (address, engine's, original's).
pub fn memory_differences(ours: &[u8], theirs: &AmigaMemory) -> Vec<(u32, u8, u8)> {
    let chip = theirs.chip.len();
    let mut out = Vec::new();
    for (h, &size) in HUNK_SIZES.iter().enumerate() {
        let base = HUNK_BASES[h];
        for off in 0..size {
            let a = base + off;
            let (o, t) = (byte_ours(ours, chip, a), theirs.byte_at(a).unwrap_or(0));
            if o != t && !left_out(h, off) {
                out.push((a, o, t));
            }
        }
    }
    out
}

/// Of a frame's differing bytes, those not one frame on given the next frame's RAM (`next_ours`
/// the engine's, `next_theirs` the original's). A byte is one frame on when one side's values
/// over the three frames are the other's a frame later: the engine ahead (its value is the
/// original's next, its previous the original's now), or behind (the original's value is the
/// engine's next, the original's previous the engine's now). The frame's end then fell a few
/// instructions apart in the same work; a one-frame glitch that reverts does not fit either.
pub fn one_frame_on(bytes: &[Byte], next_ours: &[u8], next_theirs: &AmigaMemory) -> Vec<Byte> {
    let chip = next_theirs.chip.len();
    bytes
        .iter()
        .copied()
        .filter(|b| {
            let (eng_next, ori_next) = (
                byte_ours(next_ours, chip, b.at),
                next_theirs.byte_at(b.at).unwrap_or(0),
            );
            let ahead = b.engine == ori_next && b.engine_before == b.original;
            let behind = b.original == eng_next && b.original_before == b.engine;
            !(ahead || behind)
        })
        .collect()
}

/// The first (register, value) whose count differs between two frames' custom writes, by register
/// then value; `None` if every count is equal. Paula's registers are left out (compared in order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustomDifference {
    pub reg: u16,
    pub value: u16,
    pub original: u32,
    pub engine: u32,
}

/// [`CustomDifference`] of `ours` (the engine's) against `theirs` (the original's), each
/// `(register, value)`.
pub fn custom_difference(ours: &[(u16, u16)], theirs: &[(u16, u16)]) -> Option<CustomDifference> {
    let mut counts: BTreeMap<(u16, u16), (u32, u32)> = BTreeMap::new();
    for &(reg, value) in theirs.iter().filter(|(r, _)| !is_paula(*r)) {
        counts.entry((reg, value)).or_default().0 += 1;
    }
    for &(reg, value) in ours.iter().filter(|(r, _)| !is_paula(*r)) {
        counts.entry((reg, value)).or_default().1 += 1;
    }
    counts
        .into_iter()
        .find(|(_, (o, e))| o != e)
        .map(|((reg, value), (original, engine))| CustomDifference {
            reg,
            value,
            original,
            engine,
        })
}

/// One side's pass ends not yet paired: `irq_count` → (oracle frame, what was read there: the
/// probed values, or the whole RAM).
struct PassEnds<T> {
    open: BTreeMap<u32, (u32, T)>,
    /// The highest `irq_count` this side has ended a pass at.
    last: Option<u32>,
    /// The original's side: the last pass paired, so a pass it never showed is counted unseen.
    paired: Option<u32>,
}

impl<T> Default for PassEnds<T> {
    fn default() -> Self {
        PassEnds {
            open: BTreeMap::new(),
            last: None,
            paired: None,
        }
    }
}

impl<T> PassEnds<T> {
    /// The pass ended at `irq_count` `k`, in oracle frame `frame`, `read` there.
    fn add(&mut self, frame: u32, k: u32, read: T) {
        self.last = Some(self.last.map_or(k, |l| l.max(k)));
        self.open.entry(k).or_insert((frame, read));
    }
}

/// The lowest pass both sides ended, (irq_count, the original's, the engine's). The oracle shows a
/// step's first pass end only (docs/09): a pass the engine ended that the original has gone past
/// unseen is dropped, and every pass the original skipped is counted in `unseen`.
#[allow(clippy::type_complexity)]
fn pair_ends<A, B>(
    theirs: &mut PassEnds<A>,
    ours: &mut PassEnds<B>,
    unseen: &mut u32,
) -> Option<(u32, (u32, A), (u32, B))> {
    if let Some(o) = theirs.last {
        ours.open
            .retain(|k, _| *k >= o || theirs.open.contains_key(k));
    }
    let k = *theirs.open.keys().find(|k| ours.open.contains_key(k))?;
    if let Some(p) = theirs.paired {
        *unseen += k.saturating_sub(p + 1);
    }
    theirs.paired = Some(k);
    Some((k, theirs.open.remove(&k)?, ours.open.remove(&k)?))
}

/// A pass one side ended more than [`PASS_LAG`] frames before `frame` that the other has not.
fn ends_apart<A, B>(theirs: &PassEnds<A>, ours: &PassEnds<B>, frame: u32) -> Option<Outcome> {
    let first = [
        theirs.open.iter().next().map(|(k, (at, _))| (*k, *at)),
        ours.open.iter().next().map(|(k, (at, _))| (*k, *at)),
    ];
    (0..2).find_map(|side| {
        let (k, at) = first[side]?;
        (frame > at + PASS_LAG).then(|| Outcome::PassEndsApart {
            pass: k,
            frame: at,
            why: format!(
                "irq_count {k} ended at oracle frame {at} in the {}, not by frame {} in the {}",
                SIDE[side],
                at + PASS_LAG,
                SIDE[1 - side]
            ),
        })
    })
}

const SIDE: [&str; 2] = ["original", "engine"];

/// A pass the original ended in oracle frame `theirs` and the engine in `ours`, more than
/// [`PASS_LAG`] frames apart.
fn too_far(pass: u32, theirs: u32, ours: u32, what: &str) -> Option<Outcome> {
    (theirs.abs_diff(ours) > PASS_LAG).then(|| Outcome::PassEndsApart {
        pass,
        frame: theirs,
        why: format!(
            "{what}: the pass ended at oracle frame {theirs} in the original, {ours} in the engine"
        ),
    })
}

/// One side's custom writes cut into passes at its own [`PASS_END`] write (which closes a pass),
/// the closed passes not yet paired with the other side's.
#[derive(Default)]
struct Passes {
    open: Vec<(u16, u16)>,
    closed: VecDeque<(u32, Vec<(u16, u16)>)>,
    /// Passes paired so far.
    paired: u32,
}

impl Passes {
    fn add(&mut self, frame: u32, writes: &[(u16, u16)]) {
        for &w in writes {
            self.open.push(w);
            if w == PASS_END {
                self.closed
                    .push_back((frame, std::mem::take(&mut self.open)));
            }
        }
    }

    /// The next pass both sides closed: (its number from the transplant, the original's frame,
    /// the engine's, the original's writes, the engine's).
    #[allow(clippy::type_complexity)]
    fn pair(p: &mut [Passes; 2]) -> Option<(u32, u32, u32, Vec<(u16, u16)>, Vec<(u16, u16)>)> {
        if p[0].closed.is_empty() || p[1].closed.is_empty() {
            return None;
        }
        let (at, theirs) = p[0].closed.pop_front()?;
        let (ours_at, ours) = p[1].closed.pop_front()?;
        p[0].paired += 1;
        Some((p[0].paired, at, ours_at, theirs, ours))
    }

    /// A pass one side closed more than [`PASS_LAG`] frames before `frame` that the other has not.
    fn apart(p: &[Passes; 2], frame: u32, what: &str) -> Option<Outcome> {
        (0..2).find_map(|side| {
            let (at, _) = p[side].closed.front()?;
            (frame > at + PASS_LAG).then(|| Outcome::PassEndsApart {
                pass: p[0].paired + 1,
                frame: *at,
                why: format!(
                    "{what}: the pass ended at oracle frame {at} in the {}, not by frame {} in the {}",
                    SIDE[side],
                    at + PASS_LAG,
                    SIDE[1 - side]
                ),
            })
        })
    }
}

/// Set on the register of an engine's write a read of a write-only register made
/// ([`FrameEngine::paula`], [`FrameEngine::custom_writes`]): its word is whatever the bus held,
/// which on the original is the last DMA slot's (measured, translator T47: bitplane data under
/// `mt_end`'s `clr.w AUDxVOL` at frame 5001). Such a write is compared by register only (owner,
/// 2026-11-12): no engine without slot-exact DMA could know the word, and it is never seen or
/// heard (a volume held for 4 cycles before the 0, a strobe).
pub const LEFTOVER: u16 = 0x8000;

/// `ours` with each [`LEFTOVER`] write unmarked and given the value of the original's write at
/// the same place when that is to the same register: Paula's writes, compared in order.
pub fn leftover_in_order(ours: &[(u16, u16)], theirs: &[(u16, u16)]) -> Vec<(u16, u16)> {
    ours.iter()
        .enumerate()
        .map(|(i, &(reg, value))| {
            if reg & LEFTOVER == 0 {
                return (reg, value);
            }
            let reg = reg & !LEFTOVER;
            match theirs.get(i) {
                Some(&(r, v)) if r == reg => (reg, v),
                _ => (reg, value),
            }
        })
        .collect()
}

/// `ours` with each [`LEFTOVER`] write unmarked and given the value of an original write to the
/// same register that `ours` is short of, if there is one: the other custom writes, compared as
/// (register, value) counts.
pub fn leftover_counted(ours: &[(u16, u16)], theirs: &[(u16, u16)]) -> Vec<(u16, u16)> {
    let mut short: BTreeMap<(u16, u16), i64> = BTreeMap::new();
    for &w in theirs {
        *short.entry(w).or_default() += 1;
    }
    for &w in ours.iter().filter(|(r, _)| r & LEFTOVER == 0) {
        *short.entry(w).or_default() -= 1;
    }
    ours.iter()
        .map(|&(reg, value)| {
            if reg & LEFTOVER == 0 {
                return (reg, value);
            }
            let reg = reg & !LEFTOVER;
            let found = short
                .iter_mut()
                .find(|((r, _), n)| *r == reg && **n > 0)
                .map(|((_, v), n)| {
                    *n -= 1;
                    *v
                });
            (reg, found.unwrap_or(value))
        })
        .collect()
}

fn as_paula(w: &[(u16, u16)]) -> Vec<PaulaWrite> {
    w.iter()
        .map(|&(reg, value)| PaulaWrite {
            reg,
            value,
            hpos: 0,
            vpos: 0,
        })
        .collect()
}

/// A (register, value)'s writes since the two sides' counts of it last agreed: the original's,
/// the engine's, and how many pass ends they have disagreed across.
#[derive(Debug, Clone, Copy, Default)]
pub struct Carried {
    original: u32,
    engine: u32,
    passes: u32,
}

/// [`custom_difference`] with a pass of slack (owner, 2026-11-12, T30): `main_loop`'s blits are
/// cut by the pass end wherever they have got to, so a write may fall a pass early or late on a
/// machine of another pace. Each (register, value)'s counts are carried from pass to pass; they
/// must agree again by the next pass end, or differ (the counts then cover both passes). A write
/// missing or added for good fails a pass after it, however often the same write recurs.
pub fn custom_carried(
    carried: &mut BTreeMap<(u16, u16), Carried>,
    ours: &[(u16, u16)],
    theirs: &[(u16, u16)],
) -> Option<CustomDifference> {
    for &(reg, value) in theirs.iter().filter(|(r, _)| !is_paula(*r)) {
        carried.entry((reg, value)).or_default().original += 1;
    }
    for &(reg, value) in ours.iter().filter(|(r, _)| !is_paula(*r)) {
        carried.entry((reg, value)).or_default().engine += 1;
    }
    carried.retain(|_, c| c.original != c.engine);
    for c in carried.values_mut() {
        c.passes += 1;
    }
    carried
        .iter()
        .find(|(_, c)| c.passes > PASS_LAG)
        .map(|(&(reg, value), c)| CustomDifference {
            reg,
            value,
            original: c.original,
            engine: c.engine,
        })
}

#[cfg(test)]
mod os_tail_tests {
    use super::OsTail;

    /// A load list: the program hands the machine to the OS, loads twice, runs on with the OS's
    /// interrupts, then takes the machine back; the stretch after the first load is the tail.
    #[test]
    fn the_os_stretch_after_a_load_is_inside_it_until_the_program_takes_the_machine_back() {
        let mut t = OsTail::new(true);
        // system_on: the OS has the machine, no load yet: compared as before.
        assert_eq!(t.frame(0, false), (0, false));
        // A load returns, the OS keeps the machine: the tail.
        assert_eq!(t.frame(1, false), (0, true));
        assert_eq!(t.frame(0, false), (0, true));
        assert_eq!(t.frame(1, false), (0, true));
        // system_off: taken back, counted once, compared again.
        assert_eq!(t.frame(0, true), (1, false));
        assert_eq!(t.frame(0, true), (0, false));
    }

    #[test]
    fn a_load_and_the_taking_back_in_one_frame_count_and_leave_no_tail() {
        let mut t = OsTail::new(false);
        assert_eq!(t.frame(1, true), (1, false));
    }

    /// The entry installing its handlers with no load before it is not a taking back: power-on
    /// cases line up as before.
    #[test]
    fn taking_the_machine_with_no_load_before_counts_nothing() {
        let mut t = OsTail::new(false);
        assert_eq!(t.frame(0, false), (0, false));
        assert_eq!(t.frame(0, true), (0, false));
    }

    /// A load the program makes while it keeps the machine leaves no tail and no count.
    #[test]
    fn a_load_while_the_program_has_the_machine_leaves_no_tail() {
        let mut t = OsTail::new(true);
        assert_eq!(t.frame(1, true), (0, false));
        assert_eq!(t.frame(0, false), (0, false));
        assert_eq!(t.frame(0, true), (0, false));
    }
}

#[cfg(test)]
mod one_frame_on_tests {
    use super::*;

    fn mem() -> (Vec<u8>, AmigaMemory) {
        let theirs = AmigaMemory {
            chip: vec![0; 0x8_0000],
            slow: vec![0; 0x8_0000],
        };
        (vec![0; 0x10_0000], theirs)
    }

    /// `hN+$off`'s index in the engine's RAM and its address.
    fn at(hunk: usize, off: u32) -> (usize, u32) {
        let a = HUNK_BASES[hunk] + off;
        let i = if a >= SLOW_BASE {
            0x8_0000 + (a - SLOW_BASE) as usize
        } else {
            a as usize
        };
        (i, a)
    }

    fn set_theirs(m: &mut AmigaMemory, a: u32, v: u8) {
        if a >= SLOW_BASE {
            m.slow[(a - SLOW_BASE) as usize] = v;
        } else {
            m.chip[a as usize] = v;
        }
    }

    /// The next frame's RAM: the engine's byte at `i` and the original's at `a`.
    fn next(i: usize, a: u32, engine: u8, original: u8) -> (Vec<u8>, AmigaMemory) {
        let (mut ours, mut theirs) = mem();
        ours[i] = engine;
        set_theirs(&mut theirs, a, original);
        (ours, theirs)
    }

    /// Engine and original at the held frame, and each the frame before.
    fn held(a: u32, now: (u8, u8), before: (u8, u8)) -> Held {
        Held {
            pass: 1,
            frame: 100,
            bytes: vec![Byte {
                at: a,
                engine: now.0,
                original: now.1,
                engine_before: before.0,
                original_before: before.1,
            }],
        }
    }

    #[test]
    fn the_beam_sample_and_the_os_values_are_left_out_and_other_bytes_are_not() {
        let (mut ours, theirs) = mem();
        for (h, off) in [(0, 0x8dfa), (0, 0x8dfb), (0, 0x908e)] {
            ours[at(h, off).0] = 7;
        }
        assert_eq!(memory_differences(&ours, &theirs), vec![]);
        let (i, a) = at(0, 0x8dfc);
        ours[i] = 7;
        assert_eq!(memory_differences(&ours, &theirs), vec![(a, 7, 0)]);
        assert_eq!(memory_difference(&ours, &theirs).unwrap().at, "h0+$8dfc");
    }

    /// A screen byte drawn (0 to $ff) a few instructions before the frame's end on one side and
    /// after it on the other: the measured view_tiles case (world 2, h3+$43f4, T51).
    #[test]
    fn a_byte_one_frame_on_either_way_passes() {
        let (i, a) = at(3, 0x43f4);
        // Engine ahead: it drew at frame 100, the original at 101.
        let (o, t) = next(i, a, 0xff, 0xff);
        let h = held(a, (0xff, 0x00), (0x00, 0x00));
        assert_eq!(h.left_after(Some((101, &o, &t))), vec![]);
        // Engine behind: the original drew at 100, the engine at 101.
        let h = held(a, (0x00, 0xff), (0x00, 0x00));
        assert_eq!(h.left_after(Some((101, &o, &t))), vec![]);
    }

    #[test]
    fn a_third_value_on_both_sides_is_not_one_frame_on() {
        let (i, a) = at(3, 0x43f4);
        let (o, t) = next(i, a, 0x42, 0x42);
        let h = held(a, (0xff, 0x00), (0x00, 0x00));
        assert_eq!(h.left_after(Some((101, &o, &t))), h.bytes);
    }

    /// The engine's byte wrong for one frame and right again after: its next value is the
    /// original's, but its previous was not the original's previous, so it fails.
    #[test]
    fn a_one_frame_glitch_that_reverts_is_not_one_frame_on() {
        let (i, a) = at(0, 0x7cde);
        let (o, t) = next(i, a, 0x10, 0x10);
        let h = held(a, (0x11, 0x10), (0x10, 0x10));
        assert_eq!(h.left_after(Some((101, &o, &t))), h.bytes);
    }

    #[test]
    fn a_held_frame_with_no_very_next_frame_keeps_every_byte() {
        let (i, a) = at(3, 0x43f4);
        let (o, t) = next(i, a, 0xff, 0xff);
        let h = held(a, (0xff, 0x00), (0x00, 0x00));
        assert_eq!(h.left_after(Some((102, &o, &t))), h.bytes);
        assert_eq!(h.left_after(None), h.bytes);
    }
}
