Model only: `claude-opus-5-5` (Opus 5.5) in place of the baseline's `claude-opus-5`. No prompt changed.

The route under test is `feedback/coach.py::coach_through_golf_mcp`, called through the runner's
client with `model="claude-opus-5-5"` and a `GolfMcpServer` over `evals/coach_feedback/sessions/`. Its
system prompt, brief, effort (`low`), `max_tokens` (4,096) and adaptive thinking are the
baseline's, and so is the grading: the four code checks in `src/golf_coach/evals/coach_feedback.py`
and the judge in `src/golf_coach/evals/coach_judge.py`, on `claude-opus-5-5` (effort `high`,
`max_tokens` 16,000, no `fallbacks`), which never sees which model wrote a paragraph. Harness sha
`df41c54a485a` over `evals/coach_feedback/cases.json`, `evals/coach_feedback/cases.md`,
`src/golf_coach/evals/coach_feedback.py`, `src/golf_coach/evals/coach_judge.py` and
`src/golf_coach/evals/runner.py`. The grading is the one the user signed off as graded in M43 P20.
Same 16 cases, and the baseline's rep count. (M43 P21)
