//! The tape reader end to end, on synthetic photos with a known truth: the M46 plan's P13 gate.
//! [M46 P13]
//!
//! No label is hit before P29 (P3 finding 10), so the whole reader, the finder (P9), the
//! orientation (P10), the mark (P11) and the diff (P12), is gated here on drawn photos before the
//! app is built on it. Each label type goes through four drills, one a hand and a way up crossed
//! with square and 20 to 30° off square. A drill strikes one label nine times and photographs it
//! after each strike, each photo from its own pose, as a hand-held phone takes them. One photo is
//! skipped, so the next holds two new marks. One strike lands over an older mark, one runs off the
//! label's bottom edge, and one misses the tape altogether. Then a fresh label is stuck on and
//! struck once. Every photo is read against the crop of the last photo whose new marks were
//! accounted for, as the phone reads it against `last/<club>.rgb` (P12 finding 9), and in each drill
//! one reference is moved as if the last photo's fit had placed it a millimetre off.
//!
//! **The gate is C24's.** A reading is **wrong** when one of its marks lies more than [`WRONG_MM`]
//! from the scene's own on either axis. It is also wrong when it reads a mark that is not new or
//! misses one that is, when its sides are not the scene's (a guessed sign, rule 6), or when it
//! calls a used label new or a new one used, since that restarts or carries on a label's strike
//! count (C8's `n_on_label`). One wrong reading fails the gate. Every photo's outcome is also held
//! to what its scene calls for, so a reader that starts withholding fails here rather than passing
//! with nothing read.
//!
//! **The record**: coverage, P50 and P95 a label type, printed with `--nocapture` for the plan's
//! finding only. A synthetic error is not the reader's real one, so it sets neither
//! `TAPE_ERROR_P95_MM` nor C9's floor, and `TAPE_READER_VERSION` stays where P5 put it. Those are
//! P29's, from real strikes.
//!
//! The four drills of a label type run on their own threads, since a debug build draws a photo in
//! about half a second and diffs one in about two (P12 finding 8).

mod tape_scene;

use std::thread;

use impact::tape::{
    diff_marks, read_tape, LabelType, TapeOutcome, TapeReading, TapeRefusal, STRIKEPRO_DRIVER,
    STRIKEPRO_HYBRID, STRIKEPRO_IRON,
};
use impact::Frame;
use tape_scene::{
    bottom_at, brief, framed, inside, keep, moved, ring_radius_mm, truth, Camera, Hand, Mark,
    Scene, PHOTO_HEIGHT, PHOTO_WIDTH, SHAFT_FROM_MM,
};

/// C24's "wrong", provisional: a reading more than this far from the truth on either axis, in
/// millimetres. P29 sets the real one with the user, against real strikes.
const WRONG_MM: f64 = 5.0;

/// The least gap between two marks' edges, millimetres, so that each is its own spot: P12's 12.6 mm
/// between 5 mm marks (P12 finding 8).
const MARK_GAP_MM: f64 = 2.6;

/// How far inside the outline a whole mark's edge stays, millimetres, so that it is not
/// `mark_clipped` (P11 finding 7).
const WHOLE_INSIDE_MM: f64 = 1.5;

/// How far a whole mark's edge stays from the red ring's line, millimetres. A mark whose edge ran
/// along it, 0.03 mm off, read 0.18 mm off (P12 finding 6), which is no wrong reading but is the
/// stroke and not the reader, and P12 asked the set to keep clear of it.
const RING_CLEAR_MM: f64 = 0.75;

/// How far a fresh label's first mark stands from every old mark's place, millimetres, in the
/// printed frame: twice the diff's 4 mm, inside which a lone spot is the same label struck again
/// and needs a pick (P12 finding 2).
const FRESH_CLEAR_MM: f64 = 8.0;

/// How much shaft every photo shows, millimetres up the hosel's line from where it begins: above
/// the reader's 25 mm least (P10 finding 3), and short of the 32 mm P10 framed by default.
const SHAFT_IN_FRAME_MM: f64 = 30.0;

/// How far inside the photo the label's box stays, pixels.
const LABEL_IN_FRAME_PX: f64 = 10.0;

/// What happens to the tape at one strike.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Strike {
    /// A whole mark, clear of every other: read.
    Whole(Mark),
    /// A mark over part of an older one: the golfer's pick.
    Over(Mark),
    /// A mark the label's edge cuts: `mark_clipped`.
    Clipped(Mark),
    /// The ball missed the tape and left nothing: `no_new_mark`, which the golfer declares (C6).
    OffTape,
}

impl Strike {
    fn mark(self) -> Option<Mark> {
        match self {
            Strike::Whole(mark) | Strike::Over(mark) | Strike::Clipped(mark) => Some(mark),
            Strike::OffTape => None,
        }
    }
}

/// One strike, and whether a photo follows it.
#[derive(Debug, Clone, Copy)]
struct Step {
    /// A fresh label is stuck on before the strike.
    fresh_label: bool,
    strike: Strike,
    /// `false` is the photo the golfer missed, so the next one holds two new marks (C10).
    photo: bool,
}

fn mark(centre_mm: [f64; 2], radius_mm: f64) -> Mark {
    Mark {
        centre_mm,
        radius_mm,
    }
}

/// Where each label type is struck, in the printed frame, with each mark's radius. The marks are
/// 4.5 to 5.5 mm in radius, above the 4.5 mm P11 found a whole mark needs (P11 finding 6), because
/// no real mark's size is known before P29.
struct Places {
    /// Seven whole marks, in the order they are struck.
    whole: [([f64; 2], f64); 7],
    /// A mark half over the first whole one.
    over: ([f64; 2], f64),
    /// Its centre's `x`: the mark is drawn 1 mm in from the bottom edge there, clear of the bottom
    /// band the finder compares the logo with (P9 finding 4).
    clipped_x: f64,
    /// The fresh label's first mark.
    fresh: ([f64; 2], f64),
}

fn places(label: &LabelType) -> Places {
    match label.id {
        "strikepro_driver" => Places {
            whole: [
                ([-26.0, 4.0], 5.0),
                ([-13.0, -6.0], 4.5),
                ([0.0, 4.0], 5.5),
                ([13.0, -6.0], 5.0),
                ([26.0, 4.0], 5.0),
                ([-13.0, 12.0], 4.5),
                ([13.0, 12.0], 5.0),
            ],
            over: ([-30.5, 4.0], 5.0),
            clipped_x: 17.0,
            fresh: ([0.0, -10.0], 5.0),
        },
        "strikepro_hybrid" => Places {
            whole: [
                ([-22.0, 2.0], 5.0),
                ([-10.0, -6.0], 4.5),
                ([2.0, 3.0], 4.5),
                ([14.0, -6.0], 5.0),
                ([26.0, 1.0], 4.5),
                ([-10.0, 7.0], 4.5),
                ([14.0, 7.0], 4.5),
            ],
            over: ([-26.5, 2.0], 5.0),
            clipped_x: -20.0,
            fresh: ([0.0, -8.0], 5.0),
        },
        _ => Places {
            whole: [
                ([-24.0, 3.0], 5.0),
                ([-11.0, -5.0], 4.5),
                ([1.0, 3.0], 5.0),
                ([12.0, -5.0], 5.0),
                ([25.0, 2.0], 4.5),
                ([-12.0, 9.0], 4.5),
                ([12.0, 10.0], 5.0),
            ],
            over: ([-28.5, 3.0], 5.0),
            clipped_x: 27.0,
            fresh: ([3.0, -7.0], 5.0),
        },
    }
}

/// A label's drill: nine strikes on one label, ten photos, then a fresh label struck once.
fn steps(label: &LabelType) -> Vec<Step> {
    let p = places(label);
    let whole = |i: usize| Strike::Whole(mark(p.whole[i].0, p.whole[i].1));
    let step = |strike| Step {
        fresh_label: false,
        strike,
        photo: true,
    };
    vec![
        step(whole(0)),
        step(whole(1)),
        Step {
            photo: false,
            ..step(whole(2))
        },
        step(whole(3)),
        step(Strike::OffTape),
        step(Strike::Over(mark(p.over.0, p.over.1))),
        step(whole(4)),
        step(Strike::Clipped(mark(
            [p.clipped_x, bottom_at(label, p.clipped_x) + 1.0],
            5.0,
        ))),
        step(whole(5)),
        step(whole(6)),
        Step {
            fresh_label: true,
            ..step(Strike::Whole(mark(p.fresh.0, p.fresh.1)))
        },
    ]
}

/// A pose: the camera's pitch, yaw and roll, and its distance from the point it is aimed at,
/// millimetres. Every photo of a drill has its own, as a hand-held phone's are.
type Pose = ([f64; 3], f64);

/// Face-on, give or take the 10° a hand holds a phone to.
const SQUARE: [Pose; 10] = [
    ([0.0, 0.0, 0.0], 200.0),
    ([4.0, -5.0, 3.0], 195.0),
    ([-6.0, 3.0, -4.0], 205.0),
    ([5.0, 6.0, 6.0], 200.0),
    ([-3.0, -6.0, 2.0], 210.0),
    ([7.0, 2.0, -7.0], 190.0),
    ([-5.0, -4.0, 5.0], 200.0),
    ([3.0, 7.0, -3.0], 205.0),
    ([-7.0, 5.0, 4.0], 195.0),
    ([6.0, -3.0, -5.0], 200.0),
];

/// 20 to 30° off square, held level within 20°, and once upside down.
const TILTED: [Pose; 10] = [
    ([20.0, 10.0, 5.0], 200.0),
    ([-15.0, 20.0, -8.0], 200.0),
    ([25.0, -12.0, 10.0], 200.0),
    ([10.0, 26.0, -15.0], 200.0),
    ([-22.0, -8.0, 180.0], 200.0),
    ([18.0, -18.0, 0.0], 200.0),
    ([-12.0, -22.0, -10.0], 200.0),
    ([26.0, 6.0, -12.0], 200.0),
    ([-20.0, 15.0, -5.0], 200.0),
    ([12.0, -24.0, 15.0], 200.0),
];

/// One drill: the club's hand, which way up its first label is stuck, the poses, whether the
/// fresh label is stuck the other way up, and the photo whose reference is moved, and by how many
/// crop pixels (a tenth of a millimetre each).
///
/// **No moved reference holds the cut mark**, except the one the fresh label is read against. A
/// reference moved toward the edge that cut a mark pushes that mark further past the crop's fixed
/// interior. Its part inside can then fall under `MARK_MIN_AREA_MM2`, so the reference holds no spot
/// there. The same mark is a spot in the next photo, has not darkened, and is `Unclear`, so the
/// photo needs a pick. A move one way would pass and the other fail, and the set should not hang on
/// which (P13's finding).
struct Drill {
    name: &'static str,
    hand: Hand,
    logo_up: bool,
    poses: &'static [Pose; 10],
    fresh_turned: bool,
    moved_photo: usize,
    moved_px: [i64; 2],
}

const DRILLS: [Drill; 4] = [
    Drill {
        name: "square right logo-up",
        hand: Hand::Right,
        logo_up: true,
        poses: &SQUARE,
        fresh_turned: false,
        moved_photo: 5,
        moved_px: [8, -5],
    },
    Drill {
        name: "square left logo-down",
        hand: Hand::Left,
        logo_up: false,
        poses: &SQUARE,
        fresh_turned: true,
        moved_photo: 2,
        moved_px: [-12, 6],
    },
    Drill {
        name: "tilted left logo-up",
        hand: Hand::Left,
        logo_up: true,
        poses: &TILTED,
        fresh_turned: true,
        moved_photo: 5,
        moved_px: [5, 11],
    },
    Drill {
        name: "tilted right logo-down",
        hand: Hand::Right,
        logo_up: false,
        poses: &TILTED,
        fresh_turned: false,
        moved_photo: 9,
        moved_px: [-9, -9],
    },
];

/// The photo `photo` of a drill: its label on the club, the marks on it, from that photo's pose,
/// aimed so that what the photo must hold, the label's box and the shaft's first
/// [`SHAFT_IN_FRAME_MM`], sits in the photo's middle. Perspective makes the plane's middle of those
/// a poor aim off square, so the aim is moved twice by what the photo shows.
fn scene(
    label: &'static LabelType,
    drill: &Drill,
    photo: usize,
    logo_up: bool,
    marks: &[Mark],
) -> Scene {
    let ([pitch, yaw, roll], distance) = drill.poses[photo];
    let mut scene = framed(label, drill.hand, logo_up, [pitch, yaw, roll], distance);
    let shaft_top = scene
        .hosel_mm(SHAFT_FROM_MM + SHAFT_IN_FRAME_MM)
        .expect("a head");
    let points: Vec<[f64; 2]> = label
        .box_corners_mm()
        .into_iter()
        .map(|corner| scene.plane_of(corner))
        .chain([shaft_top])
        .collect();
    let middle = |points: &mut dyn Iterator<Item = [f64; 2]>| {
        let (low, high) = points.fold(
            ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
            |(low, high), [x, y]| {
                (
                    [low[0].min(x), low[1].min(y)],
                    [high[0].max(x), high[1].max(y)],
                )
            },
        );
        [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0]
    };
    let mut aim = middle(&mut points.iter().copied());
    for _ in 0..2 {
        let camera = Camera::aimed(pitch, yaw, roll, distance, aim);
        let shown = middle(&mut points.iter().map(|&p| camera.project(p)));
        let centre = [PHOTO_WIDTH as f64 / 2.0, PHOTO_HEIGHT as f64 / 2.0];
        let (to, from) = (camera.unproject(shown), camera.unproject(centre));
        aim = [aim[0] + to[0] - from[0], aim[1] + to[1] - from[1]];
    }
    scene.camera = Camera::aimed(pitch, yaw, roll, distance, aim);
    scene.marks = marks.to_vec();
    scene
}

/// What a photo's scene calls for.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    Read { marks: usize, new_label: bool },
    Pick,
    Refused(TapeRefusal),
}

fn kind(outcome: &TapeOutcome) -> Expect {
    match outcome {
        TapeOutcome::Read(reading) => Expect::Read {
            marks: reading.marks.len(),
            new_label: reading.new_label,
        },
        TapeOutcome::NeedsPick(_) => Expect::Pick,
        TapeOutcome::NotRead { reason, .. } => Expect::Refused(*reason),
    }
}

/// What the reader owes a photo, from the strikes since the reference it is read against and
/// whether that reference was of this label. The reader's rules (P11, P12) as the scene sees them.
fn expect(pending: &[Strike], fresh: bool) -> Expect {
    let marks: Vec<Strike> = pending
        .iter()
        .copied()
        .filter(|s| s.mark().is_some())
        .collect();
    if marks.is_empty() {
        Expect::Refused(TapeRefusal::NoNewMark)
    } else if marks.iter().any(|s| matches!(s, Strike::Clipped(_))) {
        Expect::Refused(TapeRefusal::MarkClipped)
    } else if marks.iter().any(|s| matches!(s, Strike::Over(_))) || marks.len() > 2 {
        Expect::Pick
    } else if fresh && marks.len() == 2 {
        // Which of two marks on a new label came first is the golfer's to say (P2 finding 3).
        Expect::Pick
    } else {
        Expect::Read {
            marks: marks.len(),
            new_label: fresh,
        }
    }
}

/// How each strike ended, for the record.
#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    photos: usize,
    strikes: usize,
    read: usize,
    picked: usize,
    no_new_mark: usize,
    mark_clipped: usize,
    /// Refused for any other reason, which leaves its strikes waiting for a retake.
    refused_else: usize,
    wrong: usize,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.photos += other.photos;
        self.strikes += other.strikes;
        self.read += other.read;
        self.picked += other.picked;
        self.no_new_mark += other.no_new_mark;
        self.mark_clipped += other.mark_clipped;
        self.refused_else += other.refused_else;
        self.wrong += other.wrong;
    }
}

/// One drill's run: its tally, each read mark's error in millimetres, and every failure, named.
struct Run {
    tally: Tally,
    errors_mm: Vec<f64>,
    failures: Vec<String>,
}

/// Why a reading is wrong by C24 (the module doc), or nothing; and each read mark's distance from
/// the new mark it was matched to.
fn judge(
    reading: &TapeReading,
    scene: &Scene,
    new: &[Mark],
    fresh: bool,
) -> (Vec<String>, Vec<f64>) {
    let mut wrong = Vec::new();
    if reading.orientation != truth(scene) {
        wrong.push(format!(
            "sides {:?} where the club lies {:?}",
            reading.orientation,
            truth(scene)
        ));
    }
    if reading.new_label != fresh {
        wrong.push(format!(
            "new_label {} on a label that is {}",
            reading.new_label,
            if fresh { "new" } else { "used" }
        ));
    }
    if reading.marks.len() != new.len() {
        wrong.push(format!(
            "{} marks read where {} are new",
            reading.marks.len(),
            new.len()
        ));
    }
    let mut left: Vec<[f64; 2]> = new
        .iter()
        .map(|m| truth(scene).to_reading_mm(m.centre_mm))
        .collect();
    let mut errors = Vec::new();
    for read in &reading.marks {
        let distance = |[h, v]: [f64; 2]| (read.offset_h_mm - h).hypot(read.offset_v_mm - v);
        let Some(nearest) =
            (0..left.len()).min_by(|&a, &b| distance(left[a]).total_cmp(&distance(left[b])))
        else {
            wrong.push(format!("{read:?} is no new mark"));
            continue;
        };
        let [h, v] = left.remove(nearest);
        let (dh, dv) = (read.offset_h_mm - h, read.offset_v_mm - v);
        if dh.abs() > WRONG_MM || dv.abs() > WRONG_MM {
            wrong.push(format!(
                "a mark read at [{:.2}, {:.2}] where the nearest new one is [{h:.2}, {v:.2}]",
                read.offset_h_mm, read.offset_v_mm
            ));
        }
        errors.push(dh.hypot(dv));
    }
    (wrong, errors)
}

/// What the diff made of a photo, for a failure's message: its alignment, and each spot's place,
/// area and holding. Empty with no reference or no label.
fn verdict(outcome: &TapeOutcome, previous: Option<&Frame>, label: &LabelType) -> String {
    let (Some(found), Some(previous)) = (outcome.found(), previous) else {
        return String::new();
    };
    match diff_marks(&found.crop, previous, label) {
        Err(miss) => format!("; diff: {miss:?}"),
        Ok(change) => {
            let spots: Vec<String> = change
                .spots
                .iter()
                .map(|(spot, holds)| {
                    format!(
                        "{holds:?} at [{:.1}, {:.1}] {:.0} mm²",
                        spot.centre_mm[0], spot.centre_mm[1], spot.area_mm2
                    )
                })
                .collect();
            let gone = change.old.iter().filter(|(_, here)| !here).count();
            format!(
                "; diff: shift {:?} px, residual {:.4}, share {:.3}, {gone} old marks gone, spots {}",
                change.shift_px,
                change.residual,
                change.match_share,
                spots.join(", ")
            )
        }
    }
}

/// Runs one drill of `label` through the reader, photo by photo.
fn run_drill(label: &'static LabelType, drill: &Drill) -> Run {
    let mut run = Run {
        tally: Tally::default(),
        errors_mm: Vec::new(),
        failures: Vec::new(),
    };
    let mut logo_up = drill.logo_up;
    let mut on_label: Vec<Mark> = Vec::new();
    // The strikes since the reference was taken, and whether the label has changed since.
    let mut pending: Vec<Strike> = Vec::new();
    let mut fresh = true;
    let mut reference: Option<Frame> = None;
    let mut photo = 0;
    for step in steps(label) {
        if step.fresh_label {
            assert!(
                pending.iter().all(|s| s.mark().is_none()),
                "{}: a label taken off with a strike unaccounted for",
                drill.name
            );
            on_label.clear();
            fresh = true;
            if drill.fresh_turned {
                logo_up = !logo_up;
            }
        }
        on_label.extend(step.strike.mark());
        pending.push(step.strike);
        if !step.photo {
            continue;
        }
        let scene = scene(label, drill, photo, logo_up, &on_label);
        let name = format!("{} {} photo {photo}", label.id, drill.name);
        let image = scene.draw();
        keep(&image, &name);
        let previous = reference.as_ref().map(|crop| {
            if photo == drill.moved_photo {
                moved(crop, drill.moved_px)
            } else {
                crop.clone()
            }
        });
        let outcome = read_tape(&image, previous.as_ref(), label, Some(drill.hand));
        let expected = expect(&pending, fresh);
        let new: Vec<Mark> = pending.iter().filter_map(|s| s.mark()).collect();
        run.tally.photos += 1;
        // The gate: C24's wrong, whatever was expected.
        if let TapeOutcome::Read(reading) = &outcome {
            let (wrong, errors) = judge(reading, &scene, &new, fresh);
            if !wrong.is_empty() {
                run.tally.wrong += 1;
                run.failures.push(format!(
                    "{name}: WRONG: {} ({})",
                    wrong.join("; "),
                    brief(&outcome)
                ));
            }
            run.errors_mm.extend(errors);
        }
        let got = kind(&outcome);
        if got != expected {
            run.failures.push(format!(
                "{name}: {got:?} where the scene calls for {expected:?} ({}){}",
                brief(&outcome),
                verdict(&outcome, previous.as_ref(), label)
            ));
        }
        // How the pending strikes end, and whether the reference moves on (P12 finding 3: on a
        // reading, a pick and a clipped mark, whose new marks are all accounted for).
        let accounted = pending.len();
        let advance = match got {
            Expect::Read { .. } => {
                run.tally.read += accounted;
                true
            }
            Expect::Pick => {
                run.tally.picked += accounted;
                true
            }
            Expect::Refused(TapeRefusal::MarkClipped) => {
                run.tally.mark_clipped += accounted;
                true
            }
            Expect::Refused(TapeRefusal::NoNewMark) => {
                // The golfer declares the strike off the tape (C6), and the reference stays.
                run.tally.no_new_mark += accounted;
                pending.clear();
                false
            }
            Expect::Refused(_) => {
                run.tally.refused_else += accounted;
                false
            }
        };
        if advance {
            run.tally.strikes += accounted;
            reference = outcome.found().map(|found| found.crop.frame.clone());
            pending.clear();
            fresh = false;
        } else if matches!(got, Expect::Refused(TapeRefusal::NoNewMark)) {
            run.tally.strikes += accounted;
        }
        photo += 1;
    }
    if !pending.is_empty() {
        run.failures.push(format!(
            "{} {}: {} strikes never accounted for",
            label.id,
            drill.name,
            pending.len()
        ));
    }
    run
}

/// The `p`th quantile of `sorted`, by nearest rank.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// One line of the record: what became of the strikes, and the read marks' error.
fn record(name: &str, tally: &Tally, errors: &[f64]) -> String {
    let mut sorted = errors.to_vec();
    sorted.sort_by(f64::total_cmp);
    format!(
        "{name}: {} photos, {} strikes: {} read, {} picked, {} no_new_mark, {} mark_clipped, {} \
         refused otherwise; {} wrong; error P50 {:.3} mm, P95 {:.3} mm, worst {:.3} mm over {} marks",
        tally.photos,
        tally.strikes,
        tally.read,
        tally.picked,
        tally.no_new_mark,
        tally.mark_clipped,
        tally.refused_else,
        tally.wrong,
        quantile(&sorted, 0.5),
        quantile(&sorted, 0.95),
        sorted.last().copied().unwrap_or(f64::NAN),
        sorted.len()
    )
}

/// Runs every drill of a label type side by side, prints its record, and holds the gate.
fn gate(label: &'static LabelType) {
    let runs: Vec<Run> = thread::scope(|s| {
        let handles: Vec<_> = DRILLS
            .iter()
            .map(|drill| s.spawn(move || run_drill(label, drill)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut tally = Tally::default();
    let mut errors: Vec<f64> = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    // Printed with `--nocapture`, for the plan's finding, the label type's line last.
    let mut lines = Vec::new();
    for (drill, run) in DRILLS.iter().zip(&runs) {
        lines.push(record(
            &format!("  {} {}", label.id, drill.name),
            &run.tally,
            &run.errors_mm,
        ));
        tally.add(&run.tally);
        errors.extend(&run.errors_mm);
        failures.extend(run.failures.iter().cloned());
    }
    lines.push(record(label.id, &tally, &errors));
    eprintln!("{}", lines.join("\n"));
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    assert_eq!(tally.wrong, 0);
}

#[test]
fn the_driver_label_is_read_end_to_end_with_no_wrong_reading() {
    gate(&STRIKEPRO_DRIVER);
}

#[test]
fn the_hybrid_label_is_read_end_to_end_with_no_wrong_reading() {
    gate(&STRIKEPRO_HYBRID);
}

#[test]
fn the_iron_label_is_read_end_to_end_with_no_wrong_reading() {
    gate(&STRIKEPRO_IRON);
}

// --- The set is what it says ----------------------------------------------------------------------

/// The distance from a point to a polygon's edge, millimetres.
fn to_outline(outline: &[[f64; 2]], [x, y]: [f64; 2]) -> f64 {
    (0..outline.len())
        .map(|i| {
            let ([x0, y0], [x1, y1]) = (outline[i], outline[(i + 1) % outline.len()]);
            let (dx, dy) = (x1 - x0, y1 - y0);
            let t = (((x - x0) * dx + (y - y0) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (x - x0 - t * dx).hypot(y - y0 - t * dy)
        })
        .fold(f64::INFINITY, f64::min)
}

fn gap(a: &Mark, b: &Mark) -> f64 {
    let [ax, ay] = a.centre_mm;
    let [bx, by] = b.centre_mm;
    (ax - bx).hypot(ay - by) - a.radius_mm - b.radius_mm
}

/// Every strike lands as its kind says, on every label type, with the drill's skipped photo, its
/// overlap and its clip where the section asks, so that a photo's expectation is the scene's and not
/// the reader's. No photo is drawn.
#[test]
fn every_strike_lands_as_its_kind_says() {
    for label in [&STRIKEPRO_DRIVER, &STRIKEPRO_HYBRID, &STRIKEPRO_IRON] {
        let outline = label.outline_mm;
        let ring = ring_radius_mm(label);
        let all = steps(label);
        let first: Vec<Strike> = all
            .iter()
            .take_while(|s| !s.fresh_label)
            .map(|s| s.strike)
            .collect();
        let marks: Vec<Mark> = first.iter().filter_map(|s| s.mark()).collect();
        assert!((6..=10).contains(&marks.len()), "{}", label.id);
        for (i, strike) in first.iter().enumerate() {
            let Some(m) = strike.mark() else { continue };
            let touching: Vec<Mark> = marks
                .iter()
                .copied()
                .filter(|o| *o != m && gap(&m, o) < MARK_GAP_MM)
                .collect();
            let edge = to_outline(outline, m.centre_mm);
            let is_inside = inside(outline, m.centre_mm);
            let from_cross = m.centre_mm[0].hypot(m.centre_mm[1]);
            match strike {
                Strike::Whole(_) => {
                    assert!(
                        is_inside && edge >= m.radius_mm + WHOLE_INSIDE_MM,
                        "{} {m:?}: {edge:.2} mm from the edge",
                        label.id
                    );
                    let ring_gap = ((from_cross - ring).abs() - m.radius_mm).abs();
                    assert!(
                        ring_gap >= RING_CLEAR_MM,
                        "{} {m:?}: {ring_gap:.2} mm from the ring",
                        label.id
                    );
                    // A whole mark touches only a later mark struck over it.
                    for o in &touching {
                        assert!(
                            first[i + 1..].contains(&Strike::Over(*o)),
                            "{} {m:?} runs into {o:?}",
                            label.id
                        );
                    }
                }
                Strike::Over(_) => {
                    assert!(
                        is_inside && edge >= m.radius_mm + WHOLE_INSIDE_MM,
                        "{} {m:?}",
                        label.id
                    );
                    assert_eq!(touching.len(), 1, "{} {m:?}: {touching:?}", label.id);
                    assert!(
                        gap(&m, &touching[0]) < -m.radius_mm,
                        "{} {m:?}: not half over",
                        label.id
                    );
                    assert!(
                        first[..i].contains(&Strike::Whole(touching[0])),
                        "{}",
                        label.id
                    );
                }
                Strike::Clipped(_) => {
                    assert!(is_inside && edge < m.radius_mm - 2.0, "{} {m:?}", label.id);
                    assert!(touching.is_empty(), "{} {m:?}: {touching:?}", label.id);
                }
                Strike::OffTape => unreachable!(),
            }
        }
        // The fresh label's mark: whole, clear of the ring, and clear of every old mark's place.
        let fresh = all
            .iter()
            .find(|s| s.fresh_label)
            .and_then(|s| s.strike.mark())
            .unwrap();
        let edge = to_outline(outline, fresh.centre_mm);
        assert!(
            inside(outline, fresh.centre_mm) && edge >= fresh.radius_mm + WHOLE_INSIDE_MM,
            "{} {fresh:?}",
            label.id
        );
        let from_cross = fresh.centre_mm[0].hypot(fresh.centre_mm[1]);
        assert!(
            ((from_cross - ring).abs() - fresh.radius_mm).abs() >= RING_CLEAR_MM,
            "{} {fresh:?}",
            label.id
        );
        for old in &marks {
            let [x, y] = old.centre_mm;
            let apart = (fresh.centre_mm[0] - x).hypot(fresh.centre_mm[1] - y);
            assert!(apart >= FRESH_CLEAR_MM, "{} {fresh:?} on {old:?}", label.id);
        }
        // Played through as the reader should answer: one photo holds two new marks, one an
        // overlap, one a clipped mark and one nothing new, and every other one mark.
        let mut pending = Vec::new();
        let mut fresh_label = true;
        let mut answers = Vec::new();
        for step in &all {
            fresh_label |= step.fresh_label;
            pending.push(step.strike);
            if step.photo {
                let answer = expect(&pending, fresh_label);
                answers.push(answer);
                if answer != Expect::Refused(TapeRefusal::NoNewMark) {
                    fresh_label = false;
                }
                pending.clear();
            }
        }
        let count = |e: Expect| answers.iter().filter(|a| **a == e).count();
        assert_eq!(answers.len(), 10, "{}", label.id);
        assert_eq!(
            count(Expect::Read {
                marks: 1,
                new_label: true
            }),
            2,
            "{}",
            label.id
        );
        assert_eq!(
            count(Expect::Read {
                marks: 2,
                new_label: false
            }),
            1,
            "{}",
            label.id
        );
        assert_eq!(
            count(Expect::Read {
                marks: 1,
                new_label: false
            }),
            4,
            "{}",
            label.id
        );
        assert_eq!(count(Expect::Pick), 1, "{}", label.id);
        assert_eq!(
            count(Expect::Refused(TapeRefusal::MarkClipped)),
            1,
            "{}",
            label.id
        );
        assert_eq!(
            count(Expect::Refused(TapeRefusal::NoNewMark)),
            1,
            "{}",
            label.id
        );
    }
}

/// Every photo frames the whole label and enough shaft to orient it, and every pose is as square or
/// as tilted as its drill says. No photo is drawn.
#[test]
fn every_photo_frames_the_label_and_the_shaft() {
    for pose in SQUARE {
        let camera = Camera::new(pose.0[0], pose.0[1], pose.0[2], pose.1, [0.0, 0.0]);
        assert!(
            camera.tilt_deg() <= 10.0,
            "{pose:?}: {:.1}°",
            camera.tilt_deg()
        );
    }
    for pose in TILTED {
        let camera = Camera::new(pose.0[0], pose.0[1], pose.0[2], pose.1, [0.0, 0.0]);
        let tilt = camera.tilt_deg();
        assert!((20.0..=30.0).contains(&tilt), "{pose:?}: {tilt:.1}°");
    }
    let mut misses = Vec::new();
    let in_frame = |[x, y]: [f64; 2], margin: f64| {
        (margin..=PHOTO_WIDTH as f64 - margin).contains(&x)
            && (margin..=PHOTO_HEIGHT as f64 - margin).contains(&y)
    };
    for label in [&STRIKEPRO_DRIVER, &STRIKEPRO_HYBRID, &STRIKEPRO_IRON] {
        for drill in &DRILLS {
            {
                for photo in 0..drill.poses.len() {
                    let scene = scene(label, drill, photo, drill.logo_up, &[]);
                    let what = format!("{} {} photo {photo}", label.id, drill.name);
                    for corner in scene.true_corners() {
                        if !in_frame(corner, LABEL_IN_FRAME_PX) {
                            misses.push(format!("{what}: the label at {corner:.0?}"));
                        }
                    }
                    for along in [SHAFT_FROM_MM, SHAFT_FROM_MM + SHAFT_IN_FRAME_MM] {
                        let at = scene.camera.project(scene.hosel_mm(along).unwrap());
                        if !in_frame(at, 0.0) {
                            misses.push(format!("{what}: the shaft {along} mm up at {at:.0?}"));
                        }
                    }
                }
            }
        }
    }
    assert!(
        misses.is_empty(),
        "
{}",
        misses.join(
            "
"
        )
    );
}
