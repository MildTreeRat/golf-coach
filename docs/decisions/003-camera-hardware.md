# ADR-003: Camera Hardware

## Status
Accepted

## Date
2026-03-16

## Context
Need cameras to record golf swings for pose estimation and club detection. Key factors: frame rate (120fps minimum for club tracking), resolution, global vs rolling shutter, ease of integration with OpenCV (UVC compliant), cost, and the ability to scale from one to two cameras (face-on + down-the-line angles).

Golf swing analysis uses two standard camera positions:
- **Down-the-line** (behind golfer, looking at target): reveals swing plane, club path, shoulder tilt
- **Face-on** (perpendicular to target line): reveals weight shift, head movement, hip sway, hand position

Both angles are needed for complete analysis, but a single camera is sufficient to validate the full pipeline end-to-end.

## Options Considered

### Option A: Standard USB Webcam (e.g., Logitech C920/C922)
- **Pros**: Cheap (~$50-80). Plug and play with OpenCV. 1080p at 30fps.
- **Cons**: 30fps is far too slow for club head tracking. Rolling shutter. No manual lens control.

### Option B: ELP/SVPRO OV4689 USB Camera (~$40-60)
- **Pros**: 120fps at 720p, 60fps at 1080p. UVC plug-and-play. Varifocal lens options (2.8-12mm). Cheapest high-speed option. Widely used in golf simulator community.
- **Cons**: Rolling shutter causes motion blur on club head. 720p at 120fps limits resolution for pose estimation.

### Option C: SVPRO IMX577 USB Camera (~$60-80)
- **Pros**: 120fps at full 1080p. 12MP sensor. 3X optical zoom. Better low-light performance (0.1 lux). UVC plug-and-play.
- **Cons**: Rolling shutter. Slightly more expensive. Less community validation than OV4689.

### Option D: ELP AR0234 Global Shutter USB Camera (~$78)
- **Pros**: Global shutter eliminates motion blur — all pixels exposed simultaneously, critical for capturing fast-moving club head without distortion. 120fps at 720p, 90fps at 1080p. Wide-angle no-distortion lens (85/101/112/126 degree options). External trigger + software trigger support for future sync capability. Mini 38x38mm form factor. UVC plug-and-play.
- **Cons**: 720p (not 1080p) at 120fps. Slightly more expensive than rolling shutter alternatives. AR0234 is 2.3MP (vs 12MP on IMX577 for stills).

### Option E: Uneekor Swing Optix (~$800+ pair)
- **Pros**: 180fps. Purpose-built for golf. Integrates with Uneekor ecosystem.
- **Cons**: Way over budget. Proprietary ecosystem — defeats the purpose of building from scratch.

## Decision
**ELP AR0234 Global Shutter USB Camera** — buy 2 units, integrate 1 initially.

The global shutter is the deciding factor. A golf club head moves at 80-120+ mph through impact. Rolling shutter cameras capture the frame line-by-line, causing the club head to appear warped or smeared. Global shutter captures the entire frame simultaneously, producing sharp images of the club at any point in the swing. This directly impacts the quality of our YOLOv8 club detection (Milestone 2).

720p at 120fps is sufficient — MediaPipe Pose works well at 720p, and for club detection we care more about temporal resolution (frames per second) than spatial resolution (pixels). The external trigger support also gives us a path to synchronize two cameras later.

**Start with one camera in down-the-line position** (more informative for swing plane and club path analysis). Add the second camera (face-on) after the single-camera pipeline is fully working.

## Shopping List

| Item | Qty | Est. Price | Notes |
|------|-----|-----------|-------|
| ELP AR0234 Global Shutter USB Camera (85° no-distortion lens) | 2 | ~$78 each / ~$156 total | Buy both now for hardware consistency. Model: ELP-USBGS2M-AR023-85 or similar |
| Tripod with adjustable height | 2 | ~$25 each / ~$50 total | Must reach hand height (~3-4 feet). Standard 1/4-20 mount. |
| USB 3.0 extension cable (active, 15ft) | 2 | ~$15 each / ~$30 total | Cameras need to reach hitting position. Active cable prevents signal loss. |
| Small Phillips screwdriver | 1 | ~$5 | For mounting camera to tripod plate |
| **Total** | | **~$240** | |

### Camera Placement Guide

**Down-the-line camera (integrate first):**
- Position: Behind golfer, aligned with target line, at hand height (~3-4 feet)
- Distance: ~7-8 feet from the ball
- Lens: 85° wide-angle should capture full swing arc
- Frame the golfer in the left-center with room above head and below feet

**Face-on camera (integrate in Milestone 6+):**
- Position: Perpendicular to target line, centered on golfer's stance, at hand height
- Distance: ~5-7 feet from golfer
- Use alignment stick on ground to ensure consistent positioning

## Consequences
- Global shutter provides the best possible input data for club head detection — avoids training YOLOv8 on distorted images.
- 720p at 120fps is the operating resolution. All pipeline components should be designed for this spec.
- Two identical cameras means no hardware mismatch when adding the second angle.
- External trigger support enables frame-synchronized dual capture in the future without software hacks.
- USB UVC compliance means OpenCV `cv2.VideoCapture()` works out of the box — no vendor SDK needed.

## Addendum (2026-06-20): Global Shutter ≠ No Motion Blur

The Decision section above overstates the global shutter's effect. To be precise:
- **Global shutter** eliminates *geometric distortion* (the rolling-shutter "jello"/skew
  on fast objects) by exposing all pixels simultaneously.
- **Motion blur** is governed by **exposure time (shutter speed)**, not shutter type. A
  global-shutter camera will still smear a ~110 mph club head at impact unless the
  exposure is short (~1/2000s or faster), which in turn requires **bright lighting**.

Implication: sharp impact-zone club frames depend on a **lighting + fast-shutter** setup,
not just the camera. This is the highest-leverage factor for YOLOv8 detection and is now
explicitly tested in the M1.5 detectability spike (ROADMAP.md) before any labeling effort.
- Buying both cameras upfront (~$156) keeps total hardware spend well under budget.

## Addendum (2026-07-02): Pose camera goes FACE-ON (3 o'clock)

**Canonical placement for the pose-estimation camera: face-on.** Using the ball's flight
direction as a clock face — **the ball flies toward 12 o'clock after impact, so the pose
camera sits at 3 o'clock** (perpendicular to the ball-flight/target line, facing the
golfer's chest). This is for a **right-handed** golfer; mirror to **9 o'clock** for a
left-handed golfer. Camera at hand/hip height, level (not tilted down), full body framed
with margin.

**Why this refines the original Decision.** The Decision section says to "start with one
camera in down-the-line position." That guidance was written for *club-path / swing-plane*
reasons, before we had any pose data. Empirically, down-the-line is the **worst** angle for
MediaPipe body tracking: the legs stack front-to-back along the camera axis and occlude each
other, and MediaPipe is trained largely on frontal footage. Two real clips of the same swing
confirmed it (see
[M1 findings — angle comparison, 2026-07-02](../M1_CAPTURE_FLOW.md#m1-findings-angle-comparison-2026-07-02)):
moving from ~5 o'clock to face-on 3 o'clock lifted **knee** confidence 0.71 → 0.88 (+24%),
lower-body 0.70 → 0.83, and overall visibility 0.78 → 0.89 — and, critically, kept the knees
confident *through the bent-over swing posture* where they previously collapsed to ~0.60.

**Reconciliation with the two-angle plan.** The two standard golf angles still both matter,
but they serve different streams:
- **Pose stream (body, MediaPipe)** → **face-on (3 o'clock)**. This is the primary/first
  camera now, superseding the "down-the-line first" note for the pose use case.
- **Club/ball stream (YOLOv8, swing plane) → down-the-line**. Still the right angle for M2
  club-path work, but MediaPipe body tracking there is poor — don't judge pose quality off
  the down-the-line camera.

When the second camera is added, the down-the-line unit is for club detection, not pose.

## Addendum (2026-07-02b): Two cameras (not three), stream assignment & the spine caveat

**We need two cameras, not three.** It's tempting to think of three roles — body landmarks,
spine/hip, and club head — as three cameras. But the spine/hip role and the club role share
the **same viewpoint** (down-the-line), and one global-shutter camera there runs both
MediaPipe (body) and YOLOv8 (club) on the same frames. So the setup collapses to two angles.
The original two-camera buy in the Shopping List above already covers this; nothing new to
purchase.

**Stream → camera assignment:**

| Camera | Streams it owns |
|--------|-----------------|
| **Face-on (3 o'clock)** — primary, integrate first | body landmarks: knees, hands, weight shift, sway, head movement, lateral tilt; **tempo** |
| **Down-the-line (~6 o'clock)** — second | forward **spine tilt**, swing plane, shoulder/hip turn projected into this view; **club/ball detection** (YOLOv8) |

**Spine-angle caveat (why the assignment matters).** We measured estimated spine tilt
(shoulder-midpoint → hip-midpoint vs. vertical) on two real clips of the same swing:
- ~5 o'clock view: **~37°** at address — a believable golf posture.
- Face-on view: **~2°** at address in the image — looks dead vertical.

The 2° is a **projection artifact, not a real posture**: forward spine tilt happens along the
camera's depth axis, which a face-on camera looks straight down, so it foreshortens to nearly
zero. A depth-aware (MediaPipe `z`) estimate put the face-on clip at ~66–71° — if anything
*more* bent — but `z` is unreliable, so treat it as directional only. **Rule: measure forward
spine tilt from down-the-line, never from face-on.** This is *not* a reason to move the pose
camera off face-on (face-on still wins decisively for landmark tracking, per the 2026-07-02
addendum) — it's the reason the *spine/plane* stream is assigned to the down-the-line camera.

**Trustworthy 3D spine/hip angles** (real forward tilt, hip rotation, X-factor) require fusing
both views by triangulation, which requires the cameras to be **synchronized** — planned in
[ADR-011](011-camera-synchronization.md).
