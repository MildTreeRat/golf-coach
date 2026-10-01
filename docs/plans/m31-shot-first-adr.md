# Plan: M31 — the pivot decided (ADR-034, docs only)

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-09-29. **Milestone**: M31 of the program plan
[m31-m40-shot-first-pivot.md](m31-m40-shot-first-pivot.md#m31--adr-034-shot-first-the-launch-monitor-screen-is-the-product-the-phone-is-the-host)
— it has no `ROADMAP.md` section yet, because writing one is P8's job.
**Governing decisions**: ADR-034 (written by P3), amending ADR-002, 009, 010, 014, 016, 024, 030,
031, 032 and 033 by addendum (P4–P7).

---

## Status checklist

**Stop after every phase. Every time.** Update this table when a phase is done, in the same change
as the phase, and append what the phase *found* to its section below. The findings are what the next
phase is planned against.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document | ✅ Done *(2026-09-29)* |
| **P1** | Fold the interview's decisions into the program plan | ✅ Done *(2026-09-29)* |
| **P2** | Verify the facts ADR-034 will cite (read-only) | ✅ Done *(2026-09-29)* |
| **P3** | Write ADR-034 | ✅ Done *(2026-09-29)* |
| **P4** | Addenda, "where it runs": ADR-030, 002, 031, 033 | ✅ Done *(2026-09-29)* |
| **P5** | Addendum, "how it is checked": ADR-032, plus `docs/CONFORMANCE.md` §5 | ✅ Done *(2026-09-29)* |
| **P6** | Addenda, "how it is judged": ADR-009, 010 | ✅ Done *(2026-09-30)* |
| **P7** | Addenda, "how a shot arrives and is filed": ADR-014, 016, 024 | ✅ Done *(2026-09-30)* |
| **P8** | `ROADMAP.md`: the new M31–M40 group and its status rows | ✅ Done *(2026-09-30)* |
| **P9** | `ROADMAP.md`: re-scope the milestones the pivot moves | ✅ Done *(2026-09-30)* |
| **P10** | `CLAUDE.md` and the root `README.md` | ✅ Done *(2026-09-30)* |
| **P11** | `docs/PROJECT_CHARTER.md` §0 and `docs/FLOW.md` §1/§4 | ✅ Done *(2026-09-30)* |
| **P12** | Close: verify, WORKLOG, checklists, memory | ✅ Done *(2026-09-30)* |

**Exit** (the program plan's, unchanged): `pytest tests/test_docs_truth.py` passes. P12 runs the
whole verify suite anyway.

---

## What this milestone is

The pivot of 2026-09-29, written down as a decision. The golfer photographs the launch-monitor
screen, many times. Per-club strengths, weaknesses and grades come from those numbers. Video becomes
an optional visual aid, and the first host is a standalone iPhone app. The program plan
([m31-m40-shot-first-pivot.md](m31-m40-shot-first-pivot.md)) is the authority for *why*, and for
what the code already says. This milestone turns it into ADR-034, amends the ten ADRs it moves, and
brings the roadmap and the reader-facing docs in line.

**No code changes.** Nothing under `src/`, `crates/`, `tests/`, `spec/` or `scripts/` is touched,
`ANALYSIS_VERSION` stays where it is, and no vector moves. The only suite that can fail is
`tests/test_docs_truth.py`, and it fails on counts: the markdown-document count, the ADR count, the
addenda total, every per-ADR addenda cell, and the three ADRs that count their own addenda in words.

**Every phase that adds a file or an addendum updates those counts in the same change.** Counting
commands:

```bash
git ls-files '*.md' ':!.claude' | wc -l              # docs/README.md line 3; new files must be `git add`ed first
grep -c '^#\+ *Addendum' docs/decisions/*.md        # the per-ADR cells, and their sum is "N addenda between them"
```

Addenda counts at planning time (2026-09-29): 002 **4**, 009 none, 010 **8**, 014 **1**, 016 **2**,
024 **2**, 030 **2**, 031 **1**, 032 **11**, 033 **4**; total **76**. Three ADRs state their own
count in their Status block, and `test_an_adr_that_counts_its_own_addenda_counts_them_correctly`
reads it: **ADR-030 "Two addenda"**, **ADR-032 "eleven addenda"** and **ADR-033 "Four addenda"**.
Adding an addendum to any of them means rewording that phrase in the same edit. After M31 the total
is **86**.

---

## Decisions taken in the M31 interview, and why

These are the user's, 2026-09-29, and they **add to** the program plan's five decisions rather than
replacing them. P1 writes them into the program plan, and P3 writes them into ADR-034. A phase that
wants to revisit one needs a reason the interview did not have, and should record it here rather
than quietly diverge.

| # | Decision | Chosen | Why / the user's words |
|---|---|---|---|
| 6 | **Vocabulary** | **Shots are *tracked*. Device stats are *printed*.** A stat the launch monitor does not print is "not printed", never "not tracked". The program plan's `tracked_fields(shots)` becomes `printed_fields(shots)`, and "tracked = declared ∩ `fields_present`" becomes "printed = declared ∩ `fields_present`". | "Tracked" had come to mean two things. The user's own use is about shots, so the device side gets the other word. |
| 7 | **Which shots count** | **Derived from intent, reusing ADR-009's `PracticeGoal`** (`contracts/intent.py`). Its `mode` is set per session and overridable per shot, exactly as ADR-009 §Concepts already says. **A `DRILL` shot does not count; every other mode does.** It is derived, not stored as a second flag. An untracked shot is still stored, shown in history and analysed on its own, but never enters club or player stats. | "Some drills we do not want to add to the player stats as if they were actually swinging or actually playing." Only drill is excluded, and shot-shaping shots count. It is distinct from ADR-028's mishit rule, which is automatic and metric-scoped. |
| 8 | **Two levels** | Every tracked shot counts toward its **club's** stats and the **player's** overall stats. **Raw player-level averages exist only for club-independent stats**: strike location, face-to-path, start line, consistency rates. Carry, ball speed, launch and spin are per club only. Each stat declares its scope. | "If a player takes a swing with a 7 iron, that should go into their average as a player, but also keep track of that swing with a 7 iron." A driver-plus-wedge carry average means nothing. |
| 9 | **Grades** | **A grade plus a list, per topic.** A topic's grade is the **share of good shots**: the % of the club's tracked shots meeting the topic's criterion, carried as a rate with a Wilson interval (the program plan's `RateEstimate`) and **withheld below the minimum n**. No tour bands. Outliers lower a grade naturally. | "7 iron should be scored on low point, where on the club the user typically hits, how often they hit an average shot (so like a huge outlier would make this score lower)." A share is honest without a band. |
| 10 | **Topics** | **Strike location** (where on the face: centered rate, heel/toe). **Consistency** (how often a shot lands near your typical shot for that club). **Low point** (only where the device prints it, so **never for HD Golf**). **Fade**, **Draw** and **Straight**: three shot-shape topics, each graded **only over shots declared as that shape** (Decision 17). **A shape never declared is absent**: not graded, not mentioned, not in the blend. | "If I am trying to hit a draw, grade how well I draw and then land the ball on target. If you don't know don't score it." Then: "how often they can actually hit a fade when told to hit a fade… for a fade, draw, and straight shot." The exact criterion for strike location and consistency is M37's, against hand-worked vectors. |
| 11 | **Blends** | **A club blend and a player blend**, each the **equal-weight mean of the graded topics**, with the three shape topics counting as **three topics**. At player level, a topic's share pools every tracked shot across clubs, with **each shot judged against its own club's criterion**, so shares pool honestly where raw averages cannot. A topic that is printed but ungraded (n too small) is excluded from the blend **and named**, as `unscored` checkpoints are today. A topic that cannot exist (not printed, or a shape never declared) is simply absent. | The user asked for both blends, and for the shapes as three topics. Equal weights because no weighting has been measured. The pooling rule is what lets a player-level "consistency" exist at all. |
| 12 | **Mechanics** | **A separate panel.** When video exists, the pose checkpoints are scored as today (ADR-009's mechanics axis) and **never enter the topic grades or blends**. | Adding video must never move a club grade, and a player with no video is graded on the same terms. |
| 13 | **Android** | **Deferred.** iPhone only for M31–M40. The `TextRecognizer` boxes seam keeps ML Kit reachable later, and nothing is designed against Android. | — |
| 14 | **M26 (Ship it)** | **Paused and re-scoped under M40 whole**, like M21/M24/M25. M38's free Personal Team signing is the iOS path until then. | — |
| 15 | **Extra docs** | Beyond the program plan's file list, M31 also brings in line: `docs/PROJECT_CHARTER.md` §0; the root `README.md`; `docs/FLOW.md` (§1 and §4 redrawn, §2/§3 bannered); and addenda to **ADR-002, 010, 031 and 033**. | Each one says something the pivot made false. Examples: the charter says the phone is only a camera, and FLOW draws iPhones → Tailscale → laptop. |
| 16 | **ROADMAP detail** | **Short sections that route to the program plan**: status, the ask, depends-on, where it runs, exit, and a link. The detail lives in one place. | `CLAUDE.md`: pointed at, never copied. |
| 17 | **Shot-shape grading** | See the section below. | Added after the first draft of this plan, on the user's ask. |

### Decision 17 — shot shapes, in full

1. **Graded only when a shape was declared.** The declaration is `PracticeGoal.target_shape`
   (`STRAIGHT`/`DRAW`/`FADE`, which already exist in `contracts/intent.py`), with mode
   `SHOT_SHAPING`. It is set in one of two ways:
   - **by the golfer**, per session or per shot;
   - **by a challenge mode**, where the app calls fade, draw or straight before each shot and
     records the call as that shot's `target_shape`.
   Challenge shots are shot-shaping shots, not drills, so they **count fully**: they feed the shape
   topics and every other topic.
2. **Classified from numbers, not the screen's word.**
   - A shot is **straight** when its face-to-path is within `METRIC_TARGETS["face_to_path_deg"]`'s
     tolerance (reused, not a new constant).
   - Otherwise it is a **fade** or a **draw** by the sign, which flips for a left-hander.
   - Missing face-to-path on a shot → that shot is unscored for shape. Missing handedness →
     `no_handedness`.
   - HD Golf's `Shot Type` word is **not** used to grade. It stays what it is today: the cross-check
     on the spin-axis sign.
   - *Preliminary, from planning (2026-09-29):* face-to-path is computable on 12 of the 13 stored
     shots, and the word and the numbers disagree on one of them (−1.3°, printed `SLIGHT FADE`). P2
     confirms both.
3. **Two shares per shape.**
   - **"Hit the shape"**;
   - **"hit the shape and finished on line"**.
   **The second is the topic's grade** in the blends, and the first shows beside it, to say whether
   the shaping or the aiming failed.
4. **"Finished on line"** uses the device's printed landing offline where it prints one. **HD Golf
   (as seen so far) does not**: its `Horizontal Angle` is the start direction, not the finish.
   - Where the offline is not printed, it is **projected from the printed start direction and carry,
     including the curve** (ADR-027's flight model).
   - It is **named as projected** everywhere it appears, following ADR-024 §3's start-line
     projection and ADR-027 §6's measured/simulated split.
   - Where the projection cannot be computed, the on-line share is withheld and named.
   - **This is the one sanctioned exception to program-plan Decision 2 ("never inferred")**: the
     user's choice, made knowing the objection.
   - It must include the curve, because a start-line-only projection would mark every well-hit fade
     offline (a fade starts left on purpose).
   - The on-line tolerance comes from `METRIC_TARGETS`' offline row unless M37 finds a reason
     otherwise.
   - If the bay trip finds that HD Golf's `Custom` tile can print a landing offline, the printed
     value replaces the projection for this device, with no code change.

### How the grades relate to what already exists

This is written out so P1, P3 and P6 say the same thing:

- **ADR-009's per-swing `outcome_score` stays `None`.** A share of one shot is 0 or 100 and means
  nothing, so grading happens at club and player level, over many shots. ADR-009's
  shot-shaping/performance/drill *policies* for a single swing stay unbuilt. The three shape topics
  (Decisions 10 and 17) are where `SHOT_SHAPING` and `target_shape` finally get judged.
- **The strengths/weaknesses list keeps the program plan's rule** (its M31 §5). A strength needs an
  established equivalence claim (the CI of the mean inside target ± tolerance, *and* the SD's upper
  bound below tolerance). A weakness is an established bias or scatter, ranked by how far it exceeds
  tolerance. "Nothing established" is never a strength. The grade and the list sit side by side:
  the grade says how often, and the list says what is established.
- **"No score beats a wrong one" gains two neighbours** (ADR-010 §2):
  - *not printed* produces nothing at all (no measurement, no `unscored`, no caveat, no tip, no
    topic);
  - *not declared* (a shape never attempted) produces no topic.
  A topic that *is* printed but has too few shots is withheld and named.
- **One sanctioned inference**: the projected landing offline of Decision 17 §4. It is named as
  projected wherever it appears, and ADR-034 records it as the only exception to "never inferred".

### Explicit non-goals

- **No code, no tests, no vectors.** Finding 8's `tests/test_conformance.py::_PACKAGE_DATA` comment
  ("OCR stays Python") changes in **M34**, when the parser ports, not here. P5 changes only
  `docs/CONFORMANCE.md` §5's *decision*.
- **No `docs/ARCHITECTURE.md` edit.** It is AS-BUILT, and nothing that runs changes.
- **M7, M20, M27 and M30 are untouched.** M7's open Phase 0 is lab video work the pivot does not
  move. M20's trigger is reused on the phone in M39. M27 stays closed (ADR-034 reinforces it). M30
  trims the lab corpus. The `docs/README.md` row calling M7 "the current live plan" is stale; leave
  it to `/doc-check`, which the user runs.
- **The unrelated uncommitted edits present at planning time** (the `/plan` → `/plan-phases`
  rename in `.claude/skills/`, and the one-word edits to `ROADMAP.md` and
  `docs/plans/m23-pose-sidecar.md`) are the user's. Leave them as they are. A phase that commits
  `ROADMAP.md` will carry that one line, and that is fine.
- **No agent runs.** `/doc-check`, `/refactor-review` and `/design` are manual-only.

---

## Rules every phase keeps

- **Point, don't copy.** An addendum routes to ADR-034's numbered clause rather than restating it.
  A ROADMAP section routes to the program plan. No band value, no checkpoint count and no test count
  goes into prose (`CLAUDE.md`, and `test_volatile_counts_stay_out_of_prose`).
- **A number cited in an ADR comes from P2's findings**, with its date. It is never taken from the
  program plan unverified.
- **House style**: read two existing addenda in the ADR you are amending before writing yours, and
  match the heading form it uses (`## Addendum (date, …)` or `## Addendum, date — …`; both match the
  pin's `^#+ *Addendum`). Addenda here explain *why*, and name what they do **not** change.
- **Dates**: an addendum is dated the day it is written, not 2026-09-29.
- **`docs/README.md`'s ADR rows** have four cells, `| [NNN](decisions/…) | Decision | Status |
  Addenda |`. The last cell is `—` or `**N** — summary`, and the pin reads the first `**N**` in it.
  Keep each summary to what the addendum decided.

---

## Phases

### P0 — This plan document

**Goal**: this file, in the repo, with the counts that it moves.

**Files**:
- `docs/plans/m31-shot-first-adr.md` (new; this file).
- `docs/README.md` line 3–4:
  - markdown documents **74 → 75**;
  - `docs/` **61 → 62**;
  - `plans/` **2 → 3**.
- `docs/plans/m31-m40-shot-first-pivot.md`, status checklist: M31 → `🟡 In progress`, with a link
  to this plan.

**Done when**: the plan exists and is tracked, and the docs-truth suite passes.

**Verify**:
```bash
git add docs/plans/m31-shot-first-adr.md       # the count reads `git ls-files`
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py
```

### P1 — Fold the interview's decisions into the program plan

**Goal**: the program plan says what the interview decided, so M32–M40's sessions read one
authority rather than two.

**Files**: `docs/plans/m31-m40-shot-first-pivot.md` only.

**Changes**:
1. **Decisions**: add items 6–17 from the table above, dated "2026-09-29, M31 interview", with
   Decision 17's four points in full, plus the bullets under "How the grades relate to what
   already exists". Decision 2 ("never inferred") gains a pointer to its one exception (Decision
   17 §4).
2. **Vocabulary (Decision 6), throughout.** `grep -n -i "track" docs/plans/m31-m40-shot-first-pivot.md`
   and change every *device-field* use to "printed". Known sites:
   - Decisions §2;
   - "What the HD Golf data…" (low point "never tracked");
   - finding 1;
   - M31's "The ADR records" §1;
   - M32's `capability.py` bullet (`tracked_fields` → `printed_fields`) and its test bullets
     (`low_point`/`attack_angle` "never printed for `hd_golf`");
   - M37 ("low point, only when printed");
   - M39 P2 ("only when the device prints low point");
   - Invariants ("not printed is not unscored").
   Leave the per-shot meaning where it already appears.
3. **M31 section**:
   - the file list gains `docs/PROJECT_CHARTER.md` §0, `README.md`, `docs/FLOW.md` and the
     addenda to ADR-002/010/031/033;
   - "Roadmap changes" gains **M26 → under M40**, and **M5's "superseded in shape by" moves from
     M25 to M38**;
   - add a line routing to this plan for the phases.
4. **M35**:
   - record a `PracticeGoal` per shot (session default, per-shot override; ADR-009's shape,
     `contracts/intent.py`, which already exists);
   - add one rule, in `contracts/intent.py`, for whether a mode counts toward stats (`DRILL` does
     not);
   - `read_corpus` / the aggregates leave untracked shots out, and a pin says so.
   Name `analysis/scoring.py`'s exclusion of `unscored` from `overall_score` as the precedent for
   "excluded and named".
5. **M37**:
   - add the six topics: strike location, consistency, low point, fade, draw and straight;
   - add the share-of-good-shots grade (`RateEstimate`, Wilson) and the minimum-n withholding;
   - add the club and player blends with the pooling rule, and "ungraded is named, impossible is
     absent";
   - mechanics is a separate panel;
   - add the per-stat scope (club-only vs club-independent);
   - add Decision 17 whole:
     - classification by face-to-path against `METRIC_TARGETS`' tolerance, handedness-aware;
     - the two shares, with shape + on line as the grade;
     - the curved, flight-model projection named as projected;
     - hand-worked vectors that cross the straight tolerance's edge and a left-handed shot;
   - the contract gains a topic grade and blend shape beside `StrikeProfile`;
   - `rank_profile` is unchanged in rule.
6. **M38**:
   - the capture flow gains a session intent picker (mode and target shape) and a per-shot
     override;
   - it also gains **challenge mode**: the app calls fade, draw or straight before each shot and
     records it as `target_shape`;
   - P3's profile screens show per-topic grade + list (both shape shares), the club blend and the
     player blend.
7. **M40**: gains M26 whole (CI, packaging, signing, distribution).

**Done when**:
- the grep in step 2 shows no device-field use of "track";
- decisions 6–17 are present;
- docs-truth passes.

**Verify**: the grep; `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P2 — Verify the facts ADR-034 will cite (read-only)

**Goal**: every number ADR-034 states is measured on this box today, not copied from a TARGET-tier
plan.

**Files**: this plan's P2 findings section only. Nothing under `data/` is written. Scratch scripts go
in the session scratchpad.

**Claims to check**, each recorded as *confirmed* or *corrected*, with the number and the command
that produced it:
1. **Stored shots.** The program plan's numbers:
   - 13 shots in `data/processed/shots/*.shot.json`;
   - spin blank on 11 of them;
   - smash factor 0.89–1.00;
   - `impact_position` one of {CENTER, HEEL, TOE} on every shot.
2. **Screen layouts.**
   - Reference: the two reference photos in `data/raw/shot_screens/` show `Bounce & Roll`.
   - Bay: the bay photos show `Custom` and `Impact Position V`, and V reads `---`.
   - Use stored parse warnings and provenance first (the "no tile found" warning). Run the
     recognizer from a scratch script only if the tile inventory cannot be recovered from what is
     stored (the `ocr` extra is installed here).
   - Record how many bay photos there are, and where they live.
3. **What HD Golf prints.** It prints no attack angle, dynamic loft or low point: read ADR-027's
   statement and `launch_monitor/screen/profiles.json`'s targets.
4. **Unpinned pose model.** `pose/estimator.py`'s model URL is a `float16/latest` URL. Give the line.
5. **Swing-bound corpus.**
   - `storage/corpus.py`'s `ExclusionReason.NO_FACE_ON` admission rule;
   - `CorpusSwing.artifact_key` returning `shot:{sha}`.
   Give both lines.
6. **The intent contract.**
   - `contracts/intent.py::PracticeGoal`'s fields and `PracticeMode`'s members;
   - where a `PracticeGoal` is set today (`api/pipeline.py`, `analysis/engine.py`), and whether
     any stored artifact records a mode other than `fundamentals`.
7. **The "excluded and named" precedent.** `analysis/scoring.py` excludes `unscored` checkpoints
   from `overall_score` rather than counting them as zero.
8. **Targets and tolerances.** Which metrics `contracts/dispersion.py::METRIC_TARGETS` carries, with
   the tolerance each one has (names only in the ADR; no values in prose).
9. **Excluded shot metrics.** `analysis/shot_measure.py`'s docstring excludes smash factor and club
   speed, and says why.
10. **Shape classification (Decision 17 §2).**
    - How many stored shots have both `club_face_angle` and `club_path`.
    - The contract's sign convention for face-to-path, and which sign is a fade for a
      right-hander. Cite ADR-014's §Sign conventions and `measure_face_to_path`.
    - How handedness reaches a shot.
    - Per shot, the numeric class (±`METRIC_TARGETS["face_to_path_deg"]`'s tolerance) against the
      printed `shot_type` word. Planning saw 12 computable and 1 disagreement; confirm or correct
      both.
11. **Landing offline (Decision 17 §4).**
    - Confirm that no HD Golf tile in `profiles.json` prints a landing offline, and what
      `Horizontal Angle` maps to.
    - For each stored shot, can ADR-027's flight model (`analysis/flight*.py`, `spin_solve.py`)
      produce a lateral landing position including curve, and from which printed inputs? Spin axis
      is blank on most shots, so record which input carries the curve when it is, and how many
      shots the projection can reach.
    - This number decides how often the on-line share exists on HD Golf, and ADR-034 must state it.

**Done when**: all eleven are recorded here. A *corrected* one is also noted as an erratum at the foot
of the program plan's "What the code says" section, because that plan says a finding contradicting
it wins.

**Verify**: none beyond re-reading. Run docs-truth only if the program plan was touched.

### P3 — Write ADR-034

**Goal**: `docs/decisions/034-shot-first-phone-first.md`, "Shot-first: the launch-monitor screen is
the product, the phone is the host", **Accepted**.

**Decided after P2 (user, 2026-09-29)**: Decision 17 is written **as-is**. P2 found that the
"finished on line" projection needs a printed spin axis, which 2 of the 13 stored shots have and
none of the 11 from the 2026-08-23 bay session do. The ADR states that consequence plainly: at the
bay, the on-line part of every shape grade is withheld until the device prints a spin axis, and
only "hit the shape" shows. M37 may still revise the grade. Do not reopen it here.

**Read first**:
- the program plan's Why, Decisions, "What the HD Golf data…", and M31 sections;
- this plan's decisions table, Decision 17 in full, and P2's findings;
- `docs/decisions/000-template.md`;
- ADR-033's Status block, as the house model for a Status that routes.

**Structure**: the template's sections, in order.

- **Status**: Accepted, the date, the milestone (link the program plan). Name the ADRs it amends by
  addendum (002, 009, 010, 014, 016, 024, 030, 031, 032, 033), and say those addenda are written by
  M31 P4–P7. State that it partially supersedes ADR-030 (§5 and the laptop premise). **Do not write
  "**N addenda**"**: it has none.
- **Date**, and **Context**:
  - what the product was (ARCHITECTURE §1's "attached and displayed but never scored");
  - the pivot;
  - what HD Golf data supports, with P2's verified numbers.
- **Options Considered**:
  - A, keep ADR-030's laptop-first video product;
  - B, shot-first with the phone as a thin capture client to the laptop;
  - **C, shot-first on a standalone iPhone (chosen)**.
  - Then a sub-decision for OCR on the phone: **Apple Vision (chosen)**; PaddleOCR on-device; ML
    Kit (Android-first); a vision LLM, still rejected on ADR-014's grounds.
- **Decision**, as numbered clauses, so addenda can cite them:
  1. The unit of the product is the shot. Video is optional; mechanics are a separate panel
     (Decision 12).
  2. Printed and not printed: the device capability model.
     - `devices.json`, each field `analysed` or `shown_only` with a note;
     - printed = declared ∩ `fields_present`;
     - not printed produces nothing;
     - printed but blank on one shot goes through `unscored` (ADR-010 §2).
  3. Tracked shots: derived from `PracticeGoal.mode`, with `DRILL` excluded. Stored, shown and
     analysed alone, but never aggregated. Distinct from ADR-028.
  4. Two levels: per club and per player, with raw player-level averages for club-independent
     stats only.
  5. Grades:
     - per topic, a share with a Wilson interval, withheld below the minimum n;
     - the six topics and their availability rules;
     - **shot shapes (Decision 17) as their own sub-clause**:
       - declared by the golfer or called by challenge mode;
       - challenge shots count fully;
       - classified from face-to-path against `METRIC_TARGETS`' tolerance, handedness-aware, and
         not from the printed word;
       - two shares, with shape + on line as the grade;
       - **the projected landing offline, named as the one exception to clause 2's "never
         inferred"**, with the user's reason, the curve requirement, P2's reach number, and
         printed-replaces-projected;
     - the two blends, equal-weight, with the pooling rule and the shapes as three topics;
     - ungraded is named, impossible is absent;
     - no tour bands for shot metrics (personal baseline + `METRIC_TARGETS`);
     - the list's equivalence-claim rule.
  6. The phone is the host.
     - Standalone iPhone; the Rust core on-device via Flutter + `flutter_rust_bridge`.
     - Storage is on-device; export is how data leaves.
     - The laptop is a later client (M40). Android is deferred.
  7. OCR on the phone.
     - Apple Vision behind the `TextRecognizer` boxes seam, with VisionKit rectification.
     - The parser and validator port to Rust (M34).
     - PaddleOCR + OpenCV stay as the lab reader. The vision LLM stays rejected.
  8. Pose on the phone is reopened behind a conformance gate (M39 P0), with the laptop as the
     fallback (M40).
  9. The oracle per vector family:
     - ports are recorded from Python;
     - new analysis is Rust-first against hand-worked vectors (`provenance.oracle: "hand"`);
     - the lab reaches Rust-only analysis through `golf-core` subcommands.
  10. No LLM coaching on the phone.
- **Consequences**:
  - which milestones move (M21/M24/M25/M26 → M40, M28 → M39, M29 blocked on M40, M3's OCR →
    M32/M33, M4 full → M37, M5 → M38);
  - `ANALYSIS_VERSION`'s forced bump in M32 (finding 3);
  - two places that say "OCR stays Python" (finding 8).
- **Deferred, by choice**: Android; the paid Apple Developer Program; R10 BLE and launch-monitor
  APIs (M40); low point for HD Golf (the device does not print it); user-set blend weights.

**Files**:
- `docs/decisions/034-shot-first-phone-first.md` (new);
- `docs/README.md`:
  - the ADR row for 034, addenda cell `—`, placed after 033;
  - "33 decisions" **→ 34**;
  - markdown documents **75 → 76**;
  - `docs/` **62 → 63**;
  - ADRs **34 → 35** (the template is one of them).

**Done when**: the ADR exists and is tracked, and docs-truth passes.

**Verify**:
```bash
git add docs/decisions/034-shot-first-phone-first.md
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py
```

### P4 — Addenda, "where it runs": ADR-030, ADR-002, ADR-031, ADR-033

**Goal**: the four ADRs that place work on a machine say where the pivot moved it.

**Read first**: ADR-034 clauses 6 and 8, and the Status block and last addendum of each ADR below.

**Addenda**:
- **ADR-030** (the largest).
  - Status block: add "**partially superseded by
    [ADR-034](034-shot-first-phone-first.md)**", and reword "**Two addenda**" → "**Three
    addenda**".
  - The addendum:
    - §5 and the "the machine is a laptop" premise are superseded;
    - §1 is amended: storage is on-device too;
    - §2 is reopened *for the phone* behind M39 P0's gate, and still holds on the laptop;
    - the Deferred bullet "Pose on the phone" is now M39;
    - M21/M24/M25/M26 are re-scoped under M40, M28 is superseded by M39, and M29 is now blocked on
      M40;
    - §7 (cloud closed) is **reinforced**: the phone needs no network.
- **ADR-002**: pose on the phone.
  - iOS `PoseLandmarker`, the same `.task` pinned by sha256, VIDEO mode, CPU delegate;
  - the laptop's `float16/latest` URL is pinned first (P2 finding 4);
  - the gate is M39 P0: every `passed` verdict identical, every delta within tolerance;
  - if it fails, mechanics are computed on the laptop (M40).
  - `ranges.json` is untouched either way.
- **ADR-031**: M21 is paused and re-scoped under M40 as the desktop target of the same Flutter app.
  - `crates/capture` stays built and callerless;
  - the camera choice its crate doc has not made stays unmade.
- **ADR-033**:
  - the sidecar is **laptop-only**, because the phone has no Python;
  - its first caller moves from M24 to M40, or to M39's fallback if the gate fails;
  - nothing in the protocol changes.
  - Reword "**Four addenda**" → "**Five addenda**".

**Files**:
- the four ADRs;
- `docs/README.md`:
  - rows 002 (**4 → 5**), 030 (**2 → 3**, and its Status cell's "M24–M26 remain"), 031
    (**1 → 2**), 033 (**4 → 5**);
  - total **76 → 80**.

**Done when**: docs-truth passes. It covers the total, the per-row cells and the self-counts.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P5 — Addendum, "how it is checked": ADR-032, plus `docs/CONFORMANCE.md` §5

**Goal**: the conformance rules say how the new work is gated, and §5's inventory says what now
ships where.

**Read first**:
- ADR-034 clauses 7 and 9;
- ADR-032's Status block, §3 and last addendum;
- `docs/CONFORMANCE.md` §5 and "What this does not cover";
- the program plan's findings 5 and 8.

**ADR-032 addendum**:
1. **The oracle rule per family.**
   - A port is recorded from Python.
   - *New* analysis (M37's strike profile and grades) is Rust-first against hand-worked vectors,
     `provenance.oracle: "hand"`, and M37 adds the pin that every family names its oracle.
   - The lab reaches Rust-only analysis through `golf-core` subcommands.
2. **The screen parser has its own portability edges** (finding 5), and they are **M34's list, not
   additions to §3's six**:
   - `difflib.SequenceMatcher.ratio`;
   - the look-behind `_THOUSANDS` with no `regex` crate;
   - float floor division;
   - `{text!r}` in compared warnings;
   - `>=` vs `>` tie rules;
   - Unicode `upper()`/`split()`.
3. The planned **`crates/pyfmt` split** (M34), so `screen` need not depend on `analysis`, pending
   `docs/REFACTOR_LEDGER.md`.

Reword "**eleven addenda**" → "**twelve addenda**".

**`docs/CONFORMANCE.md` §5**, dispositions only:
- `launch_monitor/screen/`'s parser and validator → **ports to Rust, M34**. Recognizer and
  preprocessing stay the lab's.
- Tier 3 (the Python sidecar) → **laptop-only** per ADR-034. The phone has no Python, and pose
  there is M39's gate.
- `feedback/coach.py` → laptop/lab only (no LLM on the phone).
- Tier 4's retirement → §M29, **now blocked on M40**.

Do **not** touch `tests/test_conformance.py`'s comment (that is M34).

**Files**:
- ADR-032;
- `docs/CONFORMANCE.md`;
- `docs/README.md`:
  - row 032 **11 → 12**, total **80 → 81**;
  - the CONFORMANCE living-docs row, if its summary names §5's dispositions.

**Done when**: docs-truth passes.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P6 — Addenda, "how it is judged": ADR-009, ADR-010

**Goal**: the two scoring ADRs say how grades, tracked shots and "not printed" sit beside what they
decided.

**Read first**:
- ADR-034 clauses 1–5;
- this plan's "How the grades relate…";
- ADR-009 whole (it is short);
- ADR-010 §2 and its last two addenda;
- P2's findings 6 and 7.

**ADR-009** (its first addendum):
- the outcome axis becomes **per-topic grades at club and player level**, with two equal-weight
  blends, **not** a per-swing `outcome_score`, which stays `None`;
- `PracticeGoal` gains a second job: `mode` decides whether a shot counts (`DRILL` excluded,
  derived and not stored);
- `SHOT_SHAPING` + `target_shape` are what the fade/draw/straight topics are graded against,
  whether declared by the golfer or called by challenge mode, and a shape never declared is
  absent;
- the mechanics axis is optional and a separate panel, never blended into shot grades;
- the single-swing shot-shaping/performance/drill policies stay unbuilt.
- Say what is *not* changed: `PracticeGoal`'s shape, and "selected at session level, overridable
  per shot".

**ADR-010**:
- §2 extended:
  - not printed ≠ unscored (a field the device does not print produces no measurement, no
    `unscored` entry, no caveat, no tip, no topic);
  - printed but blank → `unscored`;
  - an ungraded topic is excluded from a blend and named;
  - a shape never declared is absent, because nothing was asked;
  - the projected landing offline (Decision 17 §4) is the one sanctioned inference, and is named as
    projected.
- Shot metrics get **no tour bands**. `ranges.json` is untouched, and the personal baseline plus
  `METRIC_TARGETS` judge them.

**Files**:
- ADR-009 and ADR-010;
- `docs/README.md`:
  - row 009 `—` → **1**, row 010 **8 → 9**;
  - total **81 → 83**.

**Done when**: docs-truth passes.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P7 — Addenda, "how a shot arrives and is filed": ADR-014, ADR-016, ADR-024

**Goal**: ingestion, topology and per-club history say what the pivot changed and what it left.

**Read first**:
- ADR-034 clauses 2, 3, 4, 6 and 7;
- the Status block and last addendum of each ADR below;
- P2's findings 1, 2 and 5.

**ADR-014**:
- OCR moves to the phone: Apple Vision behind the `TextRecognizer` boxes seam, with VisionKit
  rectification;
- the parser and validator port to Rust (M34);
- PaddleOCR + OpenCV remain the lab reader, and Option D (vision LLM) stays rejected on its own
  grounds;
- `devices.json` and "printed = declared ∩ `fields_present`" (M32);
- the `Impact Position V` tile, and the screen's `Custom` configurability, with **P2's numbers**,
  so layouts vary per golfer.

**ADR-016**:
- the phone is the host, and nothing on it listens on a port;
- the Tailscale serve/funnel upload topology stays the lab's path until M40 re-scopes it;
- export (M38 P4) is how data leaves the phone;
- ADR-019's key never reaches the phone, because there is no LLM there.
- The addendum says what is **reinforced**: no cloud, no router port.

**ADR-024**:
- the shot is the unit, and photo-only entries are admitted to the corpus (M35, program-plan
  finding 2);
- the club is read from the session cursor on the phone, as today;
- a `DRILL` shot never enters per-club or per-player aggregates;
- player-level raw averages exist only for club-independent stats, while topic grades pool across
  clubs;
- §3's rule, *a projection ships named as one*, is the precedent the projected landing offline
  follows (ADR-034's shot-shape sub-clause). It is a second projection beside §3's start-line one,
  and it includes the curve where §3's does not;
- ADR-028's mishit exclusion is unchanged and distinct: automatic and metric-scoped, where tracking
  is intent-derived and whole-shot.

**Files**:
- the three ADRs;
- `docs/README.md`:
  - rows 014 **1 → 2**, 016 **2 → 3**, 024 **2 → 3**;
  - total **83 → 86**.

**Done when**: docs-truth passes, and `grep -c '^#\+ *Addendum' docs/decisions/*.md` sums to 86.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P8 — `ROADMAP.md`: the new M31–M40 group and its status rows

**Goal**: M31–M40 exist in the roadmap, short, each routing to the program plan.

**Read first**:
- `ROADMAP.md` lines 1–70 (the status table);
- the `# The app — from an offline pipeline…` group's preamble (around line 1663), for the house
  form of a group;
- the program plan's status checklist.

**Changes**:
- **A new top-level group**, `# Shot-first, phone-first (M31–M40)`, placed immediately before
  `# The app — …`. Its preamble is 5–10 lines: the pivot in two sentences, then ADR-034 and the
  program plan as the routes.
- **Ten sections**, `## M31: …` through `## M40: …`. Each has:
  - **Status**;
  - **The ask** (2–4 lines);
  - **Depends on**;
  - **Where it runs** (desk / this box / Mac / bay);
  - **Exit**;
  - a link to the program plan's section.
  M31's section also links this plan.
- **Ten status-table rows** after M30's:
  - M31 `🟡 In progress`, with its phase count;
  - the rest ⬜ or 🔒, with what each needs to start, from the program plan's table.
- `## Last Updated`, to the date of the edit.
- **NEXT ACTION**: prepend a paragraph.
  - The next desk work is M32.
  - M33 (Mac) is the product's biggest unknown and runs early on purpose.
  - The next bay trip also serves the pivot: enumerate the screen's configurable tiles, and M38
    P5's photo session.
  - Keep the existing bay-session paragraph below it.

**Done when**:
- docs-truth passes (`test_volatile_counts_stay_out_of_prose` reads `ROADMAP.md`);
- every new `#anchor` link resolves: check the GitHub slug of each new heading by hand, the way the
  existing rows' anchors are formed.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P9 — `ROADMAP.md`: re-scope the milestones the pivot moves

**Goal**: no existing section still describes a plan ADR-034 moved.

**Changes**: each section gets a dated banner at its top, and its status-table row is updated. The
bodies stay as the record; the repo's convention is to record what changed, not rewrite what it
said.
- **M3** closes, with its open items routed:
  - OCR tuning and the tile enumeration → M32/M33;
  - the R10 → M40;
  - "Connect MCP server to analysis engine data merger" → read what it meant, then route it (likely
    M35/M37) or retire it with a reason.
- **M4 full**: superseded by M37 for the outcome axis. Its second-view and detection items stay
  parked where they are gated (M2, ADR-011).
- **M5**: "superseded in shape by M25" → **by M38**.
- **M21, M24, M25, M26**: `⏸ Paused`, re-scoped under M40 as the desktop target of the same Flutter
  app. M24's "open: how shot data arrives in a live flow" is answered by ADR-034 (a photo, on the
  phone).
- **M28**: superseded by M39.
- **M29**: blocked on **M40** instead of M25. Its job is unchanged.
- **The `# The app (M18–M28)` preamble** gets a dated banner: the premise changed a second time,
  and the machine is no longer a laptop but the phone (ADR-034). Its "Order" paragraph is
  historical.
- **Leave M7, M20, M27 and M30 unchanged** (non-goal).

**Done when**: docs-truth passes, and
`grep -n "Blocked on M25\|Blocked on M24\|M25 only" ROADMAP.md` shows only bannered or historical
lines.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P10 — `CLAUDE.md` and the root `README.md`

**Goal**: the two files every reader sees first say what the product is becoming, and still say
what runs today.

**`CLAUDE.md`** (loaded into every session, so keep it terse):
- **The product sentence at the top.** Keep today's as-built description: face-on video → pose → …,
  shot data by OCR. Add one sentence: the product is now shot-first and phone-first per ADR-034,
  and none of that is built yet.
- **The "What is the app written in, and why?" row.** ADR-030 *as amended by ADR-034*:
  - a standalone iPhone app, with the Rust core on-device via Flutter + `flutter_rust_bridge`;
  - pose on the phone behind M39's gate;
  - the Python sidecar is laptop-only (M40).
- **A new routing row**: "What is the product now, and why did it pivot?" → ADR-034, then the
  program plan.
- Every path named must exist (`test_claude_md_routes_only_to_files_that_exist`).

**Root `README.md`**:
- The product blurb: the shot-first direction in two sentences, then today's working pipeline,
  unchanged.
- The stale NEXT ACTION (M6's live key, proven 2026-08-14) → a pointer to `ROADMAP.md`'s NEXT
  ACTION.
- **Its milestone table** is a second copy of `ROADMAP.md`'s status table that drifts. Replace it
  with a link to "Status at a glance", per "point, don't copy". If a table must stay, it gains
  M31–M40 and the re-scoped rows.

**Done when**: docs-truth passes.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P11 — `docs/PROJECT_CHARTER.md` §0 and `docs/FLOW.md` §1/§4

**Goal**: the founding doc records the second scope move, and the target diagrams draw the new
target.

**Charter** (FOUNDING tier; it records what changed and never rewrites §1–§6):
- The "Last Updated" line.
- In §0:
  - the "Mobile app" bullet is amended: the phone is now the **host**, not only a camera (ADR-034);
  - a new bullet says the shot is the unit, and mechanics are optional;
  - the dual-axis bullet notes that outcome is now per-topic grades at club and player level.
- "Multi-user support" stays out of scope, with no accounts and no sync. The golfer picker is local.

**FLOW.md** (TARGET; its ✅ markers are the source of truth for what exists):
- The "Last reviewed" date.
- **§1, the milestone status map**, redrawn (mermaid) to include M18–M40 and the pivot:
  - done / paused / planned;
  - M31 → M32 → {M33, M34, M35};
  - M35 → M36 → M37 → M38 → M39;
  - M38 → M40.
- **§4, deployment**, redrawn:
  - **iPhone**: Flutter shell, Rust core via FRB, Apple Vision OCR, on-device storage; optional
    video and pose behind the M39 gate.
  - → export →
  - **laptop lab**: Python `api/`, `mcp/`, `scripts/`, the pose sidecar.
  - **M40 laptop client**: R10 BLE, API adapters, cameras.
- **§2 and §3** get a dated banner: pre-pivot; redrawn when M35/M37 land.
- Keep the tier banner within the first 8 lines (`test_every_living_doc_declares_a_tier`).

**Done when**: docs-truth passes, and both mermaid blocks parse. Paste each into a Mermaid renderer,
or check the syntax by eye against the existing blocks.

**Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P12 — Close

**Goal**: M31 is closed, and every checklist says so.

**Steps**:
1. The full verify suite (below). Run `cargo test` unpiped, and not concurrently with pytest.
   Nothing here should move it, and that is the point of running it.
2. **`WORKLOG.md`**: one entry at the top, following M23 P9's precedent. The plan holds the
   per-phase record; the entry covers the close. Include:
   - what M31 decided;
   - what the docs said that was wrong;
   - what P2 corrected;
   - what M32 should read first.
3. **Checklists**:
   - this plan → every phase ✅;
   - the program plan's M31 row → `✅ Done`;
   - `ROADMAP.md`'s M31 row and section → `✅ Done`, and M32 → "Nothing — this box".
4. **Memory**: update `app-transition-decisions.md` in the user's auto-memory directory:
   - ADR-034 exists and is the authority;
   - the vocabulary: shots tracked, stats printed;
   - drills don't count;
   - grades are per-topic shares with club and player blends;
   - fade, draw and straight are graded only when declared;
   - the projected landing offline is the one sanctioned inference;
   - the next milestone is M32.
5. Tell the user that `/doc-check` is available if they want a reconciliation pass. Do not run it.

**Done when**: all six verify commands are green, and the three checklists agree.

---

## Verify (the whole suite, P12; docs-truth every phase)

```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
.venv/Scripts/python.exe scripts/conformance.py check
cargo test
cargo clippy --all-targets && cargo fmt --check
```

`cargo test | tail` reports `tail`'s exit status (WORKLOG, M23 P9), so run it unpiped or check
`PIPESTATUS`.

---

## Phase findings

*(Each phase appends what it found here, under its own heading. A finding that contradicts the plan
wins, and the phase after it is planned against the finding.)*

### P1 — found (2026-09-29)

Only `docs/plans/m31-m40-shot-first-pivot.md` changed. Decisions 6–17 are its new
"Added in the M31 interview" subsection, with Decision 17 and "How the grades relate…" as their own
subsections, and Decision 2 links to Decision 17 §4 as its one exception. The done-when grep now
hits only the per-shot meaning, plus Decision 6 quoting the retired phrase "not tracked".

1. **One device-field site was missing from step 2's list:** M35's "`analyze_shot(...)`, filtered
   by `tracked_fields`". It now reads `printed_fields`.
2. **M37 is renamed in the program plan**, in both its table row and its heading: "Strike profile,
   topic grades and strengths/weaknesses (Rust first)". **P8** should use that name.
3. **M40's table row** now reads "M21/M24/M25/M26 re-scoped", and the program plan's M31 row reads
   2/13 phases. **Each later phase moves that count**, until P12 marks the row done.
4. **The program plan's header** now lists all ten amended ADRs, with links. It had listed six.
5. **`analysis/scoring.py` excludes nothing itself.** `_mean_percent` averages whatever list it is
   handed. The exclusion happens in `analysis/engine.py`'s checkpoint loop, where a score of `None`
   goes to `unscored` and never to `mechanics`, and again in `_without_contradicted_scores`, which
   re-combines only the scores it kept. The program plan's M35 now names the engine as where the
   precedent lives.
   - **P2 claim 7** should confirm this with line numbers.
   - **P6** should cite the engine *and* `scoring.py`, not `scoring.py` alone.
6. **`METRIC_TARGETS` has one offline row, `start_line_offline_yds`, and it is a start-line
   projection** (`carry * sin(start_line_deg)`, its tolerance converted from `_JUDGED_DEGREES`). It
   is not a landing offline. Decision 17 §4 takes "the offline row" for the on-line tolerance
   anyway, so the landing projection would be judged by a start-line-derived width.
   - **P2 claim 8** should record this.
   - M37 already holds the "unless M37 finds a reason otherwise" clause, so no decision is
     reopened here.
7. **What P1 added that the steps did not spell out.** Each follows from a decision rather than
   being new:
   - **M35** names its pins: `tests/contracts/test_intent.py` (new), and a `DRILL` case in
     `tests/storage/test_corpus.py`.
   - **The Invariants** gain "a `DRILL` shot never reaches an aggregate".
   - **M38 P1's facade** exposes the topic grades and blends.

### P2 — found (2026-09-29)

Read-only. Nothing under `data/` was written. The scratch scripts (a read of the shot artifacts, an
OCR tile dump, and a flight-reach join) lived in the session scratchpad. The program plan changed
only by an erratum at the foot of "What the code says", plus its M31 row's phase count (3/13).
**Four claims are corrected in part (1, 2, 3 and 10). Claims 10 and 11 move P3**, so read both
before writing ADR-034.

1. **Stored shots: two confirmed, two corrected.**
   - *Confirmed*: 13 shots (`ls data/processed/shots/*.shot.json | wc -l`), all `device: hd_golf`,
     all `needs_review: false`.
   - *Confirmed*: spin is blank on 11, and spin axis is blank on the same 11.
     - Only the 2026-08-10 pair (`2026-08-10-1`, `-2`) prints both.
     - Every 2026-08-23 shot reads `---` for both.
     - Both sessions show the same layout (claim 2), so whether spin prints varies by session, not
       by layout.
   - *Corrected*: smash factor reads **0.76–1.06**, not 0.89–1.00 (8 below 1.00, 2 at 1.00, 3
     above).
     - The conclusion stands. Every value is far below what a real strike produces.
     - `ball_speed / club_head_speed` reproduces the printed smash to rounding on all 13, so the
       simulator prints these numbers and the OCR reads them faithfully.
     - The stale range comes from `analysis/shot_measure.py`'s docstring (claim 9). ADR-027:370's
       "0.89–1.06" is stale at the low end.
   - *Corrected*: `impact_position` is one of {CENTER, HEEL, TOE} on **12 of 13** shots (7 CENTER,
     4 HEEL, 1 TOE). `2026-08-10-1` has `None`, for the reason in claim 2's last bullet.
2. **Screen layouts: `Custom` corrected, the rest confirmed.**
   - **How this was measured.** Stored artifacts cannot answer it. `raw_fields` and the "no tile
     found" warning cover stored fields only, and `Custom` is `target: null`. So the recognizer was
     run from a scratch script:
     - `prepare_screen(load_image(p), PaddleOCRRecognizer(), load_profile("hd_golf"))` over all 15
       photos;
     - two of them checked by eye.
   - **Where the photos are.**
     - Reference: `data/raw/shot_screens/IMG_2738.jpeg` and `IMG_2739.jpeg`. **Neither is a stored
       shot** (no hash matches). Only `tests/launch_monitor/test_screen_integration.py` reads them.
     - Bay: **13 photos, which are all 13 stored shots.**
       - The 2026-08-23 session's 11 are at `data/raw/processing-manual/shot-{1..11}.jpg`, with
         HEIC originals in `pictures/`. Shot 1 was stored from its HEIC.
       - The 2026-08-10 pair are at `data/raw/aaron-{1,2}/Aaron-shot-{1,2}.png`.
       - Each is also copied per swing to `data/processed/sessions/<session>/<n>/shot_screen.<sha12>.*`.
   - **Reference layout** (15 tiles):
     - row 1: Shot Distance, Carry, **Bounce & Roll**, Ball Speed, Launch Angle, Club Speed, Club
       Path, Custom;
     - row 2: Club Face Angle, Spin, Smash Factor, Impact Position, Shot Type, Horizontal Angle,
       Spin Axis.
   - **Bay layout** (15 tiles, the same on all 13 bay photos):
     - row 1: Shot Distance, Carry, Ball Speed, Launch Angle, Club Speed, Club Path, Club Face
       Angle, Custom;
     - row 2: Spin, Smash Factor, Impact Position, Shot Type, Horizontal Angle, Spin Axis,
       **Impact Position V**.
   - *Corrected*: **`Custom` is on both layouts.**
     - It is a settings-gear tile with no value, top right, and `profiles.json:67` already carries
       it as boundary-only.
     - So the two layouts differ by one tile, `Bounce & Roll` ↔ `Impact Position V`.
     - `Custom` shows that the screen is configurable. It does not distinguish the two photo sets.
   - *Confirmed*: `Impact Position V` reads `---` on all 13 bay photos.
     - The OCR returns no text under it on any of them.
     - `---` was seen by eye on `shot-2.jpg`, and on `Aaron-shot-2.png`, where spin *was* printed.
   - **New, and it corrects ADR-014's addendum.**
     - On 2 of the 13 bay photos (`Aaron-shot-1.png`, and shot 1's jpg and HEIC), the OCR reads the
       V tile's label as plain `Impact Position`. Two boxes then tie at 1.0 for one field, and
       `_find_labels` keeps the first (`>`).
     - On `Aaron-shot-1.png` the V tile wins. `impact_position` reads blank, and the real tile's
       `CENTER` spills into `Shot Type` (`'CENTER SLIGHT FADE'`).
     - A re-run of the parse confirmed it: the V box wins there, and the real one wins on shot 1's
       HEIC (HEEL, correct).
     - ADR-014's addendum blamed a cell boundary on an uncropped frame. The actual cause is
       program-plan finding 6's hazard, seen on a real photo, with the `V` dropped rather than
       misread as `Y`.
     - The result was a missing value plus a contaminated `shot_type` string. The normalized shape
       survived, because curvature words are matched first.
     - M32/M34 own the fix and its test. P7's ADR-014 addendum is the place to correct the cause.
3. **What HD Golf prints: confirmed, with the attribution corrected.**
   - No profile field targets attack angle, dynamic loft or low point (`profiles.json`), and
     `ShotData` has no field for any of them. Neither layout above has such a tile.
   - *Corrected*: ADR-027 makes this claim only for **attack angle** (ADR-027:372, and
     §Deferred's "needs an attack angle"). It says nothing about dynamic loft or low point, so
     those two should cite the tile inventory, not ADR-027.
   - What `Custom` can be set to show is unknown until the bay trip.
4. **The unpinned pose model: confirmed.** `pose/estimator.py:43-45` defines
   `_MODEL_URL = ".../pose_landmarker_{variant}/float16/latest/pose_landmarker_{variant}.task"`, and
   `float16/latest` is on line 45. It is fetched by `urllib.request.urlretrieve` at :98 with no hash
   check.
5. **The swing-bound corpus: confirmed.**
   - `storage/corpus.py:92-100`: `face_on = manifest.roles.get(Role.FACE_ON)`, and when it is
     `None` the swing gets `ExclusionReason.NO_FACE_ON` (:97; the enum is `contracts/career.py:116`).
   - `CorpusSwing.artifact_key` is at `contracts/career.py:292`, and
     `return f"shot:{self.shot_sha256}"` is at :336.
   - It returns `None` first in two cases: a flagged parse or a missing photo hash (:329), and the
     two distance metrics of a mishit (ADR-028).
6. **The intent contract: confirmed, plus one blocker for M35.**
   - `contracts/intent.py`:
     - `PracticeGoal` has four fields: `mode` (`PracticeMode`, default `FUNDAMENTALS`),
       `target_shape` (`TargetShape | None`), `club` (`ClubCategory`, default `ALL`) and
       `focus_checkpoint` (`str | None`).
     - `PracticeMode` is `FUNDAMENTALS`, `SHOT_SHAPING`, `PERFORMANCE`, `DRILL`.
     - `TargetShape` is `STRAIGHT`, `DRAW`, `FADE`.
     - Its docstring already says "Selected at session level, overridable per shot."
   - Where it is set today:
     - `api/pipeline.py:1251` passes `intent=PracticeGoal(club=options.club)`, so the mode is never
       set;
     - `analysis/engine.py:412` defaults to `intent or PracticeGoal()`.
   - Stored: 15 `analysis.json` files carry an `intent`, all `mode: fundamentals` and
     `target_shape: null`. Nothing on disk records any other mode.
   - **New: every mode except `FUNDAMENTALS` raises today.**
     - `analysis/scoring.py:54-62`'s `policy_for` raises `NotImplementedError`, and
       `crates/analysis/src/scoring.rs:90-96` panics.
     - `engine.py:463` calls it on every `analyze_swing`.
     - So a `DRILL` or `SHOT_SHAPING` intent cannot reach the engine until either a policy exists
       or the mode travels beside the swing rather than into it. M35, and P6's ADR-009 addendum,
       should say which.
7. **"Excluded and named": confirmed where P1 finding 5 put it, with one edge.**
   - `analysis/engine.py:433-447` is the loop. A `None` score goes to `unscored` (:445) and never
     to `mechanics` (:442).
   - :463 combines only `mechanics`, and `_without_contradicted_scores` (:729) re-combines only the
     scores it kept (:763-766).
   - `analysis/scoring.py:37-41`'s `_mean_percent` averages whatever list it is handed.
   - **The edge: it returns 0.0 for an empty list.** A swing with every checkpoint unscored prints
     `overall_score` 0.0, not `None`, because `AxisScores.overall` is a `float`. M37's blends should
     not copy this: a blend with no graded topic is absent, not zero.
8. **Targets and tolerances: confirmed.**
   - `METRIC_TARGETS` is at `contracts/dispersion.py:328`.
   - The launch-monitor rows, with tolerances as of 2026-09-29 (the code is the authority):
     - with a target of 0: `face_to_path_deg` (2.0°), `start_line_deg` (2.0°) and
       `start_line_offline_yds` (9.0 yd);
     - with `target: None` and a `no_target_reason`: `carry_distance_yds` (5.0 yd),
       `total_distance_yds` (5.0 yd), `ball_speed_mph` (4.0 mph) and `launch_angle_deg` (2.0°).
   - The pose rows: `head_sway_norm`, `finish_balance_norm`, `hip_sway_norm`,
     `hip_shift_at_top_norm`, `head_hip_offset_impact_norm`, `head_hip_gain_norm`, `tempo_ratio`,
     `hand_separation_norm`, `hand_height_norm`, `hand_offset_from_hips_norm`,
     `trail_hand_roll_deg`, `backswing_ms` and `downswing_ms`.
   - The shot tolerances are **judgment** (`_JUDGED_DEGREES`, `_JUDGED_YARDS`, `_JUDGED_OFFLINE`:
     "no instrument-error evidence exists for the OCR path"). The pose tolerances are measured.
   - P1 finding 6 is confirmed.
     - The only offline row is `start_line_offline_yds`, which is `carry * sin(start line)`. Its
       9 yd is 2° at a 250 yd driver carry.
     - There is no row for `flight_landing_offline_yds`.
   - `MetricTarget.tolerance` is documented as measurement error, and it already does two jobs
     (bias and scatter). Decision 17 §2 gives it a third, the straight band.
     `_JUDGED_DEGREES`'s own words support that reading: "2 degrees is below the level a coaching
     action follows from."
9. **Excluded shot metrics: confirmed, but the docstring is stale.**
   - `analysis/shot_measure.py:36-51` excludes `smash_factor` and `club_head_speed`, for two
     reasons:
     - a smash below 1.0 is a device artifact, and the OCR reproduces it faithfully;
     - the 2026-08-10 pair (the same ball speed and near-equal carry, but club speeds 7.3 mph
       apart) points at club speed as the wrong reading, so ball speed stays.
   - Its numbers are stale: "every shot on disk reads 0.89–1.00" (claim 1), "passes on all three",
     and "one shot per session on disk". So is `measure_face_to_path`'s "three shots recorded so
     far".
   - M31 changes no code. `/doc-check` or M32 owns the fix.
10. **Shape classification: both preliminary numbers confirmed, the handedness rule corrected.**
    - `club_face_angle` and `club_path` are both present on **12 of 13** shots. `2026-08-23-3`
      misread its face angle ("could not read a number from 'Open'").
    - The signs:
      - `contracts/shot.py:68-76`: face `+ = open`, path `+ = in-to-out`, `spin_axis` `+ = fade`,
        `launch_direction` `+ = right`;
      - ADR-014 §Sign conventions (:123) maps the printed words to those signs;
      - `measure_face_to_path` (`shot_measure.py:94`) is face − path, and **positive is a fade**
        for a right-hander.
    - *Corrected*: **that sign does not flip for a left-hander under the contract.**
      - Face, path and spin axis are signed in golfer-relative words (open, in-to-out, fade). Only
        `launch_direction` is geometric.
      - `flight_infer._direction_of` (:414) reads face-to-path as fade or draw with no handedness:
        "a sign read and not a conversion".
      - The flip lives in `infer_spin_axis` (:403), where `+ = fade` becomes `+ = right`.
      - What flips is **which side of the target line a shape finishes on**. So handedness belongs
        to Decision 17 §4's projection, and a missing handedness withholds the on-line share, not
        the shape class.
      - Caveat: no left-handed shot exists, so whether HD Golf prints golfer-relative words for a
        left-hander is unverified. M37's left-handed vector pins the contract's reading.
      - P3 should write "handedness-aware" in this sense.
    - How handedness reaches a shot:
      - It is not on `ShotData`. It arrives through `storage/flight_inputs.py::read_flight_inputs`
        (:102): the photo's sha256 → the `SwingManifest` holding it as `SHOT_SCREEN` →
        `player_id` → `GolferStore` → `Golfer.handedness` (`contracts/golfer.py:47`).
      - All 13 shots resolve as right-handed.
      - A photo with no swing gets `None`, so M35's photo-only entries need another route.
    - Per shot, against ±2.0° (`face_to_path_deg`'s tolerance) and the printed word:
      - 10 fades, 1 draw (`-6`, −2.9°, printed `SLIGHT DRAW`) and 1 straight;
      - the straight one is `-11`, at **−1.3° with `SLIGHT FADE` printed**, and it is the one
        disagreement;
      - 1 cannot be computed;
      - two sit near the edge: `-8` at +2.6° (`SLIGHT FADE`) and `-6` at −2.9°;
      - `scripts/simulate_flight.py --shots` flags `-11`'s disagreement itself.
11. **Landing offline: confirmed not printed. The projection reaches 2 of 13 shots, and none at
    the bay.**
    - No profile field is a landing offline. `Horizontal Angle` maps to `launch_direction`
      (`profiles.json:104-112`), which is the start direction. Neither layout has an offline tile
      (claim 2; ADR-027:1544, "prints no offline tile at all").
    - **The curve is carried by the printed `Spin Axis` tile and nothing else.**
      - Face-to-path gives the curve's direction but not its magnitude (ADR-027's 2026-09-05g
        addendum).
      - With no axis, the flight is planar and `flight_measure._landing_offline` (:205) returns
        `None` rather than repeating `start_line_offline_yds`.
      - The spin rate is either printed or solved from carry with the club's loft, and handedness
        flips the axis.
    - **Reach, as stored: 2 of 13.**
      - These are the 2026-08-10 pair, the only shots with a spin axis. They land at −9.6 and
        +15.2 yd, against start-line offlines of −11.6 and +8.4.
      - Forcing right-handedness, or a loft, onto every shot still leaves 2.
      - On the 11 bay-session shots the reach is **0**. Only one of them flies at all (`-4`, on a
        solved spin, planar); 7 are `carry_unreachable`, 2 `no_club_loft` and 1
        `spin_not_recoverable`.
      - Commands: `scripts/simulate_flight.py --shots`, and a scratch join of
        `read_flight_inputs` → `fly_shot` → `_landing_offline`.
    - **At the bay as seen, the on-line share is withheld on every shot.** That share is Decision
      17 §3's grade for fade, draw and straight, so only "hit the shape" would exist.
      - Decision 17 §4 already provides for this ("withheld and named"), so P3 can write it as
        decided. ADR-034 must state the number.
      - The bay trip that enumerates `Custom` should also check whether spin and spin axis can be
        made to print. The 2026-08-10 pair shows that they sometimes do, on the same layout.

### P3 — found (2026-09-29)

`docs/decisions/034-shot-first-phone-first.md` exists, **Accepted**, and is tracked. `docs/README.md`
gained its row after 033 (addenda `—`) and moved four counts: 75 → 76 documents, 62 → 63 in
`docs/`, 34 → 35 ADRs, 33 → 34 decisions. The addenda total stays **76**, so P4 starts from 76 → 80.
The program plan's M31 row reads 4/13. Docs-truth: 92 passed.

1. **The clause numbers the addenda cite.** Clauses 1–10 follow P3's list in order. Clause 5 is
   split into sub-clauses, and P4–P7 should cite them by number:
   - 5.1 the share and `RateEstimate`;
   - 5.2 the six topics and their availability;
   - **5.3 shot shapes**, which holds the projected landing offline (the one exception to clause
     2's "nothing stands in"), the reach number and the bay consequence;
   - 5.4 the blends and the pooling rule;
   - 5.5 ungraded is named, impossible is absent;
   - 5.6 no tour bands;
   - 5.7 the list's equivalence-claim rule.
2. **The user's post-P2 decision is written as decided**, in 5.3's last paragraph: at the bay, the
   on-line share of every shape topic is withheld and only "hit the shape" shows, until the device
   prints a spin axis. It cites 2 of 13 and 0 of 11, and says M37 may revise the grade and this ADR
   does not reopen it.
3. **"Handedness-aware" is written in P2's corrected sense** (5.3): the fade/draw sign does not
   flip, handedness decides the finishing side, and a missing handedness withholds the on-line
   share rather than the shape class. The program plan's M37 section still says "the sign flips
   for a left-hander"; its erratum covers it, and M37 should read the erratum.
4. **Consequences P3 added from P2's findings**, each routed rather than decided:
   - `policy_for` raising for every mode but `FUNDAMENTALS` → M35 decides, and **P6's ADR-009
     addendum records which**;
   - no handedness route for a photo-only shot → M35;
   - the `Impact Position V` label hazard → M32/M34 fix it, and **P7's ADR-014 addendum corrects
     the cause**;
   - `_mean_percent`'s 0.0 on an empty list, which 5.5 says the blends do not copy.
5. **`rank_profile` does not exist yet.** No module or crate holds strengths/weaknesses logic
   (`grep -i strength src/` finds only comments). The program plan's M37 says
   `crates/feedback/src/profile_rules.rs::rank_profile` is "unchanged in rule", which reads as if it
   existed. It means the rule, and the function is M37's to write. ADR-034 5.7 states the rule and
   names no function, and **P6** should do the same.
6. **Two reasons the ADR gives that the plan did not spell out**, both following from its own
   decisions:
   - clause 9: the phone runs only Rust, so a Python-first implementation of new analysis would be
     born to be retired (ADR-030's 2026-09-22 addendum, ADR-032 §7);
   - the OCR sub-decision: PaddleOCR on-device buys less than it seems, because the vectors gate
     the parser, not the recognizer.
7. **For P6: "ADR-010 §2" is correct but easy to misread.** §2's heading is "Ranges resolve as a
   function, with fallback", and the no-score rule is its last sentence. ADR-010's own later text
   calls it "§2 says no score beats a wrong one".
8. **The map's 034 row links `[M31](../ROADMAP.md)`**, the form the other rows use. It resolves
   today and becomes specific once P8 writes the M31 section.

### P4 — found (2026-09-29)

Four addenda, one per ADR, each dated 2026-09-29:

- ADR-030: "the machine is the phone";
- ADR-002: "pose on the phone, behind a gate, and the model pinned first";
- ADR-031: "paused under M40";
- ADR-033: "laptop-only".

The ADR-030 and ADR-033 self-counts read **Three** and **Five**. `docs/README.md` rows 002/030/031/033
read 5/3/2/5, and the total moved **76 → 80** (`grep -c` sums to 80). The program plan's M31 row
reads 5/13. Docs-truth: 92 passed. Every new `#anchor` was checked against its target's headings by
a scratch slug script, and all resolve.

1. **Status edits beyond the plan's list**, each because the sentence had become false:
   - ADR-031's Status gained a "Paused 2026-09-29" paragraph that routes to the new addendum;
   - ADR-033's "The first *caller* is M24" now reads "was to be M24, and is now M40's";
   - the map's 030 Status cell also had "M21 and M23 are part-built" (M23 closed 2026-09-27), so it
     now says M23 built §3's pool;
   - the 031 and 033 Status cells gained "paused" and "laptop-only" respectively.
2. **ADR-030's addendum covers two things the plan did not list**, both following from §5's
   supersession:
   - §6's capture sources split three ways: file/upload stays, USB moves with M21 to M40, and phone
     over Wi-Fi goes with M28;
   - the last Consequence's `config.py::REPO_ROOT` now dies at M40's packaging, because M26 moved
     whole.
3. **Two facts from ADR-002's addendum that M39 P0 should read:**
   - **The gate is a tolerance gate on purpose.** ADR-033's zero-tolerance determinism was measured
     on one desktop, and the phone has a different decoder and a different CPU architecture.
     Verdicts must still match exactly.
   - **Pinning matters beyond the comparison.** The `pose_estimator` stamp that `keypoints_for`
     compares names a variant (`mediapipe:heavy`) and not bytes, so a moved `latest` would change
     the laptop's instrument with no re-pose triggered.
   - The pinned variant is `heavy`, from `settings.pose_model_variant` and all 30 stored files.
     `ranges.json` is still cut from `mediapipe:lite` (`dataset_info().pose_estimator`, read on
     2026-09-29), so the gate compares the phone's heavy against the laptop's heavy, not against
     the bands' instrument.
4. **ADR-033's addendum names every M24/M26 sentence that now means M40**, rather than editing each
   one: clause 7's backlog and thermal budget, the fourth addendum's churn cost, pool width under
   load, contention with capture, and clause 9's bundling. `ready` names an estimator and a variant,
   not bytes. If M39 P0 wants the hash in the handshake, that is a protocol change and needs its own
   addendum to 033.
5. **Forward references this phase wrote, which later phases make true:**
   - ADR-030's addendum says what ADR-034 changes in ADR-016 "is recorded in ADR-016". **P7** writes
     it.
   - ADR-030's addendum says the screen parser's gating is "ADR-032's to record, by its own
     addendum". **P5** writes it.
6. **A pre-existing broken link, left alone.** ADR-033's fourth addendum links
   `010-scoring-model-bands-and-caveats.md`, and the file is `010-benchmark-ranges.md`. It is outside
   this phase's scope, so `/doc-check` or P12 should fix it.
7. **Line endings.** The working copy is CRLF, and a bash heredoc appends LF, which leaves a file
   mixed. P4 normalised the four ADRs back to CRLF. Git normalises on commit anyway, but later
   phases should append with the Edit tool or normalise afterwards.

### P5 — found (2026-09-29)

ADR-032 gained "Addendum, 2026-09-29 — who records each family, and the screen parser's edges are
M34's" (anchor `#addendum-2026-09-29--who-records-each-family-and-the-screen-parsers-edges-are-m34s`).
It has four sections: the oracle per family, the six parser edges, the `pyfmt` split, and what it
does not change. Its Status block now reads **twelve addenda**, lists the twelfth, and says §M29 is
blocked on M40. `docs/README.md` row 032 reads **12**, and the total moved **80 → 81** (`grep -c`
sums to 81).

`docs/CONFORMANCE.md` §5 changed in four places:

- an intro sentence routing to ADR-034;
- a new **"2 — the screen reader"** row: parser, validator and profiles port in M34, with
  `recognizer.py`'s `TextBox` as the seam, while `paddle.py`, `preprocess.py` and `importer.py` stay
  the lab's reader;
- tier 3 is **laptop-only**, and `coach.py` never reaches the phone;
- tier 4's §M29 is **blocked on M40**.

The program plan's M31 row reads 6/13. Docs-truth: 92 passed, and every link in the two edited
files resolves (scratch slug script).

1. **No vector family names its oracle today.** The `provenance` keys, read per family:
   - `format`: `edges`, `kind`, `note`;
   - `stages`: `derived_from`, `kind`, `note`;
   - `synthetic`: `kind`, `note`;
   - `corpus`: `frame_offset`, `kind`, `note`, `source_clip_frames`;
   - `audio`: `clip_duration_s`, `clip_samples`, `kind`, `note`, `source_sha256`.

   So M37's pin retrofits all five.
2. **There are three kinds of oracle, not two.** ADR-034 clause 9 names Python-recorded and hand.
   The program plan's M37 also has "Rust-recorded corpus vectors", which are a changelog rather than
   an oracle, and the addendum names them as a third kind. **M37 picks the `oracle` string for
   them.** The addendum fixes only `"hand"`.
3. **Three of finding 5's edges were checked on the Rust side** with `rustc`, from a scratch file:
   - float `//`: `1.0 // 0.1` is `9.0` in CPython, while `(1.0 / 0.1).floor()` and
     `f64::div_euclid` both give `10`. The hazard is not the sign, since `center_y` is non-negative.
     It is that CPython's float `//` is not the floor of the quotient. M34's tables should carry
     cases like this one.
   - `{:?}` always double-quotes, and it writes `\u{1c}` where `repr` writes `\x1c`;
   - `split_whitespace` does not split on U+001C, and CPython's `split()` does.
4. **The parser also reaches two of ADR-032 §3's own edges**: `:g` and `.3f` in `validate.py`'s
   warnings, and `round(parsed.confidence, 3)` in `to_shot_data`. That is the concrete reason for
   the `pyfmt` split, and neither finding 5 nor M34 had stated it.
5. **The split has a precedent, and no ledger row.**
   - `crates/feedback` already re-spells `pyfmt::percent`, pinned by
     `the_percent_fallback_agrees_with_pyfmt`. It also re-spells `phases::first_argmax` as
     `first_max_by_tail`, because it cannot import `analysis`.
   - `docs/REFACTOR_LEDGER.md` has no `pyfmt` row, so the addendum says "planned, not decided".
6. **What the plan did not list, and why it changed.**
   - "What this does not cover" had called `coach.py` **tier 4**, while §5's table puts it in tier 3.
     The bullet is corrected, with the old wording named.
   - Tier 4 kept `store.py` and `source.py` from the screen package as **open**. The program plan's
     M36 puts a shot store in `crates/storage`, but that is TARGET, so nothing was decided here.
7. **Left alone, for `/doc-check`**:
   - `docs/README.md`'s CONFORMANCE row says "**five** Rust-specific edges" and lists five, but §3
     has six (it omits `max`'s first maximum). That row names no §5 disposition, so P5 did not
     touch it.
   - ADR-032 §7's body still names M24 and M25. The addendum routes them, and the body stays as the
     record.
8. **For P7**: ADR-014's addendum can cite this addendum's edge 5 (`_find_labels`' `>`) as the
   mechanism behind the `Impact Position V` hazard. ADR-030's forward reference ("ADR-032's to
   record, by its own addendum") is now true.

### P6 — found (2026-09-30)

Two addenda, each dated 2026-09-30 (the day written) and headed in ADR-010's `(date, M31)` form,
which ADR-009 had no addendum of its own to set:

- ADR-009 (its first): "the outcome axis is graded over many shots, and intent decides which shots
  count";
- ADR-010: "not printed is not unscored, and shot metrics get no bands".

Neither ADR counts its own addenda, so no Status block changed. `docs/README.md` rows 009/010 read
**1**/**9**, and the total moved **81 → 83** (`grep -c` sums to 83). The program plan's M31 row reads
7/13. Docs-truth: 92 passed, and every link in the two ADRs and the map resolves (scratch slug
script). **P7 starts from 83 → 86.**

1. **`policy_for` is recorded as open, not decided.** P3's finding 4 said "P6's ADR-009 addendum
   records which" option M35 picks, but M35 has not run. The addendum states the constraint (a
   `DRILL` or `SHOT_SHAPING` shot with video raises today), gives the choice to M35 per ADR-034's
   Consequences, and says **M35 records its answer by a further addendum to ADR-009**. That will be
   ADR-009's second addendum, and the map's row and total move with it.
2. **One constraint the addendum draws from ADR-034, which M35 should check before relying on it.**
   Clause 1 scores the pose checkpoints "exactly as today", and clause 5 leaves the single-swing
   policies unbuilt. Read together, neither of M35's options is a new weighting: the mode decides
   whether a shot counts and which shape it is graded against, never how a swing's mechanics score.
   ADR-034 does not say this in one sentence; the addendum says it is read from the two clauses.
3. **Three sentences of the original ADRs became false, and the addenda name them** rather than
   leaving a reader to reconcile them with ADR-034:
   - ADR-009 §Concepts: intent flows in "as range parameterization". Shape intent instead selects a
     topic, and one face-to-path rule classifies every shot. `analysis/checkpoints/outcome.py`,
     drawn in ADR-009's code shape, has never existed and now will not.
   - ADR-009 Option C's per-swing "policy-weighted blend of the two sub-scores": nothing weights
     mechanics against outcome anywhere.
   - ADR-010 §3's outcome-axis keying, §4's TrackMan/Arccos per-club outcome norms, and the last
     Consequence's "intent selects *which* range applies". All are superseded for shot metrics and
     stand for the mechanics panel. §4's own "personal baseline" line is the route shot metrics
     take.
4. **For M35/M37: `refilming_helps` has no photo-side meaning yet.** "Printed but blank → `unscored`"
   reuses `contracts/unscored.py`, whose load-bearing bit asks whether *re-filming* helps. On a
   photo the question splits the same way: `---` printed by the device (a new photo cannot help)
   against an OCR misread (a new photo would). No existing reason names either case. The ADR-010
   addendum does not decide it, and says only that "not printed" is not a new reason, because it
   produces no entry at all. The shot-side reasons and their bit are M32's or M35's to add.
5. **A stale code comment, left for M35/M37** (M31 changes no code): `analysis/engine.py`'s
   docstring near line 506 says outcome checkpoints "need per-club benchmark bands `ranges.json`
   does not have". Under ADR-034 5.6 they never will, because shot metrics are not banded.
   `contracts/intent.py`'s `SHOT_SHAPING` and `TargetShape` comments ("full M4") are stale in the
   same way.

### P7 — found (2026-09-30)

Three addenda, each dated 2026-09-30 and headed in the `(date, M31)` form P6 used, which ADR-016's
`(2026-09-21, M18)` had set:

- ADR-014: "the screen is read on the phone, what it prints is decided per golfer, and the first
  addendum named the wrong cause";
- ADR-016: "the phone is the host and listens on nothing, and this topology is the lab's";
- ADR-024: "the shot is the unit, a drill is not tracked, and the landing offline is a second
  projection".

None of the three counts its own addenda, so no Status block changed. `docs/README.md` rows
014/016/024 read **2**/**3**/**3**, and the total moved **83 → 86**. `grep -c` sums to 86, the
plan's post-M31 total, so **every addendum M31 planned now exists**. The program plan's M31 row
reads 8/13. Docs-truth: 92 passed. Every link in the three ADRs and the map resolves, checked by a
scratch slug script that was first shown to catch a broken anchor.

1. **P4's forward reference is now true.** ADR-030's addendum says what ADR-034 changes in ADR-016
   "is recorded in ADR-016", and ADR-016's addendum records it. ADR-014's addendum cites ADR-032's
   edge 5 by its anchor, as P5 finding 8 suggested.
2. **The corrected cause has more evidence than P2 gave**, read on 2026-09-30 from the stored
   `provenance.warnings` (a Python one-liner over `data/processed/shots/*.shot.json`):
   - **6 of the 13 stored shots** carry "screen outline not found - parsing the photo uncropped",
     and only `2026-08-10-1` shows the `CENTER` spill. An uncropped frame does not produce the bug;
   - the 2026-08-23 session's first shot (`HEEL`, where P2 saw the real box win) has no outline
     warning, so the dropped `V` also happens on a rectified frame;
   - `2026-08-10-1`'s stored warnings include "Impact Position: no value text under the label",
     which is the V box winning the field;
   - the contaminated string still normalizes to a fade because
     `shot_measure.normalize_shot_shape` tries curvature words first (`_SHAPE_TOKENS`). P2 said
     this; the addendum names the function.
3. **What the addenda say that the plan did not list**, each because an existing sentence had
   become false, or because a reader would otherwise conflate two things:
   - **ADR-014**: the surviving `no tile found for 'Bounce & Roll'` warning now meets clause 2,
     because it tells a golfer about a stat their screen does not show. **M32 decides** whether it
     stays in `provenance.warnings` and stops reaching the golfer, or stops firing; the addendum
     does not. It also records that M33's gate re-checks the confidence blend's OCR term for
     Vision, and that §Sign conventions now decides a grade, so a left-hander's printed words are
     unverified.
   - **ADR-016**: the M18 addendum's "the phone talks to a laptop" and "what the phone sends"
     paragraphs are superseded with ADR-030 §5. The map's Status cell, "one clause superseded", is
     still true of the body, so it was left alone. How the export file travels (M38 P4) is **not**
     assumed to be the upload route, so the addendum claims only that the API key never reaches the
     phone, not the upload token.
   - **ADR-024**: a table separating the three exclusions §4's statistics now see: untagged (no
     club, still feeds pose metrics), mishit (automatic, carry and total) and untracked (`DRILL`,
     every aggregate). And M35's per-shot `PracticeGoal` keeps `club` at `ALL`, by the
     Alternatives' own trap.
4. **For M37, two notes the ADR-024 addendum puts on the record without deciding:**
   - **The on-line tolerance flatters a share.** ADR-024's Consequences justify the single offline
     tolerance, set at the widest club, as erring wide, which is safe *for a claim* because it costs
     claims. For a share of good shots it inflates the grade, most for the shortest clubs. That is a
     reason, on the record, for 5.3's "unless M37 finds a reason otherwise".
   - **`start_line_offline_yds` reads as club-only.** Clause 4 lists start line (degrees) as
     club-independent, but the offline in yards scales with carry. M37's per-stat scope places it.
5. **ADR-024 §3's deferred flight model had no addendum saying it was built.** ADR-027's Context
   lifts the deferral from its own side, and ADR-024's new addendum now points at it, as the model
   the landing projection flies through. §3's "`shot_measure.py` still refuses to record"
   `spin_axis` is stale too, and is left to `/doc-check`.
6. **Line endings.** ADR-014, ADR-016 and both plans are LF in the working copy, while ADR-024 and
   `docs/README.md` are CRLF. A `sed -i` turned the map to LF, so P7 converted it back. P8 should
   check `ROADMAP.md`'s endings after editing it.

### P8 — found (2026-09-30)

Only `ROADMAP.md` changed, plus this checklist and the program plan's M31 row (**9/13**). No file
and no addendum was added, so nothing in `docs/README.md` moved. Docs-truth: 92 passed.

- **The group**, `# Shot-first, phone-first (M31–M40)`, sits immediately before `# The app — …`: a
  two-paragraph preamble routing to ADR-034 and the program plan, then ten sections, each with
  Status, The ask, Depends on, Where it runs, Exit and Detail.
- **The rows**, after M30's: M31 `🟡`, 9/13; M32 `⬜ Not started` (needs M31); M33–M40 `🔒 Blocked`,
  each with its program-plan dependency and where it runs.
- **`## Last Updated`** reads 2026-09-30, and a NEXT ACTION paragraph is prepended above the
  bay-session one, which stays.
- **Every link in `ROADMAP.md` resolves**: the 30 new ones (25 distinct targets) and every
  pre-existing one. A scratch
  GitHub-slug script checked them, after it was first shown to catch a broken anchor and a missing
  file. It also resolves all the old rows' anchors, and that is the evidence its slug rule matches
  the house one. The rule is: lowercase, drop every character but letters, digits, `_`, `-` and
  space, then spaces → `-`. So ` — ` becomes `--`, and `M31–M40` becomes `m31m40`.

1. **The program plan gives an exit line for four of the ten**: M31, M32, M34 and M38. M33's and
   M39 P0's gates serve as theirs. **M35, M36 and M37 name pins and vector families instead, and
   M40 names nothing.** Their ROADMAP Exit lines say exactly that, and that the milestone's own plan
   sets the exit, rather than invent one. Each of those plans should replace the line when it is
   written.
2. **The M31 row carries a phase count, so P9, P10 and P11 each move it** along with the program
   plan's row, until P12 marks both done. The M31 *section* routes to this plan instead of
   carrying the count, which leaves one fewer copy to drift.
3. **"M38 P5's photo session" is written as conditional.** P5 needs the app M38 builds, so the next
   bay trip is that session only if M38 exists by then. The NEXT ACTION says "once there is an app
   to take it with". It also says why the `Custom` enumeration matters, from ADR-034 §5.3: a printed
   landing offline replaces the projection, and a printed spin axis is what lets the projection
   reach a shot at all.
4. **Left for P9.** No old section or row changed. M24's row still reads "M21 only", M29's "M25
   only", M5's "superseded in shape by M25", and M28 has no banner. The new text states only what
   defines the new milestones: M40's row and section say M21, M24, M25 and M26 are re-scoped under
   it, and the preamble routes to ADR-034's Consequences for the full list of moves rather than
   copying it. The anchors P9's banners will want are `#m37-strike-profile-topic-grades-and-strengthsweaknesses-rust-first`,
   `#m38-the-iphone-app`, `#m39-optional-video-on-the-phone` and `#m40-the-laptop-client-resumes`.
5. **M35's section carries ADR-034's two open questions**: how a non-`FUNDAMENTALS` mode reaches
   `policy_for`, and how handedness reaches a photo with no swing. Both are the kind a later session
   would otherwise rediscover. P6 finding 1 already says M35 records its policy answer by a further
   ADR-009 addendum.
6. **Line endings.** `ROADMAP.md` is CRLF in the working copy (`git ls-files --eol`: `i/lf
   w/crlf`), and the Edit tool kept it so: 4,354 CRLF out of 4,354 newlines. **Git Bash's `grep -c
   $'\r$'` and `cat -A` both report this file as LF**, so check with Python's byte counts, not
   those. The user's one-word `/plan-phases` edit at §M30 is still in the diff, as the non-goals
   said it would be.

### P9 — found (2026-09-30)

Only `ROADMAP.md` changed, plus this checklist and the program plan's M31 row (**10/13**). No file
and no addendum was added, so nothing in `docs/README.md` moved. Docs-truth: 92 passed.

- **Ten status rows**:
  - M3 `✅ Closed`;
  - M4 full `❌ Superseded` for the outcome axis;
  - M21, M24, M25 and M26 `⏸ Paused` under M40;
  - M28 `❌ Superseded` by M39;
  - M29 blocked on M40;
  - M5 superseded in shape by M38;
  - M31 at 10/13.
- **Ten banners**, each a blockquote directly under its heading: the same nine sections, plus the
  `# The app (M18–M28)` preamble. No body text was rewritten.
- **The done-when grep** now hits three lines: the `**Status**` lines of M25, M26 and M28, each
  under its banner.
- **Every link in `ROADMAP.md` resolves**: 266, 0 broken. The scratch slug script was first shown to
  catch two deliberately broken anchors.
- **Line endings**: `ROADMAP.md` is still CRLF throughout (4,481 of 4,481, by Python's byte counts).

1. **Headings were kept, stale words and all.** M3's still ends "— in progress", and its banner says
   so and why. The status table links every heading by its slug, and ADR-033 links two of them
   (§M26, §M29), so renaming one breaks links elsewhere. That is also the file's own convention:
   "Detail sections keep their original wording". A rename, if wanted, belongs to `/doc-check`, and
   it must move the anchors with it.
2. **Dates: the state cells and banners carry ADR-034's date, 2026-09-29**, not the day written.
   ADR-031's Status already reads "Paused 2026-09-29", and one event should have one date. The
   plan's "dated the day it is written" rule is about addenda, and `## Last Updated` stays
   2026-09-30.
3. **Glyphs.** `⏸` is new to `ROADMAP.md`, and the plan named it. No "superseded" glyph existed, so
   M4 full and M28 use M27's `❌` form. M3 uses `✅ **Closed**` because its built half is done, not
   withdrawn.
4. **M3's "Connect MCP server to analysis engine data merger" is retired, not routed.**
   - It was `docs/FLOW.md` §3's `MCP --> MERGE` arrow: shot data reaching the engine *through* the
     MCP server, into a `merge.py` timeline.
   - The join was built another way. The swing directory's `SHOT_SCREEN` role (`storage/manifest.py`)
     leads to `api/pipeline.py::_shot_for`, which reads `ShotStore` by the photo's hash and passes
     `shot=` to the engine. `mcp/query.py` then reads the joined shot back out of `analysis.json`,
     so the MCP server reads the engine and does not feed it.
   - What `merge.py` still names is the detection stream, which stays parked with M4 full on M2.
   - FLOW §3 still draws the arrow. **P11's** pre-pivot banner on §3 covers it, and the banner
     could name it.
5. **M4 full, item by item.** Two of its lines were already false before the pivot, and the banner
   names both rather than editing them:
   - "Populate `SwingResult.shot` — the field exists and has never been set": it is set today
     (`analysis/engine.py`, the `SwingResult` construction).
   - "Store results in SQLite (M7 Phase 3 builds the store)": M7 Phase 3 shipped a file store,
     `storage/bundle_store.py`. SQLite is **not routed**. ADR-034 §6's on-device storage is M36's and
     M38's to shape, and neither names SQLite.
   - The face-angle checkpoint stays parked on M2 as written, because it was the *video-detected*
     face. HD Golf already prints a face angle, so M37 may want to say whether the parked checkpoint
     is still wanted.
6. **M28's open question has no home yet.** Its thermal cost of recording on an iPhone passes to
   M39 P1, which records on the phone, but the program plan's §M39 does not name it. **M39's plan
   should.** M28's banner says so.
7. **Two moves the plan did not list, both following from ADR-034's Consequences:**
   - `api/pipeline.py`'s orchestration, which §M29's body gives to M24, moves to M40 with M24;
   - M26's status line, "CI can start as soon as there is a `Cargo.toml`", goes with M26, because
     ADR-034 moves CI with it.
   Both banners say so.
8. **Left alone, as outside this phase or already historical:**
   - the `# In progress` group preamble, which still says OCR tuning "is the remainder of M3", and
     whose M6 half was stale before the pivot. `/doc-check` should fix it;
   - the older NEXT ACTION paragraphs naming "M3's remaining OCR work", which P8 chose to keep;
   - the M22 and M23 rows' "no caller until M24". These are done milestones' records, and ADR-033's
     addendum routes the caller;
   - the app group's second paragraph, which says M20 is done while M20's row reads 6/7. That was
     wrong before the pivot, and the new banner states M20 correctly;
   - the `Software / Hardware Tracks` table.
9. **For P10 and P11:** the M31 row reads 10/13 in both places, and each of them moves it.

### P10 — found (2026-09-30)

`CLAUDE.md` and the root `README.md` changed, plus this checklist, the program plan's M31 row and
`ROADMAP.md`'s M31 row (**11/13** in both; the ROADMAP row's "what is left" now names only the
charter and `FLOW.md`). No file and no addendum was added, so nothing in `docs/README.md` moved.
Docs-truth: 92 passed. All 14 links in the two files resolve, checked by a scratch slug script that
was also shown to catch a broken anchor and a missing file.

- **`CLAUDE.md`**, the plan's three edits:
  - the product paragraph keeps its as-built sentences and gains one: that is what *runs*, the
    product is now shot-first and phone-first (ADR-034), and none of it is built;
  - a new row, **"What is the product now, and why did it pivot?"**, placed directly above the
    platform row. It routes to ADR-034, then the program plan, then `ROADMAP.md`'s status table,
    and copies no status;
  - the **"What is the app written in"** row reads ADR-030 *as amended by ADR-034*: the iPhone app
    with the core through Flutter and `flutter_rust_bridge`, pose behind M39's gate (the laptop
    computing mechanics if it fails, ADR-034 §8), and the sidecar laptop-only for M40.
- **`README.md`**:
  - the blurb is two sentences of direction, then the old one-line description kept word for word
    as "what runs today". The two "Working today" paragraphs are unchanged;
  - the NEXT ACTION is a pointer. ROADMAP's NEXT ACTION has no heading of its own, so the link is
    `ROADMAP.md#status-at-a-glance`, and the text says it sits under that table;
  - **the milestone table is gone**, replaced by a link to *Status at a glance*. No table stayed,
    so no M31–M40 rows were needed. Nothing in the repo linked to the table.

1. **One edit beyond the plan's list, because the sentence had become false.** `CLAUDE.md`'s Rust
   paragraph said the M22 crates were "wired to nothing until M24". It now says their first caller
   is the phone app (M38), because ADR-034 paused M24 under M40. The source is ADR-032's 2026-09-29
   addendum: "the core gains callers on the phone (M38) before it gains one on the laptop". The
   ROADMAP M22/M23 rows' "no caller until M24" stay, as P9 finding 8 decided for done milestones'
   records. `CLAUDE.md` is present-tense guidance, not a record.
2. **Left for `/doc-check`: the README is stale below its first screen, and none of it is the
   pivot's doing.** Each item was already false before 2026-09-29:
   - Tech Stack says the UI is JavaScript/React (M5). ADR-030 chose Flutter, and M5's screens are
     now M38's;
   - Tech Stack says the pose variant is **lite**. `config.py`'s `pose_model_variant` is `heavy`,
     and only `ranges.json` is cut from lite (P4 finding 3);
   - Tech Stack's SQLite row reads "reserved". P9 finding 5 found SQLite routed nowhere;
   - Architecture says "Nine packages" and never mentions the Rust workspace;
   - Project Structure lists `frontend/` as the React UI and `decisions/` as "ADRs 000–016";
   - Decision Log says "16 of them", and there are 35 ADR files;
   - "The five working CLIs" introduces seven;
   - the "Working today" paragraph says "six checkpoints", a count in prose. The plan kept that
     paragraph unchanged, so this phase did too.
3. **`docs/README.md` line 46** describes the root README as "what this is, what state it's in, and
   the three commands". It still says what state the product is in (mid-pivot, and what runs). Where
   each milestone stands is now a link, which is close enough that the row was left alone.
4. **Line endings**: `README.md`, `CLAUDE.md` and `ROADMAP.md` are CRLF throughout (Python's byte
   counts); both plans are LF throughout. The Edit tool kept each file's form.
5. **For P11:** the charter and FLOW §4 should draw the host the way the README's blurb and ADR-034
   §6 describe it: the phone holds storage, and export (M38 P4) is how data leaves it. The README
   does not draw a diagram, so FLOW §4 is the only picture of the new topology.

### P11 — found (2026-09-30)

`docs/PROJECT_CHARTER.md` and `docs/FLOW.md` changed, plus this checklist, the program plan's M31
row and `ROADMAP.md`'s M31 row (**12/13** in both; the ROADMAP row's "what is left" now reads "the
close"). No file and no addendum was added, so nothing in `docs/README.md` moved. Docs-truth: 92
passed. All 15 links in FLOW and 10 in the charter resolve, by the scratch slug script, which was
shown again to catch a broken anchor and a single-dash spelling of ADR-033's em-dash anchor.
**Both redrawn mermaid blocks parse, checked by a renderer rather than by eye**: `mmdc` 11.16
(mermaid-cli) is installed on this box at `%APPDATA%\npm\mmdc`. It renders all five FLOW blocks, and
it exits 1 on a copy of §1 with one label left unclosed. GitHub's renderer was not checked. Both
files are CRLF throughout, by Python's byte counts.

- **Charter** (§1–§6 untouched):
  - **Last Updated** is 2026-09-30. It records the second scope move (ADR-034, 2026-09-29) beside
    the first (ADR-030), and it says what changed: §1 still names both halves, but the launch
    monitor now leads.
  - **§0** gains a new first bullet: the shot is the unit, video is optional, mechanics are a
    separate panel, and the biggest unknown moves from the club head to whether the phone can read
    the screen (M33). The intro now reads "three sharpened, a fourth has turned the product round".
  - The **dual-axis** bullet gains the ADR-034 reading: `outcome_score` stays `None`, and outcome
    is per-topic shares at club and player level. The axes are no longer combined, and intent now
    decides which shots count and which shape is graded.
  - The **"Mobile app"** bullet keeps its ADR-030 record and adds the second move: the phone is
    the host (clause 6), pose waits behind M39's gate, and the laptop is M40's.
  - **Multi-user** stays out of scope: no accounts, no sync, and the golfer picker is names on one
    phone rather than logins.
- **FLOW**:
  - The banner reads "Last reviewed 2026-09-30" and says §1 and §4 are redrawn and §2 and §3 are
    pre-pivot. The tier is still on line 3.
  - **§1** has every milestone from M0 to M40 in four groups: the spine, the lab, the app platform
    and the pivot. M6–M17 are one node, which covers M6.5, Career mode and the Hands spike. It
    uses two new classes beside the old four: `paused` (blue) and `retired` (grey, dashed) for
    superseded or closed milestones. The prose under it is rewritten: the critical path, what the
    pivot paused and left built, and hardware.
  - **§2 and §3** have dated pre-pivot banners. §3's banner names the `MCP --> MERGE` arrow as
    wrong, following P9 finding 4.
  - **§4** is redrawn as the plan listed it: the iPhone, then export, then the laptop lab, and the
    M40 client on the same laptop. Its prose covers the phone listening on nothing, the lab's ports
    and the M40 client.

1. **The old §1 prose was false before the pivot, and a redrawn map could not sit above it.**
   FLOW's own rule is that when a marker and a sentence disagree, the sentence is the bug. The old
   text had four such errors:
   - "M1.5 is the oldest unstarted item": it ran on 2026-08-14 and said no-go.
   - M3 "MCP server pending", M6 ⬜ and M7 📋: all three are stale.
   - "M2 needs a global-shutter camera", and an `HW -.-> M2` edge marked "required". ROADMAP §M2
     and ADR-018 say M2 needs lighting, not a camera.
   All of them are rewritten. The hardware edge now reads "lighting for a ~1/2000 s exposure".
2. **Edges beyond the plan's list, each with a source.**
   - **Dependencies**:
     - M34 -.-> M38, "skeleton", and M33 -.-> M38, "go / no-go before app work", both from the
       program plan's status checklist and its M33 note;
     - M40 --> M29.
   - **"Where it went"** edges, all from ADR-034's Consequences:
     - M3's OCR items → M32;
     - M4 full → M37;
     - M5 → M38;
     - M28 → M39;
     - the paused four → M40.
   - **Reuse and history**:
     - M20 → M39, because M39 P1 runs `crates/trigger` on the phone;
     - M21 → M28, "split out", from ADR-031 §8.
   - **Layout only**: three invisible links (`~~~`) put the four groups in order top to bottom, and
     they claim nothing.
   - `~~~` and an edge that ends on a subgraph (`PAUSED -.-> M40`) are both newer mermaid syntax.
     mmdc 11.16 renders them.
3. **The old §4 was wrong about the lab, not just old.**
   - It drew the MCP server on port 8081 and fed it from the R10. The server speaks stdio, and
     `mcp_port` binds nothing (`mcp/server.py`, `scripts/run_mcp_server.py`). The prose now says so.
   - It drew a SQLite file that nothing routes to (P9 finding 5). The new drawing leaves it out.
4. **§4 draws three things the plan did not list**:
   - the phone-browser upload over Tailscale, kept as the lab's path until M40, per ADR-016's
     2026-09-30 addendum;
   - the M40 client as the pose sidecar's first caller, per ADR-033's 2026-09-29 addendum;
   - a dotted edge from the phone's clip to the laptop client for the case where M39's gate fails
     (ADR-034 §8).
   Nothing on the phone carries ✅. The Rust crates exist, but nothing on a phone runs them.
5. **For P12: FLOW §1's M31 node is a status marker.** It reads 🟡 with class `wip`. P12 should
   flip it to ✅ and move it to `done` in the same change that marks M31 done, because FLOW treats
   its markers as the source of truth. P12's step 3 does not name FLOW, so add it there. M32's ⬜
   stays correct once M31 is done.
6. **The picker sentence claims less than the plan's wording.** The source is the program plan's
   M38 P2 ("pick golfer and bag"). The charter says it is local names and not logins. It does not
   say how M38 stores them, because on-device storage is M36's and M38's to shape.
7. **Left for `/doc-check`, each false before the pivot**:
   - the charter's closing paragraph says "the M1.5 spike that would settle it has not been run".
     It ran on 2026-08-14. The new §0 bullet routes around it rather than editing the risk
     paragraph;
   - FLOW §5 says "today `scripts/` fills that role", but the orchestrator has been `api/pipeline.py`
     since M7 Phase 5, as FLOW §2's own prose says. §5 also calls it the React `frontend`;
   - FLOW §6's "Neither exists yet", which its own last paragraph half-corrects;
   - FLOW §4's timing table. It is the old product's swing-to-feedback latency, still labelled
     never measured. It was left as it is, because the plan did not list it and it claims nothing
     false.

### P12 — found (2026-09-30)

**M31 is closed.** All six verify commands are green, each run unpiped and one at a time, with
`cargo test` after pytest had finished rather than beside it:

- `pytest`: **1,991 passed** (docs-truth 92). The one warning is `.pytest_cache` being unwritable
  on this box, and it is not a test;
- `ruff check src tests scripts`: clean;
- `mypy src`: 118 files clean;
- `conformance.py check`: **21/21 at v16**;
- `cargo test`: exit 0, **30 binaries, 582 passed, 0 failed**, tallied from the `test result:` lines
  of a file rather than through a pipe;
- `cargo clippy --all-targets` and `cargo fmt --check`: both clean.

These are the counts M23 P9 closed on, so the docs-only claim holds. Docs-truth passed again after
the close's own edits.

**The three checklists agree**, and FLOW is a fourth:
- this plan: every phase ✅;
- the program plan's M31 row: `✅ Done`, 13/13;
- `ROADMAP.md`: the M31 row reads `✅ Done`, 13/13, and its "needs to start" cell reads "— (desk
  work; done)". The §M31 Status line reads `✅ Done`. M32's cell reads "Nothing — this box, which
  holds `data/` and the `ocr` extra";
- **`docs/FLOW.md` §1**, as P11 finding 5 asked: the M31 node reads ✅ and has moved from `wip` to
  `done`, and M32 stays ⬜ `planned`. mmdc 11.16 still renders the block.

`WORKLOG.md` gained one entry, on M23 P9's pattern: what M31 decided, what P2 corrected, what the
docs had wrong, and what M32 should read first. The auto-memory's `app-transition-decisions.md`
now names ADR-034 as the authority and carries the vocabulary, the drill rule, the grades and
blends, declared-only shapes, the one sanctioned inference, and M32 as next. Its `MEMORY.md` index
line moved with it. `/doc-check` was not run.

1. **Three edits beyond the step list**, each because a sentence became false when M31 closed, or
   because an earlier phase routed it here:
   - **The program plan's header** said M31–M40 "do not exist in `ROADMAP.md` yet" and called
     ADR-034 "to be written by M31". It now says M31 wrote both, and links the ADR;
   - **ROADMAP's NEXT ACTION** said "Once M31 closes". It now says M31 closed on 2026-09-30. Its
     heading, "the pivot, then M32", was left alone;
   - **ADR-033's fourth addendum** linked `010-scoring-model-bands-and-caveats.md`, and it now links
     `010-benchmark-ranges.md`. P4 finding 6 routed it to "`/doc-check` or P12". It changes no
     addendum count.
2. **Everything else the findings routed to `/doc-check` is still there**, and the WORKLOG entry
   lists it in one place: the README below its first screen, FLOW §5 and §6, the charter's M1.5
   sentence, ROADMAP's `# In progress` preamble, and `docs/README.md`'s CONFORMANCE row ("five"
   edges) and M7 row.
3. **Line endings**: `WORKLOG.md`, `ROADMAP.md`, `docs/FLOW.md` and ADR-033 are still CRLF
   throughout, and both plans are still LF, by Python's byte counts.
4. **Nothing was committed.** The working tree holds all of M31, plus the user's own unrelated
   edits. The non-goals named three of them: the `/plan` → `/plan-phases` rename, and the one-word
   edits to `ROADMAP.md` and `docs/plans/m23-pose-sidecar.md`. A fourth, the untracked
   `.claude/skills/orchestrate/`, appeared after planning. The user decides whether they go in with
   M31's commit or on their own.
