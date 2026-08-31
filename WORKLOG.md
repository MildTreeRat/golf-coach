# Work Log

Write a short entry every time you sit down to work. Reverse chronological (newest first).
This is your "pick up where I left off" document.

---

## 2026-08-30 — Two clocks in one container, and the floor that could not land without them

**Duration**: ~1 session. `src/golf_coach/audio/ffmpeg.py` (new `video_start_seconds`),
`src/golf_coach/audio/impact.py` (`_MIN_RELATIVE_PROMINENCE`, a second floor in `detect_strikes`),
`src/golf_coach/contracts/audio.py` (`AudioClipMetadata.video_start_s`, `AudioStrike.frame`'s second
`None` case, `AUDIO_DETECTOR_VERSION` 1 -> 2), `src/golf_coach/api/pipeline.py` (`audio_for` probes
the video timebase, `_frames_derived` subtracts it, new `_frame_of`),
`src/golf_coach/contracts/swing.py` (`ANALYSIS_VERSION` 12 -> 13),
`src/golf_coach/analysis/alignment.py` (a docstring that recorded the defect as unfixed). Docs:
`docs/M11_ACOUSTIC_SYNC.md` (P10, P11, the status line, the addendum's handoff), ADR-025's **first
addendum**, `docs/README.md`, `docs/ARCHITECTURE.md`, `ROADMAP.md`. Tests: 11 new across
`tests/audio/test_ffmpeg_source.py`, `tests/audio/test_impact.py` and
`tests/api/test_pipeline_audio.py`. `pytest` green, `ruff` and `mypy` clean. Corpus re-analysed and
re-rendered 15/15 with `reanalyze.py --all --video`.

**This is M11's addendum, picked up the day it was written: the anchor was wrong two ways at once
and the two errors were partly cancelling.** Every clip's earliest candidate could be a quiet
transient 2-3 frames ahead of the ball, and `with_measured_impact` takes the earliest. On the four
down-the-line clips whose container carries a leading empty edit, the sample index was *also*
landing 6.3-7.5 frames late, because `FfmpegAudioSource` decodes on the presentation timeline and a
frame index counts from the decoder's first frame. On those four the errors ran opposite ways, so
the stored anchor came out ~2.5 frames late instead of 6 or 3 — which is why the corpus looked
merely imprecise rather than broken, and why P9 had to revert its floor after building it.

**The container reading in the addendum was wrong, and correcting it made the fix simpler.** It
read the offset as the empty edit *plus* the 85 ms `media_time` trim, 90.5 ms. Neither decoder drops
the trimmed head at all: with `-ignore_editlist 1`, ffmpeg's frames 0, 1, 2 are pixel-identical to
`cv2.VideoCapture`'s frames 0, 1, 2 of the same clip, so `media_time` never reaches a frame index.
What does is the empty edit, applied as a uniform shift — on `2026-08-23/9`, frame 0 presents at
0.105 s and the 449 steps after it are 445 of 16.67 ms, three of 18.33 ms, and one 66.67 ms jump at
the last frame. So the correction is the empty edit alone, it is per-clip (105, 107, 117, 125 ms on
bundles 9, 1, 7, 11), and **it is measured rather than derived**: `video_start_seconds` asks ffmpeg
for the first frame's presentation timestamp, because the question is what a decoder *did* with the
edit list and not what the atom says. This also closes §E4's loose end — those four clips decode
three fewer frames than their `stts` counts, and the three are at the tail.

**It went on the audio artifact, not on `ClipMetadata` where the addendum guessed.** The pose path
decodes with OpenCV, which reports 0.0 for `CAP_PROP_POS_MSEC` on the first frame of a clip that
presents 105 ms late, so there is no switch to flip there and no way to see the number without
dragging `imageio-ffmpeg` into the vision extras. The audio path already has ffmpeg open on the same
file, and the only question the offset answers is the one that artifact exists to answer.

**Ten clips have now been read frame by frame, four of them for the first time, and the anchor lands
on contact in all ten.** `2026-08-23/1` and `/11` were the two edit-list bundles nobody had checked:
contact at down-the-line 372 and 339 and face-on 442 and 225, against anchors of exactly those.
`2026-08-23/9`'s render — the one whose panels struck four output frames apart — now strikes on
output frame 142 in both panels, and `/10` does the same at 108.

**The re-run returned two `tempo` readings from `unscored`, and that is P9's finding with the sign
flipped.** Eight of fifteen bundles moved an anchor and none moved a window. On `2026-08-23/1` and
`/11` the cross-view contradiction that withdrew `tempo` on 2026-08-29 turns out to have been the
mis-registered anchor rather than the swing: with the down-the-line impact corrected the two
downswings agree (0.384 s against 0.450 s, and 0.334 against 0.350), so the reading is scored again
— and both **fail**, at 2.61 and 2.50:1 against a 2.72 floor. Their mechanics scores fall, 99.68 ->
98.80 and 100.00 -> 98.16, because a failing score *came back*, not because a swing got worse. The
other three withdrawals narrowed and held. `tour_trajectory_t2_dtl` and `tour_trajectory_q_dtl`
moved on the six bundles whose down-the-line impact moved, and nothing else did.

**Two sessions were editing this repo at once, and this is the seam.** The pose lite -> heavy switch
below landed while the corpus was re-running. No files collide — that work is in `pose/`, `config.py`
and `contracts/keypoints.py`, this one in `audio/` and the audio half of `pipeline.py` — but
`ANALYSIS_VERSION` moved twice on one day, 12 -> 13 here and 13 -> 14 there, and **the corpus needs
re-analysing again under the heavy model**. The keypoint caches were untouched during this re-run
(their mtimes are still 2026-08-23), so every number recorded here is `mediapipe:lite` and isolates
P10 and P11 cleanly; that is stated where the tables are.

---

## 2026-08-30 — The render was replaying one panel at 2.08x, and a duration bound was picking swings

**Duration**: ~1 session. `src/golf_coach/analysis/alignment.py` (`warp_speeds`, the `pair_frames`
rate guard, `_MAX_WARP_SPEED_ERROR`, the plausibility veto removed from `_shared_tops`),
`src/golf_coach/analysis/phases.py` (`_PLAUSIBLE_DOWNSWING_S` -> `_POSSIBLE_DOWNSWING_S`, 0.15-0.45
-> 0.12-0.80), `src/golf_coach/contracts/alignment.py` and `src/golf_coach/analysis/engine.py`
(docstrings that described the removed veto), `src/golf_coach/api/pipeline.py` (logs the warp speed
on every render). Tests: `tests/analysis/test_alignment.py` (3 rewritten, 5 new),
`tests/analysis/test_select_swing.py` (2 new, fixture guard widened),
`tests/api/test_pipeline_auto_window.py` and `tests/analysis/test_engine_bundle.py` (fixtures
resized). `pytest` 1205 passed, `ruff` and `mypy` clean. Corpus re-rendered with
`reanalyze.py --all --video`.

**The complaint was "the down-the-line video is sped up after the top of the backswing", on
`2026-08-23/9`. It was, by 2.08x, and `aligned.mp4` is a resample rather than a playback.**
`pair_frames` drives the timeline off the face-on clip at its native rate and samples the other
clip at whatever frame shows the same tau, so two views that disagree about where the top is can
only express that disagreement as *speed*. Face-on measured 13 frames of downswing where
down-the-line measured 27 of the same swing; forcing both to reach tau=2 together means the
down-the-line panel covers 27 of its frames in 13 output frames. Output frame 65 is tau=1.000 and
the panel steps +1 through the backswing and +2 from there to the end of the clip.

**`_shared_tops` had already detected it and then declined to act, and declining is not neutral.**
Its whole job is to hold both panels to one duration so neither is resampled. It refused because
the duration it would impose (0.4503 s) missed `_PLAUSIBLE_DOWNSWING_S`'s 0.45 ceiling **by 0.3
ms**, and its `return None, None` re-enabled the very warp it had just proved wrong. Seven of
fifteen bundles rendered a panel off-speed; four of those (1, 4, 5, 9 at 1.48x, 3.11x, 2.70x,
2.08x) carried the note "leaving the warp in place", and three more (3, 8, 2026-08-10/2) said
nothing at all because a 0.30 relative gap is *inside* `_DOWNSWING_AGREEMENT` and still means a
30% rate error. `_DOWNSWING_AGREEMENT`'s own comment predicted exactly this: "a viewer reads that
as one camera running fast, which is worse than a visible seam".

**Two layers now, and they are not the same rule.** `_shared_tops` *repairs* — it can move a top,
so it only fires where a shared clock says which top to move, and it keeps `_DOWNSWING_AGREEMENT`
because the tier it sets (`IMPACT_ONLY`) is a claim about evidence. Lowering that trigger to the
render tolerance was tried and reverted: it re-labelled honest bundles (session 8 reports `full` on
disk). `pair_frames` *refuses* — it cannot repair anything, so it fires wherever the resulting
playback speed would leave `_MAX_WARP_SPEED_ERROR` and maps the follower rigidly from tau=2 at its
native rate. The guard is load-bearing rather than decorative: on four bundles the repair declines
and the guard is what carries the render, including the pre-top region `_shared_tops` never
touches. All fifteen now render at 1.00x.

**The plausibility veto is gone and the band it read is now an observation.** A reference nobody
can swing is still the better of the two things to hold both panels to: imposing it costs the top
banner a few frames, and not imposing it costs the viewer the tempo the side-by-side exists to
show. `_POSSIBLE_DOWNSWING_S` is still read at that site and decides nothing — it writes a caution
into the note instead. What the veto was protecting was never carried by the warp anyway:
`ClipAlignment.top_late_by` carries it, `_arbitrate_tops` sets it on its own evidence, and
`analysis.engine` reads *that*. **No score moved.**

**The second half started from "we should not decide what is a practice swing based on length".
Correct, and the code already agreed — rule 0 is the ball strike, and a practice swing makes no
sound.** What the band was actually doing was worse than mis-naming: its 0.45 ceiling rejected
**nine ball-struck descents** on the corpus (0.467 x2, 0.484 x3, 0.500, 0.501, 0.517, 0.567), every
one a real swing, on nothing but length. Measured over all 62 candidate descents in the 26 distinct
stored clips, the real ones run 0.167-0.567 s continuously, then there is a **0.534 s gap** — the
largest below 4 s anywhere in the set — and everything above 1.101 s is not a swing. 0.80 sits
inside that gap; the floor moved to 0.12 to clear two 5-frame tracking fragments.

**Swept through `api.pipeline._auto_windows` over all fifteen bundles, the widening moves no window
at all** — the strike rule and the cross-view reference were already making every choice. Removing
the bound *entirely* moves exactly one: `2026-08-23/5` face-on, where a 2.351 s descent that ends
near a transient becomes "the last" and windows the whole clip. That single case is why the bound
survives as a sanity check, and it is why `_POSSIBLE_` and not `_PLAUSIBLE_` — a threshold named
for likelihood invites being used to rank candidates.

**A correction to something said mid-session**: the claim that the band was choosing between real
shots on `2026-08-07-aaron1/1`'s down-the-line clip was wrong, inferred from a candidate listing
rather than run. The cross-view reference already picks that swing; the band was redundant there.
The structural objection stands and is now fixed, but the corpus never exercised it.

**Carried forward, and it is the next thing worth doing.** `2026-08-23/11` was checked end to end
after the re-render and its panels still do not strike the ball together, for reasons this session
did not touch. Its down-the-line container was parsed directly and carries the video edit list
§E2 predicted — `timescale 19200`, an empty edit of 125 ticks then `media_time 1632`, so 91.5 ms
(~5.5 frames) that the audio decode applies and the video decode ignores. On top of that, *both*
views anchored tau=2 on a precursor rather than the ball: face-on took frame 222 (prominence 1.3 M)
over 225 (13.2 M), down-the-line took 343 (1.4 M) over 346 (14.2 M). Net effect is roughly seven
frames of daylight at impact with the down-the-line panel leading, and a TOP banner ~15 frames
early in face-on because the imposed 0.417 s reference is inflated by that late impact anchor. The
`tempo` withdrawal on that swing is probably the right call for the wrong reason.
docs/M11_ACOUSTIC_SYNC.md §Addendum has the design: **correct the video edit list first, then floor
the candidates** — the two fixes cancel each other if either ships alone.

---

## 2026-08-30 — Fifteen swings nobody could reach, and the sidecar that hid their video

**Duration**: ~1 session. `src/golf_coach/api/app.py` (`GET /api/sessions`, `_swing_row` lifted out
of `session_detail`), `src/golf_coach/api/static/library.html` (new), links into it from the other
three pages, `src/golf_coach/api/pipeline.py` (`_recorded_video`, `PipelineOutcome.render_attempted`).
Tests: `tests/api/test_library_route.py` (new, 8), four more in `tests/api/test_state.py`. Docs:
`docs/ARCHITECTURE.md` §1 route table and its static-page list, `ROADMAP.md` M5. `pytest` 1199
passed, `ruff` and `mypy` clean. Corpus re-rendered with `reanalyze.py --all --video`.

**The complaint was "I can only find a swing by typing its URL, and then I can't see the video."
Both halves were true and they had different causes.** The first is a gap: nothing in the API
enumerated. `/api/sessions/current` answers *today* and `/api/sessions/{id}` answers a session you
already know the id of, so `index.html` — which only ever renders the current session — was the
whole navigation surface. Fifteen analyzed swings across four sessions were on disk and reachable
only by hand-writing `results.html?session=…&swing=…`.

**The second is a bug, and yesterday's own corpus re-analysis caused it.** `record_state` wrote
`video=outcome.video_path.name if outcome.video_path else None` — unconditionally. `reanalyze.py`
keeps the render off by default and its docstring promises the opposite ("anything analyzed without
a render keeps whatever `aligned.mp4` it already had"), but the sidecar is what `has_video` is read
from. So the `--no-video` sweep at 18:33Z left thirteen of fifteen swings advertising no video with
a perfectly good H.264 file sitting in each directory, and `results.html` took its fallback branch
and served the raw upload instead — HEVC in a QuickTime container, which is why the video "didn't
work" rather than merely being absent. The two swings that *did* still play were 9 and 10, the two
re-rendered by hand for M11's eye check.

**`video_path is None` cannot say why there is no video, so `render_attempted` now does.** A run
that skipped the render keeps the previous sidecar's `video` — after stat'ing the file, because the
sidecar outlives anything deleted by hand. A run that *tried* and produced nothing clears it: the
views would not align, and re-advertising the old file would claim a video this analysis says
cannot be made. Staleness in the other direction — anchors moved under a file this run did not
re-render — stays `reanalyze._video_went_stale`'s job, which reports rather than deletes.

**The library page does not repeat `results.html`'s fallback, deliberately.** Falling back to the
raw clip is right on a face-on-only bundle opened on an iPhone and wrong in a list on a laptop: a
play button over an undecodable HEVC file is a button that does nothing. When the sidecar names no
render the row says "no video" and the golfer keeps their time.

**`GET /api/sessions` sends whole rows rather than ids.** Four sessions is four extra round trips
to draw one screen on a phone, and the row is `_swing_row` — the same projection `session_detail`
sends, lifted out rather than copied, with `test_the_list_and_the_detail_describe_a_swing_identically`
pinning it. Sessions holding only a `session.json` are skipped: four of those exist, each one a
session someone opened at the bay and filmed nothing in.

**Carried forward**: `results.html`'s own HEVC fallback is untouched and still shows an
undecodable clip on a desktop browser for a face-on-only bundle. The honest fix is to render a
single-panel `aligned.mp4` when there is no second angle, which is a pipeline change, not a page
change.

---

## 2026-08-30 — M11's eye check: the video decode ignores an edit list the audio decode applies

**Duration**: ~1 session. `src/golf_coach/audio/impact.py` (a docstring that says why the obvious
fix is not there), `src/golf_coach/contracts/audio.py` (`AUDIO_DETECTOR_VERSION`,
`AudioFile.detector_version`), `src/golf_coach/api/pipeline.py` (`audio_for` re-detects behind that
version, and keeps the older list with a note on an install that cannot),
`src/golf_coach/analysis/alignment.py` (one docstring). Tests: three new in
`tests/api/test_pipeline_audio.py`, one in `tests/contracts/test_audio.py`, one fixture stamped in
`tests/api/test_pipeline_auto_window.py`. Docs: `docs/M11_ACOUSTIC_SYNC.md` §Addendum, `ROADMAP.md`,
`docs/ARCHITECTURE.md` §4. Corpus re-analysed twice and left exactly where it started; `pytest`
1187 passed, `ruff` and `mypy` clean.

**The two carried-forward items were supposed to be independent and they were not.** One was P9's
precursor floor; the other was §Verification's "watch two renders by eye". Doing the second
disproved the first, which is precisely why that line was in §Verification and not in a phase.

**The floor was built exactly as P9 specified and it works.** A candidate must clear a fraction of
its clip's loudest transient; measured over all 30 cached clips it wants 0.25 (precursors 0.02-0.10,
the ball never under 0.61). Every clip went from 7-42 candidates to 2-4, eleven anchors on seven
bundles moved onto the loud onset, `ANALYSIS_VERSION` went 12 -> 13. Then I watched the renders and
`2026-08-23/9`'s panels were four output frames apart.

**Contact, read off the video frame by frame, on six clips.** Face-on: `9` 550-551 against an audio
ball at 550.3, `10` 773-774 against 773.4, `7` 1107-1108 against 1106.5 — in sync. Down-the-line:
`10` 1242-1243 against 1242.9 — in sync; but `9` **321-322 against 327.7** and `7` **669-670 against
676.3**. Two clips out by ~110 ms, same direction.

**`offset_between` said which half was lying, without naming a transient.** On `10` the audio offset
between the two clips (-7.8350 s) matches the offset implied by the two verified contacts
(-7.8257 s) to 0.6 frames. On `9` the audio says +3.7100 s and the video says +3.8169 s — 107 ms of
disagreement inside one bundle.

**§E2 had already written down the cause, a milestone before anyone needed it.** Bundles 1, 7, 9 and
11's down-the-line clips carry a video edit list at timescale 19200: an empty edit of 5.5 ms, then a
start 1632 ticks (85 ms) into the media. ffmpeg applies it to the audio. The video path reads frames
from media time zero and ignores it. 90.5 ms of register error, plus `detect_strikes`'s +18 ms
window convention, predicts 108.5 ms against the 103 and 113 measured. §E2's own conclusion — "the
decode path must let a real demuxer apply the edit list rather than reading samples from zero" — was
applied to audio and never to video.

**So P9 was right about which peak, and the floor still cannot ship yet.** On those four clips the
precursor sat ~3 frames early against a container offset ~6 frames late, and the two partly
cancelled. Remove the precursor and the anchor becomes correct in audio time and 6 frames late in
video time, which is the time tau=2 is measured in: `7` 672 -> 676 against contact at 669-670, `9`
324 -> 327 against 321-322. The panels get further apart, so it is reverted with the measurement
written into `detect_strikes` so nobody rebuilds it blind.

**§E4's headline is downgraded.** "The down-the-line pose impact is 5.7-7.5 frames early" was the
gap between a pose estimate and an anchor whose clip was 90 ms out of register. Against the video
the pose estimate is right to within a frame on both edit-list clips checked.

**`2026-08-23/11` is the sensitivity warning.** Under the floor its two anchors each moved 3 frames,
lengthening both downswings equally — and a *relative* gap shrinks when a constant is added to both
sides, so it crossed `_DOWNSWING_AGREEMENT` (0.320 -> 0.286 against 0.30) and its `tempo` came back
from withdrawn to scored-and-failing at 2.50:1. Three frames decides whether that swing is judged on
five fundamentals or six.

**One thing kept from the reverted work**: `AUDIO_DETECTOR_VERSION`. `{role}.audio.json` is keyed on
the *clip's* sha256, which cannot see a changed detector, so the floor would have shipped invisibly
onto every stored bundle. It is at 1, detection is unchanged by it, and the next real fix moves it.

**Left at**: M11 committed in one commit, corpus untouched by the session. The next change is the
edit list, not the detector: make the video decode honour it (or measure the offset once and carry
it on `ClipMetadata`), which moves frame indices on four clips and so is an `ANALYSIS_VERSION` bump
plus a pose-cache invalidation. *Then* land P9's floor, which is already measured. Before either,
read `2026-08-23/1` and `/11` against the video — they are the two edit-list clips nobody has
checked, and three bundles of ground truth is thin.


## 2026-08-29 — M11 P9: the corpus re-run, and the milestone closes

**Duration**: ~1 session, no source files touched. `reanalyze.py --all --video` over 15 bundles,
then `2026-08-23/1` again with `--coaching`. Docs only: `docs/M11_ACOUSTIC_SYNC.md` (P9 ticked, its
*As built*, header to tier REFERENCE and 10/10), `ROADMAP.md` (status row, §M11, §M10's handoff
paragraph, the NEXT ACTION), `docs/README.md`'s map row. `pytest` 1183 passed, `ruff` and `mypy`
clean over 101 files, before the run and after the doc edits.

**Every bundle on disk reads `synchronized`, 30/30 clips pinned to a strike they heard.** No
bundle sits at any other tier, which makes §Design's tier inversion the ordinary case rather than a
corner: the ladder that ranks inferred anchors is now the fallback, not the normal report. Four of
the fifteen were shot weeks before any of this was designed and synchronize anyway — the audio was
always there, which was ADR-025's whole claim.

**The late top is on seven bundles, not three, and that is the milestone working.** M10 handed over
2, 4 and 5; 1, 7, 9 and 11 joined the moment a measured impact was underneath, exactly as P6's
*As built* predicted — pinning the down-the-line impact later *widens* the downswing gap rather
than closing it. So seven `tempo` scores are withdrawn, and the two that matter most are 7 and 9,
which were **passes** at 4.27 and 4.06:1. §E3 wrote down in advance that a pass here was not a
result to preserve; they were passing on the same short face-on downswing that made the others
fail, and they are now withdrawn on a clock instead of surviving on a suspicion.

**Five scores moved and every one of them went up, and none of it is a swing improving.**
`2026-08-23/4` reads 100.00 where it read 88.50 because a wrong score left — ADR-010 §2, judged on
five fundamentals rather than six, with `unscored` saying so. If that column is ever read as
progress the milestone will have made things worse rather than better; the artifact says which it
is and the coaching prose repeats it.

**The clearest evidence that P8's derivation works is a sentence nobody wrote.** The regenerated
paragraph on `2026-08-23/1` ends: *"tempo couldn't be scored here because the two cameras disagree
about when your backswing ended … so this score comes off five fundamentals, not six, and there's
nothing to re-film."* `CROSS_VIEW_CONTRADICTED` carries `refilming_helps=False` and
`contracts/caveats.py` builds the rest. The same holds through MCP: `get_swing("2026-08-23", "4")`
reports `synchronized`, five checkpoints and `tempo` unscored — §Verification's line, satisfied.

**One residual, and it is not the one P6 deferred.** `with_measured_impact` takes the earliest
transient in its window; on four bundles a **low-confidence precursor sits 2–3 frames ahead of the
ball** and wins. Bundle 7 chose an onset at confidence 0.34 over the ball at 0.90 three frames
later. Cross-view error is ≈1.5 frames on three of the four (the same rule fires in both views and
mostly cancels) and 3.9 on `2026-08-23/7`. The strong onsets are the ball on independent evidence —
each pairs with a second loud transient at §E5's measured ball→screen gap. Do **not** "fix" this by
taking the most confident candidate: §E5 measured the screen strike louder than the ball on every
clip, so that rule is worse. Prominence separates the precursor by an order of magnitude
(19.2M against 2.5M on `2026-08-23/9`), so the fix is a candidate floor in `audio/impact.py` where
prominence already lives — not in `analysis/`, and not the ≈22 ms container bias, which is still
deferred and is worth 1.3 frames against this.

**Two things that did not move, and both are load-bearing.** `career_corpus.py` still reports
`tempo_ratio` at `n = 13`: seven withdrawn **scores** cost zero **measurements**, which is
`measure.py` and `mechanics.py` being separate subsystems rather than a lucky accident.
And `2026-08-23/2`'s down-the-line backswing is still 1 frame — session 5's was repaired by the
corrected impact but 2's has its *motion start* wrong, not its top, so nothing here reaches it.

**Left at**: M11 is closed, 10/10. Two things carried forward, neither blocking. The residual
precursor above is the next piece of desk work and wants a candidate floor plus a test built from
`2026-08-23/7`. And §Verification's *"watch two renders by eye"* (`2026-08-23/9` and `/10`) has not
been done — both were re-rendered on 2026-08-29 and what wants confirming is the new half of the
claim: that the two panels strike the ball on the same output frame. `scripts/align_swings.py`
still builds its own anchors and so can report neither `synchronized` nor an arbitration; the
pipeline path does both, and P9 went through `analyze_swing_dir`.


## 2026-08-29 — M11 P8: ADR-025, and the tempo that was never coaching truth

**Duration**: ~1 session, desk work. New `docs/decisions/025-acoustic-synchronization.md`; addendum
#2 on ADR-015. `src/golf_coach/contracts/unscored.py` gains `CROSS_VIEW_CONTRADICTED`,
`contracts/alignment.py` gains `ClipAlignment.top_late_by` and a `top_is_late` property,
`contracts/checkpoints.py` gains `CONTRADICTED_BY_A_LATE_TOP`, `contracts/swing.py` goes
`ANALYSIS_VERSION` 11 -> 12, `contracts/caveats.py` widens one derived clause;
`analysis/alignment.py` decides the arbitration once and threads it; `analysis/engine.py` gains
`_without_contradicted_scores`. Docs: `ARCHITECTURE.md` §1-§3, `README.md`, `ROADMAP.md`,
`M11_ACOUSTIC_SYNC.md`. New sections in `tests/analysis/test_alignment.py` and
`tests/analysis/test_engine_bundle.py`, two pinned sets updated in `tests/contracts/test_unscored.py`.
`pytest` 1183 passed, `ruff` and `mypy` clean over 101 files.

**The phase list called P8 "the paperwork" and P7's handoff attached a code decision to it.** The
paperwork was real — ADR-025 taking ADR-015's parked Option C, the addendum recording that "there
is no shared clock" is true of *timestamps* and false of *events*, the version bump, the doc map.
But the thing worth a session was the question P7 left open: three stored `tempo` readings score
and **fail** at 4.92, 6.08 and 6.09:1 on a denominator the other view contradicts, and M10 P10's
*As built* already says in as many words that they are not coaching truth. They are now withdrawn.

**The field is `top_late_by`, and the reason it is not `warp_top` is the whole of why this took
code rather than prose.** P7 corrected one bundle and diagnosed seven, because
`_PLAUSIBLE_DOWNSWING_S` refuses to impose a reference past 0.45 s and four of the seven run to
0.48-0.58 s. Keying the withdrawal off the *correction* would therefore have left exactly the
bundles this milestone exists for looking sound — `2026-08-23/4`, the swing §Verification names, is
one of them. So the contract carries the **diagnosis** (how many frames late, on the clip that
carries it) and the warp keeps carrying the correction, and `align_swings` computes the arbitration
once and threads it into `_shared_tops` so the two cannot disagree about one pair.

**Withdrawn, not restated — and `_tempo_restated` already knows the answer, which is what makes it a
decision.** On bundle 2 the corrected top reads 2.35:1, a pass. Writing that into a
`CheckpointScore` would ship a number whose value came from the alignment and whose band came from
the engine, measured over frames `segment_phases` never agreed to; and on four bundles there is no
restatement to write at all. ADR-010 §2. The repair belongs in `phases.py` where the boundary is
found, and what P8 hands that eventual fix is the thing tuning a detector against its own symptom
could never supply: a per-bundle label saying which view was wrong and by how many frames.

**`CONTRADICTED_BY_A_LATE_TOP` holds `tempo` alone, and `hip_shift_at_top` is deliberately out.**
It reads the top too, so the name argues for it. Nothing has measured what a ten-frame shift does
to a hip position sampled there, and the name is not evidence — a second member has to be earned
the way the first was. A checkpoint left out still ships beside an alignment that says the top was
late, so the finding is disclosed either way; the set decides only which scores are *withdrawn*.

**Two things the derivation did for free, and one it did not.** Adding the reason with
`refilming_helps=False` updated the coaching prose and the MCP guidance without either being
edited — `contracts/caveats.py` builds that bullet from `UNSCORED_REASONS`. The three
`test_docs_truth.py` failures P8 predicted are exactly the three that fired. What was *not* free is
the two hand-listed sets in `tests/contracts/test_unscored.py`, which is correct: those pin the
`refilming_helps` split by name precisely so a new member cannot join it silently.

**Left at**: P9, the corpus re-run, which is now the only thing between this milestone and its
*after* column. `scripts/reanalyze.py --all --dry-run` then `--video`, then `2026-08-23/1` again
with `--coaching` (M10 P10's trap: `build_feedback` rebuilds the rules half every run and leaves
`coaching_text` to the flag). Expect scores to move on more bundles than the tempo ones — better
windows and a corrected impact re-cut what gets scored. Two carry-forwards:
`scripts/align_swings.py` still builds its own anchors and can report neither `synchronized` nor an
arbitration, which is now the only route into the alignment that cannot see the clock; and
§Verification's `get_swing("2026-08-23", "4")` line is satisfiable but not satisfied — the stored
artifact says 6.08:1 until `reanalyze.py` runs.


## 2026-08-29 — M11 P7: which top is the late one

**Duration**: ~1 session, desk work. One source file: `src/golf_coach/analysis/alignment.py` gains
`_Arbitration`, `_arbitrate_tops`, `_top_at` and `_tempo_restated`; `_shared_tops` picks its
reference through the arbitration; `_which_half_is_wrong` gains a branch above the "different
swings" fallback. New section in `tests/analysis/test_alignment.py`. `pytest` 1172 passed, `ruff`
and `mypy` clean over 101 files.

**The shorter downswing is the late top, and the asymmetry is mechanical.** `_top_and_impact` puts
the top at the start of the major rising run and the failure `_DRAWDOWN_FLOOR` documents is that
run fragmenting — a hover at the top splits the descent and the later half is taken, which
*shortens* the downswing. Nothing in the rule can move a top earlier: `_MAJOR_RISE_FRACTION` needs
80% of the largest rise in the clip before a run is a candidate. Replayed over all eleven
2026-08-23 bundles with tau=2 measured in both views, **face-on is the late view on every one of
the seven that disagree**, by 10–17 frames. Not one exception.

**What changed is which view the reference comes from, and the old one was the broken half.**
`_shared_tops` held both panels to the *face-on* duration on ADR-015's grounds — the tuned and
scored view. Face-on is also the late view on all seven, so the pre-P7 rule was taking the wrong
duration and imposing it on the good panel. With both impacts pinned to a heard strike the two
downswings measure one interval in real time, so the reference becomes the *sound* view's: the
wrong top moves, the sound top stays where it was detected. Without a shared clock nothing changed.

**`_PLAUSIBLE_DOWNSWING_S` now decides how much of this actually lands, and it is a face-on band.**
On the arbitrated route the reference is a *down-the-line* duration read off the trail wrist, and
down-the-line reads systematically longer — 0.367–0.484 s across the corpus, with 0.484 s occurring
on bundle 6, where the two views agree. So P7 **diagnoses seven bundles and corrects one**: bundle 2
(reference 0.384 s, top 10 frames earlier, tempo restated 4.92 → 2.35:1), while 5 misses by 0.0003 s,
4 sits at 0.484 s, and §E4's 1/7/9/11 run to 0.48–0.58 s once P6 pins their down-the-line impact
later. The guard is kept: it is `phases`' constant, shared with `select_swing`, and on §E4's four
the refusal is *correct* — 0.5 s is no downswing, which is a residual for P9 to read. **Open
question for P8: whether the arbitrated route needs its own ceiling sized on down-the-line
durations.** Bundles 4 and 5 are what it would buy.

**The three failing tempo readings are named, not retired.** `analyze_swing` scores tempo off the
face-on phases long before `align_swings` runs, so `2026-08-23/4` still ships 6.08:1 as a fail with
a note saying its denominator is 17 frames short. Retiring the score needs a new `UnscoredReason`
and an engine that re-opens a scored result on a cross-view finding — a `contracts/` change, so it
belongs with P8's `ANALYSIS_VERSION` bump. **§Verification's `get_swing("2026-08-23", "4")` line is
not yet satisfied**, and that is the decision it waits on.

**Two things checked and deliberately left.** `analysis/phases.py` was in the phase's file list and
needed nothing — the evidence to decide lives at the cross-view seam, not inside one clip's
segmentation. And `_shared_motion_starts` derives from the same face-on duration and looks like it
wants the same flip, but the `IMPACT_ONLY` branch overwrites it whenever `_shared_tops` returns
tops; the only time its reference matters is when `_shared_tops` refused, which on the arbitrated
route is exactly when that duration was just judged implausible.

**Left at**: P8 — ADR-025, `ANALYSIS_VERSION` 11 → 12, and the doc map. Take the tempo decision
above with it. `scripts/align_swings.py` still builds its own anchors and passes them straight to
`align_swings`, so it can report neither `synchronized` nor an arbitration — the audio-free gap P5
recorded at `align_swings.py:300`, now widened a second time.


## 2026-08-29 — M11 P6: the ball strike is the clock

**Duration**: ~1 session, desk work. `src/golf_coach/contracts/alignment.py` gains
`AlignmentQuality.SYNCHRONIZED`, an `is_degraded` property and `SwingAnchors.impact_measured`;
`src/golf_coach/analysis/alignment.py` gains `with_measured_impact` and `_synchronized`;
`src/golf_coach/analysis/engine.py` gains `_anchored_on_strike` and two `*_strikes` arguments on
`analyze_swing_bundle`; `src/golf_coach/api/pipeline.py` hoists the `Audio:` stage out of
`auto_window`. New sections in `tests/analysis/test_alignment.py`,
`tests/analysis/test_engine_bundle.py` and `tests/api/test_pipeline_auto_window.py`. `pytest`
1166 passed, `ruff` and `mypy` clean over 101 files.

**The earliest transient wins, and every other rule fails on this corpus.** "Take the loudest" takes
the impact screen on *every* clip §E5 measured; "take the nearest" takes it on any clip whose pose
impact was already right, which is seven of eleven. And no window separates them — the correction
being made runs to 7.5 frames while the gap to reject starts at 5, so the admitting band is wider
than the rejecting one. Ordering is what is left, and it is physics rather than statistics: the ball
is the first sound a shot makes. The club-and-mat pair 15–20 ms ahead of it is under
`audio/impact.py`'s 50 ms separation floor and arrives already merged into one onset.

**The window is P5's `_STRIKE_TOLERANCE_S`, imported rather than re-derived.** It was sized on
exactly the three measurements this needed, and it is the same quantity read for a second purpose:
how far a pose impact may sit from the transient that made it. Two guards ride alongside it — a
candidate must land after the top and inside the clip — and the first is live, not defensive: M10's
offenders measure 0.183 s of face-on downswing against a 0.20 s window, so the window opens before
the top on its own.

**`SYNCHRONIZED` overwrites the anchor count; the notes are what make that safe.** `_synchronized`
runs last and replaces whatever the ladder came to, which is the inversion §Design named — a bundle
whose tops were refused now reads `synchronized` where it read `impact_only`, and the tier that
means "one anchor, and it was a guess" no longer sits on the best number in the system. Nothing is
lost, because every anchor `align_swings` refuses already appends its own note. Half a pair earns a
note and no tier.

**The warp underneath is untouched, `_shared_tops` included.** Only the label moved. Measuring
impact does not say which view's *top* is wrong — it makes the question answerable, which is P7 —
and pinning the down-the-line impact 5–7 frames later actually *widens* the downswing disagreement
on §E4's four bundles. That is the point: the gap was always there, split between two wrong anchors.

**One consumer change, and the phase guessed the wrong one.** The results page and
`pose/side_by_side.py` read through `quality_summary` and needed nothing, as predicted. But three
call sites gate a caveat on `is not FULL` and they do not want the same answer: `engine.py:448` was
calling the new tier *"alignment degraded"*, so it now reads `quality.is_degraded`, while
`feedback/coach.py` and `mcp/query.py` keep `is not FULL` **deliberately** — `ALIGNMENT_CAVEAT`
warns that correspondence is interpolated between anchors, which is still true of a synchronized
pair and produces the right sentence verbatim.

**§E4's ≈22 ms container bias is not applied, and that is deferred rather than dropped.** It needs
the *video* track's edit-list offset, and nothing in the package can see a container's edit list
today — `audio/ffmpeg.py` lets ffmpeg apply the audio one and never reports it, and P0 read the
video one with a throwaway MOV atom parser. Exposing it is a change to the decode port, not to three
`analysis/` modules. It is worth 1.3 frames against the 5.7–7.5 this phase corrects. **P9's corpus
re-run is where a residual that size shows up**; if it does, the fix belongs in `FfmpegAudioSource`
beside the stream index it already records.

**Left at**: P7, arbitrating the late top. `_backswing_disagreement_note` and `_which_half_is_wrong`
are untouched and still only *name* the suspect boundary; `_DRAWDOWN_FLOOR`'s comment
(`phases.py:103-132`) already holds the ground truth for which way to decide. One thing P6 leaves
by name: `scripts/align_swings.py` builds its own anchors and passes them straight to
`align_swings`, so it can never report `synchronized` — the same audio-free gap P5 recorded at
`align_swings.py:300`, now widened from selection to the tier.


## 2026-08-29 — M11 P5: a rehearsal makes no crack

**Duration**: ~1 session, desk work. `src/golf_coach/analysis/phases.py` gains `_struck`,
`_STRIKE_TOLERANCE_S` and a `strike_frames` argument on both selection rules;
`src/golf_coach/api/pipeline.py` gains an `Audio:` stage, `PipelineOptions.force_audio` and a
`strikes` argument threaded to `_pick_swing`. New sections in `tests/analysis/test_select_swing.py`
and `tests/api/test_pipeline_auto_window.py`. `pytest` 1147 passed, `ruff` and `mypy` clean over
101 files.

**Rule 0 filters the field; it never decides alone.** Candidates whose `impact` lands within
`_STRIKE_TOLERANCE_S` of a heard transient become the pool, and the duration band and "take the
last" then run *on that pool*. One struck descent reaches the existing `_lone_candidate_choice` and
the band is overruled; several, and the band chooses among them as before; none, and the whole
field goes through untouched. No fourth escape, which is what the phase asked for.

**`pool = struck or candidates`, and that `or` is the design.** A transient near no descent is
evidence about the bay, not evidence against every candidate at once — a dropped club must not cost
a window.

**0.20 s of tolerance, and it is loose on purpose.** §E4's worst pose-impact error is 0.125 s,
§E5's ball-to-screen gap is 0.145 s, and the detector's own onset convention is 18 ms late.
Selection never has to know *which* of the bay's four transients was the ball — any of them says a
ball was hit here — so the tolerance swallows that rather than resolving it. The decoys it must
reject sit 15-24 s away, so nothing sits in between.

**Silence is a note and never a decline.** An empty strike list is the rehearsal signal, and it is
also what a phone across the bay records; declining would score every motion in the clip as one
swing, which is worse than scoring the right descent with a caveat. The cross-modal check went in
one-directional: a shot attached with no crack in *either* view says the footage and the shot may
not be the same swing; strikes with no shot says nothing, because an unimported screen photo is the
ordinary state of most bundles.

**Left at**: P6, sync — `AlignmentQuality.SYNCHRONIZED`, `SwingAnchors.impact_measured`, and
`align_swings` pinning `tau = 2` to the measured impact in both views. Two things P5 left for it by
name: the audio stage runs only under `options.auto_window` and needs hoisting, since the alignment
wants strikes even when the windows were given by hand; and `scripts/align_swings.py` still carries
its own audio-free copy of the selection ordering (`align_swings.py:300`), so its windows can now
differ from the pipeline's on a clip a strike decides.

## 2026-08-29 — M11 P4: the audio artifact, cached the way pose is

**Duration**: ~1 session, desk work. `src/golf_coach/api/pipeline.py` gains `audio_for` and
`_frames_derived`; new `tests/api/test_pipeline_audio.py`; one more pin in
`tests/api/test_pipeline_imports.py`. `pytest` 1132 passed, `ruff` and `mypy` clean over 101 files.

**A direct sibling of `keypoints_for`**, and deliberately boring: same cache key (the clip's own
sha256, compared against the manifest's), same lazy import that degrades with a note instead of
raising, same `_note` rule that anything a reader of the *result* would need lands in
`analysis.json`. Writes `{role}.audio.json`.

**The one deviation from the phase's file list is `fps`, and it is a parameter.** The detector has
never seen the video, so a frame index is derived and not measured — and the caller doing the
deriving has to be the one already holding the frame rate, which is `_auto_windows` with
`KeypointsFile.clip.fps` in hand. Reading the fps off the clip a second time inside `audio_for`
would be a second answer to a question the pipeline has already answered. With `fps=None` every
strike keeps `frame=None`, which is exactly what P6 wants: cross-correlation works in samples.

**Frames are re-derived from the cache, never re-heard.** A call with no fps followed by one with an
fps costs a single decode — the sample index is the measurement and it has not moved, so the frames
are filled in from the stored artifact and written back with the fps they were derived under beside
them. It does not run in reverse: a caller without an fps does not strip frames a caller that had
one already wrote.

**Five ways it comes back with nothing, four of them notes.** Role never arrived (silent — no clip
is not an audio fault), clip missing from disk, `audio` extra absent, no audio stream in the
container, decode failed. `NoAudioTrackError` is caught apart from `OSError` because the two mean
different things to a golfer: one says this bundle cannot be anchored on the ball strike at all.
An empty `strikes` list is **not** one of the five — it is the result P5 reads.

**The import pin now names numpy as well as `imageio_ffmpeg`.** That half is the one that would
have slipped: ADR-008's stdlib-only rule is about `analysis/`, so numpy arriving in `api/pipeline`
at module scope would have broken a base install with nothing else complaining.

**Left at**: P5, selection — thread `strike_frames` through `select_swing` /
`select_matching_swing` and `_pick_swing`, and have `_narrate_choice` say when a strike decided it.
`audio_for` has no caller until then; `PipelineOptions` grows `force_audio` in that phase, where it
is a switch on a path something actually takes.

## 2026-08-29 — M11 P3: the bay's audio, asked two different questions

**Duration**: ~1 session, desk work. New `src/golf_coach/audio/impact.py` and
`tests/audio/test_impact.py`; `numpy` added to the `audio` extra. `pytest` 1116 passed, `ruff` and
`mypy` clean over 101 files.

**Two functions, and the split is the point.** `detect_strikes` finds the transients in one clip
and is the hard half; `offset_between` cross-correlates two clips' flux envelopes and identifies
no transient at all, which is why P6 can be built before P5 exists. Both were calibrated against
the 22 real clips of 2026-08-23 rather than against intuition, and the numbers are in the phase's
*As built* note.

**What the corpus says.** The ball and screen strikes stand at z = 113-599 above a clip's own flux
floor where 20 s of room tone never exceeds 4.4 — so a rehearsal returns an empty list, which is
the discriminator P5 wants. §E5's warning survived contact: the screen strike outranks the ball on
about half the clips, so `strikes[0]` is not impact, and a test now pins that.

**The finding P6 has to act on.** Whole-clip, the offset agrees with the detected strikes to
0-20 ms on ten of eleven bundles at r = 0.838-0.923. On bundle 8 it is confidently wrong: that
80-second down-the-line clip holds *two* shots, and the face-on clip matches the second at
r = 0.923 where the right answer scores 0.672. No margin rule catches that — both matches are
real — so `offset_between` takes a `max_lag_s` and **P6 must pass one** from the pose anchors it
already holds.

**Left at**: P4, `audio_for` in `api/pipeline.py` — the cached per-view read that turns a decode
plus a detection into `{role}.audio.json`, and the import pin that keeps `imageio_ffmpeg` and
numpy out of a bare `api.pipeline` import.

## 2026-08-26 — M10 closes: the corpus is on the fix, and the face-on top is what is left

**Duration**: ~1 session, desk work only. **No source changes** — P10 is a data run plus its
paperwork, and `ANALYSIS_VERSION` stays at 11 because P9 bumped it and this is the phase that makes
the artifacts agree with it.

**What ran.** `scripts/reanalyze.py --all --video` over all 15 stored bundles: 15/15, exit 0, ~26
minutes. A second `--dry-run` reports every stored result current, and `scripts/career_corpus.py`
counts 13 distinct swings over 21 metrics with nothing excluded as `OUTDATED` — before this, every
one of them was excluded. `2026-08-23/1` was then re-run with `--coaching`: it is the only bundle
carrying a written paragraph, and `build_feedback` rebuilds the rules half on every run while
leaving `coaching_text` to the flag, so without that second run it would have kept a paragraph
describing the pre-M10 window.

**What moved, over the eleven bundles from 2026-08-23.** Windows that pointed at no swing at all:
4 → 1. Face-on motion starts never detected: 3 → 0. Worst tau=0 disagreement: 0.300 s → 0.167 s,
and no `full` bundle now exceeds `_BACKSWING_AGREEMENT_S` where two did. The windows landed exactly
where the M10 doc's §A1 table predicted they would — session 4's down-the-line window went from
*none* to `(1130, 1391)` and session 10's from the whole clip to `(1099, 1315)`, frame for frame.

**The tier count got worse and that is the fix working.** Sessions 9 and 6 were the two bundles
§B2 caught claiming `full` while 0.233 s and 0.200 s apart; both now degrade — to `impact_only` and
`top_impact` — and say why in their notes. Session 10, the worst offender, came *up* to `full`.
Watched by eye at tau = +0.40: before, session 10's face-on club head had barely left the ball
while the down-the-line panel was a third into its takeaway; after, both panels have the hands at
hip height. Session 9 is the same story in the other direction and now says *aligned on impact
only* rather than claiming what it does not have.

**Four scores moved, eleven did not, and three of the four moved for a reason worth distrusting.**
`2026-08-23/8` rose 88.78 → 93.07 because it finally got a face-on window and stopped measuring
finish balance over its own walk-off. The other three fell — 2, 4 and 5 — because `tempo` left
`unscored` and immediately failed, at 4.92, 6.08 and 6.09:1. **Do not read those as coaching
truth.** On four bundles the two views now disagree about the downswing in the same direction every
time: face-on measures 0.183-0.267 s where down-the-line measures 0.384-0.484 s of the same swing.
The face-on top is landing late, which is the class `_DRAWDOWN_FLOOR` (`analysis/phases.py:132`)
was fitted against and evidently did not finish, and it is also why `2026-08-23/5` still cannot be
windowed down-the-line — a 0.183 s reference will not match the clip's only 0.450 s descent. That
is the defect M10 hands to whatever comes next; the notes in each artifact say "one view's top is
wrong" and stop there, which is the honest reading.

**One piece of noise that is not a change.** The renderer's first-choice H.264 encoder fails to
load on this machine (`openh264-1.8.0-win64.dll`, wrong version), so OpenCV prints a `VideoWriter`
failure per bundle before falling back. All 15 files are h264 and read back at their full frame
count.

**State of the tree.** P6-P10 are all still uncommitted, by request — `git status` shows the P6-P9
source and test work alongside this phase's documentation edits (`ROADMAP.md`, `docs/README.md`,
`docs/M10_ALIGNMENT_ACCURACY.md`, this file). Suite green: 1065 tests, ruff and mypy clean.

---

## 2026-08-22 — The tempo trainer grows a career scope, and the metronome moves out of the page

**Duration**: ~1 session. Two source modules, three static files (one new), one ADR addendum, one
ledger row. **No `ANALYSIS_VERSION` bump** — nothing new is measured and nothing new is written to
an artifact; `CareerTempo` is derived at read time from measurements that have been on disk since
ADR-023 shipped.

**What prompted it**: two questions. *"Does a long video cost more than a short one?"* and *"where
is the tempo feature — can I see my own tempo from the career page?"*

**The first answer is yes, and it is linear.** `api/pipeline.py::keypoints_for` streams the whole
clip through `estimate_pose` **before** `phases.select_swing` picks a window, so the swing selector
saves nothing on the only expensive stage — it decides which frames get *scored*, not which get
posed. `docs/M7_TWO_PHONE_SPIKE.md` has the measured rate: ~9.5 fps end-to-end at 4K60, so roughly
6.3 s of compute per second of clip, per view. Nothing was changed on the strength of it (the same
call that doc's Q2 already made), but it is now written where someone asking will find it:
trimming on the phone before upload is the only real lever.

**The second was the actual work.** The trainer existed only on `results.html`, gated on the tempo
checkpoint having *failed* on the swing being viewed — so a golfer whose tempo passed never saw a
metronome, and there was no surface anywhere answering "what is *my* tempo", which is the question
every simulator user has and the one with a different answer per person. `backswing_ms` and
`downswing_ms` had been measured per swing since ADR-023 and were read by nothing but the anchor of
the swing they came from.

**The shape, and the one decision that made it shippable today.** `CareerTempo` layers by what each
layer is allowed to assert: the per-swing readings are **measurements** and print at any `n`; the
typical values are `PersonalBaseline`'s guarded means and stay `None` until `CENTER` lifts; the plan
is a target. Without that split there was nothing to ship — `tempo_ratio` needs 8 samples for a
center claim and the corpus holds **2 distinct swings** (2.417:1 and 2.348:1; the other two
`analysis.json` files on disk are re-uploads of one clip and collapse). A view that could only show
a mean would have shown a blank page while the two numbers wanted were sitting in the artifacts.
Printing the readings is not the guard being bent — the guard governs what may be asserted *about
the golfer*, and one swing's durations assert one swing. `SessionSample` already draws that line.

**`TempoAnchor` has three values because a career view has three answers.** `TempoPlan.anchored` is
a bool and right for one swing. Career mode needs `CAREER_MEAN` → `LATEST_SWING` → `TOUR_MEDIAN`,
and the middle rung is the whole point: a golfer whose mean is withheld still has a measured
backswing, and one measured swing beats a population median for someone the population does not
describe. **The reported anchor is read back off the built plan, never decided beside it** —
`_anchor_backswing` can reject a backswing outside the tour p10–p90, and a view branching on its own
copy of that rule would print "matched to your own backswing" over a drill built on the median.
Aaron's live payload comes back `latest_swing`, pace 1.0006, anchored to 901.2 ms.

**`build_tempo_plan` split rather than being copied.** `build_tempo_plan_for(observed_backswing_ms=,
observed_downswing_ms=)` is the shared half; the phases version is now four lines on top of it.
Keyword-only on purpose — two floats in a fixed order is a numeric boolean trap, and
`build_tempo_plan_for(384, 901)` would have built a target with the halves swapped and looked fine.

**The metronome moved to `api/static/tempo.js`.** A second Web Audio scheduler is a second thing
that drifts, and one of the things that would have drifted is the pace slider's bounds, which
`tests/analysis/test_tempo_trainer.py` reads out of the file to pin against the anchor guard's own
range. The ledger's 2026-08-13 row declines a *node toolchain* for these pages, not a second file,
and `StaticFiles` already serves the directory whole — so a plain `<script src>` is inside that
decision rather than around it. **Framing prose stayed per-page**: the anchor sentence has two cases
on the results page and three on the career page, and one shared sentence would have been wrong on
one of them.

**Surprises worth carrying forward:**

- **The career page already had `tempo_ratio`, `backswing_ms` and `downswing_ms` cards** and had had
  them since career mode step 6. They read "no typical value yet" and always would at n=2, so the
  feature looked missing while being present — a refusal with no evidence beside it is
  indistinguishable from an absent feature. The new section is above the cards partly for that.
- **The `#tempo` fragment did nothing on arrival.** `career.html` renders from a fetch, so the
  browser resolved the fragment against an empty page. Scrolled explicitly after render.
- **The two withheld sentences differ only by a number** (8 vs 5), because `WithheldClaim` carries
  no metric name — it never needed one on a card already titled with the metric. Sorted
  strictest-first in `_center_refusals` so the one gating the headline reads first. If a third
  metric ever joins, that field is worth adding rather than sorting around.
- **Between the two floors** (n=5–7) the halves' means are sayable while the ratio's is not, so the
  typical tile prints "withheld" with the halves under it. Withholding a number the guard has
  already released would be its own kind of false.

**Verified**: full suite green, 996 → **1008** (+12: 9 in `test_tempo_trainer.py`, 3 in
`test_career_route.py`). The two `test_docs_truth.py` addendum counters went red on the new ADR
addendum before `docs/README.md`'s total and ADR-023's own row were corrected — which is the pin
doing its job, and worth recording as the second time this session's prose was caught by a test
rather than by a reader. `ruff check src
tests scripts` and `mypy src` clean. No browser was available, so both pages' trainers were exercised
through a DOM stub under `node` — mode toggle, pace pre-set, strip roles, facts and anchor prose all
read back correct on `results.html` as well as `career.html`, and the live route was hit against the
real corpus through `TestClient`.

**Left for next time**: the diagnosis sentence. The page now *shows* the two halves beside the ratio,
which is what makes "your backswing is fine, the downswing is slow" visible — ADR-023 records the
case exactly, 901/384 is a tour-median backswing with a downswing 28% past p90 — but saying it in the
product's own voice needs a rule about when it is safe to say, and that rule needs more than two
swings. Still deferred, and now deferred with the evidence rendered.

---

## 2026-08-22 — M9 P20: the docs catch up, and the pins that stop them falling behind again

**Duration**: ~1 session. Ten files, three new tests, no source behaviour changed. Suite 993 →
**996** (+3, all in `tests/test_docs_truth.py`). **No `ANALYSIS_VERSION` bump** — nothing measured,
nothing written to an artifact, same as P12–P19. **M9 is closed at 20/20.**

**What prompted it**: "can we work on p20 to wrap up the M9 milestone?" — with the verification
asked for first. P1–P19 check out: 993 passed, `ruff check src tests scripts` clean, `mypy src`
clean across 95 files, all seven M9 routes live in `api/app.py`, both club tools registered,
`SHOT_MEASUREMENTS` grown 2 → 7 and `TOOL_DESCRIPTIONS` 8 → 10.

**The phase said to run `tests/test_docs_truth.py` first and work only from its failures. It had
none, and that was the finding.** The suite was *fully green* while `ARCHITECTURE.md` §4 described a
repo that stopped existing at P4. Fifteen consecutive phases wrote a note in
`docs/M9_PLAYER_TRACKING.md` saying so — "still stale for P20, `tests/test_docs_truth.py` pins none
of it, so nothing goes red" — and every one of them was right. Running the doc-truth suite first is
only a *method* where the suite covers the surface being changed; where it does not, it is a green
light with nothing behind it. So P20 did the prose by hand **and** extended the cover, which is the
half that outlives the phase.

**Three pins, each watched fail before being trusted** (the P3–P19 habit):

1. **The MCP tool count**, derived from `TOOL_DESCRIPTIONS` and spelled with `caveats._count_word`.
   Reinstating "eight tools" in `scripts/ask_swing.py` fails it by name. Pinned on one phrase shape —
   *"the same N tools"* — and deliberately **not** on every `N tools` in the repo, because a subset
   count is a legitimate sentence: `mcp/club.py` really does hold two. `WORKLOG.md` is exempt, since
   a dated record of what a handshake advertised in August is not a claim about today.
2. **Every route in `api/app.py` appears in a route table.** There was no route table anywhere in
   the repo, which is exactly how seven endpoints landed in silence. Removing one row fails it with
   the path named. It parses the decorators out of the **source text** rather than importing
   `app.py`, because this suite runs on a base install with no `fastapi` — the constraint
   `tests/api/test_pipeline_imports.py` exists to hold.
3. **The phase doc's status line and the map's row agree on the count.** Both drifted independently
   this milestone: the map said "start at P8" while 19 phases were in. Setting the status line back
   to 19/20 fails it. Pinned to each other rather than to a literal, so the next milestone to close
   this way needs no edit here.

**What the prose actually had wrong**, beyond the tool count in six places: §4 called `session.json`
the "golfer cursor" when it has held two cursors since P4, and its manifest row named only
`player_id` when `club` has been stamped beside it since P5. §1 listed every CLI but
`club_profile.py`, and called `api/static/` "the two static pages" when there are three. §2's
diagram left the bag store off `storage/` and the club tools off `mcp/`. §3 described the pose
measurements and the tempo durations and said nothing at all about the launch-monitor half, which
M9 grew from two entries to seven.

**Two things were deliberately not written.** The seven `SHOT_MEASUREMENTS` names are *not* listed
in §3 — the registry is named and the rule is stated, which is CLAUDE.md's derive-don't-copy rule
and what §3 already does correctly for the placements. And `ROADMAP.md`'s two dated MCP-handshake
records **lost their digit rather than gaining a new one**: what was advertised on 2026-08-14 was
true on 2026-08-14, and this repo's own precedent for a count that moves is to delete the number and
keep the point (`test_volatile_counts_stay_out_of_prose`).

**Two status flips, each with a test watching the pair.** `docs/M9_PLAYER_TRACKING.md` went TARGET →
**REFERENCE**: nothing in it is a plan any more, but it is dense with per-phase snapshot numbers,
which is the map's own definition of that tier — and `test_the_map_agrees_with_each_doc_about_its_tier`
fails unless `docs/README.md` moves in the same commit. **ADR-024 flipped to Accepted with no
addendum**: none of its four decisions was corrected by building them, and inventing one would have
broken the map's addendum count for no reader's benefit.

**The NEXT ACTION moved off the board's last piece of desk work.** ROADMAP's banner now reads *one
bay session* — M7 Phase 0's field spike, M3's remaining OCR work and M2's lighting test all want the
screen in front of you, and career mode plus every per-club answer are built, correct and refusing at
`n = 2`. The banner carries the one preflight step M9 added: **set the club cursor before the first
swing**, because a session hit without club tags produces data that can never be split by club
afterwards.

**One self-inflicted scare worth recording.** Verifying the pins meant breaking each doc on purpose;
reverting with `git checkout -- <file>` reverts to **HEAD**, not to the working state, so it
discarded the session's own uncommitted edits to three files including `ARCHITECTURE.md`. Recovered
by re-running the edit scripts. The lesson is cheap and general: while a change is uncommitted,
`git checkout --` is a delete, not an undo — break a *copy*, or revert the breakage with the inverse
edit.

**Left deliberately unfixed, and neither was caused by M9**: root `README.md` says "ADRs 000–016"
and "16 of them" where 25 exist, and "Nine packages" where there are ten; `docs/FLOW.md`'s milestone
graph has no M9 node. All three are real, all three are out of this phase's scope, and none is
pinned — so they are recorded here rather than left to be rediscovered.

---

## 2026-08-22 — the per-swing club control: M9's blocker, which was never a phase

**Duration**: ~1 session. One static file, four prose blocks across three files, no new tests. Suite **993 → 993**, and
that number not moving is the point — nothing importable changed. **No `ANALYSIS_VERSION` bump.**

**What prompted it**: "can we work on the next step for M9?" The ROADMAP's NEXT ACTION, put there by
P19's own driving: `POST /api/sessions/{session}/swings/{swing}/club` had worked since P6 and had no
UI on any page, so every swing on disk — all of which predate the tag — was unreachable from a
phone, and every M9 surface correctly reported the empty state.

**What it does.** `index.html`'s swing list gained a club row beside the golfer one: which club the
swing was hit with, `no club` in the warning colour when it says nothing, and a *change* control
that opens the upload picker's chip grid scoped to that swing.

**The chip grid, not a `prompt()`, and that was the session's one real decision.** The golfer's
repair control prompts because a name is unbounded. The club is a closed vocabulary of 22 ids the
page already holds, and free text on a phone turns a typo into a 400 rather than a tag — which is
`parse_club` refusing to guess, correct at the boundary and useless in a bay.

**Three shapes worth knowing, all of them consequences of where the state lives:**

1. **`clubChips` gained a `selected` argument**, defaulting to the cursor's club; the retag picker
   passes the *swing's*. Highlighting `currentClub` inside a swing row would assert the swing was
   hit with whatever the bay is on now — the one fact a repair control must not invent, since it is
   exactly what the golfer opened it to correct.
2. **Collapsed behind *change*, where the bar at the top is never collapsed.** P7's argument run the
   other way: the cursor changes every few shots, a retag happens once per swing.
3. **`retagSwing` and `retagPending` are page state, not DOM state.** `renderStatus` replaces that
   subtree every five seconds — held in the markup the picker shuts itself mid-tap, and a poll
   landing across the write flashes the old club back.

**No new tests, and that is the standing precedent** (P7, P19): nothing here tests a static file,
and the route is already pinned in `tests/api/test_uploads.py`. The page's own script was driven
against a live server on a scratch data directory with a stubbed DOM instead — an untagged swing
beside a tagged one, the cursor deliberately parked on a *third* club — through 22 checks: the two
row states, the picker highlighting nothing on the untagged swing and `7i` on the tagged one, the
retag itself against the real manifest, a poll landing on an open picker, and the pending guard.

**Three behaviours were watched fail rather than assumed**, the P3–P19 habit. Reverting `clubChips`
to P7's shape fails 4 checks — including one that says the cursor's club would have been
highlighted on another swing's picker. Removing the `retagPending` guard fails 1. Rendering the open
picker from the markup instead of state fails 5.

**Then driven on a copy of the real corpus, which is the payoff.** With the two stored swings
retagged through the route, `scripts/club_profile.py` leaves the empty state and prints a `7i` row —
2 swings, 2 shot photos, 2 sessions, every claim withheld against the 5-, 10- and 12-sample floors —
and `mcp/club.py` moves from `NEVER_HIT` to `NOT_ENOUGH_ON_THIS_CLUB` with `untagged_swings` at 0.
That is the first time any M9 surface has spoken about a club on real numbers, and the correct
answer is still a refusal.

**The real `data/` was not touched.** Only the person who hit those two swings knows what hit them —
ADR-024 §5's own argument — so that tap is the golfer's to make, not this session's.

**Four prose sites that said the button did not exist are corrected**: `mcp/club.py`'s
`NOTHING_TAGGED`, `career.html`'s `nothingTagged()`, and both blocks in `scripts/club_profile.py`.
Each keeps its own audience — the model and the CLI still name the route, the page names the
control. Two stale P19-era comments in `index.html` went with them ("empty until P19 can write one";
a bag is written on the career page now).

**Next**: M9 P20, the docs reconciliation, and M9 closes. `pytest tests/test_docs_truth.py` first
and work only from its failures; `docs/ARCHITECTURE.md` §4 still calls `session.json` the "golfer
cursor" and no route table knows the seven routes M9 has added.

---

## 2026-08-22 — M9 P19: the bag page, and the first thing that ever wrote a bag

**Duration**: ~1 session. Three routes, a section inside `career.html`, one new test file and two
carry-over honesty fixes. Suite 979 → **993** (+14). **No `ANALYSIS_VERSION` bump** — nothing
measured, nothing written to an artifact, same as P12–P18.

**What prompted it**: "can we work on the next thing for m9?" P19 was the ROADMAP's NEXT ACTION.

**What it does.** `GET /api/golfers/{player_id}/bag` over the same three lines
`scripts/club_profile.py` and `mcp/club.py` run, plus `POST` and `DELETE` on
`/api/golfers/{player_id}/bag/{club}`. `career.html` gained a **Bag** section above what is now
**Whole bag**: one collapsed `<details>` per club carrying the headline number, expanding to the
bag entry, the three evidence counters, P16's caveat and the per-club metric cards. Every club row
edits its loft, make, model, shaft and length in place, and a picker adds a club to the bag.

**Two open questions in the box, both settled with the user before any code.** A section inside
`career.html` rather than a new `bag.html` (one page per golfer), and set + remove rather than
set + remove + restore (the retired shelf is still written and still not served).

**The route cannot serve its contract raw, and the career route beside it can.**
`ClubProfile.category`, `clubs_used` and `clubs_declared` are plain properties — P14 chose that and
named this surface as the projector. Deriving `category` in JavaScript would put a second copy of
`CLUB_CATEGORY` in a static file nothing tests, which is exactly the failure `GET /api/clubs`
exists to prevent. `_bag_summary` projects it; the two lists travel as counts so the membership
rule stays on the contract.

**Both writers return the whole bag**, so the page has one render path and a save shows what a
reload would — including P16's caveat, which declaring an entry is what *creates*. A response
assembled from the request is the one place that caveat could go missing.

**Unplanned but obvious once written: `_registered`.** Four golfer-scoped routes open with the same
two steps and `player_id` reaches a filesystem path in all of them. The career route was moved onto
it.

**Two prose errors found by driving rather than reasoning, both fixed:**

1. `fmt` printed `154.400 yards` on **two** pages. `career.html` and `results.html` both carried
   `1dp for degrees, 3dp otherwise`, written before P8–P10 added yards and mph. P17 fixed this in
   the CLIs and carried the table into both rather than let them disagree; same carry here.
2. `scripts/club_profile.py` said a stored swing is retagged "which the same page does with one
   tap". **There is no such button.** `index.html` has a per-swing *golfer* repair control and no
   club counterpart, so `POST /api/sessions/{s}/swings/{sw}/club` is API-only. Both the CLI and the
   new page now say so.

**That second one is the real finding, and it is now the ROADMAP's NEXT ACTION.** Every swing on
disk predates the club tag, and nothing on a phone can retag them — so every M9 surface correctly
reports the empty state and will keep doing so until that control exists. New uploads are tagged
fine by P7's picker. It is a small change against a route that already works.

**Two of the new tests were wrong before the code was.** The counter-divergence test tried to make
two swings share one shot photograph; they cannot, because `assign_from_path` dedupes an identical
upload back onto the swing that already holds it — the real shape of that gap is a clip filmed
without the screen being photographed. And the traversal test copied `test_career_route.py`'s
`..%2F..%2Fetc` form, which **httpx normalises away before it is sent**, so that assertion passes on
a routing 404 and never reaches `_safe` at all. This file pins the character class instead. The
career one still carries the weaker form — not wrong, just not testing what it reads as, and worth
knowing before it is copied a third time.

**Driven both ways, since nothing tests a static file.** The page's own script was run against a
live server with a stubbed DOM: against the real corpus it renders the empty state (0 clubs,
`untagged_swings` 2), agreeing with `scripts/club_profile.py` and `get_bag_profile` — three surfaces
over one builder. Against a seeded corpus it renders a 7 iron at `carries 154.3 yards over 12 shots`
across 13 swings and 4 sessions, with the ceiling line, P16's all-predate caveat, a driver refusing
on one shot, and a declared-but-unhit 3 wood.

**Next**: the per-swing club control, then P20 (docs reconciliation) closes M9.

---

## 2026-08-22 — M9 P18: the MCP tools, and the discovery that "no numbers" has five spellings

**Duration**: ~1 session. One new module, both adapters, two new tool descriptions, a new caveats
block and one carry-over into `get_swing`. Suite 956 → **979** (+23). **No `ANALYSIS_VERSION`
bump** — nothing measured, nothing written to an artifact, same as P12–P17.

**What prompted it**: "can you work on next m9 thing?" P18 was the ROADMAP's NEXT ACTION and P17's
own entry set it up: the honest answer for an untagged corpus is not a refusal, and the tool
descriptions have to let Claude tell those apart.

**What it does.** `src/golf_coach/mcp/club.py` — `career.py`'s sibling, one cut further in.
`bag_profile` and `club_profile` over `read_corpus` + `BagStore.get` + `build_bag_profile`, the
same three lines `scripts/club_profile.py` runs. Registered in **both** adapters, because
`test_runner_tools.py` compares the live server to the in-process runner and to
`TOOL_DESCRIPTIONS`; a tool added to one of them is a failure, not a half-feature. The server now
advertises **10** tools with a registry configured.

**A new module rather than an extension of `career.py`**, which the box allowed either way. It
reaches into it for `Refusal`, `resolve_golfer`, `THE_UNBLOCK`, `_refusal`, `_points` and
`_low`/`_high` — the same intra-package reach `career.py` already makes into `query._missing`.

**The shape decision: `ClubMetric` is not `career.MetricProfile`.** That model carries the tour
join and there is no per-club tour join to put in it — ADR-024's second trap is that `ranges.json`
holds `club_category: "all"` rows only. Reusing it would have shipped `tour_standing: "withheld"`
on every metric of every club forever: a refusal of a claim nothing ever intended to make, which
reads to a model as "ask again with more data". So the baseline and dispersion halves are mirrored
field for field, the tour block is absent, and a test asserts no field starting with `tour_` exists.
~15 duplicated field declarations, and they are the price of not fabricating a refusal.

**The phase turned out to be about counting silences, and two of them were found by driving it.**
The box named the refusal case and P17 had found the untagged one. Rendering a *populated* bag —
the real swings retagged `7i` with a bag entry a day later — showed the first cut giving the
withheld sentence to a driver that had been declared that morning and never hit. Nothing was
claimed about it, so nothing was refused: "every claim about this club is withheld" was a refusal
the module invented, which is exactly the failure the tour block is left out to avoid, one field
over. Same for swings that carried no measurement, which is not about the club at all. Five
constants now, one per silence, plus a sixth for untagged swings sitting *beside* a populated bag —
both are true at once there, and a note carrying one describes half the situation as the whole.

**`scripts/club_profile.py` is what made that findable.** P17 already separated these cases on
screen (`_no_metrics_reason`), and the first cut of the payload was less honest than the dev CLI,
which is the wrong way round. The CLI was the reference the whole way through.

**One carry-over outside the box: `SwingView.club`.** `get_swing` already loads the manifest that
carries it, and a model that can ask about a 7 iron but cannot see which club hit a given swing is
a gap with a four-line fix. `None` is explained on the field rather than left to read as an error.

**The prose channels, because a model reads three of them.** `GET_BAG_PROFILE` and
`GET_CLUB_PROFILE` say the four things that are easy to misread — offline is where the ball
*started* projected to the carry and never where it landed, curve is `face_to_path_deg`, `n_shots`
caps every distance, and a club with no bag entry has no loft — and `caveats.READING_A_BAG` says
them once at connect, gated on the same registry flag as `READING_A_PERSONAL_HISTORY` and shipped
beside it rather than folded in.

**Driven, twice, on real data.** On disk: `get_bag_profile("aaron")` returns the empty state with
`untagged_swings` of 2, byte-identical to what the CLI prints; `"7 iron"` resolves to `7i` and
reports never-hit; `"wedge"` is refused as a category; an unknown golfer is the `NotFound` shape.
Populated: the retagged corpus rendered P16's all-predate caveat and the declared-but-unhit driver,
and a synthetic 12-swing history across 4 sessions rendered `center 154 yards`, `sd 3.133` and
P16's *mixed* form, "3 of the 12 swings…".

**Next**: M9 P19 — the bag page. `GET /api/golfers/{player_id}/career` is the route template and
`career.html` the page template; the refusal strings are written for a human and should be rendered
verbatim rather than as blank cells. P18's five silences apply there unchanged, and a blank cell
cannot express any of them.

---

## 2026-08-22 — M9 P17: the first reader, and the output the phase list could not have predicted

**Duration**: ~1 session. One new script, plus a two-line carry-over into two existing ones. **No
tests** — `tests/` has no `scripts/` mirror and none of the three career CLIs carries one; suite
stays 956. No `ANALYSIS_VERSION` bump, same as P12–P16.

**What prompted it**: "Can we work on the next thing for M9?" P17 was the ROADMAP's NEXT ACTION, and
P16's own entry argued for it over P18/P19 on the grounds that a caveat nothing renders is a caveat
nobody has seen.

**What it does.** `scripts/club_profile.py` — the fourth career-shaped CLI and the first reader of
anything M9 built. `read_corpus` + `BagStore.get` + `build_bag_profile`, then one block per club:
the physical club off the bag entry, all three evidence counters, P16's caveat, and every metric
with its center, spread, findings and refusals. Argument shape is the career CLIs' `--name` /
`--player-id`, plus `--club` through `parse_club`.

**The box's Done-when described an output that cannot happen, and finding that out is the phase.**
It predicted "a table of refusals — expected, since every swing on disk is untagged". There is no
table: `build_bag_profile` for `aaron` returns **zero club profiles**, because a club with no swings
and no bag entry gets no row at all. There is nothing to refuse *about*. P15 and P16 both verified
that exact number and neither of us noticed it contradicted P17's acceptance criterion sitting four
boxes down.

**So the CLI grew a branch the plan did not have.** `_report_empty` prints the untagged count, says
those swings are real history no per-club number can be cut from, and names where a tag comes from.
The two states need opposite responses — a refusal table means go and hit balls, an empty profile
means nothing on disk names a club at all, which no bay session fixes — and they look identical from
a distance. A golfer's name followed by a blank would read as a bug in the CLI when the finding is
about the disk.

**The one output decision I checked instead of assuming.** `MetricDispersion.withheld` is not
printed, because `analysis/dispersion.py`'s `_carry_refusals` builds it by *filtering*
`MetricBaseline.withheld` — so every sentence in it is already on screen. I read that off the source
and then went and proved it (`d <= b`, two metrics, in a REPL), which was worth doing: on output
that is nothing but refusals, being wrong would have meant every metric block printing each
sentence twice.

**The formatting rule is where the templates stopped being copyable, and it was already broken
upstream.** Both career CLIs spell it `1dp if unit == "degrees" else 3dp` — written when every
metric in the repo was shoulder-width-normalized. P8–P10 added `yards` and `mph`, and `ms` was
always there, so the rule prints `154.400 yards`. Replaced here by a unit → precision table and
**carried back into both career CLIs**: `career_dispersion.py` has printed both distances since P8,
so the wart is already theirs and is only invisible because no center has cleared its sample floor
yet. Three copies of a six-entry dict, since `scripts/` is not a package — a per-script `_fmt` is
the standing shape and a fourth spelling would be worse than a third copy.

**Two scratch drives, because a renderer that has only rendered nothing is not verified.** Nothing
on disk can produce a club profile, so: (1) the real stored swings retagged `7i` in memory with a
bag entry recorded a day later, which put P16's all-predate caveat on screen for the first time
outside a test and exercised the declared-but-never-hit branch with a driver; (2) a synthetic
12-swing 7-iron history across 4 sessions with the entry recorded midway. The second is what proves
the formatting change — `center 154.4` in yards beside `center 3.014` for `tempo_ratio` in the same
output — and it rendered P16's *mixed* form, "9 of the 12 swings…", which nothing else has produced.

**One ordering fix found only by looking at it.** `unavailable` ("no target for this metric") first
printed between the finding row and the refusals explaining the missing numbers, which put a
paragraph between a question and its answer. Moved last, which is `career_dispersion.py`'s order and
for its stated reason: the refusal a bay session fixes and the one it never will need opposite
responses, and the terminal one reads best as the last word.

**Next**: M9 P18 — the MCP tools, `get_bag_profile` / `get_club_profile`. The view-model pattern in
`mcp/career.py` is the template, and the tool descriptions are load-bearing (`test_docs_truth.py`
reads them). P17 is a useful precedent for one thing in particular: the honest answer for an
untagged corpus is not a refusal, and the tool descriptions have to let Claude tell those apart.

---

## 2026-08-22 — M9 P16: the bag-changed caveat, and the sentence that needed two forms

**Duration**: ~1 session. One helper in one existing module, 8 tests. Suite 948 → 956. **No
`ANALYSIS_VERSION` bump** — nothing measured, nothing written to an artifact, same as P12–P15.

**What prompted it**: "Can we work on the next thing for M9?" P16 was the ROADMAP's NEXT ACTION and
the last hole in P15's builder — `_profile_for`'s docstring literally said *"`caveats` is left
empty: P16 owns it"*.

**What it does.** `_bag_changed_caveats(entry, swings)` in `analysis/club_profile.py`. When some of
a club's swings were captured before its `BagEntry.recorded_at`, the profile carries one sentence
saying so. **No statistic is withheld**, which is the whole posture: `contracts/bag.py` and
`contracts/club_profile.py` both already stated it on the fields themselves, because an entry
recorded late for a club that never changed is the likelier case and the cost of being wrong has to
be a sentence rather than a verdict. Same shape as `SESSION_DRIFT_FACTOR` one module over.

**The one real decision was that the sentence needs two forms, and I only found it by printing it.**
The phase list specifies one: *"shots before and after it may have been hit with a different club"*.
Built that way and driven against a real stored swing, it printed **"1 of these 1 swings were hit
before…"** — broken English on the smallest history there is. Fixing the grammar surfaced the actual
problem behind it: when **every** swing predates the entry, nothing is being pooled at all. There is
one population of unknown provenance, and what is in doubt is whether the make, model and loft on
the entry describe the club that hit any of it. Telling that golfer their carry average "mixes two
clubs" is alarming and false.

**And that is the common case, not the exotic one.** Nothing writes a bag until P19, so the day a
golfer first declares one, *every* club they own lands on the all-predate branch simultaneously. A
caveat that is wrong on the case it fires on most is a caveat people learn to skip — including on
the mixed case, where it means something real. So the mixed form names the proportion and says the
numbers pool both; the all-predate form drops the count (there is no proportion when every swing is
on one side) and points at the make, model and loft instead.

**The count is one more than the box asked for, and it earns its place.** The sentence is
permanently true once it fires — the earliest swing never moves — so a bare date reads identically
on the day a club is declared and a year later. A proportion deflates on its own as history
accumulates past the entry, and "2 of 40" is a different situation from "2 of 4" in the only way a
reader can act on.

**Four breaks watched fail, and the pair on withholding is the one worth remembering.** The obvious
break — zero the caveat — fails on `len(caveats) == 1` and proves nothing about the statistics. The
*dangerous* one is the plausible bad fix a future session would actually write: keep the caveat, and
narrow the numbers back to post-entry swings so the mean "describes the declared club". That one
fails on `BaselineClaim.CENTER in ready`, with a mean that was fine at `n = 6` gone to `None` at
`n = 3`. Only the second exercises the assertions that matter, and I had to break it twice to learn
that. Also confirmed: handing the helper `corpus.swings` instead of the narrowing fails on the
**count** (`"2 of the 8 swings"` for a club with four), not on the presence of a caveat — a test
asserting only "a caveat exists" would have sailed through.

**A small trap for whoever writes P17–P19: the phase list's file paths for P16 were stale.** It
names `tests/analysis/test_club_profile.py`, which collides with P14's
`tests/contracts/test_club_profile.py` and **interrupts the entire pytest run at collection** rather
than skipping a file. P15 found this and wrote it up; the P16 box predated that. Corrected in the
box, so nobody rediscovers it.

**One flaky test, and it is not this change's.**
`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` failed once in a full
run and passes in isolation and on re-runs. It is a threading test and nothing in `analysis/` is in
its import graph. Noting it rather than filing it — if it recurs it is worth a look.

**Real-data drive, with its limit said out loud.** `read_corpus` + `build_bag_profile` for `aaron`
is byte-identical to P15's verified result: 2 distinct swings, **0 club profiles**,
`untagged_swings` 2. That is the useful check (nothing moved) and it exercises no caveat — nothing
on disk can, because every swing predates the club tag and no bag exists. The caveat's own drive was
a REPL: one real stored swing retagged `7i`, a bag entry recorded a day after it, and the sentence
came out right with `carry_distance_yds` still refusing at `n = 1` exactly as it did without the
bag. **P17's CLI is the first thing that will render one for real**, which is a good argument for
doing P17 next rather than P18 or P19.

**Next**: M9 P17 — `scripts/club_profile.py`, templated on `career_baseline.py` /
`career_dispersion.py`. Acceptance is "refuses cleanly on real data", the same criterion career mode
used; every club on disk is untagged, so a table of refusals is the correct output.

---

## 2026-08-22 — M9 P15: the builder, and the import question M9 has been carrying since P13

**Duration**: ~1 session. One new source module, one new test file, two existing files changed.
Suite 938 → 948. **No `ANALYSIS_VERSION` bump** — nothing measured, nothing written to an artifact,
same as P12–P14.

**What prompted it**: "Can we work on the next thing for M9?" P15 was the ROADMAP's NEXT ACTION.

**The phase's decision was the import direction, and the box's preferred answer turned out to be
the right one.** `analysis` may import `contracts` alone (ADR-008); `narrow_to` lived in
`storage/corpus.py`; the builder has to narrow per club. So `CareerCorpus.narrowed_to` now holds the
filter and the whole docstring, and `storage.corpus.narrow_to` delegates in one line — the name
stays because `mcp/career.py` calls it at three sites and it is where a caller holding a corpus off
disk looks for the operation that narrows one. `CorpusSwing.artifact_key` was already the precedent
and it is exact: a rule both sides need lives on the shape both sides hold.

**The move dragged a second function with it, and that is the part worth remembering.**
`narrowed_to` recomputes `metric_counts`, and that recomputation *was* `_count_metrics`, private to
the reader. A filter that does not recompute its counts is precisely the bug the filter exists to
prevent, so it could not be left behind — it moved as the public `contracts.career.count_metrics`,
a module function rather than a method because `read_corpus` needs it on a bare list before there is
a corpus to call it on. **This is the shape of most "just move the pure half" jobs**: the pure half
is never quite as small as it looks from the call site.

**The move was verified before the feature was written**, which is why it cost nothing later: 431
existing tests plus `mcp/career.py`'s three call sites passed **without a single edit**. If any of
them had needed one, the move would not have been behaviour-preserving and that is the moment to
find out — not after a new module is sitting on top of it.

**`CareerCorpus.distinct_sessions` was added to finish the trio**, so all three of the builder's
evidence counters come off the narrowing instead of one being assembled by hand. Its docstring
carries the warning that has to travel with it: it is **not** the number a TREND claim gates on.
That gate reads `MetricBaseline.n_sessions`, which counts only sessions that contributed a sample to
*that metric*, so an unanalyzed session raises one and not the other. Both are right, and a
mismatch nobody explained is a bug report waiting to be filed.

**Two things the builder deliberately does not do, both commented at the line.** It never reads
`Bag.retired` — the shelf is retention, not the versioning ADR-024 defers, so a club that left the
bag keeps every statistic and loses only its loft. And it does not collapse the second
`build_baseline`: `build_dispersion` takes a corpus and builds its own on purpose, so its guarded
statistics and its raw per-session samples provably describe the same read, and handing it one built
in the loop would be the seam through which one club's spread pairs with another's sessions.

**The unbudgeted find: two test files cannot share a basename in this repo, and the failure is
total.** `tests/` holds no `__init__.py`, so pytest's default `prepend` mode imports every test
module under its bare name — and `tests/analysis/test_club_profile.py` beside P14's
`tests/contracts/test_club_profile.py` does not skip one file, it **interrupts the whole run at
collection**. `--import-mode=importlib` is the real fix and was tried; it fails ten modules across
`tests/analysis/` and `tests/launch_monitor/` that do `from conftest import ...`, which is the same
`prepend` idiom. **That is its own commit and is not done** — converting those ten to real fixtures
(or to `tests.analysis.conftest`) is a contained job that unblocks the mirror convention permanently
and should be taken before the next name collision, because the next one also stops the suite dead.
Meanwhile the file is `test_club_profile_builder.py` and its docstring explains the deviation, so
nobody restores the mirror and rediscovers this at the worst moment.

**Ten tests, and the new pin is the one that would not have existed without this phase.** R1 and R2
are `[survey]` rules — `/refactor-review` reads them and nothing enforces them — so a static
`ast` check that `analysis/club_profile.py` imports no `storage` is now the thing standing between
the move and its own quiet undoing. Read statically for `test_comparison.py`'s reason:
`analysis/__init__.py` imports `engine`, so a `sys.modules` check would answer a question about the
package rather than about this file.

**Six behaviours watched fail rather than assumed**, each broken in-process: club order off
`bag.entries` instead of walked from `ClubId` (the `BagProfile` validator catches it), the whole
corpus handed to `build_baseline` and separately to `build_dispersion` (both fail on `n`, not on the
club list — the count pin doing what a list pin cannot), `untagged_swings` read off a narrowed
corpus, `bag_entry` falling back to the shelf, and the convenient one-line storage import, which
fails **only** the new pin and nothing else. That last one is the whole argument for having it.

**Full suite green**: 948 passed, `ruff check src tests scripts` clean, `mypy src` clean across 94
files (93 → 94). **Driven on the real corpus**: `aaron` profiles **no clubs at all** with
`untagged_swings` 2, because both swings on disk predate the tag — the correct output, and a club
appearing there would have been the bug. Driven once more with an in-memory bag (nothing written to
disk) to exercise the declared-but-unhit branch: `driver` and `7i` come back in canonical order with
their lofts, `in_bag=True`, and no statistics.

**Still stale for P20**, unchanged since P4: `docs/ARCHITECTURE.md` §4 calls `session.json` the
"golfer cursor", its manifest row names only `player_id`, and no route table knows the four routes
M9 has added. Nothing pins it, so nothing goes red.

---

## 2026-08-22 — M9 P14: the shape a per-club answer comes back in, with two counters instead of one

**Duration**: ~1 session. One new contract module, one new test file, no other source touched.
Suite 929 → 938. **No `ANALYSIS_VERSION` bump** — nothing here is measured and nothing is written
to an artifact, same as P12 and P13. Worth saying because P8, P9 and P10 each carried one.

**What prompted it**: "Can we work on the next task for M9?" P13 had just closed, and P14 —
`contracts/club_profile.py` — is what the ROADMAP's NEXT ACTION pointed at.

**The phase is a declaration, not a computation, and that framing is the whole of it.** Career mode
already built the guard; P13's `narrow_to(club=)` already runs it over one club's swings with
`metric_counts` recomputed. What was missing was the shape the answer comes back in. `ClubProfile`
and `BagProfile` compose `MetricBaseline`, `MetricDispersion` and `BagEntry` rather than restating
their fields — the move `contracts/dispersion.py` already makes with `Interval` and `WithheldClaim`,
one level up (R5). Nothing here averages, gates or judges.

**The one design decision was the counters, and the box named one where two were owed.** `CareerCorpus`
already splits `distinct_swings` from `distinct_shots`, and per club they diverge in a way that
matters: a 7 iron filmed six times with two shot-screen photos is **six swings of history and a
carry ceiling of two**, because every launch-monitor claim dedupes on the photo's hash. One counter
called `n_shots` would either undercount the history or overstate what a distance statistic can be
built from — and a carry average is exactly the number nobody audits. So `n_swings` and `n_shots`
both ship, both always populated and never gated, on the same footing `MetricBaseline.n` sits
outside the guard: they are facts about how much data exists, not claims about the golfer.
`clubs_used` derives off `n_swings`, so a club hit on video with no screen photo keeps its history.

**Two other departures from the box, both toward the repo's existing precedent.** `category` is a
derived `@property` through `category_of` rather than the stored field the box listed — `club.py` is
explicit that `CLUB_CATEGORY` is *the* one table, and a copy on every profile is a second home free
to disagree (R4). And `BagProfile.untagged_shots` shipped as `untagged_swings`, because its only
source is `CareerCorpus.untagged_swings` and a number renamed on the way through is two spellings
free to be reported differently.

**Two shapes the box did not specify.** `clubs` is a flat tuple, not a dict keyed by club:
`ClubProfile.club` already names the slot, and a second key is a second thing that can disagree with
the entry it holds — the bug `Bag._keys_match_entries` exists to catch. And a `model_validator`
**pins canonical bag order** and rejects a repeated club, because `club.py` says declaration order
is read and not decorative and that sorting downstream is the bug it exists to prevent; a validator
makes that enforceable rather than a convention P15 can forget. `profile_for` is there for the
`Bag.retired_for` reason — P17's `--club 7i` and P18's `get_club_profile` are both scans over
`clubs`, and two hand-rolled ones are two places to get it wrong.

**Nine tests, and the round-trip is the one carrying weight.** It nests a real `MetricBaseline` with
both a ready claim and a `WithheldClaim` inside it, plus a `MetricDispersion` with an `unavailable`
entry, because a refusal that fails to survive storage renders as a blank cell — which reads as
zero, the one thing `contracts/baseline.py` exists to prevent. `category` is asserted across every
`ClubId` rather than a written-out list (R6). The four-combination test reads **both** derived lists,
since checking one would pass with the two collapsed.

**Both pins watched fail, on different failures.** Collapsing `clubs_declared` onto `n_swings > 0`
fails the four-combination test with `[driver, 7i] != [driver, 3w]` — the two lists answering one
question. Dropping the order clause from the validator fails the ordering test with `DID NOT RAISE`
in both the insertion-order and the alphabetical direction.

**Nothing to drive end to end**, and that is correct for this phase rather than a gap: no builder,
no route, no artifact. The real-data run belongs to P15/P17, whose acceptance criterion is that
`scripts/club_profile.py aaron` prints refusals for every club, because both swings on disk predate
the tag.

**One stale line fixed on the way past.** `docs/M9_PLAYER_TRACKING.md`'s status block still read
*"11/20 phases … start at P13"* — P13's session ticked its own box and updated the ROADMAP but not
this header. It now reads 14/20 and points at P15.

**Full suite green**: 938 passed, `ruff check src tests scripts` clean, `mypy src` clean across 93
files (92 → 93 for the new module).

**Still stale for P20**, unchanged since P4: `docs/ARCHITECTURE.md` §4 calls `session.json` the
"golfer cursor", its manifest row names only `player_id`, and no route table knows the four routes
M9 has added. Nothing pins it, so nothing goes red.

---

## 2026-08-22 — M9 P13: the club clause, plus the version bump that never reached the disk

**Duration**: ~1 session. Three lines of code, four tests, one `reanalyze.py` run, and two boxes
of paperwork that were owed. Suite 925 → 929. **No `ANALYSIS_VERSION` bump** — P13 changes no
measurement and writes no artifact.

**What prompted it**: "What is the next item to work on for M9?" P13 was the answer, and the two
loose ends P12 flagged were in front of it. The user took all three in one change.

**The disk was a day behind the code, and that was the real find.** P10 shipped in `f930973` with
`ANALYSIS_VERSION` 9 → 10 and no `scripts/reanalyze.py` run, so all four stored `analysis.json`
files still read version 9. Every swing therefore read `OUTDATED`, and `Honest n per metric`
printed **(none)** — the corpus counted nothing at all. That is not a cosmetic staleness: it would
have made P13's real-data verification meaningless, because a narrowed corpus recomputes an empty
`metric_counts` either way.

`--dry-run` named all four with one reason each and nothing wanting a human, and the run itself was
clean: **version 9 → 10, measurements 19 → 21 on every swing, every `overall_score` byte-identical**
(97.45951982132875 ×3, 96.88296555239968). 21 metrics came back into the counts, including P10's
`ball_speed_mph` and `launch_angle_deg` at n = 2.

**One thing surfaced that was hiding behind the empty counts**: `Unrecognised measurement sources:
population:golfdb`. That is M8's placement provenance being *named rather than absorbed*, which is
exactly what `CareerCorpus.unknown_sources` is for — it was invisible only because no swing carried
a counted measurement. Pre-existing, correct, and left alone.

**P13 itself is three lines and no new judgment.** `ClubId` imported, `club: ClubId | None = None`
on the keyword-only signature, `and (club is None or swing.club == club)` in the comprehension. The
`model_copy(update=...)` block was not touched, and that is the whole point: it already recomputes
`metric_counts`, so the per-club `n` is honest for free and a per-club mean carry refuses at exactly
the thresholds a whole-corpus metric refuses at.

**The docstring says two things, and the second is the one that needed writing.** The unlock is
obvious. Less obvious is that a club narrowing is where an untagged swing *finally* drops out —
`read_corpus` keeps it on purpose, because the club was never an input to measuring head sway, and
a per-club view is the one place the tag is load-bearing. `untagged_swings` follows to 0 with
nothing here to remember, because P12 shipped it derived; the docstring says so at the call site
that shape was chosen for.

**`contracts/dispersion.py` was deliberately not touched.** Its three provenance constants each
name `narrow_to(club=)` as what makes a per-club tolerance *measurable*, and that stays true —
revising them needs a per-club **sample**, which needs a bay session, not this commit. All three
also say they should be revised in one pass, and that instruction is left standing.

**A test was wrong before the code was, and the reader caught it.** The first draft of the counts
test built three swings with distinct clubs and asserted `carry_distance_yds` counted 3. It counted
**1** — launch-monitor metrics are keyed on the *shot photo's* hash, and `write_swing` defaults
every swing to one `shot-hash`, so the fixture had built one shot photographed once and attributed
to three swings. The corpus reader was right; the fixture was the lie. Fixed by giving each swing
its own shot, which is also the shape the dedupe rules describe.

**Both pins watched fail, on different assertions.** Removing the club clause fails all four new
tests. Carrying the whole read's `metric_counts` through instead of recomputing fails five — the
four plus the pre-existing narrowing test — and fails them on **counts, not lists**:
`{'carry_distance_yds': 3} == 2`, which is the count assertion doing the job the list assertion
cannot. A fourth test the box did not ask for pins the untagged interaction from both ends, and it
is the one that goes red if `untagged_swings` is ever converted to a stored field.

**Driven on the real corpus.** The whole read is 2 swings with `carry_distance_yds` n = 2; narrowing
to a 7 iron or a driver returns **0 swings, `untagged_swings` 0, no counts** — honestly empty,
because both swings on disk predate the tag. `scripts/career_corpus.py --player-id aaron` is
**byte-identical** to the post-reanalyze run, which was the acceptance criterion: nothing in that
script passes `club=`, so any movement would have been a bug.

**Paperwork, owed and now paid.** P10's box is checked with what actually shipped (both metrics,
the two `METRIC_TARGETS` rows P11's parity pin forced into the same change, and the version bump
whose `reanalyze` run was missing). P13's box is checked. ROADMAP's status row went 11/20 →
**13/20**, the false sentence "P10 is open and was deliberately not taken" is gone, and NEXT ACTION now points
at **P14** — `contracts/club_profile.py`, the start of the profile track P13 made buildable.

**Full suite green**: 929 passed, `ruff check src tests scripts` clean, `mypy src` clean across 92
files. `tests/test_docs_truth.py` 57 passed.

**Still stale for P20**, unchanged since P4: `docs/ARCHITECTURE.md` §4 calls `session.json` the
"golfer cursor", its manifest row names only `player_id`, and no route table knows the four routes
M9 has added. Nothing pins it, so nothing goes red.

---

## 2026-08-21 — M9 P12: the club crosses into the corpus, and a counter that cannot go stale

**Duration**: ~1 session. One field, one derived property, one CLI block, six tests. **No
`ANALYSIS_VERSION` bump** — nothing here is stored in an artifact. Suite 919 → 925.

**What prompted it**: "Can we work on the next stage of M9?" — P12, which is what ROADMAP's NEXT
ACTION said. Scope was held to P12 alone at the user's instruction; **P13 was explicitly not taken**,
and it is the next thing.

**The phase is one line of code and two arguments about shape.** `_corpus_swing` now passes
`club=manifest.club` into the `CorpusSwing(...)` constructor. Everything else is deciding *which*
manifest and *what* to count.

**Which manifest: the survivor's, and nothing borrows from a duplicate.** A duplicate group is one
clip uploaded more than once, and every upload stamps the session cursor as it stood when *that* file
arrived — so a clip re-sent after the golfer moved on to a wedge carries a wedge. The survivor is the
earliest arrival, closest to the capture, so its tag wins. That also keeps P5's instruction that the
club appears at every site `player_id` does and at no others, since `player_id` is read off the
survivor with no fallback. **No `conflicting_clubs`**, deliberately, and the comment sits right above
`_conflicting_shots` so the asymmetry is readable: a second shot photo names a *repair*, because
whichever of the two is misattached is attached to some swing being scored on it, whereas a
duplicate's club is attached to a directory that contributes nothing. And an untagged survivor does
**not** borrow a tagged duplicate's club — that infers what hit the swing from what the cursor said
when someone re-sent the clip, which is exactly the guess `parse_club` refuses at the boundary.

**What to count: `untagged_swings` shipped as a derived property, where the phase box said `int`.**
The box's reasoning — "counted for the same reason `unattributed_swings` is" — carries over; its
*shape* does not, and noticing that was most of the phase. An unattributed manifest never becomes a
`CorpusSwing` at all, because it is excluded before the swings are grouped, so that counter has to be
tallied during the scan or not at all. An untagged manifest is a real swing sitting in `swings`. So
this one is read back off them, and the payoff is specific: **P13 has no field to forget.** A stored
version would be a fifth entry in `narrow_to`'s hand-listed `model_copy(update=...)`, and a narrowed
corpus reporting the whole read's untagged figure beside a filtered swing list is precisely the
printed-`n`-describes-a-different-set failure that function exists to prevent. It also shares
`distinct_swings`' denominator, which is what lets "2 of 2 swings name no club" be one honest
sentence rather than two numbers from different populations.

**`scripts/career_corpus.py` gained a block, and that is why the file list grew past the box.** A
counter nothing prints is a counter nobody notices is wrong. It is also where the design note reaches
a human: the block says these swings *still count toward every metric above it* and are absent only
from per-club views, which is the ADR-024 rule stated where someone reading a refusal will meet it.

**Six tests, and both pins were watched fail rather than assumed.** Breaking the read in-process
(`swing.club = duplicates[-1].club`) failed the two survivor directions on **different assertions** —
`SAND_WEDGE is not SEVEN_IRON` on one, `SAND_WEDGE is not None` on the other — which is P4's lesson
arriving again and the reason both directions are written out separately instead of folded into one
test. The design-note pin was broken the same way: excluding untagged swings in-process turned
`{"head_sway_norm": 2}` into `{"head_sway_norm": 1}`, the silently-shrunk mechanics `n` the note
warns about, as a visible failure. The untagged test also asserts
`build_baseline(corpus).metrics[...].n == 2`, not just the count — a count that included the untagged
swing while the pooling did not would be the `artifact_key` disagreement arriving by a new route.

**Driven on the real data, and the acceptance criterion is that nothing moved.**
`scripts/career_corpus.py --player-id aaron` prints **2 distinct swing(s) naming no club (of 2)** —
every swing on disk predates the tag, which is the expected state — and every other line is
byte-identical to the run before the change. `Honest n per metric` unmoved, `Contributing no sample`
unmoved, no new exclusion reason. A metric count that had shifted was the bug this phase was most
likely to produce.

**Two things found in the working tree that are not P12's, flagged rather than absorbed.**

- **P10 is implemented and undocumented.** `ball_speed_mph` and `launch_angle_deg` are in
  `SHOT_MEASUREMENTS`, both have `METRIC_TARGETS` rows, and `ANALYSIS_VERSION` is **10** — but
  `docs/M9_PLAYER_TRACKING.md` still shows P10 unchecked, ROADMAP still calls it "open and
  deliberately not taken", and there is no worklog entry. **The four stored swings are still at
  version 9**, so every one of them currently reads `outdated` and `Honest n per metric` prints
  *(none)*. `scripts/reanalyze.py` is the unblock, and P10's box and status lines still need writing.
  Left alone here because it is not this phase.
- **One pre-existing `ruff` failure was fixed**: `contracts/dispersion.py:246` was 101 characters, in
  P10's uncommitted comment. Reflowed only — no wording changed — because the lint gate had to be
  clean for this phase's own verification to mean anything.

**Full suite green**: 925 passed, `ruff check src tests scripts` clean, `mypy src` clean across 92
files.

---

## 2026-08-21 — M9 P9: the lateral miss, in yards, and the sentence that keeps it honest

**Duration**: ~1 session. One measurement, one target row, one version bump, five tests.
`ANALYSIS_VERSION` 8 → 9, all four stored swings re-analyzed onto it. Suite 910 → 915.

**What prompted it**: "Can we do the next step in M9?" — P9, which is what ROADMAP's NEXT ACTION
said. Scope was held to P9 alone at the user's instruction; **P10 was explicitly not taken**, and it
is now the only measurements phase left.

**The phase's real content is a docstring, and it is about what the number is not.**
`start_line_offline_yds` is `carry * sin(start_line_deg)` — three lines of arithmetic over two
printed numbers, no physics invented, no parameter fitted. What needed writing down is that this is
**where the ball would have landed if it never curved**, which is not where it landed. The HD Golf
screen prints no offline tile at all, so a true offline is not available to be read; the only
horizontal quantity it prints is `Horizontal Angle`. The consequence is specific and is in the
docstring because it is the way this number misleads: **a golfer who starts it straight and slices
30 yards reads about 0 here**, and the whole of that miss lives in `face_to_path_deg`. Any prose
over this must say *started* and never *finished* — so the registry's `detail` string says it too,
since that is the one line about the metric a reader is guaranteed to see.

**It reads its inputs through the other extractors, not off `ShotData`.** `measure_carry_distance`
and `measure_start_line` already exist and already define which field is which; going to
`shot.carry_distance` directly would have been a second definition free to drift from the first.
Three lines either way, and the version that reuses is the one where renaming a `ShotData` field
breaks in one place.

**The pin that had to fail first did — and the fixture that P8 had to feed did not need feeding.**
`test_every_production_metric_has_a_tolerance` went red on the bare registry row, exactly as P8's box
predicted for any new metric. But `test_registry_entries_are_well_formed` stayed green untouched: its
one shot already carries `launch_direction=4.0` and `carry_distance=125.6`, because both inputs are
already registered metrics. **A metric derived from two existing ones inherits its fixture**, which
is worth knowing for P10 — those two fields are measured by nothing today, so P10 will have to feed
it the way P8 did.

**P11 is done, and it never ran as a phase.** Its last row was `start_line_offline_yds`'s target, and
the strict-equality pin means a target row cannot lag its metric by even one commit — so P8 took two
thirds of it and P9 took the rest. This is not scope creep into a second phase; it is the same
constraint P8 documented, arriving on schedule. P11's box is marked done and now records the
argument rather than the work.

**The tolerance is 9.0 yards and it is a conversion, not a fresh judgment.** `_JUDGED_DEGREES`
already claims 2° about the start line; the offline metric is that angle multiplied by a carry, so
its error is that claim carried through the same multiplication. The one free parameter is *which
carry*, and the widest club in the bag decides it, because one constant has to hold for every club
that shares it: `250 * sin(2 deg)` = 8.7, rounded up to 9. The 250 is written into the provenance
string rather than left implicit — it is the one number a bay session can replace with a measured
driver carry. **The wedge end is judged wide on purpose**: at a 125-yard carry, 2° is 4.4 yards, so
that club gets roughly twice the tolerance its own geometry asks for. That is the documented safe
direction (erring wide costs claims; erring narrow buys confident claims about the simulator's own
noise) but it is a deferral, and `_JUDGED_OFFLINE` says so beside `_JUDGED_YARDS`, which defers the
same thing for the two distances. Both should be revised in one pass.

**It gets a target where the distances did not, and the line between them is the whole point.**
Zero is straight **by geometry** — a ball that starts on the target line is offline by nothing,
whoever is swinging, whatever club — which is the same argument `start_line_deg` and
`face_to_path_deg` already make. That is not a claim about a population, which is the only kind of
target this repo declines to invent (ADR-010 §2). How far you *should* hit a 7 iron is such a claim;
which way you should start it is not.

**`ANALYSIS_VERSION` 8 → 9, the cheapest bump on that list.** Same shape as `3 -> 4`, `6 -> 7` and
`7 -> 8` — one new `measurements` entry, nothing judged, no score moved. What makes it cheaper than
P8's: the quantity is derived from two fields a version-8 artifact **already carries**, so a
version-8 file was never missing the information, only the arithmetic. As last time, nothing went
red to say the bump was owed — every test derives the number from the constant, which is right, and
means the rule has to be remembered rather than caught.

**Tests: 5 added, suite 910 → 915.** The arithmetic on a hand-checkable case (150 yd at 2° → 5.23,
and 2° is not incidental — it is what the tolerance beside it claims, so the case also says what
that tolerance is worth in yards at a mid-iron). The sign pin is written to compare the two metrics
**to each other** over four angles rather than each to a constant, because two readings of one
quantity in two units is exactly where a dropped minus sign produces a coherent-looking artifact
that says the golfer misses the other way. Zero is pinned beside the `None` cases deliberately:
`0.0` and `None` are the same falsy value to a careless reader and here they mean *dead straight*
and *unknown*. And a unit pin asserting offline is `yards` **while `start_line_deg` is still
`degrees`** — if both were degrees, `analysis.baseline` would average them together, since it keys
on name and unit and has no idea one is a projection of the other.

**Driven end to end.** `scripts/reanalyze.py` moved all four stored swings `version 8 -> 9 |
measurements 18 -> 19`, with **every `overall_score` byte-identical** to P8's figures —
97.45951982132875 on three, 96.88296555239968 on the fourth. The two real shots on disk miss in
**opposite directions**: −5.3° at 125.6 yd → −11.6017 yd, +4.0° at 121.0 yd → +8.4405 yd. That is
the sign working on real data in both directions, which no unit test can buy.
`scripts/career_dispersion.py` then printed the row beside `start_line_deg` — `target 0.000 +/-
9.000`, `n = 2 over 2 sessions`, both claims withheld against the 5- and 10-sample floors. Refusing
is the correct output at n=2 and is what M9's own verification criterion asks to see.

**Full suite green**: 915 passed, `ruff check src tests scripts` clean, `mypy src` clean across 92
files.

---

## 2026-08-21 — M9 P8: distance becomes measurable, and the pin that said the phase was bigger

**Duration**: ~1 session. Two measurements, two target rows, one version bump, five tests.
`ANALYSIS_VERSION` 7 → 8, and all four stored swings re-analyzed onto it.

**What prompted it**: "can we complete the next thing on M9? I think its going to be P8" — correct,
and it was ROADMAP's NEXT ACTION. Scope was held to P8 alone; P9, P10 and P11 stay open.

**The phase's real content is a docstring, and it is about timing.** `carry_distance` has been
sitting on `ShotData` since the OCR worked, and was left out of `SHOT_MEASUREMENTS` anyway. Not
because it was hard — it is a pass-through — but because **a carry pooled across clubs is not a weak
statistic, it is a meaningless one**: a mean over a driver and a sand wedge describes nobody's shot,
and its spread is mostly which clubs happened to be hit. The club tag M9 P4 put on `SwingManifest`
is what makes distance poolable at all. So the module docstring got a new section, *"What arrived
late, and why it could not arrive earlier"*, sitting **above** the exclusions rather than inside
them — the three exclusions (`smash_factor`, `club_head_speed`, `spin_axis`) are a different
argument and are untouched.

**The box was wrong about one thing, and the pin is what said so.** P8's file list names
`shot_measure.py` and its tests. But `test_every_production_metric_has_a_tolerance` asserts
`set(POSE_MEASUREMENTS) | set(SHOT_MEASUREMENTS) == set(METRIC_TARGETS)` — **strict equality**. P11
had deferred those rows on the belief that an unregistered metric is "measured and permanently
silent"; it is not, it is a red suite. So P11's two distance rows came forward with it, and the
phase went L1 → L2. That escalation is in the box, and **P11 is amended** so the next session does
not re-derive it: what is left there is `start_line_offline_yds` alone.

**`ANALYSIS_VERSION` 7 → 8, which the box also did not name.** Precedent is exact — `6 -> 7`
(ADR-023) was two new `measurements` entries with no checkpoint, band or score moved, and a
version-7 artifact is *missing* two quantities rather than disagreeing about any. Worth noticing
that **nothing went red to say the bump was owed**: every test derives the number from the constant,
which is right, and means the only thing that catches a missed bump is remembering the rule.

**Neither distance gets a target, and that is the whole judgment.** How far a golfer *should* hit a
club is not a number this repo has. Every distribution here is cut from GolfDB, which is pose over
broadcast video with no ball flight in it, and a tour carry band would additionally judge an amateur
against a population they are not in. They still register, because a tolerance is what buys the
*scatter* finding — how repeatable this golfer's distance is — which is answerable without anyone
declaring what good is. `_JUDGED_YARDS` = 5 yards, modelled line for line on `_JUDGED_DEGREES`: no
instrument-error evidence exists for the OCR path, 5 yards is below the level a coaching action
follows from, erring wide costs claims where erring narrow buys confident claims about the
simulator's own noise. The provenance string carries the same deferral P11 records for offline —
distance error scales with the club, so one constant is the wrong shape and the right tolerance is
per club.

**Two pins were watched fail first**, the P3–P7 habit, and one of them is how the missing dependency
was found rather than assumed. `test_every_production_metric_has_a_tolerance` fails on the bare
registry addition. And `test_registry_entries_are_well_formed` fails on **its own fixture** — it
measures every registered entry against one `ShotData` and asserts non-`None`, so the fixture had to
gain a carry and a total. That is the pin working, and it is the cheapest place in the repo to
notice a new metric has no test of its own, so it was fed rather than weakened.

**Tests: 5 added, suite 905 → 910.** Three in `test_shot_measure.py` — distances carried as printed,
a missing distance is `None` and never 0.0 (a duff that never happened would otherwise pool into a
mean), both registered in `yards`. Two in `test_engine.py`: the registry-to-artifact path, and the
direction that matters — a swing with **no** shot records no distance at all. The engine test pins
`source == "launch_monitor:hd_golf"` because `storage.corpus` keys its honest sample counts off that
string; a distance recorded under `pose:face_on` would be counted per face-on clip rather than per
shot photo, which is the "two dedupe keys" argument in `contracts/career.py`.

**Driven end to end, because the point of the phase is that the number reaches a stored artifact.**
`scripts/reanalyze.py` found all four stored swings still on **engine 6** and moved them to 8:
`measurements 14 -> 18` on each, `carry_distance_yds` 125.6 / `total_distance_yds` 131.0 under
`launch_monitor:hd_golf`, and **every `overall_score` byte-identical** — 97.45951982132875 on three,
96.88296555239968 on the fourth — which is the expected non-change for a bump of this shape. Then
`scripts/career_dispersion.py`, which is the first place M9's "expect refusals" criterion is
actually observable: `n = 2 over 2 sessions` (four directories deduping to two shot photos, exactly
as `contracts/career.py` predicts), both claims waiting on their floors, and the no-target reason
printed rather than the metric going quietly absent.

**One thing left commented for P13.** Until `narrow_to(club=)` lands, everything reading these two
pools them whole-bag. Nothing false ships today — `DEFAULT_MINIMUM_N[CENTER]` is 5 against 2
samples — but the guard that saves it is a sample count, not an argument about clubs, so it would go
on being satisfied by a mixed bag. Commented at the registry rows rather than left to be
rediscovered.

**What is next.** P9: `start_line_offline_yds`, `carry * sin(launch_direction)`, where the ball
*would* have landed if it never curved. Its docstring is the deliverable, same as this phase's.

---

## 2026-08-21 — M9 P7: the picker that closes the ingest spine, and the list it refused to copy

**Duration**: ~1 session. One static page, one new API route, one API test file.
`ANALYSIS_VERSION` unchanged; nothing analysis-side reads the field yet.

**What prompted it**: "can we work on M9 P7?" — the first open box, and ROADMAP's NEXT ACTION.
P6 had left the upload page unable to upload on purpose; this is the phase that pays that off.

**The one decision, which P6 deferred here by name: the picker derives its list.** P6's entry below
records the question — *does the picker derive its list from a route, or inline it?* Inlining the 22
ids would have put a second copy of `contracts/club.py`'s taxonomy in a static file nothing tests,
and the failure mode is silent: a club added to `ClubId` parses at every route in `api/app.py` while
being unpickable at the bay, forever, with nothing red. So **`GET /api/clubs`** was added and the
phase became an L2 rather than the L1 its box implied. That escalation is noted in the box.

**The route carries two things and the docstring says why.** `{"clubs": [...], "bag": [...]}` — the
full taxonomy off `ClubId`, plus the session golfer's declared clubs through `Bag.club_ids`. One
round trip because a phone on cellular pays for them, and neither half is useful alone: the taxonomy
cannot put the golfer's own clubs first, and the bag cannot offer the club they just borrowed. Both
sides walk the enum, so **canonical bag order comes from the declaration** and nothing sorts.

**No labels are invented, and that is the same rule as the list.** The ids go over the wire as they
are and the page uppercases them in CSS. A server-side `"7 iron"` would be a second spelling table
beside `_build_aliases`, free to disagree with the one that does the parsing.

**`BagStore` is derived, not injected.** `BagStore(golfer_store.root)` in `create_app`, because
`bag_store.py` writes `<player_id>.bag.json` into the directory `GolferStore` writes
`<player_id>.golfer.json` into and the suffixes are what keep them apart. Deriving the root means
the pair cannot be pointed at different directories by a caller — and it meant no `create_app`
parameter and no fixture change, so `golfer_client` already isolated the bag to `tmp_path` without
knowing it existed.

**The picker is an always-open chip grid, not the golfer bar.** The box's *"unlike the golfer"* is
contrasting against that bar's collapse-then-change flow rather than asking for it: the club changes
every few shots, so collapsing would cost a tap on almost every shot. Two sections when the golfer
has a bag (**In the bag**, then **Every club**), one when they do not — which is every golfer today,
since nothing writes a bag until P19.

**The file input is disabled while no club is selected**, and starts disabled in the markup so there
is no window at load where it looks usable. That makes the golfer bar's stated asymmetry visible on
the page: that bar never blocks the input, this one does, and the comment beside the CSS says why —
an untagged golfer is repairable after the session and an untagged club is not. And the 409 is now
**surfaced rather than dumped**: the failure branch used to print the raw response body, so someone
standing in a bay read `{"detail":"..."}`. It shows `detail`, and on 409 re-reads the cursor, because
reaching that status means the page is stale rather than that the golfer did something wrong.

**Two things the box did not name and the page needed.** `clubPending` holds the 5s poll off across
the POST — `editingGolfer` applied to a subtree that would otherwise flash the previous chip back.
And polling the cursor is what **survives midnight**: the cursor is per-session and a session is a
day, so the rolled-over directory answers `null`, the picker goes unset and the input disables
rather than the page showing a club it is no longer tagging anything with. Both are commented,
because both read as incidental.

**Tests: 8 added, 39 → 47 in the file, suite 897 → 905.** They assert against `ClubId` itself and
never a written-out list — a literal there would be the duplicate the route exists to prevent, and
it would pass on the day a club is added and forgotten everywhere else. One test walks *every* club
the route serves through the cursor route, which is the pin that the picker's list and the parser
are one vocabulary rather than two that happen to agree today.

**Two pins watched fail, and then the half no test covers was driven.** Serving `bag.entries`
returns insertion order and fails the bag test with `['pw', 'driver', '7i']`; sorting `clubs` fails
the canonical-order test at index 0. Then, because **nothing in this repo tests a static file**, the
page's own script was extracted and run against a live server on a scratch data directory with a
stubbed DOM, in all three states: club set (input enabled, chip highlighted), a bag declared
`sw, driver, 7i, pw` (rendered `driver 7i pw sw`), and the cursor cleared (`unset`, header
`none selected`, input disabled). The end-to-end also ran for real — `7i` then `pw`, two swings,
`["7i", "pw"]` on the read route and `"club": "7i"` in swing 1's manifest — which is the box's own
"done when".

**What is next.** P8, and it is a different track: carry and total distance become measurements.
P1–P7 were strictly ordered and are finished; P8–P11 are independent of all of them.

---

## 2026-08-21 — M9 P6: the club becomes required, and the page stops being able to upload

**Duration**: ~1 session. One API module amended, five API test files touched.
`ANALYSIS_VERSION` unchanged; nothing analysis-side reads the field yet.

**What prompted it**: "can we take a look at the next plan/phase for M9 and make a plan to get it
done?" — P6, agreed by ROADMAP's NEXT ACTION and the phase doc's first open box. Scope was
confirmed as **P6 alone**, and the club-taxonomy question P7 will need (does the picker derive its
list from a route, or inline it?) was explicitly deferred to P7 rather than pre-decided here.

**What landed.** `parse_club` finally has a caller, which is what P1 was for. `GET`/`POST
/api/sessions/current/club` are the cursor; `POST /api/uploads` reads both cursors in **one**
`load_session_meta` before it streams a byte and answers **409** when no club is selected;
`POST /api/sessions/{session_id}/swings/{swing_id}/club` is the repair route. The golfer routes were
the template and were followed literally — `ClubRequest` beside `GolferRequest`, `_resolve_club`
beside `_resolve_golfer`, `_safe()` on both path segments, and the club routes declared *above*
`/api/sessions/{session_id}` so the literal `current` is not swallowed.

**The one real decision the phase list did not contain: `ClubRequest.club` is a `str`.** Typed as
`ClubId`, pydantic rejects "7 iron" with a 422 before `parse_club` ever runs — and those tolerant
spellings are the entire reason that parser exists. So parsing happens in `_resolve_club`, which
keeps `contracts/club.py`'s claim to be the only place free text becomes a `ClubId` and keeps the
boundary's own 400 (R12). Getting this backwards would have looked like better typing and would have
made the bay retype "7 iron" as "7i" forever.

**`session_id` is now computed once and threaded down.** The handler used to read it *after* the
stream; the check and the write have to agree about which session they mean, or a large upload
spanning midnight checks one day's cursor and writes the swing into the next day's — a mistag with
no downstream symptom at all. The comment beside it says that, because a year from now it reads like
a pointless local variable.

**Two read routes gained `club` beyond the phase list.** `session_detail`'s per-swing rows and
`swing_detail` report it wherever they already report `player_id` — P5's design instruction ("the
club appears at every site `player_id` does and at no others") applied one layer up. Without it P7
would have a repair route it cannot show the current value for. `_club_value` is the single place
`ClubId | None` becomes JSON.

**Tests: 16 added, 23 → 39 in the file, suite 881 → 897.** They sit under their own divider
mirroring the golfer block, because the two asymmetries only read as deliberate side by side: the
club is required where the golfer is not, and it has no bulk backfill where the golfer has one.

**Fifteen existing tests across five files had to start selecting a club**, and the two styles were
chosen deliberately. In `tests/api/test_uploads.py` it is explicit at each call site via
`_pick_club`, so the tests that *do not* call it are visibly the refusal ones. In `test_results.py`,
`test_worker.py`, `test_career_route.py` and `test_conversation_routes.py` it is one line at the
construction point with a comment saying these tests are not about the club — burying it in a
fixture in the club's own test file would have hidden the thing under test.

**Three pins were watched fail rather than assumed**, the P3/P4/P5 habit, each mutated in-process
and restored. Moving the 409 to after the streaming block fails "a clubless upload writes nothing to
disk" — and the failure shows the orphaned `.part` left in `.incoming`, which is the whole point of
checking before the stream rather than after. Echoing the cursor instead of `manifest.club` in the
upload response fails the dedupe test, which is P5's `AssignmentResult.club` earning its place.
Rebuilding `SessionMeta` instead of going through `_update_cursor` fails "picking a club leaves the
golfer alone" — P4's bug, one layer up, and the reason that pin exists is that this repo has already
made that mistake once.

**Verified.** Full suite 897 green; ruff and mypy clean; `tests/api/test_pipeline_imports.py` green.
Checked on disk rather than in the model, as P4 and P5 did — against a real filesystem store in a
scratch root, not `data/processed/`, since the check writes swings: a clubless upload 409s and
leaves `.incoming` empty, "7 Iron" stores as `"club": "7i"` in both `session.json` and
`manifest.json`, moving the cursor to `pw` leaves swing 1 saying `7i` and leaves the golfer alone,
and the repair route rewrites swing 1 to `sw`.

**Known, deliberate, and the first thing the next session should hear: the upload page cannot upload
any more.** It has no club picker, so every file it sends is refused with the 409. That is the cost
of landing P6 without P7 and it is not a regression to hunt — P7 is a single HTML file and the
golfer bar is its literal template. ROADMAP says so in the NEXT ACTION block too.

**Docs touched beyond the phase.** The phase doc's status line and P6's as-built entry; ROADMAP's
status row, NEXT ACTION heading and the P5/P6 narrative. `docs/ARCHITECTURE.md` stays stale on
purpose for P20, as P4 and P5 both left it — §4 still calls `session.json` the "golfer cursor", its
manifest row names only `player_id`, and no route table knows about the three routes added here.
`tests/test_docs_truth.py` pins no route list, so nothing goes red in the meantime.

**Where it was left**: M9 is 6/20. Next is **P7** — the club picker on `api/static/index.html`:
canonical bag order, the current club shown prominently because it changes every few shots, the
upload control disabled while no club is selected, and the 409 surfaced if one slips through. The
open question it inherits is where the picker gets its 22 clubs from — a `GET /api/clubs` derived
from `ClubId`, or a list inlined in the JS. P8–P11 remain fully independent of the ingest spine.

---

## 2026-08-21 — M9 P5: the club gets a writer, and the branch that is only defensive

**Duration**: ~1 session. One storage module amended, one test file extended.
`ANALYSIS_VERSION` unchanged; nothing analysis-side reads the field yet.

**What prompted it**: "can we work on the next step for M9?" — P5, agreed by ROADMAP's NEXT
ACTION and the phase doc's first open box. Scope was confirmed as **P5 alone** rather than the
P5–P7 slice that would have reached the bay, and the measurement track (P8–P11) was left for its
own session.

**What landed.** `bundle_store` now writes the field P4 created. `assign_from_path(..., club=None)`
threads into `_new_manifest` and `_place`; `set_club(session_id, swing_id, club)` is the repair
route; there is no `attribute_unlabeled` analogue and the docstring says why. The club appears at
every site `player_id` does and at no others, which was the whole design instruction and is the
thing to check if this ever needs re-reading.

**The phase list did not name `AssignmentResult`, and it had to change.** It gained `club` for two
reasons that only surface once the dedupe path is written out. P6 builds the upload response
field-by-field off the result, so the field has to be there to be reported. And the deduped early
return needs somewhere to echo `manifest.club` — the club the swing *says*, not the one the retry
asked for. Without that distinction a phone retrying an upload after the cursor had moved on would
be told its stale club won, which is a wrong answer that looks exactly like a right one.

**The one place the club is genuinely not the golfer.** `_place`'s stamp-if-empty branch is the
entire two-phone fix for `player_id`: one phone uploads before a golfer is picked, the other after,
and the swing gets attributed on the second file rather than staying anonymous because of which
phone was faster. For the club that same branch is **defensive rather than load-bearing** — P6
refuses an upload with no club, so a swing created since M9 cannot reach `_place` untagged. What it
still covers is a swing written before the field existed receiving a later role, and tagging that
from the cursor current *now* is right, because now is when the swing is being completed. The
comment beside it says all of that, so the two branches are not read as equally important by
someone later deciding one of them is dead code.

**Tests: 10 added, 19 → 29 in the file, suite 871 → 881.** They mirror the golfer block below their
own divider, which is what makes the asymmetries legible — the club block has no
`attribute_unlabeled` mirror, and that absence is now itself a test.

**Three pins were watched fail rather than assumed**, the P3/P4 habit, run as one scripted pass
that mutates, runs the single named test, and restores. Dropping the write-once guard fails
"a tagged swing is never retagged by a later upload". Making the dedupe echo the *argument* instead
of the manifest fails the stored-club test. And rebuilding the manifest inside `set_club` instead of
mutating the loaded one fails "set_club leaves the golfer alone" — which is P4's `set_current_player`
bug one layer down, and the reason that pin exists at all is that this repo has already made that
mistake once. The cross-field pin is written in both directions, for P4's reason: each direction is
caught by a different test.

**Verified.** Full suite 881 green; ruff and mypy clean; `tests/api/test_pipeline_imports.py` green,
which is what would have caught `storage/` importing `contracts/club.py` the wrong way. Checked on
disk rather than in the model, as P4 did: a manifest written through the real `assign_from_path`
carries `"club": "7i"`, and `set_club` rewrites it to `"pw"`.

**Docs touched beyond the phase.** The phase doc's status line said "3/20, start at P4" while P4 was
already checked `[x]` in the same file — corrected, along with P5's as-built entry. ROADMAP's NEXT
ACTION and status table also still said P5, which would have sent the next session to redo finished
work; both now say P6. `docs/ARCHITECTURE.md` §4 stays stale on purpose, still P20's.

**Where it was left**: M9 is 5/20. Next is **P6** — the upload endpoint requiring a club: the
`GET`/`POST /api/sessions/current/club` cursor routes, a **409 before the body is streamed to disk**
when no club is selected, `club` added to the upload response (read it off `AssignmentResult`, it is
already there), and the per-swing repair route calling `set_club`. `parse_club` is the boundary
parser and P6 is where it gets its first caller. P8–P11 remain fully independent of the ingest
spine.

---

## 2026-08-21 — M9 P4: the club reaches two things that run, and the setter that replaced too much

**Duration**: ~1 session. Two storage modules amended, one test file split in two.
`ANALYSIS_VERSION` unchanged; no stored analysis re-scores — nothing analysis-side reads either
new field yet.

**What prompted it**: "can we work on the next step for M9? I think its P4 but please double
check" — it was P4, confirmed from three places that agreed: ROADMAP's NEXT ACTION, the phase
doc's first open box, and the previous entry's closing line.

**What landed.** `SwingManifest.club: ClubId | None` and `SessionMeta.club: ClubId | None`, plus
`set_current_club`. This is the first time anything from P1–P3 touches a module that already runs;
until today `contracts/club.py` had zero importers. Still **no writer** — P5 stamps the field, P6
makes it required at the boundary — so the storage layer can now record a club and nothing yet
does.

**The phase's real content was a latent bug, and the phase list flagged it in advance.**
`set_current_player` constructed a whole fresh `SessionMeta` on every call. That was correct while
`session.json` held one cursor and silently destructive the moment it held two: picking a golfer
would have wiped the club, on disk, mid-session, surfacing much later as a shot tagged with the
wrong club or none. Both setters now go through one private `_update_cursor` that loads, applies
only what it was asked to change, and saves.

**One helper rather than load-modify-save written out twice**, which was the one design decision
here. Carrying the sibling field by hand in each setter fixes it today and re-introduces it the
day a third cursor arrives and one setter forgets to carry it — the same class of bug, just
deferred to someone with less context. Putting preservation in one place means a new cursor
inherits it without asking. It merges onto a `model_dump` and re-runs `model_validate` rather than
`model_copy(update=...)`, for the reason P3 already established in `bag_store._write`:
`model_copy` skips validators. There are none on `SessionMeta` today; the point is that one added
later still runs.

**One `updated_at` for the whole record, and that is a limitation worth having written down.**
Nothing reads the field (grepped `src/` and `tests/` — no readers), and "when the session's
choices last changed" is a truthful reading of a single timestamp. But it cannot answer "when was
the club chosen", which is a question P19's bag page might well want. Said in the class docstring
so P19 finds it before shipping a wrong timestamp rather than after.

**The manifest field's description says something `player_id`'s does not.** For `player_id`,
`None` is an ordinary, repairable state — uploads are never blocked on it. For `club`, `None` can
only mean *predates the field*, because P6 refuses an untagged upload outright. Same shape, and
the asymmetry between them is the whole of ADR-024 §5, so the description states it rather than
leaving two identical-looking optional fields to be read as identical.

**Tests moved, not just added.** The five session-cursor tests were living in
`tests/storage/test_golfer_store.py`, which was fine while `session.json` was about golfers; it
stops being fine the moment club-cursor tests would land in a file named for the golfer store.
They moved to a new `tests/storage/test_session_meta.py`, which is where the mirror rule puts
them. 15 tests there (5 moved unchanged, 10 new) and 3 added to `test_manifest.py` — the
suite goes 858 → 871, which is +13 net and exactly accounts for the 5 that changed file.

**Both directions of the cross-cursor pin are written out separately, and it turned out to
matter.** Each setter was reverted to its replacing form in-process and the suite watched go red —
the P3 habit, and it paid: the two breaks were caught by *two different tests*. Breaking
`set_current_player` failed "setting the golfer preserves the club"; breaking `set_current_club`
failed the clearing pin and the on-disk one. A single direction would have missed a real break.

**Verified.** Full suite green; `tests/storage/` 150; `tests/api/test_pipeline_imports.py` green,
which is the pin that would have caught `storage/` reaching into `contracts/` the wrong way;
`tests/test_docs_truth.py` green; ruff and mypy clean. Also checked on disk rather than in the
model: both cursors written to a real `session.json`, a golfer swap leaving the club intact, and
the club surviving a manifest round trip as a `ClubId` and not a bare string.

**Left stale on purpose, for P20.** `docs/ARCHITECTURE.md` §4 still calls `session.json` the
"golfer cursor" (~line 400) and its manifest row (~397) names only `player_id`. Docs reconciliation
is P20 by the phase list; `tests/test_docs_truth.py` does not cover those tables, so nothing is red
in the meantime. Named here so P20 inherits a list instead of a search.

**Where it was left**: M9 is 4/20. Next is **P5** — `bundle_store` threading the club through
`assign_from_path` write-once and a `set_club` repair route, with **no `attribute_unlabeled`
analogue** (a session has many clubs, so reaching backwards would mislabel). P2 and P3 are still
uncommitted in the tree alongside P4, on the user's call. P8–P11, the measurement track, remain
fully independent of the ingest spine.

---

## 2026-08-21 — M9 P3: the bag on disk, and the shelf that turned one phase into a decision

**Duration**: ~1 session. One new storage module, one amended contract, both still with no
consumers. `ANALYSIS_VERSION` unchanged; no stored analysis re-scores.

**What prompted it**: "can we work on the next thing" — ROADMAP and the previous entry both named
M9 P3, still the ingest half's next link and still needing neither a bay session nor more `n`.

**What landed.** `storage/bag_store.py`: `BagStore` with `get`, `save`, `set_entry`,
`remove_entry`, `restore_entry`, mirroring `GolferStore`'s class shape and `save_manifest`'s atomic
write. `contracts/bag.py` gained `retired`, `retired_at`, `same_club_as` and `retired_for`.

**The bag lives beside the golfer**, at `data/processed/golfers/<player_id>.bag.json`, on the
user's call — reusing `settings.golfers_dir` rather than adding a `bags_dir`. The `.golfer.json`
suffix already existed so that directory could hold more than one record kind per player, and
`GolferStore.list_all` globs `*.golfer.json` while this store globs nothing, so neither sees the
other's files. A separate top-level directory would have named the same thing twice. There is a
test that writes both and asserts each store sees only its own, because that is the half of this
decision that can only fail on disk.

**The phase gained a decision the phase list did not anticipate, and it came from the user:
nothing deletes a club.** The spec said `set_entry` upserts and `remove_entry` removes. Asked
which way `recorded_at` should move on a re-save, the answer was to keep every club ever entered
and only replace the one actively in the bag — so a golfer who goes back to last year's 7 iron
still has its loft. That is right, and it is right for the reason the whole milestone exists:
"loft is unrecoverable after the fact" was the argument for recording it before any model uses it,
and overwriting the entry on replacement re-creates exactly that loss, one club at a time, through
normal use of the feature.

So `Bag.retired` is an append-only shelf of finished stints and `BagEntry.retired_at` says when
each one ended. `restore_entry` **copies** off the shelf and leaves the stint in place — out, back
and out again is three stints, not one overwritten record. A pop would make a club's second
departure look like its first.

**Retention is not the versioning ADR-024 defers, and the difference needed writing down.** The
ADR's *Deferred, by choice* defers *bag entry versioning* — attributing each stored shot to the
stint that hit it. That stays deferred. Nothing reads `Bag.retired`; P16 will still caveat from the
current entry's `recorded_at`, naming a date rather than modelling a history. The shelf makes the
modelling possible later without promising it now. The addendum states the staleness test in one
line: **if anything joins a shot to a member of `Bag.retired`, versioning has happened** and it
needs a decision rather than an addendum.

**Re-saving an unchanged entry writes nothing.** Identity is `BagEntry.same_club_as`, which
compares every descriptive field and neither timestamp — derived by *excluding* the two timestamps
rather than listing the six fields, so a field added to `BagEntry` later is compared from the day
it is added. Without the short-circuit, P19's save button on an unedited row retires a club and
hands P16 a bag-changed caveat over shots all hit with the same club: a false positive produced by
the UI working correctly, which is the worst kind to debug.

**The guard the shelf made necessary.** `get` is tolerant — corrupt reads as `None`, as
`GolferStore.get` does. The three mutators are not: they read through `_load_for_write`, which
tells "no bag yet" from "bag I cannot parse" and raises on the second. A writer treating an
unreadable file as an empty bag replaces the bag *and its entire shelf* with the one club it was
asked to set. That failure existed before the shelf and got materially worse with it. All three
mutators are parametrized over the same test, because they share one reader and a fourth added
later that reached for `get` would reintroduce it.

**`_write` constructs a `Bag` rather than `model_copy`-ing one**, deliberately: `model_copy`
skips validators, and both of `Bag`'s guard invariants that these three methods are the only thing
maintaining. Building the model is what makes a mistake in that file fail there instead of on the
next read.

**Verified.** 21 new store tests and 7 added to the contract's (13 total there), full suite green
at 858, ruff and mypy clean.
The three load-bearing pins were checked by **removing the behaviour in-process and watching them
fail** rather than assumed: the clobber guard rewired to the tolerant reader, `same_club_as` forced
to `False`, and `restore_entry` reimplemented to pop. All three fired. `tests/test_docs_truth.py`
caught the stale addendum count in `docs/README.md` on the first run, which is the doc-truth test
doing precisely its job.

**Escalation note.** P3 is an L2 phase that had to touch a `contracts/` shape, which CLAUDE.md
routes to L3. Taken anyway rather than deferred: `Bag` and `BagEntry` have zero importers and P2
was still uncommitted, so the usual L3 cost — a consumer nobody read — was nil. It will never be
cheaper than it was today.

**Where it was left**: M9 is 3/20. Next is **P4** (`club` on `SwingManifest` and the session
cursor) — the first phase where the club reaches something that already runs, and the one carrying
the `set_current_player` load-modify-save fix. P8–P11, the measurement track, remain fully
independent of the ingest spine and can go in any session.

---

## 2026-08-21 — M9 P2: the bag entry, and the two guards the phase list did not ask for

**Duration**: ~1 session, one new contracts module with no consumers. `ANALYSIS_VERSION`
unchanged; no stored analysis re-scores.

**What prompted it**: "implement the next thing" — ROADMAP named M9 P2 and it is still the ingest
half's next link, needing neither a bay session nor more `n`.

**What landed.** `contracts/bag.py`: `BagEntry` (`club`, `loft_deg`, `make`, `model`, `shaft`,
`length_in`, `recorded_at`) and `Bag` (`player_id`, `entries` keyed by `ClubId`, `updated_at`,
plus a `club_ids` property). Pure contract — P3 puts it on disk, P14 composes the entry into a
club profile, P16 reads `recorded_at`. Nothing imports it yet, and that is the deliverable.

**`recorded_at` is required, with no default, and that is a decision.** `Golfer.created_at` is the
precedent: the contract stays dumb and `GolferStore` stamps it. P3's `set_entry` will re-stamp on
upsert the same way. A `default_factory` reading the clock was the alternative and was rejected —
it would put a second stamping site in a different layer from the first, which is exactly the
drift `recorded_at` exists to make visible.

**Two guards the phase list did not specify, both added because the failure is silent.** The phase
detail listed fields and a property; what it did not name is that `entries` is keyed by `ClubId`
*and* `BagEntry` carries `club`, so the two can disagree. They both have to exist — an entry
travels alone through `set_entry` and `ClubProfile.bag_entry` — so a `model_validator` rejects a
mismatch rather than picking a winner. Filing a sand wedge under `7i` attaches the wrong loft to a
club's history and nothing downstream can detect it. Same posture as `_build_aliases`' collision
raise from P1.

The second is a `field_validator` on `player_id` against `contracts/golfer.py:PLAYER_ID`, copied
from `Golfer`. P3's spec says the id is "already slug-validated, so no second sanitising step" and
then reads it straight into `<root>/<player_id>.json` — that sentence was only true of `Golfer`
until now.

**Defaults on the descriptive fields.** `make`/`model`/`shaft` default to `""` and both float
fields to `None`, so the only required inputs are the club and the timestamp. A golfer who has
never had their lofts measured still has a bag; the work that needs loft refuses per club, which
is `SwingResult.unscored`'s posture applied to a different missing input.

**The `club_ids` ordering pin needed a club set where all three orders differ**, or it pins
nothing. pw / 3w / driver / 7i does it: inserted in that order, alphabetical by value is
`3w, 7i, driver, pw`, and only bag order puts the driver first and the wedge last. The test
asserts against all three.

**Verified.** 6 new tests, full suite green (830), ruff and mypy clean. All four pins checked by
violating them in-process rather than assumed — including the club/key mismatch arriving as raw
JSON with string keys, since pydantic coerces the key before the validator sees it and that is the
path `BagStore` will actually take.

**Where it was left**: M9 is 2/20. Next is **P3** (`storage/bag_store.py`), mirroring
`storage/golfer_store.py`. P8-P11, the measurement track, remain fully independent of the ingest
spine and can go in any session.

---

## 2026-08-21 — M9 P1: the club vocabulary, and three sessions squashed into one commit

**Duration**: ~1 session. One commit of accumulated work, then one new contracts module with no
consumers. `ANALYSIS_VERSION` unchanged; no stored analysis re-scores.

**What prompted it**: "what's the next thing / small phase" — ROADMAP named M9 P1 and it is still
the only substantial item needing neither a bay session nor more `n`.

**Part 1 — the tree was three sessions deep and green.** The `unscored` change (2026-08-19), the
tempo trainer built on top of it (2026-08-20), and the M9 design docs (2026-08-20) were all
uncommitted together: 41 files, suite green, ruff and mypy clean. Squashed into one commit at the
user's call rather than split, with the message naming all three halves and the dependency between
the first two — Part 2 of the tempo work reads `tempo_timings`' three refusal paths, which the
`unscored` change introduced, so they only make sense in that order.

**Part 2 — `contracts/club.py`, 22 clubs and two functions.** `ClubId` in canonical bag order
(driver, woods, hybrids, 1i-9i, wedges, putter), `CLUB_CATEGORY` mapping each onto the existing
`ClubCategory` from `intent.py`, `category_of`, and `parse_club` as the tolerant boundary parser.
Nothing imports it. That is the deliverable — P1 is the vocabulary, not a feature.

**The alias table is derived from the enum, and the collision guard is the reason that is safe.**
`parse_club` accepts three spellings per club and all three are read off the declaration: the
value (`7i`), the member name folded (`seveniron`, which is how "seven iron" normalises), and for
numeric clubs the digit with its suffix expanded (`7iron`). The member names are spelled out only
because `3w` is not a valid Python identifier — but that constraint is what makes the second form
free. A hand-listed alias table's failure mode is a club added later that silently parses as
`None` at the bay; this one cannot have that. `_build_aliases` raises on any spelling claimed by
two clubs, verified by making `3h` derive `3wood` and watching it fire.

**No bare-number aliases, and "wedge" does not parse.** "7" is a 7 iron or a 7 wood and nothing
here can tell which; "wedge" and "iron" name a category rather than a club. Both return `None`
rather than a nearest match, because the costs are asymmetric — a refused tag costs one retype at
the bay, a wrong one pools a wedge's carries into a 7 iron's average where nothing downstream can
detect it.

**`CLUB_CATEGORY` is written out rather than derived from the digit, deliberately.** 1i-4i long /
5i-7i mid / 8i-9i short is a convention, not arithmetic. Burying it in a range comparison makes it
look like a fact; a table can be read and argued with. The exhaustiveness test is what lets
`category_of` index it bare, same posture as `UnscoredCheckpoint.spec`.

**The trap was not sprung.** `PracticeGoal.club` stays `ClubCategory.ALL` and nothing was wired
into `resolve_range`. `ranges.json` holds club-agnostic rows only, so handing it a real category
would resolve no band for every checkpoint and take the fundamentals panel dark. `category_of`
exists for when per-club bands exist, not now.

**Verified.** 9 new tests, full suite green, ruff and mypy clean. All three of the new pins were
checked by violating them in-process — deleting a `CLUB_CATEGORY` row, pointing a club at
`ClubCategory.ALL`, and forcing an alias collision — rather than assumed to bite.

**Where it was left**: M9 is 1/20. Next is **P2** (`contracts/bag.py`), then P3 (`bag_store`).
P8-P11, the measurement track, remain fully independent of the ingest spine and can go in any
session.

---

## 2026-08-20 — M9 planned: the club tag, and why distance was never measurable

**Duration**: ~1 session, **design only — no code**. New ADR-024, new
[docs/M9_PLAYER_TRACKING.md](docs/M9_PLAYER_TRACKING.md) (20 phases), ROADMAP updated. Nothing
under `src/` was touched.
**What prompted it**: the ask was "player tracking" — clubs used, average distance per club,
relative dispersion, and eventually club fitting off swing speed and launch. Exploring for it
turned up that almost all of it is already built.

**The finding that shaped the whole plan: career mode already does this, minus one field.** The
corpus reader, the honest `n`, the confidence intervals, the minimum-`n` guard, the bias/scatter
discriminator — all built, all validated, all silent. And `storage.corpus.narrow_to` already
filters a corpus *and recomputes `metric_counts`*, which exists so a filtered swing list can never
sit beside an `n` describing a different set. **Adding `club=` to that one function makes the
entire career pipeline produce per-club answers with nothing new learning the rules.** So M9 is
mostly wiring, and the estimate should be read that way.

**Why carry distance was never a measurement, which I had assumed was an oversight.** It is not —
it is that a carry pooled across clubs is not a noisy estimate of anything, it is an average of two
different questions. `shot_measure.py` records `face_to_path_deg` and `start_line_deg` and stops.
**The club tag is what makes distance poolable at all**, which is why carry arrives with M9 rather
than with M6.5, and that reason belongs in the module docstring when P8 lands.

**"Average yards left or right" is not measurable and I nearly planned it as if it were.** Checked
`launch_monitor/screen/profiles.json` field by field: the HD Golf screen prints Shot Distance,
Carry, Bounce & Roll, Ball Speed, Launch Angle, Club Speed, Club Path, Club Face Angle, Spin,
Smash Factor, Impact Position, Shot Type, Horizontal Angle, Spin Axis. **No offline tile, no
landing coordinate, and no club tile either.** So lateral miss ships as
`start_line_offline_yds = carry * sin(start_line)` — exact trigonometry, where the ball *would*
have landed if it never curved — and the docstring has to say *started*, never *finished*. A real
flight model is deferred on `spin_axis`, which ADR-014's addendum already caught storing two fades
as draws.

**Two traps found while planning, both written into the ADR because both look like oversights.**
`PracticeGoal.club` already exists and threads into `resolve_range`, so wiring the new tag into it
looks like the obvious payoff — but `ranges.json` holds **6 rows, all `club_category: "all"`**, so
a real category makes every checkpoint resolve no band and the fundamentals panel goes dark. And
club must *not* get an `attribute_unlabeled` equivalent: reaching backwards over a session's
untagged swings is safe for golfer (usually one per session) and destructive for club (many).

**The one place I overrode the stated preference, and why.** The ask was "club required at
upload". Taken literally that means the client sends it — but `api/app.py` reads `player_id` from
a *server-side session cursor* with the comment *"both phones post into the same swing, and only
one of them is being held by someone who knows whose swing it is."* Two phones with two
`localStorage` club values would disagree and the disagreement would land in the manifest. So
requiredness is enforced as **409 on `/api/uploads` when the club cursor is unset** — genuinely
blocking, one copy of the value.

**Where it was left**: nothing built. Start at **M9 P1** (`contracts/club.py`). P1–P7 are the
ingest spine and run in order; **P8–P11 are the measurements and are fully independent of them**,
so the two tracks can go in parallel or to separate sessions. P10 is skippable.

**Expect refusals throughout.** Every swing on disk is untagged and the guard needs 5 shots per
club. The correct output at every stage is a refusal with a correct `n`; a number appearing early
is the bug. Same acceptance criterion career mode shipped under.

**Note the working tree** still carries the uncommitted tempo/`unscored` work from the previous two
sessions. This entry's changes are docs-only and touch none of it.

---

## 2026-08-20 — the tempo trainer, and the mph axis the corpus refused

**Duration**: ~1 session. Implements the tempo plan end to end. New ADR-023, two new modules,
`ANALYSIS_VERSION` 6 -> 7.
**What prompted it**: picking up a plan a previous session wrote and did not start. Note the
working tree still carries that session's **`unscored` change, uncommitted** — this is built on top
of it (Part 2 relies on `tempo_timings`' three refusal paths, which that change introduced), so the
two want committing in order.

**The ask was a tempo indexed by club speed, and the corpus said no.** Over the 310 non-slow-motion
GolfDB clips with a real frame rate, between-club sd of downswing duration is **6.9 ms** — a fifth
of one frame — against **47.0 ms** between golfers. Tour downswing duration does not move with club
across the widest speed range in golf. That reproduces ADR-010's 2026-08-18 ratio finding on the
*absolute* durations, which is stronger: the ratio could have held while both halves scaled, and it
does not. Add that `shot_measure.py` already refuses `club_head_speed` because every stored shot
reads a smash factor under 1.0, and an mph axis would have been a fabricated relationship keyed off
a known-bad number. A **pace** scale ships instead: multiplies both halves, moves no ratio.

**Two beat patterns, and that is the design rather than a feature.** No single pulse marks both the
top and impact — the intervals differ by 3.4x, so 67 BPM cannot mark impact and 225 BPM is not a
groove. `GRID` snaps the backswing to whole downswing-length ticks (steady, loopable, ratio
rounded); `CUES` plays three tones at the exact medians (true ratio, ~900 ms of silence). They carry
different durations, which is why those live on `BeatPattern` and not on `TempoPlan` — one `ratio`
field beside two patterns would be false for one of them, and plausibly so.

**The surprise in Part 1: every duration-eligible clip is down-the-line.** The plan predicted the
sample exactly (n=310, 168 golfers, p50 901 / 267 ms) but not its composition. The face-on half of
the pose cache predates the `clip` envelope, so no face-on clip has a recoverable frame rate, and
the source videos are not kept. It does not compromise the numbers — a duration comes from
ground-truth event labels and the clip's fps and reads no landmark — but the sample is one view, so
`Distribution` gained a per-row `provenance` and those two rows state it. The `dataset` block now
points at that field rather than restating the exception.

**Two landmines, and only one of them was the pin's doing.** `test_dispersion` failed honestly —
a metric in `measure.py` and nowhere else. The other was silent: **`derive_pose_metrics.py` would
have written frame counts into `swings.jsonl` under a millisecond name.** It measures every
registered metric at *labelled* instants, and `_phases_from_events` puts frame indices in
`start_ms` by design, since ~47% of the corpus is slow-motion and only ratios were ever read from
them. Those frame counts would have sat beside the real milliseconds `derive_reference.py` computes
and every duration band would have been cut from the mixture. Nothing in `tests/` covers the
research scripts. `tune_spatial_metric.py` had the same bug in the other direction — it printed
`backswing_ms` at ratio 0.1, **"MOSTLY NOISE"**, which reads exactly like a finding and is frames
minus milliseconds. Both now refuse the set, and the set is
`measure.FPS_DEPENDENT_MEASUREMENTS`, derived from the metric's own unit rather than listed.

**The two tolerances are composed, not tuned, and that is stated.** The harness cannot measure them,
so they come from the M4-REF instant errors — a duration is a subtraction of two instants, so its
error is theirs summed: backswing (2+7) frames = 300 ms, downswing (1+2) = 100 ms. Address dominates
the first exactly as it dominates `tempo_ratio`'s tuned 0.943, and 300 ms on a 267 ms downswing is
~1.1 of ratio — the same size, arrived at independently. That agreement is the only cross-check
available.

**Verified.** 803 tests green, ruff and mypy clean. `derive_reference.py` re-run: every existing
distribution byte-identical, only the provenance stamps and the two new metrics moved. The page's
728 lines of JS parse under `node --check`, and its gating was driven in node — renders only when
tempo failed, hidden when it passed, was unscored, or no plan came; the verdict prose is escaped.

**`overall_score` was proven unmoved rather than assumed.** Ran the engine twice over the same
stored keypoints, once with the two metrics registered and once without: `overall_score`,
`mechanics_score`, `checkpoint_scores` and `unscored` all byte-identical, the only difference being
the two new measurements. Done in-process — `reanalyze.py --all` would have overwritten four real
untracked artifacts to prove less.

**Then the premise was challenged, and it was right.** *"The tempo of a 70 mph swing and a 100
mph swing are going to vary."* They do. The club test above answers a different question: a longer
club raises head speed by **lengthening the lever**, not by rotating faster, so duration holds —
that is the 6.9 ms. A golfer who swings harder rotates faster, which compresses the swing, and the
club test could not see it. GolfDB carries a real speed cohort in `sex`: LPGA against PGA, driver
only, one vote per golfer — backswing **1001 ms against 834**, downswing **267 against 234**. About
five times the club effect. ADR-023 now carries an addendum saying so, and the Status line points at
it, because the Context section as written would send the next reader the wrong way.

**The fix is not an mph axis, it is an anchor.** `build_tempo_plan` now targets the golfer's own
measured backswing and derives the downswing from it at the tour ratio. A slower golfer's longer
backswing *is* the speed signal, measured rather than inferred — which matters because there is
still no usable club-head speed to key on. The ratio stays the tour's: it is the anchor's length
that follows the golfer, never the shape, since the ratio is what the checkpoint judges.
**Guarded** by the corpus p10-p90, so a backswing that is itself the fault is not rehearsed. With
no usable backswing it collapses onto exactly what shipped this morning, and that is pinned.

**Then: can the golfer still use the slider?** Yes, and it now opens where the fit put it. The
anchoring was moved out of the beats and onto `TempoPlan.pace`, so the patterns are always the tour
reference and one place applies a pace — the renderer. A slow swinger's control opens at **111%**, a
quick one's at **83%**, unanchored at 100%, and dragging is a plain override. Baked into the beats
it read 100% for everyone, showing nothing of the decision it overrides. This also removed a latent
double-application: `build_tempo_plan` took a `pace` and pre-multiplied *while* the page multiplied
by its slider. Nothing ever passed one, so it never bit; the parameter is gone rather than
documented. The slider's bounds are pinned against the guard's own range, read out of
`results.html`, so widening the guard without widening the control fails a test.

**A pre-existing display bug fell out of testing it.** `num()` stripped trailing zeros with
`.replace(/\.?0+$/, "")`, which anchors on any run of trailing zeros rather than ones after a
decimal point — so it ate them out of whole numbers too. **A score of 90 rendered as "9", a clamped
percentile of 10 as "1", 890 ms as "89", and exactly 0 as the empty string.** Every result is a
plausible number, which is why it survived on a page that has shipped for milestones. Fixed, and
pinned through node, having first confirmed the pin fails on the old regex.

**One self-inflicted scare worth recording.** I ran `git checkout` on `results.html` to undo a
deliberately-broken formatter and reverted the *whole file*, discarding the tempo block with it —
the file had a session's uncommitted work in it and HEAD did not. Rebuilt from the same content and
re-verified. **Do not `git checkout` a file in this tree while the `unscored` and tempo work is
uncommitted**; there is no second copy.

**PICK UP HERE.** Two things. First, **the stored swings are still version 6** — nothing on disk was
rewritten, so `scripts/reanalyze.py --all` is what gives the four of them their durations, and it is
a deliberate uncommitted-work decision rather than an oversight. Second, the deferral worth having:
**which half is off.** On `2026-08-10/2` the backswing is 901 ms — *exactly* the tour median — and
the downswing is 384 ms, 28% past the p90 edge. Tempo fails there because the downswing is slow,
which is the opposite of what its own headline, "Tempo too quick", sounds like it means. Diagnosing
that needs a `PersonalBaseline` over the two new measurements, so it is gated on `n` like everything
else on this board.

---

## 2026-08-19 — `unscored` says why, and two consumers stop guessing

**Duration**: ~1 session. One new contracts module, one ADR-010 addendum, no version bump.
**What prompted it**: "what's the next thing to work on?" — the board is all bay-or-`n` now, and
M5-FB's deferred list still held one box that needed neither.

**The box was "reasons, not just names, on `unscored`", and the interesting part is who was
already answering it.** `SwingResult.unscored` was `list[str]`. Its own field description admitted
the gap. But a golfer still had to be told *something*, so two places downstream reconstructed the
cause and **neither was the source**: `feedback/rules.py` held a `_UNSCORED_REMEDY` table keyed on
checkpoint name and decided between "film it again" and "pick a golfer" by checking whether
`head_hip_gain_norm` had survived into `measurements` — if the number was there the frames must
have been readable. `api/pipeline.py` narrated the same case again as free text.

**That heuristic was right and unextendable, which is a worse state than wrong.** It inferred the
failure from a side effect rather than from the failure, so it worked for the one checkpoint
someone wrote a row for and could never cover a second. A `NO_BAND` — swing measured fine, repo has
no benchmark — would have been told to steady the camera. Nobody would have noticed, because the
advice is plausible.

**The causes were always enumerable: 13 `return None` sites, six conditions.** Every one is a
helper in `measure.py` failing, a `resolve_range` miss, or a missing `handedness`. Each knows
exactly which, and has the window and landmark group still in hand — all of which is gone by the
time a `None` reaches `feedback`. New `contracts/unscored.py` names them
(`PHASE_NOT_SEGMENTED`, `BOUNDARY_ESTIMATED`, `TIMING_DEGENERATE`, `LANDMARKS_UNCONFIDENT`,
`TOO_FEW_FRAMES`, `SCALE_UNAVAILABLE`, `NO_BAND`, `NO_HANDEDNESS`) with one `ReasonSpec` row each.
`measure_*` returns `MeasureOutcome`, evaluators return `CheckpointOutcome`.

**The load-bearing field is `refilming_helps`, not the reason.** It is the bit every consumer needs
and none could derive, and it is exactly what the old table was inferring. `contracts/caveats.py`
derives its warning from it rather than listing causes in prose, so a reason added later cannot
leave the coaching models reading a stale list — two `test_docs_truth.py` pins hold both directions
of that, because a capture problem listed as "not a capture problem" fails silently.

**Where the vocabulary lives is the whole reason it works.** `feedback` may not import `analysis`
(ADR-008) — that is what forced `rules.py` to retype two checkpoint names as literals with a
comment apologising for it. Putting the vocabulary in `contracts/` removes the need rather than
working around it. Same argument `caveats.py` and `dispersion.py::METRIC_TARGETS` already make.

**One split kept deliberately: `MEASUREMENT_REASONS`.** `measure.py` may report only the six
measurement causes; `NO_BAND` and `NO_HANDEDNESS` cannot come out of it. That is M6.5's
measure/judge separation made *checkable* — a `NO_BAND` escaping into `measure.py` would mean the
measuring layer had started consulting bands again, which is the fusion that kept the panel at
three checkpoints. Pinned in `tests/analysis/test_measure.py`, and the two sets are asserted to
partition the enum so neither pin can go vacuous.

**`ANALYSIS_VERSION` deliberately did not move.** Every number and verdict is where it was; what
changed is what a refusal says about itself. Its own rule is "not for anything that leaves every
number where it was".

**Two things nearly went wrong, both caught by a tool rather than by reading.**
`scripts/golfdb/derive_pose_metrics.py` and `tune_spatial_metric.py` iterate `POSE_MEASUREMENTS`
and did `if value is None: continue` — and a `MeasureOutcome` is never `None`, so both would have
stored the *tuple* as the metric and taken every band derived from that file with it. Nothing in
`tests/` covers the research scripts. Second: mypy caught `outcome` colliding with the outcome
*axis* list in `engine.py`, which is a name this repo uses for something else entirely.

**Back-compat, and the one unacceptable option.** Old artifacts store `unscored` as bare names.
`SwingResult` coerces those to `UNRECORDED` in one `field_validator`; `mcp/query.py` does the same
for the raw JSON it reads without building the model, and also swallows an unrecognised reason from
a *newer* build. Dropping the entries was never on: a swing judged on five fundamentals would start
reading as one judged on six. The engine never writes `UNRECORDED` — pinned, so "this file does not
know" cannot decay into "we did not bother to say". All four stored swings happen to have empty
`unscored`, so nothing on disk changed.

**Verified.** 779 tests green, ruff and mypy clean. Driven in-process over the real
`2026-08-10/2` face-on keypoints rather than through `analyze_bundle.py`, because
`data/processed/` is untracked and re-running the CLI would overwrite a real artifact and could
fire a paid coaching call. Unattributed, that swing reports `head_stays_back` / `no_handedness`,
`refilming_helps=False`, and the tip says pick a golfer *and will score without re-filming* —
where the old code got there by a guess that happened to be right.

**PICK UP HERE.** M5-FB has one box left and it is not desk work: **per-club percentiles**, which
needs the band and the percentile to move together (ADR-010 addendum) and is gated behind the same
`ResolvedRange` contract change the 2026-08-18 entry describes. Everything else on the board still
splits the way ROADMAP's NEXT ACTION says — bay session, or `n`.

If someone picks up the UI thread instead: `results.html` renders the reason token and puts
`detail` in a `title`, deliberately **not** the remedy prose — that is written once on the server
and arrives in the Tips list. Adding it to the page would be the second copy this change removed.

`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` remains **flaky** —
threading test, 5s timeout, fails under load and passes in isolation. Unrelated; re-run before
believing it.

---

## 2026-08-18 — Per-club bands, gated and declined; M8's checklist is empty

**Duration**: ~1 session. One new gate script, one ADR addendum, no code, no version bump.
**What prompted it**: M8's last open box, "per-club bands are now costed".

**Costed was not gated, and that is the whole session.** The box carried counts — face-on driver
341 clips, iron 69, fairway 32 clearing `MIN_SAMPLES` — which answer *could we cut a per-club
band?* and say nothing about *would it differ from the one already shipped?*
`scripts/golfdb/tune_per_club_bands.py` asks the second question: screen each per-club p90 against
the all-club p90 in units of the metric's own error at the 2.0x bar, then put a player-clustered
bootstrap on anything that clears. **Two of nineteen strata survive, and they do not make a panel**
— `head_sway_norm` x iron at 2.85x and `finish_balance_norm` x fairway at 2.56x, different metrics
on different clubs.

**Driver never differs from the all-club band on any metric** (0.17–0.56x). Not a fact about
drivers: driver is 341 of the 458 face-on clips, so the all-club band very largely *is* the driver
band, and a driver row would restate a shipped one. Worth knowing before anyone costs this again.

**The two survivors say one physical thing, so the honest test was the pooled one.** A shorter club
is swung with less movement — so pool every non-driver club onto 42 golfers instead of 14 or 27.
Exactly one effect survives: head sway, p90 **0.2614** against all-club **0.4311**, worst-case
**2.20x**. `finish_balance_norm` drops to 1.49x. The real result is narrower than the framing the
box used: **head sway is lower with non-driver clubs, and nothing else in the panel depends on club
at all.** There is no `ClubCategory` for "not a driver", which is where that finding stops.

**Tempo was the interesting one, and the axis was wrong rather than the band.** It screens out at
every club (0.17–0.55x) while being the metric anyone would expect to track club length — so the
variance got decomposed both ways over the 1,399 labelled swings. Between **golfers**, sd of means
**0.7618**. Between **clubs**, **0.1626**. The golfer effect is **4.7x** the club effect, and the
club effect is **5.8x smaller than our own tempo error** of 0.943. A per-club tempo band would key
on a quantity five times below the noise of the measurement it gets applied to. Tempo is a personal
signature and its home is `PersonalBaseline`, which is built and waiting on `n` — not a row in
`ranges.json`. *(This was Aaron's call before it was measured; the decomposition is what turned it
from an instinct into a number.)*

**A structural find that outlives the decision.** `ClubCategory` is
`driver, wood, hybrid, long_iron, mid_iron, short_iron, wedge, putter, all`; the corpus labels
clubs `driver, iron, fairway, wedge, hybrid`. `contracts/reference.py` has always said the mapping
is "the consumer's" and **nothing has ever had to write it**. It is not writable from this corpus:
three `ClubCategory` iron members would have to be seeded from one undifferentiated stratum, and
`resolve_range` matches `club_category` exactly. That is a blocker no amount of extra data fixes.

**The plumbing, checked and found complete.** `analyze_bundle.py --club` →
`PipelineOptions.club` → `PracticeGoal(club=...)` → every evaluator → `resolve_range`. It works
end to end today and does nothing, because no per-club row exists. What is missing is not wiring
but a *recorded* club: it is a manual flag, `Shot` carries speed/face/path and no club identity, and
all four stored swings read `club: all`.

**One wrong turn worth recording.** The first screen ran on three metrics because I took their
error floors out of `ranges.json` provenance prose, where only three are quoted, and reported
"five of six fail" on a table with two blanks in it. All six floors are in **one place** —
`contracts/dispersion.py::METRIC_TARGETS` — and reading them there flipped the result: `head_sway`
and `finish_balance` clear the screen on iron and fairway. The gate script imports that mapping
rather than restating any of it. **The floors live in `contracts`, not in band prose.**

**Verified.** Full suite green, ruff and mypy clean. The gate re-runs in about a minute on a base
install — stdlib only, no `research` extra — and is committed rather than concluded, the same
reason `tune_arm_parallel.py` stayed in the tree after killing two candidate checkpoints.

**PICK UP HERE. M8's checklist is empty** — both boxes closed today, neither by writing a band.
Nothing on the board is desk work any more, and the split is the one ROADMAP's NEXT ACTION already
describes:

- **Wants a bay session**: §Phase F's trail-wrist result unproven at n=2, M3's remaining OCR work,
  M2's lighting/shutter test. `docs/BAY_SESSION_RUNBOOK.md` sequences it.
- **Wants `n`**: career mode and every surface on it stays silent until `n` crosses 5 — read
  [ADR-022's fourth addendum](docs/decisions/022-learned-artifacts-as-committed-data.md) *before*
  analysing that session.

If per-club ever reopens, the one candidate with evidence is pooled non-driver head sway, and the
work is `_population_placement` drawing its percentile from the stratum the band resolved to —
`ResolvedRange` carries `(low, high, source)` and not which stratum that was, so it is a contract
change rather than a data edit.

`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` remains **flaky** —
threading test, 5s timeout, fails under load and passes in isolation. Unrelated; re-run before
believing it.

---

## 2026-08-18 — A band edge kept, and the sentence that made it look like a band change

**Duration**: ~1 session. One provenance rewrite, one ADR addendum, one pin, two docstrings. No
code, no version bump.
**What prompted it**: "what's the next thing to work on?" — and M8's checklist had exactly one box
left that needed neither a bay session nor a larger `n`.

**`hip_sway_norm`'s lower edge stays at 0.14, and the interesting part is why it looked like it
shouldn't.** The box was opened by M8's player-clustered bootstrap: 458 GolfDB clips come from 122
golfers, several often cut from one broadcast of one swing, so resampling *players* rather than
clips widens p10's 95% interval from a point estimate of 0.1414 down to **0.0801**. Both ROADMAP
and the last two WORKLOG entries recorded that as "1.6× the 0.050 error floor rather than the 2.8×
ADR-010 justified it on" — and that sentence is what made this read as a band change.

**It mixed two measurements that answer different questions.** 2.8× is a claim about *resolution*:
can the pipeline tell 0.14 from zero? It can, and **clustering does not enter that calculation at
all** — it is a property of the instrument, not of the sample. The bootstrap is a claim about
*placement*: is the tour p10 actually at 0.14? Best estimate yes, honestly somewhere at or above
0.08. Only the second degraded. 1.6× is a real quantity — where an edge placed at the interval's
floor would sit against the noise — but it does not describe the edge that ships. Reading it as
though it had superseded the 2.8× turns a placement result into an apparent resolution failure, and
that is the whole of what happened here.

**So `hip_shift_at_top_norm` is not the precedent it resembles.** Its edge was dropped because its
p10 of 0.015 sits *below* its 0.053 floor — genuinely unmeasurable, separating golfers this
pipeline cannot tell apart. Nothing about `hip_sway_norm` is in that condition. Two rows looking
alike while resting on different arguments is exactly the trap the 2026-08-12 addendum was written
about; it just arrived from the other direction this time.

**A second reason, found in the code rather than the statistics.** `one_sided` feeds
`contracts/caveats.py`'s bullet telling every coaching model that *"on the one-sided checkpoints …
below the band is the good side"*. Dropping the edge would file `hip_sway` under that sentence and
assert the one thing this ADR has now twice refused to assert — that less lateral hip travel is
better. The flag is not just a band shape; it is a claim about which direction is good, and only
one of the three options on the table kept it true.

**The deliverable is auditability, not behaviour.** Nothing executable moved. `ranges.json`'s row
now carries the clustered interval *beside* the 2.8× instead of stating the confident half alone,
and the new [ADR-010 addendum](docs/decisions/010-benchmark-ranges.md) records the split, the
retention, and what would reopen it — bay footage measuring near the edge, or a re-derivation over
more *distinct golfers*, which is the quantity that flattered the original interval, not more clips.

**The pin is on the reasoning, because there is no behaviour to pin.**
`test_the_hip_sway_lower_edge_carries_its_clustered_interval` asserts the row's provenance still
names 0.0801. That knowingly breaks `test_benchmarks.py`'s own rule against literals, and the
docstring says so: the rule exists to keep re-sourcing a band cheap, and a re-derivation would
*replace* this interval rather than drop it, so failing on one is the alarm working. Without it the
only thing that can silently regress is the argument going missing, leaving a future session free
to tighten this band on a number it does not know has an error bar.

**Verified.** Full suite green. `scripts/reanalyze.py --dry-run` reports every stored result
current, which is the check that matters for a no-version-bump change. All four stored swings still
read `hip_sway` at 0.27 / 0.27 / 0.27 / 0.41, score 1.0, band 0.14–0.5 — unchanged, and the reason
this was the cheap moment to decide it: the same call taken after a bay session would be moving
scores while deciding.

**PICK UP HERE.** **M8's checklist now has one box left, and it is not desk work in the cheap
sense**: per-club bands, costed at face-on driver 341, iron 69, fairway 32 clearing
`MIN_SAMPLES = 30`, with wedge 11 and hybrid 5 short. Before starting it, read
`analysis/checkpoints/mechanics.py::_population_placement`'s docstring — it warns that the band and
the percentile must move *together*, because a band cut from a narrower stratum than the percentile
lets a swing read "inside the band" and "past the 90th percentile" at once. That makes it a scoring
change, not a data edit.

Everything else still wants the bay session, unchanged from the last two entries: §Phase F's
trail-wrist result is unproven at n=2, and career mode plus every surface built on it stays silent
until `n` crosses 5 — read [ADR-022's fourth
addendum](docs/decisions/022-learned-artifacts-as-committed-data.md) *before* analysing that
session.

`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` remains **flaky** —
threading test, 5s timeout, fails under load and passes in isolation. Unrelated; re-run before
believing it.

---

## 2026-08-17 — The same five numbers, in the channel a golfer actually looks at

**Duration**: ~1 session. One shared resolver, one new page block, 4 pins, no version bump.
**What prompted it**: "what is the next thing to work on?" — and the answer turned out to be the
previous entry's own gap rather than anything on the roadmap.

**M8.3 fixed two channels out of three.** It gave the MCP client `SwingView.population` and taught
`build_brief` to render placements, and it never reached `results.html`. That page's
`measurementsBlock` rendered all fourteen measurements as `name / value / unit` and dropped
`detail` — the *identical* field `mcp/query.py` had been dropping — under a caption reading "These
have no tour benchmark band yet, so nothing here is a pass, a fail, or a percentile. They are
recorded so bands can be derived from real swings later." Every clause of that is false for a
placement: the dropped `detail` states a percentile in so many words, and per ADR-022's third
addendum a band is deliberately never coming. So `tour_trajectory_q_dtl: 11.055` sat in a browser
as the largest number on the page, and on three of four stored swings it is the mis-detected
down-the-line anchor. **The bug M8.3 was written to fix outlived it by one channel.**

**The find is the lesson, not the fix.** M8.3's own worklog entry, its ADR addendum and its commit
message all describe the gap as *the MCP channel's*. It was never the MCP channel's — it was
`measurements`-with-`detail`-dropped, and two consumers did that. Naming a bug after the first
place you saw it is how you fix it once.

**`api/state.py::resolve_placements` is the one definition**, and it lives there because that
module's docstring already claims the tolerant readers for `analysis.json`, and because
`mcp/query.py` already imports it under ADR-008's recorded exception — `api` importing `mcp` would
have closed that into a cycle. `mcp/query.py` now wraps it in `PlacementView` and `app.py` serves
it as a derived `population` key beside `result`, never folded into it, since `load_analysis`
returns what the pipeline wrote. All 55 MCP pins passed unchanged through that refactor, which is
what "behaviour-preserving" is supposed to look like.

**The page hard-codes nothing, and there is a pin for it.**
`test_the_results_page_hard_codes_no_placement_name` reads `results.html` and asserts no
`spec.name` appears — a set restated in JavaScript is a second copy, and that is exactly how five
placements shipped with no prose naming them. It failed on first run against *my own comments*,
which is the pin being blunter than the invariant: a comment naming `tour_trajectory_q_dtl` is
what makes the comment worth reading. It now strips `/* */` blocks and whole-line `//` before
asserting, and line comments only at start-of-line so a `//` inside a string can't hide a real
hard-code.

**Grey, not amber.** The uncalibrated flag deliberately does not use `#c93`, the page's `.miss`
colour. A placement borrowing the checkpoint table's miss colour asserts the fault this whole ADR
says no band supports — the flag is a statement about how well the number is known, not about the
swing.

**A second thing was wrong in that block and is now fixed too.** It was captioned "Measured, not
yet judged" while listing the six metrics behind the checkpoint table immediately above it —
false about 6 of 14 rows, pre-existing, and unmissable once the caption was being rewritten
anyway. `api/state.py::judged_metrics` maps through `CheckpointSpec.metric` and drops them. On
`2026-08-07-aaron1/1` that block goes **14 rows → 3**, and its existing sentence is true as
written for the first time.

**Verified by rendering, not by reading.** `node` evaluating the page's own `populationBlock` and
`measurementsBlock` against the real stored payload for `2026-08-07-aaron1/1` — the adversarial
swing — which is worth more than eyeballing the diff: it is the page's output, not the payload it
was handed. `tour_trajectory_q_dtl` comes out at 11.05 under a `down-the-line` heading with
`not calibrated` on the value row and "NOT calibrated, same caveat as the face-on Q" beneath it.

**No version bump, and `scripts/reanalyze.py --dry-run` reports every stored result current.**
Nothing on the analysis path moved — the diff is `api/` and one static page. That dry run is the
cheap form of the check; a full `analyze_bundle` re-run would have cost a pose pass and a live
LLM call to prove something the version counter already answers.

**PICK UP HERE.** The three threads from the previous entry are all still open and unchanged. Two
want the bay session (§Phase F's trail-wrist result is unproven at n=2; career mode's placement
pooling decides itself the day `n` crosses 5 — read ADR-022's fourth addendum *before* analysing
that session). The one piece of desk work left on M8's checklist is **`hip_sway_norm`'s lower
edge**: `ranges.json` justifies its 0.14 at 2.8× the 0.050 error floor, but M8's player-clustered
bootstrap puts p10's 95% interval down at 0.0801 — 1.6×. All four stored swings sit comfortably
inside the band (0.2733, 0.2733, 0.2733, 0.4123), so **no score moves whichever way it is
decided**, which makes now the cheap moment to decide it. `hip_shift_at_top_norm` is the precedent
for dropping an unmeasurable lower edge. The other open box, per-club bands, is costed already:
face-on driver 341, iron 69, fairway 32 clear `MIN_SAMPLES = 30`; wedge 11 and hybrid 5 do not.

`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` remains **flaky** —
threading test, 5s timeout, fails under load and passes in isolation. Unrelated; re-run before
believing it.

---

## 2026-08-17 — Five numbers nobody was allowed to read, and the policy that let them be said

**Duration**: ~1 session. One new contract module, two ADR addenda, 18 pins, two live coaching
calls, no version bump.
**What prompted it**: the third of M8's three loose threads — "nothing yet *says* any of this to a
golfer" — which needed a band or a policy, not more fitting. The session then closed the two
follow-ups it created: the policy is exercised against a live model, and the render finally checks
its own output.

**It is a policy, and the reason is the whole decision.** A band would put a placement on the
scoring path and there is nothing to cut one from: every clip behind these three models is a tour
professional, so they describe the shape of swings that *work* and hold nothing at all about swings
that do not. "Far from the tour population" covers the golfer doing something wrong and the tour
player with an unusual action equally, and scoring that fails every amateur forever while telling
them nothing to change. So the firewall stays exactly where ADR-010's 2026-08-04 addendum put it —
placements ride on `measurements`, `overall_score` does not move. **What changed is not what is
computed but what may be said.**

**The gap was worse than "not yet spoken", and that is the finding.** `contracts/caveats.py` named
**none** of the five placements while all five shipped — so `mcp/query.py`, which flattens
`measurements` to `name -> float` and drops `detail`, was handing an MCP client
`tour_trajectory_q_dtl: 11.06` as a bare number, *under a field description promising there was no
percentile*. The percentile, the population size and the "NOT calibrated" warning were all in the
detail string it dropped. On three of the four stored swings that figure is the mis-detected
down-the-line anchor from §Phase F — the largest number on the swing, with nothing attached to say
it is not about the golfer. That is the M6.5 caveat bug exactly, in the channel M8 opened.

**`contracts/placements.py` is `checkpoints.py`'s argument repeated rather than a new one.** Prose
that has to name a set must derive it, and `caveats.py` cannot import `analysis` (ADR-008). The
registry now feeds four consumers: `engine.py` takes each name and unit off it, `caveats.py` builds
two new bullets from it — including the uncalibrated split, filtered on `PlacementSpec.calibrated`
— `coach.py` partitions the brief with it, and `mcp/query.py` decides `population` membership by
it rather than by a `tour_` prefix, since a prefix test would silently reclassify a future name in
the direction that loses the caveat. `benchmarks/trajectory.py` now imports its two view strings
from it too, so one spelling reaches the artifact keys, the measurement and the prose.

**`build_brief` had never rendered `measurements` at all** — the coaching call, the only channel
that speaks to a golfer, saw none of M8's output. It does now, placements only, with calibration
stamped on the line that carries the value rather than left to the caveat block three hundred words
above it. Same reason `_checkpoint_line` repeats `one_sided`: a model applies a general warning to
the numbers it remembers.

**The policy in the prompt is four lines**: a placement is context and never the headline; it earns
a clause when it sharpens the finding already being named — most usefully a metric that passes its
own band while driving the departure, which is the sentence no band can produce and why the joint
model was fitted; an uncalibrated one says so where it is stated; unusual is not bad. ADR-022's
third addendum has the argument.

**`feedback/rules.py` was deliberately left alone.** A rule-based tip about a placement would be a
verdict, and there is no band to earn one. The LLM gets context; the ranked verdict stays six
checkpoints deep.

**No score moved and `ANALYSIS_VERSION` did not bump** — nothing about the engine's output changed
meaning, only what is said about it. Verified rather than assumed: re-analysing `2026-08-10/2`
reproduced its stored `analysis.json` unchanged.

**`aligned.mp4` was stale on all four swings, not the one the last entry named.** Every render sat
eight days behind its `analysis.json`, because `alignment` moved in v5 when the down-the-line clip
went to the trail wrist. **All four re-rendered** and verified readable (167/167/167/159 frames);
all four re-analyses reported `no change`, which is the byte-identical result above confirmed
across the whole corpus rather than the one swing it was diffed on.

Two things about that check worth keeping. `aligned.mp4` is written *before* `analysis.json` in the
same pipeline run, so a raw `render_mtime < analysis_mtime` test reports every freshly processed
swing as stale — use a tolerance. And the render logs `Failed to load OpenH264 library` plus
`Failed to initialize VideoWriter` and then **succeeds anyway**, because OpenCV falls back to
another fourcc; the errors look fatal, the exit code is 0, and the file is valid. Probe the output
rather than reading the log.

**That last sentence is now code rather than advice.** `pose/side_by_side.py::probe_render` reopens
the finished file and returns the frames it actually decodes; `RenderResult` carries `frames_read`
beside `frames`, and both render callers log the pair and warn when they disagree. The distinction
it turns on is that `isOpened()` — previously the only check — is asked of the **writer**, before a
single frame is written, so it cannot see an encoder that opens and then produces nothing. This is
the one output nothing downstream reads back: a score is read by the report, keypoints by the
engine, and `aligned.mp4` by a human in a browser days later. An unopenable file raises; a short
one is logged, because a bad render is not a wrong score. `2026-08-10/2` re-rendered at
`159 frames (avc1) [reads back 159]`.

**PICK UP HERE.** M8 is closed — fitted, validated, surfaced, spoken with a policy, and that policy
now proven against a live model. What is left are the threads it exposed, and both of the remaining
ones want the same bay session:

1. **§Phase F's trail-wrist result is still unproven on our own footage.** Two distinct DTL clips on
   disk, one segments sensibly and one reports a one-frame backswing on either wrist. That same clip
   is behind three swings' `q_dtl` of 11.05 and behind their stale renders. n=2 cannot confirm a
   corpus result — a `check_metric_transfer.py`-shaped question for a session with more clips in it,
   and the thing most worth capturing at the next bay session.
2. **`hip_sway_norm`'s lower edge** still sits on the player-clustered interval M8 flagged (p10
   0.1414, 95% reaching to 0.0801, i.e. 1.6× the error floor rather than 2.8×). Untouched.
3. ~~The placement policy is unexercised against a live model.~~ **Done — it holds, on both the easy
   swing and the adversarial one.** `2026-08-10/2` and `2026-08-07-aaron1/1`, real `claude-opus-5`
   calls, ~1,050 characters each. Both led on tempo, the one failing checkpoint. Neither made a
   placement the headline. Both produced *exactly* the sentence the joint model was fitted for —
   "the head-hip relationship is the biggest single contributor to how unusual your six numbers
   look together against the tour group, **even though it passes its band comfortably**" — which is
   policy line 2 verbatim and is a sentence no band can generate. Line 4 held too: "that's a style
   marker to be aware of, not something to change." **Line 3 is still untested**, and that is the
   honest gap: `2026-08-07-aaron1/1` carries `tour_trajectory_q_dtl = 11.05`, the largest number on
   the swing, rendered into the brief labelled `NOT calibrated - read beside its T2, never alone` —
   and the model simply never reached for it, going to the calibrated joint distance instead. That
   is the safest possible outcome and it means nothing yet exercises "an uncalibrated placement
   says so where it is stated". Both runs' `analysis.json` diffed clean: `feedback.coaching` and
   `feedback.coaching_text` are the only fields that moved.
4. ~~Career mode pools the placements as if they were metrics, and nobody decided that.~~
   **Deferred on purpose, and written down** — [ADR-022's fourth
   addendum](docs/decisions/022-learned-artifacts-as-committed-data.md), with a pointer from
   `contracts/placements.py`, a line in ROADMAP §Career, and
   `test_a_placement_pools_as_a_metric_today_and_that_is_deferred` pinning today's behaviour so it
   cannot change by accident. **No code moved.** Three things found while writing it up that change
   how alarming it is:
   - **The worst-sounding half is already refused.** Placements are absent from `METRIC_TARGETS`,
     so `dispersion_for` returns early and **no bias/scatter finding is ever emitted** — the "cause
     discriminator applied to a Mahalanobis distance" is wrong about the framing and never reaches
     a reader. `analysis/comparison.py` refuses all five too. `build_baseline` is the *only*
     unrefused layer.
   - **It is worse than un-filtered: it is un-deduplicated.** `population:golfdb` matches neither
     prefix in `CorpusSwing.artifact_key`, so placements key on `swing:{ref}` — and `CorpusSwing`
     carries no down-the-line hash, so the two `_dtl` placements *cannot* be deduplicated at all.
   - **`storage/corpus.py` had already reported this and nobody read it.** `career_corpus.py`
     prints `Unrecognised measurement sources: population:golfdb — Counted per swing rather than
     per artifact, check the dedupe key still fits.` The coverage warning fired correctly the day
     the placements shipped.

   **The deadline is the bay session, not "someday".** Today `n = 2` and every claim is withheld,
   so nothing false is said; the default `CENTER` floor is **5**, so the first 20–30-swing session
   is also the moment `tour_trajectory_q_dtl` acquires a center. Read the addendum *before*
   analysing that session. The seam when it is decided is one branch in `CorpusSwing.artifact_key`,
   the documented single definition of the dedupe rule.

`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` remains **flaky** —
threading test, 5s timeout, fails under load and passes in isolation. Unrelated; re-run before
believing it.

---

## 2026-08-16 — The model that was supposed to predict the ball, and the one that shipped instead

**Duration**: ~1 session. Two ADRs, one new corpus, two gates, one fitted artifact, 12 pins.
**What prompted it**: "where can we get more pro data, and why isn't there a trained model?" —
which turned out to be two questions with opposite answers.

**The pro data was never the bottleneck.** 1,399 GolfDB swings, 458 face-on from 122 golfers,
92 published distributions. What was missing was any corpus with mechanics *and* ball flight on the
same row, which is precisely what ADR-012 said and `NO_LAUNCH_MONITOR_POPULATION` says in code.

**Found one: CaddieSet** (arXiv 2508.20491, MIT, one 508 KB CSV). 924 face-on shots, eight golfers
of mixed skill, per-phase joint metrics over the same eight events GolfDB annotates, plus carry,
ball speed, direction and spin axis. Ingested under the same gitignored root as GolfDB
([ADR-021](docs/decisions/021-caddieset-paired-reference-data.md)).

**Then it failed, and the failure is the most useful thing here.** Leave-one-golfer-out over the
924 face-on shots: spin axis 0.532 against 0.572 for *knowing which club was hit*, carry R² of
**-0.205** — worse than predicting that golfer's own average. Only start direction cleared its
baseline, 0.594 vs 0.535, and only marginally. A regularisation sweep moved nothing.

**Centering each feature on the golfer's own mean made it worse** (0.443, 0.502 — at or below
chance), which is the detail that settles it: the little signal there was lived *between* the eight
golfers, not inside any of them. Traits of eight people, not a lever anyone can pull.

Which is what ball-flight physics predicts and what the ROADMAP already suspected — "we can measure
that a club face is open; we cannot see *why*". The club sets the ball; a face-on camera pointed at
a body does not see the club. So this is not "the checkpoints are worthless", it is the first
empirical support ADR-009's two-axis split has ever had. **The gate did its job: it killed the
centrepiece of the plan before it was built**, the way `tune_arm_parallel.py` killed two
checkpoints.

**What face-on pose can answer is a different question, and the data for it was already on disk.**
Six independent bands cannot say a *combination* is one no tour player produces. New
`tune_joint_structure.py` checked whether there was anything to say: `head_sway_norm` ×
`hip_shift_at_top_norm` at **+0.441**, × `head_hip_gain_norm` at **-0.385**, two more past the gate,
condition number 4.8. There is.

So `derive_joint_model.py` fits a robust center, scale and inverse correlation over those 458 clips
and commits ~50 numbers; `analysis/benchmarks/joint.py` evaluates them in stdlib.
**No invariant moved** — fitting uses the `research` extra, the artifact is data, evaluation is dot
products, and `test_pipeline_imports.py` still passes. That is what `ranges.json` already is
([ADR-022](docs/decisions/022-learned-artifacts-as-committed-data.md)).

**It generalises**: leave-one-*player*-out, 11.1% of held-out swings exceed the refitted model's own
p90 against a 10% target.

**And it immediately says something the panel cannot.** `2026-08-10/2` scores **96.9** — everything
it can be judged on passes except tempo — and sits at the **73rd percentile** of unusualness as a
combination. The decomposition names why: `head_hip_gain_norm` contributes **38.6% of the departure
while passing its band**, because its value is odd *given* the tempo.

**Two things found while the data was open.** 458 clips from 122 golfers are not 458 independent
samples: a player-clustered bootstrap widens the band intervals by up to **×2.16**
(`hip_shift_at_top_norm`'s p90), and `hip_sway_norm`'s p10 of 0.1414 has a clustered 95% interval
reaching down to **0.0801** — where it sits 1.6× the 0.050 error floor above zero rather than the
2.8× ADR-010's addendum justified it on. Worth a look before that band is trusted tightly. And
per-club face-on strata: driver 341, iron 69, fairway 32 clear `MIN_SAMPLES`; wedge 11 and hybrid 5
do not.

**Where it was left.** The model is fitted, committed, tested and **not surfaced** — it is on no
`SwingResult`, `ANALYSIS_VERSION` did not move, no stored analysis changed. That is M6.5's ordering
on purpose: the quantity should be inspectable across swings before it is spoken.

**NEXT ACTION — read [ROADMAP.md §M8.1](ROADMAP.md#m81--the-trajectory-model-next-action-agreed-2026-08-16),
which has the whole agreed plan with its dependency order.** In one line: the shipped model reads
six scalars where the Tier 1 cache holds 461 clips as full 33-landmark time series, so the next
model is a trajectory model over 12 landmarks, ~60 timesteps resampled onto the annotated events,
PCA to 15–30 components.

**The `z` gate is done** (`scripts/golfdb/tune_z_channel.py`, 2026-08-16): `z` **passed** at a
median ratio of 2.69 against `x` 9.86 and `y` 18.09, all 12 landmarks clearing 2.0 at n = 3,521.
So it is not pure noise — but it carries ~22% of the planar signal-to-noise and **the screen
flatters it**, because lite and full are one architecture at two sizes and make correlated
monocular-depth mistakes. 2.69 is an upper bound. Decision: carry `z` into the trajectory model and
**fit twice, with and without it**, letting leave-one-player-out exceedance settle it.
`docs/M4_POSE_BAKEOFF.md` §Phase D has the writeup, including two silent bugs found on the way —
GolfDB event indices need rebasing on `start`, and spread must be measured hip-relative.

**The trajectory model is fitted too** (§M8.1 step 3, 2026-08-16):
`analysis/benchmarks/trajectory_model_v1.json`, 98 KB — 12 landmarks, x/y, 40 timesteps on 3
detected anchors, PCA to 10 components, 73.6% variance over 415 clips from 116 golfers,
leave-one-player-out T² exceedance **8.9%** against a 10% target. Three findings in
`docs/M4_POSE_BAKEOFF.md` §Phase E: **`z` lost the A/B** (it *lowered* variance explained and
worsened both calibrations — §Phase D's warning was right); **the anchor set nearly shipped
unusable**, because four of GolfDB's eight annotated events are ones `segment_phases()` cannot
produce, so a model keyed to them could never score a real swing; and **`Q` is not calibrated** at
14% against 10%, which is a property of a residual rather than a tuning failure.

**Both models are now surfaced.** `ANALYSIS_VERSION` 3 → 4, `measurements` 9 → 12 on all four
stored swings, and **every `overall_score` identical** — the three placements ride on
`measurements`, never `checkpoint_scores`, which is the firewall verified rather than assumed. A
new `source` value, `population:golfdb`, sits alongside `pose:face_on` and `launch_monitor:*` and
says plainly that these are population-relative rather than measured off the body.

The feature builder is `analysis/trajectory.py` — **stdlib, in the package, and imported by the
fitting script** rather than duplicated in numpy. That was the one design point worth insisting on:
a vector built one way at fit time and another at scoring time gives a model evaluated against
numbers never fitted to it, which produces plausible output and no error.

**A pixel-aspect bug, found by reading `derive_pose_metrics.py` rather than by any test.**
`videos_160` squashes a non-square crop to a square, so x and y sit on different scales per clip
(ADR-012's third accepted limit). Metrics built from x-ratios cancel it; the trajectory model mixes
axes in every component and did not. Correcting it moved variance explained **73.6% → 80.3%** —
about seven points of the basis had been describing GolfDB's cropping — and **changed the optimal
component count from 10 to 6**. Never carry a hyperparameter across a change in how features are
built.

**The down-the-line corpus is extracted too** (§M8.1 step 2): all **584** DTL clips, 0 missing,
1,501 s at 109 fps. The Tier 1 cache went from 461 clips to **1,045** — it had only ever held the
face-on half. GolfDB's remaining 354 clips are view `other` and were skipped deliberately; they are
neither of our camera positions.

**M8.2 started, and its first question is answered.** The 584 DTL clips immediately paid for
themselves: `tune_phases.py` gained `--view` and `--wrist`, and scoring the shipped rule
down-the-line for the first time showed it **misses the top on 30% of clips and impact on 35%**,
against 9% and 7% face-on. `--detail` says why — it is the pose track, not the rule. From behind
the lead wrist is the far arm, tracked in **39%** of frames against the trail wrist's **70%**, and
the clips that fail are the poorly-tracked ones (confidence 0.32 against 0.42), whereas face-on the
failing clips are tracked *better* than average. No rule tuning reaches that.

**On the trail wrist the same rule gets 7% and 2% — better than face-on.** So `segment_phases` now
takes the landmark, defaulting to the lead wrist so face-on is byte-identical, and the bundle path
passes `TRAIL_WRIST` for the down-the-line clip. `ANALYSIS_VERSION` 4 → 5; every stored score is
unchanged, `alignment` moves. This answers **M7 Spike Q1** — named there as the biggest unmeasured
risk under the whole two-phone ladder — and **reverses M4_POSE_BAKEOFF §Phase B7's** "no view-aware
landmark selection is warranted", which was measured on one bay swing against 1,045 labelled clips
here.

A fixture bug fell out of it: `conftest._frame` animated only the lead wrist, so every synthetic
swing described a golfer holding the club one-handed. Invisible until something read the other
wrist.

**PICK UP HERE.** Two things, neither blocking:
1. **The transfer is unproven on our own footage.** Of the two distinct DTL clips on disk one
   segments sensibly and the other still reports a one-frame backswing on either wrist. n=2 cannot
   confirm a corpus result — this is a `check_metric_transfer.py`-shaped question for a session
   with more clips in it.
2. **`aligned.mp4` is stale for `2026-08-10/2`** — `reanalyze.py` said so itself. Re-run it with
   `--video` when convenient; nothing reads the render.
3. **The DTL landmark list is measured** (`tune_landmarks.py`, M4_POSE_BAKEOFF §Phase G): from
   behind, **the whole lead arm is gone**, not just the wrist — elbow 0.46, wrist 0.47,
   thumb/index/pinky 0.37-0.40, against 0.84-0.87 on the trail side. Shoulders, hips, knees and
   ankles are fine on both sides, so it is specifically the arm that crosses the body. Two of the
   face-on twelve fail, so that list cannot be reused; the proposed DTL twelve is ears, shoulders,
   hips, knees, ankles, trail elbow and trail wrist.

   **Fitted, and the list mattered more than the screen said.** `derive_trajectory_model.py` now
   takes `--view` and `--landmarks` and writes one artifact per view.
   `trajectory_model_dtl_v1.json` (63 KB) fits **510 clips from 166 golfers** — broader than the
   face-on model's 415/116, since GolfDB has more DTL clips than face-on ones — at
   leave-one-player-out T² exceedance **10.2%** against a 10% target.

   The comparison the screen could not make: handing the **face-on** twelve to a down-the-line fit
   skips **441 of 584 clips (72%)**, because the lead arm is missing too much of its timeline, and
   what survives is the biased remnant where it stayed visible. Q calibration 20.3% against 12.2%.
   So the wrong landmark list does not cost accuracy so much as *corpus*.

   Found on the way: `analysis/trajectory.py::LANDMARK_INDEX` had no ankles, so the new DTL set
   referenced names it did not know and `build_trajectory` returned None for **every** swing —
   silent, not loud. The map now has to cover the union of all views' lists.

   ⚠️ **Do not read 93.7% variance explained as a better model.** Down-the-line the swing runs
   toward and away from the camera, so the features are redundant and fewer directions describe
   them. Well-calibrated, and probably seeing *less* of the swing than face-on. What DTL uniquely
   carries — spine tilt, swing plane — lives in the depth direction one camera cannot resolve,
   which is ADR-011's "aligned but never fused" appearing as a number.

**The DTL model is surfaced too.** Per-view loading in `benchmarks/trajectory.py`;
`analyze_swing_bundle` records `tour_trajectory_t2_dtl` / `_q_dtl` beside the face-on pair.
`ANALYSIS_VERSION` 5 → 6, face-on untouched, every stored score identical, `measurements` 12 → 14
on a two-view bundle. 24 pins in `tests/analysis/test_trajectory.py`.

**The design question answered: the two views are never blended.** Two cameras answer the same
question about different planes of one swing, and a mean of them answers neither — blending would
be the mistake ADR-009 avoided by keeping mechanics and outcome apart. Disagreement is a *finding*:
a swing that places ordinary face-on and unusual from behind departed in the plane face-on cannot
see. Separate measurement *names* rather than one name with a view field, because
`baseline.pooled_samples` groups by name and would otherwise pool two cameras into one baseline.

Anchors are reused from the alignment pass rather than recomputed — cheaper, but really so the
frames the trajectory is read from cannot drift from the frames the warp pins to. That left two
implementations of "read three instants off a phase chain" (`alignment.anchors_from_phases` and
`trajectory.anchors_from_phases`), now pinned to each other by a test, since nothing else would
catch the drift.

**The stored swings surfaced a caveat immediately.** Three of four report `tour_trajectory_q_dtl`
of 11.05 against 5.19 for the fourth — and those three share the DTL clip whose segmentation still
reports a one-frame backswing. Q is doing its job (a shape the basis cannot represent) while the
*cause* is a broken anchor set, not an unusual swing. Q cannot tell those apart, which is one more
reason it ships uncalibrated and labelled.

**PICK UP HERE.** M8 is complete — three models, one per camera, all fitted, validated, surfaced
and pinned. Nothing in it is half-done. What remains are the threads it exposed rather than steps
in it:
1. **§Phase F's trail-wrist result is unproven on our own footage.** Of two distinct DTL clips on
   disk one segments sensibly and one reports a one-frame backswing on either wrist, and the
   `q_dtl` gap above is that same clip showing up downstream. n=2 cannot confirm a corpus result —
   this is a `check_metric_transfer.py`-shaped question for a session with more clips.
2. **`aligned.mp4` is stale for `2026-08-10/2`** — `reanalyze.py` said so itself. Re-run with
   `--video`; nothing reads the render.
3. **Nothing yet *says* any of this to a golfer.** Five population placements are recorded on every
   swing and `feedback/` reads none of them. That is M6.5's ordering working as intended — the
   quantities should be inspectable across swings before they are spoken — but with the models
   calibrated it is the obvious next milestone, and it needs a band or a policy, not more fitting.

`tests/api/test_worker.py::test_failure_is_recorded_and_the_consumer_survives` is **flaky** — a
threading test with a 5s timeout that fails under load and passes in isolation. Unrelated to any of
this; re-run before believing it.

---

## 2026-08-15 — The conversation that refused to average two sessions (M6 closed, ADR-020)

**Duration**: ~1 session. One ADR, one addendum to another, one new subsystem, 70 pins, and the
last desk-work item on the board.
**What prompted it**: "what's the next thing to work on" — and for once both boards already
agreed. `ROADMAP.md`'s NEXT ACTION and this log's top entry named the same item.

**M6 is done.** A golfer can now ask follow-up questions about a swing, from the terminal
(`scripts/ask_swing.py`) or from a chat panel on the results page. The conversation seeds from the
swing's own stored brief — the same `build_brief` the coaching paragraph is written from, so the
two cannot describe one swing differently — and looks everything else up through the same eight
tools the MCP server offers external clients. **Called in-process, not over stdio**: that round
trip exists for Claude Desktop and Claude Code, and re-paying it to reach functions already
importable buys nothing.

**The acceptance test was never going to be a test.** Asked "is that worse than last session?"
against the two swings on disk, it answered: *"The comparison tool withheld every per-metric mean,
tempo included: it needs 8 swings in a session to state a typical tempo and each of these had
one."* That is the whole point. The withheld figure sits in the payload next to the per-session
counts that justify withholding it, the arithmetic is trivial, and nothing in a JSON document can
stop a model doing it — `READING_A_PERSONAL_HISTORY` saying so is the only control that exists,
and it held. It also volunteered that the 8/09 session contributes no samples because its clip is
a byte-identical re-upload, which is `duplicates_collapsed` explaining itself unprompted.

**It refused the causal question too**, which was not asked of it: *"this data can't tell you why —
it measures the ratio, not the cause… any cause I named would be invented."* No spine angle, no
hip rotation, no swing plane, no club path. The one figure that looked like an invention — "your
downswing is around 0.38 to 0.42 seconds" — is in the brief's alignment note (0.384s and 0.417s),
correctly attributed to the two views agreeing.

**A transcript stores the model's own blocks verbatim, and that is load-bearing rather than
lazy.** Thinking is on by default on `claude-opus-5`, thinking blocks are only legal replayed
*unchanged*, and only into the model that produced them. So `Transcript.model` is a replay guard:
resuming under a changed `coaching_model` reseeds instead of replaying. Without it the failure is a
400 on turn three of a conversation that worked twice, long after the config moved. Turn three was
run live to prove the replay: it worked.

**The seam that took the most thought was prose, not code.** A model picks a tool by reading its
description and nothing else, and there were now two adapters needing the identical eight
paragraphs. They moved to `contracts/tool_descriptions.py` — the answer `caveats.py` already
established — and `NotFound` plus its six miss hints moved to `query.py` for the same reason: a
wrong id has to read as a retry, identically, either way. Both moves verified byte-for-byte
against HEAD. **The move also closed a gap it did not set out to**: the descriptions were
unreachable from `test_docs_truth.py` while they lived in a module needing the MCP SDK, and the
count-claim scan now covers them.

**One trap worth writing down.** `@beta_tool` reads a function's description from `__doc__` **at
decoration time**. Decorate first and assign `.description` afterwards and you get an empty
description, no error — a tool the model silently never calls. `_tool()` assigns `__doc__` before
calling `beta_tool`, and says so.

**Found by running it, not by testing it**: the results page opened with the entire swing brief in
a bubble labelled as the golfer's own words. The seeded turn is now two content blocks rather than
one joined string, so `visible_turns` can render the question alone — a block boundary survives a
rewording of the preamble, and pattern-matching on it would not.

**And ADR-019's pin fired for the first time**, on the full-suite run at the end: `ask_swing.py`
unwraps the API key for the SDK, which made a fourth `get_secret_value` site. The test named the
file and pointed at the ADR, which now carries an addendum saying why the site is right. That is
the friction ADR-019 called the feature, working on its first real exercise.

**Where I left off**: green — 694 tests (70 new), ruff and mypy clean. Five live turns against
`2026-08-10/2` across CLI and HTTP; three conversations on disk under
`data/processed/conversations/`.

**The board has no desk work left.** Everything remaining needs a bay session or needs `n`, and
one bay session is the unblock for both — career mode is built, complete and silent at n=2, and
every surface over it, now including the conversation, refuses rather than guesses. 20–30 swings
in one session turns all of them on at once.

---

## 2026-08-14 — The warning that cried wolf for thirty sessions, and the fade it was hiding

**Duration**: ~1 session. One ADR addendum, twelve pins, one wrong number corrected on disk.
**What prompted it**: "what can we work on next", and the smallest open item on the board —
every shot ever parsed carried `screen title 'SHOT DATA' not found`.

**The warning was noise. What it was sitting on top of was not.** `spin_axis` was stored with
its sign inverted. HD Golf prints that one tile already signed (`-9.3 °`) where every other angle
is a magnitude plus a direction word; ADR-014's sign section only anticipated the second shape, so
the parser found no direction word, warned "sign unknown", and stored the number as printed. The
contract is `+ = fade`. **Both shots on disk were fades recorded as draws** — and the MCP server
has been serving them to Claude that way.

**Three readings of one screen agreed, which is why this was a correction and not a guess.** The
`Shot Type` tile one column away reads `FADE` on both. The face sits 13.2 ° and 10.9 ° open to the
path, which curves the ball right for a right-handed golfer. And the magnitudes are what that
face-to-path would produce. The screen was carrying its own answer the whole time.

**So the fix is now self-checking, which matters more than the fix.** `validate.py` gained a third
cross-check beside the two identities: `sign(spin_axis)` must agree with the curvature word in
`shot_type`. This is the one misread the arithmetic cannot see — every magnitude can be perfect and
the shot still be reported bending the wrong way, which is exactly what happened. If `printed_sign`
is ever wrong for some shot shape, it now says so. The polarity itself is profile *data*
(`printed_sign: -1`), so the next device that prints its own signs is a JSON edit.

**The title warning was two bugs wearing one message.** PaddleOCR returns the wide-tracked banner
as one token, `SHOTDATA`; the check was a substring test against `SHOT DATA`. It passed in the
tests because the fixture's title was written by hand *with* the space — **a fixture kinder than
the OCR engine, testing the parser against a screen that does not exist.** That is the transferable
lesson here; conftest now says so in its docstring. Underneath it, `rectify` crops the reference
photos below the banner entirely, so the check also had to stop treating a cropped-out title as
evidence of anything.

**Why chase warnings at all.** They reach a golfer — `provenance.warnings` and `needs_review` are
carried out by the MCP server, named in the standing caveats, and rendered into the coaching brief.
Two of four warnings on a typical shot were unfalsifiable, and `needs_review` was `False` on a shot
carrying five of them. The noise had already decoupled from the signal. A warning that fires on
every correct parse is what teaches a reader, human or model, to skip the ones that are real.

**Two warnings survive because they are true**, and both want the bay rather than a decision.
`no tile found for 'Bounce & Roll'` is correct — the bay's screen shows `Impact Position V` where
the reference photos show `Bounce & Roll`, so the profile describes a layout HD Golf can be
configured out of. And on the one photo where `rectify` fails, `Impact Position`'s value is claimed
by the tile next door (`'CENTER SLIGHT FADE'`), a cell-boundary bug that only appears uncropped.

**Where I left off**: green — 624 tests (12 new), ruff and mypy clean. All four swings re-imported
with `--force-ocr`, so the corrected sign is on disk and the MCP server serves `spin_axis: 9.3`
beside `shot_type: FADE`. Warnings on the cleanly-cropped shot went 3 → 1. M3's remaining OCR work
now genuinely needs a bay session. The board's only sizeable desk-work item left is M6's follow-up
questions over a transcript store.

---

## 2026-08-14 — The key that arrived, and the two axes `.gitignore` never covered (ADR-019)

**Duration**: ~1 session. One ADR, one addendum, five pins, and both of M6's standing "never
actually tried it" items closed — the live coaching call and the MCP client handshake.
**What prompted it**: an Anthropic API key, and the question of where to put it so it stays put.

**M6 is proven live, on both distinct swings.** `analyze_bundle.py --no-video` against
`claude-opus-5` returned paragraphs that led on tempo — the one checkpoint outside its band in both
— asserted nothing the brief did not contain, and volunteered the caveats that applied (shot data
attached but unscored; the second view anchored only at top and impact). No spine angle, hip
rotation, swing plane or club path. `analysis.json` carries the model, timestamp and a sha256 of the
brief. **The first real requests this repo has ever sent**, ~30 sessions after the path was built.

**Only two distinct swings exist on disk, and three directories are one of them.** `2026-08-07-aaron1/1`,
`2026-08-09/2` and `2026-08-10/1` are byte-identical re-uploads — same sha256, same 104,089,834
bytes, and the MCP `get_swing` scores agree to fifteen decimal places. So "run it on both swings"
cost one call, not three. This is exactly the case `CareerCorpus.duplicates_collapsed` and the
briefing's closing line ("`n` counts distinct swings, not swing directories") were written for; nice
to see the design meet the data it predicted.

**The MCP handshake is proven, which closes the other standing "never actually tried it".** A real
stdio client completed `initialize` (protocol `2025-11-25`, capabilities negotiated, the
5k-character briefing delivered on connect), `list_tools` returned all 8 with schemas, and
`call_tool` served four real queries plus a deliberate miss. **The `NotFound` shape survives the
wire as a normal result with `is_error=False`** — a client sees "no swing '9' in session 'nope',
call list_sessions" as an answer, not a protocol fault, which is what `_missing` was built for.
Registered with Claude Code via `claude mcp add`; `claude mcp list` reports `✔ Connected`, a second
client completing the same handshake independently. Note for anyone writing a client here: the
Python SDK at `mcp` 2.0.0 is **snake_case** (`server_info`, `input_schema`, `is_error`), not the
camelCase the wire protocol uses.

**The security work was not where I expected it.** `.gitignore` was already right — `.env` ignored
since the first commit, never tracked on any branch, no key ever in history, and nothing key-shaped
in the tree. What it does not cover is the two axes that actually bound the risk: **disclosure**
(can the value reach a log, a traceback, a `model_dump()`?) and **at rest** (who can read the
bytes?). It answers a third, narrower question — does this reach the public repo — and answers it
well. That is the whole of ADR-019's context section.

**Three measurements turned a tidy-up into a decision:** `.env`'s ACL granted `BUILTIN\Users` →
Modify, so every local account could read both secrets; `.gitignore` protected exactly one
*filename*, so `cp .env .env.bak` before an edit would have committed a live token to a public
repo; and `run_server.py` printed the upload token in full on every start, into scrollback that
outlives the session.

**`SecretStr` for both secrets, and the keychain declined.** `keyring` would have removed
plaintext-at-rest, but its Windows backend *is* Credential Manager — platform lock-in in a repo
that is otherwise portable and stdlib-first, and no defence against the realistic local threat.
Declined deliberately, written down in ADR-019 so it does not get re-proposed. That trade is why
the masking is pinned rather than trusted: `tests/test_config.py` fails if either annotation
reverts to `str`, and fails if a **fourth** file learns to call `.get_secret_value()`. Exactly
three may — the Anthropic SDK, `compare_digest`, and the operator's setup link.

**The pin had to parse rather than grep.** `config.py` names `get_secret_value` in its own
docstring, explaining the mechanism; a text search would have forced the prose to avoid the word it
exists to describe. `ast.walk` for `Attribute` nodes ignores strings and comments entirely.

**Where I left off**: green — 612 tests, ruff and mypy clean. Nothing outstanding from this work.
M6's only remaining item is follow-up questions over a transcript store, which is a milestone of
its own. The smaller open thing is M3's OCR tuning: every shot parsed so far carries
`screen title 'SHOT DATA' not found`, and one of them lost `Impact Position` entirely.
**Deliberately left open**: `api/app.py:135` still waves everything through when no token is
configured, and the only guard against a wide-open bind lives in `scripts/run_server.py` — so
`uvicorn ... --host 0.0.0.0`, a container or a systemd unit bypasses it. Closing it changes API
behaviour and cuts against ADR-016's premise that tailnet membership *is* the access control, so it
wants its own ADR rather than a ride-along. Named in ADR-019's Consequences.

---

## 2026-08-14 — The club head that our light destroyed (M1.5, closed as a no-go)

**Duration**: ~1 session. A spike, an ADR, two addenda, and a roadmap correction about money.
**What prompted it**: "what can we work on next" — with the constraint that it had to be pure
desk work. Every other track is blocked on a bay trip, an API key, or footage nobody can shoot
right now. M1.5 turned out to be the one milestone whose exit criteria were reachable from
footage already on disk.

**M1.5 is closed and the answer is no-go.** `spikes/club-head-detectability/` (thresholds,
probe, log) and **ADR-017**. Addenda on ADR-005 (labelling deferred on evidence) and ADR-003 (the
number behind "global shutter ≠ no motion blur"). No `src/` change — `detection/detector.py`
still raises, and its message is still accurate.

**It needed no new footage.** `data/raw/aaron-{1,2}` already held two swings × two views at
2160×3840/60fps, and the pipeline had already computed impact on all four clips, so the spike
read the frames the system actually calls impact rather than re-detecting them. Thresholds went
into `thresholds.md` before a single frame was extracted, per the M7 Phase 0 precedent.

**The finding is not the one the milestone was written to expect.** The club head is a *good*
detection target at rest — 42 px across the short axis at 4K, crisp, behind a crisp ball. It is
destroyed by **exposure time**. At the bay's 1/60 s it smears 600–980 px, 14x to 23x its own
size: a translucent band you can see the mat through, present in the impact zone for about three
frames of the sixty in that second and boundable in none of them. Pure-ML has nothing to label
there, and **a marker does not fix it** — a marker raises contrast, and contrast is not what is
missing. So M2's real gate is **~1/2000 s, about 30x the bay's present light**.

That corrects a standing roadmap claim about money: the one purchase that blocks something is
**not** a global-shutter camera. Shutter *type* does not appear in the calculation anywhere;
exposure *duration* is all of it, and a global-shutter camera in the current light still records
a smear. The purchase is lighting, evaluated against minimum exposure and lux.

**Three things found by running it rather than reasoning about it:**

1. **The scale that matters is the ball's, not the golfer's, and they differ by 1.66x.** The
   first cut normalised by the body, as the rest of the repo does. Wrong ruler: the head at
   impact is in the *ball's* plane, nearer a slightly downward-looking phone, and the ball
   measures 57 px where a body-plane ruler predicts 34. Every blur figure on the body plane is
   40% too small. The ball is the better ruler anyway — 42.7 mm by rule, against an assumed
   shoulder width.
2. **Shoulder width is not a ruler down-the-line, for a reason already written down here.**
   Face-on it gives 887/883 px/m across the two swings; down-the-line, 198/112. From behind the
   golfer the shoulders are edge-on. That is M6.5's finding again — *a quantity comparing two
   body parts at one instant does not survive a change of camera yaw* — and the fix has the same
   shape: use a **vertical** extent, which yaw leaves alone.
3. **The pre-committed sharpness metric was uninformative, and is recorded as such rather than
   quietly swapped.** Moving-region ÷ static-patch Laplacian variance lands at 1.4–5.4 — the
   *moving* region always scores *higher*, because it is full of body edges while the static
   patch is a smooth mat. It measures scene texture, not blur. The verdict rests on the extent
   criterion and on looking at the frames, which is what the M1.5 checklist asked for. ADR-017
   carries an addendum on why the threshold table pointed at marker-assisted and the ADR did
   not follow it.

**One number in the shot data does not survive contact.** Both swings carry a physically
impossible smash factor (1.00 and 0.92 — a real strike cannot exceed ~1.5 or plausibly fall
below ~1.1), so the launch monitor's club-speed reading is not trustworthy. Rather than assume
it, the club speed is **bracketed** from `ball_speed / 1.5` up to the reading as printed, and the
verdict is checked across the bracket. It holds either way: 1/1714 s at the low end, 1/2793 s at
the high end. Worth remembering when M3's OCR tuning finally happens.

**Verified**: 607 tests, ruff and mypy clean — unchanged, because nothing in `src/` moved.
`spikes/` is outside both gates by design (`ruff check src tests scripts`, `mypy src`).
`tests/test_docs_truth.py` pins the doc map's counts, so the new ADR and the two addenda are
reflected there: 42 → 45 markdown documents, 16 → 17 decisions, 15 → 18 addenda.

**Also corrected here**: this log was stale by two commits. The 2026-08-14 governance and
refactor-pass commits landed without an entry, and the count of **552 tests** quoted in the entry
below has been **607** since then. Neither is re-narrated; this note is the pointer.

**Where I left off**: M1.5 is closed. The single open question is the one no desk work can
answer — one clip of a swing under bright light with the shutter forced short flips pure-ML and
marker-assisted together, and `probe.py` re-runs against it unchanged by adding a row to
`SWINGS`.
**Blockers**: none new. The standing NEXT ACTION is still an Anthropic API key for M6's live
path; note that API billing is separate from a claude.ai subscription and runs on prepaid
credits, so a few dollars covers hundreds of coaching calls at this volume.

---

## 2026-08-13 — The band that did not survive our camera (M6.5, closed)

**Duration**: ~1 session, a new harness + implementation + tests + a re-analysis of everything on disk
**What prompted it**: M6.5's last open item, `head_hip_offset_impact_norm`. The roadmap framed it as
an architecture problem — `analysis` is pure and cannot read `Golfer.handedness` — and that framing
was right about the blocker and wrong about the risk.

**What landed**: the handedness seam, a new transfer-check harness, a rejected metric, a promoted
one, and the panel at **six checkpoints**. `ANALYSIS_VERSION` 2 → 3. **552 tests** (up from 547),
ruff and mypy clean.

**The metric M6.5 named was the wrong one, and the check that says so did not exist.** The repo had
`tune_spatial_metric.py`, which asks *is this metric signal or jitter* — entirely inside the
reference corpus. Nothing asked the question that comes after it: **does the population the band was
cut from project the way ours does?** So `scripts/golfdb/check_metric_transfer.py` measures the same
quantity **at address**, where the body is square, the hips have not rotated, and there is no swing
yet to disagree about. The result:

| | at address | at impact | impact − address |
|---|---|---|---|
| GolfDB (broadcast) | −0.198 | −0.617 | −0.404 |
| our bay clips | **+0.124** | −0.032 | −0.156 |
| gap | **+0.322** | +0.585 | +0.249 |

**55% of the disagreement is present before the golfer moves**, at ~4x the metric's own 0.082 error.
Promoting the absolute offset would have scored 0.39–0.50 on all four stored swings, made "you are
not staying behind the ball" the top-ranked tip on every one of them, and dropped every score from
~96 to ~87 — roughly half of it camera.

**Why this metric and not the other five.** Every shipped checkpoint differences *one landmark
across time* — sway and shift are travel, finish balance is drift from its own mean, tempo is a
ratio of durations — so a fixed camera bias is common-mode and cancels in the subtraction. The
absolute head-hip offset compares *two different body parts at one instant*. Shoulder-width
normalization removes the `1/Z` scale, so distance from the camera is handled; it does not remove
**yaw**, and at impact the hips have rotated open while the head has not, so the two sit at
genuinely different depths exactly where the metric reads. That distinction is now written into
`mechanics.py`'s module docstring, because it generalises past this metric: *a band cut from
broadcast footage transfers to a phone only for quantities where the camera cancels.*

**So the delta was promoted instead, and it costs almost nothing.** `head_hip_gain_norm` is impact
minus address under one shared address-window ruler — the same coaching concept, in the regime where
the bias cancels. Ratio **7.1** against the absolute's 7.6. Band two-sided `[-0.67, -0.14]`, both
edges 3.4x the 0.080 error. On the real swings it reads −0.16 to −0.17: **inside** the tour range,
near the shallow edge at the 87th percentile — the opposite verdict from the one the absolute would
have delivered, and the impact frame agrees with it.

**Three things found by running it rather than reasoning about it:**

1. **One golfer, two handednesses.** Classifying handedness independently per signed metric gave
   `TOBY KEITH` opposite labels, from medians of −0.110 and +0.015 that are both inside measurement
   error. A person does not swing right-handed under one measurement and left-handed under another,
   so handedness is resolved **once** per subject, from the metric whose population sits furthest
   from zero. The classifier validates itself by name: four of 122 subjects fold, and two are Phil
   Mickelson and Bubba Watson.
2. **A stored `sd` that was half artifact.** `head_hip_offset_impact_norm` carried sd 0.504; two
   clips of one Stacy Lewis driver swing read **5.44** and **6.13** shoulder-widths — a collapsed
   `shoulder_width` denominator, not a body. Dropping them gives 0.249. Quantiles were robust so no
   shipped band ever moved, which is precisely why nothing caught it.
3. **The unscored tip told the golfer to re-film a clip that was fine.** With no golfer attributed,
   `head_stays_back` drops out and `feedback` emitted its standard "try a clip with the whole swing
   in frame and the camera steady" — confident, and wrong: the swing measured perfectly, the *form
   field* was empty. The remedy is now chosen from what is actually missing, keyed on whether the
   measurement came through.

**The seam itself is small, which is the point.** `analyze_swing(..., handedness=)`, resolved from
the manifest's `player_id` by `api/pipeline.py`. `analysis` imports `Handedness` from `contracts`
and no registry; `None` costs that one checkpoint and names it in `unscored`, because guessing
right-handed would read a left-handed golfer's ordinary impact position as a gross fault — silently,
and in the direction nothing downstream can detect.

**Verified**: 552 tests, ruff and mypy clean. `derive_reference.py` re-run touched **only** the two
signed metrics' rows — 7 added, 7 changed, every other row byte-identical, which is the regression
property that made the change auditable. All four swings re-analyzed `version 2 -> 3`; sidecars
agree with `analysis.json`; the MCP `get_swing` view carries the sixth checkpoint; career mode picked
up the ninth metric with no career-mode change at all and still refuses everything at n = 2.

**Where I left off**: M6.5 is closed. The rejected metric is still measured on every swing, so a
later camera-geometry fix can revisit it without re-capturing anything.
**Blockers**: none. The roadmap's standing NEXT ACTION is still an Anthropic API key — M6's live
path has never made a real request.

---

## 2026-08-12 — The panel widens to five, and the default band shape was wrong for both (M6.5)

**Duration**: ~1 session, implementation + tests + a re-analysis of everything on disk
**What prompted it**: M6.5's last open item — "decide what to promote". The measuring/judging split
shipped five metrics recorded and none scored, and the panel had sat at three checkpoints for four
milestones. With the bay session a while off, this is the largest user-visible change available
that needs no new data at all.

**What landed**: `evaluate_hip_sway` and `evaluate_hip_shift_at_top` in
`analysis/checkpoints/mechanics.py`, two rows in `ranges.json`, engine wiring, an ADR-010 addendum,
and `ANALYSIS_VERSION` 1 → 2. **547 tests** (up from 540), ruff and mypy clean. The mechanics panel
is now **five checkpoints**.

**The roadmap's own note about what blocked this was aimed at the wrong thing.** It said promotion
"wants more than one golfer's swings behind it". It does not: the bands are cut from 458 face-on
GolfDB swings by 122 tour golfers, not from ours, and they were already derived and committed by
M6.5. What actually had to be decided was **band shape**, and that is where the work was.

**`derive_reference.py`'s default would have shipped a wrong band for both metrics.** It recommends
a one-sided `[0, p90]` band for anything named `_norm`, which encodes *less is better*. That is
established for head sway and finish drift. It is **not** established for hip travel — some of it is
the weight shift a swing needs — and this repo had already written that down, in career mode step 5,
as the reason both metrics were denied a bias target. The tour distribution says the same thing out
loud: `hip_sway_norm`'s p10 is **0.14**, so 90% of tour swings move the hips *further* than that. A
`[0, p90]` band would have scored a golfer who barely moves their lower body as perfect.

**Then the two metrics needed opposite treatment, which is the part I did not expect.** Having
rejected the default for both, the obvious move is to make both two-sided. The measurement error
says no:

| metric | p10 | p90 | noise + boundary | p10 vs error |
|---|---|---|---|---|
| `hip_sway_norm` | 0.138 | 0.499 | 0.050 | **2.8x above** |
| `hip_shift_at_top_norm` | 0.015 | 0.207 | 0.053 | **0.3x — below it** |

So `hip_sway_norm` is two-sided `[0.14, 0.50]`, and `hip_shift_at_top_norm` is one-sided
`[0, 0.21]` — the same shape as head sway, reached by the opposite argument. Not because less is
better, but because a lower edge at 0.015 would separate golfers this pipeline cannot tell apart.
The rule that falls out is worth more than either row: **assert a band edge only where it clears the
instrument.** `tune_spatial_metric.py` already computed the numbers for step 5's tolerances; this is
the second thing they have been used for, and neither use needed new extraction.

**Two things found by running it rather than reasoning about it:**

1. **The default synthetic swing now fails a checkpoint, and it should.** `make_swing`'s body holds
   its hips perfectly still, which no golfer does, so it fails the two-sided `hip_sway` while
   passing all four others. `test_a_fully_measured_swing_leaves_nothing_unscored` asserted
   `== 3` and was the only test in the suite that broke. Rather than just bumping the number, it now
   pins the pass/fail split too — a later fixture default that adds a hip shift has to come past
   that line instead of quietly flipping a checkpoint back to green.
2. **Adding checkpoints raised every stored score, which is why the version had to move.**
   `overall_score` is a mean over survivors, so two new passing checkpoints dilute the one failing
   tempo: 94.92 → **96.95** on three swings and 93.77 → **96.26** on the fourth. Nothing about those
   swings changed. This is the coupling the 2026-08-01 ADR-010 addendum flagged for band *width*,
   showing up for panel *membership* — and it is the case `ANALYSIS_VERSION` exists for.
   `reanalyze.py` found all four unprompted and `record_state` kept the sidecars in step, so career
   step 3's two mechanisms both paid off without being touched.

**`one_sided` stopped being an internal ranking detail.** With `tempo_ratio` and `hip_sway_norm` both
two-sided, "a low percentile is good news" is now wrong on two of five checkpoints.
`contracts/caveats.py` says so explicitly, because a model handed a low number will otherwise
congratulate the golfer for it — the same class of defect as career step 6's amber "outside tour
range" pill, caught this time before it shipped rather than by a render.

**Verified**: 547 tests, ruff and mypy clean. Against the real disk, all four swings re-analyzed
`version 1 -> 2`, and on `2026-08-07-aaron1/1` the new checkpoints read `hip_sway` 0.27 (percentile
41.5) and `hip_shift_at_top` 0.08 (percentile 48.6) — both near the tour median, which is the
sanity check that they are not manufacturing faults on a real amateur swing. Feedback still leads
with tempo, the only failure. `analysis.state.json` agrees with `analysis.json` on all four, and the
three career CLIs still report `n = 2` for all eight metrics, unchanged: promotion judges
measurements, it does not alter them.

**Where I left off**: `head_hip_offset_impact_norm` is the one candidate left, and it is an
architecture change rather than a data edit — the band exists and the ratio is 7.6, but its sign is
camera-relative, and `analysis` is pure and cannot read `Golfer.handedness` without a seam that does
not exist. That is the next decision, not the next commit.
**Blockers**: none. Nothing here needed the bay session.

---

## 2026-08-12 — A band is not a target, and "outside" is not a verdict (career mode, step 6)

**Duration**: ~1 session, implementation + tests
**What prompted it**: the last step of the milestone. Steps 1–5 built the whole mechanism and left
it unreachable from anything but three dev CLIs. Step 6 is the surfacing — the two MCP tools ADR-006
deferred, a page, and the personal-vs-tour join that turns "a consistent 0.31 of head sway" into
"and that sits inside tour range".

**What landed**: `contracts/comparison.py` + `analysis/comparison.py` (the join),
`storage.corpus.narrow_to`, `mcp/career.py` with `get_golfer_profile` / `get_shot_trends` /
`compare_sessions`, `GET /api/golfers/{id}/career`, `static/career.html`, an "Against your own
history" block on the swing page, `READING_A_PERSONAL_HISTORY` in `contracts/caveats.py`, and a
`vs tour` row in `career_baseline.py`. 540 tests (up from 501), ruff and mypy clean.

**The join is decided by the interval, like everything else in this milestone.** `Standing` is read
off the mean's 95% CI, never the mean: whole CI inside p10–p90 is `inside`, whole CI past an edge is
`outside`, anything crossing is `straddles` — which is "unresolved", not "borderline". A center a
hair past p90 with an interval over it has not been shown to be outside anything, and reporting so
would be a placement that flips on the next swing.

**The four target-less metrics never needed targets.** Going in, the plan was that `tempo_ratio`
could acquire a bias finding via "a one-line edit to `METRIC_TARGETS`". It could, and it would have
been wrong: the recorded reason for its missing target is that *its target is a band*, and asking
whether the center's CI sits inside p10–p90 is that band's own question asked directly. A point
target would have meant declaring the midpoint of 2.72–4.71 to be what good is — a claim with
nothing behind it. `METRIC_TARGETS` is untouched and the join answers what the targets were wanted
for.

**Five things found by building it:**

1. **The one metric a personal baseline can read is the one the tour band cannot.** Step 4's finding
   was that `head_hip_offset_impact_norm`'s sign is readable personally, because a personal corpus
   is single-handed by construction. The stored distribution is cut from GolfDB's *mixed*-handedness
   population — the exact reason M6.5 blocked it as a checkpoint. So the join must refuse the only
   metric that has both a center and a distribution. A naive implementation returns a confident
   percentile here and every "did we get a row back" test passes.
2. **`sd` against `sd` looks like a free second finding and is a category error.** `Distribution`
   carries an `sd` one field from the p10 already being read. But it is *between-player* variation
   (458 clips, 122 players, under four each) against a personal *within-player* one. "Your spread is
   tighter than the tour's" compares one golfer's repeatability to how much a field differs from
   itself, and is true for everybody. Recorded in `unavailable` rather than computed — and only once
   the golfer actually has a spread, since that is when the absence becomes a question.
3. **"Outside" is a verdict for half the panel and its opposite for the other half.** Caught by
   rendering the speaking path over a synthetic 18-swing corpus: `finish_balance_norm` sits *below*
   p10 on a one-sided `[0, high]` band — better balance than tour — and the page printed "outside
   tour range" in amber, identically to a center above p90. Fixed structurally rather than in
   wording: every standing pill is neutral (the contract carries no `score` and no `passed` so it
   cannot read as a verdict, and colour was re-adding one), and `outside` always names its side.
   This is step 5's defect one milestone later, and again only a render caught it — every assertion
   was green.
4. **A refusal keeps its evidence, and a model can do arithmetic.** `n`, `n_sessions` and the
   per-session counts stay populated because that is what makes a refusal actionable. Handed
   "session A: 4 swings, session B: 5" and a withheld mean, a model can average them and narrate the
   trend the guard just declined. Nothing in a payload stops it, so `READING_A_PERSONAL_HISTORY`
   names the move and forbids it — kept *separate* from the block `feedback/coach.py` gets, which
   writes about one swing and would be reading rules for tools it does not have.
5. **The import-boundary test everyone would write cannot work.** `analysis.baseline` and
   `analysis.dispersion` promise to import no `benchmarks`, and the obvious check — import them in a
   subprocess, inspect `sys.modules` — fails immediately: `analysis/__init__.py` imports `engine`,
   which reads the bands, so importing *anything* under `golf_coach.analysis` pulls them in before
   the module body runs. Had it happened to pass it would have been passing for the wrong reason
   forever. The property is about what those two files import, so the test parses their source.

**The window and the two sessions turned out to be one operation.** `get_shot_trends` narrows by
date, `compare_sessions` narrows by session id, and both hand the result to the same
`build_baseline` — so a per-session mean faces the identical CENTER floor a pooled mean faces, asked
of less data, and no second threshold exists to drift. `narrow_to` recomputes `metric_counts` rather
than filtering the swing list beside a stale count, which is step 4's pooling-vs-counting lesson
arriving by a different route.

**One output looked like a bug and was the dedupe rule working.** `compare_sessions` on the real
disk reports session `2026-08-09` with a mean score of 94.9 and **zero samples on every metric** —
its clips are re-uploads of a swing already counted in an earlier session, so it scores normally and
contributes no `n`. Left unexplained a reader concludes the corpus reader is broken, so the payload
says it in a sentence.

**Verified**: 540 tests, ruff and mypy clean. Against the real disk all three career CLIs still
agree on `n = 2` for every metric, the tour join refuses all eight placements (five waiting on `n`,
three blocked for want of a population), and the MCP server registers eight tools with a registry
and five without. Both pages were rendered head-less against a real payload and against a synthetic
18-swing corpus, and that second render is what caught defect 3.

**Where I left off**: career mode is complete. The milestone is `6/6` and moved to ROADMAP's Done
section, with the caveat stated there: it is finished and currently silent.
**Blockers**: none to build, for the first time in this milestone. Everything now waits on one bay
session — 20–30 swings with shots attached — after which every surface built here starts speaking
and the tolerances in `METRIC_TARGETS` are the first thing to revise against real repeats.
**Notes**: the em-dash/cp1252 note still applies to all three career CLIs (`PYTHONIOENCODING=utf-8`).
Two test modules briefly collided on the basename `test_career.py` — `tests/` has no `__init__.py`,
so basenames must be unique across test packages; they are `test_career_route.py` and
`test_career_tools.py`.

---

## 2026-08-12 — The shape of a miss, and a reading meant for one metric (career mode, step 5)

**Duration**: ~1 session, implementation + tests
**What prompted it**: step 4 built the baseline and made `pooled_samples()` public specifically as
this step's door. Step 5 is the argument the whole milestone rests on: this instrument cannot see
*why* a face is open — grip, lead wrist and release are all invisible to it — but the **shape** of a
miss is itself evidence. A miss that repeats at the same size is produced by something that is the
same every swing; a miss that moves cannot be. Those have different fixes, and separating them needs
no view of the body.

**What landed**: `contracts/dispersion.py` (`Finding`, `DispersionPattern`, `MetricTarget`,
`METRIC_TARGETS`, `PATTERN_READING`, `MetricDispersion`, `GolferDispersion`),
`analysis/dispersion.py` (`build_dispersion`, `dispersion_for`), `scripts/career_dispersion.py`, and
`analysis/baseline._refuse` promoted to `refuse`. 501 tests (up from 481), ruff and mypy clean.

**Two findings, never one verdict.** `bias` (the center is further from the target than measurement
error explains) and `scatter` (the spread is larger than measurement error explains) are answered
independently, because the *contrast* is the whole signal — the same 6° average miss means opposite
things at sd 1 and at sd 9, and collapsing them into one number throws away exactly the thing this
step exists to read. Both are decided by an interval and never a point estimate: bias needs the
mean's 95% CI to clear the tolerance band entirely, scatter needs the sd's CI **lower** bound to
clear it. So a tolerance set wrong surfaces as `NOT_ESTABLISHED` — an honest "cannot tell" — rather
than as a confident wrong pattern. No new statistics: `mean_ci` and `sd_ci` already existed.

**Five things found by building it:**

1. **The reading was written for one metric and printed for all eight.** The `BIASED` text named
   the checks outright — "grip, alignment, ball position, face at address" — which reads correctly
   under `face_to_path_deg` and is nonsense under `head_sway_norm`, where it printed **unchanged**.
   Found by rendering the speaking path on a synthetic session rather than by reasoning about it:
   the tests all passed while the output would have sent a golfer to check their grip about a head
   that moves. One reading serves every metric, so it may name only the class of cause; naming the
   specific check needs per-metric vocabulary and belongs in `feedback`. There is now a test that
   asserts the pose and shot readings are the *same string* and that it names no specific check.
2. **Six of the eight tolerances were already measured — they just had never been written down.**
   `tune_spatial_metric.py` computes `noise` + `bound`, which is precisely "the smallest difference
   distinguishable from this pipeline's own error", and it prints them and stores nothing. Re-ran it
   over the 461-clip face-on corpus (both estimator caches were already on disk, so no extraction):
   0.024 for `finish_balance_norm` up to 0.943 for `tempo_ratio`. Tempo's `noise` column is
   **0.000**, which is structural rather than lucky — it reads phase instants only and both
   estimators were handed GolfDB's labelled ones, so its entire error term is our own address
   detection, the weakest instant we have. The two launch-monitor metrics have no analogue for a
   photographed screen, so 2.0° is judgment, recorded as judgment.
3. **The tolerance does double duty, and both uses are the same quantity used correctly.** It bounds
   how far a center must sit from the target before a bias is real, and — because measurement error
   inflates observed spread (`sd_obs² ≈ sd_true² + sd_err²`) — it is also the level a spread must
   exceed before the scatter is the golfer's rather than the instrument's. A sample sd sitting at
   the tolerance is exactly what a perfectly repeatable golfer measured by this pipeline produces.
4. **Four of the eight may carry a scatter finding and must not carry a bias one.** Declaring a
   target is declaring what *good* is, which this repo does in exactly one place: a band with a
   derivation behind it. `hip_sway_norm` / `hip_shift_at_top_norm` have no band and "less is better"
   is not established for either; `head_hip_offset_impact_norm` has a readable sign (step 4's
   finding) but no known right amount; `tempo_ratio`'s target *is* a band, and reading it here would
   import `benchmarks` into the personal-baseline path — the boundary step 4 held deliberately. Each
   is refused with its reason attached, in `unavailable` rather than `withheld`, because the two
   need opposite responses: one says book another bay hour, the other says this needs a band first.
5. **A bias on a one-sided magnitude asserts less than it reads.** Zero head sway is unattainable,
   so "distinguishable from 0" is established for every golfer alive. What it actually says is *a
   consistent amount, above measurement error* — the half of the contrast this step needs — and not
   that the amount is too much. That is the tour band's question. Documented on the two entries it
   applies to, because the finding is true and the obvious reading of it is not.

**The guard is inherited, not re-invented.** `MetricDispersion` is built from the `MetricBaseline`
step 4 already sealed, so when `CENTER` was refused there is no mean *in the input* to test a bias
against — absent rather than ignored, which is "withheld means absent" holding by construction one
level further out. The one thing raw samples are read for is the within-session spread, and that is
itself gated on `SPREAD` so it cannot become a route around the seal. Only `_refuse` had to become
public; a second copy of the guard is how two floors drift apart with the looser one deciding what
gets said, which is step 4's own `artifact_key` lesson repeating.

**Pooled spread mixes two different quantities**, and the cause reading is only about one of them:
"your release is inconsistent" is a claim about one bay hour, while a pooled sd across sessions also
contains whatever the golfer changed in between. When two sessions each carry ≥2 samples the pooled
within-session sd is computed alongside, and a caveat fires when the two diverge. Classification
stays on the pooled figure — a proper variance decomposition has its own `n` requirements and there
is no corpus to test one against, so naming the possibility is the honest amount to say today.

**Verified**: 501 tests, ruff and mypy clean. `tests/analysis/test_baseline.py` passes with **no
assertion moved**, which is the check that making `refuse` public changed nothing. Against the real
disk, `career_dispersion.py --name Aaron` lists all eight metrics at n=2 with both findings withheld
and each naming its shortfall, and `career_corpus.py` / `career_baseline.py` / `career_dispersion.py`
agree on `n` for every metric. The speaking path was rendered separately over a synthetic 12-swing
session — a consistently open face reads `biased`, a wandering start line reads `scattered`, and
that render is what caught defect 1.

**Where I left off**: step 6, the last one — `get_shot_trends` / `compare_sessions`, the results
page, and the personal-vs-tour join. That join is what turns "a consistent 0.31 of head sway" into
"and that sits inside tour range", and it is also what would let the four target-less metrics
acquire a bias finding: one line each in `METRIC_TARGETS`.
**Blockers**: none to build. Everything after this wants the bay session — 20–30 swings with shots
attached.
**Notes**: the em-dash/cp1252 note from step 4 applies to this CLI too (`PYTHONIOENCODING=utf-8`),
unchanged for the same reason — it is a terminal setting, and it now affects all three career CLIs
equally.

---

## 2026-08-12 — A baseline that refuses to speak (career mode, step 4)

**Duration**: ~1 session, implementation + tests
**What prompted it**: steps 1–3 assembled the inputs and step 3's re-run brought `n` to 2 for all
eight metrics. Step 4 is the consumer: turn a `CareerCorpus` into per-metric statistics, and refuse
to report them below a threshold.

**What landed**: `contracts/baseline.py` (`PersonalBaseline`, `MetricBaseline`, `BaselineClaim`,
`WithheldClaim`, `SessionSample`, `MetricSample`, and the threshold table), `analysis/baseline.py`
(`build_baseline`, `pooled_samples`), `mean_ci` / `sd_ci` / `mean_and_sd` in `analysis/stats.py`,
and `scripts/career_baseline.py`. Against the disk it refuses all 24 claims (8 metrics × 3), each
naming what it waits for. 481 tests, ruff and mypy clean.

**The gate is per (metric, claim).** `CENTER`, `SPREAD` and `TREND` have different appetites for
`n` — the sample sd is a noisier estimator than the sample mean (relative error `1/sqrt(2(n-1))`,
still 24% at n=10), and a trend needs repeated *occasions* rather than repeated swings, so it gates
on `n_sessions` separately. One threshold per metric would either block a defensible mean or ship a
spread the data cannot support.

**Every statistic carries a 95% CI**, Student-t for the mean and chi-square for the sd. No scipy on
the base install (ADR-008), so both are small hardcoded critical-value tables for df 1..30 with
Cornish-Fisher / Wilson-Hilferty fallbacks beyond — pinned in `test_stats.py` against published
values and hand-computed intervals.

**Four things found by building it:**

1. **Pooling could have disagreed with counting, invisibly.** Step 2's dedupe rule lived inside
   `storage/corpus.py::_count_metrics`, which returned *counts and not values*. A baseline
   iterating `swing.measurements` naively would have averaged a different number of values than the
   `n` printed beside it — and only where the rule matters (a re-uploaded clip; one shot photo
   across two real swings), which is to say only where nobody would see it. Lifted the rule to
   `CorpusSwing.artifact_key`, called by both sides; the existing 21 corpus tests passing unchanged
   is the check that nothing moved, and a new end-to-end test pins
   `metric_counts[name] == baseline.metrics[name].n`.
2. **M6.5's spread/error ratios cannot set the thresholds.** The obvious move, and wrong: that
   ratio is *population* spread over *instrument* error, while what binds a personal baseline is
   the golfer's own shot-to-shot variability — unmeasured and much larger. Deriving from tempo's
   r = 2.4 gives a usefully-resolved personal mean at n ≈ 3. So the floors are judgment, documented
   as such, and the CI is what makes that safe: too low a floor reads as a visibly wide interval,
   not as a confident wrong number.
3. **Withheld had to mean absent, not flagged.** A statistic shipped beside `ready: false` is one
   forgotten conditional away from being rendered. Gated fields are `None` — `Measurement`'s
   "structurally incapable of reading as a verdict", one level up. What stays populated is the
   evidence (`n`, `n_sessions`, per-session counts), because that is what makes a refusal
   actionable rather than merely silent. The CLI demonstrates it: it never calls `supports()`, it
   just checks whether there is a number.
4. **`head_hip_offset_impact_norm` is readable here and nowhere else.** M6.5 blocked it as a
   checkpoint because its sign is camera-relative and a GolfDB band over mixed handedness would be
   meaningless. A personal corpus is single-handed by construction, so a personal baseline reads
   the sign without consulting `Golfer.handedness` at all.

**Where I left off**: step 5 (dispersion as cause discriminator) reads `pooled_samples()`. Step 6
surfaces it, and is where the personal-vs-tour join lands — `MetricBaseline` mirrors
`Distribution`'s shape so that is a lookup, not a redesign. `TREND` is gated but exposes only the
per-session breakdown; the slope and its significance wait for a corpus to test against.
**Blockers**: none to build. The `n` still needs a bay session — 20–30 swings with shots attached.
**Notes**: pre-existing and cosmetic — both career CLIs print em-dashes, which the Windows console
garbles at cp1252. `PYTHONIOENCODING=utf-8` fixes it; not changed here since it affects
`career_corpus.py` equally and is a terminal setting rather than a code defect.

---

## 2026-08-12 — The backfill, and the staleness nothing could see (career mode, step 3)

**Duration**: ~1 session, implementation + the real backfill over the four swings on disk
**What prompted it**: step 2's counter printed `n = 1` for all eight metrics, and named its own
worklist: three of the four `analysis.json` predate M6.5 and carry no `measurements`. Re-running
them was supposed to be the whole job.

**It was, but "which ones need re-running?" turned out to be unanswerable.** The only signal was
`measurements: []`, and that works *once*, by accident — M6.5 happened to add a field. The
counterexample is three entries below this one: the 2026-08-09 `_DRAWDOWN_FLOOR` fix moved a
stored tempo from 0.43 to 2.42 and changed the shape of nothing. An artifact from before it is
indistinguishable from one after it, and `AnalysisState.matches` cannot help — it compares the
*inputs*, which a re-analysis does not change. That is a live hazard for step 4 rather than a
tidiness complaint: a `PersonalBaseline` reads **spread**, so pooling two engine generations
manufactures variance out of a code change. Same failure as counting a re-uploaded clip, inverted.

1. **`contracts/swing.ANALYSIS_VERSION`** + `SwingBundleResult.analysis_version`. The default is
   **0, not the current version**, and that is the load-bearing part: the field is read back by
   parsing artifacts written before it existed, and a default of "current" would make every legacy
   file claim to be up to date — the one wrong answer indistinguishable from a right one. The
   engine sets it explicitly; `api/state.py` gains `stored_analysis_version` / `is_outdated`.
2. **`ExclusionReason.OUTDATED`** and `CareerCorpus.outdated_swings`. `counts_toward_metrics()`
   now asks three questions of three different things — the artifact (`analyzed`), the bytes under
   it (`stale`), the code that joined them (`outdated`).
3. **`scripts/reanalyze.py`** — targets whatever needs it, `--dry-run`, `--all`, `--video` and
   `--coaching` off by default. Walks the bundle store rather than `read_corpus`, deliberately:
   the corpus collapses re-uploads, but each of those directories still holds an `analysis.json`
   the results page and the MCP server will serve.
4. **`analyzed_without_measurements` survived, narrowed.** It gates on `counts_toward_metrics()`,
   so a pre-M6.5 artifact is now `outdated` and no longer lands there. What is left is
   "current-engine swing where every metric returned None" — measurement failed rather than the
   engine being old, a different problem with a different fix. Both read 0 now.

**The bug I found while making sure the backfill would not create one.** `analysis.state.json` is a
denormalised copy of `analysis.json` — the upload page's 5-second poll reads it so it does not
parse a 7 KB file per swing. Only `api/worker.py` wrote it, and `analyze_swing_dir` did not. So
every CLI re-analysis desynced them, and `2026-08-09/2` had been sitting for three days with a
sidecar reading **66.67** and the pre-fix "Tempo too quick - 0.4:1" headline beside an analysis
reading **94.92** / "2.4:1" — the session list showing 67/100 for a swing whose results page showed
95/100, with nothing anywhere able to flag it. Step 3 is a bulk CLI re-run, so shipping it as
planned would have produced two more copies of this while fixing something else. `record_state`
now lives in `api/pipeline.py` and runs as part of writing the analysis; the worker keeps
`queued` / `running` / crash-`failed`, which are the three states no analysis on disk corresponds
to. `tests/api/test_state.py` is new and pins the invariant, including that `matches()` returns
True across the desync — the reason nothing caught it.

**Key decisions**:
- **A re-run rewrites the whole artifact, and that is the honest choice.** A measurement-only patch
  was the cautious-looking option and is worse: today's code computing measurements while
  yesterday's checkpoint scores stay put yields one file whose `checkpoint_scores[tempo].observed`
  and `measurements[tempo_ratio].value` both claim to be this swing's tempo and can disagree.
- **`--video` off by default, with a check instead of an assumption.** Pose keypoints and shots are
  both content-addressed caches, so a re-run is seconds; the render is minutes. But a moved
  alignment anchor would leave `aligned.mp4` disagreeing with the JSON beside it, so the script
  compares anchors across the run and says so. They did not move here — which is exactly why it
  should be checked rather than assumed.
- **All four swings were re-run, including the one that already had measurements.** It was written
  before the stamp existed, so it cannot prove it is current, and "cannot prove it" is the whole
  rule. The corpus is now a single generation.
- **No ADR.** Nothing cross-cutting was decided; the rule lives in the `ANALYSIS_VERSION` and
  `ExclusionReason.OUTDATED` docstrings, which is where anyone meets it. Next free number stays 017.

**Two mistakes in my own reporting, both caught by running it.** The first run warned that the
alignment anchors had moved on all four swings and printed `window [574, 790] -> (574, 790)` as a
change. Neither was real: `_anchors` was reading the analysis dict one level too high so the
"before" side was always unknown, and the window row was comparing a JSON list against a tuple by
their string forms. Worth recording because the failure mode is the one this repo keeps meeting —
a diff tool that reports drift it invented is indistinguishable from a real regression, and I very
nearly re-rendered four videos to fix nothing.

**Verified**: 442 tests (up from 426), ruff and mypy clean. Against the real four swings:
`reanalyze.py --dry-run` found all four outdated, the run produced `version 0 -> 1` and
`measurements 0 -> 8` with **no other field moving** — scores, checkpoints, phases, windows and
anchors all identical to the pre-run backup — and a second plain run reports nothing to do.
`career_corpus.py --name Aaron` now prints **`n = 2` for all eight metrics**, no outdated swings,
and nothing excluded but the two known duplicates. Through the real API: session-list score and
results-page score agree on every swing, `2026-08-09/2` included. `aligned.mp4` mtimes untouched.
`mcp.query.get_swing("2026-08-07-aaron1", "1")` returns all eight measurements.

**Where I left off**: step 4 — `PersonalBaseline` and the per-metric minimum-N guard. Pure
functions over `CareerCorpus.metric_counts`, testable on synthetic input, no bay session needed to
*build*. The two `face_to_path_deg` samples now on disk are 10.9 and 13.2, which is what step 5
will consume and nowhere near enough to conclude anything from.
**Blockers**: none for step 4. Steps 5–6 want the bay session.

---

## 2026-08-11 — The corpus reader, and a dedupe key that only works one way (career mode, step 2)

**Duration**: ~1 session, implementation + verification against the real four swings
**What prompted it**: step 1 made "who swung this" addressable; nothing could *assemble* it. Every
reader in the repo is per-session — `get_session`, `get_session_summary` — so "every swing Aaron
has ever hit" was not a question the code could answer, and steps 4-5 are pure math over exactly
that list.

**The counting is the feature.** Four swing directories hold two swings: the same three files were
re-uploaded three times while the upload path was being tested. Counting directories would hand a
personal baseline one swing's numbers three times, which does not merely inflate `n` — it drives
the variance toward zero, and variance is the single quantity career mode exists to read (a tight
spread points at a static cause, a wide one at timing). The most confident possible wrong answer,
manufactured out of a testing artifact.

1. **`contracts/career.py`** — `CareerCorpus`, `CorpusSwing`, `ExcludedSwing`/`ExclusionReason`.
   `CorpusSwing.measurements` carries `Measurement` whole rather than flattening to name -> value
   the way `query._measurements` does, because `source` is what decides the sample count.
2. **`storage/corpus.py`** — `read_corpus(sessions_dir, player_id)`. Pure reads over artifacts that
   already exist; no file is re-hashed, since `RoleFile.content_sha256` was recorded as the upload
   streamed in.
3. **`scripts/career_corpus.py`** — the honest-`n` counter, printing per-metric `n`, the collapsed
   re-uploads, and everything contributing nothing, with reasons.
4. **`SwingBundleStore.list_session_ids()`** — the same listing existed inline in
   `mcp/query.list_sessions` and `scripts/backfill_golfer._sessions`; this would have been a third.

**The design error the test caught, and it was mine.** I argued for two dedupe keys — pose metrics
counting distinct face-on clips, launch-monitor metrics distinct shot photos — and the case I used
to justify it was one clip re-uploaded with a different shot photo attached, which I claimed was
one pose sample and two shot samples. The test asserting `n=2` failed, and it should have: the
duplicates share the face-on bytes, so they are one physical swing, and one swing produced one ball
flight. The second photo is misattached. Counting it would have put a `face_to_path_deg` into a
dispersion that no swing ever produced — the exact class of error the milestone is being built to
avoid, arrived at while arguing for the mechanism meant to prevent it.

The two keys are still right, but they diverge in only **one** direction: `bundle_store`'s "newest
swing missing this role" rule can attach one photo to two genuinely different swings, which is two
pose samples and one shot sample. The other direction is a data conflict, reported as
`conflicting_shots` for repair — the same posture `bundle_store` already takes toward a
misattributed upload, which it documents and hands to a human rather than engineering around.

**Key decisions**:
- **No fallback to `checkpoint.observed`.** Three of the four `analysis.json` predate M6.5 and
  carry `measurements: []` while their checkpoints still hold `observed` (tempo 2.35, head sway
  0.25). Reading those as measurements would have lifted `n` to 2 for three metrics immediately and
  mixed two derivation paths under one name. Reported as `analyzed_without_measurements` instead,
  which is precisely step 3's worklist.
- **`excluded` means "contributes no sample", not "absent from the corpus".** An unanalyzed or
  stale swing is a real distinct swing carrying no usable numbers yet; both are fixed by re-running
  the pipeline, so they are reported as work rather than as absence.
- **Survivor of a duplicate group is the earliest arrival.** A re-upload's timestamp dates the
  upload, so taking the latest would file a swing under the day someone retested the upload path.
- **`storage` imports `api.state`**, which inverts ADR-008's direction. Deliberate, and the
  precedent `mcp/query.py` set: a second copy of a tolerant reader is a second copy that drifts.
  The honest fix — moving `load_analysis`/`load_state` to `storage/analysis_io.py` — is contained
  and is not this commit.

**Verified**: `python scripts/career_corpus.py --name Aaron` reports 4 directories -> 2 distinct
swings, 2 distinct shots, 2 collapsed re-uploads of `face_on 91b9d32c`, and `n = 1` for all eight
metrics. That last number is the point: it is what the ROADMAP already asserted in prose, now
produced by code, and it makes step 3's scope self-evident. Full suite green (426 tests), ruff and
mypy clean.

---

## 2026-08-11 — Who swung this, recorded before the bay session (career mode, step 1)

> **Written 2026-08-12, after the fact.** This session shipped without an entry; the record below is
> reconstructed from the ROADMAP §Career step-1 paragraph, which was written at the time. It carries
> only what that section already states — no findings have been added from memory.

**Duration**: ~1 session, implementation + a backfill over the four swings on disk
**What prompted it**: career mode was about to be declared "nothing to build, only `n` to collect",
and that was wrong about exactly one thing — **capture-time metadata**. Everything else career mode
needs is derivable from artifacts after the fact, so it can be built whenever. Who swung a clip and
which way they face are *recorded or lost*. Those had to land before the bay session, not after it.

**What landed**: `contracts/golfer.py` (`Golfer`, `Handedness`, `slugify`), a flat-file registry in
`storage/golfer_store.py`, a per-session cursor in `storage/session_meta.py`, `player_id` stamped
**write-once** onto `SwingManifest`, a golfer bar on the upload page, and `scripts/backfill_golfer.py`.
The backfill has been run: all four existing swings are `aaron`, right-handed.

**Uploads are deliberately never blocked on it.** A phone at a bay uploading three files is the one
moment in this system where friction costs data that cannot be recovered — the swing is over. So an
unlabeled upload is accepted, setting a golfer *adopts* the swings that arrived without one, and each
swing row carries a repair link. Identity is a thing you attach, not a gate you pass.

**Handedness is on the record for a reason that only pays off later.** It is what will eventually let
`head_hip_offset_impact_norm`'s camera-relative sign be interpreted, since the GolfDB band behind it
is cut from a mostly right-handed population.

**Where I left off**: step 2, the cross-session corpus reader — every reader in the repo is
per-session, so "every swing Aaron has ever hit" was not yet a question the code could answer.
**Blockers**: none.

---

## 2026-08-11 — Claude writes the verdict, and a percentile of 90 printed as 9 (M6)

> **Written 2026-08-12, after the fact.** This session shipped without an entry; the record below is
> reconstructed from the ROADMAP §M6 section, which was written at the time. It carries only what
> that section already states — no findings have been added from memory.

**Duration**: ~1 session, implementation + tests against a fake client
**What prompted it**: the analysis produces numbers, bands, percentiles and ranked tips, and a golfer
still has to assemble them into a sentence. M6 is the sentence.

**What landed**: `feedback/coach.py` (prompt template + the API call), `contracts/caveats.py`,
`CoachingProvenance` on `contracts/feedback.py`, the coaching card in `api/static/results.html`, and
`coaching_model` / `coaching_enabled` in `config.py`. The model moved to **`claude-opus-5`** — the
2026-08-10 entry below flagged `claude-opus-4-8` sitting behind a comment claiming "latest, most
capable" and deferred it to here, which is where it belongs.

**The brief is rendered, not dumped.** Every value is labelled with the vocabulary the caveats warn
about — `unscored`, `percentile`, `needs_review`, `alignment_caveat` — so a warning about `unscored`
lands beside a line that actually says `unscored`. Keypoints and phases are excluded: several hundred
frames of landmarks that no coach reasons from and that would dominate the prompt.

**It never raises for an expected failure.** No key, no `llm` extra, a rate limit, a refusal, a
truncation — each returns a *reason* that becomes a note on the result. Coaching is the last thing
that happens to a swing and the least important thing about it; it must never be able to cost a
golfer their score.

**One source of truth for the caveats.** The standing warnings moved to `contracts/caveats.py` and
are composed by **both** `mcp/server.py` and `feedback/coach.py`. ADR-008 forbids either importing
the other, and the alternative was two copies of load-bearing prose drifting apart — which this repo
has been bitten by three times.

**The bug, found by running it rather than reasoning about it.** The first cut trimmed trailing zeros
unconditionally, so a percentile of **90 rendered as 9** and 10 as **1**. Plausible, wrong, and aimed
straight at a model that would have repeated it as fact. Chasing it exposed a second, real inaccuracy
in the caveat *text*: it claimed every failing checkpoint reports "about 90", but a two-sided metric
that misses **low** clamps to 10 — as tempo does on `2026-08-10/2`. Both fixed, the second for the
MCP server too.

**Display is gated on provenance, not just on text.** `results.html` renders the paragraph as a
tinted card, never as a fourth `.tip`, with the model named *above* the prose. Unattributed prose on
a page of measurements is indistinguishable from a measurement.

**Verified**: full suite green, ruff and mypy clean — against a **fake client**. A swing analyzed
with a key configured carries `feedback.coaching_text` and `feedback.coaching` (model, timestamp, and
a sha256 of the brief, so a stored result can tell when the numbers moved underneath it).

**Where I left off**: the live path is unproven. **No real request has ever been sent** — no
`GOLF_ANTHROPIC_API_KEY` is configured. Put a real key in `.env`, run
`python scripts/analyze_bundle.py 2026-08-10/2 --no-video`, and read the paragraph against the
numbers above it: it must assert nothing the brief did not contain, and must not mention spine angle,
hip rotation, swing plane or club path — none of which this system measures.
**Blockers**: an API key, and nothing else.

---

## 2026-08-11 — Measure now, judge later, and the circle that kept the panel at three (M6.5)

> **Written 2026-08-12, after the fact.** This session shipped without an entry; the record below is
> reconstructed from the ROADMAP §M6.5 section, which was written at the time. It carries only what
> that section already states — no findings have been added from memory.

**Duration**: ~1 session, implementation + a full re-derivation over the 461-clip face-on corpus
**What prompted it**: the panel had sat at three checkpoints for two milestones, and the reason was
not that any metric was hard. **Measurement and judgment were fused.** `evaluate_head_sway` measured,
resolved a band, scored, and returned `None` if *any* step failed — so a metric with no band could
not be measured at all, while bands are derived from populations of measurements. That circle is the
blocker, and it is a code-shape problem rather than a data problem.

**What landed**: `analysis/measure.py` and `analysis/shot_measure.py` (the measuring half — pure, no
`resolve_range`, where `None` means *could not measure*), the `Measurement` contract,
`scripts/golfdb/tune_spatial_metric.py`, and a registry-driven `derive_pose_metrics.py`.
`checkpoints/mechanics.py` keeps the judging half and the three evaluators are unchanged in
behaviour — pinned by the existing `tests/analysis` suite passing with **no assertion moved**.

**`Measurement` is ADR-010 §2 expressed as a type rather than a convention.** No band, no `passed`,
no `score`. A metric earns a verdict only once a population exists to judge it against, and until
then it is data — so a measurement is made *structurally incapable* of rendering as a fault, however
it travels.

**Five new metrics, and the ones deliberately left out matter as much.** Three face-on pose:
`hip_sway_norm`, `hip_shift_at_top_norm`, `head_hip_offset_impact_norm` — all hips/shoulders/ears,
all windowed, all `x`-over-`x` and so immune to the 16:9 pixel-aspect assumption. Vertical and
shoulder-tilt metrics were excluded for being aspect-*sensitive*; ankles and knees for having zero
recorded reliability evidence. Two launch-monitor: `face_to_path_deg` and `start_line_deg`.
`smash_factor` and `club_head_speed` are excluded because every shot on disk reads smash 0.89–1.00 —
ball speed *below* club speed — and the OCR is faithful, so the **simulator itself** is printing a
physically impossible number. `spin_axis` too: its sign contradicts the contract and the parser warns
that it stored an uninterpreted magnitude.

**The gate the repo did not have.** There were three harnesses for *temporal* rules and none for
spatial quantities, so "should we add this checkpoint?" was answerable only by argument.
`tune_spatial_metric.py` scores population spread against measurement error, where error is estimator
disagreement (`lite` vs `full`, both already cached for all 461 face-on clips — no new extraction)
plus segmentation error (labelled vs detected instants). All six pose metrics clear it
(spread / error): `finish_balance` **8.2**, `head_hip_offset_impact` **7.6**, `hip_sway` **7.1**,
`head_sway` **6.7**, `hip_shift_at_top` **3.6**, `tempo` **2.4**.

**The harness validates itself three ways**, which is the only reason to believe the numbers above:
it reproduces `head_sway_norm`'s shipped band (p10–p90 **0.029–0.430** against `ranges.json`'s
0.0–0.43), `finish_balance_norm`'s p90 (**0.287** against 0.29), and the face-on `tempo_ratio` p90 of
**5.000** that `mechanics.py` documents against the all-view 4.71.

**Two things found by running it:**

1. **Normalizing the simulator's shape text matched `CENTER` before `FADE`**, classifying a real
   recorded `"CENTER SLIGHT FADE"` as **straight**. Curvature words now beat centering words.
2. **The corpus had been storing `CheckpointScore.observed`, which is rounded to 2dp.** Measurements
   now carry full precision. Re-deriving moves 42 distribution rows by under 0.005 and leaves every
   shipped band unchanged at the precision it quotes — which is the check that says this was a
   latent-precision bug rather than a change of answer.

**`head_hip_offset_impact_norm` is signed and camera-relative.** It is empirically *not* bimodal on
this corpus (p10 −0.88 to p90 −0.33, consistently head-behind-hips), so a band is derivable — but
that is a fact about GolfDB's handedness mix, not a guarantee, and handedness has to be resolved
before it can become a checkpoint. `derive_reference.py`'s recommended band for it is also garbage
(`low=0.00 high=-0.33`): its one-sided heuristic assumes non-negative values.

**Where I left off**: nothing is scored and `ranges.json` is untouched — **promotion is deliberately
a separate decision**, and it wants more than one golfer's swings behind it. Exit criteria for the
milestone are otherwise met: every measurable quantity is recorded on every analyzed swing, with a
harness that says which of them a band is worth deriving from.
**Blockers**: none to build.

---

## 2026-08-10 — The MCP server, and a tool list three milestones out of date (M3)

**Duration**: ~1 session, implementation + verification against real bay data
**What prompted it**: Looking for the next task. The MCP server was the last M3 item, fully
unblocked — `mcp>=1.0` declared in the `llm` extra since March, `scripts/run_mcp_server.py` a
`NotImplementedError` stub, ADR-006 accepted. Straightforward. It was not.

**ADR-006's tool list predated everything that makes this project worth querying.** All five tools
it specifies — `get_recent_shots`, `get_shot_by_id`, `get_session_summary`, `compare_sessions`,
`get_shot_trends` — are shot-only, written 2026-03-16 when M3 *was* the system. Since then the
pose-only track delivered scored swings with tour percentiles and ranked tips, and M7 Phase 4
started writing `analysis.json` per swing.

So the shot metrics are now the **less** differentiated half of what this repo holds. They are HD
Golf's own readout, photographed and OCR'd — the simulator already shows them on a screen in front
of the golfer. What only this system can say is where a swing sits against 458 tour swings. Built
to the table as written, Claude could report "club speed 98.3, carried 121 yards" and would have
been unable to say "your head sway sits higher than 83% of tour swings." That inverts the point of
a coaching interface, so the tool set covers both axes of ADR-009's model. ADR-006 has a second
addendum recording it; no new ADR, because nothing was re-decided, only re-scoped.

The join turned out to be free: `data/processed/shots/*.shot.json` carries `session_id`, and the
bundles live at `data/processed/sessions/<session_id>/<swing>/`.

1. **`mcp/query.py`** — reads sessions, swings and shots. Imports no MCP SDK, so it installs and
   tests on the base install; a subprocess test pins that the way `test_pipeline_imports.py` pins
   fastapi out of `api/pipeline.py`. Reuses `SwingBundleStore`, `api/state.py`'s `load_state` /
   `load_analysis`, and `ScreenShotDataSource` behind `CompositeShotDataSource` — whose docstring
   has named the MCP server as an intended consumer since M3 shipped.
2. **`mcp/server.py`** — five tools, stdio transport, delegating every call. `settings.mcp_port`
   (8081) stays unwired: under stdio the client launches the process and talks over the pipe.
3. **`scripts/run_mcp_server.py`** — the stub replaced, `mcp` imported lazily so a base install
   gets an instruction instead of a traceback out of a process holding a client's pipe.

**The views are not passthroughs, and that is the substance of the phase.** An LLM presents what it
is handed as fact, so everything this repo knows to be provisional had to survive the trip out:
`needs_review` hoisted from inside `ShotProvenance` to the top level, the alignment tier turned
into an actual sentence about what not to conclude from the side-by-side, and `unscored` labelled
as "dropped, not failed" — with the same three restated in the server's `instructions`, which the
model reads once on connect, alongside a note that spine angle and swing plane are *not* measured
here and must not be inferred.

**Key decisions / surprises**:
- **Two tools from the original table are deliberately not built.** `get_shot_trends` and
  `compare_sessions` both invite Claude to narrate a trend, and the store holds **three** shots. A
  trend tool over n=3 reports noise in a confident voice — the same failure ADR-012 found when a
  120-clip result vanished at 461. Recorded as a decision in the addendum so it does not read as an
  oversight.
- **ADR-008 puts the MCP server in `launch_monitor/`**, which was right for a shot-only server and
  wrong now: it would make the launch-monitor module import `analysis` and `storage` to serve tools
  that have nothing to do with a launch monitor. New top-level `src/golf_coach/mcp/`. (`from mcp.server
  import ...` inside `golf_coach/mcp/` resolves to the SDK — Python 3 has no implicit relative
  imports. It reads like a shadowing bug and is not one.)
- **A tool returning `None` serializes to zero content blocks**, which reads identically to a call
  that silently did nothing — and my tool descriptions were claiming "returns null". Found by
  running it, not by reasoning about it. The three lookup tools now return an explicit `NotFound`
  that names the miss and points at the tool listing valid ids.
- **The listing said "not analyzed" for swings that were analyzed.** The 2026-08-07 sessions have an
  `analysis.json` but no `analysis.state.json` — analyzed by the CLI before Phase 5 introduced the
  sidecar. Reading only the sidecar contradicted `get_swing`, which returns their full result a
  moment later. Falls back to `analysis.json` when the sidecar is absent, which costs one parse on
  legacy swings only.
- **`mcp` 2.0 is not the API in most examples.** `FastMCP` is `MCPServer`, and the wire fields are
  snake_case (`input_schema`, `is_error`) where older versions used camelCase. Checked against the
  installed package rather than recalled.

**Verification**: 294 tests pass (27 new), ruff and mypy clean. Against the real bay data rather
than fixtures: `list_sessions` returns four sessions newest-first, `get_swing("2026-08-10", "1")`
returns **94.9** with all three checkpoints, their bands and percentiles, the `top_impact` caveat
attached, and the joined shot at 125.6 yards carry — matching `analysis.json` exactly.

**Where I left off**: M3's only remaining no-hardware item is tuning OCR preprocessing on a real
range session's photos. The server is ready for M6 to build on.
**Blockers**: None. Not yet registered with a real Claude Desktop / Claude Code client — the tools
were exercised through `call_tool` in-process, which validates the schemas and the data but not the
client handshake.
**Notes**: `config.py:coaching_model` is `claude-opus-4-8` and its comment says "latest, most
capable"; the current model is `claude-opus-5`. Unread until M6, so left alone — fold it in there.

---

## 2026-08-09 — One noise bug behind three separate symptoms (tempo, playback speed, sync)

**Duration**: ~1 session
**What prompted it**: Looking at `results.html?session=2026-08-09&swing=2` and finding three things
wrong at once — the down-the-line panel of the aligned video plays visibly fast, tempo reads 0.4:1
against an eyeballed ~2:1, and the face-on swing visibly starts before the down-the-line swing.

**They were all one bug, and it was not in the video path.** There is no client-side sync to get
wrong: `results.html` plays a single pre-rendered `aligned.mp4` and there is no `playbackRate`
anywhere in the repo. The distortion was baked in at render time by a wrong phase anchor.

`_rising_runs` closed a descent run when the drawdown exceeded `_DRAWDOWN_TOLERANCE` of the run's
**own accumulated rise** — which is near zero in a run's first frames, so any noise clears it. The
golfer hovered at the top; the smoothed lead wrist drifted 0.3205 → 0.3244 over nine frames and
wobbled back 0.0020, which was more than a quarter of the 0.0039 accumulated. That split the
descent, and `_top_and_impact` took the second fragment: **top at 704 instead of 694, a 14-frame
downswing where the truth was 24.**

Everything downstream inherited it:

| | before | after |
|---|---|---|
| face-on downswing | 14 fr (0.234s) | 24 fr (0.400s) — matches DTL exactly |
| scored window (`select_swing` leads by 5 downswings) | (634, 760) | (574, 790) |
| face-on `motion_start` | 698 — 6 frames before the top | 636, still `detected` |
| tempo | **0.43:1** | **2.42:1** |
| DTL panel speed | **1.69×** | 0.99× |
| panel offset at first frame | **+983 ms** | −2 ms |

The tempo number was never a `_motion_start` bug, which is where ROADMAP and the runbook both
predicted it. `_motion_start` was doing its job on a top that was 10 frames late; widen the window
by fixing the top and it finds the takeaway unaided.

**The occlusion hypothesis was wrong, and I nearly built on it.** The 2026-08-07 entry above (and
`phases.py`'s own `_PLAUSIBLE_DOWNSWING_S` comment) blamed the disagreement on the DTL lead wrist
being the far, occluded arm. Measured before planning around it: on DTL the two wrists **agree**
(LEFT → 24 frames, RIGHT → 25). Face-on was the outlier at 14. Three of the four measurements
cluster at a top of ~694 in face-on coordinates; only face-on's LEFT_WRIST said 704. There is no
DTL landmark problem to fix, and no view-aware wrist selection was written.

**Validation.** The floor was swept against the 461-clip GolfDB corpus, paired per-clip against the
floor-0 baseline rather than compared on pooled columns — the pooled table is too coarse to choose
with (median and impact do not move at all). Impact is untouched at every floor tried: **0 clips
move.** Top at the chosen 0.012: median 2.0 unchanged, mean 10.56 → 10.58, the >10 tail count
unchanged at 46, and 12 clips better against 12 worse. Past 0.014 the tail count and the win/loss
balance both turn. Address re-measured after (it scales its quiet run off top/impact): median 7.0
unchanged, mean 22.9 → 22.8, PCE 15.8% → 16.1%.

**Also landed, as a guard rather than a fix**: `align_swings` now checks the two views' downswing
*durations in seconds* and, when they disagree beyond 30%, emits `AlignmentQuality.IMPACT_ONLY` —
a tier that was defined, documented and never reachable. There both clips take one shared duration
measured back from impact and each panel advances at its own native rate, so a disagreement about
instants can no longer be paid for in playback speed. Run against the *pre-fix* anchors it turns
the 1.69× panel into 0.99×, which is what it is for. It does not fire on this swing any more.

One guard I wrote and then deleted: refusing the fallback when the two clips' reported frame rates
differ. A 30 fps phone beside a 60 fps one is an ordinary pairing, not a broken clock, and slo-mo's
stretched rate is not detectable from the number anyway. Replaced with the check that can actually
be made — that the duration about to be imposed on both panels is a physically possible downswing.
There is a test pinning the 30/60 case specifically.

**Next**: the two views still disagree on *tempo* (2.42 vs 1.50) because DTL's `motion_start` reads
~22 frames late, so quality stays `top_impact`. Harmless now — the fallback anchor is symmetric and
derived from downswings that agree — but it is the remaining soft-anchor weakness on real footage.

---

## 2026-08-09 — The doc that planned work already done (M7 Phase 6, closed out)

**Duration**: ~1 session, docs and validation only — no `src/` or `scripts/` change
**What I did**: Started this session by pasting the Phase 6 planning prompt out of
`docs/M7_TWO_PHONE_CAPTURE.md` into a fresh context, as the doc instructs. **Phase 6 shipped
three days ago, and the prompt contradicts what shipped point by point** — bind the tailnet IP
(the bind never widens), add a host to `config.py` (deliberately not a field), `api_port: int =
8080` (it is 3000), tailnet membership is the access control (it stopped being sufficient the
moment Funnel existed). Building it as written would have reversed ADR-016 and deleted a guard
that has a regression test.

So this commit is the close-out Phase 6 never got, plus disarming the trap:

1. **`docs/BAY_SESSION_RUNBOOK.md`** (new, AS-BUILT) — the two bullets of the Phase 6 prompt that
   were genuinely never delivered: what to check before driving out, and the failure modes to
   expect on-site. At-home preflight, the guest/Funnel path, 1080p60-not-4K with the reasoning,
   a timings table built only from measured numbers, the per-swing loop, a symptom→cause→check
   table, and what footage to bring back for Phase 0.
2. **Bannered the planning prompts.** Phases 1–6 are built, so their prompts are history, not
   instructions; Phase 6's carries a `🛑 SUPERSEDED` banner with the point-by-point diff, because
   it is the one that is affirmatively wrong rather than merely dated. Kept, not deleted — the
   divergence is the interesting part, and this repo banners rather than rewrites.
3. **Fixed the stale counts** that let this happen quietly: the M7 ladder still showed four built
   phases as `[ ]`, the doc header said "no phase implemented yet", `README` said ADRs 000–014,
   `docs/README` said 14 decisions and 31 documents, `ROADMAP` was dated 2026-08-05 and described
   `run_server.py` as hard-coding `127.0.0.1`.

**Key decisions / surprises**:
- **A planning doc that outlives its phase becomes an instruction to redo it.** The prompts were
  written to be self-contained so a cold session wouldn't need the surrounding doc — which is
  exactly what made this one dangerous, because the context that would have corrected it was the
  part deliberately left out. Self-contained prompts need an expiry marker; that is the general
  lesson, and the banner is the cheap version of it.
- **The doc count is now spelled out by directory rather than given as a bare number** (38 = 31 in
  `docs/` + 7 elsewhere, checkable with `git ls-files '*.md'`). A bare count had already gone
  stale twice; a number nobody can verify is one nobody updates.
- **The tempo defect is now in the runbook as a "do not trust this" item.** It was tracked in
  ROADMAP, which is not what anyone reads at a driving range. Whoever takes this to a bay would
  otherwise have been handed a confident, wrong "work on tempo first" with no caveat anywhere in
  reach.
- **No ADR written.** Nothing new was decided here — ADR-016 already covers all of it. The next
  free number stays 017.

**Where I left off**: M7 has two things left and one trip closes both — the Phase 0 field spike
(method locked 2026-08-07, thresholds pre-committed, waiting only on footage) and the on-site
validation of everything Phases 3–6 built. The runbook is written to be the thing you actually
carry.

**Blockers**: **The Funnel guest path is still unvalidated** and I could not validate it from
here — it needs a physical phone that is not on the tailnet. The procedure is written up in §2 of
the runbook and takes about five minutes at home: `tailscale funnel --bg 3000`, confirm 401
without a token and 200 with `?t=`, upload one clip, time it, `funnel --bg off`. The measured
time fills the one blank row in the runbook's timings table. Serve was verified on 2026-08-06,
but tailnet membership was also protecting the endpoint then, so the token has never actually
been the only thing holding the door.

**Notes**: Next commit is the tempo defect — `phases._motion_start` or `evaluate_tempo` via the
`MIN_PLAUSIBLE_TEMPO` floor `analysis/alignment.py` already has, measured against GolfDB first.

---

## 2026-08-09 — No command at all (M7 Phase 5, completed)

**Duration**: ~1 session, implementation + end-to-end verification through the real server
**What I did**: Closed the loop. An upload used to land a file and stop; now the third file
starts the analysis and a results page renders it.

1. **`api/pipeline.py`** — the orchestration lifted out of `scripts/analyze_bundle.py`
   near-verbatim. `print()` became an injected `log` callback so a headless worker doesn't lose
   the narration, and anything a *reader of the result* needs — a short decode, a shot that
   couldn't be read, a swing the selector declined to pick — became a durable note in
   `analysis.json` instead of a line on stderr nobody is watching. The CLI kept every flag and
   every exit code and is now presentation only. It imports no fastapi, so it still runs on a
   `vision`-only install; `tests/api/test_pipeline_imports.py` pins that in a subprocess.
2. **`api/worker.py`** — asyncio queue, one consumer, `asyncio.to_thread` for the heavy part.
   Triggers **only on a complete bundle**; a partial one waits for an explicit "Analyze anyway"
   behind a confirm dialog. No timeout heuristic, deliberately — it would be wrong in both
   directions. `api/state.py` persists an `analysis.state.json` sidecar keyed on the
   role→sha256 map, so a re-uploaded clip invalidates its own result, and the 5 s status poll
   reads a denormalised score rather than parsing a 7 KB JSON per swing.
3. **Routes + `results.html`** — swing detail, the analyze override, and a video route (byte
   ranges confirmed; iOS Safari won't play a `<video>` without them). Path segments are now
   validated — `..` was a live traversal into `sessions_dir`'s parent.

**Three things I was wrong about, all caught by running it rather than reasoning about it:**

- **The `avc1` codec fallback.** OpenCV's bundled FFmpeg carries no libx264, only libopenh264,
  and dlopen's a DLL that isn't shipped — so it prints `Failed to load OpenH264 library` and
  fails. I expected the fallback to `mp4v` to fire. It doesn't: OpenCV falls back to Media
  Foundation, which encodes real H.264. The output is `avc1`, 85/85 frames, honouring the
  requested fps exactly. `isOpened()` is the only thing worth believing; the stderr lies.
- **`isOpened()` reporting success mid-failure** is why `RenderResult` now carries the codec
  that actually won rather than the one that was asked for.
- **`asyncio.Queue.put_nowait` from a non-loop thread** enqueues without waking a consumer
  parked on `get()`. Only bit a test — every production caller is a route handler — but the
  constraint is now documented on `submit()`.

**Verified end to end** through `run_server.py` against the real bay footage: three uploads
(35 MB in 0.1 s), `queued=False` on the first two, `complete` + `queued=True` on the third,
`running → done` in **31.8 s**, score 86.1 matching the CLI exactly, and the video served as
H.264 with `accept-ranges: bytes`. The partial path was checked separately: face-on alone
auto-started nothing, the override produced `partial=true` with the three degradations spelled
out in the notes, no aligned video, and the raw face-on clip served as the fallback.
Also found that uvicorn leaves the root logger at WARNING, so the worker was running silently —
`run_server.py` now configures logging and the whole pipeline narrates to the terminal.

258 tests pass (31 new, all on the base install thanks to the injectable `runner` seam), ruff
and mypy clean.

**Where I left off**: M7 has only the Phase 0 field spike outstanding. The upload leg of Q2 is
still unmeasured — nobody has confirmed what iOS Safari does to a `.mov` on submit, and one real
phone upload plus `spikes/2026-08-07-two-phone/probe.py inspect` answers it. The louder problem
is the tempo checkpoint: it is untrustworthy on real footage and the tips now lead with a
confident, wrong "work on tempo first" to a reader who never sees a caveat on a terminal.

---

## 2026-08-08 — One command, whole bundle (M7 Phase 4)

**Duration**: ~1 session, implementation + verification against real footage
**What I did**: Joined everything that already existed into one command, and found out along the
way that the hard part was not the joining.

1. **`analyze_swing_bundle()`** (`analysis/engine.py`) — pure. Scores the face-on view through
   `analyze_swing()` **unchanged**, uses down-the-line for alignment anchors only, attaches the
   shot. Reuses the phases `analyze_swing` already computed for the anchors rather than
   segmenting twice, so the frame a checkpoint was measured on and the frame the warp pins to
   τ=1 *cannot* drift. New `SwingBundleResult` contract; `SwingResult.shot` is finally populated.
2. **`scripts/analyze_bundle.py`** — the one command. Writes `analysis.json` (7 KB — heavy
   streams excluded, the keypoints already sit beside it), `aligned.mp4`, and per-view keypoints
   cached against the clip's sha256. Cold run 50 s, warm 9 s.
3. **`phases.select_swing()`**, which was not in the plan and turned out to be the substance of
   the phase — see below.
4. **Two moves, no duplication**: the side-by-side renderer out of `align_swings.py` into
   `pose/side_by_side.py`, and `_import_one` out of `import_shot_screens.py` into
   `launch_monitor/screen/importer.py`. The renderer move needed a seam — it would otherwise
   have made `pose` import `analysis` — so the frame correspondence became
   `alignment.pair_frames()` returning `FramePairing`s, and the renderer now only draws.

**Verification**: 227 passed (up from 201 passed / 4 skipped — the OCR integration tests run for
the first time), ruff clean, mypy clean. `aaron-swing-2` still reports **TOP @ 400 / IMPACT @
423**. The renderer move is provably faithful: re-rendering `aaron-1-aligned.mp4` gives 98 frames
at **0.000 mean absolute pixel difference** against the committed reference — every pixel.
End-to-end on the genuine `aaron-1` triple: auto-selection reproduced the Phase 2 verified pair
unaided (face-on 704/718, down-the-line 1550/1574), a *new* shot photo OCR'd at conf 0.93, and
all 24 banner frames render on **both** panels with zero strays — at IMPACT the club is at the
ball in both views.

**Key decisions / surprises**:
- **The window is not framing — it decides what gets scored.** This is the whole reason
  `select_swing` exists. Unaided, `segment_phases` picks a *setup move* on all four real bay
  clips. Whole-clip `aaron-1-front` scores 58/100 with tempo unscored and finish balance a 0.53
  MISS; the actual swing scores 67/100 with both in band. On the 2026-08-07 bundle the same
  effect took finish balance from 0.46 (a MAJOR fault) to 0.07 — the unwindowed figure was
  measuring the golfer *walking away after the shot*.
- **"Take the last descent" is half right, and the half that fails does so silently.** Aaron's
  idea — nobody rehearses after hitting — is correct on both face-on clips and wrong on both
  down-the-line ones, because the DTL phone keeps rolling 15–24 s past impact on the busy side
  of the bay. On `aaron-2-back` it picks a 5-frame artifact at the very end of the clip. And
  *nothing catches it*: `align_swings`' tempo cross-check is skipped once the soft anchor has
  already been refused for another reason, so the wrong pick renders a confident, plausible,
  completely wrong video. What rescues the idea is **downswing duration** — real swings measured
  0.23/0.38/0.40/0.42 s against a setup-move cluster at 0.48–0.53 s — so: filter by duration,
  then take the last. Correct on all four.
- **`candidate_downswings`' default 0.80 rise threshold hides the swing.** On `aaron-1-back` the
  largest descent is a *bystander* (0.417), which puts the cut at 0.334 and excludes the real
  swing (0.257) entirely. Selection uses the same looser threshold `--list-swings` already did,
  now shared as `CANDIDATE_MIN_RISE`, so the set a human chooses from and the set the rule
  chooses from are identical.
- **The window's lead-in was sized for viewing, not measuring.** Phase 2's 2 downswings before
  the top is *inside* a tour-tempo backswing (~3.5), so motion start went undetected on two of
  six clips, which drops the tempo checkpoint and degrades the alignment. Swept 2–8 across all
  six: **5** is the smallest that detects motion start everywhere while leaving top and impact
  exactly where they were. On the 2026-08-07 bundle that alone took the alignment from
  `top_impact` to `full` and made tempo measurable.
- **The band is calibrated on 60 fps and a 30 fps clip reads longer** — the bundle's face-on view
  measures 0.60 s for its only swing. Widening the band would swallow the whole setup-move
  cluster at 60 fps, so instead: **one candidate wins on its own**, whatever it measures. There
  is nothing to choose between, and `segment_phases` would pick that same descent anyway — the
  only question is whether it gets measured in isolation or with the clip's dead air mixed in.
- **`paddle.py` had a latent bug that only a real install could find.** It was written to absorb
  the PaddleOCR 2.x/3.x split in the *result* shape but still called the 2.x *constructor*, and
  3.x validates argument names strictly. Nobody had noticed because `paddleocr` had never been
  installed here — the integration tests had always skipped. Also had to disable oneDNN:
  paddlepaddle 3.3.1's kernel aborts at *predict* time on this CPU, well after a clean
  construction, so no constructor fallback could catch it.
- **The `2026-08-07/1` bundle is not a real paired capture.** Its two clips show different
  swings in different rooms — it came from the Phase 6 *networking* test and holds whatever
  three files were on the phone. Worth knowing before trusting anything measured on it. It also
  demonstrates the limitation exactly: the tempo cross-check passed (1.89 vs 1.54, gap 0.19
  against a 0.35 threshold), and the alignment reported `full` on two different swings. A τ warp
  makes any two swings look aligned; the video is not evidence they are the same swing. The real
  end-to-end check used the `aaron-1` triple, assembled through `SwingBundleStore` as
  `2026-08-07-aaron1` (≈380 MB of copied video — delete it whenever).

**Noticed, not fixed** — and now a named ROADMAP item: **the tempo checkpoint is untrustworthy
on real footage.** `_motion_start` walks back from the top for a quiet stretch, and a golfer who
pauses at the top hands it one immediately, so the boundary collapses onto the top: `aaron-1`
reads a backswing of **0.43 downswings**, which is not physically possible. `analyze_swing`
scores it anyway and the ranked tips lead with "work on tempo first, the downswing is rushing the
backswing" — a confidently wrong instruction. Phase 4 deliberately changes no scoring and instead
makes it impossible to render without seeing it (a note, printed *above* the tips). The fix
belongs in `_motion_start`, or in `evaluate_tempo` applying the floor `alignment.py` already has
as `MIN_PLAUSIBLE_TEMPO` — measured against GolfDB first, since that is where the band came from.

**Where I left off**: the full use case works end to end offline. Phase 5's remaining piece is
the background worker that triggers this on upload instead of a human running it.
**Blockers**: None.

---

## 2026-08-07 — Two views of one swing, aligned (M7 Phase 2)

**Duration**: ~1 session, implementation + first real bay footage
**What I did**: Built the alignment engine, then met real footage and had to build one more thing.

1. **`contracts/alignment.py` + `analysis/alignment.py`** — event-anchored piecewise-linear warp on
   a normalized swing-time axis τ (0 = motion start, 1 = top, 2 = impact), ADR-011's Option C
   standalone. Pure, stdlib + pydantic. `AlignmentQuality` (`full` / `top_impact` / `impact_only` /
   `unaligned`) carries outward how much of the swing was actually anchored.
2. **`scripts/align_swings.py`** — text report with no video needed; `--out` renders the
   side-by-side MP4. Both clips **stream** (the warp is monotone, so the follower only moves
   forward), reusing `draw_skeleton`/`annotate_frame` unchanged.
3. **Multi-swing selection**, which was not in the plan — `phases.candidate_downswings()` (a pure
   addition; `segment_phases` untouched), `--list-swings`, `--window START:END`.
4. **`analyze_swing.py` now derives its instants from `anchors_from_phases`** instead of its own
   copy, so the frame the TOP banner is stamped on and the frame the warp pins to τ=1 cannot drift.
5. **ADR-015**, which also settles the `FrameBundle` question ADR-011's addendum left open: they
   stay two types. `FrameBundle` pairs frames within a millisecond tolerance and so presumes a
   clock; this tier has none.

**Verification**: 201 passed / 4 skipped, ruff clean, mypy clean (52 files, `--python-version 3.12`
per the standing numpy-stub issue). `aaron-swing-2` still reports **TOP @ 400 / IMPACT @ 423** after
the `_instants` refactor — the pinned baseline did not move. The real proof is visual, on the first
bay pair: at τ=1.00 both panels are stamped TOP OF BACKSWING (face-on 704, down-the-line 1551) and
at τ=2.00 both are stamped IMPACT (718 / 1574), with the club at the ball in both. Independently, by
eye, the ball leaves the mat between down-the-line frames 1570 and 1580.

**Key decisions / surprises**:
- **The practice-swing problem is the common case, not the tail**, and Aaron flagged it before the
  footage confirmed it. `_top_and_impact` takes the *earliest* major descent — right for the
  single-swing GolfDB corpus, wrong for a 41-second bay clip. Unaided, both views picked a **setup
  move**: face-on a 0.50 s "downswing" at 0.9 s, down-the-line one at 15.2 s. Downswing *duration*
  is what makes the real swing obvious in the listing (~0.23 s against 0.4-9.1 s for the decoys),
  so `--list-swings` prints it. Worth remembering: **N swings yield about N+1 descents**, because
  the hands coming back down to address is a descent like any other.
- **The tempo cross-check earned its place immediately, and not for the reason I wrote it.** It was
  meant to catch two clips locking onto different swings. What it actually caught on the first real
  pair was `motion_start` **collapsing onto the top** in *both* views — the golfer pauses at the
  top, and `_motion_start` walks back from the top looking for exactly such a quiet stretch. The
  backswing measured 0.43 and 0.04 downswings. `phases.py` reports `detected=True` and is not
  wrong to: from inside one clip nothing looks off. So alignment now refuses any anchor implying a
  backswing shorter than its downswing — no reference distribution needed to know that is not a
  golf swing.
- **A τ warp makes *any* two swings look aligned.** That is the feature, and it means a
  convincing-looking side-by-side is *not* evidence the two clips show the same physical swing.
  Here they do — both are the last swing in their clip and the ball departs in both — but the video
  cannot establish that by itself, and a results page must not imply otherwise.
- **Down-the-line generates far more spurious candidates than face-on** (7 vs 3): a bystander walks
  through frame, the phone gets lowered, and MediaPipe's single-person model tracks both. Down-the-
  line is filmed from the busy side of the bay.

**Noticed, not fixed**:
- ~~**The down-the-line top reads early**~~ — 23 frames of downswing against face-on's 14 for the
  same swing at the same frame rate. Attributed here to the lead wrist being the far, occluded arm
  from behind. **This was the wrong way round — see 2026-08-09 below.** Face-on was the outlier;
  down-the-line had it right all along.
- **The clips are 4K60 portrait, not the 1080p60 the M7 doc asks for**, because nobody told the
  phones otherwise. Pose runs at ~9-14 fps end-to-end at that resolution — tolerable, so the
  deferred pre-inference downscale stays deferred, but that is now a measured number rather than a
  guess.
- I wasted a chunk of this session on a self-inflicted harness bug: my throwaway extraction script
  walked parent directories looking for `pyproject.toml` from a path outside the repo, which never
  terminates, and I read the resulting 100%-CPU-zero-IO process as "4K pose is very slow" and
  started optimising a problem that did not exist. The tell was there and I misread it: zero bytes
  read over four seconds is not a slow decode, it is no decode.

---

## 2026-08-07 — Capture layer survives phone footage (M7 Phase 1)

**Duration**: ~1 session, implementation
**What I did**: The three things standing between the pipeline and a real iPhone clip.

1. **Killed the OOM.** `run_pose.py` held every decoded BGR frame. Both it and
   `analyze_swing.py --overlay` — same bug, and the one you actually point at phone footage —
   now stream. Pose and overlay are two passes over the file rather than one pass and a list.
2. **`camera_id`** on `Frame` and `FrameKeypoints`, ADR-011's Phase 1 seam, prescribed in July
   and never built. `run_pose.py --camera-id face_on`.
3. **Clip metadata persisted.** fps existed only inside `FileVideoSource` and died with the
   process. The keypoints JSON grew an envelope — `{"clip": {...}, "frames": [...]}` — carrying
   fps, width, height, decoded frame count and the source clip's sha256.

Touched `capture/source.py`, `capture/file.py`, `contracts/keypoints.py`, `pose/estimator.py`,
new `storage/keypoints_io.py`, `storage/manifest.py` (+`hash_file`), `run_pose.py`,
`analyze_swing.py`, the three `scripts/golfdb/` readers/writers, the spike probe's loader, and
+19 tests.

**Verification**: 184 passed / 4 skipped, ruff clean. The real proof is an identity, not a
judgement call: re-running pose on the committed sample produced all **656 × 33 × 4 landmark
values bit-identical** to the previous file, `TOP @ 400 / IMPACT @ 423` unmoved, and the spike's
pre-flight still exits 0. Peak RSS **971 MB → 233 MB** on that 480×854 clip (measured, not
estimated), and the new number no longer scales with clip length. Clip metadata came out exactly
as predicted: `fps=58.913` — not 60 — `480×854`, 656 frames, sha matching `Get-FileHash`. The
461-clip GolfDB cache loads untouched, pinned by a test that reads the real files when present.

**Key decisions / surprises**:
- **Dropped the pre-inference downscale**, one of the four items in the phase plan. The
  justification for it — "MediaPipe resizes internally, so 4K buys zero accuracy" — is right
  about the ceiling but skips that downscaling first makes it a *two*-step resample, and that
  difference has never been measured here. It is a throughput optimisation, not a correctness
  fix, and streaming is what actually makes 4K survivable. Revisit with Phase 0 footage and a
  real throughput number. Dropping it is also what let the sample come out bit-identical, which
  is a far stronger check on a refactor this wide than "the instants still look right".
- **The envelope had to be a format change, not a sidecar**, and that put the 461-clip cache in
  the blast radius. One sniffing loader (`load_keypoints`) absorbs it: a top-level array is
  legacy, an object is enveloped, and legacy files report `clip=None` — which is *true*, not a
  missing value. Nothing was migrated; nothing needs to be.
- **`analyze_swing.py` had the same OOM and nobody had noticed**, because it is behind
  `--overlay` and the only clip anyone ran it on is 2.7 MB. Worth remembering the pattern is
  copy-pasted, not unique. The one remaining `list(source.frames())` is in
  `scripts/golfdb/extract_pose.py` and is left alone deliberately: 160px clips, ~15 MB, and the
  estimator interface takes a Sequence.

**Noticed, not fixed**: `mypy src/golf_coach` now dies inside numpy's own `__init__.pyi` —
"Type statement is only supported in Python 3.12 and greater". Nothing to do with this work
(`--python-version 3.12` is clean across all 50 source files); the installed numpy's stubs have
outrun `python_version = "3.11"` in `pyproject.toml`. Bumping the mypy target is its own call.

---

## 2026-08-06 — Phone reaches the server from anywhere (M7 Phase 6)

**Duration**: ~1 hour, implementation
**What I did**: Made the upload server reachable from a phone on any network, which was the whole
point of Phases 3 & 5 and the one thing they stopped short of. **The bind did not change** —
uvicorn still listens on `127.0.0.1` and Tailscale proxies to it over real TLS. Wrote
**ADR-016**; touched `config.py`, `api/app.py`, `api/static/index.html`, `scripts/run_server.py`,
`tests/api/test_uploads.py` (+7 tests), and the M7 doc / ROADMAP / FLOW / READMEs.
**Verification**: full suite green (169 passed, 4 skipped), ruff and mypy clean. Live server on
`127.0.0.1:3000`: 401 with no token and with a wrong token, 200 with the header, 200 with `?t=`,
static page still reachable unauthenticated, and a real 2.6 MB `.mov` uploaded end-to-end through
the query-param token. `--host 0.0.0.0` with no token exits 2 instead of starting.

**Then verified for real, same day, from an actual iPhone.** Installed Tailscale 1.102.2, enabled
MagicDNS + HTTPS certificates, `tailscale serve --bg 3000` →
`https://desktop-snhi10c.taila73d7e.ts.net`. Over that URL: static page 200 with a trusted cert,
`/api` 401 without a token and 200 with either the header or `?t=`. A complete swing bundle landed
from the phone in 30 seconds — `IMG_2712.mov` (33.6 MB, face-on), `IMG_2746.mov` (11.1 MB,
down-the-line), `IMG_2739.jpeg` (4.5 MB, shot screen) — all three grouped into swing 1, status
`complete`, no warnings. The 33.6 MB face-on clip sits right in the 30–50 MB range estimated for
1080p60, which is the number the Cloudflare-vs-Tailscale decision turned on.

`netstat` during all of it showed exactly one listener, `127.0.0.1:3000` — and Tailscale connected
to it as a *loopback client* (`127.0.0.1:7629 → 127.0.0.1:3000`). That is the whole design in one
line of output: nothing was exposed, Tailscale reaches in from inside.

**Still outstanding: Funnel.** `tailscale funnel --bg 3000` (the guest-phone path) is untested, and
it's the one that actually needs the token — Serve was verified with the token on, but tailnet
membership was also protecting it. Deliberately not changed at the same time as Serve.
**Key decisions / surprises**:
- **The plan of record was wrong, and the reason was a person, not a protocol.** Phase 6 said bind
  to the tailnet IP because tailnet membership is the access control. That holds right up until
  the down-the-line phone belongs to whoever is at the bay — a helper can't join my tailnet. So:
  `tailscale serve` for my devices, `tailscale funnel` for guests, and because Funnel is public,
  a `GOLF_UPLOAD_TOKEN` gate on `/api/`. The token is what buys the guest case.
- **Serving via Tailscale is strictly better than binding the tailnet IP**, which is a bonus I
  wasn't looking for. Loopback bind stays loopback (the dangerous config becomes unreachable, not
  merely warned against), and TLS termination gives a secure context — so `getUserMedia` is
  available whenever the page wants to capture directly instead of via the camera roll.
- **Cloudflare Tunnel is out on a hard number**: free and Pro cap request bodies at 100 MB with an
  edge-side 413. A 1080p60 clip (~30–50 MB, measured against the 2.6 MB sample) usually fits; 4K
  doesn't, and the workaround is client-side chunking. Tailscale Funnel documents no body cap.
- **`create_app(token=...)` needed a sentinel.** `None` has to mean "no auth", so it can't also
  mean "look it up in settings" — without the sentinel, tests asserting the unauthenticated path
  would flip to 401 the moment someone put `GOLF_UPLOAD_TOKEN` in their `.env`. Defaulting to the
  sentinel also keeps the lookup fail-closed.
- **The guard is a route dependency, not middleware**, specifically so it resolves before the
  handler reaches `request.stream()`. A rejected upload writes zero bytes to `.incoming/`; there's
  a test for exactly that.
- **`fetch` cannot report upload progress** — only `XMLHttpRequest.upload.onprogress` can. On
  cellular a 50 MB clip left the page showing a motionless "Uploading…", which reads as hung.
  Swapped to XHR with a progress bar. Nothing to do with networking, everything to do with the
  page being usable on a phone.
- **The 8080 → 3000 port note below is now fixed in code, not just remembered.** `api_port`
  defaults to 3000 with the excluded-range explanation inline.
- **`httpx2` in `pyproject.toml` is not a typo** — I assumed it was and was wrong. starlette 1.4.1
  depends on `httpx2` 2.9.1, which is installed and is what `TestClient` uses. Left alone.

---

## 2026-08-06 — Swing bundle store + phone upload server (M7 Phases 3 & 5, trimmed)

**Duration**: ~2 hours, implementation
**What I did**: Built the storage and API layers that let a phone browser upload a swing bundle
(face-on video, down-the-line video, shot-tracker photo) and have it land on the desktop,
correctly grouped — trimmed from the full M7 Phase 3 + Phase 5 design in
[docs/M7_TWO_PHONE_CAPTURE.md](docs/M7_TWO_PHONE_CAPTURE.md) at the user's explicit direction:
no auto-triggered analysis (Phase 4 + the Phase 5 worker), no Tailscale (Phase 6) — both are
separate future work. New: `storage/manifest.py`, `storage/bundle_store.py`, `api/app.py`,
`api/static/index.html`, `scripts/run_server.py`, plus `tests/storage/` and `tests/api/` (21
new tests, all passing; suite stays green with only the base + `api` + `dev` extras).
**Verification**: `pytest -q` green (21 new + all existing). Manually exercised the running
server on localhost: all three roles landing in one swing in every arrival order, dedupe on a
repeated upload, the documented "different bytes reopens a new swing" behavior and its
`swing_id` repair path, and confirmed via `netstat` that the bind stays on `127.0.0.1` (never
`0.0.0.0`).
**Key decisions / surprises**:
- **Swing identity assigned by the store, exactly as ADR-011's addendum and the M7 doc call
  for** — a role-only upload (`face_on` / `down_the_line` / `shot_screen`) slots into the
  newest swing missing that role. Pinned down, not silently accepted: if two swings are
  simultaneously missing the same role, "newest wins" can misattribute an out-of-order upload.
  A test (`test_newest_wins_when_two_swings_are_missing_the_same_role`) locks the behavior down
  so a future change can't alter it unnoticed; the escape hatch is an explicit `swing_id` query
  param that overwrites a role slot by hand.
- **`status()` is derived, not persisted, and deliberately not named `pending`/`ready`/
  `analyzed`.** Those words imply something consumes `ready` to fire analysis, which nothing
  does yet. `collecting`/`complete` says only what's actually true; `analyzed_at` can be added
  later as a pure addition with no manifest migration.
- **`StaticFiles(..., html=True)` only serves a directory-relative `index.html`** — the upload
  page had to be named `index.html`, not `upload.html`, or `GET /` 404s. Caught during manual
  verification, not by the test suite (the API tests never hit `/`).
- **Port 8080 — this repo's own configured `api_port` — is inside a Windows TCP port exclusion
  range (`netsh interface ipv4 show excludedportrange`, 8069–8168 here) on this machine.** Binding
  failed with `WinError 10013` until moved to port 3000 for the manual check. Environment-specific
  and not a code issue, but worth remembering if `scripts/run_server.py` ever refuses to bind on
  the default port on this box.
- Also fixed doc drift while in the area: `README.md` still described `storage/` as SQLite and
  `api/` as unused/declared-only. Left `docs/decisions/008-project-structure.md` alone —
  ADRs in this repo get addenda, not rewrites (see ADR-011's pattern), and 008 is a decision
  record, not living documentation.

---

## 2026-08-05 — Investigation: can two phones at a sim feed this thing? (M7 planned)

**Duration**: ~1 hour, investigation and planning only
**What I did**: Answered a use-case question — *record one swing at an indoor sim on two iPhones
(face-on + down-the-line), photograph the HD Golf screen, send all three in, get analysis back* —
and turned the answer into a seven-phase plan at
[docs/M7_TWO_PHONE_CAPTURE.md](docs/M7_TWO_PHONE_CAPTURE.md). No code written. ADR-011 has a
2026-08-05 addendum; ROADMAP has an M7 section and two corrected `Future` entries.
**Verification**: docs only — nothing under `src/`, `scripts/` or `tests/` touched, so the suite is
unchanged.
**Key decisions / surprises**:
- **The answer was "no", and the missing parts are the hard parts.** Everything downstream of *"a
  video file is already on the desktop"* works. Everything upstream is absent: no upload path, no
  `camera_id`, no structure holding two views of one swing, no session registry (`storage/` is a
  4-line docstring), no host (`api/` is a 6-line docstring with fastapi already declared and unused).
  Call it 60% built — but the remaining 40% is ingestion, identity, and alignment.
- **`run_pose.py:36` will OOM on the first real phone clip.** `frames = list(source.frames())`
  materializes every decoded BGR frame; a 10-second 4K60 iPhone clip is 600 × ~24.9 MB ≈ **15 GB**.
  It has survived purely because the one committed sample is small. Found while tracing whether the
  desktop could do the compute — the answer is yes, comfortably, but not through that line.
- **3D fusion is off the table here, and that is a construction fact, not a scheduling one.**
  ADR-011 reads as though multi-camera work leads to triangulation. It does — for the fixed ELP rig.
  Two phones held by two people have no stable extrinsics to solve for, so spine angle, hip rotation
  and X-factor stay fixed-rig-only. Wrote the addendum specifically so the next person reading
  ADR-011 doesn't spend a day trying to calibrate hand-held phones.
- **But ADR-011's Option C survives on its own, and is the better tool here anyway.** Aligning the
  two clips on the phase instants each already produces (top, impact — median 2 and 1 frames against
  461 GolfDB clips) needs no shared clock and no calibration. It also absorbs the thing host
  timestamps would choke on: two consumer phones aren't configured alike, and **iPhone slo-mo**
  stores 120/240fps with a stretched playback rate, so `CAP_PROP_FPS` may not describe real time.
- **Swing identity has to be assigned server-side.** The tempting design has each phone label its
  upload with a swing number. Two people, two phones, mid-session — they will drift within about
  three swings. Each phone declares only its *role*; the store does the matching.
- **Split into seven one-commit phases with a planning prompt each**, at Aaron's request, after the
  first draft came back as one large plan. Alignment alone is its own problem with its own failure
  modes and deserves its own planning pass. The prompts exist so each phase is planned in a fresh
  session rather than at the tail of a stretched context.

---

## 2026-08-04 — M3: shot data arrives, read off a photo of the simulator screen

**Duration**: ~3 hours
**What I did**: Unblocked M3 without buying anything. The HD Golf simulator has no data export
of any kind, so shot metrics are now read off photographs of its `SHOT DATA` screen: rectify the
screen out of the photo, recover its orientation, OCR it, parse the tile grid geometrically,
cross-check the numbers against the screen's own arithmetic, and cache the result. Plus
`CompositeShotDataSource`, so screen captures, mock shots, and a future R10 mix behind one port.
Decision: [ADR-014](docs/decisions/014-screen-capture-shot-ingestion.md); ADR-004 has an addendum.
**Verification**: `pytest` → **141 passed, 4 skipped** (was 96); `ruff` clean; `mypy` clean on
`contracts` + `launch_monitor`. Rectification verified by eye on both reference photos. The 4 skips
are the OCR integration tests — `paddleocr` isn't installed in this venv yet.
**Key decisions / surprises**:
- **Chose local OCR over vision-model parsing, knowing it is the riskier build.** The photos are
  the hard case: shot at an angle, ceiling glare, the room reflected across the tiles. What made
  it worth it is that the choice is *cheap to reverse* — the engine sits behind a `TextRecognizer`
  Protocol, so if it proves too fragile in practice, a vision adapter is one new class and parsing,
  validation, caching and the source are untouched.
- **Quad detection failed on both real photos, and the cause was scale, not thresholds.** The
  morphology kernel that bridges gaps in the bezel edge is a fixed pixel size; on a 5712px phone
  photo it is far too small to close anything, so the screen contour came back shattered every
  time. Running detection on a normalized 1000px copy and scaling the corners back fixed both
  photos immediately. I had been about to start tuning Canny thresholds — that would have been
  hours down the wrong hole.
- **The "upside-down" photo was never upside-down to the code.** IMG_2739 displays inverted in
  viewers that ignore EXIF, and I wrote a test asserting the rotation search would correct it.
  `cv2.imread` applies the EXIF orientation tag, so it loads upright and the assertion was wrong.
  The rotation search still earns its place — EXIF does not survive re-encodes, screenshots, or
  video frames — but it is now tested on synthetic images where I control the orientation, and the
  integration test asserts what actually matters (the screen got cropped, the labels are legible).
- **The screen validates its own parse.** It prints redundant metrics: `smash == ball / club` and
  `total == carry + bounce & roll`. Both hold *exactly* on both reference photos, which makes a
  mismatch evidence about the parse rather than about the shot. That is the whole answer to "how
  do I know OCR didn't drop a digit" — a 128.1 read as 28.1 is a well-formed number that breaks
  the arithmetic.
- **Fidelity is not plausibility, and conflating them would have been a bug.** IMG_2739 shows a
  159.5 mph club speed and a 0.89 smash factor — nonsense as a golf shot, but the screen's own
  arithmetic checks out, so the parse is correct and passes clean. Flagging it would train me to
  ignore flags. Range checks are deliberately wide enough to admit it; they exist to catch a
  dropped digit, not an unusual swing.
- **No fixed ROIs anywhere.** Tiles are located from the labels' own geometry — find the labels,
  group into rows, derive each column from its neighbours, read what falls in the cell below. A
  pixel-coordinate template would break the first time I stood somewhere else to take the photo.
  Device knowledge (labels, target fields, sign rules) is `profiles.json`, so a second launch
  monitor is a data change.
- **Signs are the most dangerous part of the pipeline.** `1.6 ° O>I` must become `-1.6`, `Closed`
  negative, `L` negative, and `---` must become `None` and never `0`. A flipped sign is invisible
  downstream. A number with no direction word is stored as a magnitude *with a warning*, never
  with a guess.
**Where I left off**: Ingestion works end-to-end on the two reference photos. Next: install the
`ocr` extra, run `import_shot_screens.py` over a full range session, and tune preprocessing on
photos that were not the two I developed against. Then the MCP server itself, which is now the
only thing left in M3.
**Blockers**: None. `paddleocr` is not installed yet, so the integration tests skip.

---

## 2026-08-04 — M5: the feedback learns to prioritise, and the panel fails to widen

**Duration**: ~3 hours
**What I did**: Turned three equal-weight readouts into ranked coaching. Wired the reference
distributions — built, tested and *never called* since M4-REF — onto every `CheckpointScore`, ranked
tips by what actually discriminates within their group, added a headline, and stopped unmeasurable
checkpoints vanishing without a word. Then tried to widen the panel from 3 to 5 using the
arm-parallel metrics already sitting in `golfdb_v1.json`, and the gate said no. Design doc:
[docs/M5_COACHING_FEEDBACK.md](docs/M5_COACHING_FEEDBACK.md); ADR-010 has a new addendum.
**Verification**: `pytest` → **96 passed** (was 75); `ruff` clean; `mypy` clean on `analysis` +
`feedback`. `analyze_swing.py` re-run on both clips. `tune_arm_parallel.py` scored 461 face-on clips.
**Key decisions / surprises**:
- **The motivating bug was a 100/100 swing.** `golf_swing-aaron-1` passes all three checkpoints and
  scores perfect — while its head sway sits higher than **83% of 458 tour swings**. Nothing we stored
  could say so, because `_score_within_range` returns exactly 1.0 for *every* pass. Three passes,
  three identical scores, no ordering derivable. That is the whole argument for percentiles, and it
  is a better one than "show the golfer a stat".
- **The percentile saturates exactly where the band ends.** The bands *are* the reference p10/p90
  (ADR-012) and `percentile_of` clamps there, so every failing checkpoint reports 90 whether it
  missed by a hair or by triple. I had planned to rank failures by percentile; that would have
  flattened them all into one bucket. Ranking needs **two** signals: `score` for failures (it decays
  in band-widths and does separate them), percentile for passes (score cannot). Severity stays on
  `score` for the same reason.
- **Percentiles query the same stratum the band was cut from**, not the most specific match.
  `tempo_ratio` face-on p90 is 5.00 against the all-view 4.71 — mixing them would let a swing read
  "inside the band" and "past the 90th percentile" at once. There is now a test that blinds the
  evaluators to the distributions and asserts `score`/`passed` do not move, so ADR-010 §2's firewall
  is executable rather than a comment.
- **The panel did not widen, and that is the result.** GolfDB's mid-backswing/mid-downswing are
  *lead arm parallel to the ground* — a genuine body pose, unlike `toe_up`, which is a club event
  gated on M2. Built `tune_arm_parallel.py` and measured before writing either checkpoint.
  **`prior_frac` — which reads no pose signal at all, just "assume the tour-median fraction" — beats
  every pose rule on every column, for both events.** frac_err 0.043 against argmin's 0.059
  (mid_backswing) and 0.038 against cross's 0.100 (mid_downswing). A checkpoint built on our
  detection would be worse than a constant. First time `tune_address.py`'s `prior_*` bar has actually
  rejected something.
- **Reported `frac_err`, not frames.** The error in the metric the checkpoint would *report*, against
  the p10–p90 width of the band it would be compared against. A 4-frame error sounds excellent until
  you notice `mid_backswing_frac`'s entire tour spread is ~4 frames on a real-time clip.
- **A hypothesis I checked and had wrong.** I assumed the tight spread was frame quantization — an
  8-frame downswing can only produce a handful of fractions. It is not: spread is **0.148 real-time
  vs 0.149 slow-motion** for mid_backswing, on 29- vs 108-frame backswings. Four times the temporal
  resolution does not narrow it. The variation is real; our detector cannot resolve it.
- **Unscored checkpoints are named now.** Tempo drops on ~14% of clips (ADR-013) and `overall_score`
  is a mean over survivors, so a 2-checkpoint and a 3-checkpoint swing printed the same number.
  `SwingResult.unscored` carries the names. The score is deliberately *not* penalised — dropping the
  checkpoint is right, dropping it silently was not.
**Where I left off**: The panel stays at three checkpoints, now ranked and quantified against the
tour population. `tune_arm_parallel.py` is kept runnable alongside the rejected address signals. The
two things that would change its verdict are both hardware/milestone gated: M2 club detection gives
`toe_up` directly, and the down-the-line view (ADR-011) sees arm-parallel far more cleanly than a
160×160 face-on crop.
**Blockers**: None.
**Notes**: `unscored` carries names but not *reasons* — "tempo could not be measured" does not say
"because the address boundary was estimated". A reason means the evaluators returning one instead of
`None`, which touches every return site; deferred deliberately. Also: the coaching prose still lives
in `mechanics.py`. Fine at three checkpoints, extract a `feedback/catalogue.py` the moment that
changes.

---

## 2026-08-02 — M4-REF Phase B6: address detection, and the posture bug hiding behind it

**Duration**: ~3 hours
**What I did**: Closed the last open M4-REF item. Replaced the address rule's one fps-dependent
constant with a clip-relative one, bounded its failure path, made it report when it fails, and —
the part that turned out to matter more — stopped the posture checkpoints depending on the address
boundary being right. New `scripts/golfdb/tune_address.py` is the runnable address report that
never existed; ADR-013 records the decision; M4_POSE_BAKEOFF Phase B6 and
`docs/M4_ADDRESS_DETECTION.md` carry the numbers and the flow.
**Verification**: `pytest` → **75 passed** (was 67); `ruff` clean. `tune_address.py` reproduces
median **7.0** / mean 22.9 / 40% over 10 / med_norm 0.122 / PCE 15.8% against the old rule's
9.0 / 27.2 / 46% / 0.133 / 14.3%. `bakeoff.py --merge` confirms top and impact **unchanged** at 2
and 1. Full band re-derivation chain re-run; `analyze_swing.py` re-run on both own clips.
**Key decisions / surprises**:
- **Splitting the corpus by capture speed is what cracked it.** The famous "median 9 frames" is
  really 5 real-time and **17 slow-motion**, and the corpus is 47% slow-motion — so the headline was
  substantially reporting the corpus mix. That immediately implicated `_MOTION_STALL_FRAMES = 4`, a
  frame count sitting in a corpus whose downswing runs 8 frames real-time and 30 slow-motion. Now
  **0.25 of the clip's own downswing duration**. Median 9 → 8 on its own.
- **The frame-0 fallback was a second, separate bug.** It fired on 11% of clips at a median of 31
  frames, because frame 0 is not a neutral answer — GolfDB clips carry a median 59 frames of
  pre-roll. Bounded it to `top - 3.5 x downswing` and marked the segment `detected=False`. Median
  → **7**, mean 27.2 → 22.9.
- **Six alternative signals, all worse.** Setup-ball displacement, noise-floor/MAD, ramp
  back-extrapolation, torso energy, shoulder rotation, directional persistence. Upper-body motion
  energy *ties* the lead wrist for eight extra landmarks. Kept every one of them runnable in
  `tune_address.py` rather than deleting the evidence — at 160x160 the torso simply does not move
  enough during a takeaway.
- **The sobering number**: a rule using **no pose signal at all** (`top - 3.5 x downswing`) scores
  median 11. We score 7. That is the honest size of what the lead wrist contributes, and it is now
  a permanent row in the harness so no future candidate gets graded against a soft baseline.
- **The real find was in posture, not in the boundary.** `head_sway` averaged the head across
  `[0, motion_start]` — which starts at frame 0, i.e. the golfer walking in. On
  `golf_swing-aaron-1` that read **1.21 shoulder-widths** of sway, nearly 3x the tour p90, when the
  head sits +0.41 off setup early in the clip and only settles by ~frame 400. Sampling a short
  window *ending at* the boundary gives **0.36** — a hard fail becomes a comfortable pass, and the
  clip goes to 100/100. Same shape as the top-detection defect ADR-012 found: a plausible number
  measuring the wrong thing.
- **One predicted win did not happen.** I expected the shoulder-width ruler (it divides *both*
  posture checkpoints) to improve with the window. It did not — 10% of clips off by >10% before,
  11% after. That error is pose noise, not window content. Recorded as a miss rather than quietly
  dropped.
- **Tempo now drops rather than guesses on 14% of clips.** The fallback boundary is derived from an
  assumed tempo ratio, so scoring it would hand the assumption back as an observation. ADR-010 §2.
  The most contestable call here, and the one to revisit if it annoys in practice.
- **Metric definitions v2 → v3.** Changing *where* a metric samples changes the metric, so the bands
  were re-derived rather than assumed: `head_sway_norm` 0.42 → **0.43**, `finish_balance_norm`
  0.28 → **0.29**. Small, which is exactly the drift ADR-012 §4 exists to catch.
**Where I left off**: M4-REF exit criteria met. Address is still the weakest instant (7 frames, 40%
over 10) but it is measured, improved by a different rule rather than a better constant, and can no
longer fail silently. Two follow-ups on the ROADMAP: switch the tracked headline from pooled frame
median to `med_norm` + the slow-mo split, and decide whether the remaining headroom (SwingNet 31.7%
PCE vs our 15.8%) justifies a learned model — which needs its own ADR, since ADR-008 keeps the
analysis core stdlib-only.
**Blockers**: None.
**Notes**: `smoothing.py`'s 5-frame window is now the *only* absolute left in the address path, and
it is still the value tuned by eye on ~60fps phone clips. Changing it globally moves top and impact
too, so it stays its own item — but it is the obvious next thing to question. Also worth knowing:
the synthetic test fixtures cannot prove the fps-invariance claim, because their setup is perfectly
still and both the old and new rules find it. The corpus harness is the evidence there; the unit
tests only guard the contract.

---

## 2026-08-02 — M4-REF Phase B: estimator bake-off settled, both provisional bands recalibrated

**Duration**: ~4 hours (mostly background extraction)
**What I did**: Finished the GolfDB reference work. Locked `_MAJOR_RISE_FRACTION` against the full
corpus, ran the four-variant pose bake-off to completion, and replaced the last two
`PROVISIONAL / UNCALIBRATED` bands in `ranges.json` with tour-derived ones. Every number is in
`docs/M4_POSE_BAKEOFF.md` + `docs/pose_bakeoff_v1.json`; ADR-002 has an addendum recording the
variant decision.
**Verification**: `pytest` → **67 passed** on the base install (was 66; one rewritten, one new);
`ruff` clean; `mypy` clean on `analysis` + `feedback`. End-to-end re-run of `scripts/analyze_swing.py`
on both clips against the new bands.
**Key decisions / surprises**:
- **`_MAJOR_RISE_FRACTION` 0.50 → 0.80**, picked from the **plateau centre** (0.75–0.85 are flat at
  ~10.5 mean top error) rather than the argmin. The old 0.50 was fit on 97 clips; on all 461 it costs
  8 frames of mean error. Median top error is now 2 frames, impact 1 — against 21 and 35 for the
  `argmin` rule this replaced. No effect at all on my own clips: both have a single dominant rising
  run, so every fraction from 0.40–0.95 gives identical instants.
- **Keep MediaPipe lite.** Twelve paired McNemar tests across lite/full/heavy — **not one reaches
  p < 0.05**. full's +9.1pp at the top looked real at n=120 (p=0.099) and **evaporated to +1.9pp
  (p=0.494) at n=461**, with lite ahead on mean PCE. Same overfitting-to-sample-size trap as the 0.50
  constant, caught this time *before* it got baked into a band. heavy costs 4.4x and buys nothing,
  which retires ADR-002's one-clip judgement with an actual number.
- **RTMPose rejected by 24.7pp**, and the "3 fps, run it overnight or drop it" dilemma was false:
  rtmlib's `Body` re-runs a YOLOX detector every frame, which is **35x** the pose model. Skipping it
  (the clips are already bbox crops) gave **118 fps** — faster than lite — so the whole question cost
  5 minutes instead of 3 hours. It still lost badly: same trajectory (r=0.948 with lite) but 1.5–4.3x
  noisier per landmark, worst at the hip.
- **Both eyeballed bands were loose; `finish_balance` by more than 2x.** head_sway 0.5 → **0.42**,
  finish_balance 0.6 → **0.28** (p90 of 458 face-on swings from 122 tour golfers, measured at
  GolfDB's *annotated* instants, never at our own segmentation). `aaron-swing-2` drops 100 → 78: its
  0.47 finish drift was always there and sits above the 90th percentile of tour finishes.
- **The tightened band exposed a vacuous test.** `test_finish_balance_survives_a_single_bad_frame`
  asserted `passed is True` while the metric read 0.48 — it had only ever passed because 0.48 < 0.6,
  and never demonstrated the p90 property at all. Root cause: `smooth_keypoints` is a 5-frame moving
  average, so **one bad frame becomes five**, and p90 can only reject it beyond ~50 follow-through
  frames. The fixture hardcoded **8**; the real corpus is p10 50 / p50 89. The metric is fine (only
  8% of clips fall in the bad regime) — the fixture was testing a regime real footage never reaches.
  A median center was tried and is *worse*; the center was never the problem.
- **Calibrated the address constants too** — `_MOTION_QUIET_FRAC` / `_MOTION_STALL_FRAMES` were the
  last things in `phases.py` still set from one clip. Swept over all 461: **0.08/3 → 0.05/4**, median
  address error **13 → 9 frames**. Took the plateau centre again, *not* the grid minimum (8.0 at
  0.03/2), which sits on the grid edge and falls apart one step in any direction. My own clips barely
  move (address 322 → 319, tempo 3.39 → 3.52, no verdict change).
- **Verified the corpus by eye, finally.** Added `scripts/golfdb/spot_check.py` — 5 clips x 4
  ground-truth instants with production skeletons. Every other check this milestone was a statistic,
  and I'd cut two committed bands from 458 clips without once looking at a pose. Torso/hips/
  shoulders/legs correct in all 20 tiles, which is what the bands actually rest on.
**Where I left off**: M4-REF is complete. `ranges.json` has no uncalibrated rows left, all three
bands cite an inspectable distribution, and nothing in `phases.py` is tuned on a single clip any
more. The one open item is **improving address detection** (median 9 frames, 46% of clips over 10) —
now calibrated as far as the current rule goes, so further gains need a *different* rule, not a
better constant. Intrinsically hard: SwingNet manages only 31.7% PCE there.
**Blockers**: None.
**Notes**: ~10% of corpus clips contain a *practice swing* before the annotated one, which no
estimator fixes — those failures have **higher** tracking confidence than the successes (0.85 vs
0.82), so they are structural, not pose quality. Nothing under `data/reference/` is committed; the
only new committed data files are `golfdb_v1.json` and `pose_bakeoff_v1.json`.

---

## 2026-08-01 — Hardened motion-start detection (velocity-anchored) → tempo is now honest

**Duration**: ~1.5 hours
**What I did**: Fixed the last-flagged segmentation error from the 07-16 session — motion-start
landing mid-takeaway and collapsing tempo. Replaced the height-crossing rule in
`analysis/phases.py` with a **velocity-anchored** one: new `_motion_start` measures **2D lead-wrist
speed** (new `_wrist_speed`; refactored `_lead_wrist_y` → `_lead_wrist_xy` since we now need `x`
too) and, walking back from the top, takes the takeaway to begin just after the last sustained
*quiet* stretch (`_MOTION_QUIET_FRAC` of peak speed for `_MOTION_STALL_FRAMES` frames). Extended the
synthetic fixture (`tests/analysis/conftest.py` `make_swing`) with a near-horizontal `takeaway_x`
preamble — the sideways move a wrist-*height* rule can't see — and added two tests (a motion-start
robustness test in `test_phases.py`, a tempo-not-collapsed test in `test_checkpoints.py`).
**Verification**: `pytest` → **41 passed** on the base install (was 39); `ruff` clean; `mypy
src/golf_coach/analysis src/golf_coach/feedback` clean. Then the decisive real-clip check on face-on
`aaron-swing-2`: dumped the smoothed lead-wrist speed profile and re-ran `scripts/analyze_swing.py
--overlay`. **Extracted and eyeballed the annotated frames** — ADDRESS marker @322 lands with the
hands *still at the ball* (frame 290 setup and 322 look identical; motion only appears by 345),
where it used to land mid-backswing.
**Key decisions / surprises**:
- **The clip's tempo is genuinely ~1.5:1, not a bug.** The speed profile settles this: the lead
  wrist is provably *still* (< ~3% of peak, flat `y`) from frame ~250 to ~318, then onsets sharply
  (12% → 22% → 54% …) at ~320. So motion-start @322 is *correct*; this golfer just swings quick.
  The move 1.05 → **1.53:1** is the backswing finally being counted from the true onset, not the
  number magically becoming 3:1. Honest > flattering.
- **Height can't see the takeaway; speed can.** The early takeaway is near-horizontal — the lead
  wrist moves back at roughly constant height — so the old "wrist rose past address height" rule
  skipped it. Using 2D speed is what fixed it; anchoring on wrist-`y` alone never could.
- **Tuned `_MOTION_QUIET_FRAC` to 0.08** against the real speed profile: setup waggle jitter tops
  out ~3% of peak, the takeaway jumps past ~12%, so 8% sits cleanly between. TOP @383 / IMPACT @423
  unchanged throughout (only motion-start moved).
**Where I left off**: Tempo is now trustworthy end-to-end (correct instant, honest value, visual
proof). The three pose-only checkpoints are solid. Next is either the **Hardware Re-Validation
Gate** (recalibrate the provisional sway/balance bands, validate instants against club/R10 timing)
when hardware lands, or non-pose work (M1.5 club spike / full-M4 outcome axis).
**Blockers**: None — pose-only, no hardware.
**Notes**: `_MOTION_QUIET_FRAC` is tuned on one clip; re-check when higher-fps / global-shutter
footage arrives (folded into the existing `HARDWARE-REVALIDATE:` note on smoothing/instants). The
annotated overlay + extracted frames live under gitignored `data/processed/`.

---

## 2026-07-16 — M4-PoC+: hardened Fundamentals panel (smoothing, 2 checkpoints, verification overlay)

**Duration**: ~2 hours
**What I did**: Made the pose analysis *trustworthy without hardware*, per the approved plan
([docs/M4_FUNDAMENTALS_PANEL.md](docs/M4_FUNDAMENTALS_PANEL.md)). New `analysis/smoothing.py`
(visibility-weighted centered moving average, stdlib only), applied once in `engine.analyze_swing`
so phase segmentation + checkpoints read a denoised signal. Added two pose-only mechanics
checkpoints in `checkpoints/mechanics.py` — `evaluate_head_sway` (lateral nose travel to impact)
and `evaluate_finish_balance` (post-impact hip-center settle), both shoulder-width normalized;
extracted the shared `_score_within_range`/geometry helpers. Seeded two **PROVISIONAL /
UNCALIBRATED** benchmark rows (`head_sway_norm`, `finish_balance_norm`) — data-only, ADR-010
addendum. New `scripts/analyze_swing.py`: prints a scores+tips report AND (with `--overlay`)
renders an annotated clip stamping ADDRESS/TOP/IMPACT + a score HUD, via a new
`pose/overlay.py:annotate_frame`. Wrote the feature doc (two mermaid diagrams, reliable-vs-deferred
table, SOLID/GRASP notes), a ROADMAP **M4-PoC+** section + a new **Hardware Re-Validation Gate**,
and the ADR-010 addendum.
**Verification**: `pytest` → **39 passed** on the base install (was 27); `ruff` clean; `mypy
src/golf_coach/analysis src/golf_coach/feedback` clean. Real-clip run on face-on `aaron-swing-2`:
produced a 3-checkpoint SwingResult (tempo MISS 1.05:1, head_sway PASS, finish_balance PASS) and a
656-frame annotated overlay; extracted and eyeballed the marked frames.
**Key decisions / surprises**:
- **Dropped the plan's velocity-based impact detector.** Implementing it revealed it *miscalibrates*
  tempo: peak hand speed precedes ball contact, so it shortens the downswing and inflates the ratio,
  breaking the Tour-Tempo calibration (and the synthetic tests). Reverted impact to the correct
  return-to-address-height rule; **smoothing is the real robustness win**, not a new impact rule.
- **The overlay earned its keep immediately** — it localized the remaining tempo error. TOP @ 383 and
  IMPACT @ 423 are visually correct, but **motion-start @ 341 lands mid-takeaway**, truncating the
  backswing to ~42 frames → the ~1:1 reading. So tempo is low because of *motion-start*, not top/impact.
- Provisional bands are labelled as such (greppable) and gated in the Hardware Re-Validation Gate;
  `HARDWARE-REVALIDATE:` comments mark every spot to revisit when cameras/R10 arrive.
**Where I left off**: M4-PoC+ done & verified. Next segmentation task is **hardening motion-start**
(anchor the takeaway on the last stable-setup frame, not the last frame at/above address height) —
that's what will finally make tempo believable. Sway/balance await calibration data.
**Blockers**: None — pose-only, no hardware.
**Notes**: Tip/HUD/banner text stays ASCII (plain hyphen) for the Windows console. The `pytest`
run needs `--basetemp`/`-p no:cacheprovider` redirected to the scratchpad under the sandbox.

---

## 2026-07-03 — M4-PoC implemented: pose-only Fundamentals analysis spine

**Duration**: ~2 hours
**What I did**: Implemented the whole [M4-PoC plan](docs/archive/M4_POC_PLAN.md) — the pose-only
Fundamentals analysis spine, end-to-end. New: `contracts/intent.py` (`PracticeGoal` + enums),
the benchmark store (`analysis/benchmarks/` — `ranges.json` seeded with Tour Tempo ~3:1 +
`resolve_range` with most-specific→`all` fallback), `analysis/phases.py` (lead-wrist
segmentation → 6 phases), `analysis/checkpoints/mechanics.py` (`evaluate_tempo`),
`analysis/scoring.py` (`FundamentalsPolicy` + `policy_for`). Extended `SwingResult` with
`intent`/`mechanics_score`/`outcome_score`, wired `analyze_swing`, and implemented
`feedback/rules.py` (`build_feedback`). Added the synthetic-swing fixture + 7 test modules (23
new tests). Also **added the missing runtime sequence diagram to `docs/archive/M4_POC_PLAN.md`** (the
plan only had a data-flow ASCII block), wrote the milestone flow doc
**[docs/archive/M4_ANALYSIS_POC.md](docs/archive/M4_ANALYSIS_POC.md)** (mermaid data + sequence, GRASP
callouts, files, findings), an **ADR-010 addendum** (JSON-not-YAML), and checked off the
ROADMAP M4-PoC boxes.
**Verification**: `pytest` → **27 passed** on the **base install** (no vision/ML extras);
`ruff check` clean; `mypy src/golf_coach/analysis src/golf_coach/feedback` clean. Real-clip
eyeball: fed the face-on `data/processed/aaron-swing-2.keypoints.json` through
`analyze_swing → build_feedback` → a full `SwingResult` + tempo tip. Exit criterion met.
**Key decisions / surprises**:
- **Phase segmentation had to anchor on the top of the backswing, not "first motion."** First
  pass forward-scanned for the first wrist movement; on the real clip the golfer waggles/sets
  up for ~5.8 s, so that swallowed the setup into the backswing → a nonsense **9.6:1** tempo.
  Re-anchored on the global wrist-`y` minimum (the top) and walked backward to the start of the
  final rise → address phase collapses correctly (0–342) and tempo reads a believable **~1.1:1**
  (amateur-quick; the "too quick" tip is the right cue). Synthetic tests stayed green.
- **Benchmark store ships JSON, not YAML** to keep the analysis core stdlib-only (ADR-010
  addendum). One seeded row (Tour Tempo).
- Kept the guardrails: no `merge.py`, no outcome checkpoints, one policy — all named seams
  (`outcome=[]`, `outcome_score=None`, absent `checkpoints/outcome.py`).
**Where I left off**: M4-PoC is **done and verified**. The spine is proven; the thing to harden
next (full M4) is segmentation *accuracy* — landmark smoothing, validating top/impact against
the overlay video, then more mechanics checkpoints (posture/hip rotation, several needing
down-the-line / synced 3D per ADR-011) and the outcome axis + other scoring policies.
**Blockers**: None — pose-only, no hardware.
**Notes**: Runtime tip text uses a plain hyphen (not em-dash) so the Windows-console eyeball
doesn't mojibake. Real-clip number is one heuristic on one clip — good enough to prove the
spine, not yet a trustworthy tempo measurement.

---

## 2026-07-02 (cont. 2) — M4-PoC implementation plan written up (no code yet)

**Duration**: ~0.5 hour
**What I did**: Reviewed the M4-PoC scope (ROADMAP + ADR-009/010), re-verified the current
contracts/stubs against the plan, and documented the agreed implementation plan into the repo
as **[docs/archive/M4_POC_PLAN.md](docs/archive/M4_POC_PLAN.md)** so it survives outside the local scratch
plan. Confirmed nothing in the plan is stale: `analysis/engine.py` and `feedback/rules.py` are
still `NotImplementedError` stubs; `contracts/{intent}.py` and
`analysis/{benchmarks,phases,checkpoints,scoring}` don't exist yet (all new); hatchling packages
all of `src/golf_coach`, so a `benchmarks/ranges.json` ships with no pyproject change.
**Plan in one line**: build the pose-only Fundamentals spine
(`FrameKeypoints + PracticeGoal → analyze_swing → SwingResult → build_feedback → FeedbackPayload`)
with a single **tempo** checkpoint, the dual-axis/intent seam in place, benchmark ranges as
JSON-with-provenance (Tour Tempo ~3:1 seeded), all on the stdlib base install. Full breakdown of
the 10 change-sets (new modules, contract extensions, tests, verification) is in the plan doc.
**Key decisions (all captured in the plan doc)**:
- **Scope = M4-PoC only, one tempo checkpoint.** No `merge.py`, no outcome checkpoints, no extra
  scoring policies, no SQLite — those are full-M4 and left as named seams.
- **Analysis core stays pure-Python/stdlib** (no numpy/MediaPipe) so the spine + tests run on
  `pip install -e .`; benchmark store ships as **JSON not YAML** to keep base deps tiny.
**Where I left off**: Plan is documented and approved; **no analysis code written yet**. Next
session: implement change-sets 1→9 in `docs/archive/M4_POC_PLAN.md` (contracts → benchmarks → phases →
checkpoint → scoring → engine → feedback → tests), then write the `docs/archive/M4_ANALYSIS_POC.md`
milestone flow doc and check off the ROADMAP boxes.
**Blockers**: None — pose-only, no hardware.
**Notes**: Real-clip eyeball can reuse the existing face-on keypoints JSON in `data/processed/`
(from `aaron-swing-2.mov`) for the exit-criterion sanity check.

---

## 2026-07-02 — M1 angle re-shoot: face-on confirmed as canonical pose placement

**Duration**: ~0.5 hour
**What I did**: Re-shot the swing from a **face-on** angle (`data/raw/aaron-swing-2.mov`,
480×854, 58.9 fps, 656 frames) to test the self-occlusion hypothesis from the 06-28 review,
ran it through `run_pose.py`, and computed the same metrics on both clips for a
side-by-side (detection, per-group visibility, knee-by-decile, jitter).
**Findings** (full write-up:
[M1_CAPTURE_FLOW.md → angle comparison](docs/archive/M1_CAPTURE_FLOW.md#m1-findings-angle-comparison-2026-07-02)):
- Face-on wins on **every** body metric. **Knees 0.71 → 0.88 (+24%)**, lower body
  0.70 → 0.83, overall visibility 0.78 → 0.89. Leg jitter also dropped.
- The big one: knee confidence now stays 0.85–0.95 *through the bent swing posture* where it
  used to collapse to ~0.60. Confirms the occlusion diagnosis — at ~5 o'clock the legs
  stacked front-to-back; face-on separates them.
- Honest caveat: all-landmark jitter rose slightly, but that's genuine (larger arm arc in
  face-on), not noise. Temporal smoothing still worth doing.
**Key decision**:
- **Canonical pose-camera placement = face-on / 3 o'clock** (ball flies to 12; RH golfer,
  mirror to 9 for LH). Recorded in **ADR-003 addendum (2026-07-02)**, which also reconciles
  this with the ADR's original "down-the-line first" note: down-the-line is for the *club*
  stream (M2/YOLOv8), face-on is for the *pose* stream.
**Where I left off**: M1 pose setup is now solid. The "re-record & re-review lower body" open
refinement is **done**; only the optional temporal-smoothing pass remains. Ready to start
**M4-PoC** (tempo) on the new keypoints JSON whenever we choose.
**Blockers**: None.
**Notes**: old raw clip (`golf_swing-aaron-1.mov`) no longer in `data/raw`, but its
keypoints JSON remains in `data/processed` and was used for the comparison.

---

## 2026-07-02 (cont.) — Spine-angle investigation → camera topology & sync plan

**Duration**: ~0.5 hour
**What I did**: Investigated a hunch that shot 2's spine looked "very vertical." Estimated
spine tilt (shoulder-mid → hip-mid vs vertical) on both clips, in 2D and depth-aware 3D.
Used the result to settle the camera topology and to plan synchronization.
**Findings**:
- Shot 1 (~5 o'clock): ~37° spine tilt at address (believable). Shot 2 (face-on): ~2° in the
  image — looks dead vertical, but that's a **projection artifact**: forward tilt lives in the
  camera's depth axis and foreshortens to ≈0 face-on. `z`-based 3D put shot 2 at ~66–71° (if
  anything more bent), but MediaPipe `z` is unreliable. **Can't** conclude a more upright
  stance — the "vertical" look is the camera angle. Rule: forward spine tilt = down-the-line
  measurement, not face-on.
**Key decisions (documented)**:
- **Two cameras, not three.** The spine/hip stream and the club stream share the
  down-the-line viewpoint; one global-shutter camera there runs both MediaPipe + YOLOv8. →
  **ADR-003 addendum (2026-07-02b)** with the stream→camera assignment table + spine caveat.
- **ADR-011 (Proposed): Camera Synchronization & Multi-View 3D Fusion** — phased plan
  (Phase 1 single-cam now, build `camera_id`+timestamp+`FrameBundle` seam; Phase 2 two-cam
  software/event sync + calibration; Phase 3 hardware trigger for frame-accurate dynamic 3D).
  Unlocks true spine angle / hip rotation / X-factor / kinematic sequence for M4 mechanics.
- Also noted the spine caveat in M1_CAPTURE_FLOW.md and added both items to ROADMAP Future.
**Where I left off**: Camera plan is now on record end-to-end (angle, count, stream
assignment, sync roadmap). No code change — ADR-011's capture-seam work (`camera_id` on
`Frame`, `FrameBundle`) is a small future task, not needed for M1/M4-PoC.
**Blockers**: None.
**Notes**: ADR-011 is Proposed/forward-looking — no multi-cam hardware yet; seam designed now.

---

## 2026-06-28 (cont. 2) — M1 accuracy review & documentation

**Duration**: ~1 hour
**What I did**: Ran the M1 pipeline on the first real swing clip
(`data/raw/golf_swing-aaron-1.mov`, 480×854, 58.9 fps, 674 frames) and did the accuracy
review. Then documented the model + this step thoroughly so it's easy to pick back up.
**Findings** (full write-up in
[docs/archive/M1_CAPTURE_FLOW.md → M1 findings](docs/archive/M1_CAPTURE_FLOW.md#m1-findings-accuracy-review-2026-06-28)):
- 100% detection, avg visibility 0.78; **upper body tracks well**; **59 fps is plenty**.
- **Knees/lower body weak during the bent swing posture**, fine once standing; plus some
  **jitter**.
- Ran a **lite-vs-heavy model experiment on the same clip** — heavy did NOT fix the knees
  (and is 3–5× slower). Inspecting matched frames, the cause is the **picture** (dim light,
  dark shorts on dark floor = no leg contrast, cluttered background), not model size.
**Key decisions**:
- **Keep the lite model** (`pose_landmarker_lite.task`). Documented the whole model story —
  Tasks API vs legacy Solutions API, the lite/full/heavy variants, the download URL/location
  (`data/models/`, via `_ensure_model`) — in **ADR-002 addendum** and the M1 flow doc's new
  **"Pose model (reference)"** section.
- The lower-body fix is a **better recording**, not code: more/even lower-body lighting,
  declutter, contrast the legs, tripod framing. Captured in the findings.
**Where I left off**: M1 is effectively done and fully documented. Pose-setup refinement
continues **later**: (1) re-record with the lighting/background fixes, (2) optionally add a
temporal-smoothing pass for jitter. Otherwise ready to start **M4-PoC** (tempo) on the
keypoints JSON whenever we choose.
**Blockers**: None.
**Notes**: heavy model + heavy overlay were generated as a one-off experiment (gitignored
under data/); committed default remains lite.

---

## 2026-06-28 (cont.) — M1 implementation

**Duration**: ~1.5 hours
**What I did**: Implemented Milestone 1 (Capture & Skeleton). Wrote `FileVideoSource`
(`capture/file.py`, a `VideoSource` adapter over `cv2.VideoCapture`), implemented
`estimate_pose` (`pose/estimator.py`) with the raw→`FrameKeypoints` mapping isolated in a
pure `_to_frame_keypoints` helper, added `draw_skeleton` (`pose/overlay.py`) working purely
off our contract, and wired the lot in `scripts/run_pose.py` (capture → pose → keypoints
JSON + skeleton overlay mp4). Added tests for the pure mapping (no ML deps) and for
`FileVideoSource` (guarded by `importorskip` so the base suite still runs). Set up a `.venv`,
installed `.[vision,dev]`, and verified end-to-end on a synthetic clip. Wrote the M1 design
doc `docs/archive/M1_CAPTURE_FLOW.md` (with mermaid data-flow + sequence diagrams).
**Key decisions / surprises**:
- **MediaPipe Tasks API, not the legacy Solutions API.** The installed mediapipe (0.10.35
  on Python 3.13) has *removed* `mp.solutions` — only `Image`/`ImageFormat`/`tasks` remain.
  Rewrote the estimator to use `PoseLandmarker` (VIDEO mode); it needs a `.task` model
  bundle, so `_ensure_model()` downloads `pose_landmarker_lite.task` (~5 MB) into
  `data/models/` on first run. Corrected `docs/archive/M1_CAPTURE_FLOW.md` accordingly.
- Undetected frames emit 33 placeholder landmarks at visibility 0 (one record per frame) so
  the timeline stays aligned for M4-PoC.
**Verification**: `ruff check` clean; `pytest -q` → 9 passed (4 contract + 3 mapping + 2
capture); `python scripts/run_pose.py <synthetic.mp4>` produced a 15-frame keypoints JSON
(33 landmarks each) and an overlay mp4. Synthetic artifacts cleaned up afterward.
**Where I left off**: M1 code is complete and verified mechanically. **Next: the human
accuracy review** — drop a real swing clip at `data/raw/swing.mp4`, run `run_pose.py`, and
confirm the skeleton tracks address→follow-through; then document findings (30fps enough?
keypoints stable?) to close M1. After that, M4-PoC (tempo) can consume the keypoints JSON.
**Blockers**: None — needs a real swing clip for the accuracy review (no hardware required).
**Notes**: `.venv` created locally; `data/models/` and `data/processed/` are gitignored.

---

## 2026-06-28

**Duration**: ~1 hour
**What I did**: Design session on the central question — *how does the system decide what a
good swing is, distinguish good from bad, and pick the correction?* Surveyed where real
benchmark data comes from (outcome: TrackMan tour/amateur averages, Arccos/Shot Scope,
FlightScope, ShotLink; mechanics: TPI kinematic sequence + X-factor, GEARS/AMM3D/K-Vest,
academic biomechanics, Tour Tempo). Concluded the industry moat is *owning the data*, so v1
leans on published norms stored as cited data and migrates to our own captured data later.
Made four decisions and wrote them up as ADR-009 and ADR-010, then updated ROADMAP.
**Key decisions**:
- ADR-009 — **dual-axis scoring**: separate `mechanics_score` and `outcome_score`, combined
  by a **scoring policy chosen by the user's practice mode** (fundamentals / shot-shaping /
  performance / drill). Intent (`PracticeGoal`) parameterizes outcome ranges, so "good fade
  when I wanted straight = bad" and "don't care where it went, grade my fundamentals" both
  fall out naturally. `SwingResult` + `analyze_swing` carry intent and two sub-scores from
  day one. Supersedes the original single-score M4 framing.
- ADR-010 — **benchmark ranges as versioned data with provenance** (not hardcoded), resolved
  via `resolve_range(checkpoint, club, profile)` with most-specific→`all` fallback;
  parameterized by `ClubCategory` and `PlayerProfile`. v1 seeds **Tour Tempo (~3:1)** only;
  expand to TPI / TrackMan / Arccos as M2/M3 land, then to our own calibration data.
- PoC scope — **M4-PoC: pose-only Fundamentals analysis**. Prove phases→checkpoint→score→tip
  end-to-end on the M1 skeleton with the tempo checkpoint, one scoring policy, the intent +
  dual-axis seam in place but only Fundamentals implemented. No club detection, no hardware.
**Where I left off**: Decisions captured in ADR-009/010; ROADMAP now has M4-PoC before the
full M4. No analysis code written yet — `analysis/engine.py` and `feedback/rules.py` are
still stubs. M1 (capture + pose) remains the prerequisite for M4-PoC.
**Blockers**: None. M4-PoC depends on M1 producing keypoints first.
**Notes**: M1 is still the next *code* step; M4-PoC is the first analysis iteration that
follows it.

---

## 2026-06-20

**Duration**: ~1.5 hours
**What I did**: Picked the project back up after the planning session. Reviewed all docs/ADRs to re-orient. Decided to decouple software development from hardware acquisition and wrote ADR-007 to capture it; annotated ROADMAP milestones with explicit hardware dependencies. Discussed the role of YOLOv8 and how swing path is captured. Assessed how hard reliable club-head tracking really is and decided to de-risk it before investing — added **Milestone 1.5: Club-Head Detectability Spike** with a go/no-go gate. Then designed and scaffolded the whole project per ADR-008: `src/golf_coach/` package, the shared `contracts/` seam (fully implemented Pydantic models), ports + a working mock shot source, module stubs, pyproject with optional extras, tests, scripts, spikes/, frontend/, data/. Verified: `pip install -e '.[dev]'` works, `pytest` green (4 contract tests), `ruff` clean. Added `docs/FLOW.md` with PROPOSED-flow mermaid diagrams (runtime data flow, decoupling seam, build order + hardware gates, two-source swing path) and flagged ARCHITECTURE.md's diagrams as proposed too.
**Key decisions**:
- ADR-007 — software and hardware tracks run in parallel. M1 (capture + MediaPipe pose) starts now with phone/sample video, no purchase needed. MCP server (M3) gets a mock `ShotData` mode from the start. Hardware purchase is a parallel task, not a blocker.
- Added M1.5 detectability spike: prove the club head is detectable (especially through impact) BEFORE labeling 200–500 images. Strategy options to choose from once we see real frames: pure-ML / marker-assisted / fusion+interpolation. M2 labeling is now gated on this spike. Charter risk register and ADR-003 updated accordingly.
- ADR-008 — project structure & decoupling: a shared `contracts/` package is the seam (modules never import each other), ports+adapters at I/O boundaries (real vs mock), analysis is a pure functional core, heavy deps are optional extras so any module runs without the ML stack.
**Clarified**: YOLOv8 detects the club head + ball (MediaPipe only tracks the body); its detections through a tracker produce the visual club-path arc. The Garmin R10's `club_path` is the numeric counterpart. YOLOv8 is also the train-it-yourself ML exercise (ADR-005) and the most cuttable piece if scope tightens. Important correction: **global shutter removes distortion, not motion blur** — sharp impact frames need fast shutter + bright light (ADR-003 addendum).
**Where I left off**: Project is scaffolded and the contracts seam is real (tests green). Next concrete step is implementing **M1**: install the `vision` extra, write `FileVideoSource` (capture/file.py), implement `estimate_pose` (pose/estimator.py) over a sample/phone clip, and render a skeleton overlay via `scripts/run_pose.py`. In parallel, the M1.5 spike — grab a few swing clips with the impact zone and eyeball club-head detectability before any labeling.
**Blockers**: None for M0/M1/M1.5. Hardware purchase (cameras + R10) pending but no longer blocking.
**Notes**: —

---

## 2026-03-16

**Duration**: ~1 hour
**What I did**: Project planning session. Created project charter, architecture doc, roadmap, ADR templates, and decision log. Defined tech stack and milestone sequence.
**Key decisions**: Python stack, MediaPipe for pose, YOLOv8 for detection, MCP server for launch monitor, Claude API for coaching. See `docs/decisions/` for full ADRs.
**Where I left off**: No code yet. Next step is Milestone 1 — acquire/set up camera, record a test swing, run MediaPipe.
**Blockers**: Need to decide on and purchase camera hardware (ADR-003) and launch monitor (ADR-004).
**Notes**: —

---

<!--
TEMPLATE — copy this for each session:

## YYYY-MM-DD

**Duration**: 
**What I did**: 
**Key decisions**: 
**Where I left off**: 
**Blockers**: 
**Notes**: 

-->
