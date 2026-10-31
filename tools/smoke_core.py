"""The pure core of smoke (smoke-tests DESIGN §2.2, §2.3, §3.1, §3.2).

Parses a project's smoke.toml, reports its problems against the project's test files and turns it into
commands. Files, clock, processes and environment belong to tools/smoke.py: everything here takes its
inputs as arguments, and test_smoke_core.py scans the imports to keep it so.

[tests] keys are relative to the manifest's `workspace` folder and [python] keys to the project folder,
as the owner decided on 2026-10-31 (DESIGN §2.2's example over its sentence). Project.files and
Project.sources stay relative to the project folder.

A crate's package name is taken to be its folder name: Project carries no Cargo.toml text, and this holds
for every crate in the repo today.
"""
import re
import tomllib
from dataclasses import dataclass

_RUST_TEST = re.compile(r'crates/([^/]+)/tests/(?:([^/]+)\.rs|([^/]+)/main\.rs)$')
_RUST_SRC = re.compile(r'crates/([^/]+)/src/(.+\.rs)$')
_PYTHON_TEST = re.compile(r'(?:(.*)/)?(test_[^/]*\.py)$')


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


# --- Reading Rust source ---------------------------------------------------------------------------

_LEXEME = re.compile(r'''
    //[^\n]*                                # a line comment
  | /\*                                     # a block comment: nests, so it is walked by hand
  | (?<!\w)b?r(?P<hashes>\#*)"             # a raw string: closed by its own run of hashes
  | "(?:\\.|[^"\\])*"                       # a string
  | '(?:\\.[^']*|[^\\'])'                   # a char; a lifetime has no closing quote and does not match
''', re.VERBOSE | re.DOTALL)


def _mask(text: str) -> str:
    """Blank comments and the contents of strings and chars, so that a `#[test]` or a brace in a fixture
    string (the purity tests hold plenty) reads as nothing."""
    out = []
    pos = 0
    while m := _LEXEME.search(text, pos):
        out.append(text[pos:m.start()])
        lexeme = m.group()
        pos = m.end()
        if lexeme == '/*':
            depth = 1
            while depth and pos < len(text):
                if text.startswith('/*', pos):
                    depth, pos = depth + 1, pos + 2
                elif text.startswith('*/', pos):
                    depth, pos = depth - 1, pos + 2
                else:
                    pos += 1
            out.append(' ')
        elif m.group('hashes') is not None:
            close = text.find('"' + m.group('hashes'), pos)
            pos = len(text) if close < 0 else close + 1 + len(m.group('hashes'))
            out.append('""')
        elif lexeme.startswith('//'):
            out.append(' ')
        elif lexeme.startswith('"'):
            out.append('""')
        else:
            out.append("' '")
    out.append(text[pos:])
    return ''.join(out)


_CLOSER = {'(': ')', '[': ']', '{': '}'}


def _balanced_end(text: str, start: int) -> int:
    """The index just past the bracket matching the one at `start`, in masked text."""
    opener, closer = text[start], _CLOSER[text[start]]
    depth = 0
    for i in range(start, len(text)):
        if text[i] == opener:
            depth += 1
        elif text[i] == closer:
            depth -= 1
            if depth == 0:
                return i + 1
    return len(text)


def _is_test_attr(attr: str) -> bool:
    return re.fullmatch(r'#\s*\[\s*test\s*\]', attr) is not None


def _has_ignore(text: str) -> bool:
    return re.search(r'#\s*\[\s*ignore\b', text) is not None


@dataclass(frozen=True)
class _Macro:
    name_repeats: bool     # one test per item, as code_cases!, rather than one per invocation, as case!
    ignored: bool          # the macro's own body says #[ignore


_MACRO_DEF = re.compile(r'\bmacro_rules!\s*(\w+)\s*(?=[({\[])')


def _test_macro(body: str) -> _Macro | None:
    """What a macro definition makes, if it makes a test: `#[test]`, then `fn $name`."""
    if not re.search(r'#\s*\[\s*test\s*\]', body):
        return None
    fn = re.search(r'\bfn\s+\$(\w+)', body)
    if not fn:
        return None
    name = re.search(r'\$' + fn.group(1) + r'\s*:\s*ident\b', body)
    if not name:
        return None
    repeats = False
    for group in re.finditer(r'\$\(', body):
        if group.start() < name.start() < _balanced_end(body, group.end() - 1):
            repeats = True
    return _Macro(repeats, _has_ignore(body))


def _strip_macro_defs(text: str) -> tuple[str, dict[str, _Macro]]:
    """Remove every macro_rules! so its body is not read as code, keeping those that make tests."""
    macros = {}
    out = []
    pos = 0
    while m := _MACRO_DEF.search(text, pos):
        end = _balanced_end(text, m.end())
        macro = _test_macro(text[m.end():end])
        if macro:
            macros[m.group(1)] = macro
        out.append(text[pos:m.start()])
        pos = end
    out.append(text[pos:])
    return ''.join(out), macros


def _split_items(args: str) -> list[str]:
    """Split a macro's arguments at its top-level commas and semicolons."""
    items = []
    start = i = 0
    while i < len(args):
        if args[i] in _CLOSER:
            i = _balanced_end(args, i)
            continue
        if args[i] in ',;':
            items.append(args[start:i])
            start = i + 1
        i += 1
    items.append(args[start:])
    return items


_LEADING_ATTRS = re.compile(r'\s*#\s*\[')
_NEXT_ATTR = re.compile(r'\s*(#\s*\[)')
_LEADING_NAME = re.compile(r'\s*([A-Za-z_]\w*)')
_TEST_FN = re.compile(r'\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)')


def _item_name(item: str) -> str | None:
    """The identifier an item starts with, past any attributes."""
    pos = 0
    while m := _LEADING_ATTRS.match(item, pos):
        pos = _balanced_end(item, m.end() - 1)
    m = _LEADING_NAME.match(item, pos)
    return m.group(1) if m else None


def _invocation_tests(args: str, macro: _Macro) -> list[tuple[str, bool]]:
    items = _split_items(args)
    if not macro.name_repeats:
        # The name is the macro's first argument, and an #[ignore anywhere in the call holds for it.
        name = _item_name(items[0])
        return [(name, macro.ignored or _has_ignore(args))] if name else []
    return [(name, macro.ignored or _has_ignore(item))
            for item in items if (name := _item_name(item))]


def _tests_in(text: str) -> dict[str, bool]:
    """Every test in Rust source: its path within the file → whether it is #[ignore]d."""
    code, macros = _strip_macro_defs(_mask(text))
    alternatives = [r'\bmod\s+(?P<mod>\w+)\s*\{', r'\{', r'\}', r'#\s*\[']
    if macros:
        names = '|'.join(re.escape(n) for n in macros)
        alternatives.append(r'\b(?P<macro>' + names + r')!\s*[({\[]')
    token = re.compile('|'.join(alternatives))
    tests = {}
    mods: list[tuple[str, int]] = []
    depth = 0
    pos = 0

    def prefix() -> str:
        return ''.join(name + '::' for name, _ in mods)

    while m := token.search(code, pos):
        pos = m.end()
        lexeme = m.group()
        if m.group('mod'):
            depth += 1
            mods.append((m.group('mod'), depth))
        elif lexeme == '{':
            depth += 1
        elif lexeme == '}':
            if mods and mods[-1][1] == depth:
                mods.pop()
            depth -= 1
        elif macros and m.group('macro'):
            end = _balanced_end(code, pos - 1)
            for name, ignored in _invocation_tests(code[pos:end - 1], macros[m.group('macro')]):
                tests[prefix() + name] = ignored
            pos = end
        else:
            # An attribute block, in either order: #[ignore] may stand before #[test] or after it.
            attrs = []
            start = m.start()
            while True:
                end = _balanced_end(code, code.index('[', start))
                attrs.append(code[start:end])
                next_attr = _NEXT_ATTR.match(code, end)
                if not next_attr:
                    break
                start = next_attr.start(1)
            pos = end
            if any(_is_test_attr(a) for a in attrs):
                fn = _TEST_FN.match(code, pos)
                if fn:
                    tests[prefix() + fn.group(1)] = any(_has_ignore(a) for a in attrs)
                    pos = fn.end()
    return tests


def _src_module(path_in_src: str) -> str:
    """The module path a file under src/ gives its tests, with a trailing `::` unless it is the crate root."""
    if path_in_src.startswith('bin/'):
        return ''
    parts = path_in_src[:-len('.rs')].split('/')
    if parts[-1] in ('lib', 'main', 'mod'):
        parts.pop()
    return ''.join(p + '::' for p in parts)


# --- What the project holds ------------------------------------------------------------------------

@dataclass(frozen=True)
class _Target:
    crate: str
    test: str | None       # the --test name; None for a crate's inline tests
    flags: tuple[str, ...]     # --lib and --bins, for inline tests
    tests: dict[str, bool]     # test path → ignored


def _workspace_prefix(workspace: str) -> str:
    return '' if workspace in ('.', '') else workspace.rstrip('/') + '/'


def _rust_targets(project: Project, workspace: str) -> dict[str, _Target]:
    """Every label key the workspace needs, whatever the manifest says: test files, and src of crates
    that have inline tests."""
    prefix = _workspace_prefix(workspace)
    targets = {}
    inline: dict[str, dict[str, bool]] = {}
    has_lib: set[str] = set()
    has_bin: set[str] = set()
    for path in project.files:
        if not path.startswith(prefix):
            continue
        key = path[len(prefix):]
        if m := _RUST_TEST.fullmatch(key):
            targets[key] = _Target(m.group(1), m.group(2) or m.group(3), (), _tests_in(project.sources.get(path, '')))
        elif m := _RUST_SRC.fullmatch(key):
            crate, in_src = m.group(1), m.group(2)
            if in_src == 'lib.rs':
                has_lib.add(crate)
            if in_src == 'main.rs' or in_src.startswith('bin/'):
                has_bin.add(crate)
            module = _src_module(in_src)
            found = _tests_in(project.sources.get(path, ''))
            inline.setdefault(crate, {}).update({module + n: ig for n, ig in found.items()})
    for crate, tests in inline.items():
        if tests:
            flags = (('--lib',) if crate in has_lib else ()) + (('--bins',) if crate in has_bin else ())
            targets[f'crates/{crate}/src'] = _Target(crate, None, flags, tests)
    return targets


def _python_files(project: Project) -> list[str]:
    return [path for path in project.files if _PYTHON_TEST.fullmatch(path)]


def _existing_crate_sources(project: Project, workspace: str) -> set[str]:
    prefix = _workspace_prefix(workspace)
    crates = set()
    for path in project.files:
        if path.startswith(prefix) and (m := _RUST_SRC.fullmatch(path[len(prefix):])):
            crates.add(f'crates/{m.group(1)}/src')
    return crates


# --- The check -------------------------------------------------------------------------------------

def _parse(project: Project) -> tuple[dict | None, list[str]]:
    where = f'{project.root}/smoke.toml'
    if project.manifest_text is None:
        return None, [f'{where}: missing; every project has one, even with no tests in it']
    try:
        return tomllib.loads(project.manifest_text), []
    except tomllib.TOMLDecodeError as e:
        return None, [f'{where}: not valid TOML: {e}']


def _table(manifest: dict, name: str, where: str, problems: list[str]) -> dict:
    table = manifest.get(name, {})
    if not isinstance(table, dict):
        problems.append(f'{where}: [{name}] must be a table')
        return {}
    return table


def _areas(manifest: dict, where: str, problems: list[str]) -> list[dict]:
    areas = manifest.get('area', [])
    if not isinstance(areas, list) or not all(isinstance(a, dict) for a in areas):
        problems.append(f'{where}: area must be written as [[area]] tables')
        return []
    return areas


def _check_rust_label(key: str, label, target: _Target, where: str) -> list[str]:
    if label in ('smoke', 'full'):
        return []
    if not isinstance(label, list) or not label or not all(isinstance(n, str) for n in label):
        return [f'{where}: {key}: a label is "smoke", "full" or a non-empty list of test names']
    problems = []
    for name in label:
        if name not in target.tests:
            problems.append(f'{where}: {key}: names test {name}, which is not in that file')
        elif target.tests[name]:
            problems.append(f'{where}: {key}: names test {name}, which is #[ignore]d, so smoke would skip it')
    return problems


def _check_cover(area: str, cover: str, rust: dict, python: dict, targets: dict[str, _Target],
                 where: str) -> list[str]:
    head = f'{where}: area "{area}": cover {cover}'
    key, _, name = cover.partition('::')
    if key in python:
        if name:
            return [f'{head}: a Python file is covered whole, not by test name']
        return [] if python[key] == 'smoke' else [f'{head}: not labelled smoke']
    if key not in rust:
        return [f'{head}: not labelled smoke (it has no label)']
    label = rust[key]
    if label == 'full':
        return [f'{head}: not labelled smoke (it is full)']
    target = targets.get(key)
    if target is None:
        return []  # the label itself is already reported as naming nothing
    if isinstance(label, list):
        return [] if not name or name in label else [f"{head}: not in the label's list of tests"]
    if label != 'smoke':
        return []  # the bad label is already reported
    if not name:
        if any(not ignored for ignored in target.tests.values()):
            return []
        return [f'{head}: has no live test, every one in it is #[ignore]d']
    if name not in target.tests:
        return [f'{head}: no such test in that file']
    return [f'{head}: that test is #[ignore]d, so smoke skips it'] if target.tests[name] else []


def check(project: Project) -> list[str]:
    """Problems, each naming the file and the rule; [] is clean."""
    manifest, problems = _parse(project)
    if manifest is None:
        return problems
    where = f'{project.root}/smoke.toml'
    workspace = manifest.get('workspace')
    if not isinstance(workspace, str):
        problems.append(f'{where}: workspace must name the cargo workspace folder ("." for this one)')
        workspace = '.'
    rust = _table(manifest, 'tests', where, problems)
    python = _table(manifest, 'python', where, problems)
    areas = _areas(manifest, where, problems)
    targets = _rust_targets(project, workspace)
    crate_sources = _existing_crate_sources(project, workspace)
    python_files = _python_files(project)

    for key in targets:
        if key not in rust:
            what = 'crate with inline tests' if key.endswith('/src') else 'test file'
            problems.append(f'{where}: {key}: {what} with no label')
    for key, label in rust.items():
        if key in targets:
            problems.extend(_check_rust_label(key, label, targets[key], where))
        elif key in crate_sources:
            problems.append(f'{where}: {key}: labelled, but that crate has no inline tests')
        else:
            problems.append(f'{where}: {key}: labelled, but no such test file or crate')

    for path in python_files:
        if path not in python:
            problems.append(f'{where}: {path}: Python test file with no label')
    for key, label in python.items():
        if key not in python_files:
            problems.append(f'{where}: {key}: labelled, but no such Python test file')
        if label not in ('smoke', 'full'):
            problems.append(f'{where}: {key}: a Python label is "smoke" or "full"')

    if (targets or python_files) and not areas:
        problems.append(f'{where}: the project has test files and no [[area]]')
    for i, area in enumerate(areas):
        name = area.get('name', f'#{i + 1}')
        covers = area.get('covers')
        if not isinstance(covers, list) or not covers or not all(isinstance(c, str) for c in covers):
            problems.append(f'{where}: area "{name}": covers must list at least one test')
            continue
        for cover in covers:
            problems.extend(_check_cover(name, cover, rust, python, targets, where))
    return problems


# --- The run list ----------------------------------------------------------------------------------

def run_list(project: Project) -> list[Entry]:
    """The smoke entries, Rust then Python, in manifest order. Only called when check() is clean."""
    manifest = tomllib.loads(project.manifest_text)
    workspace = manifest['workspace']
    targets = _rust_targets(project, workspace)
    cwd = (project.root + '/' + _workspace_prefix(workspace)).rstrip('/')
    entries = []
    for key, label in manifest.get('tests', {}).items():
        if label == 'full':
            continue
        target = targets[key]
        argv = ['cargo', 'test', '-q', '-p', target.crate]
        argv += ['--test', target.test] if target.test else list(target.flags)
        if isinstance(label, list):
            argv += ['--', '--exact', *label]
        entries.append(Entry(project.root, 'rust', cwd, argv))
    for path, label in manifest.get('python', {}).items():
        if label == 'smoke':
            entries.append(_python_entry(project.root, path))
    return entries


def _python_entry(root: str, path: str) -> Entry:
    # The working directories of today's test.sh scripts, because tests may rely on them (DESIGN §3.2):
    # the toolkit's from the repo root, a game's from its own folder.
    m = _PYTHON_TEST.fullmatch(path)
    folder, name = m.group(1) or '.', m.group(2)
    if root.startswith('games/'):
        cwd, python, start = root, '../../venv/bin/python', folder
    else:
        cwd, python, start = '.', 'venv/bin/python', f'{root}/{folder}' if folder != '.' else root
    return Entry(root, 'python', cwd, [python, '-m', 'unittest', 'discover', '-q', '-s', start, '-p', name])
