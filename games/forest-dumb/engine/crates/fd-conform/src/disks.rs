//! The two disks' ADF bytes for the oracle, from the owner's zips, which are only read.

use std::path::Path;

/// Disks 1 and 2 from wherever `fd-tools` finds them, or an error naming every place it looked.
pub fn load() -> Result<Vec<Vec<u8>>, String> {
    read(&fd_tools::disk_dir()?)
}

/// Disks 1 and 2 from the zips in `dir`, or an error naming the path that failed.
pub fn read(dir: &Path) -> Result<Vec<Vec<u8>>, String> {
    let zips = fd_tools::zip_paths(dir)?;
    Ok(vec![
        fd_tools::read_adf(&zips[0])?,
        fd_tools::read_adf(&zips[1])?,
    ])
}
