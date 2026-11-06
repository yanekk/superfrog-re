//! Shell: the `fd-app` binary, the app the owner opens (DESIGN §2.3, §2.7, §2.9, §2.12).
//!
//! At launch, inside the event loop: the single-copy lock, the remembered settings, the game's data
//! from the two zips (`disks.rs`; a plain message naming where it looked, and exit 1, when they
//! cannot be read), then straight into `W1-1` at the level sync point, silent. The game is stepped
//! once a PAL frame of 313 lines on the wall clock (`Pacer::new(rate, 313)`, docs/frame-length.md)
//! and the newest frame is presented at each refresh at a whole-number scale. Keys and pads feed
//! the game through `input.rs`; Controls and Quit are in the menu bar (`menu.rs`), Esc also quits.
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
use amiga_app::video::Video;
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
    let mut app = App {
        args,
        home,
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
}

/// The window's inner size at the scale: `N(n)` is n times the picture, `Auto` twice.
fn initial_size(scale: Scale, picture: (u32, u32)) -> LogicalSize<u32> {
    let n = match scale {
        Scale::Auto => 2,
        Scale::N(n) => n,
    };
    LogicalSize::new(picture.0 * n, picture.1 * n)
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
            .with_min_inner_size(LogicalSize::new(size.0, size.1))
            .with_active(!self.args.unattended());
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(event_loop, &format!("no window: {e}")),
        };
        let video = match Video::new(window.clone(), scale) {
            Ok(v) => v,
            Err(e) => return self.fail(event_loop, &e),
        };
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
                    let mut h = std::io::BufWriter::new(std::fs::File::create(dir.join(fd_app::HASHES))?);
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
        match c {
            Command::Controls => self.open_controls(event_loop),
            // Settings are saved when they change, never on the way out: a damaged file stays as
            // it is until the owner changes a setting (DESIGN §2.12).
            Command::Quit => self.finish(event_loop),
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
        let events: Vec<Event> = self.script.as_ref().map_or(Vec::new(), |s| script::at(s, n).collect());
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
                    let _ = self.run.as_ref().expect("running").window.request_inner_size(LogicalSize::new(w, h));
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
        for e in run.game.take_events() {
            match e {
                GameEvent::Unported(v) => eprintln!("fd-app: object value {v} is not ported; left out"),
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
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(2)));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.controls.as_ref().is_some_and(|c| c.window.id() == id) {
            match event {
                WindowEvent::CloseRequested => self.close_controls(),
                WindowEvent::KeyboardInput { event, .. } if !event.repeat && event.state == ElementState::Pressed => {
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
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(run) = &mut self.run {
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
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_opens_at_the_scale_and_auto_is_twice() {
        assert_eq!(initial_size(Scale::N(1), (320, 256)), LogicalSize::new(320, 256));
        assert_eq!(initial_size(Scale::N(3), (320, 256)), LogicalSize::new(960, 768));
        assert_eq!(initial_size(Scale::Auto, (320, 256)), LogicalSize::new(640, 512));
    }
}
