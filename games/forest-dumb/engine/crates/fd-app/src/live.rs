//! The map page's live file (plans/forest-dumb-map-page DESIGN §2.5, §2.7, §2.9, §2.10, §3.4): every
//! [`LIVE_EVERY`]th step the program thread samples play (`fd_core::live`) and hands a
//! [`LiveFrame`] to [`LiveWriter`]'s thread, which writes `live.js` into the page's folder. The game
//! never waits on the file: the hand-over holds one frame, a newer one replacing it unsent, and a
//! write that fails is dropped, said once on stderr, and tried again with the next frame.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use amiga_routeview::live::{LIVE_FILE, LiveFrame, write_live};
use fd_core::live::{Live, live_from_ram, live_ranges};

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
/// sample's `irq_count`, the next one's `prev_irq` (DESIGN §2.7).
pub struct RamSampler {
    chip: Vec<u8>,
    slow: Vec<u8>,
    prev_irq: Option<u16>,
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
            prev_irq: None,
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
        let live = live_from_ram(&self.chip, &self.slow, self.prev_irq, start);
        self.prev_irq = Some(live.irq);
        live
    }
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
    fn a_sample_becomes_the_pages_frame() {
        let live = Live {
            playing: true,
            world: Some(2),
            char_xy: Some((0x7e, 0x79)),
            items_left: Some(272),
            creatures: vec![(200, 336, 4), (-16, 48, 7)],
            changed: vec![(3, 4, 0)],
            irq: 41,
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
        let whole = live_from_ram(&chip, &slow, Some(0), &none);
        let mut s = RamSampler::new();
        s.prev_irq = Some(0);
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
