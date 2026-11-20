# The owner's controls

Moved from Claude's memory 2026-10-30; checked against the owner's settings.toml and the hand-over
script the same day. Still true.

The owner plays with **Up (jump) on Space and Fire on Left Option** (`up = ["Space"]`,
`fire = ["AltLeft"]`; rebound in the app's Controls window on 2026-10-27, during a widescreen
hand-over). The game's `tools/handover.py` reads the mapping from the owner's settings.toml
(`~/Library/Application Support/Superfrog-RE/settings.toml`), so hand-over scripts follow it; never
hard-code arrow keys or Space-as-fire.

**Why:** a script pressing keys the owner has bound differently scrambles the route the owner takes
over.

**How to apply:** when generating any app script for the owner, regenerate it after the owner changes
controls.
