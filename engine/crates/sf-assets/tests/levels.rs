//! `decode` on the owner's real disks: every level round-trips, `GameData` is complete, the
//! catalogue is the game's own file table, and damaged chunks are refused by name.

mod common;

use common::{chunk, chunks, data, disks, named};
use sf_assets::*;
use sf_disk::carve;

/// Every T7MP file on the disks: 26 levels plus W6's second set of four maps, `d4_c16` among them
/// (a straggler, decoded with the sector after its carve).
#[test]
fn all_thirty_maps_decode_and_encode_back_byte_for_byte() {
    let mut n = 0;
    for (id, bytes) in chunks() {
        if bytes.starts_with(b"T7MP") {
            let t = t7mp_decode(bytes, *id, "map").unwrap_or_else(|e| panic!("{e}"));
            assert_eq!((t.width, t.height), (300, 84), "{id}");
            assert_eq!(t7mp_encode(&t), *bytes, "{id} does not encode back");
            n += 1;
        }
    }
    assert_eq!(n, 30);
}

#[test]
fn every_level_is_in_game_data_with_its_chunk() {
    let d = data();
    assert_eq!(d.levels.len(), 30);
    for (id, l) in &d.levels {
        assert_eq!(l.id, *id);
        assert_eq!(t7mp_encode(&l.t7mp), chunk(l.chunk), "{id}");
        assert!(l.extra.is_empty());
    }
    let at = |w, level, alt| {
        d.levels[&LevelId {
            world: w,
            level,
            alt,
        }]
            .chunk
            .to_string()
    };
    assert_eq!(at(World::W1, 1, false), "d1_c21");
    assert_eq!(at(World::W5, 4, false), "d2_c35");
    assert_eq!(at(World::W6, 1, false), "d4_c02");
    assert_eq!(at(World::W6, 3, true), "d4_c16");
    assert_eq!(at(World::Bonus, 1, false), "d2_c42");
    assert_eq!(at(World::Final, 1, false), "d4_c13");
    // W6's two sets: the first maps are the same file, the others differ in some cells.
    let w6 = |l, alt| {
        t7mp_encode(
            &d.levels[&LevelId {
                world: World::W6,
                level: l,
                alt,
            }]
                .t7mp,
        )
    };
    assert_eq!(w6(1, false), w6(1, true));
    for l in 2..=4 {
        let (a, b) = (w6(l, false), w6(l, true));
        let differ = a.iter().zip(&b).filter(|(x, y)| x != y).count();
        println!("W6-{l}: {differ} bytes differ from the second-drive copy");
        assert!(
            differ > 0 && differ < 1_000,
            "W6-{l}: {differ} bytes differ"
        );
    }
}

#[test]
fn game_data_is_complete() {
    let d = data();
    assert_eq!(d.worlds.len(), 8);
    for w in &d.worlds {
        let tiles = if w.world == World::Bonus { 640 } else { 840 };
        assert_eq!(w.tiles.tiles.len(), tiles, "{:?}", w.world);
        assert_eq!(
            w.masks.as_ref().map(|m| m.masks.len()),
            (w.world != World::Bonus).then_some(840)
        );
        assert_eq!(
            (w.bobs.sheet.width, w.bobs.sheet.height, w.bobs.sheet.depth),
            (320, 320, 5)
        );
        if w.world != World::Bonus {
            assert_eq!(w.palette.0[0], 0, "{:?}: colour 0 is black", w.world);
            // The game's grey ramp sits at 26..=29 of every world's palette (docs/06).
            assert_eq!(
                &w.palette.0[26..30],
                &[0x111, 0x222, 0x444, 0xaaa],
                "{:?}",
                w.world
            );
        }
    }
    assert_eq!(d.world(World::Final).palette, d.world(World::W6).palette);
    assert_eq!(d.sprites.len(), 2);
    assert_eq!(d.pictures.len(), 6 + 6 + 7 + 1);
    for (id, p) in &d.pictures {
        let b = &p.bitmap;
        assert!(b.pixels.iter().all(|&px| px < 1 << b.depth), "{id:?}");
        assert_eq!(
            b.pixels.len(),
            b.width as usize * b.height as usize,
            "{id:?}"
        );
    }
    let charset = &d.fonts[0].charset;
    assert_eq!(charset.len(), 40);
    assert!(
        (b'A'..=b'Z')
            .chain(b'0'..=b'9')
            .all(|c| charset.contains(&c))
    );
    for (id, frames) in [(AnimId::End1, 112), (AnimId::End2, 25)] {
        let a = &d.anims[&id];
        assert_eq!(a.frames.len(), frames, "{id:?}");
        for f in &a.frames {
            assert_eq!(
                (f.bitmap.width, f.bitmap.height, f.bitmap.depth),
                (320, 256, 5)
            );
        }
    }
    let ids: Vec<ExeId> = d.exes.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), 7, "{ids:?}");
    let game = d.exe(ExeId::Game);
    let sizes: Vec<(u32, bool)> = game.hunks.iter().map(|h| (h.size, h.present)).collect();
    assert_eq!(
        sizes,
        [
            (0x4834c, true),
            (0x352a0, true),
            (0x73fc, false),
            (0x77168, false)
        ]
    );
    for o in [ExeId::InsertDiskOverlay, ExeId::AccessingDiskOverlay] {
        assert!(!d.exe(o).hunks.is_empty(), "{o:?}");
    }
    assert_eq!(d.tables.file_table.len(), 95);
    assert_eq!(d.tables.disk_ids, [*b"2222", *b"3333", *b"4444", *b"5555"]);
    assert_eq!(d.tables.passwords.len(), 28);
    assert_eq!(d.tables.obj_marker_stubs.len(), 7);
    assert_eq!(d.tables.obj_templates.len(), 160);
    assert_eq!(d.tables.hiscores.len(), 10);
    assert!(
        d.tables
            .hiscores
            .windows(2)
            .all(|w| w[0].score > w[1].score)
    );
    for name in ["MUSC", "L1FX", "L1MU", "LBMU", "INFT", "intro-gfx-4"] {
        assert!(d.raw.contains_key(name), "{name}");
    }
    // The straggler W1 music, decompressed with its trailer sector.
    assert_eq!(d.raw["L1MU"].len(), 40_514);
}

/// The catalogue: every chunk on the disks named exactly once, the `file_table` names are the
/// game's own strings, and the entries land where docs/08 says.
#[test]
fn the_catalogue_names_every_chunk_by_the_game_file_table() {
    let cat = catalogue(chunks()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(cat.len(), 95 + 11);
    let mut ids: Vec<ChunkId> = cat.iter().map(|c| c.1).collect();
    ids.sort();
    ids.dedup();
    let carved: Vec<ChunkId> = chunks()
        .iter()
        .map(|c| c.0)
        .filter(|c| c.index < INTRO_SPAN_INDEX)
        .collect();
    assert_eq!(ids, carved);
    let game = chunk(named("game", 0));
    for (name, _) in &cat[..95] {
        let s = format!("SDDISK1:{name}\0");
        assert!(
            game.windows(s.len()).any(|w| w == s.as_bytes()),
            "{name} is not a name in the game"
        );
    }
    for (name, nth, id) in [
        ("END1", 0, "d3_c09"),
        ("L1BM", 0, "d1_c18"),
        ("L1LP", 0, "d1_c17"),
        ("L2ET", 0, "d4_c01"),
        ("L6MA1", 0, "d4_c14"),
        ("L6MA1", 1, "d4_c02"),
        ("L1FX", 1, "d2_c01"),
        ("LBLP", 0, "d2_c39"),
        ("L6BM", 0, "d4_c07"),
        ("intro-gfx-4", 0, "d3_c08"),
    ] {
        assert_eq!(named(name, nth).to_string(), id, "{name} #{nth}");
    }
    // Sectors are the carver's offsets.
    for (id, _) in chunks().iter().filter(|c| c.0.index < INTRO_SPAN_INDEX) {
        let c = carve(disks().disk(id.disk))
            .into_iter()
            .find(|c| c.index == id.index)
            .unwrap();
        assert_eq!(id.sector as u32 * 512, c.offset, "{id}");
    }
}

/// The story intro's four sector spans follow the carved chunks, raw: `d3_c05` and `d3_c06` are
/// one packed bank each, the last two several back to back (docs/13), and decode keeps them all.
#[test]
fn the_story_intro_s_sector_spans_are_kept_raw() {
    let spans: Vec<&(ChunkId, Vec<u8>)> = chunks()
        .iter()
        .filter(|c| c.0.index >= INTRO_SPAN_INDEX)
        .collect();
    assert_eq!(spans.len(), 4);
    for (k, ((id, bytes), sectors)) in spans.iter().zip([0x25, 0x1b, 0x10a, 0x110]).enumerate() {
        assert_eq!(
            (id.disk, bytes.len()),
            (sf_disk::DiskSlot::D3, sectors * 512),
            "{id}"
        );
        assert!(bytes.starts_with(b"ATN!"), "{id}");
        assert_eq!(data().raw[&format!("intro-span-{}", k + 1)], *bytes);
        // The first packed bank of each is the carved chunk at that sector.
        let carved = chunks()
            .iter()
            .find(|c| {
                c.0.index < INTRO_SPAN_INDEX && c.0.disk == id.disk && c.0.sector == id.sector
            })
            .unwrap();
        assert_eq!(sf_disk::decompress(bytes).unwrap(), carved.1, "{id}");
    }
    let packed = |b: &[u8]| b.windows(4).filter(|w| *w == b"ATN!").count();
    assert_eq!((packed(&spans[2].1), packed(&spans[3].1)), (6, 4));
}

/// The one-drive copies on disk 2 are the same files as on disk 1; decode reads disk 1's.
#[test]
fn second_copies_equal_the_first() {
    for name in ["L1FX", "FRUT", "FRUC", "FRUM", "LFFX", "LFMU"] {
        assert_eq!(chunk(named(name, 0)), chunk(named(name, 1)), "{name}");
    }
}

/// Every chunk `decode` reads (not those it keeps as bytes) cut short, and a few corrupted, gives
/// an `AssetError` naming that chunk; nothing panics.
#[test]
fn a_damaged_chunk_is_refused_by_name() {
    // Not read: what decode keeps as bytes, and the second copies on disk 2.
    // (Raw entries that are not files by the game's name, such as the pictures packed in the game,
    // are read from chunks already in the list.)
    let names: Vec<&str> = catalogue(chunks()).unwrap().iter().map(|c| c.0).collect();
    let mut unread: Vec<ChunkId> = data()
        .raw
        .keys()
        .filter(|n| names.contains(&n.as_str()))
        .map(|n| named(n, 0))
        .collect();
    unread.extend(["L1FX", "FRUT", "FRUC", "FRUM", "LFFX", "LFMU"].map(|n| named(n, 1)));
    let decodes_with = |id: ChunkId, bytes: Vec<u8>| {
        let mut set = chunks().to_vec();
        set.iter_mut().find(|c| c.0 == id).unwrap().1 = bytes;
        decode(&set)
    };
    let mut refused = 0;
    let cat = catalogue(chunks()).unwrap();
    for (name, id) in cat.into_iter().filter(|c| !unread.contains(&c.1)) {
        let full = chunk(id);
        let mut cuts = vec![0, 7, full.len() / 2, full.len() - 1];
        cuts.dedup();
        for cut in cuts {
            let e = decodes_with(id, full[..cut].to_vec())
                .err()
                .unwrap_or_else(|| panic!("{id} ({name}) cut to {cut} decoded"));
            assert_eq!(e.chunk, id, "{id} ({name}) cut to {cut}: {e}");
            refused += 1;
        }
    }
    assert!(refused > 200, "{refused}");

    // Missing.
    let set: Vec<_> = chunks()
        .iter()
        .filter(|c| c.0 != named("L3BM", 0))
        .cloned()
        .collect();
    let e = decode(&set).unwrap_err();
    assert_eq!((e.what.as_str(), &e.why), ("L3BM", &Why::Missing));
    assert_eq!(e.to_string(), "d2 sector 272 (L3BM): chunk missing");

    // A BODY one cell short, a T7MP size that lies, a hunk block nobody knows, an ANIM delta of
    // another kind.
    let lvl = named("L2MA3", 0);
    let mut b = chunk(lvl).to_vec();
    let body = b.windows(4).position(|w| w == b"BODY").unwrap();
    b[body + 4..body + 8].copy_from_slice(&50_398u32.to_be_bytes());
    b.truncate(b.len() - 2);
    assert_eq!(decodes_with(lvl, b).unwrap_err().chunk, lvl);
    let mut b = chunk(lvl).to_vec();
    b[7] ^= 1;
    assert_eq!(decodes_with(lvl, b).unwrap_err().chunk, lvl);
    let exe = named("intro", 0);
    let mut b = chunk(exe).to_vec();
    let at = b.len() - 4;
    b[at..].copy_from_slice(&0x3e7u32.to_be_bytes());
    assert_eq!(decodes_with(exe, b).unwrap_err().chunk, exe);
    let end = named("END2", 0);
    let mut b = chunk(end).to_vec();
    let anhd = b.windows(4).position(|w| w == b"ANHD").unwrap();
    b[anhd + 8] = 7;
    let e = decodes_with(end, b).unwrap_err();
    assert_eq!(e.chunk, end);
    assert!(e.to_string().contains("ANHD operation 7"), "{e}");
}

/// A later frame carrying a `BMHD` of another size is refused, not drawn into buffers sized for
/// the first frame (it panicked).
#[test]
fn an_anim_frame_resized_mid_file_is_refused() {
    let id = named("END2", 0);
    let b = chunk(id);
    let forms: Vec<usize> = b
        .windows(4)
        .enumerate()
        .filter(|(_, w)| *w == b"FORM")
        .map(|(i, _)| i)
        .collect();
    // The outer FORM ANIM, frame 0's FORM ILBM, then frame 1's.
    let f1 = forms[2];
    assert_eq!(&b[f1 + 8..f1 + 12], b"ILBM");
    let mut bmhd = [0u8; 28];
    bmhd[..4].copy_from_slice(b"BMHD");
    bmhd[4..8].copy_from_slice(&20u32.to_be_bytes());
    bmhd[8..10].copy_from_slice(&320u16.to_be_bytes());
    bmhd[10..12].copy_from_slice(&600u16.to_be_bytes());
    bmhd[16] = 5;
    bmhd[18] = 1;
    let mut c = b[..f1 + 12].to_vec();
    c.extend_from_slice(&bmhd);
    c.extend_from_slice(&b[f1 + 12..]);
    for at in [0, f1] {
        let len = u32::from_be_bytes(c[at + 4..at + 8].try_into().unwrap()) + 28;
        c[at + 4..at + 8].copy_from_slice(&len.to_be_bytes());
    }
    let e = decode_anim(AnimId::End2, id, &c).unwrap_err();
    assert_eq!(e.chunk, id);
    assert!(e.to_string().contains("BMHD"), "{e}");
}

/// The pure decoders on every prefix of a real file (sampled) never panic.
#[test]
fn decoders_never_panic_on_a_prefix() {
    let lvl = chunk(named("L1MA1", 0));
    for cut in (0..lvl.len()).step_by(97) {
        assert!(t7mp_decode(&lvl[..cut], named("L1MA1", 0), "map").is_err());
    }
    let anim = chunk(named("END2", 0));
    for cut in (0..anim.len()).step_by(4_999) {
        assert!(decode_anim(AnimId::End2, named("END2", 0), &anim[..cut]).is_err());
    }
    let exe = chunk(named("bootstrap", 0));
    for cut in 0..exe.len() / 4 {
        let _ = parse_exe(ExeId::Bootstrap, named("bootstrap", 0), &exe[..cut * 4]);
    }
}

/// The slot machine's tables and pictures (T71), as the game's code reads them: the ladder's ten
/// rungs are the first ten lamps, five pointer stops, 32 reel symbols on a 32×64 grid of FRUC, 26
/// lamps, attract patterns of 8 and 6 rows of 0/1 lamps; FRUT and FRUC six planes of 320×256,
/// FRUM one.
#[test]
fn the_slot_machine_tables_and_pictures() {
    let t = &data().tables;
    assert_eq!(
        t.slot_ladder,
        (0..10).map(|k| 0x6842 + 8 * k).collect::<Vec<u32>>()
    );
    assert_eq!(t.slot_pointer_stops, [0x26, 0x60, 0x90, 0xc0, 0x116]);
    assert_eq!(t.slot_reel_frames.len(), 32);
    assert_eq!(t.slot_reel_frames[0], ReelFrame { x: 0, y: 0 });
    assert_eq!(t.slot_reel_frames[11], ReelFrame { x: 0x20, y: 0x40 });
    assert!(
        t.slot_reel_frames
            .iter()
            .all(|f| f.x % 32 == 0 && f.y % 64 == 0)
    );
    assert_eq!(t.slot_lamp_rects.len(), SLOT_LAMPS);
    assert_eq!(
        t.slot_lamp_rects[0],
        LampRect {
            x: 0x110,
            y: 8,
            w: 0x30,
            h: 0x20,
            rest: [0, 0]
        }
    );
    assert!(
        t.slot_lamp_rects
            .iter()
            .all(|r| r.x + r.w <= 320 && r.y + r.h <= 256)
    );
    assert_eq!(t.slot_attract_first.len(), 8);
    assert_eq!(t.slot_attract_later.len(), 6);
    let rows = t.slot_attract_first.iter().chain(&t.slot_attract_later);
    assert!(rows.flatten().all(|&v| v <= 1));
    let d = data();
    for (name, len) in [("FRUT", 0xf000), ("FRUC", 0xf000), ("FRUM", 0x2800)] {
        assert_eq!(d.raw[name].len(), len, "{name}");
    }
}
