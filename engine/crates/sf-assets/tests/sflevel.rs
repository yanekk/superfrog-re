//! `.sflevel` (DESIGN §2.10, docs/14) on the owner's real levels: every level survives the file,
//! an unchanged export plays as the disk level does, an edit plays as edited, and every way a file
//! can be refused is refused with its reason.

mod common;

use std::path::Path;

use common::data;
use sf_assets::*;
use sf_conform::script::InputScript;
use sf_core::{Game, Start, VarId};

fn script(name: &str) -> InputScript {
    InputScript::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../conform/scripts/{name}.txt")))
        .unwrap_or_else(|e| panic!("{e}"))
}

fn w1_1() -> LevelId {
    "W1-1".parse().unwrap()
}

#[test]
fn every_level_survives_the_file() {
    let d = data();
    assert_eq!(d.levels.len(), 30);
    for (id, l) in &d.levels {
        let file = sflevel_encode(l);
        assert_eq!(sflevel_decode(&file).as_ref(), Ok(l), "{id}");
        assert_eq!(sflevel_replaces(&file), Some(*id));
    }
}

#[test]
fn a_level_file_names_its_level_and_carries_its_sections() {
    let d = data();
    let mut l = d.levels[&w1_1()].clone();
    let file = sflevel_encode(&l);
    assert!(file.starts_with("{\n  \"format\": \"sflevel\",\n  \"version\": 1,\n  \"replaces\": \"W1-1\",\n"), "{file}");
    assert!(file.contains("\"chunk\": {\"disk\": 1, \"index\": 21, "), "{file}");
    // Another section is kept by name, and a file with no `chunk` is still a level.
    l.extra.insert("notes".into(), b"made by hand".to_vec());
    let back = sflevel_decode(&sflevel_encode(&l)).unwrap();
    assert_eq!(back, l);
    let bare = format!(
        "{{\"sections\":{{\"t7mp\":\"{}\"}},\"replaces\":\"W1-1\",\"version\":1,\"format\":\"sflevel\"}}",
        section(&sflevel_encode(&l))
    );
    let b = sflevel_decode(&bare).unwrap();
    assert_eq!((b.id, b.chunk.index, &b.t7mp), (l.id, 0, &l.t7mp));
}

/// The `t7mp` section of a file `sflevel_encode` wrote.
fn section(file: &str) -> &str {
    let at = file.find("\"t7mp\": \"").unwrap() + 9;
    &file[at..at + file[at..].find('"').unwrap()]
}

/// Each frame's picture and sound for `frames` frames of `script`, and every modelled variable
/// on each frame `probe_every` divides.
fn play(mut g: Game, s: &InputScript, frames: u32, probe_every: u32) -> Vec<(sf_core::FrameOut, Vec<(VarId, u32)>)> {
    (0..frames)
        .map(|f| {
            let out = g.step(s.input_at(f));
            (out, if f % probe_every == 0 || f + 1 == frames { g.probe() } else { Vec::new() })
        })
        .collect()
}

#[test]
fn an_unchanged_export_plays_as_the_disk_level_for_1500_frames() {
    let d = data();
    let s = script("frog-wall");
    // W1-1 with the frog script it was written for, and every other level with the same inputs.
    for (id, l) in &d.levels {
        let ov = sflevel_decode(&sflevel_encode(l)).unwrap();
        let frames = if *id == w1_1() { 1500 } else { 300 };
        let disk = play(Game::new(d, Start::Level(*id)), &s, frames, 25);
        let file = play(Game::new_with_override(d, ov), &s, frames, 25);
        for (f, (a, b)) in disk.iter().zip(&file).enumerate() {
            assert!(a == b, "{id}: frame {f} differs from the disk level's");
        }
    }
}

const FROG_X: VarId = VarId::FrogX;

fn frog_x(probe: &[(VarId, u32)]) -> u32 {
    probe.iter().find(|(v, _)| *v == FROG_X).unwrap().1
}

#[test]
fn a_wall_cell_removed_lets_the_frog_through_and_nothing_else_differs_before() {
    let d = data();
    let s = script("frog-wall");
    let disk_level = &d.levels[&w1_1()];
    // W1-1:92,49: the cell of the tunnel's pillar the frog walks into (FINDINGS, T28).
    let mut ov = disk_level.clone();
    let cell = 49 * ov.t7mp.width as usize + 92;
    assert_eq!(ov.t7mp.body[cell] & 0b11000 << 10, 0b11000 << 10, "a solid cell");
    ov.t7mp.body[cell] = 0;
    let ov = sflevel_decode(&sflevel_encode(&ov)).unwrap();
    let run = |mut g: Game| {
        g.frog_only();
        play(g, &s, 1500, 1)
    };
    let disk = run(Game::new(d, Start::Level(w1_1())));
    let edited = run(Game::new_with_override(d, ov));
    let max = |r: &[(sf_core::FrameOut, Vec<(VarId, u32)>)]| r.iter().map(|(_, p)| frog_x(p)).max().unwrap();
    // The pillar's left side is x 1440; past its first column is past x 1488.
    assert!(max(&disk) < 1488, "the disk level's frog got past the pillar: {}", max(&disk));
    assert!(max(&edited) > 1488, "the frog did not pass the removed cell: {}", max(&edited));
    let first = (0..disk.len()).find(|&f| frog_x(&disk[f].1) != frog_x(&edited[f].1)).unwrap();
    assert!(first > 400, "the frog parts from the disk level's at {first}, before it reaches the wall");
    for f in 0..first {
        // The map, and the playfield the cell is drawn into once it scrolls into view.
        let drawn = [VarId::LevelMap, VarId::PlayfieldBuffer0, VarId::PlayfieldBuffer1, VarId::PlayfieldClean];
        let mut other: Vec<_> = disk[f].1.iter().zip(&edited[f].1).filter(|(a, b)| a != b).map(|(a, _)| a.0).collect();
        other.dedup();
        assert!(
            other.iter().all(|v| drawn.contains(v)),
            "frame {f}, before the frog parts: {other:?} differ besides the map and its picture"
        );
    }
}

#[test]
fn each_refusal_from_a_crafted_file() {
    let d = data();
    let good = sflevel_encode(&d.levels[&w1_1()]);
    let t7mp = section(&good).to_string();
    let with = |fields: &str| format!("{{\"format\":\"sflevel\",{fields}}}");
    let cases: Vec<(String, SflevelError)> = vec![
        ("".into(), SflevelError::NotJson),
        ("PK\u{3}\u{4} a zip".into(), SflevelError::NotJson),
        (good[..good.len() / 2].into(), SflevelError::NotJson),
        ("[1, 2]".into(), SflevelError::WrongFormat),
        (good.replace("\"sflevel\"", "\"sfstate\""), SflevelError::WrongFormat),
        (with("\"replaces\":\"W1-1\""), SflevelError::WrongFormat),
        (with("\"version\":\"1\",\"replaces\":\"W1-1\""), SflevelError::WrongFormat),
        (with("\"version\":1.5,\"replaces\":\"W1-1\""), SflevelError::WrongFormat),
        (with("\"version\":1"), SflevelError::WrongFormat),
        (good.replace("\"chunk\": {\"disk\": 1", "\"chunk\": {\"disk\": 9"), SflevelError::WrongFormat),
        (with("\"version\":1,\"replaces\":\"W1-1\",\"sections\":{\"t7mp\":5}"), SflevelError::WrongFormat),
        (good.replace("\"version\": 1", "\"version\": 2"), SflevelError::Version(2)),
        (good.replace("\"version\": 1", "\"version\": 0"), SflevelError::Version(0)),
        (good.replace("\"W1-1\"", "\"W7-1\""), SflevelError::UnknownLevel("W7-1".into())),
        (with("\"version\":1,\"replaces\":\"W1-1\""), SflevelError::MissingSection("t7mp".into())),
        (
            with("\"version\":1,\"replaces\":\"W1-1\",\"sections\":{\"T7MP\":\"\"}"),
            SflevelError::MissingSection("t7mp".into()),
        ),
    ];
    for (file, want) in &cases {
        assert_eq!(sflevel_decode(file).as_ref(), Err(want), "{file:.80}");
        assert!(!want.to_string().is_empty());
    }
    // Damaged maps: not base64, not a T7MP, cut short, the wrong size.
    let map = |b64: &str| with(&format!("\"version\":1,\"replaces\":\"W1-1\",\"sections\":{{\"t7mp\":\"{b64}\"}}"));
    let bad = |file: String, says: &str| match sflevel_decode(&file) {
        Err(SflevelError::BadT7mp(why)) => assert!(why.contains(says), "{why}"),
        other => panic!("{other:?}, not a damaged map ({says})"),
    };
    bad(map("not base64!"), "base64");
    bad(map("AAAA"), "T7MP");
    bad(map(&t7mp[..t7mp.len() / 2 / 4 * 4]), "");
    let mut small = d.levels[&w1_1()].t7mp.clone();
    small.height = 83;
    small.body.truncate(300 * 83);
    let small = sflevel_encode(&LevelData {
        t7mp: small,
        ..d.levels[&w1_1()].clone()
    });
    bad(small, "300×84");
    // The level a refused file names, for the offer to play it from the disks instead.
    assert_eq!(sflevel_replaces(&good.replace("\"version\": 1", "\"version\": 2")), Some(w1_1()));
    assert_eq!(sflevel_replaces("not json"), None);
    assert_eq!(SflevelError::Version(2).to_string(), "it is a version 2 level file; this Superfrog reads version 1");
}
