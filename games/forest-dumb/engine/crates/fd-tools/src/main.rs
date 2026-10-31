//! `fd-tools extract [--dir DIR] [--out DIR]`: both disks into `build/`, the inventory into `docs/`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: fd-tools extract [--dir DIR] [--out games/forest-dumb/build]";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fd-tools: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    // Anchored on this crate, so the worktree it was built in is the one written to, from any cwd.
    let game = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("the crate sits at games/forest-dumb/engine/crates/fd-tools");
    let mut dir = None;
    let mut out = game.join("build");
    let mut args = args.into_iter();
    if args.next().as_deref() != Some("extract") {
        return Err(USAGE.to_string());
    }
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(USAGE)?;
        match flag.as_str() {
            "--dir" => dir = Some(PathBuf::from(value)),
            "--out" => out = PathBuf::from(value),
            _ => return Err(USAGE.to_string()),
        }
    }
    let dir = match dir {
        Some(d) => d,
        None => fd_tools::disk_dir()?,
    };
    let files = fd_tools::extract(&dir, &out)?;
    for f in &files {
        println!("disk{} {} {} {}", f.disk, f.name, f.size, f.sha1);
    }
    let inventory = game.join("docs/04-file-inventory.txt");
    let text = fd_tools::inventory_text(&dir, &files)?;
    std::fs::write(&inventory, text).map_err(|e| format!("{}: {e}", inventory.display()))?;
    println!(
        "{} files, inventory written to {}",
        files.len(),
        inventory.display()
    );
    Ok(())
}
