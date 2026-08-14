---
description: Review one module against docs/CODE_STANDARDS.md, or --all to survey and route
argument-hint: "<module> | --all"
---

Use the Agent tool with `subagent_type: architect-refactor`.

Scope requested: $ARGUMENTS

If that is empty, ask which module before spawning anything — a deep pass on a guessed scope wastes
the whole run. If it is `--all`, run Mode B (the survey that routes). Otherwise run Mode A (the deep
pass) on the named module.

Pass the agent this instruction:

> Read `docs/CODE_STANDARDS.md` and `docs/REFACTOR_LEDGER.md` before any code, run ruff/mypy/the
> boundary tests, then review the requested scope. Every finding needs a rule ID, a `file:line`,
> evidence, what breaks today, and a fix cost. `verdict: clean` is a correct and expected result.

Relay the findings to me as returned. **Do not implement any of them** — I decide which ones are
worth doing, and anything I decline goes into `docs/REFACTOR_LEDGER.md` as a new row so it is not
raised again.
