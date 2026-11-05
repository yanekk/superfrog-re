//! Game → Controls…: the window that rebinds the joystick's five inputs to keys and pad inputs
//! (DESIGN §2.6), remembered in `settings.toml` (DESIGN §2.9). The model and the AppKit view are the
//! toolkit's (`amiga_app::controls_window`, moved there by forest-dumb-level-1 T12); this is the
//! app's glue: opening the window, routing its keys and pads, saving, and the log a test follows.
//! A script reaches it as the clicks do (`script.rs`: `controls fire keys`, `controls defaults`,
//! `controls close`).

use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;

pub use amiga_app::controls_window::*;

use crate::App;
use crate::input::HostInput;
use crate::pad::{PadEvent, PadState};

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
                // What the pads already hold is not a new press: the window starts from it.
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

