# T17 — haiku-backend

**Phase:** 0 · **Depends on:** — · **Weight:** light

## Goal

Switch the two model tools that stay, `generate.py` and `reshape.py`, from Muse on OpenRouter to Haiku through the owner's subscription, with a call cap instead of a dollar cap; and move the judge plumbing they borrow from Muse's `translate.py` into a module of its own, so T18 can replace `translate.py`.

## Design sections this implements

DESIGN §2.13, §5.3.

## Files

- `tools/model.py`, `tools/test_model.py` (new): the Haiku call.
- `tools/judge_run.py` (new): `JUDGE_SECONDS`, the judge lock, `flow_text`, the judge invocation, moved from `translate.py` unchanged.
- `tools/generate.py`, `tools/reshape.py`, `tools/translator_trial.py`, `tools/task_pack.py`, and the tests that import `translate` for the judge (`test_generate.py`, `test_generate_judge.py`): import from `judge_run` and `model`; `tools/README.md` sections for them.
- Remove from them: `OPENROUTER_API_KEY`, the price lookup, `--max-usd`.

## Interface

```python
# model.py
def ask(system: str, prompt: str, *, model: str = 'haiku', timeout_s: int = 300,
        log: str) -> dict   # {"result": "ok"|"no-model"|"timeout", "text": str, "secs": float}
# runs: claude -p --model haiku --tools "" --output-format json --max-turns 1 --system-prompt <system>
# appends one line to <out>/calls.jsonl; a run-wide counter refuses past --max-calls (default 40)
```

## Tests

- [ ] `model.ask` with a fake `claude` on PATH (a script echoing JSON): ok, non-zero exit is `no-model`, a slow fake is `timeout`, the 41st call is refused.
- [ ] `generate.py` and `reshape.py` tests pass on the fake.
- [ ] No file under `games/forest-dumb/tools/` mentions `OPENROUTER` except `translate.py`, `translate_batch.py` and their tests (`test_translate.py`, `test_translate_batch.py`), which T18 rewrites or deletes.

## Done when

- [ ] One real `generate.py` call on a level 1 routine returns a generator judged by coverage (§5.3 `worker`); its line is in FINDINGS.
- [ ] Muse's model call is gone from `generate.py` and `reshape.py`.
