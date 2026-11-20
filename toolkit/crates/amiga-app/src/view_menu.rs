//! The View menu's Scale, Fullscreen and Filter items, which every game's app offers alike:
//!
//! ```text
//! View    Scale ▸ 1× 2× 3× Auto · Fullscreen ⌃⌘F · Filter ▸ None Basic Lottes Geom Easymode · <the game's own>
//! ```
//!
//! [`ViewItems`] holds the items and their checks; [`ViewItems::view_menu`] builds the "View" menu
//! with any items the game adds after Filter. A [`ViewCommand`] is what one of them asks for; its
//! [`ViewCommand::id`] is the item's id and a script's `menu …` name, and a game wraps it in its own
//! command. Acting on it (resizing the window, saving the settings) is the game's.
//!
//! Before the app finishes launching it calls [`before_launch`], or AppKit adds its own "Enter Full
//! Screen" to a menu titled View, beside the app's Fullscreen.

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{CheckMenuItem, IsMenuItem, PredefinedMenuItem, Submenu};
use winit::window::{Fullscreen, Window};

use crate::video::{Filter, Scale};

/// The scales the View menu offers.
pub const SCALES: [Scale; 4] = [Scale::N(1), Scale::N(2), Scale::N(3), Scale::Auto];

/// What a View item asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewCommand {
    /// View → Scale ▸ this scale.
    Scale(Scale),
    /// View → Fullscreen, ⌃⌘F.
    ToggleFullscreen,
    /// View → Filter ▸ this filter.
    Filter(Filter),
}

impl ViewCommand {
    /// Its menu item's id, also its name in a script's `menu …` event.
    pub fn id(self) -> String {
        match self {
            ViewCommand::Scale(Scale::Auto) => "scale-auto".into(),
            ViewCommand::Scale(Scale::N(n)) => format!("scale-{n}"),
            ViewCommand::ToggleFullscreen => "fullscreen".into(),
            ViewCommand::Filter(f) => format!("filter-{}", f.name()),
        }
    }

    /// The command whose [`ViewCommand::id`] is `id`. A script may name any whole scale 1 to 16,
    /// though the menu shows only [`SCALES`].
    pub fn from_id(id: &str) -> Option<ViewCommand> {
        match id {
            "scale-auto" => Some(ViewCommand::Scale(Scale::Auto)),
            "fullscreen" => Some(ViewCommand::ToggleFullscreen),
            _ if id.starts_with("filter-") => Filter::named(&id[7..]).map(ViewCommand::Filter),
            _ => {
                let n: u32 = id.strip_prefix("scale-")?.parse().ok()?;
                (1..=16).contains(&n).then_some(ViewCommand::Scale(Scale::N(n)))
            }
        }
    }
}

/// The View menu's Scale, Fullscreen and Filter items, whose checks follow the app's state.
pub struct ViewItems {
    scales: Vec<(Scale, CheckMenuItem)>,
    fullscreen: CheckMenuItem,
    filters: Vec<(Filter, CheckMenuItem)>,
}

fn check(c: ViewCommand, title: &str, accelerator: Option<Accelerator>) -> CheckMenuItem {
    CheckMenuItem::with_id(c.id(), title, true, false, accelerator)
}

impl ViewItems {
    /// The items, nothing checked yet. On the main thread.
    pub fn new() -> ViewItems {
        let scales = SCALES
            .iter()
            .map(|&s| {
                let title = match s {
                    Scale::Auto => "Auto".to_string(),
                    Scale::N(n) => format!("{n}×"),
                };
                (s, check(ViewCommand::Scale(s), &title, None))
            })
            .collect();
        let fullscreen = check(
            ViewCommand::ToggleFullscreen,
            "Fullscreen",
            Some(Accelerator::new(Modifiers::CONTROL | Modifiers::META, Code::KeyF)),
        );
        let filters = Filter::ALL
            .iter()
            .map(|&f| (f, check(ViewCommand::Filter(f), f.title(), None)))
            .collect();
        ViewItems { scales, fullscreen, filters }
    }

    /// The "View" menu: Scale ▸, a separator, Fullscreen, a separator, Filter ▸, then `extra`.
    pub fn view_menu(&self, extra: &[&dyn IsMenuItem]) -> muda::Result<Submenu> {
        let scale_items: Vec<&dyn IsMenuItem> = self.scales.iter().map(|(_, i)| i as &dyn IsMenuItem).collect();
        let scale = Submenu::with_items("Scale", true, &scale_items)?;
        let filter_items: Vec<&dyn IsMenuItem> = self.filters.iter().map(|(_, i)| i as &dyn IsMenuItem).collect();
        let filter = Submenu::with_items("Filter", true, &filter_items)?;
        let (sep1, sep2) = (PredefinedMenuItem::separator(), PredefinedMenuItem::separator());
        let mut items: Vec<&dyn IsMenuItem> = vec![&scale, &sep1, &self.fullscreen, &sep2, &filter];
        items.extend_from_slice(extra);
        Submenu::with_items("View", true, &items)
    }

    /// Check `scale` among the scales, Fullscreen when `fullscreen`, `filter` among the filters.
    pub fn show(&self, scale: Scale, fullscreen: bool, filter: Filter) {
        for (s, i) in &self.scales {
            i.set_checked(*s == scale);
        }
        self.fullscreen.set_checked(fullscreen);
        for (f, i) in &self.filters {
            i.set_checked(*f == filter);
        }
    }
}

impl Default for ViewItems {
    fn default() -> ViewItems {
        ViewItems::new()
    }
}

/// Fullscreen on the window's display, or back to the window.
pub fn set_fullscreen(window: &Window, on: bool) {
    window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
}

/// Before the app finishes launching: AppKit adds "Enter Full Screen" to a menu titled View unless
/// told not to, and View → Fullscreen is the app's own.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_view_id_names_its_command_and_bad_ones_name_none() {
        let mut all: Vec<ViewCommand> = SCALES.map(ViewCommand::Scale).to_vec();
        all.push(ViewCommand::Scale(Scale::N(16)));
        all.push(ViewCommand::ToggleFullscreen);
        all.extend(Filter::ALL.map(ViewCommand::Filter));
        for c in all {
            assert_eq!(ViewCommand::from_id(&c.id()), Some(c), "{}", c.id());
        }
        let ids: Vec<String> = SCALES.map(|s| ViewCommand::Scale(s).id()).to_vec();
        assert_eq!(ids, ["scale-1", "scale-2", "scale-3", "scale-auto"]);
        let filters: Vec<String> = Filter::ALL.map(|f| ViewCommand::Filter(f).id()).to_vec();
        assert_eq!(filters, ["filter-none", "filter-basic", "filter-lottes", "filter-geom", "filter-easymode"]);
        for bad in ["", "scale-0", "scale-17", "scale-x", "scale-", "filter-", "filter-crt", "filter-Basic", "Fullscreen", "widescreen"] {
            assert_eq!(ViewCommand::from_id(bad), None, "{bad}");
        }
    }
}
