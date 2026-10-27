//! real-widescreen T12 (DESIGN §1, §5, D5): every level finished in widescreen. Each
//! `conform/scripts/wide-<level>.txt` is replayed from the level's start as Game ▸ Start Level…
//! makes it (the dialog's defaults, Spud ticked where the route says `# spud: on`, power-on seed
//! `N N` from its `# seed:` line) with View ▸ Widescreen on before the first step, and must end the
//! level (`level_state` leaves −2: the exit, the bonus level's marker 55, the witch beaten) within
//! the script's frames with no life lost. Engine only: the widescreen game has no original.
//!
//! 23 normal levels' routes are `sf-plan <level> --widescreen`'s; WB-1's is `plan_bonus
//! --widescreen`'s; WW-1's is `final-win`'s fire pattern. **W6-4's is owner-played**, not the
//! planner's (`# played: owner`; owner, 2026-10-27): the planner stalls 48 cells from its exit in
//! widescreen even guided by the owner's run, so the route is that run's recording. W1-1 runs in the block (about 4 000
//! frames); the whole set is about 200 000 frames and is `#[ignore]`d:
//! `cargo test -p sf-conform --test widescreen_complete -- --ignored`.

use sf_conform::InputScript;
use sf_core::frog::{FROG, f};
use sf_core::recording::{LevelSetup, levels};
use sf_core::{Game, LevelId, Rng};

const LEVEL_STATE: u32 = sf_core::layout::h0(0xa06);
const PLAYING: u32 = 0xffff_fffe;
/// `level_state` is not −2 on a level's first frames (sf-plan's `app::replay` waits as long).
const SETTLE: u32 = 10;

fn path(level: LevelId) -> std::path::PathBuf {
    sf_conform::case::script_path(&format!("wide-{}", level.to_string().to_lowercase()))
}

/// The header line `# key: value`'s value, among the script's leading comment lines.
fn header<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .take_while(|l| l.trim_start().starts_with('#'))
        .find_map(|l| l.trim().strip_prefix(&format!("# {key}:")))
        .map(str::trim)
}

/// Replays `level`'s widescreen route; the frame its level ended on, or why it did not.
fn finish(level: LevelId) -> Result<u32, String> {
    let p = path(level);
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    if header(&text, "timing") != Some("app") || header(&text, "widescreen") != Some("on") {
        return Err(format!("{}: not an app-timed widescreen route", p.display()));
    }
    let seed: u32 = header(&text, "seed").and_then(|s| s.parse().ok()).ok_or("no # seed: line")?;
    let setup = LevelSetup {
        level,
        spud: header(&text, "spud") == Some("on"),
        ..Default::default()
    };
    let script = InputScript::parse(&text).map_err(|e| format!("{}: {e}", p.display()))?;
    let n = script.lines.last().map_or(0, |l| l.0);
    let mut g = Game::start_level(sf_conform::display_data(), &setup, Rng::seeded(seed, seed));
    g.set_widescreen(true);
    let lives = g.state().w(FROG + f::LIVES);
    for k in 0..n {
        g.step_unseen(script.input_at(k));
        let st = g.state();
        if st.w(FROG + f::LIVES) < lives || st.w(FROG + f::DYING) != 0 {
            return Err(format!("{level}: a life lost at frame {k}"));
        }
        if k >= SETTLE && st.l(LEVEL_STATE) != PLAYING {
            return Ok(k + 1);
        }
    }
    Err(format!("{level}: still playing at the script's end, frame {n}"))
}

#[test]
fn w1_1_is_finished_in_widescreen() {
    let level: LevelId = "W1-1".parse().unwrap();
    assert!(finish(level).is_ok(), "{:?}", finish(level));
}

/// W6-4's route says it is the owner's play, so it is never taken for a planner result.
#[test]
fn w6_4s_route_is_labelled_owner_played() {
    let text = std::fs::read_to_string(path("W6-4".parse().unwrap())).unwrap();
    assert_eq!(header(&text, "played"), Some("owner"));
    assert!(text.contains("OWNER-PLAYED, not the planner's"));
}

/// Every level a game plays (W1-1 … W6-4, the bonus level, the final level) has a widescreen
/// route, and each replays to the level's end.
#[test]
#[ignore = "about 200 000 frames; run with --ignored"]
fn every_level_is_finished_in_widescreen() {
    let all = levels();
    assert_eq!(all.len(), 26);
    let fails: Vec<String> = all.iter().filter_map(|&l| finish(l).err()).collect();
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}
