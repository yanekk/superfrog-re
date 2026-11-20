//! A page opened from disk in headless Chrome, driven by a test: evaluate JavaScript, set the
//! viewport, click and scroll with real input events, take screenshots. Every request that is not
//! `file://` is failed and listed, the whole session has a hard time limit, and dropping a
//! [`Browser`] kills Chrome's process group and removes its temporary profile.
//!
//! Chrome speaks the DevTools protocol over `--remote-debugging-pipe`: it reads commands on fd 3 and
//! writes replies and events on fd 4, each message JSON ended by a NUL byte. Events are handled while
//! any call waits, so a paused request never stalls the page.

use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAC_CHROME: &str = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

/// Chrome's executable: `$CHROME` when set, else the macOS install. The error names where it looked.
pub fn chrome() -> Result<PathBuf, String> {
    let looked = match std::env::var_os("CHROME") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(MAC_CHROME),
    };
    if looked.is_file() {
        Ok(looked)
    } else {
        Err(format!(
            "Chrome not found at {} (set CHROME to its executable)",
            looked.display()
        ))
    }
}

/// How a session runs: its whole life is bounded by `limit`, and the page starts at `width`×`height`.
#[derive(Clone, Debug)]
pub struct Opts {
    pub limit: Duration,
    pub width: u32,
    pub height: u32,
}

impl Default for Opts {
    fn default() -> Opts {
        Opts {
            limit: Duration::from_secs(30),
            width: 1280,
            height: 800,
        }
    }
}

enum Msg {
    Json(Value),
    Closed,
}

/// Shared with the backstop thread. `done` is set, under the lock, before Chrome is reaped, so the
/// backstop never signals a process group whose id could have been reused.
struct Watch {
    done: Mutex<bool>,
    wake: Condvar,
}

/// One headless Chrome with one page.
pub struct Browser {
    child: Child,
    pgid: i32,
    profile: PathBuf,
    to_chrome: File,
    from_chrome: Receiver<Msg>,
    reader: Option<JoinHandle<()>>,
    watch: Arc<Watch>,
    backstop: Option<JoinHandle<()>>,
    deadline: Instant,
    limit: Duration,
    session: String,
    next_id: u64,
    loaded: HashSet<String>,
    blocked: Vec<String>,
    dead: Option<String>,
}

static SESSIONS: AtomicU64 = AtomicU64::new(0);

/// A pipe whose two ends are both above fd 4 and close on exec: `dup2` onto 3 and 4 in the child
/// would otherwise close an end that already sits there (FINDINGS, plan review probe).
fn high_pipe() -> Result<(i32, i32), String> {
    let mut fds = [0i32; 2];
    // SAFETY: `fds` is a valid two-int buffer; each fd is duplicated then its original closed.
    unsafe {
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            return Err(format!("pipe: {}", std::io::Error::last_os_error()));
        }
        let mut high = [0i32; 2];
        for (i, fd) in fds.iter().enumerate() {
            high[i] = libc::fcntl(*fd, libc::F_DUPFD_CLOEXEC, 10);
            libc::close(*fd);
            if high[i] < 0 {
                return Err(format!("fcntl: {}", std::io::Error::last_os_error()));
            }
        }
        Ok((high[0], high[1]))
    }
}

fn temp_profile() -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = SESSIONS.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "amiga-browser-{}-{}-{}",
        std::process::id(),
        n,
        nanos
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    Ok(dir)
}

impl Browser {
    /// Starts Chrome headless with a fresh profile, opens one blank page at the size `opts` names,
    /// and starts failing every request that is not `file://`.
    pub fn launch(opts: Opts) -> Result<Browser, String> {
        let exe = chrome()?;
        let started = Instant::now();
        let profile = temp_profile()?;
        let (to_read, to_write) = high_pipe()?; // Chrome reads commands from `to_read`
        let (from_read, from_write) = high_pipe()?; // Chrome writes replies to `from_write`
        let log = File::create(profile.join("chrome.log"))
            .map_err(|e| format!("creating Chrome's log: {e}"))?;
        let mut cmd = Command::new(&exe);
        cmd.arg("--headless=new")
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("--remote-debugging-pipe")
            .arg(format!("--window-size={},{}", opts.width, opts.height))
            .args([
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-sync",
                "--disable-extensions",
                "--disable-default-apps",
                "--disable-crash-reporter",
                "--disable-breakpad",
                "--metrics-recording-only",
                "--password-store=basic",
                "--use-mock-keychain",
                // A second fence behind the Fetch block: no host name resolves.
                "--host-resolver-rules=MAP * ~NOTFOUND",
                "about:blank",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .process_group(0);
        // SAFETY: only async-signal-safe calls between fork and exec. dup2 clears close-on-exec on
        // the new fds, so Chrome inherits exactly 3 and 4 of these pipes.
        unsafe {
            cmd.pre_exec(move || {
                if libc::dup2(to_read, 3) < 0 || libc::dup2(from_write, 4) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let spawned = cmd.spawn();
        // SAFETY: the child holds its own copies; the parent no longer needs these ends.
        unsafe {
            libc::close(to_read);
            libc::close(from_write);
        }
        let child = match spawned {
            Ok(child) => child,
            Err(e) => {
                // SAFETY: the parent's ends are ours and unused.
                unsafe {
                    libc::close(to_write);
                    libc::close(from_read);
                }
                let _ = std::fs::remove_dir_all(&profile);
                return Err(format!("starting {}: {e}", exe.display()));
            }
        };
        let pgid = child.id() as i32;
        // SAFETY: each fd is owned by exactly one File from here on.
        let to_chrome = unsafe { File::from_raw_fd(to_write) };
        let mut from = unsafe { File::from_raw_fd(from_read) };
        let (tx, rx) = channel();
        let reader = std::thread::spawn(move || {
            let mut pending = Vec::new();
            let mut buf = vec![0u8; 1 << 16];
            loop {
                match from.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        pending.extend_from_slice(&buf[..n]);
                        while let Some(end) = pending.iter().position(|&b| b == 0) {
                            let message: Vec<u8> = pending.drain(..=end).collect();
                            if let Ok(v) = serde_json::from_slice(&message[..end])
                                && tx.send(Msg::Json(v)).is_err()
                            {
                                return;
                            }
                        }
                    }
                }
            }
            let _ = tx.send(Msg::Closed);
        });
        let deadline = started + opts.limit;
        let watch = Arc::new(Watch {
            done: Mutex::new(false),
            wake: Condvar::new(),
        });
        let backstop = {
            let watch = Arc::clone(&watch);
            // A second later than the deadline the calls themselves keep, so a call reports the
            // limit first; this catches a test stuck outside the rig.
            let at = deadline + Duration::from_secs(1);
            std::thread::spawn(move || {
                let mut done = watch.done.lock().unwrap();
                while !*done {
                    let now = Instant::now();
                    if now >= at {
                        // SAFETY: the group leader is not yet reaped (`done` is false), so the id
                        // is still Chrome's.
                        unsafe { libc::kill(-pgid, libc::SIGKILL) };
                        return;
                    }
                    done = watch.wake.wait_timeout(done, at - now).unwrap().0;
                }
            })
        };
        let mut browser = Browser {
            child,
            pgid,
            profile,
            to_chrome,
            from_chrome: rx,
            reader: Some(reader),
            watch,
            backstop: Some(backstop),
            deadline,
            limit: opts.limit,
            session: String::new(),
            next_id: 0,
            loaded: HashSet::new(),
            blocked: Vec::new(),
            dead: None,
        };
        browser
            .attach(opts.width, opts.height)
            .map_err(|e| browser.with_log(e))?;
        Ok(browser)
    }

    fn attach(&mut self, width: u32, height: u32) -> Result<(), String> {
        let target = self.call_browser("Target.createTarget", json!({"url": "about:blank"}))?;
        let target_id = target["targetId"]
            .as_str()
            .ok_or("no targetId")?
            .to_string();
        let attached = self.call_browser(
            "Target.attachToTarget",
            json!({"targetId": target_id, "flatten": true}),
        )?;
        self.session = attached["sessionId"]
            .as_str()
            .ok_or("no sessionId")?
            .to_string();
        self.call("Page.enable", json!({}))?;
        self.call("Page.setLifecycleEventsEnabled", json!({"enabled": true}))?;
        self.call("Runtime.enable", json!({}))?;
        self.call("Fetch.enable", json!({"patterns": [{"urlPattern": "*"}]}))?;
        self.viewport(width, height)
    }

    /// Chrome's process id, which is also its process group's id.
    pub fn pid(&self) -> u32 {
        self.pgid as u32
    }

    /// The temporary profile directory, removed on drop.
    pub fn profile(&self) -> &Path {
        &self.profile
    }

    /// Opens a page from disk and waits for its load event.
    pub fn open(&mut self, page: &Path) -> Result<(), String> {
        let abs = std::fs::canonicalize(page).map_err(|e| format!("{}: {e}", page.display()))?;
        let url = file_url(&abs);
        let nav = self.call("Page.navigate", json!({"url": url}))?;
        if let Some(err) = nav["errorText"].as_str() {
            return Err(format!("opening {url}: {err}"));
        }
        let loader = nav["loaderId"].as_str().unwrap_or_default().to_string();
        let deadline = self.deadline;
        self.pump_until(|b| b.loaded.contains(&loader), deadline)?;
        Ok(())
    }

    /// Sets the page's size in CSS pixels, at one device pixel each.
    pub fn viewport(&mut self, w: u32, h: u32) -> Result<(), String> {
        self.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width": w, "height": h, "deviceScaleFactor": 1, "mobile": false}),
        )?;
        Ok(())
    }

    /// Evaluates an expression in the page, awaiting a promise, and returns its value as JSON
    /// (`null` for undefined). A thrown exception is an error carrying its text.
    pub fn eval(&mut self, js: &str) -> Result<Value, String> {
        let out = self.call(
            "Runtime.evaluate",
            json!({"expression": js, "returnByValue": true, "awaitPromise": true}),
        )?;
        if let Some(ex) = out.get("exceptionDetails") {
            let text = ex["exception"]["description"]
                .as_str()
                .or(ex["text"].as_str())
                .unwrap_or("exception");
            return Err(format!("{js}: {text}"));
        }
        Ok(out["result"].get("value").cloned().unwrap_or(Value::Null))
    }

    /// Evaluates `js` until it is truthy, at most `within`, and returns the truthy value.
    pub fn wait_for(&mut self, js: &str, within: Duration) -> Result<Value, String> {
        let until = (Instant::now() + within).min(self.deadline);
        loop {
            let v = self.eval(js)?;
            if truthy(&v) {
                return Ok(v);
            }
            if Instant::now() >= until {
                return Err(format!("{js}: not true within {within:?} (last {v})"));
            }
            let next = (Instant::now() + Duration::from_millis(50)).min(until);
            self.pump_until(|_| false, next)?;
        }
    }

    /// Writes the viewport as a PNG.
    pub fn screenshot(&mut self, png: &Path) -> Result<(), String> {
        let out = self.call("Page.captureScreenshot", json!({"format": "png"}))?;
        let data = out["data"].as_str().ok_or("no screenshot data")?;
        let bytes = base64_decode(data)?;
        std::fs::write(png, bytes).map_err(|e| format!("{}: {e}", png.display()))
    }

    /// A left click at a viewport point: move, press, release, as a mouse would.
    pub fn click(&mut self, x: f64, y: f64) -> Result<(), String> {
        self.mouse(json!({"type": "mouseMoved", "x": x, "y": y}))?;
        for kind in ["mousePressed", "mouseReleased"] {
            self.mouse(json!({"type": kind, "x": x, "y": y, "button": "left", "clickCount": 1}))?;
        }
        Ok(())
    }

    /// A wheel turn at a viewport point, by `dx`, `dy` pixels.
    pub fn scroll(&mut self, x: f64, y: f64, dx: f64, dy: f64) -> Result<(), String> {
        self.mouse(json!({"type": "mouseWheel", "x": x, "y": y, "deltaX": dx, "deltaY": dy}))
    }

    /// Every non-`file://` URL the page asked for, in order; each was failed.
    pub fn blocked_requests(&self) -> Vec<String> {
        self.blocked.clone()
    }

    fn mouse(&mut self, params: Value) -> Result<(), String> {
        self.call("Input.dispatchMouseEvent", params).map(|_| ())
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let session = self.session.clone();
        self.call_in(method, params, Some(&session))
    }

    fn call_browser(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.call_in(method, params, None)
    }

    fn call_in(
        &mut self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<Value, String> {
        let id = self.send(method, params, session)?;
        let mut reply = None;
        let deadline = self.deadline;
        self.pump_with(
            |_, msg| {
                if msg["id"].as_u64() == Some(id) {
                    reply = Some(msg.clone());
                    true
                } else {
                    false
                }
            },
            deadline,
        )?;
        let reply = reply.unwrap();
        if let Some(err) = reply.get("error") {
            return Err(format!(
                "{method}: {}",
                err["message"].as_str().unwrap_or("error")
            ));
        }
        Ok(reply["result"].clone())
    }

    fn send(&mut self, method: &str, params: Value, session: Option<&str>) -> Result<u64, String> {
        if let Some(why) = &self.dead {
            return Err(why.clone());
        }
        self.next_id += 1;
        let mut msg = json!({"id": self.next_id, "method": method, "params": params});
        if let Some(s) = session {
            msg["sessionId"] = json!(s);
        }
        let mut bytes = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
        bytes.push(0);
        if let Err(e) = self.to_chrome.write_all(&bytes) {
            return Err(self.die(format!("writing to Chrome: {e}")));
        }
        Ok(self.next_id)
    }

    /// Handles messages until `pred` holds after one or `until` passes (Ok(false)). Passing the
    /// session's deadline kills Chrome and is an error.
    fn pump_until(
        &mut self,
        mut pred: impl FnMut(&Browser) -> bool,
        until: Instant,
    ) -> Result<bool, String> {
        if pred(self) {
            return Ok(true);
        }
        self.pump_with(|b, _| pred(b), until)
    }

    fn pump_with(
        &mut self,
        mut stop: impl FnMut(&Browser, &Value) -> bool,
        until: Instant,
    ) -> Result<bool, String> {
        loop {
            if let Some(why) = &self.dead {
                return Err(why.clone());
            }
            let now = Instant::now();
            if now >= self.deadline {
                let limit = self.limit;
                return Err(self.die(format!("time limit of {limit:?} reached; Chrome killed")));
            }
            if now >= until {
                return Ok(false);
            }
            match self
                .from_chrome
                .recv_timeout(until.min(self.deadline) - now)
            {
                Ok(Msg::Json(msg)) => {
                    self.handle(&msg)?;
                    if stop(self, &msg) {
                        return Ok(true);
                    }
                }
                Ok(Msg::Closed) | Err(RecvTimeoutError::Disconnected) => {
                    let why = if Instant::now() >= self.deadline {
                        format!("time limit of {:?} reached; Chrome killed", self.limit)
                    } else {
                        "Chrome closed its pipe".to_string()
                    };
                    let why = self.die(why);
                    return Err(self.with_log(why));
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    fn handle(&mut self, msg: &Value) -> Result<(), String> {
        match msg["method"].as_str() {
            Some("Fetch.requestPaused") => {
                let p = &msg["params"];
                let url = p["request"]["url"].as_str().unwrap_or_default().to_string();
                let request_id = p["requestId"].clone();
                let session = msg["sessionId"].as_str().map(str::to_string);
                // Replies to these are not awaited: they arrive later and match no call.
                if url.starts_with("file:") {
                    self.send(
                        "Fetch.continueRequest",
                        json!({"requestId": request_id}),
                        session.as_deref(),
                    )?;
                } else {
                    self.blocked.push(url);
                    self.send(
                        "Fetch.failRequest",
                        json!({"requestId": request_id, "errorReason": "BlockedByClient"}),
                        session.as_deref(),
                    )?;
                }
            }
            Some("Page.lifecycleEvent") if msg["params"]["name"] == "load" => {
                if let Some(loader) = msg["params"]["loaderId"].as_str() {
                    self.loaded.insert(loader.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Kills Chrome's group, reaps it and remembers why; every later call returns `why`.
    fn die(&mut self, why: String) -> String {
        if self.dead.is_none() {
            self.kill_group();
            self.dead = Some(why.clone());
        }
        why
    }

    fn kill_group(&mut self) {
        {
            let mut done = self.watch.done.lock().unwrap();
            if !*done {
                // SAFETY: Chrome is not reaped yet, so the group id is still its own.
                unsafe { libc::kill(-self.pgid, libc::SIGKILL) };
                *done = true;
            }
            self.watch.wake.notify_all();
        }
        let _ = self.child.wait();
        if let Some(h) = self.backstop.take() {
            let _ = h.join();
        }
    }

    fn with_log(&self, why: String) -> String {
        let log = std::fs::read_to_string(self.profile.join("chrome.log")).unwrap_or_default();
        let tail: Vec<&str> = log.lines().rev().take(5).collect();
        if tail.is_empty() {
            why
        } else {
            let tail: Vec<&str> = tail.into_iter().rev().collect();
            format!("{why}\nChrome's log ends:\n{}", tail.join("\n"))
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        self.kill_group();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        // Helpers outside the group may still be closing files in the profile for a moment.
        for _ in 0..20 {
            if std::fs::remove_dir_all(&self.profile).is_ok() || !self.profile.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// JavaScript's truthiness, as far as JSON carries it (NaN arrives as null).
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `file://` and the absolute path, every byte but the unreserved set and `/` percent-encoded.
pub fn file_url(abs: &Path) -> String {
    let mut url = String::from("file://");
    for &b in abs.as_os_str().as_encoded_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            url.push(b as char);
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    url
}

fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' => continue,
            _ => return Err(format!("not base64: {:?}", c as char)),
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decodes_with_and_without_padding() {
        assert_eq!(base64_decode("TWFu").unwrap(), b"Man");
        assert_eq!(base64_decode("TWE=").unwrap(), b"Ma");
        assert_eq!(base64_decode("TQ==").unwrap(), b"M");
        assert!(base64_decode("T*").is_err());
    }

    #[test]
    fn file_urls_encode_spaces_and_keep_slashes() {
        assert_eq!(
            file_url(Path::new("/a b/c-1.html")),
            "file:///a%20b/c-1.html"
        );
    }

    #[test]
    fn truthiness_follows_javascript() {
        for v in [json!(null), json!(false), json!(0), json!(""), json!(0.0)] {
            assert!(!truthy(&v), "{v}");
        }
        for v in [json!(true), json!(1), json!("0"), json!([]), json!({})] {
            assert!(truthy(&v), "{v}");
        }
    }
}
