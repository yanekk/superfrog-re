//! Find the four Superfrog disks among the files the player offered (DESIGN §2.2).
//!
//! Each offered file is a zip (by its name or its first bytes) or else taken as a raw ADF. Inside a
//! zip every `.adf` entry is a candidate, whatever folder it sits in; other files are ignored. A
//! candidate is a Superfrog disk only if its SHA-1 is one of the four below. Every problem is
//! collected, so the player hears about all of them at once.

use std::fmt;

use crate::{ADF_SIZE, Adf, DiskSlot, sha1_hex, zip};

/// SHA-1 of each accepted ADF, the "[cr CSL]" set the owner has. A digest cannot reproduce the
/// disk, so it is not disk data (DESIGN §2.3). Also recorded in docs/01-disk-format.md.
const KNOWN: [(DiskSlot, &str); 4] = [
    (DiskSlot::D1, "14f177b59787c3c8a667e6c9ac78ce02874f59f7"),
    (DiskSlot::D2, "c5528eac0c09fed318cb25fa8d75069b780e44f0"),
    (DiskSlot::D3, "99ac304d5665991d7f9b2e792e7e8d2762ff5aa4"),
    (DiskSlot::D4, "f5ef23973fb5d5822224738aceda0091e795382c"),
];

/// A zip entry larger than this is refused before it is inflated.
const MAX_ENTRY: usize = 4 * ADF_SIZE;

/// A file the shell read on the player's behalf.
#[derive(Clone, PartialEq, Eq)]
pub struct Offered {
    /// The file's name as the player knows it (the shell passes the file name or path).
    pub name: String,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for Offered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Offered")
            .field("name", &self.name)
            .field("len", &self.bytes.len())
            .finish()
    }
}

/// The four disks, `disks[slot.index()]` holding that slot's disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskSet {
    pub disks: [Adf; 4],
}

impl DiskSet {
    pub fn disk(&self, slot: DiskSlot) -> &Adf {
        &self.disks[slot.index()]
    }
}

/// Why the offered files do not make a disk set. `Display` gives the message the player sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// No offered file is this disk.
    Missing(DiskSlot),
    /// A disk image that is none of the four: another version, another game, or a damaged copy.
    Unknown { name: String, sha1: String },
    /// A file that could not be read as a disk image at all.
    Damaged { name: String, why: String },
    /// This disk was offered more than once.
    Duplicate(DiskSlot),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Missing(slot) => write!(
                f,
                "Superfrog {slot} is missing. Choose its disk image too, zipped or as an .adf file."
            ),
            ImportError::Unknown { name, sha1 } => write!(
                f,
                "“{name}” is not one of the four Superfrog disks this game accepts. It may be a \
                 different version or a damaged copy (SHA-1 {sha1})."
            ),
            ImportError::Damaged { name, why } => {
                write!(f, "“{name}” could not be read as a disk image: {why}.")
            }
            ImportError::Duplicate(slot) => write!(
                f,
                "Superfrog {slot} was chosen more than once. Choose each disk once."
            ),
        }
    }
}

/// The slot whose accepted ADF has this SHA-1.
pub fn known_sha1(slot: DiskSlot) -> &'static str {
    KNOWN[slot.index()].1
}

/// Find all four disks in `offered`, or say everything that is wrong.
pub fn identify(offered: &[Offered]) -> Result<DiskSet, Vec<ImportError>> {
    let mut found: [Option<Adf>; 4] = [None, None, None, None];
    let mut duplicated = [false; 4];
    let mut errors = Vec::new();
    for file in offered {
        let candidates = match candidates(file) {
            Ok(c) => c,
            Err(why) => {
                errors.push(ImportError::Damaged {
                    name: file.name.clone(),
                    why,
                });
                continue;
            }
        };
        for (name, bytes) in candidates {
            if bytes.len() != ADF_SIZE {
                let why = format!(
                    "it is {} bytes, and an Amiga disk image is {ADF_SIZE} bytes",
                    bytes.len()
                );
                errors.push(ImportError::Damaged { name, why });
                continue;
            }
            let sha1 = sha1_hex(&bytes);
            let Some(&(slot, _)) = KNOWN.iter().find(|(_, s)| *s == sha1) else {
                errors.push(ImportError::Unknown { name, sha1 });
                continue;
            };
            let i = slot.index();
            if found[i].is_some() {
                if !duplicated[i] {
                    duplicated[i] = true;
                    errors.push(ImportError::Duplicate(slot));
                }
                continue;
            }
            found[i] = Some(Adf {
                slot,
                source: file.name.clone(),
                bytes,
            });
        }
    }
    for slot in DiskSlot::ALL {
        if found[slot.index()].is_none() {
            errors.push(ImportError::Missing(slot));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let [Some(d1), Some(d2), Some(d3), Some(d4)] = found else {
        unreachable!("every empty slot was reported missing");
    };
    Ok(DiskSet {
        disks: [d1, d2, d3, d4],
    })
}

/// The disk images in one offered file, each with the name to report it by.
fn candidates(file: &Offered) -> Result<Vec<(String, Vec<u8>)>, String> {
    let named_zip = file.name.to_lowercase().ends_with(".zip");
    if !named_zip && !zip::is_zip(&file.bytes) {
        return Ok(vec![(file.name.clone(), file.bytes.clone())]);
    }
    let entries = zip::read_entries(&file.bytes, is_disk_entry, MAX_ENTRY)?;
    if entries.is_empty() {
        return Err("the zip holds no .adf disk image".into());
    }
    Ok(entries
        .into_iter()
        .map(|e| (format!("{} in {}", e.name, file.name), e.bytes))
        .collect())
}

/// An `.adf` entry, but not the `__MACOSX/._x.adf` resource forks macOS adds to zips it makes.
fn is_disk_entry(name: &str) -> bool {
    let base = name.rsplit('/').next().unwrap_or(name);
    name.to_lowercase().ends_with(".adf")
        && !base.starts_with("._")
        && !name.starts_with("__MACOSX/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(name: &str, bytes: Vec<u8>) -> Offered {
        Offered {
            name: name.into(),
            bytes,
        }
    }

    #[test]
    fn nothing_offered_is_four_missing_disks() {
        let e = identify(&[]).unwrap_err();
        assert_eq!(e, DiskSlot::ALL.map(ImportError::Missing).to_vec());
    }

    #[test]
    fn a_wrong_sized_file_is_damaged_and_an_unknown_disk_is_named_by_its_sha1() {
        let e = identify(&[
            offer("short.adf", vec![0; 1000]),
            offer("blank.adf", vec![0; ADF_SIZE]),
        ])
        .unwrap_err();
        assert!(matches!(&e[0], ImportError::Damaged { name, why }
            if name == "short.adf" && why.contains("1000 bytes")));
        assert_eq!(
            e[1],
            ImportError::Unknown {
                name: "blank.adf".into(),
                sha1: sha1_hex(&vec![0; ADF_SIZE])
            }
        );
        assert_eq!(e.len(), 6, "plus four missing: {e:?}");
    }

    #[test]
    fn a_zip_by_name_that_is_not_a_zip_is_damaged() {
        let e = identify(&[offer("Disk 1.zip", b"hello".to_vec())]).unwrap_err();
        assert!(matches!(&e[0], ImportError::Damaged { name, why }
            if name == "Disk 1.zip" && why.contains("not a zip")));
    }

    #[test]
    fn disk_entries_skip_mac_resource_forks() {
        assert!(is_disk_entry("a/b/Disk 1.ADF"));
        assert!(!is_disk_entry("readme.txt"));
        assert!(!is_disk_entry("__MACOSX/a/._Disk 1.adf"));
        assert!(!is_disk_entry("a/._Disk 1.adf"));
    }

    #[test]
    fn every_message_names_the_slot_or_the_file() {
        let cases = [
            (ImportError::Missing(DiskSlot::D3), "Disk 3 of 4"),
            (ImportError::Duplicate(DiskSlot::D2), "Disk 2 of 4"),
            (
                ImportError::Unknown {
                    name: "x.adf".into(),
                    sha1: "abc".into(),
                },
                "“x.adf”",
            ),
            (
                ImportError::Damaged {
                    name: "y.zip".into(),
                    why: "it broke".into(),
                },
                "“y.zip”",
            ),
        ];
        for (e, want) in cases {
            let msg = e.to_string();
            assert!(msg.contains(want), "{msg}");
        }
    }

    #[test]
    fn known_sha1s_are_in_slot_order() {
        for slot in DiskSlot::ALL {
            assert_eq!(KNOWN[slot.index()].0, slot);
            assert_eq!(known_sha1(slot).len(), 40);
        }
    }
}
