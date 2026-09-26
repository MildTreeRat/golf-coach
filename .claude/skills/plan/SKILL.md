---
name: plan
description: Deep phased planning. Interviews the user thoroughly, then produces a plan split into small, low-context phases, documents it, and stops after every phase. Use when the user types /plan or asks to plan a feature, refactor, or project.
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
- For each phase list: goal, files likely touched, done criteria, how to verify.
- **P0 is always:** write this plan to `docs/plans/<short-name>.md` with a status checklist, so other sessions can see where things stand.
- The plan must state: **stop after every phase. Every time.** Update the checklist in the plan doc when a phase is done.
- The plan itself can be verbose.

## 4. After approval
Do P0 only, then stop. Tell the user to `/clear` and run `/next-phase` to continue.

## Communication
Keep all messages to the user brief and concise. Only the plan document may be long.
