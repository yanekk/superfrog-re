//! The recording bar: while a bug recording runs, a strip under the game picture, never over it,
//! with a red dot, "Recording", the recording's length, the ⌘. hint and STOP. Native AppKit views
//! on the game window's own view (as the Controls window builds its views), not drawn by wgpu.
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │               game picture               │
//! ├──────────────────────────────────────────┤
//! │ ● Recording  01:23           ⌘.  [STOP]  │  BAR_HEIGHT points
//! └──────────────────────────────────────────┘
//! ```
//!
//! The game's window grows by [`BAR_HEIGHT`] while the bar shows and shrinks back after, so the
//! picture keeps its size and scale; in fullscreen the picture's scale is the largest that fits
//! above it. Growing the window is the game's (its window code), as are showing and hiding the bar.
//! The time is the recording's own length, the frames it holds at the PAL rate, so it stands still
//! while the game is paused or a question is open. STOP is Game ▸ Stop Recording, as ⌘. is.

/// The bar's height in points: room for a regular push button and 13-point labels.
pub const BAR_HEIGHT: u32 = 34;

/// The recording's length after `frames` PAL frames (313 lines of 227 Paula clocks each):
/// `mm:ss`, and `h:mm:ss` from an hour.
pub fn elapsed(frames: usize) -> String {
    use amiga_paula::{CLOCKS_PER_LINE, LINES_PER_FRAME, PAULA_CLOCK_PAL};
    let clocks = frames as u64 * (LINES_PER_FRAME * CLOCKS_PER_LINE) as u64;
    let s = clocks / PAULA_CLOCK_PAL as u64;
    match s / 3600 {
        0 => format!("{:02}:{:02}", s / 60, s % 60),
        h => format!("{h}:{:02}:{:02}", s / 60 % 60, s % 60),
    }
}

/// Points between the bar's edge and its first and last views, and between views.
const PAD: f64 = 12.0;
const GAP: f64 = 8.0;
/// Between the dot and "Recording", which read as one.
const DOT_GAP: f64 = 4.0;
/// STOP is never narrower than this, so it reads as a button even with a short title.
const STOP_MIN: f64 = 72.0;

/// Where the bar's five views go across a bar `width` points wide, from their widths
/// `[dot, label, time, hint, stop]`: each view's `x`. The dot, "Recording" and the time from the
/// left, the hint and STOP from the right. `None` when they would overlap: the caller then hides
/// the hint, which is the only one that can go.
pub fn place(width: f64, w: [f64; 5]) -> Option<[f64; 5]> {
    let dot = PAD;
    let label = dot + w[0] + DOT_GAP;
    let time = label + w[1] + GAP;
    let stop = width - PAD - w[4];
    let hint = stop - GAP - w[3];
    (time + w[2] + GAP <= hint).then_some([dot, label, time, hint, stop])
}

#[cfg(target_os = "macos")]
pub use view::RecordingBar;

#[cfg(not(target_os = "macos"))]
pub use elsewhere::RecordingBar;

/// Only macOS is built for; elsewhere the bar is not shown and ⌘. stops.
#[cfg(not(target_os = "macos"))]
mod elsewhere {
    pub struct RecordingBar;

    impl RecordingBar {
        pub fn new(_: &winit::window::Window) -> Result<RecordingBar, String> {
            Err("the recording bar is macOS only".into())
        }
        pub fn set_shown(&mut self, _: bool) {}
        pub fn set_time(&mut self, _: &str) {}
        pub fn layout(&mut self) {}
        pub fn next_click() -> bool {
            false
        }
    }
}

#[cfg(target_os = "macos")]
mod view {
    use std::sync::atomic::{AtomicU32, Ordering};

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
    use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
    use objc2_app_kit::{
        NSAccessibility, NSAutoresizingMaskOptions, NSBox, NSBoxType, NSButton, NSColor, NSControl,
        NSFont, NSResponder, NSTextField, NSTitlePosition, NSView,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use winit::window::Window;

    use super::{BAR_HEIGHT, STOP_MIN, place};

    /// STOP's clicks not yet taken by [`RecordingBar::next_click`].
    static CLICKS: AtomicU32 = AtomicU32::new(0);

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and `Target` implements no `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "AmigaAppRecordingBarTarget"]
        struct Target;

        impl Target {
            #[unsafe(method(stop:))]
            fn stop(&self, _sender: &NSControl) {
                CLICKS.fetch_add(1, Ordering::Relaxed);
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
        // SAFETY: NSBox may be subclassed; `Strip` adds no state and implements no `Drop`.
        #[unsafe(super(NSBox, NSView, NSResponder, NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "AmigaAppRecordingBarStrip"]
        struct Strip;

        /// A click on the strip (not on STOP) is the bar's, not the game's: unhandled it would
        /// travel up to winit's view and reach the game as a mouse button.
        impl Strip {
            #[unsafe(method(mouseDown:))]
            fn mouse_down(&self, _event: &AnyObject) {}

            #[unsafe(method(mouseUp:))]
            fn mouse_up(&self, _event: &AnyObject) {}

            #[unsafe(method(rightMouseDown:))]
            fn right_mouse_down(&self, _event: &AnyObject) {}

            #[unsafe(method(rightMouseUp:))]
            fn right_mouse_up(&self, _event: &AnyObject) {}
        }

        unsafe impl NSObjectProtocol for Strip {}
    );

    fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
    }

    pub struct RecordingBar {
        /// winit's view of the game window, which the strip is a subview of. Retained, so the
        /// strip is never left on a view that is gone.
        parent: Retained<NSView>,
        strip: Retained<Strip>,
        /// `[dot, label, time, hint]`.
        labels: [Retained<NSTextField>; 4],
        stop: Retained<NSButton>,
        _target: Retained<Target>,
        /// The time last shown, so only a change is set.
        time: String,
    }

    impl RecordingBar {
        /// The bar on `window`'s view, hidden.
        pub fn new(window: &Window) -> Result<RecordingBar, String> {
            let mtm = MainThreadMarker::new().ok_or("the recording bar is built on the main thread")?;
            let handle = window.window_handle().map_err(|e| format!("the game window: {e}"))?;
            let RawWindowHandle::AppKit(h) = handle.as_raw() else {
                return Err("the game window is not an AppKit window".into());
            };
            // SAFETY: winit's view, alive while the window is; retained here. Main thread only.
            let parent: Retained<NSView> = unsafe { Retained::retain(h.ns_view.cast::<NSView>().as_ptr()) }
                .ok_or("the game window has no view")?;
            let this = Strip::alloc(mtm).set_ivars(());
            // SAFETY: NSBox's designated initializer, on a freshly allocated object.
            let strip: Retained<Strip> = unsafe { msg_send![super(this), initWithFrame: rect(0.0, 0.0, 1.0, 1.0)] };
            strip.setBoxType(NSBoxType::Custom);
            strip.setTitlePosition(NSTitlePosition::NoTitle);
            strip.setBorderWidth(0.0);
            strip.setCornerRadius(0.0);
            strip.setContentViewMargins(NSSize::new(0.0, 0.0));
            // The prototype's #2b2b2b, whatever the system's appearance.
            strip.setFillColor(&NSColor::colorWithSRGBRed_green_blue_alpha(0.169, 0.169, 0.169, 1.0));
            strip.setAccessibilityIdentifier(Some(&NSString::from_str("recording-bar")));
            strip.setHidden(true);
            let content = strip.contentView().ok_or("the bar has no content view")?;

            let label = |text: &str, id: &str, color: Retained<NSColor>, font: Retained<NSFont>| {
                let l = NSTextField::labelWithString(&NSString::from_str(text), mtm);
                l.setTextColor(Some(&color));
                l.setFont(Some(&font));
                l.setAccessibilityIdentifier(Some(&NSString::from_str(id)));
                content.addSubview(&l);
                l
            };
            let regular = NSFont::systemFontOfSize(13.0);
            let dot = label("●", "recording-dot", NSColor::systemRedColor(), regular.clone());
            let text = label(
                "Recording",
                "recording-label",
                NSColor::colorWithWhite_alpha(0.93, 1.0),
                NSFont::boldSystemFontOfSize(13.0),
            );
            // Digits all one width, so the time does not jitter as it counts.
            let time = label(
                &super::elapsed(0),
                "recording-time",
                NSColor::colorWithWhite_alpha(0.67, 1.0),
                NSFont::monospacedDigitSystemFontOfSize_weight(13.0, 0.0 /* NSFontWeightRegular */),
            );
            let hint = label("⌘.", "recording-hint", NSColor::colorWithWhite_alpha(0.67, 1.0), regular);

            let target = Target::new(mtm);
            // SAFETY: the target outlives the button (both are this struct's); `stop:` is the
            // target's method taking the sender.
            let stop = unsafe {
                NSButton::buttonWithTitle_target_action(
                    &NSString::from_str("STOP"),
                    Some(&target as &AnyObject),
                    Some(sel!(stop:)),
                    mtm,
                )
            };
            // The keyboard stays with the game: Space must not click STOP.
            stop.setRefusesFirstResponder(true);
            stop.setAccessibilityIdentifier(Some(&NSString::from_str("recording-stop")));
            content.addSubview(&stop);

            parent.addSubview(&strip);
            let mut bar = RecordingBar {
                parent,
                strip,
                labels: [dot, text, time, hint],
                stop,
                _target: target,
                time: super::elapsed(0),
            };
            bar.layout();
            Ok(bar)
        }

        pub fn set_shown(&mut self, on: bool) {
            self.strip.setHidden(!on);
            if on {
                self.layout();
            }
        }

        /// Show `time` as the recording's length.
        pub fn set_time(&mut self, time: &str) {
            if time != self.time {
                self.labels[2].setStringValue(&NSString::from_str(time));
                self.time = time.to_string();
                // From an hour the time is wider.
                self.layout();
            }
        }

        /// The strip along the bottom of the window's view, `BAR_HEIGHT` high, its views placed
        /// across it at their natural sizes, centred in its height.
        pub fn layout(&mut self) {
            let bounds = self.parent.bounds();
            let (w, h) = (bounds.size.width, bounds.size.height);
            let bar = BAR_HEIGHT as f64;
            // winit's view is flipped, y down from the top; an unflipped one has it up.
            let y = if self.parent.isFlipped() { h - bar } else { 0.0 };
            self.strip.setFrame(rect(0.0, y, w, bar));
            // On a later resize the strip keeps to the bottom by itself, until laid out again.
            let bottom = if self.parent.isFlipped() {
                NSAutoresizingMaskOptions::ViewMinYMargin
            } else {
                NSAutoresizingMaskOptions::ViewMaxYMargin
            };
            self.strip.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | bottom);

            let mut sizes = [NSSize::new(0.0, 0.0); 5];
            for (i, l) in self.labels.iter().enumerate() {
                l.sizeToFit();
                sizes[i] = l.frame().size;
            }
            self.stop.sizeToFit();
            let s = self.stop.frame().size;
            sizes[4] = NSSize::new(s.width.max(STOP_MIN), s.height);
            let widths = sizes.map(|s| s.width);
            let (xs, hint) = match place(w, widths) {
                Some(xs) => (xs, true),
                None => {
                    let mut no_hint = widths;
                    no_hint[3] = 0.0;
                    (place(w, no_hint).unwrap_or([0.0; 5]), false)
                }
            };
            self.labels[3].setHidden(!hint);
            let views: [&NSView; 5] = [&self.labels[0], &self.labels[1], &self.labels[2], &self.labels[3], &self.stop];
            for ((v, x), size) in views.into_iter().zip(xs).zip(sizes) {
                v.setFrame(rect(x, ((bar - size.height) / 2.0).round(), size.width, size.height));
            }
        }

        /// Was STOP clicked since the last call? One click a call.
        pub fn next_click() -> bool {
            CLICKS
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1))
                .is_ok()
        }
    }

    impl Drop for RecordingBar {
        fn drop(&mut self) {
            self.strip.removeFromSuperview();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_is_the_recording_s_length_at_the_pal_rate() {
        assert_eq!(elapsed(0), "00:00");
        assert_eq!(elapsed(49), "00:00");
        assert_eq!(elapsed(50), "00:01");
        // A PAL frame is 313 × 227 / 3 546 895 s, a little over a fiftieth.
        assert_eq!(elapsed(2_995), "00:59");
        assert_eq!(elapsed(2_996), "01:00");
        assert_eq!(elapsed(179_713), "59:59");
        assert_eq!(elapsed(179_714), "1:00:00");
        assert_eq!(elapsed(amiga_record::MAX_FRAMES as usize), "24:02:17", "a day at 50 a second");
    }

    /// Widths as AppKit measures the views at 13 points.
    const WIDTHS: [f64; 5] = [12.0, 70.0, 40.0, 18.0, 72.0];

    #[test]
    fn at_the_smallest_window_every_view_fits_without_overlap() {
        // The smallest game window the bar is laid out on so far, 334 points wide.
        let min = 334.0;
        let xs = place(min, WIDTHS).expect("all five fit 334 points");
        for i in 0..4 {
            assert!(xs[i] + WIDTHS[i] <= xs[i + 1], "view {i} overlaps the next: {xs:?}");
        }
        assert!(xs[0] >= 0.0 && xs[4] + WIDTHS[4] <= min, "{xs:?}");
        assert_eq!(xs[4] + WIDTHS[4], min - PAD, "STOP keeps to the right edge");
    }

    #[test]
    fn too_narrow_for_all_says_so() {
        assert!(place(200.0, WIDTHS).is_none());
        let mut no_hint = WIDTHS;
        no_hint[3] = 0.0;
        assert!(place(260.0, no_hint).is_some(), "without the hint");
    }
}
