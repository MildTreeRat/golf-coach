//! Turning triggers into clips: how much to keep, and where two swings are cut apart. [M20 P4]
//!
//! [`crate::stream`] answers *did a shot happen*. This answers *what do I write to disk*, and it
//! is the half with a downstream contract: a clip is only worth cutting if the pipeline that
//! opens it can measure the swing inside. So the numbers here are not taste. They are read out of
//! `src/golf_coach/analysis/phases.py`, which decides — before any checkpoint is scored — how much
//! footage a swing needs on either side of impact.
//!
//! # Where the five seconds come from
//!
//! `phases.py::window_around` keeps `[top - lead, impact + 3*dn)`, where `dn` is the downswing's
//! own length and `lead = max(5*dn, 1.5 s)`. The worst case is the *slowest* downswing the module
//! will admit at all — `POSSIBLE_DOWNSWING_S` tops out at 0.80 s — so the most footage a swing can
//! ask for before impact is `0.80 + 5*0.80 = 4.8 s`, and after it `3 * 0.80 = 2.4 s`. (The 1.5 s
//! address floor never binds at that end of the band: `5 * 0.80` is already 4.0.)
//!
//! Those are [`MIN_LEAD_S`] and [`MIN_TRAIL_S`], and [`PRE_ROLL_S`] and [`POST_ROLL_S`] are the
//! round numbers above them. **The margin at the front is 0.15 s and that is the interesting
//! number in this module** — five seconds of pre-roll is not a comfortable choice, it is very
//! nearly the minimum, and anything that lengthens the lead (a slower downswing band, a longer
//! address floor) has to move the pre-roll with it. `tests/audio/test_trigger.py` is what fails if
//! `phases.py` moves and this does not.
//!
//! # Cutting is irreversible and analysis is not
//!
//! Every rule below resolves the same way when it is unsure: **keep more footage, not less.** The
//! ring buffer is gone the moment it is overwritten, so a lead cut too short can never be
//! recovered; a clip that turned out to hold two swings, or one that holds none, costs a pose run
//! and stays on disk for a human to look at. That asymmetry is the same one
//! [`crate::stream::MIN_TRIGGER_Z`] is set by, one step further down the pipe.
//!
//! It is why two triggers inside one post-roll become **one clip rather than two**: the second
//! clip would have to start inside the first swing's follow-through, so it could not have a lead
//! and neither clip would be measurable. One clip holding both is a case
//! `phases.py::select_swing` was built for — it picks among candidates and records the ones it did
//! not pick — and both swings are still on the disk.
//!
//! # Two swings, and the gap that decides
//!
//! Clips may overlap by up to `POST_ROLL_S - MIN_TRAIL_S`, but a clip never reaches back over the
//! strike before it: the earliest a clip may start is `previous trigger + MIN_TRAIL_S`. So the
//! later swing buys its lead out of the earlier clip's *spare* trail and never out of the earlier
//! swing itself, and no strike is ever cut into two clips and scored twice.
//!
//! That makes the gap between two swings decide three ways, and the boundaries are derived rather
//! than chosen:
//!
//! | gap between triggers | result |
//! |---|---|
//! | under [`POST_ROLL_S`] (5 s) | one clip, both strikes inside it |
//! | 5 s to `MIN_LEAD_S + MIN_TRAIL_S` (**7.25 s**) | two clips; the later is short of lead and [`Clip::is_measurable`] says so |
//! | over 7.25 s | two clips, both measurable |
//!
//! 7.25 s is therefore *the closest two swings can be and both still be measured*, and nobody
//! chose it — it is `phases.py`'s two window constants added together.
//!
//! # Samples, and whose clock they are on
//!
//! Spans here are absolute sample indices on the **microphone's** clock, because that is the clock
//! the trigger was found on. The video edge converts, and it must *measure* its offset rather than
//! assume it: M11 P10 found `video_start_seconds` was a 105-125 ms surprise on four of the stored
//! clips, and ROADMAP §M21 carries that measurement as its own task.

use crate::ring::Ring;
use crate::stream::{Trigger, PEAK_HOLD_S};

/// The longest downswing `analysis/phases.py::POSSIBLE_DOWNSWING_S` admits, in seconds.
///
/// The *upper* bound is what matters here and it is the only one that does: the pre-roll has to
/// cover the slowest swing the analysis will accept, and a fast swing simply uses less of it.
const LONGEST_DOWNSWING_S: f64 = 0.80;

/// `phases.py::_WINDOW_LEAD`, in downswing-lengths.
const WINDOW_LEAD_DOWNSWINGS: f64 = 5.0;

/// `phases.py::_WINDOW_TRAIL`, in downswing-lengths.
const WINDOW_TRAIL_DOWNSWINGS: f64 = 3.0;

/// The footage `phases.py::window_around` needs *before* impact, worst case, in seconds.
///
/// `dn + max(5*dn, 1.5)` at `dn = 0.80`, which is 4.8 — plus [`crate::stream::PEAK_HOLD_S`],
/// because the instant a trigger names is the loudest hop within 50 ms of the bar being crossed
/// and so can sit that far *after* the ball. Fifty milliseconds is not much and it is the
/// difference between a margin of 0.20 s and one of 0.15 s, which on a quantity this tight is
/// worth being explicit about rather than rounding away.
pub const MIN_LEAD_S: f64 = LONGEST_DOWNSWING_S * (1.0 + WINDOW_LEAD_DOWNSWINGS) + PEAK_HOLD_S;

/// The footage `phases.py::window_around` needs *after* impact, worst case, in seconds.
pub const MIN_TRAIL_S: f64 = LONGEST_DOWNSWING_S * WINDOW_TRAIL_DOWNSWINGS;

/// How much of the past a clip keeps.
///
/// Five seconds, which is [`MIN_LEAD_S`] rounded up — a margin of 0.15 s. See the module docs:
/// this is close to the floor rather than comfortably above it, and it is the number to revisit if
/// `phases.py`'s window constants ever move.
pub const PRE_ROLL_S: f64 = 5.0;

/// How much of the future a clip keeps, and how long a clip stays open to a second trigger.
///
/// Doing both jobs with one number is deliberate. The alternative — holding the clip open longer
/// than its post-roll so a slightly later swing can still be merged — buys a rare case and pays
/// for it in latency on *every* shot, and bounded latency is the reason the live detector exists.
/// A clip is complete, and can be written, exactly [`POST_ROLL_S`] after the last thing heard in
/// it.
pub const POST_ROLL_S: f64 = 5.0;

/// The longest clip a merge chain may produce.
///
/// Merging is unbounded on its own — a trigger every four seconds extends one clip forever — so it
/// needs a ceiling, and the ceiling is priced in pose rather than in disk. ADR-002 measured pose at
/// ~24 fps, so a nominal 10 s clip at 60 fps is ~25 s of pose per view and ~50 s for a two-view
/// swing; 30 s caps a merge chain at three times that. Beyond it the bay is producing strike-loud
/// transients every few seconds, which is a microphone or a neighbour rather than a golfer, and the
/// honest response is to stop growing the clip rather than to keep believing it.
pub const MAX_CLIP_S: f64 = 30.0;

/// How much more than the longest clip the ring buffer holds.
///
/// A clip is asked for after it is complete, so the buffer has to still contain its start: one
/// analysis window (21 ms at 48 kHz), one peak-hold (50 ms) and whatever block size the capture
/// device hands over. One second covers all three for any plausible device and costs 96 kB of
/// audio.
pub const SLACK_S: f64 = 1.0;

/// How much recent signal a session must keep buffered, in seconds.
///
/// The video ring multiplies this by its own frame rate; the audio one uses
/// [`ring_capacity_samples`].
pub const BUFFER_S: f64 = MAX_CLIP_S + SLACK_S;

/// How many samples a [`Ring`] fed by a microphone at `rate` must hold.
pub fn ring_capacity_samples(rate: u32) -> usize {
    (BUFFER_S * f64::from(rate)).ceil() as usize
}

/// A span of the stream worth writing to disk, and the strikes that caused it.
///
/// `[start, end)` in absolute samples, the same origin [`Trigger::sample`] uses, so it can be
/// handed straight to [`Ring::copy`].
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub start: usize,
    pub end: usize,
    pub rate: u32,
    /// Every trigger that fell inside this clip, in the order they were heard. More than one means
    /// two swings arrived inside one post-roll and were kept together; see the module docs.
    pub triggers: Vec<Trigger>,
}

impl Clip {
    /// Seconds of footage before the first strike.
    pub fn lead_s(&self) -> f64 {
        self.seconds(self.first_trigger() - self.start)
    }

    /// Seconds of footage after the last strike.
    pub fn trail_s(&self) -> f64 {
        self.seconds(self.end - self.last_trigger())
    }

    pub fn duration_s(&self) -> f64 {
        self.seconds(self.end - self.start)
    }

    /// Whether `phases.py::window_around` can frame the swing inside this clip.
    ///
    /// False is not "throw it away" — it is "this clip holds a swing the address-dependent
    /// checkpoints will not reach", and the caller records that rather than guessing the
    /// difference (ADR-010 §2). Motion start is what goes first, and tempo with it.
    pub fn is_measurable(&self) -> bool {
        self.lead_s() >= MIN_LEAD_S && self.trail_s() >= MIN_TRAIL_S
    }

    fn first_trigger(&self) -> usize {
        self.triggers.first().expect("a clip has a trigger").sample
    }

    fn last_trigger(&self) -> usize {
        self.triggers.last().expect("a clip has a trigger").sample
    }

    fn seconds(&self, samples: usize) -> f64 {
        samples as f64 / f64::from(self.rate)
    }
}

/// Triggers in, clip spans out.
///
/// Pairs with [`crate::stream::OnlineDetector`] and is deliberately not fused to it: the detector
/// reads a microphone, and the spans this produces are cut out of the *video* ring as well.
///
/// ```ignore
/// let triggers = detector.push(block);
/// for clip in cutter.push(&triggers, detector.position()) {
///     let audio = extract(&ring, &clip).expect("the ring is sized for this");
/// }
/// ```
pub struct Cutter {
    rate: u32,
    pre: usize,
    post: usize,
    max: usize,
    min_trail: usize,
    open: Option<Clip>,
    /// The last strike already cut into a finished clip. The next clip may not start before
    /// `guard + min_trail`, which is what keeps one swing out of two clips.
    guard: Option<usize>,
}

impl Cutter {
    pub fn new(rate: u32) -> Self {
        let samples = |seconds: f64| (seconds * f64::from(rate)).round() as usize;
        Self {
            rate,
            pre: samples(PRE_ROLL_S),
            post: samples(POST_ROLL_S),
            max: samples(MAX_CLIP_S),
            min_trail: samples(MIN_TRAIL_S),
            open: None,
            guard: None,
        }
    }

    /// Absorb the triggers from one block and return the clips that are now complete.
    ///
    /// `position` is [`crate::stream::OnlineDetector::position`] — how much of the stream has been
    /// analysed. A clip is reported on the first `push` whose position reaches its end, so a caller
    /// that keeps pushing keeps getting clips and never has to ask.
    pub fn push(&mut self, triggers: &[Trigger], position: usize) -> Vec<Clip> {
        let mut done = Vec::new();
        for trigger in triggers {
            match &mut self.open {
                // Still inside the open clip's post-roll, so this is part of it. Extend to cover
                // the new strike, up to the ceiling — `max` binds only on a chain, and when it does
                // the next trigger lands outside `end` and opens a clip of its own.
                Some(open) if trigger.sample < open.end => {
                    let wanted = open.end.max(trigger.sample + self.post);
                    open.end = wanted.min(open.start + self.max);
                    open.triggers.push(*trigger);
                }
                Some(_) => {
                    done.push(self.close());
                    self.open = Some(self.start(*trigger));
                }
                None => self.open = Some(self.start(*trigger)),
            }
        }
        if self.open.as_ref().is_some_and(|open| position >= open.end) {
            done.push(self.close());
        }
        done
    }

    /// End of stream: give up whatever is still open, truncated to what actually exists.
    ///
    /// A clip cut short here is reported rather than dropped — the strike happened, and a short
    /// trail is a thing [`Clip::is_measurable`] can describe where a missing clip is not.
    pub fn finish(&mut self, position: usize) -> Vec<Clip> {
        match self.open.take() {
            None => Vec::new(),
            Some(mut clip) => {
                clip.end = clip.end.min(position.max(clip.start));
                self.guard = Some(clip.triggers.last().expect("a clip has a trigger").sample);
                vec![clip]
            }
        }
    }

    /// Whether a clip is open and waiting for its post-roll to run out.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    fn start(&self, trigger: Trigger) -> Clip {
        // The floor: never back over the previous strike's own trail. Without a previous strike the
        // floor is the start of the stream, which is the one case a short lead is nobody's fault —
        // the session had not been recording long enough.
        let floor = self.guard.map_or(0, |guard| guard + self.min_trail);
        Clip {
            start: trigger
                .sample
                .saturating_sub(self.pre)
                .max(floor)
                .min(trigger.sample),
            end: trigger.sample + self.post,
            rate: self.rate,
            triggers: vec![trigger],
        }
    }

    fn close(&mut self) -> Clip {
        let clip = self
            .open
            .take()
            .expect("close is only called on an open clip");
        self.guard = Some(clip.triggers.last().expect("a clip has a trigger").sample);
        clip
    }
}

/// Cut the footage for a clip out of the ring it was buffered in.
///
/// A thin convenience over [`Ring::copy`] that exists to name the failure: `None` means the ring
/// was too small or the caller fell behind, never that the clip was wrong.
pub fn extract<T: Copy>(ring: &Ring<T>, clip: &Clip) -> Option<Vec<T>> {
    ring.copy(clip.start, clip.end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::OnlineDetector;
    use crate::testing::{bay, whoosh, RATE};

    fn at(seconds: f64) -> usize {
        (seconds * f64::from(RATE)) as usize
    }

    /// A trigger of the shape the detector produces, for the rule-level tests.
    ///
    /// The end-to-end tests below drive a real [`OnlineDetector`]; these feed the cutter directly,
    /// because a gap of 26 seconds is a rule to check and not 26 seconds of FFT to run.
    fn trigger(seconds: f64) -> Trigger {
        Trigger {
            sample: at(seconds),
            z: 300.0,
            confidence: 0.9,
        }
    }

    /// Everything a session would do, in one line: detect, then cut.
    fn cut(audio: &[i16], block: usize) -> Vec<Clip> {
        let mut detector = OnlineDetector::new(RATE);
        let mut cutter = Cutter::new(RATE);
        let mut clips = Vec::new();
        for chunk in audio.chunks(block) {
            let triggers = detector.push(chunk);
            clips.extend(cutter.push(&triggers, detector.position()));
        }
        clips.extend(cutter.finish(audio.len()));
        clips
    }

    #[test]
    fn a_strike_is_cut_with_the_lead_and_trail_the_pipeline_needs() {
        let clips = cut(&bay(20.0, &[10.0]), 4096);
        assert_eq!(clips.len(), 1, "got {clips:?}");
        let clip = &clips[0];
        assert!(
            clip.lead_s() >= MIN_LEAD_S,
            "lead was {:.3}s",
            clip.lead_s()
        );
        assert!(
            clip.trail_s() >= MIN_TRAIL_S,
            "trail was {:.3}s",
            clip.trail_s()
        );
        assert!(clip.is_measurable());
        assert!((clip.duration_s() - (PRE_ROLL_S + POST_ROLL_S)).abs() < 0.01);
    }

    /// A practice swing has a whoosh and no crack, so it costs nothing.
    ///
    /// The same evidence `phases.py::_struck` uses one stage later, applied one stage earlier: a
    /// swing that did not hit a ball never becomes a clip, a pose run or a row in the history.
    #[test]
    fn a_practice_swing_cuts_nothing() {
        assert!(cut(&whoosh(12.0, 6.0), 4096).is_empty());
    }

    #[test]
    fn two_swings_ten_seconds_apart_are_two_whole_clips() {
        let clips = cut(&bay(30.0, &[8.0, 18.0]), 4096);
        assert_eq!(clips.len(), 2, "got {clips:?}");
        for clip in &clips {
            assert!(clip.is_measurable(), "{clip:?} lead {:.3}s", clip.lead_s());
            assert_eq!(clip.triggers.len(), 1);
        }
        // Ten seconds apart is the gap at which the two spans meet exactly, so the detector's
        // own few milliseconds of jitter decide the sign. What must hold either way is that the
        // later clip does not reach back over the earlier strike.
        assert!(clips[1].start > clips[0].triggers[0].sample);
        assert!(
            clips[1].start + at(0.05) >= clips[0].end,
            "overlap is more than jitter"
        );
    }

    /// The whole point of the guard: no strike is ever cut into two clips and scored twice.
    #[test]
    fn a_clip_never_reaches_back_over_the_strike_before_it() {
        let mut cutter = Cutter::new(RATE);
        let mut clips = cutter.push(&[trigger(10.0)], at(10.0));
        clips.extend(cutter.push(&[trigger(16.0)], at(16.0)));
        clips.extend(cutter.finish(at(30.0)));
        assert_eq!(clips.len(), 2, "got {clips:?}");
        // The clips overlap — the later one buys its lead out of the earlier one's spare trail —
        // but the earlier strike is outside the later clip.
        assert!(clips[1].start < clips[0].end, "the overlap is the point");
        assert!(
            clips[1].start > at(10.0),
            "clip 2 must not contain strike 1"
        );
        assert!((clips[0].trail_s() - POST_ROLL_S).abs() < 0.01);
    }

    /// Between five and 7.25 seconds apart, both swings are kept and the later one says what it is
    /// short of rather than pretending.
    #[test]
    fn a_crowded_pair_is_two_clips_and_the_later_one_admits_its_short_lead() {
        let mut cutter = Cutter::new(RATE);
        let mut clips = cutter.push(&[trigger(10.0)], at(10.0));
        clips.extend(cutter.push(&[trigger(16.0)], at(16.0)));
        clips.extend(cutter.finish(at(30.0)));
        assert_eq!(clips.len(), 2);
        assert!(clips[0].is_measurable());
        assert!(
            !clips[1].is_measurable(),
            "6 s apart cannot give 4.85 s of lead"
        );
        assert!((clips[1].lead_s() - (6.0 - MIN_TRAIL_S)).abs() < 0.01);
    }

    /// Two triggers inside one post-roll are one clip, because two clips could not both have a lead
    /// and the footage between them is the thing that cannot be recovered.
    #[test]
    fn two_strikes_inside_one_post_roll_become_one_clip() {
        let mut cutter = Cutter::new(RATE);
        let mut clips = cutter.push(&[trigger(10.0), trigger(13.0)], at(13.0));
        clips.extend(cutter.finish(at(30.0)));
        assert_eq!(clips.len(), 1, "got {clips:?}");
        assert_eq!(clips[0].triggers.len(), 2);
        assert!((clips[0].duration_s() - (PRE_ROLL_S + 3.0 + POST_ROLL_S)).abs() < 0.01);
        assert!(clips[0].is_measurable(), "the first swing is still framed");
    }

    /// A merge chain stops growing at the ceiling and the next strike opens a clip of its own.
    #[test]
    fn a_merge_chain_stops_at_the_maximum_clip() {
        let mut cutter = Cutter::new(RATE);
        let mut clips = Vec::new();
        let mut t = 10.0;
        while t < 60.0 {
            clips.extend(cutter.push(&[trigger(t)], at(t)));
            t += 4.0;
        }
        clips.extend(cutter.finish(at(90.0)));
        assert!(clips.len() > 1, "the chain must be broken somewhere");
        for clip in &clips {
            assert!(
                clip.duration_s() <= MAX_CLIP_S + 0.01,
                "{:.3}s exceeds the ceiling",
                clip.duration_s()
            );
        }
    }

    /// The ring is sized so the clip is still there when it is asked for. If this ever fails,
    /// `SLACK_S` is wrong and clips are being cut from footage that has been overwritten.
    #[test]
    fn the_footage_is_still_in_the_ring_when_the_clip_is_reported() {
        let audio = bay(25.0, &[10.0, 18.0]);
        let mut detector = OnlineDetector::new(RATE);
        let mut cutter = Cutter::new(RATE);
        let mut ring: Ring<i16> = Ring::with_capacity(ring_capacity_samples(RATE));
        let mut seen = 0;
        for chunk in audio.chunks(4096) {
            ring.extend(chunk);
            let triggers = detector.push(chunk);
            for clip in cutter.push(&triggers, detector.position()) {
                let footage = extract(&ring, &clip).expect("the ring must still hold the clip");
                assert_eq!(footage.len(), clip.end - clip.start);
                assert_eq!(footage[..], audio[clip.start..clip.end]);
                seen += 1;
            }
        }
        assert_eq!(
            seen, 2,
            "both clips should have been reported while streaming"
        );
    }

    /// How the caller chops up the stream is the caller's business — the same property
    /// [`crate::stream`] asserts about the detector, carried through the cutter.
    #[test]
    fn the_block_size_does_not_change_the_clips() {
        let audio = bay(30.0, &[8.0, 18.0]);
        let whole = cut(&audio, audio.len());
        for block in [97, 1024, 4096, 48_000] {
            assert_eq!(
                cut(&audio, block),
                whole,
                "block size {block} changed the answer"
            );
        }
    }

    /// A stream that ends inside a post-roll still yields its clip, short trail and all.
    #[test]
    fn a_clip_cut_short_by_the_end_of_the_stream_is_reported_not_dropped() {
        let mut cutter = Cutter::new(RATE);
        assert!(cutter.push(&[trigger(10.0)], at(11.0)).is_empty());
        assert!(cutter.is_open());
        let clips = cutter.finish(at(11.0));
        assert_eq!(clips.len(), 1);
        assert!((clips[0].trail_s() - 1.0).abs() < 0.01);
        assert!(!clips[0].is_measurable(), "1 s of trail is not 2.4");
    }

    /// A strike near the start of a session gets whatever lead exists, and says it is short.
    #[test]
    fn a_strike_before_the_buffer_has_filled_keeps_what_there_is() {
        let mut cutter = Cutter::new(RATE);
        let mut clips = cutter.push(&[trigger(2.0)], at(2.0));
        clips.extend(cutter.push(&[], at(7.1)));
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].start, 0);
        assert!(!clips[0].is_measurable());
    }

    /// The gap that decides is `phases.py`'s own two constants added together, not a chosen one.
    ///
    /// Either side of 7.25 s, not on it: a gap of exactly `MIN_LEAD_S + MIN_TRAIL_S` puts
    /// [`Clip::lead_s`] on the wrong side of the comparison by one bit, which is a fact about f64
    /// and not about cutting. The claim worth pinning is that the boundary is *there*.
    #[test]
    fn the_closest_two_measurable_swings_can_be_is_derived() {
        assert!((MIN_LEAD_S - 4.85).abs() < 1e-9, "{MIN_LEAD_S}");
        assert!((MIN_TRAIL_S - 2.40).abs() < 1e-9, "{MIN_TRAIL_S}");
        let boundary = MIN_LEAD_S + MIN_TRAIL_S;
        for (gap, both) in [(boundary + 0.1, true), (boundary - 0.1, false)] {
            let mut cutter = Cutter::new(RATE);
            let mut clips = cutter.push(&[trigger(10.0)], at(10.0));
            clips.extend(cutter.push(&[trigger(10.0 + gap)], at(10.0 + gap)));
            clips.extend(cutter.finish(at(40.0)));
            assert_eq!(clips.len(), 2, "{gap:.2}s apart is always two clips");
            assert_eq!(
                clips.iter().all(Clip::is_measurable),
                both,
                "{gap:.2}s apart: {clips:?}"
            );
        }
    }
}
