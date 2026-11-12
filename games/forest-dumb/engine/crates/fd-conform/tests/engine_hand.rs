//! `--engine hand` (translator T29) is the runner every case ran on before the translated engine:
//! every case under `engine/conform/cases/` passes through `reading::run_on(.., Hand)` as through
//! `fd_conform::run`. Full: the disks and Kickstart, and every case, in parallel.

use fd_conform::Case;
use fd_conform::reading::{EngineKind, Reading, Row, run_on};

#[test]
#[ignore = "full: every case on the hand engine, the disks and Kickstart"]
fn every_case_passes_on_the_hand_engine_as_before() {
    let dir = fd_conform::conform_dir().join("cases");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    paths.sort();
    // A power-on case (`start = "entry"`, translator T32) is the translated program's alone: the
    // hand engine starts at level 1 and refuses it.
    let cases: Vec<Case> = paths
        .iter()
        .map(|p| Case::load(p).unwrap_or_else(|e| panic!("{e}")))
        .filter(|c| !c.is_entry())
        .collect();
    assert!(cases.len() >= 43, "{} cases in {}", cases.len(), dir.display());
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let rows: Vec<std::sync::Mutex<Option<Row>>> =
        cases.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..workers.min(cases.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(c) = cases.get(i) else { break };
                    *rows[i].lock().unwrap() = Some(Row::new(&c.name, run_on(c, EngineKind::Hand)));
                }
            });
        }
    });
    let reading = Reading {
        engine: "hand",
        rows: rows.into_iter().map(|r| r.into_inner().unwrap().unwrap()).collect(),
    };
    let failed: Vec<String> = reading
        .rows
        .iter()
        .filter(|r| !r.passed())
        .map(|r| format!("{}: {}", r.case, r.result()))
        .collect();
    assert!(failed.is_empty(), "{}\n{}", reading.summary(), failed.join("\n"));
}
