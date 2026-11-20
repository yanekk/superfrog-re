#!/usr/bin/env python3
"""The translator's one command: a routine's, or a task's, translation and its judge result
(translator DESIGN §2.12, §2.13; T18). It replaces Muse's translate.py under the same name, and the
trial's two steps (`pcode_export.py`, then `translator_trial.py --skip-export`).

    python3 -I games/forest-dumb/tools/translate.py 'hN+$off' | <symbol> | --task T{nn} [--gen <generator>] [--cases 2000] [--fresh]
    python3 -I games/forest-dumb/tools/translate.py --build

Exports the whole program (`pcode_export.py`) and translates it (`pcode_rust.py`, no hints: a hint is
a hand edit, DESIGN §2.2) into build/translator/tree/, once: the tree is kept while the program's SHA-1
and the translator's own version (a hash of its sources and the export's seed files) are those of
build/translator/translate-stamp.json; `--fresh` redoes both. Then builds fd-judge optimised with the
tree in it (FD_TRANSLATED), finds the routine's generator (`--gen`, else the first `fd-judge
--generators` registers for its address, by name, `hN_off` or a symbol of re/symbols.toml), surveys
the original alone and judges the translated routine on `--cases` states. Writes
build/translate/<hN-off>/translated.rs, the routine's generated function and every function it calls,
routine first, then callees in the order first called (for reading: it is not compiled on its own),
and prints one line a routine:

    {"routine":"h0+$5462","name":"char_sprite_position","result":"pass|fail|no-gen|no-oracle|stop",
     "first":"...","states":2000,"rust":"games/forest-dumb/build/translate/h0-5462/translated.rs"}

`fail`: the translation differs from the original, `first` the first difference. `stop`: on some
state the translation stopped (untranslated, a write into code, an unsupported instruction, a cap),
`first` the first stop. `no-gen`: no generator registered for the routine. `no-oracle`: the original
returned on no state, or drew with a blit the rig does not model. `states` is how many were judged.

`--task T{nn}` takes the routines whose `port_task` or `gen_task` is T{nn} in re/translate.toml, else
the routines its task doc names (`hN+$off`, or a backticked symbol of re/symbols.toml, from its
`## Routines` section if it has one, else the whole doc): the doc in plans/*/tasks/T{nn}-*.md, the
current branch's plan first when several plans have one; `--task <path.md>` names the doc. One line a
routine. `--build` stops after the tree: what fd-translated includes; it prints the translation's
stats line.

Exit 0 when every routine passes (or `--build` built), 1 otherwise or when a step failed (named on
stderr), 2 on usage. Holds judge_run's build/translate/judge.lock from the build to the end, waiting for
it: the fd-judge binary is shared with every tool that builds it.

Test only: FD_TRIAL_EXPORT, FD_TRIAL_BUILD and FD_TRIAL_JUDGE replace the export, the cargo build and
fd-judge, as for translator_trial.py. Standard library only, for `python3 -I`.
"""
import argparse, fcntl, glob, hashlib, json, os, re, subprocess, sys, tomllib
from dataclasses import dataclass

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import judge_run as jr  # noqa: E402
import translator_trial as trial  # noqa: E402

GAME = jr.GAME
REPO = os.path.dirname(os.path.dirname(GAME))
PROGRAM = os.path.join(GAME, 'build', 'files', 'disk1', 'FOREST_DUMB_FOREVER')
# What the tree is made from besides the program: a change to any of these is a new translator.
SOURCES = ('tools/pcode_export.py', 'tools/ghidra/ExportPcode.java', 'tools/ghidra/SetFlowOverride.java',
           'tools/pcode_rust.py', 'tools/pcode_fix/*.py', 're/symbols.toml', 're/entries.toml')
RESULTS = ('pass', 'fail', 'no-gen', 'no-oracle', 'stop')
STOP_KEYS = ('untranslated', 'write_into_code', 'unsupported', 'stopped_hangs')


class Usage(Exception):
    """exit 2"""


@dataclass
class Paths:
    translator: str = os.path.join(GAME, 'build', 'translator')
    out: str = jr.JUDGE_DIR
    program: str = PROGRAM
    inventory: str = jr.INVENTORY
    symbols: str = jr.SYMBOLS
    plans: str = os.path.join(REPO, 'plans')
    game: str = GAME
    # Paths in the result line are shown from here when inside it.
    root: str = REPO

    @property
    def tree(self):
        return os.path.join(self.translator, 'tree')

    @property
    def pcode(self):
        return os.path.join(self.translator, 'pcode.json')

    @property
    def stamp(self):
        return os.path.join(self.translator, 'translate-stamp.json')


def sha1_file(path, h=None):
    h = h or hashlib.sha1()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 16), b''):
            h.update(chunk)
    return h


def tool_version(game):
    """One hash over the translator's sources and the export's seeds, each with its path."""
    h = hashlib.sha1()
    for pat in SOURCES:
        for p in sorted(glob.glob(os.path.join(game, pat))):
            h.update(os.path.relpath(p, game).encode() + b'\0')
            sha1_file(p, h)
    return h.hexdigest()


def stamp_of(paths):
    if not os.path.exists(paths.program):
        raise Usage(f'no program at {paths.program}: run `cargo run -p fd-tools -- extract` in '
                    'games/forest-dumb/engine/ (disks: docs/01-disk-location.md)')
    return {'program_sha1': sha1_file(paths.program).hexdigest(), 'tool': tool_version(paths.game)}


def read_json(path):
    try:
        with open(path, encoding='utf-8') as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


def ensure_tree(paths, fresh, log):
    """Export and translate the whole program unless the stamp says the tree is current.
    -> the translation's stats."""
    want = stamp_of(paths)
    have = read_json(paths.stamp)
    if not fresh and have and have.get('stamp') == want and os.path.exists(os.path.join(paths.tree, 'mod.rs')):
        log('tree: current, export and translation reused')
        return have.get('stats', {})
    os.makedirs(paths.translator, exist_ok=True)
    # The stamp goes first: a run killed between export and translation leaves no stale claim.
    if os.path.exists(paths.stamp):
        os.remove(paths.stamp)
    log(f'export: {trial.export(paths.translator)}')
    stats = trial.translate_tree(paths.pcode, paths.tree)
    log(f'translate: {json.dumps(stats)}')
    trial.write(paths.stamp, json.dumps({'stamp': want, 'stats': stats}))
    return stats


def take_lock(lock_dir):
    """judge_run's judge.lock, waited for: one fd-judge build at a time."""
    os.makedirs(lock_dir, exist_ok=True)
    f = open(os.path.join(lock_dir, 'judge.lock'), 'w')
    fcntl.flock(f, fcntl.LOCK_EX)
    return f


# --- Which routines ---------------------------------------------------------------------------------

def load_symbols(path):
    with open(path, 'rb') as f:
        return [s for s in tomllib.load(f).get('sym', []) if s.get('exe', 'forest_dumb_forever') == 'forest_dumb_forever']


def name_of(hunk, off, syms):
    for s in syms:
        if (s['hunk'], s['offset']) == (hunk, off):
            return s['name']
    return f'h{hunk}_{off:x}'


def resolve(arg, syms):
    try:
        return jr.resolve(arg, syms)
    except jr.Usage as e:
        raise Usage(str(e))


def current_plan():
    """The plan of this checkout's pir branch (`pir/{plan}` or `pir/{plan}-T{nn}`), or None."""
    try:
        r = subprocess.run(['git', '-C', HERE, 'branch', '--show-current'], capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    m = re.fullmatch(r'pir/(.+?)(?:-T\d+)?', r.stdout.strip())
    return m.group(1) if m else None


def task_doc(task, plans_dir):
    if task.endswith('.md'):
        if not os.path.exists(task):
            raise Usage(f'no task doc at {task}')
        return task
    hits = sorted(glob.glob(os.path.join(plans_dir, '*', 'tasks', f'{task}-*.md')))
    if len(hits) > 1:
        plan = current_plan()
        mine = [h for h in hits if plan and os.path.basename(os.path.dirname(os.path.dirname(h))) == plan]
        if len(mine) == 1:
            return mine[0]
        raise Usage(f'{task} is in no re/translate.toml row and several plans have its doc; name one: '
                    + ', '.join(os.path.relpath(h, os.path.dirname(plans_dir)) for h in hits))
    if not hits:
        raise Usage(f'{task}: no routine in re/translate.toml and no task doc plans/*/tasks/{task}-*.md')
    return hits[0]


def doc_routines(path, syms):
    with open(path, encoding='utf-8') as f:
        text = f.read()
    m = re.search(r'(?ms)^## Routines\s*\n(.*?)(?=^## |\Z)', text)
    text = m.group(1) if m else text
    names = {s['name'] for s in syms}
    out = []
    for m in re.finditer(r'h[0-4]\+\$[0-9a-fA-F]+|`([A-Za-z_][A-Za-z0-9_]*)`', text):
        if m.group(1) is None:
            out.append(resolve(m.group(0), syms))
        elif m.group(1) in names:
            out.append(resolve(m.group(1), syms))
    return out


def task_routines(task, paths, syms):
    """The task's routines as (hunk, offset), in address order for the inventory, doc order else."""
    if not re.fullmatch(r'T\d+', task) and not task.endswith('.md'):
        raise Usage(f'--task takes T{{nn}} or a task doc path, not `{task}`')
    with open(paths.inventory, 'rb') as f:
        rows = tomllib.load(f).get('routine', [])
    picked = sorted({(r['hunk'], r['offset']) for r in rows if task in (r.get('port_task'), r.get('gen_task'))})
    if picked:
        return picked
    seen, out = set(), []
    for k in doc_routines(task_doc(task, paths.plans), syms):
        if k not in seen:
            seen.add(k)
            out.append(k)
    if not out:
        raise Usage(f'{task}: no routine in re/translate.toml or in its task doc')
    return out


def find_gen(hunk, off, names, syms):
    """The generator registered for the routine at (hunk, off): its own name first, then the first
    whose name is `hN_off` or a symbol at that address."""
    own = name_of(hunk, off, syms)
    if own in names:
        return own
    for n in names:
        m = re.fullmatch(r'h([0-4])_([0-9a-f]+)', n)
        if m and (int(m.group(1)), int(m.group(2), 16)) == (hunk, off):
            return n
        if any(s['name'] == n and (s['hunk'], s['offset']) == (hunk, off) for s in syms):
            return n
    return None


# --- The routine's Rust ---------------------------------------------------------------------------

FN_RE = re.compile(r'^pub fn h(\d+)_([0-9a-f]+)\(', re.M)
CALL_RE = re.compile(r'(?:super::p\d+::)?\bh(\d+)_([0-9a-f]+)\(r, bus, img\)')


def tree_functions(tree):
    """(hunk, off) -> the function's text, from its `pub fn` to the next, from every part of the tree.
    The text holds the seam's entry and its `NAME_from` body (translator T28, T35). The attributes
    above each (`#[allow(..)]`, `#[inline]`) are dropped: noise when reading."""
    fns = {}
    for p in sorted(glob.glob(os.path.join(tree, '*.rs'))):
        with open(p, encoding='utf-8') as f:
            text = f.read()
        found = list(FN_RE.finditer(text))
        for i, m in enumerate(found):
            end = found[i + 1].start() if i + 1 < len(found) else len(text)
            body = re.sub(r'(?m)^#\[(?:allow\([^\n]*\)|inline)\]\n', '', text[m.start():end])
            fns[(int(m.group(1)), int(m.group(2), 16))] = body.rstrip() + '\n'
    return fns


def routine_rust(fns, hunk, off):
    """The routine's function and its callees, depth first in call order, cross-part paths dropped.
    None when the tree has no function at the routine."""
    if (hunk, off) not in fns:
        return None
    order, seen = [], set()

    def walk(k):
        seen.add(k)
        order.append(k)
        for m in CALL_RE.finditer(fns[k]):
            c = (int(m.group(1)), int(m.group(2), 16))
            if c in fns and c not in seen:
                walk(c)

    walk((hunk, off))
    return order, ''.join(re.sub(r'super::p\d+::', '', fns[k]) + '\n' for k in order)


def write_rust(paths, fns, hunk, off, name):
    got = routine_rust(fns, hunk, off)
    if got is None:
        return None
    order, text = got
    d = os.path.join(paths.out, jr.slug(hunk, off))
    os.makedirs(d, exist_ok=True)
    head = (f'// {jr.label(hunk, off)} `{name}` as the translator generated it, with its {len(order) - 1} '
            f'callee(s), routine first.\n// Read-only: copied from build/translator/tree/ for reading, never '
            f'compiled from here. Derived from the game\'s code: never commit it.\n\n')
    path = os.path.join(d, 'translated.rs')
    trial.write(path, head + text)
    return path


def shown(path, paths):
    """The path as the command line shows it: from the repo root when inside it."""
    rel = os.path.relpath(path, paths.root)
    return path if rel.startswith('..') else rel


# --- One routine --------------------------------------------------------------------------------------

def one(paths, fns, hunk, off, gen, names, cases, syms):
    lab, name = jr.label(hunk, off), name_of(hunk, off, syms)
    rust = write_rust(paths, fns, hunk, off, name)
    line = {'routine': lab, 'name': name, 'result': None, 'first': '', 'states': 0,
            'rust': shown(rust, paths) if rust else None}
    if rust is None:
        line.update(result='stop', first=f'untranslated: {lab} is no routine of the tree')
        return line
    gen = gen or find_gen(hunk, off, names, syms)
    if gen is None:
        line.update(result='no-gen', first=f'no generator registered for {lab}: write one (tools/generate.py)')
        return line
    s = trial.judge(lab, gen, cases, '--survey', release=True)
    if s.get('outcome') == 'usage':
        if 'no generator' in s['why']:
            line.update(result='no-gen', first=f'no generator `{gen}` registered')
            return line
        raise trial.StepFailed(f'survey {lab}: {s["why"]}')
    if 'outcome' in s:
        raise trial.StepFailed(f'survey {lab}: {s["why"]}')
    if s['passed'] == 0:
        line.update(result='no-oracle', first=f"the original returned on none of {s['cases']} states "
                                               f"({s['hangs']} hang(s), {s['faults']} fault(s))")
        return line
    if s.get('unmodelled_blits', 0):
        line.update(result='no-oracle', first=f"a line or fill blit on {s['unmodelled_blits']} of {s['passed']} states")
        return line
    j = trial.judge(lab, gen, cases, '--translated', release=True)
    if j.get('outcome') == 'usage':
        raise trial.StepFailed(f'judge {lab}: {j["why"]}')
    if 'outcome' in j:
        line.update(result='fail', first=j['why'])
        return line
    line['states'] = j.get('passed', 0) + j.get('failures', 0)
    if j['pass']:
        line['result'] = 'pass'
    elif any(j.get(k, 0) for k in STOP_KEYS):
        line.update(result='stop', first=j.get('first_stop') or j.get('first_failure', ''))
    else:
        line.update(result='fail', first=j.get('first_failure', ''))
    return line


def run(paths, targets, build_only, gen, cases, fresh, log, emit):
    """targets: [(hunk, off)]. Emits one line a routine; -> exit code."""
    lock = take_lock(paths.out)
    try:
        stats = ensure_tree(paths, fresh, log)
        if build_only:
            emit({'tree': shown(paths.tree, paths), **stats})
            return 0
        trial.build(paths.tree, release=True)
        syms = load_symbols(paths.symbols)
        names = [] if gen else trial.generators(release=True)
        fns = tree_functions(paths.tree)
        ok = True
        for hunk, off in targets:
            line = one(paths, fns, hunk, off, gen, names, cases, syms)
            ok &= line['result'] == 'pass'
            emit(line)
        return 0 if ok else 1
    finally:
        lock.close()


def main(argv=None, paths=None):
    paths = paths or Paths()
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('routine', nargs='?', help="`hN+$off` (quoted) or a name from re/symbols.toml")
    ap.add_argument('--task', help='T{nn}: its routines from re/translate.toml or its task doc')
    ap.add_argument('--build', action='store_true', help='export and translate the whole program, no judging')
    ap.add_argument('--gen', help='the generator to judge on (default: the one registered for the routine)')
    ap.add_argument('--cases', type=int, default=jr.CASES)
    ap.add_argument('--fresh', action='store_true', help='export and translate again, whatever the cache says')
    try:
        a = ap.parse_args(argv)
    except SystemExit as e:
        return 2 if e.code else 0
    log = lambda m: print(f'translate: {m}', file=sys.stderr, flush=True)  # noqa: E731
    emit = lambda d: print(json.dumps(d, separators=(',', ':')), flush=True)  # noqa: E731
    try:
        if sum(map(bool, (a.routine, a.task, a.build))) != 1:
            raise Usage('name one of: a routine, --task T{nn}, --build')
        if a.cases <= 0:
            raise Usage('--cases takes a whole number above 0')
        if a.build and a.gen:
            raise Usage('--build judges nothing: --gen does not go with it')
        syms = load_symbols(paths.symbols)
        if a.routine:
            targets = [resolve(a.routine, syms)]
        elif a.task:
            if a.gen:
                raise Usage("--task judges each routine on its own generator: --gen does not go with it")
            targets = task_routines(a.task, paths, syms)
        else:
            targets = []
        return run(paths, targets, a.build, a.gen, a.cases, a.fresh, log, emit)
    except Usage as e:
        print(f'translate: {e}', file=sys.stderr)
        return 2
    except trial.Usage as e:
        print(f'translate: {e}', file=sys.stderr)
        return 2
    except trial.StepFailed as e:
        print(f'translate: {e}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
