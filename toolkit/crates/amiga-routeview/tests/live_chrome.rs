//! The page's live mode in headless Chrome (amiga-browser), the test playing the game: it writes
//! `live.js` frames with `write_live` beside a made-up site of two map-only levels and checks what
//! the page draws and says. At 1280×800, 1920×1080 and 800×600: the character's dot at its pixel,
//! its cell and a counter in the status line, Follow scrolling to it and stopping on a hand scroll,
//! changed cells and creatures drawn, the level following play unless picked by hand, and the three
//! not-playing lines. A page with no live mode never asks for the file. Chrome missing fails, never
//! skips. Screens go to the test's temp folder.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use amiga_browser::{Browser, Opts};
use amiga_routeview::{Category, Entry, LevelDump, LiveFrame, LiveSpec, PageSpec, write_live, write_site};

const CELL: i32 = 8;

/// A map-only level, `cols` × `rows` cells of two greys, with coins at (12,5) and (14,5) and a
/// creature's home cell at (16,5).
struct Level {
    id: &'static str,
    cols: usize,
    rows: usize,
    /// More objects, each `,[c,r,value,"cat","name"]`.
    extra: &'static str,
}

impl LevelDump for Level {
    fn level_id(&self) -> &str {
        self.id
    }

    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> {
        let tiles: String = (0..64).map(|i| if (i / 8 + i) % 2 == 0 { "00" } else { "01" }).collect();
        write!(
            out,
            r##"{{"level":"{}","cols":{},"rows":{},"palette":["#404040","#808080"],"tiles":{{"0":"{tiles}"}},"body":[{}],"objects":[[12,5,32,"coin","coin"],[14,5,32,"coin","coin"],[16,5,7,"enemy","creature"]{}],"track":[],"coins_needed":2}}"##,
            self.id,
            self.cols,
            self.rows,
            vec!["0"; self.cols * self.rows].join(","),
            self.extra
        )
    }

    fn entry(&self) -> Entry {
        Entry {
            id: self.id.into(),
            label: format!("Map {}", self.id),
            group: "Made up".into(),
            route_file: None,
            verdict: None,
            wall_s: None,
            ref_frames: None,
        }
    }
}

/// Every script the page inserts, and its errors, collected from its first line on.
const CATCH: &str = "<script>window.ERRS=[];window.SRCS=[];addEventListener('error',e=>ERRS.push(String(e.message)));\
addEventListener('unhandledrejection',e=>ERRS.push(String(e.reason)));\
{const ce=console.error;console.error=(...a)=>{ERRS.push(a.join(' '));ce(...a)}}\
{const ap=Node.prototype.appendChild;Node.prototype.appendChild=function(n){if(n.tagName==='SCRIPT')SRCS.push(n.src);return ap.call(this,n)}}</script>";

fn spec(live: bool) -> PageSpec {
    PageSpec {
        title: "Made-up Game".into(),
        cell_size: (8, 8),
        categories: vec![
            Category::new("gems", "Gems", "#ff00ff", true, &["coin"]),
            Category::new("foes", "Foes", "#ffff00", true, &["enemy"]),
            Category::new("rest", "Everything else", "#00ffff", true, &[]),
            Category::new("secrets", "Secrets", "#c792ff", false, &["secret"]),
        ],
        actor: "hero".into(),
        anchor: (0, 0),
        cell_origin: (0, 0),
        live_box: (0, 0, 8, 8),
        live: live.then(|| LiveSpec::new(&[("items_left", "Items left")])),
    }
}

fn site(name: &str, live: bool) -> PathBuf {
    site_with(name, live, "", "")
}

/// The two-level site, L1 holding `extra` objects too, and `script` run before the page's own.
fn site_with(name: &str, live: bool, extra: &'static str, script: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    let l1 = Level { id: "L1", cols: 300, rows: 40, extra };
    let l2 = Level { id: "L2", cols: 40, rows: 20, extra: "" };
    write_site(&root, &spec(live), &[&l1, &l2]).unwrap();
    let index = root.join("index.html");
    let html = std::fs::read_to_string(&index).unwrap();
    std::fs::write(&index, html.replacen("<body>", &format!("<body>{CATCH}{script}"), 1)).unwrap();
    root
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

/// Playing `level` with the hero at cell (c, 5)'s middle.
fn at(level: &str, c: i32) -> LiveFrame {
    LiveFrame {
        t_ms: now_ms(),
        playing: true,
        level: Some(level.into()),
        actor: Some((c * CELL + 4, 5 * CELL + 4)),
        cell: Some((c as u16, 5)),
        actors: vec![],
        cells: vec![],
        counters: vec![("items_left".into(), 7)],
    }
}

fn eval_str(b: &mut Browser, js: &str) -> String {
    b.eval(js).unwrap().as_str().unwrap_or_default().to_string()
}

fn status(b: &mut Browser) -> String {
    eval_str(b, "document.getElementById('livebar').textContent")
}

/// Waits until the status line holds `text`.
fn wait_status(b: &mut Browser, text: &str, within: Duration) {
    let js = format!("document.getElementById('livebar').textContent.includes({text:?})");
    if b.wait_for(&js, within).is_err() {
        panic!("status never said {text:?}; it says {:?}", status(b));
    }
}

fn wait_level(b: &mut Browser, label: &str) {
    b.wait_for(
        &format!("document.getElementById('lvl').textContent === '{label}' && document.getElementById('loading').hidden"),
        Duration::from_secs(5),
    )
    .unwrap_or_else(|e| panic!("never showed {label}: {e}"));
}

/// The map canvas's colour at a level pixel, or null off the canvas.
fn rgb(b: &mut Browser, (x, y): (i32, i32)) -> Option<(u8, u8, u8)> {
    let v = b
        .eval(&format!(
            "(() => {{ const v = ROUTEVIEW.view(), c = document.getElementById('cv'), d = devicePixelRatio || 1; \
             const sx = Math.round(({x} * v.scale + v.ox) * d), sy = Math.round(({y} * v.scale + v.oy) * d); \
             if (sx < 0 || sy < 0 || sx >= c.width || sy >= c.height) return null; \
             return [...c.getContext('2d').getImageData(sx, sy, 1, 1).data].slice(0, 3); }})()"
        ))
        .unwrap();
    v.as_array().map(|a| (a[0].as_u64().unwrap() as u8, a[1].as_u64().unwrap() as u8, a[2].as_u64().unwrap() as u8))
}

const GREEN: (u8, u8, u8) = (0x7d, 0xff, 0x6a);
const MAGENTA: (u8, u8, u8) = (0xff, 0x00, 0xff);
const YELLOW: (u8, u8, u8) = (0xff, 0xff, 0x00);

fn cell_mid(c: i32, r: i32) -> (i32, i32) {
    (c * CELL + 4, r * CELL + 4)
}

/// Whether a level pixel is on the map's box, well inside it.
fn in_view(b: &mut Browser, (x, y): (i32, i32)) -> bool {
    b.eval(&format!(
        "(() => {{ const v = ROUTEVIEW.view(), m = document.getElementById('mapbox'); const sx = {x} * v.scale + v.ox, sy = {y} * v.scale + v.oy; \
         return sx > 0 && sy > 0 && sx < m.clientWidth && sy < m.clientHeight; }})()"
    ))
    .unwrap()
    .as_bool()
    .unwrap()
}

/// Waits until the page draws `want` at a level pixel (a frame read and drawn).
fn wait_rgb(b: &mut Browser, p: (i32, i32), want: Option<(u8, u8, u8)>, what: &str) {
    let until = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let got = rgb(b, p);
        if got == want {
            return;
        }
        if std::time::Instant::now() > until {
            panic!("{what}: at {p:?} {got:?}, wanted {want:?}");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_not_rgb(b: &mut Browser, p: (i32, i32), not: (u8, u8, u8), what: &str) {
    let until = std::time::Instant::now() + Duration::from_secs(3);
    while rgb(b, p) == Some(not) {
        assert!(std::time::Instant::now() < until, "{what}: still {not:?} at {p:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn click(b: &mut Browser, js: &str) {
    let r = b
        .eval(&format!(
            "(() => {{ const e = {js}; if (!e) return null; e.scrollIntoView({{block: 'center'}}); const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; }})()"
        ))
        .unwrap();
    assert!(!r.is_null(), "nothing found by {js}");
    b.click(r[0].as_f64().unwrap(), r[1].as_f64().unwrap()).unwrap();
}

fn errors(b: &mut Browser) -> Vec<String> {
    let v = b.eval("window.ERRS").unwrap();
    v.as_array().unwrap().iter().map(|s| s.to_string()).collect()
}

fn follow(b: &mut Browser) -> bool {
    b.eval("ROUTEVIEW.view().follow").unwrap().as_bool().unwrap()
}

/// Opens the live site on a frame of play: the dot at its pixel, the status naming the cell and
/// the counter.
fn open_playing(b: &mut Browser, root: &Path, (w, h): (u32, u32)) {
    write_live(root, &at("L1", 10)).unwrap();
    b.viewport(w, h).unwrap();
    b.open(&root.join("index.html")).unwrap();
    wait_level(b, "Map L1");
    wait_status(b, "hero at cell 10,5", Duration::from_secs(3));
    let s = status(b);
    assert!(s.contains("Playing Map L1") && s.contains("Items left 7"), "{w}x{h}: {s:?}");
    wait_rgb(b, cell_mid(10, 5), Some(GREEN), &format!("{w}x{h}: the dot"));
    assert!(follow(b), "{w}x{h}: Follow is on at load");
}

/// The live check smoke runs: one frame drawn at 1280×800, and a page with no live mode asks
/// for no file.
#[test]
fn a_live_frame_draws_the_character_and_a_page_without_live_asks_for_nothing() {
    let root = site("routeview-live-smoke", true);
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    open_playing(&mut b, &root, (1280, 800));
    assert_eq!(errors(&mut b), Vec::<String>::new());

    let quiet = site("routeview-live-none", false);
    write_live(&quiet, &at("L1", 10)).unwrap();
    b.open(&quiet.join("index.html")).unwrap();
    wait_level(&mut b, "Map L1");
    std::thread::sleep(Duration::from_millis(700));
    let srcs = b.eval("window.SRCS").unwrap();
    assert!(srcs.as_array().unwrap().iter().all(|s| !s.as_str().unwrap().contains("live.js")), "{srcs}");
    assert_eq!(b.eval("document.getElementById('livebar').hidden").unwrap().as_bool(), Some(true));
    assert_eq!(b.eval("document.getElementById('zfollow').checkVisibility()").unwrap().as_bool(), Some(false));
    assert_ne!(rgb(&mut b, cell_mid(10, 5)), Some(GREEN), "no dot without live mode");
    assert_eq!(errors(&mut b), Vec::<String>::new());
    let blocked = b.blocked_requests();
    assert!(blocked.iter().all(|u| u.starts_with("https://fonts.googleapis.com/")), "{blocked:?}");
}

fn drive(root: &Path, (w, h): (u32, u32)) {
    let mut b = Browser::launch(Opts { width: w, height: h, ..Opts::default() }).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    let b = &mut b;
    let shot = |b: &mut Browser, name: &str| b.screenshot(&root.join(format!("{w}x{h}-{name}.png"))).unwrap();
    let wh = format!("{w}x{h}");
    open_playing(b, root, (w, h));
    shot(b, "playing");

    // Changed cells: the coin at (12,5) taken, a coin's value at (18,5); a creature in view.
    wait_rgb(b, cell_mid(12, 5), Some(MAGENTA), &format!("{wh}: the coin before it is taken"));
    let mut f = at("L1", 10);
    f.cells = vec![(12, 5, 0), (18, 5, 32)];
    f.actors = vec![(150, 30, "enemy".into())];
    write_live(root, &f).unwrap();
    wait_rgb(b, (150, 30), Some(YELLOW), &format!("{wh}: the creature"));
    wait_not_rgb(b, cell_mid(12, 5), MAGENTA, &format!("{wh}: the taken coin"));
    assert_eq!(rgb(b, cell_mid(18, 5)), Some(MAGENTA), "{wh}: the coin's value drawn where the game put it");
    assert_eq!(rgb(b, cell_mid(14, 5)), Some(MAGENTA), "{wh}: an untouched coin");
    shot(b, "cells");
    f.actors = vec![(170, 30, "enemy".into())];
    write_live(root, &f).unwrap();
    wait_rgb(b, (170, 30), Some(YELLOW), &format!("{wh}: the creature moved"));
    assert_ne!(rgb(b, (150, 30)), Some(YELLOW), "{wh}: the creature's old place");

    // Follow: 200 cells on, the view goes with the dot.
    let far = cell_mid(210, 5);
    assert!(!in_view(b, far), "{wh}: cell 210 already in view: the test proves nothing");
    write_live(root, &at("L1", 210)).unwrap();
    wait_rgb(b, far, Some(GREEN), &format!("{wh}: followed the dot 200 cells"));
    shot(b, "followed");

    // A hand scroll turns Follow off: the next frame out of view does not move the view.
    let r = b.eval("(() => { const r = document.getElementById('mapbox').getBoundingClientRect(); return [r.x + r.width / 3, r.y + r.height / 2]; })()").unwrap();
    b.scroll(r[0].as_f64().unwrap(), r[1].as_f64().unwrap(), 0.0, 120.0).unwrap();
    b.wait_for("!ROUTEVIEW.view().follow", Duration::from_secs(2)).unwrap();
    assert_eq!(eval_str(b, "document.getElementById('zfollow').getAttribute('aria-pressed')"), "false");
    let view = b.eval("JSON.stringify(ROUTEVIEW.view())").unwrap();
    write_live(root, &at("L1", 40)).unwrap();
    wait_status(b, "hero at cell 40,5", Duration::from_secs(3));
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(b.eval("JSON.stringify(ROUTEVIEW.view())").unwrap(), view, "{wh}: the view moved with Follow off");
    assert!(!in_view(b, cell_mid(40, 5)), "{wh}: cell 40 in view anyway: the test proves nothing");
    click(b, "document.getElementById('zfollow')");
    assert!(follow(b), "{wh}: Follow back on");
    wait_rgb(b, cell_mid(40, 5), Some(GREEN), &format!("{wh}: Follow scrolled back to the dot"));

    // The level follows play; a hand pick holds until play changes level again.
    write_live(root, &at("L2", 3)).unwrap();
    wait_level(b, "Map L2");
    wait_rgb(b, cell_mid(3, 5), Some(GREEN), &format!("{wh}: the dot on L2"));
    assert_eq!(b.eval("document.getElementById('tip').hidden").unwrap().as_bool(), Some(true), "{wh}: a tip of the level left behind");
    shot(b, "switched");
    click(b, "[...document.querySelectorAll('.rt')].find(b => b.textContent.endsWith('Map L1'))");
    wait_level(b, "Map L1");
    write_live(root, &at("L2", 4)).unwrap();
    wait_status(b, "Playing Map L2 · showing Map L1", Duration::from_secs(3));
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(eval_str(b, "document.getElementById('lvl').textContent"), "Map L1", "{wh}: the hand pick held");
    write_live(root, &at("L1", 5)).unwrap();
    wait_status(b, "hero at cell 5,5", Duration::from_secs(3));
    write_live(root, &at("L2", 6)).unwrap();
    wait_level(b, "Map L2");

    // A level this map does not have: said, and no dot.
    write_live(root, &at("W9-9", 6)).unwrap();
    wait_status(b, "Playing a level this map does not have", Duration::from_secs(3));
    wait_not_rgb(b, cell_mid(6, 5), GREEN, &format!("{wh}: no dot on an unknown level"));
    shot(b, "unknown");

    // Not playing, then the app gone: by an old stamp, and by no file.
    write_live(root, &at("L2", 6)).unwrap();
    wait_rgb(b, cell_mid(6, 5), Some(GREEN), &format!("{wh}: the dot back"));
    write_live(root, &LiveFrame { t_ms: now_ms(), playing: false, ..LiveFrame::default() }).unwrap();
    wait_status(b, "Not playing", Duration::from_secs(3));
    wait_not_rgb(b, cell_mid(6, 5), GREEN, &format!("{wh}: no dot when not playing"));
    write_live(root, &at("L2", 6)).unwrap();
    wait_rgb(b, cell_mid(6, 5), Some(GREEN), &format!("{wh}: the dot again"));
    // Nothing written for 4 s: the frame's stamp is past 3 s.
    std::thread::sleep(Duration::from_secs(4));
    wait_status(b, "App not running", Duration::from_secs(1));
    assert_ne!(rgb(b, cell_mid(6, 5)), Some(GREEN), "{wh}: an old position drawn as current");
    shot(b, "stale");
    write_live(root, &at("L2", 6)).unwrap();
    wait_rgb(b, cell_mid(6, 5), Some(GREEN), &format!("{wh}: the dot once more"));
    std::fs::remove_file(root.join("live.js")).unwrap();
    wait_status(b, "App not running", Duration::from_secs(3));
    wait_not_rgb(b, cell_mid(6, 5), GREEN, &format!("{wh}: no dot with no file"));

    // Nothing clipped sideways, the status line inside the page, no error, nothing but files asked for.
    let wide = b.eval("document.documentElement.scrollWidth <= innerWidth").unwrap();
    assert_eq!(wide.as_bool(), Some(true), "{wh}: the page scrolls sideways");
    assert_eq!(errors(b), Vec::<String>::new(), "{wh}");
    let blocked = b.blocked_requests();
    assert!(blocked.iter().all(|u| u.starts_with("https://fonts.googleapis.com/")), "{wh}: {blocked:?}");
}

#[test]
fn live_play_at_three_sizes() {
    for size in [(1280, 800), (1920, 1080), (800, 600)] {
        drive(&site(&format!("routeview-live-{}x{}", size.0, size.1), true), size);
    }
}

/// Follow pressed on a map-only level while nothing is playing: no dot to go to and no error, and
/// the first dot is followed (a review find: the click once read the empty track).
#[test]
fn follow_with_nothing_playing_on_a_map_only_level() {
    let root = site("routeview-live-follow-idle", true);
    write_live(&root, &LiveFrame { t_ms: now_ms(), playing: false, ..LiveFrame::default() }).unwrap();
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    b.open(&root.join("index.html")).unwrap();
    wait_level(&mut b, "Map L1");
    wait_status(&mut b, "Not playing", Duration::from_secs(3));
    click(&mut b, "document.getElementById('zfollow')");
    assert!(!follow(&mut b), "Follow turned off by the click");
    click(&mut b, "document.getElementById('zfollow')");
    assert!(follow(&mut b), "Follow back on by a click");
    write_live(&root, &at("L1", 210)).unwrap();
    wait_rgb(&mut b, cell_mid(210, 5), Some(GREEN), "Follow went to the first dot");
    assert_eq!(errors(&mut b), Vec::<String>::new());
}

/// Holds back the page's insertion of L2's data script by 1.5 s, as a big level's data takes a
/// while to load beside a running app (map page T12's drill: W2-1 beside the translated app).
const SLOW_L2: &str = "<script>{const ap=Node.prototype.appendChild;Node.prototype.appendChild=function(n){\
if(n.tagName==='SCRIPT'&&/L2\\.js$/.test(n.src)&&!n.dataset.late){n.dataset.late=1;setTimeout(()=>ap.call(this,n),1500);return n}\
return ap.call(this,n)}}</script>";

/// Play moves to a level whose data is still loading while more frames arrive (map page T12's
/// drill): nothing of the new level is drawn over the old map meanwhile, and once it is shown the
/// view is zoomed in on the character as at load, not left at the whole level.
#[test]
fn a_live_switch_draws_nothing_on_the_old_map_and_zooms_in_once_loaded() {
    let root = site_with("routeview-live-slow-switch", true, "", SLOW_L2);
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    open_playing(&mut b, &root, (1280, 800));
    // L2's dot at cell (3,5), which is also a point of L1's map, where the old view is centred.
    let p = cell_mid(3, 5);
    let until = std::time::Instant::now() + Duration::from_secs(6);
    let mut loading_seen = 0;
    loop {
        write_live(&root, &at("L2", 3)).unwrap();
        let loading = !b.eval("document.getElementById('loading').hidden").unwrap().as_bool().unwrap();
        if loading {
            loading_seen += 1;
            let shown = b.eval("ROUTEVIEW.view().current").unwrap();
            assert_eq!(shown.as_str(), Some("L2"), "switching");
            assert_ne!(rgb(&mut b, p), Some(GREEN), "L2's character drawn on L1's map while L2 loads");
        } else if loading_seen > 0 {
            break;
        }
        assert!(std::time::Instant::now() < until, "L2 never loaded");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(loading_seen >= 5, "{loading_seen} looks at the loading level: the test proves nothing");
    wait_level(&mut b, "Map L2");
    write_live(&root, &at("L2", 3)).unwrap();
    wait_rgb(&mut b, p, Some(GREEN), "the dot on L2");
    let scale = b.eval("ROUTEVIEW.view().scale").unwrap().as_f64().unwrap();
    assert!(scale >= 1.5, "after a live switch the view is at {scale}, not zoomed in on the character");
    // Ten writes a second can be read torn (`a_torn_read_…`), which raises only this.
    let errs = errors(&mut b);
    assert!(errs.iter().all(|e| e == "\"Script error.\""), "{errs:?}");
}

/// Counts the canvas's pixels near `rgb` over level pixels `x0..x1`, `y0..y1`.
fn count_rgb(b: &mut Browser, (x0, y0, x1, y1): (i32, i32, i32, i32), (r, g, bl): (u8, u8, u8)) -> i64 {
    b.eval(&format!(
        "(() => {{ const v = ROUTEVIEW.view(), c = document.getElementById('cv'), d = devicePixelRatio || 1; \
         const sx = p => Math.round((p * v.scale + v.ox) * d), sy = p => Math.round((p * v.scale + v.oy) * d); \
         const a = Math.max(0, sx({x0})), t = Math.max(0, sy({y0})), z = Math.min(c.width, sx({x1})), w = Math.min(c.height, sy({y1})); \
         if (z <= a || w <= t) return -1; \
         const px = c.getContext('2d').getImageData(a, t, z - a, w - t).data; let n = 0; \
         for (let i = 0; i < px.length; i += 4) if (Math.abs(px[i] - {r}) < 8 && Math.abs(px[i+1] - {g}) < 8 && Math.abs(px[i+2] - {bl}) < 8) n++; return n; }})()"
    ))
    .unwrap()
    .as_i64()
    .unwrap()
}

/// A secret (value 0: no object-map value of its own) on a cell whose object the game took: the
/// game's change hides the object, never the secret's mark (map page T09's finding, W3-1 41,78).
#[test]
fn a_secret_keeps_its_mark_when_the_object_on_its_cell_is_taken() {
    let root = site_with("routeview-live-secret", true, r#",[12,5,0,"secret","hidden passage"]"#, "");
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    open_playing(&mut b, &root, (1280, 800));
    click(&mut b, "[...document.querySelectorAll('#legend .chip')].find(b => b.textContent === 'Secrets')");
    const SECRET: (u8, u8, u8) = (0xc7, 0x92, 0xff);
    let cell = (12 * CELL, 5 * CELL, 13 * CELL, 6 * CELL);
    let before = count_rgb(&mut b, cell, SECRET);
    assert!(before > 0, "no secret mark drawn at all: the test proves nothing");
    let mut f = at("L1", 10);
    f.cells = vec![(12, 5, 0)];
    write_live(&root, &f).unwrap();
    wait_not_rgb(&mut b, cell_mid(12, 5), MAGENTA, "the taken coin");
    let after = count_rgb(&mut b, cell, SECRET);
    assert!(after * 2 >= before, "the secret's mark went with the coin: {before} pixels before, {after} after");
    assert_eq!(errors(&mut b), Vec::<String>::new());
}

/// An exit of 2×3 cells is labelled EXIT once, not once a cell (map page T08's drill: six labels
/// overlapping into one smudge).
#[test]
fn an_exit_block_is_labelled_once() {
    let exit = r#",[20,2,255,"exit","level exit"],[21,2,255,"exit","level exit"],[20,3,255,"exit","level exit"],[21,3,255,"exit","level exit"],[20,4,255,"exit","level exit"],[21,4,255,"exit","level exit"]"#;
    let count = "<script>window.TEXTS=[];{const ft=CanvasRenderingContext2D.prototype.fillText;\
CanvasRenderingContext2D.prototype.fillText=function(t,...a){TEXTS.push(t);return ft.call(this,t,...a)}}</script>";
    let root = site_with("routeview-exit-label", true, exit, count);
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    open_playing(&mut b, &root, (1280, 800));
    b.eval("TEXTS.length = 0").unwrap();
    click(&mut b, "document.getElementById('zin')");
    let labels = b.eval("TEXTS.filter(t => t === 'EXIT').length").unwrap().as_i64().unwrap();
    assert_eq!(labels, 1, "one draw labelled the 2×3 exit {labels} times");
    assert_eq!(errors(&mut b), Vec::<String>::new());
}

/// A read of `live.js` that runs but sets no frame: Chrome now and then reads the file half
/// replaced despite the rename (map page T12: 1 read in about 950 with a writer replacing it
/// thousands of times a second), a syntax error the page cannot catch. The last frame stays until
/// it is stale; it is not "App not running" for a poll.
#[test]
fn a_torn_read_of_the_live_file_keeps_the_last_frame_until_it_is_stale() {
    let root = site("routeview-live-torn", true);
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    open_playing(&mut b, &root, (1280, 800));
    let tmp = root.join("live.js.tmp");
    std::fs::write(&tmp, "window.LIVE = {\"t\":").unwrap();
    std::fs::rename(&tmp, root.join("live.js")).unwrap();
    b.wait_for("ERRS.length > 0", Duration::from_secs(3)).expect("the torn file was never read");
    std::thread::sleep(Duration::from_millis(600));
    let s = status(&mut b);
    assert!(s.contains("hero at cell 10,5"), "a torn read dropped the frame: {s:?}");
    assert_eq!(rgb(&mut b, cell_mid(10, 5)), Some(GREEN), "the dot went with a torn read");
    // Still only the last frame: once its stamp is past 3 s, the app is gone.
    wait_status(&mut b, "App not running", Duration::from_secs(4));
    assert_ne!(rgb(&mut b, cell_mid(10, 5)), Some(GREEN), "an old position drawn as current");
    let errs = errors(&mut b);
    assert!(errs.iter().all(|e| e == "\"Script error.\""), "{errs:?}");
}

/// With the Secrets layer off (as at load) the heading's object count and the Level summary leave
/// the secrets out, so they give nothing away; switching the layer on counts them (owner, map page
/// T12's drill, 2026-11-20).
#[test]
fn secrets_are_left_out_of_the_counts_while_their_layer_is_off() {
    let root = site_with("routeview-secret-count", true, r#",[12,5,0,"secret","hidden passage"]"#, "");
    let mut b = Browser::launch(Opts::default()).unwrap_or_else(|e| panic!("amiga-browser: {e}"));
    open_playing(&mut b, &root, (1280, 800));
    let sub = |b: &mut Browser| eval_str(b, "document.getElementById('sub').textContent");
    let stats = |b: &mut Browser| eval_str(b, "document.getElementById('stats').textContent");
    assert_eq!(sub(&mut b), "map only · 3 objects", "Secrets off at load");
    assert!(!stats(&mut b).contains("Secrets"), "{}", stats(&mut b));
    let chip = "[...document.querySelectorAll('#legend .chip')].find(b => b.textContent === 'Secrets')";
    click(&mut b, chip);
    assert_eq!(sub(&mut b), "map only · 4 objects", "Secrets on");
    assert!(stats(&mut b).contains("Secrets1"), "{}", stats(&mut b));
    click(&mut b, chip);
    assert_eq!(sub(&mut b), "map only · 3 objects", "Secrets off again");
    assert_eq!(errors(&mut b), Vec::<String>::new());
}
