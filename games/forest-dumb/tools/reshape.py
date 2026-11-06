#!/usr/bin/env python3
"""Muse turns a literal port into engine code on typed state, judged by the task's own rig test, then
tidies it against an automatic quality check: the engine-shape trial (plans/forest-dumb-level-1 T62),
used by every porting task from T22 on (owner, 2026-11-06; DESIGN §4.3, §4.4, §5.3;
docs/11-translation.md, "Reshaping by Muse").

    python3 -I games/forest-dumb/tools/reshape.py 'h0+$4274' --task T08 [--tries 3] [--tidy 2] [--max-usd 0.10] [--out build/reshape]
    python3 -I games/forest-dumb/tools/reshape.py --task T08 --all [--only fn_a,fn_b] [...]
    python3 -I games/forest-dumb/tools/reshape.py --opus-cost [--conversations DIR]

Prints one JSON line a unit and nothing else on stdout:

    {"routine":"h0+$4274","routines":["h0_4274"],"fn":"rise_step","result":"pass|fail|no-key","tries":n,
     "tidy_rounds":n,"tokens_in":n,"tokens_out":n,"usd":x,"port":"<out>/h0-4274/port.rs" or null,
     "quality":["what the builder still has to tidy", ...] or null}

Exit 0 when every unit passes, 1 on any other result, 2 on a usage error (a routine the task has no
unit for, a missing literal port), with the reason on stderr.

Leave one out. A unit is one engine function of a task in re/reshape.toml and the routines its rig
test checks with it: one routine for most, four for T09's `follow` and `scroll`, which serve four each.
The prompt holds the unit's literal ports (T40's, from `build/translate`), the subsystem's typed
state as a skeleton (structs, enums, consts in full; every other function's doc and signature, no
body) with the unit's function and the helpers only it calls removed, and one other unit's engine
function, literal port and rig line as the example. The judge copies the engine workspace into
build/reshape/scratch, cuts the unit's functions out of the copy, appends the answer, and runs the task's
existing rig test there on that unit's routines only (10 000 states each, the test's own
generators). The committed `fd-core` is never changed. A wrong answer gets the rig's first failing
case back, one that does not compile the compiler's errors; up to --tries answers, under --max-usd,
as translate.py (whose call, retries and spend guard this uses).

Tidy. The prompt asks for the file's own style (docs naming the routine, names not numbers, no repeats).
A passing answer goes through quality(): no model call, it lists what still reads like a literal port,
plus the compiler's warnings on the answer's lines. Issues go back to Muse for up to --tidy rounds;
each tidied answer must pass the rig again. `port` is the passing answer with the fewest issues and
`quality` what is left of them, for the builder.

`--opus-cost` sums what T08's, T09's and T51's sessions cost, from the run's conversation logs, and
divides by the routines each ported. Python's standard library only, so it runs under `python3 -I`.
"""
import argparse, fcntl, glob, json, os, re, shutil, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import translate as tr  # noqa: E402  the model call, key, price and spend guard

GAME = os.path.dirname(HERE)
ENGINE = os.path.join(GAME, 'engine')
RE = os.path.join(GAME, 're')
REPO = os.path.dirname(os.path.dirname(GAME))
OUT = os.path.join(GAME, 'build', 'reshape')
# The judge's engine copy and its cargo target, whatever --out is: fd-tools finds the owner's zips
# through git from its crate, so the copy must sit inside the checkout (build/ is ignored).
SCRATCH = os.path.join(OUT, 'scratch')
CASES = 10_000
RUN_SECONDS = 300
BUILD_SECONDS = 900


def main_checkout():
    """`<git common dir>/..`: where T40's batch left its ports, whichever worktree this runs in."""
    try:
        r = subprocess.run(['git', '-C', HERE, 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                           capture_output=True, text=True, timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return os.path.dirname(r.stdout.strip()) if r.returncode == 0 else None


def port_dirs():
    """Where literal ports are looked for: this checkout's build/translate, then the main checkout's."""
    dirs = [tr.OUT]
    main = main_checkout()
    if main:
        dirs.append(os.path.join(main, 'games', 'forest-dumb', 'build', 'translate'))
    return dirs


def conversations_dir():
    main = main_checkout() or os.path.dirname(os.path.dirname(GAME))
    return os.path.join(main, 'plans', 'forest-dumb-level-1', '.parallel', 'control', 'conversations')


# --- The units --------------------------------------------------------------------------------------

def unit(fn, routines, helpers=()):
    """`fn` the engine function; `routines` (name, hunk 0 offset) its rig test checks with it;
    `helpers` the private functions only `fn` calls, removed with it."""
    return {'fn': fn, 'routines': list(routines), 'helpers': list(helpers)}


# Which helpers are exclusive was read off the source; test_reshape.py holds that nothing left in the
# file calls a removed one. `char_update`, `char_probe` and `camera_update` have no passing literal
# port (T40's table), so they are not units.
TASKS_FILE = os.path.join(RE, 'reshape.toml')


def rig_line(name, fn):
    """The routine table's line for rig name `name` in a task's rig test."""
    return rf'^\s*\("{name}",.*$'


def load_tasks(path=TASKS_FILE):
    """re/reshape.toml as {task: config}, each unit as unit() makes it."""
    import tomllib
    with open(path, 'rb') as f:
        raw = tomllib.load(f)
    tasks = {}
    for task, cfg in raw.items():
        cfg = dict(cfg)
        cfg['units'] = [unit(u['fn'], [(n, o) for n, o in u['routines']], u.get('helpers', ()))
                        for u in cfg['units']]
        cfg['rig_line'] = rig_line
        tasks[task] = cfg
    return tasks


TASKS = load_tasks()


class Usage(tr.Usage):
    pass


def find_unit(task, routine):
    """-> (task config, unit) for a routine given as `hN+$off` or its rig name."""
    if task not in TASKS:
        raise Usage(f'--task is one of {", ".join(TASKS)}')
    cfg = TASKS[task]
    m = re.fullmatch(r'h0\+\$([0-9a-fA-F]+)', routine)
    off = int(m.group(1), 16) if m else None
    for u in cfg['units']:
        for name, o in u['routines']:
            if o == off or name == routine:
                return cfg, u
    raise Usage(f'{routine} is not a {task} routine with a literal port and an engine function: '
                + ', '.join(f'h0+${o:x}' for u in cfg['units'] for _, o in u['routines']))


def slug(u):
    return f"h0-{u['routines'][0][1]:x}"


# --- Reading Rust source ----------------------------------------------------------------------------

def mask(src):
    """`src` with comments and string and char literals blanked, same length: braces and keywords
    found in it are code."""
    out, i, n = list(src), 0, len(src)

    def blank(a, b):
        for k in range(a, b):
            if out[k] != '\n':
                out[k] = ' '

    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            blank(i, j)
            i = j
        elif src.startswith('/*', i):
            j = src.find('*/', i + 2)
            j = n if j < 0 else j + 2
            blank(i, j)
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'":
            m = re.match(r"'(\\.|\\u\{[0-9a-fA-F]+\}|\\x[0-9a-fA-F]{2}|[^\\'])'", src[i:])
            if m:
                blank(i + 1, i + m.end() - 1)
                i += m.end()
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return ''.join(out)


ITEM = re.compile(r'(?m)^([ \t]*)(?:pub(?:\([a-z]+\))?\s+)?(?:const\s+fn|fn|struct|enum|const|static|type|impl|mod|use|trait)\b')
KIND = re.compile(r'(?:pub(?:\([a-z]+\))?\s+)?(const\s+fn|fn|struct|enum|const|static|type|impl|mod|use|trait)\b\s*(?:<[^>]*>\s*)?([A-Za-z_][A-Za-z0-9_]*)?')


def _end(m, at):
    """The end of the item starting at `at` in mask `m`: past its `;` or its matching `}`."""
    depth = 0
    for k in range(at, len(m)):
        c = m[k]
        if c in '([':
            depth += 1
        elif c in ')]':
            depth -= 1
        elif c == ';' and depth == 0:
            return k + 1, None
        elif c == '{' and depth == 0:
            d = 0
            for j in range(k, len(m)):
                if m[j] == '{':
                    d += 1
                elif m[j] == '}':
                    d -= 1
                    if d == 0:
                        return j + 1, k
    return len(m), None


def _docs_start(src, at):
    """Back from line start `at` over doc comments and attributes."""
    lines_before = src[:at].split('\n')[:-1]
    start = at
    for l in reversed(lines_before):
        s = l.strip()
        if s.startswith('///') or s.startswith('#[') or s.startswith('//') and not s.startswith('//!'):
            start -= len(l) + 1
        else:
            break
    return start


def items(src, m=None, lo=0, hi=None, indent=''):
    """The items directly in src[lo:hi] at `indent`: dicts with kind, name, start (with docs), head
    (where the body's `{` is, or None), end."""
    m = mask(src) if m is None else m
    hi = len(src) if hi is None else hi
    out, pos = [], lo
    while True:
        it = ITEM.search(m, pos, hi)
        if not it:
            return out
        if it.group(1) != indent:
            pos = it.end()
            continue
        k = KIND.match(m, it.start() + len(indent))
        if k.group(1) == 'use':
            # A `use` group's braces are not a body.
            end, head = m.index(';', it.start()) + 1, None
        else:
            end, head = _end(m, it.start())
        out.append({'kind': re.sub(r'\s+', ' ', k.group(1)), 'name': k.group(2), 'start': _docs_start(src, it.start()),
                    'at': it.start(), 'head': head, 'end': end, 'text_head': src[it.start():head] if head else None})
        pos = end


def fn_spans(src, names):
    """(start, end) of each fn called one of `names`, at any depth, docs included."""
    m = mask(src)
    spans = []
    for name in names:
        hits = [x for x in re.finditer(rf'(?m)^([ \t]*)(?:pub(?:\([a-z]+\))?\s+)?fn\s+{name}\b', m)]
        if len(hits) != 1:
            raise Usage(f'`fn {name}` is defined {len(hits)} times in the source, not once')
        end, _ = _end(m, hits[0].start())
        spans.append((_docs_start(src, hits[0].start()), end))
    return spans


def fn_text(src, name):
    (a, b), = fn_spans(src, [name])
    return src[a:b]


def skeleton(src, drop=()):
    """The file as the prompt shows it: `use`, structs, enums, consts and types in full; every fn as
    its docs and signature; impl blocks with their fns so; no module docs, no tests, no `Debug`
    impls; the fns in `drop` left out."""
    m = mask(src)
    parts = []

    def fn_sig(it):
        return src[it['start']:it['head']].rstrip() + ' { ... }'

    for it in items(src, m):
        kind = it['kind']
        if kind == 'mod' and '#[cfg(test)]' in src[it['start']:it['at']]:
            continue
        if kind == 'impl' and it['head'] is not None:
            head = src[it['start']:it['head']].rstrip()
            if re.search(r'\bfor\b', head):
                continue
            body = [fn_sig(f) for f in items(src, m, it['head'] + 1, it['end'] - 1, ' ' * 4)
                    if f['kind'] in ('fn', 'const fn') and f['name'] not in drop]
            parts.append(head + ' {\n' + '\n\n'.join(body) + '\n}')
        elif kind in ('fn', 'const fn'):
            if it['name'] not in drop:
                parts.append(fn_sig(it))
        else:
            parts.append(src[it['start']:it['end']].rstrip())
    return '\n\n'.join(parts) + '\n'


# --- The prompt -------------------------------------------------------------------------------------

def read(path):
    with open(path, encoding='utf-8') as f:
        return f.read()


def literal_port(name, off, dirs=None):
    """T40's passing literal port of `h0+$off`."""
    for d in dirs if dirs is not None else port_dirs():
        r = tr.read_result(os.path.join(d, f'h0-{off:x}'))
        if r and r.get('result') == 'pass':
            return read(os.path.join(d, f'h0-{off:x}', 'port.rs'))
    raise Usage(f'{name} (h0+${off:x}) has no passing literal port in ' + ' or '.join(dirs or port_dirs())
                + ': run translate.py on it first')


def rig_text(cfg, u, test_src):
    """What of the task's rig test is this unit's own: its routines' table lines and, for T09, the
    adapter function named like the engine's."""
    out = []
    for name, _ in u['routines']:
        out += [l.group(0) for l in re.finditer(cfg['rig_line'](name, u['fn']), test_src, re.M)]
    if cfg.get('adapter_fn'):
        try:
            out.append(fn_text(test_src, u['fn']))
        except Usage:
            pass
    return out


def example_unit(cfg, u):
    """The first of the task's examples that is not `u`."""
    fn = next(f for f in cfg['examples'] if f != u['fn'])
    return next(x for x in cfg['units'] if x['fn'] == fn)


def removed(u):
    return [u['fn']] + u['helpers']


SYSTEM = """You rewrite literal Rust ports of a 68000 Amiga game's routines as engine code on typed state.

A literal port works on `Cpu { d, a, mem }`: registers and memory at their original addresses. The
engine keeps the game's state in typed structs instead, each field named and at the original's
width; an address in the literal port is the variable LAYOUT names (hunk 0 offset), and the
struct field documented with that name or address. You get:

- the literal port of the routine (or the routines one engine function serves);
- the subsystem's typed state: its structs, enums and consts in full, and every other function as
  its doc and signature (the bodies are left out, but they exist and you may call them);
- an example: another routine's literal port, the engine function written for it, and how the rig
  test calls that function.

Write the engine function named in the request, with exactly the signature given, as a method in
an `impl` block of the type named, behaving exactly as the literal port does on every input: it is
judged against the original routine on a 68000 emulator over 10 000 generated states by the rig
test. Where the original calls a routine another function here ports, call that function. You may
add private helper functions, named so they do not clash with any name shown. Use wrapping
arithmetic: the build panics on overflow. No `unsafe`, no new crates, no I/O.

Write it as the file's own code is written, not as the literal port is: a person maintains it.
- Every function you write, the engine function and each helper, has a `///` doc comment: what it
  does in the game's terms, and which routine of the original it is (its label and `h0+$offset`).
- Use names, never numbers that have one: a variable is its struct field, a value the typed state
  already names is its const. A new number that means something gets its own `const` with a doc
  comment; only bit masks and shifts may stay bare.
- No register shuffling: keep a value in a local, and set `scratch` registers only where the rig
  compares them, as the example does. No `Cpu`, no memory reads or writes by address.
- Say a thing once. Mirror-image cases (left and right, up and down) are one helper taking the
  direction; no block of lines appears twice.
- Match the example's style and length: it was written by hand for this file.

Answer with one ```rust block holding the `impl` block (and any helpers), and nothing after it."""

TIDY = """It passes the rig: it behaves exactly as the original. Now make it read like the file's own code \
without changing what it does. Fix these:

{issues}

Keep the signature. Return the whole impl block (and any helpers) in one ```rust block."""

TIDY_FAILED = """\n\nThat was your tidied version, and it no longer behaves as your passing one did. Fix it so it \
passes again and stays tidy. Return the whole impl block."""


def prompt(task, u, root=ENGINE, dirs=None):
    """The user message for unit `u` of `task`, from the committed sources under `root`."""
    cfg = TASKS[task]
    src = read(os.path.join(root, cfg['src']))
    test_src = read(os.path.join(root, cfg['test']))
    ex = example_unit(cfg, u)
    sig = src[fn_spans(src, [u['fn']])[0][0]:].split('{', 1)[0]
    sig = '\n'.join(l for l in sig.split('\n') if not l.strip().startswith(('///', '#[', '//'))).strip()
    names = ', '.join(f'{n} (h0+${o:x})' for n, o in u['routines'])
    head = (f"Write `{sig}` in `{cfg['impl']} {{ ... }}`: the engine function for {names}.\n\n"
            f"{cfg['compared']}")
    parts = [head]
    for n, o in u['routines']:
        parts.append(f'The literal port of {n} (h0+${o:x}):\n\n```rust\n{literal_port(n, o, dirs)}```')
    state = []
    for path in cfg['state']:
        text = read(os.path.join(root, path))
        drop = removed(u) if path == cfg['src'] else ()
        state.append(f'// {path}\n{skeleton(text, drop)}')
    if cfg.get('layout'):
        lay = read(os.path.join(root, 'crates/fd-core/src/layout.rs'))
        state.append('// LAYOUT: name, hunk, offset, width\n'
                     + '\n'.join(l.strip() for l in lay.splitlines() if l.strip().startswith('var("')))
    parts.append('The typed state:\n\n```rust\n' + '\n\n'.join(state) + '```')
    en, eo = ex['routines'][0]
    top = {x['name']: x for x in items(test_src)}
    harness = '\n\n'.join(test_src[top[h]['start']:top[h]['end']] for h in cfg['harness'])
    parts.append(
        f"The example, {en} (h0+${eo:x}). Its literal port:\n\n```rust\n{literal_port(en, eo, dirs)}```\n\n"
        f"The engine function written for it:\n\n```rust\n{fn_text(src, ex['fn'])}```\n\n"
        f"The rig test's adapter, shared by every routine:\n\n```rust\n{harness}```\n\n"
        f"How the rig test calls the example:\n\n```rust\n" + '\n\n'.join(rig_text(cfg, ex, test_src)) + '\n```')
    return '\n\n'.join(parts) + '\n'


# --- The quality check ------------------------------------------------------------------------------

# Values that are masks or sign bits, never a name's.
MASKS = {0xff, 0xff00, 0xffff, 0xffff0000, 0xffffff00, 0xffffffff, 0x7f, 0x80, 0x7fff, 0x8000, 0x7fffffff,
         0x80000000, 0xf0, 0x0f, 0xf, 0xfff, 0xfffe, 0xfffffffe}
CONST = re.compile(r'(?m)^\s*(?:pub(?:\([a-z]+\))?\s+)?const\s+([A-Z][A-Z0-9_]*)\s*:\s*[^=]+=\s*(0x[0-9a-fA-F_]+|\d+)\s*;')
NUM = re.compile(r'\b(0x[0-9a-fA-F_]+|\d[\d_]*)(?:_?[iu](?:8|16|32|64|size))?\b')
# `h(0, NAME)` is how the engine loads a table's address into a scratch register; `h(0, 0x...)` is the
# literal port's addressing left in.
LEFTOVER = re.compile(r'\bcpu\.|\bmem\.[rw][bwl]\b|\bh\(\s*0\s*,\s*(?:0x|\d)')
REPEAT = 4  # lines


def number(text):
    t = text.replace('_', '')
    return int(t, 16) if t.lower().startswith('0x') else int(t)


def named_values(cfg, root=ENGINE):
    """{value: const name} for every const of the task's typed state."""
    names = {}
    for path in cfg['state']:
        p = os.path.join(root, path)
        if os.path.exists(p):
            for m in CONST.finditer(read(p)):
                names.setdefault(number(m.group(2)), m.group(1))
    return names


def quality(code, cfg, root=ENGINE):
    """What in a passing answer still reads like a literal port: a list of plain sentences, empty when
    it reads like the file's own code. Deterministic and free; the compiler's warnings come from the rig's
    build (run_rig's `lint`)."""
    issues = []
    m = mask(code)
    for hit in re.finditer(r'(?m)^[ \t]*(?:pub(?:\([a-z]+\))?\s+)?(?:const\s+)?fn\s+(\w+)', m):
        if '///' not in code[_docs_start(code, hit.start()):hit.start()]:
            issues.append(f'`fn {hit.group(1)}` has no `///` doc comment: say what it does in the game\'s terms '
                          'and which routine of the original it is (label, h0+$offset).')
    if LEFTOVER.search(m):
        issues.append('It still reads or writes memory by address (`cpu.`, `mem.rb`/`wb`/`rl`/`wl`, `h(0, 0x...)`): '
                      'work on the typed fields instead.')
    names = named_values(cfg, root)
    defined = {mm.group(1) for mm in CONST.finditer(m)}
    known, bare = {}, []
    code_lines = m.splitlines()
    for ln, line in enumerate(code_lines):
        if CONST.match(line):
            continue
        for lit in NUM.finditer(line):
            v = number(lit.group(1))
            if v < 16 or v in MASKS:
                continue
            if v in names and names[v] not in defined:
                known.setdefault(lit.group(1), names[v])
            elif lit.group(1).lower().startswith('0x') and v >= 0x100:
                if lit.group(1) not in bare:
                    bare.append(lit.group(1))
    for lit, name in known.items():
        issues.append(f'`{lit}` is `{name}` in the typed state: use the name.')
    if bare:
        issues.append('Bare numbers ' + ', '.join(f'`{b}`' for b in bare[:8]) + ': give each that means something '
                      'a named `const` with a doc comment, or use the field or const that holds it.')
    issues += repeats(code)
    return issues


def repeats(code):
    """A block of REPEAT code lines that appears twice, numbers and directions aside."""
    def norm(l):
        l = re.sub(r'\b(0x[0-9a-fA-F_]+|\d+)\b', 'N', l.strip())
        return re.sub(r'(?i)left|right|up|down|neg|pos', 'D', l)
    lines = [norm(l) for l in mask(code).splitlines()]
    lines = [l for l in lines if l and not re.fullmatch(r'[{}();,\s]*|\} else \{|else \{', l)]
    seen, found = {}, []
    for i in range(len(lines) - REPEAT + 1):
        key = tuple(lines[i:i + REPEAT])
        if key in seen and seen[key] + REPEAT <= i:
            if not any(abs(i - j) < REPEAT for j in found):
                found.append(i)
        else:
            seen.setdefault(key, i)
    return [f'These lines appear twice (directions and numbers aside): `{lines[i]}` ...: say it once, as a '
            'helper taking what differs.' for i in found[:3]]


# --- The judge --------------------------------------------------------------------------------------

def absolute_paths(manifest, crate_dir, engine):
    """A copied Cargo.toml with every path dependency outside the engine made absolute."""
    def fix(mt):
        p = os.path.normpath(os.path.join(crate_dir, mt.group(2)))
        inside = os.path.commonpath([p, engine]) == engine
        return mt.group(0) if inside else f'{mt.group(1)}"{p}"'
    return re.sub(r'(path\s*=\s*)"([^"]+)"', fix, manifest)


def scratch(task, u, answer, root=ENGINE):
    """A copy of the engine under SCRATCH with the unit's functions cut out of its source, the
    answer appended, and the rig test's loop limited to the routines in RESHAPE_ONLY. -> its folder."""
    cfg = TASKS[task]
    base = SCRATCH
    eng = os.path.join(base, 'engine')
    if os.path.isdir(eng):
        shutil.rmtree(eng)
    # copy2 keeps mtimes, so cargo rebuilds only what the answer changed.
    shutil.copytree(root, eng, ignore=shutil.ignore_patterns('target'), copy_function=shutil.copy2)
    # The tests read `../../../re` from their crate: games/forest-dumb/re.
    link = os.path.join(base, 're')
    if not os.path.exists(link):
        os.symlink(RE, link)
    # fd-tools' build tag (T14) hashes the toolkit's crates at `<workspace>/../../../toolkit`: from the
    # copy that is build/toolkit, so it points there to the checkout's toolkit.
    kit = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(eng))), 'toolkit')
    if not os.path.exists(kit):
        os.symlink(os.path.join(REPO, 'toolkit'), kit)
    for mf in glob.glob(os.path.join(eng, 'crates', '*', 'Cargo.toml')):
        crate = os.path.join(root, os.path.relpath(os.path.dirname(mf), eng))
        text = read(mf)
        fixed = absolute_paths(text, crate, os.path.abspath(root))
        if fixed != text:
            with open(mf, 'w', encoding='utf-8') as f:
                f.write(fixed)
    src_path = os.path.join(eng, cfg['src'])
    src = read(src_path)
    for a, b in sorted(fn_spans(src, removed(u)), reverse=True):
        src = src[:a] + src[b:]
    with open(src_path, 'w', encoding='utf-8') as f:
        f.write(src.rstrip('\n') + '\n\n' + MARK + '\n' + answer.rstrip('\n') + '\n')
    test_path = os.path.join(eng, cfg['test'])
    test = read(test_path)
    if test.count(cfg['loop']) != 1:
        raise Usage(f"{cfg['test']}: the rig loop `{cfg['loop']}` is not there once: update TASKS")
    test = test.replace(cfg['loop'], cfg['loop'] + '\n        if std::env::var("RESHAPE_ONLY")'
                        '.is_ok_and(|o| !o.split(\',\').any(|x| x == name)) { continue; }', 1)
    with open(test_path, 'w', encoding='utf-8') as f:
        f.write(test)
    return eng


MARK = "// The trial's answer (tools/reshape.py)."


def warnings(cfg, eng, stderr):
    """The compiler's warnings on the answer's own lines, as quality issues, numbered from the answer's
    first line."""
    try:
        lines = read(os.path.join(eng, cfg['src'])).splitlines()
        start = lines.index(MARK) + 1
    except (OSError, ValueError):
        return []
    out = []
    for m in re.finditer(r'(?m)^(\S+?):(\d+):\d+: warning: (.+)$', stderr):
        if m.group(1).endswith(cfg['src']) and int(m.group(2)) > start:
            out.append(f'The compiler warns at line {int(m.group(2)) - start} of your answer: {m.group(3)}')
    return out


def run_rig(task, u, answer, root=ENGINE):
    """Build the scratch copy and run the unit's routines on the task's rig test. -> {'outcome':
    'pass'|'fail'|'compile'|'hang', 'feedback', 'paths'}."""
    cfg = TASKS[task]
    os.makedirs(SCRATCH, exist_ok=True)
    env = dict(os.environ, CARGO_TARGET_DIR=os.path.join(SCRATCH, 'target'), NO_COLOR='1',
               RESHAPE_ONLY=','.join(n for n, _ in u['routines']))
    test = os.path.splitext(os.path.basename(cfg['test']))[0]
    with open(os.path.join(SCRATCH, 'judge.lock'), 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        eng = scratch(task, u, answer, root)
        b = subprocess.run(['cargo', 'test', '-q', '-p', 'fd-conform', '--test', test, '--no-run',
                            '--message-format', 'short'], cwd=eng, env=env, capture_output=True, text=True,
                           timeout=BUILD_SECONDS)
        if b.returncode != 0:
            errs = [l for l in b.stderr.splitlines() if 'error' in l or l.startswith(' ')][:60]
            return {'outcome': 'compile', 'feedback': 'It did not compile:\n' + '\n'.join(errs)
                    + '\n\nReturn the whole corrected impl block.'}
        try:
            r = subprocess.run(['cargo', 'test', '-q', '-p', 'fd-conform', '--test', test, '--', cfg['test_fn'],
                                '--exact', '--nocapture'], cwd=eng, env=env, capture_output=True, text=True,
                               timeout=RUN_SECONDS)
        except subprocess.TimeoutExpired:
            return {'outcome': 'hang', 'feedback': f'The rig stopped after {RUN_SECONDS} seconds: on at least one '
                    'state your function never returns. Return the whole corrected impl block.'}
    text = r.stdout + r.stderr
    # T08's test prints `paths: name N (h hangs), ...`, T09's `name: N paths`.
    ran = {a or c: int(b or d) for a, b, c, d in
           re.findall(r'\b(\w+) (\d+) \(\d+ hangs\)|\b(\w+): (\d+) paths', text)}
    paths = sum(ran.values())
    if r.returncode == 0:
        # A routine name RESHAPE_ONLY matches nowhere skips it and the test passes with nothing
        # compared: a pass counts only when the rig reports every routine of the unit.
        missing = [n for n, _ in u['routines'] if n not in ran]
        if missing:
            raise Usage(f"{cfg['test']} passed without running {', '.join(missing)}: the name is not in "
                        f"its routine table, or the test no longer prints its paths: update TASKS")
        return {'outcome': 'pass', 'paths': paths, 'lint': warnings(cfg, eng, b.stderr)}
    m = re.search(r"panicked at [^\n]*\n(.*?)(?:\nnote: run with|\n\nfailures:|\Z)", text, re.S)
    why = (m.group(1) if m else text[-2500:]).strip()[:3000]
    return {'outcome': 'fail', 'feedback': 'The rig ran your function against the original and it differs:\n'
            f'{why}\n\nReturn the whole corrected impl block.'}


# --- The service ------------------------------------------------------------------------------------

def reshape(routine, task, tries=3, max_usd=0.10, out=OUT, dirs=None, tidy=2):
    """-> the result line as a dict. Raises Usage.

    Up to `tries` answers until one passes the rig; then up to `tidy` rounds in which the quality
    check's issues go back to Muse and each tidied answer must pass the rig again. `port` is the
    passing answer with the fewest issues, `quality` what is left of them for the builder."""
    cfg, u = find_unit(task, routine)
    out = os.path.abspath(out)
    first = u['routines'][0][1]
    res = {'routine': f'h0+${first:x}', 'routines': [n for n, _ in u['routines']], 'fn': u['fn'],
           'result': None, 'tries': 0, 'tidy_rounds': 0, 'tokens_in': 0, 'tokens_out': 0, 'usd': 0.0,
           'paths': 0, 'port': None, 'quality': None}
    key, looked = tr.find_key()
    if not key:
        print(f'reshape: no OpenRouter key: looked in {looked}', file=sys.stderr)
        return dict(res, result='no-key')
    user = prompt(task, u, dirs=dirs)
    d = os.path.join(out, slug(u))
    os.makedirs(d, exist_ok=True)
    for f in os.listdir(d):
        if re.fullmatch(r'(answer\d+\.md|try\d+\.rs|port\.rs|result\.json|prompt\.md)', f):
            os.remove(os.path.join(d, f))
    with open(os.path.join(d, 'prompt.md'), 'w', encoding='utf-8') as f:
        f.write(user)
    per_in, per_out = tr.price(key)
    messages = [{'role': 'system', 'content': SYSTEM}]
    log_path = os.path.join(out, 'calls.jsonl')
    best = None  # (issues, code, paths) of the passing answer with the fewest issues
    n_call = 0

    def log(rec):
        with open(log_path, 'a', encoding='utf-8') as f:
            f.write(json.dumps(rec) + '\n')

    while True:
        if best is None and res['tries'] >= tries:
            break
        if best is not None and (not best[0] or res['tidy_rounds'] >= tidy):
            break
        messages.append({'role': 'user', 'content': user})
        worst = sum(len(m['content']) for m in messages) / 3 * per_in + tr.MAX_TOKENS * per_out
        if res['usd'] + worst > max_usd:
            messages.pop()
            res['stopped'] = 'max-usd'
            print(f"reshape: stopped before call {n_call + 1}: it could cost ${worst:.4f}, which would take "
                  f"${res['usd']:.4f} past --max-usd {max_usd}", file=sys.stderr)
            break
        tidying = best is not None
        if tidying:
            res['tidy_rounds'] += 1
        else:
            res['tries'] += 1
        n_call += 1
        t0 = time.time()
        rec = {'time': time.strftime('%Y-%m-%dT%H:%M:%S'), 'routine': res['routine'], 'task': task,
               'fn': u['fn'], 'model': tr.MODEL, 'try': n_call, 'tidy': tidying}
        try:
            answer, usage = tr.call_model(messages, key)
        except tr.CallError as e:
            messages.pop()
            log(dict(rec, outcome='error', error=str(e), secs=round(time.time() - t0)))
            continue
        messages.append({'role': 'assistant', 'content': answer})
        res['tokens_in'] += usage['in']
        res['tokens_out'] += usage['out']
        res['usd'] += usage['cost']
        rec.update(secs=round(time.time() - t0), tokens_in=usage['in'], tokens_out=usage['out'],
                   reasoning=usage['reasoning'], usd=usage['cost'], finish=usage.get('finish'))
        with open(os.path.join(d, f'answer{n_call}.md'), 'w', encoding='utf-8') as f:
            f.write(answer)
        blocks = re.findall(r'```rust\n(.*?)```', answer, re.S)
        if not blocks:
            log(dict(rec, outcome='no-code'))
            user = 'Your answer had no ```rust block. Answer with the whole impl block in one ```rust block.'
            continue
        code = blocks[-1]
        with open(os.path.join(d, f'try{n_call}.rs'), 'w', encoding='utf-8') as f:
            f.write(code)
        j = run_rig(task, u, code)
        if j['outcome'] == 'pass':
            issues = quality(code, cfg) + j.get('lint', [])
            log(dict(rec, outcome='pass', issues=len(issues)))
            if best is None or len(issues) < len(best[0]):
                best = (issues, code, j['paths'])
            user = TIDY.format(issues='\n'.join(f'- {i}' for i in best[0]))
            # The next round tidies the best version, whichever answer that was.
            if code != best[1]:
                user = ('That version has more issues than your earlier passing one, shown here; tidy that one:\n\n'
                        f'```rust\n{best[1]}```\n\n' + user)
        else:
            log(dict(rec, outcome=j['outcome'], feedback=j.get('feedback', '')[:300]))
            user = j['feedback'] + (TIDY_FAILED if tidying else '')
    if best is not None:
        final = os.path.join(d, 'port.rs')
        with open(final, 'w', encoding='utf-8') as f:
            f.write(best[1])
        res.update(result='pass', port=os.path.relpath(final), paths=best[2], quality=best[0])
    else:
        res['result'] = 'fail'
    res['usd'] = round(res['usd'], 6)
    with open(os.path.join(d, 'result.json'), 'w', encoding='utf-8') as f:
        json.dump(dict(res, task=task), f)
    return res


def reshape_all(task, tries=3, max_usd=0.10, out=OUT, tidy=2, only=None):
    """reshape() every unit of `task` (or those whose fn is in `only`), one after another. -> results."""
    if task not in TASKS:
        raise Usage(f'--task is one of {", ".join(TASKS)}')
    results = []
    for u in TASKS[task]['units']:
        if only and u['fn'] not in only:
            continue
        results.append(reshape(f"h0+${u['routines'][0][1]:x}", task, tries, max_usd, out, tidy=tidy))
    return results


# --- Opus's cost ------------------------------------------------------------------------------------

# Routines each task ported to engine code and proved on the rig: the tables of
# character_routines.rs (14), camera_routines.rs (10) and character_anim_routines.rs (6). T51 was
# split out of T08's character work before T08 started (its row `blocks T08`).
PORTED_BY = {'T08': 14, 'T09': 10, 'T51': 6}


def opus_cost(conv=None):
    """Each task's sessions (implement and review, restarts included) from the run's conversation
    logs: the summed `costUSD` of every `result` event's `model_usage_since_last_result`. -> {task:
    {'sessions', 'usd', 'routines', 'usd_per_routine'}}."""
    conv = conv or conversations_dir()
    out = {}
    for task, n in PORTED_BY.items():
        files = sorted(glob.glob(os.path.join(conv, f'{task}-*.ndjson')))
        usd = 0.0
        for path in files:
            with open(path, encoding='utf-8') as f:
                for line in f:
                    try:
                        e = json.loads(line).get('event') or {}
                    except ValueError:
                        continue
                    if e.get('type') == 'result':
                        usd += sum(u.get('costUSD', 0.0) for u in (e.get('model_usage_since_last_result') or {}).values())
        out[task] = {'sessions': [os.path.basename(p)[:-len('.ndjson')] for p in files], 'usd': round(usd, 2),
                     'routines': n, 'usd_per_routine': round(usd / n, 3)}
    return out


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('routine', nargs='?', help="`h0+$off` (quoted) or the routine's rig name")
    ap.add_argument('--task', choices=sorted(TASKS))
    ap.add_argument('--all', action='store_true', help="every unit of --task, one result line each")
    ap.add_argument('--only', help='with --all: comma-separated engine fns')
    ap.add_argument('--tidy', type=int, default=2, help='rounds of the quality check sent back after a pass')
    ap.add_argument('--tries', type=int, default=3)
    ap.add_argument('--max-usd', type=float, default=0.10)
    ap.add_argument('--out', default=OUT)
    ap.add_argument('--opus-cost', action='store_true', help="T08's, T09's and T51's session cost per routine")
    ap.add_argument('--conversations', help='the run\'s conversation logs (default: the main checkout\'s)')
    a = ap.parse_args(argv)
    if a.opus_cost:
        print(json.dumps(opus_cost(a.conversations), separators=(',', ':')))
        return 0
    if a.all and a.task:
        try:
            results = reshape_all(a.task, a.tries, a.max_usd, a.out, a.tidy,
                                  set(a.only.split(',')) if a.only else None)
        except tr.Usage as e:
            print(f'reshape: {e}', file=sys.stderr)
            return 2
        for res in results:
            print(json.dumps(res, separators=(',', ':')))
        return 0 if all(res['result'] == 'pass' for res in results) else 1
    if not a.routine or not a.task:
        ap.error('a routine and --task (or --task and --all) are needed')
    try:
        res = reshape(a.routine, a.task, a.tries, a.max_usd, a.out, tidy=a.tidy)
    except tr.Usage as e:
        print(f'reshape: {e}', file=sys.stderr)
        return 2
    print(json.dumps(res, separators=(',', ':')))
    return 0 if res['result'] == 'pass' else 1


if __name__ == '__main__':
    sys.exit(main())
