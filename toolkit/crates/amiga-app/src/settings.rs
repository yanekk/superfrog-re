//! `settings.toml` in a game's app-data folder: scale, fullscreen, filter, widescreen and the key
//! and pad bindings. A damaged file gives the defaults with a one-line message, never a refusal to
//! start: [`load`] also writes the defaults over it, [`read`] leaves it for the player to fix.
//! A game with no filter or widescreen in its menus leaves them at their defaults.
//!
//! ```toml
//! scale = "auto"        # or a whole number, 1 to 16
//! fullscreen = false
//! filter = "none"      # or "basic", "lottes", "geom", "easymode"; an older `crt = true` is "basic"
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

use crate::atomic_write::write as atomic_write;
use crate::keys::{KeyMap, OriginalInput, key_name, key_named};
use crate::pad::{PadInput, PadMap};

pub use crate::video::{Filter, Scale};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub scale: Scale,
    pub fullscreen: bool,
    /// View → Filter.
    pub filter: Filter,
    /// View → Widescreen: 16:9, the level's extra columns each side.
    pub widescreen: bool,
    pub keys: KeyMap,
    pub pad: PadMap,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            scale: Scale::Auto,
            fullscreen: false,
            filter: Filter::None,
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
    filter: Option<String>,
    /// Before View → Filter: `true` is Basic, `false` None. Read, never written.
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
            filter: Some(self.filter.name().into()),
            crt: None,
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
        s.filter = match (file.filter, file.crt) {
            (Some(name), _) => Filter::named(&name).ok_or_else(|| {
                let names: Vec<String> = Filter::ALL.iter().map(|f| format!("{:?}", f.name())).collect();
                format!("filter {name:?} is not one of {}", names.join(", "))
            })?,
            (None, Some(true)) => Filter::Basic,
            (None, Some(false)) | (None, None) => Filter::None,
        };
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
    match parse(home) {
        Ok(s) => (s, None),
        Err(why) => {
            let s = Settings::default();
            let note = match save(home, &s) {
                Ok(()) => "they are back to their defaults",
                Err(_) => "the defaults are used",
            };
            (s, Some(damaged(&why, note)))
        }
    }
}

/// [`load`], but a damaged file is left as it is, to be written only when a setting changes: the
/// defaults are used and the message says so.
pub fn read(home: &Path) -> (Settings, Option<String>) {
    match parse(home) {
        Ok(s) => (s, None),
        Err(why) => (Settings::default(), Some(damaged(&why, "the defaults are used"))),
    }
}

/// The settings `home` holds, the defaults with no file; `Err` says what is wrong with the file.
fn parse(home: &Path) -> Result<Settings, String> {
    let text = match std::fs::read(path(home)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(e) => return Err(e.to_string()),
    };
    String::from_utf8(text)
        .map_err(|_| "it is not text".to_string())
        .and_then(|t| Settings::from_toml(&t))
}

/// The one line saying the file could not be read, and `note`, what was done instead.
fn damaged(why: &str, note: &str) -> String {
    format!("Your settings could not be read ({}), so {note}.", why.replace('\n', " "))
}

pub fn save(home: &Path, s: &Settings) -> std::io::Result<()> {
    atomic_write(&path(home), s.to_toml().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("amiga-app-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn settings_round_trip_through_the_file() {
        let mut s = Settings {
            scale: Scale::N(3),
            fullscreen: true,
            filter: Filter::Lottes,
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
            "filter = \"crt\"\n",
            "filter = true\n",
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
    fn every_filter_round_trips_through_the_file() {
        for f in Filter::ALL {
            let s = Settings {
                filter: f,
                ..Settings::default()
            };
            let text = s.to_toml();
            assert!(text.contains(&format!("filter = \"{}\"\n", f.name())), "{text}");
            assert!(!text.contains("crt"), "{text}");
            assert_eq!(Settings::from_toml(&text).unwrap(), s);
            assert_eq!(Filter::named(f.name()), Some(f));
        }
        assert_eq!(Settings::from_toml("").unwrap().filter, Filter::None, "neither field: none");
    }

    #[test]
    fn an_older_crt_line_reads_as_basic_or_none_and_filter_beats_it() {
        let filter = |text: &str| Settings::from_toml(text).unwrap().filter;
        assert_eq!(filter("crt = true\n"), Filter::Basic);
        assert_eq!(filter("crt = false\n"), Filter::None);
        assert_eq!(filter("crt = true\nfilter = \"geom\"\n"), Filter::Geom);
        assert_eq!(filter("crt = false\nfilter = \"easymode\"\n"), Filter::Easymode);
        assert_eq!(filter("filter = \"none\"\ncrt = true\n"), Filter::None);
    }

    #[test]
    fn an_unknown_filter_names_the_allowed_ones() {
        let e = Settings::from_toml("filter = \"royale\"\n").unwrap_err();
        assert!(e.contains("royale"), "{e}");
        for f in Filter::ALL {
            assert!(e.contains(&format!("\"{}\"", f.name())), "{e}");
        }
    }

    #[test]
    fn a_migrated_crt_file_is_written_back_with_a_filter_line() {
        let h = home("migrate");
        std::fs::create_dir_all(&h).unwrap();
        std::fs::write(path(&h), "scale = 2\ncrt = true\n").unwrap();
        let (s, msg) = load(&h);
        assert_eq!((s.filter, msg), (Filter::Basic, None));
        save(&h, &s).unwrap();
        let text = std::fs::read_to_string(path(&h)).unwrap();
        assert!(text.contains("filter = \"basic\"\n") && !text.contains("crt"), "{text}");
        assert_eq!(load(&h).0, s);
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn read_names_a_damaged_file_but_leaves_it_alone() {
        let h = home("read");
        std::fs::create_dir_all(&h).unwrap();
        std::fs::write(path(&h), "scale = 0\n").unwrap();
        let (s, msg) = read(&h);
        assert_eq!(s, Settings::default());
        let msg = msg.expect("a message");
        assert!(msg.starts_with("Your settings could not be read") && msg.ends_with("so the defaults are used."), "{msg}");
        assert_eq!(std::fs::read_to_string(path(&h)).unwrap(), "scale = 0\n", "not overwritten");
        std::fs::remove_dir_all(&h).unwrap();
        assert_eq!(read(&h), (Settings::default(), None), "no file: the defaults, silently");
        assert!(!h.exists());
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
