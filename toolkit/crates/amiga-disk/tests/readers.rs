//! The readers on synthetic files: a minimal hunk executable, small ILBMs (raw and ByteRun1), a
//! two-frame ANIM, and files cut short, which must report the byte offset they stop at.

use amiga_disk::ReadError;
use amiga_disk::anim::anim;
use amiga_disk::hunk::{self, Block, Blocks, HunkKind, Reloc};
use amiga_disk::iff::{byterun1, chunks, ilbm};
use amiga_disk::palette::{Palette, rgb888};
use amiga_disk::planar::Bitmap;

fn longs(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|l| l.to_be_bytes()).collect()
}

/// Two hunks: code (2 longs, chip memory, one relocation to hunk 1) and data (1 long).
fn exe() -> Vec<u8> {
    longs(&[
        0x3f3, 0, 2, 0, 1, // header: no libraries, 2 hunks 0..1
        0x4000_0002, 1, // sizes: hunk 0 in chip memory
        0x3e9, 2, 0x1111_1111, 0, // code
        0x3ec, 1, 1, 4, 0, // reloc32: one offset into hunk 1
        0x3f2, // end
        0x3ea, 1, 0x2222_2222, // data
        0x3f2,
    ])
}

#[test]
fn hunk_file_parses_code_data_and_relocations() {
    let hunks = hunk::parse(&exe()).unwrap();
    assert_eq!(hunks.len(), 2);
    let (code, data) = (&hunks[0], &hunks[1]);
    assert_eq!((code.kind, code.mem, code.size, code.present), (HunkKind::Code, 1, 8, true));
    assert_eq!(code.data, longs(&[0x1111_1111, 0]));
    assert_eq!(code.relocs, vec![Reloc { target: 1, offsets: vec![4] }]);
    assert_eq!((data.kind, data.mem, data.size, data.present), (HunkKind::Data, 0, 4, true));
    assert_eq!(data.data, longs(&[0x2222_2222]));
    assert!(data.relocs.is_empty());
}

#[test]
fn header_and_block_walk() {
    let bytes = exe();
    let h = hunk::header(&bytes).unwrap();
    assert_eq!((h.count, h.first, h.last, h.table_at, h.end), (2, 0, 1, 8, 28));
    assert_eq!(h.sizes[0].raw, 0x4000_0002);
    assert_eq!((h.sizes[0].mem, h.sizes[0].bytes), (1, 8));
    let walk = Blocks::new(&bytes, h.end);
    assert_eq!(walk.peek().unwrap().unwrap(), (28, 0x3e9));
    let blocks: Vec<_> = walk.map(Result::unwrap).collect();
    let kinds: Vec<_> = blocks.iter().map(|(at, b)| (*at, b.kind())).collect();
    assert_eq!(kinds, vec![(28, 0x3e9), (44, 0x3ec), (64, 0x3f2), (68, 0x3ea), (80, 0x3f2)]);
    assert_eq!(blocks[1].1, Block::Reloc32(vec![(1, vec![4])]));
}

#[test]
fn absent_hunk_is_kept_sized_and_names_symbols_debug_are_stepped_over() {
    let bytes = longs(&[
        0x3f3, 0, 2, 0, 1, 3, 5, // hunk 1 is declared and never carried
        0x3e8, 1, 0x4e41_4d45, // HUNK_NAME
        0x3e9, 1, 7, //
        0x3f0, 1, 0x5359_4d00, 0, 0, // HUNK_SYMBOL: one entry, then 0
        0x3f1, 1, 9, // HUNK_DEBUG
        0x3f2,
    ]);
    let hunks = hunk::parse(&bytes).unwrap();
    assert_eq!(hunks[0].data, longs(&[7]));
    assert_eq!((hunks[1].kind, hunks[1].size, hunks[1].present), (HunkKind::Bss, 20, false));
    assert!(hunks[1].data.is_empty());
}

#[test]
fn hunk_file_cut_short_reports_its_offset() {
    let bytes = exe();
    // Cut inside the code hunk's contents, which start at byte 36.
    assert_eq!(hunk::parse(&bytes[..40]), Err(ReadError::Truncated { at: 36 }));
    // Cut inside the size table.
    assert_eq!(hunk::parse(&bytes[..22]), Err(ReadError::Truncated { at: 20 }));
}

#[test]
fn hunk_file_refusals() {
    assert!(matches!(hunk::parse(&longs(&[0x3f2])), Err(ReadError::Malformed { at: 0, .. })));
    let mut bad = exe();
    bad[28..32].copy_from_slice(&0x3efu32.to_be_bytes()); // HUNK_EXT: not measured
    let err = hunk::parse(&bad).unwrap_err();
    assert_eq!(err, ReadError::Malformed { at: 28, what: "unknown hunk block 0x3ef at byte 28".into() });
    let mut walk = Blocks::new(&bad, 28);
    assert!(walk.next().unwrap().is_err());
    assert!(walk.next().is_none(), "the walk ends after an error");
    let mut reloc = exe();
    reloc[52..56].copy_from_slice(&2u32.to_be_bytes()); // relocation to hunk 2 of 2
    assert!(matches!(hunk::parse(&reloc), Err(ReadError::Malformed { at: 44, .. })));
    // A table that does not start at 0 is the header's to report, the hunk list's to refuse.
    let first1 = longs(&[0x3f3, 0, 1, 1, 1, 1]);
    assert!(hunk::header(&first1).is_ok());
    assert!(matches!(hunk::parse(&first1), Err(ReadError::Malformed { at: 8, .. })));
}

fn iff(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = tag.to_vec();
    out.extend((payload.len() as u32).to_be_bytes());
    out.extend(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    out
}

fn bmhd(w: u16, h: u16, depth: u8, compression: u8) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b.extend([0, 0, 0, 0, depth, 0, compression, 0, 0, 0, 10, 11]);
    b.extend(w.to_be_bytes());
    b.extend(h.to_be_bytes());
    b
}

fn form(kind: &[u8; 4], parts: &[Vec<u8>]) -> Vec<u8> {
    let mut p = kind.to_vec();
    for part in parts {
        p.extend(part);
    }
    iff(b"FORM", &p)
}

/// 16×2, 2 planes. Row 0: plane 0 `FF00`, plane 1 `0F0F`; row 1: plane 0 `0000`, plane 1 `FFFF`.
const ROWS: [[u8; 2]; 4] = [[0xff, 0x00], [0x0f, 0x0f], [0x00, 0x00], [0xff, 0xff]];
const CMAP: [u8; 12] = [0, 0, 0, 0xf0, 0x80, 0x10, 0x20, 0x30, 0x40, 0xff, 0xff, 0xff];

fn expected() -> Bitmap {
    let planar: Vec<u8> = [ROWS[0], ROWS[2], ROWS[1], ROWS[3]].concat();
    Bitmap::from_planar(&planar, 16, 2, 2).unwrap()
}

fn ilbm_file(compression: u8, body: &[u8]) -> Vec<u8> {
    form(
        b"ILBM",
        &[
            iff(b"BMHD", &bmhd(16, 2, 2, compression)),
            iff(b"CMAP", &CMAP),
            iff(b"CAMG", &[0, 0, 0x80, 0]),
            iff(b"BODY", body),
        ],
    )
}

#[test]
fn ilbm_two_planes_uncompressed() {
    let pic = ilbm(&ilbm_file(0, &ROWS.concat())).unwrap();
    assert_eq!(pic.bitmap, expected());
    // Pixel 0: plane 0 set, plane 1 clear; pixel 4: both; pixel 8: plane 1 only on row 0.
    assert_eq!((pic.bitmap.pixel(0, 0), pic.bitmap.pixel(4, 0), pic.bitmap.pixel(8, 0)), (1, 0b11, 0));
    assert_eq!(pic.bitmap.pixel(12, 0), 0b10);
    assert_eq!(pic.bitmap.pixel(0, 1), 0b10);
    let pal = pic.palette.unwrap();
    assert_eq!(&pal.0[..4], &[0x000, 0xf81, 0x234, 0xfff]);
    assert_eq!(pal.rgb888(1), [0xff, 0x88, 0x11]);
    assert_eq!(pic.chunks, vec![(*b"CAMG", vec![0, 0, 0x80, 0])]);
}

#[test]
fn ilbm_two_planes_byterun1() {
    // Row 0 plane 0 literal, plane 1 literal, row 1 plane 0 a run of 2, plane 1 a run of 2 with a
    // no-op (-128) between.
    let body = [1, 0xff, 0x00, 1, 0x0f, 0x0f, 0xff, 0x00, 0x80, 0xff, 0xff];
    assert_eq!(ilbm(&ilbm_file(1, &body)).unwrap().bitmap, expected());
}

#[test]
fn ilbm_cut_short_reports_file_offsets() {
    let file = ilbm_file(0, &ROWS.concat());
    // The FORM claims more than is left: its payload starts at byte 8.
    assert_eq!(ilbm(&file[..30]).unwrap_err(), ReadError::Truncated { at: 8 });
    // A ByteRun1 literal of 2 with one byte left: the offset is the byte in the file, not in BODY.
    let file = ilbm_file(1, &[1, 0xff]);
    let body_at = file.len() - 2;
    assert_eq!(ilbm(&file).unwrap_err(), ReadError::Truncated { at: body_at + 1 });
}

#[test]
fn ilbm_refusals() {
    assert!(matches!(ilbm(&form(b"ANIM", &[])), Err(ReadError::Malformed { at: 0, .. })));
    assert!(matches!(ilbm(&ilbm_file(2, &ROWS.concat())), Err(ReadError::Malformed { .. })));
    let odd = form(b"ILBM", &[iff(b"BMHD", &bmhd(12, 2, 2, 0)), iff(b"BODY", &[0; 8])]);
    let err = ilbm(&odd).unwrap_err();
    assert!(matches!(&err, ReadError::Malformed { what, .. } if what == "BMHD 12x2x2"), "{err}");
}

#[test]
fn byterun1_and_chunk_walk() {
    assert_eq!(byterun1(&[0xfd, 7, 0, 9], 5).unwrap(), vec![7, 7, 7, 7, 9]);
    assert!(matches!(byterun1(&[0xfd, 7], 3), Err(ReadError::Malformed { .. })));
    let two = [iff(b"AAAA", &[1, 2, 3]), iff(b"BBBB", &[])].concat();
    assert_eq!(chunks(&two, 0).unwrap(), vec![(*b"AAAA", &[1u8, 2, 3][..]), (*b"BBBB", &[][..])]);
}

fn anhd(reltime: u32) -> Vec<u8> {
    let mut a = vec![5, 0];
    a.extend([0; 12]);
    a.extend(reltime.to_be_bytes());
    a.extend([0, 0]); // interleave 0
    a.extend([0, 0]);
    a.extend([0; 4]); // bits
    a.extend([0; 16]);
    a
}

#[test]
fn anim_first_frame_and_op5_delta() {
    let body = [1, 0xff, 0x00, 1, 0x0f, 0x0f, 0xff, 0x00, 0xff, 0xff];
    let frame0 = form(
        b"ILBM",
        &[iff(b"BMHD", &bmhd(16, 2, 2, 1)), iff(b"CMAP", &CMAP), iff(b"DPAN", &[1, 2]), iff(b"BODY", &body)],
    );
    // Plane 0, column 0: skip a row, then copy 0xAA into row 1. Column 1: no ops.
    let mut dlta = longs(&[64]);
    dlta.extend([0; 60]);
    dlta.extend([2, 1, 0x81, 0xaa, 0]);
    let frame1 = form(b"ILBM", &[iff(b"ANHD", &anhd(3)), iff(b"DLTA", &dlta)]);
    // Frame 2 lands in frame 0's buffer: a run of 2 rows of 0x01 into plane 1, column 1.
    let mut dlta = longs(&[0, 64]);
    dlta.extend([0; 56]);
    dlta.extend([0, 1, 0, 2, 0x01]);
    let frame2 = form(b"ILBM", &[iff(b"ANHD", &anhd(4)), iff(b"DLTA", &dlta)]);
    let file = form(b"ANIM", &[frame0, frame1, frame2]);

    let a = anim(&file).unwrap();
    assert_eq!(a.frames.len(), 3);
    assert_eq!((a.frames[0].bitmap.clone(), a.frames[0].reltime), (expected(), 0));
    assert_eq!(&a.palette.0[..4], &[0x000, 0xf81, 0x234, 0xfff]);
    assert_eq!(a.chunks, vec![(*b"DPAN", vec![1, 2])]);

    let mut want1 = [ROWS[0], [0xaa, 0x00], ROWS[1], ROWS[3]].concat();
    assert_eq!(a.frames[1].bitmap, Bitmap::from_planar(&want1, 16, 2, 2).unwrap());
    assert_eq!(a.frames[1].reltime, 3);
    want1 = [ROWS[0], ROWS[2], [0x0f, 0x01], [0xff, 0x01]].concat();
    assert_eq!(a.frames[2].bitmap, Bitmap::from_planar(&want1, 16, 2, 2).unwrap());
    assert_eq!(a.frames[2].reltime, 4);

    // Cut inside frame 1's DLTA: the FORM ANIM's payload no longer fits.
    assert_eq!(anim(&file[..file.len() - 30]).unwrap_err(), ReadError::Truncated { at: 8 });
}

#[test]
fn planar_round_trip_and_palette() {
    let b = expected();
    assert_eq!(Bitmap::from_planar(&b.to_planar(), 16, 2, 2).unwrap(), b);
    assert_eq!(Bitmap::planar_len(16, 2, 2), 8);
    assert!(Bitmap::from_planar(&[0; 7], 16, 2, 2).is_none());
    let words = longs(&[0x0fff_0000; 16]);
    let p = Palette::read(&words, 0).unwrap();
    assert_eq!((p.0[0], p.0[1]), (0x0fff, 0));
    assert_eq!(Palette::read(&words, 2), Err(ReadError::Truncated { at: 64 }));
    assert_eq!(rgb888(0x0f80), [0xff, 0x88, 0]);
}
