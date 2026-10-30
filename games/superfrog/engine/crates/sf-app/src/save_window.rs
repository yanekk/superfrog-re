//! The save window after STOP (bug-recorder DESIGN §2.4): the screenshot, where play is, the zip's
//! name, a one-line note, Discard and Save. Native, as every panel of the app is (RE §2.7).
//!
//! ```text
//! ┌ Save Bug Recording ───────────────┐
//! │ [ screenshot, 1×              ]   │
//! │ Location: W1-2:94,51              │
//! │ Will save as bug-W1-2-….zip       │
//! │ Note                              │
//! │ [ black screen after the slot   ] │
//! │                 [Discard] [Save]  │
//! └───────────────────────────────────┘
//! ```
//!
//! It is modal: while it is open nothing else in the app runs, so the game is paused (no step is
//! taken, so the recording's inputs end at STOP's frame) and every key goes to the note, never to
//! the game. A write that fails is said in an alert over it and the window stays open with the
//! note (§2.11). On a game window it is a sheet, so it shows in fullscreen too.
//!
//! The model ([`SaveForm`]) holds no AppKit and is what the tests and the script drive.

use sf_core::recording::Location;

/// What the window shows and what the player has typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveForm {
    pub note: String,
    pub name: String,
    pub location: Location,
}

/// What the player did in the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveAction {
    /// The note's text as the field holds it.
    Note(String),
    Save,
    Discard,
}

/// What the window does after an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Still open.
    Editing,
    /// Write the zip with this note.
    Save(String),
    Discard,
}

impl SaveForm {
    pub fn new(name: &str, location: Location) -> SaveForm {
        SaveForm { note: String::new(), name: name.to_string(), location }
    }

    /// The note is one line (the recording's header holds it on one): a newline is not accepted,
    /// and one pasted in becomes a space.
    pub fn action(&mut self, a: SaveAction) -> Outcome {
        match a {
            SaveAction::Note(t) => {
                self.note = t.replace("\r\n", " ").replace(['\n', '\r'], " ");
                Outcome::Editing
            }
            SaveAction::Save => Outcome::Save(self.note.clone()),
            SaveAction::Discard => Outcome::Discard,
        }
    }

    pub fn location_line(&self) -> String {
        format!("Location: {}", self.location)
    }

    pub fn name_line(&self) -> String {
        format!("Will save as {}", self.name)
    }
}

/// The window's title.
pub const TITLE: &str = "Save Bug Recording";

#[cfg(target_os = "macos")]
pub use view::run;

/// Only macOS is built for (DESIGN §1); elsewhere the recording is discarded.
#[cfg(not(target_os = "macos"))]
pub fn run(
    _parent: Option<&winit::window::Window>,
    _shot: &[u8],
    _form: SaveForm,
    _save: &mut dyn FnMut(&str) -> Result<(), String>,
) -> bool {
    eprintln!("superfrog: the save window is macOS only; the recording is discarded");
    false
}

#[cfg(target_os = "macos")]
mod view {
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Bool, NSObject, NSObjectProtocol};
    use objc2::{AnyThread, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
    use objc2_app_kit::{
        NSAccessibility, NSAlert, NSAlertStyle, NSApplication, NSBackingStoreType, NSButton, NSColor,
        NSControl, NSEvent, NSEventModifierFlags, NSFont, NSImage, NSImageScaling, NSImageView,
        NSModalResponse, NSResponder, NSTextField, NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSData, NSPoint, NSRect, NSSize, NSString};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;

    use super::{Outcome, SaveAction, SaveForm, TITLE};

    /// The modal responses the two buttons stop the window with, as their tags.
    const SAVE: NSModalResponse = 1;
    const DISCARD: NSModalResponse = 2;

    /// The window's size in points; the screenshot is shown at 1×, the game's largest picture.
    const W: f64 = 374.0;
    const H: f64 = 455.0;
    const SHOT: (f64, f64) = (334.0, 261.0);

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and `Target` implements no `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "SuperfrogSaveTarget"]
        struct Target;

        impl Target {
            #[unsafe(method(clicked:))]
            fn clicked(&self, sender: &NSControl) {
                NSApplication::sharedApplication(self.mtm()).stopModalWithCode(sender.tag());
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

    define_class!(
        // SAFETY: NSWindow may be subclassed; `EditWindow` adds no state and no `Drop`.
        #[unsafe(super(NSWindow, NSResponder, NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "SuperfrogSaveWindow"]
        struct EditWindow;

        impl EditWindow {
            /// ⌘X, ⌘C, ⌘V, ⌘A and ⌘Z in the note. A text field takes them from the Edit menu's
            /// items, and the app's menu bar has no Edit menu (RE §2.7 fixes its menus), so the
            /// window sends them to the field itself.
            #[unsafe(method(performKeyEquivalent:))]
            fn perform_key_equivalent(&self, event: &NSEvent) -> Bool {
                let flags = event.modifierFlags() & NSEventModifierFlags::DeviceIndependentFlagsMask;
                if flags == NSEventModifierFlags::Command {
                    let key = event.charactersIgnoringModifiers().map(|s| s.to_string()).unwrap_or_default();
                    let action = match key.as_str() {
                        "x" => Some(sel!(cut:)),
                        "c" => Some(sel!(copy:)),
                        "v" => Some(sel!(paste:)),
                        "a" => Some(sel!(selectAll:)),
                        "z" => Some(sel!(undo:)),
                        _ => None,
                    };
                    if let Some(action) = action {
                        let app = NSApplication::sharedApplication(self.mtm());
                        // SAFETY: a standard responder action, sent along the responder chain
                        // from this window, the key one.
                        if unsafe { app.sendAction_to_from(action, None, Some(self)) } {
                            return Bool::YES;
                        }
                    }
                }
                // SAFETY: NSWindow's own handling, for the buttons' Return.
                unsafe { msg_send![super(self), performKeyEquivalent: event] }
            }
        }
    );

    fn rect(x: f64, top: f64, w: f64, h: f64) -> NSRect {
        // A window's content view is not flipped: y grows up from the bottom.
        NSRect::new(NSPoint::new(x, H - top - h), NSSize::new(w, h))
    }

    /// The NSWindow of winit's `window`.
    fn ns_window(window: &Window) -> Option<Retained<NSWindow>> {
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::AppKit(h) = handle.as_raw() else { return None };
        // SAFETY: winit's view, alive as long as `window`; used on the main thread only.
        let view: &NSView = unsafe { h.ns_view.cast::<NSView>().as_ref() };
        view.window()
    }

    /// Show the window for `form` with the screenshot `shot` (a PNG) until Save writes the zip
    /// (`save`, with the note) or Discard: `true` when saved. A `save` that fails says why in an
    /// alert, and the window stays open with the note.
    pub fn run(
        parent: Option<&Window>,
        shot: &[u8],
        mut form: SaveForm,
        save: &mut dyn FnMut(&str) -> Result<(), String>,
    ) -> bool {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("superfrog: the save window opens on the main thread; the recording is discarded");
            return false;
        };
        let app = NSApplication::sharedApplication(mtm);
        let style = NSWindowStyleMask::Titled;
        // SAFETY: a new window, kept alive by `window` until it is closed below; AppKit does not
        // release it on close (`setReleasedWhenClosed(false)`), so the two never both free it.
        let window: Retained<EditWindow> = unsafe {
            let w: Retained<EditWindow> = msg_send![
                EditWindow::alloc(mtm),
                initWithContentRect: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(W, H)),
                styleMask: style,
                backing: NSBackingStoreType::Buffered,
                defer: false
            ];
            w.setReleasedWhenClosed(false);
            w
        };
        window.setTitle(&NSString::from_str(TITLE));
        let Some(content) = window.contentView() else { return false };
        let target = Target::new(mtm);

        let data = NSData::with_bytes(shot);
        if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
            let v = NSImageView::imageViewWithImage(&image, mtm);
            v.setFrame(rect(20.0, 20.0, SHOT.0, SHOT.1));
            v.setImageScaling(NSImageScaling::ScaleProportionallyDown);
            v.setAccessibilityIdentifier(Some(&NSString::from_str("screenshot")));
            content.addSubview(&v);
        }
        let label = |text: &str, top: f64, id: &str| {
            let l = NSTextField::labelWithString(&NSString::from_str(text), mtm);
            l.setFrame(rect(20.0, top, W - 40.0, 18.0));
            l.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
            content.addSubview(&l);
            l
        };
        label(&form.location_line(), 291.0, "location");
        let name = label(&form.name_line(), 311.0, "name");
        name.setTextColor(Some(&NSColor::secondaryLabelColor()));
        name.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        label("Note", 341.0, "note-label");
        let note = NSTextField::textFieldWithString(&NSString::from_str(&form.note), mtm);
        note.setFrame(rect(20.0, 363.0, W - 40.0, 24.0));
        if let Some(cell) = note.cell() {
            cell.setUsesSingleLineMode(true);
            cell.setScrollable(true);
            cell.setWraps(false);
        }
        note.setAccessibilityIdentifier(Some(&NSString::from_str("note")));
        content.addSubview(&note);
        let button = |title: &str, x: f64, tag: NSModalResponse, key: &str| {
            // SAFETY: the target outlives the button (both live until this returns); `clicked:`
            // is the target's method taking the sender.
            let b = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(title),
                    Some(&target as &AnyObject),
                    Some(sel!(clicked:)),
                    mtm,
                )
            };
            b.setFrame(rect(x, 403.0, 100.0, 32.0));
            b.setTag(tag);
            b.setKeyEquivalent(&NSString::from_str(key));
            b.setAccessibilityIdentifier(Some(&NSString::from_str(&title.to_lowercase())));
            content.addSubview(&b);
            b
        };
        // Return saves; Discard has no key, so a stray key never drops the recording.
        button("Discard", W - 230.0, DISCARD, "");
        button("Save", W - 120.0, SAVE, "\r");
        window.setInitialFirstResponder(Some(&note));

        let parent = parent.and_then(ns_window);
        match &parent {
            // A sheet on the game window: modal for it, and shown in fullscreen too.
            Some(p) => p.beginSheet_completionHandler(&window, None),
            None => {
                window.center();
                window.makeKeyAndOrderFront(None);
            }
        }
        window.makeFirstResponder(Some(&note));
        let saved = loop {
            let code = app.runModalForWindow(&window);
            let a = match code {
                SAVE => SaveAction::Save,
                _ => SaveAction::Discard,
            };
            form.action(SaveAction::Note(note.stringValue().to_string()));
            match form.action(a) {
                Outcome::Editing => {}
                Outcome::Discard => break false,
                Outcome::Save(n) => match save(&n) {
                    Ok(()) => break true,
                    Err(why) => {
                        note.setStringValue(&NSString::from_str(&form.note));
                        alert(mtm, &why);
                        window.makeFirstResponder(Some(&note));
                    }
                },
            }
        };
        match &parent {
            Some(p) => p.endSheet(&window),
            None => window.orderOut(None),
        }
        window.close();
        let _ = target;
        saved
    }

    /// `message` with an OK button, over the save window.
    fn alert(mtm: MainThreadMarker, message: &str) {
        let a = NSAlert::new(mtm);
        a.setAlertStyle(NSAlertStyle::Warning);
        a.setMessageText(&NSString::from_str("The recording was not saved"));
        a.setInformativeText(&NSString::from_str(message));
        a.addButtonWithTitle(&NSString::from_str("OK"));
        a.runModal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> SaveForm {
        SaveForm::new("bug-W1-2-2026-10-20-1432.zip", Location::Level { id: "W1-2".parse().unwrap(), col: 94, row: 51 })
    }

    #[test]
    fn the_window_shows_where_and_the_name_and_saves_the_note() {
        let mut f = form();
        assert_eq!(f.location_line(), "Location: W1-2:94,51");
        assert_eq!(f.name_line(), "Will save as bug-W1-2-2026-10-20-1432.zip");
        assert_eq!(f.action(SaveAction::Save), Outcome::Save(String::new()), "an empty note may be saved");
        assert_eq!(f.action(SaveAction::Note("black screen".into())), Outcome::Editing);
        assert_eq!(f.action(SaveAction::Save), Outcome::Save("black screen".into()));
        assert_eq!(f.note, "black screen", "kept after Save, for a write that fails");
        assert_eq!(f.action(SaveAction::Discard), Outcome::Discard);
        let mut s = SaveForm::new("bug-W1-1.zip", Location::Screen("slot machine"));
        assert_eq!(s.location_line(), "Location: slot machine");
        assert_eq!(s.action(SaveAction::Save), Outcome::Save(String::new()));
    }

    #[test]
    fn a_note_s_newline_is_not_accepted() {
        let mut f = form();
        f.action(SaveAction::Note("one\ntwo\r\nthree\rfour".into()));
        assert_eq!(f.note, "one two three four");
        assert!(!f.note.contains(['\n', '\r']));
    }
}
