//! Scripted oracle runs and their cache outside git (DESIGN §3.5): `~/.cache/superfrog-re/traces/`.
//!
//! A trace is keyed by everything that decides its contents: the trace format, the vAmiga build
//! (commit and patch set), the Kickstart, the disk set, the start, the input script and the length.
//! Change any of them and the key changes, so a stale trace is never read. A file that is damaged
//! or holds another key is rebuilt. Files are written to a temporary name, then renamed.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sf_core::input::FrameInput;
use sf_disk::sha1_hex;
use sf_paula::PaulaWrite;

use crate::amiga::{Amiga, AmigaMemory, FrameCapture, Rgb12Frame, SLOW_BASE, Snapshot, hash_bytes};
use crate::config::{OracleConfig, OracleError};

/// Bump when the file layout or what a trace records changes.
const TRACE_FORMAT: u32 = 2;
const MAGIC: &[u8; 8] = b"SFTRACE\0";

/// Where a scripted run starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPoint {
    /// Switch on with disk 1 in df0.
    PowerOn,
    /// Switch on, then restore this snapshot.
    Snapshot(Snapshot),
}

impl SyncPoint {
    fn key(&self) -> String {
        match self {
            SyncPoint::PowerOn => "power-on".to_string(),
            // The held input is restored beside the bytes, which do not carry it (shim.cpp).
            SyncPoint::Snapshot(s) => format!(
                "snapshot {} {:02x} {:032x} {}",
                s.frame,
                s.input.joy.bits(),
                s.input.keys.0,
                sha1_hex(&s.bytes)
            ),
        }
    }
}

/// Inputs by frame, counted from the start of the run: each entry holds from its frame until the
/// next one. Before the first entry nothing is held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputScript {
    changes: Vec<(u32, FrameInput)>,
}

impl InputScript {
    pub fn new(mut changes: Vec<(u32, FrameInput)>) -> InputScript {
        changes.sort_by_key(|c| c.0);
        changes.dedup_by_key(|c| c.0);
        InputScript { changes }
    }

    /// Nothing held, ever.
    pub fn idle() -> InputScript {
        InputScript::default()
    }

    /// What is held on frame `frame` of the run.
    pub fn input_at(&self, frame: u32) -> FrameInput {
        match self.changes.partition_point(|c| c.0 <= frame) {
            0 => FrameInput::NONE,
            i => self.changes[i - 1].1,
        }
    }

    /// One line per change, `frame joy keys`, in hex: the script's identity in the cache key.
    pub fn canonical(&self) -> String {
        self.changes
            .iter()
            .map(|(f, i)| format!("{f} {:02x} {:032x}\n", i.joy.bits(), i.keys.0))
            .collect()
    }
}

/// One frame of a trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceFrame {
    pub capture: FrameCapture,
    /// [`AmigaMemory::hash`] after the frame.
    pub memory_hash: u64,
    /// The RAM bytes this frame changed, as (CPU address, new bytes) runs.
    pub memory_delta: Vec<(u32, Vec<u8>)>,
}

/// A scripted run: the RAM it started from and every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    /// The cache key (see [`trace_key`]).
    pub key: String,
    pub start_memory: AmigaMemory,
    pub frames: Vec<TraceFrame>,
}

impl Trace {
    /// The RAM after each frame, in order.
    pub fn memories(&self) -> impl Iterator<Item = AmigaMemory> + '_ {
        let mut mem = self.start_memory.clone();
        self.frames.iter().map(move |f| {
            for (addr, bytes) in &f.memory_delta {
                for (k, &b) in bytes.iter().enumerate() {
                    let a = addr + k as u32;
                    if a >= SLOW_BASE {
                        mem.slow[(a - SLOW_BASE) as usize] = b;
                    } else {
                        mem.chip[a as usize] = b;
                    }
                }
            }
            mem.clone()
        })
    }
}

/// How [`cached_trace_in`] got its trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheOutcome {
    Hit,
    /// No file for this key: the run was made and stored.
    Miss,
    /// A file was there but unreadable or for another key: the run was made again and stored.
    Rebuilt(String),
}

/// The cache key: a SHA-1 over everything that decides a trace's contents.
pub fn trace_key(
    build: &str,
    kickstart_sha1: &str,
    disks_sha1: &str,
    start: &SyncPoint,
    script: &InputScript,
    frames: u32,
) -> String {
    let text = format!(
        "trace format {TRACE_FORMAT}\nvamiga {build}\nkickstart {kickstart_sha1}\ndisks {disks_sha1}\n\
         start {}\nframes {frames}\nscript\n{}",
        start.key(),
        script.canonical()
    );
    sha1_hex(text.as_bytes())
}

/// [`cached_trace_in`] on `~/.cache/superfrog-re/traces/` (`SUPERFROG_CACHE` replaces the
/// `~/.cache/superfrog-re` part).
pub fn cached_trace(
    cfg: &OracleConfig,
    start: &SyncPoint,
    script: &InputScript,
    frames: u32,
) -> Result<Trace, OracleError> {
    let root = std::env::var_os("SUPERFROG_CACHE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/superfrog-re")
        });
    cached_trace_in(&root.join("traces"), cfg, start, script, frames).map(|(t, _)| t)
}

/// Run `frames` frames of `script` from `start`, or read the same run from `dir`.
pub fn cached_trace_in(
    dir: &Path,
    cfg: &OracleConfig,
    start: &SyncPoint,
    script: &InputScript,
    frames: u32,
) -> Result<(Trace, CacheOutcome), OracleError> {
    let (rom, disks) = cfg.load()?;
    let disks_sha1 = sha1_hex(
        disks
            .disks
            .iter()
            .map(|d| sha1_hex(&d.bytes))
            .collect::<Vec<_>>()
            .join(" ")
            .as_bytes(),
    );
    let key = trace_key(
        crate::BUILD,
        &sha1_hex(&rom),
        &disks_sha1,
        start,
        script,
        frames,
    );
    let path = dir.join(format!("{key}.trace"));
    let outcome = match fs::read(&path) {
        Err(_) => CacheOutcome::Miss,
        Ok(bytes) => match decode(&bytes, &key) {
            Ok(trace) => return Ok((trace, CacheOutcome::Hit)),
            Err(why) => CacheOutcome::Rebuilt(why),
        },
    };
    let trace = run(cfg, start, script, frames, key)?;
    store(dir, &path, &trace)?;
    Ok((trace, outcome))
}

fn run(
    cfg: &OracleConfig,
    start: &SyncPoint,
    script: &InputScript,
    frames: u32,
    key: String,
) -> Result<Trace, OracleError> {
    let mut amiga = Amiga::power_on(cfg)?;
    if let SyncPoint::Snapshot(s) = start {
        amiga.restore(s);
    }
    let start_memory = amiga.memory();
    let mut prev = start_memory.clone();
    let mut out = Vec::with_capacity(frames as usize);
    for i in 0..frames {
        let capture = amiga.step(script.input_at(i))?;
        let mem = amiga.memory();
        let memory_delta = delta(&prev, &mem);
        out.push(TraceFrame {
            capture,
            memory_hash: mem.hash(),
            memory_delta,
        });
        prev = mem;
    }
    Ok(Trace {
        key,
        start_memory,
        frames: out,
    })
}

/// The runs of bytes that differ, as (CPU address, new bytes); runs closer than 8 bytes merge.
fn delta(old: &AmigaMemory, new: &AmigaMemory) -> Vec<(u32, Vec<u8>)> {
    let mut out = Vec::new();
    for (base, a, b) in [
        (0u32, &old.chip, &new.chip),
        (SLOW_BASE, &old.slow, &new.slow),
    ] {
        let mut i = 0;
        while i < b.len() {
            if a.get(i) == Some(&b[i]) {
                i += 1;
                continue;
            }
            let start = i;
            let mut end = i + 1;
            let mut j = end;
            while j < b.len() && j - end < 8 {
                if a.get(j) != Some(&b[j]) {
                    end = j + 1;
                }
                j += 1;
            }
            out.push((base + start as u32, b[start..end].to_vec()));
            i = end;
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The file: MAGIC, format (u32), key length (u32), key, payload hash (u64), deflated payload.
// ---------------------------------------------------------------------------------------------

fn store(dir: &Path, path: &Path, trace: &Trace) -> Result<(), OracleError> {
    let cache = |e: std::io::Error| OracleError::Cache(format!("{}: {e}", dir.display()));
    fs::create_dir_all(dir).map_err(cache)?;
    let payload = miniz_oxide::deflate::compress_to_vec(&encode_payload(trace), 1);
    let mut file = Vec::with_capacity(payload.len() + 64);
    file.extend_from_slice(MAGIC);
    file.extend_from_slice(&TRACE_FORMAT.to_le_bytes());
    file.extend_from_slice(&(trace.key.len() as u32).to_le_bytes());
    file.extend_from_slice(trace.key.as_bytes());
    file.extend_from_slice(&hash_bytes(&payload, 0).to_le_bytes());
    file.extend_from_slice(&payload);
    // Unique per write, not only per process: parallel tests may store the same trace at once.
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let n = WRITES.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{}.{}.{n}.tmp", trace.key, std::process::id()));
    fs::write(&tmp, &file).map_err(cache)?;
    fs::rename(&tmp, path).map_err(cache)
}

fn decode(file: &[u8], key: &str) -> Result<Trace, String> {
    let mut r = Reader { b: file, at: 0 };
    if r.take(8)? != MAGIC {
        return Err("not a trace file".into());
    }
    let format = r.u32()?;
    if format != TRACE_FORMAT {
        return Err(format!("trace format {format}, expected {TRACE_FORMAT}"));
    }
    let n = r.u32()? as usize;
    let stored = r.take(n)?;
    if stored != key.as_bytes() {
        return Err("holds another key".into());
    }
    let hash = r.u64()?;
    let payload = &file[r.at..];
    if hash_bytes(payload, 0) != hash {
        return Err("checksum mismatch".into());
    }
    let raw =
        miniz_oxide::inflate::decompress_to_vec(payload).map_err(|e| format!("inflate: {e:?}"))?;
    decode_payload(&raw, key)
}

fn encode_payload(t: &Trace) -> Vec<u8> {
    let mut w = Vec::new();
    let bytes = |w: &mut Vec<u8>, b: &[u8]| {
        w.extend_from_slice(&(b.len() as u32).to_le_bytes());
        w.extend_from_slice(b);
    };
    bytes(&mut w, &t.start_memory.chip);
    bytes(&mut w, &t.start_memory.slow);
    w.extend_from_slice(&(t.frames.len() as u32).to_le_bytes());
    for f in &t.frames {
        let p = &f.capture.picture;
        for v in [p.left, p.top, p.width, p.height] {
            w.extend_from_slice(&v.to_le_bytes());
        }
        w.push(p.hires_detail as u8);
        for px in &p.pixels {
            w.extend_from_slice(&px.to_le_bytes());
        }
        w.extend_from_slice(&(f.capture.paula.len() as u32).to_le_bytes());
        for pw in &f.capture.paula {
            for v in [pw.reg, pw.value, pw.hpos, pw.vpos] {
                w.extend_from_slice(&v.to_le_bytes());
            }
        }
        w.extend_from_slice(&(f.capture.audio.len() as u32).to_le_bytes());
        for s in &f.capture.audio {
            w.extend_from_slice(&s.to_le_bytes());
        }
        w.extend_from_slice(&f.memory_hash.to_le_bytes());
        w.extend_from_slice(&(f.memory_delta.len() as u32).to_le_bytes());
        for (addr, b) in &f.memory_delta {
            w.extend_from_slice(&addr.to_le_bytes());
            bytes(&mut w, b);
        }
    }
    w
}

fn decode_payload(raw: &[u8], key: &str) -> Result<Trace, String> {
    let mut r = Reader { b: raw, at: 0 };
    let chip = r.bytes()?.to_vec();
    let slow = r.bytes()?.to_vec();
    let n = r.u32()?;
    let mut frames = Vec::with_capacity(n.min(1 << 20) as usize);
    for _ in 0..n {
        let (left, top, width, height) = (r.u16()?, r.u16()?, r.u16()?, r.u16()?);
        let hires_detail = r.take(1)?[0] != 0;
        let pixels = (0..width as usize * height as usize)
            .map(|_| r.u16())
            .collect::<Result<_, _>>()?;
        let np = r.u32()?;
        let paula = (0..np)
            .map(|_| {
                Ok(PaulaWrite {
                    reg: r.u16()?,
                    value: r.u16()?,
                    hpos: r.u16()?,
                    vpos: r.u16()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let na = r.u32()?;
        let audio = (0..na)
            .map(|_| r.u16().map(|v| v as i16))
            .collect::<Result<_, _>>()?;
        let memory_hash = r.u64()?;
        let nd = r.u32()?;
        let memory_delta = (0..nd)
            .map(|_| Ok((r.u32()?, r.bytes()?.to_vec())))
            .collect::<Result<Vec<_>, String>>()?;
        let picture = Rgb12Frame {
            left,
            top,
            width,
            height,
            pixels,
            hires_detail,
        };
        frames.push(TraceFrame {
            capture: FrameCapture {
                picture,
                paula,
                audio,
            },
            memory_hash,
            memory_delta,
        });
    }
    if r.at != raw.len() {
        return Err("trailing bytes".into());
    }
    Ok(Trace {
        key: key.to_string(),
        start_memory: AmigaMemory { chip, slow },
        frames,
    })
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&e| e <= self.b.len())
            .ok_or("truncated")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn bytes(&mut self) -> Result<&'a [u8], String> {
        let n = self.u32()? as usize;
        self.take(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::input::{Joy, KeySet};

    #[test]
    fn a_script_holds_each_input_until_the_next() {
        let fire = FrameInput::joy(Joy::FIRE);
        let s = InputScript::new(vec![(10, fire), (3, FrameInput::key(0x19))]);
        assert_eq!(s.input_at(0), FrameInput::NONE);
        assert_eq!(s.input_at(3), FrameInput::key(0x19));
        assert_eq!(s.input_at(9), FrameInput::key(0x19));
        assert_eq!(s.input_at(10), fire);
        assert_eq!(s.input_at(10_000), fire);
        assert_eq!(
            s.canonical(),
            format!("3 00 {:032x}\n10 10 {:032x}\n", 1u128 << 0x19, 0)
        );
        assert_eq!(KeySet::key(0x19).0, 1u128 << 0x19);
    }

    #[test]
    fn the_key_changes_with_every_input() {
        let s = InputScript::idle();
        let base = trace_key("c-p", "rom", "disks", &SyncPoint::PowerOn, &s, 10);
        assert_eq!(
            base,
            trace_key("c-p", "rom", "disks", &SyncPoint::PowerOn, &s, 10)
        );
        let other_script = InputScript::new(vec![(0, FrameInput::joy(Joy::FIRE))]);
        let snap = SyncPoint::Snapshot(Snapshot {
            frame: 5,
            input: FrameInput::NONE,
            bytes: vec![1, 2],
        });
        for k in [
            trace_key("c2-p", "rom", "disks", &SyncPoint::PowerOn, &s, 10),
            trace_key("c-p2", "rom", "disks", &SyncPoint::PowerOn, &s, 10),
            trace_key("c-p", "rom2", "disks", &SyncPoint::PowerOn, &s, 10),
            trace_key("c-p", "rom", "disks2", &SyncPoint::PowerOn, &s, 10),
            trace_key("c-p", "rom", "disks", &snap, &s, 10),
            trace_key(
                "c-p",
                "rom",
                "disks",
                &SyncPoint::PowerOn,
                &other_script,
                10,
            ),
            trace_key("c-p", "rom", "disks", &SyncPoint::PowerOn, &s, 11),
        ] {
            assert_ne!(k, base);
        }
        // A snapshot start is keyed by its frame and by the input then held, which restore
        // re-seats and the snapshot's bytes do not carry, as well as by its bytes.
        let snap_key = |f: fn(&mut Snapshot)| {
            let mut sn = Snapshot {
                frame: 5,
                input: FrameInput::NONE,
                bytes: vec![1, 2],
            };
            f(&mut sn);
            trace_key("c-p", "rom", "disks", &SyncPoint::Snapshot(sn), &s, 10)
        };
        let snap_base = snap_key(|_| {});
        for k in [
            snap_key(|sn| sn.frame = 6),
            snap_key(|sn| sn.bytes = vec![1, 3]),
            snap_key(|sn| sn.input = FrameInput::key(0x19)),
            snap_key(|sn| sn.input = FrameInput::joy(Joy::FIRE)),
        ] {
            assert_ne!(k, snap_base);
        }
    }

    #[test]
    fn threads_storing_the_same_trace_all_succeed() {
        // Parallel tests in one process can make the same run at once; each write needs its own
        // temporary file, or one thread renames another's away and its store fails.
        let dir = std::env::temp_dir().join(format!("sf-trace-race-{}", std::process::id()));
        let t = sample();
        let path = dir.join("k.trace");
        let failed: usize = (0..4)
            .map(|_| {
                let (dir, path, t) = (dir.clone(), path.clone(), t.clone());
                std::thread::spawn(move || {
                    (0..100).filter(|_| store(&dir, &path, &t).is_err()).count()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum();
        assert_eq!(failed, 0, "{failed} of 400 concurrent stores failed");
        assert_eq!(decode(&fs::read(&path).unwrap(), "k").unwrap(), t);
        fs::remove_dir_all(&dir).unwrap();
    }

    fn sample() -> Trace {
        let mem = AmigaMemory {
            chip: vec![0; 64],
            slow: vec![0; 32],
        };
        let mut after = mem.clone();
        after.chip[3] = 9;
        after.chip[5] = 7;
        after.slow[31] = 1;
        let capture = FrameCapture {
            picture: Rgb12Frame {
                left: 1,
                top: 2,
                width: 2,
                height: 1,
                pixels: vec![0xF00, 0x00F],
                hires_detail: false,
            },
            paula: vec![PaulaWrite {
                reg: 0xA8,
                value: 64,
                hpos: 3,
                vpos: 4,
            }],
            audio: vec![-5],
        };
        let memory_delta = delta(&mem, &after);
        Trace {
            key: "k".into(),
            start_memory: mem,
            frames: vec![TraceFrame {
                capture,
                memory_hash: after.hash(),
                memory_delta,
            }],
        }
    }

    #[test]
    fn delta_runs_merge_and_replay() {
        let t = sample();
        assert_eq!(
            t.frames[0].memory_delta,
            vec![(3, vec![9, 0, 7]), (SLOW_BASE + 31, vec![1])]
        );
        let after = t.memories().next().unwrap();
        assert_eq!(after.hash(), t.frames[0].memory_hash);
    }

    #[test]
    fn a_stored_trace_reads_back_and_damage_is_seen() {
        let dir = std::env::temp_dir().join(format!("sf-trace-unit-{}", std::process::id()));
        let t = sample();
        let path = dir.join("k.trace");
        store(&dir, &path, &t).unwrap();
        let file = fs::read(&path).unwrap();
        assert_eq!(decode(&file, "k").unwrap(), t);
        assert!(decode(&file, "other").unwrap_err().contains("another key"));
        let mut bad = file.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xFF;
        assert!(decode(&bad, "k").is_err());
        assert!(decode(&file[..file.len() / 2], "k").is_err());
        assert!(decode(b"garbage", "k").is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
