//! The native macOS menu bar (DESIGN §2.7, §7): every setting and feature lives here, nothing new is
//! drawn in the game (owner).
//!
//! ```text
//! Superfrog   About Superfrog · Services · Hide, Hide Others, Show All · Quit Superfrog ⌘Q
//! File        Import Disks… · Export Level ▸ W1-1… … WW-1… · Play Level File…
//! Game        Continue from World N · Pause · Reset
//! View        Scale ▸ 1× 2× 3× Auto · Fullscreen ⌃⌘F
//! ```
//!
//! A [`Command`] is the only path from the menu to the app's state: a menu click, its shortcut, a
//! script's `menu …` event and a key combination `input.rs` reserves for the app all become one,
//! and [`App::on_command`] acts on it. T25–T28 add their items as further variants.

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{
    AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use sf_core::{Game, LevelId, Progress, Rng, Start};
use sf_paula::Paula;
use winit::event_loop::ActiveEventLoop;

use crate::App;
use crate::settings::Scale;

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
    /// View → Scale.
    Scale(Scale),
    /// View → Fullscreen, ⌃⌘F.
    ToggleFullscreen,
    /// Game → Pause: the game stops stepping until it is chosen again.
    Pause,
    /// Game → Reset: power-on.
    Reset,
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
            Command::Scale(Scale::Auto) => "scale-auto".into(),
            Command::Scale(Scale::N(n)) => format!("scale-{n}"),
            Command::ToggleFullscreen => "fullscreen".into(),
            Command::Pause => "pause".into(),
            Command::Reset => "reset".into(),
            Command::Quit => "quit".into(),
        }
    }

    /// The command whose [`Command::id`] is `id`; the scale any whole number 1 to 16.
    pub fn from_id(id: &str) -> Option<Command> {
        Some(match id {
            "import" => Command::ImportDisks,
            "continue" => Command::ContinueFromProgress,
            "scale-auto" => Command::Scale(Scale::Auto),
            "fullscreen" => Command::ToggleFullscreen,
            "pause" => Command::Pause,
            "reset" => Command::Reset,
            "quit" => Command::Quit,
            "play-level-file" => Command::PlayLevelFile,
            _ if id.starts_with("export-") => Command::ExportLevel(sf_assets::level_id_named(&id[7..])?),
            _ => {
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
}

/// The menu bar, and the items whose title, check or enabled state follows the app's.
pub struct MenuBar {
    menu: Menu,
    continue_item: MenuItem,
    pause: CheckMenuItem,
    scales: Vec<(Scale, CheckMenuItem)>,
    fullscreen: CheckMenuItem,
    shown: Option<MenuState>,
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
        &[&continue_item, &PredefinedMenuItem::separator(), &pause, &item(Command::Reset, "Reset", None)],
    );
    let scale_items: Vec<&dyn muda::IsMenuItem> = scales.iter().map(|(_, i)| i as &dyn muda::IsMenuItem).collect();
    let scale = Submenu::with_items("Scale", true, &scale_items).expect("a menu");
    let view = Submenu::with_items("View", true, &[&scale, &PredefinedMenuItem::separator(), &fullscreen]);

    let menu = Menu::new();
    for m in [app, file, game, view] {
        menu.append(&m.expect("a menu")).expect("a menu in the menu bar");
    }
    MenuBar {
        menu,
        continue_item,
        pause,
        scales,
        fullscreen,
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
        self.pause.set_checked(s.paused);
        for (scale, i) in &self.scales {
            i.set_checked(*scale == s.scale);
        }
        self.fullscreen.set_checked(s.fullscreen);
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
        match c {
            Command::ImportDisks => self.import_disks(),
            Command::ExportLevel(id) => self.export_level(id),
            Command::PlayLevelFile => self.play_level_file(),
            Command::ContinueFromProgress => self.continue_game(),
            Command::Scale(s) => self.set_scale(s),
            Command::ToggleFullscreen => self.toggle_fullscreen(),
            Command::Pause => {
                let run = self.run.as_mut().expect("running");
                run.paused = !run.paused;
            }
            Command::Reset => self.power_on(),
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
        });
    }

    /// Power-on, as a fresh launch makes it: the same seed, a new sound chip, not paused.
    fn power_on(&mut self) {
        let run = self.run.as_mut().expect("running");
        let seed = Rng::seeded(run.seed.0, run.seed.1);
        run.game = Game::with_display(run.display.clone(), Start::PowerOn { seed });
        run.paula = Paula::new(run.rate);
        run.paused = false;
    }

    /// Start the remembered world as its level code would (DESIGN §2.9); refused, as the menu item
    /// is disabled, until World 2 is reached.
    fn continue_game(&mut self) {
        let run = self.run.as_mut().expect("running");
        let Some(p) = run.furthest.filter(|&p| continue_world(Some(p)).is_some()) else {
            eprintln!("superfrog: no progress to continue from: Continue opens at World 2");
            return;
        };
        let seed = Rng::seeded(run.seed.0, run.seed.1);
        run.game = Game::start_at_with(run.display.clone(), &p, seed);
        run.paula = Paula::new(run.rate);
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
            Command::Scale(Scale::N(1)),
            Command::Scale(Scale::N(3)),
            Command::Scale(Scale::Auto),
            Command::ToggleFullscreen,
            Command::Pause,
            Command::Reset,
            Command::Quit,
        ];
        all.extend(export_levels().into_iter().map(Command::ExportLevel));
        for c in all {
            assert_eq!(Command::from_id(&c.id()), Some(c), "{}", c.id());
        }
        for bad in ["", "scale-0", "scale-17", "scale-x", "about", "Quit", "export-", "export-W7-1"] {
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
