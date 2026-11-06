//! The native macOS menu bar (DESIGN §2.7): only what the design lists.
//!
//! ```text
//! Forest Dumb   About · Services · Hide, Hide Others, Show All · Quit Forest Dumb ⌘Q
//! Game          Controls…
//! ```
//!
//! Start Level… joins Game with the recorder (T14). A [`Command`] is the one path from the menu to
//! the app: a click, its shortcut, a script's `menu …` and a key `input.rs` keeps for the app all
//! become one, and the app acts on it.

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use winit::event_loop::ActiveEventLoop;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Game → Controls…: the window that rebinds keys and pad.
    Controls,
    /// Quit Forest Dumb, ⌘Q, Esc, or the window closed.
    Quit,
}

impl Command {
    pub const ALL: [Command; 2] = [Command::Controls, Command::Quit];

    /// Its menu item's id, also its name in a script's `menu …` event.
    pub fn id(self) -> &'static str {
        match self {
            Command::Controls => "controls",
            Command::Quit => "quit",
        }
    }

    pub fn from_id(id: &str) -> Option<Command> {
        Command::ALL.into_iter().find(|c| c.id() == id)
    }
}

/// The menu bar, held for as long as it shows.
pub struct MenuBar {
    menu: Menu,
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
    let game = Submenu::with_items(
        "Game",
        true,
        &[&MenuItem::with_id(Command::Controls.id(), "Controls…", true, None)],
    );
    let menu = Menu::new();
    for m in [app, game] {
        menu.append(&m.expect("a menu")).expect("a menu in the menu bar");
    }
    MenuBar { menu }
}

impl MenuBar {
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
        for c in Command::ALL {
            assert_eq!(Command::from_id(c.id()), Some(c));
        }
        assert_eq!(Command::from_id("start-level"), None, "T14's");
    }
}
