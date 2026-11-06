//! `settings.toml` (DESIGN §2.7): the toolkit's file and format (`amiga_app::settings`), with this
//! game's default keys, the owner's (`knowledge/owner-controls.md`): Up on Space, Fire on Left
//! Option, the arrows for the other directions. The toolkit's defaults (arrows, Space and Ctrl
//! fire) fill a joystick input the file leaves out, so this game's are put back over every one the
//! file does not name.
//!
//! A damaged file is read as the defaults with the problem named, and is not overwritten until a
//! setting changes (DESIGN §2.12): `amiga_app::settings::read`, not `load`.

use std::path::Path;

pub use amiga_app::settings::{Scale, Settings, save};
use amiga_app::controls_window::JOYSTICK;
use amiga_app::keys::{KeyMap, OriginalInput};
use winit::keyboard::KeyCode;

/// This game's key map: the toolkit's, with the joystick on the owner's keys. Space and Left Option
/// feed nothing else.
pub fn default_keys() -> KeyMap {
    let mut m = KeyMap::default();
    m.0.insert(OriginalInput::Up, vec![KeyCode::Space]);
    m.0.insert(OriginalInput::Fire, vec![KeyCode::AltLeft]);
    for (o, keys) in m.0.iter_mut() {
        if !matches!(o, OriginalInput::Up | OriginalInput::Fire) {
            keys.retain(|k| !matches!(k, KeyCode::Space | KeyCode::AltLeft));
        }
    }
    m
}

pub fn defaults() -> Settings {
    Settings {
        keys: default_keys(),
        ..Settings::default()
    }
}

/// The settings in `home` and, when the file could not be read, the one line saying so.
pub fn read(home: &Path) -> (Settings, Option<String>) {
    let (mut s, damaged) = amiga_app::settings::read(home);
    if damaged.is_some() {
        return (defaults(), damaged);
    }
    let named = named_keys(&std::fs::read_to_string(amiga_app::settings::path(home)).unwrap_or_default());
    let ours = default_keys();
    for o in JOYSTICK {
        if !named.contains(&o.name()) {
            s.keys.0.insert(o, ours.0[&o].clone());
        }
    }
    (s, None)
}

/// The inputs a readable file's `[keys]` names.
fn named_keys(text: &str) -> Vec<String> {
    let v: toml::Table = toml::from_str(text).unwrap_or_default();
    match v.get("keys") {
        Some(toml::Value::Table(t)) => t.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fd-app-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_defaults_are_the_owner_s_up_on_space_and_fire_on_left_option() {
        let k = default_keys();
        assert_eq!(k.0[&OriginalInput::Up], [KeyCode::Space]);
        assert_eq!(k.0[&OriginalInput::Fire], [KeyCode::AltLeft]);
        assert_eq!(k.0[&OriginalInput::Left], [KeyCode::ArrowLeft]);
        assert_eq!(k.targets(KeyCode::Space).collect::<Vec<_>>(), [OriginalInput::Up]);
        assert_eq!(k.targets(KeyCode::ArrowUp).count(), 0);
    }

    #[test]
    fn no_file_is_this_game_s_defaults() {
        let h = home("none");
        assert_eq!(read(&h), (defaults(), None));
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn a_file_s_bindings_are_kept_and_the_ones_it_leaves_out_are_this_game_s() {
        let h = home("partial");
        std::fs::write(h.join("settings.toml"), "scale = 2\n[keys]\nup = [\"KeyW\"]\n").unwrap();
        let (s, msg) = read(&h);
        assert_eq!(msg, None);
        assert_eq!(s.scale, Scale::N(2));
        assert_eq!(s.keys.0[&OriginalInput::Up], [KeyCode::KeyW]);
        assert_eq!(s.keys.0[&OriginalInput::Fire], [KeyCode::AltLeft]);
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn saved_settings_read_back() {
        let h = home("round");
        let mut s = defaults();
        s.scale = Scale::N(3);
        s.keys.0.insert(OriginalInput::Up, vec![KeyCode::KeyK]);
        save(&h, &s).unwrap();
        assert_eq!(read(&h), (s, None));
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn a_damaged_file_is_the_defaults_named_and_left_alone() {
        let h = home("bad");
        std::fs::write(h.join("settings.toml"), "scale = 0\n").unwrap();
        let (s, msg) = read(&h);
        assert_eq!(s, defaults());
        assert!(msg.unwrap().starts_with("Your settings could not be read"));
        assert_eq!(std::fs::read_to_string(h.join("settings.toml")).unwrap(), "scale = 0\n");
        std::fs::remove_dir_all(&h).unwrap();
    }
}
