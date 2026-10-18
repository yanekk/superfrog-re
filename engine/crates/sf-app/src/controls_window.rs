//! Game → Controls…: the window that rebinds the joystick's five inputs to keys and pad inputs
//! (DESIGN §2.6), remembered in `settings.toml` (DESIGN §2.9). Native, as every panel of the app
//! is (DESIGN §7): a window winit opens, holding AppKit labels and buttons.
//!
//! ```text
//!            Keys                          Pad
//! Up       [ ↑                     ]   [ D-pad ↑, Stick ↑     ]
//! …
//! Fire     [ Space, Left Ctrl, …   ]   [ A, B                 ]
//! Fire is now X; it no longer works as the Amiga's X key.
//!                                          [ Restore Defaults ]
//! ```
//!
//! Clicking a binding waits for the next key (pressed in this window) or pad input, which becomes
//! that input's only one; Esc cancels, and Esc again closes the window. A key or pad input feeds
//! one input at a time: bound to a second, it is taken from the first and the window says so
//! (the task's "resolved as the window says"). The Amiga's own keys keep their same-label keys
//! here; their bindings are in `settings.toml`. Restore Defaults restores both maps whole.
//!
//! The model ([`Controls`]) holds no AppKit and is what the tests drive; a script reaches it as
//! the clicks do (`script.rs`: `controls fire keys`, `controls defaults`, `controls close`).

use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

use crate::App;
use crate::input::{HostInput, KeyMap, OriginalInput, key_label, key_name};
use crate::pad::{PadEvent, PadInput, PadMap, PadState};
use crate::settings::Settings;

/// The rows of the window, top to bottom.
pub const JOYSTICK: [OriginalInput; 5] = [
    OriginalInput::Up,
    OriginalInput::Down,
    OriginalInput::Left,
    OriginalInput::Right,
    OriginalInput::Fire,
];

/// Which of an input's bindings: the keys, or the pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Keys,
    Pad,
}

/// What the player did in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlsAction {
    /// Clicked an input's keys or pad: the next one pressed becomes its binding.
    Rebind(OriginalInput, Device),
    /// Restore Defaults.
    Defaults,
    /// The window closed.
    Close,
}

impl ControlsAction {
    /// Its name in a script's `controls …` line: `fire keys`, `up pad`, `defaults`, `close`.
    pub fn named(words: &[&str]) -> Option<ControlsAction> {
        Some(match words {
            ["defaults"] => ControlsAction::Defaults,
            ["close"] => ControlsAction::Close,
            [input, device] => {
                let o = OriginalInput::named(input).filter(|o| JOYSTICK.contains(o))?;
                let d = match *device {
                    "keys" => Device::Keys,
                    "pad" => Device::Pad,
                    _ => return None,
                };
                ControlsAction::Rebind(o, d)
            }
            _ => return None,
        })
    }
}

/// Make `code` the only key for `o`, taking it from every other input it fed. The inputs it was
/// taken from, in order.
pub fn bind_key(map: &mut KeyMap, o: OriginalInput, code: KeyCode) -> Vec<OriginalInput> {
    let mut taken = Vec::new();
    for (other, keys) in map.0.iter_mut() {
        if *other != o && keys.contains(&code) {
            keys.retain(|&k| k != code);
            taken.push(*other);
        }
    }
    map.0.insert(o, vec![code]);
    taken
}

/// [`bind_key`] for the pad.
pub fn bind_pad(map: &mut PadMap, o: OriginalInput, p: PadInput) -> Vec<OriginalInput> {
    let mut taken = Vec::new();
    for (other, ps) in map.0.iter_mut() {
        if *other != o && ps.contains(&p) {
            ps.retain(|&q| q != p);
            taken.push(*other);
        }
    }
    map.0.insert(o, vec![p]);
    taken
}

/// What a row's keys button shows.
pub fn keys_title(map: &KeyMap, o: OriginalInput) -> String {
    let labels: Vec<String> = map.0.get(&o).into_iter().flatten().map(|&k| key_label(k)).collect();
    if labels.is_empty() { "None".into() } else { labels.join(", ") }
}

/// What a row's pad button shows.
pub fn pad_title(map: &PadMap, o: OriginalInput) -> String {
    let labels: Vec<&str> = map.0.get(&o).into_iter().flatten().map(|p| p.label()).collect();
    if labels.is_empty() { "None".into() } else { labels.join(", ") }
}

/// The window's state: which binding waits for a press, and the line under the rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Controls {
    pub waiting: Option<(OriginalInput, Device)>,
    pub status: String,
}

/// What a press or a click did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to remember.
    Shown,
    /// The bindings changed: remember them.
    Changed,
    /// Close the window.
    Close,
}

const NOTHING_CHANGED: &str = "Nothing changed.";

impl Controls {
    pub fn new() -> Controls {
        Controls {
            waiting: None,
            status: "Click a binding, then press the key or pad button for it.".into(),
        }
    }

    /// What a row's button shows: its bindings, or the prompt while it waits.
    pub fn title(&self, s: &Settings, o: OriginalInput, d: Device) -> String {
        match (self.waiting == Some((o, d)), d) {
            (true, Device::Keys) => "Press a key…".into(),
            (true, Device::Pad) => "Press a pad button…".into(),
            (false, Device::Keys) => keys_title(&s.keys, o),
            (false, Device::Pad) => pad_title(&s.pad, o),
        }
    }

    /// A click in the window.
    pub fn action(&mut self, a: ControlsAction, s: &mut Settings) -> Outcome {
        match a {
            ControlsAction::Rebind(o, d) if self.waiting == Some((o, d)) => {
                self.waiting = None;
                self.status = NOTHING_CHANGED.into();
                Outcome::Shown
            }
            ControlsAction::Rebind(o, d) => {
                self.waiting = Some((o, d));
                self.status = match d {
                    Device::Keys => format!("Press the key for {}. Esc cancels.", o.label()),
                    Device::Pad => {
                        format!("Press the pad button for {}, or push the stick. Esc cancels.", o.label())
                    }
                };
                Outcome::Shown
            }
            ControlsAction::Defaults => {
                self.waiting = None;
                s.keys = KeyMap::default();
                s.pad = PadMap::default();
                self.status = "The keys and the pad are back to their defaults.".into();
                Outcome::Changed
            }
            ControlsAction::Close => Outcome::Close,
        }
    }

    /// A key pressed in the window.
    pub fn key(&mut self, code: KeyCode, s: &mut Settings) -> Outcome {
        if matches!(code, KeyCode::SuperLeft | KeyCode::SuperRight) {
            return Outcome::Shown; // ⌘ is the menu bar's.
        }
        if code == KeyCode::Escape {
            if self.waiting.take().is_some() {
                self.status = NOTHING_CHANGED.into();
                return Outcome::Shown;
            }
            return Outcome::Close;
        }
        let Some((o, Device::Keys)) = self.waiting else {
            return Outcome::Shown;
        };
        if key_name(code).is_none() {
            self.status = "That key cannot be used. Press another, or Esc.".into();
            return Outcome::Shown;
        }
        self.waiting = None;
        let taken = bind_key(&mut s.keys, o, code);
        self.status = bound(o, &key_label(code), &taken, |t| s.keys.0.get(t).is_none_or(Vec::is_empty), "key");
        Outcome::Changed
    }

    /// A pad input newly held, on any pad.
    pub fn pad(&mut self, p: PadInput, s: &mut Settings) -> Outcome {
        let Some((o, Device::Pad)) = self.waiting else {
            return Outcome::Shown;
        };
        self.waiting = None;
        let taken = bind_pad(&mut s.pad, o, p);
        self.status = bound(o, p.label(), &taken, |t| s.pad.0.get(t).is_none_or(Vec::is_empty), "pad button");
        Outcome::Changed
    }

    /// Is the window waiting for a pad input?
    pub fn wants_pad(&self) -> bool {
        matches!(self.waiting, Some((_, Device::Pad)))
    }
}

/// The line saying what a binding did: `Fire is now X; it no longer works as Left. Left has no
/// key now.`
fn bound(
    o: OriginalInput,
    label: &str,
    taken: &[OriginalInput],
    empty: impl Fn(&OriginalInput) -> bool,
    what: &str,
) -> String {
    let mut line = format!("{} is now {label}", o.label());
    if taken.is_empty() {
        line.push('.');
        return line;
    }
    let names: Vec<String> = taken.iter().map(|t| t.label()).collect();
    line.push_str(&format!("; it no longer works as {}.", names.join(" or ")));
    for t in taken.iter().filter(|t| JOYSTICK.contains(t) && empty(t)) {
        line.push_str(&format!(" {} has no {what} now.", t.label()));
    }
    line
}

/// The open window: its model, its AppKit view, and what the pads hold as it sees them (to tell a
/// new press from one held when it opened).
pub struct OpenControls {
    pub model: Controls,
    pub window: ControlsWindow,
    pads: PadState,
}

impl App {
    /// Game → Controls…: open the window, or bring it to the front. It takes the keyboard, so the
    /// game lets go of every key held, as when its window loses focus.
    pub(crate) fn open_controls(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(c) = &self.controls {
            c.window.window.focus_window();
            return;
        }
        match ControlsWindow::open(event_loop, self.script.is_none()) {
            Ok(window) => {
                if let Some(run) = &mut self.run {
                    run.input.release_keys();
                }
                self.controls = Some(OpenControls {
                    model: Controls::new(),
                    window,
                    pads: PadState::default(),
                });
                self.show_controls();
            }
            Err(e) => {
                eprintln!("superfrog: {e}");
                self.ui.notice(&e);
            }
        }
    }

    /// A click in the window, or a script's `controls …`.
    pub(crate) fn controls_action(&mut self, a: ControlsAction) {
        let (Some(c), Some(run)) = (&mut self.controls, &mut self.run) else {
            eprintln!("superfrog: the Controls window is not open");
            return;
        };
        let o = c.model.action(a, &mut run.settings);
        self.controls_outcome(o);
    }

    /// A key pressed while the window has the keyboard.
    pub(crate) fn controls_key(&mut self, code: KeyCode) {
        let (Some(c), Some(run)) = (&mut self.controls, &mut self.run) else { return };
        let o = c.model.key(code, &mut run.settings);
        self.controls_outcome(o);
    }

    /// A pad's event: the game's, unless the window is waiting for a pad input and this is one
    /// newly held, which then becomes the binding and never reaches the game.
    pub(crate) fn pad_event(&mut self, e: PadEvent) {
        match e {
            PadEvent::Connected(n) => eprintln!("superfrog: pad {n} connected"),
            PadEvent::Disconnected(n) => eprintln!("superfrog: pad {n} disconnected"),
            _ => {}
        }
        if let (Some(c), Some(run)) = (&mut self.controls, &mut self.run) {
            let new = c.pads.apply(e);
            if let Some(&p) = new.first()
                && c.model.wants_pad()
            {
                let o = c.model.pad(p, &mut run.settings);
                return self.controls_outcome(o);
            }
        }
        if let Some(run) = &mut self.run {
            run.input.event(HostInput::Pad(e));
        }
    }

    /// Close the window; the game window takes the keyboard back.
    pub(crate) fn close_controls(&mut self) {
        if let Some(c) = self.controls.take() {
            eprintln!("superfrog: controls closed");
            drop(c);
            if let Some(run) = &self.run
                && self.script.is_none()
            {
                run.window.focus_window();
            }
        }
    }

    fn controls_outcome(&mut self, o: Outcome) {
        match o {
            Outcome::Shown => {}
            Outcome::Changed => {
                let run = self.run.as_mut().expect("running");
                run.input.map = run.settings.keys.clone();
                run.input.pad_map = run.settings.pad.clone();
                self.save_settings();
            }
            Outcome::Close => return self.close_controls(),
        }
        self.show_controls();
    }

    /// The window shows the model; a changed line under the rows is logged, so a test can follow
    /// what the window said.
    fn show_controls(&mut self) {
        let (Some(c), Some(run)) = (&mut self.controls, &self.run) else { return };
        if c.window.shown_status() != Some(c.model.status.as_str()) {
            eprintln!("superfrog: controls: {}", c.model.status);
        }
        c.window.show(&c.model, &run.settings);
    }
}

#[cfg(target_os = "macos")]
pub use view::ControlsWindow;

#[cfg(not(target_os = "macos"))]
pub use elsewhere::ControlsWindow;

/// Only macOS is built for (DESIGN §1); elsewhere Game → Controls… says so.
#[cfg(not(target_os = "macos"))]
mod elsewhere {
    pub struct ControlsWindow {
        pub window: winit::window::Window,
    }

    impl ControlsWindow {
        pub fn open(_: &winit::event_loop::ActiveEventLoop, _: bool) -> Result<ControlsWindow, String> {
            Err("The Controls window is macOS only; edit settings.toml instead.".into())
        }
        pub fn id(&self) -> winit::window::WindowId {
            self.window.id()
        }
        pub fn show(&mut self, _: &super::Controls, _: &crate::settings::Settings) {}
        pub fn shown_status(&self) -> Option<&str> {
            None
        }
        pub fn next_click() -> Option<super::ControlsAction> {
            None
        }
    }
}

/// The window itself: AppKit labels and buttons on the view of a window winit opens, so its keys
/// and its close button arrive as winit's events, by window id, as the game window's do.
#[cfg(target_os = "macos")]
mod view {
    use std::sync::Mutex;

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
    use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
    use objc2_app_kit::{NSAccessibility, NSButton, NSControl, NSFont, NSTextField, NSView};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use winit::dpi::LogicalSize;
    use winit::event_loop::ActiveEventLoop;
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::{Window, WindowId};

    use super::{Controls, ControlsAction, Device, JOYSTICK};
    use crate::settings::Settings;

    /// The window's size in points.
    const SIZE: (f64, f64) = (540.0, 330.0);

    /// The buttons' clicks, by tag ([`tag`]), as the target's action receives them.
    static CLICKS: Mutex<Vec<isize>> = Mutex::new(Vec::new());

    /// A button's tag: the rows' keys buttons, then their pad buttons, then Restore Defaults.
    fn tag(a: ControlsAction) -> isize {
        match a {
            ControlsAction::Rebind(o, d) => {
                let row = JOYSTICK.iter().position(|&j| j == o).expect("a joystick input") as isize;
                row + if d == Device::Pad { JOYSTICK.len() as isize } else { 0 }
            }
            ControlsAction::Defaults => 2 * JOYSTICK.len() as isize,
            ControlsAction::Close => -1,
        }
    }

    fn action(tag: isize) -> Option<ControlsAction> {
        let n = JOYSTICK.len() as isize;
        Some(match tag {
            t if (0..n).contains(&t) => ControlsAction::Rebind(JOYSTICK[t as usize], Device::Keys),
            t if (n..2 * n).contains(&t) => ControlsAction::Rebind(JOYSTICK[(t - n) as usize], Device::Pad),
            t if t == 2 * n => ControlsAction::Defaults,
            _ => return None,
        })
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and `Target` implements no `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "SuperfrogControlsTarget"]
        struct Target;

        impl Target {
            #[unsafe(method(clicked:))]
            fn clicked(&self, sender: &NSControl) {
                CLICKS.lock().expect("clicks").push(sender.tag());
            }
        }

        unsafe impl NSObjectProtocol for Target {}
    );

    impl Target {
        fn new(mtm: MainThreadMarker) -> Retained<Target> {
            let this = Target::alloc(mtm).set_ivars(());
            // SAFETY: NSObject's `init`, on a freshly allocated object.
            unsafe { msg_send![super(this), init] }
        }
    }

    fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
    }

    pub struct ControlsWindow {
        pub window: Window,
        buttons: Vec<(ControlsAction, Retained<NSButton>)>,
        status: Retained<NSTextField>,
        _target: Retained<Target>,
        _labels: Vec<Retained<NSTextField>>,
        /// What the buttons and the status line show, so only a change is set.
        shown: Vec<String>,
    }

    impl ControlsWindow {
        /// Open the window. `active`: take the keyboard (not under a script, which must leave the
        /// person's keyboard alone).
        pub fn open(event_loop: &ActiveEventLoop, active: bool) -> Result<ControlsWindow, String> {
            let mtm = MainThreadMarker::new().ok_or("the Controls window opens on the main thread")?;
            let attrs = Window::default_attributes()
                .with_title("Controls")
                .with_inner_size(LogicalSize::new(SIZE.0, SIZE.1))
                .with_resizable(false)
                .with_active(active);
            let window = event_loop.create_window(attrs).map_err(|e| format!("no Controls window: {e}"))?;
            let handle = window.window_handle().map_err(|e| format!("the Controls window: {e}"))?;
            let RawWindowHandle::AppKit(h) = handle.as_raw() else {
                return Err("the Controls window is not an AppKit window".into());
            };
            // SAFETY: winit's view, alive as long as `window`, which this struct owns; used on the
            // main thread only. winit's view is flipped: y grows down from the top.
            let view: &NSView = unsafe { h.ns_view.cast::<NSView>().as_ref() };
            let target = Target::new(mtm);
            let mut labels = Vec::new();
            let label = |text: &str, frame: NSRect, bold: bool| {
                let l = NSTextField::labelWithString(&NSString::from_str(text), mtm);
                l.setFrame(frame);
                if bold {
                    l.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
                }
                view.addSubview(&l);
                l
            };
            labels.push(label("Keys", rect(112.0, 18.0, 200.0, 20.0), true));
            labels.push(label("Pad", rect(328.0, 18.0, 200.0, 20.0), true));
            let mut buttons = Vec::new();
            let button = |a: ControlsAction, frame: NSRect, id: &str| {
                // SAFETY: the target outlives the button (both are this struct's); `clicked:` is
                // the target's method taking the sender.
                let b = unsafe {
                    NSButton::buttonWithTitle_target_action(
                        &NSString::from_str(""),
                        Some(&target as &AnyObject),
                        Some(sel!(clicked:)),
                        mtm,
                    )
                };
                b.setFrame(frame);
                b.setTag(tag(a));
                // The keyboard stays with winit's view, so a key pressed after a click reaches
                // the window's key handler and Space does not click the button again.
                b.setRefusesFirstResponder(true);
                b.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
                view.addSubview(&b);
                (a, b)
            };
            for (row, &o) in JOYSTICK.iter().enumerate() {
                let y = 46.0 + row as f64 * 38.0;
                labels.push(label(&o.label(), rect(20.0, y + 5.0, 84.0, 20.0), false));
                for (d, x, name) in [(Device::Keys, 106.0, "keys"), (Device::Pad, 322.0, "pad")] {
                    let id = format!("{}-{name}", o.name());
                    buttons.push(button(ControlsAction::Rebind(o, d), rect(x, y, 206.0, 30.0), &id));
                }
            }
            let status = NSTextField::wrappingLabelWithString(&NSString::from_str(""), mtm);
            status.setFrame(rect(20.0, 240.0, 500.0, 36.0));
            status.setAccessibilityIdentifier(Some(&NSString::from_str("status")));
            view.addSubview(&status);
            buttons.push(button(ControlsAction::Defaults, rect(360.0, 284.0, 164.0, 30.0), "defaults"));
            buttons.last().expect("a button").1.setTitle(&NSString::from_str("Restore Defaults"));
            window.focus_window();
            Ok(ControlsWindow {
                window,
                buttons,
                status,
                _target: target,
                _labels: labels,
                shown: Vec::new(),
            })
        }

        pub fn id(&self) -> WindowId {
            self.window.id()
        }

        /// The line under the rows as last shown.
        pub fn shown_status(&self) -> Option<&str> {
            self.shown.last().map(String::as_str)
        }

        /// Show `c` over the bindings in `s`.
        pub fn show(&mut self, c: &Controls, s: &Settings) {
            let mut texts: Vec<String> = self
                .buttons
                .iter()
                .map(|(a, _)| match *a {
                    ControlsAction::Rebind(o, d) => c.title(s, o, d),
                    _ => "Restore Defaults".into(),
                })
                .collect();
            texts.push(c.status.clone());
            if texts == self.shown {
                return;
            }
            for ((_, b), t) in self.buttons.iter().zip(&texts) {
                b.setTitle(&NSString::from_str(t));
            }
            self.status.setStringValue(&NSString::from_str(texts.last().expect("the status")));
            self.shown = texts;
        }

        /// The next click since the last call, if any.
        pub fn next_click() -> Option<ControlsAction> {
            let mut clicks = CLICKS.lock().expect("clicks");
            (!clicks.is_empty()).then(|| clicks.remove(0)).and_then(action)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn every_button_s_tag_names_its_action() {
            let mut all = vec![ControlsAction::Defaults];
            for o in JOYSTICK {
                all.push(ControlsAction::Rebind(o, Device::Keys));
                all.push(ControlsAction::Rebind(o, Device::Pad));
            }
            for a in all {
                assert_eq!(action(tag(a)), Some(a), "{a:?}");
            }
            assert_eq!(action(-1), None);
            assert_eq!(action(11), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rebind(c: &mut Controls, s: &mut Settings, o: OriginalInput, d: Device) {
        assert_eq!(c.action(ControlsAction::Rebind(o, d), s), Outcome::Shown);
    }

    #[test]
    fn a_key_pressed_after_a_click_becomes_the_input_s_only_key() {
        let mut s = Settings::default();
        let mut c = Controls::new();
        assert_eq!(c.title(&s, OriginalInput::Fire, Device::Keys), "Space, Left Ctrl, Right Ctrl");
        assert_eq!(c.title(&s, OriginalInput::Fire, Device::Pad), "A, B");
        rebind(&mut c, &mut s, OriginalInput::Fire, Device::Keys);
        assert_eq!(c.title(&s, OriginalInput::Fire, Device::Keys), "Press a key…");
        assert_eq!(c.status, "Press the key for Fire. Esc cancels.");
        assert_eq!(c.key(KeyCode::KeyX, &mut s), Outcome::Changed);
        assert_eq!(s.keys.0[&OriginalInput::Fire], vec![KeyCode::KeyX]);
        assert_eq!(s.keys.0[&OriginalInput::Key(0x32)], Vec::<KeyCode>::new(), "X no longer types X");
        assert_eq!(c.status, "Fire is now X; it no longer works as the Amiga's X key.");
        assert_eq!(c.title(&s, OriginalInput::Fire, Device::Keys), "X");
        assert_eq!(c.waiting, None);
        // Not waiting, a key does nothing.
        assert_eq!(c.key(KeyCode::KeyZ, &mut s), Outcome::Shown);
        assert_eq!(s.keys.0[&OriginalInput::Fire], vec![KeyCode::KeyX]);
    }

    #[test]
    fn a_key_bound_to_a_second_input_is_taken_from_the_first_as_the_window_says() {
        let mut s = Settings::default();
        let mut c = Controls::new();
        rebind(&mut c, &mut s, OriginalInput::Fire, Device::Keys);
        c.key(KeyCode::ArrowUp, &mut s);
        assert_eq!(c.status, "Fire is now ↑; it no longer works as Up. Up has no key now.");
        assert_eq!(s.keys.0[&OriginalInput::Up], Vec::<KeyCode>::new());
        assert_eq!(c.title(&s, OriginalInput::Up, Device::Keys), "None");
        // Each key feeds one input: nowhere is a key bound twice.
        let mut seen = std::collections::BTreeSet::new();
        for k in s.keys.0.values().flatten() {
            assert!(seen.insert(*k), "{k:?} is bound twice");
        }
        // A key no other input uses is simply bound.
        rebind(&mut c, &mut s, OriginalInput::Up, Device::Keys);
        c.key(KeyCode::ShiftLeft, &mut s);
        assert_eq!(c.status, "Up is now Left Shift.");
    }

    #[test]
    fn esc_cancels_a_wait_then_closes_and_cmd_and_unknown_keys_bind_nothing() {
        let mut s = Settings::default();
        let mut c = Controls::new();
        rebind(&mut c, &mut s, OriginalInput::Left, Device::Keys);
        assert_eq!(c.key(KeyCode::SuperLeft, &mut s), Outcome::Shown);
        assert_eq!(c.key(KeyCode::CapsLock, &mut s), Outcome::Shown);
        assert_eq!(c.status, "That key cannot be used. Press another, or Esc.");
        assert!(c.waiting.is_some(), "still waiting");
        assert_eq!(c.key(KeyCode::Escape, &mut s), Outcome::Shown);
        assert_eq!(c.status, "Nothing changed.");
        assert_eq!(s, Settings::default());
        assert_eq!(c.key(KeyCode::Escape, &mut s), Outcome::Close);
        // Clicking the waiting binding again cancels too; another switches the wait.
        rebind(&mut c, &mut s, OriginalInput::Left, Device::Keys);
        rebind(&mut c, &mut s, OriginalInput::Left, Device::Pad);
        assert_eq!(c.waiting, Some((OriginalInput::Left, Device::Pad)));
        assert!(c.wants_pad());
        rebind(&mut c, &mut s, OriginalInput::Left, Device::Pad);
        assert_eq!(c.waiting, None);
        assert_eq!(c.status, "Nothing changed.");
    }

    #[test]
    fn a_pad_input_binds_only_while_the_pad_waits_and_is_taken_from_others_too() {
        let mut s = Settings::default();
        let mut c = Controls::new();
        assert_eq!(c.pad(PadInput::North, &mut s), Outcome::Shown);
        rebind(&mut c, &mut s, OriginalInput::Fire, Device::Keys);
        assert_eq!(c.pad(PadInput::North, &mut s), Outcome::Shown, "keys wait, not the pad");
        rebind(&mut c, &mut s, OriginalInput::Fire, Device::Pad);
        assert_eq!(c.status, "Press the pad button for Fire, or push the stick. Esc cancels.");
        assert_eq!(c.title(&s, OriginalInput::Fire, Device::Pad), "Press a pad button…");
        assert_eq!(c.pad(PadInput::North, &mut s), Outcome::Changed);
        assert_eq!(s.pad.0[&OriginalInput::Fire], vec![PadInput::North]);
        assert_eq!(c.status, "Fire is now Y.");
        rebind(&mut c, &mut s, OriginalInput::Up, Device::Pad);
        c.pad(PadInput::StickLeft, &mut s);
        assert_eq!(s.pad.0[&OriginalInput::Up], vec![PadInput::StickLeft]);
        assert_eq!(s.pad.0[&OriginalInput::Left], vec![PadInput::DPadLeft]);
        assert_eq!(c.status, "Up is now Stick ←; it no longer works as Left.");
        assert_eq!(s.keys, KeyMap::default(), "the keys are untouched");
    }

    #[test]
    fn restore_defaults_restores_both_maps() {
        let mut s = Settings::default();
        let mut c = Controls::new();
        bind_key(&mut s.keys, OriginalInput::Fire, KeyCode::KeyX);
        bind_pad(&mut s.pad, OriginalInput::Fire, PadInput::North);
        rebind(&mut c, &mut s, OriginalInput::Down, Device::Keys);
        assert_eq!(c.action(ControlsAction::Defaults, &mut s), Outcome::Changed);
        assert_eq!(s, Settings::default());
        assert_eq!(c.waiting, None);
        assert_eq!(c.status, "The keys and the pad are back to their defaults.");
        assert_eq!(c.action(ControlsAction::Close, &mut s), Outcome::Close);
    }

    #[test]
    fn a_script_names_each_action() {
        assert_eq!(
            ControlsAction::named(&["fire", "keys"]),
            Some(ControlsAction::Rebind(OriginalInput::Fire, Device::Keys))
        );
        assert_eq!(
            ControlsAction::named(&["left", "pad"]),
            Some(ControlsAction::Rebind(OriginalInput::Left, Device::Pad))
        );
        assert_eq!(ControlsAction::named(&["defaults"]), Some(ControlsAction::Defaults));
        assert_eq!(ControlsAction::named(&["close"]), Some(ControlsAction::Close));
        for bad in [&["jump", "keys"][..], &["fire", "mouse"], &["key", "P"], &[], &["fire"]] {
            assert_eq!(ControlsAction::named(bad), None, "{bad:?}");
        }
    }
}
