#!/usr/bin/env python3
"""Translate one routine of the main program into a literal Rust port through Muse, judged on the
routine oracle (plans/forest-dumb-level-1 T33, DESIGN §4.3, §5.2, §5.3; docs/11-translation.md).

    python3 -I games/forest-dumb/tools/translate.py 'h0+$5462' --gen <name> [--tries 3] [--max-usd 0.10] [--out build/translate]

Prints one JSON line and nothing else on stdout:

    {"routine":"h0+$5462","result":"pass|fail|too-big|no-oracle|no-key","tries":n,"tokens_in":n,
     "tokens_out":n,"usd":x,"paths":n,"port":"<out>/h0-5462/port.rs" or null}

Exit 0 on `pass`, 1 on any other result, 2 on a usage error (an unknown routine, a split callee
with no passing port yet, a missing listing), with the reason on stderr.

Each answer is judged by `fd-judge` (fd-conform) over the generator's 2 000 states; a wrong one gets
the judge's first failing case (or the compiler's errors) back, up to --tries answers. A 429, 502 or
503 is waited out and is not a try; an answer with no code is. No call starts whose worst case (its
prompt and `max_tokens` of answer at the model's prices) would take the spend past --max-usd.
Every call appends a line to <out>/calls.jsonl. The listing, prompts, answers and ports derive from
the game's code: they stay under build/ and are never committed.

The routine is quoted, since `$5` is a shell parameter. It is `hN+$off` or a name from
re/symbols.toml. Python's standard library only, so it runs under `python3 -I`.
"""
import argparse, bisect, fcntl, json, os, re, subprocess, sys, time, tomllib, urllib.error, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = os.path.dirname(HERE)
ENGINE = os.path.join(GAME, 'engine')
LISTING = os.path.join(GAME, 'build', 're', 'forest_dumb_forever.lst')
SYMBOLS = os.path.join(GAME, 're', 'symbols.toml')
# T34's inventory: where a routine's `split_at` callees are named.
INVENTORY = os.path.join(GAME, 're', 'translate.toml')
OUT = os.path.join(GAME, 'build', 'translate')

MODEL = 'meta/muse-spark-1.3-contributor'
MAX_TOKENS = 96_000
# The spike's measured line (README, rounds 2 and 3): 24 of 24 passed up to 183 instructions, 2 of 6
# at 298 to 461. Larger routines are split at their callees first (T34).
MAX_INSTRUCTIONS = 180
CASES = 2000
JUDGE_SECONDS = 120
URL = 'https://openrouter.ai/api/v1'
# Used when OpenRouter's model list cannot be read: above what the spike paid ($0.20 a million out).
FALLBACK_PRICE = (0.5e-6, 1.0e-6)
# A busy provider or a rate limit is waited out, this long before each retry, then counts as a try.
WAITS = (20, 40, 80, 160, 320)
sleep = time.sleep  # a parameter of the module, so tests wait no time

SYSTEM = """You translate one routine of an Amiga game from its 68000 disassembly into Rust, exactly.

Write one Rust file. It is included in a module that already has `use super::*;`, which gives you:

    pub struct Cpu { pub d: [u32; 8], pub a: [u32; 8], pub mem: Mem }  // a[7] is the stack pointer
    impl Mem {
        pub fn rb(&self, addr: u32) -> u8;   pub fn rw(&self, addr: u32) -> u16;   pub fn rl(&self, addr: u32) -> u32;
        pub fn wb(&mut self, addr: u32, v: u8); pub fn ww(&mut self, addr: u32, v: u16); pub fn wl(&mut self, addr: u32, v: u32);
    }   // big-endian, addresses wrap to 24 bits, every address is readable
    pub fn h(hunk: usize, off: u32) -> u32;  // the address of hN+$off in the listing

The file must define `pub fn port(cpu: &mut Cpu)`: entered with the registers and memory the routine
is called with, it must leave d0-d7, a0-a6 and every memory byte exactly as the original does on return.
Condition codes are not compared. Bytes the routine pushes on its own stack are not compared.

How the judge checks: it runs the original routine on a 68000 emulator and your port from the same
generated states (registers random, game variables set to many values), about 2000 of them.

The listing: `0000abcd  <bytes>  <instruction>` lines, labels like `h0+$2014:`. An absolute operand
hN+$x is the address h(N, 0x x); a listing address 0000abcd is h0+$abcd only inside hunk 0.
Routines it calls (bsr/jsr) are in the listing too and must be translated as part of it. Constant
tables are in memory; read them through cpu.mem. Data may sit between instructions and be decoded as
nonsense instructions; a label the code writes to with move is a variable.

Mind the 68000's semantics: .b/.w operations on a data register change only its low byte/word;
operations on an address register are 32-bit (a .w source is sign-extended); mulu.w multiplies the
low words; lsr/lsl/asr/rol by a register count use it modulo 64; dbra counts the low word down to -1.
Use wrapping arithmetic: the build panics on overflow.

The hardware: $DFF000-$DFF1FF are the custom chips' registers and $BFxxxx the CIAs. Reach them
through cpu.mem like any other address, with the access size the instruction uses. The judge
compares every custom-chip write, in order, with the original's: a long write is two word writes,
high word first; a byte write reaches the register as that byte on both halves. A write to BLTSIZE
($DFF058) runs the blit at once; cpu.mem does it for you from the blitter registers written before.
A read of $DFF006 or $DFF007 (VHPOSR) returns a counter that steps by one on every byte read, so a
raster wait ends; any other hardware read returns what the judge set for that state, often 0. Make
the hardware reads and writes the original makes, in the same order and sizes: an instruction that
reads and writes (and.b, or.b, bset on a register) is a read, then a write. Labels named in the
listing (mt_counter and the like) are listed with their addresses after it.

Answer with the whole file in one ```rust block and nothing else after it."""


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


def split_at(hunk, off):
    """The callees T34's inventory splits this routine at, as (hunk, offset); none without one."""
    if not os.path.exists(INVENTORY):
        return []
    with open(INVENTORY, 'rb') as f:
        inv = tomllib.load(f)
    syms = symbols()
    for entries in inv.values():
        if not isinstance(entries, list):
            continue
        for e in entries:
            if isinstance(e, dict) and e.get('hunk') == hunk and e.get('offset') == off:
                return [resolve(c, syms) for c in e.get('split_at', [])]
    return []


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


def prompt(hunk, off, text, n, splits, syms=None):
    name = {(s['hunk'], s['offset']): s['name'] for s in (syms if syms is not None else symbols())}.get((hunk, off))
    head = f"Translate the routine at {label(hunk, off)}{f' ({name})' if name else ''} ({n} instructions with what it calls)."
    if splits:
        head += ('\n\nThese routines it calls are translated already and are not in the listing: '
                 'where the original calls one (bsr or jsr), call its function with the same cpu, '
                 'which leaves registers and memory as that routine does.\n' +
                 '\n'.join(f'    {label(h, o)}: {callee_fn(h, o)}(cpu)' for h, o in splits))
    return f'{head}\n\n```\n{text}\n```'


# --- The key, the model and its price -------------------------------------------------------------

def find_key():
    """-> (key or None, where it looked)."""
    zshrc = os.path.join(os.path.expanduser('~'), '.zshrc')
    looked = f'the environment variable OPENROUTER_API_KEY and an `export OPENROUTER_API_KEY=` line in {zshrc}'
    if os.environ.get('OPENROUTER_API_KEY', '').strip():
        return os.environ['OPENROUTER_API_KEY'].strip(), looked
    try:
        with open(zshrc, encoding='utf-8', errors='replace') as f:
            for l in f:
                m = re.match(r'\s*export\s+OPENROUTER_API_KEY=["\']?([^"\'\s]+)', l)
                if m:
                    return m.group(1), looked
    except OSError:
        pass
    return None, looked


def price(key):
    """(dollars per prompt token, per answer token) from OpenRouter's model list."""
    try:
        req = urllib.request.Request(f'{URL}/models', headers={'Authorization': f'Bearer {key}'})
        with urllib.request.urlopen(req, timeout=60) as r:
            for m in json.load(r)['data']:
                if m['id'] == MODEL:
                    p = m['pricing']
                    return float(p['prompt']), float(p['completion'])
    except (OSError, ValueError, KeyError):
        pass
    return FALLBACK_PRICE


class CallError(Exception):
    pass


def call_model(messages, key):
    """One answer: (text, usage) with usage's in, out, reasoning and cost. A 429, 502 or 503 is
    waited out (WAITS) and not returned; any other failure raises CallError."""
    body = {'model': MODEL, 'max_tokens': MAX_TOKENS, 'usage': {'include': True}, 'messages': messages}
    d = None
    for wait in WAITS + (None,):
        req = urllib.request.Request(f'{URL}/chat/completions', json.dumps(body).encode(),
                                     {'Authorization': f'Bearer {key}', 'Content-Type': 'application/json'})
        try:
            with urllib.request.urlopen(req, timeout=1200) as r:
                d = json.load(r)
        except urllib.error.HTTPError as e:
            msg = e.read().decode(errors='replace')[:400]
            if e.code in (429, 502, 503) and wait is not None:
                sleep(wait)
                continue
            raise CallError(f'HTTP {e.code}: {msg}')
        except (OSError, ValueError) as e:
            raise CallError(str(e)[:400])
        if 'choices' in d or wait is None:
            break
        sleep(wait)
    if not d or 'choices' not in d:
        raise CallError(json.dumps(d)[:400])
    text = d['choices'][0]['message'].get('content') or ''
    u = d.get('usage') or {}
    return text, {'in': u.get('prompt_tokens', 0), 'out': u.get('completion_tokens', 0),
                  'reasoning': (u.get('completion_tokens_details') or {}).get('reasoning_tokens', 0),
                  'cost': u.get('cost', 0.0) or 0.0,
                  'finish': d['choices'][0].get('finish_reason')}


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


# --- The service ----------------------------------------------------------------------------------

def translate(routine, gen, tries=3, max_usd=0.10, out=OUT):
    """-> the result line as a dict. Raises Usage."""
    hunk, off = resolve(routine)
    out = os.path.abspath(out)
    res = {'routine': label(hunk, off), 'result': None, 'tries': 0, 'tokens_in': 0, 'tokens_out': 0,
           'usd': 0.0, 'paths': 0, 'port': None}

    key, looked = find_key()
    if not key:
        print(f'translate: no OpenRouter key: looked in {looked}', file=sys.stderr)
        return dict(res, result='no-key')

    splits = split_at(hunk, off)
    text, n = flow_text(hunk, off, splits)
    if n > MAX_INSTRUCTIONS:
        print(f'translate: {label(hunk, off)} is {n} instructions with its callees, over {MAX_INSTRUCTIONS}: '
              'split it at its callees in re/translate.toml (`split_at`)', file=sys.stderr)
        return dict(res, result='too-big')
    d = os.path.join(out, slug(hunk, off))
    # A split routine is judged with its callees' passing ports: missing ones stop it here.
    compose(out, '', splits)

    s = survey(hunk, off, gen, out)
    res['paths'] = s['paths']
    if s['passed'] == 0:
        print(f"translate: the original returned on none of {s['cases']} states of `{gen}` "
              f"({s['hangs']} hang(s), {s['faults']} fault(s)): nothing to judge a port against", file=sys.stderr)
        return dict(res, result='no-oracle')
    if s.get('unmodelled_blits', 0):
        print(f"translate: on {s['unmodelled_blits']} of {s['passed']} states the original started a line or fill "
              "blit, which the routine oracle does not model: a port agreeing there proves nothing", file=sys.stderr)
        return dict(res, result='no-oracle')

    os.makedirs(d, exist_ok=True)
    for f in os.listdir(d):
        if re.fullmatch(r'(answer\d+\.md|try\d+\.rs|judged\d+\.rs|port\.rs|result\.json|prompt\.md)', f):
            os.remove(os.path.join(d, f))
    user = prompt(hunk, off, text, n, splits)
    with open(os.path.join(d, 'prompt.md'), 'w', encoding='utf-8') as f:
        f.write(user)
    per_in, per_out = price(key)
    messages = [{'role': 'system', 'content': SYSTEM}]
    log_path = os.path.join(out, 'calls.jsonl')

    def log(rec):
        with open(log_path, 'a', encoding='utf-8') as f:
            f.write(json.dumps(rec) + '\n')

    for n_try in range(1, tries + 1):
        messages.append({'role': 'user', 'content': user})
        # The worst case: the whole conversation at three characters a token, and a full answer.
        worst = sum(len(m['content']) for m in messages) / 3 * per_in + MAX_TOKENS * per_out
        if res['usd'] + worst > max_usd:
            res['stopped'] = 'max-usd'
            print(f"translate: stopped before try {n_try}: it could cost ${worst:.4f}, which would take "
                  f"${res['usd']:.4f} past --max-usd {max_usd}", file=sys.stderr)
            break
        res['tries'] = n_try
        t0 = time.time()
        rec = {'time': time.strftime('%Y-%m-%dT%H:%M:%S'), 'routine': res['routine'], 'gen': gen, 'model': MODEL,
               'try': n_try}
        try:
            answer, usage = call_model(messages, key)
        except CallError as e:
            # A lost try, as a broken answer is; the question stands for the next.
            messages.pop()
            log(dict(rec, outcome='error', error=str(e), secs=round(time.time() - t0)))
            continue
        messages.append({'role': 'assistant', 'content': answer})
        res['tokens_in'] += usage['in']
        res['tokens_out'] += usage['out']
        res['usd'] += usage['cost']
        rec.update(secs=round(time.time() - t0), tokens_in=usage['in'], tokens_out=usage['out'],
                   reasoning=usage['reasoning'], usd=usage['cost'], finish=usage.get('finish'))
        with open(os.path.join(d, f'answer{n_try}.md'), 'w', encoding='utf-8') as f:
            f.write(answer)
        blocks = re.findall(r'```rust\n(.*?)```', answer, re.S)
        if not blocks:
            log(dict(rec, outcome='no-code'))
            user = 'Your answer had no ```rust block. Answer with the whole file in one ```rust block.'
            continue
        port = os.path.join(d, f'try{n_try}.rs')
        with open(port, 'w', encoding='utf-8') as f:
            f.write(blocks[-1])
        judged = os.path.join(d, f'judged{n_try}.rs')
        with open(judged, 'w', encoding='utf-8') as f:
            f.write(compose(out, blocks[-1], splits))
        j = judge_answer(judged, hunk, off, gen, out)
        log(dict(rec, outcome=j['outcome'], failures=j.get('failures'), feedback=j.get('feedback', '')[:300]))
        if j['outcome'] == 'pass':
            final = os.path.join(d, 'port.rs')
            with open(final, 'w', encoding='utf-8') as f:
                f.write(blocks[-1])
            res['result'], res['port'] = 'pass', os.path.relpath(final)
            res['paths'] = j['paths']
            break
        user = j['feedback']
    if res['result'] is None:
        res['result'] = 'fail'
    res['usd'] = round(res['usd'], 6)
    with open(os.path.join(d, 'result.json'), 'w', encoding='utf-8') as f:
        json.dump(dict(res, gen=gen, split_at=[label(h, o) for h, o in splits]), f)
    return res


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('routine', help="`hN+$off` (quoted) or a name from re/symbols.toml")
    ap.add_argument('--gen', required=True, help='the generator registered in fd-conform/src/gen/')
    ap.add_argument('--tries', type=int, default=3)
    ap.add_argument('--max-usd', type=float, default=0.10)
    ap.add_argument('--out', default=OUT)
    a = ap.parse_args(argv)
    try:
        res = translate(a.routine, a.gen, a.tries, a.max_usd, a.out)
    except Usage as e:
        print(f'translate: {e}', file=sys.stderr)
        return 2
    print(json.dumps(res, separators=(',', ':')))
    return 0 if res['result'] == 'pass' else 1


if __name__ == '__main__':
    sys.exit(main())
