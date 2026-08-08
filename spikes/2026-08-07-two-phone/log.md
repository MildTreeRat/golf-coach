# Bay session log — M7 Phase 0 spike

Fill this in **at the bay**, one row per clip, as you go. Uploads are content-addressed
(`face_on.<sha>.mov` under `data/processed/sessions/<session>/<swing>/`, see
`storage/manifest.py:76`), so this table is the only thing tying a file on disk back to what was
actually in front of the lens. Reconstructing it afterwards from timestamps is miserable.

**Session**: `___________`  **Date**: `___________`  **Bay / sim**: `___________`

Phones:

| Phone | Owner | Model / iOS | Role | Record Video setting | Format setting |
|---|---|---|---|---|---|
| A |  |  | face-on |  |  |
| B |  |  | down-the-line |  |  |

## Clips

| Set | Swing | Angle | Club | Mode | swing_id echoed by upload page | Notes |
|---|---|---|---|---|---|---|
| A | 1 | face_on | 7i | 60 |  |  |
| A | 1 | down_the_line | 7i | 60 |  |  |
| A | 2 | face_on | 7i | 60 |  |  |
| A | 2 | down_the_line | 7i | 60 |  |  |
| A | 3 | face_on | 7i | 60 |  |  |
| A | 3 | down_the_line | 7i | 60 |  |  |
| A | 4 | face_on | Dr | 60 |  |  |
| A | 4 | down_the_line | Dr | 60 |  |  |
| A | 5 | face_on | Dr | 60 |  |  |
| A | 5 | down_the_line | Dr | 60 |  |  |
| A | 6 | face_on | Dr | 60 |  |  |
| A | 6 | down_the_line | Dr | 60 |  |  |
| B | 7 | face_on |  | 60 (**Most Compatible / H.264**) |  | codec control |
| C | 8 | face_on |  | 60 |  | mixed-rate pair |
| C | 8 | down_the_line |  | **240 slo-mo** |  | mixed-rate pair, ~3 s only |
| D | 9 | face_on |  | 60 |  | tempo-invariance pair — **both phones face-on** |
| D | 9 | face_on (phone B) |  | **240 slo-mo** |  | tempo-invariance pair |
| E | — | — | — | 60 |  | phone A films a ms stopwatch, ~5 s |
| E | — | — | — | 240 |  | phone A stopwatch |
| E | — | — | — | 60 |  | phone B stopwatch |
| E | — | — | — | 240 |  | phone B stopwatch |
| F | 10 | face_on |  | 60 |  | full practice swing first |
| F | 10 | down_the_line |  | 60 |  | full practice swing first |
| F | 11 | face_on |  | 60 |  | walk into the shot |
| F | 11 | down_the_line |  | 60 |  | walk into the shot |

Notes column: anything that could explain a bad result later — camera moved / panned, mis-hit,
golfer stepped out and re-addressed, someone walked through frame, ball not visible.

## Cable control

One set-A clip pulled off the phone **by cable** as well as through the upload server. This is the
only way to tell "OpenCV cannot read HEVC" apart from "Safari transcoded on upload", and the two
have completely different remedies.

- Clip: `___________`
- Cable copy path: `___________`
- Uploaded copy path: `___________`
- Same sha256? `___________`

## Shot screens (set G, secondary)

One `SHOT DATA` photo per set-A swing, uploaded with role `shot_screen`. Free evidence for the
ADR-014 OCR path; not part of any Phase 0 verdict.

## Anything that went wrong

Write it down even if it seems irrelevant — network, upload failures, phone storage, battery,
someone's phone auto-locking mid-swing.
