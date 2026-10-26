# Chapter template

The section layout every chapter follows (plans/engine-architecture DESIGN §2.3). Use the
[glossary](glossary.md)'s terms for the mechanisms. Cite code by repo path and item name
(`engine/crates/sf-core/src/state.rs`, `State::probe`); link to the RE notes in
[docs/08](../08-executable-map.md)–[docs/13](../13-front-end.md) and to
[the engine's design](../../plans/rust-engine/DESIGN.md) rather than restating them. Describe the
code as it is today. Where a claim cannot be settled by reading, say what is unknown and how to
find out.

A chapter opens with its front matter (DESIGN §2.4) between `+++` lines: `id`, `title`, `tier`,
`depth`, `kind`, `mix`, `reuse` and `owns`. Leave `owns` as the plan assigned it; a file that
belongs elsewhere goes in the plan's `FINDINGS.md`.

## Sections of a deep chapter

1. **What it is.** What this part of the engine does, in a paragraph, and the files it owns.
2. **Kind mix.** The `mix` percentages, which parts of the code are which kind, and one sentence
   on how the estimate was judged.
3. **Mechanisms it uses.** The glossary mechanisms this part leans on, and where. A mechanism
   chapter says instead where else in the engine its mechanism is used.
4. **What depends on it.** The other chapters and checks that would break if it changed.
5. **What a native rewrite must preserve.** For each Amiga-shaped part: the behaviour the
   original shows that a rewrite must keep, and the existing check that proves it (an
   `sf-conform` case, a crate test, an `sf-e2e` test), by name. Faithful stays the bar: name what
   to keep, never what may be dropped.
6. **Native sketch (non-binding).** A shape the rewrite could take, labelled non-binding:
   designing the rewrite is a later plan's job.
7. **Reuse.** The rating, what would change to reuse it in another Amiga game, and the size in
   lines from `python tools/architecture.py totals`. Where another Team 17 game might share more,
   say so and mark it unchecked.
8. **Sources.** The RE notes, design sections and code this chapter was written from.

## A light chapter

A light chapter (`depth = "light"`) keeps sections 2 (kind mix) and 7 (reuse), and in place of
3–6 one section, **Where it touches the Amiga-shaped parts**: which mechanisms it drives,
imitates or depends on. It may drop the rest.
