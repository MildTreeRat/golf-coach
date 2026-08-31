# M11 — Acoustic Sync: the ball strike is the clock

> **Tier: REFERENCE.** Read the reasoning, not the digits. Every phase is built, so the plan text
> is history and the *As built* notes are what happened; every table is a measurement of the 15
> bundles on disk on the day it is dated — 2026-08-29 throughout — and a bundle re-analysed after
> that will not match. **§The evidence (§E1–§E5)** is the pre-build measurement and **P9's *As
> built* is the post-build one**; where they disagree, P9 is later. **§E4 was found by P0 after
> P5–P7 were written, and contradicted an assumption they rested on** — it is why P6 and P7 read
> the way they do. The
> *why* behind the existing alignment is
> [ADR-015](decisions/015-handheld-two-phone-capture-and-event-anchored-alignment.md); this
> document proposes taking the option that ADR parked.

**Status: 12/12 phases built**, closed 2026-08-30 — **and read §Addendum before §E4 and P9's *As
built***, because the eye check §Verification asks for was run the day after P9 and it moved the
defect: on four down-the-line clips the *video* decode ignored an edit list the *audio* decode
applies, so a frame index and a sample index described timelines ~105 ms apart. §E4's "5.7–7.5
frames early" is that offset, not a pose error. P0 verified M10's handoff and established the
baseline; P1–P4 built the audio path; P5 put it into swing *selection*; P6 into *synchronization*;
P7 closed M10's residual defect; P8 was the paperwork and `ANALYSIS_VERSION` 12; P9 re-ran the
corpus; **P10 and P11 are the addendum's two fixes, which had to land together** — the video's own
presentation offset, and the candidate floor P9 specified and reverted. Every stored bundle reads
`synchronized`, 30/30 clips are pinned to a strike they heard, and the anchor now agrees with
contact by eye to within a frame on all ten clips read frame by frame.

## What this milestone is

M10 repaired the *windowing* and handed forward one defect it could not close: **the face-on top
lands late.** On four bundles (`2026-08-23/2, 4, 5, 9`) the two views disagree about the downswing
in the same direction every time — face-on measures 0.183–0.267 s where down-the-line measures
0.384–0.484 s of the same swing. It costs three `tempo` readings that score and *fail* at 4.92,
6.08 and 6.09:1 against a 4.71 ceiling, and M10 P10's *As built* says in as many words that those
are **not coaching truth**. One bundle still has no down-the-line window at all.

Every one of those failures has the same root: **the two views infer the swing independently, from
pose, and nothing external adjudicates.** ADR-015 built the normalized `tau` axis around the
premise that "there is no shared clock", and M10 P9's addendum then had to record that fps had
entered the design in five places anyway, because *the anchors need real time even though the warp
does not*.

**There is a shared clock, and it has been on disk the whole time.** Both phones record the ball
strike: a sharp broadband transient, at one instant, from a physical event neither camera can
misinterpret. ADR-015 considered exactly this — Option C, audio cross-correlation — and **parked
rather than rejected** it:

> **Not rejected on merit**: it is the strongest alternative here and would compose with the chosen
> option as a refinement, which is where it is parked. […] Revisit only with a concrete need — it
> costs a dependency and an assumption this design currently does not make.

M10's residual defect is that concrete need. This milestone spends the dependency and tests the
assumption.

**Outcome:** two panels that leave address together *and* strike the ball on the same output frame,
because they are anchored on one physical event rather than two independent inferences; a swing
selector that can tell a real swing from a rehearsal by whether a ball was struck; and an end to
the late-top defect and the three untrustworthy tempo readings.

## Three things that will look obvious and are wrong

- **Audio does not give you the top.** It gives you *impact*. The top stays a pose inference and it
  is the broken one. What audio buys is that the two views' tops become comparable **in real time**,
  which turns "one of these is wrong" from an observation into a decidable question (P7). Do not
  plan as though impact detection fixes `tempo` directly — it does not, it makes fixing it possible.
- **Do not detect impact absolutely when you only need an offset.** Cross-correlating the two clips'
  audio recovers the clip-to-clip offset *without identifying which transient is the ball*, because
  every transient appears in both recordings. That is the robust half, and it is all P6 needs.
  Absolute strike identification is harder and only selection (P5) requires it. This is why P6 does
  not depend on P5.
- **The bay has four transients, not one.** Club–ball contact; the club striking the mat
  ~immediately after; the ball hitting the impact screen later; and the simulator's own speakers
  playing ball-flight audio. Any rule that takes "the loudest peak" will find the screen strike.
  ADR-015 named this cost and it is real — **and P0 measured it worse than this bullet first
  guessed**: the screen lands 85–145 ms after the ball, not 60–80, and it is the louder of the two
  on *every* clip measured, not some. §E5 has the numbers; use those, not these.

## Reading order for a fresh session

`CLAUDE.md`, then ADR-015 (especially Option C and the 2026-08-26 addendum), then this file, then
the two-to-four files your phase names. Every phase states its own files and what to reuse, so a
phase number is a complete handoff.

**This is an L3 — STRUCTURAL change** by CLAUDE.md's ladder: a new subsystem, a new extra,
additions to `contracts/`, and an `ANALYSIS_VERSION` bump. Read ADR-008 (import rules), ADR-010 §2
(no score beats a wrong one), ADR-013 (clip-relative detection, and decline rather than guess),
ADR-015, and `docs/ARCHITECTURE.md` §2–§3 before touching `contracts/alignment.py`.

---

## The evidence

Container facts measured **2026-08-29** over `data/processed/sessions/`, 30 clips across 15
bundles, by parsing the MOV atom tree directly — no ffmpeg, no decode. These are the facts the
design rests on; re-measure before trusting them if the corpus has changed.

### E1 — the audio survives, on every clip

ADR-015's Option C carried one unverified assumption: *"it assumes the upload path preserves the
audio track."* It does.

| fact | value |
|---|---|
| clips with an audio track | **30 / 30** |
| codec / channels / rate | `mp4a`, stereo, **48 000 Hz** on every clip |
| audio samples per video frame at ~60 fps | **800** |

So the *resolution* available for an anchor is roughly 1/800th of a frame. The limit on this
milestone is not precision; it is disambiguation and timebase bookkeeping.

### E2 — the audio edit lists are non-uniform, and that is the trap

Read from each track's `edts/elst`, alongside `mdhd` timescales:

```
2026-08-23/2   face_on    soun  elst=[(11834, 2112)]     <- skip 2112 samples
               face_on    soun  elst=[(11834, 0)]        <- SECOND audio track, no skip
               vide             elst=[(11835, 0)]        ts=600
2026-08-23/9   down_line  soun  elst=[(7574, 0)]         <- no skip at all
               down_line  vide  elst=[(105, -1), (7555, 1632)]   ts=19200, empty edit
```

Three findings, each load-bearing:

- **`media_time = 2112` samples is AAC encoder priming — 44 ms, or 2.64 frames at 60 fps.** Decode
  naively from sample zero and every such clip inherits it.
- **It is not uniform.** Most tracks carry it; `2026-08-23/9`'s down-the-line track carries `0`. A
  *constant* bias would cancel between the two views and cost nothing. One that is 44 ms on one clip
  and 0 on the other does not cancel, and it is the same order as the error this milestone exists to
  remove.
- **Face-on clips carry two `soun` tracks** — iPhone spatial audio — with *different* edit offsets.
  Picking the wrong one is a silent 44 ms. (**Amended by P1, 2026-08-29**: ffmpeg identifies the
  second as `apac`, Apple Positional Audio Codec, and ships no decoder for it, so taking it raises
  rather than shifting silently. The stream is still selected explicitly, because *which* one a
  default selection picks is a heuristic and not this project's decision.)

`2026-08-23/9`'s down-the-line video is also the odd one out structurally: timescale 19200 with a
two-entry edit list including an empty edit, where the clips around it are a plain timescale 600. It
is therefore the best single test case in the corpus for a timebase bug, and it is one of the four
bundles carrying the late-top defect.

**Amended by P0, 2026-08-29: it is not alone, and the company it keeps is the point.** Four
down-the-line clips share that structure — bundles **1, 7, 9 and 11** — and they are exactly the
four where the two views' impact anchors disagree in real time (§E4). The face-on clips are uniform
across the whole corpus: video timescale 600, no video edit, 2112 samples of audio priming. So the
non-uniformity is not scattered noise; it is one phone, in one recording mode, on one role.

**The consequence for the design:** the decode path must let a real demuxer apply the edit list
rather than reading samples from zero. That is an argument for the dependency, not merely a
convenience of it.

### E3 — what M10 handed over

From `docs/M10_ALIGNMENT_ACCURACY.md` P10 *As built* and its §Verification, and **verified
2026-08-29 by P0** against the stored `analysis.json` of all eleven 2026-08-23 bundles. M10's
document is tier REFERENCE and its numbers were dated 2026-08-26, so this was drift-checking, not
formality. Every row survived; one needs widening.

| symptom | bundles claimed | measured 2026-08-29 |
|---|---|---|
| face-on downswing 0.183–0.267 s against DTL 0.384–0.484 s | 2, 4, 5, 9 | **holds exactly** on those four — and **bundle 7 belongs with them**: 0.250 s against 0.367 s, carrying the identical `downswing durations disagree` note. The family is **five** bundles; M10's stated range simply stops just above 7's numbers |
| `tempo` scores and fails at 4.92 / 6.08 / 6.09:1 | 2, 4, 5 | holds exactly — scores 0.89 / 0.31 / 0.31, all `passed=false` |
| no down-the-line window at all | 5 | holds — `down_the_line_window` is `null` |
| down-the-line backswing collapsed to ≤1 frame | 2, 5 | holds — 1 frame, 0.017 s, on both |
| tier spread over the eleven 2026-08-23 bundles | 3 `full`, 3 `top_impact`, 5 `impact_only` | holds exactly |

**One thing that table does not say, and P9 will need.** `tempo` passes on exactly two of the eleven
bundles — 7 at 4.27:1 and 9 at 4.06:1 — and *both are in the disagreement family*. They pass on the
same short face-on downswing that makes 2, 4 and 5 fail; they are simply less short. A corrected,
longer downswing lowers a tempo ratio, so **M11 will move the passes as well as the failures**, and
a pass here is not a result to preserve. Baseline for P9, all eleven bundles, `tempo` observed:
2.61, 4.92, 2.57, 6.08, 6.09, 2.36, **4.27 ✓**, 1.89, **4.06 ✓**, 2.50, 2.50.

**And one claim the stored alignment notes make that P0 found to be false.** Bundles 4, 7 and 9
each carry the note
*"Most likely the two clips are showing DIFFERENT swings; check for a practice swing in one of
them"*. They are not. §E4 cross-correlates the two views' audio on each of those bundles and finds
one strike, matched across both clips at r = 0.76–0.83 — the same swing, filmed twice, with a bad
boundary in one view. **P5 should not be designed to catch what that note describes**, because in
every instance the corpus has of it, the note is wrong about the cause.

### E4 — the impact anchor is wrong too, on four bundles, by five to seven frames

**This is the finding P0 was not looking for, and it changes P6.** Measured 2026-08-29 through
the P1 decode path over all eleven 2026-08-23 bundles.

Method, twice over so that neither leg carries the claim alone. *Onset spotting:* a 5 ms-hop RMS
envelope in a ±0.6 s window around each view's own pose-inferred impact, reading off where the
transients actually land. *Cross-correlation:* the P6 primitive, on flux envelopes over ±1.5 s, no
transient identified at all. The two agree everywhere.

| bundle | tier | audio-vs-pose impact offset | verdict |
|---|---|---|---|
| 2, 3, 4, 5, 6, 8, 10 | mixed | −0.3 … +0.3 frames | pose impact is right, in **both** views, to within the 5 ms hop |
| 1 | `top_impact` | −7.2 frames | down-the-line impact is **early** |
| 7 | `impact_only` | −6.9 frames | down-the-line impact is **early** |
| 9 | `impact_only` | −5.7 frames | down-the-line impact is **early** |
| 11 | `full` | −7.5 frames | down-the-line impact is **early** |

The correlation peak is decisive rather than marginal: r = 0.59–0.85 at the winning lag against a
best rival of 0.15–0.47. The rival peaks do sit at ±6–8 frames, which is the ball-to-screen gap of
§E5 — the four-transient trap is real and visible in the correlation function — but it never wins,
and never by less than 0.12.

**Which view is wrong is not a guess.** On the seven healthy bundles the pose impact lands within
one frame of an audio onset *in both views*, which is the calibration. On all four offenders
face-on still does (+0.010 to +0.015 s) and down-the-line does not (+0.100 to +0.115 s). So the
down-the-line impact is the early one, consistently, and face-on impact is sound throughout the
corpus.

**Three explanations were excluded before this was written down.**

- *A container A/V bias.* Those four are exactly the bundles whose down-the-line clip has the odd
  structure §E2 flagged on `2026-08-23/9`: `mvhd` timescale 1000, video timescale 19200, a
  two-entry edit list `[(≈110, -1), (N, 1632)]`, and **no** audio priming offset where every other
  track has 2112 samples of it. That is a real bias and it is in the right direction — but it is
  worth only **≈22 ms**, one and a third frames, against the ≈110 ms measured. It accounts for a
  fifth of the error and is a correction P6 must apply, not the cause.
- *A variable frame rate.* Ruled out by walking each video track's `stts`: media time and
  `frame_index / fps` agree to ≤0.001 s on every clip in the corpus, offenders included.
- *Sound travel.* ADR-015 and §Risks both name it; at ~34 cm/ms, 110 ms is 37 metres. Not a bay.

One loose end P0 did not chase, recorded so the next session does not rediscover it: on those same
four down-the-line clips the container's `stts` counts **three more frames** than the stored
`SwingAnchors.frame_count` (527/524, 806/803, 453/450, 431/428), while `cv2.VideoCapture` reports
the container's number. Every other clip agrees exactly. Three frames is 50 ms and the sign is
untested; it may be a trailing-frame difference costing nothing.

**What this means for the milestone.** `AlignmentQuality` currently treats a `full` tier as three
agreeing anchors — and bundle 11 is `full` while its two views' impacts are 7.5 frames apart in real
time. So the tier inversion §Design names is worse than it looked: `full` is not evidence that the
impact anchor is right either. It also means P6 has a defect to fix that is *separate from and
larger than* the one M10 handed over, on a partly different set of bundles — only bundle 9 is in
both. Do not assume the late-top set and the early-impact set are one problem.

### E5 — the four transients, measured

§Three things predicted the bay has four transients and that the screen strike lands 60–80 ms after
the ball. Measured over fourteen clips — both views of bundles 1, 2, 4, 5, 7, 9 and 11:

- The **screen strike lands 85–145 ms after the ball**, not 60–80 — 5 to 9 frames at 60 fps.
- It is **louder than the ball on every clip measured**, typically by 3–4× in onset flux (6100–8900
  against 400–3000). "Take the loudest peak" does not fail on *some* clips; it fails on all of them.
- The club–mat transient is not separable at a 5 ms hop; where the ball onset resolves into two it
  is 15–20 ms apart, not the 3 ms §Three things assumed.

P3's synthetic double-onset test should use these numbers rather than the estimates it was written
against.

---

## Design

### The seam: audio is an I/O-edge adapter, exactly like pose

The analysis core is stdlib + `contracts` only and that is load-bearing (ADR-008). So this mirrors
the shape `vision` already has rather than inventing a new one:

| Pose (existing) | Audio (new) |
|---|---|
| `capture/source.py` — `VideoSource` port | `audio/source.py` — `AudioSource` port |
| `capture/file.py` — `FileVideoSource` | `audio/ffmpeg.py` — `FfmpegAudioSource` |
| `pose/estimator.py::estimate_pose` | `audio/impact.py::detect_strikes` |
| `contracts/keypoints.py` — `KeypointsFile` | `contracts/audio.py` — `AudioFile` |
| `{role}.keypoints.json`, keyed on `source_sha256` | `{role}.audio.json`, keyed on `source_sha256` |
| `api/pipeline.py::keypoints_for` | `api/pipeline.py::audio_for` |

What crosses into `analysis/` is **an integer frame index and a confidence** — plain data, the same
way ADR-022 has fitted models cross as JSON. `analysis/alignment.py` and `analysis/phases.py` stay
stdlib and never learn what a decoder is.

### A new alignment tier, because this is a synchronization and not an alignment

`AlignmentQuality` currently ranks how many *pose* anchors were available. An audio-anchored pair is
different in kind: it has a real shared clock, which ADR-015 §Context says the phone tier does not
have. Add one member above `FULL`:

```python
SYNCHRONIZED = "synchronized"   # summary: "synchronized on the ball strike"
```

A pure addition — no artifact on disk can carry it, and `_QUALITY_SUMMARY` gains one row. Without
it the results page reports `full` for two qualitatively different things.

**Note the inversion this creates, and handle it deliberately.** `IMPACT_ONLY` is today the *worst*
non-failing tier, on the reasoning that one anchor is less evidence than three. With a *measured*
impact that reasoning flips: one measured anchor beats three inferred ones. P6 must not leave a
bundle reading `impact_only` when its single anchor is the most trustworthy number in the system.

### What this does not change

- **The `tau` axis stays.** ADR-015's normalized axis is still right for the reason it was chosen,
  and M10's addendum already drew the line this milestone works along: *the warp does not need fps;
  the anchors do.* Audio improves an **anchor**. Do not delete the axis.
- **Down-the-line stays capture-and-align only.** A shared clock is not calibration; ADR-015's
  Option D remains unreachable and no DTL checkpoint becomes scoreable here.
- **No re-tuning of `_DRAWDOWN_FLOOR`.** Its 0.012 is the argmax of a documented 461-clip sweep and
  it is correct for single-view work. P7 fixes the top at the *cross-view seam*, which is the only
  place the evidence to decide exists.

---

## Phases

Each phase is independently commit-ready. `tests/` mirrors `src/golf_coach/` package by package, so
a module's test file is its path with `tests/` on the front.

### [x] P0 — verify M10's claims, and establish the baseline

**Goal.** Know that §E3 is still true before building on it, and produce the *before* column P9
fills in.

**Files.** None in `src/`. This document's §E3 table gains a measured column.

**Detail.**
```bash
.venv/Scripts/python.exe -m pytest
.venv/Scripts/python.exe -m ruff check src tests scripts
.venv/Scripts/python.exe -m mypy src
.venv/Scripts/python.exe scripts/reanalyze.py --all --dry-run
.venv/Scripts/python.exe scripts/career_corpus.py
```

Then read the stored `analysis.json` for `2026-08-23/{2,4,5,9}` and check each row of §E3 against
it. **Record what disagrees rather than correcting it silently** — M10's doc is REFERENCE tier and
dated, so drift is information, not a failure.

**As built.** Ran 2026-08-29. Nothing in `src/` changed; §E3 gained its measured column and §E4 and
§E5 are new.

*The gates are green and the corpus is current.* `pytest` 1080 passed; `ruff check src tests
scripts` and `mypy src` clean over 98 files; `reanalyze.py --all --dry-run` reports all 15 swings up
to date at engine version 11, so the *before* column is a real baseline and not a stale artifact;
`career_corpus.py` counts 13 distinct swings over 21 metrics with 2 re-uploads collapsed and nothing
excluded as `OUTDATED`.

*§E3 holds, with one row widened.* Every symptom M10 handed over is still on disk exactly as
described. The one correction is that the downswing-disagreement family is **five** bundles rather
than four — bundle 7 carries the identical note and M10's quoted range just stops short of its
numbers.

*P0 went past its brief, deliberately, and the reason is §E4.* §Risks asks P0 for spike output
showing real envelopes before P5 depends on them. Producing those meant decoding real clips through
P1's path, and the envelopes showed something the plan does not contain: on four bundles the
**impact anchor itself** — the one anchor this milestone treats as the reliable half — is five to
seven frames out between the two views. That is a larger error than the late top M11 was written to
fix, it sits on a partly different set of bundles, and one of them is `full` tier. It was measured
two independent ways and three alternative explanations were excluded before being written down.
**Read §E4 before planning P6 or P7**; the phase text for both was written without it.

*Three consequences the later phases should absorb, none of them acted on here.* P3's synthetic
onsets should use §E5's measured 85–145 ms screen gap rather than the 60–80 ms estimate, and should
know the screen is the *louder* transient. P6 must apply a per-clip container correction of ≈22 ms
on the four §E4 bundles before an audio offset can be laid against frame indices — the audio and
video timelines are offset *within* those clips. P5 should not be built around the "two clips are
showing DIFFERENT swings" note: it fires on bundles 4, 7 and 9, and on all three the audio proves
one swing filmed twice.

*The spike scripts were not kept.* Five throwaway scripts — envelope, cross-correlation, correlation
ambiguity, MOV atom/edit-list parse, `stts` walk — ran from a scratch directory and are not in
`scripts/`. Nothing in this repo consumes them, and P3 rebuilds the two that matter behind a test.
The measurements they produced are recorded above so nobody has to re-derive them; re-run the
milestone's own code if the corpus changes.

### [x] P1 — the `audio` extra and the decode port

**Goal.** Turn one clip into mono PCM, with the edit list applied.

**Files.** `pyproject.toml`; new `src/golf_coach/audio/__init__.py`, `source.py`, `ffmpeg.py`.
**Tests.** New `tests/audio/test_ffmpeg_source.py`.

**Detail.** New extra, modelled on the `ocr` block's comment style — the closest precedent, an extra
needed only to *import* while the readers run bare:

```toml
audio = [
    "imageio-ffmpeg>=0.4",   # ships a static ffmpeg binary as a wheel; no system install
]
```

Add it to the `all` convenience extra, and to the extras list in `CLAUDE.md` §Commands.

`FfmpegAudioSource` subprocesses that binary to decode one clip's audio to mono 16-bit PCM at a
fixed rate. **Three things it must get right, each with a comment recording why** (§E2 has the
measurements):

1. Let ffmpeg apply the edit list — pass nothing that disables it. The 2112-sample priming offset is
   2.64 frames and is present on some tracks and absent on others.
2. Select the stream explicitly (`-map 0:a:0`), because face-on clips carry two.
3. Downmix to mono and resample to one known rate, so nothing downstream needs a per-clip case.

Follow `capture/source.py`'s port/adapter split, including its docstring convention of naming the
adapters in the module docstring, and keep numpy out of the port's signature the way `capture` keeps
pixels out of `contracts`.

**As built.** Landed as planned, with three things worth recording.

*The edit list is applied, and the asymmetry is real.* Verified through the decode path itself on
2026-08-29 by decoding each track twice and differencing the sample counts: `2026-08-23/2` skips
2112 samples on **both** views — so it cancels there and costs nothing — while `2026-08-23/9` skips
2112 on face-on and **0** on down-the-line. That is 44 ms uncancelled, 2.64 frames at 60 fps, on one
of the four bundles carrying M10's late-top defect. §E2 predicted this from the atom tree; it now
holds through ffmpeg as well.

*The second face-on audio stream cannot be decoded at all.* §E2 called it "iPhone spatial audio"
from the container; ffmpeg identifies it as `apac` (Apple Positional Audio Codec) and this build
ships no decoder for it, so `stream_index=1` raises rather than silently returning a 44 ms-shifted
waveform. `-map 0:a:0` is still explicit rather than default — which stream ffmpeg's selection
heuristic picks is not this project's decision, and it prefers by channel count and bitrate, where
the undecodable track is the 4.0 one.

*Two deviations from the file list.* The port carries an `AudioClip` and a `NoAudioTrackError`
alongside the `AudioSource` protocol — a clip with no audio track is a bundle that cannot be
acoustically anchored (a note, then fall back), where a decode failure is a broken tool, and P4
needs to tell them apart. And there is a second test file, `tests/audio/test_source.py`, which
takes no `importorskip`: it is the only place a **base install** proves the port imports without
ffmpeg or numpy, which is the ADR-008 invariant this milestone is most likely to break.

Samples cross the seam as a stdlib `array("h")`, not a numpy array, so the port stays importable
bare; `np.frombuffer(clip.samples, "<i2")` is a zero-copy view of it for P3.

### [x] P2 — contracts and storage for an audio track

**Files.** New `src/golf_coach/contracts/audio.py`; new `src/golf_coach/storage/audio_io.py`.
**Tests.** New `tests/contracts/test_audio.py`, `tests/storage/test_audio_io.py`.

**Detail.** Mirror `contracts/keypoints.py`, including its rule that every field is optional and
`None` means unknown, never zero:

- `AudioStrike` — `frame: int`, `sample: int`, `confidence: float`, `prominence: float`.
- `AudioClipMetadata` — `sample_rate`, `duration_s`, `source_sha256`, `stream_index`.
- `AudioFile` — `clip: AudioClipMetadata | None`, `strikes: list[AudioStrike]`.

`load_audio` / `save_audio` go in `storage/audio_io.py` written against
`storage/keypoints_io.py`'s tolerant-reader pattern. **Reuse that module's shape rather than writing
a second tolerant reader** — CLAUDE.md's warning that a second copy is a second thing that drifts
applies directly here.

**As built.** Landed as planned, with one field added, one type widened, and one piece of
`keypoints_io`'s shape deliberately not copied.

*`AudioStrike.frame` is `int | None`, not `int`.* P3's `detect_strikes(samples, rate)` is handed a
waveform and a sample rate and has never seen the video, so it cannot compute a frame index — fps
arrives only at P4, which holds the manifest. Making the field non-optional would have forced the
detector to invent one, and ADR-010 §2 says an underivable number is `None` and not zero. `sample`
is the measurement; `frame` is the derived convenience filled in later.

*`AudioClipMetadata` carries `fps`, which the file list did not name.* It is the rate every `frame`
on that file was derived under, recorded for exactly the reason `stream_index` is: a sample index
means nothing without the stream it counts into, and a stored frame index that cannot be re-derived
is a number nobody can check. It is not a copy of `keypoints.ClipMetadata.fps` for its own sake —
nothing reads it as a video fact.

*No bare-array branch in `load_audio`.* `load_keypoints` accepts one because hundreds of
pre-envelope files exist and will not be regenerated. Audio has no such history, so a bare-array
branch here would not be tolerance of a real file — it would be a shape invented out of symmetry
that then has to be supported forever. The `isinstance` seam is written so a future legacy shape
lands on it, and `test_a_bare_array_is_rejected_and_names_the_file` pins the current answer.

The two savers are four near-identical lines (`mkdir`, `model_dump_json(exclude_none=True)`) and
were left duplicated: CLAUDE.md's warning is about a second tolerant *reader*, and the readers are
the halves that legitimately differ. `tests/storage/test_audio_io.py` ends with a subprocess check
that importing `storage.audio_io` pulls in neither `imageio_ffmpeg` nor numpy — P4 adds the matching
pin for `api/pipeline.py`, but the storage half of the split is provable now and is what lets a base
install read what the `audio` extra wrote.

### [x] P3 — strike detection and clip-to-clip offset

**Files.** New `src/golf_coach/audio/impact.py`.
**Tests.** New `tests/audio/test_impact.py`, building synthetic waveforms in the style
`tests/analysis/conftest.py` builds synthetic wrist tracks — a stated shape, with the reason for
each wobble in a comment.

**Detail.** Two functions, deliberately separate (see §Three things, second bullet):

- `detect_strikes(samples, rate) -> list[AudioStrike]` — onset detection on a short-window energy /
  spectral-flux envelope, returning **every** candidate ranked by prominence rather than one answer.
  Same design choice `phases.candidate_downswings` made, for the same reason: the caller sees how
  many there are instead of discovering by eye that the wrong one was taken.
- `offset_between(a, b, rate) -> float | None` — cross-correlation, returning seconds. Identifies no
  transient at all, which is what makes it the robust primitive P6 leans on.

numpy is fine in this module: it lives behind the `audio` extra, not in `analysis/`.

**Test the four-transient case explicitly.** A synthetic double onset 3 ms apart (ball then mat) and
a second onset 70 ms later (the screen) are the cases that decide whether P5 is usable.

**As built.** Landed as planned, with a richer return type than the file list named, a second line
in the extra, and one measurement P6 must act on.

*The detector is half-wave-rectified spectral flux at a 5 ms hop over a 1024-sample window,
thresholded clip-relative (ADR-013).* Measured across all 22 clips of the 2026-08-23 session: the
ball and screen strikes stand at **z = 113–599** above the clip's own flux floor, in median-absolute
deviations, where 20 s of pure room tone — and of amplitude-modulated noise standing in for a voice
— never exceeds **z = 4.4**. The listing floor is z = 8, an order of magnitude below every real
strike and roughly twice the loudest thing an empty bay makes, so **a rehearsal returns an empty
list** and P5's discriminator is as clean as §What this milestone is hoped. A whole clip yields
7–42 candidates: two strong ones per shot plus the reverberant tail behind them.

*§E5's warning holds and is now pinned by a test.* Ranking is by prominence, and the screen strike
outranks the ball on roughly half the corpus, so `strikes[0]` is **not** impact — the earlier of the
top two is. The club–mat transient is deliberately absorbed rather than listed: 50 ms of peak
suppression is the largest that still keeps the ball and the screen apart at §E5's 85–145 ms gap
(measured 70–140 ms by this detector), and it costs a transient no consumer in this milestone needs.

*`offset_between` returns a `ClipOffset`, not a `float`.* A deviation from the file list, for
ADR-010 §2's reason: P6 has to *write down* why it trusted or declined an offset, and a bare float
carries neither how good the match was nor how close the runner-up came. The dataclass is three
numbers — `seconds`, `r`, `runner_up_r` — and stays on the `audio` side of the seam, so if P6 needs
to store one it is a contract to add, not a shape to copy.

*The offset is right on ten of eleven bundles and confidently wrong on the eleventh — read this
before P6.* Measured whole-clip over the 2026-08-23 session against the difference of the two views'
detected ball onsets: **agreement to 0–20 ms (0–1.2 frames at 60 fps)** on bundles 1–7 and 9–11, at
r = 0.838–0.923 against a best rival of 0.125–0.525. Bundle **8** is the exception and it is
instructive: its 80-second down-the-line clip holds **two shots** 43.8 s apart, and the face-on
clip's single shot matches the *second* one at r = 0.923 where the correct answer scores 0.672. No
margin rule can catch that, because both matches are real. **P6 must pass `max_lag_s`**, bounded
from the pose anchors it already holds; that is what the parameter exists for. The margin rule
catches only the case where the two candidates are genuinely alike, which is what the synthetic
repeated-shot test pins.

*numpy joined the `audio` extra, and P1's comment on it was corrected.* That comment said everything
downstream of the decode reads bare; that is true of the stored artifact and was never going to be
true of the detector — an FFT per 5 ms hop is not stdlib arithmetic's job. The boundary that matters
is unchanged and still pinned: `analysis/` has no numpy, and `{role}.audio.json` still reads on a
base install (`tests/storage/test_audio_io.py`).

*One convention P4 will derive frames from.* `AudioStrike.sample` is the first sample of the
analysis window whose new energy produced the rise. Calibrated against the raw waveform on all 22
clips, that runs **+18 ms late on average (sd 25 ms)** against where the amplitude actually starts
climbing — most of the spread being the crudeness of the reference, not of the estimate. It is a
property of the window, so it is identical in both views and cancels in every cross-view comparison
M11 makes; it is *not* zero against an absolute frame index, and P7 should not read a one-frame
face-on discrepancy as signal.

### [x] P4 — `audio_for`: the cached per-view read

**Files.** `src/golf_coach/api/pipeline.py`.
**Tests.** New `tests/api/test_pipeline_audio.py`; extend `tests/api/test_pipeline_imports.py`.

**Detail.** `audio_for(swing_dir, manifest, role, *, force, log, notes)` written as a direct sibling
of `keypoints_for` (`pipeline.py:138`): same cache-on-`source_sha256` contract, same lazy
`try: import ... except ImportError` guard that degrades with a note instead of raising, same
`_note` usage. Writes `{role}.audio.json`.

**The import pin is mandatory.** Add a test to `tests/api/test_pipeline_imports.py` in that file's
existing subprocess style asserting that importing `golf_coach.api.pipeline` does **not** import
`imageio_ffmpeg`. It is the same boundary the file already holds against `fastapi` and `anthropic`,
and it is what keeps the analysis core installable bare.

**As built.** Landed as planned, with one parameter the file list did not name and one consequence
of it worth reading before P5.

*`audio_for` takes an `fps`, and that is a deviation.* The signature is
`audio_for(swing_dir, manifest, role, *, fps=None, force=False, log, notes)`. The detector has
never seen the video (`contracts/audio.py`), so `AudioStrike.frame` is derived and not measured,
and the caller doing the deriving has to be the one holding the frame rate — which is
`_auto_windows`, already carrying `KeypointsFile.clip.fps`. Opening the clip a second time inside
`audio_for` to rediscover a number the pipeline is holding would be a second answer to the same
question, and the two can disagree. With `fps=None` every strike keeps `frame=None`, which is what
P6's cross-correlation should get: it works in samples and has no use for a frame index.

*The frame is re-derived from the cache, never re-heard.* A first call with no fps and a later one
with an fps cost **one** decode between them: the sample index is the measurement and has not
changed, so `_frames_derived` fills the frames in from the stored artifact and writes it back with
the fps they were derived under recorded beside them. The reverse does not happen — a caller
without an fps does not strip frames a caller that had one already wrote (ADR-010 §2). Rounding is
a floor, because a frame index answers *which frame was being exposed*.

*Five ways it returns `None`, four of them notes.* No such role in the manifest (silent — a bundle
with no down-the-line clip is not an audio fault); the clip missing from disk; the `audio` extra
absent; no audio stream in the container; a decode that failed. `NoAudioTrackError` is caught
separately from `OSError` for the reason `audio/source.py` names — one says this bundle cannot be
acoustically anchored, the other says the tool broke — and its note says so in words a reader of
`analysis.json` can act on. **An empty `strikes` list is not one of the five**: it is a result, the
file is written, and it is the signal P5 reads.

*The import pin now covers three extras.* `tests/api/test_pipeline_imports.py` asserts that
importing `golf_coach.api.pipeline` loads neither `imageio_ffmpeg` **nor numpy** — the second half
matters more than it looks, since ADR-008's stdlib-only rule is about `analysis/` and nothing else
would have complained about numpy arriving here.

*Nothing calls it yet.* `analyze_swing_dir` does not run audio; P5 is the phase that threads it
into `_auto_windows`, and `PipelineOptions` grows its `force_audio` flag there rather than here,
where it would be a switch on a path nobody takes. Cost when it is called, measured on this
corpus's clips: a decode is a subprocess of a second or two and detection is an FFT per 5 ms hop,
so a first read is seconds where pose is minutes — and every read after it is the cache.

### [x] P5 — selection: a rehearsal makes no crack

**Goal.** The highest-value half, and worth landing before the sync.

**Files.** `src/golf_coach/analysis/phases.py`, `src/golf_coach/api/pipeline.py`.
**Tests.** `tests/analysis/test_select_swing.py`, `tests/api/test_pipeline_auto_window.py`.

**Detail.** `select_swing` (`phases.py:513`) ranks candidates by `_PLAUSIBLE_DOWNSWING_S`, a
duration band whose own comment concedes the margin is thin — *"The upper bound sits in a 0.06 s gap
(0.42 real against 0.48 decoy), which is thin."* A ball strike is a far better discriminator: a
practice swing has a whoosh and no crack.

Add an optional `strike_frames: list[int] | None` to `select_swing` and `select_matching_swing`
(`phases.py:595`). When supplied, a candidate whose `impact` lands near a strike is preferred
**before** the duration band is consulted. When `None`, behaviour must be identical to today — the
same way `window_around` keeps its no-fps path. Reuse `_lone_candidate_choice` rather than adding a
fourth escape.

In `pipeline.py::_auto_windows` (`pipeline.py:409`), thread strikes through `_pick_swing`
(`pipeline.py:327`). `_narrate_choice` should say when a strike decided it — every other selection
reason in that module already says why.

**Cross-modal check worth taking while here:** the count of strikes in a clip is comparable against
the shots OCR'd from the simulator screen for that session (ADR-014). A disagreement is a note, not
an error.

**As built.** Landed as planned. `select_swing` and `select_matching_swing` each take an optional
`strike_frames: list[int] | None`; `_struck` (`phases.py`) is the filter and `_STRIKE_TOLERANCE_S`
its one constant. `PipelineOptions.force_audio`, an `Audio:` stage in `analyze_swing_dir`, and a
`strikes` argument on `_auto_windows` thread it. `pytest` 1147 passed, `ruff` and `mypy` clean over
101 files. Six things are worth reading before P6.

*The filter narrows the field; it never decides alone.* Rule 0 keeps the candidates whose `impact`
sits within `_STRIKE_TOLERANCE_S` of a heard transient, and rules 1–3 then run **on that pool**.
That is what makes it "before the band is consulted" without adding a fourth escape: with one
struck descent the existing `_lone_candidate_choice` fires and the band is overruled; with several,
the band chooses among them exactly as it did; with none, the whole field goes through untouched.
`_lone_candidate_choice` grew two keywords for it — `candidates`, so `SwingChoice.candidates` still
lists *every* descent when a strike narrowed the pool (a filtered listing cannot show a human that
the filter threw away the right one), and `struck`, because the two justifications differ: without
audio the band is a tie-break with no tie, with it the band has been overruled by the event it only
ever stood in for.

*A strike list that matches nothing is not evidence against anything.* `pool = struck or
candidates`, not `if strike_frames is not None`. The bay is noisy between swings, and a rule that
declined whenever a transient landed near no descent would turn a dropped club into a lost window.

*`_STRIKE_TOLERANCE_S` is 0.20 s and it is loose on purpose.* Sized from §E4's worst pose-impact
error (±0.125 s), §E5's ball-to-screen gap (+0.145 s) and the detector's own +0.018 s onset
convention. The rule does not need to know which of the four transients was the ball — any of them
says a ball was hit here — so the tolerance swallows that ambiguity rather than resolving it, which
is P6's job. Nothing it must reject is close: the decoys are 15–24 s away (M10 §A2). An
**asymmetric** window was rejected in the comment: it is truer to the physics and wrong about the
measurement, since §E4's four offenders have pose impact landing ~0.1 s *after* the audio.

*Selection is deliberately per-view, with no offset between the clips.* Each view is filtered
against the transients in its own footage, in its own frame numbering. There is no cross-clip
arithmetic anywhere in P5 — that is the whole of P6 — and `_pick_swing`'s docstring says so, since
handing one view the other's strike frames would be a silent error rather than a loud one.

*Silence is a note, and never a decline.* An empty `strikes` list is the rehearsal signal, but it
is also what a phone across the bay records, so the window stands and `analysis.json` carries *"no
ball strike was heard in the {view} clip"* (ADR-010 §2 — declining would score every motion in the
clip as one swing, which is strictly worse than scoring the right descent with a caveat).

*The cross-modal check was taken, in one direction only.* When a shot is attached and **no** view
heard anything, a second note says the footage and the shot data may not describe the same swing.
The reverse direction — strikes with no shot — is not noted: an unimported screen photo is the
ordinary state of most bundles and the note would fire constantly. Per-bundle rather than
per-session as the phase suggested, because a bundle is one swing and one shot.

*Two things left undone, on purpose.* `scripts/align_swings.py` keeps its own copy of the
face-on-first ordering (`align_swings.py:300`) and has **no** audio in it, so its windows can now
differ from the pipeline's on a clip where a strike decides; it is not in this phase's file list
and the corpus re-run (P9) goes through `analyze_swing_dir`. And the audio stage runs only under
`options.auto_window`, since selection is its only consumer so far — **P6 will need to hoist it**,
because the alignment wants strikes even when the windows were given by hand.

*One noise cost, recorded rather than fixed.* `audio_for` makes a missing `audio` extra a note (P4's
call, matching `coaching`), so a base install now gains one such note per view on every run — where
the comparable missing-`vision` case is a log line only, because "notes describe the swing". Two
notes per run is a real cost and P9's corpus re-run is what should decide it: if it reads as noise
there, the fix is in `audio_for`, not here.

### [x] P6 — sync: `SYNCHRONIZED`, and impact as a measured anchor

**Files.** `src/golf_coach/contracts/alignment.py`, `src/golf_coach/analysis/alignment.py`,
`src/golf_coach/analysis/engine.py`.
**Tests.** `tests/analysis/test_alignment.py`, `tests/analysis/test_engine_bundle.py`.

**Detail.**

- `contracts/alignment.py`: add `AlignmentQuality.SYNCHRONIZED` and its `_QUALITY_SUMMARY` row. Add
  `impact_measured: bool = False` to `SwingAnchors`, defaulting False so every artifact on disk
  still reads — the same way `ClipAlignment.warp_top` was added as optional.
- `analysis/alignment.py::align_swings` (`alignment.py:268`): when both views carry a measured
  impact, pin `tau = 2` to it in both clips and report `SYNCHRONIZED`. Keep
  `_backswing_disagreement_note` (`:359`) and `_which_half_is_wrong` (`:377`) — P7 reads them.
- Handle the tier inversion named in §Design: a pair with measured impacts must not fall through to
  `IMPACT_ONLY`'s degraded fps path, which exists for the case where impact was *inferred*.

`engine.py:445-450` already extends notes from the alignment, so the new tier reaches the results
page and MCP through `quality_summary` with no consumer change — **verify that rather than assume
it**, since `contracts/alignment.py`'s own docstring records a consumer printing the bare enum at a
viewer for two milestones.

**As built.** Landed as planned, with one deliberate deferral and one consumer change the phase
said to verify rather than assume. `contracts/alignment.py` gains `AlignmentQuality.SYNCHRONIZED`,
its `_QUALITY_SUMMARY` row, an `is_degraded` property and `SwingAnchors.impact_measured`;
`analysis/alignment.py` gains `with_measured_impact` and `_synchronized`; `analysis/engine.py`
gains `_anchored_on_strike` and two `*_strikes` arguments on `analyze_swing_bundle`;
`api/pipeline.py` hoists the `Audio:` stage out of `auto_window` and threads the strikes through.
New sections in `tests/analysis/test_alignment.py`, `tests/analysis/test_engine_bundle.py` and
`tests/api/test_pipeline_auto_window.py`. `pytest` 1166 passed, `ruff` and `mypy` clean over 101
files. Five things are worth reading before P7.

*The earliest candidate wins, and that rule is what §E5's trap actually needs.* The intuitive rules
both fail on this corpus: "take the loudest" takes the impact screen on **every** clip measured,
and "take the nearest" takes it on any clip whose pose impact was already right, which is seven of
eleven. No window separates them either — the correction being made runs to 7.5 frames (§E4) and
the gap to exclude starts at 5 (§E5), so the admitting band is wider than the rejecting one.
Ordering is the only discriminator left, and it is sound for a physical reason rather than a
statistical one: the ball is the first sound a shot makes. The club-and-mat pair 15–20 ms ahead of
it is below `audio/impact.py`'s own 50 ms separation floor and has already been merged into one
onset by the time `with_measured_impact` sees it.

*The window is `phases._STRIKE_TOLERANCE_S`, imported rather than re-derived.* P5 sized it on
exactly the three measurements P6 needs (§E4's ±0.125 s, §E5's +0.145 s, the detector's +0.018 s
convention) and the quantity is the same one read twice: how far a pose impact may sit from the
transient that made it. A second constant 0.05 s away would have been two things to drift with one
justification between them. Two guards ride with it rather than in it — a candidate must land after
the top and inside the clip — and the first is a live branch, not a defensive one: M10's offenders
measure 0.183 s of face-on downswing against a 0.20 s window, so the window opens before the top
all by itself.

*`SYNCHRONIZED` overwrites the anchor count, and the notes are what makes that safe.* `_synchronized`
runs last and replaces whatever the ladder came to, which is the inversion §Design named: a bundle
whose tops were refused now reads `synchronized` where it read `impact_only`, and the tier that
exists to say "one anchor, and it was a guess" no longer sits on the best number in the system.
Nothing is lost by that, because every anchor `align_swings` refuses already appends its own note —
so a synchronized pair whose tops disagree still carries *"downswing durations disagree … one
view's top is wrong"* in `notes`. Half a pair earns a note and no tier.

*The warp underneath is untouched, including `_shared_tops`.* Only the label moved. Measuring
impact does not say which view's *top* is wrong — it makes the question answerable for the first
time, which is P7 — and until it is answered, holding both panels to one duration back from impact
still beats replaying one of them fast (`_DOWNSWING_AGREEMENT`). Note that pinning the
down-the-line impact 5–7 frames later *widens* the downswing disagreement on §E4's four bundles
rather than narrowing it. That is honest and it is the point: the gap was always there and was
being split between two wrong anchors.

*One consumer change was needed, and the phase's guess about which was wrong.* The results page
(`results.html:709`) and `pose/side_by_side.py` do read through `quality_summary` and needed
nothing, as the phase said. But three call sites gate a caveat on `is not FULL`, and they do not
all want the same answer: `engine.py:448` was calling the new tier *"alignment degraded"*, which is
backwards, so it now reads `quality.is_degraded`. `feedback/coach.py:294` and `mcp/query.py:610`
keep `is not FULL` **deliberately** — `ALIGNMENT_CAVEAT` warns that correspondence is exact at the
anchors and interpolated between them, and that is still true of a synchronized pair, which
produces the correct sentence verbatim: *"The two camera views were synchronized on the ball
strike, not on every instant."* The two gates ask different questions and `AlignmentQuality.is_degraded`
carries the distinction in its docstring so the next reader does not "fix" one into the other.

*§E4's ≈22 ms container bias is **not** applied, and that is a deferral rather than an omission.*
The phase asked for it. Applying it needs the *video* track's edit-list offset — §E2's four
down-the-line clips carry a two-entry list and no audio priming where every other track has 2112
samples of it — and nothing in the package can see a container's edit list today: `audio/ffmpeg.py`
lets ffmpeg apply the *audio* one and never reports it, and P0 read the video one by parsing MOV
atoms in a throwaway script. Exposing it is a change to the decode port (P1's file list), not to
three `analysis/` modules. It is worth 1.3 frames against the 5.7–7.5 this phase corrects, so the
anchor is better with it missing than without the phase; **P9's corpus re-run is where a residual
bias of that size will show up**, and if it does, the fix belongs in `FfmpegAudioSource` beside the
stream index it already records.

*Left for P7, by name.* `_backswing_disagreement_note` and `_which_half_is_wrong` are untouched and
still only *name* the suspect boundary. `scripts/align_swings.py` builds its anchors itself and
passes them straight to `align_swings`, so it never reports `synchronized` — the same audio-free
gap P5 recorded at `align_swings.py:300`, now widened from selection to the tier. Not in this
phase's file list, and P9 goes through `analyze_swing_dir`.

### [x] P7 — arbitrate the late top

**Goal.** M10's handoff defect, reachable only now.

**Files.** `src/golf_coach/analysis/alignment.py`, `src/golf_coach/analysis/phases.py`.
**Tests.** `tests/analysis/test_alignment.py`, `tests/analysis/test_phases.py`.

**Detail.** With impact measured in both views on one clock, the two downswing durations are
directly comparable: face-on's 0.183–0.267 s against down-the-line's 0.384–0.484 s stops being "one
view's top is wrong" and becomes a decidable disagreement.

**Which way to decide is already recorded.** `_DRAWDOWN_FLOOR`'s comment (`phases.py:103-132`) has
the ground truth for this failure mode — on 2026-08-09 swing 2 the *late* top was the wrong one, the
two wrists agree with each other down-the-line at 24 and 25 frames, and face-on read 14. So **the
earlier top / longer downswing is the correct pick.**

Extend `_which_half_is_wrong` (`alignment.py:377`) from *naming* the suspect boundary to *resolving*
it when impact is measured. Report the correction in `notes` — never substitute silently, per
ADR-010 §2 and this repo's habit of saying what it did.

**As built.** One file. `analysis/alignment.py` gains `_Arbitration`, `_arbitrate_tops`, `_top_at`
and `_tempo_restated`; `_shared_tops` chooses its reference duration through the arbitration;
`_which_half_is_wrong` gains a branch above the "different swings" fallback. New section in
`tests/analysis/test_alignment.py`, and `test_a_measured_impact_outranks_a_refused_top` lost the
"the warp did not move" half it was holding for this phase. `pytest` 1172 passed, `ruff` and `mypy`
clean over 101 files. **`analysis/phases.py` was in this phase's file list and needed nothing** —
see the last note below.

*The shorter downswing is the late top, and the argument is mechanical rather than statistical.*
`_top_and_impact` puts the top at the start of the major rising run, and the failure
`_DRAWDOWN_FLOOR` documents is that run *fragmenting*: a hover at the top splits the descent and
the later half is taken, which shortens the downswing. Nothing in that rule can move a top the
other way — `_MAJOR_RISE_FRACTION` needs 80% of the largest rise in the clip before a run is a
candidate at all, so a pre-top wobble cannot be mistaken for the descent. The corpus agrees without
a single exception: replayed over all eleven bundles with tau=2 measured in both views, **face-on
is the late view on every one of the seven that disagree** (1, 2, 4, 5, 7, 9, 11), by 10 to 17
frames.

*The change is a reference flip, and the old reference was the wrong view.* `_shared_tops` held
both panels to the **face-on** clip's duration, on ADR-015's grounds that it is the tuned and
scored view. Face-on is also the view whose top is late on all seven — so the pre-P7 rule was
taking the broken duration and imposing it on the good panel. With a shared clock the reference
becomes the *sound* view's, which means the corrected top moves and the sound top stays exactly
where it was detected. Without one, nothing has changed: two inferred impacts cannot settle this,
and the face-on fallback stands.

*`_PLAUSIBLE_DOWNSWING_S` now decides how much of this lands, and it is the wrong ruler for the
job.* The guard refuses a reference that is not a possible downswing, and on the arbitrated route
the reference is a **down-the-line** duration read off the trail wrist — while the band (0.15,
0.45) was swept on **face-on** clips for `select_swing`. Down-the-line reads systematically longer:
across the eleven bundles it returns 0.367–0.484 s, and 0.484 s occurs on bundle 6, where the two
views *agree*. Replayed over the corpus, P7 therefore **diagnoses seven bundles and corrects one**:

| bundle | face-on | sound view | outcome |
|---|---|---|---|
| 2 | 0.217 s | 0.384 s | **corrected** — top moves 10 frames earlier, tempo restated 4.92 → 2.35:1 |
| 5 | 0.183 s | 0.450 s | refused, by 0.0003 s over the ceiling |
| 4 | 0.200 s | 0.484 s | refused |
| 1, 7, 9, 11 | 0.250–0.384 s | 0.484–0.584 s | refused — these are §E4's bundles, where P6 pins down-the-line's impact 6–8 frames later and lengthens its downswing further |

The guard is kept as it is rather than widened here, for two reasons. It is `phases`'
constant, shared with `select_swing`, and re-tuning a swept band as a side effect of an alignment
phase is the move §Design rules out for `_DRAWDOWN_FLOOR`. And on §E4's four the refusal is
*correct*: 0.5–0.58 s is no downswing, so those bundles are saying that pinning the impact later
without re-examining the down-the-line top leaves a residual — which is precisely what P9 is for.
**The open question for P8/P9 is whether the arbitrated route needs its own ceiling, sized on
down-the-line durations rather than face-on ones.** Bundles 4 and 5 are the ones it would buy.

*The three failing `tempo` readings are named, not retired, and that is a deliberate stop.* The
checkpoint is scored inside `analyze_swing` off the face-on phases, long before `align_swings`
runs, so `2026-08-23/4` still ships 6.08:1 as a *fail* — what the notes beside it now add is
that its denominator is 17 frames short. Only the *corrected* route restates the ratio, and only
because a top it has accepted is standing behind the number: bundle 2's note reads *"where its
backswing reads 2.35:1 rather than 4.92:1"*, and a refused reference deliberately quotes nothing,
since the whole content of a refusal is that the duration is not to be trusted. Retiring the
score itself means a new `UnscoredReason` and an engine that re-opens a scored result on a
cross-view finding: a `contracts/` change, which is P8's file list and its `ANALYSIS_VERSION` bump,
not this phase's. §Verification's `get_swing("2026-08-23", "4")` line is **not yet satisfied** and
needs that decision taken.

*`phases.py` needed nothing, and `_shared_motion_starts` was checked and deliberately left.* The
motion-start fallback also derives from the face-on downswing and looks like it wants the same
flip — but it is overwritten by the `IMPACT_ONLY` branch whenever `_shared_tops` returns tops, so
the only time its reference matters is when `_shared_tops` refused. On the arbitrated route that is
exactly when the sound duration was just judged implausible. Changing it would have been a no-op at
best and a propagated error at worst.

### [x] P8 — ADR-025, `ANALYSIS_VERSION`, docs

**Files.** New `docs/decisions/025-acoustic-synchronization.md`; a second addendum on
`docs/decisions/015-...md`; `src/golf_coach/contracts/swing.py` (`ANALYSIS_VERSION` 11 → 12);
`docs/ARCHITECTURE.md` §1–§3; `docs/README.md` (doc map counts + ADR table row);
`ROADMAP.md`; `CLAUDE.md` (§Commands' extras list gains `audio`).

**Detail.** ADR-025 supersedes ADR-015's Option C parking bullet by *taking* the parked option. It
must say which of ADR-015's three objections held and which did not:

- *"assumes the upload path preserves the audio track"* — **did not hold**, measured 30/30 (§E1).
- *"recovers an offset only, so it does not survive a frame-rate difference"* — **holds,
  conditionally.** Two cameras filming one event in real time need no warp; the objection bites only
  for slo-mo or genuinely different rates, and that is checkable per bundle rather than assumed.
- *"the strike is not the only sharp transient"* — **holds**, and §E2 plus P3 are the answer.

ADR-015's addendum #2 records that the "there is no shared clock" premise in its §Context now has a
documented exception.

**`tests/test_docs_truth.py` is the mechanism here, not an obstacle.**
`test_the_documentation_map_counts_the_documents_correctly`,
`test_the_map_counts_the_addenda_correctly`, `test_each_adr_row_states_that_adrs_own_addendum_count`
and `test_the_map_and_each_phase_doc_agree_on_the_phase_count` will each fail until the map is
brought along. That is what they are for.


**As built (2026-08-29).** ADR-025 written and accepted; ADR-015 carries addendum #2 recording that
its "there is no shared clock" premise now has an exception. `ANALYSIS_VERSION` 11 -> 12 with its
history entry. `docs/ARCHITECTURE.md` §1 gained the audio edge in the pipeline diagram and a
paragraph on what it buys, §2 gained `audio/` as an adapter beside `pose/` plus a *Ball strikes*
row in the interface table, and §3 gained a **third** rule: the one finding that reaches back into
a score after `analyze_swing` has returned. `docs/README.md` map counts and the ADR table, and
`ROADMAP.md`. `CLAUDE.md` already listed the `audio` extra (P1). `pytest` 1183 passed, `ruff` and
`mypy` clean over 101 files.

**The phase list said "the paperwork"; the tempo decision P7 deferred came with it, and it is
code.** M10 P10's *As built* says the three `tempo` readings at 4.92, 6.08 and 6.09:1 are not
coaching truth, and P7 left the question of retiring them to this phase's version bump because
retiring one needs a `contracts/` change. It got three, all small:

- `contracts/unscored.py` — `UnscoredReason.CROSS_VIEW_CONTRADICTED`, `refilming_helps=False`. The
  membership is worth stating: unlike the other three False reasons this one *is* about the
  footage, and it belongs there anyway, because the cause is a golfer pausing at the top and a
  second clip of the same swing reproduces it. `contracts/caveats.py` picks it up derived, so the
  coaching prose and the MCP guidance gained it without being edited.
- `contracts/alignment.py` — `ClipAlignment.top_late_by: int | None`, with a `top_is_late` property
  over it. **This is the field the whole phase turns on**, and the reason it is not `warp_top` is
  §E4: `_PLAUSIBLE_DOWNSWING_S` refuses to *correct* a top whose reference runs past 0.45 s, which
  is four of the seven bundles that disagree — including `2026-08-23/4`, the swing §Verification
  names. Reading the correction as the finding would leave exactly the bundles this milestone was
  written for looking sound.
- `contracts/checkpoints.py` — `CONTRADICTED_BY_A_LATE_TOP`, holding `tempo` alone.
  `hip_shift_at_top` reads the top too and is deliberately out: nothing has measured what a
  ten-frame shift does to a hip position sampled there, and the name is not evidence.

`analysis/alignment.py::align_swings` now decides the arbitration once and threads it into
`_shared_tops` rather than letting the two recompute it, so the warp and the flag cannot disagree
about one pair. `analysis/engine.py::_without_contradicted_scores` is the withdrawal: the score
leaves `checkpoint_scores` for `unscored`, `mechanics` and `outcome` are split back apart by
registry membership so `combine` still weighs the two axes (ADR-009), and the entry is sorted into
registry order rather than appended — `tempo` is the *first* checkpoint and an append would put it
behind `head_stays_back`.

**Withdrawn, never restated, and that is the ADR-010 §2 call.** `_tempo_restated` already computes
what the ratio reads on the corrected top — 2.35:1 on bundle 2, a pass. Writing it into a
`CheckpointScore` would ship a number whose value came from the alignment and whose band came from
the engine, over frames `segment_phases` never agreed to; and on the four bundles where the
correction is declined there is no restatement to write at all. The repair belongs in
`phases.py` where the boundary is found. What P8 hands *that* eventual fix is the thing tuning the
detector against its own symptom could never supply: a per-bundle label saying which view was wrong
and by how many frames.

**Two claims checked rather than asserted.** `tests/api/test_pipeline_imports.py` really does hold
`imageio_ffmpeg` (line 62, beside `numpy`), so §2's sentence about the extras boundary is a fact
and not a plan. And the `test_docs_truth.py` failures P8 predicted are exactly the three that
fired — the two map counts and the per-ADR addendum row — which is the mechanism working.

**Left at**: P9, the corpus re-run, and it is now the phase that produces every *after* number this
document still has blank. Two things to carry into it. `scripts/align_swings.py` still builds its
own anchors and passes them straight to `align_swings`, so it can report neither `synchronized` nor
an arbitration — the audio-free gap P5 recorded at `align_swings.py:300`, now the only route into
the alignment that cannot see the clock, and worth a line in P9's write-up rather than a fix inside
it. And §Verification's `get_swing("2026-08-23", "4")` line is now *satisfiable* but not yet
satisfied: the stored artifact still says 6.08:1 until `reanalyze.py` runs.

### [x] P9 — re-run the corpus and record before/after

**Files.** `WORKLOG.md` (new top entry), this document (fill in the *after* column).

**Detail.** Modelled on M10 P10, which is the precedent for this exact run:

```bash
.venv/Scripts/python.exe scripts/reanalyze.py --all --dry-run   # confirm 15 targeted
.venv/Scripts/python.exe scripts/reanalyze.py --all --video     # ~26 min last time
```

Then re-run `2026-08-23/1` with `--coaching`. It is the only bundle carrying a written paragraph,
and `build_feedback` rebuilds the rules half on every run while leaving `coaching_text` to the flag
— without that second run it keeps prose describing the pre-M11 window. M10 P10 records this trap;
it is easy to repeat.

Fill in against P0's baseline: tier spread, the three tempo readings, session 5's window, sessions 2
and 5's collapsed backswings, and the cross-view downswing disagreement on all four bundles.
**Expect scores to move, and say so** — better windows and a corrected top change which frames get
scored, which is the intended effect and the reason for the version bump.

Known noise, not a regression: OpenCV's bundled H.264 encoder fails to load on this machine
(`openh264-1.8.0-win64.dll`, wrong version) and prints a `VideoWriter` failure per bundle before
falling back. M10 P10 records the same.

**As built.** Ran 2026-08-29. `reanalyze.py --all --video` re-analysed and re-rendered 15/15 and
exited 0; a plain `--dry-run` afterwards reports every stored result current, `career_corpus.py` is
still 13 distinct swings over 21 metrics with nothing excluded as `OUTDATED`, and `pytest` is 1183
passed with `ruff` and `mypy` clean over 101 files before and after. The eleven 2026-08-23 bundles,
against P0's baseline:

| bundle | tier | `tempo` | mechanics | face-on / DTL downswing |
|---|---|---|---|---|
| 1 | `top_impact` → `synchronized` | 2.61 ✗ → **withdrawn**, top 11 frames late | 98.80 → 99.68 | 0.384 / 0.567 s |
| 2 | `impact_only` → `synchronized` | 4.92 ✗ → **withdrawn**, top 10 late | 95.68 → 96.95 | 0.200 / 0.367 s |
| 3 | `top_impact` → `synchronized` | 2.57 ✗ (unmoved) | 98.70 → 98.70 | 0.367 / 0.467 s |
| 4 | `impact_only` → `synchronized` | 6.08 ✗ → **withdrawn**, top 19 late | 88.50 → **100.00** | 0.150 / 0.467 s |
| 5 | `impact_only` → `synchronized` | 6.09 ✗ → **withdrawn**, top 17 late | 83.47 → 94.04 | 0.167 / 0.450 s |
| 6 | `top_impact` → `synchronized` | 2.36 ✗ (unmoved) | 96.98 → 96.98 | 0.400 / 0.417 s |
| 7 | `impact_only` → `synchronized` | 4.27 ✓ → **withdrawn**, top 10 late | 100.00 → 100.00 | 0.233 / 0.400 s |
| 8 | `full` → `synchronized` | 1.89 ✗ (unmoved) | 93.07 → 93.07 | 0.467 / 0.350 s |
| 9 | `impact_only` → `synchronized` | 4.06 ✓ → **withdrawn**, top 14 late | 100.00 → 100.00 | 0.217 / 0.450 s |
| 10 | `full` → `synchronized` | 2.50 ✗ (unmoved) | 98.16 → 98.16 | 0.350 / 0.334 s |
| 11 | `full` → `synchronized` | 2.50 ✗ → **withdrawn**, top 8 late | 98.16 → **100.00** | 0.283 / 0.417 s |

*Every clip on disk heard its own strike: 30/30, and 15/15 bundles read `synchronized`.* No bundle
sits at any other tier, so the inversion §Design warned about is not a corner case to handle — it is
the whole corpus, and the ladder that ranks inferred anchors is now what a pair falls back to rather
than what it normally reports. The four older bundles (`2026-08-07-aaron1/1`, `2026-08-09/2`,
`2026-08-10/1`, `2026-08-10/2`) synchronize too, on footage shot weeks before this was designed.

*The late top is on **seven** bundles, not three, and finding the extra four is the milestone
working rather than a surprise.* M10 handed over 2, 4 and 5; 1, 7, 9 and 11 joined them the moment a
measured impact was underneath. P6's *As built* predicted exactly this — pinning the down-the-line
impact later **widens** the downswing gap rather than narrowing it — and the two withdrawals that
prove the point are 7 and 9, which were `tempo` **passes** at 4.27 and 4.06:1. §E3 said in advance
that a pass here was not a result to preserve: both were passing on the same short face-on downswing
that made 2, 4 and 5 fail, and both are now withdrawn on a clock rather than on a suspicion.

*Five scores moved, all upward, and not one of them is a swing that got better.* `2026-08-23/4`
reads 100.00 where it read 88.50 because a 6.08:1 `tempo` scored against a denominator the other
view contradicts has been withdrawn — the swing is unchanged and is now judged on five fundamentals
rather than six, which `unscored` says and the coaching prose repeats. Read that column as *"a wrong
score left"*, never as improvement. The other six bundles keep `overall_score` to the digit.

*Session 5 got its down-the-line window and its backswing back; session 2 did not.* M10's last
windowless view is closed: `2026-08-23/5` reads `(1400, 1652)` where it read `null`, and its
down-the-line backswing is 0.717 s rather than the 1 frame §E3 recorded — the corrected impact gave
`select_matching_swing` a reference it could match. `2026-08-23/2`'s down-the-line backswing is
**still 1 frame** (0.017 s), and it is the one M10 symptom this milestone does not touch: what that
view has wrong there is its motion start, not its top.

*The note that was false is gone corpus-wide.* §E3 recorded that bundles 4, 7 and 9 each claimed
*"the two clips are showing DIFFERENT swings; check for a practice swing"*, and that P0 had
disproved it. Every one of them now reads *"both views heard the strike, so tau=2 is one instant in
real time and this is one swing filmed twice, not two"*, followed by which top is late and by how
many frames. That sentence is the milestone's thesis, written into the artifact.

*The residual is real, it is not the container bias P6 deferred, and it is measurable.* P6 said P9
was where a bias of that size would show. What showed is a different mechanism: on the §E4 bundles
the down-the-line clip carries a **low-confidence precursor onset 2–3 frames ahead of the ball**,
and `with_measured_impact`'s earliest-candidate rule takes it.

| bundle | DTL: pose → chosen (conf) | strong onset in the same window | §E4 predicted | cross-view residual |
|---|---|---|---|---|
| 1 | 373 → 379 (0.90) | — (379 *is* it) | 7.2 frames | 1.2 frames |
| 7 | 670 → 672 (**0.34**) | 676 (0.90) | 6.9 | **3.9** |
| 9 | 322 → 324 (**0.44**) | 327 (0.96) | 5.7 | 0.7 |
| 11 | 340 → 343 (**0.66**) | 346 (0.95) | 7.5 | 1.5 |

The strong onsets are the ball on independent evidence: each pairs with a second loud transient 5–9
frames later, which is §E5's measured ball→screen gap, and on `2026-08-23/9` those two carry
prominence 19.2M and 17.5M against 2.5M for everything between them. (**Confirmed against the video
on 2026-08-30 — and the floor this table specifies still must not ship alone**, because on these
same clips the precursor error was cancelling a 90 ms container offset. §Addendum.) The same rule
fires face-on
(bundles 4, 9, 10 and 11 each chose a candidate 3 frames early), which is why the *cross-view*
residual is smaller than the per-clip error — the two views cancel most of it. Frame accuracy
therefore holds to ≈1.5 frames on three of the four and misses by 3.9 on bundle 7. **This is not the
ordering rule being wrong**: §E5's screen strike is louder than the ball on every clip measured, so
"take the most confident" is a worse rule, not a better one. What the corpus supplies that P6 could
not is that a precursor exists at all, that it sits 2–3 frames out rather than the 15–20 ms §E5
measured for club–mat, and that prominence separates it by an order of magnitude. A candidate floor
therefore belongs in `audio/impact.py`, where prominence already lives, and not in `analysis/`.

*The coaching path was re-run and disclosed the withdrawal without being asked to.* `2026-08-23/1`
regenerated 1221 characters from `claude-opus-5`, and the paragraph ends: *"tempo couldn't be scored
here because the two cameras disagree about when your backswing ended … so this score comes off five
fundamentals, not six, and there's nothing to re-film."* Nothing wrote that sentence —
`CROSS_VIEW_CONTRADICTED` carries `refilming_helps=False` and `contracts/caveats.py` derives the
rest. Via MCP, `get_swing("2026-08-23", "4")` reports `alignment_quality: synchronized`, five
checkpoints, and `tempo` in `unscored` with `refilming_helps: false` — the line §Verification asks
for.

*Career mode did not move, and that is measuring and judging being separate.* Seven withdrawn
`tempo` **scores** cost zero `tempo_ratio` **measurements**: `career_corpus.py` still reports
`n = 13` for it. A withdrawal is a refusal to judge, not a refusal to measure.

*Two things that look like P9 findings and are not.* The `openh264` `VideoWriter` noise above
appeared once per bundle and all 15 renders read back fine. And `--all` always lists every swing as
*"re-run by request"* — the staleness check that means anything is a plain `--dry-run`, which
reports nothing to do.

*Left undone, and it wants eyes rather than code.* §Verification's "watch two renders by eye"
(`2026-08-23/9` and `/10`) has not been done. Both were re-rendered on 2026-08-29 and the claim they
now carry is the new half of it: that the two panels strike the ball on the same output frame.
**Done on 2026-08-30, and it failed on `9`** — its two panels are about four output frames apart,
where `/10`'s agree to within one. That is what §Addendum is.

---

### [x] P10 — make a frame index and a sample index mean the same time

**Goal.** Stop comparing two clocks as though they were one. On four down-the-line clips the audio
decode honours a leading empty edit and the frame counter does not, so every strike frame derived
on them was 6.3–7.5 frames late (§Addendum).

**Files.** `src/golf_coach/audio/ffmpeg.py` (new `video_start_seconds`);
`src/golf_coach/contracts/audio.py` (`AudioClipMetadata.video_start_s`, `AudioStrike.frame`'s second
`None` case, `AUDIO_DETECTOR_VERSION` 1 → 2); `src/golf_coach/api/pipeline.py` (`audio_for` probes,
`_frames_derived` subtracts, new `_frame_of`); `src/golf_coach/contracts/swing.py`
(`ANALYSIS_VERSION` 12 → 13). **Tests.** `tests/audio/test_ffmpeg_source.py` (4 new),
`tests/api/test_pipeline_audio.py` (4 new, and the `decoder` fixture gains the probe).

**Detail.** The addendum offered two routes and this took the cheaper one: measure the offset once
per clip and carry it on the artifact, rather than teach the video decode to honour the edit list.
Two reasons it is also the *better* one here. The pose path decodes with OpenCV, which shows the
offset in no property it exposes — `CAP_PROP_POS_MSEC` reads 0.0 on the first frame of a clip whose
video presents 105 ms late — so there is no decoder-side switch to flip. And the offset is less a
fact about the file than a fact about what a decoder *did* with the file, which is worth storing
beside the sample indices it corrects rather than re-deriving at each read.

It is deliberately **not** on `keypoints.ClipMetadata`, which is where the addendum guessed it would
go. That artifact is written by the vision path, which cannot see the number without dragging
`imageio-ffmpeg` into the pose extras; the audio path already has ffmpeg open on the same file, and
the only question the offset answers — which frame a sample index lands on — is the one the audio
artifact exists to answer.

**As built.** `video_start_seconds` asks ffmpeg for the presentation timestamp of the first frame it
hands back (`showinfo`, one frame, `-fps_mode passthrough`). Passthrough is load-bearing: ffmpeg's
default output mode *pads* the empty edit with duplicates of the first frame — seven copies of it on
`2026-08-23/9` — and a timestamp read off that padding is 0.0 and useless.

*Measured over all 30 clips on disk, 2026-08-30.* Twenty-six report 0.0. The four that do not are
exactly §E2's four, and they do not share a number:

| clip | video edit list | `video_start_seconds` | frames at 60 fps |
|---|---|---|---|
| `2026-08-23/1` down-the-line | `[(107, -1), (8757, 1632)]` | 0.107 s | 6.4 |
| `2026-08-23/7` down-the-line | `[(117, -1), (13427, 1632)]` | 0.117 s | 7.0 |
| `2026-08-23/9` down-the-line | `[(105, -1), (7555, 1632)]` | 0.105 s | 6.3 |
| `2026-08-23/11` down-the-line | `[(125, -1), (7155, 1632)]` | 0.125 s | 7.5 |
| every other clip | one entry, `media_time` 0 or 2112 | 0.0 s | 0 |

*The empty edit is the whole offset, and §Addendum's reading of the container was wrong about how.*
The addendum added the empty edit to the 85 ms `media_time` and got 90.5 ms; the decoders do
something simpler. **Neither drops the trimmed head at all**: with `-ignore_editlist 1` ffmpeg's
frames 0, 1, 2 are pixel-identical to `cv2.VideoCapture`'s frames 0, 1, 2 of the same clip, so both
hand back media sample 0 as frame 0 and `media_time` never reaches a frame index. What does reach it
is the empty edit, applied as a **uniform shift of the whole presentation timeline**: on bundle 9,
frame 0 is presented at 0.105 s and the 449 steps after it are 445 of 16.67 ms, three of 18.33 ms —
the container's own periodic long sample, visible in its `stts` — and one 66.67 ms jump at the very
last frame. So the correction is a constant, it is the empty edit's duration, and it is 105 ms
rather than 20.

*This also closes §E4's loose end.* Those four clips decode three fewer frames than their `stts`
counts (527/524, 806/803, 453/450, 431/428) and P0 left the sign untested. The three are at the
**tail** — the presentation window ends before the media does — which is why nothing was ever wrong
with a frame index near the strike, and why the missing frames cost nothing.

*Both decoders agree about which frame is frame 0, verified frame by frame.* That is the assumption
the phase rests on: the number is measured through ffmpeg and applied to indices produced by
OpenCV. On bundle 9's down-the-line clip the diagonal is the minimum and every off-diagonal is
clearly worse, for cv2 against ffmpeg passthrough and for cv2 against the edit-list-ignoring decode
alike, and both decoders return 450 frames.

*A strike before the first decoded frame now has no frame at all.* `_frame_of` floors a value that
can be negative, where `int()` truncated toward zero and would have called it frame 0. It is not a
hypothetical on a clip whose video presents 125 ms late; it is honest, and `_auto_windows` already
drops strikes with no frame.

*The probe is a note, not a failure.* An unreadable timebase leaves `video_start_s` None, keeps the
strikes — they are still the measurement — and says in the notes that the frames were derived
assuming the two tracks agree. None is distinguishable from a measured 0.0 on purpose.

### [x] P11 — land the candidate floor P9 specified

**Goal.** Stop the earliest-wins rule anchoring on a transient that is not the ball.

**Files.** `src/golf_coach/audio/impact.py` (`_MIN_RELATIVE_PROMINENCE`, a second floor in
`detect_strikes`, and its docstring); `src/golf_coach/analysis/alignment.py`
(`with_measured_impact`'s docstring, which recorded the defect as unfixed). **Tests.**
`tests/audio/test_impact.py` (3 new).

**Detail.** Exactly what P9 specified and measured: drop any candidate under 0.25 of the loudest
transient in the same clip. Precursors run 0.02–0.10 of the clip maximum and the ball never falls
below 0.61, so the threshold sits 2.5x above the loudest thing it removes and 2.4x below the
quietest thing it keeps, with nothing measured in the gap between those two populations.

It ships **with P10 and could not ship without it.** On the four edit-list clips the precursor error
ran ~3 frames early against a container offset of ~6 frames late, and the two partly cancelled;
removing the precursor alone would have left the anchor right in audio time and 6 frames late in
video time, which is the only time tau=2 is measured in. That is why P9 built this, measured it, and
reverted it the same day.

**As built.** Landed as specified. Two details worth recording.

*It is not a listing floor at heart.* `_MIN_PROMINENCE_Z` still decides what counts as an onset;
everything this removes was already known to be one. What it removes is the chance to be *chosen* —
and because the iteration is by descending prominence, it is a `break` rather than a `continue`,
resting on the same ordering the z floor already rests on.

*Confidence would not have caught it.* The synthetic precursor the tests pin reads `confidence`
0.83, because it is a perfectly real onset that simply is not the ball. That is the argument for a
floor on prominence *relative to this clip's loudest* rather than on the [0, 1] number that looks
like it should mean this.

**The re-run, 2026-08-30.** `reanalyze.py --all --video` re-analysed and re-rendered 15/15 and
exited 0; a plain `--dry-run` afterwards reported every stored result current. All fifteen still
read `synchronized`, and **every window is the one version 12 picked** — the corrected strike frames
moved no `select_swing` verdict, which is the quiet half of the result.

| bundle | anchors face-on / down-the-line | `top_late_by` | `tempo` | score |
|---|---|---|---|---|
| 1 | — / **379 → 372** | 11 → **none** | withdrawn → **2.61 ✗ scored** | 99.68 → **98.80** |
| 4 | **754 → 757** / — | 19 → 16 | withdrawn (unmoved) | — |
| 6 | — / **1161 → 1164** | — | — | — |
| 7 | — / **672 → 669** | 10 → 7 | withdrawn (unmoved) | — |
| 8 | — / **3993 → 3996** | — | — | — |
| 9 | **547 → 550** / **324 → 321** | 14 → 8 | withdrawn (unmoved) | — |
| 10 | **770 → 773** / **1239 → 1242** | — | — | — |
| 11 | **222 → 225** / **343 → 339** | 8 → **none** | withdrawn → **2.50 ✗ scored** | 100.00 → **98.16** |
| 2, 3, 5, and the four older bundles | unchanged | — | — | — |

*Every anchor that moved, moved toward contact, and the two fixes are legible in the sizes.* Four
face-on anchors moved **+3 frames** and two down-the-line ones did (6 and 8): that is P11 alone, a
precursor dropped on a container whose tracks already agreed. The four edit-list clips moved the
other way — `7`, `9` and `11` by −3, −3 and −4, which is P11's +3 against P10's −7.0, −6.3 and −7.5,
and `1` by the full **−7**, because its earliest candidate was already the ball and only its clock
was wrong. Seven bundles did not move at all.

*Two `tempo` readings came back from `unscored`, and both fail.* This is P9's "a wrong score left"
read backwards, and it is the most interesting thing the re-run says. Bundles 1 and 11 had their
`tempo` **withdrawn** on 2026-08-29 because the two views contradicted each other about the top —
and on both, the contradiction was the mis-registered anchor rather than the swing. With the
down-the-line impact corrected, bundle 1's two downswings read 0.384 s against 0.450 s where they
had read 0.384 against 0.567 (a 15% gap, inside `_DOWNSWING_AGREEMENT`), and bundle 11's read
0.334 against 0.350. Nothing contradicts the top any more, so the face-on `tempo` is scored — 2.61
and 2.50:1 against a 2.72 floor, both **failures**. The two mechanics scores fall because a failing
score *returned*, not because a swing got worse; read that column the same careful way P9's asks to
be read, with the sign flipped. The other three withdrawals (4, 7, 9) narrowed and stayed: their
views still disagree by more than the threshold, on an anchor that is now right.

*What moved in `measurements`, and nothing else did.* `tour_trajectory_t2_dtl` and
`tour_trajectory_q_dtl` on the six bundles whose down-the-line impact moved — the two quantities
resampled onto that anchor, exactly as `ANALYSIS_VERSION`'s note predicts. No other measurement,
and no mechanics checkpoint outside the two returning `tempo` readings, changed by a digit.

*The renders were watched, which is where this started.* `2026-08-23/9` — the bundle whose panels
struck about four output frames apart on 2026-08-30 — now strikes on **one** output frame: number
142, showing face-on 550 beside down-the-line 321, with the ball on the mat at 141 in both panels
and gone by 143. `2026-08-23/10`, the control, does the same at output frame 108. Every bundle's two
panels reach their own measured impact on the same output frame, which `pair_frames` guarantees by
construction — what is new is that the frame each panel calls impact is the frame the ball leaves.

*Measured before the pose variant moved.* This re-run is `mediapipe:lite` at `ANALYSIS_VERSION` 13,
so the numbers above isolate P10 and P11 from the lite → heavy switch that landed the same day
(ADR-002's third addendum, `ANALYSIS_VERSION` 14). They will not survive that re-run, and they are
not meant to: what they pin is the size and direction of this fix.

## Addendum — the eye check, and the video edit list under it (2026-08-30)

**§Verification's last line was run and it failed**, which is the best thing that happened to this
milestone. `2026-08-23/9`'s two panels strike the ball about four output frames apart; `/10`'s are
within a frame of each other. Chasing the difference found a defect that is **not** in the strike
detector at all, and it explains §E4's numbers better than §E4 did.

### The measurement: contact, by eye, against the anchor the audio produced

Contact is unambiguous at 4K — the club head reaches the ball on one frame, the ball is a vertical
blur off the mat on the next. Six clips, read frame by frame:

| clip | contact, by eye | ball in the audio | gap |
|---|---|---|---|
| `9` face-on | 550–551 | 550.3 | ~0 |
| `9` down-the-line | **321–322** | **327.7** | **+6.2 frames (103 ms)** |
| `10` face-on | 773–774 | 773.4 | ~0 |
| `10` down-the-line | 1242–1243 | 1242.9 | ~0 |
| `7` face-on | 1107–1108 | 1106.5 | ~−1 |
| `7` down-the-line | **669–670** | **676.3** | **+6.8 frames (113 ms)** |

Five of six clips put the ball where the video does. Two down-the-line clips are out by ~110 ms, in
the same direction and by nearly the same amount.

### It is the *video* edit list, and §E2 wrote it down before anyone needed it

`offset_between` settles which half is wrong without naming a transient. On `2026-08-23/10` the
audio offset between the two clips is −7.8350 s and the offset implied by the two verified contacts
is −7.8257 s — agreement to 0.6 frames, so both containers put audio and video on one timeline. On
`2026-08-23/9` the audio offset is +3.7100 s against a video-derived +3.8169 s: **107 ms of
disagreement inside one bundle.**

§E2 already holds the cause. Four down-the-line clips — bundles **1, 7, 9 and 11**, the same four
§E4 flagged — carry a video track at timescale 19200 whose edit list is `[(105, -1), (7555, 1632)]`:
an empty edit of 5.5 ms, then a start 1632 ticks (**85 ms**) into the media. The audio path decodes
through ffmpeg, which *applies* the edit list. The video path reads frames from media time zero and
ignores it. So on exactly those clips a frame index and a sample index describe timelines
**90.5 ms apart**, and adding `detect_strikes`'s +18 ms window convention predicts 108.5 ms against
the 103 and 113 measured. §E2's own conclusion — "the decode path must let a real demuxer apply the
edit list rather than reading samples from zero" — was applied to audio and never to video.

### What this says about P9's residual, which is not what P9 said

**P9 identified the right peak.** The loud onset *is* the ball; the quiet one 2–3 frames ahead of it
is a precursor, exactly as P9 read it. The candidate floor P9 specified was built, measured over all
30 clips (it wants 0.25 of the clip maximum: precursors run 0.02–0.10 and the ball never falls below
0.61) and it selects the ball on every clip in the corpus.

**And it must not ship on its own, which is why it is not here.** On the four edit-list clips the
precursor error ran ~3 frames *early* against a container offset of ~6 frames *late*, so the two
partly cancelled and the stored anchor came out ~2.5 frames late. Take the precursor away and the
cancellation goes with it: the anchor becomes right in audio time and lands **6 frames late in video
time**, which is the only time that matters — tau=2 is a video frame. Measured on the corpus: `7`
down-the-line 672 → 676 against a contact at 669–670, `9` 324 → 327 against 321–322. The rendered
panels get *further* apart. So the floor is reverted, `detect_strikes` carries a comment saying why,
and the two fixes belong in one change: **correct the video edit list first, then floor the
candidates.**

**The face-on anchors are the ones the floor would improve, and they are unfixed today.** Their
containers are uniform (§E2: timescale 600, no video edit), so nothing cancels there and the
precursor is simply an error: `9` anchors 547 against a contact at 550–551, `10` anchors 770 against
773–774. Three frames early, on four bundles.

**§E4's headline is downgraded.** "The down-the-line pose impact is 5.7–7.5 frames early" measured
the gap between a pose estimate and an anchor whose clip was 90 ms out of register. Against the
video the pose estimate is right to within a frame on the two edit-list clips checked (322 against
321–322, 670 against 669–670). The late-top arbitration inherits this: on bundles 1, 7, 9 and 11 it
compares the two views through an anchor that is ~6 frames late on one side.

### What is untouched

ADR-025's claim — both phones record the strike — and `offset_between`, which never identified a
transient and is the thing that *proved* the container offset. `SYNCHRONIZED` still means both views
heard the shot. What is not safe to read as ±1 frame is the down-the-line tau=2 on bundles 1, 7, 9
and 11, and every cross-view comparison drawn through it.

### What picked this up — P10 and P11, the same day

All four items below were done in one change, because the first two are not separable:

1. **The offset is measured, not derived** — `audio/ffmpeg.py`'s `video_start_seconds`, carried on
   `AudioClipMetadata.video_start_s` and subtracted in `_frames_derived`. It went on the *audio*
   artifact rather than on `ClipMetadata` (P10 says why). `ANALYSIS_VERSION` 12 → 13.
2. **P9's floor landed** on top of it (P11), gated by `AUDIO_DETECTOR_VERSION` 1 → 2 exactly as this
   list intended.
3. **The ground truth was taken first.** `2026-08-23/1` and `/11` were read frame by frame in both
   views, which makes ten clips checked rather than six — the table in P10's *As built*.
4. **One reading here was wrong and P10 corrects it.** The offset is the empty edit alone (105 ms on
   bundle 9), not the empty edit plus the 85 ms `media_time`: the decoders never drop the trimmed
   head, so `media_time` reaches no frame index. The arithmetic below that adds the two and lands
   near the measurement does so by coincidence.

## Verification, end to end

Beyond the per-phase suite:

- `pytest` green; `ruff check src tests scripts` and `mypy src` clean.
- **`tests/api/test_pipeline_imports.py` green throughout** — the fastest signal that the extras
  boundary survived. It must now hold `imageio_ffmpeg` as well as `fastapi` and `anthropic`.
- A base install with **no extras** still imports `golf_coach.analysis.*` and passes the analysis
  tests. This is the ADR-008 invariant this milestone is most likely to break.
- `scripts/reanalyze.py --all --dry-run` reports every stored result current after P9.
- `scripts/career_corpus.py` still counts 13 distinct swings over 21 metrics with nothing excluded
  as `OUTDATED`.
- **Watch two renders by eye** — `2026-08-23/9` and `2026-08-23/10`, the two M10 P10 nominated —
  and confirm the panels leave address together *and* strike the ball on the same output frame. The
  second half is new and is what this milestone actually claims. **Run 2026-08-30 and it failed**,
  which is where §Addendum, P10 and P11 come from; re-run after them and both bundles pass —
  `9` strikes on output frame 142 in both panels and `10` on 108.
- Via MCP: `get_swing("2026-08-23", "4")` reports a `tempo` that either passes or is honestly
  unscored — not a 6.08:1 failure resting on a denominator the other view contradicts.

## Risks

- **The strike may not be separable from the mat.** Club–ball and club–turf are milliseconds apart.
  P3's synthetic double-transient test is the early warning; P0's spike output should show real
  envelopes before P5 depends on them.
- **The simulator's own speakers.** If HD Golf's ball-flight audio rivals the strike, absolute
  detection (P5) degrades while cross-correlation (P6) does not. That is why they are separate
  phases and why P6 does not depend on P5.
- **`2026-08-23/9`'s container** (timescale 19200, empty edit, no audio priming offset) is the most
  likely single clip to expose a timebase bug. Use it as a test case, not an afterthought.
- **Sound travels.** Two phones at different distances from the ball hear it at different times —
  ~34 cm per millisecond, so a 3 m difference is ~9 ms, comfortably inside one 60 fps frame but not
  inside the sub-frame precision §E1 advertises. Frame-accurate sync is safe; do not claim better
  without measuring it.
