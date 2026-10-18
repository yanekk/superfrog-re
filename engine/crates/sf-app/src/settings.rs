//! `settings.toml` in the app-data folder (DESIGN §2.9, §3.5): scale, fullscreen, CRT, widescreen
//! and the key and pad bindings. A damaged file is replaced by the defaults with a one-line message, never
//! a refusal to start (DESIGN §2.9).
//!
//! ```toml
//! scale = "auto"        # or a whole number, 1 to 16
//! fullscreen = false
//! crt = false
//! widescreen = false
//!
//! [keys]                # each of the original's inputs, and the host keys that feed it
//! up = ["ArrowUp"]
//! fire = ["Space", "ControlLeft", "ControlRight"]
//! "key P" = ["KeyP"]
//!
//! [pad]                 # the same, for the pad's inputs (pad.rs PAD_INPUTS)
//! fire = ["South", "East"]
//! left = ["DPadLeft", "StickLeft"]
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic_write::atomic_write;
use crate::input::{KeyMap, OriginalInput, key_name, key_named};
use crate::pad::{PadInput, PadMap};

/// How large the game is drawn (DESIGN §2.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    /// The largest whole multiple that fits the window.
    Auto,
    /// This multiple of the game's 320×256, or the largest that fits when the window is smaller.
    N(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub scale: Scale,
    pub fullscreen: bool,
    /// The CRT filter (T27).
    pub crt: bool,
    /// Widescreen (T27).
    pub widescreen: bool,
    pub keys: KeyMap,
    pub pad: PadMap,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            scale: Scale::Auto,
            fullscreen: false,
            crt: false,
            widescreen: false,
            keys: KeyMap::default(),
            pad: PadMap::default(),
        }
    }
}

pub fn path(home: &Path) -> PathBuf {
    home.join("settings.toml")
}

/// The file as written: every field optional, so a file from an older build still reads.
#[derive(Serialize, Deserialize, Default)]
struct File {
    #[serde(skip_serializing_if = "Option::is_none")]
    scale: Option<toml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fullscreen: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    crt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    widescreen: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keys: Option<BTreeMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pad: Option<BTreeMap<String, Vec<String>>>,
}

impl Settings {
    pub fn to_toml(&self) -> String {
        let keys = self
            .keys
            .0
            .iter()
            .map(|(o, ks)| {
                let names = ks.iter().filter_map(|&k| key_name(k)).map(String::from).collect();
                (o.name(), names)
            })
            .collect();
        let pad = self
            .pad
            .0
            .iter()
            .map(|(o, ps)| (o.name(), ps.iter().map(|p| p.name().to_string()).collect()))
            .collect();
        let file = File {
            scale: Some(match self.scale {
                Scale::Auto => toml::Value::String("auto".into()),
                Scale::N(n) => toml::Value::Integer(n as i64),
            }),
            fullscreen: Some(self.fullscreen),
            crt: Some(self.crt),
            widescreen: Some(self.widescreen),
            keys: Some(keys),
            pad: Some(pad),
        };
        toml::to_string(&file).expect("settings serialise")
    }

    /// The settings `text` holds; a field it leaves out keeps its default. `Err` says what is
    /// wrong with it.
    pub fn from_toml(text: &str) -> Result<Settings, String> {
        let file: File = toml::from_str(text).map_err(|e| e.message().to_string())?;
        let mut s = Settings::default();
        match file.scale {
            None => {}
            Some(toml::Value::String(a)) if a == "auto" => s.scale = Scale::Auto,
            Some(toml::Value::Integer(n)) if (1..=16).contains(&n) => s.scale = Scale::N(n as u32),
            Some(v) => return Err(format!("scale {v} is not \"auto\" or a whole number 1 to 16")),
        }
        s.fullscreen = file.fullscreen.unwrap_or(s.fullscreen);
        s.crt = file.crt.unwrap_or(s.crt);
        s.widescreen = file.widescreen.unwrap_or(s.widescreen);
        if let Some(keys) = file.keys {
            // The bindings named replace the defaults for those inputs; the others keep theirs.
            for (input, names) in keys {
                let o = OriginalInput::named(&input)
                    .ok_or_else(|| format!("[keys] has no input called {input:?}"))?;
                let codes = names
                    .iter()
                    .map(|n| key_named(n).ok_or_else(|| format!("[keys] has no key called {n:?}")))
                    .collect::<Result<Vec<_>, _>>()?;
                s.keys.0.insert(o, codes);
            }
        }
        if let Some(pad) = file.pad {
            for (input, names) in pad {
                let o = OriginalInput::named(&input)
                    .ok_or_else(|| format!("[pad] has no input called {input:?}"))?;
                let ps = names
                    .iter()
                    .map(|n| PadInput::named(n).ok_or_else(|| format!("[pad] has no pad input called {n:?}")))
                    .collect::<Result<Vec<_>, _>>()?;
                s.pad.0.insert(o, ps);
            }
        }
        Ok(s)
    }
}

/// The settings in `home`, and a one-line message for the player when the file was damaged (it
/// is then replaced by the defaults). No file is the defaults, silently.
pub fn load(home: &Path) -> (Settings, Option<String>) {
    let p = path(home);
    let text = match std::fs::read(&p) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Settings::default(), None),
        Err(e) => return damaged(home, &e.to_string()),
    };
    match String::from_utf8(text)
        .map_err(|_| "it is not text".to_string())
        .and_then(|t| Settings::from_toml(&t))
    {
        Ok(s) => (s, None),
        Err(why) => damaged(home, &why),
    }
}

fn damaged(home: &Path, why: &str) -> (Settings, Option<String>) {
    let s = Settings::default();
    let note = match save(home, &s) {
        Ok(()) => "they are back to their defaults",
        Err(_) => "the defaults are used",
    };
    let msg = format!(
        "Your settings could not be read ({}), so {note}.",
        why.replace('\n', " ")
    );
    (s, Some(msg))
}

pub fn save(home: &Path, s: &Settings) -> std::io::Result<()> {
    atomic_write(&path(home), s.to_toml().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sf-app-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn settings_round_trip_through_the_file() {
        let mut s = Settings {
            scale: Scale::N(3),
            fullscreen: true,
            crt: true,
            widescreen: false,
            keys: KeyMap::default(),
            pad: PadMap::default(),
        };
        s.keys.0.insert(OriginalInput::Fire, vec![KeyCode::KeyX, KeyCode::Space]);
        s.pad.0.insert(OriginalInput::Fire, vec![PadInput::North]);
        s.pad.0.insert(OriginalInput::Up, vec![]);
        let text = s.to_toml();
        assert!(text.split("[pad]").nth(1).unwrap().contains("\nup = []\n"), "{text}");
        assert!(text.contains("fire = [\"North\"]"), "{text}");
        assert!(text.contains("left = [\"DPadLeft\", \"StickLeft\"]"), "{text}");
        assert!(text.contains("scale = 3"), "{text}");
        assert!(text.contains("fire = [\"KeyX\", \"Space\"]"), "{text}");
        assert!(text.contains("\"key P\" = [\"KeyP\"]"), "{text}");
        assert_eq!(Settings::from_toml(&text).unwrap(), s);
        assert_eq!(Settings::from_toml(&Settings::default().to_toml()).unwrap(), Settings::default());
    }

    #[test]
    fn a_partial_file_keeps_the_defaults_it_leaves_out() {
        let s = Settings::from_toml("scale = \"auto\"\n[keys]\nfire = [\"KeyZ\"]\n").unwrap();
        assert_eq!(s.scale, Scale::Auto);
        assert!(!s.fullscreen);
        assert_eq!(s.keys.0[&OriginalInput::Fire], vec![KeyCode::KeyZ]);
        assert_eq!(s.keys.0[&OriginalInput::Up], vec![KeyCode::ArrowUp]);
        assert_eq!(s.pad, PadMap::default(), "no [pad]: the pad's defaults");
        let s = Settings::from_toml("[pad]\nfire = [\"West\"]\n").unwrap();
        assert_eq!(s.pad.0[&OriginalInput::Fire], vec![PadInput::West]);
        assert_eq!(s.pad.0[&OriginalInput::Left], vec![PadInput::DPadLeft, PadInput::StickLeft]);
        assert_eq!(s.keys, KeyMap::default());
    }

    #[test]
    fn a_damaged_file_gives_the_defaults_a_message_and_a_good_file() {
        for bad in [
            "scale = 0\n",
            "scale = \"huge\"\n",
            "fullscreen = \"yes\"\n",
            "this is not toml",
            "[keys]\nfire = [\"NoSuchKey\"]\n",
            "[keys]\njump = [\"Space\"]\n",
            "[pad]\nfire = [\"Space\"]\n",
            "[pad]\njump = [\"South\"]\n",
            "pad = 1\n",
        ] {
            let h = home("bad");
            std::fs::create_dir_all(&h).unwrap();
            std::fs::write(path(&h), bad).unwrap();
            let (s, msg) = load(&h);
            assert_eq!(s, Settings::default(), "{bad}");
            let msg = msg.unwrap_or_else(|| panic!("no message for {bad:?}"));
            assert!(msg.starts_with("Your settings could not be read"), "{msg}");
            assert!(!msg.contains('\n'), "one line: {msg}");
            assert_eq!(load(&h), (Settings::default(), None), "the file was replaced: {bad}");
            std::fs::remove_dir_all(&h).unwrap();
        }
        let h = home("binary");
        std::fs::create_dir_all(&h).unwrap();
        std::fs::write(path(&h), [0xff, 0xfe, 0]).unwrap();
        assert!(load(&h).1.is_some());
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn a_field_from_a_later_build_is_ignored() {
        assert_eq!(Settings::from_toml("mouse = 1\n").unwrap(), Settings::default());
    }

    #[test]
    fn no_file_is_the_defaults_without_a_message_and_writes_nothing() {
        let h = home("none");
        assert_eq!(load(&h), (Settings::default(), None));
        assert!(!h.exists());
    }

    #[test]
    fn saved_settings_load_back() {
        let h = home("save");
        let s = Settings {
            scale: Scale::N(2),
            ..Settings::default()
        };
        save(&h, &s).unwrap();
        assert_eq!(load(&h), (s, None));
        std::fs::remove_dir_all(&h).unwrap();
    }
}
