//! The translated program as a case's engine (translator T28, DESIGN §3.3): `fd-translated`'s
//! [`TranslatedGame`] behind [`Engine`] and [`Transplant`], so `run_with` drives it as it drives
//! the hand engine.
//!
//! **The thread.** `TranslatedGame::run` drives and calls back once a frame; `Engine::pass` is a
//! step the runner pulls. So the game runs on a thread of its own ([`RUN_STACK`] of stack), and
//! each pass hands one input across a channel and takes one frame back: lockstep, so a run is as
//! deterministic as the game.
//!
//! **The start.** A case transplants at the original's first pass end after its sync point, giving
//! only RAM; the translated program needs the CPU's registers and the custom registers too. So
//! [`case_start`] runs the original itself from power-on along the case's way to its sync point
//! (`reach_case`'s, uncached: the cache keeps no custom registers), collects every CPU write to a
//! custom register, and at the end of the step whose pass end the runner transplants at takes RAM,
//! the registers and that collection. [`Transplant::transplant`] checks that the pass is the same.
//! At that step's end the original is inside `level3_handler` (docs/09), mid-function, which
//! `fd-translated` resumes.
//!
//! **One pass, one frame.** A pass runs the translated program from one frame boundary to the next,
//! the frame whose level 3 interrupt runs `game_frame`. The probe reads RAM at that boundary. Under
//! the pass runner (`run_with`) the original is read at its pass end (`PASS_END`) instead; the
//! frame bar (DESIGN §2.7) is judged by [`crate::frames`] ([`FrameEngine`], [`run_translated`]),
//! which reads both sides' state at each pass end, where the CPU writes `PASS_END`
//! (`FrameEngine::pass_ends`, the machine's `watch_write`; T30), and the picture at the frame's end.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use amiga_conform::script;

use amiga_oracle::{Amiga, FrameInput, OracleConfig, PaulaWrite, kickstart_path};
use fd_core::{Mem, Transplant, Var, layout_var};
use fd_translated::{Control, RUN_STACK, Regs68k, Start, TranslatedGame};
use pcode_machine::{Disks, Input, Picture, custom, irq};
use pcode_rt::Stop;

use crate::frames::{FrameEngine, run_frames};
use crate::{
    Case, Compare, Engine, HUNK_BASES, Inputs, Options, OracleMem, PASS_END, Verdict, address,
    disks, sync,
};

/// Chip RAM's size on the oracle's A500, where slow RAM starts in a snapshot's bytes.
const CHIP: usize = 512 << 10;
const SLOW_BASE: u32 = 0xC0_0000;

/// The original at the step whose pass end a case's runner transplants at.
#[derive(Clone)]
pub struct CaseStart {
    /// Chip RAM then slow RAM at the step's end.
    pub mem: Vec<u8>,
    pub regs: Regs68k,
    /// Each custom register's last CPU write, by `reg / 2`; DMACONR, INTENAR and INTREQR the
    /// set/clear state of the CPU's writes; CLXDAT the collisions not yet read (`Amiga::clxdat`);
    /// VPOSR and VHPOSR line 0, where a step ends.
    pub custom: [u16; 256],
    /// Disk 2, in df0 since frame [`sync::DISK_2_SWAP`].
    pub disk2: Vec<u8>,
    /// `irq_count` at the pass end: the pass the runner transplants at.
    pub pass: u32,
    /// The oracle frame the step ended at.
    pub frame: u32,
    /// The step's input: the keys it holds the original's keyboard has sent by its end.
    pub input: FrameInput,
}

/// [`CaseStart`] of `case`: the original from power-on along `reach_case`'s way (the level 1
/// script up to [`sync::LEVEL_1_SYNC`], then the case's inputs, then its `set`), then the case's
/// steps until a pass end, as `run_inputs` steps them. A title case's start is the end of its sync
/// frame's step, its script and swap on the way (T32); its `pass` is 0, as nothing is transplanted.
pub fn case_start(case: &Case) -> Result<CaseStart, String> {
    if case.is_entry() {
        return Err(format!(
            "{}: a power-on case starts at the program's entry, not a snapshot",
            case.name
        ));
    }
    let level = case.is_level();
    let inputs = Inputs::load(case)?;
    let (_, level_1) = sync::level_1_script()?;
    let kickstart = kickstart_path().map_err(|e| e.to_string())?;
    let disks = disks::load()?;
    let disk2 = disks[1].clone();
    // A pass ends within a step or two of any step in play; 50 is room, not a wait.
    let cap = case.sync_frame + 50;
    let mut am = Amiga::power_on(&OracleConfig::new(kickstart, disks, cap + 1))
        .map_err(|e| e.to_string())?;
    let mut regs = [0u16; 256];
    let (mut dmacon, mut intena, mut intreq) = (0u16, 0u16, 0u16);
    let mut collect = |am: &Amiga| {
        for w in am.cpu_writes() {
            let reg = w.reg & 0x1FE;
            regs[reg as usize >> 1] = w.value;
            match reg {
                custom::DMACON => dmacon = irq::set_clear(dmacon, w.value),
                irq::INTENA => intena = irq::set_clear(intena, w.value),
                irq::INTREQ => intreq = irq::set_clear(intreq, w.value),
                _ => {}
            }
        }
    };
    // The sync frame step's input: the keys held at a title case's start.
    let mut last = FrameInput::default();
    for frame in 0..case.sync_frame {
        let swap = if level {
            Some(sync::DISK_2_SWAP)
        } else {
            case.swap.map(|s| s.frame)
        };
        if swap == Some(frame) {
            am.insert_disk(0, case.swap.map_or(1, |s| s.disk as usize - 1));
        }
        let input = if level && frame < sync::LEVEL_1_SYNC {
            script::input_at(&level_1.lines, frame)
        } else {
            inputs.at(frame)
        };
        am.step(input).map_err(|e| e.to_string())?;
        collect(&am);
        last = input;
    }
    if !level {
        return Ok(snapshot(&am, regs, (dmacon, intena, intreq), disk2, 0, last));
    }
    for (name, value) in &case.set {
        let v = layout_var(name).ok_or_else(|| format!("`{name}` is not in fd-core's LAYOUT"))?;
        let bytes = &value.to_be_bytes()[4 - v.width.bytes()..];
        am.poke(address(name)?, bytes);
    }
    am.watch_write(Some(PASS_END));
    let irq_count = layout_var("irq_count").ok_or("no irq_count in LAYOUT")?;
    for step in 0..cap - case.sync_frame {
        let input = inputs.at(case.sync_frame + step);
        am.step(input).map_err(|e| e.to_string())?;
        collect(&am);
        if let Some(mem) = am.watched_memory() {
            let pass = OracleMem(&mem).value(irq_count);
            return Ok(snapshot(&am, regs, (dmacon, intena, intreq), disk2, pass, input));
        }
    }
    Err(format!(
        "{}: no pass ended in the {} steps after frame {}",
        case.name,
        cap - case.sync_frame,
        case.sync_frame
    ))
}

/// The original's state at the end of its last step, as a [`CaseStart`]: `regs` each custom
/// register's last CPU write, `(dmacon, intena, intreq)` those registers' set/clear state.
fn snapshot(
    am: &Amiga,
    mut regs: [u16; 256],
    (dmacon, intena, intreq): (u16, u16, u16),
    disk2: Vec<u8>,
    pass: u32,
    input: FrameInput,
) -> CaseStart {
    regs[custom::DMACONR as usize >> 1] = dmacon & 0x07FF;
    regs[irq::INTENAR as usize >> 1] = intena;
    regs[irq::INTREQR as usize >> 1] = intreq;
    // The collisions Denise gathered since the program's last CLXDAT read: below the beam's line
    // then, so the machine, starting at the frame's start, cannot draw them.
    regs[pcode_machine::collide::CLXDAT as usize >> 1] = am.clxdat();
    // VPOSR, VHPOSR: a step ends just past the frame's end, at line 0.
    regs[0x004 >> 1] = 0;
    regs[0x006 >> 1] = 0;
    let ram = am.memory();
    let mut mem = ram.chip.clone();
    mem.extend_from_slice(&ram.slow);
    let c = am.cpu();
    CaseStart {
        mem,
        regs: Regs68k {
            d: c.d,
            a: c.a,
            pc: c.pc,
            sr: c.sr,
            usp: c.usp,
            ssp: c.ssp,
        },
        custom: regs,
        disk2,
        pass,
        frame: am.frame(),
        input,
    }
}

/// One frame as the game thread hands it over.
struct Frame {
    picture: Picture,
    paula: Vec<PaulaWrite>,
    /// The CPU's custom writes, (register, value): `FrameOut::cpu_writes`, never the copper's.
    custom: Vec<(u16, u16)>,
    stop: Option<Stop>,
    /// Chip RAM then slow RAM at the frame's end.
    mem: Vec<u8>,
    /// Chip RAM then slow RAM at each pass end of the frame (`FrameOut::watched`).
    pass_ends: Vec<Vec<u8>>,
    /// Loads returned in the frame, and whether one runs at its end (power-on only).
    loads: (u32, bool),
}

/// Where the game thread starts.
enum Begin {
    Snapshot(CaseStart),
    /// The program file loaded as Kickstart would and entered at `h0+$0` on `line`, disk 2 in
    /// df0, its file routines watched for `align = "loads"` (T32).
    PowerOn {
        program: Vec<u8>,
        disk2: Vec<u8>,
        line: u16,
    },
}

struct Link {
    to: Option<Sender<Input>>,
    from: Receiver<Frame>,
    thread: Option<JoinHandle<()>>,
}

/// The translated program as a level case's engine. Built from a [`CaseStart`]; the game starts on
/// the first [`Engine::pass`].
pub struct TranslatedEngine {
    start: Option<Begin>,
    pass: u32,
    link: Option<Link>,
    /// RAM as the last pass left it (the start's until the first).
    mem: Vec<u8>,
    picture: Option<Picture>,
    paula: Vec<PaulaWrite>,
    custom: Vec<(u16, u16)>,
    /// RAM at each pass end of the last frame.
    pass_ends: Vec<Vec<u8>>,
    stop: Option<(u32, Stop)>,
    passes: u32,
    loads: (u32, bool),
}

impl TranslatedEngine {
    pub fn new(start: CaseStart) -> TranslatedEngine {
        TranslatedEngine {
            pass: start.pass,
            mem: start.mem.clone(),
            start: Some(Begin::Snapshot(start)),
            link: None,
            picture: None,
            paula: Vec::new(),
            custom: Vec::new(),
            pass_ends: Vec::new(),
            stop: None,
            passes: 0,
            loads: (0, false),
        }
    }

    /// The translated program from power-on (T32): the program file from disk 1 at the hunk
    /// bases, disk 2 in df0, entered at `h0+$0` on the first frame, every load of
    /// [`sync::LOAD_ROUTINES`] reported.
    pub fn power_on(disks: &[Vec<u8>]) -> Result<TranslatedEngine, String> {
        let program = crate::os_run::program(&disks[0])?;
        Ok(TranslatedEngine {
            pass: 0,
            mem: Vec::new(),
            start: Some(Begin::PowerOn {
                program,
                disk2: disks[1].clone(),
                line: 0,
            }),
            link: None,
            picture: None,
            paula: Vec::new(),
            custom: Vec::new(),
            stop: None,
            passes: 0,
            pass_ends: Vec::new(),
            loads: (0, false),
        })
    }

    /// How the translated program stopped, and after how many passes; `None` while it runs.
    pub fn stop(&self) -> Option<&(u32, Stop)> {
        self.stop.as_ref()
    }

    /// The passes run.
    pub fn passes(&self) -> u32 {
        self.passes
    }

    /// RAM as the last pass left it, chip then slow.
    pub fn mem(&self) -> &[u8] {
        &self.mem
    }

    fn spawn(start: Begin, first: Input) -> Link {
        let (to, inputs) = channel::<Input>();
        let (frames, from) = channel::<Frame>();
        let thread = thread::Builder::new()
            .name("translated game".into())
            .stack_size(RUN_STACK)
            .spawn(move || {
                let mut g = match &start {
                    Begin::Snapshot(start) => {
                        let mut g = TranslatedGame::new(Start::Snapshot {
                            mem: &start.mem,
                            regs: start.regs,
                            custom: &start.custom,
                            disks: Disks {
                                df0: start.disk2.clone(),
                            },
                        });
                        g.held_at_start(&Input::from(start.input));
                        g
                    }
                    Begin::PowerOn {
                        program,
                        disk2,
                        line,
                    } => {
                        let mut g = TranslatedGame::new(Start::PowerOn {
                            program,
                            disks: Disks { df0: disk2.clone() },
                        });
                        g.watch_returns(&sync::LOAD_ROUTINES);
                        g.start_at_line(*line);
                        g
                    }
                };
                g.set_input(first);
                g.watch_write(Some(PASS_END));
                g.run(&mut |out| {
                    let mut mem = out.chip.to_vec();
                    mem.extend_from_slice(out.slow);
                    // The machine's beam positions are its own; the comparison ignores them.
                    let paula = out
                        .paula
                        .iter()
                        .map(|&(_, reg, value)| PaulaWrite {
                            reg,
                            value,
                            hpos: 0,
                            vpos: 0,
                        })
                        .collect();
                    let custom = out.cpu_writes.iter().map(|&(_, r, v)| (r, v)).collect();
                    let sent = frames.send(Frame {
                        picture: out.picture.clone(),
                        paula,
                        custom,
                        stop: out.stop.clone(),
                        mem,
                        loads: (out.returns.len() as u32, !out.inside.is_empty()),
                        pass_ends: out.watched.clone(),
                    });
                    // The engine dropped: nobody wants the next frame.
                    match (sent, inputs.recv()) {
                        (Ok(()), Ok(input)) => Control::Next(input),
                        _ => Control::Stop,
                    }
                });
            })
            .expect("a thread for the translated game");
        Link {
            to: Some(to),
            from,
            thread: Some(thread),
        }
    }

    /// Wait for the game thread's next frame; a panic there goes on here.
    fn receive(&mut self) -> Frame {
        let link = self.link.as_mut().expect("started");
        match link.from.recv() {
            Ok(f) => f,
            Err(_) => {
                let t = link.thread.take().expect("joined once");
                match t.join() {
                    Err(e) => std::panic::resume_unwind(e),
                    Ok(()) => panic!("the translated game's thread ended without a stop"),
                }
            }
        }
    }
}

impl Drop for TranslatedEngine {
    fn drop(&mut self) {
        if let Some(mut link) = self.link.take() {
            // Closing the input channel ends the run at the frame it waits in.
            link.to.take();
            while link.from.recv().is_ok() {}
            if let Some(t) = link.thread.take() {
                let _ = t.join();
            }
        }
    }
}

/// `v`'s bytes in `mem` (chip RAM, then slow RAM from [`SLOW_BASE`]), zero-extended.
fn value(mem: &[u8], v: &Var) -> u32 {
    let at = HUNK_BASES[v.hunk as usize] + v.offset;
    (0..v.width.bytes() as u32).fold(0, |acc, k| {
        let a = (at + k) & 0xFF_FFFF;
        let i = if a >= SLOW_BASE {
            CHIP + (a - SLOW_BASE) as usize
        } else {
            a as usize
        };
        acc << 8 | mem.get(i).copied().unwrap_or(0) as u32
    })
}

impl Transplant for TranslatedEngine {
    /// The start was taken at the same pass end on the case's own way there; a different pass means
    /// the two runs of the original went apart, which no comparison after it could trust.
    fn transplant(&mut self, mem: &dyn Mem) {
        let irq_count = layout_var("irq_count").expect("irq_count in LAYOUT");
        let theirs = mem.value(irq_count);
        assert_eq!(
            theirs, self.pass,
            "the runner transplants at pass {theirs}, the translated start was taken at pass {}",
            self.pass
        );
    }

    fn probe(&self, v: &Var) -> u32 {
        value(&self.mem, v)
    }
}

impl TranslatedEngine {
    /// One frame of the translated program on `input`. After a stop, nothing runs.
    fn advance(&mut self, input: FrameInput) {
        if self.stop.is_some() {
            return;
        }
        let input = Input::from(input);
        match self.start.take() {
            Some(start) => self.link = Some(Self::spawn(start, input)),
            None => {
                let link = self.link.as_ref().expect("started");
                // A send fails only when the thread has ended; `receive` reports why.
                let _ = link.to.as_ref().expect("open").send(input);
            }
        }
        let f = self.receive();
        self.passes += 1;
        self.mem = f.mem;
        self.picture = Some(f.picture);
        self.paula = f.paula;
        self.custom = f.custom;
        self.loads = f.loads;
        self.pass_ends = f.pass_ends;
        if let Some(s) = f.stop {
            self.stop = Some((self.passes, s));
        }
    }
}

impl Engine for TranslatedEngine {
    fn pass(&mut self, input: FrameInput) {
        self.advance(input);
    }

    /// The machine's whole picture of the last frame, not cut to the play window (T29's).
    fn picture(&self) -> Option<Picture> {
        self.picture.clone()
    }
}

impl FrameEngine for TranslatedEngine {
    fn frame(&mut self, input: FrameInput) {
        self.advance(input);
    }

    fn picture(&self) -> Option<Picture> {
        self.picture.clone()
    }

    fn paula(&self) -> &[PaulaWrite] {
        &self.paula
    }

    fn custom_writes(&self) -> &[(u16, u16)] {
        &self.custom
    }

    fn pass_ends(&self, vars: &[&Var]) -> Vec<Vec<u32>> {
        self.pass_ends
            .iter()
            .map(|m| vars.iter().map(|v| value(m, v)).collect())
            .collect()
    }

    fn stopped(&self) -> Option<String> {
        self.stop.as_ref().map(|(_, s)| s.to_string())
    }

    fn ram(&self) -> Option<&[u8]> {
        Some(&self.mem)
    }

    fn loads(&self) -> (u32, bool) {
        self.loads
    }

    fn start_line(&mut self, at: u16) {
        if let Some(Begin::PowerOn { line, .. }) = &mut self.start {
            *line = at;
        }
    }
}

/// `case` as the frame bar judges the translated program (DESIGN §2.7): the case's state (every
/// play variable in `LAYOUT` where a level case lists none; a power-on case's hunks, and its `vars`
/// if it lists any), the picture, Paula's writes in order and the CPU's other custom writes as a
/// multiset, every frame. A level case's exclusions are left out: each names a hand engine task,
/// and an acceptance for the translated program is written with its own measurement (§2.7).
pub fn frame_bar(case: &Case) -> Case {
    let vars = if case.vars.is_empty() && case.is_level() {
        crate::bug::compared_vars()
    } else {
        case.vars.clone()
    };
    Case {
        compare: Compare {
            vars: !vars.is_empty(),
            picture: true,
            writes: true,
            memory: case.is_entry(),
        },
        vars,
        exclude: Vec::new(),
        ..case.clone()
    }
}

/// `case` on the translated program, judged on the frame bar ([`frame_bar`]): a level case from
/// its pass end, a title case from its sync frame, a power-on case (`start = "entry"`, T32) from
/// the program's entry with its loads lined up.
pub fn run_translated(case: &Case, opts: &Options) -> Result<Verdict, String> {
    let bar = frame_bar(case);
    if case.is_entry() {
        let mut e = TranslatedEngine::power_on(&disks::load()?)?;
        return crate::frames::run_entry_frames(&bar, &Inputs::load(&bar)?, &mut e, opts);
    }
    let mut e = TranslatedEngine::new(case_start(&bar)?);
    run_frames(&bar, &mut e, opts)
}
