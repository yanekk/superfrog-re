# T01 — smoke-room

**Phase:** 0 · **Depends on:** — · **Weight:** light

## Goal
Make room in the smoke set for this plan's tests, because it runs at up to 887 of 900 seconds, and a plan whose end gate times out cannot finish.
## Design sections this implements
§4, §5; root CLAUDE.md Conventions.
## Reuses
`tools/smoke.py`, `~/.cache/superfrog-re/smoke-times.tsv`, each project's `smoke.toml`.
## Files
`toolkit/smoke.toml`, `games/superfrog/smoke.toml`, `games/forest-dumb/smoke.toml`; `tools/` only if the time report needs it.
## Interface
No code interface. Fast non-crucial tests move to `full`; each `[[area]]`'s covering test stays smoke.
## Tests
- [ ] `tools/smoke.sh` labels check passes.
- [ ] Every `[[area]]` still resolves to a smoke test.
## Done when
- [ ] A measured smoke run is at least 180 s under its limit, named in the commit with before and after.
- [ ] No crucial area lost coverage; `tools/full.sh` still includes every moved test.
- [ ] If this is impossible with crucial tests alone, the owner was asked.
