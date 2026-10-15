# Plan: M37 — Strike profile, topic grades and strengths/weaknesses (Rust first)

**Tier: TARGET.** This is a plan, not a record of what was built. Verify every claim in it against
code before acting on one. Phase findings are appended as phases close, and a finding contradicting
the plan wins.

**Planned**: 2026-10-10. **Milestone**:
[ROADMAP §M37](../../ROADMAP.md#m37-strike-profile-topic-grades-and-strengthsweaknesses-rust-first).
**The design is the program plan's
[§M37](m31-m40-shot-first-pivot.md#m37--strike-profile-topic-grades-and-strengthsweaknesses-rust-first)**,
with Decisions 8–12 and 17 above it, and M35's handoff ("What M37, M29 and M38 should know" in the
top WORKLOG entry of 2026-10-10). This file is its phase list. Where this file and §M37 disagree,
this file's interview decisions, findings and calls say why, and **this file wins**. The biggest
disagreement is strike location: §M37 reads it off the printed `impact_position` word, and this
plan grades it only from a measured face-impact offset that a new milestone, M42, will read from
the iPhone Live Photo of Full Swing's impact video (decision 3).

**Governing decisions**: [ADR-034](../decisions/034-shot-first-phone-first.md) §2 (printed / not
printed / printed blank), §3 (tracked), §4 (club and player), §5 whole (the grades, the six topics,
the shapes, the blends, ungraded named and impossible absent, no tour bands, the list), §9 (the
oracle per family); [ADR-024](../decisions/024-per-club-shot-history.md) and its M35 addendum (the
corpus); ADR-027 (the flight model behind the projected landing); ADR-028 (mishits);
[ADR-010](../decisions/010-benchmark-ranges.md) §2 (no score beats a wrong one);
[ADR-035](../decisions/035-rust-everywhere-python-where-required.md) clauses 3 (Rust records) and 4
(frozen Python); ADR-008 (as cargo edges).

**Level: L3.** A new contract module (`strike.rs`), a shape change to `CorpusSwing` under a new
`CAREER_VERSION`, a new device entry in `devices.json`, a new vector family, a new `feedback`
module, a new `golf-core` verb, and the first analysis written Rust first with no Python twin at
all. Every phase reads its own list below rather than the repo.

---

## Status checklist

**Stop after every phase. Every time.** A phase is one session. When it is done:
1. tick its row here, in the same change as the phase;
2. append what it *found* under "Phase findings" at the foot — the findings are what the next phase
   is planned against — and address anything a later phase must know to that phase by number
   ("**For P9**: …");
3. stop, and tell the user to `/clear` and run `/next-phase`.

**Nothing is committed until P13** (call 15, as M35). Every phase leaves its work in the working
tree, and P13 makes one commit onto `main`.

| Phase | What | State |
|---|---|---|
| **P0** | This plan document, the pointers to it, and the M42 stub | ✅ Done *(2026-10-10)* — approved by the user |
| **P1** | `crates/contracts` I: `strike.rs` — the shapes, the constants, the stat scopes; the `synthetic` device; the `data/` snapshot | ✅ Done *(2026-10-10)* |
| **P2** | `crates/analysis` I: the Wilson interval, and the per-shot judgments | ✅ Done *(2026-10-10)* |
| **P3** | `crates/contracts` II: `CorpusSwing` carries the shot and its shot-side refusals (shape only) | ✅ Done *(2026-10-10)* |
| **P4** | `crates/storage`: `read_corpus` lifts them; `CAREER_VERSION` 3; `career-v3` | ✅ Done *(2026-10-11)* |
| **P5** | The hand-worked profile vectors, parsed and skipped by name until P9 | ✅ Done *(2026-10-14)* — after the user's answer to finding 12 |
| **P6** | `crates/analysis` II: the per-club strike profile and topic grades | ✅ Done *(2026-10-14)* |
| **P7** | `crates/analysis` III: player pooling, and the two blends | ✅ Done *(2026-10-14)* |
| **P8** | `crates/feedback`: `rank_profile`, the strengths and weaknesses | ✅ Done *(2026-10-14)* |
| **P9** | `crates/core` I: `run_profile`, the profile family's runner and gate | ✅ Done *(2026-10-14)* |
| **P10** | `crates/core` II: `golf-core profile`, its report and `--json` | ✅ Done *(2026-10-14)* |
| **P11** | The profile family's corpus cases; `rerecord` walks the family; the oracle pin | ✅ Done *(2026-10-15)* — resumed after the user's answer to finding 3, (b) |
| **P12** | Docs: CONFORMANCE, ARCHITECTURE, the ADR addenda, the program plan, `CLAUDE.md` | ✅ Done *(2026-10-15)* |
| **P13** | Close: verify, WORKLOG, ROADMAP, memory, and the one commit | ✅ Done *(2026-10-15)* |

---

## Rules every phase keeps

- **Every phase ends green on everything it can reach.** If a phase cannot end green, stop and
  record why as a finding; do not carry a red suite into the next session. A vector a phase cannot
  pass yet is skipped *by name* in its gate, with the phase that will pass it, never by a blanket
  ignore.
- **Frozen Python is untouched outside `scripts/conformance.py` and `tests/test_conformance.py`**
  (ADR-035 clause 4), and those change only for what this plan names: `check`'s deferral of the
  profile family to `cargo test` (P5 or P9, whichever first adds the directory). `git diff -- src/`
  is empty at every phase.
- **Nothing under `data/` is written by any phase.** P10's verb only reads. P1 takes the `data/`
  digest (method as M35's finding 11; expected still `bd384cc4…` over 192 files) and P4, P10 and P13
  compare against it.
- **Only these write `spec/vectors/`**: P4's `golf-core rerecord` under
  `spec/declarations/career-v3.json` (the storage and career families); P5 writes new
  `spec/vectors/profile/hand/` cases by hand (or by a throwaway scratchpad script the phase deletes);
  P11 writes new `spec/vectors/profile/corpus/` cases and may re-run `career-v3`. Any other change
  under `spec/vectors/` is a finding and a stop. **The engine, stage, shot, screen, audio and format
  families do not change in this milestone**:
  `git diff -- spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}` is empty at every
  phase. `ANALYSIS_VERSION` and `SCREEN_PARSER_VERSION` do not move.
- **New code is Rust.** The Python changes are conformance tooling only.
- **ADR-008 as cargo edges.** `analysis` depends on `contracts` and `pyfmt` alone; `feedback` on
  `contracts` (and `pyfmt`) and **never** on `analysis`; `storage` never on `analysis`. `crates/core`
  is the one crate that holds `analysis` and `feedback` together, so `run_profile`, the verb and the
  family gate live there (call 13).
- **Mechanics never enters a grade, a blend or the list** (Decision 12). No pose measurement is read
  by anything this milestone builds; a test says so (P6, P8).
- **No new dependency** without a finding first. The Wilson interval is `f64::sqrt` and arithmetic.
- **House style** (`CLAUDE.md` §House style, `docs/CODE_STANDARDS.md`): comments say *why* and carry
  the measurement or the rejected alternative. Every module this milestone adds **has no Python
  twin and says so** in its first lines, as `crates/contracts/src/capability.rs` does.
- **Do not write counts into prose** that a test or a registry owns. Where this plan quotes a count,
  it is a snapshot for the phase to re-measure.
- **The doc-count pin reads `git ls-files`**, so this file is invisible to it until P13 stages it. Do
  not touch `docs/README.md`'s document count before P13. This file avoids the
  `**Status: … N/M phases` spelling on purpose (`tests/test_docs_truth.py::_PHASE_STATUS`).
- **Line endings**: the plain `.json` vectors Python wrote are CRLF, and `rerecord`'s writer keeps
  each file's own. Git Bash heredocs mangle `—`, `±` and `°`; use the Edit tool or Windows Python for
  in-place edits.

## Verify — the commands, and when each runs

```bash
cargo test                                          # every phase that touches crates/ or spec/
cargo clippy --all-targets && cargo fmt --check     # every phase that touches crates/
.venv/Scripts/python.exe -m pytest                  # P5 or P9 (whichever touches conformance.py), P12, P13 — run it alone, not beside cargo test (M35 P13 finding: a worker-timeout test flakes under load)
.venv/Scripts/python.exe -m ruff check src tests scripts   # any phase touching Python; P13
.venv/Scripts/python.exe -m mypy src                # P13 (M37 changes nothing under src/)
.venv/Scripts/python.exe scripts/conformance.py check      # any phase touching conformance.py; P13
.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py # every phase that edits a doc or moves a path
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/v17.json --dry-run        # P4, P13: the engine, stage and shot families are unchanged
cargo run --release --bin golf-core -- rerecord --declare spec/declarations/career-v3.json --dry-run  # P4 (after the real run), P11, P13: 0 changed
find data/processed/sessions data/processed/golfers data/processed/shots -type f | LC_ALL=C sort | xargs -d '\n' sha256sum | sha256sum   # P1, P4, P10, P13
```

---

## Decisions taken in the interview (2026-10-10)

The user's, in four rounds. A later phase that wants to revisit one needs a reason the interview
did not have.

1. **Consistency.** A shot is consistent when its carry is within **±10% of its club's median
   carry** *and* its start-line offline is within **±10% of that median carry** of the club's
   median offline. One fraction, scaled per club: about ±15 yd for a 150-yard 7 iron, ±25 yd for a
   250-yard driver. The lateral number is `start_line_offline_yds`, a straight-line projection, and
   that is right here: consistency asks whether the shot was like the golfer's other shots, not
   where it finished against the target.
2. **Minimum n: 10**, for every topic grade and every share shown beside one. Below it the grade is
   withheld and named. Wilson 95% at a 50% share is about ±26 points at n = 10 (±33 at 5, ±20 at 20).
3. **Strike location is graded only from a measured face-impact offset** (`ShotData.impact_offset_h`
   and `impact_offset_v`, millimetres, + = toe and + = high, the fields M32 added). **HD Golf's
   printed `CENTER`/`HEEL`/`TOE` word is not graded** — "picture only". The user's direction: the
   bay will also run **Full Swing's** software, whose impact video shows where the club met the
   ball, and the iPhone's Live Photo captures that video beside the still. Reading the offset from
   it is a new vision task and a new milestone, **M42**, starting with a feasibility spike on real
   Live Photos (decision 11). Until M42 lands no real device supplies the offset, so on real data
   the strike-location topic is **absent**, the stored 13 shots included, and M37 proves it on the
   `synthetic` device (call 2).
4. **Centered = within a provisional 10 mm radius of face center**: `hypot(h, v) ≤ 10` where both
   offsets hold values; `|h| ≤ 10` where only the horizontal does. A named judgment constant
   (`CENTERED_RADIUS_MM`), whose provenance says M42 revises it from the reader's measured error.
5. **The data path: the whole `ShotData` rides on `CorpusSwing`.** `read_corpus` lifts the
   artifact's `shot` (from `analysis.json` or `shot_analysis.json`) onto each entry, so
   `capability::printed_on` decides printed against blank per shot, and the next topic needs no new
   field. `CAREER_VERSION` 2 → 3, and `career-v3` re-records the storage and career families.
6. **A mishit counts as a miss for consistency.** It stays in the club's judged count and fails the
   criterion, so an outlier lowers the grade, which is what the user asked for ("a huge outlier
   would make this score lower").
7. **Low point: ahead of the ball (> 0 in, + = ahead) is good**, for irons, wedges and hybrids. The
   topic is **absent for driver and woods** (hitting up is correct there) and for a putter. No
   device prints low point today, so this too is proven on the `synthetic` device.
8. **On line: ±9 yd**, `METRIC_TARGETS`' `start_line_offline_yds` row reused, as ADR-034 §5.3
   defaulted. Confirmed for a landing (ADR-034 asked M37 to confirm it), knowing it is generous for
   a wedge and tight for a driver.
9. **`golf-core profile --name <golfer> [--club <club>] [--json]`**: a text report like the other
   career verbs, and `--json` for the struct the phone and M29's `rmcp` tool will read.
10. **Desk only.** No bay phase. P13 runs `golf-core profile` read-only over `data/` as a smoke test.
11. **Full Swing and HD Golf are both in use**, each with its own profile and capability entry.
    **One new milestone, M42**, "Full Swing: the second device and its impact video", carries the
    Live Photo spike, the impact reader that fills `impact_offset_h/v`, Full Swing's screen profile
    and its `devices.json` entry. P0 adds a short ROADMAP stub for it.

---

## What the planning read found (2026-10-10)

Verify each before leaning on it.

1. **`rank_profile` does not exist**, in Python or Rust. §M37 calls it "unchanged in rule", which
   means the rule (ADR-034 §5.7); the function is M37's to write (the M31 plan's P3 finding 5,
   `docs/plans/m31-shot-first-adr.md:1106`).
2. **`CorpusSwing` carries measurement rows only** (`crates/contracts/src/career.rs:257`). The
   categorical tiles, the M32 keys and the printed landing offline are on `ShotData`, which both
   artifacts carry whole: `SwingResult.shot` (`crates/contracts/src/swing.rs:322`) and
   `ShotResult.shot` (`crates/contracts/src/shot_result.rs:64`). `read_corpus` lifts
   `measurements` at `crates/storage/src/corpus.rs:440` (a swing) and `:501` (a photo entry).
3. **Only `hd_golf` and `mock` are declared** (`crates/contracts/devices.json`). Neither declares
   `impact_offset_h/v`, `low_point` or `carry_offline`, so three of M37's inputs are reachable only
   through a device that does (call 2).
4. **The shot rows** `shot_measure` writes are `ball_speed_mph`, `carry_distance_yds`,
   `face_to_path_deg`, `launch_angle_deg`, `start_line_deg`, `start_line_offline_yds` and
   `total_distance_yds`. **`flight_landing_offline_yds`** (`crates/analysis/src/flight_measure.rs`,
   source `model:flight_v1`) is the curved projection, recorded **only when the curve was drawn**,
   which needs a printed spin axis: 2 of 13 stored shots, none of the 11 from 2026-08-23. So on HD
   Golf as seen, every shape topic's on-line share is withheld and only "hit the shape" shows
   (ADR-034 §5.3, accepted by the user).
5. **The face-to-path sign does not flip for a left-hander** (ADR-034 §5.3, correcting §M37's "the
   sign flips"): face, path and spin axis are golfer-relative. Handedness decides which side of the
   line a shape finishes on, and that is already inside `flight_landing_offline_yds`. A missing
   handedness shows up as that row's absence with `no_handedness` in the artifact's `unscored`.
6. **`METRIC_TARGETS`** (`crates/contracts/src/dispersion.rs:346`): `face_to_path_deg` 0 ± 2.0,
   `start_line_deg` 0 ± 2.0, `start_line_offline_yds` 0 ± 9.0; carry and ball speed have no target.
   `target_for(metric)` is the lookup.
7. **`crates/analysis/src/stats.rs`** has `percentile` (the median is `q = 0.5`), `mean_and_sd`,
   `mean_ci` and `sd_ci`. There is no Wilson interval. `contracts::baseline::Interval` is the
   interval shape to reuse.
8. **Every swing on disk is `fundamentals`** (intent `None`), so no shape is declared anywhere in
   `data/`, and every shape topic is absent on real data. The real-golfer corpus is
   `spec/vectors/career/real/aaron.json.gz`.
9. **No career shape has a schema** under `spec/schemas/` (`club_profile`, `dispersion` and
   `baseline` have none), so the profile gets none either (call 14).
10. **The career family's runner** is `crates/core/src/career_family.rs::run_career`, gated by
    `crates/core/tests/career.rs`; a career case may adopt a storage case's `expected.corpus`
    (`adopted_from`), and `rerecord` derives the copy. The career verbs dispatch through
    `golf_core::reports::{Verb, Request, Answer}` (`crates/core/src/bin/golf_core.rs:169`).
11. **`build_bag_profile`** (`crates/analysis/src/club_profile.rs:71`) already gives each club its
    baseline and dispersion through `CareerCorpus::narrowed_to`. Ball-speed SD and the carry,
    start-line and face-to-path dispersion are **read from it, never recomputed** (§M37 says
    "through the existing `dispersion`").
12. **The `UnscoredReason`s the on-line share names already exist**: `NoHandedness`,
    `SpinAxisUnresolved`, `PrintedBlank`, `Misread` (`crates/contracts/src/unscored.rs`).

---

## Calls this plan makes where §M37 and the interview left it open

Each is the plan's, not the user's, and approving the plan approves them. A phase that finds one
wrong records why and stops.

1. **The profile vectors are a new family, `spec/vectors/profile/`**, with `hand/` (P5,
   `oracle: "hand"`) and `corpus/` (P11). It belongs to `CAREER_VERSION`: a `career-v<N>.json`
   declaration re-records storage, career *and* profile together from P11 on. **One bump in this
   milestone**: P4 bumps to 3 for the corpus lift before the family exists, and the family is born
   at 3, so nothing about it needs a second.
2. **A `synthetic` device in `devices.json`**, declaring every `ShotData` reading as `analysed`,
   with a `source` saying it exists so hand-worked vectors can exercise what no real device prints
   yet (strike offsets, low point, a printed landing offline). `mock` already sets the precedent of
   a test device. A pin holds that no committed storage, career, shot or screen vector uses it.
   M42's `full_swing` entry is what later makes strike location real.
3. **`CorpusSwing` gains `shot: Option<ShotData>` and `shot_unscored: Vec<UnscoredCheckpoint>`**,
   both lifted off the survivor's artifact, both **written only when set** (as `intent` is), so a
   swing with no photo serializes exactly as before. `shot_unscored` is the artifact's `unscored`
   entries whose name is **not** a `CHECKPOINT_REGISTRY` checkpoint — the shot and flight side,
   which is where the on-line share's reasons live. A duplicate's shot is never consulted, as its
   club is not.
4. **The club's typical shot is the median of its non-mishit judged shots**; a mishit then fails
   consistency whatever its numbers (decision 6). The median rather than the mean, so the outliers
   the grade punishes do not also move the yardstick.
5. **The shape boundary is inclusive**: straight when `|face_to_path_deg| ≤ 2.0`, fade when
   `> +2.0`, draw when `< −2.0`, for either hand (finding 5). The hand vectors put shots at 1.9,
   2.0, 2.1 and −2.0, −2.1.
6. **Each shot's landing offline has a basis**: `carry_offline` where `capability::analysed_on`
   names it (**printed**), else `flight_landing_offline_yds` (**projected**, and named as projected
   wherever it appears), else **none**, with the reason taken from `shot_unscored`
   (`spin_axis_unresolved`, `no_handedness`, the flight's refusal). A shot with no basis drops out
   of the on-line share's denominator and is counted under its reason; the topic grade carries how
   many shots were printed and how many projected.
7. **A topic exists for a club when its input is printed for that club** — `printed_fields` over
   the club's shots: strike location needs `impact_offset_h`; consistency needs `carry_distance`
   and `launch_direction`; low point needs `low_point` and an iron, wedge or hybrid; a shape needs
   `club_face_angle` and `club_path` **and** at least one tracked shot of that club declared
   `shot_shaping` with that `target_shape`. Not printed, or never declared, is absent: no grade, no
   name, no blend entry.
8. **A topic's judged shots** are the club's tracked shots that count toward metrics
   (`counts_toward_metrics`, not `shot_needs_review`) and hold the topic's inputs. A shot whose
   input is printed but blank is **unscored for that topic**, counted under its reason
   (`printed_blank`/`misread` from `shot_unscored` where the artifact named one, otherwise a plain
   blank) and never counted as a miss. Fewer than 10 judged shots is ungraded, named.
9. **Untagged swings** (no club) count toward the player-level strike-location and shape topics,
   whose criteria need no club, and not toward consistency or low point, which do. Their number is
   named.
10. **The impact grid is 3 × 3 with per-axis bins at `±CENTERED_RADIUS_MM`**, horizontal heel /
    center / toe by `h`, vertical low / center / high by `v` (or one row when `v` is not printed),
    while the **centered rate uses the radius** (decision 4). The two differ at the corners of the
    center cell on purpose — the grid is a picture of where, the rate is the judgment — and the
    module doc says so. **Heel/toe** is the heel and toe counts among off-center horizontal hits,
    plus the mean horizontal offset with its 95% CI (`stats::mean_ci`) once the club holds
    `DEFAULT_MINIMUM_N`'s `Center` floor.
11. **A blend is the arithmetic mean of its graded topics' point shares**, with no interval of its
    own; it lists the topics it averaged and the topics it excluded, each named with why. **A blend
    with no graded topic is absent, not zero** (ADR-034 §5.5).
12. **`rank_profile` reads launch-monitor metrics only** (`launch_monitor:` sources; the `flight_*`
    rows are outside `METRIC_TARGETS` already), so mechanics stays a separate panel. Per ADR-034
    §5.7: a **weakness** is an established bias or scatter (the dispersion's own findings), ranked
    by its point estimate's ratio to tolerance (`|mean − target| / tolerance` for bias,
    `sd / tolerance` for scatter), ties in `METRIC_TARGETS` order; a **strength** is a metric with a
    target whose mean CI lies inside `target ± tolerance` *and* whose SD's upper bound is below
    tolerance; "nothing established" is never a strength. The player-level list reads a
    whole-corpus baseline and dispersion, kept to club-independent metrics.
13. **`golf_core::profile::run_profile(corpus, bag) -> GradedProfile`** assembles the grades
    (`analysis`) and the list (`feedback`), because only `core` holds both.
14. **No schema for the profile** (finding 9). Its round trip is pinned in `crates/contracts/tests`,
    and the phone reads the Rust struct through the bridge.
15. **One commit, at P13**, as M35.
16. **`get_strike_profile` waits for M29's `rmcp` server**, which has not landed; M29 carries it
    (§M37's own rule).
17. **The stat scope is a table in `strike.rs`** keyed by measurement name: club-only for carry,
    total, ball speed, launch, spin and `start_line_offline_yds` (yards scale with the club);
    club-independent for `face_to_path_deg`, `start_line_deg` and the impact offsets. Only a
    club-independent stat gets a raw player-level number (Decision 8).
18. **`oracle: "rust"`** for a vector born from Rust's own run (P11's corpus cases), since no Python
    and no hand stands behind it. Its note says its correctness rests on the hand cases and that it
    pins stability. P11 checks ADR-035 §3 and CONFORMANCE §2 first; if either rules this out, it
    records a finding and stops.

---

## Phases

### P0 — This plan document, the pointers to it, and the M42 stub

- **Goal**: this file, approved; the pointers; M42 recorded.
- **Files**: `docs/plans/m37-strike-profile-and-grades.md` (this file); `ROADMAP.md` — §M37 gains a
  **Plan** line pointing here, its status row links the plan, and a **M42** row plus a short §M42
  section after §M41 (status ⬜, the ask from decision 11, depends on sample Live Photos and M37's
  strike topic, where: this box with photos, the bay to take them); the program plan's status table
  gains an M42 row as it carries M41's (`docs/plans/m31-m40-shot-first-pivot.md:76`), routing to the
  ROADMAP.
- **Done when**: the user has approved the plan, the pointers resolve, docs-truth is green.
- **Verify**: `.venv/Scripts/python.exe -m pytest tests/test_docs_truth.py`.

### P1 — `crates/contracts` I: `strike.rs`; the `synthetic` device; the `data/` snapshot

- **Goal**: the answer's shape and every constant, before anything produces one.
- **Read**: `crates/contracts/src/{baseline,dispersion,club_profile,intent,capability,lib}.rs` (the
  shapes to reuse: `Interval`, `WithheldClaim`, `ClubId`, `ClubCategory`, `TargetShape`);
  `crates/contracts/devices.json`; `crates/contracts/tests/capability.rs`;
  `crates/screen/tests/capability.rs` (it holds only `hd_golf` to the profile).
- **Builds** — `crates/contracts/src/strike.rs`, no Python twin:
  - `RateEstimate` (successes, n, share, `Interval`) — the shape only; the arithmetic is P2's.
  - `Topic` (strike location, consistency, low point, fade, draw, straight), `OfflineBasis`,
    `ShapeShares` (hit the shape; hit the shape and on line), `TopicGrade` (the topic, a graded
    `RateEstimate` or the reason it is withheld, the judged count, the unscored counts by reason,
    for a shape both shares and the printed/projected split), `Blend` (call 11), `ImpactCell` and
    the grid (call 10), `HeelToe`, `StrikeProfile` (per club), `ClubGrades`, `PlayerGrades`, and
    `GradedProfile` (clubs, player, and the strengths/weaknesses list's shape: `ProfileFinding` with
    metric, kind and ratio). **Not `PlayerProfile`**: `intent.rs` has one.
  - Constants with provenance: `RATE_MINIMUM_N = 10` (decision 2), `CENTERED_RADIUS_MM = 10.0`
    (decision 4), `CONSISTENCY_FRACTION = 0.10` (decision 1). The on-line tolerance and the
    straight tolerance are **not** new constants: they are `target_for("start_line_offline_yds")`
    and `target_for("face_to_path_deg")`, read where used.
  - `StatScope` and its table (call 17); `Validate` on each shape; registered in `lib.rs`.
  - `devices.json` gains `synthetic` (call 2), and its `_comment` says why.
- **Tests**: every shape round trips, absent keys apart from null; `RateEstimate` refuses
  `successes > n`; the scope table names every `shot_measure` row; `synthetic` declares every
  `ShotData` reading (read off the struct, as `fields_holding_a_value` is) and every field
  `analysed`.
- **Also**: take the `data/` digest and record it in the findings.
- **Files**: `crates/contracts/src/{strike,lib}.rs`; `crates/contracts/devices.json`;
  `crates/contracts/tests/{strike,capability}.rs`.
- **Done when**: green; the digest matches.
- **Verify**: `cargo test -p contracts`; `cargo test`; clippy; fmt.

### P2 — `crates/analysis` I: the Wilson interval, and the per-shot judgments

- **Goal**: every yes/no a grade counts, as pure functions over one shot, before any aggregate.
- **Read**: `crates/analysis/src/stats.rs` (its style and its interval tests); `strike.rs` (P1);
  `crates/contracts/src/capability.rs` (`printed_on`, `analysed_on`);
  `crates/contracts/src/career.rs` (`CorpusSwing::is_mishit`, `counts_toward_metrics`).
- **Builds**:
  - `stats::wilson(successes, n) -> Option<RateEstimate>` (95%, z = 1.959963984540054), `None` for
    n = 0, with its formula and a published reference value in the doc.
  - `crates/analysis/src/strike.rs`, no Python twin, the **judgments** half: `is_centered(shot)`
    (decision 4), `impact_cell(shot)` (call 10), `low_point_ahead(shot, category)` (decision 7),
    `shape_of(face_to_path)` (call 5), `offline_basis(shot, measurements, shot_unscored)` (call 6),
    `on_line(offline)` (decision 8), and `is_consistent(carry, offline, typical, mishit)`
    (decisions 1 and 6, call 4). Each takes the analysed view (`capability::analysed_view`), so a
    reading outside printed ∩ analysed cannot reach a judgment.
- **Tests**: Wilson against published values (n = 10, 0/10, 10/10, 5/10) and its bounds inside
  [0, 1]; each judgment at and either side of its edge — 10 mm on the radius and on `h` alone,
  low point 0 and +0.1, face-to-path ±2.0 and ±2.1, offline ±9.0 and ±9.1, carry at ±10% — and a
  left-handed shot classified as its sign says; a mishit inside both windows is not consistent; a
  shown-only or undeclared field never reaches a judgment.
- **Files**: `crates/analysis/src/{stats,strike,lib}.rs`; `crates/analysis/tests/strike.rs` (new).
- **Done when**: green.
- **Verify**: `cargo test -p analysis`; `cargo test`; clippy; fmt.

### P3 — `crates/contracts` II: `CorpusSwing` carries the shot and its shot-side refusals

- **Goal**: call 3's shape, with nothing filling it yet, so no recorded vector moves.
- **Read**: `crates/contracts/src/career.rs` (`CorpusSwing`, its remote serde and `Validate`,
  `narrowed_to`, `intent`'s "written only when set" precedent); `crates/contracts/tests/` (the
  corpus round trips).
- **Builds**: the two fields, written only when set; `narrowed_to` carries them; the field docs say
  why (a topic needs the tiles, and the capability model needs `provenance`).
- **Tests**: a corpus without them serializes byte-for-byte as before; with them it round trips;
  `narrowed_to` keeps them.
- **Files**: `crates/contracts/src/career.rs`; `crates/contracts/tests/*`.
- **Done when**: green, with every storage and career vector untouched.
- **Verify**: `cargo test`; clippy; fmt.

### P4 — `crates/storage`: `read_corpus` lifts them; `CAREER_VERSION` 3; `career-v3`

- **Goal**: decision 5 on disk, and its re-record.
- **Read**: `crates/storage/src/corpus.rs` (the swing lift at `:440`, the photo entry at `:501`, and
  the survivor rule for `club`); `crates/contracts/src/career.rs`'s `CAREER_VERSION` ledger;
  `spec/declarations/career-v2.json` (the declaration's shape); the M35 plan's P10 section.
- **Builds**: `read_corpus` fills `shot` and `shot_unscored` from the survivor's artifact (call 3);
  `CAREER_VERSION` 2 → 3 with its ledger entry. **The re-record**: a dry run lists every moved path;
  each is checked against call 3 (expected: every storage corpus case with a shot gains `shot`,
  some gain `shot_unscored`, and their career twins' `input.corpus` follow by adoption; **no
  aggregate and no report moves**) and named in `spec/declarations/career-v3.json` with the reason
  in its `note`. Then the real run, then a second run that writes nothing.
- **Tests**: a swing and a photo entry each carry their artifact's shot; a duplicate's shot is never
  read; a swing with no photo carries none; `shot_unscored` holds no checkpoint name.
- **Measure**: `read_corpus` over `data/` lifts a shot onto each of the stored shots' entries and
  pools what it pooled before. The digest is re-checked.
- **Files**: `crates/storage/src/corpus.rs`; `crates/contracts/src/career.rs`;
  `spec/declarations/career-v3.json` (new); the re-recorded `spec/vectors/{storage,career}/**`.
- **Done when**: green; the dry run reports only declared paths; the second run writes nothing; the
  v17 dry run reports 0 changed; `data/` is unchanged.
- **Verify**: the `career-v3` dry run, the real run, the dry run again; the v17 dry run;
  `cargo test`; clippy; fmt; the digest.

### P5 — The hand-worked profile vectors, parsed and skipped by name until P9

- **Goal**: every rule worked by hand before the code that follows it.
- **Read**: decisions 1–8 and calls 2–12; `strike.rs` (P1); `spec/vectors/storage/hand/` (M35 P9's
  cases: their provenance block and the working in `note`); `crates/core/tests/career.rs` (how a
  family is listed and skipped); `scripts/conformance.py`'s deferral list (how M35 deferred the shot
  family).
- **Builds** — `spec/vectors/profile/hand/*.json`, `career_version` 3, `oracle: "hand"`, each an
  `input.corpus` (and `input.bag` where it matters) built by hand and an `expected` (a whole
  `GradedProfile`) worked by hand, its working in `provenance.note`. At least:
  - the **straight edge from both sides** (call 5), and a **left-handed** golfer's shots classified
    by sign;
  - a shape topic with a **projected** landing, one with a **printed** one (`synthetic`), and one
    whose on-line share is **withheld** with its reasons counted while "hit the shape" grades;
  - **consistency** with a mishit inside both windows (a miss) and an outlier;
  - **strike location** on `synthetic`, radius against box at the center cell's corner, `v` printed
    and not; **heel/toe**;
  - **low point**: an iron graded, a driver with low point printed and the topic absent;
  - **ungraded is named** (9 judged shots), **impossible is absent** (HD Golf: no strike, no low
    point; no shape declared), **a blend with nothing graded is absent**;
  - **player pooling** across two clubs, each shot judged by its own club's median, untagged shots
    in strike and shape only (call 9);
  - a **drill** in the input (already excluded by the corpus, so nothing changes) and a pose-heavy
    swing (mechanics moves no grade);
  - the **list**: a bias weakness, a scatter weakness, a strength, and a "nothing established".
- **The gate**: `crates/core/tests/profile.rs` parses every case's input and expected into their
  types and checks each case's counts against its own swings, then skips the comparison by name,
  "P9 passes these". `conformance.py check` defers the family to `cargo test`, and
  `tests/test_conformance.py` pins that.
- **Files**: `spec/vectors/profile/hand/*.json` (new); `crates/core/tests/profile.rs` (new);
  `scripts/conformance.py`; `tests/test_conformance.py`.
- **Done when**: green with every comparison skipped by name; each note shows its working.
- **Verify**: `cargo test`; clippy; fmt; pytest (alone); ruff; `conformance.py check`.

### P6 — `crates/analysis` II: the per-club strike profile and topic grades

- **Goal**: one club's `StrikeProfile` and its `TopicGrade`s.
- **Read**: P2's judgments; `crates/analysis/src/club_profile.rs` (how a club is narrowed, and its
  `build_bag_profile`); calls 4, 6–10; the P5 findings.
- **Builds**: `strike::build_club_grades(corpus narrowed to one club, &ClubProfile) -> ClubGrades`:
  availability (call 7), judged shots and unscored counts (call 8), the medians (call 4), the grid,
  the centered rate and heel/toe (call 10), each topic's `RateEstimate` through `stats::wilson`,
  withheld below `RATE_MINIMUM_N`. Ball-speed SD and the dispersion numbers come from the
  `ClubProfile` passed in (finding 11).
- **Tests**: unit tests per topic; no pose measurement is read (a corpus whose pose rows change
  produces the same grades); the P5 cases this phase can already decide are checked here as a
  preview, and the gate's skips stay until P9.
- **Files**: `crates/analysis/src/strike.rs`; `crates/analysis/tests/strike.rs`.
- **Done when**: green.
- **Verify**: `cargo test`; clippy; fmt.

### P7 — `crates/analysis` III: player pooling, and the two blends

- **Goal**: Decision 11 and ADR-034 §5.4–§5.5.
- **Read**: P6's builder; calls 9 and 11; the P5 and P6 findings.
- **Builds**: `strike::build_player_grades(corpus, &[ClubGrades]) -> PlayerGrades` — each topic's
  share pooled over every tracked shot, each judged by its own club's criterion; the player's raw
  numbers kept to club-independent stats (call 17); `Blend` for each club and for the player
  (call 11); `strike::build_grades(corpus, &BagProfile)` joining the two.
- **Tests**: a shot judged consistent for its club pools as consistent at player level even when
  another club's median would fail it; untagged shots enter strike and shape only; the blend averages
  graded topics, names excluded ones, omits absent ones, and is absent when nothing is graded.
- **Files**: `crates/analysis/src/strike.rs`; `crates/analysis/tests/strike.rs`.
- **Done when**: green.
- **Verify**: `cargo test`; clippy; fmt.

### P8 — `crates/feedback`: `rank_profile`, the strengths and weaknesses

- **Goal**: ADR-034 §5.7 as code (call 12).
- **Read**: `crates/feedback/src/{lib,rules}.rs` (its style; it depends on `contracts` only);
  `crates/contracts/src/{baseline,dispersion,club_profile}.rs` (`MetricBaseline`'s intervals,
  `MetricDispersion`'s findings); finding 1.
- **Builds**: `crates/feedback/src/profile_rules.rs::rank_profile(&ClubProfile) -> Vec<ProfileFinding>`
  and its player-level twin over a whole-corpus `PersonalBaseline` and `GolferDispersion`, kept to
  club-independent metrics. No Python twin, and the module doc says the rule is ADR-034 §5.7's.
- **Tests**: a bias and a scatter rank by ratio; a strength needs both halves; "nothing established"
  is never a strength; a pose metric never appears; the cargo edge holds (`feedback` has no
  `analysis` dependency).
- **Files**: `crates/feedback/src/{profile_rules,lib}.rs`; `crates/feedback/tests/profile_rules.rs`
  (new).
- **Done when**: green.
- **Verify**: `cargo test`; clippy; fmt.

### P9 — `crates/core` I: `run_profile`, the profile family's runner and gate

- **Goal**: the whole answer in one call (call 13), and the hand cases passing.
- **Read**: `crates/core/src/{career_family,lib}.rs`; `crates/core/tests/{profile,career}.rs`; the
  P5–P8 findings.
- **Builds**: `golf_core::profile::run_profile(&CareerCorpus, Option<&Bag>) -> GradedProfile`
  (`build_bag_profile`, then `strike::build_grades`, then `rank_profile` per club and for the
  player); `crates/core/src/profile_family.rs::run_profile_case`, which the gate and P11's
  re-record will share; the P5 skips removed.
- **Files**: `crates/core/src/{profile,profile_family,lib}.rs`; `crates/core/tests/profile.rs`.
- **Done when**: every hand case passes; green.
- **Verify**: `cargo test`; clippy; fmt.

### P10 — `crates/core` II: `golf-core profile`, its report and `--json`

- **Goal**: decision 9.
- **Read**: `crates/core/src/bin/golf_core.rs` (`parse_career`, `career`, `print_answer`, the
  `--club` flag); `crates/core/src/reports/{mod,club_profile}.rs` (a report's shape and wording
  conventions); `crates/core/tests/reports.rs`.
- **Builds**: `Verb::Profile`; `crates/core/src/reports/profile.rs` — per club the grid, the
  centered rate and heel/toe, each topic's grade with its interval and n (shape topics with both
  shares and "projected" where it is), the blend with what it averaged and excluded, and the list;
  then the player's. Withheld and named say why in words; absent says nothing. `--json` prints the
  `GradedProfile`. The verb reads through `read_corpus` and the bag store and writes nothing.
- **Tests**: the report on a temp tree; `--json` round trips into `GradedProfile`; `--club` narrows;
  an unknown golfer is refused as the other career verbs refuse one.
- **Measure**: `golf-core profile --name Aaron` over `data/` — expected mostly absent and withheld
  (findings 3 and 8); record the output's shape in the findings. The digest is re-checked.
- **Files**: `crates/core/src/{bin/golf_core,reports/mod,reports/profile}.rs`;
  `crates/core/tests/reports.rs`.
- **Done when**: green; `data/` unchanged.
- **Verify**: `cargo test`; clippy; fmt; the digest.

### P11 — The profile family's corpus cases; `rerecord` walks the family; the oracle pin

- **Goal**: Rust-recorded corpus cases beside the hand ones, re-recordable, and §M37's oracle pin.
- **Read**: `crates/core/src/{rerecord,career_family}.rs` (adoption and the career walk);
  `docs/CONFORMANCE.md` §2 and ADR-035 §3 (call 18); `spec/declarations/career-v3.json`.
- **Builds**:
  - `spec/vectors/profile/corpus/` — one case per career case with a corpus worth grading (the real
    golfer, and the storage-adopted twins), each adopting that career case's `input.corpus` and
    `input.bag`, with `expected` (the `GradedProfile` and the `profile` report text) written once by
    `run_profile_case` and `oracle: "rust"` (call 18).
  - `golf-core rerecord` under a `career-v<N>` declaration walks the profile family after career,
    deriving adopted inputs as it does for career twins. The `career-v3` dry run then reports 0
    changed across all three families.
  - **The oracle pin**: every committed vector under `spec/vectors/` names an oracle in
    {`python`, `hand`, `rust`}; and, if it holds on today's files, every `provenance.rerecords`
    entry names its declaration (§M32's option). If the second half does not hold, record which
    files break it and pin only the first.
- **Files**: `spec/vectors/profile/corpus/*.json` (new); `crates/core/src/{rerecord,profile_family}.rs`;
  `crates/core/tests/{profile,rerecord}.rs`.
- **Done when**: green; the `career-v3` dry run reports 0 changed; the oracle pin passes.
- **Verify**: the `career-v3` dry run; `cargo test`; clippy; fmt.

### P12 — Docs: CONFORMANCE, ARCHITECTURE, the ADR addenda, the program plan, `CLAUDE.md`

- **Goal**: every doc that describes what M37 changed says so, and docs-truth is green.
- **Files**:
  - `docs/CONFORMANCE.md` §2 — the profile family (hand and corpus, `oracle: "rust"`, under
    `CAREER_VERSION`), the oracle pin; §4 — `career-v<N>` now covers three families.
  - `docs/ARCHITECTURE.md` §1 (the `profile` verb and its flags), §2 (`analysis::strike`,
    `feedback::profile_rules`, `core::profile`), §4 (`CorpusSwing.shot`).
  - ADR-034 addendum (M37): decisions 1–8 as built, strike location picture-only and why, the 9-yard
    landing tolerance confirmed, the `synthetic` device, and `rank_profile` written to §5.7.
    ADR-024 addendum: the corpus carries the shot, `career-v3`. ADR-032: the profile family's
    edges, if P2–P11 found any (Wilson's last bit, the median of an even count).
  - The program plan: a correction note at §M37 (strike location from M42's offset, not the
    printed word; the sign does not flip) and its status row.
  - `CLAUDE.md`: the crate table (`strike`, `profile_rules`, `profile`, the profile family), the
    `profile` verb in Commands, and the `CAREER_VERSION` invariant naming three families.
  - `crates/contracts/devices.json`'s `_comment` and `docs/README.md`'s map rows if their counts move
    (not the document count; that is P13's).
- **Done when**: docs-truth and the full pytest are green; `git diff -- src/` is empty.
- **Verify**: pytest (alone); docs-truth.

### P13 — Close: verify, WORKLOG, ROADMAP, memory, and the one commit

- **Goal**: the exit below, checked, recorded, and committed once.
- **Steps**: run every command in "Verify", the v17 dry run (0 changed) and the `career-v3` dry run
  (0 changed); compare the `data/` digest a last time; run `golf-core profile --name Aaron` over
  `data/` (decision 10) and record what it says; `git diff -- src/` empty;
  `git diff -- spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}` empty. Write the
  WORKLOG entry: what was built, what M42, M38 and M29 should know (strike location waits on M42's
  offset and its `full_swing` capability entry; `CENTERED_RADIUS_MM` is M42's to revise; the shape
  on-line share is withheld on HD Golf until a spin axis is printed; `get_strike_profile` is M29's),
  and what is left open. ROADMAP: the status row, §M37 to ✅ with the date, NEXT ACTION; the program
  plan's status row the same; `docs/README.md`'s document count (this file becomes tracked). Update
  the `app-transition-decisions` memory (M37 done, M42 raised, what is next). Then one commit onto
  `main`.
- **Done when**: the commit is made and the exit holds.

---

## Exit

M37 is done when all of these hold, each checked at P13:

1. `crates/contracts/src/strike.rs` holds the profile's shapes and constants, and
   `crates/analysis/src/strike.rs` builds a `StrikeProfile` and the six topics' grades per club,
   each a Wilson `RateEstimate` withheld below 10 judged shots.
2. Shot shapes are classified from face-to-path against the reused 2.0° tolerance, inclusive, with
   no flip for a left-hander, and graded on "hit the shape and finished on line" with "hit the
   shape" beside it; the landing is printed where the device prints it, otherwise projected and
   named as projected, otherwise withheld with its reasons counted.
3. Player grades pool every tracked shot, each judged by its own club's criterion; the club and
   player blends average graded topics only, name the excluded ones, omit the absent ones, and are
   absent when nothing is graded. Mechanics enters none of it.
4. `rank_profile` lists established weaknesses ranked by ratio to tolerance and strengths only on an
   equivalence claim.
5. Strike location is graded only from a measured impact offset; on real data (HD Golf) it is
   absent, and on the `synthetic` device it is graded by the 10 mm radius.
6. `CorpusSwing` carries its shot, `CAREER_VERSION` is 3, `career-v3` moved only declared paths, and
   a second run writes nothing.
7. `spec/vectors/profile/` exists — hand cases crossing the straight edge from both sides with a
   left-handed shot, and Rust-recorded corpus cases — gated by `cargo test` and re-recorded under
   `CAREER_VERSION`; every vector family names its oracle.
8. `golf-core profile` prints the report and `--json` the struct, read-only.
9. ROADMAP carries M42. `git diff -- src/` is empty, `data/` is unchanged, and the engine, stage,
   shot, screen, audio and format families are untouched.
10. `cargo test`, clippy, fmt, pytest, ruff, mypy and `conformance.py check` are green.

---

## Phase findings

*(Appended as phases close. Address a later phase by number.)*

### P0 (2026-10-10)

1. Plan approved by the user as written. Pointers: ROADMAP's status row and §M37 (a **Plan**
   paragraph; "Where it runs" is now desk only), and the program plan's M37 row.
2. **M42 is recorded** as a ROADMAP status row and a short §M42 after §M41, plus a program-plan
   status row routing to it, the same way M41 is. Its exit is left to its own plan.
3. Docs-truth green (92 passed). Nothing outside `ROADMAP.md`, the program plan and this file
   changed.

### P1 (2026-10-10)

1. **Landed**: `crates/contracts/src/strike.rs` (registered in `lib.rs`, whose doc names it the fourth
   module with no Python twin); `synthetic` in `devices.json` with a `_comment` paragraph;
   `crates/contracts/tests/strike.rs` (24 tests, new) and two new tests in `tests/capability.rs`.
   `cargo test` (65 binaries, 0 failed), clippy and `fmt --check` green. `git diff -- src/ spec/vectors/`
   is empty. **The `data/` digest matches**: `bd384cc40b236b73e3013b817ae79908d6e5ff4dded25b8803fc35c6b7af01cc`
   over 192 files.
2. **One file outside the Files line, a test only**: `contracts` cannot see `analysis`, so the exact
   "every `shot_measure` row has a scope" pin is `every_shot_and_flight_row_declares_a_stat_scope` in
   `crates/analysis/src/shot_measure.rs`'s tests, over `SHOT_MEASUREMENTS` and `FLIGHT_MEASUREMENTS`
   both. The `contracts` side holds the table to every row the committed `spec/vectors/shot/` family
   records, as `screen/tests/capability.rs` holds `devices.json` from the crate that can see both.
3. **The shapes, as built** (refinements of the Builds line, each to keep two spellings from
   disagreeing):
   - `Share` is an enum, `Graded(RateEstimate)` or `Withheld(WithheldShare { have_n, need_n })`, on the
     wire `{"graded": {…}}` / `{"withheld": {…}}`. Too few judged shots is the only withheld reason.
   - **The on-line share is `TopicGrade.grade`** and is not repeated in `ShapeShares`, which holds
     `hit_shape`, the `printed`/`projected` split and `landing_unscored` (judged shots with no
     landing, by reason).
   - **The list is not on `ClubGrades`/`PlayerGrades`.** `GradedClub { grades, list }` and
     `GradedPlayer { grades, list }` pair them, so `analysis`'s builders return no list at all
     rather than an empty one that reads as "nothing established". `GradedProfile { player_id, clubs:
     Vec<GradedClub>, player: GradedPlayer }`.
   - `StrikeProfile { impacts: Option<ImpactMap { grid, heel_toe }>, ball_speed:
     Option<MetricBaseline>, dispersion: BTreeMap<String, MetricDispersion> }`, the last two carried
     whole from the `ClubProfile`. **"Start line" is read as `start_line_deg`**
     (`STRIKE_DISPERSION_METRICS` = carry, `start_line_deg`, face-to-path). **The centered rate is the
     strike-location topic's grade**, not a field of its own.
   - Every `Option` is skipped when `None` (absent key, never `null`), and a `null` still reads as
     absent. The carried `MetricBaseline`/`MetricDispersion` still write `null`, as their career twins
     do, and the no-null test skips them by key.
   - Also there: `Topic::{ALL, as_str, shape, for_shape, needs_a_club, inputs}` (calls 7 and 9 as
     data), `low_point_graded_for(ClubCategory)` (decision 7), `OfflineBasis { Printed, Projected }`
     (no "none" variant: a shot with no basis is counted under its reason), `STRAIGHT_TOLERANCE_ROW`
     and `ON_LINE_TOLERANCE_ROW` (row names read through `target_for`, and pinned to 0 ± 2.0 and
     0 ± 9.0), `BALL_SPEED_METRIC`.
4. **The scope table** (call 17) holds the seven shot rows, **the six flight rows (all club-only;
   "spin" is `flight_spin_rpm`, since a printed `spin_rate` is no measurement row)**, and the two
   offsets keyed by their `ShotData` names (`impact_offset_h`/`_v`, club-independent), because no
   measurement row carries them. A metric not in it, every pose metric included, has no scope, and
   `is_club_independent` answers false for it.
5. **What `Validate` holds**, which every builder and every hand vector must meet:
   - `RateEstimate`: `n ≥ 1`, `0 ≤ successes ≤ n`, `share == successes as f64 / n as f64`
     **exactly**, and `0 ≤ low ≤ share ≤ high ≤ 1`. A graded `Share` has `n ≥ RATE_MINIMUM_N`; a
     withheld one has `need_n == 10` and `0 ≤ have_n < 10`.
   - `TopicGrade`: a non-shape grade's `n == judged`. For a shape, `hit_shape.n == judged`,
     `grade.n + Σ landing_unscored == judged`, and `printed + projected == grade.n`. Each
     `UnscoredCount` list has each reason once, `n ≥ 1`, in `UnscoredReason` declaration order.
   - Topics are in `Topic::ALL` order, once each, with `judged ≤ n_shots`. The blend is `None`
     exactly when nothing is graded; otherwise `averaged` is the graded topics and `excluded` the
     withheld ones, both in topic order, and `mean ∈ [0, 1]`. The mean's arithmetic is not
     re-checked.
   - `ImpactMap`: the grid is sparse (`n ≥ 1` per cell), ordered by `(vertical, horizontal)` with
     the no-vertical cells first, then low to high and heel to toe. `Σ grid == heel_toe.n`, and
     heel/toe are the outer columns' sums. `HeelToe`'s mean comes with its CI, only at
     `n ≥ minimum_n("impact_offset_h", Center)` (5), and inside it. `impacts` is present exactly
     when the strike-location topic is, for a club and for the player.
   - `ClubGrades`: low point only where `low_point_graded_for(category_of(club))`. `PlayerGrades`:
     every raw metric club-independent (so `ball_speed` is always `None`), and
     `untagged_shots ≤ n_shots`.
   - `ProfileFinding`: `ratio` finite and ≥ 0, and the metric has both `target_for` and `scope_of`,
     so no pose metric and no flight row ever lists. A list runs weaknesses then strengths. A
     `(metric, kind)` appears once, and no metric is both a strength and a weakness; bias and
     scatter on one metric is allowed. The player's list is club-independent only. `GradedProfile`'s
     clubs are in strict `ClubId` order.
6. **`synthetic`** is claimed through `provenance.device` (there is no `ShotSource` for it). The
   new tests hold it to declaring every reading (as a set, read off the struct) all `analysed`, and
   hold `storage`, `career`, `shot` and `screen` to never naming it, through a `device` key or a
   `launch_monitor:` source. The walk finds `hd_golf`, so it is not vacuous.
   `crates/screen/tests/capability.rs` holds only `hd_golf` and did not move.
7. **For P2**: `stats::wilson` must **clamp at 0 and n successes**, because `RateEstimate` refuses
   an interval that fails to hold the share by one ulp. The judgments take the analysed view, and
   an *unstamped* synthetic shot prints only declared ∩ held, so a test that wants a printed-but-blank
   input must stamp `fields_present`. `Topic::inputs`, `low_point_graded_for` and the two row
   names are already there to reuse.
8. **For P5**: write each `share` as the shortest repr of `k / n` (`repr(7/12)`), make every
   interval hold its share inside [0, 1], list unscored reasons in declaration order, and omit
   absent keys (`null` parses, but the round trip writes none). The expected document is a
   `GradedProfile`, with each club as `{"grades": …, "list": […]}`.
9. **For P6 and P7**: build `ClubGrades` and `PlayerGrades` with no list, and copy `ball_speed` and
   the three dispersions from the `ClubProfile` (the player's: `face_to_path_deg` and
   `start_line_deg` only). One observation on call 6, left as approved: a shot that missed its shape
   *and* has no landing leaves the on-line denominator, though it cannot have "hit the shape and
   finished on line" whatever its landing. `TopicGrade`'s check encodes call 6 as written, so if
   P6 revisits the call, that check moves with it.
10. **For P8**: `ProfileFinding.ratio`'s doc defines a strength's ratio as the nearer of its two
    bounds to failing, `max(|ci end − target|, sd_ci.high) / tolerance` (below 1, smaller is
    stronger), because call 12 ranks only weaknesses. Revise it with a finding if P8 finds a better
    reading.
11. **For P9**: `run_profile` assembles `GradedClub { grades, list }` per club and `GradedPlayer`,
    and calls `validate()` on the result, which checks every rule above in one pass.

### P2 (2026-10-10)

1. **Landed**: `stats::wilson` and its `WILSON_Z`, with six tests in `stats.rs`'s own test module;
   `crates/analysis/src/strike.rs`, registered in `lib.rs` (whose doc gains an M37 section);
   `crates/analysis/tests/strike.rs` (23 tests, new). `cargo test` (66 binaries, 1196 passed,
   0 failed), clippy and `fmt --check` green. `git diff -- src/ spec/vectors/` is empty. No new
   dependency. Nothing outside the Files line changed.
2. **`wilson(successes: usize, n: usize) -> Option<RateEstimate>`**: `None` at n = 0, a panic when
   `successes > n`, each bound clamped into [0, share] and [share, 1]. z is `WILSON_Z =
   1.959963984540054`, kept apart from the career intervals' six-figure `Z_975`, which must stay as
   it is for their bit-reproduction. Newcombe (1998), Table II, method 3, is reproduced at four
   places (81/263, 15/148, 0/20, 1/29). **The clamp is load-bearing**: 10/10's unclamped upper bound
   is 0.9999999999999999, and every count to n = 60 now validates as a `RateEstimate`.
3. **The judgments as built**, and where they refine the Builds line:
   - **`AnalysedShot`** is a newtype whose only constructor, `AnalysedShot::of(&ShotData) ->
     Result<_, ContractError>`, is `capability::analysed_view`. Every judgment takes
     `&AnalysedShot`, so "takes the analysed view" is a type rule; `.shot()` reads the view.
   - **`shape_of(&AnalysedShot) -> Option<TargetShape>` takes the view, not a number** (the Builds
     line wrote `shape_of(face_to_path)`). Face-to-path is `shot_measure::measure_face_to_path` over
     the view, so the shape and the `face_to_path_deg` row subtract the same way. `None` when either
     angle is missing.
   - `is_centered -> Option<bool>`; `impact_cell -> Option<(FaceColumn, Option<FaceRow>)>`, with
     one `bin` for both axes; `low_point_ahead(&AnalysedShot, ClubCategory) -> Option<bool>`, which is
     also `None` for a category that `low_point_graded_for` refuses.
   - `offline_basis(&AnalysedShot, &[Measurement], &[UnscoredCheckpoint]) -> Result<Landing {
     offline_yds, basis }, UnscoredReason>`. **Its order**: the view's `carry_offline` (printed);
     then the `flight_landing_offline_yds` row, matched by name (projected); then the reason, taken
     from the entry named `carry_offline`, then `flight_landing_offline_yds`, then `flight_carry_yds`,
     **else `Unrecorded`**. A printed-but-blank `carry_offline` falls through to the projection when
     there is one ("used wherever it exists"). The three names are the constants
     `PRINTED_LANDING_FIELD`, `PROJECTED_LANDING_ROW` and `FLIGHT_REFUSAL_ROW`. A test holds the two
     flight names to `FLIGHT_MEASUREMENTS`.
   - `on_line(f64) -> bool`. `CarryAndOffline { carry_yds, offline_yds }`, which is also the typical
     shot's shape; `carry_and_offline(&AnalysedShot)`, read through `shot_measure`'s own
     `measure_carry_distance` and `measure_start_line_offline`; and `is_consistent(shot: CarryAndOffline,
     typical: CarryAndOffline, mishit: bool) -> bool`, where a mishit is `false` first.
4. **Edges**: inclusive everywhere except low point (`> 0`). The test values are exact in f64:
   `0.1 × 150`, `× 250` and `× 100` round to exactly 15, 25 and 10, and `hypot(6, 8)` is exactly 10.
   So a hand vector at the edge passes or fails exactly as written, if it uses the same values.
5. **Shown-only**: no device declares a judged reading `shown_only` today. HD Golf's two
   `shown_only` fields are club speed and smash, and no judgment reads either. So the test holds the
   view's rule per device (a judged reading survives the view exactly when the device declares it
   `analysed`), which starts to bite on the first device that declares one shown-only. HD Golf's
   club speed is the live case of the mechanism.
6. **For P6**:
   - Build the typical shot as `CarryAndOffline { median carry, median offline }` with
     `stats::percentile(_, 0.5)` over the `carry_and_offline` of the club's non-mishit judged shots,
     taking each median separately (call 4).
   - **A club whose judged shots are all mishits has no typical shot.** Every one of them is a miss
     anyway (`is_consistent` answers `false` before it reads `typical`), so the grade is 0 of n with
     no median to build. Decide that case explicitly rather than let a `None` median drop the shots.
   - A judgment's `None` is "cannot judge". Whether that is a printed blank (counted under its reason)
     or not printed (absent) is the builder's question: ask it with `capability::printed_on` on the
     *raw* shot. The reason comes from the `shot_unscored` entry named by the input's `ShotData` key,
     because the screen's tile refusals are named by that key (`crates/screen/src/tiles.rs`).
   - Low point is `low_point_ahead(view, category_of(club))`. For a shape, "hit the shape" is
     `shape_of(view) == Some(declared)` and the grade's success is that `&& on_line(landing)`. An
     `Err` from `offline_basis` goes to `landing_unscored[reason]`.
7. **For P5**: a shot with no landing and no flight entry in its artifact's `unscored` is counted
   under `unrecorded`, which comes before `printed_blank` in declaration order. Use finding 4's
   values at the edges.

### P3 (2026-10-10)

1. **Landed**: `CorpusSwing.shot: Option<ShotData>` and `CorpusSwing.shot_unscored:
   Vec<UnscoredCheckpoint>` in `crates/contracts/src/career.rs`, declared after `intent` in that
   order. They are written only when set (`skip_serializing_if` `Option::is_none` and
   `Vec::is_empty`), and the field docs say why. `narrowed_to`'s doc says a kept swing is carried
   whole. It already was, through `.cloned()`, so the code did not change. Four new tests sit in
   `career.rs`'s own test module, beside `intent`'s precedent, and not under `crates/contracts/tests/`:
   - the exact bytes a swing with neither field writes;
   - the round trip, with both fields written last and in that order;
   - `narrowed_to` by club and by window keeps both;
   - the refusals.
   `tests/career.rs`'s vector walk is still the by-value pin over every recorded corpus, and it
   passes unchanged. `cargo test` ran 66 binaries, 1200 passed and 0 failed. Clippy and
   `fmt --check` are green. `git diff -- src/ spec/vectors/` is empty, and `CAREER_VERSION` is
   still 2.
2. **Outside the Files line, mechanically**: a struct literal must name every field, so five
   `CorpusSwing` literals gained `shot: None, shot_unscored: Vec::new()`:
   - the test builders in `crates/analysis/src/{baseline,club_profile,comparison,dispersion}.rs`;
   - `crates/storage/src/corpus.rs::entry`, the constructor that builds each entry before the
     artifact readers run.
3. **One rule beyond the Builds line**: `Validate` refuses a `shot_unscored` entry whose name is a
   `CHECKPOINT_REGISTRY` checkpoint, naming `CorpusSwing.shot_unscored[i].name`. This is call 3's
   definition held at the shape, not left to each reader. `shot` is validated through `ShotData`'s
   own `Validate`, which bounds the provenance. The names that reach an artifact's `unscored` today:
   - checkpoints, from `analysis/src/engine.rs` at `:570` and `:1137`;
   - flight rows, `flight_carry_yds` and `flight_landing_offline_yds`, from
     `flight_measure.rs` at `:308` and `:321`;
   - photo-side `ShotData` keys, from `core/src/lib.rs:559` and `screen`'s `tile_refusals`.

   No pivot or placement writes one, so "not a checkpoint" is exactly the shot and flight side.
4. **For P4**: filter the artifact's `unscored` by `CHECKPOINT_REGISTRY` before setting
   `shot_unscored`. A checkpoint left in makes every career case that adopts the corpus fail to
   read: the career family reads `input.corpus` through this `Validate`. Fill both fields where
   `entry`'s `measurements` is filled, from the `SwingResult` or `ShotResult` already parsed. That
   parse has already turned the legacy bare-name `unscored` into entries, so `CorpusSwing`'s plain
   `Deserialize` needs no `unscored_accepting_bare_names` of its own. Expect each re-recorded corpus
   swing to gain `shot` (and sometimes `shot_unscored`) as its last keys, after `intent`.
5. **For P5**: a hand corpus's `shot` must pass `ShotData`'s bounds: `parse_confidence` in [0, 1]
   and `parser_version` ≥ 0. Its `shot_unscored` must not use a checkpoint name, or the vector
   refuses to read. Write the keys after `intent`, `shot` before `shot_unscored`, and leave both
   out where there is nothing to write.
6. **For P6**: `narrowed_to` hands each kept swing on whole, which
   `narrowing_keeps_each_swing_s_shot_and_refusals` pins. Read the topics off the club's narrowed
   corpus, as `build_bag_profile` does.

### P4 (2026-10-11)

1. **Landed**: `read_corpus` fills `CorpusSwing.shot` and `.shot_unscored` off the survivor's own
   artifact, in `crates/storage/src/corpus.rs`:
   - a swing reads `analysis.json`'s `swing.shot` through `ShotData`'s validating trait;
   - a photo entry reads `shot_analysis.json`'s `shot`;
   - `shot_side` drops every `CHECKPOINT_REGISTRY` name from the artifact's `unscored` (P3 finding 4).

   `needs_review` became `shot(raw) -> (Option<ShotData>, bool)`, so the flag and the shot come from
   one parse. A swing's shot that the contract refuses stays flagged and is **not carried**, because
   `CorpusSwing` could not validate holding it. Both fields are lifted whatever the entry's repair
   state, as `measurements` is. The module doc gained "The shot and its refusals, carried whole", and
   `crates/storage/src/lib.rs`'s `corpus` line says so (one doc line outside the Files line).
   `CAREER_VERSION` 2 → 3, with its ledger entry. Four new tests sit in `corpus.rs`'s own module, and
   `a_shot_out_of_bounds_reads_as_needing_review` now also asserts the shot is carried or not. The
   four:
   - `a_swing_and_a_photo_entry_each_carry_their_artifacts_shot`;
   - `a_duplicates_shot_is_never_read`, where the survivor holds no shot and does not borrow its
     re-upload's;
   - `an_entry_with_no_shot_carries_none_and_writes_neither_key`;
   - `shot_unscored_holds_no_checkpoint_name`.

   Negative controls: with the filter forced open, the last test fails. With the swing's shot not
   lifted, two tests fail. Both are reverted.
2. **P3 finding 4 was wrong about the swing path.** It said `entry` is filled from a `SwingResult`
   "already parsed". A swing's `analysis.json` is read as a raw map (`load_analysis`, which is
   tolerant on purpose), not typed. So the swing's `unscored` is read **entry by entry**, as
   `measurements` is, and an entry the contract refuses is dropped. A **bare name is dropped too**,
   rather than coerced to `unrecorded` as `SwingResult`'s private `unscored_accepting_bare_names`
   coerces it:
   - that form predates 2026-08-19, and the first non-checkpoint refusal is ADR-027's, accepted
     2026-09-04, so every bare name is a checkpoint, which `shot_side` drops anyway;
   - `contracts` was left untouched as a result.

   `data/` holds no bare name: its 15 analyses' 11 `unscored` entries are all full-form flight rows.
3. **The re-record.** `spec/declarations/career-v3.json` (new) adds the one key `shot` at 37
   distinct paths, moves `career_version`, and moves no value.

   | Kind of path | Where |
   |---|---|
   | `expected.corpus.swings[i].shot` | the storage cases |
   | `expected.narrowed.club-3w/club-7i.corpus.swings[i].shot` | `storage/corpus/real` |
   | `input.corpus.swings[i].shot` | each adopting career case (`career/real/aaron` and five `career/synthetic/storage-*`) |

   - **Recorded cases**: `conflicting-shot-photos`, `every-source`, `mishit-ineligible`,
     `one-photo-two-clips`, `real` and `shot-review`.
   - **Hand-worked M35 cases**: `mixed-session`, `no-face-on-and-no-photo`, `photo-entries-one-photo`,
     `photo-entry-pooled` and `photo-entry-repairs`. Each now carries a `rerecords` entry, as M36's
     hand cases did at `career-v2`.
   - **The counts**: 69 added keys across 17 vectors.
   - **Not foreseen by the plan: no vector gains `shot_unscored`.** No recorded artifact's `unscored`
     holds a non-checkpoint entry. The real corpus's analyses were recorded with `measurements` and
     `shot` alone, the synthetic ones hold empty lists, and no hand `shot_analysis.json` names a
     refusal. So the reader's tests are the only pin on that lift until P5's and P11's cases.
   - **Spot checks**: no duplicate's shot appears, and `shot-review`'s shot, the one missing a
     required key, is not carried. In `photo-entry-repairs`, every analyzed entry carries its shot,
     stale or outdated or not, and the unreadable one does not.
   - **What did not move**: no aggregate and no report, as planned.
   - **The runs**: the dry run listed only declared paths. The real run wrote 132 files. A second run
     and a dry run after it each print *"nothing differs … 132 vectors run (76 storage, 56 career);
     0 changed; 0 files written"*.
   - **The audit**: a scratch script, since deleted, compared each changed file with `HEAD`, less
     `career_version` and `provenance.rerecords`. Every one differs only by added `shot` keys, with
     no line-ending flip, and `real.json.gz` is still gzipped. The shot store's two op vectors
     already held `shot` keys and are otherwise equal.
4. **The measurement**: `golf-core career-corpus --name Aaron --json` over `data/` gives 15
   sessions, 15 directories, 13 swings, 2 duplicates, 0 outdated and 34 metrics with n > 0. That is
   M35 P10 finding 7's numbers exactly, so nothing pooled moved. All 13 entries now carry a shot
   whose `image_sha256` is their `shot_sha256`, and 11 carry `shot_unscored`:
   - `flight_carry_yds`: `carry_unreachable` ×7, `no_club_loft` ×2 and `spin_not_recoverable` ×1;
   - `flight_landing_offline_yds`: `spin_axis_unresolved` ×1.

   The `data/` digest is `bd384cc4…` over 192 files, before and after.
5. **Verified**:
   - `cargo test`: 66 binaries, 1204 passed, 0 failed;
   - clippy and `fmt --check`: clean;
   - the v17 dry run: 0 changed;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty.
6. **For P5**: the profile hand cases are born at `career_version` 3. A hand corpus's `shot` is
   compared as Rust writes `ShotData`, which is **every field, `null`s included**, with `provenance`
   whole (`corrections`, `fields_present`, `raw_fields` and the rest). The re-recorded storage cases
   show the spelling (`storage/hand/photo-entry-pooled.json`). A `shot` written sparsely still
   parses. The comparison is by value, so the input side is fine as long as each case's
   `input.corpus` reads through `Validate`.
7. **For P6**: on real data only 1 of the 13 entries names `flight_landing_offline_yds`. The other
   no-landing shots carry only a `flight_carry_yds` refusal, or none. So P2's `offline_basis` order
   (`carry_offline`, then `flight_landing_offline_yds`, then `flight_carry_yds`, else `unrecorded`) is
   what names most of them. A shot whose shot was refused is `shot_needs_review`, holds no `shot`, and
   is outside the judged set by call 8 already.
8. **For P12**: CONFORMANCE §2's storage and career sections, and §4's runs list, need `career-v3`.
   It adds `shot` and moves nothing, and no vector gains `shot_unscored`. The `rerecord --declare`
   examples at `docs/ARCHITECTURE.md:277` and `docs/CONFORMANCE.md:1140` print `career-v2.json`,
   which `golf-core` now refuses (it is at 3). `CLAUDE.md`'s "re-recorded both under
   `career-v2.json`" line needs the same.

### P5 (2026-10-14)

1. **Resumed.** The first attempt started 2026-10-11 00:01 and hit a usage limit at 00:20. By then
   it had written the nine cases, the gate, and `conformance.py`'s deferral. Nothing outside the
   Files line had changed. This session:
   - re-derived every grade, share, median, grid cell and list entry by hand from each case's own
     swings, against decisions 1–8 and calls 4–12. All of them hold;
   - added the pytest pin;
   - deleted the throwaway pieces: the scratch test `crates/core/tests/zz_m37_p5_scratch.rs`, and
     the generator under `target/m37p5/` (gitignored);
   - ran the verify.
2. **Landed**:
   - **Nine cases** under `spec/vectors/profile/hand/`, all at `career_version` 3 with
     `oracle: "hand"`. They use LF line endings, as the shot family's hand cases do (`spec/** -text`
     keeps them byte-for-byte). The cases: `straight-edge-printed-landing`,
     `left-handed-fade-and-draw-projected`, `draw-on-line-withheld`,
     `consistency-mishit-and-outliers`, `strike-location-radius-and-grid`,
     `low-point-iron-and-driver`, `player-pools-two-clubs-and-untagged`,
     `ungraded-named-impossible-absent` and `the-list-beside-a-drill-and-pose`.
   - **The gate**, `crates/core/tests/profile.rs` (new), with four tests:
     - every case reads through `Validate` and is spelled exactly as `GradedProfile` re-serializes
       it;
     - every count is held to the case's own swings (clubs listed, `n_shots`, `untagged_shots`,
       judged + unscored ≤ what the corpus offers, the player's club-bound topics equal to the clubs'
       sums);
     - `AWAITING` names each case with P9;
     - `the_hand_cases_reach_every_rule_the_plan_names` fails by name if a case drifts off its
       purpose.
   - **The deferral**: `scripts/conformance.py` gains `profile_vector_paths` and a `check` line.
     `tests/test_conformance.py::test_check_defers_the_profile_family_and_judges_no_staleness` pins
     it, and a negative control (the line silenced) fails it.
3. **The case format**: `{career_version, id, input: {bag, corpus}, expected: {profile},
   provenance: {kind: "profile", note, oracle, source}}`.
   - `expected` keys the `GradedProfile` under `profile`, so P11 can put the report text beside it.
   - Each corpus is built by hand the way `read_corpus` writes one. Every swing carries a whole
     `ShotData` (every reading key, `null`s included). Its `launch_monitor:<device>` rows are
     `shot_measure`'s over the analysed view. The flight landing rows (`model:flight_v1`) and pose
     rows are hand stand-ins with `detail: "hand"`.
   - `provenance.source` is shared and states the rules as worked. Each `note` is an intro followed
     by "Working.", giving per-owner counts, medians, misses by swing, the grades with their Wilson
     bounds, the blends and the lists.
4. **Carried, not worked.** These numbers were computed by running Rust once, and **For P6–P9**
   they must come out of exactly the same functions, or the comparison differs in the last bits:
   - `strike.ball_speed` and `strike.dispersion`: `build_bag_profile`'s per club, carried whole;
   - the player's `strike.dispersion` and list: `build_dispersion` over the **whole** corpus;
   - heel/toe's `mean_offset_mm` and `mean_offset_ci`: `stats::mean_and_sd` and `stats::mean_ci`
     over the judged shots' `impact_offset_h`, in corpus order.

   Everything else the hand computed with P2's formulas op for op:
   - `stats::wilson`'s arithmetic and clamp, so its bounds are bit-identical;
   - `stats::percentile(_, 0.5)`'s interpolation for both medians;
   - offline as `carry * sin(to_radians(start))`;
   - the blend as a left-to-right sum in `Topic::ALL` order over the count.

   No judged value sits within 1e-9 of an edge except the exact edge values (P2 finding 4).
5. **What the cases decide that the plan left open.** **For P6, P7 and P9**: meet these, or record a
   finding and change the case.
   - `GradedProfile.clubs` lists **only clubs with a shot** (a club and a `shot_sha256`), in `ClubId`
     order. A bag's never-hit clubs (3w, 5i, 9i and pw in the pooling case) are not clubs of the
     profile, and the gate pins this.
   - A club's `n_shots` is the number of distinct `shot_sha256` among its swings. The player's is
     the same over the whole corpus, and `untagged_shots` the same over swings with no club.
   - A topic exists from the **union** of `printed_on` over all of the owner's shots (call 7). On a
     club where the topic exists, a shot whose input is not printed on it is neither judged nor
     unscored. Only a printed blank is unscored.
   - The untagged shots are an owner of their own. The player pools a club-free topic from them
     only when it exists for them by the same rule, so a shape needs a declared untagged shot.
   - A club whose every topic is withheld has no blend. The player's blend averages the player's
     own graded topics, not the clubs' blends.
6. **The list as worked** (**For P8**):
   - **What lists.** Only `launch_monitor:` rows with a stat scope list. The club's list reads
     club-only rows too, so it holds scatter on `carry_distance_yds` and `ball_speed_mph` (no
     target, a tolerance) and a strength on `start_line_offline_yds`. A strength needs a target.
   - **Order.** Weaknesses come first, largest ratio first, ties in `METRIC_TARGETS` order, then
     bias before scatter. Strengths follow, smallest ratio first.
   - **Ratios**, read off `MetricDispersion`'s own `offset`, `sd`, `tolerance`, `center_ci` and
     `sd_ci`: bias is `|offset| / tolerance`; scatter is `sd / tolerance`; a strength is
     `max(|center_ci.low − t|, |center_ci.high − t|, sd_ci.high) / tolerance`. A strength needs the
     CI inside `t ± tolerance` inclusive and `sd_ci.high < tolerance` strictly.
   - **Pose rows.** In the list case the pose rows establish a bias in the 7i's own
     `ClubProfile.dispersion` (`head_sway_norm`, `finish_balance_norm`) and in the whole-corpus
     dispersion. Only the source filter keeps them off the list. The 7i list is scatter on carry
     (2.7555), bias on face-to-path (2.0), then strengths on `start_line_offline_yds` and
     `start_line_deg`.
7. **Handedness.** The corpus carries none, so "left-handed" exists only in the case's signs: the
   fades' projected landings are negative. The profile reads no handedness (the plan's finding 5),
   and **For P6** nothing may start reading one.
8. **For P9**: delete `AWAITING` and its test along with the skips, and compare `expected.profile`
   with `run_profile(corpus, bag)` by value. The read test already holds each expected document to
   `GradedProfile`'s own spelling.
9. **For P11**: two spots assume every profile case is a hand case, and both must loosen when the
   `oracle: "rust"` corpus cases land:
   - the new pytest pin asserts `oracle == "hand"` for every committed case;
   - `conformance.py`'s `check` line says "worked by hand".

   Also, `read_case` in the gate checks `oracle` only under `profile/hand/`. `rerecord` does not walk
   the family yet.
10. **For P12**: CONFORMANCE §2 gains the family. `profile_vector_paths`' docstring calls it the
    eighth deferred family (after audio, stage, format, screen, storage, career and shot).
11. **Verified**:
    - `cargo test`: 67 binaries, 1208 passed, 0 failed, 1 ignored;
    - clippy: clean;
    - `fmt --check`: clean after `cargo fmt` reflowed the gate;
    - ruff: clean;
    - `conformance.py check`: exits 0 and names "9 profile vectors";
    - the new pytest pin and its neighbours: green;
    - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty.
12. **Blocked: the full pytest, run alone, has 5 failures (2050 passed), and P4 caused them, not
    P5.** P4's verify list had no pytest, so they went unseen. All five are in
    `tests/test_conformance.py`, and all five stop in the frozen career recorder,
    `scripts/conformance_vectors.py`, which neither P4 nor P5 edited. They break because
    `career-v3` added `shot` to every corpus swing that has a photo, and frozen Python's
    `CareerCorpus` has no such field. It still *reads* the vectors (`model_validate` passes). It
    just cannot write the key back:
    - **Four fail in `_career_adopted`'s round trip** (`conformance_vectors.py:6122`,
      "conflicting-shot-photos.json does not round-trip"):
      `test_the_career_cases_build_in_memory_and_write_nothing`,
      `test_the_career_cases_cross_every_floor_and_reach_every_reading`,
      `test_every_report_branch_has_recorded_text` and
      `test_frozen_python_answers_a_career_case_only_under_its_own_engine`.
    - **One fails on the real corpus**: `test_the_real_career_case_starts_from_the_real_storage_corpus`,
      at `:6162`. Frozen `read_corpus` over `data/` no longer equals
      `storage/corpus/real.json.gz`'s `expected.corpus`, and `:6168`'s round trip would fail next.

    The plan lets this phase change frozen-adjacent Python only for the profile deferral (Rules,
    second bullet), and does not name `conformance_vectors.py` at all. So this phase stops for the
    user rather than editing it. The narrowest fix seen: have those two comparisons first remove
    the keys frozen Python has no field for (`shot`, `shot_unscored` on each swing), with a comment
    citing ADR-035 §4. The alternative is to skip the five tests by name. Whichever is chosen, the
    next run re-runs the full pytest and then ticks P5. **Everything else P5 owns is done and
    green.** The fix may show further differences that the first failing assertion hides.

    **The user's answer (2026-10-14): option (a), strip the keys in the recorder.** The next run
    may edit `scripts/conformance_vectors.py`, at `_career_adopted` (`:6122`) and the real-corpus
    checks (`:6162`/`:6168`). Each comparison drops `shot` and `shot_unscored` from each swing
    before it compares, with a comment citing ADR-035 §4: frozen Python reads what Rust writes,
    but it cannot write back keys its `CareerCorpus` has no field for. Do **not** skip the five
    tests. The authorisation covers only removing keys frozen Python has no field for. If the fix
    turns up a difference of another kind, stop and report it as BLOCKED again. Then re-run the
    full pytest and tick P5.
13. **The fix, as authorised (third session, 2026-10-14).** `scripts/conformance_vectors.py` gains
    `_RUST_ONLY_SWING_KEYS = ("shot", "shot_unscored")` and `_as_frozen_python_writes(dumped)`,
    which copies the corpus without those keys on each swing. The comment above them cites
    ADR-035 §4. Three comparisons now go through it, and nothing else changed:
    - `_career_adopted`'s round trip;
    - `_career_real`'s check that frozen `read_corpus` over `data/` equals the storage vector;
    - `_career_real`'s round trip.

    Each case's `input.corpus` still carries the keys, as Rust recorded it, and frozen Python's
    `_run_career` reads it as before because pydantic ignores the extras. **No difference of
    another kind surfaced.** All five tests pass, and so does the real corpus: `data/` read by
    frozen Python equals the stored corpus less the two keys.

    **Verified**:
    - full pytest, alone: 2055 passed, 0 failed;
    - ruff: clean (`ruff format --check` would still reflow this file, but only in blocks that
      predate this phase);
    - `conformance.py check`: exits 0 and names "9 profile vectors";
    - `cargo test`: 67 binaries, 1208 passed, 0 failed, 1 ignored;
    - clippy and `fmt --check`: clean;
    - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty.
14. **For P11 and P13**: the Rules' second bullet names only `conformance.py` and
    `test_conformance.py`, but `conformance_vectors.py` now differs from `HEAD` too, by the user's
    answer to item 12. Frozen Python itself (`src/`) is untouched. If P11's corpus cases, or any
    later `career-v<N>`, give `CorpusSwing` another key that frozen Python has no field for, it
    belongs in `_RUST_ONLY_SWING_KEYS`. Those same five tests are what fail otherwise, and **only
    the full pytest reaches them**, so P11 should run it even though its Verify line omits pytest.
    **For P12**: CONFORMANCE §2's career section can say that frozen Python's career recorder
    compares the adopted corpora without `shot` and `shot_unscored`, and why.

### P6 (2026-10-14)

1. **Landed**, in `crates/analysis/src/strike.rs`:
   - `build_club_grades(&CareerCorpus, &ClubProfile) -> ClubGrades`, its private tally machinery,
     and a public `IMPACT_OFFSET_H`;
   - the module doc now covers both halves, the judgments and the counting.

   `crates/analysis/tests/strike.rs` gains 12 tests, 35 in all. Verified:
   - `cargo test`: 67 binaries, 1220 passed, 0 failed, 1 ignored;
   - clippy and `fmt --check`: clean;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty.

   Nothing outside the Files line changed, and `analysis/src/lib.rs`'s doc already said "P6 and P7
   add the builders".
2. **The preview holds every hand case.** `every_hand_cases_club_grades_are_built_as_worked` builds
   every club of every case: 12 clubs across the 9 cases, each through `build_bag_profile`'s
   `ClubProfile`. It compares each by value with `expected.profile.clubs[i].grades`. All matched on
   the first build: the strike profile, every topic, every Wilson bound and every blend, to the bit.
   **No hand case changed.** Negative control: with the blank reason's default switched to
   `misread`, the strike-location case fails, and the change is reverted. The gate's `AWAITING`
   skips stay for P9.
3. **The calls as built.** Each one refines the Builds line rather than contradicting it:
   - **Signature.** It takes the corpus whole *or* narrowed, and narrows to `profile.club` itself.
     Narrowing is idempotent, so neither caller can hand it the wrong club's shots.
   - **Candidates** (call 8). A candidate counts toward metrics, is not `shot_needs_review`, has a
     tracked intent (checked defensively, since the corpus already drops drills), and carries both
     `shot` and `shot_sha256`. **Each photo is judged once**, on its first swing in corpus order,
     after that filter. That is `baseline::pooled_samples`' rule, and it keeps `judged ≤ n_shots`
     (a `ClubGrades` check) when two clips share a photo. `n_shots` is
     `narrowed.distinct_shots()`, every photo, which equals `ClubProfile.n_shots`.
   - **"Analysed", not "printed".** Topic existence and whether a shot is seen use
     `capability::analysed_on`, not `printed_on`. The two agree on every topic input for all three
     declared devices today. They differ only for a future device that prints a judged reading
     shown-only: then the topic is absent, rather than every shot `printed_blank` because the view
     blanked it. The module doc says so.
   - **A shot whose device is undeclared** is not a candidate, since `AnalysedShot::of` refuses it.
     It still counts in `n_shots`, and the builder stays infallible.
   - **Declared means `shot_shaping` with that `target_shape`** (call 7). A `target_shape` under
     `fundamentals` declares nothing. The P5 gate's `declared` checks only the shape, which is an
     upper bound, so the two agree.
   - **The unscored reason** is the `shot_unscored` entry named for the first input (in
     `Topic::inputs` order) that holds no value in the view, else `printed_blank`.
   - **A club of only mishits** has no typical shot and grades 0 of n. Its shots are not dropped
     (P2 finding 6).
   - **The club blend is built here** (`blend_of`), not in P7, because `ClubGrades` does not
     validate without it. It is the left-to-right sum in topic order over the count.
4. **For P7**:
   - **What to reuse.** `OwnerTally::of(swings, club: Option<ClubId>)` already handles the
     untagged owner: with `None`, consistency and low point are absent through `needs_a_club`.
     `Tally` keeps raw `successes` even when its share is withheld, and pooling needs that, since
     `Share::Withheld` carries no successes. `blend_of` is the player's blend as it stands.
   - **`count_under` adds one shot.** Pooling needs it generalised to add `n`, and it must keep
     the declaration-order sort.
   - **Rebuild the player's heel/toe from the whole corpus, in corpus order.** Club strikes
     concatenated club by club are *not* in corpus order, and `mean_and_sd`'s compensated sum can
     differ in the last bit with order (P5 finding 4). The same holds for deduping photos at player
     level: if one photo sits under two clubs (a data error), summing the clubs' tallies counts it
     twice and the player's `judged` can exceed its `n_shots`. Either judge the whole corpus once,
     tagging each candidate with its own club, or record why summing is safe.
   - **`GradedProfile.clubs`** lists only clubs with a photo (P5 finding 5), while
     `build_bag_profile` also profiles a declared, never-hit club. So `build_grades` filters on
     `n_shots > 0`, or on the club holding a `shot_sha256`.
   - **A player-level topic exists** when it exists for any club, or for the untagged owner (P5
     finding 5).
5. **For P9**: `crates/analysis/tests/strike.rs`'s preview compares `ClubGrades` alone. Its lists
   are P8's and the player's grades P7's. The gate's whole-profile comparison is still P9's.

### P7 (2026-10-14)

1. **Landed**, in `crates/analysis/src/strike.rs`:
   - `build_player_grades(&CareerCorpus) -> PlayerGrades`;
   - `build_grades(&CareerCorpus, &BagProfile) -> ProfileGrades`, and `ProfileGrades { clubs:
     Vec<ClubGrades>, player: PlayerGrades }`. It has no serde and no `Validate`, because the wire
     shape is P9's `GradedProfile`;
   - the builder's internals, restructured so one tally serves a club and the player:
     - `OwnerTally` became `Criteria`: an owner's existing topics, its category, and its typical
       shot, taken from the owner's own candidates;
     - `Owned` pairs a candidate with its owner's `Criteria`;
     - `PoolTally` tallies a pool of `Owned` shots, each topic seeing only the shots whose owner
       has that topic.

   The tally functions now take `&[Owned]`, and `candidates` takes any iterator of swings. The
   typical shot moved out of `tally_consistency` into `Criteria::of`, over the same shots in the same
   order. The module doc gains a section on the player.

   `crates/analysis/tests/strike.rs` gains 6 tests, 41 in all:
   - own-club median;
   - untagged shots reach strike location and the shapes only;
   - a photo under two clubs is judged once;
   - the player's blend;
   - the player's raw numbers are club-independent;
   - `build_grades` lists only clubs with a photo.

   The preview is renamed `every_hand_cases_grades_are_built_as_worked`. It now goes through
   `build_grades` and compares, by value, the clubs listed, every club's grades and the player's
   grades. **All 9 cases matched on the first build, the player's to the bit**, with no hand case
   changed. The pose test now covers the player too.

   Verified:
   - `cargo test`: 67 binaries, 1226 passed, 0 failed, 1 ignored;
   - clippy and `fmt --check`: clean;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty;
   - files changed since the marker: the two Files-line files and this plan.
2. **The signature differs from the Builds line.** It is `build_player_grades(corpus)`, with no
   `&[ClubGrades]`, because summing the clubs cannot work:
   - a withheld club share carries no successes to add;
   - summing would also count twice a photo filed under two clubs.

   Instead it judges the whole corpus once, each photo on its first candidate swing (P6 finding 4's
   first option), and each shot by the `Criteria` of its own owner. Those `Criteria` come from the
   owner's swings alone: `swing.club == club`, which is `narrowed_to`'s club filter, or
   `club.is_none()` for the untagged owner. So a shot is judged exactly as its club's grade judged
   it. A topic exists for the player when any owner's `Criteria` has it, which is P5 finding 5.

   `count_under` did not need generalising, because nothing adds tallies now.
3. **Negative controls**, each reverted:
   - every owner's `Criteria` taken from the whole corpus: the own-median test, the blend test and
     the preview fail;
   - `PoolTally` with no per-owner topic filter: the untagged test fails (an untagged shot reaches
     low point) and so does the preview;
   - the clubs' candidates concatenated with no dedupe across clubs: the two-clubs test fails with
     `PlayerGrades` refusing "consistency judged 11 of 10 shots".

   **What the controls do not show.** The hand cases still pass under the third control, so no
   committed case shows a heel/toe mean that changes with summation order. Corpus order is kept on
   P6 finding 4's reasoning, not on a measurement.
4. **For P8**: `build_player_grades` computes `build_dispersion(corpus)` over the whole corpus and
   keeps only the keys `is_club_independent` passes. It does not return the rest. The player's list
   must therefore call `build_dispersion` itself, which gives the same bits because it is the same
   function on the same corpus (P5 finding 4).
5. **For P9**: `run_profile` is three steps:
   1. `build_bag_profile(corpus, bag)`;
   2. `strike::build_grades(corpus, &bag_profile)`, which already drops never-hit clubs;
   3. the lists.

   `ProfileGrades` carries no `player_id`, so take `corpus.player_id`. The analysis preview now
   compares every grade, the player's included. That leaves only the lists, and the whole document,
   for the gate.

### P8 (2026-10-14)

1. **Landed**:
   - `crates/feedback/src/profile_rules.rs` (new, no Python twin, says so): `rank_profile(&ClubProfile)
     -> Vec<ProfileFinding>` and `rank_player(&GolferDispersion) -> Vec<ProfileFinding>`. The rule is
     P5 finding 6 as worked. It re-decides nothing: a weakness is the dispersion's own established
     finding, and a strength is its two intervals read the other way;
   - `crates/feedback/src/lib.rs` registers it, names it as the second module and the one with no
     twin, and its stale "depends on `contracts` and nothing else" now names `pyfmt` too;
   - `crates/feedback/tests/profile_rules.rs` (new, 12 tests). Every fixture is a `MetricDispersion`
     decided by `analysis::dispersion`'s rule, restated in the test because the crate cannot reach
     it. The tests cover: bias and scatter ranked by ratio, a bias ranked by its size on either side,
     ties, a strength needing both halves, the two strength edges, the strength ratio's nearest
     bound, nothing established, no pose metric and no projection, only a launch-monitor reading,
     the player's list club-independent, an empty dispersion, and the cargo edge. The edge test
     reads every workspace member's manifest and holds `feedback` to `contracts` and `pyfmt`.

   Verified:
   - `cargo test`: 68 binaries, 1238 passed, 0 failed, 1 ignored;
   - clippy and `fmt --check`: clean;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty;
   - files changed since the marker: the three Files-line files, this plan, and one doc comment in
     `crates/contracts/src/strike.rs` (finding 5).
2. **Every hand case's list matches, to the bit, with no case changed.** That is 21 lists across the
   9 cases: each graded club and each player. A throwaway test in `crates/core/tests/` built them as
   P9 will and compared by value, then was deleted.
3. **The player's twin takes `&GolferDispersion` alone**, not the Builds line's `PersonalBaseline`
   as well. The rule reads only `MetricDispersion`'s fields (P5 finding 6), and `center_ci` and
   `sd_ci` there are the baseline's `mean_ci` and `sd_ci`, carried. A second argument would be a
   second spelling of the same two intervals. **For P9**, per profile:
   - each club: `rank_profile(bag_profile.profile_for(club))`, for each club `build_grades` lists;
   - the player: `rank_player(&build_dispersion(corpus))` (P7 finding 4).

   That is exactly what finding 2's throwaway did.
4. **P5 finding 6's "only the source filter keeps [the pose rows] off the list" is imprecise.** No
   pose metric has a stat scope, so the scope filter refuses them too. Measured: the hand cases pass
   with either filter removed.

   Both filters stay, each with a test of its own that fails without it:
   - `no_pose_metric_and_no_projection_ever_lists` fails without the scope filter;
   - `only_a_launch_monitor_reading_lists` fails without the source filter. Its rows are
     `start_line_deg` from four non-launch-monitor sources.

   The module doc says what the source filter alone guards: a shot number's name read by another
   instrument, or a pose metric that gains a scope. Either would otherwise be ranked against a
   `JUDGED_*` OCR tolerance.
5. **A strength's ratio is exactly 1 at the inclusive edge**, which a test pins. `ProfileFinding`'s
   doc in `crates/contracts/src/strike.rs` said "below 1" and now says "at most 1". This is the
   revision P1 finding 10 invited, and its reading of the ratio otherwise stands.
6. **Negative controls**, each reverted:
   - the mean's interval made strict, or the SD's bound made inclusive: the edge test fails;
   - no `METRIC_TARGETS` tie-break among weaknesses: the tie test fails, because `carry_distance_yds`
     sorts ahead of `start_line_offline_yds` alphabetically and after it in the table;
   - no club-independent filter on the player: two tests fail;
   - either listing filter dropped: finding 4.

   **What the controls do not show.** Dropping the strengths' tie-break fails nothing, and no test
   can make it. The three metrics that can be strengths today (`face_to_path_deg`,
   `start_line_deg`, `start_line_offline_yds`) sort the same alphabetically and in the table, so the
   test pins the outcome and says so.
7. **For P12**: `crates/feedback/Cargo.toml`'s `description` still reads "the port of
   src/golf_coach/feedback/rules.py". It is outside P8's Files line, so it is left as found.

### P9 (2026-10-14)

1. **Landed**:
   - `crates/core/src/profile.rs` (new, no Python twin, says so): `run_profile`, P7 finding 5's three
     steps as P8 finding 3 spelled them. `build_bag_profile`, then `build_grades`, then
     `rank_profile(bag_profile.profile_for(club))` per graded club and
     `rank_player(&build_dispersion(corpus))` for the player, with `corpus.player_id`. One unit test:
     an empty corpus answers an empty profile (no club, no topic, no blend, nothing listed) rather
     than a refusal;
   - `crates/core/src/profile_family.rs` (new): `KIND`, `run_profile_case(&Value) -> Result<Value,
     String>` answering `{"profile": …}`, and `differences`, which is §3's `compare` under
     `profile.`. Two unit tests: a vector of another family is an `Err`, and the answer keys only the
     profile;
   - `crates/core/src/lib.rs` registers both and gains a "The graded profile" section;
   - `crates/core/tests/profile.rs`: `AWAITING`, its `P9` and `KIND` constants, and
     `every_comparison_awaits_its_phase_by_name` are deleted. Two comparisons replace them (finding
     3), and the module doc now describes what is checked rather than what waits.

   Verified:
   - `cargo test`: 68 binaries, 1242 passed, 0 failed, 1 ignored;
   - clippy and `fmt --check`: clean;
   - docs-truth: 92 passed;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty;
   - files changed since the marker: the four Files-line files, this plan, and one doc comment
     (finding 6).
2. **Every hand case passes on the first run, under §3's rules and by value, with no case changed.**
   That is 9 cases and 746 floats bit-identical. The grades had matched since P7 and the lists since
   P8's throwaway, so this is the first time the whole document, assembled by the function the verb
   will call, is compared.
3. **The gate is two tests**, mirroring the career family's split:
   - `every_profile_case_is_run_profiles_answer` runs every case through `run_profile_case` (the
     runner P11's re-record will share) and compares under §3's rules on every target. It walks the
     directory, so a new case is compared as soon as it is committed.
   - `every_profile_is_run_profiles_by_value` is P5 finding 8's typed comparison
     (`GradedProfile == GradedProfile`), and it reports any float that differs by path, to the bit.
     It runs only on `cfg(all(target_os = "windows", target_env = "msvc"))`. The reason is the one
     the career family's bit test gives: the carried numbers (dispersions, the ball-speed baseline,
     heel/toe's interval) go through `analysis::stats`' `pow`, which is the C library's, and another
     libm may differ in the last bit.

   **Negative controls**, each reverted:
   - every club's list emptied: both tests fail (8 differences under §3's rules, 7 by value);
   - one weakness ratio nudged by one ulp: §3's gate **passes**, and the by-value test fails on the
     4 cases whose first club finding is a weakness. That is why the second test exists.
4. **The signature differs from the Builds line.** It is `run_profile(&CareerCorpus, Option<&Bag>)
   -> Result<GradedProfile, ContractError>`, not a bare `GradedProfile`. P1 finding 11 has it call
   `validate()` on the whole profile, and a refusal is returned rather than panicked, as
   `analyze_shot` returns one, so a builder bug met over `data/` is reported, not an abort. The
   builders promise the refusal cannot happen, and no case or control reached it. The one `expect`
   left is `profile_for` on a club `build_grades` took from that same `BagProfile`.
5. **`run_profile_case` refuses by `provenance.kind`, not `recorded_by`**, because no profile case
   carries a `recorded_by` (no script recorded one). It reads `input.corpus` and `input.bag` and not
   `career_version`, which stays the gate's to check and the re-record's to move, as in
   `run_career`.
6. **Outside the Files line**: `crates/analysis/tests/strike.rs`'s preview doc said "P9's gate …
   keeps its skips until then". That one comment now names `crates/core/tests/profile.rs` as the
   gate. No code there changed.
7. **For P10**:
   - The verb calls `run_profile(&corpus, bag.as_ref())` and should turn an `Err` into a refusal on
     stderr with a non-zero exit, as the career verbs do with theirs.
   - An empty corpus answers an empty `GradedProfile`, not an `Err`. A golfer with nothing graded
     is the report's "nothing to grade" case, not the unknown-golfer refusal.
8. **For P11**:
   - `run_profile_case` answers `{"profile": …}` alone. The corpus cases' `expected` carries the
     report text beside it, so the runner (or the re-record) has to add that key, and `differences`
     compares only `profile` today.
   - The gate's `read_case` pins `expected`'s keys to exactly `["profile"]`, and every test in
     `tests/profile.rs` reads every case through it (the hand-reach test filters to `profile/hand/`
     only after reading). That pin must loosen, or split by `profile/hand/` against
     `profile/corpus/`, when the corpus cases land.
   - A corpus case needs `provenance.kind: "profile"`, or the runner refuses it.
   - P5 finding 9's two hand-only spots still stand.
9. **For P12**: CONFORMANCE §2's profile section should say that the gate compares under §3's rules
   on every target and by value on the target the cases were worked on, and give the reason
   (finding 3).

### P10 (2026-10-14)

1. **Landed**:
   - `crates/core/src/reports/profile.rs` (new; no Python twin and no script, and says so):
     `profile(&GradedProfile, display_name, Option<ClubId>) -> String`. Each club prints its shots,
     then the impact grid (high, center and low rows, plus a "no height" row; heel, center and toe
     columns) and the lean (heel and toe counts beyond the radius, and the mean offset with its CI
     or the floor it waits for). Then the spreads: ball speed and the three strike dispersions, each
     an sd with its CI, or the baseline's own withheld sentence. Then each topic's grade with its
     interval and `successes of n`. A shape topic also prints hit-the-shape, its landings
     ("projected by the flight model and not printed") and its landing refusals by reason. Then the
     blend, with what it averaged and left out, and the list. The player follows, under "all
     clubs". Three unit tests.
   - `reports/mod.rs`:
     - `Verb::Profile` joins `Verb::ALL`, which now has five read verbs; `Verb::takes_club` and
       `Verb::takes_verbose` are new.
     - `answer`'s arm calls `run_profile(&corpus, bags.get(player_id).as_ref())`. An `Err` writes a
       stderr line naming the golfer and the field, and exits 1 once the other golfers have
       printed (P9 finding 7). One unit test covers that line.
     - `--json` writes `{"display_name", "profile"}` per golfer.
   - `bin/golf_core.rs`: `--club` where `takes_club`, `--verbose` refused for `profile`, the USAGE
     line, and a `# profile` section in the module doc.
   - `tests/reports.rs`: `tree_of(case)` generalises `hand_tree`, and there are three new tests:
     - the verb over `storage/hand/photo-entry-repairs.json`'s tree, covering the report, `--club`,
       `--json`'s round trip, an unregistered id, and a byte snapshot showing the tree is unwritten;
     - the empty state over `HAND_CASE`;
     - the renderer against all nine hand profiles.

     `Verb::Profile` also joins the JSON-keys test and both `Verb::ALL` loops (the refusals and the
     binary).

   Verified:
   - `cargo test`: 68 binaries, 1249 passed, 0 failed, 1 ignored;
   - clippy and `fmt --check`: clean;
   - docs-truth: 92 passed;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty;
   - the `data/` digest: `bd384cc4…` over 192 files, before the verb ran over `data/` and after;
   - files changed since the marker: the four Files-line files and this plan.
2. **"An unknown golfer is refused as the other career verbs refuse one" does not match what those
   verbs do, so P10 followed the clause "as the other career verbs".** They do not refuse an
   unregistered golfer:
   - `targets` reads an id with no record as `<id> (not registered)` with a corpus of zero, because
     a swing can name a golfer whose record was deleted.
   - Their only refusal is a `--name` with no usable characters (exit 2). An empty registry is
     reported as a finding with exit 0, not a refusal.
   - Only `flag-mishit --list` refuses a name that no record answers.

   `profile` shares `answer`'s `targets`. So `--name '!!!'` exits 2 with the same sentence (on
   stderr under `--json`), and an unregistered id prints "(not registered)" and "nothing to grade
   yet", exit 0. If an unregistered id should be refused instead, that is one `Targets` branch for
   `Verb::Profile` alone.
3. **`--verbose` is refused** (exit 2, `profile: unrecognized arguments: --verbose`). That is this
   phase's call:
   - decision 9 names `--name`, `--club` and `--json`;
   - the report already prints everything the profile carries;
   - a flag accepted and ignored would be a lie.

   `--player-id`, `--sessions-dir` and `--golfers-dir` are accepted, as every career verb accepts
   them.
4. **`--club` narrows the text and not the JSON**, as `club-profile`'s does:
   - Under `--club` the player block is not printed.
   - Text that is not a club is refused before the golfer is looked up (exit 2).
   - A club with no tracked shot prints one line saying so.
   - `--json` prints the whole `GradedProfile` under `profile`. It reads back, through `Validate`,
     equal to `run_profile`'s answer.
5. **The measurement**: `golf-core profile --name Aaron` over `data/`, read-only, digest unchanged.
   - **Two clubs**: 3w (6 shots) and 7i (5 shots). Each has consistency alone, withheld at 6 and 5
     of 10, with no blend, nothing listed, and every spread withheld ("needs 10 samples").
   - **Strike location, low point and every shape are absent**, as planning findings 3 and 8
     expected.
   - **The player**: 13 shots, 2 untagged. Consistency is **graded**: 55% (6 of 11), 95% CI
     28% .. 79%. The blend is 55%.
     - The list: face-to-path bias at 3.80× tolerance and scatter at 3.07×, start-line scatter at
       2.01×.
     - The spreads: face-to-path sd 6.1° (n 12), start-line sd 4.0° (n 13).
   - **So the first grade real data shows is the player's.** Pooling reaches the floor where no
     club does, because each shot is judged by its own club's median (ADR-034 §5.4). That is the
     design rather than a defect, and worth one line in the WORKLOG.
   - **`--json`**: one array entry with the keys `display_name` and `profile`. Every `null` in it
     sits inside a carried `MetricBaseline` or `MetricDispersion`, the exemption `tests/strike.rs`
     already makes.
6. **The report text is Rust's own wording, and until P11 nothing recorded pins it.** The renderer
   test holds it to each hand profile structurally:
   - a topic is printed exactly when it is carried;
   - a withheld grade's row starts with "withheld";
   - a projected landing is named as projected;
   - the blend prints its mean, or says why it has none;
   - every list metric is printed;
   - no pose-metric text appears anywhere.

   That test reads the key column at column 14, 24 wide (`profile.rs`' `LABEL_WIDTH` and
   `KEY_WIDTH`).
7. **For P11**:
   - A corpus case's report text is `golf_core::reports::profile::profile(&profile, display_name,
     None)`. The verb's trailing blank line is not part of it, the same split `run_career` makes.
   - The career runner takes the name from `input.display_name`. An adopted case can carry the
     career case's value.
8. **For P12**:
   - `profile` is the sixth career verb: five read verbs in `Verb::ALL`, plus `flag-mishit`. Two
     docs still say five: `CLAUDE.md`'s Commands block ("one of five career verbs") and
     ARCHITECTURE §1 ("the five and their flags").
   - ARCHITECTURE §1 needs `profile`'s flags: `--verbose` is refused, and `--club` narrows the text
     and not the JSON.
   - Finding 2's reading of "refused" should be stated wherever the verb is described.
9. **For P13**: the smoke run over `data/` should print what finding 5 records, unless `data/` has
   changed by then.

### P11 (2026-10-14)

1. **Landed: everything, the pin's audio and format half included** once the user answered finding 3
   (finding 10 is the resumed run that applied the answer).
   - **`spec/vectors/profile/corpus/` (new), six cases**, all at `career_version` 3 with
     `oracle: "rust"`, LF and indent 2 as the hand cases are. There is one per career case whose
     corpus carries a shot: `real-aaron` (`career/real/aaron`) and five storage twins,
     `storage-conflicting-shot-photos`, `storage-every-source`, `storage-mishit-ineligible`,
     `storage-one-photo-two-clips` and `storage-shot-review`.
     - `input` is `{bag, corpus, display_name}`, copied whole from the career case. The storage
       case has no bag or name, so the career case is the source.
     - `expected` is `{profile, report}`, written once by `run_profile_case` through a throwaway
       test, since deleted. `report` is `reports::profile::profile(&profile, name, None)`, without
       the verb's trailing blank line (P10 finding 7).
     - `provenance` is `{kind, note, oracle, source}`. `source` begins with the career case's path.
       Each `note` (ASCII) says what the case holds, and that its correctness rests on `hand/`
       while it pins stability (call 18).
     - `real-aaron` is a plain 212 KB file. That is twice the largest hand case, and it stays plain
       so the gate and `conformance.py` read the family as one kind of file.
   - **`profile_family.rs`**:
     - `run_profile_case` answers `report` exactly when the input carries `display_name`, so the
       hand cases keep `{profile}`.
     - `differences` compares the whole `expected`, profile and report together.
     - New: `ADOPTED` and `adopted_from`. The source must begin `spec/vectors/career/`, and any
       other vector path is an `Err`.
     - The module doc is rewritten for two sub-families, and one unit test became two.
   - **`rerecord.rs`**:
     - New: `PROFILE`, `PROFILE_DOCUMENTS = ["hand", "corpus"]` and `Family::Profile`.
     - `walk_career` walks the profile family after the career family. It keeps each career
       document as it will be written (as committed if refused, as for storage), and derives a
       corpus case's `ADOPTED` inputs from it.
     - Every entry under `vectors/profile/` must be placed, and a run over a `spec/` with no
       `vectors/profile/` is refused.
     - The report's count line reads `(N storage, N career, N profile)`.
   - **`tests/profile.rs`**:
     - `read_case` splits by sub-family. A hand case must have `oracle: "hand"`, input
       `{bag, corpus}`, expected `{profile}`, and adopt nothing. A corpus case must have
       `oracle: "rust"`, input `ADOPTED`, expected `{profile, report}`, and adopt a career case.
       Any other sub-directory panics.
     - New: `every_adopted_input_is_its_career_cases`, which is the career gate's adoption pin
       one link further.
     - New: `every_career_case_carrying_a_shot_has_its_corpus_case`, the census both ways, with
       each career case adopted at most once.
   - **`tests/rerecord.rs`**:
     - Slices copy the profile family, and the count line has three counts.
     - The version-move test (renamed `…_lands_on_its_three_families_…`) checks the walk order,
       storage then career then profile. It also checks that every document keeps its oracle,
       `hand` and `rust` included.
     - The sub-family test gains `profile/unplaced/`.
     - New: `an_adopted_profile_input_moves_with_its_career_case_and_only_where_declared`.
     - New: the oracle pin, `every_committed_vector_names_its_oracle_and_every_ledger_entry_its_declaration`.
   - **Python**, P5 finding 9's two spots:
     - `conformance.py`: the profile docstring, and the check line now reads "worked by hand or
       recorded by Rust".
     - The pytest pin: `hand` under `hand/`, `rust` under `corpus/`, both present. Its "ahead"
       case is now taken from `hand/` explicitly.
   - **Outside the Files line**: three doc comments now name the profile family in a career run,
     two in `crates/core/src/lib.rs` and one in `bin/golf_core.rs`'s module doc.
2. **The `career-v3` dry run** reports *"nothing differs from the committed vectors … 147 vectors
   run (76 storage, 56 career, 15 profile); 0 changed; 0 files written"*. No real run was needed:
   nothing differed, so no new case gained a ledger.
3. **Blocked: the oracle pin's first half cannot hold on today's files without breaking a Rule.**
   - **The census.** Of 305 committed vectors, every family names an oracle in
     {`python`, `hand`, `rust`} except `audio` (30) and `format` (13), which carry no `oracle` key.
   - **The conflict.** CONFORMANCE §2 says M37's pin "extends `oracle` to them", and ADR-032's
     twelfth addendum says the pin "covers the five that exist". But the Rules forbid changing
     either family in M37 (`git diff -- spec/vectors/{…,audio,format}` empty at every phase).
   - **The key is more than one file edit.** `test_the_format_table_regenerates_byte_identically`
     compares every format file with `conformance_vectors.build_format()`'s output. So writing the
     key into the files also needs `_format_vector` to write it, and that is frozen tooling this
     plan does not name.
   - **What is built meanwhile.** The pin skips `audio` and `format` by name
     (`AWAITING_AN_ORACLE` in `tests/rerecord.rs`), with the assertion turned round: it fails if
     either starts carrying the key, so the skip cannot outlive the answer.
   - **The options:**
     - **(a)** Write `"oracle": "python"` into the 43 files' `provenance`, and have
       `conformance_vectors.py`'s `_format_vector` (and `_audio`, to keep it in step) write it too.
       This breaks the "audio and format unchanged" Rule and touches frozen tooling.
     - **(b)** The pin names those two families' oracle in its own table (`audio` → `python`,
       `format` → `python`) and leaves the files untouched. P12 then corrects CONFORMANCE §2's
       sentence. **Recommended**: it keeps both Rules, and the pin still fails on any family or file
       with no oracle or an unknown one.
     - **(c)** Leave the two out of the pin, named, until a later milestone.
   - **The second half holds and is pinned.** 450 ledger entries name 5 declarations. Each
     declaration is committed, each entry carries its declaration's own version key and value, and
     each is `by` the verb.
   - **What the next run does**: it applies the answer, re-runs `cargo test` (and, under (a), the
     full pytest and `conformance.py check`), and then ticks P11.
   - **The user's answer (2026-10-15): (b).** The pin names `audio` → `python` and
     `format` → `python` in its own table, the 43 files and `conformance_vectors.py` stay
     untouched, and P12 corrects CONFORMANCE §2's sentence to match. Replace the
     `AWAITING_AN_ORACLE` skip with that table, keeping a check that fails if either family starts
     carrying the key in its files, so the table and the files cannot both claim it.
   - **Also left by the cut-off run**: finding 7's full-pytest line still reads the placeholder
     `PYTEST_RESULT`. The run was cut off by the usage limit before pytest reported, so the next run
     runs the full pytest alone and writes in its real result.
4. **What the corpus cases grade.**
   - `real-aaron` is P10 finding 5's measurement exactly. 3w (6 shots) and 7i (5) have consistency
     alone, withheld, with no blend and an empty list. The player has 13 shots (2 untagged) and
     consistency graded at 6 of 11, 95% CI 28% .. 79%, with a blend of 55%. Its list holds
     face-to-path bias 3.80×, face-to-path scatter 3.07× and start-line scatter 2.01×.
   - The five storage twins have no topic at all. Their shots print nothing, so what they pin is
     the counting: one photo is one shot across two swings, a swing with no photo hash is no shot,
     and the 7i in `mishit-ineligible` is a club of the profile with 8 shots and no topic. They
     also pin the report's no-topic and no-club branches over recorded corpora.
5. **A refusal does not cascade down the chain.** A career twin the gate refuses is passed on as
   committed, so the profile case that copies it is not refused a second time for the same missing
   path. The new rerecord test pins this (one refusal, at the twin). Since a career twin and a
   profile corpus case spell their copy at the same path (`input.corpus…`), one declared path
   covers both.
6. **Negative controls**, each restored and checked by sha256:
   - a corpus case's report text altered: the gate fails at `report`;
   - a corpus case removed: the census fails;
   - a format file given an oracle: the pin's skip fails (re-run against the table in finding 10,
     where it fails at `ORACLE_BY_FAMILY`);
   - a shot case's oracle set to `robot`: the pin fails;
   - a ledger entry with no declaration: the pin fails;
   - a ledger entry at another version than its declaration: the pin fails.
7. **Verified**:
   - `cargo test`: 68 binaries, 1254 passed, 0 failed, 1 ignored;
   - clippy and `fmt --check`: clean;
   - ruff: clean;
   - `conformance.py check`: exits 0 and names "15 profile vectors";
   - the full pytest, alone: 2055 passed, 2 warnings, run by the resumed session (finding 10). The
     warnings are `.pytest_cache` writes denied by Windows, not a test's;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty.
   The controls left three committed files with new mtimes and the same bytes:
   `career/synthetic/thin.json`, `format/lower.json` and `shot/corpus-2026-08-23-11.json`.
8. **For P12**:
   - CONFORMANCE §2 gains the profile family: `hand/` and `corpus/`, their oracles, `report` on
     the corpus cases, and the adoption chain storage → career → profile.
   - Its "Who recorded each value" paragraph needs the oracle pin as finding 3's answer settles it,
     and the sentence about `audio` and `format` corrected to match.
   - §4: a `career-v<N>` declaration re-records three families, and the run line now ends
     `(N storage, N career, N profile)`.
   - ARCHITECTURE and `CLAUDE.md` should say the same wherever they name what a career declaration
     re-records.
9. **For P13**: the `career-v3` dry run now reports 147 vectors (76 + 56 + 15), not 132.
10. **Resumed 2026-10-15 to apply finding 3's answer, (b).** The resumed run listed the files newer
    than the marker. They were the Files line's, the three outside-the-Files-line comments
    (finding 1) and finding 7's three restored controls. It kept all of them as the cut-off run
    left them.
    - **`tests/rerecord.rs`**: `AWAITING_AN_ORACLE` is gone. In its place is
      `ORACLE_BY_FAMILY = [("audio", "python"), ("format", "python")]`, whose doc says why each is
      Python's: the deleted `audio/impact.py` recorded audio (M20), and format is CPython's output
      through `build_format`. A family in the table is counted under the table's oracle, and the
      pin fails in three cases: any of the family's files carries `provenance.oracle` itself (the
      table and the files cannot both claim it); the table names an oracle outside `ORACLES`; or a
      table row names a family with no committed vector (a stale row).
    - **The census, now whole**: 305 vectors, every family with an oracle. They are audio python
      30, career python 56, corpus python 15, format python 13, profile hand 9 and rust 6, screen
      hand 7 and python 41, shot hand 25, stages python 21, storage hand 15 and python 61, and
      synthetic python 6. There are 450 ledger entries naming 5 declarations.
    - **Control**: `format/lower.json` given `"oracle": "python"` fails the pin at
      `ORACLE_BY_FAMILY`. Restored, and checked identical by sha256.
    - **Verified**: `cargo test` 68 binaries, 1254 passed, 0 failed, 1 ignored. clippy and
      `fmt --check` are clean, ruff is clean, and `conformance.py check` exits 0 naming "15 profile
      vectors". The `career-v3` dry run reports 147 run (76 storage, 56 career, 15 profile) with 0
      changed. The full pytest, run alone, has 2055 passed. And
      `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}` is empty,
      so the 43 audio and format files and `conformance_vectors.py` are untouched.
    - **For P12**: CONFORMANCE §2's "Who recorded each value" says `audio` and `format` "carry
      neither key yet" and that M37's pin "extends `oracle` to them". Correct it to the answer: the
      files still carry no `oracle` key, and the pin names both as `python` in its own table
      (`ORACLE_BY_FAMILY`, `crates/core/tests/rerecord.rs`). The table fails if a file starts
      carrying the key. ADR-032's twelfth addendum says the pin "covers the five that exist", and
      that needs the same note in its M37 addendum.

### P12 (2026-10-15)

1. **Landed**, every item on the Files line and every "For P12" finding (P4 8, P5 10 and 14, P8 7,
   P9 9, P10 8, P11 8 and 10):
   - **`docs/CONFORMANCE.md`**:
     - the banner and §2's intro now say ten families;
     - "Who recorded each value" gives the `career-v3` ledgers, the profile oracles and the oracle
       pin. The `audio`/`format` sentence is corrected to `ORACLE_BY_FAMILY`, as the user answered
       P11 finding 3;
     - the storage table's version column is moved from 2 to 3 (finding 2);
     - a paragraph on the corpus carrying the shot (finding 3);
     - the career section gains version 3 and "How they came to be at version 3", which includes
       the frozen recorder stripping `shot` and `shot_unscored`;
     - a new `### Profile` section covers both sub-families, the file shape, carried against worked
       numbers, the three-link adoption chain, the two-comparison gate and why it has two, the two
       arithmetic choices, no schema, and what no real device reaches;
     - §4 covers `career-v3` in the command block, the career key naming three families, the walk
       order and count line, the runner, the adoption chain one link longer with no cascading
       refusal, LF profile cases, M37's run among the worked examples, `profile/corpus/` as the one
       other way a vector is born, `check`'s deferral and `regenerate`'s "nor a profile family";
     - §5 records that `feedback` holds `profile_rules` and depends on `contracts` and `pyfmt`;
     - "What this does not cover" gains a bullet for what the profile cannot reach on real data.
   - **`docs/ARCHITECTURE.md`**:
     - §1 has `profile` with its flags and P10 findings 2–4's refusals: `--verbose` refused,
       `--club` narrowing the text and not the JSON, and an unregistered id exiting 0 with "nothing
       to grade". It also has the four M37 modules in the Rust paragraph, the profile family in the
       `cargo test` sentence, and `career-v3` in the re-record line;
     - §2 gains a paragraph on M37 adding modules and no crate, and the edge that placed each;
     - the interface table's `CareerCorpus` row now names `shot` and `shot_unscored`, and a new
       `GradedProfile` row is added;
     - §4 gains "since M37 each Rust corpus entry carries its shot".
   - **ADR addenda**, three of them:
     - ADR-034 (M37) covers the criteria table for decisions 1–8 and calls 4 and 7–9. It records
       that §5.2's strike location on HD Golf is superseded, and why (no magnitude, picture only,
       M42). It confirms the 9-yard landing, describes each landing's basis, and records that the
       sign does not flip. It also covers the `synthetic` device, `rank_profile` written to §5.7,
       and what `data/` shows;
     - ADR-024 (M37) covers the corpus carrying the shot and `career-v3`;
     - ADR-032 is the seventeenth addendum. It covers the third kind of oracle as built, and the
       oracle pin with P11 finding 3's answer. It records the profile family as a third family on
       `CAREER_VERSION`, and Wilson's z and clamp and the two medians as choices rather than §3's
       edges. Its Status block's self-count now reads **seventeen**, with a clause for this
       addendum.
   - **`docs/README.md`** says 108 addenda, and its rows count 024 at **7**, 032 at **17** and 034
     at **3**, each with a clause. ADR-034's Status cell names the M37 addendum, and CONFORMANCE's
     row names the tenth family. The document count is untouched, since that is P13's.
   - **The program plan**: the M37 row is 🔄 "Built, closing", with a summary. §M37 opens with a
     correction note: strike location only from M42's offset, the sign not flipping, and four
     smaller as-built differences. There are inline marks at the centered-rate bullet, at "the sign
     flips", and at `get_strike_profile`, which M29 carries.
   - **`CLAUDE.md`**:
     - the intro names M37;
     - Commands has six career verbs, a `profile` line, the families `cargo test` covers, and the
       profile family in the re-record comment;
     - the storage paragraph gains `career-v3`, plus a short "M37 added no crate" paragraph;
     - the `CAREER_VERSION` invariant names three families;
     - the port-checked row covers `career-v3`, the profile family and the oracle pin;
     - the crate row gains a sentence each for `contracts`, `analysis`, `feedback`, `core` and
       `storage`.
   - **`crates/feedback/Cargo.toml`**'s `description` now names the list (P8 finding 7).
   - `devices.json`'s `_comment` states no count, so it did not move.
2. **Outside the Files line, all of them docs:**
   - `spec/README.md`. It describes the families, and it still printed the `career-v2` command,
     "the other seven", and "M37's pin extends `oracle` to them". It gains the profile family in
     the tree, the runners list, a paragraph, the ages list, the ledger section and the refusals.
   - Two doc comments in `crates/contracts/src/career.rs`. The module's "What ages on
     `CAREER_VERSION`" and the constant's doc named only the storage and career families, and now
     name the profile family as the third.
3. **Found while writing**:
   - CONFORMANCE §2's storage table still said version 2 for every sub-family, though `career-v3`
     moved all of them to 3. A census confirms every storage and career vector carries a `career-v3`
     entry: M35's twelve hand cases have it as their only entry, and M36's three have it as their
     second.
   - **`storage/corpus/real.json.gz`'s slimmed analyses keep no `unscored`**. They keep the three keys
     `read_corpus` read when the case was recorded. So the real case, and every copy adopted from
     it (`career/real/aaron`, `profile/corpus/real-aaron`), carries no `shot_unscored`. Rust's
     `read_corpus` over `data/` itself lifts it onto 11 of 13 entries (P4 finding 4).
     - It moves no grade today, because no shape topic exists on real data.
     - It is named in CONFORMANCE's storage section, not fixed.
     - The only vectors whose corpora carry `shot_unscored` are two profile hand cases,
       `draw-on-line-withheld` and `strike-location-radius-and-grid`.
   - In ADR-034's new criteria table, `|h|` inside a table cell needed `\|` escapes.
4. **For P13**:
   - The program plan's M37 row reads "🔄 Built, closing *(P0–P12 done; P13 verifies and
     commits)*". Change it to "✅ Done *(date)*, 14/14 phases" and keep the summary.
   - `docs/README.md`'s document count is still P13's, once this file is staged.
   - ADR-024, ADR-032 and ADR-034's map rows now count 7, 17 and 3. The total is 108.
   - The smoke run over `data/` should print what P10 finding 5 and P11 finding 4 record. The new
     docs (ADR-034's addendum, ARCHITECTURE §1, CONFORMANCE's profile section) quote it. If it
     prints anything else, those quotes move with it.
   - `ARCHITECTURE.md`'s banner date ("reviewed 2026-09-27") was not touched. M35 left it too.
5. **Verified**:
   - docs-truth: 92 passed, before and after the last edit;
   - the full pytest, run alone: 2055 passed, 2 warnings (`.pytest_cache` writes denied by
     Windows);
   - `cargo test`: 68 binaries, 1254 passed, 0 failed, 1 ignored;
   - clippy and `fmt --check`: clean;
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty;
   - files changed since the marker: the Files line's docs, this plan, `crates/feedback/Cargo.toml`,
     and finding 2's two. `CLAUDE.md` sits at the root, outside the `find`.

### P13 (2026-10-15)

1. **Every Verify command is green, and the exit holds item by item.**
   - `cargo test`: 68 binaries, 1254 passed, 0 failed, 1 ignored (the `clip::Cutter` doc-test).
   - `cargo clippy --all-targets`: no warnings. `cargo fmt --check`: clean.
   - The full pytest, run alone: 2055 passed, 2 warnings (the `.pytest_cache` writes Windows denies).
   - ruff: clean. mypy: clean over 118 source files.
   - `conformance.py check`: exits 0, deferring the storage (76), career (56), shot (25) and profile
     (15) families to `cargo test`.
   - The v17 dry run: 67 vectors run (21 engine, 21 stage, 25 shot), 0 changed, 0 files written.
   - The `career-v3` dry run: 147 vectors run (76 storage, 56 career, 15 profile), 0 changed, 0
     files written, as P11 finding 9 said it would.
   - The `data/` digest: `bd384cc4…` over 192 files, before the smoke run and after it.
   - `git diff -- src/ spec/vectors/{synthetic,corpus,stages,shot,screen,audio,format}`: empty.
   - docs-truth: green after the plan was staged and the document count moved.
2. **The smoke run printed what P10 finding 5 and P11 finding 4 record, unchanged**, so none of the
   docs that quote it (P12 finding 4) moves. `golf-core profile --name Aaron` over `data/`: 3w (6)
   and 7i (5) with consistency alone, withheld; the player at 13 shots (2 naming no club), its
   consistency graded 55% (6 of 11), 95% CI 28% .. 79%, the blend 55%, and the list face-to-path
   bias 3.80×, face-to-path scatter 3.07× and start-line scatter 2.01×. `--json` is one array entry
   with `display_name` and `profile`.
3. **The close's doc edits**:
   - `ROADMAP.md`: the M37 row and §M37's Status are ✅ with the date. The M42, M38 and M29 rows and
     sections no longer wait on M37 or run beside it, and M29's now names `get_strike_profile`.
     NEXT ACTION is "M29 or M41, by the user's order; photos for M42", and it does not choose
     between them: M29 needs nothing more, M41 and M42 each start with new photos.
   - The program plan's M37 row: "✅ Done *(2026-10-15)*, 14/14 phases", its summary kept.
   - `docs/README.md`: 83 documents, 70 in `docs/`, 9 in `plans/`, with this file staged.
   - `WORKLOG.md`: the entry, with what M42, M38 and M29 should know and what is left open.
   - The `app-transition-decisions` memory and its index line: M37 done, next by the user's order.
4. **The one commit** is onto `main`, staging the working tree's M37 changes and this file, and
   nothing under `data/`.
