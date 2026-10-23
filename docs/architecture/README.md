# Engine architecture

How the Rust engine in `engine/` is built, chapter by chapter, separating the parts that imitate
the Amiga from Superfrog's own rules and from data copied in from the original, and rating each
part for reuse in another Amiga game. It is the input to a later plan that rewrites the
Amiga-shaped parts as plain native code while the game stays faithful
(`plans/engine-architecture/DESIGN.md`).

Stub: the overview, with the totals and the recommended rewrite order, is written by T18.

- [Glossary](glossary.md): the mechanisms and terms every chapter uses.
- [Chapter template](chapter-template.md): the sections each chapter has.

## Chapters

Mechanisms: [memory model](memory-model.md), [copied-in data](copied-in-data.md),
[CPU time and the beam](cpu-time-and-beam.md),
[the frame, interrupts and input](frame-interrupts-input.md),
[the display hardware](display-hardware.md), [the sound chip](sound-chip.md).

Subsystems: [the sound driver](sound-driver.md), [game flow](game-flow.md),
[the front end](front-end.md), [the story and the ending](story-and-ending.md),
[the level](level.md), [the frog and the items](frog-and-items.md),
[enemies and bosses](enemies-and-bosses.md), [the slot machine and the bonus level](slot-and-bonus.md).

Components: [disks and assets](disks-and-assets.md), [the app](app.md),
[verification](verification.md).

Each chapter's front matter owns its files: `tools/test_architecture_docs.py` fails when a tracked
file under `engine/crates/` or `engine/conform/` belongs to no chapter.
