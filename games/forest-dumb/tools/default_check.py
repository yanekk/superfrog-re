#!/usr/bin/env python3
"""Stage 7's check (translator DESIGN §2.12, T21): does a fresh session, given a porting-style task and
only the repo's docs, reach for the translator unprompted?

    python3 -I games/forest-dumb/tools/default_check.py [--runs 3] [--max-turns 15] [--minutes 10] [--routine <symbol>]

Each run makes a scratch worktree of HEAD (detached, under .claude/worktrees/), runs in it

    claude -p --verbose --output-format stream-json --max-turns N --strict-mcp-config --no-session-persistence
           --tools Read Grep Glob Bash Write Edit
           --allowedTools Read Grep Glob 'Bash(python3 -I games/forest-dumb/tools/*)'

on the default model with PROMPT, keeps the transcript in build/default-check/run-<n>.jsonl and removes
the worktree, pass or fail. Write and Edit are offered but not allowed: `-p` denies them, so a session
that sets out to write Rust by hand shows it in the transcript and changes nothing. Only HEAD is seen:
a doc fix must be committed before a rerun.

A run passes when the first tool call that produces Rust from the original runs tools/translate.py.
Producing Rust is: a Bash call naming translate.py (the pass), another tool that writes Rust
(RUST_TOOLS) or a shell redirect into a `.rs` file, or a Write or Edit of a `.rs` file. Reading the
docs or the listing first is fine. A session that never produces Rust in its turns fails. Prints one line:

    {"runs":3,"pass":3,"per_run":[{"run":1,"result":"pass|fail|timeout|no-model","first_tool":"...",
      "translate_at_turn":n,"first_rust":"...","turns":n,"secs":s,"usd":x}]}

`translate_at_turn` is the 1-based assistant turn of the first translate.py call, null when none.
Exit 0 when every run passes, 1 otherwise, 2 on usage. Seatbelts (DESIGN §5.2): runs, turns and the
wall-clock minutes a run are bounded here; macOS has no `timeout`. Standard library only, for `python3 -I`.
"""
import argparse, json, os, re, shutil, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = os.path.dirname(HERE)
REPO = os.path.dirname(os.path.dirname(GAME))
OUT = os.path.join(GAME, 'build', 'default-check')

PROMPT = ('Port {routine} of Forest Dumb Forever to Rust for a level plan. '
          "Start the way this repo's docs say.")
TOOLS = ['Read', 'Grep', 'Glob', 'Bash', 'Write', 'Edit']
ALLOWED = ['Read', 'Grep', 'Glob', 'Bash(python3 -I games/forest-dumb/tools/*)']
TRANSLATE = re.compile(r'(^|[\s/])translate\.py\b')
# Tools that write Rust of a routine other than by translating it: Haiku's draft rewrites and generators,
# and the translator's parts run by hand, which skip the judge.
RUST_TOOLS = re.compile(r'(^|[\s/])(reshape|generate|pcode_rust|translator_trial)\.py\b')
RS_REDIRECT = re.compile(r'(>>?|\btee(\s+-a)?)\s*[\'"]?[^\s\'";|&]+\.rs\b')
WRITERS = {'Write', 'Edit', 'MultiEdit', 'NotebookEdit'}
# A nested session inheriting these would take itself for a child of this one, or of pir.
ENV_DROP = re.compile(r'^(CLAUDECODE|CLAUDE_CODE_.*|CLAUDE_PID|CLAUDE_EFFORT|CLAUDE_AGENT_SDK_.*|PIR_.*)$')


def tool_calls(lines):
    """stream-json lines -> [(turn, name, input)] in order; a turn is one assistant message, 1-based."""
    calls, ids = [], []
    for line in lines:
        try:
            ev = json.loads(line)
        except ValueError:
            continue
        if not isinstance(ev, dict) or ev.get('type') != 'assistant':
            continue
        msg = ev.get('message') or {}
        mid = msg.get('id') or f'#{len(ids)}'
        if mid not in ids:
            ids.append(mid)
        for block in msg.get('content') or []:
            if isinstance(block, dict) and block.get('type') == 'tool_use':
                calls.append((ids.index(mid) + 1, block.get('name', ''), block.get('input') or {}))
    return calls


def describe(name, inp):
    arg = inp.get('command') or inp.get('file_path') or inp.get('pattern') or inp.get('path') or ''
    return f'{name} {arg}'.strip()[:200]


def rust_kind(name, inp):
    """'translate', 'other' or None: whether this call produces Rust from the original, and how."""
    if name == 'Bash':
        cmd = str(inp.get('command', ''))
        if TRANSLATE.search(cmd):
            return 'translate'
        if RUST_TOOLS.search(cmd) or RS_REDIRECT.search(cmd):
            return 'other'
        return None
    if name in WRITERS:
        path = str(inp.get('file_path') or inp.get('notebook_path') or '')
        return 'other' if path.endswith('.rs') else None
    return None


def judge(lines):
    """A transcript -> {'result', 'first_tool', 'translate_at_turn', 'first_rust', 'turns'}."""
    calls = tool_calls(lines)
    row = {'result': 'fail', 'first_tool': describe(*calls[0][1:]) if calls else None,
           'translate_at_turn': None, 'first_rust': None, 'turns': calls[-1][0] if calls else 0}
    for turn, name, inp in calls:
        kind = rust_kind(name, inp)
        if kind == 'translate' and row['translate_at_turn'] is None:
            row['translate_at_turn'] = turn
        if kind and row['first_rust'] is None:
            row['first_rust'] = describe(name, inp)
            row['result'] = 'pass' if kind == 'translate' else 'fail'
    for line in lines:  # the session's own count when it ended: turns without a tool call count too
        try:
            ev = json.loads(line)
        except ValueError:
            continue
        if isinstance(ev, dict) and ev.get('type') == 'result':
            row['turns'] = ev.get('num_turns', row['turns'])
            row['usd'] = ev.get('total_cost_usd', 0.0) or 0.0
    return row


def argv(prompt, max_turns):
    return (['claude', '-p', '--verbose', '--output-format', 'stream-json', '--max-turns', str(max_turns),
             '--strict-mcp-config', '--no-session-persistence', '--tools', *TOOLS, '--allowedTools', *ALLOWED,
             '--'] + [prompt])


def child_env():
    return {k: v for k, v in os.environ.items() if not ENV_DROP.match(k)}


def git(*args, cwd=REPO):
    return subprocess.run(['git', *args], cwd=cwd, capture_output=True, text=True, check=True).stdout


def one_run(n, args, *, repo=REPO, out=OUT, runner=subprocess.run):
    scratch = os.path.join(repo, '.claude', 'worktrees', f'default-check-{os.getpid()}-{n}')
    log = os.path.join(out, f'run-{n}.jsonl')
    os.makedirs(out, exist_ok=True)
    t0 = time.monotonic()
    row = None
    git('worktree', 'add', '--detach', '--quiet', scratch, 'HEAD', cwd=repo)
    try:
        with open(log, 'w', encoding='utf-8') as f:
            try:
                p = runner(argv(PROMPT.format(routine=args.routine), args.max_turns), cwd=scratch, stdout=f,
                           stderr=subprocess.PIPE, text=True, stdin=subprocess.DEVNULL, env=child_env(),
                           timeout=args.minutes * 60)
                err = p.stderr.strip()[:400] if p.returncode else None
            except subprocess.TimeoutExpired:
                row, err = {'result': 'timeout'}, None
            except OSError as e:
                row, err = {'result': 'no-model'}, str(e)[:400]
        with open(log, encoding='utf-8') as f:
            judged = judge(f.read().splitlines())
        if row is None:
            # A session stopped at its turn cap exits non-zero, and is judged like any other.
            row = judged
            if err and not judged['turns']:
                row = dict(judged, result='no-model', error=err)
        else:
            row = dict(judged, result=row['result'] if judged['result'] != 'pass' else 'pass',
                       **({'error': err} if err else {}))
    finally:
        subprocess.run(['git', 'worktree', 'remove', '--force', scratch], cwd=repo, capture_output=True)
        if os.path.isdir(scratch):
            shutil.rmtree(scratch, ignore_errors=True)
        subprocess.run(['git', 'worktree', 'prune'], cwd=repo, capture_output=True)
    return dict({'run': n}, **row, secs=round(time.monotonic() - t0, 1),
                transcript=os.path.relpath(log, repo))


def main(argv_=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--runs', type=int, default=3)
    ap.add_argument('--max-turns', type=int, default=15)
    ap.add_argument('--minutes', type=float, default=10)
    ap.add_argument('--routine', default='char_sprite_position')
    try:
        args = ap.parse_args(argv_)
    except SystemExit as e:
        return 2 if e.code else 0
    if args.runs < 1 or args.max_turns < 1 or args.minutes <= 0:
        print('default_check: --runs, --max-turns and --minutes must be positive', file=sys.stderr)
        return 2
    if shutil.which('claude') is None:
        print('default_check: no `claude` on PATH', file=sys.stderr)
        return 1
    per_run = []
    for n in range(1, args.runs + 1):
        per_run.append(one_run(n, args))
        print(json.dumps(per_run[-1]), file=sys.stderr, flush=True)
    passed = sum(r['result'] == 'pass' for r in per_run)
    print(json.dumps({'runs': args.runs, 'pass': passed, 'routine': args.routine, 'per_run': per_run}))
    return 0 if passed == args.runs else 1


if __name__ == '__main__':
    sys.exit(main())
