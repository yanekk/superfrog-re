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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import judge_run  # noqa: E402
from judge_run import (  # noqa: E402  moved there by T17
    GAME, ENGINE, LISTING, SYMBOLS, INVENTORY, CASES, JUDGE_SECONDS, Usage, symbols, resolve, label, slug,
    load_listing, _CC, BRANCH_RE, flow_text, callee_fn, judge_bin, run_judge, judge_answer, survey, compose,
    read_result)

OUT = judge_run.JUDGE_DIR

MODEL = 'meta/muse-spark-1.3-contributor'
MAX_TOKENS = 96_000
# The spike's measured line (README, rounds 2 and 3): 24 of 24 passed up to 183 instructions, 2 of 6
# at 298 to 461. Larger routines are split at their callees first (T34).
MAX_INSTRUCTIONS = 180
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



# --- The routine and its listing ------------------------------------------------------------------

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
