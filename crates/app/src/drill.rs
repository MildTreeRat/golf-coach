//! The impact drill by STRIKEPRO tape, on the phone: one photo of the label on the club to the
//! strikes it shows, stored with a crop each, the club's block judged when it fills, and the golfer's
//! two declarations, a tap on the new mark and "missed the tape" [M46 P18]. Then the drill's actions:
//! the clubs it runs on, a club's drill opened, a suggestion answered and a stage changed [M46 P19].
//!
//! This adds no reading, judging, storing or wording of its own. The photo is read by
//! `impact::tape::read_tape`, a block is judged by `analysis::drill`, the drill is stored by
//! `storage::drill_store`, and every word is `feedback::drill_rules`' or `TapeNotRead::sentence`'s.
//! What is the phone's is the order they run in, the counters the ids come from, the strikes that wait
//! for the golfer, and the answer Dart gets. `api::drill` holds the shapes and three one-line calls,
//! outside this module because the codegen binds every `pub` item in `api` (the M38 plan's rule 7).
//!
//! # The photo (C16)
//!
//! Swift decodes the photo as `ScreenReader.swift` does and writes it as raw RGBA under
//! `<root>/.app/tmp/`. A path anywhere else is refused before anything is touched, so a bug in a
//! caller cannot hand this a store's file to delete. Past that check the file is Rust's, and
//! `TakenPhoto` removes it whatever the answer, refusals included, so that nothing a photo leaves
//! piles up in `tmp/`. **That `Drop` is the one place a switch to keep originals would go** (the plan's
//! P3 finding 10, for P29, which may want them for a desk re-run).
//!
//! **A photo in which no label was found is kept**, as M38's C13 keeps a failed screen photo, because
//! those are the photos that improve the finder: in `.app/failed/`, which the export carries. It is
//! kept as a JPEG at [`FAILED_JPEG_QUALITY`] rather than as the raw file, which is 48 MB at 12 MP and
//! would ride in every export after it. The desk tool reads a JPEG (`golf-lab read-tape`).
//!
//! # The reference: the club's last crop (C16, the plan's P12 finding 3)
//!
//! Each photo is read against `last/<club>.rgb`, raw RGB at the label type's crop grid, because a
//! JPEG's artefacts would show up in a diff as marks. It advances on `Read`, on `NeedsPick` (whatever
//! becomes of the pick, since its mark is on the label either way) and on `mark_clipped`: every
//! outcome whose new marks are accounted for. It stays on `no_label`, `no_orientation`, `no_new_mark`
//! and `decode_failed`, so that a retake still shows the strike. This is the rule P13's gate held the
//! reader to, and `golf-lab read-tape --sequence` keeps it too.
//!
//! # The two counters (C8)
//!
//! `strikes_issued` gives every stored or waiting strike its id, and never gives one twice.
//! `labels_issued` names each label applied, and the club's `AppliedLabel` holds the place on it of
//! the last mark. **The place advances with the reference**, by one a mark accounted for. A label is
//! new when the reader says so, or when it read with no reference at all (D4: a fresh label), and the
//! club then takes the next label id. A strike whose mark is not on the label, or not seen (declared
//! off the tape, or not read), records the place its mark would take, `n + 1` on the club's label or
//! the next label id at 1, and moves nothing, so the retake that reads it lands at the same place.
//!
//! # Strikes that wait for the golfer (C15)
//!
//! A `NeedsPick`, or a `no_new_mark` with the label found, is held as a **pending** strike, under
//! `.app/drill_pending/` (which stays home: the export walks named paths, `crate::export`). Its id is
//! a strike id, issued when the photo is taken, and it holds the strike **as it is stored if nobody
//! resolves it**: not read, with its crop. `pick_mark` and `declare_off_tape` resolve it, and the
//! club's next photo discards it, storing it as it stands. So a retake after `no_new_mark` leaves
//! that photo's strike stored as `no_new_mark`, and a pick nobody made is stored as `not_picked`, the
//! one reason the facade gives rather than the reader (P6 finding 3 left it to P18). Nothing is
//! dropped, which is the milestone's exit 2.
//!
//! A pick also takes a `no_new_mark` (P12 finding 9: a strike on an old mark can look like nothing
//! new). Its crop's sides are not in the reader's `NotRead`, so [`read_photo`] asks `impact::tape::orient`
//! for them again, which the reader had just answered.
//!
//! **A pending strike is resolved only into the club as it was when the photo was taken.** A stage
//! or level changed since refuses the resolution, and the next photo stores the strike as not read.
//!
//! # Blocks (C10, C11, C13, the plan's P15 finding 5)
//!
//! A strike counts when it is read, picked or declared off the tape. It joins the club's block, and
//! the block is judged at its tenth by its stage: a baseline sets the level
//! (`level_or_widest(start_level(..))`, the user's rule for a baseline whose widest circle holds too
//! few) and moves the club to the ladder; a ladder block moves the level by `judge_block`; a recentre
//! or differential block keeps it. Then the history gains its entry and `suggestion` is asked, and its
//! answer stored, whatever was waiting before.
//!
//! **A photo with two new marks on a block of nine** closes the block with the first and opens the
//! next with the second, as C10's "a block is 10 counted strikes" reads (P14 finding 4, confirmed
//! here). The marks are stamped one at a time, so the second carries the stage and the circle the
//! judgment left, and a judged block never holds strikes of two stages.
//!
//! **The baseline's R50** (C11, P16 finding 3) is the median distance of its ten strikes from the
//! label's centre, worked here. A strike declared off the tape has no distance, but it lies past the
//! label's inscribed radius, so the median is still known when every distance it reads is inside that
//! radius (`baseline_r50`). Otherwise it is `None`, and the tip says the baseline had a miss.
//!
//! # The order things are written in
//!
//! `[root]/drills/` is written in an order that a crash at any point leaves usable:
//!
//! 1. **The counters first**, alone. A strike id that reached `strikes.jsonl` with the counter behind
//!    it would be issued again, and `DrillStore::append_strike` refuses an id it holds, so every later
//!    strike would be refused. A crash after this step costs only unused ids.
//! 2. The pending strike, the crops, then the strikes. A crop is never named by a strike before it is
//!    on disk.
//! 3. The reference, then the state with the blocks and the levels. A crash between leaves a strike
//!    stored and outside any block, where the other order would leave the reference behind and show
//!    the same mark as new on the next photo.
//!
//! The whole call holds `crate::record::one_writer()` (the plan's P17 finding 2), so no two calls draw
//! one counter, and the export never zips half a strike. The store's own lock is inner and short.
//!
//! # The drill's actions (C13, C15, the plan's P19)
//!
//! **A stage changes in two ways**: the golfer accepts a suggestion (`answer_suggestion`), or picks a
//! stage (`set_stage`). Both keep the level, and both go through `Drill::change_stage`, which:
//!
//! 1. stores the club's waiting strike as it stands (`discard_pending`), because a tap after the
//!    change would be refused, and the strike would wait for the next photo (the plan's P18 finding 4);
//! 2. abandons the block in progress. Its strikes stay stored, and it is never judged (C10);
//! 3. clears the waiting suggestion. A suggestion is about what follows the block that raised it, and
//!    the golfer has now said what follows. `analysis::drill::suggestion` works each one again after
//!    every block, so nothing is lost (C13).
//!
//! Choosing the stage the club is already in changes nothing, block included, and writes nothing.
//! **A club whose first baseline has not set a level stays in the baseline**, because
//! `ClubDrillState` refuses a later stage with no level. A judged baseline always leaves a level,
//! the widest when its widest circle held too few (`level_or_widest`, the user's rule), so after any
//! baseline every suggestion can be accepted.
//!
//! Declining a suggestion only clears it. The stage stays, so a waiting strike stays resolvable.
//!
//! **Opening a club writes nothing** (`start`). A club with no drill state is shown at the start of
//! its baseline, as `Drill::open` makes it in memory, and its first stored strike writes it. So
//! browsing the picker leaves `state.json` as it was, and `drill_clubs` lists a club as started only
//! once it has a strike.
//!
//! **The suggestion's sentence is worked again from the record** each time a view asks
//! (`Drill::suggestion_line`), never stored as words (the plan's P16 finding 4). Its inputs are read
//! as they stood when the suggestion was made: the pattern over the club's strikes through the last
//! judged block's last, at the club's level, which no later strike or change can have moved while the
//! suggestion waits.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use analysis::drill::{
    call_made, inside, judge_block, judge_differential, judge_recentre, level_or_widest,
    off_centre, pattern, start_level, suggestion,
};
use analysis::stats;
use contracts::club::{category_of, ClubId};
use contracts::drill::{
    crop_file, label_id, ladder_radii, strike_id, AppliedLabel, ClubDrillState, DrillStage,
    DrillState, LevelEntry, StrikeCall, StrikeSource, Suggestion, TapeNotRead, TapeStrike,
    BLOCK_SIZE,
};
use contracts::face_impact::TAPE_READER_VERSION;
use contracts::golfer::Handedness as GolferHand;
use contracts::intent::ClubCategory;
use contracts::Timestamp;
use feedback::drill_rules::{
    baseline_progress, baseline_tip, call_verdict, differential_progress, differential_result,
    differential_suggestion, fresh_baseline_suggestion, inside_verdict, ladder_progress,
    ladder_result, level_line, position_line, recentre_progress, recentre_result,
    recentre_suggestion, return_to_ladder_suggestion, stage_name, DifferentialWhy, NO_LABEL_FITS,
    NO_LEVEL_YET,
};
use impact::tape::{
    self, CategoryLabel, CropGrid, Handedness, LabelType, Orientation, TapeRefusal, CROP_PX_PER_MM,
};
use impact::Frame;
use serde::{Deserialize, Serialize};
use storage::bag_store::BagStore;
use storage::drill_store::DrillStore;
use storage::golfer_store::GolferStore;
use storage::StoreError;

use crate::api::drill::{
    DrillCall, DrillClubView, DrillStage as StageView, DrillView, TapeIn, TapeOutcome, TapeStatus,
    TapeStrikeCard,
};
use crate::record::one_writer;
use crate::{club_id, parse_now, state, Dirs};

/// The JPEG quality a strike's crop is kept at (D6). **P18's, provisional**: P7 measured the three
/// label types' crops at 36 to 58 KB at 75 and 47 to 76 KB at 85, D6's "tens of KB", and 80 is the
/// middle (the plan's P7 finding 4). What revises it is P29's first real crops: whether a mark's edge
/// survives it by eye on the desk tool's review, and what a session's crops weigh in an export. No
/// reading depends on it, since the diff's reference is raw.
pub const CROP_JPEG_QUALITY: u8 = 80;

/// The JPEG quality a photo with no label found is kept at in `.app/failed/` (the module doc). Higher
/// than a crop's, because the whole photo is the finder's evidence and only those photos are kept.
/// **P18's, provisional**, revised if P29's failed photos weigh too much in an export.
pub const FAILED_JPEG_QUALITY: u8 = 90;

/// The pending strikes' directory, under `Dirs::app` (the module doc).
pub const PENDING_DIR: &str = "drill_pending";

/// Where Swift writes each photo for this call, under `Dirs::app` (C16).
pub const TMP_DIR: &str = "tmp";

/// What a pending strike's crop and record are named by: `<id>.jpg` and `<id>.json`.
const PENDING_JPEG: &str = "jpg";
const PENDING_RECORD: &str = "json";

/// The sentence for a drill state that cannot be read (the plan's P17 finding 4): never read as
/// absent, because the counters every id comes from live in it.
const UNREADABLE: &str = "The drill's record on this phone cannot be read, so nothing was stored.";

/// What reads one photo: the reader's outcome, and the label's sides where the outcome found them.
///
/// `impact::tape::TapeOutcome::NotRead` carries the label and not its orientation, which a tap on a
/// `no_new_mark` needs (the module doc), so the sides travel beside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub outcome: tape::TapeOutcome,
    pub sides: Option<Orientation>,
}

/// Reads one photo: the club's last crop as `previous`, the golfer's hand. [`read_photo`] is the
/// phone's, and [`record_tape_photo_with`] takes another so that a test can stand in for the reader.
pub type Reader<'a> = dyn FnMut(&Frame, Option<&Frame>, &LabelType, Handedness) -> Reading + 'a;

/// The reader the phone runs: `read_tape` with the golfer's hand, then `orient` again for a
/// `no_new_mark`, whose outcome does not carry the sides the reader found.
pub fn read_photo(
    photo: &Frame,
    previous: Option<&Frame>,
    label: &LabelType,
    hand: Handedness,
) -> Reading {
    let outcome = tape::read_tape(photo, previous, label, Some(hand));
    let sides = match &outcome {
        tape::TapeOutcome::Read(reading) => Some(reading.orientation),
        tape::TapeOutcome::NeedsPick(pick) => Some(pick.orientation),
        tape::TapeOutcome::NotRead {
            reason: TapeRefusal::NoNewMark,
            found: Some(found),
        } => tape::orient(photo, found, label, Some(hand))
            .ok()
            .map(|(orientation, _)| orientation),
        tape::TapeOutcome::NotRead { .. } => None,
    };
    Reading { outcome, sides }
}

/// `record_tape_photo`'s body.
pub(crate) fn record(photo: TapeIn) -> Result<TapeOutcome, String> {
    record_tape_photo_with(photo, &mut read_photo)
}

/// `record_tape_photo` with the reader given (the module doc, top to bottom). Outside `api`, so the
/// codegen never binds it.
pub fn record_tape_photo_with(photo: TapeIn, read: &mut Reader<'_>) -> Result<TapeOutcome, String> {
    let _one_at_a_time = one_writer();
    let root = PathBuf::from(&photo.root);
    let taken = TakenPhoto::of(&root, &photo.raw_path)?;
    let captured_at = parse_now(&photo.captured_at)?;
    let now = parse_now(&photo.now)?;
    let club = club_id(&photo.club)?;
    let call = photo.call.map(stored_call);
    let drill = Drill::open(&root, &photo.player_id, club, now)?;
    drill.check_call(call)?;
    discard_pending(&root, &drill.store, club)?;

    let frame = decode_raw(&taken.0, photo.width, photo.height);
    let reference = drill.reference()?;
    let reading = match &frame {
        Some(frame) => read(frame, reference.as_ref(), drill.label, drill.hand),
        None => Reading {
            outcome: tape::TapeOutcome::NotRead {
                reason: TapeRefusal::DecodeFailed,
                found: None,
            },
            sides: None,
        },
    };
    let fresh = reference.is_none();

    let mut next = drill.state.clone();
    let mut club_strikes = drill.club_strikes()?;
    let mut plan = Plan::default();
    let status = match reading.outcome {
        tape::TapeOutcome::Read(seen) => {
            let crop = jpeg(&seen.found.crop.frame, CROP_JPEG_QUALITY)?;
            for (at, mark) in seen.marks.iter().enumerate() {
                let id = issue(&mut next, &drill.install);
                let place = drill.place(&mut next, seen.new_label && at == 0, true);
                let mut strike = drill.stamp(&next, id, place, captured_at, call);
                strike.offset_h_mm = Some(mark.offset_h_mm);
                strike.offset_v_mm = Some(mark.offset_v_mm);
                strike.source = Some(StrikeSource::Read);
                strike.crop = Some(crop_file(&strike.strike_id));
                drill.land(
                    &mut next,
                    &mut club_strikes,
                    &mut plan,
                    strike,
                    Some(crop.clone()),
                    now,
                )?;
            }
            plan.reference = Some(seen.found.crop.frame.rgb);
            TapeStatus::Read
        }
        tape::TapeOutcome::NeedsPick(pick) => {
            let id = issue(&mut next, &drill.install);
            let place = drill.place(&mut next, fresh, true);
            let mut strike = drill.stamp(&next, id, place, captured_at, call);
            strike.not_read = Some(TapeNotRead::NotPicked);
            strike.crop = Some(crop_file(&strike.strike_id));
            let crop = jpeg(&pick.found.crop.frame, CROP_JPEG_QUALITY)?;
            plan.pending = Some((
                Pending {
                    kind: PendingKind::NeedsPick,
                    sides: Some(Sides::of(pick.orientation)),
                    strike,
                },
                crop,
            ));
            plan.reference = Some(pick.found.crop.frame.rgb);
            TapeStatus::NeedsPick
        }
        tape::TapeOutcome::NotRead { reason, found } => {
            let reason = stored_reason(reason)?;
            let id = issue(&mut next, &drill.install);
            // The one refusal whose mark is on the label and accounted for (the module doc).
            let clipped = reason == TapeNotRead::MarkClipped;
            let place = drill.place(&mut next, fresh && clipped, clipped);
            let mut strike = drill.stamp(&next, id, place, captured_at, call);
            strike.not_read = Some(reason);
            let crop = match &found {
                Some(found) => {
                    strike.crop = Some(crop_file(&strike.strike_id));
                    Some(jpeg(&found.crop.frame, CROP_JPEG_QUALITY)?)
                }
                None => None,
            };
            plan.reason = Some(reason);
            match (reason, crop) {
                (TapeNotRead::NoNewMark, Some(crop)) => {
                    plan.pending = Some((
                        Pending {
                            kind: PendingKind::NoNewMark,
                            sides: reading.sides.map(Sides::of),
                            strike,
                        },
                        crop,
                    ));
                    TapeStatus::NoNewMark
                }
                (_, crop) => {
                    if clipped {
                        plan.reference = found.map(|found| found.crop.frame.rgb);
                    }
                    if reason == TapeNotRead::NoLabel {
                        if let Some(frame) = &frame {
                            plan.failed = Some((
                                format!("tape-{}.jpg", strike.strike_id),
                                jpeg(frame, FAILED_JPEG_QUALITY)?,
                            ));
                        }
                    }
                    drill.land(&mut next, &mut club_strikes, &mut plan, strike, crop, now)?;
                    TapeStatus::NotRead
                }
            }
        }
    };
    drill.commit(&mut next, now, &plan)?;
    drill.answer(&next, &club_strikes, &plan, status)
}

/// `pick_mark`'s body: the tap is 0 to 1 across the crop, from its left and its top.
pub(crate) fn pick(
    root: &str,
    pending_id: &str,
    x: f64,
    y: f64,
    now: &str,
) -> Result<TapeOutcome, String> {
    let across = |value: f64| value.is_finite() && (0.0..=1.0).contains(&value);
    if !(across(x) && across(y)) {
        return Err(format!(
            "a tap is from 0 to 1 across the crop, and ({x}, {y}) is not on it"
        ));
    }
    resolve(root, pending_id, Declared::Pick([x, y]), now)
}

/// `declare_off_tape`'s body.
pub(crate) fn declare(root: &str, pending_id: &str, now: &str) -> Result<TapeOutcome, String> {
    resolve(root, pending_id, Declared::OffTape, now)
}

/// What the golfer said of a pending strike.
enum Declared {
    /// The tap, 0 to 1 across the crop.
    Pick([f64; 2]),
    OffTape,
}

/// A pending strike resolved by the golfer's declaration (the module doc).
fn resolve(
    root: &str,
    pending_id: &str,
    declared: Declared,
    now: &str,
) -> Result<TapeOutcome, String> {
    let _one_at_a_time = one_writer();
    let now = parse_now(now)?;
    let root = PathBuf::from(root);
    let pending = read_pending(&root, pending_id)?
        .ok_or_else(|| format!("no strike is waiting under '{pending_id}'"))?;
    let mut strike = pending.strike.clone();
    let drill = Drill::open(&root, &strike.player_id, strike.club, now)?;
    let mut next = drill.state.clone();
    let club = drill.club_state(&next);
    if club.stage != strike.stage || drill.radius_of(club) != strike.level_radius_mm {
        return Err(
            "This club's stage or level has changed since the photo, so its strike \
                    cannot be placed now. The club's next photo stores it as not read."
                .to_string(),
        );
    }
    let mut club_strikes = drill.club_strikes()?;
    if club_strikes.iter().any(|held| held.strike_id == pending_id) {
        // An earlier call stored it and was cut off before it took the pending away.
        remove_pending(&root, pending_id)?;
        return Err(format!("strike '{pending_id}' is already stored"));
    }

    strike.not_read = None;
    let status = match declared {
        Declared::Pick([x, y]) => {
            let sides = pending.sides.ok_or(
                "this photo's sides were not found, so a tap on it cannot be placed; take the \
                 next photo",
            )?;
            let grid = CropGrid::new(drill.label, CROP_PX_PER_MM);
            let printed = grid.to_label_mm([x * grid.width as f64, y * grid.height as f64]);
            let [h, v] = sides.orientation().to_reading_mm(printed);
            strike.offset_h_mm = Some(h);
            strike.offset_v_mm = Some(v);
            strike.source = Some(StrikeSource::Picked);
            // A `NeedsPick`'s place was kept when its photo was taken; a `no_new_mark`'s mark is on
            // the label only now that the golfer says so.
            if pending.kind == PendingKind::NoNewMark {
                (strike.label_id, strike.n_on_label) = drill.place(&mut next, false, true);
            }
            TapeStatus::Picked
        }
        Declared::OffTape => {
            if pending.kind != PendingKind::NoNewMark {
                return Err(
                    "A new mark was seen on this label, so the ball did not miss the \
                            tape. Tap the new mark."
                        .to_string(),
                );
            }
            strike.source = Some(StrikeSource::DeclaredOffTape);
            (strike.label_id, strike.n_on_label) = drill.place(&mut next, false, false);
            TapeStatus::OffTape
        }
    };
    let crop = fs::read(pending_path(&root, pending_id, PENDING_JPEG)).ok();
    if crop.is_none() {
        if strike.source == Some(StrikeSource::Picked) {
            return Err(format!(
                "the crop strike '{pending_id}' waited with is gone, so no tap on it can be stored"
            ));
        }
        strike.crop = None;
    }
    let mut plan = Plan {
        resolved: Some(pending_id.to_string()),
        ..Plan::default()
    };
    drill.land(&mut next, &mut club_strikes, &mut plan, strike, crop, now)?;
    drill.commit(&mut next, now, &plan)?;
    drill.answer(&next, &club_strikes, &plan, status)
}

/// One golfer's drill on one club, as a call finds it.
struct Drill {
    root: PathBuf,
    store: DrillStore,
    install: String,
    club: ClubId,
    category: ClubCategory,
    label: &'static LabelType,
    hand: Handedness,
    /// The state as read, with the club's entry made if it had none: a club's first photo starts its
    /// baseline (C11).
    state: DrillState,
}

impl Drill {
    /// Everything a strike is stamped with that is not the photo's own, or why it cannot be.
    fn open(root: &Path, player_id: &str, club: ClubId, now: Timestamp) -> Result<Drill, String> {
        let install = state::read(root)
            .install_id
            .ok_or("this phone has no install id yet, so no strike can be named")?;
        let golfer = GolferStore::new(Dirs::of(root).golfers)
            .get(player_id)
            .ok_or_else(|| format!("golfer '{player_id}' is not on this phone"))?;
        // `impact` cannot see `contracts`, so the two spellings of a hand meet here (P10 finding 10).
        let hand = match golfer.handedness {
            GolferHand::Right => Handedness::Right,
            GolferHand::Left => Handedness::Left,
        };
        let category = category_of(club);
        let label = tape_label(category).ok_or_else(|| format!("{NO_LABEL_FITS} ({club})."))?;
        let store = DrillStore::open(root, player_id).map_err(|refused| refused.to_string())?;
        let mut state = store
            .state()
            .map_err(|unreadable| format!("{UNREADABLE} {unreadable}"))?
            .unwrap_or_else(|| DrillState {
                player_id: player_id.to_string(),
                strikes_issued: 0,
                labels_issued: 0,
                clubs: BTreeMap::new(),
                updated_at: now,
            });
        state.clubs.entry(club).or_insert_with(|| ClubDrillState {
            stage: DrillStage::Baseline,
            level: None,
            block: Vec::new(),
            suggestion: None,
            history: Vec::new(),
            label: None,
        });
        Ok(Drill {
            root: root.to_path_buf(),
            store,
            install,
            club,
            category,
            label,
            hand,
            state,
        })
    }

    fn club_state<'a>(&self, state: &'a DrillState) -> &'a ClubDrillState {
        state
            .clubs
            .get(&self.club)
            .expect("Drill::open makes the club's entry")
    }

    fn club_state_mut<'a>(&self, state: &'a mut DrillState) -> &'a mut ClubDrillState {
        state
            .clubs
            .get_mut(&self.club)
            .expect("Drill::open makes the club's entry")
    }

    /// A differential strike is hit at a call, and no other strike is (C19).
    fn check_call(&self, call: Option<StrikeCall>) -> Result<(), String> {
        match (self.club_state(&self.state).stage, call) {
            (DrillStage::Differential, None) => Err(
                "this club is in differential practice, so each photo needs the call it was hit at"
                    .to_string(),
            ),
            (DrillStage::Baseline | DrillStage::Ladder | DrillStage::Recentre, Some(_)) => {
                Err("only a differential strike is hit at a call".to_string())
            }
            _ => Ok(()),
        }
    }

    /// The club's strikes as stored, oldest first.
    fn club_strikes(&self) -> Result<Vec<TapeStrike>, String> {
        Ok(self
            .store
            .strikes(self.club)
            .map_err(|failed| failed.to_string())?
            .strikes)
    }

    /// The club's last crop, at its label type's grid (the plan's P17 finding 5). Bytes of another
    /// length make a reference the reader's diff refuses into a pick, never a reading.
    fn reference(&self) -> Result<Option<Frame>, String> {
        let grid = CropGrid::new(self.label, CROP_PX_PER_MM);
        Ok(self
            .store
            .last(self.club)
            .map_err(|failed| failed.to_string())?
            .map(|rgb| Frame {
                width: grid.width,
                height: grid.height,
                rgb,
            }))
    }

    /// The circle a strike in this stage is judged against: none in the baseline (C11).
    fn radius_of(&self, club: &ClubDrillState) -> Option<f64> {
        match club.stage {
            DrillStage::Baseline => None,
            DrillStage::Ladder | DrillStage::Recentre | DrillStage::Differential => {
                Some(ladder_radii(self.category)[level_or_widest(club.level)])
            }
        }
    }

    /// Where the next mark lands on the club's label (the module doc's counters): a new label when
    /// `fresh` or when the club holds none. `advance` keeps the place, which only a mark accounted
    /// for on the label does.
    fn place(&self, next: &mut DrillState, fresh: bool, advance: bool) -> (String, i64) {
        let held = self.club_state(next).label.clone().filter(|_| !fresh);
        let (label, issued) = match held {
            Some(held) => (
                AppliedLabel {
                    n_on_label: held.n_on_label + 1,
                    ..held
                },
                false,
            ),
            None => (
                AppliedLabel {
                    label_id: label_id(&self.install, next.labels_issued + 1),
                    n_on_label: 1,
                },
                true,
            ),
        };
        let place = (label.label_id.clone(), label.n_on_label);
        if advance {
            if issued {
                next.labels_issued += 1;
            }
            self.club_state_mut(next).label = Some(label);
        }
        place
    }

    /// A strike as the club stands in `next`, with no position, source or reason yet.
    fn stamp(
        &self,
        next: &DrillState,
        strike_id: String,
        (label_id, n_on_label): (String, i64),
        captured_at: Timestamp,
        call: Option<StrikeCall>,
    ) -> TapeStrike {
        let club = self.club_state(next);
        TapeStrike {
            strike_id,
            player_id: next.player_id.clone(),
            club: self.club,
            category: self.category,
            label_type: self.label.id.to_string(),
            label_id,
            n_on_label,
            captured_at,
            stage: club.stage,
            level_radius_mm: self.radius_of(club),
            call,
            offset_h_mm: None,
            offset_v_mm: None,
            source: None,
            not_read: None,
            reader_version: TAPE_READER_VERSION,
            crop: None,
        }
    }

    /// A strike to store: into the plan, and, when it counts, into the club's block, which is judged
    /// at its tenth (the module doc's blocks).
    fn land(
        &self,
        next: &mut DrillState,
        club_strikes: &mut Vec<TapeStrike>,
        plan: &mut Plan,
        strike: TapeStrike,
        crop: Option<Vec<u8>>,
        now: Timestamp,
    ) -> Result<(), String> {
        let counts = strike.counts();
        let id = strike.strike_id.clone();
        club_strikes.push(strike.clone());
        plan.strikes.push((strike, crop));
        if !counts {
            return Ok(());
        }
        let club = self.club_state_mut(next);
        club.block.push(id);
        if club.block.len() >= BLOCK_SIZE {
            plan.judged = Some(self.judge(club, club_strikes, now)?);
        }
        Ok(())
    }

    /// The club's full block judged by its stage, its history entry pushed, and the suggestion asked
    /// for after it (the plan's P15 finding 5). Answers the block's result, in `feedback`'s words.
    fn judge(
        &self,
        club: &mut ClubDrillState,
        strikes: &[TapeStrike],
        now: Timestamp,
    ) -> Result<String, String> {
        let refused = |refusal: contracts::ContractError| refusal.to_string();
        let radii = ladder_radii(self.category);
        let block = strikes_named(&club.block, strikes)?;
        let from = club.level;
        let level = level_or_widest(from);
        let result = match club.stage {
            DrillStage::Baseline => {
                let start = start_level(&block, self.category).map_err(refused)?;
                let to = level_or_widest(start);
                club.level = Some(to);
                club.stage = DrillStage::Ladder;
                baseline_tip(
                    baseline_r50(&block, self.label.inscribed_radius_mm),
                    radii[to],
                    start.is_some(),
                )
            }
            DrillStage::Ladder => {
                let judged = judge_block(&block, level, self.category).map_err(refused)?;
                club.level = Some(judged.new_level);
                ladder_result(
                    judged.inside,
                    judged.counted,
                    radii[level],
                    radii[judged.new_level],
                    judged.at_tightest_in,
                    judged.at_widest_out,
                )
            }
            DrillStage::Recentre => {
                let judged = judge_recentre(&block, level, self.category).map_err(refused)?;
                recentre_result(judged.mean, judged.goal_met, radii[level])
            }
            DrillStage::Differential => {
                let judged = judge_differential(&block, level, self.category).map_err(refused)?;
                differential_result(judged.made, judged.counted, judged.fresh_baseline)
            }
        };
        let block = std::mem::take(&mut club.block);
        club.history.push(LevelEntry {
            at: now,
            from,
            to: club.level,
            block,
        });
        club.suggestion = suggestion(self.club, club, strikes).map_err(refused)?;
        Ok(result)
    }

    /// Everything the plan holds, written in the module doc's order.
    fn commit(&self, next: &mut DrillState, now: Timestamp, plan: &Plan) -> Result<(), String> {
        let failed = |what: &str, error: StoreError| format!("could not store {what}: {error}");
        next.updated_at = now;
        if let Some((name, bytes)) = &plan.failed {
            write_file(&Dirs::of(&self.root).app.join("failed").join(name), bytes)
                .map_err(|error| format!("could not keep the photo with no label: {error}"))?;
        }
        if (next.strikes_issued, next.labels_issued)
            != (self.state.strikes_issued, self.state.labels_issued)
        {
            let reserved = DrillState {
                strikes_issued: next.strikes_issued,
                labels_issued: next.labels_issued,
                updated_at: now,
                ..self.state.clone()
            };
            self.store
                .put_state(&reserved)
                .map_err(|error| failed("the drill's counters", error))?;
        }
        if let Some((pending, crop)) = &plan.pending {
            write_pending(&self.root, pending, crop)
                .map_err(|error| format!("could not hold the strike for the golfer: {error}"))?;
        }
        for (strike, crop) in &plan.strikes {
            if let Some(crop) = crop.as_ref().filter(|_| strike.crop.is_some()) {
                self.store
                    .put_crop(&strike.strike_id, crop)
                    .map_err(|error| failed("a strike's crop", error))?;
            }
        }
        for (strike, _) in &plan.strikes {
            self.store
                .append_strike(strike)
                .map_err(|error| failed("a strike", error))?;
        }
        if let Some(rgb) = &plan.reference {
            self.store
                .put_last(self.club, rgb)
                .map_err(|error| failed("the club's last crop", error))?;
        }
        self.store
            .put_state(next)
            .map_err(|error| failed("the drill's state", error))?;
        if let Some(id) = &plan.resolved {
            remove_pending(&self.root, id)?;
        }
        Ok(())
    }

    /// The card (C20): the strikes stored, the waiting one, the club's block as it now stands, and
    /// the suggestion a block finished here raised.
    fn answer(
        &self,
        next: &DrillState,
        club_strikes: &[TapeStrike],
        plan: &Plan,
        status: TapeStatus,
    ) -> Result<TapeOutcome, String> {
        let club = self.club_state(next);
        let suggestion = match plan.judged {
            Some(_) => self.suggestion_line(club, club_strikes)?,
            None => None,
        };
        let crop_path = match &plan.pending {
            Some((pending, _)) => Some(pending_path(
                &self.root,
                &pending.strike.strike_id,
                PENDING_JPEG,
            )),
            None => plan
                .strikes
                .iter()
                .find(|(strike, _)| strike.crop.is_some())
                .map(|(strike, _)| self.store.crop_path(&strike.strike_id)),
        };
        Ok(TapeOutcome {
            status,
            strikes: plan
                .strikes
                .iter()
                .map(|(strike, _)| card(strike))
                .collect(),
            pending_id: plan
                .pending
                .as_ref()
                .map(|(pending, _)| pending.strike.strike_id.clone()),
            crop_path: crop_path.map(|path| path.display().to_string()),
            message: plan.reason.map(|reason| reason.sentence().to_string()),
            retake_helps: plan.reason.is_some_and(TapeNotRead::retake_helps),
            block_line: self.block_line(club, club_strikes),
            block_result: plan.judged.clone(),
            suggestion,
        })
    }

    /// The club's baseline or block so far (C20), with each strike against the circle it is hit at.
    fn block_line(&self, club: &ClubDrillState, strikes: &[TapeStrike]) -> String {
        let block: Vec<&TapeStrike> = club
            .block
            .iter()
            .filter_map(|id| strikes.iter().find(|strike| strike.strike_id == *id))
            .collect();
        let radius = ladder_radii(self.category)[level_or_widest(club.level)];
        let counted = block.len();
        match club.stage {
            DrillStage::Baseline => baseline_progress(counted),
            DrillStage::Ladder => ladder_progress(
                counted,
                block
                    .iter()
                    .filter(|strike| inside(strike, radius) == Some(true))
                    .count(),
            ),
            DrillStage::Recentre => recentre_progress(counted),
            DrillStage::Differential => differential_progress(
                counted,
                block
                    .iter()
                    .filter(|strike| {
                        strike
                            .call
                            .and_then(|call| call_made(strike, call, radius))
                            .unwrap_or(false)
                    })
                    .count(),
            ),
        }
    }

    /// The club's page between photos (C15's `DrillView`).
    fn view(&self, state: &DrillState, strikes: &[TapeStrike]) -> Result<DrillView, String> {
        let club = self.club_state(state);
        Ok(DrillView {
            club: self.club.as_str().to_string(),
            stage: stage_view(club.stage),
            stage_name: stage_name(club.stage).to_string(),
            level_line: level_text(club.level, self.category),
            block_line: self.block_line(club, strikes),
            suggestion: self.suggestion_line(club, strikes)?,
            call_wanted: club.stage == DrillStage::Differential,
        })
    }

    /// The waiting suggestion in words, with why it was made (the plan's P16 finding 4), from the
    /// record as it stood when it was made (the module doc's actions). `strikes` is the club's.
    fn suggestion_line(
        &self,
        club: &ClubDrillState,
        strikes: &[TapeStrike],
    ) -> Result<Option<String>, String> {
        let Some(suggested) = club.suggestion else {
            return Ok(None);
        };
        let last = club
            .history
            .last()
            .ok_or("a suggestion is waiting with no judged block behind it")?;
        let block = strikes_named(&last.block, strikes)?;
        let radii = ladder_radii(self.category);
        let radius = radii[level_or_widest(club.level)];
        let sentence = match suggested {
            Suggestion::Recentre => {
                // The pattern `suggestion` saw: the club's strikes through the judged block's last.
                let through = strikes
                    .iter()
                    .rposition(|strike| last.block.last() == Some(&strike.strike_id))
                    .map_or(0, |end| end + 1);
                match pattern(&strikes[..through]) {
                    Some(found) => {
                        let off = off_centre(&found, radius);
                        recentre_suggestion(found.centre, off.heel_toe, off.high_low)
                    }
                    // Only a record changed since the suggestion was made lands here, and the words
                    // with no number in them still say why it was.
                    None => recentre_suggestion((0.0, 0.0), false, false),
                }
            }
            Suggestion::Differential => {
                let why = match block.first().map(|strike| strike.stage) {
                    Some(DrillStage::Baseline) => DifferentialWhy::Baseline,
                    // `analysis::drill::suggestion` raises it after a baseline or a ladder block only.
                    Some(DrillStage::Ladder | DrillStage::Recentre | DrillStage::Differential) => {
                        DifferentialWhy::AtWidest
                    }
                    None => {
                        return Err("a judged block in the drill's record names no strikes".into())
                    }
                };
                // Index 0 is every table's widest circle (`ladder_radii`'s doc).
                differential_suggestion(why, radii[0])
            }
            Suggestion::ReturnToLadder => return_to_ladder_suggestion(radius),
            Suggestion::FreshBaseline => {
                let judged = judge_differential(&block, level_or_widest(last.from), self.category)
                    .map_err(|refused| refused.to_string())?;
                fresh_baseline_suggestion(judged.made, judged.counted)
            }
        };
        Ok(Some(sentence))
    }

    /// The club moved to `stage` at its level (the module doc's actions): its waiting strike stored,
    /// its block abandoned and its suggestion cleared. The stage it is in changes nothing.
    fn change_stage(&self, next: &mut DrillState, stage: DrillStage) -> Result<(), String> {
        let club = self.club_state_mut(next);
        if club.stage == stage {
            return Ok(());
        }
        if club.level.is_none() && stage != DrillStage::Baseline {
            return Err(NO_LEVEL_TO_LEAVE.to_string());
        }
        discard_pending(&self.root, &self.store, self.club)?;
        club.stage = stage;
        club.block.clear();
        club.suggestion = None;
        Ok(())
    }

    /// The state written whole, stamped `now`: an action's one write.
    fn save(&self, next: &mut DrillState, now: Timestamp) -> Result<(), String> {
        next.updated_at = now;
        self.store
            .put_state(next)
            .map_err(|error| format!("could not store the drill's state: {error}"))
    }
}

/// The refusal for a stage past the baseline before the club's first baseline sets a level, which
/// `ClubDrillState` refuses (the module doc's actions).
const NO_LEVEL_TO_LEAVE: &str = "This club's first baseline has not set its level yet, so it \
                                 stays in the baseline until the baseline's last strike.";

/// The refusal for an answer when no suggestion waits.
const NO_SUGGESTION: &str = "No suggestion is waiting for this club.";

/// `drill_clubs`' body: the bag in bag order, each club with its label and its place in the drill.
pub(crate) fn clubs(root: &str, player_id: &str) -> Result<Vec<DrillClubView>, String> {
    let root = Path::new(root);
    let Some(bag) = BagStore::new(Dirs::of(root).golfers).get(player_id) else {
        return Ok(Vec::new());
    };
    let state = DrillStore::open(root, player_id)
        .map_err(|refused| refused.to_string())?
        .state()
        .map_err(|unreadable| format!("{UNREADABLE} {unreadable}"))?;
    Ok(bag
        .club_ids()
        .into_iter()
        .map(|club| {
            let category = category_of(club);
            let label = tape_label(category);
            let held = state.as_ref().and_then(|state| state.clubs.get(&club));
            let line = match (label, held) {
                (None, _) => NO_LABEL_FITS.to_string(),
                (Some(_), Some(held)) => level_text(held.level, category),
                (Some(_), None) => NO_LEVEL_YET.to_string(),
            };
            let held = held.filter(|_| label.is_some());
            DrillClubView {
                club: club.as_str().to_string(),
                category: category.as_str().to_string(),
                label_type: label.map(|label| label.id.to_string()),
                can_start: label.is_some(),
                stage: held.map(|held| stage_view(held.stage)),
                stage_name: held.map(|held| stage_name(held.stage).to_string()),
                line,
            }
        })
        .collect())
}

/// `start_drill`'s body. Writes nothing (the module doc's actions).
pub(crate) fn start(
    root: &str,
    player_id: &str,
    club: &str,
    now: &str,
) -> Result<DrillView, String> {
    let _one_at_a_time = one_writer();
    let now = parse_now(now)?;
    let club = club_id(club)?;
    let drill = Drill::open(Path::new(root), player_id, club, now)?;
    drill.view(&drill.state, &drill.club_strikes()?)
}

/// `answer_suggestion`'s body.
pub(crate) fn answer_suggestion(
    root: &str,
    player_id: &str,
    club: &str,
    accept: bool,
    now: &str,
) -> Result<DrillView, String> {
    let _one_at_a_time = one_writer();
    let now = parse_now(now)?;
    let club = club_id(club)?;
    let drill = Drill::open(Path::new(root), player_id, club, now)?;
    let mut next = drill.state.clone();
    let suggested = drill.club_state(&next).suggestion.ok_or(NO_SUGGESTION)?;
    if accept {
        drill.change_stage(&mut next, stage_taken(suggested))?;
    }
    drill.club_state_mut(&mut next).suggestion = None;
    drill.save(&mut next, now)?;
    drill.view(&next, &drill.club_strikes()?)
}

/// `set_stage`'s body.
pub(crate) fn set_stage(
    root: &str,
    player_id: &str,
    club: &str,
    stage: StageView,
    now: &str,
) -> Result<DrillView, String> {
    let _one_at_a_time = one_writer();
    let now = parse_now(now)?;
    let club = club_id(club)?;
    let drill = Drill::open(Path::new(root), player_id, club, now)?;
    let mut next = drill.state.clone();
    let stage = stored_stage(stage);
    if drill.club_state(&next).stage != stage {
        drill.change_stage(&mut next, stage)?;
        drill.save(&mut next, now)?;
    }
    drill.view(&next, &drill.club_strikes()?)
}

/// The stage an accepted suggestion moves the club to (C13). A fresh baseline keeps the club's
/// level until it is judged, and the ladder starts from what it sets (the plan's P15 finding 5).
fn stage_taken(suggested: Suggestion) -> DrillStage {
    match suggested {
        Suggestion::Recentre => DrillStage::Recentre,
        Suggestion::Differential => DrillStage::Differential,
        Suggestion::ReturnToLadder => DrillStage::Ladder,
        Suggestion::FreshBaseline => DrillStage::Baseline,
    }
}

/// The label a club's category takes, or `None` for a category no label fits and so with no ladder
/// (the putter, the plan's P3 finding 8).
fn tape_label(category: ClubCategory) -> Option<&'static LabelType> {
    match tape::category_label(category.as_str()) {
        Some(CategoryLabel::Label(label)) if !ladder_radii(category).is_empty() => Some(label),
        _ => None,
    }
}

/// A club's level line, or [`NO_LEVEL_YET`] before its first baseline sets one. A validated
/// `DrillState` holds no level past its table, so `level_line`'s `None` is only ever that case's.
fn level_text(level: Option<usize>, category: ClubCategory) -> String {
    level
        .and_then(|level| level_line(level, category))
        .unwrap_or_else(|| NO_LEVEL_YET.to_string())
}

/// The stage as Dart sees it.
fn stage_view(stage: DrillStage) -> StageView {
    match stage {
        DrillStage::Baseline => StageView::Baseline,
        DrillStage::Ladder => StageView::Ladder,
        DrillStage::Recentre => StageView::Recentre,
        DrillStage::Differential => StageView::Differential,
    }
}

/// The stage as stored.
fn stored_stage(stage: StageView) -> DrillStage {
    match stage {
        StageView::Baseline => DrillStage::Baseline,
        StageView::Ladder => DrillStage::Ladder,
        StageView::Recentre => DrillStage::Recentre,
        StageView::Differential => DrillStage::Differential,
    }
}

/// The strikes `ids` name, in that order, or a refusal naming one that is not stored.
fn strikes_named(ids: &[String], strikes: &[TapeStrike]) -> Result<Vec<TapeStrike>, String> {
    ids.iter()
        .map(|id| {
            strikes
                .iter()
                .find(|strike| strike.strike_id == *id)
                .cloned()
                .ok_or_else(|| {
                    format!("the drill's record names strike '{id}', which is not stored")
                })
        })
        .collect()
}

/// What one call writes, gathered before anything is (the module doc's order).
#[derive(Default)]
struct Plan {
    /// The strikes to append, in the order they landed, each with its crop's JPEG.
    strikes: Vec<(TapeStrike, Option<Vec<u8>>)>,
    /// A strike to hold for the golfer, and its crop's JPEG.
    pending: Option<(Pending, Vec<u8>)>,
    /// The club's next reference, raw RGB.
    reference: Option<Vec<u8>>,
    /// A photo kept in `.app/failed/`: its name and its JPEG.
    failed: Option<(String, Vec<u8>)>,
    /// The pending strike this call resolved, to take away once it is stored.
    resolved: Option<String>,
    /// Why the photo placed nothing, for the card.
    reason: Option<TapeNotRead>,
    /// The result of a block this call finished.
    judged: Option<String>,
}

/// One strike on the card.
fn card(strike: &TapeStrike) -> TapeStrikeCard {
    let verdict = match (strike.stage, strike.level_radius_mm) {
        (DrillStage::Differential, Some(radius)) => strike
            .call
            .and_then(|call| call_made(strike, call, radius).map(|made| call_verdict(call, made))),
        (DrillStage::Ladder | DrillStage::Recentre, Some(radius)) => {
            inside(strike, radius).map(|inside| inside_verdict(inside, radius))
        }
        _ => None,
    };
    TapeStrikeCard {
        strike_id: strike.strike_id.clone(),
        position: position_line(strike),
        verdict,
    }
}

/// The next strike id (C8).
fn issue(next: &mut DrillState, install: &str) -> String {
    next.strikes_issued += 1;
    strike_id(install, next.strikes_issued)
}

/// The baseline's R50 (C11): the median of its strikes' distances from the label's centre, or `None`
/// when a strike declared off the tape leaves the median unknown (the module doc, and the plan's P15
/// finding 4).
///
/// A declared strike lies past `inscribed_mm`, the label's inscribed radius, so it sorts after every
/// distance at or inside it. The median is known when the order statistics `stats::percentile` reads
/// all lie at or inside it: those are then the placed distances, in order. Each declared strike stands
/// in as `f64::MAX`, which sorts last and, when it is read at all, is read with no weight, so the
/// answer is `percentile`'s own.
pub(crate) fn baseline_r50(block: &[TapeStrike], inscribed_mm: f64) -> Option<f64> {
    let mut distances: Vec<f64> = block
        .iter()
        .filter_map(TapeStrike::position)
        .map(|(h, v)| h.hypot(v))
        .collect();
    let declared = block
        .iter()
        .filter(|strike| strike.source == Some(StrikeSource::DeclaredOffTape))
        .count();
    distances.sort_by(f64::total_cmp);
    if declared > 0 {
        let n = distances.len() + declared;
        // `percentile`'s position for the median, and the highest index it reads with weight.
        let highest = (0.5 * (n - 1) as f64).ceil() as usize;
        if distances
            .get(highest)
            .is_none_or(|&distance| distance > inscribed_mm)
        {
            return None;
        }
    }
    distances.extend(std::iter::repeat_n(f64::MAX, declared));
    stats::percentile(&distances, 0.5)
}

/// The call as stored.
fn stored_call(call: DrillCall) -> StrikeCall {
    match call {
        DrillCall::Heel => StrikeCall::Heel,
        DrillCall::Centre => StrikeCall::Centre,
        DrillCall::Toe => StrikeCall::Toe,
    }
}

/// The reader's reason as a stored one, by the wire name both spell (`contracts::drill`'s
/// `TapeNotRead` doc), as `golf-lab read-tape` maps it.
fn stored_reason(reason: TapeRefusal) -> Result<TapeNotRead, String> {
    serde_json::from_value(serde_json::Value::from(reason.as_str())).map_err(|_| {
        format!(
            "the reader gave '{}', which no stored strike can name",
            reason.as_str()
        )
    })
}

/// The photo Swift wrote for this call, Rust's once [`TakenPhoto::of`] accepts its path, and
/// removed when this is dropped, whatever the answer (the module doc).
struct TakenPhoto(PathBuf);

impl TakenPhoto {
    /// Refuses a path that is not a file directly under `<root>/.app/tmp/`, touching nothing.
    fn of(root: &Path, raw_path: &str) -> Result<TakenPhoto, String> {
        let path = PathBuf::from(raw_path);
        let tmp = Dirs::of(root).app.join(TMP_DIR);
        if path.parent() != Some(tmp.as_path()) || path.file_name().is_none() {
            return Err(format!(
                "a photo is taken from {}, where Swift writes it, and not from {raw_path}",
                tmp.display()
            ));
        }
        Ok(TakenPhoto(path))
    }
}

impl Drop for TakenPhoto {
    fn drop(&mut self) {
        // Already gone is what this wants; any other failure leaves a file in `tmp/`, which a
        // refusal here could not mend.
        let _ = fs::remove_file(&self.0);
    }
}

/// Raw RGBA, `width × height` pixels with no header, as RGB, or `None` when the file cannot be read
/// or is not that size: `decode_failed`. The alpha is dropped in place, since a 12 MP photo is 48 MB
/// before it and the finder holds more than that again (the plan's P9 finding 13).
fn decode_raw(path: &Path, width: i64, height: i64) -> Option<Frame> {
    let width = usize::try_from(width).ok().filter(|width| *width > 0)?;
    let height = usize::try_from(height).ok().filter(|height| *height > 0)?;
    let pixels = width.checked_mul(height)?;
    let mut bytes = fs::read(path).ok()?;
    if bytes.len() != pixels.checked_mul(4)? {
        return None;
    }
    for pixel in 1..pixels {
        bytes.copy_within(pixel * 4..pixel * 4 + 3, pixel * 3);
    }
    bytes.truncate(pixels * 3);
    Some(Frame {
        width,
        height,
        rgb: bytes,
    })
}

/// A frame as a JPEG at `quality` (C16's `jpeg-encoder`).
fn jpeg(frame: &Frame, quality: u8) -> Result<Vec<u8>, String> {
    let too_large = || {
        format!(
            "a {} by {} image is too large for a JPEG",
            frame.width, frame.height
        )
    };
    let width = u16::try_from(frame.width).map_err(|_| too_large())?;
    let height = u16::try_from(frame.height).map_err(|_| too_large())?;
    let mut bytes = Vec::new();
    jpeg_encoder::Encoder::new(&mut bytes, quality)
        .encode(&frame.rgb, width, height, jpeg_encoder::ColorType::Rgb)
        .map_err(|failed| format!("could not encode an image as JPEG: {failed}"))?;
    Ok(bytes)
}

// --- Pending strikes ------------------------------------------------------------------------------

/// A strike held for the golfer (the module doc). Never crosses to Dart and never leaves the phone.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pending {
    kind: PendingKind,
    /// Which way the club lay across the crop, which turns a tap into the reading frame. Absent only
    /// for a `no_new_mark` whose sides were not found again, which then takes no tap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sides: Option<Sides>,
    /// The strike as it is stored if nobody resolves it: not read, with its reason and its crop.
    strike: TapeStrike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PendingKind {
    NeedsPick,
    NoNewMark,
}

/// An `Orientation`, which `impact` keeps free of `serde`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Sides {
    toe_printed_right: bool,
    top_line_printed_top: bool,
}

impl Sides {
    fn of(orientation: Orientation) -> Sides {
        Sides {
            toe_printed_right: orientation.toe_printed_right,
            top_line_printed_top: orientation.top_line_printed_top,
        }
    }

    fn orientation(self) -> Orientation {
        Orientation {
            toe_printed_right: self.toe_printed_right,
            top_line_printed_top: self.top_line_printed_top,
        }
    }
}

fn pending_path(root: &Path, id: &str, suffix: &str) -> PathBuf {
    Dirs::of(root)
        .app
        .join(PENDING_DIR)
        .join(format!("{id}.{suffix}"))
}

/// The pending strike under `id`, or `None`. The id names files, so it is held to a strike id's
/// characters first, as the drill store holds a strike's.
fn read_pending(root: &Path, id: &str) -> Result<Option<Pending>, String> {
    let well_formed = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !well_formed {
        return Err(format!("'{id}' is not a strike id"));
    }
    match fs::read(pending_path(root, id, PENDING_RECORD)) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|refused| {
            format!("the strike waiting under '{id}' cannot be read: {refused}")
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "could not read the strike waiting under '{id}': {error}"
        )),
    }
}

/// The crop first, then the record that names it, each renamed into place whole.
fn write_pending(root: &Path, pending: &Pending, crop: &[u8]) -> io::Result<()> {
    let id = &pending.strike.strike_id;
    write_file(&pending_path(root, id, PENDING_JPEG), crop)?;
    let record = serde_json::to_vec_pretty(pending).map_err(io::Error::other)?;
    write_file(&pending_path(root, id, PENDING_RECORD), &record)
}

/// The record first, so a crash between the two leaves a crop no record names, never a record
/// naming a crop that is gone.
fn remove_pending(root: &Path, id: &str) -> Result<(), String> {
    for suffix in [PENDING_RECORD, PENDING_JPEG] {
        match fs::remove_file(pending_path(root, id, suffix)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("could not take away strike '{id}': {error}")),
        }
    }
    Ok(())
}

/// The club's pending strike stored as it stands, and taken away (the module doc): a new photo of
/// the club, or a change of its stage (P19), discards it. One a call cut off had already stored is
/// only taken away.
pub(crate) fn discard_pending(root: &Path, store: &DrillStore, club: ClubId) -> Result<(), String> {
    let dir = Dirs::of(root).app.join(PENDING_DIR);
    let mut names: Vec<String> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .collect(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("could not look for a waiting strike: {error}")),
    };
    names.sort();
    let held: Vec<String> = store
        .strikes(club)
        .map_err(|failed| failed.to_string())?
        .strikes
        .into_iter()
        .map(|strike| strike.strike_id)
        .collect();
    for name in names {
        let Some(id) = name.strip_suffix(&format!(".{PENDING_RECORD}")) else {
            continue;
        };
        // One that cannot be read is left, since nothing can store it or resolve it.
        let Ok(Some(pending)) = read_pending(root, id) else {
            continue;
        };
        if pending.strike.player_id != store.player_id() || pending.strike.club != club {
            continue;
        }
        if !held.iter().any(|held| held == id) {
            let mut strike = pending.strike;
            match fs::read(pending_path(root, id, PENDING_JPEG)) {
                Ok(crop) if strike.crop.is_some() => {
                    store
                        .put_crop(id, &crop)
                        .map_err(|failed| format!("could not store a strike's crop: {failed}"))?;
                }
                _ => strike.crop = None,
            }
            store
                .append_strike(&strike)
                .map_err(|failed| format!("could not store a strike: {failed}"))?;
        }
        remove_pending(root, id)?;
    }
    Ok(())
}

/// `bytes` at `path` through a `.tmp` beside it and a rename, as every store here writes.
fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(placed: &[f64], declared: usize) -> Vec<TapeStrike> {
        let shell = |n: usize| TapeStrike {
            strike_id: strike_id("a1b2", n as i64 + 1),
            player_id: "aaron".into(),
            club: ClubId::SevenIron,
            category: ClubCategory::MidIron,
            label_type: "strikepro_iron".into(),
            label_id: "a1b2-L1".into(),
            n_on_label: 1,
            captured_at: Timestamp::parse("2026-11-13T07:00:00Z").unwrap(),
            stage: DrillStage::Baseline,
            level_radius_mm: None,
            call: None,
            offset_h_mm: None,
            offset_v_mm: None,
            source: Some(StrikeSource::DeclaredOffTape),
            not_read: None,
            reader_version: TAPE_READER_VERSION,
            crop: None,
        };
        let mut strikes: Vec<TapeStrike> = placed
            .iter()
            .enumerate()
            .map(|(n, &distance)| TapeStrike {
                offset_h_mm: Some(distance),
                offset_v_mm: Some(0.0),
                source: Some(StrikeSource::Read),
                ..shell(n)
            })
            .collect();
        strikes.extend((0..declared).map(|n| shell(placed.len() + n)));
        strikes
    }

    /// With no declaration it is the plain median of ten, `percentile`'s mean of the fifth and sixth.
    #[test]
    fn a_baseline_with_every_strike_placed_has_the_median_of_its_ten_distances() {
        let placed = [9.0, 1.0, 2.0, 8.0, 3.0, 4.0, 7.0, 5.0, 6.0, 20.0];
        assert_eq!(baseline_r50(&baseline(&placed, 0), 12.0), Some(5.5));
    }

    /// Four declarations, and the sixth smallest placed distance inside the label: the median is the
    /// placed fifth and sixth, whatever the declared strikes' distances were.
    #[test]
    fn declarations_past_the_median_leave_it_known() {
        let placed = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert_eq!(baseline_r50(&baseline(&placed, 4), 12.0), Some(5.5));
    }

    /// Five declarations reach the sixth order statistic, and a sixth placed distance past the
    /// inscribed radius (a label's corner) could sort either side of a declared one.
    #[test]
    fn a_declaration_the_median_could_read_leaves_it_unknown() {
        assert_eq!(
            baseline_r50(&baseline(&[1.0, 2.0, 3.0, 4.0, 5.0], 5), 12.0),
            None
        );
        let corner = [1.0, 2.0, 3.0, 4.0, 5.0, 13.0];
        assert_eq!(baseline_r50(&baseline(&corner, 4), 12.0), None);
        assert_eq!(baseline_r50(&baseline(&corner, 4), 13.0), Some(9.0));
    }
}
