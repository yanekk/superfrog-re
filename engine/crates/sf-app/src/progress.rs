//! `progress.toml` in the app-data folder (DESIGN §2.9, §3.5): the furthest level a game has
//! reached, from [`sf_core::Game::progress`], offered at the next launch as a start point so
//! passwords need not be retyped. A damaged file is forgotten with a one-line message.
//!
//! ```toml
//! world = 2   # 1..6, 7 the bonus level
//! level = 1   # 1..4
//! ```

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sf_core::Progress;

use crate::atomic_write::atomic_write;

pub fn path(home: &Path) -> PathBuf {
    home.join("progress.toml")
}

/// Where `p` comes in a game: worlds 1 to 5, the bonus level (world 7) after World 5, then World
/// 6. World 0 is no game played.
fn order(p: Progress) -> (u8, u8) {
    let w = match p.world {
        7 => 11,
        6 => 12,
        w => w * 2,
    };
    (w, p.level)
}

/// Is `p` further into the game than `than`?
pub fn further(p: Progress, than: Option<Progress>) -> bool {
    p.world != 0 && than.is_none_or(|t| order(p) > order(t))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    world: u8,
    level: u8,
}

/// The furthest progress remembered in `home` (`None`: no game has reached a level), and a
/// one-line message for the player when the file was damaged (it is then removed).
pub fn load(home: &Path) -> (Option<Progress>, Option<String>) {
    let p = path(home);
    let bytes = match std::fs::read(&p) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (None, None),
        Err(e) => return damaged(&p, &e.to_string()),
    };
    let parsed = String::from_utf8(bytes)
        .map_err(|_| "it is not text".to_string())
        .and_then(|t| toml::from_str::<File>(&t).map_err(|e| e.message().to_string()))
        .and_then(|f| {
            let ok = matches!(f.world, 1..=7) && matches!(f.level, 1..=4);
            ok.then_some(Progress { world: f.world, level: f.level })
                .ok_or_else(|| format!("World {} level {} is no level of the game", f.world, f.level))
        });
    match parsed {
        Ok(progress) => (Some(progress), None),
        Err(why) => damaged(&p, &why),
    }
}

fn damaged(p: &Path, why: &str) -> (Option<Progress>, Option<String>) {
    let _ = std::fs::remove_file(p);
    let msg = format!(
        "Your progress could not be read ({}), so the game starts from the beginning.",
        why.replace('\n', " ")
    );
    (None, Some(msg))
}

pub fn save(home: &Path, p: Progress) -> std::io::Result<()> {
    let text = toml::to_string(&File {
        world: p.world,
        level: p.level,
    })
    .expect("progress serialises");
    atomic_write(&path(home), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(world: u8, level: u8) -> Progress {
        Progress { world, level }
    }

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sf-app-progress-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn further_follows_the_game_s_order_with_the_bonus_level_after_world_5() {
        assert!(further(at(1, 1), None));
        assert!(!further(at(0, 0), None), "no level played is no progress");
        assert!(further(at(1, 2), Some(at(1, 1))));
        assert!(further(at(2, 1), Some(at(1, 4))));
        assert!(!further(at(1, 4), Some(at(2, 1))));
        assert!(!further(at(2, 1), Some(at(2, 1))));
        assert!(further(at(7, 1), Some(at(5, 4))));
        assert!(further(at(6, 1), Some(at(7, 1))));
        assert!(!further(at(7, 1), Some(at(6, 1))));
    }

    #[test]
    fn progress_round_trips_and_no_file_is_none() {
        let h = home("trip");
        assert_eq!(load(&h), (None, None));
        save(&h, at(3, 2)).unwrap();
        assert_eq!(std::fs::read_to_string(path(&h)).unwrap(), "world = 3\nlevel = 2\n");
        assert_eq!(load(&h), (Some(at(3, 2)), None));
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn a_damaged_file_is_forgotten_with_a_message() {
        for bad in ["world = 9\nlevel = 1\n", "world = 2\n", "garbage", "world = 2\nlevel = 1\nx = 1\n"] {
            let h = home("bad");
            std::fs::create_dir_all(&h).unwrap();
            std::fs::write(path(&h), bad).unwrap();
            let (p, msg) = load(&h);
            assert_eq!(p, None, "{bad}");
            let msg = msg.unwrap();
            assert!(msg.starts_with("Your progress could not be read"), "{msg}");
            assert!(!msg.contains('\n'), "{msg}");
            assert_eq!(load(&h), (None, None), "{bad}: the damaged file is gone");
            std::fs::remove_dir_all(&h).unwrap();
        }
    }
}
