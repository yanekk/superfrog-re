//! The native macOS menu bar (DESIGN §2.7, §7): every setting and feature lives here, nothing new is
//! drawn in the game (owner).
//!
//! ```text
//! Superfrog   About Superfrog · Services · Hide, Hide Others, Show All · Quit Superfrog ⌘Q
//! File        Import Disks… · Export Level ▸ W1-1… … WW-1… · Play Level File…
//! Game        Continue from World N · Start Level… · Stop Recording ⌘. · Pause · Reset · Controls… ·
//!             Save State ▸ 1–5 ⇧⌘1–5 · Load State ▸ 1–5 ⌘1–5
//! View        Scale ▸ 1× 2× 3× Auto · Fullscreen ⌃⌘F · CRT Filter · Widescreen
//! ```
//!
//! A [`Command`] is the only path from the menu to the app's state: a menu click, its shortcut, a
//! script's `menu …` event and a key combination `input.rs` reserves for the app all become one,
//! and [`App::on_command`] acts on it. T25–T28 add their items as further variants.
//!
//! While a bug recording runs (`recorder.rs`), the commands that would change the game end it, and
//! first ask "Recording in progress" (bug-recorder DESIGN §2.2): Load State, Start Level, Reset,
//! Continue, Import Disks, Play Level File and Quit. The rest leave it running.

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{
    AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use sf_core::{Game, LevelId, Progress, Rng, Start};
use sf_core::recording::Location;
use sf_paula::Paula;
use winit::event_loop::ActiveEventLoop;

use crate::App;
use crate::recorder::{EndChoice, Recorder};
use crate::settings::Scale;
use crate::states::SLOTS;

/// What the player asked the app to do, from the menu bar or its shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// File → Import Disks…: the picker; the game starts again on the disks chosen.
    ImportDisks,
    /// File → Export Level ▸ this level: the save panel; the disk level written there as a
    /// `.sflevel` (DESIGN §2.10, `levelfile.rs`).
    ExportLevel(LevelId),
    /// File → Play Level File…: the open panel; the level file played in place of its disk level.
    PlayLevelFile,
    /// Game → Continue from World N: the furthest world reached, as its level code starts it.
    ContinueFromProgress,
    /// Game → Start Level…: the dialog; the level started with its values, recording or not
    /// (bug-recorder DESIGN §2.1).
    StartLevel,
    /// Game → Stop Recording, ⌘.: the save window for the recording running (§2.3, §2.4).
    StopRecording,
    /// View → Scale.
    Scale(Scale),
    /// View → Fullscreen, ⌃⌘F.
    ToggleFullscreen,
    /// View → CRT Filter.
    ToggleCrt,
    /// View → Widescreen.
    ToggleWidescreen,
    /// Game → Pause: the game stops stepping until it is chosen again.
    Pause,
    /// Game → Reset: power-on.
    Reset,
    /// Game → Controls…: the window that rebinds keys and pad (`controls_window.rs`).
    Controls,
    /// Game → Save State ▸ N, ⇧⌘N: the whole game into slot N (`states.rs`).
    SaveState(u8),
    /// Game → Load State ▸ N, ⌘N: the game in slot N, or a message why not.
    LoadState(u8),
    /// Superfrog → Quit Superfrog, ⌘Q, or the window closed.
    Quit,
}

/// The levels File → Export Level offers, in the game's order: worlds 1–6, W6's second set of
/// maps, the bonus level, the final level (docs/LEVEL-INDEX.md).
pub fn export_levels() -> Vec<LevelId> {
    let mut ids: Vec<String> = (1..=6).flat_map(|w| (1..=4).map(move |l| format!("W{w}-{l}"))).collect();
    ids.extend((1..=4).map(|l| format!("W6-{l}alt")));
    ids.extend(["WB-1".to_string(), "WW-1".to_string()]);
    ids.iter().map(|s| sf_assets::level_id_named(s).expect("a level id")).collect()
}

/// The scales the View menu offers.
pub const SCALES: [Scale; 4] = [Scale::N(1), Scale::N(2), Scale::N(3), Scale::Auto];

impl Command {
    /// Its menu item's id, also its name in a script's `menu …` event (`scale 2` is `scale-2`).
    pub fn id(self) -> String {
        match self {
            Command::ImportDisks => "import".into(),
            Command::ExportLevel(id) => format!("export-{id}"),
            Command::PlayLevelFile => "play-level-file".into(),
            Command::ContinueFromProgress => "continue".into(),
            Command::StartLevel => "start-level".into(),
            Command::StopRecording => "stop-recording".into(),
            Command::Scale(Scale::Auto) => "scale-auto".into(),
            Command::Scale(Scale::N(n)) => format!("scale-{n}"),
            Command::ToggleFullscreen => "fullscreen".into(),
            Command::ToggleCrt => "crt".into(),
            Command::ToggleWidescreen => "widescreen".into(),
            Command::Pause => "pause".into(),
            Command::Reset => "reset".into(),
            Command::Controls => "controls".into(),
            Command::SaveState(n) => format!("save-state-{n}"),
            Command::LoadState(n) => format!("load-state-{n}"),
            Command::Quit => "quit".into(),
        }
    }

    /// The command whose [`Command::id`] is `id`; the scale any whole number 1 to 16.
    pub fn from_id(id: &str) -> Option<Command> {
        Some(match id {
            "import" => Command::ImportDisks,
            "continue" => Command::ContinueFromProgress,
            "start-level" => Command::StartLevel,
            "stop-recording" => Command::StopRecording,
            "scale-auto" => Command::Scale(Scale::Auto),
            "fullscreen" => Command::ToggleFullscreen,
            "crt" => Command::ToggleCrt,
            "widescreen" => Command::ToggleWidescreen,
            "pause" => Command::Pause,
            "reset" => Command::Reset,
            "controls" => Command::Controls,
            "quit" => Command::Quit,
            "play-level-file" => Command::PlayLevelFile,
            _ if id.starts_with("export-") => Command::ExportLevel(sf_assets::level_id_named(&id[7..])?),
            _ => {
                let slot = |prefix: &str| -> Option<u8> {
                    let n: u8 = id.strip_prefix(prefix)?.parse().ok()?;
                    (1..=SLOTS).contains(&n).then_some(n)
                };
                if let Some(n) = slot("save-state-") {
                    return Some(Command::SaveState(n));
                }
                if let Some(n) = slot("load-state-") {
                    return Some(Command::LoadState(n));
                }
                let n: u32 = id.strip_prefix("scale-")?.parse().ok()?;
                (1..=16).contains(&n).then_some(Command::Scale(Scale::N(n)))?
            }
        })
    }
}

/// The world Game → Continue offers: the furthest reached, once it is World 2 or later (DESIGN
/// §2.9). The bonus level (world 7) comes after World 5.
pub fn continue_world(furthest: Option<Progress>) -> Option<u8> {
    furthest.map(|p| p.world).filter(|&w| w >= 2)
}

/// Game → Continue's title.
pub fn continue_title(furthest: Option<Progress>) -> String {
    match continue_world(furthest) {
        None => "Continue".into(),
        Some(7) => "Continue from the Bonus Level".into(),
        Some(w) => format!("Continue from World {w}"),
    }
}

/// The menu bar's titles, checks and enabled items, from the app's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuState {
    pub furthest: Option<Progress>,
    pub paused: bool,
    pub scale: Scale,
    pub fullscreen: bool,
    pub crt: bool,
    pub widescreen: bool,
    /// Which save-state slots hold a state: Load State ▸ N can be chosen only then.
    pub states: [bool; SLOTS as usize],
    /// A bug recording is running: Stop Recording can be chosen only then.
    pub recording: bool,
}

/// The number key for slot `n` (1–5), its shortcut's key.
pub fn slot_key(n: u8) -> Code {
    [Code::Digit1, Code::Digit2, Code::Digit3, Code::Digit4, Code::Digit5][n as usize - 1]
}

/// The menu bar, and the items whose title, check or enabled state follows the app's.
pub struct MenuBar {
    menu: Menu,
    continue_item: MenuItem,
    stop_recording: MenuItem,
    pause: CheckMenuItem,
    scales: Vec<(Scale, CheckMenuItem)>,
    fullscreen: CheckMenuItem,
    crt: CheckMenuItem,
    widescreen: CheckMenuItem,
    loads: Vec<MenuItem>,
    shown: Option<MenuState>,
}

fn as_items(v: &[MenuItem]) -> Vec<&dyn muda::IsMenuItem> {
    v.iter().map(|i| i as &dyn muda::IsMenuItem).collect()
}

fn item(c: Command, title: &str, accelerator: Option<Accelerator>) -> MenuItem {
    MenuItem::with_id(c.id(), title, true, accelerator)
}

fn check(c: Command, title: &str, accelerator: Option<Accelerator>) -> CheckMenuItem {
    CheckMenuItem::with_id(c.id(), title, true, false, accelerator)
}

/// The menu bar, before it is shown: [`MenuBar::install`] puts it in place. On the main thread.
pub fn build_menu() -> MenuBar {
    let about = AboutMetadata {
        name: Some("Superfrog".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        ..Default::default()
    };
    let quit = item(
        Command::Quit,
        "Quit Superfrog",
        Some(Accelerator::new(Modifiers::META, Code::KeyQ)),
    );
    let continue_item = MenuItem::with_id(Command::ContinueFromProgress.id(), continue_title(None), false, None);
    let stop_recording = MenuItem::with_id(
        Command::StopRecording.id(),
        "Stop Recording",
        false,
        Some(Accelerator::new(Modifiers::META, Code::Period)),
    );
    let pause = check(Command::Pause, "Pause", None);
    let scales: Vec<(Scale, CheckMenuItem)> = SCALES
        .iter()
        .map(|&s| {
            let title = match s {
                Scale::Auto => "Auto".to_string(),
                Scale::N(n) => format!("{n}×"),
            };
            (s, check(Command::Scale(s), &title, None))
        })
        .collect();
    let fullscreen = check(
        Command::ToggleFullscreen,
        "Fullscreen",
        Some(Accelerator::new(Modifiers::CONTROL | Modifiers::META, Code::KeyF)),
    );
    let crt = check(Command::ToggleCrt, "CRT Filter", None);
    let widescreen = check(Command::ToggleWidescreen, "Widescreen", None);

    let app = Submenu::with_items(
        "Superfrog",
        true,
        &[
            &PredefinedMenuItem::about(Some("About Superfrog"), Some(about)),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(Some("Hide Superfrog")),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &quit,
        ],
    );
    let saves: Vec<MenuItem> = (1..=SLOTS)
        .map(|n| {
            let key = Accelerator::new(Modifiers::META | Modifiers::SHIFT, slot_key(n));
            item(Command::SaveState(n), &n.to_string(), Some(key))
        })
        .collect();
    let loads: Vec<MenuItem> = (1..=SLOTS)
        .map(|n| {
            let key = Accelerator::new(Modifiers::META, slot_key(n));
            MenuItem::with_id(Command::LoadState(n).id(), n.to_string(), false, Some(key))
        })
        .collect();
    let save_menu = Submenu::with_items("Save State", true, &as_items(&saves)).expect("a menu");
    let load_menu = Submenu::with_items("Load State", true, &as_items(&loads)).expect("a menu");
    let exports: Vec<MenuItem> = export_levels()
        .into_iter()
        .map(|id| item(Command::ExportLevel(id), &format!("{id}…"), None))
        .collect();
    let export_items: Vec<&dyn muda::IsMenuItem> = exports.iter().map(|i| i as &dyn muda::IsMenuItem).collect();
    let export = Submenu::with_items("Export Level", true, &export_items).expect("a menu");
    let file = Submenu::with_items(
        "File",
        true,
        &[
            &item(Command::ImportDisks, "Import Disks…", None),
            &PredefinedMenuItem::separator(),
            &export,
            &item(Command::PlayLevelFile, "Play Level File…", None),
        ],
    );
    let game = Submenu::with_items(
        "Game",
        true,
        &[
            &continue_item,
            &item(Command::StartLevel, "Start Level…", None),
            &stop_recording,
            &PredefinedMenuItem::separator(),
            &pause,
            &item(Command::Reset, "Reset", None),
            &PredefinedMenuItem::separator(),
            &item(Command::Controls, "Controls…", None),
            &PredefinedMenuItem::separator(),
            &save_menu,
            &load_menu,
        ],
    );
    let scale_items: Vec<&dyn muda::IsMenuItem> = scales.iter().map(|(_, i)| i as &dyn muda::IsMenuItem).collect();
    let scale = Submenu::with_items("Scale", true, &scale_items).expect("a menu");
    let view = Submenu::with_items(
        "View",
        true,
        &[
            &scale,
            &PredefinedMenuItem::separator(),
            &fullscreen,
            &PredefinedMenuItem::separator(),
            &crt,
            &widescreen,
        ],
    );

    let menu = Menu::new();
    for m in [app, file, game, view] {
        menu.append(&m.expect("a menu")).expect("a menu in the menu bar");
    }
    MenuBar {
        menu,
        continue_item,
        stop_recording,
        pause,
        scales,
        fullscreen,
        crt,
        widescreen,
        loads,
        shown: None,
    }
}

impl MenuBar {
    /// Make this the app's menu bar. AppKit's window tabs, and their items in the View menu, are
    /// turned off.
    pub fn install(&self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::ActiveEventLoopExtMacOS;
            event_loop.set_allows_automatic_window_tabbing(false);
            self.menu.init_for_nsapp();
        }
    }

    /// Show `s`: the checks, Continue's title and whether it can be chosen. Only what changed is
    /// set.
    pub fn show(&mut self, s: MenuState) {
        if self.shown == Some(s) {
            return;
        }
        self.continue_item.set_text(continue_title(s.furthest));
        self.continue_item.set_enabled(continue_world(s.furthest).is_some());
        self.stop_recording.set_enabled(s.recording);
        self.pause.set_checked(s.paused);
        for (scale, i) in &self.scales {
            i.set_checked(*scale == s.scale);
        }
        self.fullscreen.set_checked(s.fullscreen);
        self.crt.set_checked(s.crt);
        self.widescreen.set_checked(s.widescreen);
        for (i, filled) in self.loads.iter().zip(s.states) {
            i.set_enabled(filled);
        }
        self.shown = Some(s);
    }
}

/// Before the app finishes launching: AppKit adds "Enter Full Screen" to a menu titled View unless
/// told not to, and View → Fullscreen is the app's own (DESIGN §2.7: the largest integer scale).
pub fn before_launch() {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::{NSDictionary, NSNumber, NSString, NSUserDefaults};
        // Registered, not set: nothing is written to the user's preferences.
        let key = NSString::from_str("NSFullScreenMenuItemEverywhere");
        let no = NSNumber::new_bool(false);
        let no: &objc2::runtime::AnyObject = &no;
        let defaults = NSDictionary::from_slices(&[&*key], &[no]);
        // SAFETY: a property-list dictionary (a string key, a number), as the method requires.
        unsafe { NSUserDefaults::standardUserDefaults().registerDefaults(&defaults) };
    }
}

/// The next command chosen in the menu bar since the last call, if any.
pub fn next_command() -> Option<Command> {
    while let Ok(e) = MenuEvent::receiver().try_recv() {
        // A predefined item (About, Hide…) is AppKit's own and sends nothing the app acts on.
        if let Some(c) = Command::from_id(e.id.as_ref()) {
            return Some(c);
        }
    }
    None
}

impl App {
    /// Act on `c`: the one place a menu item, its shortcut or a script's `menu …` changes the app.
    pub(crate) fn on_command(&mut self, event_loop: &ActiveEventLoop, c: Command) {
        let Some(run) = &self.run else { return };
        // The step it came after, so a test can line the frames up with it.
        eprintln!("superfrog: menu {} after step {}", c.id(), run.steps);
        if let Some(action) = self.ends_recording(c)
            && !self.end_recording(&action)
        {
            return self.show_menu();
        }
        match c {
            Command::ImportDisks => self.import_disks(),
            Command::ExportLevel(id) => self.export_level(id),
            Command::PlayLevelFile => self.play_level_file(),
            Command::ContinueFromProgress => self.continue_game(),
            Command::StartLevel => self.start_level(),
            Command::StopRecording => self.stop_recording(),
            Command::Scale(s) => self.set_scale(s),
            Command::ToggleFullscreen => self.toggle_fullscreen(),
            Command::ToggleCrt => self.toggle_crt(),
            Command::ToggleWidescreen => self.toggle_widescreen(),
            Command::Pause => {
                let run = self.run.as_mut().expect("running");
                run.paused = !run.paused;
            }
            Command::Reset => self.power_on(),
            Command::Controls => self.open_controls(event_loop),
            Command::SaveState(n) => self.save_state(n),
            Command::LoadState(n) => self.load_state(n),
            Command::Quit => {
                self.save_settings();
                self.finish(event_loop);
            }
        }
        self.show_menu();
    }

    /// The menu bar's state, from the app's.
    pub(crate) fn show_menu(&mut self) {
        let (Some(menu), Some(run)) = (&mut self.menu, &self.run) else { return };
        menu.show(MenuState {
            furthest: run.furthest,
            paused: run.paused,
            scale: run.video.scale,
            fullscreen: run.settings.fullscreen,
            crt: run.settings.crt,
            widescreen: run.settings.widescreen,
            states: run.states,
            recording: run.recorder.is_recording(),
        });
    }

    /// What "Recording in progress" calls `c`, when taking it would end the recording running:
    /// a command that replaces the game or ends the app. A disabled one (Continue before World 2,
    /// an empty slot's Load State) changes nothing and asks nothing.
    fn ends_recording(&self, c: Command) -> Option<String> {
        let run = self.run.as_ref()?;
        if !run.recorder.is_recording() {
            return None;
        }
        Some(match c {
            Command::LoadState(n) if run.states[n as usize - 1] => format!("Load State {n}"),
            Command::StartLevel => "Start Level".into(),
            Command::Reset => "Reset".into(),
            Command::ContinueFromProgress if continue_world(run.furthest).is_some() => {
                continue_title(run.furthest)
            }
            Command::ImportDisks => "Import Disks".into(),
            Command::PlayLevelFile => "Play Level File".into(),
            Command::Quit => "Quit".into(),
            _ => return None,
        })
    }

    /// Ask "Recording in progress" before `action`: `false` for Cancel (still recording, the
    /// action not taken); else the recording saved or dropped as answered, and the action goes on.
    fn end_recording(&mut self, action: &str) -> bool {
        let choice = self.ui.recording_in_progress(action);
        self.after_panel();
        match choice {
            EndChoice::Cancel => {
                eprintln!("superfrog: {action} cancelled; still recording");
                false
            }
            EndChoice::Discard => {
                self.run.as_mut().expect("running").recorder = Recorder::Off;
                eprintln!("superfrog: recording discarded");
                true
            }
            EndChoice::Save => {
                self.save_recording();
                true
            }
        }
    }

    /// Game → Stop Recording, ⌘.: the save window; refused when no recording runs (the item is
    /// disabled then).
    pub(crate) fn stop_recording(&mut self) {
        if !self.run.as_ref().is_some_and(|r| r.recorder.is_recording()) {
            eprintln!("superfrog: not recording");
            return;
        }
        self.save_recording();
    }

    /// The save window for the recording, which ends here (DESIGN §2.4): its screenshot the 4:3
    /// picture of the last frame recorded at 1×, where play is, the zip's name. Saved, the path is
    /// on the clipboard and in a notice; a write that fails says so and asks again, so the note
    /// is not lost and the recording is kept until it is saved or discarded (§2.11).
    pub(crate) fn save_recording(&mut self) {
        let run = self.run.as_mut().expect("running");
        let at: Location = run.game.location();
        let Some(mut rec) = std::mem::take(&mut run.recorder).finish(at, String::new()) else {
            return;
        };
        // The 4:3 picture whatever the window shows: a headless replay draws that view, and the
        // screenshot is what it compares with (DESIGN §2.5). In widescreen the step's picture is
        // no use narrowed (a level's status rows are laid out for the width), so the 4:3 view is
        // drawn again; that misses only a picture a pass ended mid-frame on.
        let pic = match run.game.view() == sf_core::View::default() {
            true => run.picture.clone(),
            false => run.game.render(sf_core::View::default()),
        };
        let shot = crate::capture::png(pic.width as u32, pic.height as u32, &crate::video::picture_rgba(&pic));
        let path = crate::recorder::zip_name(
            &self.bugs_dir,
            crate::recorder::named_level(&at, &rec),
            crate::recorder::local_now(),
        );
        let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        loop {
            let Some(note) = self.ui.save_recording(&shot, &at, &name) else {
                eprintln!("superfrog: recording discarded");
                break;
            };
            rec.note = note.replace(['\n', '\r'], " ");
            match crate::recorder::write_zip(&path, &rec, &shot) {
                Ok(()) => {
                    let shown = path.display().to_string();
                    eprintln!("superfrog: recording saved to {shown} ({} frames)", rec.inputs.len());
                    self.ui.copy(&shown);
                    self.ui.notice(&format!("The bug recording is saved as {shown}. Its path is on the clipboard."));
                    break;
                }
                Err(e) => {
                    let m = format!("The bug recording could not be saved as “{}”: {e}.", path.display());
                    eprintln!("superfrog: {m}");
                    self.ui.notice(&m);
                }
            }
        }
        self.after_panel();
    }

    /// Game → Start Level…: the dialog, then the level started fresh with its values (DESIGN
    /// §2.1), recording from its first frame when the recorder was on. The game it replaces never
    /// moves the saved progress from here on. Cancelled, the game carries on.
    fn start_level(&mut self) {
        let answer = self.ui.start_level();
        self.after_panel();
        let Some((setup, record)) = answer else { return };
        if let Err(why) = setup.validate() {
            let m = format!("That level cannot be started: {why}.");
            eprintln!("superfrog: {m}");
            return self.ui.notice(&m);
        }
        let run = self.run.as_mut().expect("running");
        let seed = Rng::seeded(run.seed.0, run.seed.1);
        run.game = Game::start_level(run.display.clone(), &setup, seed);
        run.game.set_view(crate::window::view(run.settings.widescreen));
        run.paula = Paula::booted(run.rate);
        run.paused = false;
        run.from_dialog = true;
        run.recorder = match record {
            true => Recorder::start(setup, seed, crate::home::build()),
            false => Recorder::Off,
        };
        // Frame 0 at once: the picture a STOP before any step would save.
        let pic = run.game.render(run.game.view());
        let size = (pic.width as u32, pic.height as u32);
        run.shown.completed((run.steps, size, crate::video::picture_rgba(&pic)));
        run.picture = pic;
        let recording = if record { ", recording" } else { "" };
        eprintln!("superfrog: started {} after step {}{recording}", setup.level, run.steps);
        // What the game holds at the level's first frame, so a test that started it through the
        // real window can see the values arrived (bug-recorder T08).
        let held: Vec<String> = ["frog_lives", "score", "frog_hearts", "opt_game_mode", "frog_spud_on", "frog_flight"]
            .iter()
            .filter_map(|&name| sf_core::layout::LAYOUT.iter().find(|d| d.name == name))
            .map(|d| {
                let bytes = run.game.peek(d.addr, d.width.len());
                format!("{} {}", d.name, bytes.iter().fold(0u32, |v, &b| v << 8 | b as u32))
            })
            .collect();
        eprintln!("superfrog: start-level holds {}", held.join(", "));
        // Starting ran the game unseen for seconds: carry on from now rather than catching up.
        self.after_panel();
        self.show_newest();
    }

    /// Power-on, as a fresh launch makes it: the same seed, a new sound chip, not paused.
    fn power_on(&mut self) {
        eprintln!("superfrog: power-on");
        let run = self.run.as_mut().expect("running");
        run.from_dialog = false;
        let seed = Rng::seeded(run.seed.0, run.seed.1);
        run.game = Game::with_display(run.display.clone(), Start::PowerOn { seed });
        run.game.set_view(crate::window::view(run.settings.widescreen));
        run.paula = Paula::booted(run.rate);
        run.paused = false;
    }

    /// Game → Save State ▸ `n`: the game as it is after the last step, into slot `n`.
    fn save_state(&mut self, n: u8) {
        let run = self.run.as_mut().expect("running");
        match crate::states::save(&self.home, n, &run.game, run.from_dialog, crate::home::build(), &run.cache_hash) {
            Ok(()) => {
                run.states[n as usize - 1] = true;
                eprintln!("superfrog: saved state {n} at game frame {}", run.game.frame());
            }
            Err(e) => {
                eprintln!("superfrog: {e}");
                self.ui.notice(&e);
            }
        }
    }

    /// Game → Load State ▸ `n`: the game in slot `n`, its picture shown at once (paused, it
    /// stays); refused with a message, the game carries on untouched.
    fn load_state(&mut self, n: u8) {
        let run = self.run.as_mut().expect("running");
        match crate::states::load(&self.home, n, run.display.clone(), crate::home::build(), &run.cache_hash) {
            Ok(game) => {
                eprintln!("superfrog: loaded state {n} at game frame {}", game.frame());
                run.game = game;
                // The loaded game is the one saved, dialog-started or not (bug-recorder DESIGN §2.1).
                run.from_dialog = crate::states::from_dialog(&self.home, n);
                run.game.set_view(crate::window::view(run.settings.widescreen));
                run.paula = Paula::booted(run.rate);
                let pic = run.game.render(run.game.view());
                let size = (pic.width as u32, pic.height as u32);
                run.shown.completed((run.steps, size, crate::video::picture_rgba(&pic)));
            }
            Err(e) => {
                eprintln!("superfrog: {e}");
                self.ui.notice(&e);
                // The panel held the main thread: the game carries on from now rather than
                // catching up.
                let run = self.run.as_mut().expect("running");
                let now = run.clock.samples() + run.lead;
                run.pacer.skip_to(now);
            }
        }
        self.show_newest();
    }

    /// Start the remembered world as its level code would (DESIGN §2.9); refused, as the menu item
    /// is disabled, until World 2 is reached.
    fn continue_game(&mut self) {
        let run = self.run.as_mut().expect("running");
        let Some(p) = run.furthest.filter(|&p| continue_world(Some(p)).is_some()) else {
            eprintln!("superfrog: no progress to continue from: Continue opens at World 2");
            return;
        };
        eprintln!("superfrog: continuing from World {}", p.world);
        run.from_dialog = false;
        let seed = Rng::seeded(run.seed.0, run.seed.1);
        run.game = Game::start_at_with(run.display.clone(), &p, seed);
        run.game.set_view(crate::window::view(run.settings.widescreen));
        run.paula = Paula::booted(run.rate);
        run.paused = false;
    }

    /// File → Import Disks…: the picker; on four good disks the cache is replaced and the game
    /// starts again from power-on on them. Cancelled, or with disks that cannot be used (said in a
    /// message), the game carries on.
    fn import_disks(&mut self) {
        let data = match crate::import::choose(&self.home, &mut *self.ui) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("superfrog: {e}");
                self.ui.notice(&e);
                None
            }
        };
        self.after_panel();
        if let Some(data) = data {
            let run = self.run.as_mut().expect("running");
            run.display = std::sync::Arc::new(sf_core::DisplayData::new(&data));
            run.cache_hash = sf_assets::cache_hash(&data);
            run.levels = data.levels;
            self.power_on();
        }
    }

    /// After a panel: it held the main thread, so the game carries on from now rather than
    /// catching up.
    pub(crate) fn after_panel(&mut self) {
        let run = self.run.as_mut().expect("running");
        let now = run.clock.samples() + run.lead;
        run.pacer.skip_to(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_s_id_names_it() {
        let mut all = vec![
            Command::ImportDisks,
            Command::PlayLevelFile,
            Command::ContinueFromProgress,
            Command::StartLevel,
            Command::StopRecording,
            Command::Scale(Scale::N(1)),
            Command::Scale(Scale::N(3)),
            Command::Scale(Scale::Auto),
            Command::ToggleFullscreen,
            Command::ToggleCrt,
            Command::ToggleWidescreen,
            Command::Pause,
            Command::Reset,
            Command::Controls,
            Command::SaveState(1),
            Command::SaveState(5),
            Command::LoadState(1),
            Command::LoadState(5),
            Command::Quit,
        ];
        all.extend(export_levels().into_iter().map(Command::ExportLevel));
        for c in all {
            assert_eq!(Command::from_id(&c.id()), Some(c), "{}", c.id());
        }
        for bad in ["", "scale-0", "scale-17", "scale-x", "about", "Quit", "save-state-0", "save-state-6", "load-state-", "load-state-x", "export-", "export-W7-1"] {
            assert_eq!(Command::from_id(bad), None, "{bad}");
        }
    }

    #[test]
    fn export_offers_every_level_on_the_disks_once() {
        let ids = export_levels();
        assert_eq!(ids.len(), 30, "the 30 maps on the disks");
        let names: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
        assert_eq!(&names[..2], ["W1-1", "W1-2"]);
        assert_eq!(&names[24..], ["W6-1alt", "W6-2alt", "W6-3alt", "W6-4alt", "WB-1", "WW-1"]);
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 30);
    }

    #[test]
    fn continue_opens_at_world_2_and_names_the_furthest_world() {
        let p = |world| Some(Progress { world, level: 3 });
        assert_eq!(continue_world(None), None);
        assert_eq!(continue_world(p(1)), None);
        assert_eq!(continue_title(p(1)), "Continue");
        assert_eq!(continue_world(p(2)), Some(2));
        assert_eq!(continue_title(p(2)), "Continue from World 2");
        assert_eq!(continue_title(p(6)), "Continue from World 6");
        assert_eq!(continue_title(p(7)), "Continue from the Bonus Level");
    }
}
