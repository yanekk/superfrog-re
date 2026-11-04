#!/usr/bin/env python3
"""Spike (2026-11-04, no plan): can Muse Spark 1.3 contributor (OpenRouter) translate Forest Dumb
routines as well as Opus, and at what token cost?

    python3 -I run.py [--models muse,opus] [--routines 5462,...] [--runs 3] [--tries 3] [--out DIR]

Each run: one prompt (the routine's listing lines and the port interface), the answer judged by
judge/ (the original on Moira against the port, ~2000 generated states), the judge's first failing
case or the compiler's errors sent back, up to --tries answers. Every call is a line in
<out>/calls.jsonl. The listing and every answer derive from the game's code: they stay under build/.
Spend guards: Muse stops starting calls past $1, Opus past $15 (its reported cost).
"""
import argparse, json, os, re, subprocess, sys, threading, time, urllib.request, uuid
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = os.path.dirname(os.path.dirname(HERE))
LISTING = os.path.join(GAME, 'build', 're', 'forest_dumb_forever.lst')
JUDGE = os.path.join(HERE, 'judge')
MUSE = 'meta/muse-spark-1.3-contributor'
OPUS = 'claude-opus-5-5'
CEILING = {'muse': 1.0, 'opus': 15.0}

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
Use wrapping arithmetic: the build panics on overflow.{hw}

Answer with the whole file in one ```rust block and nothing else after it."""


HW = """

The hardware: $DFF000-$DFF1FF are the custom chips' registers and $BFxxxx the CIAs. Reach them
through cpu.mem like any other address, with the access size the instruction uses. The judge
compares every custom-chip write, in order, with the original's: a long write is two word writes,
high word first; a byte write reaches the register as that byte on both halves. A write to BLTSIZE
($DFF058) runs the blit at once; cpu.mem does it for you from the blitter registers written before.
A read of $DFF006 or $DFF007 (VHPOSR) returns a counter that steps by one on every byte read, so a
raster wait ends; any other hardware read returns what the judge set for that state, often 0. Make
the hardware reads and writes the original makes, in the same order and sizes: an instruction that
reads and writes (and.b, or.b, bset on a register) is a read, then a write. Labels named in the
listing (mt_counter and the like) are listed with their addresses after it."""


SYS = [SYSTEM.replace('{hw}', '')]
MUSE_MAX = [32000]


def load_listing():
    lines = open(LISTING).read().splitlines()
    ins, at = {}, {}
    for i, l in enumerate(lines):
        m = re.match(r'^([0-9a-f]{8})  ([0-9a-f]+)\s+(.*)$', l)
        if m:
            a = int(m.group(1), 16)
            ins[a] = (m.group(3), len(m.group(2)) // 2)
            at[a] = i
    return lines, ins, at


def target(txt):
    m = re.search(r'h0\+\$([0-9a-f]+)', txt.split(None, 1)[1] if ' ' in txt else '')
    return int(m.group(1), 16) if m else None


def extent(ins, start):
    """The routine at start: (end, callees). It ends at an rts/bra/jmp past every forward branch."""
    pc, far, calls = start, start, set()
    while pc in ins:
        txt, sz = ins[pc]
        op = txt.split()[0]
        t = target(txt) if len(txt.split()) > 1 else None
        if op.startswith('bsr') or op == 'jsr':
            if t is not None:
                calls.add(t)
        elif (op.startswith('b') or op.startswith('db')) and t is not None and start < t < start + 0x1000:
            far = max(far, t)
        if op in ('rts', 'rte') or op.startswith('bra') or op == 'jmp':
            if pc >= far:
                return pc + sz, calls
        pc += sz
    raise SystemExit(f'routine at {start:x} runs off the listing')


def routine_text(routine):
    lines, ins, at = load_listing()
    todo, spans = [int(routine, 16)], []
    seen = set()
    while todo:
        a = todo.pop()
        if a in seen:
            continue
        seen.add(a)
        end, calls = extent(ins, a)
        spans.append((a, end))
        todo += list(calls)
    lo, hi = min(s for s, _ in spans), max(e for _, e in spans)
    first = at[lo]
    while first > 0 and not re.match(r'^[0-9a-f]{8} ', lines[first - 1]):
        first -= 1  # the label and xref lines above
    last = max(i for a, i in at.items() if a < hi)
    out = [l for l in lines[first:last + 1] if 'falls inside this instruction' not in l]
    return '\n'.join(out), len([a for a in ins if lo <= a < hi])


def flow_text(routine):
    """Every instruction reachable from the routine (fall-through, branches both ways, calls), as
    listing lines in address order; gaps marked. Then the named labels it uses, with addresses."""
    import tomllib
    lines, ins, at = load_listing()
    labels = {}
    for i, l in enumerate(lines):
        m = re.match(r'^(\S+):$', l)
        if m:
            j = i + 1
            while j < len(lines) and not re.match(r'^[0-9a-f]{8} ', lines[j]):
                j += 1
            if j < len(lines):
                labels[m.group(1)] = int(lines[j][:8], 16)
    addrs = sorted(ins)
    import bisect

    def containing(t):
        k = bisect.bisect_right(addrs, t) - 1
        return addrs[k] if k >= 0 and t < addrs[k] + ins[addrs[k]][1] else None

    hunk, off = routine.split('-') if '-' in routine else ('0', routine)
    syms = tomllib.load(open(os.path.join(GAME, 're', 'symbols.toml'), 'rb'))['sym']
    named = {(s['hunk'], s['offset']): s['name'] for s in syms}
    start = labels.get(f'h{hunk}+${off}') or labels[named[(int(hunk), int(off, 16))]]
    seen, todo = set(), [start]
    while todo:
        pc = todo.pop()
        while pc is not None and pc in ins and pc not in seen:
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
            if t is not None and (op.startswith('b') or op.startswith('db') or op == 'jsr'):
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
    syms = tomllib.load(open(os.path.join(GAME, 're', 'symbols.toml'), 'rb'))['sym']
    used = [s for s in syms if re.search(r'\b' + re.escape(s['name']) + r'\b', text)]
    if used:
        text += '\n\nNamed labels:\n' + '\n'.join(f"{s['name']} = h{s['hunk']}+${s['offset']:x}" for s in used)
    return text, len(seen)


def zsh_key():
    if os.environ.get('OPENROUTER_API_KEY'):
        return os.environ['OPENROUTER_API_KEY'].strip()
    for l in open(os.path.expanduser('~/.zshrc')):
        m = re.match(r'\s*export\s+OPENROUTER_API_KEY=["\']?([^"\'\s]+)', l)
        if m:
            return m.group(1)
    raise SystemExit('no OPENROUTER_API_KEY')


spent = {'muse': 0.0, 'opus': 0.0}
spent_lock = threading.Lock()
judge_lock = threading.Lock()
log_lock = threading.Lock()


def call_muse(state, user):
    state['messages'].append({'role': 'user', 'content': user})
    body = {'model': MUSE, 'max_tokens': MUSE_MAX[0], 'usage': {'include': True}, 'messages': state['messages']}
    req = urllib.request.Request('https://openrouter.ai/api/v1/chat/completions', json.dumps(body).encode(),
                                 {'Authorization': f'Bearer {state["key"]}', 'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=900) as r:
        d = json.load(r)
    if 'choices' not in d:
        raise RuntimeError(json.dumps(d)[:500])
    text = d['choices'][0]['message'].get('content') or ''
    state['messages'].append({'role': 'assistant', 'content': text})
    u = d.get('usage', {})
    det = u.get('completion_tokens_details') or {}
    return text, {'in': u.get('prompt_tokens', 0), 'out': u.get('completion_tokens', 0),
                  'reasoning': det.get('reasoning_tokens', 0), 'cache_read': (u.get('prompt_tokens_details') or {}).get('cached_tokens', 0),
                  'cost': u.get('cost', 0.0) or 0.0}


def call_opus(state, user):
    args = ['claude', '-p', '--model', OPUS, '--output-format', 'json', '--tools', '',
            '--system-prompt', SYS[0], '--setting-sources', '', '--strict-mcp-config']
    if state.get('session'):
        args += ['--resume', state['session']]
    else:
        state['session'] = str(uuid.uuid4())
        args += ['--session-id', state['session']]
    r = subprocess.run(args, input=user, capture_output=True, text=True, timeout=1800, cwd=state['cwd'])
    try:
        d = json.loads(r.stdout)
    except json.JSONDecodeError:
        raise RuntimeError((r.stdout + r.stderr)[-800:])
    u = d.get('usage', {})
    return d.get('result', ''), {'in': u.get('input_tokens', 0) + u.get('cache_creation_input_tokens', 0) + u.get('cache_read_input_tokens', 0),
                                 'out': u.get('output_tokens', 0), 'reasoning': None,
                                 'cache_read': u.get('cache_read_input_tokens', 0), 'cost': d.get('total_cost_usd', 0.0) or 0.0}


def judge(port_path, routine):
    with judge_lock:
        env = dict(os.environ, SPIKE_PORT=port_path)
        b = subprocess.run(['cargo', 'build', '--release', '--message-format', 'short'], cwd=JUDGE, env=env,
                           capture_output=True, text=True)
        if b.returncode != 0:
            errs = [l for l in b.stderr.splitlines() if 'error' in l or l.startswith(' ')][:60]
            return {'outcome': 'compile', 'feedback': 'It did not compile:\n' + '\n'.join(errs)}
        r = subprocess.run([os.path.join(JUDGE, 'target', 'release', 'fd-spike-judge'), routine],
                           capture_output=True, text=True, timeout=600)
    try:
        j = json.loads(r.stdout.strip().splitlines()[-1])
    except (json.JSONDecodeError, IndexError):
        return {'outcome': 'crash', 'feedback': 'The judge crashed running your port:\n' + r.stderr[-1500:]}
    if j['pass']:
        return {'outcome': 'pass', 'judge': j}
    return {'outcome': 'fail', 'judge': {k: v for k, v in j.items() if k != 'first_failure'},
            'feedback': f"The judge ran your port against the original: it differs on {j['failures']} of "
                        f"{j['valid']} states. The first:\n{j['first_failure']}\n\nReturn the whole corrected file."}


def one_run(model, routine, run, args, key, prompt, log):
    d = os.path.join(args.out, model, routine, f'run{run}')
    os.makedirs(d, exist_ok=True)
    state = {'messages': [{'role': 'system', 'content': SYS[0]}], 'key': key, 'cwd': d}
    user = prompt
    for n in range(1, args.tries + 1):
        with spent_lock:
            if spent[model] >= CEILING[model]:
                log({'model': model, 'routine': routine, 'run': run, 'try': n, 'outcome': 'budget'})
                return 'budget'
        t0 = time.time()
        try:
            text, usage = (call_muse if model == 'muse' else call_opus)(state, user)
        except Exception as e:  # a failed call is a lost try, as a broken answer is
            log({'model': model, 'routine': routine, 'run': run, 'try': n, 'outcome': 'error',
                 'error': str(e)[:400], 'secs': round(time.time() - t0)})
            if model == 'muse':
                state['messages'].pop()
            continue
        with spent_lock:
            spent[model] += usage['cost']
        open(os.path.join(d, f'answer{n}.md'), 'w').write(text)
        blocks = re.findall(r'```rust\n(.*?)```', text, re.S)
        rec = {'model': model, 'routine': routine, 'run': run, 'try': n, 'secs': round(time.time() - t0), **usage}
        if not blocks:
            rec.update(outcome='no-code')
            user = 'Your answer had no ```rust block. Answer with the whole file in one ```rust block.'
        else:
            port = os.path.join(d, f'try{n}.rs')
            open(port, 'w').write(blocks[-1])
            j = judge(port, routine)
            rec.update(outcome=j['outcome'], judge=j.get('judge'), feedback=j.get('feedback', '')[:300])
            user = j.get('feedback', '')
        log(rec)
        if rec['outcome'] == 'pass':
            return 'pass'
    return 'fail'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--models', default='muse,opus')
    ap.add_argument('--routines', default='5462,9754,24ce,2dd6,2fae')
    ap.add_argument('--runs', type=int, default=3)
    ap.add_argument('--tries', type=int, default=3)
    ap.add_argument('--parallel', type=int, default=8)
    ap.add_argument('--muse-max-tokens', type=int, default=32000)
    ap.add_argument('--hw', action='store_true', help='hardware routines: the hardware paragraph and flow extraction')
    ap.add_argument('--out', default=os.path.join(GAME, 'build', 'spike', 'muse-spark'))
    args = ap.parse_args()
    args.out = os.path.abspath(args.out)
    SYS[0] = SYSTEM.replace('{hw}', HW if args.hw else '')
    MUSE_MAX[0] = args.muse_max_tokens
    os.makedirs(args.out, exist_ok=True)
    key = zsh_key() if 'muse' in args.models else None
    calls = open(os.path.join(args.out, 'calls.jsonl'), 'a')

    def log(rec):
        with log_lock:
            calls.write(json.dumps(rec) + '\n')
            calls.flush()
            print(f"{rec['model']:4} {rec['routine']} run{rec['run']} try{rec['try']}: {rec['outcome']}"
                  f" in={rec.get('in')} out={rec.get('out')} ${rec.get('cost', 0):.4f} {rec.get('secs')}s", flush=True)

    prompts = {}
    for r in args.routines.split(','):
        text, n = (flow_text if args.hw else routine_text)(r)
        label = 'h{}+${}'.format(*r.split('-')) if '-' in r else f'h0+${r}'
        prompts[r] = f"Translate the routine at {label} ({n} instructions with what it calls).\n\n```\n{text}\n```"
        open(os.path.join(args.out, f'prompt-{r}.md'), 'w').write(prompts[r])
    # The judge built once before the clock starts, so the first try does not pay for the oracle.
    subprocess.run(['cargo', 'build', '--release', '-q'], cwd=JUDGE, check=True)
    jobs = [(m, r, k) for k in range(1, args.runs + 1) for r in args.routines.split(',') for m in args.models.split(',')]
    with ThreadPoolExecutor(args.parallel) as ex:
        for f in [ex.submit(one_run, m, r, k, args, key, prompts[r], log) for m, r, k in jobs]:
            f.result()
    print(json.dumps(spent))


if __name__ == '__main__':
    main()
