//! `sf-routeview`: draw routes over their level maps, one web page.
//!
//! ```text
//! sf-routeview --planner                    # every route the smart planner has made, with verdicts
//! sf-routeview ROUTE.txt [ROUTE2.txt …]     # any app-timed route files
//!              [--level W2-3] [--guide waypoints.txt] [--out build/route-viz] [--bare] [--jobs 4]
//!              [--results plans/smart-planner/bench/results.jsonl] [--partials build/route-viz-runs]
//! ```
//!
//! Run from `engine/`: `cargo run --release -p sf-routeview -- …`; then open
//! `build/route-viz/index.html`. The page holds the disks' graphics: keep it under `build/`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use sf_bench::results::LevelResult;
use sf_core::LevelId;
use sf_routeview::site::{self, Entry};
use sf_routeview::{dump, guide, level_of, levels, planner_results, planner_route, verdict};

fn usage() -> ! {
    eprintln!(
        "sf-routeview [--planner] [ROUTE.txt …] [--level W2-3] [--guide waypoints.txt] [--out DIR] [--bare] [--jobs N]\n\
         \x20            [--results results.jsonl] [--partials DIR]"
    );
    std::process::exit(2)
}

struct Job {
    id: String,
    label: String,
    group: String,
    level: LevelId,
    route: Option<PathBuf>,
    /// What the route is, for the verdict's facts (`--planner` only).
    source: Option<&'static str>,
    bench: Option<LevelResult>,
    guide: Option<(String, Vec<guide::Waypoint>)>,
}

fn shown(p: &Path, repo: &Path) -> String {
    p.strip_prefix(repo).unwrap_or(p).display().to_string()
}

fn main() {
    let repo = sf_bench::repo_dir().canonicalize().unwrap_or_else(|_| sf_bench::repo_dir());
    let mut planner = false;
    let mut routes = Vec::new();
    let mut level: Option<LevelId> = None;
    let mut guide_file: Option<PathBuf> = None;
    let mut out = repo.join("build/route-viz");
    let mut results = repo.join("plans/smart-planner/bench/results.jsonl");
    let mut partials = repo.join("build/route-viz-runs");
    let mut bare = false;
    let mut jobs = 4usize;
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| usage());
        match k.as_str() {
            "--planner" => planner = true,
            "--level" => level = Some(val().parse().unwrap_or_else(|_| usage())),
            "--guide" => guide_file = Some(PathBuf::from(val())),
            "--out" => out = PathBuf::from(val()),
            "--results" => results = PathBuf::from(val()),
            "--partials" => partials = PathBuf::from(val()),
            "--bare" => bare = true,
            "--jobs" => jobs = val().parse().ok().filter(|&n| n > 0).unwrap_or_else(|| usage()),
            s if s.starts_with("--") => usage(),
            _ => routes.push(PathBuf::from(k)),
        }
    }
    if !planner && routes.is_empty() {
        usage();
    }

    let mut todo: Vec<Job> = Vec::new();
    if planner {
        let bench_dir = results.parent().unwrap_or(Path::new(".")).to_path_buf();
        let got = planner_results(&results);
        for l in levels() {
            let name = l.to_string();
            let route = planner_route(&bench_dir, &partials, &name);
            let bench = got.iter().find(|r| r.level == name).cloned();
            if route.is_none() && bench.is_none() {
                continue;
            }
            todo.push(Job {
                id: name.clone(),
                label: name.clone(),
                group: name[..2].to_string(),
                level: l,
                source: route.as_ref().map(|r| r.1),
                route: route.map(|r| r.0),
                bench,
                guide: None,
            });
        }
    }
    let guide = guide_file.as_ref().map(|p| {
        let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        let pts = guide::parse(&text);
        eprintln!("sf-routeview: guide {} has {} waypoints", p.display(), pts.len());
        (p.file_name().unwrap_or_default().to_string_lossy().to_string(), pts)
    });
    for p in routes {
        let text = std::fs::read_to_string(&p).unwrap_or_else(|e| {
            eprintln!("sf-routeview: {}: {e}", p.display());
            std::process::exit(2)
        });
        let l = level.or_else(|| level_of(&p, &text)).unwrap_or_else(|| {
            eprintln!("sf-routeview: {}: no level id in its name or header; pass --level", p.display());
            std::process::exit(2)
        });
        let mut id = site::id_of(&p.file_stem().unwrap_or_default().to_string_lossy());
        while todo.iter().any(|j| j.id == id) {
            id.push('_');
        }
        todo.push(Job {
            label: id.clone(),
            id,
            group: l.to_string(),
            level: l,
            route: Some(p),
            source: None,
            bench: None,
            guide: guide.clone(),
        });
    }

    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<(usize, Entry)>> = Mutex::new(Vec::new());
    let failed = AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..jobs.min(todo.len()).max(1) {
            sc.spawn(|| {
                while let Some(j) = todo.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let text = j.route.as_ref().map(|p| std::fs::read_to_string(p).unwrap_or_default());
                    let mut d = match dump::replay(j.level, text.as_deref()) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("sf-routeview: {}: {e}", j.id);
                            failed.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    };
                    d.guide = j.guide.clone();
                    let facts = verdict::progress(&mut d);
                    let mut v = verdict::verdict(&d, j.route.is_some(), j.bench.as_ref());
                    if let Some(s) = j.source {
                        v.info.insert(0, s.to_string());
                    }
                    v.info.extend(facts);
                    if let Err(e) = site::write_data(&out, &j.id, &d.to_json()) {
                        eprintln!("sf-routeview: {}: {e}", out.display());
                        std::process::exit(1)
                    }
                    eprintln!(
                        "  {:<12} {:<7} {} frames, exit {}, {} objects",
                        j.id,
                        v.status.word(),
                        d.track.len(),
                        d.end.map_or("never".to_string(), |e| format!("at {e}")),
                        d.objects.len()
                    );
                    let e = Entry {
                        id: j.id.clone(),
                        label: j.label.clone(),
                        group: j.group.clone(),
                        route_file: j.route.as_ref().map(|p| shown(p, &repo)),
                        verdict: v,
                        wall_s: j.bench.as_ref().map(|b| b.wall_s),
                        ref_frames: j.bench.as_ref().and_then(|b| b.ref_frames),
                    };
                    let i = todo.iter().position(|t| t.id == j.id).unwrap();
                    done.lock().unwrap().push((i, e));
                }
            });
        }
    });
    let mut entries = done.into_inner().unwrap();
    entries.sort_by_key(|e| e.0);
    let entries: Vec<Entry> = entries.into_iter().map(|e| e.1).collect();
    let page = site::write_index(&out, &entries, bare).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
    let n = |s: verdict::Status| entries.iter().filter(|e| e.verdict.status == s).count();
    println!(
        "{}: {} routes, {} clean, {} with faults, {} failed",
        page.display(),
        entries.len(),
        n(verdict::Status::Clean),
        n(verdict::Status::Faults),
        n(verdict::Status::Failed)
    );
    if failed.load(Ordering::Relaxed) > 0 {
        std::process::exit(1);
    }
}
