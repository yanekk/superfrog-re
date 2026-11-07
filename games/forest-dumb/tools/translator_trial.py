#!/usr/bin/env python3
"""Run the translator trial end to end and write the table the owner decides on (translator-trial
T06; its DESIGN §2.1 to §2.4, §2.7, §2.8, §3.5).

    python3 -I games/forest-dumb/tools/translator_trial.py [--routines <name,…>] [--skip-export] [--cases N]

Steps, each named when it fails: export (`pcode_export.py`, skipped with --skip-export, which then
needs build/translator/pcode.json), translate (`pcode_rust.py`, once without and once with
re/translator-hints.toml), build (`fd-judge` with the tree built in through FD_TRANSLATED, once a
tree), survey (the original alone, per routine with a generator) and judge (`fd-judge --translated`
per judgeable routine, once a tree). When the hints change nothing, the second tree is the first and
its verdicts are reused rather than built and judged again.

Writes build/translator/trial.json and build/translator/table.md (through a temporary file and a
rename) and prints one line:

    {"judgeable":n,"pass":n,"pass_pct":x,"pass_with_hints":n,"unjudgeable":n,"unsupported":n,"untranslated":n,"write_into_code":n}

`unsupported`, `untranslated` and `write_into_code` count routines, without hints, with at least one
state stopped that way. Exit 0 when the table was written, whatever the verdicts; 1 when a step
failed (named on stderr: a tree that does not compile names the compiler's first error and the
routine it came from; a second run names the lock); 2 on usage.

Unjudgeable (DESIGN §2.4): no generator (`gen_task` absent in re/translate.toml, or none registered
in fd-conform), or `no-oracle` exactly as translate.py answers it from the judge's survey. Each judge
run has translate.py's JUDGE_SECONDS; past it the routine fails with first stop `timeout`.

The run holds translate.py's build/translate/judge.lock from start to end, taken without waiting: the
judge binary is shared with translate.py, and build/translator/ with any other trial run.

Muse's column is `result` from T40's build/translate/batch.json, in this checkout or else the main
checkout's (where the batch ran), `none` without it.

Test only: FD_TRIAL_EXPORT, FD_TRIAL_BUILD and FD_TRIAL_JUDGE each replace one step's command with
the one they hold (split as a shell would); the build stub is given FD_TRANSLATED, the export stub
`--out <dir>`, the judge stub the judge's own arguments. Standard library only, for `python3 -I`.
"""
import argparse, concurrent.futures as cf, fcntl, json, os, re, shlex, subprocess, sys, time, tomllib
from dataclasses import dataclass, field

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import translate  # noqa: E402
import translate_batch  # noqa: E402

GAME = translate.GAME
ENGINE = translate.ENGINE
GATE_PCT = 90.0
BUILD_SECONDS = 1800
SURVEY_SECONDS = translate.JUDGE_SECONDS
# Judge runs at once: each is one process over a built binary, so they overlap freely; kept low so a
# loaded machine does not push an honest routine past its wall-clock limit.
PARALLEL = 4
SUMMARY_KEYS = ('judgeable', 'pass', 'pass_pct', 'pass_with_hints', 'unjudgeable', 'unsupported',
                'untranslated', 'write_into_code')


class Usage(Exception):
    """exit 2"""


class StepFailed(Exception):
    """exit 1, the step named in the message"""


@dataclass
class Paths:
    out: str = os.path.join(GAME, 'build', 'translator')
    lock_dir: str = translate.OUT
    inventory: str = translate.INVENTORY
    hints: str = os.path.join(GAME, 're', 'translator-hints.toml')
    batches: list = field(default_factory=lambda: [os.path.join(d, 'batch.json') for d in batch_dirs()])


def batch_dirs():
    """Where T40's batch.json is looked for: this checkout's build/translate, then the main checkout's."""
    dirs = [translate.OUT]
    try:
        r = subprocess.run(['git', '-C', HERE, 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                           capture_output=True, text=True, timeout=30)
        if r.returncode == 0:
            dirs.append(os.path.join(os.path.dirname(r.stdout.strip()), 'games', 'forest-dumb', 'build', 'translate'))
    except (OSError, subprocess.TimeoutExpired):
        pass
    return dirs


def command(var, default):
    v = os.environ.get(var)
    return shlex.split(v) if v else default


# --- Inputs ----------------------------------------------------------------------------------------

def routines(inventory, only=None):
    """The inventory's routines as translate_batch reads them, with `instructions`; `only` filters by
    name or label."""
    rows = translate_batch.entries(inventory)
    with open(inventory, 'rb') as f:
        sizes = {translate.label(r['hunk'], r['offset']): r.get('instructions') for r in tomllib.load(f).get('routine', [])}
    for r in rows:
        r['instructions'] = sizes.get(r['label'])
    if only:
        known = {r['name'] for r in rows} | {r['label'] for r in rows}
        missing = [x for x in only if x not in known]
        if missing:
            raise Usage(f"not in {inventory}: {', '.join(missing)}")
        rows = [r for r in rows if r['name'] in only or r['label'] in only]
    return rows


def muse_results(batches):
    """label -> Muse's result from the first batch.json found, or {} with none."""
    for p in batches:
        try:
            with open(p, encoding='utf-8') as f:
                return {k: r.get('result', 'none') for k, r in json.load(f).get('rows', {}).items()}
        except FileNotFoundError:
            continue
        except (OSError, ValueError) as e:
            raise StepFailed(f'muse: {p} is unreadable: {e}')
    return {}


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


def build(tree):
    """Build fd-judge with `tree` in it. A compile error is the translator's: named with its routine."""
    env = dict(os.environ, FD_TRANSLATED=os.path.abspath(tree))
    env.pop('FD_PORT', None)
    argv = command('FD_TRIAL_BUILD', ['cargo', 'build', '-q', '-p', 'fd-conform', '--bin', 'fd-judge',
                                      '--message-format', 'short'])
    try:
        p = subprocess.run(argv, cwd=ENGINE, env=env, capture_output=True, text=True, timeout=BUILD_SECONDS)
    except subprocess.TimeoutExpired:
        raise StepFailed(f'build: fd-judge did not build within {BUILD_SECONDS} s')
    if p.returncode == 0:
        return
    errs = [l for l in p.stderr.splitlines() if re.search(r'\berror\b', l)]
    first = errs[0] if errs else (p.stderr.strip().splitlines() or ['(no output)'])[-1]
    name = os.path.basename(tree)
    m = re.search(re.escape(name) + r':(\d+):\d+', first)
    where = routine_at_line(tree, int(m.group(1))) if m else None
    raise StepFailed(f"build: the generated {name} does not compile, in "
                     f"{where or 'no generated routine'}: {first}")


def judge(routine, gen, cases, mode):
    """One fd-judge run (`--survey` or `--translated`) -> its JSON line, or {'outcome': ...} for a
    timeout, a crash, or exit 2 (`usage`, the stderr in `why`)."""
    argv = command('FD_TRIAL_JUDGE', [translate.judge_bin()]) + [routine, '--gen', gen, '--cases', str(cases), mode]
    limit = SURVEY_SECONDS if mode == '--survey' else translate.JUDGE_SECONDS
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


def unjudgeable(r, cases):
    """The routine's reason it cannot be judged, from its survey, or None. A missing disk or the like
    ends the run."""
    if r['gen'] is None:
        return f"no generator ({r['subsystem'] or 'none planned'})"
    s = judge(r['label'], r['gen'], cases, '--survey')
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


def verdict(r, cases):
    """The translated tree built now, judged at the routine -> the row's half for this tree."""
    j = judge(r['label'], r['gen'], cases, '--translated')
    if j.get('outcome') == 'usage':
        raise StepFailed(f"judge {r['label']}: {j['why']}")
    if 'outcome' in j:
        return {'verdict': 'fail', 'first': j['why'], 'untranslated': 0, 'write_into_code': 0, 'unsupported': 0}
    return {'verdict': 'pass' if j['pass'] else 'fail',
            'first': j.get('first_stop') or j.get('first_failure') or '',
            'untranslated': j.get('untranslated', 0), 'write_into_code': j.get('write_into_code', 0),
            'unsupported': j.get('unsupported', 0), 'failures': j.get('failures', 0), 'passed': j.get('passed', 0)}


def judge_all(rows, cases, parallel=PARALLEL):
    with cf.ThreadPoolExecutor(max_workers=parallel) as pool:
        return dict(zip([r['label'] for r in rows], pool.map(lambda r: verdict(r, cases), rows)))


# --- The table -------------------------------------------------------------------------------------

def summary(rows):
    judged = [x for x in rows if not x['unjudgeable']]
    n, ok = len(judged), sum(1 for x in judged if x['plain']['verdict'] == 'pass')
    return {'judgeable': n, 'pass': ok, 'pass_pct': round(100.0 * ok / n, 1) if n else 0.0,
            'pass_with_hints': sum(1 for x in judged if x['hints']['verdict'] == 'pass'),
            'unjudgeable': len(rows) - n,
            **{k: sum(1 for x in judged if x['plain'].get(k, 0) > 0)
               for k in ('unsupported', 'untranslated', 'write_into_code')}}


def cell(s):
    return flat(s).replace('|', '\\|')


def table(rows, s, cases):
    lines = ['| Routine | Subsystem | Instructions | Without hints | First difference or stop | Untranslated '
             '| Write into code | Unsupported | With hints | Muse |',
             '|---|---|---|---|---|---|---|---|---|---|']
    for x in rows:
        name = f" `{x['name']}`" if x['name'] != x['routine'] else ''
        if x['unjudgeable']:
            v = f"unjudgeable: {x['unjudgeable']}"
            lines.append(f"| `{x['routine']}`{name} | {x['subsystem'] or ''} | {x['instructions'] or ''} | {cell(v)} "
                         f"|  |  |  |  | {cell(v)} | {x['muse']} |")
            continue
        p, h = x['plain'], x['hints']
        lines.append(f"| `{x['routine']}`{name} | {x['subsystem'] or ''} | {x['instructions'] or ''} | {p['verdict']} "
                     f"| {cell(p['first'])} | {p['untranslated']} | {p['write_into_code']} | {p['unsupported']} "
                     f"| {h['verdict']} | {x['muse']} |")
    met = 'met' if s['judgeable'] and s['pass_pct'] >= GATE_PCT else 'not met'
    lines += ['', f"Gate: {s['pass']} of {s['judgeable']} judgeable routines pass with no hand edit "
                  f"({s['pass_pct']}%), against {GATE_PCT:g}%: {met}. With hints {s['pass_with_hints']} pass; "
                  f"{s['unjudgeable']} unjudgeable, left out of the count. {cases} states a routine."]
    return '\n'.join(lines) + '\n'


def write(path, text):
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(tmp, path)


# --- The run ---------------------------------------------------------------------------------------

def take_lock(lock_dir):
    """translate.py's judge.lock, held until the process ends; a second run is refused, not queued."""
    os.makedirs(lock_dir, exist_ok=True)
    path = os.path.join(lock_dir, 'judge.lock')
    f = open(path, 'w')
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        f.close()
        raise StepFailed(f'lock: {path} is held by another trial run or translate.py; wait for it to finish')
    return f


def run(paths, only=None, skip_export=False, cases=translate.CASES, log=lambda m: None):
    """The whole trial -> the summary dict. Raises Usage, StepFailed."""
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
        build(trees['plain'])
        log(f'build plain: {round(time.time() - t)} s')
        with cf.ThreadPoolExecutor(max_workers=PARALLEL) as pool:
            reasons = dict(zip([r['label'] for r in rows_in], pool.map(lambda r: unjudgeable(r, cases), rows_in)))
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
                build(trees[kind])
                log(f'build {kind}: {round(time.time() - t)} s')
            t = time.time()
            results[kind] = judge_all(judged, cases)
            log(f'judge {kind}: {round(time.time() - t)} s')

        muse = muse_results(paths.batches)
        rows = [{'routine': r['label'], 'name': r['name'], 'subsystem': r['subsystem'],
                 'instructions': r['instructions'], 'gen': r['gen'], 'unjudgeable': reasons[r['label']],
                 'plain': results['plain'].get(r['label']), 'hints': results['hints'].get(r['label']),
                 'muse': muse.get(r['label'], 'none')} for r in rows_in]
        s = summary(rows)
        write(os.path.join(paths.out, 'trial.json'), json.dumps(
            {'finished': time.strftime('%Y-%m-%dT%H:%M:%S'), 'secs': round(time.time() - t0), 'cases': cases,
             'hints_changed_tree': not same, 'translate': stats, 'summary': s, 'rows': rows}, indent=1))
        write(os.path.join(paths.out, 'table.md'), table(rows, s, cases))
        return s
    finally:
        lock.close()


def main(argv=None, paths=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--routines', help='comma-separated names or `hN+$off` labels of re/translate.toml')
    ap.add_argument('--skip-export', action='store_true', help='reuse build/translator/pcode.json')
    ap.add_argument('--cases', type=int, default=translate.CASES)
    a = ap.parse_args(argv)
    if a.cases <= 0:
        print('translator_trial: --cases takes a whole number above 0', file=sys.stderr)
        return 2
    only = [x.strip() for x in a.routines.split(',') if x.strip()] if a.routines else None
    try:
        s = run(paths or Paths(), only, a.skip_export, a.cases,
                log=lambda m: print(f'translator_trial: {m}', file=sys.stderr, flush=True))
    except Usage as e:
        print(f'translator_trial: {e}', file=sys.stderr)
        return 2
    except StepFailed as e:
        print(f'translator_trial: {e}', file=sys.stderr)
        return 1
    print(json.dumps({k: s[k] for k in SUMMARY_KEYS}, separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
