//! The lookup order and its error, on fake folders.

use std::path::{Path, PathBuf};

use fd_tools::locate;

fn fake(tag: &str, which: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fd-locate-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    add(&dir, which);
    dir
}

fn add(dir: &Path, which: &[u8]) {
    for n in which {
        std::fs::write(dir.join(format!("Forest (Disk {n} of 2)[cr WT].zip")), b"").unwrap();
    }
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn the_env_var_wins() {
    let env = fake("env", &[1, 2]);
    let checkout = fake("env-checkout", &[1, 2]);
    assert_eq!(locate(Some(s(&env)), &checkout), Ok(env));
}

#[test]
fn an_incomplete_or_empty_env_falls_through() {
    let env = fake("incomplete", &[1]);
    let checkout = fake("through", &[1, 2]);
    assert_eq!(locate(Some(s(&env)), &checkout), Ok(checkout.clone()));
    assert_eq!(locate(Some(""), &checkout), Ok(checkout));
}

#[test]
fn the_game_folder_comes_before_the_root() {
    let checkout = fake("order", &[1, 2]);
    assert_eq!(locate(None, &checkout), Ok(checkout.clone()));
    let game = checkout.join("games/forest-dumb");
    std::fs::create_dir_all(&game).unwrap();
    add(&game, &[1, 2]);
    assert_eq!(locate(None, &checkout), Ok(game));
}

#[test]
fn nothing_found_names_every_path_it_looked_at() {
    let env = fake("err-env", &[2]);
    let checkout = fake("err-checkout", &[]);
    let e = locate(Some(s(&env)), &checkout).unwrap_err();
    assert!(
        e.contains(&format!(
            "FOREST_DUMB_DISKS={}: no zip of disk 1 of 2",
            s(&env)
        )),
        "{e}"
    );
    assert!(
        e.contains(&format!(
            "main checkout {}: no zip of disk 1 of 2",
            s(&checkout)
        )),
        "{e}"
    );
    let game = checkout.join("games/forest-dumb");
    assert!(
        e.contains(&format!("main checkout {}: cannot read it", s(&game))),
        "{e}"
    );
    let e = locate(None, &checkout).unwrap_err();
    assert!(e.contains("FOREST_DUMB_DISKS: not set"), "{e}");
}

#[test]
fn two_zips_of_one_disk_are_refused_not_chosen_between() {
    let dir = fake("two", &[1, 2]);
    std::fs::write(dir.join("Other (Disk 1 of 2).zip"), b"").unwrap();
    let e = fd_tools::zip_paths(&dir).unwrap_err();
    assert!(e.contains("2 zips of disk 1 of 2"), "{e}");
}
