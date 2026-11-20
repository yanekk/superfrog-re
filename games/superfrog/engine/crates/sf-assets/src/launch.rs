//! What to do on launch about the game data (DESIGN §2.2, §2.11): use the cache, re-import from the
//! disks remembered from last time, or ask for the disks. The shell gathers the facts (reads the
//! cache and the remembered disk files) and acts on the answer; the decision is here.

use sf_disk::{Offered, identify};

use crate::cache::CacheError;

/// What the shell found on launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchFacts {
    /// `None`: there is no cache file. Otherwise what [`crate::cache_decode`] said of it.
    pub cache: Option<Result<(), CacheError>>,
    /// `None`: no disks were remembered. Otherwise the remembered files that could still be read
    /// (one that is gone is left out).
    pub remembered_disks: Option<Vec<Offered>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchAction {
    UseCache,
    /// Import again from the remembered disks; `reason` says why, for the player.
    Reimport {
        reason: String,
    },
    /// Show the disk picker; `reason` says why, for the player.
    AskForDisks {
        reason: String,
    },
}

pub fn decide(facts: &LaunchFacts) -> LaunchAction {
    let why_not_cache = match &facts.cache {
        Some(Ok(())) => return LaunchAction::UseCache,
        Some(Err(e)) => e.to_string(),
        None => "the game data has not been imported yet".to_string(),
    };
    let Some(files) = &facts.remembered_disks else {
        return LaunchAction::AskForDisks {
            reason: format!(
                "{}. Choose your four Superfrog disk images to import them.",
                capital(&why_not_cache)
            ),
        };
    };
    match identify(files) {
        Ok(_) => LaunchAction::Reimport {
            reason: format!(
                "{}, so it is being imported from your Superfrog disks.",
                capital(&why_not_cache)
            ),
        },
        Err(errors) => {
            let mut reason = format!(
                "{}, and the disk images used last time are gone or have changed. Choose your four \
                 Superfrog disk images again.",
                capital(&why_not_cache)
            );
            for e in errors {
                reason.push(' ');
                reason.push_str(&e.to_string());
            }
            LaunchAction::AskForDisks { reason }
        }
    }
}

/// `s` with its first letter in upper case.
fn capital(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}
