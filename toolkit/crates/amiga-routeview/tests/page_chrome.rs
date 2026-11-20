//! The page in headless Chrome (amiga-browser), at 1280×800 and 800×600: a site with one routed
//! and one map-only level. The map-only level draws its objects with "Map only" and no timeline,
//! each object layer's chip hides and shows its objects, and switching to the routed level and back
//! brings the scrubber and takes it away, with no error in the page. Chrome missing fails, never
//! skips. Screens go to the test's temp folder.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use amiga_browser::{Browser, Opts};
use amiga_routeview::verdict::{Status, Verdict};
use amiga_routeview::{Category, Entry, LevelDump, PageSpec, write_site};

const COLS: usize = 20;
const ROWS: usize = 10;

/// One 8×8 tile in two greys, so no map pixel matches a layer's colour.
fn tiles() -> String {
    (0..64).map(|i| if (i / 8 + i) % 2 == 0 { "00" } else { "01" }).collect()
}

fn base(id: &str) -> String {
    format!(
        r##""level":"{id}","cols":{COLS},"rows":{ROWS},"palette":["#404040","#808080"],"tiles":{{"0":"{}"}},"body":[{}]"##,
        tiles(),
        vec!["0"; COLS * ROWS].join(",")
    )
}

/// Coins on the magenta layer and two other objects on the cyan one.
const OBJECTS: &str = r#""objects":[[3,4,32,"coin","coin"],[5,4,32,"coin","coin"],[12,6,1,"enemy","creature"],[17,2,255,"exit","level exit"]]"#;

struct Routed;

impl LevelDump for Routed {
    fn level_id(&self) -> &str {
        "R1"
    }

    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        let n = 60;
        let track: Vec<String> = (0..n).map(|k| format!("[{},{}]", 8 + k * 2, 60)).collect();
        let inputs = vec!["8"; n].join(",");
        let live = vec!["[]"; n / 2].join(",");
        write!(
            out,
            r##"{{{},{OBJECTS},"seed":1,"timing":"app","track":[{}],"inputs":[{inputs}],"live":[{live}],"events":[],"notes":["# made up"],"coins_needed":2,"end":null,"inputs_end":{}}}"##,
            base("R1"),
            track.join(","),
            n - 1
        )
    }

    fn entry(&self) -> Entry {
        Entry {
            id: "R1".into(),
            label: "Routed R1".into(),
            group: "Made up".into(),
            route_file: Some("routes/r1.txt".into()),
            verdict: Some(Verdict { status: Status::Clean, faults: vec![], info: vec![] }),
            wall_s: None,
            ref_frames: None,
        }
    }
}

struct MapOnly;

impl LevelDump for MapOnly {
    fn level_id(&self) -> &str {
        "M1"
    }

    // Only what the level's files hold: no replay keys besides the empty track.
    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        write!(out, r#"{{{},{OBJECTS},"track":[],"coins_needed":2}}"#, base("M1"))
    }

    fn entry(&self) -> Entry {
        Entry {
            id: "M1".into(),
            label: "Map M1".into(),
            group: "Made up".into(),
            route_file: None,
            verdict: None,
            wall_s: None,
            ref_frames: None,
        }
    }
}

/// Collects the page's errors from its first line on: thrown, rejected or logged.
const CATCH: &str = "<script>window.ERRS=[];addEventListener('error',e=>ERRS.push(String(e.message)));\
addEventListener('unhandledrejection',e=>ERRS.push(String(e.reason)));\
{const ce=console.error;console.error=(...a)=>{ERRS.push(a.join(' '));ce(...a)}}</script>";

fn site() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("routeview-chrome");
    let _ = std::fs::remove_dir_all(&root);
    let spec = PageSpec {
        title: "Made-up Game".into(),
        cell_size: (8, 8),
        categories: vec![
            Category::new("gems", "Gems", "#ff00ff", true, &["coin"]),
            Category::new("rest", "Everything else", "#00ffff", true, &[]),
        ],
        actor: "hero".into(),
        anchor: (0, 0),
        cell_origin: (0, 0),
        live_box: (0, 0, 8, 8),
        live: None,
    };
    // The map-only level first, so the page opens on it.
    write_site(&root, &spec, &[&MapOnly, &Routed]).unwrap();
    let index = root.join("index.html");
    let html = std::fs::read_to_string(&index).unwrap();
    std::fs::write(&index, html.replacen("<body>", &format!("<body>{CATCH}"), 1)).unwrap();
    root
}

/// Magenta and cyan pixels on the map canvas: the two layers' objects.
const PIXELS: &str = "(() => { const c = document.getElementById('cv'), d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; \
let m = 0, y = 0; for (let i = 0; i < d.length; i += 4) { if (d[i] > 180 && d[i+1] < 70 && d[i+2] > 180) m++; else if (d[i] < 70 && d[i+1] > 180 && d[i+2] > 180) y++; } return [m, y]; })()";

fn pixels(b: &mut Browser) -> (u64, u64) {
    let v = b.eval(PIXELS).unwrap();
    (v[0].as_u64().unwrap(), v[1].as_u64().unwrap())
}

/// Clicks the middle of the element `js` finds, scrolled into view first.
fn click(b: &mut Browser, js: &str) {
    let r = b
        .eval(&format!(
            "(() => {{ const e = {js}; if (!e) return null; e.scrollIntoView({{block: 'center'}}); const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; }})()"
        ))
        .unwrap();
    assert!(!r.is_null(), "nothing found by {js}");
    b.click(r[0].as_f64().unwrap(), r[1].as_f64().unwrap()).unwrap();
}

fn chip(name: &str) -> String {
    format!("[...document.querySelectorAll('#legend .chip')].find(b => b.textContent === '{name}')")
}

fn level(label: &str) -> String {
    format!("[...document.querySelectorAll('.rt')].find(b => b.textContent.endsWith('{label}'))")
}

fn wait_level(b: &mut Browser, label: &str) {
    b.wait_for(
        &format!("document.getElementById('lvl').textContent === '{label}' && document.getElementById('loading').hidden"),
        Duration::from_secs(5),
    )
    .unwrap();
}

fn visible(b: &mut Browser, id: &str) -> bool {
    b.eval(&format!("document.getElementById('{id}').checkVisibility()")).unwrap().as_bool().unwrap()
}

fn shown_chips(b: &mut Browser) -> Vec<String> {
    let v = b.eval("[...document.querySelectorAll('#legend .chip')].filter(c => c.checkVisibility()).map(c => c.textContent)").unwrap();
    v.as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect()
}

fn errors(b: &mut Browser) -> Vec<String> {
    let v = b.eval("window.ERRS").unwrap();
    v.as_array().unwrap().iter().map(|s| s.to_string()).collect()
}

fn drive(b: &mut Browser, root: &Path, (w, h): (u32, u32)) {
    let shot = |b: &mut Browser, name: &str| b.screenshot(&root.join(format!("{w}x{h}-{name}.png"))).unwrap();
    b.viewport(w, h).unwrap();
    b.open(&root.join("index.html")).unwrap();
    wait_level(b, "Map M1");
    shot(b, "map-only");
    assert_eq!(errors(b), Vec::<String>::new(), "{w}x{h}: opening");

    // Map drawn, "Map only" in the verdict's place, no timeline, only the game's layers.
    let verdict = b.eval("document.getElementById('verdict').textContent").unwrap();
    assert!(verdict.as_str().unwrap().starts_with("Map only"), "{verdict}");
    for id in ["scrub", "play", "speed", "zfollow", "evcard"] {
        assert!(!visible(b, id), "{w}x{h}: #{id} shows on a map-only level");
    }
    assert_eq!(shown_chips(b), ["Gems", "Everything else"], "{w}x{h}");
    let (m, c) = pixels(b);
    assert!(m > 0 && c > 0, "{w}x{h}: objects not drawn ({m} magenta, {c} cyan)");

    // Each layer's chip hides its objects and shows them again.
    click(b, &chip("Gems"));
    let (m0, c0) = pixels(b);
    assert_eq!((m0, c0), (0, c), "{w}x{h}: Gems off");
    click(b, &chip("Gems"));
    assert_eq!(pixels(b), (m, c), "{w}x{h}: Gems on");
    click(b, &chip("Everything else"));
    assert_eq!(pixels(b), (m, 0), "{w}x{h}: Everything else off");
    shot(b, "rest-off");
    click(b, &chip("Everything else"));
    assert_eq!(pixels(b), (m, c), "{w}x{h}: Everything else on");

    // The readout names the cell under the pointer.
    click(b, "document.getElementById('mapbox')");
    let readout = b.eval("document.getElementById('readout').textContent").unwrap();
    let readout = readout.as_str().unwrap();
    let cell = readout.strip_prefix("M1:").unwrap_or_else(|| panic!("{w}x{h}: readout {readout:?}"));
    let (col, row) = cell.split(" ·").next().unwrap().split_once(',').unwrap();
    let (col, row): (usize, usize) = (col.parse().unwrap(), row.parse().unwrap());
    assert!(col < COLS && row < ROWS, "{w}x{h}: {readout}");

    // To the routed level: the timeline and the route's layers come; back: they go.
    click(b, &level("Routed R1"));
    wait_level(b, "Routed R1");
    shot(b, "routed");
    for id in ["scrub", "play", "zfollow"] {
        assert!(visible(b, id), "{w}x{h}: #{id} missing on a routed level");
    }
    assert!(shown_chips(b).contains(&"Route line".to_string()));
    let readout = b.eval("document.getElementById('readout').textContent").unwrap();
    assert!(readout.as_str().unwrap().starts_with("frame 59 / 59"), "{readout}");
    click(b, &level("Map M1"));
    wait_level(b, "Map M1");
    assert!(!visible(b, "scrub") && !visible(b, "play"), "{w}x{h}: timeline left behind");
    assert_eq!(pixels(b), (m, c), "{w}x{h}: map-only level redrawn the same");

    // Nothing is clipped sideways, and the page raised nothing.
    let wide = b.eval("document.documentElement.scrollWidth <= innerWidth").unwrap();
    assert_eq!(wide.as_bool(), Some(true), "{w}x{h}: the page scrolls sideways");
    assert_eq!(errors(b), Vec::<String>::new(), "{w}x{h}");
}

#[test]
fn a_map_only_level_beside_a_routed_one() {
    let root = site();
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    drive(&mut b, &root, (1280, 800));
    drive(&mut b, &root, (800, 600));
    // The toolkit page asks for its web font; a game page that opens from disk drops the line.
    let blocked = b.blocked_requests();
    assert!(blocked.iter().all(|u| u.starts_with("https://fonts.googleapis.com/")), "{blocked:?}");
}
