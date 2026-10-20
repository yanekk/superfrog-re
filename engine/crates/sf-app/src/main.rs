//! Shell: the `superfrog` binary (DESIGN §3.2, §3.4).
//!
//! At launch, inside the event loop (the native panels need it running): the single-instance lock,
//! the remembered settings and progress, then the game data from the cache, importing the disks
//! when it is missing or unusable (`import.rs`). Then [`Game`] runs from power-on at the PAL rate
//! paced by the audio device's clock, the newest frame is presented at each display refresh at an
//! integer scale (`window.rs`) and the mixer's output is played. `--script` feeds events in place
//! of the keyboard's and answers the launch's questions, `--capture` writes what was shown and
//! played; the seatbelts are in `args.rs` (DESIGN §5.2). `--pacing`, a test's, steps the game
//! from a pacing feed as `sf-conform`'s engine run alone is stepped (`sf_core::feed`, T211). Every setting and feature is reached
//! through the menu bar, as a `menu::Command` (DESIGN §2.7, §7); save states are `states.rs`. Gamepads
//! come through gilrs (`pad.rs`), except under `--script`, whose `pad …` lines stand in for them.
//! Game → Start Level… starts a level with chosen values, and with the recorder on every stepped
//! frame's input is recorded until Stop Recording saves it as a bug zip (`recorder.rs`).
//! `--replay` plays such a zip back in the window, watch-only, holding on its last frame
//! (bug-recorder DESIGN §2.7).

mod args;
mod atomic_write;
mod audio;
mod capture;
mod controls_window;
mod home;
mod import;
mod input;
mod levelfile;
mod lock;
mod menu;
mod pacing;
mod pad;
mod progress;
mod recorder;
mod script;
mod settings;
mod start_level_window;
mod states;
mod video;
mod window;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use sf_assets::LevelData;
use sf_core::feed::{Feed, FeedFrame};
use sf_core::{DisplayData, Game, LevelId, Picture, Progress, Rng, Start};
use sf_paula::Paula;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::args::Args;
use crate::audio::{Audio, Clock};
use crate::capture::Capture;
use crate::controls_window::{ControlsWindow, OpenControls};
use crate::import::{NativeUi, ScriptedUi, Ui};
use crate::input::{HostInput, Input};
use crate::lock::Lock;
use crate::menu::{Command, MenuBar};
use crate::pacing::{Pacer, Presenter};
use crate::pad::PadSource;
use crate::recorder::{Recorder, Replay};
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
    let feed = args.pacing.as_ref().map(|path| {
        std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| Feed::parse(&t))
            .and_then(|f| match f.engine == home::build() {
                true => Ok(f),
                false => Err(format!(
                    "written for engine build {:?}, and this is {:?}: record it again \
                     (`sf-conform feed`)",
                    f.engine,
                    home::build()
                )),
            })
            .unwrap_or_else(|e| {
                eprintln!("superfrog: {}: {e}", path.display());
                std::process::exit(2);
            })
    });
    if let Some(dir) = &args.capture
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("superfrog: {}: {e}", dir.display());
        std::process::exit(1);
    }

    menu::before_launch();
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        // A scripted run must not take the keyboard from whatever the person is using.
        builder.with_activate_ignoring_other_apps(!args.unattended());
        // The app's own menu bar replaces winit's (`menu.rs`).
        builder.with_default_menu(false);
    }
    let event_loop = builder.build().unwrap_or_else(|e| {
        eprintln!("superfrog: no event loop: {e}");
        std::process::exit(1);
    });
    let log = args.capture.as_ref().map(|d| d.join("prompts.txt"));
    let ui: Box<dyn Ui> = match &script {
        Some(s) => Box::new(ScriptedUi {
            picks: s.picks.iter().cloned().collect(),
            answers: s.answers.iter().copied().collect(),
            saves: s.saves.iter().cloned().collect(),
            opens: s.opens.iter().cloned().collect(),
            level_answers: s.level_answers.iter().copied().collect(),
            starts: s.starts.iter().cloned().collect(),
            endings: s.endings.iter().copied().collect(),
            notes: s.notes.iter().cloned().collect(),
            log: Vec::new(),
            log_file: log,
        }),
        None => Box::new(NativeUi { parent: None }),
    };
    // A scripted run never saves into the repo: its bugs go into its own home unless it names
    // a folder (DESIGN §5.2).
    let bugs_dir = match (&args.bugs_dir, &script) {
        (Some(d), _) => d.clone(),
        (None, Some(_)) => home.join("bugs"),
        (None, None) => recorder::bugs_dir(None),
    };
    let mut app = App {
        args,
        bugs_dir,
        home,
        script,
        feed,
        ui,
        menu: None,
        controls: None,
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
    /// Where a bug recording is saved (`recorder::bugs_dir`).
    bugs_dir: PathBuf,
    script: Option<Script>,
    /// `--pacing`: the feed, until the game starts and takes it.
    feed: Option<Feed>,
    /// Every question to the player: the native panels, or the script's answers.
    ui: Box<dyn Ui>,
    menu: Option<MenuBar>,
    /// Game → Controls…, while it is open.
    controls: Option<OpenControls>,
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
    /// The game was started from Game → Start Level…: it never moves the saved progress
    /// (bug-recorder DESIGN §2.1), until a game replaces it (Reset, Continue, Import Disks, Play
    /// Level File); a loaded state brings its own (`states::from_dialog`).
    from_dialog: bool,
    /// The bug recording, while one runs.
    recorder: Recorder,
    /// `--replay`: the recording fed in place of the player's input; `None` in normal play.
    replay: Option<Replay>,
    display: Arc<DisplayData>,
    /// [`sf_assets::cache_hash`] of the game data: what a save state is tagged with.
    cache_hash: [u8; 32],
    /// Which save-state slots hold a state (`states.rs`).
    states: [bool; states::SLOTS as usize],
    /// The disk levels, for File → Export Level (`levelfile.rs`).
    levels: BTreeMap<LevelId, LevelData>,
    seed: (u32, u32),
    game: Game,
    /// `--pacing`: what each step is fed in place of the input, in order (`sf_core::feed`).
    feed: Option<Vec<FeedFrame>>,
    /// The newest frame's picture, for the capture's hashes.
    picture: Picture,
    paula: Paula,
    /// The mixer's output rate.
    rate: u32,
    /// Game → Pause: the game is not stepped (`menu.rs`).
    paused: bool,
    input: Input,
    /// The real pads; none under a script.
    pads: Option<Box<dyn PadSource>>,
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
    /// The number of the frame last presented, and how many stepped frames were never presented
    /// (frame pacing, DESIGN §2.5): written to the capture's `pacing.txt`.
    presented: u64,
    dropped: u64,
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
        let menu = menu::build_menu();
        menu.install(event_loop);
        self.menu = Some(menu);
        let lock = match Lock::acquire(&self.home) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("superfrog: {e}");
                self.ui.notice(&e.to_string());
                self.status = 1;
                return event_loop.exit();
            }
        };
        // A replay's zip is read before any window opens: one that cannot be played says why
        // and ends the run.
        let replayed = match &self.args.replay {
            None => None,
            Some(path) => match std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| recorder::read_zip(&b)) {
                Ok(rec) => Some(rec),
                Err(e) => {
                    let m = format!("{} cannot be replayed: {e}", path.display());
                    eprintln!("superfrog: {m}");
                    self.ui.notice(&m);
                    self.status = 1;
                    return event_loop.exit();
                }
            },
        };
        let (settings, settings_damaged) = settings::load(&self.home);
        let (furthest, progress_damaged) = progress::load(&self.home);

        let scale = self.args.scale.map_or(settings.scale, Scale::N);
        let window = if self.args.max_frames == Some(0) {
            None
        } else {
            let attrs = Window::default_attributes()
                .with_title("Superfrog")
                .with_inner_size(window::initial_size(scale, settings.widescreen))
                .with_min_inner_size(window::min_size(settings.widescreen))
                .with_active(!self.args.unattended());
            match event_loop.create_window(attrs) {
                Ok(w) => Some(Arc::new(w)),
                Err(e) => return self.fail(event_loop, &format!("no window: {e}")),
            }
        };
        if self.script.is_none() {
            self.ui = Box::new(NativeUi {
                parent: window.clone(),
            });
        }
        for m in [settings_damaged, progress_damaged].into_iter().flatten() {
            eprintln!("superfrog: {m}");
            self.ui.notice(&m);
        }
        let data = match import::launch(&self.home, self.args.cache_from_disks.as_deref(), &mut *self.ui)
        {
            Ok(Some(d)) => d,
            Ok(None) => return event_loop.exit(), // The player chose Quit.
            Err(e) => {
                eprintln!("superfrog: {e}");
                if self.script.is_none() {
                    self.ui.notice(&e);
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
        // A scripted run or a replay plays nothing through the speakers unless it names a device.
        let audio = if self.args.unattended() && self.args.audio_device.is_none() {
            None
        } else {
            Audio::open(self.args.audio_device.as_deref())
                .inspect_err(|e| eprintln!("superfrog: {e}; carrying on silently"))
                .ok()
        };
        let rate = audio.as_ref().map_or(SILENT_RATE, |a| a.rate);
        let hashes = self.args.capture_hashes;
        let capture = match self.args.capture.as_ref().map(|dir| Capture::new(dir, rate, seed, hashes)) {
            None => None,
            Some(Ok(c)) => Some(c),
            Some(Err(e)) => return self.fail(event_loop, &format!("capture: {e}")),
        };

        let mut video = match Video::new(window.clone(), scale) {
            Ok(v) => v,
            Err(e) => return self.fail(event_loop, &e),
        };
        video.crt = settings.crt;
        if settings.fullscreen {
            if self.args.fullscreen_allowed() {
                window::set_fullscreen(&window, true);
                video.fullscreen = true;
            } else {
                eprintln!("superfrog: staying windowed: a scripted run goes fullscreen only with --allow-fullscreen");
            }
        }
        // A scripted run reads no real pad: its `pad …` lines are its pads. A replay's pads
        // would change nothing.
        let (pads, present) = if self.args.unattended() {
            (None, Vec::new())
        } else {
            match pad::Gilrs::open() {
                Ok((g, present)) => (Some(Box::new(g) as Box<dyn PadSource>), present),
                Err(e) => {
                    eprintln!("superfrog: {e}; carrying on with the keyboard");
                    (None, Vec::new())
                }
            }
        };
        let display = Arc::new(DisplayData::new(&data));
        let cache_hash = sf_assets::cache_hash(&data);
        let levels = data.levels;
        // A pacing feed starts where its run alone did: the RNG as the original seeded it.
        let feed = self.feed.take();
        let start_rng = feed.as_ref().map_or_else(|| Rng::seeded(seed.0, seed.1), |f| f.seed);
        // A replay starts where its recording did: the level from the dialog's values and seed.
        let mut game = match &replayed {
            Some(rec) => Game::start_level(display.clone(), &rec.setup, rec.seed),
            None => Game::with_display(display.clone(), Start::PowerOn { seed: start_rng }),
        };
        game.set_view(window::view(settings.widescreen));
        let mut run = Run {
            _lock: lock,
            input: Input::new(settings.keys.clone(), settings.pad.clone()),
            pads,
            settings,
            furthest,
            // A replay is a dialog-started game: it never moves the saved progress.
            from_dialog: replayed.is_some(),
            recorder: Recorder::Off,
            replay: replayed.as_ref().map(Replay::new),
            display,
            cache_hash,
            states: states::filled(&self.home),
            levels,
            seed,
            game,
            feed: feed.map(|f| f.frames),
            picture: Picture::default(),
            paula: Paula::booted(rate),
            rate,
            paused: false,
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
            presented: 0,
            dropped: 0,
            samples: Vec::new(),
            shot: None,
        };
        // Frame 0: the start, before any step.
        let pic = run.game.render(run.game.view());
        let size = (pic.width as u32, pic.height as u32);
        let rgba = picture_rgba(&pic);
        if let Some(c) = &mut run.capture
            && let Err(e) = c.frame(0, (size.0, size.1, &rgba), &run.game, &pic)
        {
            return self.fail(event_loop, &format!("capture: {e}"));
        }
        run.shown.completed((0, size, rgba));
        run.picture = pic;
        for e in present {
            run.input.event(HostInput::Pad(e));
        }
        self.run = Some(run);
        self.replay_finished();
        self.show_menu();
        self.show_newest();
    }

    /// One PAL frame: the script's events for it, the input, the step, the sound, the picture.
    fn step(&mut self, event_loop: &ActiveEventLoop) {
        let Some(run) = &self.run else { return };
        // Step n's events land before it: `steps` counts it once they are through.
        let n = run.steps + 1;
        let events: Vec<Event> = self.script.as_ref().map_or(Vec::new(), |s| s.at(n).collect());
        for e in events {
            match e {
                // While the Controls window is open it has the keyboard, as it would for a person.
                Event::Input(HostInput::Key { code, pressed }) if self.controls.is_some() => {
                    if pressed {
                        self.controls_key(code);
                    }
                }
                Event::Input(HostInput::Pad(e)) => self.pad_event(e),
                Event::Input(h) => self.host(event_loop, h),
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
                Event::Shot => self.run.as_mut().expect("running").shot = Some(n),
                Event::Occlude(on) => self.run.as_mut().expect("running").video.occluded = on,
                Event::Command(c) => self.on_command(event_loop, c),
                Event::Controls(a) => self.controls_action(a),
                Event::Exit => {
                    let run = self.run.as_mut().expect("running");
                    run.game.poke(sf_core::layout::h0(0xa06), &1u32.to_be_bytes());
                    eprintln!("superfrog: the level's exit set before step {n}");
                }
            }
            if event_loop.exiting() {
                return;
            }
        }
        let run = self.run.as_mut().expect("running");
        run.steps = n;
        if run.paused {
            // Paused: the game is not stepped and the picture stays; a press made meanwhile does
            // not reach the game when it resumes. The frame still counts towards --max-frames.
            let _ = run.input.frame();
            if let (Some(c), Some((_, size, rgba))) = (&mut run.capture, run.shown.at_refresh())
                && let Err(e) = c.frame(n, (size.0, size.1, rgba), &run.game, &run.picture)
            {
                return self.fail(event_loop, &format!("capture: {e}"));
            }
            if self.args.max_frames == Some(n) {
                self.finish(event_loop);
            }
            return;
        }
        let input = run.input.frame();
        // A replay feeds its recording's input in place of the player's, and holds once that has
        // run out: no further steps, the last frame shown (and captured) as it is.
        let input = match &mut run.replay {
            None => input,
            Some(r) => match r.next() {
                Some(recorded) => recorded,
                None => {
                    if self.args.max_frames == Some(n) {
                        self.finish(event_loop);
                    }
                    return;
                }
            },
        };
        // Under `--pacing` the step is the feed's, in place of the input: one frame of play as
        // the engine run alone played it (the original's step twice, frames under a loading
        // screen after it), so one or more steps of the game.
        let stepped = match &run.feed {
            None => Ok((input, vec![run.game.step(input)])),
            Some(feed) => match feed.get(n as usize - 1) {
                Some(f) => Ok((f.input, f.play(&mut run.game))),
                None => Err(feed.len()),
            },
        };
        let (input, outs) = match stepped {
            Ok(s) => s,
            Err(len) => return self.fail(event_loop, &format!("the pacing feed ends at frame {len}")),
        };
        let run = self.run.as_mut().expect("running");
        run.recorder.step(input);
        let p = run.game.progress();
        if !run.from_dialog && progress::further(p, run.furthest) {
            run.furthest = Some(p);
            if let Err(e) = progress::save(&self.home, p) {
                eprintln!("superfrog: could not save progress: {e}");
            }
        }
        // A load's vertical blanks (under `--pacing`): the mixer plays them, unheard, as the
        // original's Paula did while it loaded, so its channels leave the load as the original's.
        for blank in run.game.take_load_paula() {
            run.samples.clear();
            run.paula.frame(&blank, &run.game, &mut run.samples);
        }
        run.samples.clear();
        let mut ends = Vec::with_capacity(outs.len());
        for out in &outs {
            run.paula.frame(&out.mixed(), &run.game, &mut run.samples);
            ends.push(run.samples.len());
        }
        if let Some(a) = &run.audio {
            a.push(&run.samples);
        }
        let picture = outs.into_iter().last().expect("a frame steps").picture;
        let size = (picture.width as u32, picture.height as u32);
        let rgba = picture_rgba(&picture);
        let clock = run.clock.samples();
        if let Some(c) = &mut run.capture {
            c.audio(n, &run.samples, &ends);
            c.clock(n, clock);
            c.input(n, input);
            if let Err(e) = c.frame(n, (size.0, size.1, &rgba), &run.game, &picture) {
                return self.fail(event_loop, &format!("capture: {e}"));
            }
        }
        run.shown.completed((n, size, rgba));
        run.picture = picture;
        self.replay_finished();
        let run = self.run.as_mut().expect("running");
        if run.recorder.full() {
            eprintln!("superfrog: the recording is a day long, the longest one may be: stopped");
            self.save_recording();
        }
        if self.args.max_frames == Some(n) {
            self.finish(event_loop);
        }
    }

    /// A host event: the app's shortcut, or the game's input.
    fn host(&mut self, event_loop: &ActiveEventLoop, e: HostInput) {
        let Some(run) = &mut self.run else { return };
        if let Some(c) = run.input.event(e) {
            self.on_command(event_loop, c);
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

    /// The scale setting (View → Scale): remembered, and the window resized to it.
    fn set_scale(&mut self, s: Scale) {
        let run = self.run.as_mut().expect("running");
        run.settings.scale = s;
        run.video.scale = s;
        if let Scale::N(_) = s
            && !run.settings.fullscreen
        {
            let _ = run.window.request_inner_size(window::initial_size(s, run.settings.widescreen));
        }
        self.save_settings();
    }

    /// View → CRT Filter: the scaled picture through `crt.wgsl`, remembered.
    fn toggle_crt(&mut self) {
        let run = self.run.as_mut().expect("running");
        run.settings.crt = !run.settings.crt;
        run.video.crt = run.settings.crt;
        run.window.request_redraw();
        self.save_settings();
    }

    /// View → Widescreen: the game draws 16:9 from the next step on, remembered. A window keeps
    /// its scale and grows or shrinks to the new picture; fullscreen refits it.
    fn toggle_widescreen(&mut self) {
        let run = self.run.as_mut().expect("running");
        let wide = !run.settings.widescreen;
        run.settings.widescreen = wide;
        run.game.set_view(window::view(wide));
        run.window.set_min_inner_size(Some(window::min_size(wide)));
        if !run.settings.fullscreen {
            let n = match run.video.scale {
                Scale::N(n) => n,
                Scale::Auto => run.video.points_scale(),
            };
            let _ = run.window.request_inner_size(window::initial_size(Scale::N(n), wide));
        }
        self.save_settings();
    }

    fn save_settings(&self) {
        let run = self.run.as_ref().expect("running");
        if let Err(e) = settings::save(&self.home, &run.settings) {
            eprintln!("superfrog: could not save the settings: {e}");
        }
    }

    fn finish(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(r) = self.run.as_mut()
            && let Some(c) = r.capture.as_mut()
            && let Err(e) = c
                .finish()
                .and_then(|()| c.pacing(r.steps, r.presented, r.dropped, r.clock.samples()))
                .and_then(|()| c.unported(r.game.unported()))
                .and_then(|()| c.memory())
        {
            return self.fail(event_loop, &format!("capture: {e}"));
        }
        event_loop.exit();
    }

    /// Say once, as the replay's last input is played (at once for a recording of no frames),
    /// the frame it holds on.
    fn replay_finished(&mut self) {
        if let Some(n) = self.run.as_mut().and_then(|r| r.replay.as_mut()).and_then(Replay::just_finished) {
            eprintln!("superfrog: replay finished at frame {n}");
        }
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
        while let Some(c) = menu::next_command() {
            self.on_command(event_loop, c);
            if event_loop.exiting() {
                return;
            }
        }
        while let Some(a) = ControlsWindow::next_click() {
            self.controls_action(a);
        }
        while let Some(e) = self.run.as_mut().and_then(|r| r.pads.as_mut()).and_then(|p| p.next()) {
            self.pad_event(e);
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
            // A shot is of its own step's frame, even when later steps in this catch-up end the
            // run (a throttled window catches up many at once).
            if self.run.as_ref().is_some_and(|r| r.shot.is_some()) {
                self.show_newest();
                self.take_shot(event_loop);
            }
        }
        if due > 0 {
            self.show_menu();
            self.show_newest();
            self.take_shot(event_loop);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(2),
        ));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.controls.as_ref().is_some_and(|c| c.window.id() == id) {
            match event {
                WindowEvent::CloseRequested => self.close_controls(),
                WindowEvent::KeyboardInput { event, .. }
                    if !event.repeat && event.state == ElementState::Pressed =>
                {
                    if let PhysicalKey::Code(code) = event.physical_key {
                        self.controls_key(code);
                    }
                }
                _ => {}
            }
            return;
        }
        match event {
            WindowEvent::CloseRequested if self.run.is_some() => self.on_command(event_loop, Command::Quit),
            WindowEvent::CloseRequested => self.finish(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(run) = &mut self.run {
                    // macOS keeps a person's drag above the minimum size, but not a resize asked
                    // for by code: the app holds the line itself.
                    let at: winit::dpi::LogicalSize<f64> = size.to_logical(run.window.scale_factor());
                    let min = window::min_size(run.settings.widescreen);
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
                    if run.uploaded > run.presented {
                        run.dropped += run.uploaded - run.presented - 1;
                        run.presented = run.uploaded;
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.host(event_loop, HostInput::Key {
                        code,
                        pressed: event.state == ElementState::Pressed,
                    });
                }
            }
            WindowEvent::Focused(false) => self.host(event_loop, HostInput::FocusLost),
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.host(event_loop, HostInput::MouseLeft {
                pressed: state == ElementState::Pressed,
            }),
            _ => {}
        }
    }
}
