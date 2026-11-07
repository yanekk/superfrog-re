//! Game → Controls…: the toolkit's window (`amiga_app::controls_window`, T12) and this app's glue,
//! as `sf-app`'s: opening it, routing its keys and pads, saving, and the log a test follows (every
//! line it shows is written as `fd-app: controls: …`). Restore Defaults restores this game's keys
//! (`settings::default_keys`), not the toolkit's.

use amiga_app::controls_window::{Controls, ControlsAction, ControlsWindow, Outcome};
use amiga_app::pad::{PadEvent, PadState};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

use crate::App;
use crate::input::HostInput;

/// The open window: its model, its AppKit view, and what the pads held when it opened (to tell a
/// new press from one already held).
pub struct OpenControls {
    pub model: Controls,
    pub window: ControlsWindow,
    pads: PadState,
}

impl App {
    /// Open the window. It takes the keyboard, so the game lets go of every key held.
    pub(crate) fn open_controls(&mut self, event_loop: &ActiveEventLoop) {
        if self.controls.is_some() {
            return;
        }
        match ControlsWindow::open(event_loop, !self.args.unattended()) {
            Ok(window) => {
                let mut pads = PadState::default();
                if let Some(run) = &mut self.run {
                    run.input.release_keys();
                    pads = run.input.pads().clone();
                }
                self.controls = Some(OpenControls {
                    model: Controls::new(),
                    window,
                    pads,
                });
                self.show_controls();
            }
            Err(e) => eprintln!("fd-app: {e}"),
        }
    }

    /// A click in the window, or a script's `controls …`.
    pub(crate) fn controls_action(&mut self, a: ControlsAction) {
        let (Some(c), Some(run)) = (&mut self.controls, &mut self.run) else {
            eprintln!("fd-app: the Controls window is not open");
            return;
        };
        let o = c.model.action(a, &mut run.settings);
        if a == ControlsAction::Defaults {
            run.settings.keys = crate::settings::default_keys();
        }
        self.controls_outcome(o);
    }

    /// A key pressed while the window has the keyboard.
    pub(crate) fn controls_key(&mut self, code: KeyCode) {
        let (Some(c), Some(run)) = (&mut self.controls, &mut self.run) else {
            return;
        };
        let o = c.model.key(code, &mut run.settings);
        self.controls_outcome(o);
    }

    /// A pad's event: the game's, unless the window waits for a pad input and this is one newly
    /// held, which then becomes the binding and never reaches the game.
    pub(crate) fn pad_event(&mut self, e: PadEvent) {
        match e {
            PadEvent::Connected(n) => eprintln!("fd-app: pad {n} connected"),
            PadEvent::Disconnected(n) => eprintln!("fd-app: pad {n} disconnected"),
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

    pub(crate) fn close_controls(&mut self) {
        if self.controls.take().is_some() {
            eprintln!("fd-app: controls closed");
            if let Some(run) = &self.run
                && !self.args.unattended()
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
                if let Err(e) = crate::settings::save(&self.home, &run.settings) {
                    eprintln!("fd-app: could not save the settings: {e}");
                }
            }
            Outcome::Close => return self.close_controls(),
        }
        self.show_controls();
    }

    fn show_controls(&mut self) {
        let (Some(c), Some(run)) = (&mut self.controls, &self.run) else {
            return;
        };
        if c.window.shown_status() != Some(c.model.status.as_str()) {
            eprintln!("fd-app: controls: {}", c.model.status);
        }
        c.window.show(&c.model, &run.settings);
    }
}
