//! T28: File → Export Level and Play Level File… (DESIGN §2.10, §2.11), through the real app at
//! 2×: a disk level exported where the save panel said and nowhere else, played back in place of
//! the disk level with the disk level's frames, and a file that cannot be played refused with its
//! reason, the disk level played only when chosen.

mod common;

use std::path::Path;

use common::{cache_data, headless_frames};
use sf_core::input::FrameInput;
use sf_core::{Game, Start};
use sf_e2e::{App, Image, LaunchOpts};

fn none(_: u64) -> FrameInput {
    FrameInput::NONE
}

/// The engine's pictures for the first `frames` steps of W1-1 entered from the disks.
fn w1_1_frames(data: &sf_core::GameData, frames: u64) -> Vec<Image> {
    let mut g = Game::new(data, Start::Level("W1-1".parse().unwrap()));
    (0..frames).map(|_| Image::of_picture(&g.step(FrameInput::NONE).picture)).collect()
}

/// Every `.sflevel` under `dir`, at any depth.
fn level_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() && p.file_name().is_some_and(|n| n != "target" && n != ".git") {
            out.extend(level_files(&p));
        } else if p.extension().is_some_and(|x| x == "sflevel") {
            out.push(p);
        }
    }
    out
}

fn opts(max_frames: u64) -> LaunchOpts {
    LaunchOpts {
        max_frames,
        scale: Some(2),
        ..Default::default()
    }
}

#[test]
fn w1_1_exported_from_the_menu_and_played_from_the_menu_has_the_disk_level_s_frames() {
    let mut app = App::launch(opts(220));
    let data = cache_data(&app.home);
    let file = app.home.join("My Levels/W1-1.sflevel");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let c = app.run_script(&format!(
        "save {f}\nopen {f}\n10: menu export W1-1\n20: menu play-level-file\n",
        f = file.display()
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        c.prompts,
        format!("save: W1-1.sflevel -> {f}\nopen: -> {f}\n", f = file.display()),
        "the save panel suggested the level's name, then the open panel"
    );
    let written = std::fs::read_to_string(&file).expect("the export");
    let w1_1 = &data.levels[&"W1-1".parse().unwrap()];
    assert_eq!(written, sf_assets::sflevel_encode(w1_1), "the export is the disk level");
    // Where the player chose and nowhere else: one level file in the home, none in the repo.
    assert_eq!(level_files(&app.home), std::slice::from_ref(&file));
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    assert_eq!(level_files(&repo), Vec::<std::path::PathBuf>::new(), "an export landed in the repo");
    // Before the file: the game from power-on; from it, the disk level's frames.
    let on = headless_frames(&data, 19, none);
    for n in 1..20 {
        assert_eq!(c.frames[n], on[n - 1], "frame {n}, before the level file");
    }
    let level = w1_1_frames(&data, 201);
    for n in 20..=220 {
        assert_eq!(c.frames[n], level[n - 20], "frame {n} is not step {} of the disk level", n - 19);
    }
    assert_ne!(level[0], level[200], "W1-1's frames do not move: a weak check");
    assert_eq!((c.frames[220].width, c.frames[220].height), (level[0].width, level[0].height));
}

#[test]
fn a_cancelled_export_writes_nothing_and_the_game_carries_on() {
    let mut app = App::launch(opts(60));
    let data = cache_data(&app.home);
    let c = app.run_script("save cancel\nopen cancel\n10: menu export WW-1\n20: menu play-level-file\n");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(c.prompts, "save: WW-1.sflevel -> cancel\nopen: -> cancel\n");
    assert!(level_files(&app.home).is_empty());
    let on = headless_frames(&data, 60, none);
    for n in 1..=60 {
        assert_eq!(c.frames[n], on[n - 1], "frame {n}: a cancelled panel changed the game");
    }
}

#[test]
fn a_bad_file_is_refused_with_its_reason_and_the_disk_level_plays_only_when_chosen() {
    let mut app = App::launch(opts(160));
    let data = cache_data(&app.home);
    let w1_1 = &data.levels[&"W1-1".parse().unwrap()];
    let dir = app.home.join("files");
    std::fs::create_dir_all(&dir).unwrap();
    let junk = dir.join("junk.sflevel");
    std::fs::write(&junk, "not a level").unwrap();
    let v2 = dir.join("future.sflevel");
    std::fs::write(&v2, sf_assets::sflevel_encode(w1_1).replace("\"version\": 1", "\"version\": 2")).unwrap();
    let gone = dir.join("gone.sflevel");
    let c = app.run_script(&format!(
        "open {junk}\nopen {gone}\nopen {v2}\nanswer cancel\nopen {v2}\nanswer disk-level\n\
         20: menu play-level-file\n30: menu play-level-file\n40: menu play-level-file\n\
         60: menu play-level-file\n",
        junk = junk.display(),
        gone = gone.display(),
        v2 = v2.display()
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    let refused = "“future.sflevel” cannot be played: it is a version 2 level file; this Superfrog reads version 1.";
    let lines: Vec<&str> = c.prompts.lines().collect();
    assert_eq!(lines.len(), 8, "{}", c.prompts);
    assert_eq!(lines[1], "refused: “junk.sflevel” cannot be played: it is not a level file (not JSON).");
    assert!(lines[3].starts_with("refused: “gone.sflevel” cannot be played: No such file"), "{}", lines[3]);
    assert_eq!(lines[5], format!("refused: {refused} -> cancel"));
    assert_eq!(lines[7], format!("refused: {refused} -> disk-level"));
    // Refused, the game carries on from power-on; chosen, the disk level starts.
    let on = headless_frames(&data, 59, none);
    for n in 1..60 {
        assert_eq!(c.frames[n], on[n - 1], "frame {n}: a refused file changed the game");
    }
    let level = w1_1_frames(&data, 101);
    for n in 60..=160 {
        assert_eq!(c.frames[n], level[n - 60], "frame {n} is not the disk level's step {}", n - 59);
    }
}
