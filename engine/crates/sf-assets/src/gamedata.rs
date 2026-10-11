//! The decoded game content the engine runs on. T12 needs only the type to exist (`Game::new` takes
//! it); T08 fills it with levels, graphics and tables.

use core::fmt;
use core::str::FromStr;

/// Everything decoded from the player's disks. Empty until T08.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameData {}

/// A level as the user names it: `W<world>-<level>`, e.g. `W1-1` (docs/LEVEL-INDEX.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LevelId {
    /// 1..=6.
    pub world: u8,
    /// 1..=4.
    pub level: u8,
}

impl fmt::Display for LevelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "W{}-{}", self.world, self.level)
    }
}

impl FromStr for LevelId {
    type Err = String;

    fn from_str(s: &str) -> Result<LevelId, String> {
        let bad = || format!("`{s}` is not a level id like W1-1 (worlds 1-6, levels 1-4)");
        let (w, l) = s
            .strip_prefix('W')
            .and_then(|r| r.split_once('-'))
            .ok_or_else(bad)?;
        let world: u8 = w.parse().map_err(|_| bad())?;
        let level: u8 = l.parse().map_err(|_| bad())?;
        if !(1..=6).contains(&world) || !(1..=4).contains(&level) {
            return Err(bad());
        }
        Ok(LevelId { world, level })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ids_round_trip_and_bad_ones_are_refused() {
        let id: LevelId = "W3-2".parse().unwrap();
        assert_eq!((id.world, id.level), (3, 2));
        assert_eq!(id.to_string(), "W3-2");
        for bad in ["W0-1", "W7-1", "W1-5", "1-1", "W1", "W1-x", ""] {
            assert!(bad.parse::<LevelId>().is_err(), "{bad}");
        }
    }
}
