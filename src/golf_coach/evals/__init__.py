"""LLM evaluations: the runner and the flows it runs (ADR-036 §1, §5). [M43]

`runner.py` runs one flow's cases on one model, grades them and writes build-eval.md's contract
under `.claude/hillclimb/<flow>/`. Each flow is a module here exposing `FLOW`, a `runner.Flow`.
Nothing here runs in the default `pytest`, and nothing on the app's run path imports this package.

The import closure (ADR-036 §1): the standard library, the `anthropic` SDK, `config.py`, `llm_log`
and ADR-035 clause 1's LLM survivors, and nothing else in `golf_coach`.
"""
