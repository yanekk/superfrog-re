//! Game ▸ Start Level…: the window that starts a level with chosen values, the recorder on or off
//! among them. Native, as the Controls window is (`controls_window.rs`): AppKit dropdowns, number
//! fields, checkboxes and two buttons.
//!
//! ```text
//!   Level            [ first    ▾]          ← FieldKind::Dropdown
//!   Points           [ 0         ]  0–999   ← FieldKind::Number: the range shows only while the
//!                    [x] Enable recorder?      field is not a whole number in range, and Start is
//!                                  [ Cancel ] [ Start ]   disabled then  ← FieldKind::Checkbox
//! ```
//!
//! The game names its fields in a [`StartLevelSpec`], top to bottom, so one game lists twenty-six
//! levels and five more fields and another a single level and the recorder. The window holds
//! [`Value`]s, one a field, and gives them back on Start; what they mean is the game's.
//!
//! The window is modal to the game: the app runs it in AppKit's modal loop, so no frame is stepped
//! while it is open (the game is paused) and [`run_modal`] returns the answer. Return is Start, Esc
//! Cancel. It always opens on the fields' defaults.
//!
//! The model ([`Form`]) holds no AppKit and is what the tests drive; a script drives it with `form …`
//! lines, read by [`FieldAction::named`], answering the next Start Level… as the clicks would.

/// What kind of control a field is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// A dropdown of these titles, top to bottom; its value is a [`Value::Choice`].
    Dropdown(Vec<String>),
    /// A text field taking a whole number 0 to `max`; its value is a [`Value::Text`], as typed.
    /// `range` shows beside it while it holds anything else, and Start is disabled then.
    Number { max: u32, range: String },
    /// A checkbox titled with the field's label; its value is a [`Value::On`].
    Checkbox,
}

/// One row of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelField {
    /// Its accessibility identifier, for the OS layer, and its name in a script's `form` line.
    pub id: String,
    /// The label beside it; a checkbox's own title.
    pub label: String,
    pub kind: FieldKind,
    /// What it shows when the window opens.
    pub default: Value,
}

/// The game's fields, top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLevelSpec {
    pub fields: Vec<LevelField>,
}

/// A field's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A dropdown's item, by its index.
    Choice(usize),
    /// A number field's text, as typed.
    Text(String),
    /// A checkbox.
    On(bool),
}

/// Every field's value, in the spec's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Values(pub Vec<Value>);

impl Values {
    /// Field `i`'s dropdown item.
    pub fn choice(&self, i: usize) -> Option<usize> {
        match self.0.get(i)? {
            Value::Choice(c) => Some(*c),
            _ => None,
        }
    }

    /// Field `i`'s text.
    pub fn text(&self, i: usize) -> Option<&str> {
        match self.0.get(i)? {
            Value::Text(t) => Some(t),
            _ => None,
        }
    }

    /// Field `i`'s checkbox.
    pub fn on(&self, i: usize) -> Option<bool> {
        match self.0.get(i)? {
            Value::On(b) => Some(*b),
            _ => None,
        }
    }
}

/// `text` as a whole number 0 to `max`, `None` otherwise.
pub fn whole_number(text: &str, max: u32) -> Option<u32> {
    let t = text.trim();
    // Digits only: no sign, no spaces inside, no exponent. One digit more than `max` has may be a
    // leading zero; any longer run never reaches the parse.
    let digits = max.checked_ilog10().unwrap_or(0) as usize + 1;
    if t.is_empty() || t.len() > digits + 1 || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    t.parse().ok().filter(|&n| n <= max)
}

impl StartLevelSpec {
    /// The index of the field called `id`.
    pub fn field(&self, id: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.id == id)
    }

    /// Whether `v` is a value field `i` can hold: an item its dropdown offers, any text for a
    /// number field, on or off for a checkbox.
    pub fn accepts(&self, i: usize, v: &Value) -> bool {
        match (self.fields.get(i).map(|f| &f.kind), v) {
            (Some(FieldKind::Dropdown(items)), Value::Choice(c)) => *c < items.len(),
            (Some(FieldKind::Number { .. }), Value::Text(_)) => true,
            (Some(FieldKind::Checkbox), Value::On(_)) => true,
            _ => false,
        }
    }
}

/// What the person did in the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldAction {
    /// A field changed: a dropdown chosen, a number typed, a checkbox ticked.
    Set(usize, Value),
    Start,
    Cancel,
}

impl FieldAction {
    /// Its words in a script's `form …` line: a field's id then a dropdown's item (as titled, any
    /// case), a number field's text (nothing: empty) or `on`/`off`; or `start`, `cancel`.
    pub fn named(spec: &StartLevelSpec, words: &[&str]) -> Option<FieldAction> {
        let (id, rest) = match words {
            ["start"] => return Some(FieldAction::Start),
            ["cancel"] => return Some(FieldAction::Cancel),
            [id, rest @ ..] => (*id, rest),
            [] => return None,
        };
        let i = spec.field(id)?;
        let v = match (&spec.fields[i].kind, rest) {
            (FieldKind::Dropdown(items), [item]) => {
                Value::Choice(items.iter().position(|t| t.eq_ignore_ascii_case(item))?)
            }
            (FieldKind::Number { .. }, rest) => Value::Text(rest.join(" ")),
            (FieldKind::Checkbox, ["on"]) => Value::On(true),
            (FieldKind::Checkbox, ["off"]) => Value::On(false),
            _ => return None,
        };
        Some(FieldAction::Set(i, v))
    }

    /// The words [`FieldAction::named`] reads back.
    pub fn words(&self, spec: &StartLevelSpec) -> String {
        match self {
            FieldAction::Start => "start".into(),
            FieldAction::Cancel => "cancel".into(),
            FieldAction::Set(i, v) => {
                let Some(f) = spec.fields.get(*i) else { return String::new() };
                let value = match (&f.kind, v) {
                    (FieldKind::Dropdown(items), Value::Choice(c)) => items.get(*c).cloned().unwrap_or_default(),
                    (_, Value::Text(t)) => t.clone(),
                    (_, Value::On(b)) => if *b { "on" } else { "off" }.into(),
                    _ => String::new(),
                };
                format!("{} {value}", f.id).trim_end().to_string()
            }
        }
    }
}

/// The window's values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub spec: StartLevelSpec,
    pub values: Values,
}

impl Form {
    /// The window as it opens: every field on its default.
    pub fn new(spec: StartLevelSpec) -> Form {
        let values = Values(spec.fields.iter().map(|f| f.default.clone()).collect());
        Form { spec, values }
    }

    /// Number field `i` as a whole number in range, `None` while it holds anything else.
    pub fn number(&self, i: usize) -> Option<u32> {
        match &self.spec.fields.get(i)?.kind {
            FieldKind::Number { max, .. } => whole_number(self.values.text(i)?, *max),
            _ => None,
        }
    }

    /// Start is enabled only with every number field in range.
    pub fn can_start(&self) -> bool {
        self.spec
            .fields
            .iter()
            .enumerate()
            .all(|(i, f)| !matches!(f.kind, FieldKind::Number { .. }) || self.number(i).is_some())
    }

    /// What shows beside number field `i`: its range while it is refused, else nothing.
    pub fn hint(&self, i: usize) -> &str {
        match self.spec.fields.get(i).map(|f| &f.kind) {
            Some(FieldKind::Number { range, .. }) if self.number(i).is_none() => range,
            _ => "",
        }
    }

    /// A click, a choice or a key in the window: `None` while it stays open, `Some(None)` for
    /// Cancel, `Some(Some(..))` for Start with every value. Start with a number out of range does
    /// nothing, as its disabled button would; a value its field cannot hold changes nothing.
    pub fn apply(&mut self, a: FieldAction) -> Option<Option<Values>> {
        match a {
            FieldAction::Set(i, v) => {
                if self.spec.accepts(i, &v) {
                    self.values.0[i] = v;
                }
                None
            }
            FieldAction::Start => self.can_start().then(|| Some(self.values.clone())),
            FieldAction::Cancel => Some(None),
        }
    }
}

#[cfg(target_os = "macos")]
pub use view::run_modal;

/// Only macOS is built for; elsewhere the window says so and starts nothing. `app` names the app
/// in the log.
#[cfg(not(target_os = "macos"))]
pub fn run_modal(app: &str, _: &StartLevelSpec, _: Option<&winit::window::Window>) -> Option<Values> {
    eprintln!("{app}: the Start Level window is macOS only");
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

    use super::{FieldAction, FieldKind, Form, StartLevelSpec, Value, Values};

    /// The window's width in points; its height follows its rows.
    const WIDTH: f64 = 360.0;
    /// Above the first row, and below the last one for the buttons.
    const TOP: f64 = 20.0;
    const BOTTOM: f64 = 62.0;
    /// A dropdown's or number field's row, and a checkbox's.
    const ROW: f64 = 34.0;
    const CHECK_ROW: f64 = 26.0;
    /// The tags of the two buttons; a field's control has its index in the spec.
    const START: isize = 1000;
    const CANCEL: isize = 1001;

    /// The views the target changes as the model does.
    struct Shown {
        start: Retained<NSButton>,
        /// Each number field, by its index, and the range beside it.
        numbers: Vec<(usize, Retained<NSTextField>, Retained<NSTextField>)>,
    }

    struct Ivars {
        app: String,
        form: RefCell<Form>,
        shown: OnceCell<Shown>,
        /// Start's values or Cancel, once the window has closed on one.
        outcome: Cell<Option<Option<Values>>>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and `Target` implements no `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "AmigaAppStartLevelTarget"]
        #[ivars = Ivars]
        struct Target;

        impl Target {
            /// A dropdown, a checkbox or a button.
            #[unsafe(method(changed:))]
            fn changed(&self, sender: &NSControl) {
                let a = match sender.tag() {
                    START => Some(FieldAction::Start),
                    CANCEL => Some(FieldAction::Cancel),
                    t => {
                        let kind = self.ivars().form.borrow().spec.fields.get(t as usize).map(|f| f.kind.clone());
                        match kind {
                            Some(FieldKind::Checkbox) => {
                                // SAFETY: a checkbox's control is an NSButton.
                                let b: &NSButton = unsafe { &*(sender as *const NSControl).cast() };
                                Some(FieldAction::Set(t as usize, Value::On(b.state() == NSControlStateValueOn)))
                            }
                            Some(FieldKind::Dropdown(_)) => {
                                // SAFETY: a dropdown's control is an NSPopUpButton.
                                let p: &NSPopUpButton = unsafe { &*(sender as *const NSControl).cast() };
                                Some(FieldAction::Set(t as usize, Value::Choice(p.indexOfSelectedItem().max(0) as usize)))
                            }
                            _ => None,
                        }
                    }
                };
                if let Some(a) = a {
                    self.apply(a);
                }
            }

            /// A number field's delegate: each edit, as it is typed.
            #[unsafe(method(controlTextDidChange:))]
            fn text_changed(&self, _note: &AnyObject) {
                let texts: Vec<(usize, String)> = match self.ivars().shown.get() {
                    Some(s) => s.numbers.iter().map(|(i, f, _)| (*i, f.stringValue().to_string())).collect(),
                    None => return,
                };
                for (i, t) in texts {
                    self.apply(FieldAction::Set(i, Value::Text(t)));
                }
            }
        }

        unsafe impl NSObjectProtocol for Target {}
    );

    impl Target {
        fn new(mtm: MainThreadMarker, app: &str, form: Form) -> Retained<Target> {
            let this = Target::alloc(mtm).set_ivars(Ivars {
                app: app.to_string(),
                form: RefCell::new(form),
                shown: OnceCell::new(),
                outcome: Cell::new(None),
            });
            // SAFETY: NSObject's `init`, on a freshly allocated object.
            unsafe { msg_send![super(this), init] }
        }

        fn apply(&self, a: FieldAction) {
            let start = a == FieldAction::Start;
            let done = self.ivars().form.borrow_mut().apply(a);
            self.show();
            match done {
                Some(o) => {
                    self.ivars().outcome.set(Some(o));
                    NSApplication::sharedApplication(self.mtm()).stopModal();
                }
                None if start => eprintln!("{}: start-level: Start is disabled", self.ivars().app),
                None => {}
            }
        }

        /// Start enabled, and the range beside each number field, as the model has them.
        fn show(&self) {
            let Some(s) = self.ivars().shown.get() else { return };
            let f = self.ivars().form.borrow();
            s.start.setEnabled(f.can_start());
            for (i, _, hint) in &s.numbers {
                hint.setStringValue(&NSString::from_str(f.hint(*i)));
            }
        }
    }

    define_class!(
        // SAFETY: NSWindow may be subclassed; `EditingWindow` adds one override and no `Drop`.
        #[unsafe(super(NSWindow, NSResponder, NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "AmigaAppStartLevelWindow"]
        struct EditingWindow;

        impl EditingWindow {
            /// A game's app has no Edit menu, whose items are what ⌘A, ⌘C, ⌘V, ⌘X and ⌘Z normally
            /// reach, so the window sends their actions itself: a number field can be pasted into.
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

    /// The window's height for `spec`'s rows.
    fn height(spec: &StartLevelSpec) -> f64 {
        let rows: f64 = spec
            .fields
            .iter()
            .map(|f| if f.kind == FieldKind::Checkbox { CHECK_ROW } else { ROW })
            .sum();
        TOP + rows + BOTTOM
    }

    /// Run the window for `spec` until Start or Cancel: every field's value, `None` for Cancel.
    /// `app` names the app in the log; `parent` takes the keyboard back after.
    pub fn run_modal(app: &str, spec: &StartLevelSpec, parent: Option<&winit::window::Window>) -> Option<Values> {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("{app}: the Start Level window opens on the main thread");
            return None;
        };
        let size = (WIDTH, height(spec));
        // SAFETY: a fresh window; NSWindow's documented initialiser, which the subclass inherits.
        let window: Retained<EditingWindow> = unsafe {
            msg_send![
                EditingWindow::alloc(mtm),
                initWithContentRect: rect(0.0, 0.0, size.0, size.1),
                styleMask: NSWindowStyleMask::Titled,
                backing: NSBackingStoreType::Buffered,
                defer: false,
            ]
        };
        // SAFETY: the window is owned by this `Retained`; AppKit must not release it on close too.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str("Start Level"));
        let view: Retained<NSView> = window.contentView().expect("a window's content view");
        let form = Form::new(spec.clone());
        let target = Target::new(mtm, app, form.clone());
        // The content view is not flipped: y grows up, so rows are placed from the top down.
        let top = |y: f64, h: f64| size.1 - y - h;
        let label = |text: &str, frame: NSRect| {
            let l = NSTextField::labelWithString(&NSString::from_str(text), mtm);
            l.setFrame(frame);
            view.addSubview(&l);
            l
        };
        let mut row = TOP;
        let mut first: Option<Retained<NSView>> = None;
        // Each number field, and the row its range goes beside, added once every row is.
        let mut numbers = Vec::new();
        for (i, f) in spec.fields.iter().enumerate() {
            let control: Retained<NSControl> = match &f.kind {
                FieldKind::Number { .. } => {
                    label(&f.label, rect(20.0, top(row + 3.0, 20.0), 120.0, 20.0));
                    let text = form.values.text(i).unwrap_or_default();
                    let t = NSTextField::textFieldWithString(&NSString::from_str(text), mtm);
                    t.setFrame(rect(150.0, top(row, 24.0), 100.0, 24.0));
                    // SAFETY: the target outlives the field (both are kept until the window
                    // closes); AppKit calls `controlTextDidChange:` on a delegate that answers it.
                    let _: () = unsafe { msg_send![&*t, setDelegate: &*target] };
                    numbers.push((i, t.clone(), row));
                    row += ROW;
                    Retained::into_super(t)
                }
                FieldKind::Checkbox => {
                    // SAFETY: as the Controls window's buttons: the target outlives the checkbox.
                    let b = unsafe {
                        NSButton::checkboxWithTitle_target_action(
                            &NSString::from_str(&f.label),
                            Some(&target as &AnyObject),
                            Some(sel!(changed:)),
                            mtm,
                        )
                    };
                    b.setFrame(rect(150.0, top(row, 22.0), 190.0, 22.0));
                    b.setState(form.values.on(i).unwrap_or(false) as isize);
                    row += CHECK_ROW;
                    Retained::into_super(b)
                }
                FieldKind::Dropdown(items) => {
                    label(&f.label, rect(20.0, top(row + 3.0, 20.0), 120.0, 20.0));
                    let p = NSPopUpButton::initWithFrame_pullsDown(
                        NSPopUpButton::alloc(mtm),
                        rect(146.0, top(row, 26.0), 120.0, 26.0),
                        false,
                    );
                    for t in items {
                        p.addItemWithTitle(&NSString::from_str(t));
                    }
                    p.selectItemAtIndex(form.values.choice(i).unwrap_or(0) as isize);
                    // SAFETY: the target outlives the dropdown; `changed:` takes the sender.
                    unsafe {
                        p.setTarget(Some(&target as &AnyObject));
                        p.setAction(Some(sel!(changed:)));
                    }
                    row += ROW;
                    Retained::into_super(Retained::into_super(p))
                }
            };
            control.setTag(i as isize);
            control.setAccessibilityIdentifier(Some(&NSString::from_str(&f.id)));
            view.addSubview(&control);
            if first.is_none() && !matches!(f.kind, FieldKind::Number { .. }) {
                first = Some(Retained::into_super(control));
            }
        }
        let numbers = numbers
            .into_iter()
            .map(|(i, field, row)| {
                let hint = NSTextField::labelWithString(&NSString::from_str(""), mtm);
                hint.setFrame(rect(258.0, top(row + 3.0, 20.0), 96.0, 20.0));
                hint.setTextColor(Some(&NSColor::systemRedColor()));
                let id = format!("{}-range", spec.fields[i].id);
                hint.setAccessibilityIdentifier(Some(&NSString::from_str(&id)));
                view.addSubview(&hint);
                (i, field, hint)
            })
            .collect();
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
            // Return and Esc reach these before a field or a dropdown does.
            b.setKeyEquivalent(&NSString::from_str(key));
            b.setAccessibilityIdentifier(Some(&NSString::from_str(&title.to_lowercase())));
            view.addSubview(&b);
            b
        };
        let _cancel = button("Cancel", CANCEL, "\u{1b}", size.0 - 200.0);
        let start = button("Start", START, "\r", size.0 - 106.0);
        let _ = target.ivars().shown.set(Shown { start, numbers });
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::start_level_window::{LevelField, Value};

        #[test]
        fn the_window_is_as_tall_as_its_rows_and_the_buttons() {
            let field = |kind| LevelField { id: "x".into(), label: "X".into(), kind, default: Value::On(false) };
            let spec = StartLevelSpec {
                fields: vec![
                    field(FieldKind::Dropdown(vec!["a".into()])),
                    field(FieldKind::Number { max: 9, range: "0–9".into() }),
                    field(FieldKind::Checkbox),
                ],
            };
            assert_eq!(height(&spec), 20.0 + 34.0 + 34.0 + 26.0 + 62.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up game's window: two levels, a score and the recorder. No name in it is a game's.
    fn spec() -> StartLevelSpec {
        StartLevelSpec {
            fields: vec![
                LevelField {
                    id: "level".into(),
                    label: "Level".into(),
                    kind: FieldKind::Dropdown(vec!["L1".into(), "L2".into()]),
                    default: Value::Choice(0),
                },
                LevelField {
                    id: "points".into(),
                    label: "Points".into(),
                    kind: FieldKind::Number { max: 999_999, range: "0–999 999".into() },
                    default: Value::Text("0".into()),
                },
                LevelField {
                    id: "record".into(),
                    label: "Enable recorder?".into(),
                    kind: FieldKind::Checkbox,
                    default: Value::On(true),
                },
            ],
        }
    }

    #[test]
    fn a_one_field_spec_names_no_game_and_starts_on_its_default() {
        // The dry fit: the smallest window a second game can list, its one level.
        let one = StartLevelSpec {
            fields: vec![LevelField {
                id: "level".into(),
                label: "Level".into(),
                kind: FieldKind::Dropdown(vec!["1".into()]),
                default: Value::Choice(0),
            }],
        };
        let mut f = Form::new(one.clone());
        assert!(f.can_start());
        assert_eq!(f.hint(0), "");
        assert_eq!(f.apply(FieldAction::Set(0, Value::Choice(1))), None, "an item it does not offer");
        assert_eq!(f.apply(FieldAction::Start), Some(Some(Values(vec![Value::Choice(0)]))));
        assert_eq!(FieldAction::named(&one, &["level", "1"]), Some(FieldAction::Set(0, Value::Choice(0))));
        assert_eq!(FieldAction::named(&one, &["level", "2"]), None);
    }

    #[test]
    fn the_window_opens_on_the_defaults_and_start_returns_every_value() {
        let mut f = Form::new(spec());
        assert_eq!(f.values, Values(vec![Value::Choice(0), Value::Text("0".into()), Value::On(true)]));
        for a in [
            FieldAction::Set(0, Value::Choice(1)),
            FieldAction::Set(1, Value::Text("450000".into())),
            FieldAction::Set(2, Value::On(false)),
        ] {
            assert_eq!(f.apply(a.clone()), None, "{a:?} leaves the window open");
        }
        let got = f.apply(FieldAction::Start).unwrap().unwrap();
        assert_eq!((got.choice(0), got.text(1), got.on(2)), (Some(1), Some("450000"), Some(false)));
        assert_eq!(f.number(1), Some(450_000));
        assert_eq!(f.apply(FieldAction::Cancel), Some(None));
    }

    #[test]
    fn a_number_out_of_range_or_not_whole_disables_start_and_shows_the_range() {
        let mut f = Form::new(spec());
        for bad in ["1000000", "", " ", "-1", "+5", "1e3", "12.5", "450 000", "abc", "99999999999999999999"] {
            f.apply(FieldAction::Set(1, Value::Text(bad.into())));
            assert!(!f.can_start(), "{bad:?}");
            assert_eq!(f.hint(1), "0–999 999", "{bad:?}");
            assert_eq!(f.apply(FieldAction::Start), None, "Start refused for {bad:?}");
        }
        for (good, n) in [("450000", 450_000), ("999999", 999_999), ("0", 0), (" 12 ", 12), ("007", 7), ("0999999", 999_999)] {
            f.apply(FieldAction::Set(1, Value::Text(good.into())));
            assert!(f.can_start(), "{good:?}");
            assert_eq!((f.hint(1), f.number(1)), ("", Some(n)));
        }
        assert_eq!(f.hint(0), "", "only a number field has a range");
    }

    #[test]
    fn a_value_its_field_cannot_hold_changes_nothing() {
        let mut f = Form::new(spec());
        for a in [
            FieldAction::Set(0, Value::Choice(2)),
            FieldAction::Set(0, Value::On(true)),
            FieldAction::Set(1, Value::Choice(0)),
            FieldAction::Set(2, Value::Text("on".into())),
            FieldAction::Set(3, Value::On(true)),
        ] {
            assert_eq!(f.apply(a.clone()), None);
            assert_eq!(f, Form::new(spec()), "{a:?}");
        }
    }

    #[test]
    fn whole_numbers_take_one_leading_digit_more_than_the_maximum_has() {
        assert_eq!(whole_number("9", 9), Some(9));
        assert_eq!(whole_number("09", 9), Some(9));
        assert_eq!(whole_number("009", 9), None);
        assert_eq!(whole_number("10", 9), None);
        assert_eq!(whole_number("0", 0), Some(0));
        assert_eq!(whole_number("4294967295", u32::MAX), Some(u32::MAX));
        assert_eq!(whole_number("4294967296", u32::MAX), None);
    }

    #[test]
    fn a_script_names_each_action_and_reads_its_words_back() {
        let s = spec();
        let all = [
            FieldAction::Set(0, Value::Choice(1)),
            FieldAction::Set(1, Value::Text("1000000".into())),
            FieldAction::Set(1, Value::Text(String::new())),
            FieldAction::Set(2, Value::On(false)),
            FieldAction::Set(2, Value::On(true)),
            FieldAction::Start,
            FieldAction::Cancel,
        ];
        for a in all {
            let w = a.words(&s);
            let words: Vec<&str> = w.split(' ').collect();
            assert_eq!(FieldAction::named(&s, &words), Some(a.clone()), "{w:?}");
        }
        assert_eq!(FieldAction::named(&s, &["level", "l2"]), Some(FieldAction::Set(0, Value::Choice(1))), "any case");
        for bad in [&["level", "L3"][..], &["level"], &["record", "yes"], &["record"], &["lives", "3"], &["go"], &[]] {
            assert_eq!(FieldAction::named(&s, bad), None, "{bad:?}");
        }
    }
}
