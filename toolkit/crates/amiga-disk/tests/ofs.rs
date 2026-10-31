//! The OFS reader on synthetic volumes, built block by block here so that nothing from a game's
//! disks enters this tree and the checks do not lean on the library that does the reading.

use amiga_disk::ofs::OfsImage;
use amiga_disk::{ADF_SIZE, ReadError};

const BLOCK: usize = 512;
const ROOT: usize = 880;
const PAYLOAD: usize = 488;
const TABLE: usize = 72;
const T_HEADER: u32 = 2;
const T_DATA: u32 = 8;
const T_LIST: u32 = 16;
const ST_ROOT: u32 = 1;
const ST_FILE: u32 = -3i32 as u32;
const ST_USERDIR: u32 = 2;

/// A volume under construction: blocks are handed out upward from the one after the root.
struct Builder {
    image: Vec<u8>,
    next: usize,
}

fn put(b: &mut [u8], long: usize, v: u32) {
    b[long * 4..long * 4 + 4].copy_from_slice(&v.to_be_bytes());
}

/// Longwords summing to zero, with the sum's slot at longword `at`.
fn seal(b: &mut [u8], at: usize) {
    put(b, at, 0);
    let sum = b.chunks(4).fold(0u32, |s, l| s.wrapping_add(u32::from_be_bytes(l.try_into().unwrap())));
    put(b, at, sum.wrapping_neg());
}

fn name_at_tail(b: &mut [u8], name: &[u8]) {
    b[432] = name.len() as u8;
    b[433..433 + name.len()].copy_from_slice(name);
}

/// AmigaDOS's directory hash for `DOS\0`: ASCII case folding only.
fn hash(name: &[u8]) -> usize {
    let h = name
        .iter()
        .fold(name.len() as u32, |h, c| (h * 13 + c.to_ascii_uppercase() as u32) & 0x7ff);
    h as usize % TABLE
}

impl Builder {
    fn new(volume: &[u8]) -> Self {
        let mut image = vec![0u8; ADF_SIZE];
        image[..4].copy_from_slice(&[b'D', b'O', b'S', 0]);
        put(&mut image[..BLOCK], 2, ROOT as u32);
        // The boot block's own checksum, carry-wrapping over its two blocks.
        let sum = image[..2 * BLOCK].chunks(4).fold(0u32, |s, l| {
            let (n, carry) = s.overflowing_add(u32::from_be_bytes(l.try_into().unwrap()));
            n + carry as u32
        });
        put(&mut image[..BLOCK], 1, !sum);
        let mut b = Builder { image, next: ROOT + 1 };
        let root = b.block(ROOT);
        put(root, 0, T_HEADER);
        put(root, 3, TABLE as u32);
        put(root, 78, -1i32 as u32); // the bitmap is valid
        name_at_tail(root, volume);
        put(root, 127, ST_ROOT);
        seal(root, 5);
        b
    }

    fn block(&mut self, n: usize) -> &mut [u8] {
        &mut self.image[n * BLOCK..(n + 1) * BLOCK]
    }

    fn alloc(&mut self) -> usize {
        self.next += 1;
        self.next - 1
    }

    /// Hook block `n` named `name` at the head of a hash chain of directory `dir`.
    fn link(&mut self, dir: usize, n: usize, name: &[u8]) {
        let slot = 6 + hash(name);
        let head = u32::from_be_bytes(self.block(dir)[slot * 4..slot * 4 + 4].try_into().unwrap());
        let b = self.block(n);
        name_at_tail(b, name);
        put(b, 124, head);
        put(b, 125, dir as u32);
        seal(b, 5);
        let d = self.block(dir);
        put(d, slot, n as u32);
        seal(d, 5);
    }

    /// A file of any length: its data blocks, as many extension blocks as they need, and its header
    /// block. Returns (header block, its data blocks in order).
    fn file(&mut self, name: &[u8], data: &[u8]) -> (usize, Vec<usize>) {
        self.file_in(ROOT, name, data)
    }

    fn file_in(&mut self, dir: usize, name: &[u8], data: &[u8]) -> (usize, Vec<usize>) {
        let header = self.alloc();
        let chunks: Vec<&[u8]> = data.chunks(PAYLOAD).collect();
        let blocks: Vec<usize> = chunks.iter().map(|_| self.alloc()).collect();
        for (i, (&n, chunk)) in blocks.iter().zip(&chunks).enumerate() {
            let next = blocks.get(i + 1).copied().unwrap_or(0) as u32;
            let b = self.block(n);
            put(b, 0, T_DATA);
            put(b, 1, header as u32);
            put(b, 2, i as u32 + 1);
            put(b, 3, chunk.len() as u32);
            put(b, 4, next);
            b[24..24 + chunk.len()].copy_from_slice(chunk);
            seal(b, 5);
        }
        // The pointer tables, first in the header and the rest in a chain of extension blocks. Each
        // table fills from its last slot down.
        let tables: Vec<&[usize]> = blocks.chunks(TABLE).collect();
        let extensions: Vec<usize> = tables.iter().skip(1).map(|_| self.alloc()).collect();
        for (i, &ext) in extensions.iter().enumerate() {
            let next = extensions.get(i + 1).copied().unwrap_or(0) as u32;
            let table = tables[i + 1];
            let b = self.block(ext);
            put(b, 0, T_LIST);
            put(b, 1, ext as u32);
            put(b, 2, table.len() as u32);
            for (k, &p) in table.iter().enumerate() {
                put(b, 77 - k, p as u32);
            }
            put(b, 125, header as u32);
            put(b, 126, next);
            put(b, 127, ST_FILE);
            seal(b, 5);
        }
        let first = tables.first().copied().unwrap_or(&[]);
        let h = self.block(header);
        put(h, 0, T_HEADER);
        put(h, 1, header as u32);
        put(h, 2, first.len() as u32);
        put(h, 4, blocks.first().copied().unwrap_or(0) as u32);
        for (k, &p) in first.iter().enumerate() {
            put(h, 77 - k, p as u32);
        }
        put(h, 81, data.len() as u32);
        put(h, 126, extensions.first().copied().unwrap_or(0) as u32);
        put(h, 127, ST_FILE);
        self.link(dir, header, name);
        (header, blocks)
    }

    fn directory(&mut self, name: &[u8]) -> usize {
        let n = self.alloc();
        let b = self.block(n);
        put(b, 0, T_HEADER);
        put(b, 1, n as u32);
        put(b, 127, ST_USERDIR);
        self.link(ROOT, n, name);
        n
    }
}

/// Bytes that differ from block to block and across a block's edge.
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8 ^ (i / PAYLOAD) as u8).collect()
}

fn malformed_at(e: ReadError) -> usize {
    match e {
        ReadError::Malformed { at, .. } => at,
        other => panic!("expected Malformed, got {other:?}"),
    }
}

#[test]
fn a_volume_lists_its_name_and_files_in_directory_order() {
    let names: [&[u8]; 4] = [b"alpha", b"Beta.dat", b"c", b"delta-four"];
    let mut b = Builder::new(b"SCRATCH");
    for (i, n) in names.iter().enumerate() {
        b.file(n, &pattern(100 * i));
    }
    let image = OfsImage::parse(&b.image).unwrap();
    assert_eq!(image.volume_name(), "SCRATCH");

    // Directory order is hash-slot order, independent of creation order.
    let mut want: Vec<(usize, String, u32)> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (hash(n), String::from_utf8(n.to_vec()).unwrap(), 100 * i as u32))
        .collect();
    want.sort();
    let got: Vec<(String, u32)> = image.files().map(|(n, s)| (n.to_string(), s)).collect();
    let want: Vec<(String, u32)> = want.into_iter().map(|(_, n, s)| (n, s)).collect();
    assert_eq!(got, want);
}

#[test]
fn two_names_on_one_hash_chain_both_list_and_read() {
    // Same length and a same-slot hash: `b"ab"` and a partner found by search.
    let partner = (b'a'..=b'z')
        .flat_map(|x| (b'a'..=b'z').map(move |y| vec![x, y]))
        .find(|n| n != b"ab" && hash(n) == hash(b"ab"))
        .unwrap();
    let mut b = Builder::new(b"V");
    b.file(b"ab", b"first");
    b.file(&partner, b"second");
    let image = OfsImage::parse(&b.image).unwrap();
    assert_eq!(image.files().count(), 2);
    assert_eq!(image.read_file("ab").unwrap(), b"first");
    assert_eq!(image.read_file(std::str::from_utf8(&partner).unwrap()).unwrap(), b"second");
}

#[test]
fn single_block_and_empty_files_read_exactly() {
    let mut b = Builder::new(b"V");
    b.file(b"one", b"hello");
    b.file(b"full", &pattern(PAYLOAD));
    b.file(b"none", b"");
    let image = OfsImage::parse(&b.image).unwrap();
    assert_eq!(image.read_file("one").unwrap(), b"hello");
    assert_eq!(image.read_file("full").unwrap(), pattern(PAYLOAD));
    assert_eq!(image.read_file("none").unwrap(), b"");
}

#[test]
fn a_file_across_extension_blocks_reads_exactly() {
    // 160 data blocks: 72 in the header, 72 in the first extension, 16 in the second.
    let data = pattern(160 * PAYLOAD - 17);
    let mut b = Builder::new(b"V");
    b.file(b"big", &data);
    b.file(b"after", b"still here");
    let image = OfsImage::parse(&b.image).unwrap();
    assert!(image.files().any(|f| f == ("big", data.len() as u32)));
    assert_eq!(image.read_file("big").unwrap(), data);
    assert_eq!(image.read_file("after").unwrap(), b"still here");
}

#[test]
fn a_marker_style_name_lists_and_reads() {
    let mut b = Builder::new(b"V");
    b.file(b"!SOME_MARKER!", b"x");
    let image = OfsImage::parse(&b.image).unwrap();
    assert_eq!(image.files().collect::<Vec<_>>(), [("!SOME_MARKER!", 1)]);
    assert_eq!(image.read_file("!SOME_MARKER!").unwrap(), b"x");
}

#[test]
fn lookup_ignores_case_as_dos_library_does() {
    let mut b = Builder::new(b"V");
    b.file(b"Mixed_Case.Dat", b"found");
    let image = OfsImage::parse(&b.image).unwrap();
    for name in ["Mixed_Case.Dat", "mixed_case.dat", "MIXED_CASE.DAT"] {
        assert_eq!(image.read_file(name).unwrap(), b"found", "{name}");
    }
    // The listing keeps the name as it is on the disk.
    assert_eq!(image.files().next().unwrap().0, "Mixed_Case.Dat");
}

#[test]
fn a_latin1_name_lists_as_its_characters_and_reads() {
    let mut b = Builder::new(b"V");
    b.file(b"caf\xe9", b"x");
    let image = OfsImage::parse(&b.image).unwrap();
    assert_eq!(image.files().next().unwrap().0, "café");
    assert_eq!(image.read_file("café").unwrap(), b"x");
}

#[test]
fn files_in_subdirectories_list_by_path_and_read() {
    let mut b = Builder::new(b"V");
    let dir = b.directory(b"s");
    b.file_in(dir, b"Startup-Sequence", b"boot");
    let deeper = b.alloc();
    let d = b.block(deeper);
    put(d, 0, T_HEADER);
    put(d, 1, deeper as u32);
    put(d, 127, ST_USERDIR);
    b.link(dir, deeper, b"deep");
    b.file_in(deeper, b"leaf", b"low");
    b.file(b"plain", b"x");
    let image = OfsImage::parse(&b.image).unwrap();

    let mut listed: Vec<(&str, u32)> = image.files().collect();
    listed.sort();
    assert_eq!(listed, [("plain", 1), ("s/Startup-Sequence", 4), ("s/deep/leaf", 3)]);
    assert_eq!(image.read_file("s/Startup-Sequence").unwrap(), b"boot");
    assert_eq!(image.read_file("S/STARTUP-SEQUENCE").unwrap(), b"boot");
    assert_eq!(image.read_file("s/deep/leaf").unwrap(), b"low");
    // A directory is not a file, and a name alone does not reach into one.
    assert_eq!(malformed_at(image.read_file("s").unwrap_err()), dir * BLOCK);
    assert_eq!(malformed_at(image.read_file("leaf").unwrap_err()), ROOT * BLOCK);
}

#[test]
fn a_directory_that_loops_back_fails_instead_of_recursing() {
    let mut b = Builder::new(b"V");
    let dir = b.directory(b"a");
    // The directory's own hash table names the directory itself.
    let d = b.block(dir);
    put(d, 6 + hash(b"a"), dir as u32);
    seal(d, 5);
    assert_eq!(malformed_at(OfsImage::parse(&b.image).unwrap_err()), dir * BLOCK);
}

#[test]
fn a_missing_file_is_named_at_the_root_block() {
    let image_bytes = Builder::new(b"V").image;
    let image = OfsImage::parse(&image_bytes).unwrap();
    let long = "n".repeat(31);
    for name in ["absent", "€", long.as_str()] {
        let e = image.read_file(name).unwrap_err();
        assert!(e.to_string().contains("no file named"), "{e}");
        assert_eq!(malformed_at(e), ROOT * BLOCK);
    }
}

#[test]
fn a_bad_checksum_names_the_block_it_is_on() {
    let mut b = Builder::new(b"V");
    let (header, blocks) = b.file(b"f", &pattern(3 * PAYLOAD));

    // A flipped payload byte: the listing is fine, the read refuses that data block.
    let mut bad = b.image.clone();
    bad[blocks[1] * BLOCK + 100] ^= 1;
    let image = OfsImage::parse(&bad).unwrap();
    assert_eq!(malformed_at(image.read_file("f").unwrap_err()), blocks[1] * BLOCK);

    // A flipped header byte: the listing itself walks it.
    let mut bad = b.image.clone();
    bad[header * BLOCK + 300] ^= 1;
    assert_eq!(malformed_at(OfsImage::parse(&bad).unwrap_err()), header * BLOCK);

    // A flipped root byte.
    let mut bad = b.image.clone();
    bad[ROOT * BLOCK + 40] ^= 1;
    let e = OfsImage::parse(&bad).unwrap_err();
    assert!(e.to_string().contains("checksum"), "{e}");
    assert_eq!(malformed_at(e), ROOT * BLOCK);
}

#[test]
fn a_hash_chain_pointing_past_the_end_fails_with_an_offset() {
    let mut b = Builder::new(b"V");
    let slot = 6 + hash(b"x");
    let root = b.block(ROOT);
    put(root, slot, 5000);
    seal(root, 5);
    assert_eq!(malformed_at(OfsImage::parse(&b.image).unwrap_err()), 5000 * BLOCK);
}

#[test]
fn a_wrong_sized_image_fails_with_the_offset() {
    let image = Builder::new(b"V").image;
    assert_eq!(
        OfsImage::parse(&image[..1000]).unwrap_err(),
        ReadError::Truncated { at: 1000 }
    );
    assert_eq!(OfsImage::parse(&[]).unwrap_err(), ReadError::Truncated { at: 0 });
    let mut long = image.clone();
    long.push(0);
    assert_eq!(malformed_at(OfsImage::parse(&long).unwrap_err()), ADF_SIZE);
}

#[test]
fn a_dos_type_other_than_ofs_fails_at_byte_zero() {
    for last in [1u8, 2, 3, 7] {
        let mut image = Builder::new(b"V").image;
        image[3] = last;
        let e = OfsImage::parse(&image).unwrap_err();
        assert!(e.to_string().contains("not OFS"), "{e}");
        assert_eq!(malformed_at(e), 0);
    }
    let mut image = Builder::new(b"V").image;
    image[..4].copy_from_slice(b"KICK");
    assert_eq!(malformed_at(OfsImage::parse(&image).unwrap_err()), 0);
}

#[test]
fn a_disk_that_does_not_boot_still_reads() {
    let mut b = Builder::new(b"V");
    b.file(b"f", b"x");
    put(&mut b.image[..BLOCK], 1, 0); // no boot checksum, as a data disk may have
    assert_eq!(OfsImage::parse(&b.image).unwrap().read_file("f").unwrap(), b"x");
}
