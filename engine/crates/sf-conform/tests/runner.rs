//! The conformance runner against the original (T12). Needs what sf-oracle's tests need:
//! `tools/setup/oracle.sh` run, the four disks and Kickstart 1.3. The title sync point is cached
//! outside git after the first run (`sf_conform::sync`).

use std::path::Path;
use std::sync::OnceLock;

use sf_conform::{Case, Compare, sync};
use sf_core::layout::{LAYOUT, Ram, VarId, def};
use sf_core::{Game, GameData, Rng, Start};
use sf_disk::{DiskSlot, Offered, carve, decompress, identify};
use sf_oracle::{Cpu68k, Regs};

fn title_case() -> Case {
    Case::load(&sf_conform::cases_dir().join("title-rng.toml")).unwrap_or_else(|e| panic!("{e}"))
}

/// The committed title case: the modelled variables match for its 50 frames.
#[test]
fn the_title_case_is_green() {
    let case = title_case();
    assert_eq!(case.compare, Compare::VARS);
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
    }
}

#[test]
fn transplant_then_probe_returns_the_originals_values() {
    let case = title_case();
    let synced = sync::reach(&case.sync, &case.script, 1).unwrap();
    let game = Game::new(
        &GameData::default(),
        Start::Transplant(synced.memory.clone()),
    );
    let want: Vec<(VarId, u32)> = LAYOUT
        .iter()
        .flat_map(|d| d.pieces())
        .map(|p| (p.id, p.read(&synced.memory as &dyn Ram)))
        .collect();
    assert_eq!(game.probe(), want);
    // The title has seeded the RNG: a zero state would make this test vacuous.
    assert_ne!(game.state().rng, Rng::default());
}

#[test]
fn a_planted_one_variable_divergence_is_named_with_its_frame() {
    let case = title_case();
    let at = def(VarId::RngState).addr + 4;
    let d = sf_conform::run_with(&case, |f, game| {
        if f == 17 {
            game.poke(at + 3, &[0x5A]);
        }
    })
    .expect_err("the planted change was not seen");
    assert_eq!(d.frame, 17, "{d}");
    let (var, theirs, ours) = d.first_var.unwrap();
    assert_eq!((var.name, var.addr), ("rng_state", at));
    assert_eq!(ours & 0xFF, 0x5A);
    assert_ne!(theirs, ours);
    assert!(d.picture_diff.is_none() && d.paula_diff.is_none());
    let text = d.to_string();
    assert!(
        text.contains("frame 17") && text.contains("rng_state"),
        "{text}"
    );
}

/// The placeholder scene renders black; the title does not. The diff image goes to the temp dir.
#[test]
fn a_picture_divergence_writes_a_png_outside_the_repo_and_names_the_first_pixel() {
    let case = Case {
        compare: Compare {
            picture: true,
            ..Compare::default()
        },
        name: "picture-test".into(),
        ..title_case()
    };
    let d = sf_conform::run(&case).expect_err("a black picture matched the title");
    assert_eq!(d.frame, 0);
    let p = d
        .picture_diff
        .clone()
        .expect("no picture difference reported");
    let (x, y, theirs, ours) = p.mismatch.first.expect("no first pixel");
    assert_eq!(ours, 0);
    assert_ne!(theirs, 0);
    assert!(x < 320 && y < 256);
    let bytes = std::fs::read(&p.path).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let main = sf_conform::disks::main_checkout().unwrap();
    let path = p.path.canonicalize().unwrap();
    assert!(
        !path.starts_with(&repo) && !path.starts_with(&main),
        "{}",
        path.display()
    );
    assert!(d.to_string().contains(&format!("first pixel ({x}, {y})")));
}

/// The title music writes to Paula every frame; the placeholder scene writes nothing.
#[test]
fn a_paula_divergence_shows_the_first_differing_write_with_its_index() {
    let case = Case {
        compare: Compare {
            paula: true,
            ..Compare::default()
        },
        name: "paula-test".into(),
        ..title_case()
    };
    let d = sf_conform::run(&case).expect_err("silence matched the title music");
    let p = d.paula_diff.expect("no Paula difference reported");
    assert!(
        p.starts_with("write #0: original AUD") || p.starts_with("write #0: original DMACON"),
        "{p}"
    );
    assert!(p.contains("engine nothing"), "{p}");
}

// --- the RNG against the original's routines on Moira ---

/// `d1_c03`, the game executable, from the owner's disk 1 (chunk 3).
fn game_exe() -> Vec<u8> {
    let files: Vec<Offered> = sf_conform::disks::read_disk_files()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|(name, bytes)| Offered { name, bytes })
        .collect();
    let set = identify(&files).unwrap_or_else(|e| panic!("{e:?}"));
    let chunk = carve(set.disk(DiskSlot::D1))
        .into_iter()
        .find(|c| c.index == 3)
        .unwrap();
    decompress(&chunk.raw).unwrap()
}

struct Original {
    cpu: Cpu68k,
    next: u32,
    range: u32,
    state: u32,
}

fn original() -> &'static std::sync::Mutex<Original> {
    static O: OnceLock<std::sync::Mutex<Original>> = OnceLock::new();
    O.get_or_init(|| {
        let mut cpu = Cpu68k::new(sf_oracle::AmigaMemory {
            chip: vec![0; 512 * 1024],
            slow: vec![0; 512 * 1024],
        });
        let map = cpu.load_exe(&game_exe(), 0x1_0000).unwrap();
        std::sync::Mutex::new(Original {
            next: map.addr(0, 0x16084),
            range: map.addr(0, 0x160c0),
            state: map.addr(0, 0x160d4),
            cpu,
        })
    })
}

/// splitmix64: the test's own source of random states.
fn mix(s: &mut u64) -> u64 {
    *s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *s;
    z = (z ^ z >> 30).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ z >> 27).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ z >> 31
}

#[test]
fn rng_rs_agrees_with_the_originals_routines_for_100_000_calls() {
    let mut o = original().lock().unwrap();
    let (next, range, state) = (o.next, o.range, o.state);
    let mut seed = 0x5346_5247u64;
    for k in 0..100_000u32 {
        let r = mix(&mut seed);
        let mut ours = Rng {
            lo: r as u32,
            hi: (r >> 32) as u32,
        };
        o.cpu.write(state, &ours.to_bytes());
        let mut regs = Regs::default();
        regs.a[7] = 0x8_0000;
        let use_range = k % 2 == 1;
        let n = (mix(&mut seed) as u16).max(if k % 1000 == 1 { 0xFFFF } else { 0 });
        regs.d[0] = (mix(&mut seed) as u32) & 0xFFFF_0000 | n as u32;
        let d0_in = regs.d[0];
        let got = o
            .cpu
            .call(if use_range { range } else { next }, regs, 1_000)
            .unwrap_or_else(|e| panic!("call {k}: {e}"));
        let after = Rng::from_bytes(got.bytes_after(&o.cpu, state, 8).try_into().unwrap());
        if use_range {
            let want = ours.range(n);
            let d0 = if n == 0xFFFF {
                d0_in & 0xFFFF_0000
            } else {
                want as u32
            };
            assert_eq!(got.regs.d[0], d0, "call {k}: rng_range({n:#x}) from {r:#x}");
        } else {
            let want = ours.next();
            assert_eq!(
                (got.regs.d[0], got.regs.d[1]),
                (want, want),
                "call {k}: rng_next from {r:#x}"
            );
        }
        assert_eq!(after, ours, "call {k}: rng_state after, from {r:#x}");
    }
}

#[test]
fn rng_seed_agrees_with_the_original() {
    let mut o = original().lock().unwrap();
    let (seed_at, state) = (o.next - 0xa, o.state);
    let mut s = 7u64;
    for k in 0..1_000 {
        let mut regs = Regs::default();
        regs.a[7] = 0x8_0000;
        regs.d[0] = mix(&mut s) as u32;
        regs.d[1] = mix(&mut s) as u32;
        let got = o.cpu.call(seed_at, regs, 100).unwrap();
        let want = Rng::seeded(regs.d[0], regs.d[1]);
        assert_eq!(got.regs.d[0], want.hi, "seed {k}: d0");
        let after = Rng::from_bytes(got.bytes_after(&o.cpu, state, 8).try_into().unwrap());
        assert_eq!(after, want, "seed {k}");
    }
}
