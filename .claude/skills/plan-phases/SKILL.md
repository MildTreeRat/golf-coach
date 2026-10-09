---
name: plan-phases
description: Deep phased planning. Interviews the user thoroughly, then produces a plan split into small, low-context phases, documents it, and stops after every phase. Use when the user types /plan-phases or asks to plan a feature, refactor, or project.
disable-model-invocation: true
---

# Phased Planning

Task to plan: $ARGUMENTS

## 1. Research first (no edits)
Explore the relevant code, docs, and existing plans in `docs/plans/` before asking anything. Don't ask what you can find yourself.

## 2. Interview until ambiguity is gone
- Use the AskUserQuestion tool in rounds (up to 4 questions per round). There is NO cap on rounds.
- Cover: goals, scope/non-goals, constraints, edge cases, data/API shapes, UX, testing, rollout, and anything the code research surfaced.
- After each round, reassess. Keep going until every decision that would change the plan is answered. Simple tasks may need 1 round; complex ones may need 5+.
- Offer a recommended option in each question.
- Don't start the plan until you'd bet the plan won't change after approval.

## 3. Write the plan
Break the work into phases P0, P1, P2...
- Each phase must be small in scope and completable by a fresh session with minimal context.
- If a phase fails that test, split it until it passes.
- **One phase, one gate.** A phase's cost grows with the square of its length, because every tool call re-sends everything before it. M36's three costliest phases (P2, P3, P8) each had a title joined by "and" over parts with their own done criteria. They peaked past 300k context and cost 17–25M input tokens each, against 7–11M for the rest. If a title needs an "and" and each half could go green on its own, make it two phases. Aim for a phase that finishes under ~250k context.
- For each phase list: goal, **what to read** (files, plus the earlier phases' findings it depends on, by phase number), files likely touched, done criteria, how to verify.
- **Lay the plan out for `/next-phase`'s map.** It reads everything above `## Phases` and the phase's own section, and greps the findings. So keep what every phase must know above `## Phases`: status checklist, rules, verify commands, decisions. Phases go under `## Phases` as `### Pn — title`. Findings go last, under `## Phase findings`, as `### Pn (date)`, and anything a later phase must know is addressed to it by number (`**For P12**: …`).
- **P0 is always:** write this plan to `docs/plans/<short-name>.md` with a status checklist, so other sessions can see where things stand.
- The plan must state: **stop after every phase. Every time.** Update the checklist in the plan doc when a phase is done.
- The plan itself can be verbose.

## 4. After approval
Do P0 only, then stop. Tell the user to `/clear` and run `/next-phase` to continue.

## Communication
Keep all messages to the user brief and concise. Only the plan document may be long.
