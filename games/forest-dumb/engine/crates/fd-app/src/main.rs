//! Shell: the `fd-app` binary, the app the owner opens (DESIGN §2.3, §2.7, §2.9, §2.12).
//!
//! At launch, inside the event loop: the single-copy lock, the remembered settings, the game's data
//! from the two zips (`disks.rs`; a plain message naming where it looked, and exit 1, when they
//! cannot be read), then straight into `W1-1` at the level sync point, silent. The game is stepped
//! once a PAL frame of 313 lines on the wall clock (`Pacer::new(rate, 313)`, docs/frame-length.md)
//! and the newest frame is presented at each refresh at a whole-number scale. Keys and pads feed
//! the game through `input.rs`; Controls and Quit are in the menu bar (`menu.rs`), Esc also quits.
//!
//! Game → Start Level… starts W1-1 afresh, and with its recorder on every stepped frame's input is
//! recorded until Stop Recording saves it as a bug zip (`recorder.rs`, DESIGN §2.8); while it runs
//! the recording bar sits under the picture, the window that much taller.
//!
//! `--script` feeds events in place of the keyboard's and `--capture` writes what was shown, the
//! inputs and each frame's state hash (`fd_app::HASHES`); `--max-frames` and `--timeout` bound a
//! run (DESIGN §5.2).

mod args;
mod controls;
mod disks;
mod home;
mod input;
mod menu;
mod recorder;
mod script;
mod settings;

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use amiga_app::audio::Clock;
use amiga_app::capture::Capture;
use amiga_app::controls_window::ControlsWindow;
use amiga_app::lock::{self, Lock};
use amiga_app::pacing::{Pacer, Presenter};
use amiga_app::pad::{self, PadSource};
use amiga_app::recording_bar::{BAR_HEIGHT, RecordingBar, elapsed};
use amiga_app::video::Video;
use fd_core::recording::{LevelId, recorder as start_recording};
use fd_core::{Event as GameEvent, Game, Start};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::args::Args;
use crate::controls::OpenControls;
use crate::input::{HostInput, Input};
use crate::menu::{Command, MenuBar};
use crate::recorder::{EndChoice, NativeUi, Recorder, ScriptedUi, Ui};
use crate::script::{Event, Script};
use crate::settings::{Scale, Settings};

/// The wall clock's rate in host samples: the app is silent (DESIGN §2.3), so nothing else paces it.
const RATE: u32 = 48_000;

/// The game's frame, in lines (docs/frame-length.md).
const FRAME_LINES: u32 = 313;

/// Exit status when `--timeout` stops a run whose process group it cannot stop whole.
const TIMED_OUT: i32 = 124;

/// `fast-until`'s steps per turn of the event loop, so the window keeps drawing meanwhile.
const FAST_BATCH: u64 = 50;

fn main() {
    let args = match args::parse(std::env::args_os().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("fd-app: {e}");
            std::process::exit(2);
        }
    };
    if let Some(s) = args.timeout_s {
        watchdog(Duration::from_secs(s));
    }
    let home = home::resolve(
        args.home.as_deref(),
        std::env::var_os("FOREST_DUMB_HOME"),
        std::env::var_os("HOME"),
    );
    let script = args.script.as_ref().map(|path| {
        std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {e}", path.display()))
            .and_then(|t| script::parse(&t).map_err(String::from))
            .unwrap_or_else(|e| {
                eprintln!("fd-app: {e}");
                std::process::exit(2);
            })
    });
    if let Some(dir) = &args.capture
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("fd-app: {}: {e}", dir.display());
        std::process::exit(1);
    }
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
        eprintln!("fd-app: no event loop: {e}");
        std::process::exit(1);
    });
    let ui: Box<dyn Ui> = match &script {
        Some(s) => Box::new(ScriptedUi::new(
            s.answers.clone(),
            args.capture
                .as_ref()
                .map(|d| d.join(amiga_app::capture::PROMPTS)),
        )),
        None => Box::new(NativeUi::default()),
    };
    let bugs_dir = recorder::bugs_dir(args.bugs_dir.as_deref(), script.is_some(), &home);
    let mut app = App {
        args,
        home,
        bugs_dir,
        ui,
        script,
        menu: None,
        controls: None,
        run: None,
        started: false,
        status: 0,
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("fd-app: {e}");
        std::process::exit(1);
    }
    std::process::exit(app.status);
}

/// `--timeout`: after `limit` of wall clock, stop this process and, when it leads its own process
/// group (as `fd-e2e` launches it), every process in the group.
fn watchdog(limit: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(limit);
        eprintln!("fd-app: --timeout {} s reached; stopping", limit.as_secs());
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

struct App {
    args: Args,
    home: PathBuf,
    /// Where a bug recording is saved (`recorder::bugs_dir`).
    bugs_dir: PathBuf,
    /// The recorder's windows, real or a script's answers.
    ui: Box<dyn Ui>,
    script: Option<Script>,
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
    /// Held while the app runs (DESIGN §2.12).
    _lock: Lock,
    settings: Settings,
    game: Game,
    input: Input,
    /// The real pads; none under a script, whose `pad …` lines stand in for them.
    pads: Option<Box<dyn PadSource>>,
    pacer: Pacer,
    clock: Clock,
    capture: Option<Capture>,
    /// `--capture`'s `hashes.txt`, written as the run goes.
    hashes: Option<std::io::BufWriter<std::fs::File>>,
    window: Arc<Window>,
    video: Video,
    /// The newest frame stepped: its number, size and RGBA.
    shown: Presenter<(u64, (u32, u32), Vec<u8>)>,
    uploaded: u64,
    presented: u64,
    dropped: u64,
    steps: u64,
    shot: Option<u64>,
    /// The game picture's size: the window's least, less the bar.
    picture: (u32, u32),
    /// The bug recording, while one runs (T14).
    recorder: Recorder,
    /// The recording bar, once a recording has first shown it; whether it shows is `video.bar`.
    bar: Option<RecordingBar>,
}

/// The window's inner size at the scale: `N(n)` is n times the picture, `Auto` twice.
fn initial_size(scale: Scale, picture: (u32, u32)) -> LogicalSize<u32> {
    let n = match scale {
        Scale::Auto => 2,
        Scale::N(n) => n,
    };
    LogicalSize::new(picture.0 * n, picture.1 * n)
}

/// The window's least inner size: the picture at 1×, and the bar under it while it shows.
fn min_size(picture: (u32, u32), bar: bool) -> LogicalSize<u32> {
    LogicalSize::new(picture.0, picture.1 + if bar { BAR_HEIGHT } else { 0 })
}

/// The game's picture as the window gets it: its size and RGBA.
fn frame_of(game: &Game) -> ((u32, u32), Vec<u8>) {
    let pic = game.picture();
    let size = (pic.width as u32, pic.height as u32);
    let mut rgba = vec![0; size.0 as usize * size.1 as usize * 4];
    fd_core::render::rgba(&pic, &mut rgba);
    (size, rgba)
}

impl App {
    /// The launch: lock, settings, the disks, the window, then the game.
    fn start(&mut self, event_loop: &ActiveEventLoop) {
        let menu = menu::build_menu();
        menu.install(event_loop);
        self.menu = Some(menu);
        let lock = match lock::acquire(&self.home, "Forest Dumb") {
            Ok(l) => l,
            Err(e) => return self.refuse(event_loop, &e),
        };
        let (settings, damaged) = settings::read(&self.home);
        if let Some(m) = damaged {
            eprintln!("fd-app: {m}");
        }
        let data = match disks::folder(self.args.disks.as_deref()).and_then(|d| disks::read(&d)) {
            Ok(d) => d,
            Err(e) => return self.refuse(event_loop, &e),
        };
        let game = Game::new(data, Start::Level1);
        let (size, rgba) = frame_of(&game);
        let scale = self.args.scale.map_or(settings.scale, Scale::N);
        let attrs = Window::default_attributes()
            .with_title("Forest Dumb")
            .with_inner_size(initial_size(scale, size))
            .with_min_inner_size(min_size(size, false))
            .with_active(!self.args.unattended());
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(event_loop, &format!("no window: {e}")),
        };
        let mut video = match Video::new(window.clone(), scale) {
            Ok(v) => v,
            Err(e) => return self.fail(event_loop, &e),
        };
        video.bar_points = BAR_HEIGHT;
        self.ui.attach(window.clone());
        let (pads, present) = if self.args.unattended() {
            (None, Vec::new())
        } else {
            match pad::Gilrs::open() {
                Ok((g, present)) => (Some(Box::new(g) as Box<dyn PadSource>), present),
                Err(e) => {
                    eprintln!("fd-app: {e}; carrying on with the keyboard");
                    (None, Vec::new())
                }
            }
        };
        let (capture, hashes) = match &self.args.capture {
            None => (None, None),
            Some(dir) => {
                let made = Capture::new(dir, RATE).and_then(|mut c| {
                    c.frame(0, (size.0, size.1, &rgba))?;
                    let mut h =
                        std::io::BufWriter::new(std::fs::File::create(dir.join(fd_app::HASHES))?);
                    h.write_all(fd_app::hash_line(0, &game).as_bytes())?;
                    Ok((c, h))
                });
                match made {
                    Ok((c, h)) => (Some(c), Some(h)),
                    Err(e) => return self.fail(event_loop, &format!("capture: {e}")),
                }
            }
        };
        let mut run = Run {
            _lock: lock,
            input: Input::new(settings.keys.clone(), settings.pad.clone()),
            settings,
            game,
            pads,
            pacer: Pacer::new(RATE, FRAME_LINES),
            clock: Clock::Wall {
                start: Instant::now(),
                rate: RATE,
            },
            capture,
            hashes,
            window,
            video,
            shown: Presenter::default(),
            uploaded: 0,
            presented: 0,
            dropped: 0,
            steps: 0,
            shot: None,
            picture: size,
            recorder: Recorder::Off,
            bar: None,
        };
        run.shown.completed((0, size, rgba));
        self.run = Some(run);
        for e in present {
            self.pad_event(e);
        }
        eprintln!("fd-app: playing W1-1");
        self.show_newest();
    }

    /// A launch that cannot go on: the reason on stderr and, for a person, in a plain message;
    /// exit 1 (DESIGN §2.12).
    fn refuse(&mut self, event_loop: &ActiveEventLoop, why: &str) {
        eprintln!("fd-app: {why}");
        if !self.args.unattended() {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Forest Dumb")
                .set_description(why)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }
        self.status = 1;
        event_loop.exit();
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, why: &str) {
        eprintln!("fd-app: {why}");
        self.status = 1;
        event_loop.exit();
    }

    /// Act on `c`: the one place a menu item, its shortcut or a script's `menu …` changes the app.
    fn on_command(&mut self, event_loop: &ActiveEventLoop, c: Command) {
        let steps = self.run.as_ref().map_or(0, |r| r.steps);
        // The step it came after, so a test can line the frames up with it.
        eprintln!("fd-app: menu {} after step {steps}", c.id());
        if let Some(action) = self.ends_recording(c)
            && !self.end_recording(action)
        {
            return self.show_menu();
        }
        match c {
            Command::StartLevel => self.start_level(),
            Command::StopRecording => self.stop_recording(),
            Command::Controls => self.open_controls(event_loop),
            // Settings are saved when they change, never on the way out: a damaged file stays as
            // it is until the owner changes a setting (DESIGN §2.12).
            Command::Quit => self.finish(event_loop),
        }
        self.show_menu();
    }

    /// What "Recording in progress" calls `c`, when taking it would end the recording running.
    fn ends_recording(&self, c: Command) -> Option<&'static str> {
        if !self.run.as_ref()?.recorder.is_recording() {
            return None;
        }
        match c {
            Command::StartLevel => Some("Start Level"),
            Command::Quit => Some("Quit"),
            _ => None,
        }
    }

    /// Ask "Recording in progress" before `action`: `false` for Cancel (still recording, the
    /// action not taken); else the recording saved or dropped as answered, and the action goes on.
    fn end_recording(&mut self, action: &str) -> bool {
        let choice = self.ui.recording_in_progress(action);
        self.after_panel();
        match choice {
            EndChoice::Cancel => {
                eprintln!("fd-app: {action} cancelled; still recording");
                false
            }
            EndChoice::Discard => {
                self.run.as_mut().expect("running").recorder = Recorder::Off;
                eprintln!("fd-app: recording discarded");
                true
            }
            EndChoice::Save => {
                self.save_recording();
                true
            }
        }
    }

    /// Game → Start Level…: the window, then W1-1 started afresh, recording from its first frame
    /// when the recorder was on. Cancelled, the game carries on.
    fn start_level(&mut self) {
        let answer = self.ui.start_level();
        self.after_panel();
        let Some((level, record)) = answer else {
            return;
        };
        let run = self.run.as_mut().expect("running");
        // The one level a recording names today (`LevelId`), started as the app starts it.
        let LevelId::W1_1 = level;
        run.game = Game::new(run.game.data().clone(), Start::Level1);
        run.recorder = match record {
            true => start_recording(level, fd_tools::ENGINE_BUILD),
            false => Recorder::Off,
        };
        // The level's first frame at once: the picture a Stop before any step saves.
        let (size, rgba) = frame_of(&run.game);
        run.shown.completed((run.steps, size, rgba));
        let recording = if record { ", recording" } else { "" };
        eprintln!("fd-app: started {level} after step {}{recording}", run.steps);
        self.show_newest();
    }

    /// Game → Stop Recording: the save window; nothing when no recording runs (the item is disabled
    /// then).
    fn stop_recording(&mut self) {
        if !self.run.as_ref().is_some_and(|r| r.recorder.is_recording()) {
            eprintln!("fd-app: not recording");
            return;
        }
        self.save_recording();
    }

    /// The save window for the recording, which ends here: its screenshot the last frame stepped
    /// at 1× (what `fd-conform bug replay` holds its last picture against), where play is, the
    /// zip's name. A write that fails says so and the window stays open, so the note is not lost.
    fn save_recording(&mut self) {
        let run = self.run.as_mut().expect("running");
        let at = run.game.location();
        let Some(mut rec) = std::mem::take(&mut run.recorder).finish(at, String::new()) else {
            return;
        };
        let ((w, h), rgba) = frame_of(&run.game);
        let shot = amiga_app::capture::png(w, h, &rgba);
        let path = recorder::zip_name(&self.bugs_dir, rec.level, recorder::local_now());
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut save = |note: &str| {
            rec.note = note.replace(['\n', '\r'], " ");
            recorder::write_zip(&path, &rec, &shot).map_err(|e| {
                let m = format!(
                    "The bug recording could not be saved as “{}”: {e}.",
                    path.display()
                );
                eprintln!("fd-app: {m}");
                m
            })
        };
        if self.ui.save_recording(&shot, &at, &name, &mut save) {
            let shown = path.display().to_string();
            eprintln!(
                "fd-app: recording saved to {shown} ({} frames)",
                rec.inputs.len()
            );
            self.ui
                .notice(&format!("The bug recording is saved as {shown}."));
        } else {
            eprintln!("fd-app: recording discarded");
        }
        self.after_panel();
    }

    /// A modal window has closed: the time it was open is not caught up, the game carries on from
    /// now.
    fn after_panel(&mut self) {
        if let Some(run) = self.run.as_mut() {
            run.pacer.skip_to(run.clock.samples());
        }
    }

    /// The menu and the bar follow the recorder: Stop Recording enabled and the bar shown exactly
    /// while a recording runs.
    fn show_menu(&mut self) {
        let Some(run) = &mut self.run else { return };
        let on = run.recorder.is_recording();
        if let Some(m) = &self.menu {
            m.show_recording(on);
        }
        if on != run.video.bar {
            run.video.bar = on;
            if on && run.bar.is_none() {
                match RecordingBar::new(&run.window) {
                    Ok(b) => run.bar = Some(b),
                    Err(e) => eprintln!("fd-app: no recording bar: {e}"),
                }
            }
            if let Some(b) = &mut run.bar {
                b.set_shown(on);
            }
            eprintln!(
                "fd-app: recording bar {} after step {}",
                if on { "shown" } else { "hidden" },
                run.steps
            );
            // The size before the minimum changes: winit grows a window below a new minimum at
            // once, and the bar's height is added to the size it had, not to that.
            let at = run
                .window
                .inner_size()
                .to_logical::<f64>(run.window.scale_factor());
            run.window
                .set_min_inner_size(Some(min_size(run.picture, on)));
            let by = if on { BAR_HEIGHT as f64 } else { -(BAR_HEIGHT as f64) };
            let _ = run
                .window
                .request_inner_size(LogicalSize::new(at.width, at.height + by));
        }
        self.show_bar_time();
    }

    /// The bar's time: the recording's length.
    fn show_bar_time(&mut self) {
        let Some(run) = &mut self.run else { return };
        if let (Some(b), Some(frames)) = (&mut run.bar, run.recorder.frames()) {
            b.set_time(&elapsed(frames));
        }
    }

    /// A host event: the app's, or the game's input.
    fn host(&mut self, event_loop: &ActiveEventLoop, e: HostInput) {
        let Some(run) = &mut self.run else { return };
        if let Some(c) = run.input.event(e) {
            self.on_command(event_loop, c);
        }
    }

    /// One PAL frame: the script's events for it, the input, the step, the picture.
    fn step(&mut self, event_loop: &ActiveEventLoop) {
        let Some(run) = &self.run else { return };
        // Step n's events land before it.
        let n = run.steps + 1;
        let events: Vec<Event> = self
            .script
            .as_ref()
            .map_or(Vec::new(), |s| script::at(s, n).collect());
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
                Event::Command(c) => self.on_command(event_loop, c),
                Event::Controls(a) => self.controls_action(a),
                Event::Shot => self.run.as_mut().expect("running").shot = Some(n),
                Event::Resize(w, h) => {
                    let _ = self
                        .run
                        .as_ref()
                        .expect("running")
                        .window
                        .request_inner_size(LogicalSize::new(w, h));
                }
                Event::Hang => {
                    eprintln!("fd-app: the script hangs at frame {n}");
                    loop {
                        std::thread::sleep(Duration::from_secs(3600));
                    }
                }
            }
            if event_loop.exiting() {
                return;
            }
        }
        let run = self.run.as_mut().expect("running");
        run.steps = n;
        let input = run.input.frame();
        run.game.step(input);
        run.recorder.step(input);
        for e in run.game.take_events() {
            match e {
                GameEvent::Unported(v) => {
                    eprintln!("fd-app: object value {v} is not ported; left out")
                }
                other => eprintln!("fd-app: {other:?}"),
            }
        }
        let (size, rgba) = frame_of(&run.game);
        if let Some(c) = &mut run.capture {
            c.clock(n, run.clock.samples());
            c.input(n, input);
            let written = c.frame(n, (size.0, size.1, &rgba)).and_then(|()| {
                let h = run.hashes.as_mut().expect("hashes with the capture");
                h.write_all(fd_app::hash_line(n, &run.game).as_bytes())
            });
            if let Err(e) = written {
                return self.fail(event_loop, &format!("capture: {e}"));
            }
        }
        run.shown.completed((n, size, rgba));
        if run.recorder.full() {
            eprintln!("fd-app: the recording is a day long, the longest one may be: stopped");
            self.save_recording();
            self.show_menu();
        }
        if self.args.max_frames == Some(n) {
            self.finish(event_loop);
        }
    }

    fn finish(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(r) = self.run.as_mut() {
            eprintln!("fd-app: quit after step {}", r.steps);
            if let Some(c) = r.capture.as_mut() {
                let done = r
                    .hashes
                    .as_mut()
                    .map_or(Ok(()), |h| h.flush())
                    .and_then(|()| c.finish())
                    .and_then(|()| c.pacing(r.steps, r.presented, r.dropped, r.clock.samples()))
                    .and_then(|()| c.memory());
                if let Err(e) = done {
                    return self.fail(event_loop, &format!("capture: {e}"));
                }
            }
        }
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

    /// A `shot` the script asked for: what the window shows for the newest frame.
    fn take_shot(&mut self, event_loop: &ActiveEventLoop) {
        let Some(run) = &mut self.run else { return };
        let (Some(n), Some(c)) = (run.shot.take(), &mut run.capture) else {
            return;
        };
        let written = run.video.shot(&c.window_path(n)).and_then(|shown| {
            c.window_shown(n, run.uploaded, &shown)
                .map_err(|e| e.to_string())
        });
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
        while RecordingBar::next_click() {
            self.on_command(event_loop, Command::StopRecording);
            if event_loop.exiting() {
                return;
            }
        }
        while let Some(e) = self
            .run
            .as_mut()
            .and_then(|r| r.pads.as_mut())
            .and_then(|p| p.next())
        {
            self.pad_event(e);
        }
        let Some(run) = &mut self.run else { return };
        // `fast-until N`: steps before N as fast as they run, in batches; reaching N, the pacer
        // counts the time spent as stepped, so the game carries on from now.
        if let Some(until) = self.script.as_ref().and_then(|s| s.fast_until)
            && run.steps < until
        {
            for _ in 0..(until - run.steps).min(FAST_BATCH) {
                self.step(event_loop);
                if event_loop.exiting() {
                    return;
                }
            }
            self.show_newest();
            let run = self.run.as_mut().expect("running");
            if run.steps >= until {
                run.pacer.skip_to(run.clock.samples());
            }
            event_loop.set_control_flow(ControlFlow::Poll);
            return;
        }
        let due = run.pacer.due(run.clock.samples());
        for _ in 0..due {
            self.step(event_loop);
            if event_loop.exiting() {
                return;
            }
            // A shot is of its own step's frame, even when later steps in this catch-up end the run.
            if self.run.as_ref().is_some_and(|r| r.shot.is_some()) {
                self.show_newest();
                self.take_shot(event_loop);
            }
        }
        if due > 0 {
            self.show_newest();
            self.show_bar_time();
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
            WindowEvent::CloseRequested if self.run.is_some() => {
                self.on_command(event_loop, Command::Quit)
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(run) = &mut self.run {
                    run.video.resized(size.width, size.height);
                    if let Some(b) = &mut run.bar {
                        b.layout();
                    }
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
                    self.host(
                        event_loop,
                        HostInput::Key {
                            code,
                            pressed: event.state == ElementState::Pressed,
                        },
                    );
                }
            }
            WindowEvent::Focused(false) => self.host(event_loop, HostInput::FocusLost),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_opens_at_the_scale_and_auto_is_twice() {
        assert_eq!(
            initial_size(Scale::N(1), (320, 256)),
            LogicalSize::new(320, 256)
        );
        assert_eq!(
            initial_size(Scale::N(3), (320, 256)),
            LogicalSize::new(960, 768)
        );
        assert_eq!(
            initial_size(Scale::Auto, (320, 256)),
            LogicalSize::new(640, 512)
        );
    }

    #[test]
    fn the_bar_adds_its_height_under_the_picture_and_its_views_fit_the_smallest_window() {
        assert_eq!(min_size((320, 256), false), LogicalSize::new(320, 256));
        assert_eq!(
            min_size((320, 256), true),
            LogicalSize::new(320, 256 + BAR_HEIGHT)
        );
        // The bar's views as AppKit measures them at 13 points (Superfrog's measure).
        let widths = [12.0, 70.0, 40.0, 18.0, 72.0];
        // The play picture is 320 wide (T04); the app's own end-to-end tests see its size.
        let pic = 320;
        assert!(
            amiga_app::recording_bar::place(pic as f64, widths).is_some(),
            "all five fit {pic} points"
        );
    }
}
