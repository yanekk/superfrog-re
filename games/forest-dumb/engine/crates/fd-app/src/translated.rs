//! The translated program as the app's engine (translator DESIGN §2.10, §3.3): `fd-translated`'s
//! [`TranslatedGame`] from power-on, level 1's hand engine mounted through the seam.
//!
//! `TranslatedGame::run` drives and calls back at each frame's end; the app pulls one frame a
//! step. So the game runs on a thread of its own ([`RUN_STACK`] of stack, for the translated
//! frames' depth) and each step hands one input across a channel and takes one frame back, in
//! lockstep, so the app's run is the same as a headless run on the same inputs.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use amiga_input::FrameInput;
use fd_core::GameData;
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
    /// [`fd_app::in_level_1`] at the frame's end.
    pub level_1: bool,
    /// How the program stopped, on its last frame.
    pub stop: Option<String>,
}

impl Frame {
    /// [`fd_app::memory_hash`] of the frame's end, when hashes were asked for.
    pub fn hash(&self) -> Option<u64> {
        self.mem.as_ref().map(|(c, s)| fd_app::memory_hash(c, s))
    }
}

/// The translated program running on its thread.
pub struct Translated {
    to: Option<Sender<Input>>,
    from: Receiver<Frame>,
    thread: Option<JoinHandle<()>>,
    /// The newest frame: before the first step, the start's.
    frame: Frame,
}

impl Translated {
    /// The program file loaded as Kickstart would and entered at `h0+$0`, disk 2 in df0, level
    /// 1's hand engine mounted from `data`; `hashes` asks for each frame's state hash.
    pub fn power_on(
        program: Vec<u8>,
        disk2: Vec<u8>,
        data: Arc<GameData>,
        hashes: bool,
    ) -> Translated {
        let (to, inputs) = channel::<Input>();
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
                let (chip, slow) = (g.machine().chip(), g.machine().slow());
                let (l, t, w, h) = fd_app::CANVAS;
                let start = Frame {
                    picture: Picture::blank(l, t, w, h),
                    mem: hashes.then(|| (chip.into(), slow.into())),
                    level_1: fd_app::in_level_1(chip, slow),
                    stop: None,
                };
                if frames.send(start).is_err() {
                    return;
                }
                let Ok(first) = inputs.recv() else { return };
                g.set_input(first);
                g.run(&mut |out| {
                    let frame = Frame {
                        picture: fd_app::canvas(&out.picture),
                        mem: hashes.then(|| (out.chip.into(), out.slow.into())),
                        level_1: fd_app::in_level_1(out.chip, out.slow),
                        stop: out.stop.as_ref().map(|s| s.to_string()),
                    };
                    let stopped = frame.stop.is_some();
                    // The app dropped, or the program stopped: nobody sends the next input.
                    if frames.send(frame).is_err() || stopped {
                        return Control::Stop;
                    }
                    match inputs.recv() {
                        Ok(i) => Control::Next(i),
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
        }
    }

    /// One frame on `input`. After a stop nothing runs and the stop's frame stays.
    pub fn step(&mut self, input: FrameInput) -> &Frame {
        if self.frame.stop.is_some() {
            return &self.frame;
        }
        // A send fails only when the thread has ended, which `receive` reports.
        let _ = self.to.as_ref().expect("open").send(Input::from(input));
        self.frame = self.receive();
        &self.frame
    }

    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// `--level 1`: from power-on to level 1's first frame on [`fd_app::to_level_1_input`],
    /// the program's own way through the title. The frames it took, or why it did not get there
    /// within [`fd_app::TO_LEVEL_1_FRAMES`].
    pub fn to_level_1(&mut self) -> Result<u64, String> {
        for n in 0..fd_app::TO_LEVEL_1_FRAMES {
            let f = self.step(fd_app::to_level_1_input(n));
            if let Some(s) = &f.stop {
                return Err(format!(
                    "the translated program stopped on its way to level 1, frame {}: {s}",
                    n + 1
                ));
            }
            if f.level_1 {
                return Ok(n + 1);
            }
        }
        Err(format!(
            "the translated program did not reach level 1 in {} frames",
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
