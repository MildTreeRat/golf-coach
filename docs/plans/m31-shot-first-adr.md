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
| **P1** | Fold the interview's decisions into the program plan | ⬜ Not started |
| **P2** | Verify the facts ADR-034 will cite (read-only) | ⬜ Not started |
| **P3** | Write ADR-034 | ⬜ Not started |
| **P4** | Addenda, "where it runs": ADR-030, 002, 031, 033 | ⬜ Not started |
| **P5** | Addendum, "how it is checked": ADR-032, plus `docs/CONFORMANCE.md` §5 | ⬜ Not started |
| **P6** | Addenda, "how it is judged": ADR-009, 010 | ⬜ Not started |
| **P7** | Addenda, "how a shot arrives and is filed": ADR-014, 016, 024 | ⬜ Not started |
| **P8** | `ROADMAP.md`: the new M31–M40 group and its status rows | ⬜ Not started |
| **P9** | `ROADMAP.md`: re-scope the milestones the pivot moves | ⬜ Not started |
| **P10** | `CLAUDE.md` and the root `README.md` | ⬜ Not started |
| **P11** | `docs/PROJECT_CHARTER.md` §0 and `docs/FLOW.md` §1/§4 | ⬜ Not started |
| **P12** | Close: verify, WORKLOG, checklists, memory | ⬜ Not started |

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
