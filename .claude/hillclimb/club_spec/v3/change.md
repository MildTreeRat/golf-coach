Model only: `claude-haiku-5-5` (Haiku 5.5) in place of the baseline's `claude-opus-5`. No prompt changed.

The route under test is `clubs/lookup.py::look_up_set`, called through the runner's client with
`llm_model="claude-haiku-5-5"`. Its system prompt, brief, output schema, effort (`high`), `max_tokens`
(16,000) and adaptive thinking are the baseline's, and so is the harness: sha `9b23640d87f9` over
`evals/club_spec/cases.json`, `src/golf_coach/evals/club_spec.py` and
`src/golf_coach/evals/runner.py`. The grading and tolerances are the ones the user signed off in
M43 P13 (loft and lie ±0.5°, length ±0.25"). Same 14 cases, 2 reps. (M43 P14)
