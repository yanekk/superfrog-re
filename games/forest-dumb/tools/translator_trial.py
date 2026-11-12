#!/usr/bin/env python3
"""Run the translator trial end to end and write the table the owner decides on (translator-trial
T06; its DESIGN §2.1 to §2.4, §2.7, §2.8, §3.5).

    python3 -I games/forest-dumb/tools/translator_trial.py [--routines <name,…> | --whole] [--machine] [--skip-export] [--cases N]

Steps, each named when it fails: export (`pcode_export.py`, skipped with --skip-export, which then
needs build/translator/pcode.json), translate (`pcode_rust.py`, once without and once with
re/translator-hints.toml), build (`fd-judge` with the tree built in through FD_TRANSLATED, once a
tree), survey (the original alone, per routine with a generator) and judge (`fd-judge --translated`
per judgeable routine, once a tree). When the hints change nothing, the second tree is the first and
its verdicts are reused rather than built and judged again.

Writes build/translator/trial.json and build/translator/table.md (through a temporary file and a
rename) and prints one line:

    {"judgeable":n,"pass":n,"pass_pct":x,"pass_with_hints":n,"unjudgeable":n,"unsupported":n,"untranslated":n,"write_into_code":n,"self_modifying":n}

`unsupported`, `untranslated` and `write_into_code` count routines, without hints, with at least one
state stopped that way. `self_modifying` counts routines with at least one state on which the
original itself changed a translated instruction's byte: those states are out of the routine's count
and not judged (translator DESIGN §2.5), and the table's column gives how many. Exit 0 when the table was written, whatever the verdicts; 1 when a step
failed (named on stderr: a tree that does not compile names the compiler's first error and the
routine it came from; a second run names the lock); 2 on usage.

`--whole` (translator T16, its DESIGN §2.2, §2.3, §2.12) is stage 1 and 2's gate on the whole tree.
It judges every routine of re/translate.toml and every generator `fd-judge --generators` lists (a
name `hN_off` or `hN+$off` is that address, any other is looked up in re/symbols.toml), and reads the
tree itself: its instructions, those translated as an unsupported stop, and every computed jump or
call the export gives no target. A call through a library base, `jsr -n(An)`, is the OS's calling
convention: its target is the OS's jump table outside every hunk, answered by the machine (DESIGN
§2.1, §2.8), so it is listed as a library call, not as unresolved. Every other unresolved site would
need a hint. The gate is met when nothing is unsupported or unresolved, re/translator-hints.toml
holds no hint, and every judgeable routine passes; the summary line gains WHOLE_KEYS, trial.json a
"whole" object and table.md the gate with every site that fails it.

`--machine` (translator T27, its DESIGN §2.8, §4) judges the translated tree on `pcode-machine`
(`fd-judge --translated --machine`) instead of the judge's copy of the oracle's hardware mode, and
writes trial-machine.json and table-machine.md, leaving the plain reading's files alone. The survey
is the original alone either way.

Unjudgeable (DESIGN §2.4): no generator (`gen_task` absent in re/translate.toml, or none registered
in fd-conform), or `no-oracle` exactly as translate.py answers it from the judge's survey. Each judge
run has judge_run's JUDGE_SECONDS; past it the routine fails with first stop `timeout`.

The run holds judge_run's build/translate/judge.lock from start to end, taken without waiting: the
judge binary is shared with every tool building fd-judge, and build/translator/ with any other trial run.

Test only: FD_TRIAL_EXPORT, FD_TRIAL_BUILD and FD_TRIAL_JUDGE each replace one step's command with
the one they hold (split as a shell would); the build stub is given FD_TRANSLATED, the export stub
`--out <dir>`, the judge stub the judge's own arguments. Standard library only, for `python3 -I`.
"""
import argparse, concurrent.futures as cf, fcntl, json, os, re, shlex, subprocess, sys, time, tomllib
from dataclasses import dataclass

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import judge_run as jr  # noqa: E402

GAME = jr.GAME
ENGINE = jr.ENGINE
GATE_PCT = 90.0
BUILD_SECONDS = 1800
SURVEY_SECONDS = jr.JUDGE_SECONDS
# Judge runs at once: each is one process over a built binary, so they overlap freely; kept low so a
# loaded machine does not push an honest routine past its wall-clock limit.
PARALLEL = 4
SUMMARY_KEYS = ('judgeable', 'pass', 'pass_pct', 'pass_with_hints', 'unjudgeable', 'unsupported',
                'untranslated', 'write_into_code', 'self_modifying')
WHOLE_KEYS = ('instructions', 'routines', 'unsupported_insns', 'unresolved_computed', 'library_calls', 'hints',
              'self_modifying_states', 'gate')


class Usage(Exception):
    """exit 2"""


class StepFailed(Exception):
    """exit 1, the step named in the message"""


@dataclass
class Paths:
    out: str = os.path.join(GAME, 'build', 'translator')
    lock_dir: str = jr.JUDGE_DIR
    inventory: str = jr.INVENTORY
    hints: str = os.path.join(GAME, 're', 'translator-hints.toml')
    symbols: str = jr.SYMBOLS


def command(var, default):
    v = os.environ.get(var)
    return shlex.split(v) if v else default


# --- Inputs ----------------------------------------------------------------------------------------

def entries(path):
    """The inventory's routines in address order: dicts with label, name, gen (None for sound),
    port_task, split_at (labels)."""
    with open(path, 'rb') as f:
        inv = tomllib.load(f)
    out = []
    for r in inv.get('routine', []):
        out.append({'label': jr.label(r['hunk'], r['offset']), 'name': r['name'],
                    'hunk': r['hunk'], 'offset': r['offset'],
                    # The generator is named as the routine (T35 to T39, T53, T54, T56); sound has none.
                    'gen': r['name'] if r.get('gen_task') else None,
                    'port_task': r.get('port_task'), 'subsystem': r.get('subsystem'),
                    'split_at': [jr.label(*jr.resolve(c)) for c in r.get('split_at', [])]})
    return sorted(out, key=lambda e: (e['hunk'], e['offset']))


def routines(inventory, only=None):
    """The inventory's routines as entries() reads them, with `instructions`; `only` filters by
    name or label."""
    rows = entries(inventory)
    with open(inventory, 'rb') as f:
        sizes = {jr.label(r['hunk'], r['offset']): r.get('instructions') for r in tomllib.load(f).get('routine', [])}
    for r in rows:
        r['instructions'] = sizes.get(r['label'])
    if only:
        known = {r['name'] for r in rows} | {r['label'] for r in rows}
        missing = [x for x in only if x not in known]
        if missing:
            raise Usage(f"not in {inventory}: {', '.join(missing)}")
        rows = [r for r in rows if r['name'] in only or r['label'] in only]
    return rows


def judge_bin(release):
    """The fd-judge `build` made: judge_run's debug binary, or the optimised one under --whole."""
    return os.path.join(os.path.dirname(os.path.dirname(jr.judge_bin())), 'release', 'fd-judge') if release \
        else jr.judge_bin()


def generators(release=False):
    """Every generator name the built fd-judge registers, in registry order."""
    argv = command('FD_TRIAL_JUDGE', [judge_bin(release)]) + ['--generators']
    try:
        p = subprocess.run(argv, capture_output=True, text=True, timeout=SURVEY_SECONDS)
    except OSError as e:
        raise StepFailed(f'generators: cannot run {argv[0]}: {e.strerror}')
    except subprocess.TimeoutExpired:
        raise StepFailed(f'generators: `{shlex.join(argv)}` did not answer within {SURVEY_SECONDS} s')
    try:
        return json.loads(p.stdout.strip().splitlines()[-1])['generators']
    except (IndexError, ValueError, KeyError, TypeError):
        raise StepFailed(f'generators: `{shlex.join(argv)}` exited {p.returncode} without a list: '
                         f'{flat(p.stderr or p.stdout)}')


def whole_routines(rows, names, symbols_path):
    """The inventory's rows, then one row per registered generator whose routine none of them is.
    A routine is judged once: by the inventory's generator, else the first registered for it by
    name or address (the spike's older `hN+$off` generators only fill a gap)."""
    with open(symbols_path, 'rb') as f:
        syms = [s for s in tomllib.load(f).get('sym', []) if s.get('exe', 'forest_dumb_forever') == 'forest_dumb_forever']
    by_label = {r['label']: r for r in rows}
    by_name = {r['name']: r for r in rows}
    out = list(rows)
    for n in names:
        if n in by_name:
            if by_name[n]['gen'] is None:
                by_name[n]['gen'] = n
            continue
        m = re.fullmatch(r'h([0-4])_([0-9a-f]+)', n)
        try:
            hunk, off = (int(m.group(1)), int(m.group(2), 16)) if m else jr.resolve(n, syms)
        except jr.Usage:
            raise StepFailed(f'generators: `{n}` is registered but names no routine (not `hN_off`, `hN+$off` '
                             f'or a name in {symbols_path})')
        lab = jr.label(hunk, off)
        r = by_label.get(lab)
        if r is None:
            r = {'label': lab, 'name': n, 'hunk': hunk, 'offset': off, 'gen': n, 'port_task': None,
                 'subsystem': None, 'split_at': [], 'instructions': None}
            by_label[lab] = r
            out.append(r)
        elif r['gen'] is None:
            r['gen'] = n
    return sorted(out, key=lambda e: (e['hunk'], e['offset']))


def library_call(insn):
    """`jsr d16(An)` with a negative displacement: a call into a library's jump table below its base."""
    b = insn.get('bytes') or ''
    if len(b) < 8 or int(b[:4], 16) & 0xfff8 != 0x4ea8:
        return False
    return int(b[4:8], 16) >= 0x8000


def addr_key(a):
    m = re.fullmatch(r'h(\d+)\+\$([0-9a-f]+)', a)
    return (int(m.group(1)), int(m.group(2), 16)) if m else (99, a)


def tree_reading(pcode, hints_path, translated):
    """What the whole tree holds, read from the export, the hints file and the plain translation's
    stats: instruction count, unsupported instructions, unresolved computed sites (library calls
    apart), the hints."""
    with open(pcode, encoding='utf-8') as f:
        insns = json.load(f)['insns']
    jumps, lib = [], []
    for a, i in insns.items():
        fl = i.get('flow') or {}
        if (fl.get('kind') == 'computed' or fl.get('computed')) and not fl.get('targets'):
            (lib if library_call(i) else jumps).append(a)
    with open(hints_path, 'rb') as f:
        hints = [h.get('at', '?') for h in tomllib.load(f).get('hint', [])]
    return {'instructions': len(insns), 'unsupported_insns': translated.get('unsupported', 0),
            'unresolved': sorted(jumps, key=addr_key), 'library': sorted(lib, key=addr_key), 'hints': hints}


def whole_gate(reading, s):
    """The whole-tree gate: met, and every reason it is not."""
    why = []
    if reading['unsupported_insns']:
        why.append(f"{reading['unsupported_insns']} unsupported instruction(s)")
    if reading['unresolved']:
        why.append('unresolved computed jump(s) needing a hint: ' + ', '.join(reading['unresolved']))
    if reading['hints']:
        why.append('hint(s), each a hand edit: ' + ', '.join(reading['hints']))
    if not s['judgeable']:
        why.append('no routine judgeable')
    elif s['pass'] < s['judgeable']:
        why.append(f"{s['judgeable'] - s['pass']} judgeable routine(s) fail")
    return not why, why


# --- Steps -----------------------------------------------------------------------------------------

def export(out):
    argv = command('FD_TRIAL_EXPORT', [sys.executable, '-I', os.path.join(HERE, 'pcode_export.py')]) + ['--out', out]
    p = subprocess.run(argv, capture_output=True, text=True)
    if p.returncode != 0:
        raise StepFailed(f'export: `{shlex.join(argv)}` exited {p.returncode}:\n{p.stderr.strip()[-2000:]}')
    return p.stdout.strip()


def translate_tree(pcode, tree, hints=None):
    argv = [sys.executable, '-I', os.path.join(HERE, 'pcode_rust.py'), pcode, '--out', tree]
    if hints:
        argv += ['--hints', hints]
    p = subprocess.run(argv, capture_output=True, text=True)
    if p.returncode != 0:
        raise StepFailed(f"translate{' with hints' if hints else ''}: {p.stderr.strip()[-2000:]}")
    return json.loads(p.stdout.strip().splitlines()[-1])


def routine_at_line(tree, line):
    """The `hN+$off` of the generated function holding line `line` (1-based) of `tree`, or None."""
    found = None
    with open(tree, encoding='utf-8') as f:
        for i, l in enumerate(f, 1):
            if i > line:
                break
            m = re.match(r'pub fn h(\d+)_([0-9a-f]+)\(', l)
            if m:
                found = f'h{m.group(1)}+${m.group(2)}'
    return found


def build(tree, release=False):
    """Build fd-judge with `tree` in it, optimised under --whole. A compile error is the translator's:
    named with its routine."""
    env = dict(os.environ, FD_TRANSLATED=os.path.abspath(tree))
    env.pop('FD_PORT', None)
    argv = command('FD_TRIAL_BUILD', ['cargo', 'build', '-q'] + (['--release'] if release else []) +
                   ['-p', 'fd-conform', '--bin', 'fd-judge', '--message-format', 'short'])
    try:
        p = subprocess.run(argv, cwd=ENGINE, env=env, capture_output=True, text=True, timeout=BUILD_SECONDS)
    except subprocess.TimeoutExpired:
        raise StepFailed(f'build: fd-judge did not build within {BUILD_SECONDS} s')
    if p.returncode == 0:
        return
    errs = [l for l in p.stderr.splitlines() if re.search(r'\berror\b', l)]
    first = errs[0] if errs else (p.stderr.strip().splitlines() or ['(no output)'])[-1]
    name = os.path.basename(tree)
    if os.path.isdir(tree):
        # A directory tree is compiled where it is (build.rs's `#[path]`): rustc names the part.
        m = re.search(r'(?:^|[/\\])((?:mod|p\d+)\.rs):(\d+):\d+', first)
        where = routine_at_line(os.path.join(tree, m.group(1)), int(m.group(2))) if m else None
    else:
        # rustc names fd-conform's build.rs copy (OUT_DIR/tree.rs, line for line the same), not `tree`.
        m = re.search(r'(?:^|[/\\])(?:tree\.rs|' + re.escape(name) + r'):(\d+):\d+', first)
        where = routine_at_line(tree, int(m.group(1))) if m else None
    raise StepFailed(f"build: the generated {name} does not compile, in "
                     f"{where or 'no generated routine'}: {first}")


def judge(routine, gen, cases, mode, release=False, machine=False):
    """One fd-judge run (`--survey` or `--translated`, on the machine with `machine`) -> its JSON
    line, or {'outcome': ...} for a timeout, a crash, or exit 2 (`usage`, the stderr in `why`)."""
    extra = ['--machine'] if machine and mode == '--translated' else []
    argv = command('FD_TRIAL_JUDGE', [judge_bin(release)]) + [routine, '--gen', gen, '--cases', str(cases)] + extra + [mode]
    limit = SURVEY_SECONDS if mode == '--survey' else jr.JUDGE_SECONDS
    try:
        p = subprocess.run(argv, capture_output=True, text=True, timeout=limit)
    except OSError as e:
        raise StepFailed(f'judge: cannot run {argv[0]}: {e.strerror}')
    except subprocess.TimeoutExpired:
        return {'outcome': 'timeout', 'why': f'timeout ({limit} s)'}
    if p.returncode == 2:
        return {'outcome': 'usage', 'why': p.stderr.strip()}
    try:
        return json.loads(p.stdout.strip().splitlines()[-1])
    except (IndexError, ValueError):
        sig = f'signal {-p.returncode}' if p.returncode < 0 else f'exit {p.returncode}'
        last = (p.stderr.strip().splitlines() or [''])[-1][:200]
        return {'outcome': 'crash', 'why': f'crash ({sig}){": " + last if last else ""}'}


def unjudgeable(r, cases, release=False):
    """The routine's reason it cannot be judged, from its survey, or None. A missing disk or the like
    ends the run."""
    if r['gen'] is None:
        return f"no generator ({r['subsystem'] or 'none planned'})"
    s = judge(r['label'], r['gen'], cases, '--survey', release)
    if s.get('outcome') == 'usage':
        if 'no generator' in s['why']:
            return f"no generator registered (`{r['gen']}`)"
        raise StepFailed(f"survey {r['label']}: {s['why']}")
    if 'outcome' in s:
        raise StepFailed(f"survey {r['label']}: {s['why']}")
    # translate.py's two no-oracle answers, word for word in meaning.
    if s['passed'] == 0:
        return (f"no-oracle: the original returned on none of {s['cases']} states "
                f"({s['hangs']} hang(s), {s['faults']} fault(s))")
    if s.get('unmodelled_blits', 0):
        return f"no-oracle: a line or fill blit on {s['unmodelled_blits']} of {s['passed']} states"
    return None


def flat(s, n=160):
    s = ' '.join(str(s).split())
    return s if len(s) <= n else s[:n - 1] + '…'


def verdict(r, cases, release=False, machine=False):
    """The translated tree built now, judged at the routine -> the row's half for this tree."""
    j = judge(r['label'], r['gen'], cases, '--translated', release, machine)
    if j.get('outcome') == 'usage':
        raise StepFailed(f"judge {r['label']}: {j['why']}")
    if 'outcome' in j:
        return {'verdict': 'fail', 'first': j['why'], 'untranslated': 0, 'write_into_code': 0, 'unsupported': 0,
                'self_modifying': 0}
    return {'verdict': 'pass' if j['pass'] else 'fail',
            'first': j.get('first_stop') or j.get('first_failure') or '',
            'untranslated': j.get('untranslated', 0), 'write_into_code': j.get('write_into_code', 0),
            'unsupported': j.get('unsupported', 0), 'self_modifying': j.get('self_modifying', 0),
            'failures': j.get('failures', 0), 'passed': j.get('passed', 0)}


def judge_all(rows, cases, parallel=PARALLEL, release=False, machine=False):
    with cf.ThreadPoolExecutor(max_workers=parallel) as pool:
        return dict(zip([r['label'] for r in rows], pool.map(lambda r: verdict(r, cases, release, machine), rows)))


# --- The table -------------------------------------------------------------------------------------

def summary(rows):
    judged = [x for x in rows if not x['unjudgeable']]
    n, ok = len(judged), sum(1 for x in judged if x['plain']['verdict'] == 'pass')
    return {'judgeable': n, 'pass': ok, 'pass_pct': round(100.0 * ok / n, 1) if n else 0.0,
            'pass_with_hints': sum(1 for x in judged if x['hints']['verdict'] == 'pass'),
            'unjudgeable': len(rows) - n,
            **{k: sum(1 for x in judged if x['plain'].get(k, 0) > 0)
               for k in ('unsupported', 'untranslated', 'write_into_code', 'self_modifying')}}


def self_modifying_states(rows):
    return sum(x['plain'].get('self_modifying', 0) for x in rows if not x['unjudgeable'])


def cell(s):
    return flat(s).replace('|', '\\|')


def table(rows, s, cases, whole=None):
    lines = ['| Routine | Subsystem | Instructions | Without hints | First difference or stop | Untranslated '
             '| Write into code | Self-modifying | Unsupported | With hints |',
             '|---|---|---|---|---|---|---|---|---|---|']
    for x in rows:
        name = f" `{x['name']}`" if x['name'] != x['routine'] else ''
        if x['unjudgeable']:
            v = f"unjudgeable: {x['unjudgeable']}"
            lines.append(f"| `{x['routine']}`{name} | {x['subsystem'] or ''} | {x['instructions'] or ''} | {cell(v)} "
                         f"|  |  |  |  |  | {cell(v)} |")
            continue
        p, h = x['plain'], x['hints']
        lines.append(f"| `{x['routine']}`{name} | {x['subsystem'] or ''} | {x['instructions'] or ''} | {p['verdict']} "
                     f"| {cell(p['first'])} | {p['untranslated']} | {p['write_into_code']} | {p.get('self_modifying', 0)} | {p['unsupported']} "
                     f"| {h['verdict']} |")
    if whole is None:
        met = 'met' if s['judgeable'] and s['pass_pct'] >= GATE_PCT else 'not met'
        lines += ['', f"Gate: {s['pass']} of {s['judgeable']} judgeable routines pass with no hand edit "
                      f"({s['pass_pct']}%), against {GATE_PCT:g}%: {met}. With hints {s['pass_with_hints']} pass; "
                      f"{s['unjudgeable']} unjudgeable, left out of the count. {cases} states a routine; "
                      f"{s['self_modifying']} routine(s) with self-modifying states, each judged on the rest."]
        return '\n'.join(lines) + '\n'
    rd, (ok, why) = whole['reading'], (whole['gate'], whole['why'])
    lines += ['', f"Whole tree: {rd['instructions']} instructions, {rd['unsupported_insns']} unsupported, "
                  f"{len(rd['unresolved'])} unresolved computed jump(s){': ' + ', '.join(rd['unresolved']) if rd['unresolved'] else ''}, "
                  f"{len(rd['library'])} library call(s) answered by the machine"
                  f"{': ' + ', '.join(rd['library']) if rd['library'] else ''}, "
                  f"{len(rd['hints'])} hint(s){': ' + ', '.join(rd['hints']) if rd['hints'] else ''}.",
              '', f"Whole-tree gate: {s['pass']} of {s['judgeable']} judgeable routines pass with no hand edit "
                  f"on {cases} states each, of {len(rows)} ({s['unjudgeable']} unjudgeable); "
                  f"{s['self_modifying']} routine(s) with self-modifying states ({s['self_modifying_states']} states), "
                  f"each judged on the rest: {'met' if ok else 'not met: ' + '; '.join(why)}."]
    return '\n'.join(lines) + '\n'


def write(path, text):
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(tmp, path)


# --- The run ---------------------------------------------------------------------------------------

def take_lock(lock_dir):
    """judge_run's judge.lock, held until the process ends; a second run is refused, not queued."""
    os.makedirs(lock_dir, exist_ok=True)
    path = os.path.join(lock_dir, 'judge.lock')
    f = open(path, 'w')
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        f.close()
        raise StepFailed(f'lock: {path} is held by another trial run or translate.py; wait for it to finish')
    return f


def run(paths, only=None, skip_export=False, cases=jr.CASES, log=lambda m: None, whole=False, machine=False):
    """The whole trial -> the summary dict (with WHOLE_KEYS under --whole). Raises Usage, StepFailed."""
    rows_in = routines(paths.inventory, only)
    lock = take_lock(paths.lock_dir)
    try:
        t0 = time.time()
        os.makedirs(paths.out, exist_ok=True)
        pcode = os.path.join(paths.out, 'pcode.json')
        if skip_export:
            if not os.path.exists(pcode):
                raise StepFailed(f'export: --skip-export, but {pcode} does not exist')
        else:
            log(f'export: {export(paths.out)}')
        if not os.path.exists(paths.hints):
            raise Usage(f'no hints file at {paths.hints}')
        trees = {'plain': os.path.join(paths.out, 'tree.rs'), 'hints': os.path.join(paths.out, 'tree-hints.rs')}
        stats = {'plain': translate_tree(pcode, trees['plain']),
                 'hints': translate_tree(pcode, trees['hints'], paths.hints)}
        log(f"translate: {json.dumps(stats['plain'])}; with hints {json.dumps(stats['hints'])}")
        with open(trees['plain'], 'rb') as a, open(trees['hints'], 'rb') as b:
            same = a.read() == b.read()

        # The survey runs the original alone, but through the built binary: build the plain tree first.
        t = time.time()
        # Optimised under --whole: unoptimised, view_tiles's 2 000 translated states took 340 s,
        # past JUDGE_SECONDS, where the original alone took 5 s (T16).
        build(trees['plain'], whole)
        log(f'build plain: {round(time.time() - t)} s')
        if whole:
            names = generators(whole)
            rows_in = whole_routines(rows_in, names, paths.symbols)
            log(f'whole: {len(names)} registered generators, {len(rows_in)} routines')
        with cf.ThreadPoolExecutor(max_workers=PARALLEL) as pool:
            reasons = dict(zip([r['label'] for r in rows_in], pool.map(lambda r: unjudgeable(r, cases, whole), rows_in)))
        judged = [r for r in rows_in if reasons[r['label']] is None]
        log(f'survey: {len(judged)} of {len(rows_in)} judgeable')
        results = {}
        for kind in ('plain', 'hints'):
            if kind == 'hints':
                if same:
                    results['hints'] = results['plain']
                    log('hints: the tree is unchanged, verdicts reused')
                    continue
                t = time.time()
                build(trees[kind], whole)
                log(f'build {kind}: {round(time.time() - t)} s')
            t = time.time()
            results[kind] = judge_all(judged, cases, release=whole, machine=machine)
            log(f'judge {kind}: {round(time.time() - t)} s')

        rows = [{'routine': r['label'], 'name': r['name'], 'subsystem': r['subsystem'],
                 'instructions': r['instructions'], 'gen': r['gen'], 'unjudgeable': reasons[r['label']],
                 'plain': results['plain'].get(r['label']), 'hints': results['hints'].get(r['label'])}
                for r in rows_in]
        s = summary(rows)
        w = None
        if whole:
            rd = tree_reading(pcode, paths.hints, stats['plain'])
            ok, why = whole_gate(rd, s)
            w = {'reading': rd, 'gate': ok, 'why': why}
            s.update(instructions=rd['instructions'], routines=len(rows), unsupported_insns=rd['unsupported_insns'],
                     unresolved_computed=len(rd['unresolved']), library_calls=len(rd['library']),
                     hints=len(rd['hints']), self_modifying_states=self_modifying_states(rows), gate=ok)
        doc = {'finished': time.strftime('%Y-%m-%dT%H:%M:%S'), 'secs': round(time.time() - t0), 'cases': cases,
               'hints_changed_tree': not same, 'translate': stats, 'summary': s, 'rows': rows}
        if w:
            doc['whole'] = w
        if machine:
            doc['hardware'] = 'machine'
        suffix = '-machine' if machine else ''
        write(os.path.join(paths.out, f'trial{suffix}.json'), json.dumps(doc, indent=1))
        write(os.path.join(paths.out, f'table{suffix}.md'), table(rows, s, cases, w))
        return s
    finally:
        lock.close()


def main(argv=None, paths=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument('--routines', help='comma-separated names or `hN+$off` labels of re/translate.toml')
    mode.add_argument('--whole', action='store_true',
                      help='every routine with a registered generator, and the whole-tree gate (T16)')
    ap.add_argument('--machine', action='store_true',
                    help='judge on pcode-machine; writes trial-machine.json and table-machine.md (T27)')
    ap.add_argument('--skip-export', action='store_true', help='reuse build/translator/pcode.json')
    ap.add_argument('--cases', type=int, default=jr.CASES)
    a = ap.parse_args(argv)
    if a.cases <= 0:
        print('translator_trial: --cases takes a whole number above 0', file=sys.stderr)
        return 2
    only = [x.strip() for x in a.routines.split(',') if x.strip()] if a.routines else None
    try:
        s = run(paths or Paths(), only, a.skip_export, a.cases,
                log=lambda m: print(f'translator_trial: {m}', file=sys.stderr, flush=True), whole=a.whole,
                machine=a.machine)
    except Usage as e:
        print(f'translator_trial: {e}', file=sys.stderr)
        return 2
    except StepFailed as e:
        print(f'translator_trial: {e}', file=sys.stderr)
        return 1
    keys = SUMMARY_KEYS + (WHOLE_KEYS if a.whole else ())
    print(json.dumps({k: s[k] for k in keys}, separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
