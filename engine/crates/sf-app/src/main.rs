//! Shell: the `superfrog` binary (DESIGN §3.2, §3.4).
//!
//! Loads the asset cache from the app-data folder, runs [`Game`] from power-on at the PAL rate
//! paced by the audio device's clock, presents the newest frame at each display refresh at an
//! integer scale and plays the mixer's output. `--script` feeds events in place of the keyboard's,
//! `--capture` writes what was shown and played; the seatbelts are in `args.rs` (DESIGN §5.2).

mod args;
mod audio;
mod capture;
mod home;
mod input;
mod pacing;
mod script;
mod video;

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use sf_core::{Game, Rng, Start, View};
use sf_paula::Paula;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::args::Args;
use crate::audio::{Audio, Clock, NoChip};
use crate::capture::Capture;
use crate::input::{HostInput, Input};
use crate::pacing::{Pacer, Presenter};
use crate::script::{Event, Script};
use crate::video::{Video, picture_rgba};

/// The mixer's rate when there is no audio device (a script run, or none present).
const SILENT_RATE: u32 = 48_000;

/// Exit status when `--timeout` stops a run whose process group it cannot stop whole.
const TIMED_OUT: i32 = 124;

fn main() {
    let args = match args::parse(std::env::args_os().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("superfrog: {e}");
            std::process::exit(2);
        }
    };
    if let Some(s) = args.timeout_s {
        watchdog(Duration::from_secs(s));
    }
    let home = home::resolve(
        args.home.as_deref(),
        std::env::var_os("SUPERFROG_HOME"),
        std::env::var_os("HOME"),
    );
    let data = home::load(&home, args.cache_from_disks.as_deref()).unwrap_or_else(|e| {
        eprintln!("superfrog: {e}");
        std::process::exit(1);
    });
    if args.max_frames == Some(0) {
        return; // The cache is in place; nothing to show.
    }
    let script = args.script.as_ref().map(|path| {
        std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {e}", path.display()))
            .and_then(|t| Script::parse(&t))
            .unwrap_or_else(|e| {
                eprintln!("superfrog: {e}");
                std::process::exit(2);
            })
    });
    let seed = match &script {
        Some(s) => s.seed.unwrap_or((0, 0)),
        None => host_seed(),
    };
    // A scripted run plays nothing through the speakers: the test block runs it.
    let audio = if script.is_some() {
        None
    } else {
        Audio::open()
            .inspect_err(|e| eprintln!("superfrog: {e}; carrying on silently"))
            .ok()
    };
    let rate = audio.as_ref().map_or(SILENT_RATE, |a| a.rate);
    let capture = args.capture.as_ref().map(|dir| {
        Capture::new(dir, rate).unwrap_or_else(|e| {
            eprintln!("superfrog: {}: {e}", dir.display());
            std::process::exit(1);
        })
    });

    let mut builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        // A scripted run must not take the keyboard from whatever the person is using.
        builder.with_activate_ignoring_other_apps(script.is_none());
    }
    let event_loop = builder.build().unwrap_or_else(|e| {
        eprintln!("superfrog: no event loop: {e}");
        std::process::exit(1);
    });
    let mut app = App {
        game: Game::new(&data, Start::PowerOn { seed: Rng::seeded(seed.0, seed.1) }),
        paula: Paula::new(rate),
        input: Input::default(),
        pacer: Pacer::new(rate),
        clock: match &audio {
            Some(a) => a.clock(),
            None => Clock::Wall { start: Instant::now(), rate },
        },
        lead: audio.as_ref().map_or(0, |a| a.rate as u64 / 25),
        audio,
        script,
        capture,
        window: None,
        video: None,
        shown: Presenter::default(),
        steps: 0,
        samples: Vec::new(),
        status: 0,
        args,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("superfrog: {e}");
        std::process::exit(1);
    }
    std::process::exit(app.status);
}

/// `--timeout`: after `limit` of wall clock, stop this process and, when it leads its own process
/// group (as `sf-e2e` launches it), every process in the group.
fn watchdog(limit: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(limit);
        eprintln!("superfrog: --timeout {} s reached; stopping", limit.as_secs());
        // SAFETY: plain syscalls on this process's own ids.
        unsafe {
            let group = libc::getpgrp();
            if group == libc::getpid() {
                libc::killpg(group, libc::SIGKILL);
            }
        }
        std::process::exit(TIMED_OUT);
    });
}

/// Normal play seeds the RNG from the host clock (DESIGN §2.1).
fn host_seed() -> (u32, u32) {
    let t = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    (t.subsec_nanos(), t.as_secs() as u32)
}

struct App {
    args: Args,
    game: Game,
    paula: Paula,
    input: Input,
    pacer: Pacer,
    clock: Clock,
    /// Host samples stepped ahead of the device, so its queue never runs dry.
    lead: u64,
    audio: Option<Audio>,
    script: Option<Script>,
    capture: Option<Capture>,
    window: Option<Arc<Window>>,
    video: Option<Video>,
    /// The newest frame stepped: its size and RGBA.
    shown: Presenter<((u32, u32), Vec<u8>)>,
    steps: u64,
    samples: Vec<[i16; 2]>,
    status: i32,
}

impl App {
    /// One PAL frame: the script's events for it, the input, the step, the sound, the picture.
    fn step(&mut self, event_loop: &ActiveEventLoop) {
        self.steps += 1;
        let n = self.steps;
        if let Some(s) = &self.script {
            for e in s.at(n).collect::<Vec<_>>() {
                match e {
                    Event::Input(h) => self.input.event(h),
                    Event::Hang => {
                        eprintln!("superfrog: the script hangs at frame {n}");
                        loop {
                            std::thread::sleep(Duration::from_secs(3600));
                        }
                    }
                }
            }
        }
        let input = self.input.frame();
        let out = self.game.step(input);
        self.samples.clear();
        self.paula.frame(&out.paula, &NoChip, &mut self.samples);
        if let Some(a) = &self.audio {
            a.push(&self.samples);
        }
        let size = (out.picture.width as u32, out.picture.height as u32);
        let rgba = picture_rgba(&out.picture);
        if let Some(c) = &mut self.capture {
            c.audio(&self.samples);
            c.input(n, input);
            if let Err(e) = c.frame(n, size.0, size.1, &rgba) {
                self.fail(event_loop, &format!("capture: {e}"));
            }
        }
        self.shown.completed((size, rgba));
        if self.args.max_frames == Some(n) {
            self.finish(event_loop);
        }
    }

    fn finish(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(c) = &self.capture
            && let Err(e) = c.finish()
        {
            return self.fail(event_loop, &format!("capture: {e}"));
        }
        event_loop.exit();
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, why: &str) {
        eprintln!("superfrog: {why}");
        self.status = 1;
        event_loop.exit();
    }

    fn show_newest(&mut self) {
        if let (Some(v), Some((size, rgba))) = (&mut self.video, self.shown.at_refresh()) {
            v.upload(*size, rgba);
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let s = self.args.scale;
        let attrs = Window::default_attributes()
            .with_title("Superfrog")
            .with_inner_size(winit::dpi::LogicalSize::new(320 * s, 256 * s))
            .with_min_inner_size(winit::dpi::LogicalSize::new(320, 256))
            .with_active(self.script.is_none());
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(event_loop, &format!("no window: {e}")),
        };
        match Video::new(window.clone(), s) {
            Ok(v) => self.video = Some(v),
            Err(e) => return self.fail(event_loop, &e),
        }
        self.window = Some(window);
        // Frame 0: the start, before any step.
        let pic = self.game.render(View::default());
        let size = (pic.width as u32, pic.height as u32);
        let rgba = picture_rgba(&pic);
        if let Some(c) = &mut self.capture
            && let Err(e) = c.frame(0, size.0, size.1, &rgba)
        {
            return self.fail(event_loop, &format!("capture: {e}"));
        }
        self.shown.completed((size, rgba));
        self.show_newest();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if event_loop.exiting() || self.video.is_none() {
            return;
        }
        if self.audio.as_ref().is_some_and(Audio::failed) {
            // The device is gone: carry on silently on the wall clock, from where it got to.
            let at = self.clock.samples();
            let rate = self.audio.take().map_or(SILENT_RATE, |a| a.rate);
            self.clock = Clock::Wall {
                start: Instant::now() - Duration::from_nanos(at * 1_000_000_000 / rate as u64),
                rate,
            };
            self.lead = 0;
        }
        let due = self.pacer.due(self.clock.samples() + self.lead);
        for _ in 0..due {
            self.step(event_loop);
            if event_loop.exiting() {
                return;
            }
        }
        if due > 0 {
            self.show_newest();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(2),
        ));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.finish(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(v) = &mut self.video {
                    v.resized(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(v) = &mut self.video {
                    v.present();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.input.event(HostInput::Key {
                        code,
                        pressed: event.state == ElementState::Pressed,
                    });
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.input.event(HostInput::MouseLeft {
                pressed: state == ElementState::Pressed,
            }),
            _ => {}
        }
    }
}
