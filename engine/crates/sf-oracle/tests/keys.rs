//! Superfrog's trace cache keys after the toolkit move (toolkit-extract T05): the toolkit builds the
//! key, and for Superfrog's set-ups it must be byte-equal to the text this crate hashed before the
//! move, so no cached trace is recomputed for nothing. Only `BUILD` changed, once, with the rebuild.
//! Needs the owner's Kickstart and disks (DESIGN §5).

use std::fs;

use sf_disk::{Offered, identify, sha1_hex};
use sf_oracle::*;

/// The base's disk text (`trace.rs` `cached_trace_in` before T05): the four disks' SHA-1s in slot
/// order, ` drives N` unless one drive, ` boot {:?}` unless disk 1 boots.
fn base_disks_text(cfg: &OracleConfig) -> String {
    let offered: Vec<Offered> = cfg
        .disks
        .iter()
        .map(|p| Offered {
            name: p.display().to_string(),
            bytes: fs::read(p).unwrap(),
        })
        .collect();
    let set = identify(&offered).unwrap();
    let mut text = set
        .disks
        .iter()
        .map(|d| sha1_hex(&d.bytes))
        .collect::<Vec<_>>()
        .join(" ");
    if cfg.drives != 1 {
        text += &format!(" drives {}", cfg.drives);
    }
    if cfg.boot != DiskSlot::D1 {
        text += &format!(" boot {:?}", cfg.boot);
    }
    text
}

#[test]
fn trace_keys_are_the_ones_the_base_wrote() {
    let dir = std::env::temp_dir().join(format!("sf-oracle-keys-{}", std::process::id()));
    let base = OracleConfig::locate(DEFAULT_MAX_FRAMES).unwrap_or_else(|e| panic!("{e}"));
    let rom = sha1_hex(&fs::read(&base.kickstart).unwrap());
    let script = InputScript::new(vec![(0, FrameInput::joy(Joy(0x10))), (3, FrameInput::NONE)]);
    let mut seen = Vec::new();
    for drives in [1, 2] {
        for boot in [DiskSlot::D1, DiskSlot::D3] {
            for (script, frames) in [(InputScript::idle(), 0), (script.clone(), 2)] {
                let cfg = base.clone().with_drives(drives).with_boot(boot);
                let text = base_disks_text(&cfg);
                let want = trace_key(
                    BUILD,
                    &rom,
                    &sha1_hex(text.as_bytes()),
                    &SyncPoint::PowerOn,
                    &script,
                    frames,
                );
                let (trace, outcome) =
                    cached_trace_in(&dir, &cfg, &SyncPoint::PowerOn, &script, frames).unwrap();
                assert_eq!(trace.key, want, "drives {drives}, boot {boot:?}: {text}");
                assert_eq!(outcome, CacheOutcome::Miss);
                let (_, again) =
                    cached_trace_in(&dir, &cfg, &SyncPoint::PowerOn, &script, frames).unwrap();
                assert_eq!(again, CacheOutcome::Hit);
                seen.push(want);
            }
        }
    }
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 8, "every set-up keeps a key of its own");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn superfrog_traces_stay_under_its_own_cache() {
    let cfg = OracleConfig::locate(1).unwrap_or_else(|e| panic!("{e}"));
    let generic = cfg.generic().unwrap();
    assert_eq!(generic.cache, trace_cache());
    assert!(trace_cache().ends_with("traces"));
    if std::env::var_os("SUPERFROG_CACHE").is_none() {
        assert!(
            trace_cache()
                .to_string_lossy()
                .contains(".cache/superfrog-re/traces")
        );
    }
    assert_eq!((generic.boot, generic.boot_label.as_str()), (Some(0), ""));
    let d3 = cfg.with_boot(DiskSlot::D3).generic().unwrap();
    assert_eq!((d3.boot, d3.boot_label.as_str()), (Some(2), "D3"));
}
