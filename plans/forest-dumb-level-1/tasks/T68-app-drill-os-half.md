# T68 — app-drill-os-half

**Phase:** 3 · **Depends on:** T16 · **Weight:** light

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T68` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again. A reviewer's pack has no FINDINGS: read it after your cold read.

Added by T16 with the owner's yes, given by the coordinator agent on the owner's behalf (2026-11-06): the screen was locked overnight and nobody could unlock it. T16 drove the script-layer half of the app drill (`fd-e2e/tests/drill.rs`, `drill_sizes.rs`). The real-window half cannot run on a locked screen: the window never becomes key, so focus is never lost, and the Accessibility API does not see the buttons in the app's windows (`no element "up-keys"`). Keys posted to the app, menu clicks and window captures did work locked. T28 waits for it (`blocks T28`).

## Goal
With the owner unlocking the screen first, drive the app's real windows through the OS layer (`FD_E2E_OS=1`): the Controls window's buttons, Start Level's, the save window with a long note typed and pasted, and focus lost to another app with a key held. Judge each against DESIGN §2.7, §2.8 as T16 did, and fix what has one right answer.

## Design sections this implements
§2.7, §2.8.

## Reuses
`amiga-e2e`'s OS layer (T16 added `Key::Tab` and `Key::OptionLeft`), `sf-e2e/tests/recorder_drill.rs`'s OS-layer tests and its `screen_locked` refusal (`sf-e2e/tests/menus.rs`, T57), T16's `drill.rs`.

## Handover: the first step
Ask the owner, in the session, before anything else:

```
Needs you — I cannot see this from here:

  Unlock the Mac (your login), and leave it unlocked for about 15 minutes.

Expect: nothing to do or watch. App windows will open and close by themselves at the screen's
centre, about 320×256 and 640×512 in size, each for a few seconds, then quit.
Tell me: "unlocked" when it is, or "can't now" if it has to wait.
```

## Files
`fd-e2e/tests/drill_os.rs` (new), fixes where they fall, `smoke.toml` (a label), `FINDINGS.md`.

## End to end (the worker drives this)
- suite: `fd-e2e`, OS layer, `FD_E2E_OS=1 cargo test --release -p fd-e2e --test drill_os -- --ignored --test-threads 1`. Release: a debug build writing every frame's capture keeps the main thread too busy to answer the Accessibility API (`AXError -25204`, measured by T16).
- Each test refuses a locked screen first, as `sf-e2e/tests/menus.rs`'s `os_layer()` does.
- The app menu is titled with the process name, `fd-app` (an unbundled binary): Quit is `["fd-app", "Quit Forest Dumb"]`.
- [ ] Controls: every button's title (Up shows Space, Fire Left Option), rebinding Up to W by a real key, Esc cancelling then closing, Restore Defaults, the window closed by its close button; the game window has the keyboard again after.
- [ ] Start Level: the level dropdown offers W1-1 only, Esc cancels, Return starts recording, the recorder unticked starts without one.
- [ ] Recording in progress (⌘Q, Esc, Start Level while recording): Cancel, Discard, Save….
- [ ] The save window: Esc and ⌘. keep it, a note of 2 000 characters pasted with line breaks is one line, saved whole; Return saves; the notice's OK.
- [ ] Focus: hold Left, switch to another app (`open -a Finder`), the game lets go of Left and play goes on; back, nothing is held.
- [ ] Fire on Left Option and Up on Space by real keys; Tab reaches the game as the Amiga's Tab.
- [ ] Every window captured to the scratch folder at scale 1 and the default scale, and judged.

## Done when
- [ ] A dated worker-driven row in FINDINGS says what was driven and seen.
- [ ] Each decidable defect has a test failing without its fix; two-answer questions were asked, not decided.
