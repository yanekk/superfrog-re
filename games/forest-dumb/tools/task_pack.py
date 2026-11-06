#!/usr/bin/env python3
"""A task's pack: everything a session of plan forest-dumb-level-1 would otherwise read in its first
dozens of steps, in one file it reads whole in one step (owner, 2026-11-06; DESIGN §4.4).

    python3 -I games/forest-dumb/tools/task_pack.py T22 [--review] [--no-prep] [--max-kb 150]

Writes games/forest-dumb/build/pack/T22.md (T22-review.md with --review) and prints one JSON line:

    {"pack": "<path>", "bytes": n, "tokens": n, "kinds": [...], "sections": [...], "omitted": [...], "prep": [...]}

Why: a session pays for its whole context on every step, so what it reads early is paid for again on
every later step. Read in sips of a few dozen lines, T08's opening reads were 39% of its cost and their
weight on later steps another 26% (run of 2026-11-06). One read of a pack built by a free script
replaces them.

What a pack holds, in this order (a section that would take the pack past --max-kb is left out and named
in `omitted`, with the path to read instead):
- how to use it; the task doc; PROGRESS.md; FINDINGS.md (not with --review: a reviewer reads it after
  its cold read); the DESIGN sections the task names, with §4 and §5 always;
- the work of the tasks it depends on: their commits and the files they touched;
- the files the task doc's `## Files` names, Rust as skeletons (types whole, fns as signatures);
- a porting task's routines from re/translate.toml and re/objects-W1-1.toml, each with its listing
  from build/translate (here or in the main checkout) and where its literal port is, the porting rules,
  and how to have Muse write the engine code (re/reshape.toml, tools/reshape.py);
- a frame-matching task's runner notes (engine/conform/README.md) and its cases;
- the engine's map: each source file with its line count and public items.
With --review: the task branch's commits and diffstat against the plan branch.

Prep, unless --no-prep: when this checkout has no program listing, extract the program
(`cargo run -q -p fd-tools -- extract`, local, free) and write it (`tools/re_listing.py`), so the
session never spends steps finding out how. The listing and the pack derive from the game's code: both
stay in build/ (git-ignored), never committed.

Python's standard library only (reshape.py's skeleton reader), so it runs under `python3 -I`.
"""
import argparse, glob, json, os, re, subprocess, sys, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import reshape  # noqa: E402  skeleton(), the main checkout, the reshape table

GAME = os.path.dirname(HERE)
REPO = os.path.dirname(os.path.dirname(GAME))
ENGINE = os.path.join(GAME, 'engine')
CRATES = os.path.join(ENGINE, 'crates')
PLAN = 'forest-dumb-level-1'
PLAN_DIR = os.path.join(REPO, 'plans', PLAN)
BASE = 'main'  # .pir/settings.json's baseBranch
OUT = os.path.join(GAME, 'build', 'pack')
LISTING = os.path.join(GAME, 'build', 're', 'forest_dumb_forever.lst')
PROGRAM = os.path.join(GAME, 'build', 'files', 'disk1', 'FOREST_DUMB_FOREVER')
MAX_KB = 150
# DESIGN sections every pack carries: the testing rules (porting rules, Muse) and the environment.
ALWAYS = ('4', '5')
CASE_TASKS = {'T11', 'T27', 'T48'}


def read(path):
    with open(path, encoding='utf-8') as f:
        return f.read()


def rel(path):
    """From the repo root; absolute when outside this checkout (the main checkout's build/translate)."""
    r = os.path.relpath(path, REPO)
    return os.path.abspath(path) if r.startswith('..') else r


def git(*args, cwd=REPO):
    r = subprocess.run(['git', *args], cwd=cwd, capture_output=True, text=True, timeout=60)
    return r.stdout if r.returncode == 0 else ''


# --- The task ---------------------------------------------------------------------------------------

def task_doc(task):
    hits = glob.glob(os.path.join(PLAN_DIR, 'tasks', f'{task}-*.md'))
    if len(hits) != 1:
        raise SystemExit(f'task_pack: no single task doc for {task} in {rel(PLAN_DIR)}/tasks')
    return hits[0]


def depends(doc):
    m = re.search(r'\*\*Depends on:\*\*([^·\n]*)', doc)
    return re.findall(r'T\d+', m.group(1)) if m else []


def section(doc, title):
    """The body of the doc's `## title` section."""
    m = re.search(rf'(?ms)^## {re.escape(title)}\s*\n(.*?)(?=^## |\Z)', doc)
    return m.group(1).strip() if m else ''


def design_sections(doc):
    """`§2.1`-style references in the task doc, as section numbers."""
    return sorted(set(re.findall(r'§(\d+(?:\.\d+)?)', doc)), key=lambda s: [int(x) for x in s.split('.')])


def design_text(numbers):
    """DESIGN.md's opening block (setup/test lines) and every `## n.`/`### n.m` section asked for; a
    top-level number brings its subsections."""
    text = read(os.path.join(PLAN_DIR, 'DESIGN.md'))
    heads = list(re.finditer(r'(?m)^(#{2,3}) (\d+(?:\.\d+)?)[. ]', text))
    first = heads[0].start() if heads else len(text)
    out = [text[:first].strip()]
    want = set(numbers)
    for i, h in enumerate(heads):
        num = h.group(2)
        top = num.split('.')[0]
        if num in want or top in want:
            end = heads[i + 1].start() if i + 1 < len(heads) else len(text)
            out.append(text[h.start():end].strip())
    return '\n\n'.join(out)


# --- Files ------------------------------------------------------------------------------------------

def expand(spec):
    """`fd-core/src/objects/{mod.rs,scan.rs}` -> each path."""
    m = re.search(r'\{([^}]*)\}', spec)
    if not m:
        return [spec]
    return [p for alt in m.group(1).split(',') for p in expand(spec[:m.start()] + alt.strip() + spec[m.end():])]


def resolve(spec):
    """A path as a task doc writes it, found under the engine's crates, the game, the toolkit or the
    repo; None when it does not exist yet."""
    for base in (CRATES, GAME, os.path.join(REPO, 'toolkit', 'crates'), os.path.join(REPO, 'toolkit'), REPO):
        p = os.path.join(base, spec)
        if os.path.exists(p):
            return p
    return None


def named_files(doc):
    specs = re.findall(r'`([^`\s]+\.[a-z]+[^`\s]*)`', section(doc, 'Files'))
    return [p for s in specs for p in expand(s)]


def file_view(path, small=6000):
    """A file as the pack shows it: Rust as its skeleton, other text whole when small, else its head."""
    text = read(path)
    if path.endswith('.rs'):
        try:
            return 'rust', reshape.skeleton(text)
        except Exception:  # an odd file still gets a view
            pass
    lang = os.path.splitext(path)[1][1:]
    if len(text) <= small:
        return lang, text
    head = '\n'.join(text.splitlines()[:40])
    return lang, f'{head}\n... ({len(text)} bytes; read the rest from the file)'


def engine_map():
    """Each engine source file: lines and public items, one line a file."""
    lines = []
    for path in sorted(glob.glob(os.path.join(CRATES, '*', 'src', '**', '*.rs'), recursive=True)):
        text = read(path)
        items = re.findall(r'(?m)^\s*pub(?:\([a-z]+\))?\s+(?:const\s+fn|fn|struct|enum|trait|type|mod)\s+(\w+)', text)
        shown = ', '.join(dict.fromkeys(items))
        if len(shown) > 220:
            shown = shown[:220] + ', ...'
        lines.append(f'{os.path.relpath(path, CRATES)} ({text.count(chr(10))}): {shown}')
    tests = sorted(os.path.relpath(p, CRATES) for p in glob.glob(os.path.join(CRATES, '*', 'tests', '*.rs')))
    return '\n'.join(lines) + '\n\nTest files: ' + ', '.join(tests)


# --- Porting ----------------------------------------------------------------------------------------

def port_dirs():
    return reshape.port_dirs()


def routines_for(task):
    rows = tomllib.loads(read(os.path.join(GAME, 're', 'translate.toml'))).get('routine', [])
    return [r for r in rows if r.get('port_task') == task]


def object_values(task):
    p = os.path.join(GAME, 're', 'objects-W1-1.toml')
    if not os.path.exists(p):
        return []
    return [v for v in tomllib.loads(read(p)).get('value', []) if v.get('port_task') == task]


def port_folder(r):
    for d in port_dirs():
        f = os.path.join(d, f"h{r['hunk']}-{r['offset']:x}")
        if os.path.isdir(f):
            return f
    return None


def listing_of(folder):
    """The listing block of a translate.py prompt: the routine's own flow, labels included."""
    p = os.path.join(folder, 'prompt.md')
    if not os.path.exists(p):
        return None
    m = re.search(r'```\n(.*?)```', read(p), re.S)
    return m.group(1) if m else None


def routine_block(r):
    head = (f"### `{r['name']}` h{r['hunk']}+${r['offset']:x}: own {r.get('own')}, with callees "
            f"{r.get('instructions')}, unit {r.get('unit')}")
    notes = [x for x in (r.get('note'), r.get('reason')) if x]
    if r.get('by_hand'):
        notes.insert(0, f"by hand: {r['by_hand']}")
    if r.get('split_at'):
        notes.append('split at ' + ', '.join(r['split_at']))
    parts = [head] + ([' · '.join(notes)] if notes else [])
    f = port_folder(r)
    if not f:
        parts.append('No translation folder: run `tools/translate.py` on it, or port it by hand.')
        return '\n\n'.join(parts)
    res = reshape.tr.read_result(f) or {}
    port = os.path.join(f, 'port.rs')
    parts.append(f"Literal port: {res.get('result', 'none')}"
                 + (f" (`{rel(port)}`: Muse's input, which reshape.py sends; read it only to check one)"
                    if res.get('result') == 'pass' and os.path.exists(port) else f' ({rel(f)})'))
    lst = listing_of(f)
    if lst:
        parts.append('```\n' + lst.rstrip() + '\n```')
    return '\n\n'.join(parts)


def reshape_entry(task):
    text = read(reshape.TASKS_FILE)
    m = re.search(rf'(?ms)^\[{task}\]\n.*?(?=^\[T\d+\]|\Z)', text)
    if m:
        return f'`re/reshape.toml` has `[{task}]`:\n\n```toml\n{m.group(0).strip()}\n```'
    t08 = re.search(r'(?ms)^\[T08\]\n.*?(?=^\[T\d+\]|\Z)', text)
    head = text[:text.index('[T08]')].strip()
    return (f'`re/reshape.toml` has no `[{task}]` yet: add it once the typed state and the rig test exist. '
            f'Its fields:\n\n```toml\n{head}\n\n{t08.group(0).strip() if t08 else ""}\n```')


# --- Prep -------------------------------------------------------------------------------------------

def prep():
    """Make the program listing exist in this checkout. -> what was done, as lines."""
    done = []
    if os.path.exists(LISTING):
        return [f'listing present: {rel(LISTING)}']
    if not os.path.exists(PROGRAM):
        r = subprocess.run(['cargo', 'run', '-q', '-p', 'fd-tools', '--', 'extract'], cwd=ENGINE,
                           capture_output=True, text=True, timeout=1800)
        done.append('extracted the program: `cargo run -q -p fd-tools -- extract` in engine/'
                    + ('' if r.returncode == 0 else f' FAILED: {r.stderr.strip()[-400:]}'))
    py = os.path.join(REPO, 'venv', 'bin', 'python')
    py = py if os.path.exists(py) else sys.executable
    r = subprocess.run([py, os.path.join(HERE, 're_listing.py')], cwd=REPO, capture_output=True, text=True,
                       timeout=600)
    done.append(f'wrote the listing: `{rel(py)} games/forest-dumb/tools/re_listing.py` -> {rel(LISTING)}'
                + ('' if r.returncode == 0 else f' FAILED: {r.stderr.strip()[-400:]}'))
    return done


# --- The pack ---------------------------------------------------------------------------------------

HOW = """# Pack for {task}{mode}

Built by `games/forest-dumb/tools/task_pack.py` (DESIGN §4.4). Every step you take re-sends your whole
context, so this pack replaces the opening reads in one step:

- It holds the task doc, PROGRESS.md{findings}, the DESIGN sections the task names with §4 and §5, the
  work of the tasks it depends on, the files it names (Rust as skeletons), {kind_line}and the engine's
  map. Do not read these again; they are current as of {head}.
- Read further only for what the pack does not hold: a function's body, a test you will change, a
  section listed as omitted at the end. Read whole files in one call, and several files per step.
- Where you would read a listing in pieces, the pack has each routine's listing whole below.
"""


def build(task, review=False, max_kb=MAX_KB, do_prep=True):
    doc_path = task_doc(task)
    doc = read(doc_path)
    slug = os.path.basename(doc_path)[:-3].split('-', 1)[1]
    routines = routines_for(task)
    values = object_values(task)
    kinds = (['porting'] if routines or values else []) + \
            (['frame-matching'] if slug.startswith('case-') or task in CASE_TASKS else [])
    prep_lines = prep() if do_prep and 'porting' in kinds else []
    head = git('log', '-1', '--format=%h %s').strip()
    kind_line = {('porting',): "its routines' listings, how Muse writes the engine code, ",
                 ('frame-matching',): "the conformance runner's notes and cases, "}.get(tuple(kinds), '')
    sections = []  # (name, text, required)

    def add(name, text, required=False):
        if text and text.strip():
            sections.append((name, text.strip(), required))

    add('how', HOW.format(task=task, mode=' (review)' if review else '', head=head, kind_line=kind_line,
                          findings='' if review else ', FINDINGS.md'), True)
    if prep_lines:
        add('prep', '## Prepared in this checkout\n\n' + '\n'.join(f'- {l}' for l in prep_lines), True)
    add('task', f'## The task: {rel(doc_path)}\n\n{doc}', True)
    add('progress', f'## PROGRESS.md\n\n{read(os.path.join(PLAN_DIR, "PROGRESS.md"))}', True)
    if review:
        base = f'pir/{PLAN}'
        add('branch', f'## This branch against {base}\n\n```\n'
            + git('log', '--format=%h %s', f'{base}..HEAD') + '\n'
            + git('diff', '--stat', f'{base}...HEAD') + '```', True)
    else:
        add('findings', f'## FINDINGS.md\n\n{read(os.path.join(PLAN_DIR, "FINDINGS.md"))}', True)
    nums = sorted(set(design_sections(doc)) | set(ALWAYS), key=lambda s: [int(x) for x in s.split('.')])
    add('design', '## DESIGN.md, ' + ', '.join(f'§{n}' for n in nums) + '\n\n' + design_text(nums))
    deps = []
    # Only this plan's commits: other plans number their tasks from T00 too.
    since = git('merge-base', 'HEAD', BASE).strip()
    span = f'{since}..HEAD' if since else 'HEAD'
    for t in depends(doc):
        log = git('log', '--format=%h %s', '-E', f'--grep=^{t}([: ]|$)', span).strip().splitlines()
        if not log:
            continue
        files = git('log', '--name-only', '--format=', '-E', f'--grep=^{t}([: ]|$)', span).split()
        files = [f for f in dict.fromkeys(files) if not f.startswith('plans/')]
        deps.append(f"### {t}\n\n" + '\n'.join(log[:8]) + ('\n...' if len(log) > 8 else '')
                    + '\n\nFiles: ' + (', '.join(f'`{f}`' for f in files[:30]) or 'none outside plans/'))
    add('dependencies', '## What the tasks it depends on did\n\n' + '\n\n'.join(deps) if deps else '')
    views = []
    for spec in named_files(doc):
        p = resolve(spec)
        if not p:
            views.append(f'### `{spec}`: does not exist yet')
            continue
        if os.path.isdir(p):
            continue
        lang, text = file_view(p)
        views.append(f'### `{rel(p)}`\n\n```{lang}\n{text.rstrip()}\n```')
    add('files', '## The files the task names\n\n' + '\n\n'.join(views) if views else '')
    if 'porting' in kinds:
        blocks = [routine_block(r) for r in routines]
        if values:
            vals = '\n'.join(f"- `0x{v['value']:02x}` {v.get('group')}: {v.get('what')} (routines: "
                             f"{', '.join(v.get('routines', []))})" for v in values)
            add('objects', f'## Object values this task ports (re/objects-W1-1.toml)\n\n{vals}', True)
        add('listings', f'## Its routines (re/translate.toml), with their listings\n\n'
            'Each listing is the flow translate.py sent Muse: the routine with the callees it was not split '
            'at, so a callee at a lower address can come first. '
            f'The listing of the whole program is `{rel(LISTING)}`; `re/symbols.toml` names its labels.\n\n'
            + '\n\n'.join(blocks), True)
        add('reshape', '## Muse writes the engine code\n\n' + reshape_how() + '\n\n' + reshape_entry(task), True)
        add('rules', '## toolkit/docs/porting-routines.md\n\n'
            + read(os.path.join(REPO, 'toolkit', 'docs', 'porting-routines.md')))
    if 'frame-matching' in kinds:
        cases = sorted(os.path.relpath(p, GAME) for p in glob.glob(os.path.join(ENGINE, 'conform', 'cases', '*')))
        add('conform', '## engine/conform/README.md\n\n' + read(os.path.join(ENGINE, 'conform', 'README.md'))
            + '\n\nCases: ' + ', '.join(f'`{c}`' for c in cases), True)
    add('map', '## The engine\'s map (file (lines): public items)\n\n```\n' + engine_map() + '\n```')

    # Fit: required sections always; the rest in order while they fit.
    budget = max_kb * 1024
    kept, omitted = [], []
    used = sum(len(t) for _, t, req in sections if req)
    for name, text, req in sections:
        if req:
            kept.append((name, text))
        elif used + len(text) <= budget:
            kept.append((name, text))
            used += len(text)
        else:
            omitted.append(name)
    body = '\n\n'.join(t for _, t in kept)
    if omitted:
        where = {'design': f'plans/{PLAN}/DESIGN.md', 'rules': 'toolkit/docs/porting-routines.md',
                 'ports': 'build/translate/h0-<offset>/port.rs', 'map': 'engine/crates',
                 'files': 'the files under `## Files`', 'dependencies': 'git log --grep',
                 'findings': f'plans/{PLAN}/FINDINGS.md'}
        body += '\n\n## Omitted to fit the pack\n\n' + '\n'.join(f'- {n}: read `{where.get(n, n)}`' for n in omitted)
    os.makedirs(OUT, exist_ok=True)
    out = os.path.join(OUT, f'{task}{"-review" if review else ""}.md')
    with open(out, 'w', encoding='utf-8') as f:
        f.write(body + '\n')
    return {'pack': rel(out), 'bytes': len(body.encode()), 'tokens': len(body) // 4, 'kinds': kinds,
            'sections': [n for n, _ in kept], 'omitted': omitted, 'prep': prep_lines}


def reshape_how():
    """docs/11's section on reshaping, so the pack carries the current procedure."""
    text = read(os.path.join(GAME, 'docs', '11-translation.md'))
    m = re.search(r'(?ms)^## Reshaping by Muse.*?(?=^## |\Z)', text)
    return m.group(0).strip() if m else 'See docs/11-translation.md, "Reshaping by Muse".'


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('task', help='T22')
    ap.add_argument('--review', action='store_true', help="a reviewer's pack: no FINDINGS, the branch's diffstat")
    ap.add_argument('--no-prep', action='store_true', help='do not extract the program or write the listing')
    ap.add_argument('--max-kb', type=int, default=MAX_KB)
    a = ap.parse_args(argv)
    if not re.fullmatch(r'T\d+', a.task):
        ap.error('the task is its number, e.g. T22')
    print(json.dumps(build(a.task, a.review, a.max_kb, not a.no_prep), separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
