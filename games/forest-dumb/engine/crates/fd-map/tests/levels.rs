//! `fd-map` on made-up level bytes, then on each real world (plan forest-dumb-map-page T02). The real
//! worlds need the owner's disks; absent, those tests fail naming where they looked.

use amiga_routeview::LevelDump;
use fd_assets::{
    AssetError, CollisionMap, LEVEL_ROWS, Level, OBJECT_COLUMNS, ObjectMap, SecretKind, TILE_BYTES,
    TileMap, read_level, secrets,
};
use fd_map::{MapLevel, WORLDS, items_needed, level_id, map_level, object_name};

fn longs(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|l| l.to_be_bytes()).collect()
}

/// A made-up main program of five hunks: hunk 1 data holding the level palette at `$384`, colour
/// `i` = `$0i0` + `i`; the others empty BSS.
fn program() -> Vec<u8> {
    let h1_longs = 0x404 / 4;
    let mut h1 = vec![0u32; h1_longs];
    for i in 0..32u32 {
        h1[0x384 / 4 + i as usize] = (0x180 + 2 * i) << 16 | (i << 4 | i & 0xf);
    }
    let mut v = vec![0x3f3, 0, 5, 0, 4, 1, h1_longs as u32, 1, 1, 1];
    v.extend([0x3eb, 1, 0x3f2]);
    v.extend([0x3ea, h1_longs as u32]);
    v.extend(&h1);
    v.push(0x3f2);
    for _ in 0..3 {
        v.extend([0x3eb, 1, 0x3f2]);
    }
    longs(&v)
}

/// A made-up level `cols` used columns wide (the tile map 40 wider, as worlds 1 to 5 are), tile 1
/// along row 95, with objects `objs` (col, row, value) on an object map of the game's 400 columns.
fn level(cols: u16, objs: &[(u16, u16, u8)]) -> Level {
    let tile_cols = cols + 40;
    let mut tiles = vec![0u16; tile_cols as usize * 96];
    for c in 0..tile_cols as usize {
        tiles[c * 96 + 95] = 1;
    }
    let mut objects = vec![0u8; OBJECT_COLUMNS as usize * 96];
    for &(c, r, v) in objs {
        objects[c as usize * 96 + r as usize] = v;
    }
    let mut solid = [0u8; TILE_BYTES];
    solid[0] = 0x80; // tile 1's top-left pixel: colour 1
    Level {
        tiles: TileMap {
            columns: tile_cols,
            cells: tiles,
        },
        gens: vec![[0; TILE_BYTES], solid],
        collision: CollisionMap {
            columns: cols,
            cells: vec![0; cols as usize * 96],
        },
        shapes: vec![[0; 16]],
        objects: ObjectMap {
            columns: OBJECT_COLUMNS,
            cells: objects,
        },
    }
}

/// Fifty-one coins, so one item needed, and a 2×3 exit.
fn usual() -> Vec<(u16, u16, u8)> {
    let mut o: Vec<_> = (0..51)
        .map(|i| (i % 50, 10 + i / 50, 0x20 + (i % 8) as u8))
        .collect();
    o.extend([
        (5, 3, 0x01),
        (6, 3, 0x40),
        (7, 3, 0x28),
        (8, 3, 0x10),
        (9, 3, 0x34),
        (10, 3, 0x99),
    ]);
    for c in 200..202 {
        for r in 80..83 {
            o.push((c, r, 0xff));
        }
    }
    o
}

#[test]
fn objects_layers_exit_and_items_needed_from_made_up_bytes() {
    let m = map_level(2, &level(240, &usual()), &program()).unwrap();
    assert_eq!((m.world, m.cols, m.rows), (2, 240, LEVEL_ROWS));
    assert_eq!(m.body.len(), 240 * 96);
    assert_eq!(m.body[95 * 240], 1, "row by row: row 95 is the last");
    assert_eq!(m.tiles.len(), 2);
    assert_eq!(m.tiles[1].1[0], 1);
    assert_eq!(m.palette.len(), 32);
    assert_eq!(m.palette[3], [0x00, 0x33, 0x33]);
    assert_eq!(m.items_needed, 1);
    assert_eq!(
        m.exit,
        [
            (200, 80),
            (200, 81),
            (200, 82),
            (201, 80),
            (201, 81),
            (201, 82)
        ]
    );
    let cat = |v: u8| {
        m.objects
            .iter()
            .find(|o| o.value == v)
            .map(|o| (o.cat, o.name.clone()))
    };
    assert_eq!(cat(0x20), Some(("coin", "coin".into())));
    assert_eq!(cat(0x01).unwrap().0, "enemy");
    assert_eq!(cat(0x40).unwrap().0, "pickup");
    assert_eq!(cat(0x28).unwrap().0, "hazard");
    assert_eq!(cat(0x10).unwrap().0, "spring");
    // The flame comes from objects-more.toml, not W1-1's table.
    assert_eq!(cat(0x34), Some(("flame", "flame $34".into())));
    assert_eq!(cat(0x99), Some(("other", "object $99".into())));
    assert_eq!(cat(0xff), Some(("exit", "level exit".into())));
    assert_eq!(m.objects.len(), 51 + 6 + 6);
    // The page's data file: the id, the exit and no route.
    assert_eq!(m.level_id(), "W2-1");
    let js = m.to_json();
    for want in [
        "\"level\":\"W2-1\"",
        "\"coins_needed\":1",
        "\"exit\":[[200,80],",
        "\"track\":[]",
        "[9,3,52,\"flame\",\"flame $34\"]",
    ] {
        assert!(js.contains(want), "{want} not in the data");
    }
}

#[test]
fn objects_past_the_used_columns_are_not_drawn_but_their_coins_count() {
    let mut o = usual();
    o.extend([(300, 5, 0x20), (300, 6, 0x01)]);
    let m = map_level(1, &level(240, &o), &program()).unwrap();
    assert!(m.objects.iter().all(|o| o.c < 240));
    // h0+$7a9e counts the whole 38 400 bytes (docs/10).
    assert_eq!(m.items_needed, 2);
}

/// A wall (tile 2, solid shape) with one cell of it empty inside: T07's hidden passage, listed apart
/// from the objects with cat `secret`, value 0, named by kind; and on the page before the objects.
#[test]
fn a_made_up_hidden_passage_is_a_secret() {
    let mut l = level(240, &usual());
    l.gens.push([0; TILE_BYTES]);
    l.shapes.push([0xffff; 16]);
    for c in 20..26 {
        for r in 40..50 {
            l.tiles.cells[c * 96 + r] = 2;
            l.collision.cells[c * 96 + r] = 1;
        }
    }
    l.collision.cells[22 * 96 + 45] = 0;
    // A coin in the passage stays an object, and the page keeps it the cell's last.
    l.objects.cells[22 * 96 + 45] = 0x20;
    let m = map_level(1, &l, &program()).unwrap();
    let got: Vec<_> = m
        .secrets
        .iter()
        .map(|o| (o.c, o.r, o.value, o.cat, o.name.as_str()))
        .collect();
    assert_eq!(got, [(22, 45, 0, "secret", "hidden passage")]);
    assert_eq!(
        fd_map::secret_name(SecretKind::HiddenPassage),
        "hidden passage"
    );
    assert!(m.objects.iter().all(|o| o.cat != "secret"));
    let json = m.to_json();
    let (s, c) = (
        json.find(r#"[22,45,0,"secret","hidden passage"]"#).unwrap(),
        json.find(r#"[22,45,32,"coin","coin"]"#).unwrap(),
    );
    assert!(s < c, "the secret after the coin at its cell");
    // With no wall met empty, none.
    assert!(
        map_level(1, &level(240, &usual()), &program())
            .unwrap()
            .secrets
            .is_empty()
    );
}

/// The Secrets layer is off at load and holds only `secret` (DESIGN §2.3).
#[test]
fn the_secrets_layer_starts_off() {
    let spec = fd_map::spec();
    let l = spec.categories.iter().find(|c| c.key == "secrets").unwrap();
    assert!(!l.on);
    assert_eq!(l.cats, ["secret"]);
    assert!(
        spec.categories
            .iter()
            .filter(|c| c.key != "secrets")
            .all(|c| !c.cats.iter().any(|k| k == "secret"))
    );
}

#[test]
fn w6_1_is_320_columns_wide() {
    let mut o = usual();
    o.push((319, 0, 0x41));
    let mut l = level(320, &o);
    l.tiles.columns = 320;
    l.tiles.cells.truncate(320 * 96);
    let m = map_level(6, &l, &program()).unwrap();
    assert_eq!((m.cols, m.body.len()), (320, 320 * 96));
    assert!(
        m.objects
            .iter()
            .any(|o| (o.c, o.r, o.value) == (319, 0, 0x41))
    );
    assert_eq!(level_id(6), "W6-1");
}

#[test]
fn bad_input_is_an_error_naming_what() {
    let e = map_level(1, &level(240, &[(1, 1, 0x20)]), &program()).unwrap_err();
    assert!(e.starts_with("_MapaBobow1: no exit"), "{e}");
    let e = map_level(7, &level(240, &usual()), &program()).unwrap_err();
    assert!(e.contains("world 7"), "{e}");
    let e = map_level(1, &level(240, &usual()), &program()[..64]).unwrap_err();
    assert!(e.contains("FOREST_DUMB_FOREVER"), "{e}");
    let mut p = program();
    // Colour 5's register word in hunk 1: header 10 longs, hunk 0's 3, hunk 1's two.
    let at = (10 + 3 + 2) * 4 + 0x384 + 4 * 5;
    p[at + 1] = 0;
    let e = map_level(1, &level(240, &usual()), &p).unwrap_err();
    assert!(e.contains("colour 5"), "{e}");
    let e = items_needed("_MapaBobow3", &[0x20; 49]).unwrap_err();
    assert!(e.starts_with("_MapaBobow3: 49 coins"), "{e}");
    assert_eq!(items_needed("x", &[0x27; 50]), Ok(0));
}

#[test]
fn every_value_of_w1_1_s_table_has_a_layer_and_an_unknown_is_other() {
    let g = fd_map::groups();
    // W1-1's 24 values and the flame (objects-more.toml).
    assert!(g.len() >= 25, "{g:?}");
    for (&v, group) in g {
        let (cat, name) = object_name(v);
        assert!(cat != "other", "${v:02x} in group {group} has no layer");
        assert!(!name.is_empty());
    }
    assert_eq!(object_name(0x99), ("other", "object $99".into()));
    let names: Vec<&str> = fd_map::TABLES.iter().map(|t| t.0).collect();
    for want in ["re/objects-W1-1.toml", "re/objects-more.toml"] {
        assert!(names.contains(&want), "{want} not gathered: {names:?}");
    }
}

#[test]
fn a_value_two_tables_group_differently_keeps_the_first() {
    let a = "[[value]]\nvalue = 0x34\ngroup = \"flame\"\n";
    let b = "[[value]]\nvalue = 52 # decimal\ngroup = \"bonus\"\n[[value]]\nvalue = 0x35\ngroup = \"water\"\n";
    let g = fd_map::groups_of(&[("a", a), ("b", b)]);
    assert_eq!(g.get(&0x34).map(String::as_str), Some("flame"));
    assert_eq!(g.get(&0x35).map(String::as_str), Some("water"));
}

// ---------------------------------------------------------------------------------------------
// The real worlds
// ---------------------------------------------------------------------------------------------

fn disk(name: &str) -> Result<Vec<u8>, String> {
    fd_tools::game_file(&fd_tools::disk_dir()?, name)
}

fn files(name: &str) -> Result<Vec<u8>, AssetError> {
    disk(&format!("disk2/{name}")).map_err(|e| AssetError::missing(name, e))
}

fn real(world: u8, program: &[u8]) -> (Vec<u8>, MapLevel) {
    let level = read_level(world, &files).unwrap_or_else(|e| panic!("{e}"));
    let m = map_level(world, &level, program).unwrap_or_else(|e| panic!("W{world}-1: {e}"));
    // The object file's bytes as the disk holds them, not through fd-assets.
    (files(&format!("_MapaBobow{world}")).unwrap(), m)
}

/// Each world's object counts per value equal its `_MapaBobowN` file's over the used columns; its
/// exit is a 2×3 block of `$ff`; its items needed is T00's reading on the oracle.
#[test]
fn each_real_world_matches_its_object_file() {
    let program = disk("disk1/FOREST_DUMB_FOREVER").unwrap_or_else(|e| panic!("{e}"));
    // map-page T00, 2026-11-20, read on the oracle after load_world (docs/10, "Items").
    let needed = [282, 549, 676, 396, 547, 751];
    for (i, world) in WORLDS.into_iter().enumerate() {
        let (file, m) = real(world, &program);
        let used = if world == 6 { 320 } else { 240 };
        assert_eq!(m.cols, used);
        let mut want = [0usize; 256];
        for &v in &file[..used as usize * 96] {
            want[v as usize] += 1;
        }
        want[0] = 0;
        let mut got = [0usize; 256];
        for o in &m.objects {
            got[o.value as usize] += 1;
            assert_eq!(file[o.c as usize * 96 + o.r as usize], o.value);
        }
        for v in 0..256 {
            assert_eq!(got[v], want[v], "W{world}-1: value ${v:02x}");
        }
        assert_eq!(m.exit.len(), 6, "W{world}-1 exit {:?}", m.exit);
        let (c0, r0) = m.exit[0];
        for (k, &(c, r)) in m.exit.iter().enumerate() {
            assert_eq!(
                (c, r),
                (c0 + k as u16 / 3, r0 + k as u16 % 3),
                "W{world}-1 exit"
            );
        }
        assert_eq!(m.items_needed, needed[i], "W{world}-1 items needed");
        assert!(m.objects.iter().all(|o| o.cat != "other"), "W{world}-1");
        assert_eq!(m.palette.len(), 32);
    }
}

/// Each world's secrets are `fd_assets::secrets`'s, cell for cell, all named "hidden passage".
#[test]
fn each_real_world_lists_fd_assets_secrets() {
    let program = disk("disk1/FOREST_DUMB_FOREVER").unwrap_or_else(|e| panic!("{e}"));
    let mut counts = Vec::new();
    for world in WORLDS {
        let level = read_level(world, &files).unwrap_or_else(|e| panic!("{e}"));
        let m = map_level(world, &level, &program).unwrap();
        let want: Vec<_> = secrets(&level).iter().map(|s| (s.c, s.r)).collect();
        let got: Vec<_> = m.secrets.iter().map(|o| (o.c, o.r)).collect();
        assert_eq!(got, want, "W{world}-1");
        assert!(
            m.secrets
                .iter()
                .all(|o| o.cat == "secret" && o.name == "hidden passage")
        );
        counts.push(got.len());
    }
    // docs/17: passages in W1-1, W3-1 and W4-1 only.
    assert!(
        counts[0] > 0 && counts[2] > 0 && counts[3] > 0,
        "{counts:?}"
    );
}
