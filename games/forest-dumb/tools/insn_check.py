#!/usr/bin/env python3
"""The instruction check: every census kind, translated alone, against Moira's step (translator T06,
DESIGN §2.6). Stage 1's measure; T07 to T12 fix what it reports.

    python3 -I games/forest-dumb/tools/insn_check.py [--kinds <k,…>] [--states 500]
        [--columns regs,ccr,mem,cycles] [--seed S] [--pcode build/translator/pcode.json]

Steps, each named when it fails: census (the kinds of `pcode.json`, as `pcode_census.py` counts them;
the export is `pcode_export.py`'s), translate (`pcode_rust.py --insn` over one site per kind and
opcode word, the first in address order), build (`fd-judge` optimised, with the module built in
through FD_INSN) and judge (`fd-judge --insn-all`, in PARALLEL shards).

`--kinds` takes, comma-separated, census kinds (`add.w Dn,(An)+`; a kind's own commas are matched
whole), mnemonics with or without a size (`add.w`, `add`) and family names (`bcd`, `compare`,
`arith`, `shift_bit`, `move_control`, `other`). Without it, every kind.

Writes, through a temporary file and a rename, build/translator/insn.json:

    [{"kind","family","task","cycles_task","encodings","sites","states","pass","fail","skipped",
      "columns":{"regs","ccr","mem","cycles","stop"},"first":{"encoding","at","state","column","what",
      "moira","translated"}|null,"first_skip"}]

and build/translator/insn.md, one row per kind sorted by fail count, after a table by family naming the
task that owns it (`cycles` failures belong to T11, or T12 for the data-dependent kinds). `cycles` is
`"not-counted"` while the generated code counts none. Prints one line:

    {"kinds":n,"sites":n,"failing_kinds":n,"states":n,"fail":n,"skipped":n,"columns":{..}}

Exit 0 when both files were written, whatever the verdicts; 1 when a step failed (named on stderr);
2 on usage. Holds judge_run's build/translate/judge.lock from the build to the end, refusing to wait.

Test only: FD_INSN_BUILD and FD_INSN_JUDGE replace the build's and the judge's command (split as a
shell would); the build stub is given FD_INSN, the judge stub the judge's own arguments.
Standard library only, for `python3 -I`.
"""
import argparse, concurrent.futures as cf, fcntl, json, os, re, shlex, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import judge_run as jr  # noqa: E402
import pcode_census as pc  # noqa: E402
import importlib  # noqa: E402
# The module, not pcode_fix.cycles(), the function of the same name its package defines.
pcycles = importlib.import_module("pcode_fix.cycles")

GAME = jr.GAME
ENGINE = jr.ENGINE
OUT = os.path.join(GAME, 'build', 'translator')
BUILD_SECONDS = 1800
# One shard checks about 50 sites a second at 500 states; the cap bounds a translation that loops.
JUDGE_SECONDS = 600
PARALLEL = 4
# judge_run's lock, shared with every tool building fd-judge; tests point it elsewhere.
LOCK_DIR = jr.JUDGE_DIR
COLUMNS = ('regs', 'ccr', 'mem', 'cycles')

_CC = ['hi', 'ls', 'cc', 'cs', 'ne', 'eq', 'vc', 'vs', 'pl', 'mi', 'ge', 'lt', 'gt', 'le', 'hs', 'lo']
# (family, the task fixing its regs, ccr and mem, its mnemonics), as T07 to T10 name them.
FAMILIES = [
    ('bcd', 'T07', {'abcd', 'sbcd', 'nbcd'}),
    ('compare', 'T07', {'cmp', 'cmpa', 'cmpi', 'cmpm'}),
    ('arith', 'T08', {'add', 'addx', 'addq', 'addi', 'adda', 'sub', 'subx', 'subq', 'subi', 'suba', 'neg',
                      'negx', 'clr', 'mulu', 'muls', 'divu', 'divs', 'and', 'andi', 'or', 'ori', 'eor',
                      'eori', 'not', 'ext', 'tst', 'chk'}),
    ('shift_bit', 'T09', {'asl', 'asr', 'lsl', 'lsr', 'rol', 'ror', 'roxl', 'roxr', 'btst', 'bset', 'bclr',
                          'bchg', 'tas', 'swap', 'st', 'sf'} | {'s' + c for c in _CC}),
    ('move_control', 'T10', {'move', 'movea', 'moveq', 'movem', 'lea', 'pea', 'link', 'unlk', 'exg', 'jmp',
                             'jsr', 'bsr', 'bra', 'rts', 'rte', 'rtr', 'trap', 'dbra', 'dbt', 'dbf'}
     | {'b' + c for c in _CC} | {'db' + c for c in _CC}),
]
OTHER = ('other', 'T10')
# Kinds whose cycles depend on data are T12's (pcode_fix.cycles.is_dynamic); every other kind's T11's.


class Usage(Exception):
    """exit 2"""


class StepFailed(Exception):
    """exit 1, the step named"""


def command(var, default):
    v = os.environ.get(var)
    return shlex.split(v) if v else default


# --- The census and the sites ----------------------------------------------------------------------

def family_of(mnemonic, dst=None):
    """(family, task) of a kind by its mnemonic (no size); an op on SR or CCR is move_control's."""
    m = mnemonic.lower()
    if dst in ('SR', 'CCR', 'USP'):
        return 'move_control', 'T10'
    for name, task, ms in FAMILIES:
        if m in ms:
            return name, task
    return OTHER


def cycles_task(mnemonic, src, dst=None):
    return 'T12' if pcycles.is_dynamic(mnemonic, src, dst) else 'T11'


def order(at):
    h, off = at.split('+$')
    return int(h[1:]), int(off, 16)


def kinds_and_sites(doc, path):
    """(census rows, {(kind, encoding): first site}) of an export."""
    rows = pc.census(doc, path)
    first = {}
    for at in sorted(doc['insns'], key=order):
        ins = doc['insns'][at]
        word = int(ins['bytes'][:4], 16)
        modes = pc.modes_of(ins['mnem'], ins['operands'], word, f'{path}: {at}')
        kind = ins['mnem'] + (' ' + ','.join(modes) if modes else '')
        first.setdefault((kind, ins['bytes'][:4].lower()), at)
    return rows, first


def select(rows, spec):
    """The census rows `spec` names (see --kinds); every row without one."""
    if not spec:
        return rows
    names = sorted({r['kind'] for r in rows}, key=len, reverse=True)
    chosen, i = [], 0
    while i < len(spec):
        whole = next((k for k in names if spec.startswith(k, i) and spec[i + len(k):i + len(k) + 1] in ('', ',')),
                     None)
        tok = whole or spec[i:].split(',', 1)[0]
        i += len(tok) + 1
        tok = tok.strip()
        if not tok:
            continue
        if whole:
            hit = [r for r in rows if r['kind'] == whole]
        else:
            fam = [f for f in [f[0] for f in FAMILIES] + [OTHER[0]] if f == tok]
            hit = [r for r in rows if (fam and family_of(r['mnemonic'], r['dst'])[0] == tok)
                   or r['kind'].split(' ')[0].lower() == tok.lower() or r['mnemonic'].lower() == tok.lower()]
        if not hit:
            raise Usage(f'--kinds: `{tok}` is no census kind, mnemonic or family '
                        f'({", ".join([f[0] for f in FAMILIES] + [OTHER[0]])})')
        chosen += [r for r in hit if r not in chosen]
    return [r for r in rows if r in chosen]


# --- Steps -----------------------------------------------------------------------------------------

def translate(pcode, sites, out_rs):
    path = out_rs + '.sites'
    with open(path, 'w', encoding='utf-8') as f:
        f.write('\n'.join(sites) + '\n')
    p = subprocess.run([sys.executable, '-I', os.path.join(HERE, 'pcode_rust.py'), pcode, '--insn',
                        '--sites', '@' + path, '--out', out_rs], capture_output=True, text=True)
    if p.returncode != 0:
        raise StepFailed(f'translate: pcode_rust.py --insn: {p.stderr.strip()}')
    return json.loads(p.stdout.strip().splitlines()[-1])


def build(module):
    env = dict(os.environ, FD_INSN=os.path.abspath(module))
    env.pop('FD_PORT', None)
    argv = command('FD_INSN_BUILD', ['cargo', 'build', '-q', '--release', '-p', 'fd-conform', '--bin', 'fd-judge',
                                     '--message-format', 'short'])
    try:
        p = subprocess.run(argv, cwd=ENGINE, env=env, capture_output=True, text=True, timeout=BUILD_SECONDS)
    except subprocess.TimeoutExpired:
        raise StepFailed(f'build: fd-judge did not build within {BUILD_SECONDS} s')
    if p.returncode != 0:
        errs = [l for l in p.stderr.splitlines() if re.search(r'\berror\b', l)]
        raise StepFailed(f'build: the generated {os.path.basename(module)} does not compile: '
                         f'{errs[0] if errs else p.stderr.strip()[-300:]}')


def judge_bin():
    target = os.environ.get('CARGO_TARGET_DIR') or os.path.join(ENGINE, 'target')
    return os.path.join(target, 'release', 'fd-judge')


def judge(states, columns, seed, parallel=PARALLEL):
    """Every site of the built module -> {site: its line}."""
    def shard(k):
        argv = command('FD_INSN_JUDGE', [judge_bin()]) + [
            '--insn-all', '--shard', f'{k}/{parallel}', '--states', str(states), '--columns', columns] + (
            ['--seed', seed] if seed else [])
        try:
            p = subprocess.run(argv, capture_output=True, text=True, timeout=JUDGE_SECONDS)
        except OSError as e:
            raise StepFailed(f'judge: cannot run {argv[0]}: {e.strerror}')
        except subprocess.TimeoutExpired:
            raise StepFailed(f'judge: shard {k}/{parallel} did not finish within {JUDGE_SECONDS} s')
        if p.returncode != 0:
            why = (p.stderr.strip().splitlines() or [f'exit {p.returncode}'])[-1]
            raise StepFailed(f'judge: shard {k}/{parallel}: {why}')
        out = []
        for line in p.stdout.splitlines():
            try:
                out.append(json.loads(line))
            except ValueError:
                raise StepFailed(f'judge: shard {k}/{parallel} printed a line that is not JSON: {line[:200]}')
        return out
    with cf.ThreadPoolExecutor(parallel) as ex:
        lines = [l for part in ex.map(shard, range(parallel)) for l in part]
    return {l['at']: l for l in lines}


def take_lock():
    os.makedirs(LOCK_DIR, exist_ok=True)
    path = os.path.join(LOCK_DIR, 'judge.lock')
    f = open(path, 'w')
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        f.close()
        raise StepFailed(f'lock: {path} is held by another judge run; wait for it to finish')
    return f


# --- The report ------------------------------------------------------------------------------------

def aggregate(rows, sites, lines, columns):
    """One result per kind from its sites' lines."""
    out = []
    for r in rows:
        fam, task = family_of(r['mnemonic'], r['dst'])
        k = {'kind': r['kind'], 'family': fam, 'task': task, 'cycles_task': cycles_task(r['mnemonic'], r['src'], r['dst']),
             'encodings': r['encodings'], 'sites': [], 'states': 0, 'pass': 0, 'fail': 0, 'skipped': 0,
             'columns': {c: (0 if c in columns else None) for c in COLUMNS + ('stop',)}, 'first': None,
             'first_skip': ''}
        k['columns']['stop'] = 0
        for enc in r['encodings']:
            at = sites.get((r['kind'], enc))
            line = lines.get(at)
            if line is None:
                continue
            k['sites'].append(at)
            for f in ('states', 'pass', 'fail', 'skipped'):
                k[f] += line[f]
            for c, v in line['columns'].items():
                if v == 'not-counted':
                    k['columns'][c] = 'not-counted'
                elif isinstance(v, int) and isinstance(k['columns'].get(c), int):
                    k['columns'][c] += v
            if line['first'] and k['first'] is None:
                k['first'] = dict(encoding=enc, at=at, **line['first'])
            if line['first_skip'] and not k['first_skip']:
                k['first_skip'] = f'{at}: {line["first_skip"]}'
        out.append(k)
    return out


def failing(k, col):
    v = k['columns'].get(col)
    return v if isinstance(v, int) else 0


def summary(results):
    cols = {}
    for c in COLUMNS + ('stop',):
        vals = [k['columns'][c] for k in results if k['columns'].get(c) is not None]
        cols[c] = 'not-counted' if vals and all(v == 'not-counted' for v in vals) else sum(
            v for v in vals if isinstance(v, int)) if vals else None
    return {'kinds': len(results), 'sites': sum(len(k['sites']) for k in results),
            'failing_kinds': sum(k['fail'] > 0 for k in results), 'states': sum(k['states'] for k in results),
            'fail': sum(k['fail'] for k in results), 'skipped': sum(k['skipped'] for k in results), 'columns': cols}


def cell(v):
    return '-' if v is None else str(v)


def first_text(f):
    if not f:
        return ''
    return f'{f["at"]} state {f["state"]}: {f["column"]} {f["what"]} moira {f["moira"]}, translated {f["translated"]}'


def markdown(results, s, states, columns):
    out = ['# Instruction check', '',
           f'Every census kind, translated alone (`pcode_rust.py --insn`), against Moira\'s step on {states} '
           f'states an opcode word; columns {", ".join(columns)}. Written by `tools/insn_check.py`.', '',
           f'{s["kinds"]} kinds, {s["sites"]} opcode words, {s["failing_kinds"]} kinds failing; '
           f'{s["fail"]} of {s["states"]} states fail, {s["skipped"]} skipped (the original took an exception). '
           f'By column: ' + ', '.join(f'{c} {cell(v)}' for c, v in s['columns'].items()) + '.', '',
           '## By family', '',
           '| Family | Task | Kinds | Failing | regs | ccr | mem | stop | cycles (T11/T12) |',
           '|---|---|---|---|---|---|---|---|---|']
    fams = [(f, t) for f, t, _ in FAMILIES] + [OTHER]
    for f, t in fams:
        ks = [k for k in results if k['family'] == f]
        if not ks:
            continue
        cyc = [k['columns']['cycles'] for k in ks]
        cyc_cell = ('not-counted' if all(c == 'not-counted' for c in cyc) else
                    f"{sum(failing(k, 'cycles') for k in ks if k['cycles_task'] == 'T11')}/"
                    f"{sum(failing(k, 'cycles') for k in ks if k['cycles_task'] == 'T12')}")
        out.append(f'| {f} | {t} | {len(ks)} | {sum(k["fail"] > 0 for k in ks)} | '
                   + ' | '.join(str(sum(failing(k, c) for k in ks)) for c in ('regs', 'ccr', 'mem', 'stop'))
                   + f' | {cyc_cell} |')
    out += ['', '## Kinds', '', 'Sorted by failing states; a state may fail in several columns.', '',
            '| Kind | Family | Words | States | Fail | Skipped | regs | ccr | mem | stop | cycles | First failure |',
            '|---|---|---|---|---|---|---|---|---|---|---|---|']
    for k in sorted(results, key=lambda k: (-k['fail'], k['kind'])):
        c = k['columns']
        out.append(f'| `{k["kind"]}` | {k["family"]} | {len(k["sites"])} | {k["states"]} | {k["fail"]} | '
                   f'{k["skipped"]} | ' + ' | '.join(cell(c[x]) for x in ('regs', 'ccr', 'mem', 'stop', 'cycles'))
                   + f' | {first_text(k["first"]).replace("|", "/")} |')
    return '\n'.join(out) + '\n'


def write(path, text):
    tmp = path + '.tmp'
    with open(tmp, 'w', encoding='utf-8') as f:
        f.write(text)
    os.replace(tmp, path)


# --- The run ---------------------------------------------------------------------------------------

def run(pcode, out, kinds=None, states=500, columns='regs,ccr,mem,cycles', seed=''):
    try:
        with open(pcode, encoding='utf-8') as f:
            doc = json.load(f)
    except OSError as e:
        raise StepFailed(f'census: {pcode}: {e.strerror}: run `python3 -I games/forest-dumb/tools/pcode_export.py`')
    except json.JSONDecodeError as e:
        raise StepFailed(f'census: {pcode}: not JSON: {e}')
    try:
        rows, first = kinds_and_sites(doc, pcode)
    except pc.BadInput as e:
        raise StepFailed(f'census: {e}')
    rows = select(rows, kinds)
    sites = {(r['kind'], enc): first[(r['kind'], enc)] for r in rows for enc in r['encodings']}
    os.makedirs(out, exist_ok=True)
    module = os.path.join(out, 'insn.rs')
    lock = take_lock()
    try:
        translate(pcode, sorted(set(sites.values()), key=order), module)
        build(module)
        lines = judge(states, columns, seed)
    finally:
        lock.close()
    results = aggregate(rows, sites, lines, columns.split(','))
    s = summary(results)
    write(os.path.join(out, 'insn.json'), json.dumps(results, indent=1) + '\n')
    write(os.path.join(out, 'insn.md'), markdown(results, s, states, columns.split(',')))
    return s


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    p.add_argument('--kinds')
    p.add_argument('--states', type=int, default=500)
    p.add_argument('--columns', default=','.join(COLUMNS))
    p.add_argument('--seed', default='')
    p.add_argument('--pcode', default=os.path.join(OUT, 'pcode.json'))
    p.add_argument('--out', default=OUT)
    a = p.parse_args(argv)
    try:
        if a.states < 1:
            raise Usage('--states must be 1 or more')
        bad = [c for c in a.columns.split(',') if c not in COLUMNS]
        if bad or not a.columns:
            raise Usage(f'--columns: `{",".join(bad)}` is not among {", ".join(COLUMNS)}')
        s = run(a.pcode, a.out, a.kinds, a.states, a.columns, a.seed)
    except Usage as e:
        print(f'insn_check: {e}', file=sys.stderr)
        return 2
    except StepFailed as e:
        print(f'insn_check: {e}', file=sys.stderr)
        return 1
    except OSError as e:
        print(f'insn_check: {a.out}: cannot write: {e.strerror}', file=sys.stderr)
        return 1
    print(json.dumps(s, separators=(',', ':')))
    return 0


if __name__ == '__main__':
    sys.exit(main())
