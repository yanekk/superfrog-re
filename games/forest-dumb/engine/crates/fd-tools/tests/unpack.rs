//! The Rust `ESL!` decoder against `docs/08-packers.md`'s table, against `tools/unpack.py` (the game's
//! own routine under Musashi), and on damaged files. Needs the owner's zips; fails naming where it
//! looked when they are absent.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use amiga_disk::ofs::OfsImage;
use amiga_disk::sha1::sha1_hex;
use fd_tools::{disk_dir, game_file, read_adf, unpack, zip_paths};

/// Level 1's files, from `docs/LEVEL-INDEX.md`'s `W1-1` row.
const LEVEL_1: [&str; 5] = [
    "_Mapa1-1",
    "_Gen1-1",
    "_Mapa1-1.Mapa",
    "_Gen1-1.Mapa",
    "_MapaBobow1",
];

fn game() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
}

fn dir() -> PathBuf {
    disk_dir().unwrap_or_else(|e| panic!("{e}"))
}

/// Every file of disk `n` as it is stored, packed, with its name.
fn raw_disk(dir: &Path, n: usize) -> Vec<(String, Vec<u8>)> {
    let adf = read_adf(&zip_paths(dir).unwrap()[n - 1]).unwrap();
    let image = OfsImage::parse(&adf).unwrap();
    let names: Vec<String> = image.files().map(|(n, _)| n.to_string()).collect();
    names
        .into_iter()
        .map(|name| {
            let bytes = image.read_file(&name).unwrap();
            (name, bytes)
        })
        .collect()
}

/// `name → (unpacked size, SHA-1)` for every `ESL!` row of `docs/08-packers.md`'s table.
fn doc_table() -> Vec<(String, usize, String)> {
    let doc = std::fs::read_to_string(game().join("docs/08-packers.md")).unwrap();
    doc.lines()
        .filter(|l| l.contains("| ESL! (PowerPacker format) |"))
        .map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let name = cells[1].trim_matches('`').to_string();
            (name, cells[6].parse().unwrap(), cells[7].to_string())
        })
        .collect()
}

#[test]
fn every_esl_file_unpacks_to_the_sha1_in_the_doc() {
    let dir = dir();
    let table = doc_table();
    assert_eq!(table.len(), 46, "ESL! rows in docs/08-packers.md");
    let mut seen = 0;
    for n in 1..=2 {
        for (name, bytes) in raw_disk(&dir, n) {
            if !bytes.starts_with(b"ESL!") {
                continue;
            }
            seen += 1;
            let key = format!("disk{n}/{name}");
            let (_, size, sha) = table
                .iter()
                .find(|(k, _, _)| *k == key)
                .unwrap_or_else(|| panic!("{key} is packed but not in the doc's table"));
            let out = unpack(&bytes).unwrap_or_else(|e| panic!("{key}: {e}"));
            assert_eq!(out.len(), *size, "{key}");
            assert_eq!(&sha1_hex(&out), sha, "{key}");
        }
    }
    assert_eq!(seen, 46, "ESL! files on the disks");
}

#[test]
fn unpacked_files_pass_through_unchanged() {
    let dir = dir();
    for (name, bytes) in raw_disk(&dir, 2) {
        if !bytes.starts_with(b"ESL!") {
            assert_eq!(unpack(&bytes).unwrap(), bytes, "{name}");
        }
    }
    assert_eq!(unpack(b"").unwrap(), b"");
}

#[test]
fn the_crack_intro_is_refused_not_passed_through() {
    let dir = dir();
    let (_, bytes) = raw_disk(&dir, 1)
        .into_iter()
        .find(|(n, _)| n == "BillClintonToDupek")
        .unwrap();
    let err = unpack(&bytes).unwrap_err();
    assert!(err.contains("StoneCracker"), "{err}");
}

/// `tools/unpack.py` on level 1's files, through the repo's venv, with the main program (which holds
/// the routine it runs) handed over in a temp folder so no `build/` is needed.
#[test]
fn level_1_matches_unpack_py() {
    let dir = dir();
    let root = game().ancestors().nth(2).unwrap();
    let python = root.join("venv/bin/python");
    assert!(
        python.exists(),
        "{}: missing, run the plan's setup lines",
        python.display()
    );
    let tmp = std::env::temp_dir().join(format!("fd-unpack-py-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let disk1 = raw_disk(&dir, 1);
    let main = &disk1
        .iter()
        .find(|(n, _)| n == "FOREST_DUMB_FOREVER")
        .unwrap()
        .1;
    std::fs::write(tmp.join("main"), main).unwrap();
    let disk2 = raw_disk(&dir, 2);
    let mut args = vec![tmp.join("main")];
    for name in LEVEL_1 {
        let bytes = &disk2.iter().find(|(n, _)| n == name).unwrap().1;
        assert!(bytes.starts_with(b"ESL!"), "{name} is packed");
        std::fs::write(tmp.join(name), bytes).unwrap();
        args.push(tmp.join(name));
    }
    let script = "import sys; sys.path.insert(0, sys.argv[1]); import unpack; \
                  u = unpack.Unpacker(sys.argv[2]); \
                  [open(p + '.py', 'wb').write(u.unpack(open(p, 'rb').read())) for p in sys.argv[3:]]";
    let status = Command::new(&python)
        .arg("-c")
        .arg(script)
        .arg(game().join("tools"))
        .args(&args)
        .status()
        .unwrap();
    assert!(status.success(), "unpack.py failed: {status}");
    for name in LEVEL_1 {
        let ours = game_file(&dir, name).unwrap();
        let theirs = std::fs::read(tmp.join(format!("{name}.py"))).unwrap();
        assert!(ours == theirs, "{name}: Rust and unpack.py differ");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn level_1_reads_and_the_time_is_printed() {
    let dir = dir();
    let start = Instant::now();
    for name in LEVEL_1 {
        game_file(&dir, name).unwrap();
    }
    eprintln!(
        "level 1's five files through game_file: {:?}",
        start.elapsed()
    );
}

fn game_file_raw(dir: &Path, name: &str) -> Vec<u8> {
    raw_disk(dir, 2)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
        .1
}

#[test]
fn a_file_by_disk_and_name_and_a_missing_one() {
    let dir = dir();
    assert_eq!(
        game_file(&dir, "disk2/_Gen1-1").unwrap(),
        game_file(&dir, "_Gen1-1").unwrap()
    );
    assert_eq!(game_file(&dir, "disk1/FOREVER!").unwrap().len(), 20);

    let err = game_file(&dir, "_Mapa9-1").unwrap_err();
    assert!(
        err.contains("_Mapa9-1") && err.contains(&dir.display().to_string()),
        "{err}"
    );
    let err = game_file(&dir, "disk1/_Gen1-1").unwrap_err();
    assert!(
        err.contains("disk1/_Gen1-1") && err.contains("disk 1"),
        "{err}"
    );
    let err = game_file(Path::new("/nonexistent/fd"), "_Gen1-1").unwrap_err();
    assert!(err.contains("/nonexistent/fd"), "{err}");
}

/// Writes the format's stream, as `test_unpack.py`'s `Bits`: values top bit first, read from the end.
struct Bits(Vec<u8>);

impl Bits {
    fn put(&mut self, value: u32, count: u32) {
        for i in (0..count).rev() {
            self.0.push((value >> i & 1) as u8);
        }
    }

    fn pack(&self, size: usize) -> Vec<u8> {
        let skip = (32 - self.0.len() % 32) % 32;
        let bits: Vec<u8> = std::iter::repeat_n(0, skip)
            .chain(self.0.iter().copied())
            .collect();
        let longs: Vec<u32> = bits
            .chunks(32)
            .map(|c| c.iter().enumerate().map(|(i, &b)| (b as u32) << i).sum())
            .collect();
        let mut out = b"ESL!\x09\x09\x09\x09".to_vec();
        for l in longs.iter().rev() {
            out.extend(l.to_be_bytes());
        }
        out.extend(&(size as u32).to_be_bytes()[1..]);
        out.push(skip as u8);
        out
    }
}

fn literals(bits: &mut Bits, text: &[u8]) {
    bits.put(0, 1);
    let mut left = text.len() - 1;
    while left >= 3 {
        bits.put(3, 2);
        left -= 3;
    }
    bits.put(left as u32, 2);
    for &b in text.iter().rev() {
        bits.put(b as u32, 8);
    }
}

/// `test_unpack.py`'s `stream()`: literals, a short match, literals, a long match.
fn stream() -> (Vec<u8>, Vec<u8>) {
    let mut bits = Bits(Vec::new());
    literals(&mut bits, b"abcdefgh");
    bits.put(1, 2);
    bits.put(2, 9);
    literals(&mut bits, b"XY");
    bits.put(3, 2);
    bits.put(0, 1);
    bits.put(1, 7);
    bits.put(7, 3);
    bits.put(2, 3);
    let text = [&b"XY".repeat(8)[..], b"abc", b"abcdefgh"].concat();
    let packed = bits.pack(text.len());
    (text, packed)
}

#[test]
fn the_synthetic_stream_unpacks() {
    let (text, packed) = stream();
    assert_eq!(unpack(&packed).unwrap(), text);
}

#[test]
fn damaged_files_are_errors_not_panics() {
    let (_, p) = stream();
    let n = p.len();
    let with_size = |size: u32| [&p[..n - 4], &size.to_be_bytes()[1..], &p[n - 1..]].concat();
    let with_skip = |skip: u8| [&p[..n - 1], &[skip][..]].concat();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("magic alone", b"ESL!".to_vec()),
        ("no stream", b"ESL!\x09\x09\x09\x09".to_vec()),
        ("not whole longs", [&p[..], b"\0"].concat()),
        ("truncated: first long gone", [&p[..8], &p[12..]].concat()),
        (
            "truncated: cut at the end",
            [&p[..n - 8], &p[n - 4..]].concat(),
        ),
        ("an extra first long", [&p[..8], &[0; 4], &p[8..]].concat()),
        ("size zero", with_size(0)),
        ("size one more", with_size(28)),
        ("size one less", with_size(26)),
        ("size huge", with_size(0xff_ffff)),
        ("skip over a long", with_skip(33)),
        ("skip one less", with_skip(p[n - 1] - 1)),
        ("skip one more", with_skip(p[n - 1] + 1)),
        ("width zero", [&p[..4], &[0, 9, 9, 9], &p[8..]].concat()),
    ];
    for (name, data) in cases {
        let r = std::panic::catch_unwind(|| unpack(&data));
        match r {
            Ok(Err(_)) => {}
            Ok(Ok(out)) => panic!(
                "{name}: unpacked to {} bytes instead of an error",
                out.len()
            ),
            Err(_) => panic!("{name}: panicked"),
        }
    }
}

#[test]
fn damaged_real_files_are_errors() {
    let dir = dir();
    let p = game_file_raw(&dir, "_Gen1-1");
    let n = p.len();
    for (name, data) in [
        ("truncated", [&p[..n / 2], &p[n - 4..]].concat()),
        ("size off by 2", {
            let mut d = p.clone();
            d[n - 2] ^= 2;
            d
        }),
        ("leftover bits off by 1", {
            let mut d = p.clone();
            d[n - 1] ^= 1;
            d
        }),
        ("leftover bits 40", {
            let mut d = p.clone();
            d[n - 1] = 40;
            d
        }),
    ] {
        assert!(unpack(&data).is_err(), "{name}");
    }
}
