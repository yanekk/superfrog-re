//! The OS layer (DESIGN §5, End to end): real key events posted to the app with CGEvent, menu
//! items clicked through the Accessibility API (what System Events' UI scripting drives, without
//! its separate Automation grant), and the window captured by CoreGraphics. The terminal
//! running the tests needs Accessibility (to post events and click menus) and Screen Recording (to
//! capture the window): [`os_permissions`] says which is missing and where to grant it.

use std::ffi::c_void;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::{App, Image};

/// A key on the Mac's keyboard, by what is printed on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Space,
    Return,
    Escape,
    /// A letter or digit key.
    Char(char),
}

impl Key {
    /// The macOS virtual key code (`kVK_…`, ANSI layout).
    fn code(self) -> u16 {
        match self {
            Key::Left => 0x7B,
            Key::Right => 0x7C,
            Key::Down => 0x7D,
            Key::Up => 0x7E,
            Key::Space => 0x31,
            Key::Return => 0x24,
            Key::Escape => 0x35,
            Key::Char(c) => match c.to_ascii_lowercase() {
                'a' => 0x00,
                's' => 0x01,
                'd' => 0x02,
                'f' => 0x03,
                'h' => 0x04,
                'g' => 0x05,
                'z' => 0x06,
                'x' => 0x07,
                'c' => 0x08,
                'v' => 0x09,
                'b' => 0x0B,
                'q' => 0x0C,
                'w' => 0x0D,
                'e' => 0x0E,
                'r' => 0x0F,
                'y' => 0x10,
                't' => 0x11,
                '1' => 0x12,
                '2' => 0x13,
                '3' => 0x14,
                '4' => 0x15,
                '6' => 0x16,
                '5' => 0x17,
                '9' => 0x19,
                '7' => 0x1A,
                '8' => 0x1C,
                '0' => 0x1D,
                'o' => 0x1F,
                'u' => 0x20,
                'i' => 0x22,
                'p' => 0x23,
                'l' => 0x25,
                'j' => 0x26,
                'k' => 0x28,
                'n' => 0x2D,
                'm' => 0x2E,
                _ => panic!("no key for {c:?}"),
            },
        }
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGEventCreateKeyboardEvent(source: *const c_void, key: u16, down: bool) -> *mut c_void;
    fn CGEventPostToPid(pid: i32, event: *mut c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}

/// `Ok` when this process may post events and capture windows; otherwise what to grant, where,
/// and to which app.
pub fn os_permissions() -> Result<(), String> {
    // SAFETY: both only read this process's privacy grants.
    let (ax, screen) = unsafe { (AXIsProcessTrusted(), CGPreflightScreenCaptureAccess()) };
    let mut missing = Vec::new();
    if !ax {
        missing.push("System Settings → Privacy & Security → Accessibility");
    }
    if !screen {
        missing.push("System Settings → Privacy & Security → Screen & System Audio Recording");
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the OS-layer tests need these macOS permissions for the app running them ({}):\n  {}\n\
         Enable it there, then run the tests again.",
        responsible_app(),
        missing.join("\n  ")
    ))
}

/// The first `.app` among this process's ancestors: the terminal macOS asks permissions of.
fn responsible_app() -> String {
    let mut pid = std::process::id();
    for _ in 0..32 {
        let out = Command::new("ps")
            .args(["-o", "ppid=,comm=", "-p", &pid.to_string()])
            .output();
        let Ok(out) = out else { break };
        let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let Some((ppid, comm)) = line.split_once(' ') else { break };
        if let Some(i) = comm.find(".app/") {
            return comm[..i + 4].to_string();
        }
        match ppid.trim().parse::<u32>() {
            Ok(p) if p > 1 => pid = p,
            _ => break,
        }
    }
    // A terminal started by launchd's descendants (as an agent runner's is) is named by its own
    // variables.
    std::env::var("TERM_PROGRAM")
        .or_else(|_| std::env::var("__CFBundleIdentifier"))
        .unwrap_or_else(|_| "the terminal app the tests run in".to_string())
}

/// Why a window capture can come back blank: every pixel transparent is what macOS returns for
/// another app's window when Screen Recording is not granted, whatever the preflight says.
pub(crate) fn blank_capture_message() -> String {
    format!(
        "the window capture came back blank: macOS gave no Screen Recording access. Enable \
         {} in System Settings → Privacy & Security → Screen & System Audio Recording, \
         then run the tests again.",
        responsible_app()
    )
}

impl App {
    fn os_pid(&self) -> u32 {
        self.pid()
            .expect("no app is running: launch with `LaunchOpts { os: true, .. }`")
    }

    /// The app's window: its number and bounds in points, once it is on screen.
    pub fn os_window(&self) -> (u32, [i32; 4]) {
        let pid = self.os_pid();
        let script = format!(
            "ObjC.import('CoreGraphics'); \
             const l = ObjC.deepUnwrap(ObjC.castRefToObject(\
               $.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0))); \
             const w = l.find(w => w.kCGWindowOwnerPID == {pid} && w.kCGWindowLayer == 0); \
             w ? [w.kCGWindowNumber, w.kCGWindowBounds.X, w.kCGWindowBounds.Y, \
                  w.kCGWindowBounds.Width, w.kCGWindowBounds.Height].join(' ') : ''"
        );
        let start = Instant::now();
        loop {
            let out = Command::new("osascript")
                .args(["-l", "JavaScript", "-e", &script])
                .output()
                .expect("running osascript");
            let text = String::from_utf8_lossy(&out.stdout);
            let n: Vec<i64> = text
                .split_whitespace()
                .filter_map(|v| v.parse::<f64>().ok().map(|f| f as i64))
                .collect();
            if let [id, x, y, w, h] = n[..] {
                return (id as u32, [x as i32, y as i32, w as i32, h as i32]);
            }
            assert!(
                start.elapsed() < Duration::from_secs(20),
                "the app (pid {pid}) showed no window within 20 s: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Bring the app to the front, as a click on its window would.
    pub fn os_activate(&self) {
        let app = ax::app(self.os_pid());
        ax::set_true(&app, "AXFrontmost").unwrap_or_else(|e| panic!("activating the app: {e}"));
    }

    /// Press and release `key`, posted to the app as the keyboard would.
    pub fn os_key(&mut self, key: Key) {
        let pid = self.os_pid() as i32;
        for down in [true, false] {
            // SAFETY: a keyboard event made, posted and released here.
            unsafe {
                let e = CGEventCreateKeyboardEvent(std::ptr::null(), key.code(), down);
                assert!(!e.is_null(), "CGEventCreateKeyboardEvent failed");
                CGEventPostToPid(pid, e);
                CFRelease(e);
            }
            std::thread::sleep(Duration::from_millis(60));
        }
    }

    /// Click the menu item at `path`, from the menu bar down: `["superfrog", "Quit superfrog"]`.
    pub fn os_menu(&mut self, path: &[&str]) {
        assert!(path.len() >= 2, "a menu path names a menu and an item");
        let app = ax::app(self.os_pid());
        let bar = ax::attr(&app, "AXMenuBar").unwrap_or_else(|e| panic!("the app's menu bar: {e}"));
        let mut items = ax::children(&bar);
        let mut walked = Vec::new();
        for (i, name) in path.iter().enumerate() {
            let titles: Vec<String> = items.iter().map(|e| ax::title(e)).collect();
            let at = titles
                .iter()
                .position(|t| t == name)
                .unwrap_or_else(|| panic!("no menu item {name:?} under {walked:?}; there are {titles:?}"));
            let item = &items[at];
            if i == path.len() - 1 {
                ax::press(item).unwrap_or_else(|e| panic!("clicking {path:?}: {e}"));
                return;
            }
            walked.push(*name);
            // A menu bar item or a submenu item holds one menu, which holds the items.
            let menu = ax::children(item).into_iter().next().unwrap_or_else(|| panic!("{walked:?} has no menu"));
            items = ax::children(&menu);
        }
    }

    /// The window as the screen shows it, title bar included, at the display's pixel density.
    pub fn os_capture(&self) -> Image {
        let (id, _) = self.os_window();
        let path = self.home.join(format!("window-{id}.png"));
        // `CGWindowListCreateImage` through JXA: on macOS 26 the `screencapture` tool is refused
        // ("could not create image from window") where the terminal's own grant is enough for this.
        let script = format!(
            "ObjC.import('CoreGraphics'); ObjC.import('AppKit'); \
             const img = $.CGWindowListCreateImage($.CGRectNull, \
               $.kCGWindowListOptionIncludingWindow, {id}, $.kCGWindowImageBoundsIgnoreFraming); \
             const rep = $.NSBitmapImageRep.alloc.initWithCGImage(img); \
             rep.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $({{}})) \
               .writeToFileAtomically({:?}, true); \
             rep.pixelsWide + 'x' + rep.pixelsHigh",
            path.to_string_lossy()
        );
        let out = Command::new("osascript")
            .args(["-l", "JavaScript", "-e", &script])
            .output()
            .expect("running osascript");
        assert!(
            out.status.success() && path.exists(),
            "capturing window {id} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let img = Image::read_png(&path).unwrap_or_else(|e| panic!("{e}"));
        assert!(img.rgba.chunks(4).any(|p| p[3] != 0), "{}", blank_capture_message());
        img
    }
}

/// The Accessibility API, which is what System Events' UI scripting drives. Called directly, it
/// needs only the Accessibility grant (System Events needs an Automation grant as well).
mod ax {
    use std::ffi::{CString, c_char, c_void};

    type CFRef = *const c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> CFRef;
        fn AXUIElementCopyAttributeValue(e: CFRef, attr: CFRef, out: *mut CFRef) -> i32;
        fn AXUIElementSetAttributeValue(e: CFRef, attr: CFRef, value: CFRef) -> i32;
        fn AXUIElementPerformAction(e: CFRef, action: CFRef) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFBooleanTrue: CFRef;
        fn CFStringCreateWithCString(alloc: CFRef, s: *const c_char, enc: u32) -> CFRef;
        fn CFStringGetCString(s: CFRef, buf: *mut c_char, len: isize, enc: u32) -> bool;
        fn CFGetTypeID(r: CFRef) -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFArrayGetTypeID() -> usize;
        fn CFArrayGetCount(a: CFRef) -> isize;
        fn CFArrayGetValueAtIndex(a: CFRef, i: isize) -> CFRef;
        fn CFRetain(r: CFRef) -> CFRef;
        fn CFRelease(r: CFRef);
    }

    const UTF8: u32 = 0x0800_0100;

    /// An owned Core Foundation reference.
    pub struct Cf(CFRef);

    impl Drop for Cf {
        fn drop(&mut self) {
            // SAFETY: we hold one reference.
            unsafe { CFRelease(self.0) }
        }
    }

    fn string(s: &str) -> Cf {
        let c = CString::new(s).unwrap();
        // SAFETY: a valid C string; the result is owned.
        Cf(unsafe { CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), UTF8) })
    }

    pub fn app(pid: u32) -> Cf {
        // SAFETY: creates an owned element for the pid.
        Cf(unsafe { AXUIElementCreateApplication(pid as i32) })
    }

    pub fn attr(e: &Cf, name: &str) -> Result<Cf, String> {
        let mut out: CFRef = std::ptr::null();
        // SAFETY: `out` receives an owned reference on success.
        let err = unsafe { AXUIElementCopyAttributeValue(e.0, string(name).0, &mut out) };
        if err != 0 || out.is_null() {
            return Err(format!("{name}: AXError {err}"));
        }
        Ok(Cf(out))
    }

    pub fn set_true(e: &Cf, name: &str) -> Result<(), String> {
        // SAFETY: a static CF boolean, an owned attribute name.
        let err = unsafe { AXUIElementSetAttributeValue(e.0, string(name).0, kCFBooleanTrue) };
        if err != 0 { Err(format!("{name}: AXError {err}")) } else { Ok(()) }
    }

    pub fn press(e: &Cf) -> Result<(), String> {
        // SAFETY: an owned element and action name.
        let err = unsafe { AXUIElementPerformAction(e.0, string("AXPress").0) };
        if err != 0 { Err(format!("AXPress: AXError {err}")) } else { Ok(()) }
    }

    pub fn children(e: &Cf) -> Vec<Cf> {
        let Ok(a) = attr(e, "AXChildren") else { return Vec::new() };
        // SAFETY: type-checked before use; each element is retained into its own `Cf`.
        unsafe {
            if CFGetTypeID(a.0) != CFArrayGetTypeID() {
                return Vec::new();
            }
            (0..CFArrayGetCount(a.0))
                .map(|i| Cf(CFRetain(CFArrayGetValueAtIndex(a.0, i))))
                .collect()
        }
    }

    pub fn title(e: &Cf) -> String {
        let Ok(t) = attr(e, "AXTitle") else { return String::new() };
        let mut buf = [0 as c_char; 512];
        // SAFETY: type-checked; the buffer's length is passed.
        unsafe {
            if CFGetTypeID(t.0) != CFStringGetTypeID()
                || !CFStringGetCString(t.0, buf.as_mut_ptr(), buf.len() as isize, UTF8)
            {
                return String::new();
            }
            std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_digits_have_their_ansi_key_codes() {
        let codes: Vec<u16> = "asdzqp1059mn".chars().map(|c| Key::Char(c).code()).collect();
        assert_eq!(
            codes,
            [0x00, 0x01, 0x02, 0x06, 0x0C, 0x23, 0x12, 0x1D, 0x17, 0x19, 0x2E, 0x2D]
        );
    }
}
