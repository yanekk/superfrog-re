#!/usr/bin/env python3
"""The routine oracle's plumbing shared by the model tools and the trial: the routine's listing
(`flow_text`), the judge lock and `fd-judge`'s invocation (DESIGN §2.13; moved unchanged from Muse's
translate.py by T17, so T18 can replace that file).

`run_judge` builds fd-judge with a port compiled in and runs it, holding `<out>/judge.lock` across
the build and the run; `JUDGE_DIR`'s lock is the one every tool shares by default. Python's standard
library only, so it runs under `python3 -I`.
"""
import bisect, fcntl, json, os, re, subprocess, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = os.path.dirname(HERE)
ENGINE = os.path.join(GAME, 'engine')
LISTING = os.path.join(GAME, 'build', 're', 'forest_dumb_forever.lst')
SYMBOLS = os.path.join(GAME, 're', 'symbols.toml')
# T34's inventory: where a routine's `split_at` callees are named.
INVENTORY = os.path.join(GAME, 're', 'translate.toml')
# Where translate.py's default lock lives: every tool building fd-judge serialises on it.
JUDGE_DIR = os.path.join(GAME, 'build', 'translate')

CASES = 2000
JUDGE_SECONDS = 120


class Usage(Exception):
    """A mistake in how the tool was called or set up: exit 2, no result line."""


# --- The routine and its listing ------------------------------------------------------------------

def symbols():
    with open(SYMBOLS, 'rb') as f:
        return [s for s in tomllib.load(f)['sym'] if s.get('exe', 'forest_dumb_forever') == 'forest_dumb_forever']


def resolve(arg, syms=None):
    """`hN+$off` or a symbol's name -> (hunk, offset)."""
    m = re.fullmatch(r'h([0-4])\+\$([0-9a-fA-F]+)', arg)
    if m:
        return int(m.group(1)), int(m.group(2), 16)
    for s in syms if syms is not None else symbols():
        if s['name'] == arg:
            return s['hunk'], s['offset']
    raise Usage(f"`{arg}` is neither `hN+$off` (N a hunk 0-4) nor a name in re/symbols.toml")


def label(hunk, off):
    return f'h{hunk}+${off:x}'


def slug(hunk, off):
    return f'h{hunk}-{off:x}'


def load_listing():
    if not os.path.exists(LISTING):
        raise Usage(f'{LISTING} not found: run `cargo run -p fd-tools -- extract` in games/forest-dumb/engine/, '
                    'then `python3 games/forest-dumb/tools/re_listing.py`')
    with open(LISTING, encoding='utf-8') as f:
        lines = f.read().splitlines()
    ins, at = {}, {}
    for i, l in enumerate(lines):
        m = re.match(r'^([0-9a-f]{8})  ([0-9a-f]+)\s+(.*)$', l)
        if m:
            a = int(m.group(1), 16)
            ins[a] = (m.group(3), len(m.group(2)) // 2)
            at[a] = i
    labels = {}
    for i, l in enumerate(lines):
        m = re.match(r'^(\S+):$', l)
        if m:
            j = i + 1
            while j < len(lines) and not re.match(r'^[0-9a-f]{8} ', lines[j]):
                j += 1
            if j < len(lines):
                labels[m.group(1)] = int(lines[j][:8], 16)
    return lines, ins, at, labels


# Only real control flow is followed: `btst`/`bset`/`bclr`/`bchg` (and the `bf*` bitfield ops) also start
# with `b` and take a label, which is data (T58: `btst.b #n,h0+$4ed2` led into `char_probe`). The same
# set as call_tree.BRANCH_RE, copied rather than imported because call_tree needs amitools.
_CC = 'hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le|hs|lo'
BRANCH_RE = re.compile(rf'^(bra|bsr|b({_CC})|db(ra|t|f|{_CC}))(\.[bsw])?$')


def flow_text(hunk, off, stops=()):
    """The spike's `run.py --hw` extraction: every instruction reachable from the routine
    (fall-through, branches both ways, calls), as listing lines in address order with gaps marked,
    then the named labels it uses. A call into `stops` (the split callees) is not followed.
    -> (text, instructions)."""
    lines, ins, at, labels = load_listing()
    syms = symbols()
    named = {(s['hunk'], s['offset']): s['name'] for s in syms}

    def where(h, o):
        a = labels.get(label(h, o))
        if a is None and (h, o) in named:
            a = labels.get(named[(h, o)])
        if a is None:
            raise Usage(f'{label(h, o)} has no label in the listing: is it the start of a routine?')
        return a

    start = where(hunk, off)
    stop = {where(h, o) for h, o in stops}
    addrs = sorted(ins)

    def containing(t):
        k = bisect.bisect_right(addrs, t) - 1
        return addrs[k] if k >= 0 and t < addrs[k] + ins[addrs[k]][1] else None

    seen, todo = set(), [start]
    while todo:
        pc = todo.pop()
        while pc is not None and pc in ins and pc not in seen and pc not in stop:
            seen.add(pc)
            txt, sz = ins[pc]
            parts = txt.split(None, 1)
            op = parts[0]
            t = labels.get(parts[1].split(',')[-1].strip()) if len(parts) > 1 else None
            if t is not None and t not in ins:
                t = containing(t)
            if op in ('rts', 'rte'):
                break
            if op.startswith('bra') or op == 'jmp':
                pc = t
                continue
            if t is not None and (BRANCH_RE.match(op) or op == 'jsr'):
                todo.append(t)
            pc += sz
    out, prev = [], None
    for a in sorted(seen):
        i = at[a]
        if prev is not None and a != prev:
            out.append('          ...')
        k = i
        while k > 0 and not re.match(r'^[0-9a-f]{8} ', lines[k - 1]) and not lines[k - 1].startswith('          ...'):
            k -= 1
        out += [l for l in lines[k:i] if re.match(r'^\S+:$', l)]
        out.append(lines[i])
        prev = a + ins[a][1]
    text = '\n'.join(out)
    used = [s for s in syms if re.search(r'\b' + re.escape(s['name']) + r'\b', text)]
    if used:
        text += '\n\nNamed labels:\n' + '\n'.join(f"{s['name']} = {label(s['hunk'], s['offset'])}" for s in used)
    return text, len(seen)


def callee_fn(hunk, off):
    return f'port_h{hunk}_{off:x}'


# --- The judge ------------------------------------------------------------------------------------

def judge_bin():
    target = os.environ.get('CARGO_TARGET_DIR') or os.path.join(ENGINE, 'target')
    return os.path.join(target, 'debug', 'fd-judge')


def run_judge(port_path, hunk, off, gen, out, survey=False):
    """Build fd-judge with `port_path` built in (the stub for a survey) and run it. One build and run
    at a time across processes: the port is compiled into the one binary. -> the judge's JSON, or
    {'outcome': 'compile'|'hang'|'crash', 'feedback': ...}."""
    env = dict(os.environ)
    env.pop('FD_PORT', None)
    if port_path:
        env['FD_PORT'] = os.path.abspath(port_path)
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, 'judge.lock'), 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        b = subprocess.run(['cargo', 'build', '-q', '-p', 'fd-conform', '--bin', 'fd-judge',
                            '--message-format', 'short'], cwd=ENGINE, env=env, capture_output=True, text=True)
        if b.returncode != 0:
            if not port_path:
                raise Usage('fd-judge does not build:\n' + b.stderr[-2000:])
            errs = [l for l in b.stderr.splitlines() if 'error' in l or l.startswith(' ')][:60]
            return {'outcome': 'compile', 'feedback': 'It did not compile:\n' + '\n'.join(errs)}
        argv = [judge_bin(), label(hunk, off), '--gen', gen, '--cases', str(CASES)] + (['--survey'] if survey else [])
        try:
            r = subprocess.run(argv, capture_output=True, text=True, timeout=JUDGE_SECONDS, env=env)
        except subprocess.TimeoutExpired:
            # The judge counts the original's hangs itself, so a run this long is the port looping.
            return {'outcome': 'hang', 'feedback': f'The judge stopped your port after {JUDGE_SECONDS} seconds: on at '
                    'least one state it never returns (an endless loop the original does not have). '
                    'Return the whole corrected file.'}
    if r.returncode == 2:
        raise Usage(r.stderr.strip())
    try:
        return json.loads(r.stdout.strip().splitlines()[-1])
    except (json.JSONDecodeError, IndexError):
        return {'outcome': 'crash', 'feedback': 'The judge crashed running your port:\n' + r.stderr[-1500:]}


def judge_answer(port_path, hunk, off, gen, out):
    j = run_judge(port_path, hunk, off, gen, out)
    if 'outcome' in j:
        return j
    if j['pass']:
        return {'outcome': 'pass', 'paths': j['paths']}
    valid = j['passed'] + j['failures']
    return {'outcome': 'fail', 'failures': j['failures'],
            'feedback': f"The judge ran your port against the original: it differs on {j['failures']} of "
                        f"{valid} states. The first:\n{j['first_failure']}\n\nReturn the whole corrected file."}


def survey(hunk, off, gen, out):
    j = run_judge(None, hunk, off, gen, out, survey=True)
    if 'outcome' in j:
        raise Usage(f"fd-judge's survey failed: {j.get('feedback', '')}")
    return j


def compose(out, answer, splits):
    """The file fd-judge builds: each split callee's passing port as a module (its own callees inside
    it), called as `port_hN_off`, then the answer."""
    parts = []
    for h, o in splits:
        d = os.path.join(out, slug(h, o))
        res = read_result(d)
        if not res or res.get('result') != 'pass':
            raise Usage(f'{label(h, o)} is a split callee with no passing port in {d}: translate it first')
        with open(os.path.join(d, 'port.rs'), encoding='utf-8') as f:
            inner = compose(out, f.read(), [resolve(c) for c in res.get('split_at', [])])
        mod = f'callee_h{h}_{o:x}'
        parts.append(f'pub mod {mod} {{\n#[allow(unused_imports)]\nuse super::*;\n{inner}\n}}\n'
                     f'#[allow(unused_imports)]\npub use {mod}::port as {callee_fn(h, o)};\n')
    return ''.join(parts) + answer


def read_result(d):
    try:
        with open(os.path.join(d, 'result.json'), encoding='utf-8') as f:
            return json.load(f)
    except (OSError, ValueError):
        return None
