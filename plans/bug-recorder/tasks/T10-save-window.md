# T10 — save-window

**Phase:** 2 · **Depends on:** T06 · **Weight:** medium

## Goal

The save window after STOP (screenshot, location, the zip's name, a note, Discard and Save) and the
"Recording in progress" question before a menu action that would end a recording, as the native
`Ui` behind T06's rules.

## Design sections this implements

DESIGN §2.2 (ending rule), §2.4, §2.11 (write failure keeps the window open).

## Files

- `engine/crates/sf-app/src/save_window.rs` (new): model plus view; an editable text field, so the window takes first responder (the Controls window never needed this)
- `engine/crates/sf-app/src/import.rs`: `NativeUi::save_recording`, `NativeUi::recording_in_progress` (an alert with Save…, Discard, Cancel)
- `engine/crates/sf-e2e/tests/recorder_save_window.rs` (new)

## Interface

`Ui::save_recording` and `Ui::recording_in_progress` as T06 defines them; the model:

```rust
pub struct SaveForm { pub note: String, pub name: String, pub location: Location }
pub enum SaveAction { Note(String), Save, Discard }
```

A note's newline is not accepted by the field. While either window is open the game is paused and
the game's keys go to the window, not the game.

## End to end (the worker drives this)

- suite: `sf-e2e` OS layer and script layer · sizes: 1×, 3×, fullscreen
- [ ] OS: STOP opens the window with the screenshot, `W1-1:c,r`, the name; typing a note and Save writes the zip with that note; a notice shows the path; the clipboard holds it
- [ ] OS: Discard writes nothing and the game resumes not recording
- [ ] OS: Load State 2 while recording shows the question; Cancel resumes recording; Discard loads; Save… opens the save window, then loads after Save
- [ ] OS: Quit (⌘Q and the window's close button) while recording asks the same
- [ ] OS: `--bugs-dir` read-only: Save shows the error and the window stays open with the note
- [ ] OS: typing in the note never moves the frog (the recording's inputs end at STOP's frame)

## Done when

- [ ] both windows work as above and the end-to-end list passes (OS layer recorded in PROGRESS)
- [ ] the test command is green
