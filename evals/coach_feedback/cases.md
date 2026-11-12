# Coach-feedback eval cases

<!-- Written by `python -m golf_coach.evals.coach_cases` from cases.json, through golf-mcp. tests/evals/test_coach_cases.py fails when this file, the sessions and cases.json disagree. Edit cases.json, then run that command. -->

What the coach-feedback eval asks a model to coach from (M43, [ADR-036](../../docs/decisions/036-llm-evaluation-and-observability.md)). Every swing is synthetic: pose keypoints from `make_swing`, a shot's printed values, analysed by the frozen engine and stored under `sessions/`. Each brief below is the one `golf-mcp` serves for that swing, which is the user message the model is sent. The system prompt is `coaching_system_prompt` over `golf-mcp`'s caveats, the same for every case, and is not repeated here.

16 cases.

| Case | Tags | Swing | Club | About |
|---|---|---|---|---|
| `clean-swing` | clean | 2026-07-01/1 | 7i | Everything measured and nothing degraded: two views aligned in full, a clean shot, and two checkpoints missed, so there is something to coach and no caveat beyond the standing ones. |
| `unscored-finish` | unscored | 2026-07-01/2 | 8i | The clip ends a frame after impact, so finish balance is unscored for too few frames: a capture problem worth filming again, and a checkpoint left out of overall_score rather than counted as zero. With one metric missing there is no placement either. |
| `uncalibrated-placement` | uncalibrated-placement | 2026-07-01/3 | 6i | The face-on trajectory placements kept: a calibrated T2 and the uncalibrated Q beside it, both far out, and no shot attached. Large means unusual, never wrong, and Q is never read alone. |
| `interim-pivot` | interim-pivot | 2026-07-01/4 | 9i | The face-on rotation rows kept: interim 2-D readings with no reference population. make_swing's shoulders never move, so four of the five read zero, which is no turn measured and not a turn that did not happen. |
| `flagged-shot` | flagged-shot | 2026-07-01/5 | 7i | The screen photo misread: a smash factor of 1.87 that does not divide out and is past what is possible, and a tile with no value under its label. The validator flags the shot for review, so its numbers are not to be coached from as fact. |
| `partial-alignment` | partial-alignment | 2026-07-01/6 | 5i | The down-the-line clip pauses at the top, so its motion start is estimated and the two views align on top and impact only. Nothing is scored from that view, and the side-by-side replay is not to be trusted at every instant. |
| `no-handedness` | no-handedness | 2026-07-01/7 | pw | Nobody said who swung, so head stays back, the one signed checkpoint, is unscored for want of a handedness. Not a capture problem: filming again would not change it. |
| `nothing-to-fix` | nothing-to-fix | 2026-07-01/8 | 7i | Every checkpoint scored and inside its band, two views aligned in full and a clean shot. There is nothing to fix, and a coach that invents a fault is wrong. |
| `clean-left-driver` | clean | 2026-07-01/9 | driver | A left-hander's driver, face-on only, with a clean shot and nothing degraded. The head drifts too far toward the target, so there is something to coach, and every drift is mirrored, which is what a left-hander's swing looks like from the same camera. |
| `unscored-no-loft` | unscored | 2026-07-01/10 | 4h | The screen printed no spin, so the simulated flight infers it from the carry, and two spins fly that carry with no loft on record for the hybrid to choose between them: the flight's carry is refused. Every mechanics checkpoint is scored. The refusal is for want of a loft, not a capture problem, so a better clip or photo would not change it. |
| `uncalibrated-placement-dtl` | uncalibrated-placement | 2026-07-01/11 | 3w | The down-the-line trajectory placements kept: a calibrated T2 and the uncalibrated Q from the second view, beside the face-on joint placement, with a clean shot. Large means unusual, never wrong, and Q is never read alone. |
| `interim-pivot-dtl` | interim-pivot | 2026-07-01/12 | gw | A left-hander's gap wedge with the down-the-line rotation rows kept: interim 2-D readings from the second view, with no reference population and no band, never scored and never blended with the face-on view. make_swing's shoulders never move, so the three shoulder rows read zero, which is no turn measured and not a turn that did not happen. The hips barely travel toward the target, which misses a two-sided checkpoint on its low side: less is not better there. |
| `flagged-shot-distance` | flagged-shot | 2026-07-01/13 | 6i | The screen photo misread the total: carry and roll add to 175 and the total reads 135, so one of the three is wrong. The validator flags the shot for review, so its distances are not to be coached from as fact. |
| `partial-alignment-tempo` | partial-alignment | 2026-07-01/14 | sw | The two clips disagree on tempo, so they may be two different swings, and they align on top and impact only. Nothing is scored from the down-the-line view, and the side-by-side replay is not to be trusted at every instant. |
| `no-handedness-driver` | no-handedness | 2026-07-01/15 | driver | A driver with two views and nobody named as the golfer, so head stays back is unscored for want of a handedness, as is the simulated landing's offline distance. The backswing is slow, so there is something to coach beside it. Not a capture problem: filming again would not change it. |
| `nothing-to-fix-left` | nothing-to-fix | 2026-07-01/16 | 9i | A left-hander, face-on only: every checkpoint scored and inside its band, and a clean shot. There is nothing to fix, and a coach that invents a fault, or reads the mirrored drifts as one, is wrong. |

## `clean-swing`

Tags: clean. Swing 2026-07-01/1, club 7i.

Everything measured and nothing degraded: two views aligned in full, a clean shot, and two checkpoints missed, so there is something to coach and no caveat beyond the standing ones.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 40,
    "downswing_frames": 12,
    "head_sway": 0.02,
    "hip_sway": 0.06,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 40,
    "downswing_frames": 12,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 82.0,
      "ball_speed": 112.0,
      "smash_factor": 1.37,
      "launch_angle": 18.4,
      "launch_direction": 1.1,
      "spin_rate": 6400.0,
      "spin_axis": 3.2,
      "carry_distance": 151.0,
      "bounce_and_roll": 7.0,
      "total_distance": 158.0,
      "club_path": -1.5,
      "club_face_angle": 0.8,
      "shot_type": "SLIGHT FADE",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.97,
    "read_warnings": []
  },
  "loft_deg": 34.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 1 (session 2026-07-01)

overall_score: 94 out of 100
mechanics_score: 94 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.67, tour band 2.72 to 4.71, MISS, score 0.97
  percentile 10 of 1399 tour swings (both extremes are faults)
  rule-based reading: Tempo too quick - 2.7:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 400. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
- head_sway: observed 0.13, tour band 0 to 0.43, PASS, score 1.00
  percentile 38 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.38, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 70 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.38 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 70% of 458 tour swings.
- hip_shift_at_top: observed 0.28, tour band 0 to 0.21, MISS, score 0.67
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.28 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.25, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 78 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.25 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 78% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.93 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 86.3% of them. Largest contributor: hip_shift_at_top_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.28 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.28 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [minor] tempo: Tempo too quick - 2.7:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 400. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
3. [info] head_stays_back: Head stayed behind the ball - 0.25 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 78% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.38 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 70% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.97)
carry: 151 yards
total: 158 yards
ball speed: 112 mph
club head speed: 82 mph
smash factor: 1.37
club path: -1.5 degrees, + = in-to-out
club face angle: 0.8 degrees, + = open
launch angle: 18.4 degrees
spin rate: 6400 rpm
shape: SLIGHT FADE
strike: CENTER

TWO-VIEW ALIGNMENT
quality: full

NOTES RECORDED WHILE ANALYZING THIS SWING
none
```

## `unscored-finish`

Tags: unscored. Swing 2026-07-01/2, club 8i.

The clip ends a frame after impact, so finish balance is unscored for too few frames: a capture problem worth filming again, and a checkpoint left out of overall_score rather than counted as zero. With one metric missing there is no placement either.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 44,
    "downswing_frames": 12,
    "head_sway": 0.01,
    "hip_sway": 0.04,
    "followthrough_frames": 1
  },
  "down_the_line": null,
  "shot": {
    "printed": {
      "club_head_speed": 80.0,
      "ball_speed": 106.0,
      "smash_factor": 1.33,
      "launch_angle": 20.1,
      "launch_direction": -0.6,
      "spin_rate": 7200.0,
      "spin_axis": -2.0,
      "carry_distance": 140.0,
      "bounce_and_roll": 6.0,
      "total_distance": 146.0,
      "club_path": 1.8,
      "club_face_angle": -0.4,
      "shot_type": "SLIGHT DRAW",
      "impact_position": "TOE"
    },
    "read_confidence": 0.96,
    "read_warnings": []
  },
  "loft_deg": 38.0,
  "population": [],
  "rotation": []
}
```

The brief:

```text
SWING 2 (session 2026-07-01)

overall_score: 100 out of 100
mechanics_score: 100 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 4, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 72 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 4.0:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 110 ms; at the tour ratio that wants a 299-518 ms backswing, and yours was 440. You swing with a slower tempo than 72% of 1399 tour swings.
- head_sway: observed 0.06, tour band 0 to 0.43, PASS, score 1.00
  percentile 19 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
- hip_sway: observed 0.24, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 33 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.24 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 67% of 458 tour swings.
- hip_shift_at_top: observed 0.19, tour band 0 to 0.21, PASS, score 1.00
  percentile 86 of 458 tour swings (lower is better)
  rule-based reading: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 86% of 458 tour swings.
- head_stays_back: observed -0.18, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 85 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.18 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 85% of 458 tour swings.

unscored (attempted but could not be scored, and excluded from overall_score rather than counted as zero):
- finish_balance: the window held fewer usable frames than the measurement needs

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
none - this swing did not produce a placement, which happens when a metric or a phase anchor the model needs was missing. Say nothing about where it sits.

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: All 5 checkpoints are inside tour range. Closest to the edge: hip shift at top.
1. [info] hip_shift_at_top: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 86% of 458 tour swings.
2. [info] head_stays_back: Head stayed behind the ball - 0.18 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 85% of 458 tour swings.
3. [info] tempo: Good tempo - 4.0:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 110 ms; at the tour ratio that wants a 299-518 ms backswing, and yours was 440. You swing with a slower tempo than 72% of 1399 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.24 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 67% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
6. [info] finish_balance: Finish balance could not be scored on this swing, so it is not included in the score. There were some usable frames but not enough of them to measure this reliably. Let the clip keep running through the finish rather than cutting it at impact.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.96)
carry: 140 yards
total: 146 yards
ball speed: 106 mph
club head speed: 80 mph
smash factor: 1.33
club path: 1.8 degrees, + = in-to-out
club face angle: -0.4 degrees, + = open
launch angle: 20.1 degrees
spin rate: 7200 rpm
shape: SLIGHT DRAW
strike: TOE

TWO-VIEW ALIGNMENT
no down-the-line view - nothing to align, and nothing scored from it anyway

NOTES RECORDED WHILE ANALYZING THIS SWING
- no down-the-line view in this bundle — face-on analysis only
```

## `uncalibrated-placement`

Tags: uncalibrated-placement. Swing 2026-07-01/3, club 6i.

The face-on trajectory placements kept: a calibrated T2 and the uncalibrated Q beside it, both far out, and no shot attached. Large means unusual, never wrong, and Q is never read alone.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 36,
    "downswing_frames": 12,
    "head_sway": 0.03,
    "hip_sway": 0.05,
    "finish_drift": 0.02,
    "followthrough_frames": 60
  },
  "down_the_line": null,
  "shot": null,
  "loft_deg": null,
  "population": [
    "tour_joint_distance",
    "tour_trajectory_t2",
    "tour_trajectory_q"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 3 (session 2026-07-01)

overall_score: 95.7 out of 100
mechanics_score: 95.7 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.4, tour band 2.72 to 4.71, MISS, score 0.84
  percentile 10 of 1399 tour swings (both extremes are faults)
  rule-based reading: Tempo too quick - 2.4:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 360. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
- head_sway: observed 0.19, tour band 0 to 0.43, PASS, score 1.00
  percentile 55 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.19 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is more head movement than 55% of 458 tour swings.
- finish_balance: observed 0.05, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.05 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.32, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 54 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.32 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 54% of 458 tour swings.
- hip_shift_at_top: observed 0.23, tour band 0 to 0.21, MISS, score 0.92
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.13, tour band -0.67 to -0.14, MISS, score 0.98
  percentile 90 of 458 tour swings (both extremes are faults)
  rule-based reading: Head drifting forward - only 0.13 shoulder-widths of separation between head and hips by impact. The head is travelling toward the target with the hips rather than staying back (tour swings open up 0.14-0.67). That is less head-behind separation than at least 90% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.51 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 81% of them. Largest contributor: hip_shift_at_top_norm
- tour_trajectory_t2 (face-on view, calibrated): 5.76 sd_units
  distance from the tour swing shape inside the fitted subspace; more unusual than 90% of 415 tour swings
- tour_trajectory_q (face-on view, NOT calibrated - read beside its T2, never alone): 21.1 shoulder_widths
  residual off the tour subspace — shape the basis cannot represent; most of it falls in top->impact. NOT calibrated: it over-flags golfers the basis never saw, so read it beside T2 rather than alone

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on tempo first. Tempo too quick - 2.4:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 360. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
1. [minor] tempo: Tempo too quick - 2.4:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 360. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
2. [minor] hip_shift_at_top: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
3. [minor] head_stays_back: Head drifting forward - only 0.13 shoulder-widths of separation between head and hips by impact. The head is travelling toward the target with the hips rather than staying back (tour swings open up 0.14-0.67). That is less head-behind separation than at least 90% of 458 tour swings.
4. [info] head_sway: Good head stability - 0.19 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is more head movement than 55% of 458 tour swings.
5. [info] hip_sway: Good lower-body movement - 0.32 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 54% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.05 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
none attached to this swing

TWO-VIEW ALIGNMENT
no down-the-line view - nothing to align, and nothing scored from it anyway

NOTES RECORDED WHILE ANALYZING THIS SWING
- no down-the-line view in this bundle — face-on analysis only
```

## `interim-pivot`

Tags: interim-pivot. Swing 2026-07-01/4, club 9i.

The face-on rotation rows kept: interim 2-D readings with no reference population. make_swing's shoulders never move, so four of the five read zero, which is no turn measured and not a turn that did not happen.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 46,
    "downswing_frames": 13,
    "head_sway": 0.01,
    "hip_sway": 0.05,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 46,
    "downswing_frames": 13,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 78.0,
      "ball_speed": 99.0,
      "smash_factor": 1.27,
      "launch_angle": 23.0,
      "launch_direction": 0.4,
      "spin_rate": 8100.0,
      "spin_axis": 0.5,
      "carry_distance": 128.0,
      "bounce_and_roll": 5.0,
      "total_distance": 133.0,
      "club_path": 0.2,
      "club_face_angle": 0.3,
      "shot_type": "STRAIGHT",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.98,
    "read_warnings": []
  },
  "loft_deg": 42.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": [
    "pivot_hip_axis_drift_norm",
    "pivot_shoulder_axis_drift_norm",
    "pivot_hip_path_jitter_norm",
    "pivot_shoulder_reversal_backswing_deg",
    "pivot_shoulder_reversal_downswing_deg"
  ]
}
```

The brief:

```text
SWING 4 (session 2026-07-01)

overall_score: 97.9 out of 100
mechanics_score: 97.9 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.88, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 18 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 460. You swing with a quicker tempo than 82% of 1399 tour swings.
- head_sway: observed 0.06, tour band 0 to 0.43, PASS, score 1.00
  percentile 19 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.31, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 53 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
- hip_shift_at_top: observed 0.24, tour band 0 to 0.21, MISS, score 0.87
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.24 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.25, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 78 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.25 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 78% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.32 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 78.6% of them. Largest contributor: hip_shift_at_top_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
- pivot_hip_axis_drift_norm (face-on view, interim): 0.31 shoulder_widths
  measured in one uncalibrated phone's image plane: the ratio cancels the pixel aspect but not the foreshortening, which changes with where the camera stood, so it compares within a camera position and not across sessions or views. Fiducial calibration clears it.
- pivot_shoulder_axis_drift_norm (face-on view, interim): 0 shoulder_widths
  measured in one uncalibrated phone's image plane: the ratio cancels the pixel aspect but not the foreshortening, which changes with where the camera stood, so it compares within a camera position and not across sessions or views. Fiducial calibration clears it.
- pivot_hip_path_jitter_norm (face-on view, interim): 0 shoulder_widths
  measured in one uncalibrated phone's image plane: the ratio cancels the pixel aspect but not the foreshortening, which changes with where the camera stood, so it compares within a camera position and not across sessions or views. Fiducial calibration clears it.
- pivot_shoulder_reversal_backswing_deg (face-on view, interim): 0 degrees
  a projected 2-D line angle, not a turn: face-on the shoulder line collapses toward zero width at the top of the backswing, so this reads the half of the swing its camera sees squarely and is worst-conditioned at the other end. Its down-the-line partner is conditioned the opposite way; read the pair, never one alone. Fiducial calibration clears it.
- pivot_shoulder_reversal_downswing_deg (face-on view, interim): 0 degrees
  a projected 2-D line angle, not a turn: face-on the shoulder line collapses toward zero width at the top of the backswing, so this reads the half of the swing its camera sees squarely and is worst-conditioned at the other end. Its down-the-line partner is conditioned the opposite way; read the pair, never one alone. Fiducial calibration clears it.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.24 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.24 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [info] tempo: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 460. You swing with a quicker tempo than 82% of 1399 tour swings.
3. [info] head_stays_back: Head stayed behind the ball - 0.25 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 78% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.98)
carry: 128 yards
total: 133 yards
ball speed: 99 mph
club head speed: 78 mph
smash factor: 1.27
club path: 0.2 degrees, + = in-to-out
club face angle: 0.3 degrees, + = open
launch angle: 23 degrees
spin rate: 8100 rpm
shape: STRAIGHT
strike: CENTER

TWO-VIEW ALIGNMENT
quality: full

NOTES RECORDED WHILE ANALYZING THIS SWING
none
```

## `flagged-shot`

Tags: flagged-shot. Swing 2026-07-01/5, club 7i.

The screen photo misread: a smash factor of 1.87 that does not divide out and is past what is possible, and a tile with no value under its label. The validator flags the shot for review, so its numbers are not to be coached from as fact.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 42,
    "downswing_frames": 12,
    "head_sway": 0.02,
    "hip_sway": 0.05,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": null,
  "shot": {
    "printed": {
      "club_head_speed": 83.0,
      "ball_speed": 113.0,
      "smash_factor": 1.87,
      "launch_angle": 17.9,
      "launch_direction": 2.4,
      "spin_rate": 6100.0,
      "spin_axis": 5.6,
      "carry_distance": 153.0,
      "bounce_and_roll": 8.0,
      "total_distance": 161.0,
      "club_path": -2.6,
      "club_face_angle": 1.9,
      "shot_type": "FADE",
      "impact_position": "HEEL"
    },
    "read_confidence": 0.93,
    "read_warnings": [
      "Impact Position V: no value text under the label"
    ]
  },
  "loft_deg": 34.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 5 (session 2026-07-01)

overall_score: 97.9 out of 100
mechanics_score: 97.9 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.8, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 14 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.8:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 420. You swing with a quicker tempo than 86% of 1399 tour swings.
- head_sway: observed 0.13, tour band 0 to 0.43, PASS, score 1.00
  percentile 38 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.31, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 53 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
- hip_shift_at_top: observed 0.24, tour band 0 to 0.21, MISS, score 0.88
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.24 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.19, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 84 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.19 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 84% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.42 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 79.9% of them. Largest contributor: hip_shift_at_top_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.24 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.24 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [info] tempo: Good tempo - 2.8:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 420. You swing with a quicker tempo than 86% of 1399 tour swings.
3. [info] head_stays_back: Head stayed behind the ball - 0.19 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 84% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: yes (read off a photo of the launch monitor's screen by OCR, parse confidence 0.53)
  warning: Impact Position V: no value text under the label
  warning: smash factor 1.87 does not match ball speed / club speed (113 / 83 = 1.361) - one of the three was misread
  warning: smash_factor 1.87 is outside the plausible range 0.4-1.7 - likely a dropped or extra digit
carry: 153 yards
total: 161 yards
ball speed: 113 mph
club head speed: 83 mph
smash factor: 1.87
club path: -2.6 degrees, + = in-to-out
club face angle: 1.9 degrees, + = open
launch angle: 17.9 degrees
spin rate: 6100 rpm
shape: FADE
strike: HEEL

TWO-VIEW ALIGNMENT
no down-the-line view - nothing to align, and nothing scored from it anyway

NOTES RECORDED WHILE ANALYZING THIS SWING
- no down-the-line view in this bundle — face-on analysis only
- shot data needs review (parse confidence 0.53) — the numbers below were read off a photograph and at least one check on them failed (ADR-014)
```

## `partial-alignment`

Tags: partial-alignment. Swing 2026-07-01/6, club 5i.

The down-the-line clip pauses at the top, so its motion start is estimated and the two views align on top and impact only. Nothing is scored from that view, and the side-by-side replay is not to be trusted at every instant.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 40,
    "downswing_frames": 12,
    "head_sway": 0.015,
    "hip_sway": 0.05,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 40,
    "downswing_frames": 12,
    "hover_frames": 12,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 86.0,
      "ball_speed": 120.0,
      "smash_factor": 1.4,
      "launch_angle": 14.6,
      "launch_direction": -1.8,
      "spin_rate": 5200.0,
      "spin_axis": -4.5,
      "carry_distance": 170.0,
      "bounce_and_roll": 10.0,
      "total_distance": 180.0,
      "club_path": 3.1,
      "club_face_angle": 0.6,
      "shot_type": "DRAW",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.97,
    "read_warnings": []
  },
  "loft_deg": 27.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 6 (session 2026-07-01)

overall_score: 97.7 out of 100
mechanics_score: 97.7 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.67, tour band 2.72 to 4.71, MISS, score 0.97
  percentile 10 of 1399 tour swings (both extremes are faults)
  rule-based reading: Tempo too quick - 2.7:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 400. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
- head_sway: observed 0.09, tour band 0 to 0.43, PASS, score 1.00
  percentile 28 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.09 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 72% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.31, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 53 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
- hip_shift_at_top: observed 0.23, tour band 0 to 0.21, MISS, score 0.89
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.22, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 81 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.22 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 81% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.35 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 79% of them. Largest contributor: hip_shift_at_top_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [minor] tempo: Tempo too quick - 2.7:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 400. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
3. [info] head_stays_back: Head stayed behind the ball - 0.22 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 81% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.09 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 72% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.97)
carry: 170 yards
total: 180 yards
ball speed: 120 mph
club head speed: 86 mph
smash factor: 1.4
club path: 3.1 degrees, + = in-to-out
club face angle: 0.6 degrees, + = open
launch angle: 14.6 degrees
spin rate: 5200 rpm
shape: DRAW
strike: CENTER

TWO-VIEW ALIGNMENT
quality: top_impact
alignment_caveat: The two camera views were aligned on top and impact, not on every instant. Frame correspondence is exact at those anchors and interpolated between them, so do not describe the side-by-side video as synchronized throughout, and do not read fine timing differences off it.

NOTES RECORDED WHILE ANALYZING THIS SWING
- alignment degraded: aligned on top and impact
- alignment: b: motion start was estimated, not detected
```

## `no-handedness`

Tags: no-handedness. Swing 2026-07-01/7, club pw.

Nobody said who swung, so head stays back, the one signed checkpoint, is unscored for want of a handedness. Not a capture problem: filming again would not change it.

Made from:

```json
{
  "handedness": null,
  "face_on": {
    "backswing_frames": 44,
    "downswing_frames": 12,
    "head_sway": 0.01,
    "hip_sway": 0.045,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": null,
  "shot": {
    "printed": {
      "club_head_speed": 76.0,
      "ball_speed": 92.0,
      "smash_factor": 1.21,
      "launch_angle": 27.5,
      "launch_direction": 0.9,
      "spin_rate": 9100.0,
      "spin_axis": 1.5,
      "carry_distance": 112.0,
      "bounce_and_roll": 3.0,
      "total_distance": 115.0,
      "club_path": -0.8,
      "club_face_angle": 0.5,
      "shot_type": "SLIGHT FADE",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.95,
    "read_warnings": []
  },
  "loft_deg": 46.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 7 (session 2026-07-01)

overall_score: 99.5 out of 100
mechanics_score: 99.5 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.93, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 22 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 440. You swing with a quicker tempo than 78% of 1399 tour swings.
- head_sway: observed 0.06, tour band 0 to 0.43, PASS, score 1.00
  percentile 19 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.28, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 44 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.28 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 56% of 458 tour swings.
- hip_shift_at_top: observed 0.21, tour band 0 to 0.21, MISS, score 0.98
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.21 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.

unscored (attempted but could not be scored, and excluded from overall_score rather than counted as zero):
- head_stays_back: no golfer is attributed, and this checkpoint's sign depends on which side you swing from
- flight_landing_offline_yds: no golfer is attributed, and this checkpoint's sign depends on which side you swing from

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.08 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 75.6% of them. Largest contributor: hip_shift_at_top_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.21 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.21 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [info] tempo: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 440. You swing with a quicker tempo than 78% of 1399 tour swings.
3. [info] hip_sway: Good lower-body movement - 0.28 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 56% of 458 tour swings.
4. [info] head_sway: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
5. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
6. [info] head_stays_back: Head stays back could not be scored on this swing, so it is not included in the score. The swing itself was measured fine - this checkpoint also needs to know which side you swing from, and no golfer is attributed to this swing. Pick a golfer for the session and it will score without re-filming.
7. [info] flight_landing_offline_yds: Flight landing offline yds could not be scored on this swing, so it is not included in the score. The swing itself was measured fine - this checkpoint also needs to know which side you swing from, and no golfer is attributed to this swing. Pick a golfer for the session and it will score without re-filming.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.95)
carry: 112 yards
total: 115 yards
ball speed: 92 mph
club head speed: 76 mph
smash factor: 1.21
club path: -0.8 degrees, + = in-to-out
club face angle: 0.5 degrees, + = open
launch angle: 27.5 degrees
spin rate: 9100 rpm
shape: SLIGHT FADE
strike: CENTER

TWO-VIEW ALIGNMENT
no down-the-line view - nothing to align, and nothing scored from it anyway

NOTES RECORDED WHILE ANALYZING THIS SWING
- no down-the-line view in this bundle — face-on analysis only
```

## `nothing-to-fix`

Tags: nothing-to-fix. Swing 2026-07-01/8, club 7i.

Every checkpoint scored and inside its band, two views aligned in full and a clean shot. There is nothing to fix, and a coach that invents a fault is wrong.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 48,
    "downswing_frames": 14,
    "head_sway": 0.01,
    "hip_sway": 0.035,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 48,
    "downswing_frames": 14,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 84.0,
      "ball_speed": 116.0,
      "smash_factor": 1.38,
      "launch_angle": 17.2,
      "launch_direction": 0.3,
      "spin_rate": 6200.0,
      "spin_axis": 0.8,
      "carry_distance": 158.0,
      "bounce_and_roll": 8.0,
      "total_distance": 166.0,
      "club_path": 0.4,
      "club_face_angle": 0.2,
      "shot_type": "STRAIGHT",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.98,
    "read_warnings": []
  },
  "loft_deg": 34.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 8 (session 2026-07-01)

overall_score: 100 out of 100
mechanics_score: 100 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.82, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 16 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.8:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 170 ms; at the tour ratio that wants a 462-801 ms backswing, and yours was 480. You swing with a quicker tempo than 84% of 1399 tour swings.
- head_sway: observed 0.06, tour band 0 to 0.43, PASS, score 1.00
  percentile 19 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.22, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 26 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.22 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 74% of 458 tour swings.
- hip_shift_at_top: observed 0.16, tour band 0 to 0.21, PASS, score 1.00
  percentile 80 of 458 tour swings (lower is better)
  rule-based reading: Steady hips going back - 0.16 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 80% of 458 tour swings.
- head_stays_back: observed -0.16, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 88 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.16 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 88% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 2.8 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 67.9% of them. Largest contributor: finish_balance_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: All 6 checkpoints are inside tour range. Closest to the edge: head stays back.
1. [info] head_stays_back: Head stayed behind the ball - 0.16 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 88% of 458 tour swings.
2. [info] tempo: Good tempo - 2.8:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 170 ms; at the tour ratio that wants a 462-801 ms backswing, and yours was 480. You swing with a quicker tempo than 84% of 1399 tour swings.
3. [info] hip_shift_at_top: Steady hips going back - 0.16 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 80% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.22 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 74% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.98)
carry: 158 yards
total: 166 yards
ball speed: 116 mph
club head speed: 84 mph
smash factor: 1.38
club path: 0.4 degrees, + = in-to-out
club face angle: 0.2 degrees, + = open
launch angle: 17.2 degrees
spin rate: 6200 rpm
shape: STRAIGHT
strike: CENTER

TWO-VIEW ALIGNMENT
quality: full

NOTES RECORDED WHILE ANALYZING THIS SWING
none
```

## `clean-left-driver`

Tags: clean. Swing 2026-07-01/9, club driver.

A left-hander's driver, face-on only, with a clean shot and nothing degraded. The head drifts too far toward the target, so there is something to coach, and every drift is mirrored, which is what a left-hander's swing looks like from the same camera.

Made from:

```json
{
  "handedness": "left",
  "face_on": {
    "backswing_frames": 50,
    "downswing_frames": 14,
    "head_sway": -0.08,
    "hip_sway": -0.04,
    "finish_drift": -0.01,
    "followthrough_frames": 60
  },
  "down_the_line": null,
  "shot": {
    "printed": {
      "club_head_speed": 98.0,
      "ball_speed": 143.0,
      "smash_factor": 1.46,
      "launch_angle": 12.5,
      "launch_direction": -1.2,
      "spin_rate": 2900.0,
      "spin_axis": 2.4,
      "carry_distance": 228.0,
      "bounce_and_roll": 18.0,
      "total_distance": 246.0,
      "club_path": -2.2,
      "club_face_angle": 0.4,
      "shot_type": "SLIGHT FADE",
      "impact_position": "TOE"
    },
    "read_confidence": 0.97,
    "read_warnings": []
  },
  "loft_deg": 10.5,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 9 (session 2026-07-01)

overall_score: 85.1 out of 100
mechanics_score: 85.1 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.94, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 22 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 170 ms; at the tour ratio that wants a 462-801 ms backswing, and yours was 500. You swing with a quicker tempo than 78% of 1399 tour swings.
- head_sway: observed 0.5, tour band 0 to 0.43, MISS, score 0.84
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Head sway - 0.50 shoulder-widths of lateral movement to impact. Keep your head centered over the ball (aim under 0.43). That is more head movement than at least 90% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.25, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 35 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.25 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 65% of 458 tour swings.
- hip_shift_at_top: observed 0.19, tour band 0 to 0.21, PASS, score 1.00
  percentile 86 of 458 tour swings (lower is better)
  rule-based reading: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 86% of 458 tour swings.
- head_stays_back: observed 0.25, tour band -0.67 to -0.14, MISS, score 0.27
  percentile 90 of 458 tour swings (both extremes are faults)
  rule-based reading: Head drifting forward - only 0.25 shoulder-widths of separation between head and hips by impact. The head is travelling toward the target with the hips rather than staying back (tour swings open up 0.14-0.67). That is less head-behind separation than at least 90% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 4.38 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 90% of them. Largest contributor: head_sway_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on head stays back first. Head drifting forward - only 0.25 shoulder-widths of separation between head and hips by impact. The head is travelling toward the target with the hips rather than staying back (tour swings open up 0.14-0.67). That is less head-behind separation than at least 90% of 458 tour swings.
1. [major] head_stays_back: Head drifting forward - only 0.25 shoulder-widths of separation between head and hips by impact. The head is travelling toward the target with the hips rather than staying back (tour swings open up 0.14-0.67). That is less head-behind separation than at least 90% of 458 tour swings.
2. [minor] head_sway: Head sway - 0.50 shoulder-widths of lateral movement to impact. Keep your head centered over the ball (aim under 0.43). That is more head movement than at least 90% of 458 tour swings.
3. [info] hip_shift_at_top: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 86% of 458 tour swings.
4. [info] tempo: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 170 ms; at the tour ratio that wants a 462-801 ms backswing, and yours was 500. You swing with a quicker tempo than 78% of 1399 tour swings.
5. [info] hip_sway: Good lower-body movement - 0.25 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 65% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.97)
carry: 228 yards
total: 246 yards
ball speed: 143 mph
club head speed: 98 mph
smash factor: 1.46
club path: -2.2 degrees, + = in-to-out
club face angle: 0.4 degrees, + = open
launch angle: 12.5 degrees
spin rate: 2900 rpm
shape: SLIGHT FADE
strike: TOE

TWO-VIEW ALIGNMENT
no down-the-line view - nothing to align, and nothing scored from it anyway

NOTES RECORDED WHILE ANALYZING THIS SWING
- no down-the-line view in this bundle — face-on analysis only
```

## `unscored-no-loft`

Tags: unscored. Swing 2026-07-01/10, club 4h.

The screen printed no spin, so the simulated flight infers it from the carry, and two spins fly that carry with no loft on record for the hybrid to choose between them: the flight's carry is refused. Every mechanics checkpoint is scored. The refusal is for want of a loft, not a capture problem, so a better clip or photo would not change it.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 42,
    "downswing_frames": 13,
    "head_sway": 0.015,
    "hip_sway": 0.05,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 42,
    "downswing_frames": 13,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 85.0,
      "ball_speed": 121.0,
      "smash_factor": 1.42,
      "launch_angle": 15.8,
      "launch_direction": 0.7,
      "spin_axis": 1.8,
      "carry_distance": 182.0,
      "bounce_and_roll": 11.0,
      "total_distance": 193.0,
      "club_path": -0.9,
      "club_face_angle": 0.9,
      "shot_type": "SLIGHT FADE",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.96,
    "read_warnings": [
      "Spin: no value text under the label"
    ]
  },
  "loft_deg": null,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 10 (session 2026-07-01)

overall_score: 97.5 out of 100
mechanics_score: 97.5 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.62, tour band 2.72 to 4.71, MISS, score 0.95
  percentile 10 of 1399 tour swings (both extremes are faults)
  rule-based reading: Tempo too quick - 2.6:1. Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 420. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
- head_sway: observed 0.09, tour band 0 to 0.43, PASS, score 1.00
  percentile 28 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.09 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 72% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.31, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 53 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
- hip_shift_at_top: observed 0.23, tour band 0 to 0.21, MISS, score 0.90
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.22, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 81 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.22 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 81% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

simulated ball flight (a model over the printed launch conditions, in no score at all - not a checkpoint and never part of overall_score):
- flight_carry_yds: two spins fly that carry and no club loft is on record to choose between them

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.34 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 79% of them. Largest contributor: hip_shift_at_top_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [minor] tempo: Tempo too quick - 2.6:1. Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 420. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
3. [info] head_stays_back: Head stayed behind the ball - 0.22 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 81% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.31 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 53% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.09 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 72% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.96)
  warning: Spin: no value text under the label
carry: 182 yards
total: 193 yards
ball speed: 121 mph
club head speed: 85 mph
smash factor: 1.42
club path: -0.9 degrees, + = in-to-out
club face angle: 0.9 degrees, + = open
launch angle: 15.8 degrees
shape: SLIGHT FADE
strike: CENTER

TWO-VIEW ALIGNMENT
quality: full

NOTES RECORDED WHILE ANALYZING THIS SWING
none
```

## `uncalibrated-placement-dtl`

Tags: uncalibrated-placement. Swing 2026-07-01/11, club 3w.

The down-the-line trajectory placements kept: a calibrated T2 and the uncalibrated Q from the second view, beside the face-on joint placement, with a clean shot. Large means unusual, never wrong, and Q is never read alone.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 42,
    "downswing_frames": 13,
    "head_sway": 0.02,
    "hip_sway": 0.05,
    "finish_drift": 0.015,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 42,
    "downswing_frames": 13,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 92.0,
      "ball_speed": 133.0,
      "smash_factor": 1.45,
      "launch_angle": 11.8,
      "launch_direction": -0.8,
      "spin_rate": 3700.0,
      "spin_axis": -2.6,
      "carry_distance": 208.0,
      "bounce_and_roll": 14.0,
      "total_distance": 222.0,
      "club_path": 2.4,
      "club_face_angle": -0.3,
      "shot_type": "SLIGHT DRAW",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.97,
    "read_warnings": []
  },
  "loft_deg": 15.0,
  "population": [
    "tour_joint_distance",
    "tour_trajectory_t2_dtl",
    "tour_trajectory_q_dtl"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 11 (session 2026-07-01)

overall_score: 97.5 out of 100
mechanics_score: 97.5 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.62, tour band 2.72 to 4.71, MISS, score 0.95
  percentile 10 of 1399 tour swings (both extremes are faults)
  rule-based reading: Tempo too quick - 2.6:1. Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 420. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
- head_sway: observed 0.13, tour band 0 to 0.43, PASS, score 1.00
  percentile 38 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
- finish_balance: observed 0.04, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.04 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.32, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 54 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.32 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 54% of 458 tour swings.
- hip_shift_at_top: observed 0.23, tour band 0 to 0.21, MISS, score 0.90
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.19, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 84 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.19 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 84% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.29 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 78.3% of them. Largest contributor: hip_shift_at_top_norm
- tour_trajectory_t2_dtl (down-the-line view, calibrated): 5.4 sd_units
  down-the-line: distance from the tour swing shape inside its fitted subspace; more unusual than 90% of 510 tour swings. A separate basis from the face-on one — never combine the two
- tour_trajectory_q_dtl (down-the-line view, NOT calibrated - read beside its T2, never alone): 19.52 shoulder_widths
  down-the-line: residual off that basis; most of it falls in address->top. NOT calibrated, same caveat as the face-on Q

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.23 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [minor] tempo: Tempo too quick - 2.6:1. Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 420. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
3. [info] head_stays_back: Head stayed behind the ball - 0.19 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 84% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.32 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 54% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.04 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.97)
carry: 208 yards
total: 222 yards
ball speed: 133 mph
club head speed: 92 mph
smash factor: 1.45
club path: 2.4 degrees, + = in-to-out
club face angle: -0.3 degrees, + = open
launch angle: 11.8 degrees
spin rate: 3700 rpm
shape: SLIGHT DRAW
strike: CENTER

TWO-VIEW ALIGNMENT
quality: full

NOTES RECORDED WHILE ANALYZING THIS SWING
none
```

## `interim-pivot-dtl`

Tags: interim-pivot. Swing 2026-07-01/12, club gw.

A left-hander's gap wedge with the down-the-line rotation rows kept: interim 2-D readings from the second view, with no reference population and no band, never scored and never blended with the face-on view. make_swing's shoulders never move, so the three shoulder rows read zero, which is no turn measured and not a turn that did not happen. The hips barely travel toward the target, which misses a two-sided checkpoint on its low side: less is not better there.

Made from:

```json
{
  "handedness": "left",
  "face_on": {
    "backswing_frames": 46,
    "downswing_frames": 13,
    "head_sway": 0.01,
    "hip_sway": -0.018,
    "finish_drift": -0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 46,
    "downswing_frames": 13,
    "hip_sway": 0.02,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 72.0,
      "ball_speed": 86.0,
      "smash_factor": 1.19,
      "launch_angle": 29.5,
      "launch_direction": 1.4,
      "spin_rate": 9600.0,
      "spin_axis": 2.1,
      "carry_distance": 101.0,
      "bounce_and_roll": 2.0,
      "total_distance": 103.0,
      "club_path": -1.1,
      "club_face_angle": 1.0,
      "shot_type": "SLIGHT FADE",
      "impact_position": "HEEL"
    },
    "read_confidence": 0.96,
    "read_warnings": []
  },
  "loft_deg": 50.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": [
    "pivot_hip_axis_drift_norm_dtl",
    "pivot_shoulder_axis_drift_norm_dtl",
    "pivot_hip_path_jitter_norm_dtl",
    "pivot_shoulder_reversal_backswing_deg_dtl",
    "pivot_shoulder_reversal_downswing_deg_dtl"
  ]
}
```

The brief:

```text
SWING 12 (session 2026-07-01)

overall_score: 98.8 out of 100
mechanics_score: 98.8 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.88, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 18 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 460. You swing with a quicker tempo than 82% of 1399 tour swings.
- head_sway: observed 0.06, tour band 0 to 0.43, PASS, score 1.00
  percentile 19 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.11, tour band 0.14 to 0.5, MISS, score 0.93
  percentile 10 of 458 tour swings (both extremes are faults)
  rule-based reading: Little hip movement - 0.11 shoulder-widths of lateral hip travel to impact, under the tour range 0.14-0.5. Almost every tour swing moves the hips further into the shot than this. That is less hip travel than at least 90% of 458 tour swings.
- hip_shift_at_top: observed 0.09, tour band 0 to 0.21, PASS, score 1.00
  percentile 50 of 458 tour swings (lower is better)
  rule-based reading: Steady hips going back - 0.09 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 50% of 458 tour swings.
- head_stays_back: observed -0.18, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 86 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.18 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 86% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.95 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 86.5% of them. Largest contributor: head_hip_gain_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
- pivot_hip_axis_drift_norm_dtl (down-the-line view, interim): 0.12 shoulder_widths
  measured in one uncalibrated phone's image plane: the ratio cancels the pixel aspect but not the foreshortening, which changes with where the camera stood, so it compares within a camera position and not across sessions or views. Fiducial calibration clears it.
- pivot_shoulder_axis_drift_norm_dtl (down-the-line view, interim): 0 shoulder_widths
  measured in one uncalibrated phone's image plane: the ratio cancels the pixel aspect but not the foreshortening, which changes with where the camera stood, so it compares within a camera position and not across sessions or views. Fiducial calibration clears it.
- pivot_hip_path_jitter_norm_dtl (down-the-line view, interim): 0 shoulder_widths
  measured in one uncalibrated phone's image plane: the ratio cancels the pixel aspect but not the foreshortening, which changes with where the camera stood, so it compares within a camera position and not across sessions or views. Fiducial calibration clears it.
- pivot_shoulder_reversal_backswing_deg_dtl (down-the-line view, interim): 0 degrees
  a projected 2-D line angle, not a turn: down-the-line the shoulder line is collapsed at address and opens through the turn, so this reads the half of the swing its camera sees squarely and is worst-conditioned at the other end. Its face-on partner is conditioned the opposite way; read the pair, never one alone. Fiducial calibration clears it.
- pivot_shoulder_reversal_downswing_deg_dtl (down-the-line view, interim): 0 degrees
  a projected 2-D line angle, not a turn: down-the-line the shoulder line is collapsed at address and opens through the turn, so this reads the half of the swing its camera sees squarely and is worst-conditioned at the other end. Its face-on partner is conditioned the opposite way; read the pair, never one alone. Fiducial calibration clears it.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip sway first. Little hip movement - 0.11 shoulder-widths of lateral hip travel to impact, under the tour range 0.14-0.5. Almost every tour swing moves the hips further into the shot than this. That is less hip travel than at least 90% of 458 tour swings.
1. [minor] hip_sway: Little hip movement - 0.11 shoulder-widths of lateral hip travel to impact, under the tour range 0.14-0.5. Almost every tour swing moves the hips further into the shot than this. That is less hip travel than at least 90% of 458 tour swings.
2. [info] head_stays_back: Head stayed behind the ball - 0.18 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 86% of 458 tour swings.
3. [info] tempo: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 460. You swing with a quicker tempo than 82% of 1399 tour swings.
4. [info] hip_shift_at_top: Steady hips going back - 0.09 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 50% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.96)
carry: 101 yards
total: 103 yards
ball speed: 86 mph
club head speed: 72 mph
smash factor: 1.19
club path: -1.1 degrees, + = in-to-out
club face angle: 1 degrees, + = open
launch angle: 29.5 degrees
spin rate: 9600 rpm
shape: SLIGHT FADE
strike: HEEL

TWO-VIEW ALIGNMENT
quality: full

NOTES RECORDED WHILE ANALYZING THIS SWING
none
```

## `flagged-shot-distance`

Tags: flagged-shot. Swing 2026-07-01/13, club 6i.

The screen photo misread the total: carry and roll add to 175 and the total reads 135, so one of the three is wrong. The validator flags the shot for review, so its distances are not to be coached from as fact.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 44,
    "downswing_frames": 12,
    "head_sway": 0.02,
    "hip_sway": 0.055,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": null,
  "shot": {
    "printed": {
      "club_head_speed": 84.0,
      "ball_speed": 119.0,
      "smash_factor": 1.42,
      "launch_angle": 15.2,
      "launch_direction": -0.5,
      "spin_rate": 5600.0,
      "spin_axis": -1.4,
      "carry_distance": 166.0,
      "bounce_and_roll": 9.0,
      "total_distance": 135.0,
      "club_path": 1.6,
      "club_face_angle": -0.2,
      "shot_type": "SLIGHT DRAW",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.91,
    "read_warnings": []
  },
  "loft_deg": 30.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 13 (session 2026-07-01)

overall_score: 95.8 out of 100
mechanics_score: 95.8 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.93, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 22 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 440. You swing with a quicker tempo than 78% of 1399 tour swings.
- head_sway: observed 0.13, tour band 0 to 0.43, PASS, score 1.00
  percentile 38 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.35, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 62 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.35 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 62% of 458 tour swings.
- hip_shift_at_top: observed 0.26, tour band 0 to 0.21, MISS, score 0.75
  percentile 90 of 458 tour swings (lower is better)
  rule-based reading: Hip slide going back - 0.26 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
- head_stays_back: observed -0.22, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 81 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.22 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 81% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.67 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 83.1% of them. Largest contributor: hip_shift_at_top_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on hip shift at top first. Hip slide going back - 0.26 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
1. [minor] hip_shift_at_top: Hip slide going back - 0.26 shoulder-widths of lateral hip travel to the top. The hips are travelling sideways during the backswing rather than staying centered (aim under 0.21). That is more hip slide going back than at least 90% of 458 tour swings.
2. [info] head_stays_back: Head stayed behind the ball - 0.22 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 81% of 458 tour swings.
3. [info] tempo: Good tempo - 2.9:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 440. You swing with a quicker tempo than 78% of 1399 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.35 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is more hip travel than 62% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: yes (read off a photo of the launch monitor's screen by OCR, parse confidence 0.66)
  warning: shot distance 135 does not match carry + bounce & roll (166 + 9 = 175) - one of the three was misread
carry: 166 yards
total: 135 yards
ball speed: 119 mph
club head speed: 84 mph
smash factor: 1.42
club path: 1.6 degrees, + = in-to-out
club face angle: -0.2 degrees, + = open
launch angle: 15.2 degrees
spin rate: 5600 rpm
shape: SLIGHT DRAW
strike: CENTER

TWO-VIEW ALIGNMENT
no down-the-line view - nothing to align, and nothing scored from it anyway

NOTES RECORDED WHILE ANALYZING THIS SWING
- no down-the-line view in this bundle — face-on analysis only
- shot data needs review (parse confidence 0.66) — the numbers below were read off a photograph and at least one check on them failed (ADR-014)
```

## `partial-alignment-tempo`

Tags: partial-alignment. Swing 2026-07-01/14, club sw.

The two clips disagree on tempo, so they may be two different swings, and they align on top and impact only. Nothing is scored from the down-the-line view, and the side-by-side replay is not to be trusted at every instant.

Made from:

```json
{
  "handedness": "right",
  "face_on": {
    "backswing_frames": 40,
    "downswing_frames": 12,
    "head_sway": 0.01,
    "hip_sway": 0.04,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 72,
    "downswing_frames": 12,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 68.0,
      "ball_speed": 78.0,
      "smash_factor": 1.15,
      "launch_angle": 32.0,
      "launch_direction": 0.6,
      "spin_rate": 10200.0,
      "spin_axis": 0.6,
      "carry_distance": 88.0,
      "bounce_and_roll": 2.0,
      "total_distance": 90.0,
      "club_path": 0.3,
      "club_face_angle": 0.1,
      "shot_type": "STRAIGHT",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.97,
    "read_warnings": []
  },
  "loft_deg": 56.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 14 (session 2026-07-01)

overall_score: 99.6 out of 100
mechanics_score: 99.6 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.67, tour band 2.72 to 4.71, MISS, score 0.97
  percentile 10 of 1399 tour swings (both extremes are faults)
  rule-based reading: Tempo too quick - 2.7:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 400. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
- head_sway: observed 0.06, tour band 0 to 0.43, PASS, score 1.00
  percentile 19 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.25, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 35 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.25 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 65% of 458 tour swings.
- hip_shift_at_top: observed 0.19, tour band 0 to 0.21, PASS, score 1.00
  percentile 85 of 458 tour swings (lower is better)
  rule-based reading: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 85% of 458 tour swings.
- head_stays_back: observed -0.19, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 84 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.19 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 84% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 2.94 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 72.2% of them. Largest contributor: finish_balance_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on tempo first. Tempo too quick - 2.7:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 400. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
1. [minor] tempo: Tempo too quick - 2.7:1. Your downswing was 150 ms; at the tour ratio that wants a 408-706 ms backswing, and yours was 400. Take it back longer. You swing with a quicker tempo than at least 90% of 1399 tour swings.
2. [info] hip_shift_at_top: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 85% of 458 tour swings.
3. [info] head_stays_back: Head stayed behind the ball - 0.19 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 84% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.25 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 65% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.97)
carry: 88 yards
total: 90 yards
ball speed: 78 mph
club head speed: 68 mph
smash factor: 1.15
club path: 0.3 degrees, + = in-to-out
club face angle: 0.1 degrees, + = open
launch angle: 32 degrees
spin rate: 10200 rpm
shape: STRAIGHT
strike: CENTER

TWO-VIEW ALIGNMENT
quality: top_impact
alignment_caveat: The two camera views were aligned on top and impact, not on every instant. Frame correspondence is exact at those anchors and interpolated between them, so do not describe the side-by-side video as synchronized throughout, and do not read fine timing differences off it.

NOTES RECORDED WHILE ANALYZING THIS SWING
- alignment degraded: aligned on top and impact
- alignment: tempo ratios disagree (2.67 vs 4.44) — frame rate cancels out of a ratio, so two views of one swing should not. Most likely the two clips are showing DIFFERENT swings; check for a practice swing in one of them
```

## `no-handedness-driver`

Tags: no-handedness. Swing 2026-07-01/15, club driver.

A driver with two views and nobody named as the golfer, so head stays back is unscored for want of a handedness, as is the simulated landing's offline distance. The backswing is slow, so there is something to coach beside it. Not a capture problem: filming again would not change it.

Made from:

```json
{
  "handedness": null,
  "face_on": {
    "backswing_frames": 80,
    "downswing_frames": 12,
    "head_sway": 0.02,
    "hip_sway": 0.035,
    "finish_drift": 0.01,
    "followthrough_frames": 60
  },
  "down_the_line": {
    "backswing_frames": 80,
    "downswing_frames": 12,
    "followthrough_frames": 60
  },
  "shot": {
    "printed": {
      "club_head_speed": 102.0,
      "ball_speed": 150.0,
      "smash_factor": 1.47,
      "launch_angle": 11.2,
      "launch_direction": 2.3,
      "spin_rate": 2500.0,
      "spin_axis": 4.1,
      "carry_distance": 241.0,
      "bounce_and_roll": 21.0,
      "total_distance": 262.0,
      "club_path": -3.0,
      "club_face_angle": 1.2,
      "shot_type": "FADE",
      "impact_position": "HEEL"
    },
    "read_confidence": 0.96,
    "read_warnings": []
  },
  "loft_deg": 9.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 15 (session 2026-07-01)

overall_score: 97.7 out of 100
mechanics_score: 97.7 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 4.94, tour band 2.72 to 4.71, MISS, score 0.89
  percentile 90 of 1399 tour swings (both extremes are faults)
  rule-based reading: Tempo too slow - 4.9:1. Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 790. Take it back shorter. You swing with a slower tempo than at least 90% of 1399 tour swings.
- head_sway: observed 0.13, tour band 0 to 0.43, PASS, score 1.00
  percentile 38 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.22, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 26 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.22 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 74% of 458 tour swings.
- hip_shift_at_top: observed 0.19, tour band 0 to 0.21, PASS, score 1.00
  percentile 85 of 458 tour swings (lower is better)
  rule-based reading: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 85% of 458 tour swings.

unscored (attempted but could not be scored, and excluded from overall_score rather than counted as zero):
- head_stays_back: no golfer is attributed, and this checkpoint's sign depends on which side you swing from
- flight_landing_offline_yds: no golfer is attributed, and this checkpoint's sign depends on which side you swing from

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.45 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 80.3% of them. Largest contributor: finish_balance_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: Work on tempo first. Tempo too slow - 4.9:1. Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 790. Take it back shorter. You swing with a slower tempo than at least 90% of 1399 tour swings.
1. [minor] tempo: Tempo too slow - 4.9:1. Your downswing was 160 ms; at the tour ratio that wants a 435-754 ms backswing, and yours was 790. Take it back shorter. You swing with a slower tempo than at least 90% of 1399 tour swings.
2. [info] hip_shift_at_top: Steady hips going back - 0.19 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 85% of 458 tour swings.
3. [info] hip_sway: Good lower-body movement - 0.22 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 74% of 458 tour swings.
4. [info] head_sway: Good head stability - 0.13 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 62% of 458 tour swings.
5. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
6. [info] head_stays_back: Head stays back could not be scored on this swing, so it is not included in the score. The swing itself was measured fine - this checkpoint also needs to know which side you swing from, and no golfer is attributed to this swing. Pick a golfer for the session and it will score without re-filming.
7. [info] flight_landing_offline_yds: Flight landing offline yds could not be scored on this swing, so it is not included in the score. The swing itself was measured fine - this checkpoint also needs to know which side you swing from, and no golfer is attributed to this swing. Pick a golfer for the session and it will score without re-filming.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.96)
carry: 241 yards
total: 262 yards
ball speed: 150 mph
club head speed: 102 mph
smash factor: 1.47
club path: -3 degrees, + = in-to-out
club face angle: 1.2 degrees, + = open
launch angle: 11.2 degrees
spin rate: 2500 rpm
shape: FADE
strike: HEEL

TWO-VIEW ALIGNMENT
quality: full

NOTES RECORDED WHILE ANALYZING THIS SWING
none
```

## `nothing-to-fix-left`

Tags: nothing-to-fix. Swing 2026-07-01/16, club 9i.

A left-hander, face-on only: every checkpoint scored and inside its band, and a clean shot. There is nothing to fix, and a coach that invents a fault, or reads the mirrored drifts as one, is wrong.

Made from:

```json
{
  "handedness": "left",
  "face_on": {
    "backswing_frames": 48,
    "downswing_frames": 14,
    "head_sway": -0.01,
    "hip_sway": -0.035,
    "finish_drift": -0.01,
    "followthrough_frames": 60
  },
  "down_the_line": null,
  "shot": {
    "printed": {
      "club_head_speed": 76.0,
      "ball_speed": 98.0,
      "smash_factor": 1.29,
      "launch_angle": 22.4,
      "launch_direction": -0.4,
      "spin_rate": 7900.0,
      "spin_axis": -0.6,
      "carry_distance": 126.0,
      "bounce_and_roll": 5.0,
      "total_distance": 131.0,
      "club_path": 0.6,
      "club_face_angle": -0.1,
      "shot_type": "STRAIGHT",
      "impact_position": "CENTER"
    },
    "read_confidence": 0.98,
    "read_warnings": []
  },
  "loft_deg": 42.0,
  "population": [
    "tour_joint_distance"
  ],
  "rotation": []
}
```

The brief:

```text
SWING 16 (session 2026-07-01)

overall_score: 100 out of 100
mechanics_score: 100 out of 100
outcome_score: not computed - the launch-monitor numbers are attached and displayed but not yet scored, so overall_score reflects mechanics only

CHECKPOINTS MEASURED (face-on view only)
- tempo: observed 2.82, tour band 2.72 to 4.71, PASS, score 1.00
  percentile 16 of 1399 tour swings (both extremes are faults)
  rule-based reading: Good tempo - 2.8:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 170 ms; at the tour ratio that wants a 462-801 ms backswing, and yours was 480. You swing with a quicker tempo than 84% of 1399 tour swings.
- head_sway: observed 0.06, tour band 0 to 0.43, PASS, score 1.00
  percentile 19 of 458 tour swings (lower is better)
  rule-based reading: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
- finish_balance: observed 0.03, tour band 0 to 0.29, PASS, score 1.00
  percentile 10 of 458 tour swings (lower is better)
  rule-based reading: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.
- hip_sway: observed 0.22, tour band 0.14 to 0.5, PASS, score 1.00
  percentile 26 of 458 tour swings (both extremes are faults)
  rule-based reading: Good lower-body movement - 0.22 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 74% of 458 tour swings.
- hip_shift_at_top: observed 0.16, tour band 0 to 0.21, PASS, score 1.00
  percentile 80 of 458 tour swings (lower is better)
  rule-based reading: Steady hips going back - 0.16 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 80% of 458 tour swings.
- head_stays_back: observed -0.16, tour band -0.67 to -0.14, PASS, score 1.00
  percentile 88 of 458 tour swings (both extremes are faults)
  rule-based reading: Head stayed behind the ball - 0.16 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 88% of 458 tour swings.

unscored: none - every checkpoint was scored on this swing

POPULATION PLACEMENT (recorded, never scored - nothing here has a band)
- tour_joint_distance (face-on view, calibrated): 3.98 sd_units
  Mahalanobis distance of the six metrics as a combination, against 458 face-on tour swings; more unusual than 87% of them. Largest contributor: head_hip_gain_norm

ROTATION (interim - a 2-D-per-view reading, no reference population, never a band, never part of overall_score, never blended across views)
none - this swing did not produce a rotation reading, which happens when the tracking a check needs was missing. Say nothing about the golfer's turn.

THIS SYSTEM'S OWN RANKED VERDICT (rule-based, most actionable first)
headline: All 6 checkpoints are inside tour range. Closest to the edge: head stays back.
1. [info] head_stays_back: Head stayed behind the ball - 0.16 shoulder-widths of separation opened up between head and hips through impact (inside the tour range). That is less head-behind separation than 88% of 458 tour swings.
2. [info] tempo: Good tempo - 2.8:1 backswing:downswing (inside the tour range 2.72-4.71:1). Your downswing was 170 ms; at the tour ratio that wants a 462-801 ms backswing, and yours was 480. You swing with a quicker tempo than 84% of 1399 tour swings.
3. [info] hip_shift_at_top: Steady hips going back - 0.16 shoulder-widths of lateral hip travel to the top (centered over the ball at the top). That is more hip slide going back than 80% of 458 tour swings.
4. [info] hip_sway: Good lower-body movement - 0.22 shoulder-widths of lateral hip travel to impact (inside the tour range 0.14-0.5). That is less hip travel than 74% of 458 tour swings.
5. [info] head_sway: Good head stability - 0.06 shoulder-widths of lateral head movement to impact (nicely centered over the ball). That is less head movement than 81% of 458 tour swings.
6. [info] finish_balance: Balanced finish - the body settles within 0.03 shoulder-widths through the follow-through (held and steady). That is a steadier finish than at least 90% of 458 tour swings.

LAUNCH MONITOR SHOT
needs_review: no (read off a photo of the launch monitor's screen by OCR, parse confidence 0.98)
carry: 126 yards
total: 131 yards
ball speed: 98 mph
club head speed: 76 mph
smash factor: 1.29
club path: 0.6 degrees, + = in-to-out
club face angle: -0.1 degrees, + = open
launch angle: 22.4 degrees
spin rate: 7900 rpm
shape: STRAIGHT
strike: CENTER

TWO-VIEW ALIGNMENT
no down-the-line view - nothing to align, and nothing scored from it anyway

NOTES RECORDED WHILE ANALYZING THIS SWING
- no down-the-line view in this bundle — face-on analysis only
```
