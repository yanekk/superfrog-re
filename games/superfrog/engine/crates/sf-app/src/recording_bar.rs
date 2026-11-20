//! The recording bar (bug-recorder DESIGN §2.3, D4): while a bug recording runs, a strip under the
//! game picture, never over it, with a red dot, "Recording", the recording's length, the ⌘. hint
//! and STOP. Its view, layout and clock are the toolkit's (`amiga_app::recording_bar`, moved there
//! by forest-dumb-level-1 T12); this is the app's glue: showing it while a recording runs and
//! growing the window to hold it, so the picture keeps its size and scale
//! (`window::picture_area`). STOP is Game ▸ Stop Recording, as ⌘. is.

use winit::dpi::LogicalSize;

pub use amiga_app::recording_bar::{RecordingBar, elapsed};

use crate::App;
use crate::window::{self, BAR_HEIGHT};

impl App {
    /// The bar shows exactly while a recording runs: shown or hidden with the window grown or
    /// shrunk to hold it, and its time set. Called whenever the menu bar is brought up to date.
    pub(crate) fn show_bar(&mut self) {
        let Some(run) = &mut self.run else { return };
        let on = run.recorder.is_recording();
        if on != run.video.bar {
            run.video.bar = on;
            if on && run.bar.is_none() {
                match RecordingBar::new(&run.window) {
                    Ok(b) => run.bar = Some(b),
                    Err(e) => eprintln!("superfrog: no recording bar: {e}"),
                }
            }
            if let Some(b) = &mut run.bar {
                b.set_shown(on);
            }
            eprintln!(
                "superfrog: recording bar {} after step {}",
                if on { "shown" } else { "hidden" },
                run.steps
            );
            // The size before the minimum changes: winit grows a window below a new minimum at
            // once, and the bar's height is added to the size it had, not to that.
            let at = run.window.inner_size().to_logical(run.window.scale_factor());
            run.window
                .set_min_inner_size(Some(window::min_size(run.game.widescreen(), on)));
            self.fit_window_to_bar(Some(at));
        }
        let run = self.run.as_mut().expect("running");
        if on && let (Some(b), Some(frames)) = (&mut run.bar, run.recorder.frames()) {
            b.set_time(&elapsed(frames));
        }
    }

    /// A window (not fullscreen) is the bar's height taller while the bar shows: grown or shrunk
    /// by it when that changed, now or while fullscreen. The width and the rest of the height are
    /// left as they are, so the picture keeps its scale, a resized window's included.
    /// `at`: the window's size to grow or shrink, when not the size it has now. (Never set the
    /// minimum size here: on a window going fullscreen winit applies it as a resize, and the
    /// window falls back to its windowed size.)
    pub(crate) fn fit_window_to_bar(&mut self, at: Option<LogicalSize<f64>>) {
        let run = self.run.as_mut().expect("running");
        if run.settings.fullscreen || run.window_bar == run.video.bar {
            return;
        }
        let at = at.unwrap_or_else(|| run.window.inner_size().to_logical(run.window.scale_factor()));
        let by = if run.video.bar { BAR_HEIGHT as f64 } else { -(BAR_HEIGHT as f64) };
        let _ = run.window.request_inner_size(LogicalSize::new(at.width, at.height + by));
        run.window_bar = run.video.bar;
    }

    /// The window was resized: the bar keeps to its bottom edge, across its width.
    pub(crate) fn layout_bar(&mut self) {
        if let Some(b) = self.run.as_mut().and_then(|r| r.bar.as_mut()) {
            b.layout();
        }
    }
}

#[cfg(test)]
mod tests {
    use amiga_app::recording_bar::place;

    /// Widths as AppKit measures the views at 13 points.
    const WIDTHS: [f64; 5] = [12.0, 70.0, 40.0, 18.0, 72.0];

    #[test]
    fn at_the_smallest_window_every_view_fits_without_overlap() {
        let min = crate::window::min_size(false, true).width as f64;
        let xs = place(min, WIDTHS).expect("all five fit 334 points");
        for i in 0..4 {
            assert!(xs[i] + WIDTHS[i] <= xs[i + 1], "view {i} overlaps the next: {xs:?}");
        }
        assert!(xs[0] >= 0.0 && xs[4] + WIDTHS[4] <= min, "{xs:?}");
        assert_eq!(xs[4] + WIDTHS[4], min - 12.0, "STOP keeps to the right edge");
    }

    #[test]
    fn the_time_shows_the_longest_recording() {
        assert_eq!(super::elapsed(sf_core::recording::MAX_FRAMES as usize), "24:02:17", "a day at 50 a second");
    }
}
