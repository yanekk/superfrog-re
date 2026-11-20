//! The native macOS menu bar (DESIGN §2.7): only what the design lists.
//!
//! ```text
//! Forest Dumb   About · Services · Hide, Hide Others, Show All · Quit Forest Dumb ⌘Q
//! Game          Start Level… · Stop Recording ⌘. · Show Map · Controls…
//! View          Scale ▸ 1× 2× 3× Auto · Fullscreen ⌃⌘F · Filter ▸ None Basic Lottes Geom Easymode
//! ```
//!
//! Stop Recording is enabled only while a recording runs (T14). View's items are the toolkit's
//! (`amiga_app::view_menu`), checked as the app's scale, fullscreen and filter are; no Widescreen,
//! as the original this runs has no wider view (fd-view-menu). A [`Command`] is the one path from the menu to
//! the app: a click, its shortcut, a script's `menu …` and a key `input.rs` keeps for the app all
//! become one, and the app acts on it.

use muda::accelerator::{Accelerator, Code, Modifiers};
use amiga_app::video::{Filter, Scale};
use amiga_app::view_menu::{ViewCommand, ViewItems};
use muda::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use winit::event_loop::ActiveEventLoop;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Game → Start Level…: the window that starts W1-1 afresh, the recorder on or off (T14).
    StartLevel,
    /// Game → Stop Recording, ⌘., or the recording bar's STOP: the save window (T14).
    StopRecording,
    /// Game → Show Map: the map page written fresh and opened in Chrome (map page DESIGN §2.8).
    ShowMap,
    /// Game → Controls…: the window that rebinds keys and pad.
    Controls,
    /// Quit Forest Dumb, ⌘Q, Esc, or the window closed.
    Quit,
    /// View → Scale ▸, Fullscreen or Filter ▸.
    View(ViewCommand),
}

impl Command {
    /// The Forest Dumb and Game menus' commands; View's are `ViewCommand`'s.
    pub const GAME: [Command; 5] = [
        Command::StartLevel,
        Command::StopRecording,
        Command::ShowMap,
        Command::Controls,
        Command::Quit,
    ];

    /// Its menu item's id, also its name in a script's `menu …` event (View's: `scale-2`,
    /// `scale-auto`, `fullscreen`, `filter-geom`, …).
    pub fn id(self) -> String {
        match self {
            Command::StartLevel => "start-level".into(),
            Command::StopRecording => "stop-recording".into(),
            Command::ShowMap => "show-map".into(),
            Command::Controls => "controls".into(),
            Command::Quit => "quit".into(),
            Command::View(v) => v.id(),
        }
    }

    pub fn from_id(id: &str) -> Option<Command> {
        Command::GAME
            .into_iter()
            .find(|c| c.id() == id)
            .or_else(|| ViewCommand::from_id(id).map(Command::View))
    }
}

/// The menu bar, held for as long as it shows.
pub struct MenuBar {
    menu: Menu,
    stop_recording: MenuItem,
    view: ViewItems,
}

/// The menu bar, before [`MenuBar::install`] puts it in place. On the main thread.
pub fn build_menu() -> MenuBar {
    let about = AboutMetadata {
        name: Some("Forest Dumb".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        ..Default::default()
    };
    let quit = MenuItem::with_id(
        Command::Quit.id(),
        "Quit Forest Dumb",
        true,
        Some(Accelerator::new(Modifiers::META, Code::KeyQ)),
    );
    let app = Submenu::with_items(
        "Forest Dumb",
        true,
        &[
            &PredefinedMenuItem::about(Some("About Forest Dumb"), Some(about)),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(Some("Hide Forest Dumb")),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &quit,
        ],
    );
    let stop_recording = MenuItem::with_id(
        Command::StopRecording.id(),
        "Stop Recording",
        false,
        Some(Accelerator::new(Modifiers::META, Code::Period)),
    );
    let game = Submenu::with_items(
        "Game",
        true,
        &[
            &MenuItem::with_id(Command::StartLevel.id(), "Start Level…", true, None),
            &stop_recording,
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(Command::ShowMap.id(), "Show Map", true, None),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(Command::Controls.id(), "Controls…", true, None),
        ],
    );
    let view_items = ViewItems::new();
    let view = view_items.view_menu(&[]);
    let menu = Menu::new();
    for m in [app, game, view] {
        menu.append(&m.expect("a menu"))
            .expect("a menu in the menu bar");
    }
    MenuBar {
        menu,
        stop_recording,
        view: view_items,
    }
}

impl MenuBar {
    /// Stop Recording is enabled exactly while a recording runs.
    pub fn show_recording(&self, on: bool) {
        self.stop_recording.set_enabled(on);
    }

    /// View's checks: the scale, fullscreen and the filter.
    pub fn show_view(&self, scale: Scale, fullscreen: bool, filter: Filter) {
        self.view.show(scale, fullscreen, filter);
    }

    /// Make this the app's menu bar; AppKit's window tabs are turned off.
    pub fn install(&self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::ActiveEventLoopExtMacOS;
            event_loop.set_allows_automatic_window_tabbing(false);
            self.menu.init_for_nsapp();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = event_loop;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_s_id_names_it() {
        let mut all = Command::GAME.to_vec();
        all.extend(amiga_app::view_menu::SCALES.map(|s| Command::View(ViewCommand::Scale(s))));
        all.push(Command::View(ViewCommand::ToggleFullscreen));
        all.extend(Filter::ALL.map(|f| Command::View(ViewCommand::Filter(f))));
        for c in all {
            assert_eq!(Command::from_id(&c.id()), Some(c), "{}", c.id());
        }
        assert_eq!(Command::from_id("scale-2"), Some(Command::View(ViewCommand::Scale(Scale::N(2)))));
        assert_eq!(Command::from_id("filter-geom"), Some(Command::View(ViewCommand::Filter(Filter::Geom))));
        assert_eq!(Command::from_id("show-map"), Some(Command::ShowMap));
        assert_eq!(Command::from_id("fullscreen"), Some(Command::View(ViewCommand::ToggleFullscreen)));
        for bad in ["start", "scale-0", "scale-17", "filter-crt", "filter-", "widescreen"] {
            assert_eq!(Command::from_id(bad), None, "{bad}");
        }
    }
}
