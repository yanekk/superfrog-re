# T08 — switch-and-directives

**Phase:** 3 · **Depends on:** T04, T07 · **Weight:** light

## Goal

Make smoke the repo's test line for new plans, and tell every future agent to keep it growing. Without the
directives, smoke stays frozen as the games grow, which the owner named as a requirement.

## Design sections this implements

DESIGN §2.7.

## Files

- `.pir/settings.json`: `"test": ["tools/smoke.sh"]`
- `CLAUDE.md`: Conventions gets the directive (the part above the pir method, which is the repo's own)
- `toolkit/CLAUDE.md`, `games/superfrog/CLAUDE.md`, `games/forest-dumb/CLAUDE.md`, `toolkit/template/CLAUDE.md`: one line pointing at the project's `smoke.toml` and the rule
- `toolkit/template/smoke.toml` (new): a stub with its format in a comment
- `toolkit/PLAYBOOK.md`: `Starting a game` adds `games/<game>/smoke.toml`
- `README.md`: the `tools/` row and how to run each command

## Interface

The directive, in substance (DESIGN §2.7):

```
Tests: smoke and full. pir runs tools/smoke.sh (15 minutes at most) at the end of every plan; a nightly
schedule runs tools/full.sh; a session runs neither unless its task says so.
- Every new test file gets a label in its project's smoke.toml. A file that finishes alone in 10 s or less is smoke.
- A change adding something crucial adds its test to smoke and an [[area]] for it. A plan adding a crucial
  feature puts this in that task's Done when.
- Before adding a slow test, read ~/.cache/superfrog-re/smoke-times.tsv. If smoke would pass 15 minutes, move
  fast tests to full first. If crucial tests alone do not fit, ask the owner.
```

No game names under `toolkit/`. The root and tier files stay short: rule plus pointer.

## Tests

- [ ] `venv/bin/python tools/check_tier_names.py` passes
- [ ] `tools/smoke.sh --check-only` passes, the template's stub not counted as a project
- [ ] `tools/test_select.py` still passes after the docs change

## Done when

- [ ] `.pir/settings.json` names `tools/smoke.sh`; the owner's own override file is untouched.
- [ ] A reader of root `CLAUDE.md` alone learns both commands and the labelling rule; the README says who runs each.
