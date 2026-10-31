# T01 — label-check

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

The pure core of smoke: parse a project's `smoke.toml`, report every problem with it against the repo's test
files, and turn it into a list of commands. This is what makes a forgotten test or a dead crucial area a
failure rather than a silence.

## Design sections this implements

DESIGN §2.2, §2.3, §3.1, §3.2.

## Files

- `tools/smoke_core.py` (new)
- `tools/test_smoke_core.py` (new)

## Interface

```python
@dataclass(frozen=True)
class Project:
    root: str              # "toolkit" or "games/<game>"
    manifest_text: str | None     # None: no smoke.toml
    files: list[str]       # tracked paths relative to root
    sources: dict[str, str]       # relative path of each *.rs test file or src/ file → its text

@dataclass(frozen=True)
class Entry:
    project: str
    kind: str              # "rust" | "python"
    cwd: str               # relative to the repo root
    argv: list[str]

def check(project: Project) -> list[str]            # problems, each naming the file and the rule; [] is clean
def run_list(project: Project) -> list[Entry]       # the smoke entries; only called when check() is clean
```

A test is live when its `#[test]` attribute block has no `#[ignore`. A crate has inline tests when any file
under its `src/` has `#[test]`, and is labelled by `"crates/x/src"`. `tests/common/` and other helper folders
are not test files; `tests/*.rs` and `tests/*/main.rs` are. The module imports only `re`, `tomllib` and
`dataclasses`.

## Tests

- [ ] a clean manifest gives no problems and the expected argv for whole-file, named and inline entries
- [ ] named tests produce `-- --exact a b`
- [ ] no manifest, with test files present, is a problem; no manifest and no test files is not
- [ ] an unlabelled test file; an unlabelled crate with inline tests; a label for a missing file
- [ ] a helper under `tests/common/` needs no label; `tests/x/main.rs` does
- [ ] a named test absent from its file; a named test that is `#[ignore]`d, with the attribute before and after `#[test]`
- [ ] a project with test files and no area; an area with empty `covers`
- [ ] a cover labelled full; a cover naming a test not in the label's list; a whole-file cover with only ignored tests
- [ ] a malformed manifest reports a problem and does not raise
- [ ] a Python test file without a label
- [ ] the import scan: `smoke_core.py` imports nothing else

## Done when

- [ ] Every §2.3 rule has a failing case in `tools/test_smoke_core.py` and the file passes.
- [ ] `smoke_core.py` touches no file, clock, process or environment, enforced by the scan.
