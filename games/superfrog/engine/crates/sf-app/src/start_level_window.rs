//! Game ▸ Start Level…: the window that starts a level with chosen values, and the recorder on or
//! off (bug-recorder DESIGN §2.1, D1). Native, as the Controls window is (`controls_window.rs`,
//! RE §2.7): AppKit dropdowns, a field, checkboxes and two buttons.
//!
//! ```text
//!   Level            [ W1-1     ▾]
//!   Lives            [ 3        ▾]
//!   Points           [ 0         ]  0–999 999      ← shown only while the field is not a number
//!   Energy (hearts)  [ 4        ▾]                    in range, and Start is disabled then
//!   Difficulty       [ Normal   ▾]
//!                    [ ] Spud
//!                    [ ] Wings
//!                    [x] Enable recorder?
//!                                  [ Cancel ] [ Start ]
//! ```
//!
//! The window is modal to the game: the app runs it in AppKit's modal loop, so no frame is stepped
//! while it is open (the game is paused) and `NativeUi::start_level` returns the answer as the
//! other panels do. Return is Start, Esc Cancel. It always opens on a new game's values.
//!
//! The model ([`StartLevelForm`]) holds no AppKit and is what the tests drive; a script drives it
//! with `form …` lines (`script.rs`), answering the next Game ▸ Start Level… as the clicks would.

use sf_core::LevelId;
use sf_core::recording::{Difficulty, LevelSetup, Lives, MAX_POINTS, levels};

/// What the window shows beside Points while it does not hold a whole number in range.
pub const POINTS_RANGE: &str = "0–999 999";

/// The Lives dropdown, top to bottom.
pub const LIVES: [Lives; 3] = [Lives::Three, Lives::Five, Lives::Seven];
/// The Difficulty dropdown: no Hard (D12).
pub const DIFFICULTIES: [Difficulty; 2] = [Difficulty::Easy, Difficulty::Normal];

/// What the person did in the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormAction {
    Level(LevelId),
    Lives(Lives),
    /// What the Points field holds now, as typed.
    Points(String),
    Energy(u8),
    Difficulty(Difficulty),
    Spud(bool),
    Wings(bool),
    Record(bool),
    Start,
    Cancel,
}

/// The window's values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLevelForm {
    /// Every value but the points as chosen; the points as last valid.
    pub setup: LevelSetup,
    /// Enable recorder?
    pub record: bool,
    /// The Points field's text.
    pub points_text: String,
}

impl Default for StartLevelForm {
    fn default() -> StartLevelForm {
        StartLevelForm::new()
    }
}

impl StartLevelForm {
    /// A new game's values (DESIGN §2.1), the recorder on.
    pub fn new() -> StartLevelForm {
        let setup = LevelSetup::default();
        StartLevelForm {
            record: true,
            points_text: setup.points.to_string(),
            setup,
        }
    }

    /// The Points field as a score: a whole number 0–999 999, `None` otherwise.
    pub fn points(&self) -> Option<u32> {
        let t = self.points_text.trim();
        // Digits only: no sign, no spaces inside, no exponent; seven digits already overflow
        // the score, so a long run of them never reaches the parse.
        if t.is_empty() || t.len() > 7 || !t.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        t.parse().ok().filter(|&p| p <= MAX_POINTS)
    }

    /// Start is enabled only with points in range.
    pub fn can_start(&self) -> bool {
        self.points().is_some()
    }

    /// What shows beside the Points field: the range while it is refused, else nothing.
    pub fn points_hint(&self) -> &'static str {
        if self.can_start() { "" } else { POINTS_RANGE }
    }

    /// A click, a choice or a key in the window: `None` while it stays open, `Some(None)` for
    /// Cancel, `Some(Some(..))` for Start with the values and whether to record. Start with points
    /// out of range does nothing, as its disabled button would; a level or an energy the dropdowns
    /// do not offer changes nothing.
    pub fn apply(&mut self, a: FormAction) -> Option<Option<(LevelSetup, bool)>> {
        match a {
            FormAction::Level(id) if levels().contains(&id) => self.setup.level = id,
            FormAction::Level(_) => {}
            FormAction::Lives(l) => self.setup.lives = l,
            FormAction::Points(text) => {
                self.points_text = text;
                if let Some(p) = self.points() {
                    self.setup.points = p;
                }
            }
            FormAction::Energy(e) if (1..=4).contains(&e) => self.setup.energy = e,
            FormAction::Energy(_) => {}
            FormAction::Difficulty(d) => self.setup.difficulty = d,
            FormAction::Spud(on) => self.setup.spud = on,
            FormAction::Wings(on) => self.setup.wings = on,
            FormAction::Record(on) => self.record = on,
            FormAction::Start => {
                let points = self.points()?;
                self.setup.points = points;
                return Some(Some((self.setup, self.record)));
            }
            FormAction::Cancel => return Some(None),
        }
        None
    }
}

impl FormAction {
    /// Its words in a script's `form …` line: `level W1-2`, `lives 7`, `points 450000`,
    /// `energy 3`, `difficulty easy`, `spud on`, `wings off`, `record on`, `start`, `cancel`.
    /// `points` alone empties the field.
    pub fn named(words: &[&str]) -> Option<FormAction> {
        let on = |w: &str| match w {
            "on" => Some(true),
            "off" => Some(false),
            _ => None,
        };
        Some(match words {
            ["start"] => FormAction::Start,
            ["cancel"] => FormAction::Cancel,
            ["points", rest @ ..] => FormAction::Points(rest.join(" ")),
            ["level", id] => FormAction::Level(id.parse().ok().filter(|id| levels().contains(id))?),
            ["lives", n] => FormAction::Lives(*LIVES.iter().find(|l| l.count().to_string() == *n)?),
            ["energy", n] => FormAction::Energy(n.parse().ok().filter(|e| (1..=4).contains(e))?),
            ["difficulty", "easy"] => FormAction::Difficulty(Difficulty::Easy),
            ["difficulty", "normal"] => FormAction::Difficulty(Difficulty::Normal),
            ["spud", w] => FormAction::Spud(on(w)?),
            ["wings", w] => FormAction::Wings(on(w)?),
            ["record", w] => FormAction::Record(on(w)?),
            _ => return None,
        })
    }

    /// The words [`FormAction::named`] reads back.
    pub fn words(&self) -> String {
        let on = |b: bool| if b { "on" } else { "off" };
        match self {
            FormAction::Level(id) => format!("level {id}"),
            FormAction::Lives(l) => format!("lives {}", l.count()),
            FormAction::Points(t) => format!("points {t}").trim_end().to_string(),
            FormAction::Energy(e) => format!("energy {e}"),
            FormAction::Difficulty(d) => format!("difficulty {}", difficulty_title(*d).to_lowercase()),
            FormAction::Spud(b) => format!("spud {}", on(*b)),
            FormAction::Wings(b) => format!("wings {}", on(*b)),
            FormAction::Record(b) => format!("record {}", on(*b)),
            FormAction::Start => "start".into(),
            FormAction::Cancel => "cancel".into(),
        }
    }

    /// The actions that fill the window with `setup` and `record` from its defaults and press
    /// Start: a script's `start-level W1-2 lives=7 …` line, answered through the window's model.
    pub fn filling(setup: &LevelSetup, record: bool) -> Vec<FormAction> {
        vec![
            FormAction::Level(setup.level),
            FormAction::Lives(setup.lives),
            FormAction::Points(setup.points.to_string()),
            FormAction::Energy(setup.energy),
            FormAction::Difficulty(setup.difficulty),
            FormAction::Spud(setup.spud),
            FormAction::Wings(setup.wings),
            FormAction::Record(record),
            FormAction::Start,
        ]
    }
}

/// The Difficulty dropdown's title for `d`.
pub fn difficulty_title(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "Easy",
        Difficulty::Normal => "Normal",
    }
}

/// The window's dropdowns and checkboxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Level,
    Lives,
    Energy,
    Difficulty,
    Spud,
    Wings,
    Record,
}

impl Control {
    pub const ALL: [Control; 7] = [
        Control::Level,
        Control::Lives,
        Control::Energy,
        Control::Difficulty,
        Control::Spud,
        Control::Wings,
        Control::Record,
    ];

    /// A dropdown's items, top to bottom; empty for a checkbox.
    pub fn titles(self) -> Vec<String> {
        match self {
            Control::Level => levels().iter().map(ToString::to_string).collect(),
            Control::Lives => LIVES.iter().map(|l| l.count().to_string()).collect(),
            Control::Energy => (1..=4).map(|e: u8| e.to_string()).collect(),
            Control::Difficulty => DIFFICULTIES.iter().map(|&d| difficulty_title(d).into()).collect(),
            Control::Spud | Control::Wings | Control::Record => Vec::new(),
        }
    }

    /// The dropdown item shown, or the checkbox's state (1 on, 0 off), for `f`.
    pub fn index(self, f: &StartLevelForm) -> usize {
        let s = &f.setup;
        match self {
            Control::Level => levels().iter().position(|&l| l == s.level).unwrap_or(0),
            Control::Lives => LIVES.iter().position(|&l| l == s.lives).unwrap_or(0),
            Control::Energy => s.energy.saturating_sub(1) as usize,
            Control::Difficulty => DIFFICULTIES.iter().position(|&d| d == s.difficulty).unwrap_or(0),
            Control::Spud => s.spud as usize,
            Control::Wings => s.wings as usize,
            Control::Record => f.record as usize,
        }
    }

    /// What choosing dropdown item `i`, or setting a checkbox to `i` (1 on), does.
    pub fn chosen(self, i: usize) -> Option<FormAction> {
        Some(match self {
            Control::Level => FormAction::Level(*levels().get(i)?),
            Control::Lives => FormAction::Lives(*LIVES.get(i)?),
            Control::Energy => FormAction::Energy(u8::try_from(i + 1).ok().filter(|e| (1..=4).contains(e))?),
            Control::Difficulty => FormAction::Difficulty(*DIFFICULTIES.get(i)?),
            Control::Spud => FormAction::Spud(i != 0),
            Control::Wings => FormAction::Wings(i != 0),
            Control::Record => FormAction::Record(i != 0),
        })
    }

    /// Its accessibility identifier, for the OS layer.
    pub fn id(self) -> &'static str {
        match self {
            Control::Level => "level",
            Control::Lives => "lives",
            Control::Energy => "energy",
            Control::Difficulty => "difficulty",
            Control::Spud => "spud",
            Control::Wings => "wings",
            Control::Record => "record",
        }
    }

    /// Its label: beside a dropdown, a checkbox's own title.
    pub fn label(self) -> &'static str {
        match self {
            Control::Level => "Level",
            Control::Lives => "Lives",
            Control::Energy => "Energy (hearts)",
            Control::Difficulty => "Difficulty",
            Control::Spud => "Spud",
            Control::Wings => "Wings",
            Control::Record => "Enable recorder?",
        }
    }

    pub fn is_checkbox(self) -> bool {
        matches!(self, Control::Spud | Control::Wings | Control::Record)
    }
}

#[cfg(target_os = "macos")]
pub use view::run_modal;

/// Only macOS is built for (RE §1); elsewhere the window says so and starts nothing.
#[cfg(not(target_os = "macos"))]
pub fn run_modal(_: Option<&winit::window::Window>) -> Option<(LevelSetup, bool)> {
    eprintln!("superfrog: the Start Level window is macOS only");
    None
}

/// The window itself: an AppKit window of its own, run in AppKit's modal loop.
#[cfg(target_os = "macos")]
mod view {
    use std::cell::{Cell, OnceCell, RefCell};

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
    use objc2::runtime::{Bool, Sel};
    use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
    use objc2_app_kit::{
        NSAccessibility, NSApplication, NSBackingStoreType, NSButton, NSColor, NSControl,
        NSControlStateValueOn, NSEvent, NSEventModifierFlags, NSPopUpButton, NSResponder, NSTextField,
        NSView, NSWindow, NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use sf_core::recording::LevelSetup;

    use super::{Control, FormAction, StartLevelForm};

    /// The window's content size in points.
    const SIZE: (f64, f64) = (360.0, 330.0);
    /// The tags of the two buttons; a dropdown's or checkbox's is its index in [`Control::ALL`].
    const START: isize = 100;
    const CANCEL: isize = 101;

    /// The views the target changes as the model does.
    struct Shown {
        start: Retained<NSButton>,
        hint: Retained<NSTextField>,
        field: Retained<NSTextField>,
    }

    struct Ivars {
        form: RefCell<StartLevelForm>,
        shown: OnceCell<Shown>,
        /// Start's values or Cancel, once the window has closed on one.
        outcome: Cell<Option<Option<(LevelSetup, bool)>>>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and `Target` implements no `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "SuperfrogStartLevelTarget"]
        #[ivars = Ivars]
        struct Target;

        impl Target {
            /// A dropdown, a checkbox or a button.
            #[unsafe(method(changed:))]
            fn changed(&self, sender: &NSControl) {
                let a = match sender.tag() {
                    START => Some(FormAction::Start),
                    CANCEL => Some(FormAction::Cancel),
                    t => Control::ALL.get(t as usize).and_then(|&c| {
                        let i = if c.is_checkbox() {
                            // SAFETY: a checkbox's control is an NSButton.
                            let b: &NSButton = unsafe { &*(sender as *const NSControl).cast() };
                            (b.state() == NSControlStateValueOn) as usize
                        } else {
                            // SAFETY: a dropdown's control is an NSPopUpButton.
                            let p: &NSPopUpButton = unsafe { &*(sender as *const NSControl).cast() };
                            p.indexOfSelectedItem().max(0) as usize
                        };
                        c.chosen(i)
                    }),
                };
                if let Some(a) = a {
                    self.apply(a);
                }
            }

            /// The Points field's delegate: each edit, as it is typed.
            #[unsafe(method(controlTextDidChange:))]
            fn text_changed(&self, _note: &AnyObject) {
                let text = self.ivars().shown.get().map(|s| s.field.stringValue().to_string());
                if let Some(t) = text {
                    self.apply(FormAction::Points(t));
                }
            }
        }

        unsafe impl NSObjectProtocol for Target {}
    );

    impl Target {
        fn new(mtm: MainThreadMarker) -> Retained<Target> {
            let this = Target::alloc(mtm).set_ivars(Ivars {
                form: RefCell::new(StartLevelForm::new()),
                shown: OnceCell::new(),
                outcome: Cell::new(None),
            });
            // SAFETY: NSObject's `init`, on a freshly allocated object.
            unsafe { msg_send![super(this), init] }
        }

        fn apply(&self, a: FormAction) {
            let words = a.words();
            let done = self.ivars().form.borrow_mut().apply(a);
            self.show();
            match done {
                Some(o) => {
                    self.ivars().outcome.set(Some(o));
                    NSApplication::sharedApplication(self.mtm()).stopModal();
                }
                None if words == "start" => eprintln!("superfrog: start-level: Start is disabled"),
                None => {}
            }
        }

        /// Start enabled, and the range beside Points, as the model has them.
        fn show(&self) {
            let Some(s) = self.ivars().shown.get() else { return };
            let f = self.ivars().form.borrow();
            s.start.setEnabled(f.can_start());
            s.hint.setStringValue(&NSString::from_str(f.points_hint()));
        }
    }

    define_class!(
        // SAFETY: NSWindow may be subclassed; `EditingWindow` adds one override and no `Drop`.
        #[unsafe(super(NSWindow, NSResponder, NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "SuperfrogStartLevelWindow"]
        struct EditingWindow;

        impl EditingWindow {
            /// The app has no Edit menu, whose items are what ⌘A, ⌘C, ⌘V, ⌘X and ⌘Z normally
            /// reach, so the window sends their actions itself: the Points field can be pasted into.
            #[unsafe(method(performKeyEquivalent:))]
            fn perform_key_equivalent(&self, event: &NSEvent) -> Bool {
                let flags = event.modifierFlags() & NSEventModifierFlags::DeviceIndependentFlagsMask;
                let key = event.charactersIgnoringModifiers().map(|c| c.to_string());
                if flags == NSEventModifierFlags::Command
                    && let Some(action) = key.as_deref().and_then(editing)
                {
                    let app = NSApplication::sharedApplication(self.mtm());
                    // SAFETY: a standard responder action, sent down the responder chain from
                    // the field being edited, as an Edit menu item sends it.
                    if unsafe { app.sendAction_to_from(action, None, Some(self)) } {
                        return Bool::YES;
                    }
                }
                // SAFETY: NSWindow's own handling, for every other key equivalent (Return, Esc).
                unsafe { msg_send![super(self), performKeyEquivalent: event] }
            }
        }
    );

    /// The editing action an Edit menu would give ⌘ and `key`.
    fn editing(key: &str) -> Option<Sel> {
        Some(match key {
            "a" => sel!(selectAll:),
            "c" => sel!(copy:),
            "v" => sel!(paste:),
            "x" => sel!(cut:),
            "z" => sel!(undo:),
            _ => return None,
        })
    }

    fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
    }

    /// Run the window until Start or Cancel: the values and whether to record, `None` for Cancel.
    pub fn run_modal(parent: Option<&winit::window::Window>) -> Option<(LevelSetup, bool)> {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("superfrog: the Start Level window opens on the main thread");
            return None;
        };
        // SAFETY: a fresh window; NSWindow's documented initialiser, which the subclass inherits.
        let window: Retained<EditingWindow> = unsafe {
            msg_send![
                EditingWindow::alloc(mtm),
                initWithContentRect: rect(0.0, 0.0, SIZE.0, SIZE.1),
                styleMask: NSWindowStyleMask::Titled,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        // SAFETY: the window is owned by this `Retained`; AppKit must not release it on close too.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str("Start Level"));
        let view: Retained<NSView> = window.contentView().expect("a window's content view");
        let target = Target::new(mtm);
        let form = StartLevelForm::new();
        // The content view is not flipped: y grows up, so rows are placed from the top down.
        let top = |y: f64, h: f64| SIZE.1 - y - h;
        let label = |text: &str, frame: NSRect| {
            let l = NSTextField::labelWithString(&NSString::from_str(text), mtm);
            l.setFrame(frame);
            view.addSubview(&l);
            l
        };
        let mut row = 20.0;
        let mut first: Option<Retained<NSView>> = None;
        let mut field = None;
        for (tag, &c) in Control::ALL.iter().enumerate() {
            if c == Control::Energy {
                // Points sits between Lives and Energy, as in the prototype.
                label("Points", rect(20.0, top(row + 3.0, 20.0), 120.0, 20.0));
                let f = NSTextField::textFieldWithString(&NSString::from_str(&form.points_text), mtm);
                f.setFrame(rect(150.0, top(row, 24.0), 100.0, 24.0));
                f.setAccessibilityIdentifier(Some(&NSString::from_str("points")));
                // SAFETY: the target outlives the field (both are kept until the window closes);
                // AppKit calls `controlTextDidChange:` on a delegate that answers it.
                let _: () = unsafe { msg_send![&*f, setDelegate: &*target] };
                view.addSubview(&f);
                field = Some(f);
                row += 34.0;
            }
            let control: Retained<NSControl> = if c.is_checkbox() {
                // SAFETY: as the Controls window's buttons: the target outlives the checkbox.
                let b = unsafe {
                    NSButton::checkboxWithTitle_target_action(
                        &NSString::from_str(c.label()),
                        Some(&target as &AnyObject),
                        Some(sel!(changed:)),
                        mtm,
                    )
                };
                b.setFrame(rect(150.0, top(row, 22.0), 190.0, 22.0));
                b.setState(c.index(&form) as isize);
                row += 26.0;
                Retained::into_super(b)
            } else {
                label(c.label(), rect(20.0, top(row + 3.0, 20.0), 120.0, 20.0));
                let p = NSPopUpButton::initWithFrame_pullsDown(
                    NSPopUpButton::alloc(mtm),
                    rect(146.0, top(row, 26.0), 120.0, 26.0),
                    false,
                );
                for t in c.titles() {
                    p.addItemWithTitle(&NSString::from_str(&t));
                }
                p.selectItemAtIndex(c.index(&form) as isize);
                // SAFETY: the target outlives the dropdown; `changed:` takes the sender.
                unsafe {
                    p.setTarget(Some(&target as &AnyObject));
                    p.setAction(Some(sel!(changed:)));
                }
                row += 34.0;
                Retained::into_super(Retained::into_super(p))
            };
            control.setTag(tag as isize);
            control.setAccessibilityIdentifier(Some(&NSString::from_str(c.id())));
            view.addSubview(&control);
            if first.is_none() {
                first = Some(Retained::into_super(control));
            }
        }
        let field = field.expect("the Points field");
        let hint = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        hint.setFrame(rect(258.0, top(20.0 + 2.0 * 34.0 + 3.0, 20.0), 96.0, 20.0));
        hint.setTextColor(Some(&NSColor::systemRedColor()));
        hint.setAccessibilityIdentifier(Some(&NSString::from_str("points-range")));
        view.addSubview(&hint);
        let button = |title: &str, tag: isize, key: &str, x: f64| {
            // SAFETY: as above.
            let b = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str(title),
                    Some(&target as &AnyObject),
                    Some(sel!(changed:)),
                    mtm,
                )
            };
            b.setFrame(rect(x, 16.0, 90.0, 30.0));
            b.setTag(tag);
            // Return and Esc reach these before the field or a dropdown does.
            b.setKeyEquivalent(&NSString::from_str(key));
            b.setAccessibilityIdentifier(Some(&NSString::from_str(&title.to_lowercase())));
            view.addSubview(&b);
            b
        };
        let _cancel = button("Cancel", CANCEL, "\u{1b}", SIZE.0 - 200.0);
        let start = button("Start", START, "\r", SIZE.0 - 106.0);
        let _ = target.ivars().shown.set(Shown { start, hint, field });
        target.show();
        window.setInitialFirstResponder(first.as_deref());
        window.center();
        window.makeKeyAndOrderFront(None);
        let app = NSApplication::sharedApplication(mtm);
        app.runModalForWindow(&window);
        window.orderOut(None);
        if let Some(p) = parent {
            p.focus_window();
        }
        target.ivars().outcome.take().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> LevelId {
        s.parse().unwrap()
    }

    #[test]
    fn the_window_opens_on_a_new_game_s_values_with_the_recorder_on() {
        let f = StartLevelForm::new();
        assert_eq!(f.setup, LevelSetup::default());
        assert_eq!(f.setup.level, id("W1-1"));
        assert_eq!(f.setup.lives, Lives::Three);
        assert_eq!((f.setup.points, f.setup.energy), (0, 4));
        assert_eq!(f.setup.difficulty, Difficulty::Normal);
        assert!(!f.setup.spud && !f.setup.wings && f.record);
        assert_eq!(f.points_text, "0");
        assert!(f.can_start());
        assert_eq!(f.points_hint(), "");
        let mut f = StartLevelForm::new();
        assert_eq!(f.apply(FormAction::Start), Some(Some((LevelSetup::default(), true))));
    }

    #[test]
    fn every_action_sets_its_value_and_start_returns_them_all() {
        let mut f = StartLevelForm::new();
        for a in [
            FormAction::Level(id("W1-2")),
            FormAction::Lives(Lives::Seven),
            FormAction::Points("450000".into()),
            FormAction::Energy(2),
            FormAction::Difficulty(Difficulty::Easy),
            FormAction::Spud(true),
            FormAction::Wings(true),
            FormAction::Record(false),
        ] {
            assert_eq!(f.apply(a.clone()), None, "{a:?} leaves the window open");
        }
        let want = LevelSetup {
            level: id("W1-2"),
            lives: Lives::Seven,
            points: 450_000,
            energy: 2,
            difficulty: Difficulty::Easy,
            spud: true,
            wings: true,
        };
        assert_eq!(f.setup, want);
        assert_eq!(f.apply(FormAction::Start), Some(Some((want, false))));
        // Unticked again, the flags go back off.
        f.apply(FormAction::Spud(false));
        f.apply(FormAction::Wings(false));
        f.apply(FormAction::Record(true));
        assert!(!f.setup.spud && !f.setup.wings && f.record);
        assert_eq!(f.apply(FormAction::Cancel), Some(None));
    }

    #[test]
    fn points_out_of_range_or_not_a_whole_number_disable_start_and_show_the_range() {
        let mut f = StartLevelForm::new();
        for bad in ["1000000", "", " ", "-1", "+5", "1e3", "12.5", "450 000", "abc", "99999999999999999999"] {
            assert_eq!(f.apply(FormAction::Points(bad.into())), None);
            assert!(!f.can_start(), "{bad:?}");
            assert_eq!(f.points_hint(), POINTS_RANGE, "{bad:?}");
            assert_eq!(f.apply(FormAction::Start), None, "Start refused for {bad:?}");
        }
        assert_eq!(f.setup.points, 0, "a refused field keeps the last good points");
        for (good, p) in [("450000", 450_000), ("999999", 999_999), ("0", 0), (" 12 ", 12), ("007", 7)] {
            f.apply(FormAction::Points(good.into()));
            assert!(f.can_start(), "{good:?}");
            assert_eq!(f.points_hint(), "");
            assert_eq!(f.points(), Some(p));
        }
        f.apply(FormAction::Points("450000".into()));
        assert_eq!(f.apply(FormAction::Start).unwrap().unwrap().0.points, 450_000);
    }

    #[test]
    fn a_level_or_an_energy_the_dropdowns_do_not_offer_changes_nothing() {
        let mut f = StartLevelForm::new();
        assert_eq!(f.apply(FormAction::Level(id("W6-2alt"))), None);
        assert_eq!(f.apply(FormAction::Energy(0)), None);
        assert_eq!(f.apply(FormAction::Energy(5)), None);
        assert_eq!(f, StartLevelForm::new());
    }

    #[test]
    fn each_dropdown_offers_exactly_its_values_and_each_item_chooses_its_value() {
        assert_eq!(Control::Level.titles().len(), 26, "the levels a game plays (T02, owner)");
        assert_eq!(Control::Level.titles()[0], "W1-1");
        assert_eq!(Control::Level.titles().last().unwrap(), "WW-1");
        assert!(!Control::Level.titles().iter().any(|t| t.contains("alt")));
        assert_eq!(Control::Lives.titles(), ["3", "5", "7"]);
        assert_eq!(Control::Energy.titles(), ["1", "2", "3", "4"]);
        assert_eq!(Control::Difficulty.titles(), ["Easy", "Normal"]);
        let f = StartLevelForm::new();
        for c in Control::ALL {
            let n = if c.is_checkbox() { 2 } else { c.titles().len() };
            for i in 0..n {
                let mut g = StartLevelForm::new();
                let a = c.chosen(i).unwrap_or_else(|| panic!("{c:?} {i}"));
                g.apply(a);
                assert_eq!(c.index(&g), i, "{c:?} item {i} shows itself chosen");
            }
            assert_eq!(c.chosen(n), if c.is_checkbox() { c.chosen(1) } else { None }, "{c:?}");
            // The defaults are what each control shows on opening.
            let shown = c.index(&f);
            let mut g = StartLevelForm::new();
            g.apply(c.chosen(shown).unwrap());
            assert_eq!(g, f, "{c:?} opens on the default");
        }
    }

    #[test]
    fn a_script_names_each_action_and_reads_its_words_back() {
        let all = [
            FormAction::Level(id("WB-1")),
            FormAction::Lives(Lives::Five),
            FormAction::Points("1000000".into()),
            FormAction::Points(String::new()),
            FormAction::Energy(1),
            FormAction::Difficulty(Difficulty::Easy),
            FormAction::Difficulty(Difficulty::Normal),
            FormAction::Spud(true),
            FormAction::Wings(false),
            FormAction::Record(false),
            FormAction::Start,
            FormAction::Cancel,
        ];
        for a in all {
            let w = a.words();
            let words: Vec<&str> = w.split(' ').collect();
            assert_eq!(FormAction::named(&words), Some(a.clone()), "{w:?}");
        }
        for bad in [
            &["level", "W6-2alt"][..],
            &["level", "W7-1"],
            &["lives", "4"],
            &["energy", "0"],
            &["energy", "5"],
            &["difficulty", "hard"],
            &["spud", "yes"],
            &["record"],
            &["go"],
            &[],
        ] {
            assert_eq!(FormAction::named(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn filling_from_the_defaults_starts_with_exactly_the_setup() {
        let s = LevelSetup {
            level: id("W4-3"),
            lives: Lives::Five,
            points: 123_456,
            energy: 1,
            difficulty: Difficulty::Easy,
            spud: true,
            wings: false,
        };
        for record in [true, false] {
            let mut f = StartLevelForm::new();
            let mut out = None;
            for a in FormAction::filling(&s, record) {
                out = f.apply(a);
            }
            assert_eq!(out, Some(Some((s, record))));
        }
    }
}
