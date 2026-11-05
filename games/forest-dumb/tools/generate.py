#!/usr/bin/env python3
"""Have Muse write one routine's generator, judged on the original (plans/forest-dumb-level-1 T61,
DESIGN §4.3, §5.2, §5.3; docs/11-translation.md).

    python3 -I games/forest-dumb/tools/generate.py 'h0+$5462' --name <generator> [--tries 3] [--max-usd 0.10] [--out build/generate]

Prints one JSON line and nothing else on stdout:

    {"routine":"h0+$5462","result":"covered|partial|fail|no-oracle|no-key","tries":n,"usd":x,"branches":n,
     "uncovered":[{"at":"h0+$5470","never":"taken|not-taken|either","reason":".."}],"returns":n,
     "base":bool,"gen":"<out>/<name>.rs" or null}

Exit 0 on `covered`, 1 on any other result, 2 on a usage error with the reason on stderr.

Each answer is a generator module, written as the committed ones in fd-conform/src/gen/ are. It is
built into fd-judge (`FD_GEN`, fd-conform/build.rs) and judged by `fd-judge --cover` over its 2 000
states: it compiles and lists `<name>`; the original returns on at least 95% of them with no hang,
no fault past that and no line or fill blit; and every byte a state pokes (or makes a hardware
register read) is one the original read on some state, or on the rig's stack. A try failing any of
these gets the reason back. A valid one is then measured by `Rig::coverage`: a `Bcc`/`DBcc` of the
routine's listing (translate.flow_text, callees included) some way of which no state took is
uncovered, and goes back to Muse with its listing lines, up to --tries answers. Muse may answer that
a branch cannot be reached, with a reason: that is passed through to the reviewer and not asked
again. The result is the valid try with the fewest uncovered ways; `base` says it lays one, which the
reviewer reads (a base is shown, not judged).

The key, the model call and its waits, the spend guard and <out>/calls.jsonl are translate.py's. The
listing, prompts and answers derive from the game's code and stay under build/; the generator is code
written from the listing, as the hand-written ones are, and the builder commits it into
fd-conform/src/gen/ after reading it. Python's standard library only, so it runs under `python3 -I`.
"""
import argparse, fcntl, json, os, re, subprocess, sys, time, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import translate as tr  # noqa: E402

GAME = tr.GAME
ENGINE = tr.ENGINE
GEN_DIR = os.path.join(ENGINE, 'crates', 'fd-conform', 'src', 'gen')
OUT = os.path.join(GAME, 'build', 'generate')
# The judge binary is one for both services: translate.py's default lock serialises every build and
# run of it, whatever --out either was given.
LOCK = os.path.join(tr.OUT, 'judge.lock')
CASES = tr.CASES
RETURNS = 0.95
# A traced run of 2 000 states, with a state that runs to the rig's hang limit now and then.
JUDGE_SECONDS = 300
EXAMPLES = 3
UNREACHABLE_RE = re.compile(r'^\s*UNREACHABLE\s+(h[0-4]\+\$[0-9a-fA-F]+)(?:\s+(taken|not-taken|either))?\s*:\s*(.+?)\s*$', re.M)
# Bcc and DBcc, what Rig::coverage counts: not bra or bsr.
COND_RE = re.compile(rf'^(b({tr._CC})|db(ra|t|f|{tr._CC}))(\.[bsw])?$')

SYSTEM = """You write the test-state generator for one routine of an Amiga game, from its 68000 disassembly, in Rust.

A judge runs the original routine on a 68000 emulator from about 2000 states your generator makes, and
later checks a Rust port of the routine against it on the same states. A state is only as good as the
paths through the routine it drives: your goal is states that take every conditional branch of the
routine and of what it calls both ways, while staying states the game could be in.

Write one Rust file, a module shaped like the examples you are shown. It starts with

    use super::{Gen, Generator, Setup, h};

and must define `pub const ALL: &[(&str, Generator)]` listing your generator under the exact name you
are given. What you have:

    pub struct Generator { pub base: Option<fn(&mut [u8])>, pub case: fn(&mut Gen) -> Setup }
    pub struct Setup { pub d: [u32; 8], pub a: [u32; 8], pub pokes: Vec<(u32, Vec<u8>)>, pub hw_reads: Vec<(u32, u8)>, .. }
    impl Setup {
        pub fn random(g: &mut Gen) -> Setup;           // d0-d7, a0-a6 random (a even): start from it
        pub fn b(&mut self, a: u32, v: u8); pub fn w(&mut self, a: u32, v: u16); pub fn l(&mut self, a: u32, v: u32);
        pub fn put(&mut self, a: u32, bytes: Vec<u8>);  // big-endian bytes written before the call
    }
    impl Gen { pub fn next(&mut self) -> u64; pub fn below(&mut self, n: u64) -> u64;
               pub fn pick<T: Copy>(&mut self, choices: &[T]) -> T; pub fn one_in(&mut self, n: u64) -> bool; }
    pub const fn h(hunk: usize, off: u32) -> u32;      // the address of hN+$off in the listing

`case` makes one state; `base` (usually None) lays memory once before all states, for a screen or a
map the routine reads in bulk. `hw_reads` sets what a read of a hardware byte returns (else 0;
$DFF006/$DFF007 count up so raster waits end).

Rules the judge enforces:
- Poke only bytes the routine (or what it calls) reads on some state: its variables, tables, the
  structures its address registers point to. Never poke code, and never a byte only written.
  To point an address register at a structure, set the register and poke that structure's bytes.
- The original must return on at least 95% of the states: no endless loops, no pointer it follows
  off into nothing. A loop count comes from a register or a variable you set: keep it bounded.
- No line-mode or fill-mode blit (BLTCON1 bit 0, 3 or 4 set when BLTSIZE is written).
- Bias the values to where the branches turn: either side of each compare, zero and non-zero, sign,
  each case of a jump table, a loop run zero, one and several times.

The listing: `0000abcd  <bytes>  <instruction>` lines, labels like `h0+$2014:`. An operand hN+$x is the
address h(N, 0x x); a listing address 0000abcd is h0+$abcd only inside hunk 0.

You will be told which branch ways no state took. If a branch cannot be taken some way on any state
the game can be in, say so after the code block, one line per branch way:

    UNREACHABLE h0+$5470 taken: <why, from the listing>

(the way is `taken`, `not-taken` or `either`). That goes to the person who reviews your generator.

Answer with the whole file in one ```rust block, then any UNREACHABLE lines, and nothing else."""


Usage = tr.Usage


# --- What the prompt is made of -------------------------------------------------------------------

def inventory():
    if not os.path.exists(tr.INVENTORY):
        return []
    with open(tr.INVENTORY, 'rb') as f:
        return tomllib.load(f).get('routine', [])


def gen_files():
    """{file name: its text} of every committed generator module."""
    out = {}
    for f in sorted(os.listdir(GEN_DIR)):
        if f.endswith('.rs') and f != 'mod.rs':
            with open(os.path.join(GEN_DIR, f), encoding='utf-8') as fh:
                out[f] = fh.read()
    return out


def examples(hunk, off, name, files=None, inv=None):
    """Up to EXAMPLES committed generator modules to show Muse: those registering the most routines of
    this routine's subsystem in re/translate.toml, then the smallest, never one registering `name` or
    the routine's own label (it would be the answer)."""
    files = gen_files() if files is None else files
    inv = inventory() if inv is None else inv
    own = {f'"{name}"', f'"{tr.label(hunk, off)}"', f'"h{hunk}_{off:x}"'}
    allowed = {f: s for f, s in files.items() if not any(o in s for o in own)}
    sub = next((r.get('subsystem') for r in inv if r.get('hunk') == hunk and r.get('offset') == off), None)
    peers = [r['name'] for r in inv if sub and r.get('subsystem') == sub and r.get('name')]
    score = {f: sum(f'"{p}"' in s for p in peers) for f, s in allowed.items()}
    ranked = sorted(allowed, key=lambda f: (-score[f], len(allowed[f]), f))
    return [(f, allowed[f]) for f in ranked[:EXAMPLES]]


def cond_branches(text):
    """{listing address: line} of every Bcc and DBcc in a flow_text listing."""
    out = {}
    for l in text.splitlines():
        m = re.match(r'^([0-9a-f]{8})  [0-9a-f]+\s+(\S+)', l)
        if m and COND_RE.match(m.group(2)):
            out[int(m.group(1), 16)] = l
    return out


def listing_bases():
    """Each hunk's base in the listing (`;;; hunk N ... @ $base`): a listing address is base + offset."""
    bases = {}
    with open(tr.LISTING, encoding='utf-8') as f:
        for l in f:
            m = re.match(r'^;;; hunk (\d+)\b.*@ \$([0-9a-fA-F]+)', l)
            if m:
                bases[int(m.group(1))] = int(m.group(2), 16)
    return bases


def to_listing(at, bases):
    """`hN+$off` -> its listing address, or None for an address outside the program."""
    m = re.fullmatch(r'h([0-4])\+\$([0-9a-f]+)', at)
    return bases.get(int(m.group(1)), 0) + int(m.group(2), 16) if m else None


def to_label(addr, bases):
    """A listing address -> `hN+$off`, by the nearest base at or below it."""
    h, b = max(((h, b) for h, b in bases.items() if b <= addr), key=lambda x: x[1], default=(0, 0))
    return tr.label(h, addr - b)


def prompt(hunk, off, name, text, n, shown, syms=None):
    sym = {(s['hunk'], s['offset']): s['name'] for s in (syms if syms is not None else tr.symbols())}.get((hunk, off))
    ex = '\n\n'.join(f'Example, `{f}`:\n```rust\n{s}```' for f, s in shown)
    return (f"Write the generator `{name}` for the routine at {tr.label(hunk, off)}{f' ({sym})' if sym else ''} "
            f"({n} instructions with what it calls).\n\n"
            f"These are generators written for other routines of the game; follow their shape:\n\n{ex}\n\n"
            f"The routine:\n\n```\n{text}\n```")


# --- The judge ------------------------------------------------------------------------------------

def run_cover(path, hunk, off, name, out):
    """Build fd-judge with the generator at `path` built in and run `--cover`. -> its JSON, or
    {'outcome': 'compile'|'hang'|'crash'|'unlisted', 'feedback': ...}."""
    env = dict(os.environ)
    env.pop('FD_PORT', None)
    env['FD_GEN'] = os.path.abspath(path)
    os.makedirs(os.path.dirname(LOCK), exist_ok=True)
    with open(LOCK, 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        b = subprocess.run(['cargo', 'build', '-q', '-p', 'fd-conform', '--bin', 'fd-judge',
                            '--message-format', 'short'], cwd=ENGINE, env=env, capture_output=True, text=True)
        if b.returncode != 0:
            errs = [l for l in b.stderr.splitlines() if 'error' in l or l.startswith(' ')][:60]
            return {'outcome': 'compile', 'feedback': 'It did not compile:\n' + '\n'.join(errs)}
        argv = [tr.judge_bin(), tr.label(hunk, off), '--gen', name, '--cases', str(CASES), '--cover']
        try:
            r = subprocess.run(argv, capture_output=True, text=True, timeout=JUDGE_SECONDS, env=env)
        except subprocess.TimeoutExpired:
            return {'outcome': 'hang', 'feedback': f'The judge stopped after {JUDGE_SECONDS} seconds: your generator '
                    'never finishes making a state, or the original runs near its instruction limit on most of them. '
                    'Return the whole corrected file.'}
    if r.returncode == 2:
        if 'no generator' in r.stderr:
            return {'outcome': 'unlisted', 'feedback': f'Your `ALL` does not list a generator named `{name}`. '
                    'Return the whole corrected file.'}
        raise Usage(r.stderr.strip())
    try:
        return json.loads(r.stdout.strip().splitlines()[-1])
    except (json.JSONDecodeError, IndexError):
        return {'outcome': 'crash', 'feedback': 'The judge crashed making or running your states (a panic in '
                'your generator, say):\n' + r.stderr[-1500:] + '\n\nReturn the whole corrected file.'}


def judge(j):
    """A `--cover` line -> None when the states are valid, else {'outcome', 'feedback'}."""
    cases = j['cases']
    wrong = []
    if j['hangs']:
        wrong.append(f"the original never returned on {j['hangs']} state(s) (it ran past the rig's instruction "
                     "limit): no state may hang it")
    if j['passed'] < RETURNS * cases:
        wrong.append(f"the original returned on {j['passed']} of {cases} states ({j['faults']} took an exception, "
                     f"an address error from a pointer off into nothing, say); at least {int(RETURNS * cases)} must return")
    if j.get('unmodelled_blits'):
        wrong.append(f"on {j['unmodelled_blits']} state(s) the original started a line or fill blit, which the judge "
                     "does not model: keep BLTCON1 bits 0, 3 and 4 clear on every state")
    if j['outside']:
        wrong.append(f"{j['outside']} poked byte(s) are ones the original reads on no state; the first: "
                     f"{j['first_outside']}. Poke only what the routine reads")
    if not wrong:
        return None
    outcome = 'hang' if j['hangs'] else 'returns' if j['passed'] < RETURNS * cases else \
        'blit' if j.get('unmodelled_blits') else 'outside'
    return {'outcome': outcome, 'passed': j['passed'], 'unmodelled_blits': j.get('unmodelled_blits', 0),
            'feedback': 'The judge ran the original on your states: ' + '; '.join(wrong) +
                        '.\n\nReturn the whole corrected file.'}


def uncovered(j, listed, bases):
    """The branch ways no state took: every branch the rig counted with a count 0, and every Bcc/DBcc
    of the listing it never saw, as {'at', 'never'}, by address."""
    seen, out = set(), []
    for at, taken, not_taken in j['branches']:
        a = to_listing(at, bases)
        seen.add(a)
        if taken == 0 or not_taken == 0:
            never = 'either' if taken == not_taken == 0 else 'taken' if taken == 0 else 'not-taken'
            out.append({'at': at, 'never': never, '_addr': a})
    for a in listed:
        if a not in seen:
            out.append({'at': to_label(a, bases), 'never': 'either', '_addr': a})
    return sorted(out, key=lambda u: (u['_addr'] is None, u['_addr'] or 0))


def cover_feedback(un, listed, text):
    """The uncovered ways with their listing lines: each branch and the three lines before it."""
    lines = text.splitlines()
    pos = {l: i for i, l in enumerate(lines)}
    parts = []
    for u in un:
        way = {'taken': 'no state took it', 'not-taken': 'every state took it', 'either': 'no state reached it'}[u['never']]
        l = listed.get(u['_addr'])
        ctx = '\n'.join(lines[max(0, pos[l] - 3):pos[l] + 1]) if l in pos else ''
        parts.append(f"{u['at']}: {way}" + (f"\n```\n{ctx}\n```" if ctx else ''))
    return ('Your states are valid, but these branch ways are not covered:\n\n' + '\n\n'.join(parts) +
            '\n\nChange the states so they are, or say which cannot be reached and why (UNREACHABLE lines). '
            'Return the whole corrected file.')


# --- The service ----------------------------------------------------------------------------------

def generate(routine, name, tries=3, max_usd=0.10, out=OUT):
    """-> the result line as a dict. Raises Usage."""
    hunk, off = tr.resolve(routine)
    out = os.path.abspath(out)
    if not re.fullmatch(r'[A-Za-z0-9_+$]+', name):
        raise Usage(f'`{name}` is not a generator name (letters, digits, `_`, `+`, `$`)')
    res = {'routine': tr.label(hunk, off), 'result': None, 'tries': 0, 'usd': 0.0, 'branches': 0,
           'uncovered': [], 'returns': 0, 'base': False, 'gen': None}

    key, looked = tr.find_key()
    if not key:
        print(f'generate: no OpenRouter key: looked in {looked}', file=sys.stderr)
        return dict(res, result='no-key')

    text, n = tr.flow_text(hunk, off)
    listed = cond_branches(text)
    bases = listing_bases()
    d = os.path.join(out, re.sub(r'[^A-Za-z0-9_]', '_', name))
    os.makedirs(d, exist_ok=True)
    for f in os.listdir(d):
        if re.fullmatch(r'(answer\d+\.md|try\d+\.rs|result\.json|prompt\.md)', f):
            os.remove(os.path.join(d, f))
    user = prompt(hunk, off, name, text, n, examples(hunk, off, name))
    with open(os.path.join(d, 'prompt.md'), 'w', encoding='utf-8') as f:
        f.write(user)
    per_in, per_out = tr.price(key)
    messages = [{'role': 'system', 'content': SYSTEM}]
    log_path = os.path.join(out, 'calls.jsonl')
    best = None  # (uncovered ways, try, path, cover line, reasons)
    judged = []  # each judged try's outcome: whether the original could be judged at all
    reasons = {}  # (at, never) -> reason, from UNREACHABLE lines

    def log(rec):
        with open(log_path, 'a', encoding='utf-8') as f:
            f.write(json.dumps(rec) + '\n')

    for n_try in range(1, tries + 1):
        messages.append({'role': 'user', 'content': user})
        worst = sum(len(m['content']) for m in messages) / 3 * per_in + tr.MAX_TOKENS * per_out
        if res['usd'] + worst > max_usd:
            res['stopped'] = 'max-usd'
            print(f"generate: stopped before try {n_try}: it could cost ${worst:.4f}, which would take "
                  f"${res['usd']:.4f} past --max-usd {max_usd}", file=sys.stderr)
            break
        res['tries'] = n_try
        t0 = time.time()
        rec = {'time': time.strftime('%Y-%m-%dT%H:%M:%S'), 'routine': res['routine'], 'gen': name,
               'model': tr.MODEL, 'try': n_try, 'service': 'generate'}
        try:
            answer, usage = tr.call_model(messages, key)
        except tr.CallError as e:
            messages.pop()
            log(dict(rec, outcome='error', error=str(e), secs=round(time.time() - t0)))
            continue
        messages.append({'role': 'assistant', 'content': answer})
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
        path = os.path.join(d, f'try{n_try}.rs')
        with open(path, 'w', encoding='utf-8') as f:
            f.write(blocks[-1])
        claims = {(m.group(1).lower(), m.group(2) or 'either'): m.group(3)
                  for m in UNREACHABLE_RE.finditer(answer.split('```rust')[-1].split('```', 1)[-1])}
        j = run_cover(path, hunk, off, name, out)
        bad = j if 'outcome' in j else judge(j)
        if bad:
            judged.append(bad)
            log(dict(rec, outcome=bad['outcome'], feedback=bad['feedback'][:300]))
            user = bad['feedback']
            continue
        reasons.update(claims)
        un = uncovered(j, listed, bases)
        for u in un:
            r = reasons.get((u['at'].lower(), u['never'])) or reasons.get((u['at'].lower(), 'either'))
            if r:
                u['reason'] = r
        log(dict(rec, outcome='covered' if not un else 'partial', returns=j['passed'],
                 branches=len(j['branches']), uncovered=len(un)))
        if best is None or len(un) <= len(best[0]):
            best = (un, n_try, path, j)
        open_ways = [u for u in un if 'reason' not in u]
        if not open_ways:
            break
        user = cover_feedback(open_ways, listed, text)

    ran = [b for b in judged if 'passed' in b]
    if best is not None:
        un, n_try, path, j = best
        final = os.path.join(out, f'{name}.rs')
        with open(path, encoding='utf-8') as src, open(final, 'w', encoding='utf-8') as dst:
            dst.write(src.read())
        res.update(result='partial' if un else 'covered', gen=os.path.relpath(final),
                   branches=max(len(j['branches']), len(listed)), returns=j['passed'], base=j['base'],
                   uncovered=[{k: v for k, v in u.items() if k != '_addr'} for u in un])
    elif ran and all(b['passed'] == 0 or b['unmodelled_blits'] for b in ran):
        # Every state set that ran had the original return on none of its states, or start a blit the
        # oracle does not run: the routine itself may be out of the routine oracle's reach.
        print('generate: on every judged answer the original returned on no state or started a line or fill '
              'blit: the routine oracle may not run this routine', file=sys.stderr)
        res['result'] = 'no-oracle'
    else:
        res['result'] = 'fail'
    res['usd'] = round(res['usd'], 6)
    with open(os.path.join(d, 'result.json'), 'w', encoding='utf-8') as f:
        json.dump(dict(res, name=name), f)
    return res


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('routine', help="`hN+$off` (quoted) or a name from re/symbols.toml")
    ap.add_argument('--name', required=True, help='the name the generator is registered under')
    ap.add_argument('--tries', type=int, default=3)
    ap.add_argument('--max-usd', type=float, default=0.10)
    ap.add_argument('--out', default=OUT)
    a = ap.parse_args(argv)
    try:
        res = generate(a.routine, a.name, a.tries, a.max_usd, a.out)
    except Usage as e:
        print(f'generate: {e}', file=sys.stderr)
        return 2
    print(json.dumps(res, separators=(',', ':')))
    return 0 if res['result'] == 'covered' else 1


if __name__ == '__main__':
    sys.exit(main())
