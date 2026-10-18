//! Shell: the `superfrog` binary (DESIGN §3.2, §3.4).
//!
//! At launch, inside the event loop (the native panels need it running): the single-instance lock,
//! the remembered settings and progress, then the game data from the cache, importing the disks
//! when it is missing or unusable (`import.rs`). Then [`Game`] runs from power-on at the PAL rate
//! paced by the audio device's clock, the newest frame is presented at each display refresh at an
//! integer scale (`window.rs`) and the mixer's output is played. `--script` feeds events in place
//! of the keyboard's and answers the launch's questions, `--capture` writes what was shown and
//! played; the seatbelts are in `args.rs` (DESIGN §5.2).

mod args;
mod atomic_write;
mod audio;
mod capture;
mod home;
mod import;
mod input;
mod lock;
mod pacing;
mod progress;
mod script;
mod settings;
mod video;
mod window;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use sf_core::{DisplayData, Game, Progress, Rng, Start, View};
use sf_paula::Paula;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::args::Args;
use crate::audio::{Audio, Clock, NoChip};
use crate::capture::Capture;
use crate::import::{NativeUi, ScriptedUi, Ui};
use crate::input::{HostInput, Input, Shortcut};
use crate::lock::Lock;
use crate::pacing::{Pacer, Presenter};
use crate::script::{Event, Script};
use crate::settings::{Scale, Settings};
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
    let script = args.script.as_ref().map(|path| {
        std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {e}", path.display()))
            .and_then(|t| Script::parse(&t))
            .unwrap_or_else(|e| {
                eprintln!("superfrog: {e}");
                std::process::exit(2);
            })
    });
    if let Some(dir) = &args.capture
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("superfrog: {}: {e}", dir.display());
        std::process::exit(1);
    }

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
        args,
        home,
        script,
        run: None,
        started: false,
        status: 0,
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
    home: PathBuf,
    script: Option<Script>,
    /// The game, once the launch is through.
    run: Option<Run>,
    started: bool,
    status: i32,
}

/// Everything a running game needs.
struct Run {
    /// Held while the app runs (DESIGN §2.11).
    _lock: Lock,
    settings: Settings,
    /// The furthest progress remembered (DESIGN §2.9).
    furthest: Option<Progress>,
    display: Arc<DisplayData>,
    seed: (u32, u32),
    game: Game,
    paula: Paula,
    input: Input,
    pacer: Pacer,
    clock: Clock,
    /// Host samples stepped ahead of the device, so its queue never runs dry.
    lead: u64,
    audio: Option<Audio>,
    capture: Option<Capture>,
    window: Arc<Window>,
    video: Video,
    /// The newest frame stepped: its number, size and RGBA.
    shown: Presenter<(u64, (u32, u32), Vec<u8>)>,
    /// The number of the frame last uploaded to the GPU.
    uploaded: u64,
    steps: u64,
    samples: Vec<[i16; 2]>,
    /// A `shot` asked for at this step, taken at the next present.
    shot: Option<u64>,
}

impl App {
    /// The launch: lock, settings, progress, the window, the game data, then the game. The window
    /// opens first (black), so the picker and the messages are sheets on it rather than panels of
    /// their own; an import-only run (`--max-frames 0`) opens none.
    fn start(&mut self, event_loop: &ActiveEventLoop) {
        let scripted = |s: &Script, log: Option<PathBuf>| -> Box<dyn Ui> {
            Box::new(ScriptedUi {
                picks: s.picks.iter().cloned().collect(),
                answers: s.answers.iter().copied().collect(),
                log: Vec::new(),
                log_file: log,
            })
        };
        let log = self.args.capture.as_ref().map(|d| d.join("prompts.txt"));
        let mut ui = match &self.script {
            Some(s) => scripted(s, log.clone()),
            None => Box::new(NativeUi { parent: None }),
        };
        let lock = match Lock::acquire(&self.home) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("superfrog: {e}");
                ui.notice(&e.to_string());
                self.status = 1;
                return event_loop.exit();
            }
        };
        let (settings, settings_damaged) = settings::load(&self.home);
        let (furthest, progress_damaged) = progress::load(&self.home);

        let scale = self.args.scale.map_or(settings.scale, Scale::N);
        let window = if self.args.max_frames == Some(0) {
            None
        } else {
            let attrs = Window::default_attributes()
                .with_title("Superfrog")
                .with_inner_size(window::initial_size(scale))
                .with_min_inner_size(window::min_size())
                .with_active(self.script.is_none());
            match event_loop.create_window(attrs) {
                Ok(w) => Some(Arc::new(w)),
                Err(e) => return self.fail(event_loop, &format!("no window: {e}")),
            }
        };
        if self.script.is_none() {
            ui = Box::new(NativeUi {
                parent: window.clone(),
            });
        }
        for m in [settings_damaged, progress_damaged].into_iter().flatten() {
            eprintln!("superfrog: {m}");
            ui.notice(&m);
        }
        let data = match import::launch(&self.home, self.args.cache_from_disks.as_deref(), &mut *ui)
        {
            Ok(Some(d)) => d,
            Ok(None) => return event_loop.exit(), // The player chose Quit.
            Err(e) => {
                eprintln!("superfrog: {e}");
                if self.script.is_none() {
                    ui.notice(&e);
                }
                self.status = 1;
                return event_loop.exit();
            }
        };
        let Some(window) = window else {
            return event_loop.exit(); // --max-frames 0: the cache is in place; nothing to show.
        };

        let seed = match &self.script {
            Some(s) => s.seed.unwrap_or((0, 0)),
            None => host_seed(),
        };
        // A scripted run plays nothing through the speakers unless it names a device.
        let audio = if self.script.is_some() && self.args.audio_device.is_none() {
            None
        } else {
            Audio::open(self.args.audio_device.as_deref())
                .inspect_err(|e| eprintln!("superfrog: {e}; carrying on silently"))
                .ok()
        };
        let rate = audio.as_ref().map_or(SILENT_RATE, |a| a.rate);
        let capture = match self.args.capture.as_ref().map(|dir| Capture::new(dir, rate, seed)) {
            None => None,
            Some(Ok(c)) => Some(c),
            Some(Err(e)) => return self.fail(event_loop, &format!("capture: {e}")),
        };

        let mut video = match Video::new(window.clone(), scale) {
            Ok(v) => v,
            Err(e) => return self.fail(event_loop, &e),
        };
        if settings.fullscreen {
            if self.args.fullscreen_allowed() {
                window::set_fullscreen(&window, true);
                video.fullscreen = true;
            } else {
                eprintln!("superfrog: staying windowed: a scripted run goes fullscreen only with --allow-fullscreen");
            }
        }
        let display = Arc::new(DisplayData::new(&data));
        drop(data);
        let game = Game::with_display(display.clone(), Start::PowerOn { seed: Rng::seeded(seed.0, seed.1) });
        let mut run = Run {
            _lock: lock,
            input: Input::new(settings.keys.clone()),
            settings,
            furthest,
            display,
            seed,
            game,
            paula: Paula::new(rate),
            pacer: Pacer::new(rate),
            clock: match &audio {
                Some(a) => a.clock(),
                None => Clock::Wall { start: Instant::now(), rate },
            },
            lead: audio.as_ref().map_or(0, |a| a.rate as u64 / 25),
            audio,
            capture,
            window,
            video,
            shown: Presenter::default(),
            steps: 0,
            uploaded: 0,
            samples: Vec::new(),
            shot: None,
        };
        // Frame 0: the start, before any step.
        let pic = run.game.render(View::default());
        let size = (pic.width as u32, pic.height as u32);
        let rgba = picture_rgba(&pic);
        if let Some(c) = &mut run.capture
            && let Err(e) = c.frame(0, size.0, size.1, &rgba)
        {
            return self.fail(event_loop, &format!("capture: {e}"));
        }
        run.shown.completed((0, size, rgba));
        self.run = Some(run);
        self.show_newest();
    }

    /// One PAL frame: the script's events for it, the input, the step, the sound, the picture.
    fn step(&mut self, event_loop: &ActiveEventLoop) {
        let Some(run) = &mut self.run else { return };
        run.steps += 1;
        let n = run.steps;
        let events: Vec<Event> = self.script.as_ref().map_or(Vec::new(), |s| s.at(n).collect());
        for e in events {
            match e {
                Event::Input(h) => self.host(h),
                Event::Hang => {
                    eprintln!("superfrog: the script hangs at frame {n}");
                    loop {
                        std::thread::sleep(Duration::from_secs(3600));
                    }
                }
                Event::Resize(w, h) => {
                    let run = self.run.as_mut().expect("running");
                    let _ = run.window.request_inner_size(winit::dpi::LogicalSize::new(w, h));
                }
                Event::Scale(s) => self.set_scale(s),
                Event::Shot => self.run.as_mut().expect("running").shot = Some(n),
                Event::Continue => self.continue_game(),
            }
        }
        let run = self.run.as_mut().expect("running");
        let input = run.input.frame();
        let out = run.game.step(input);
        let p = run.game.progress();
        if progress::further(p, run.furthest) {
            run.furthest = Some(p);
            if let Err(e) = progress::save(&self.home, p) {
                eprintln!("superfrog: could not save progress: {e}");
            }
        }
        run.samples.clear();
        run.paula.frame(&out.paula, &NoChip, &mut run.samples);
        if let Some(a) = &run.audio {
            a.push(&run.samples);
        }
        let size = (out.picture.width as u32, out.picture.height as u32);
        let rgba = picture_rgba(&out.picture);
        if let Some(c) = &mut run.capture {
            c.audio(&run.samples);
            c.input(n, input);
            if let Err(e) = c.frame(n, size.0, size.1, &rgba) {
                return self.fail(event_loop, &format!("capture: {e}"));
            }
        }
        run.shown.completed((n, size, rgba));
        if self.args.max_frames == Some(n) {
            self.finish(event_loop);
        }
    }

    /// A host event: the app's shortcut, or the game's input.
    fn host(&mut self, e: HostInput) {
        let Some(run) = &mut self.run else { return };
        match run.input.event(e) {
            Some(Shortcut::ToggleFullscreen) => self.toggle_fullscreen(),
            None => {}
        }
    }

    fn toggle_fullscreen(&mut self) {
        if !self.args.fullscreen_allowed() {
            eprintln!("superfrog: fullscreen refused: a scripted run needs --allow-fullscreen");
            return;
        }
        let run = self.run.as_mut().expect("running");
        run.settings.fullscreen = !run.settings.fullscreen;
        window::set_fullscreen(&run.window, run.settings.fullscreen);
        run.video.fullscreen = run.settings.fullscreen;
        self.save_settings();
    }

    /// The scale setting (T24's View menu): remembered, and the window resized to it.
    fn set_scale(&mut self, s: Scale) {
        let run = self.run.as_mut().expect("running");
        run.settings.scale = s;
        run.video.scale = s;
        if let Scale::N(_) = s
            && !run.settings.fullscreen
        {
            let _ = run.window.request_inner_size(window::initial_size(s));
        }
        self.save_settings();
    }

    fn save_settings(&self) {
        let run = self.run.as_ref().expect("running");
        if let Err(e) = settings::save(&self.home, &run.settings) {
            eprintln!("superfrog: could not save the settings: {e}");
        }
    }

    /// Start the remembered progress as its level code would (DESIGN §2.9, T24's "Continue from
    /// World N"); nothing when there is none.
    fn continue_game(&mut self) {
        let run = self.run.as_mut().expect("running");
        let Some(p) = run.furthest else {
            eprintln!("superfrog: no progress to continue from");
            return;
        };
        let seed = Rng::seeded(run.seed.0, run.seed.1);
        run.game = Game::start_at_with(run.display.clone(), &p, seed);
    }

    fn finish(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(c) = self.run.as_ref().and_then(|r| r.capture.as_ref())
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
        let Some(run) = &mut self.run else { return };
        if let Some((n, size, rgba)) = run.shown.at_refresh() {
            run.video.upload(*size, rgba);
            run.uploaded = *n;
        }
        run.window.request_redraw();
    }

    /// A `shot` the script asked for: what the window shows for the newest frame, uploaded.
    fn take_shot(&mut self, event_loop: &ActiveEventLoop) {
        let Some(run) = &mut self.run else { return };
        let (Some(n), Some(c)) = (run.shot.take(), &mut run.capture) else { return };
        let written = run
            .video
            .shot(&c.window_path(n))
            .and_then(|shown| c.window_shown(n, run.uploaded, &shown).map_err(|e| e.to_string()));
        if let Err(e) = written {
            self.fail(event_loop, &format!("shot: {e}"));
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.started {
            self.started = true;
            self.start(event_loop);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }
        let Some(run) = &mut self.run else { return };
        if run.audio.as_ref().is_some_and(Audio::failed) {
            // The device is gone: carry on silently on the wall clock, from where it got to.
            let at = run.clock.samples();
            let rate = run.audio.take().map_or(SILENT_RATE, |a| a.rate);
            run.clock = Clock::Wall {
                start: Instant::now() - Duration::from_nanos(at * 1_000_000_000 / rate as u64),
                rate,
            };
            run.lead = 0;
        }
        let due = run.pacer.due(run.clock.samples() + run.lead);
        for _ in 0..due {
            self.step(event_loop);
            if event_loop.exiting() {
                return;
            }
        }
        if due > 0 {
            self.show_newest();
            self.take_shot(event_loop);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(2),
        ));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.finish(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(run) = &mut self.run {
                    // macOS keeps a person's drag above the minimum size, but not a resize asked
                    // for by code: the app holds the line itself.
                    let at: winit::dpi::LogicalSize<f64> = size.to_logical(run.window.scale_factor());
                    let min = window::min_size();
                    if at.width < min.width as f64 || at.height < min.height as f64 {
                        let _ = run.window.request_inner_size(winit::dpi::LogicalSize::new(
                            at.width.max(min.width as f64),
                            at.height.max(min.height as f64),
                        ));
                    }
                    run.video.resized(size.width, size.height);
                    run.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(run) = &mut self.run {
                    run.video.present();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.host(HostInput::Key {
                        code,
                        pressed: event.state == ElementState::Pressed,
                    });
                }
            }
            WindowEvent::Focused(false) => self.host(HostInput::FocusLost),
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.host(HostInput::MouseLeft {
                pressed: state == ElementState::Pressed,
            }),
            _ => {}
        }
    }
}
