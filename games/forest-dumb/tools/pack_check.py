#!/usr/bin/env python3
"""Did the task packs pay? Per session of the run, from its conversation logs (owner, 2026-11-06;
DESIGN §4.4).

    python3 -I games/forest-dumb/tools/pack_check.py [--conversations DIR] [--since T42-implement-2] [--json]

A pack is re-sent on every step after it is read, so it saves only when the session then skips the
reads it replaces. For each session this prints:

- pack: the step that built it (`task_pack.py`) and the step that read it, or `-`;
- first edit: steps before the first Write/Edit, and the context then (tokens);
- re-read: reads after the pack of what the pack holds (the task doc, PROGRESS.md, FINDINGS.md,
  DESIGN.md), the waste to look for;
- steps, $ (the session's reported `costUSD`).

The baseline is the same columns for the sessions before the amendment: T08's builder took 62 steps
and ~230 000 tokens of context to its first edit. Python's standard library only.
"""
import argparse, glob, json, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
COVERED = re.compile(r'(?:tasks/T\d+-[\w-]+\.md|PROGRESS\.md|FINDINGS\.md|DESIGN\.md)')
EDIT = ('Write', 'Edit', 'NotebookEdit')


def conversations_dir():
    r = subprocess.run(['git', '-C', HERE, 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                       capture_output=True, text=True, timeout=30)
    main = os.path.dirname(r.stdout.strip()) if r.returncode == 0 else os.path.join(HERE, '..', '..', '..')
    return os.path.join(main, 'plans', 'forest-dumb-level-1', '.parallel', 'control', 'conversations')


def steps(path):
    """The session's assistant messages, merged by id: [{'ctx', 'acts': [(tool, text)]}], and its cost."""
    merged, order, usd = {}, [], 0.0
    with open(path, encoding='utf-8') as f:
        for line in f:
            try:
                e = json.loads(line).get('event') or {}
            except ValueError:
                continue
            if e.get('type') == 'result':
                usd += sum(u.get('costUSD', 0.0) for u in (e.get('model_usage_since_last_result') or {}).values())
            if e.get('type') != 'assistant':
                continue
            m = e.get('message') or {}
            mid = m.get('id')
            if mid not in merged:
                merged[mid] = {'ctx': 0, 'acts': []}
                order.append(mid)
            u = m.get('usage') or {}
            merged[mid]['ctx'] = (u.get('cache_read_input_tokens', 0) + u.get('cache_creation_input_tokens', 0)
                                  + u.get('input_tokens', 0))
            for c in m.get('content') or []:
                if c.get('type') == 'tool_use':
                    i = c.get('input') or {}
                    merged[mid]['acts'].append((c.get('name'), str(i.get('command') or i.get('file_path') or '')))
    return [merged[k] for k in order], usd


def reads(cmd):
    """A shell command that reads a file whole or in part for the session to see: `cat`, `head`,
    `less`, `sed -n`. Not an update that names the file (`sed -i`, a script writing it, `git add`,
    a `grep -c` count): every task ends by updating PROGRESS and FINDINGS, which is not a re-read
    (first run with packs, 2026-11-06)."""
    if re.search(r'\bsed\s+-i\b|\bgit\s+(add|commit)\b|open\([^)]*\)\.write|>\s*\S*(PROGRESS|FINDINGS|DESIGN)', cmd):
        return False
    return bool(re.search(r'\b(cat|head|less)\b|\bsed\s+-n\b', cmd))


def check(path):
    ss, usd = steps(path)
    built = read = first_edit = None
    rereads = []
    for n, s in enumerate(ss):
        for tool, text in s['acts']:
            if built is None and tool == 'Bash' and 'task_pack.py' in text:
                built = n
            if read is None and 'build/pack/' in text and (tool == 'Read' or re.search(r'\b(cat|sed|head)\b', text)):
                read = n
            elif read is not None and n > read and (tool == 'Read' or (tool == 'Bash' and reads(text))):
                rereads += [m for m in COVERED.findall(text)]
            if first_edit is None and tool in EDIT:
                first_edit = n
    return {'session': os.path.basename(path)[:-len('.ndjson')], 'steps': len(ss), 'usd': round(usd, 2),
            'pack_built': built, 'pack_read': read, 'first_edit': first_edit,
            'ctx_at_first_edit': ss[first_edit]['ctx'] if first_edit is not None else None,
            'reread': sorted(set(rereads))}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--conversations', default=None)
    ap.add_argument('--since', help='only sessions whose log is newer than this one (e.g. the last before the restart)')
    ap.add_argument('--json', action='store_true')
    a = ap.parse_args(argv)
    d = a.conversations or conversations_dir()
    files = sorted(glob.glob(os.path.join(d, 'T*.ndjson')), key=os.path.getmtime)
    if a.since:
        ref = os.path.join(d, a.since + '.ndjson')
        files = [f for f in files if os.path.getmtime(f) > os.path.getmtime(ref)]
    rows = [check(f) for f in files]
    if a.json:
        print(json.dumps(rows))
        return 0
    print(f"{'session':24} {'steps':>5} {'$':>6} {'pack':>9} {'1st edit':>8} {'ctx then':>9}  re-read")
    for r in rows:
        pack = '-' if r['pack_read'] is None else f"{r['pack_built']}/{r['pack_read']}"
        ctx = '-' if r['ctx_at_first_edit'] is None else f"{r['ctx_at_first_edit'] // 1000}k"
        fe = '-' if r['first_edit'] is None else r['first_edit']
        print(f"{r['session']:24} {r['steps']:>5} {r['usd']:>6.2f} {pack:>9} {fe:>8} {ctx:>9}  {', '.join(r['reread'])}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
