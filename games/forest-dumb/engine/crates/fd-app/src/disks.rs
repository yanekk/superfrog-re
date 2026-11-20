//! The game's data from the owner's two zips (DESIGN §2.9): `--disks`, else `fd-tools`' lookup
//! (`FOREST_DUMB_DISKS`, then the main checkout). There is no picker. A folder without both zips
//! or a file that cannot be read is an error naming where it looked (DESIGN §2.12).
//!
//! Each zip is inflated once and the files read from its ADF in memory, unpacked; the zips are
//! only ever read.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use amiga_disk::ofs::OfsImage;
use fd_assets::AssetError;
use fd_core::GameData;
use fd_core::program::PROGRAM_FILE;

/// The folder the zips are read from: `--disks` when given, checked here so a wrong one names
/// itself, else the lookup's.
pub fn folder(arg: Option<&Path>) -> Result<PathBuf, String> {
    match arg {
        Some(d) => fd_tools::zip_paths(d)
            .map(|_| d.to_path_buf())
            .map_err(|e| {
                format!(
                    "Forest Dumb Forever's zips not found. Looked in --disks {}: {e}",
                    d.display()
                )
            }),
        None => fd_tools::disk_dir(),
    }
}

/// What the two engines start from.
pub struct Disks {
    /// Everything the hand engine reads.
    pub data: Arc<GameData>,
    /// The main program's file, as stored on disk 1: what the translated program is loaded from
    /// (translator DESIGN §2.8).
    pub program: Vec<u8>,
    /// Disk 2's ADF, in the translated machine's df0.
    pub disk2: Vec<u8>,
    /// Each world's object map as its level file holds it (`_MapaBobowN`, world N at N - 1), what
    /// the map page's live cells are the differences from (plans/forest-dumb-map-page DESIGN §2.5);
    /// `None` for a file that cannot be read or is not a whole map, whose world then shows no cells.
    pub objects: Vec<Option<Vec<u8>>>,
}

/// Everything the game reads, from the zips in `dir`.
pub fn read(dir: &Path) -> Result<Disks, String> {
    let zips = fd_tools::zip_paths(dir)?;
    let adfs = [fd_tools::read_adf(&zips[0])?, fd_tools::read_adf(&zips[1])?];
    let mut images = Vec::new();
    for (z, adf) in zips.iter().zip(&adfs) {
        let image = OfsImage::parse(adf).map_err(|e| format!("{}: its ADF: {e}", z.display()))?;
        images.push((z, image));
    }
    let files = |name: &str| {
        // The main program is disk 1's; every level file is disk 2's (as fd-conform's game_data).
        let i = if name == PROGRAM_FILE { 0 } else { 1 };
        let (zip, image) = &images[i];
        let packed = image.read_file(name).map_err(|e| {
            AssetError::missing(name, format!("disk {} ({}): {e}", i + 1, zip.display()))
        })?;
        fd_tools::unpack(&packed).map_err(|e| {
            AssetError::missing(name, format!("disk {} ({}): {e}", i + 1, zip.display()))
        })
    };
    let data = GameData::read(&files)
        .map(Arc::new)
        .map_err(|e| format!("the game's data in {}: {e}", dir.display()))?;
    let program = images[0].1.read_file(PROGRAM_FILE).map_err(|e| {
        format!("{PROGRAM_FILE}: disk 1 ({}): {e}", images[0].0.display())
    })?;
    let objects = (1..=6)
        .map(|w| {
            let name = &fd_assets::level_file_names(w)[4];
            let whole = fd_assets::OBJECT_COLUMNS as usize * fd_assets::LEVEL_ROWS as usize;
            match files(name) {
                Ok(m) if m.len() == whole => Some(m),
                Ok(m) => {
                    eprintln!("fd-app: {name}: {} bytes, not {whole}; the map page shows no taken items in W{w}-1", m.len());
                    None
                }
                Err(e) => {
                    eprintln!("fd-app: {e}; the map page shows no taken items in W{w}-1");
                    None
                }
            }
        })
        .collect();
    Ok(Disks {
        data,
        program,
        disk2: adfs[1].clone(),
        objects,
    })
}
