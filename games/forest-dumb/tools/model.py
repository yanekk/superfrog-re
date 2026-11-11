#!/usr/bin/env python3
"""One question to Haiku through the owner's Claude subscription (translator DESIGN §2.13, §5.3).

    import model
    r = model.ask(SYSTEM, prompt, log='<out>/calls.jsonl')
    # {"result": "ok"|"no-model"|"timeout"|"max-calls", "text": str, "secs": float,
    #  "tokens_in": n, "tokens_out": n, "usd": x}

Runs `claude -p --model haiku --tools "" --output-format json --max-turns 1 --system-prompt <system>`
with the prompt on stdin. `no-model` is anything that is not an answer: no `claude` on PATH, a non-zero
exit, an error result or output that is not its JSON. `timeout` is a call past `timeout_s` (macOS has no
`timeout`, so the bound is here). `usd` is what `claude` reports at list price: the subscription is not
billed by it, it is there to compare with Muse.

The cap is a number of calls, not dollars: a process-wide counter refuses every call past `MAX_CALLS`
(40 unless a tool's `--max-calls` sets it) with `max-calls`, making no call. Every call, refused ones
included, appends one line to `log`.

`claude -p` holds no conversation: `transcript()` folds the earlier turns into one prompt.
Python's standard library only, so it runs under `python3 -I`.
"""
import json, os, shutil, subprocess, tempfile, time

MAX_CALLS = 40
calls = 0  # this process's calls so far, refused ones not counted


def set_max_calls(n):
    global MAX_CALLS
    MAX_CALLS = n


def calls_left():
    return max(0, MAX_CALLS - calls)


def available():
    """Whether there is a `claude` to call: what a tool checks before any work."""
    return shutil.which('claude') is not None


def argv(system, model='haiku'):
    return ['claude', '-p', '--model', model, '--tools', '', '--output-format', 'json', '--max-turns', '1',
            '--system-prompt', system, '--no-session-persistence', '--strict-mcp-config']


def _log(path, rec):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, 'a', encoding='utf-8') as f:
        f.write(json.dumps(rec) + '\n')


def ask(system, prompt, *, model='haiku', timeout_s=300, log, tag=None):
    """-> {'result', 'text', 'secs', 'tokens_in', 'tokens_out', 'usd'}; `tag` goes into the log line."""
    global calls
    res = {'result': None, 'text': '', 'secs': 0.0, 'tokens_in': 0, 'tokens_out': 0, 'usd': 0.0}
    rec = dict(tag or {}, time=time.strftime('%Y-%m-%dT%H:%M:%S'), model=model)
    if calls >= MAX_CALLS:
        res['result'] = 'max-calls'
        _log(log, dict(rec, result='max-calls', error=f'past --max-calls {MAX_CALLS}'))
        return res
    calls += 1
    t0 = time.monotonic()
    err = None
    # A neutral folder: run inside the repo, `claude` would read its CLAUDE.md files into the context.
    with tempfile.TemporaryDirectory(prefix='fd-model-') as cwd:
        try:
            p = subprocess.run(argv(system, model), input=prompt, capture_output=True, text=True,
                               timeout=timeout_s, cwd=cwd)
        except subprocess.TimeoutExpired:
            res['result'], err = 'timeout', f'no answer in {timeout_s} s'
        except OSError as e:
            res['result'], err = 'no-model', str(e)[:400]
    res['secs'] = round(time.monotonic() - t0, 1)
    if res['result'] is None:
        try:
            d = json.loads(p.stdout)
        except ValueError:
            d = None
        if p.returncode != 0 or not isinstance(d, dict) or d.get('is_error') or not isinstance(d.get('result'), str):
            res['result'] = 'no-model'
            err = (f'exit {p.returncode}: ' + (p.stderr.strip() or p.stdout.strip()))[:400]
        else:
            u = d.get('usage') or {}
            res.update(result='ok', text=d['result'],
                       tokens_in=sum(u.get(k, 0) or 0 for k in ('input_tokens', 'cache_creation_input_tokens',
                                                                 'cache_read_input_tokens')),
                       tokens_out=u.get('output_tokens', 0) or 0, usd=d.get('total_cost_usd', 0.0) or 0.0)
    _log(log, dict(rec, result=res['result'], secs=res['secs'], tokens_in=res['tokens_in'],
                   tokens_out=res['tokens_out'], usd=res['usd'], **({'error': err} if err else {})))
    return res


def transcript(messages):
    """[{'role': 'user'|'assistant', 'content'}] -> one prompt: the last user message alone when it is the
    only one, else the whole exchange so far with the last message to answer at the end."""
    if len(messages) == 1:
        return messages[0]['content']
    parts = ['This continues an exchange with you. It is shown whole; answer its last message.']
    for i, m in enumerate(messages):
        last = i == len(messages) - 1
        head = ('# Now answer this' if last else '# You were told' if m['role'] == 'user' and i else
                '# You were asked' if m['role'] == 'user' else '# You answered')
        parts.append(f"{head}\n\n{m['content']}")
    return '\n\n'.join(parts)
