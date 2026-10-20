//! Test-side locator for the owner's four Superfrog disk images (DESIGN §5, "Disk images").
//!
//! Looks in `SUPERFROG_DISKS` first, then in the main checkout (the parent of the git common
//! dir, so it works from any worktree). A test that needs the disks and cannot find them fails
//! with the error from here rather than skipping.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The folder holding the four disk images.
pub fn disk_dir() -> Result<PathBuf, String> {
    locate(std::env::var_os("SUPERFROG_DISKS"), main_checkout())
}

/// `disk_dir` with its two inputs passed in, so tests need not touch the process environment.
pub fn locate(env: Option<OsString>, checkout: Result<PathBuf, String>) -> Result<PathBuf, String> {
    let mut looked = Vec::new();
    match env.filter(|v| !v.is_empty()) {
        Some(v) => {
            let dir = PathBuf::from(v);
            match missing_disks(&dir) {
                None => return Ok(dir),
                Some(why) => looked.push(format!("SUPERFROG_DISKS={}: {why}", dir.display())),
            }
        }
        None => looked.push("SUPERFROG_DISKS: not set".to_string()),
    }
    match checkout {
        Ok(dir) => match missing_disks(&dir) {
            None => return Ok(dir),
            Some(why) => looked.push(format!("main checkout {}: {why}", dir.display())),
        },
        Err(e) => looked.push(format!("main checkout: {e}")),
    }
    Err(format!(
        "Superfrog disk images not found. Looked in:\n  - {}\nPut the four `Superfrog (1993)(Team 17)(Disk N of 4)...` \
         .zip or .adf files in the main checkout root, or point SUPERFROG_DISKS at their folder.",
        looked.join("\n  - ")
    ))
}

/// `<git common dir>/..`: the main checkout, whichever worktree this runs in.
pub fn main_checkout() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let common = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    common
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("git common dir {} has no parent", common.display()))
}

/// The disk files in the folder `disk_dir` finds, as (file name, bytes), in disk order. Only the
/// `.zip`/`.adf` files whose name says `disk N of 4` are read.
pub fn read_disk_files() -> Result<Vec<(String, Vec<u8>)>, String> {
    let dir = disk_dir()?;
    let mut files = Vec::new();
    for e in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = e.map_err(|e| e.to_string())?.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let lower = name.to_lowercase();
        if (lower.ends_with(".zip") || lower.ends_with(".adf"))
            && (1..=4).any(|n| lower.contains(&format!("disk {n} of 4")))
        {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.push((name, bytes));
        }
    }
    files.sort();
    Ok(files)
}

/// One chunk as `tools/atn.py` carves it: offset, decompressed size, compressed size, the length
/// of the carved bytes and their SHA-1.
pub type PyChunk = (u32, u32, u32, usize, String);

/// The chunks the Python carver (`carve_chunks` in `tools/atn.py`) finds in `adf`, run in the
/// worktree's venv (the test block's setup makes it).
pub fn python_carve(adf: &[u8]) -> Result<Vec<PyChunk>, String> {
    use std::io::Write;
    use std::process::Stdio;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let python = root.join("venv/bin/python");
    let script = "import sys, hashlib\n\
                  sys.path.insert(0, sys.argv[1])\n\
                  from atn import carve_chunks\n\
                  for o, d, c, b in carve_chunks(sys.stdin.buffer.read()):\n    \
                  print(o, d, c, len(b), hashlib.sha1(b).hexdigest())\n";
    let mut child = Command::new(&python)
        .arg("-c")
        .arg(script)
        .arg(root.join("tools"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "could not run {} ({e}); run the test block's setup",
                python.display()
            )
        })?;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(adf)
        .map_err(|e| format!("writing to the Python carver: {e}"))?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "the Python carver failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let num = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok());
            match (num(0), num(1), num(2), num(3), f.get(4)) {
                (Some(o), Some(d), Some(c), Some(n), Some(h)) if f.len() == 5 => {
                    Ok((o as u32, d as u32, c as u32, n as usize, h.to_string()))
                }
                _ => Err(format!("unexpected line from the Python carver: {l}")),
            }
        })
        .collect()
}

/// One chunk as `tools/atn.py` decompresses it under emulation.
#[derive(Debug, Clone)]
pub struct PyDecoded {
    /// The routine returned success (`d0 = -1`, ended on its `rts`).
    pub ok: bool,
    pub decomp_size: u32,
    pub data: Vec<u8>,
}

const PY_DECOMPRESS: &str = r#"
import sys, os
sys.path.insert(0, sys.argv[1])
from atn import carve_chunks, ATNDecompressor
out = sys.argv[2]
adfs = [open(os.path.join(out, f'disk{n}.adf'), 'rb').read() for n in range(1, 5)]
dec = ATNDecompressor(adfs[0])
with open(os.path.join(out, 'manifest.txt'), 'w') as m:
    for d, adf in enumerate(adfs, 1):
        for i, (o, dsz, csz, ch) in enumerate(carve_chunks(adf), 1):
            name = f'd{d}_c{i:02d}'
            dec.m.mem.clear_block(dec.SRC, 0x100000, 0)
            data, ok = dec.decompress(ch, dsz)
            open(os.path.join(out, name + '.bin'), 'wb').write(data)
            m.write(f'{name} {int(ok)} {dsz}\n')
            if len(ch) - csz < 46:
                dec.m.mem.clear_block(dec.SRC, 0x100000, 0)
                data, ok = dec.decompress(adf[o:o + len(ch) + 512], dsz)
                open(os.path.join(out, name + '_ext.bin'), 'wb').write(data)
                m.write(f'{name}_ext {int(ok)} {dsz}\n')
"#;

/// Every chunk of the four ADFs (disk order) decompressed by the emulated original in
/// `tools/atn.py`, keyed `d1_c01`… A chunk whose 46-byte trailer runs past its carved end is also
/// decoded with the next disk sector appended, keyed `<id>_ext`. The emulator's source window is
/// cleared before each chunk, so a read past the chunk sees zeros, not the previous chunk.
///
/// Works through a temp dir, removed before returning: disk images and what is unpacked from them
/// are the owner's (DESIGN §2.3).
pub fn python_decompress(
    adfs: [&[u8]; 4],
) -> Result<std::collections::BTreeMap<String, PyDecoded>, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = std::env::temp_dir().join(format!("sf-atn-ref-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = (|| {
        for (n, adf) in adfs.iter().enumerate() {
            let path = dir.join(format!("disk{}.adf", n + 1));
            std::fs::write(&path, adf).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        let python = root.join("venv/bin/python");
        let out = Command::new(&python)
            .arg("-c")
            .arg(PY_DECOMPRESS)
            .arg(root.join("tools"))
            .arg(&dir)
            .output()
            .map_err(|e| {
                format!(
                    "could not run {} ({e}); run the test block's setup",
                    python.display()
                )
            })?;
        if !out.status.success() {
            return Err(format!(
                "the Python decompressor failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let manifest = std::fs::read_to_string(dir.join("manifest.txt"))
            .map_err(|e| format!("manifest: {e}"))?;
        manifest
            .lines()
            .map(|l| {
                let f: Vec<&str> = l.split(' ').collect();
                let bad = || format!("unexpected line from the Python decompressor: {l}");
                let (name, ok, dsz) = match f[..] {
                    [name, ok, dsz] => (name, ok == "1", dsz.parse().map_err(|_| bad())?),
                    _ => return Err(bad()),
                };
                let data = std::fs::read(dir.join(format!("{name}.bin")))
                    .map_err(|e| format!("{name}.bin: {e}"))?;
                Ok((
                    name.to_string(),
                    PyDecoded {
                        ok,
                        decomp_size: dsz,
                        data,
                    },
                ))
            })
            .collect()
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// `None` if `dir` holds all four disks (zipped or raw), else what is wrong with it.
fn missing_disks(dir: &Path) -> Option<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => return Some(format!("cannot read it ({e})")),
    };
    let names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_lowercase())
        .filter(|n| n.ends_with(".zip") || n.ends_with(".adf"))
        .collect();
    let missing: Vec<String> = (1..=4)
        .filter(|n| !names.iter().any(|f| f.contains(&format!("disk {n} of 4"))))
        .map(|n| n.to_string())
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(format!("no disk {} of 4", missing.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_disks(tag: &str, which: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sf-disks-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for n in which {
            std::fs::write(
                dir.join(format!(
                    "Superfrog (1993)(Team 17)(Disk {n} of 4)[cr CSL].zip"
                )),
                b"",
            )
            .unwrap();
        }
        dir
    }

    #[test]
    fn honours_superfrog_disks() {
        let env = fake_disks("env", &[1, 2, 3, 4]);
        let other = fake_disks("other", &[1, 2, 3, 4]);
        assert_eq!(locate(Some(env.clone().into()), Ok(other)), Ok(env));
    }

    #[test]
    fn falls_back_to_main_checkout() {
        let checkout = fake_disks("checkout", &[1, 2, 3, 4]);
        assert_eq!(locate(None, Ok(checkout.clone())), Ok(checkout.clone()));
        let incomplete = fake_disks("incomplete", &[1, 3]);
        assert_eq!(
            locate(Some(incomplete.into()), Ok(checkout.clone())),
            Ok(checkout)
        );
    }

    #[test]
    fn errs_naming_both_places() {
        let a = fake_disks("err-a", &[1, 3]);
        let b = fake_disks("err-b", &[]);
        let e = locate(Some(a.clone().into()), Ok(b.clone())).unwrap_err();
        assert!(
            e.contains(&format!(
                "SUPERFROG_DISKS={}: no disk 2, 4 of 4",
                a.display()
            )),
            "{e}"
        );
        assert!(
            e.contains(&format!(
                "main checkout {}: no disk 1, 2, 3, 4 of 4",
                b.display()
            )),
            "{e}"
        );
        let e = locate(None, Err("git rev-parse failed: not a repo".into())).unwrap_err();
        assert!(
            e.contains("SUPERFROG_DISKS: not set")
                && e.contains("main checkout: git rev-parse failed"),
            "{e}"
        );
    }

    #[test]
    fn finds_the_real_disks() {
        let dir = disk_dir().unwrap_or_else(|e| panic!("{e}"));
        assert!(missing_disks(&dir).is_none());
    }
}
