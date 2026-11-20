//! The translated program as the app's engine (translator DESIGN §2.10, §3.3): `fd-translated`'s
//! [`TranslatedGame`] from power-on, level 1's hand engine mounted through the seam.
//!
//! `TranslatedGame::run` drives and calls back at each frame's end; the app pulls one frame a
//! step. So the game runs on a thread of its own ([`RUN_STACK`] of stack, for the translated
//! frames' depth) and each step hands one input across a channel and takes one frame back, in
//! lockstep, so the app's run is the same as a headless run on the same inputs. Each frame's
//! sound is mixed there too (`fd_app::audio::Sound`, T38), where the frame's chip RAM is at hand.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use amiga_input::FrameInput;
use fd_app::audio::Sound;
use fd_core::GameData;
use fd_core::recording::{LevelId, StartPoint};
use pcode_machine::Picture;
use fd_translated::{Control, RUN_STACK, Start, TranslatedGame};
use pcode_machine::{Disks, Input};

/// One frame of the translated program, as the app shows and captures it.
pub struct Frame {
    /// Through [`fd_app::CANVAS`].
    pub picture: Picture,
    /// Chip RAM and slow RAM at the frame's end, when hashes were asked for: hashed only for the
    /// frames a capture writes ([`Frame::hash`]), since hashing a megabyte each frame of
    /// `--level 1`'s run through the title took 4 s unoptimised (T37).
    mem: Option<(Box<[u8]>, Box<[u8]>)>,
    /// [`fd_app::level_in_ram`] at the frame's end: the level played, `None` on the title.
    pub level: Option<LevelId>,
    /// How the program stopped, on its last frame.
    pub stop: Option<String>,
    /// The program ended itself on this frame, its last: KONIEC's return to AmigaDOS
    /// (`TranslatedGame::exited`, translator T41). Not a stop.
    pub exited: bool,
    /// Where play is ([`fd_app::location_in_ram`]): the character's cell in level 1, else `None`.
    pub place: Option<fd_core::recording::Location>,
    /// The frame's sound at the run's rate, stereo; none for the start.
    pub samples: Vec<[i16; 2]>,
}

impl Frame {
    /// Whether the frame is in level 1's play ([`fd_app::in_level_1`]).
    pub fn level_1(&self) -> bool {
        self.level == Some(LevelId::W1_1)
    }

    /// [`fd_app::memory_hash`] of the frame's end, when hashes were asked for.
    pub fn hash(&self) -> Option<u64> {
        self.mem.as_ref().map(|(c, s)| fd_app::memory_hash(c, s))
    }
}

/// What the app hands the game's thread.
enum Msg {
    /// `TranslatedGame::write_at`, before the first frame: a world start's write.
    WriteAt((u8, u32), u32, Vec<u8>),
    /// The next frame's input.
    Input(Input),
}

/// The translated program running on its thread.
pub struct Translated {
    to: Option<Sender<Msg>>,
    from: Receiver<Frame>,
    thread: Option<JoinHandle<()>>,
    /// The newest frame: before the first step, the start's.
    frame: Frame,
    /// Whether a frame has been asked for: a world start's writes must come before.
    stepped: bool,
}

impl Translated {
    /// The program file loaded as Kickstart would and entered at `h0+$0`, disk 2 in df0, level
    /// 1's hand engine mounted from `data`; `hashes` asks for each frame's state hash; the sound
    /// is mixed at `rate` samples a second.
    pub fn power_on(
        program: Vec<u8>,
        disk2: Vec<u8>,
        data: Arc<GameData>,
        hashes: bool,
        rate: u32,
    ) -> Translated {
        let (to, inputs) = channel::<Msg>();
        let (frames, from) = channel::<Frame>();
        let thread = thread::Builder::new()
            .name("translated game".into())
            .stack_size(RUN_STACK)
            .spawn(move || {
                let mut g = TranslatedGame::new(Start::PowerOn {
                    program: &program,
                    disks: Disks { df0: disk2 },
                });
                g.mount_level_1(data);
                let mut sound = Sound::new(rate, g.machine().dmacon());
                let (chip, slow) = (g.machine().chip(), g.machine().slow());
                let (l, t, w, h) = fd_app::CANVAS;
                let start = Frame {
                    picture: Picture::blank(l, t, w, h),
                    mem: hashes.then(|| (chip.into(), slow.into())),
                    level: fd_app::level_in_ram(chip, slow),
                    stop: None,
                    exited: false,
                    place: fd_app::location_in_ram(chip, slow),
                    samples: Vec::new(),
                };
                if frames.send(start).is_err() {
                    return;
                }
                // A world start's writes come before the first input: `write_at` holds only for a
                // run not yet begun.
                let first = loop {
                    match inputs.recv() {
                        Ok(Msg::WriteAt(at, addr, bytes)) => g.write_at(at, addr, &bytes),
                        Ok(Msg::Input(i)) => break i,
                        Err(_) => return,
                    }
                };
                g.set_input(first);
                g.run(&mut |out| {
                    let mut samples = Vec::new();
                    sound.frame_out(out, &mut samples);
                    let frame = Frame {
                        picture: fd_app::canvas(&out.picture),
                        mem: hashes.then(|| (out.chip.into(), out.slow.into())),
                        level: fd_app::level_in_ram(out.chip, out.slow),
                        stop: out.stop.as_ref().map(|s| s.to_string()),
                        exited: out.exited,
                        place: fd_app::location_in_ram(out.chip, out.slow),
                        samples,
                    };
                    let stopped = frame.stop.is_some() || frame.exited;
                    // The app dropped, or the program stopped or ended: nobody sends the next input.
                    if frames.send(frame).is_err() || stopped {
                        return Control::Stop;
                    }
                    match inputs.recv() {
                        Ok(Msg::Input(i)) => Control::Next(i),
                        Ok(Msg::WriteAt(..)) => unreachable!("to_world writes before the first step"),
                        Err(_) => Control::Stop,
                    }
                });
            })
            .expect("a thread for the translated game");
        let frame = from
            .recv()
            .expect("the translated game sends its start before anything");
        Translated {
            to: Some(to),
            from,
            thread: Some(thread),
            frame,
            stepped: false,
        }
    }

    /// One frame on `input`. After a stop or the program's exit nothing runs and the last frame
    /// stays, silent.
    pub fn step(&mut self, input: FrameInput) -> &Frame {
        if self.frame.stop.is_some() || self.frame.exited {
            // Frozen: the stop's frame was heard at its own step; nothing plays after.
            self.frame.samples.clear();
            return &self.frame;
        }
        // A send fails only when the thread has ended, which `receive` reports.
        self.stepped = true;
        let _ = self.to.as_ref().expect("open").send(Msg::Input(Input::from(input)));
        self.frame = self.receive();
        &self.frame
    }

    /// `--stop-after`'s stop (translator T39): from here on the program is stopped with `why`, as
    /// it would be by a stop of its own on this frame. Nothing when it has stopped already.
    pub fn raise(&mut self, why: String) {
        self.frame.stop.get_or_insert(why);
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// `--level N [--cheat]` and Start Level… (title plan DESIGN §2.5): from power-on to
    /// `level`'s first frame on [`fd_app::to_level_1_input`], the program's own way through the
    /// title, with [`fd_app::world_start_bytes`] written as `load_world` is entered unless the
    /// start is level 1's old one ([`fd_app::world_start`]), as `fd-conform bug replay` starts it.
    /// The frames it took, or why it did not get there within [`fd_app::TO_LEVEL_1_FRAMES`]. Only
    /// on a run not yet stepped.
    // Named by the plan (T09): it runs the program to a world, it converts nothing.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_world(&mut self, level: LevelId, cheat: bool) -> Result<u64, String> {
        if self.stepped {
            return Err(format!("{level} is started from power-on, before the first step"));
        }
        let start = fd_app::world_start(level, cheat);
        if let StartPoint::World { level, cheat } = start {
            for (addr, bytes) in fd_app::world_start_bytes(level, cheat) {
                let _ = self.to.as_ref().expect("open").send(Msg::WriteAt(
                    fd_app::LOAD_WORLD,
                    addr,
                    bytes,
                ));
            }
        }
        for n in 0..fd_app::TO_LEVEL_1_FRAMES {
            let f = self.step(fd_app::to_level_1_input(n));
            if let Some(s) = &f.stop {
                return Err(format!(
                    "the translated program stopped on its way to {level}, frame {}: {s}",
                    n + 1
                ));
            }
            if f.level == start.level() {
                return Ok(n + 1);
            }
        }
        Err(format!(
            "the translated program did not reach {level} in {} frames",
            fd_app::TO_LEVEL_1_FRAMES
        ))
    }

    /// The game thread's next frame; a panic there goes on here.
    fn receive(&mut self) -> Frame {
        match self.from.recv() {
            Ok(f) => f,
            Err(_) => match self.thread.take().map(|t| t.join()) {
                Some(Err(e)) => std::panic::resume_unwind(e),
                _ => panic!("the translated game's thread ended without a stop"),
            },
        }
    }
}

impl Drop for Translated {
    fn drop(&mut self) {
        // Closing the input channel ends the run at the frame it waits in.
        self.to.take();
        while self.from.recv().is_ok() {}
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
