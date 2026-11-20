//! The map page's live file (plans/forest-dumb-map-page DESIGN §2.5, §2.7, §2.9, §2.10, §3.4): every
//! [`LIVE_EVERY`]th step the program thread samples play (`fd_core::live`) and hands a
//! [`LiveFrame`] to [`LiveWriter`]'s thread, which writes `live.js` into the page's folder. The game
//! never waits on the file: the hand-over holds one frame, a newer one replacing it unsent, and a
//! write that fails is dropped, said once on stderr, and tried again with the next frame.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use amiga_disk::ofs::OfsImage;
use amiga_routeview::live::{LIVE_FILE, LiveFrame, write_live};
use fd_assets::{AssetError, Files};
use fd_core::live::{Live, Prev, live_from_ram, live_ranges};

/// Steps between two samples: about four a second at 50 a second (DESIGN §2.5).
pub const LIVE_EVERY: u32 = 12;

/// Where slow RAM starts in the A500's address space (`fd_core::recording`'s reader).
const SLOW_BASE: u32 = 0xC0_0000;

/// The page's folder: a scripted run's own `<home>/map`, so a test never writes the owner's page;
/// else `games/forest-dumb/build/map` of the main checkout this app was built in, as the bugs
/// folder is found (`fd_map::out_in`); else `<home>/map` (DESIGN §2.9).
pub fn live_dir(home: &Path, scripted: bool) -> PathBuf {
    if scripted {
        return home.join("map");
    }
    fd_map::out_in(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|| home.join("map"))
}

/// The page's frame from a sample, stamped `t_ms` (the writer's clock, never the core's).
pub fn frame(live: &Live, t_ms: u64) -> LiveFrame {
    LiveFrame {
        t_ms,
        playing: live.playing,
        level: live.world.map(fd_map::level_id),
        actor: live.char_xy.map(|(x, y)| (x as i32, y as i32)),
        cell: live.char_xy.map(|(x, y)| ((x / 16) as u16, (y / 16) as u16)),
        actors: live
            .creatures
            .iter()
            .map(|&(x, y, _)| (x, y, "enemy".to_string()))
            .collect(),
        cells: live.changed.clone(),
        counters: live
            .items_left
            .map(|n| ("items_left".to_string(), n as i64))
            .into_iter()
            .collect(),
    }
}

/// Milliseconds since the Unix epoch, for [`LiveFrame::t_ms`].
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// The bytes of the translated program's RAM [`live_from_ram`] reads, copied at a frame's end on the
/// program thread: `live_ranges()`'s ranges, each as its start and its bytes (DESIGN §3.4: never the
/// megabyte the hash reads).
pub type LiveBytes = Vec<(u32, Vec<u8>)>;

/// [`LiveBytes`] out of `chip` and `slow` for `ranges`.
pub fn copy_ranges(chip: &[u8], slow: &[u8], ranges: &[(u32, u32)]) -> LiveBytes {
    ranges
        .iter()
        .map(|&(a, b)| {
            let (mem, base) = if a >= SLOW_BASE { (slow, SLOW_BASE) } else { (chip, 0) };
            let (from, to) = ((a - base) as usize, (b - base) as usize);
            let mut bytes = mem.get(from..to.min(mem.len())).unwrap_or(&[]).to_vec();
            // Past the memory's end reads as 0, as the reader's own `ram_byte` does.
            bytes.resize(to - from, 0);
            (a, bytes)
        })
        .collect()
}

/// The translated program's samples, put back together on the app's side: the copied ranges laid
/// into RAM-shaped buffers (every other byte 0, which the reader never reads) and the last
/// sample, the next one's `prev` (DESIGN §2.7).
pub struct RamSampler {
    chip: Vec<u8>,
    slow: Vec<u8>,
    prev: Option<Prev>,
}

impl RamSampler {
    pub fn new() -> RamSampler {
        let ranges = live_ranges();
        let end = |slow: bool| {
            ranges
                .iter()
                .filter(|r| (r.0 >= SLOW_BASE) == slow)
                .map(|r| (r.1 - if slow { SLOW_BASE } else { 0 }) as usize)
                .max()
                .unwrap_or(0)
        };
        RamSampler {
            chip: vec![0; end(false)],
            slow: vec![0; end(true)],
            prev: None,
        }
    }

    /// The sample `bytes` holds; `start(world)` is that world's object map from its level file.
    pub fn live(&mut self, bytes: &LiveBytes, start: &dyn Fn(u8) -> Option<Vec<u8>>) -> Live {
        for (a, b) in bytes {
            let (mem, at) = match a.checked_sub(SLOW_BASE) {
                Some(i) => (&mut self.slow, i as usize),
                None => (&mut self.chip, *a as usize),
            };
            mem[at..at + b.len()].copy_from_slice(b);
        }
        let live = live_from_ram(&self.chip, &self.slow, self.prev, start);
        self.prev = Some(live.prev());
        live
    }
}

/// Where Show Map looks for Chrome: the page opens in Chrome whatever the default browser is
/// (DESIGN §2.8).
pub const CHROME: &str = "/Applications/Google Chrome.app";

/// What Game ▸ Show Map did.
#[derive(Debug, Clone, PartialEq, Eq)]
// The names the task's interface gives (map page T11).
#[allow(clippy::enum_variant_names)]
pub enum Opened {
    /// The page was written and Chrome asked to open it.
    Opened(PathBuf),
    /// A scripted run: the page was written, nothing opened.
    WouldOpen(PathBuf),
    /// What went wrong, in words for the owner.
    Failed(String),
}

/// Game ▸ Show Map (DESIGN §2.8): the six levels' page written fresh into `dir` from `files`
/// (disk 2's, unpacked) and `program`, then opened in Chrome; a scripted run opens nothing. Off
/// the game loop: it reads six levels and writes a few megabytes.
pub fn show_map(dir: &Path, files: &Files, program: &[u8], scripted: bool) -> Opened {
    show_map_in(dir, files, program, scripted, Path::new(CHROME))
}

fn show_map_in(dir: &Path, files: &Files, program: &[u8], scripted: bool, chrome: &Path) -> Opened {
    let page = match fd_map::write_site(dir, files, program, None) {
        Ok(p) => p,
        Err(e) => {
            return Opened::Failed(format!(
                "The map page could not be written to {}: {e}",
                dir.display()
            ));
        }
    };
    if scripted {
        return Opened::WouldOpen(page);
    }
    // Written first, so the owner can still open it by hand when Chrome is missing.
    if !chrome.exists() {
        return Opened::Failed(format!(
            "Google Chrome is not installed (no {}), and the map opens in Chrome. The page is at {}.",
            chrome.display(),
            page.display()
        ));
    }
    match std::process::Command::new("open")
        .arg("-a")
        .arg(chrome)
        .arg(&page)
        .status()
    {
        Ok(s) if s.success() => Opened::Opened(page),
        Ok(s) => Opened::Failed(format!(
            "Chrome did not open the map page {} (`open` exited {s}).",
            page.display()
        )),
        Err(e) => Opened::Failed(format!(
            "Chrome could not be started for the map page {}: {e}",
            page.display()
        )),
    }
}

/// Disk 2's files out of its ADF, unpacked, as the page reads them (`fd_routeview::write_map`'s
/// `disk2/` files, from the app's own copy of the disk rather than the zip again).
pub fn disk2_files(adf: &[u8]) -> Result<Box<Files<'_>>, String> {
    let image = OfsImage::parse(adf).map_err(|e| format!("disk 2's ADF: {e}"))?;
    Ok(Box::new(move |name: &str| {
        let packed = image
            .read_file(name)
            .map_err(|e| AssetError::missing(name, format!("disk 2: {e}")))?;
        fd_tools::unpack(&packed).map_err(|e| AssetError::missing(name, format!("disk 2: {e}")))
    }))
}

/// The hand-over between the program thread and the writer.
#[derive(Default)]
struct Slot {
    next: Option<LiveFrame>,
    closed: bool,
}

/// The writer thread and its one-frame hand-over.
pub struct LiveWriter {
    shared: Arc<(Mutex<Slot>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl LiveWriter {
    /// The writer for `dir`, made when the first frame is written (and again if removed).
    pub fn start(dir: PathBuf) -> Self {
        let shared = Arc::new((Mutex::new(Slot::default()), Condvar::new()));
        let theirs = shared.clone();
        let thread = thread::Builder::new()
            .name("live map writer".into())
            .spawn(move || write_loop(&dir, &theirs))
            .expect("a thread for the live map");
        LiveWriter {
            shared,
            thread: Some(thread),
        }
    }

    /// Hand `f` over, replacing a frame not yet written. Never waits on the file.
    pub fn send(&self, f: LiveFrame) {
        let (slot, ready) = &*self.shared;
        slot.lock().expect("live slot").next = Some(f);
        ready.notify_one();
    }

    /// Write the frame handed over last, then stop the thread: the run's last sample is on disk
    /// when the app exits. Nothing after the first call.
    pub fn finish(&mut self) {
        let Some(t) = self.thread.take() else { return };
        let (slot, ready) = &*self.shared;
        slot.lock().expect("live slot").closed = true;
        ready.notify_one();
        let _ = t.join();
    }
}

impl Drop for LiveWriter {
    fn drop(&mut self) {
        self.finish();
    }
}

fn write_loop(dir: &Path, shared: &(Mutex<Slot>, Condvar)) {
    let (slot, ready) = shared;
    // Said once until a write works again (DESIGN §2.10).
    let mut failing = false;
    loop {
        let frame = {
            let mut s = slot.lock().expect("live slot");
            loop {
                if let Some(f) = s.next.take() {
                    break f;
                }
                if s.closed {
                    return;
                }
                s = ready.wait(s).expect("live slot");
            }
        };
        match std::fs::create_dir_all(dir).and_then(|()| write_live(dir, &frame)) {
            Ok(()) => failing = false,
            Err(e) if !failing => {
                failing = true;
                eprintln!(
                    "fd-app: live map: cannot write {}: {e}; the game carries on, trying again with the next frame",
                    dir.join(LIVE_FILE).display()
                );
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scripted_run_writes_under_its_home() {
        assert_eq!(live_dir(Path::new("/h"), true), PathBuf::from("/h/map"));
        let owner = live_dir(Path::new("/h"), false);
        assert!(
            owner.ends_with("games/forest-dumb/build/map"),
            "{}",
            owner.display()
        );
    }

    #[test]
    fn show_map_says_what_stopped_it() {
        let dir = std::env::temp_dir().join(format!("fd-app-show-map-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let none = |name: &str| -> Result<Vec<u8>, AssetError> { Err(AssetError::missing(name, "made up")) };
        match show_map_in(&dir, &none, &[], true, Path::new("/nowhere")) {
            Opened::Failed(why) => assert!(
                why.contains(&dir.display().to_string()) && why.contains("_Gen1-1: made up"),
                "{why}"
            ),
            o => panic!("{o:?}"),
        }
        assert!(!dir.join("index.html").exists(), "nothing written");
    }

    /// On the owner's disks with no Chrome where it is looked for: the page is written all the
    /// same and the message names the missing app and the page.
    #[test]
    fn without_chrome_the_page_is_written_and_the_message_names_both() {
        let d = crate::disks::folder(None)
            .and_then(|d| crate::disks::read(&d))
            .unwrap_or_else(|e| panic!("{e}"));
        let files = disk2_files(&d.disk2).unwrap();
        let dir = std::env::temp_dir().join(format!("fd-app-no-chrome-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let opened = show_map_in(&dir, &*files, &d.program, false, Path::new("/nowhere/Chrome.app"));
        let page = dir.join("index.html");
        match opened {
            Opened::Failed(why) => assert!(
                why.contains("/nowhere/Chrome.app") && why.contains(&page.display().to_string()),
                "{why}"
            ),
            o => panic!("{o:?}"),
        }
        assert!(page.exists());
        let data: Vec<_> = std::fs::read_dir(dir.join("data")).unwrap().collect();
        assert_eq!(data.len(), 6, "six levels");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_sample_becomes_the_pages_frame() {
        let live = Live {
            playing: true,
            world: Some(2),
            char_xy: Some((0x7e, 0x79)),
            items_left: Some(272),
            creatures: vec![(200, 336, 4), (-16, 48, 7)],
            changed: vec![(3, 4, 0)],
            irq: 41,
            world_index: 1,
        };
        let f = frame(&live, 99);
        assert_eq!(f.t_ms, 99);
        assert_eq!(f.level.as_deref(), Some("W2-1"));
        assert_eq!(f.actor, Some((0x7e, 0x79)));
        assert_eq!(f.cell, Some((7, 7)));
        assert_eq!(
            f.actors,
            vec![(200, 336, "enemy".into()), (-16, 48, "enemy".into())]
        );
        assert_eq!(f.cells, vec![(3, 4, 0)]);
        assert_eq!(f.counters, vec![("items_left".into(), 272)]);
        let idle = frame(&Live::default(), 5);
        assert!(!idle.playing && idle.level.is_none() && idle.cell.is_none());
        assert!(idle.counters.is_empty());
    }

    #[test]
    fn copied_ranges_read_back_as_the_whole_ram_does() {
        let mut chip = vec![0u8; 512 << 10];
        let slow = vec![0u8; 512 << 10];
        for (i, b) in chip.iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
        let ranges = live_ranges();
        let bytes = copy_ranges(&chip, &slow, &ranges);
        let none = |_: u8| None;
        let whole = live_from_ram(&chip, &slow, Some(Prev::irq(0)), &none);
        let mut s = RamSampler::new();
        s.prev = Some(Prev::irq(0));
        assert_eq!(s.live(&bytes, &none), whole);
    }

    #[test]
    fn the_last_frame_handed_over_is_on_disk_after_finish() {
        let dir = std::env::temp_dir().join(format!("fd-app-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut w = LiveWriter::start(dir.join("map"));
        for t in 1..=50 {
            w.send(LiveFrame {
                t_ms: t,
                ..LiveFrame::default()
            });
        }
        w.finish();
        let text = std::fs::read_to_string(dir.join("map").join(LIVE_FILE)).unwrap();
        assert!(text.contains("\"t\":50"), "{text}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
