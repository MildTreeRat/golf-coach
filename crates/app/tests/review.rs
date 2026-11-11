//! `correct_shot`, driven as Dart drives it: a card in hand, what a person typed, and a card back,
//! against a scratch data root. [M38 P7]
//!
//! `golf_core::session`'s own tests pin each clause of C11 on one hand-built shot. These hold the same
//! rules to what the reader really stores, and to the card a person is really looking at.
//!
//! **Everything read off the six captures here is provisional: pasted copies** (rule 12). They are
//! chat-pasted JPEGs and not the originals, by M33's decision of 2026-11-04. Those files are read and
//! never written (rule 5).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use app::api::record::{record_photo, CardRow, PhotoIn, RecordOutcome, RowState, ShotCard};
use app::api::review::{correct_shot, FieldEdit};
use app::api::setup::{create_golfer, get_state, init_state, set_state, Handedness};
use app::{engine_versions, Dirs};
use contracts::shot::ShotData;
use contracts::shot_result::ShotResult;
use storage::corpus::read_corpus;
use storage::shot_store::ShotStore;
use storage::state::load_shot_analysis;
use tempfile::TempDir;

const CAPTURES: [&str; 6] = [
    "IMG_2889",
    "IMG_2891",
    "IMG_2892",
    "IMG_2893",
    "IMG_2894",
    "2026-10-07-1",
];
const TAKEN: &str = "2026-11-04T20:42:00-05:00";
const NOW: &str = "2026-11-04T20:43:00-05:00";
const LATER: &str = "2026-11-04T20:44:00-05:00";

fn spike() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spikes/vision-ocr")
}

fn raw_boxes(id: &str) -> String {
    let path = spike().join("boxes/C0/raw").join(format!("{id}.json"));
    fs::read_to_string(&path).unwrap_or_else(|failed| panic!("{}: {failed}", path.display()))
}

/// A scratch data root with a golfer and a club chosen, as first-run setup leaves it.
struct Phone {
    dir: TempDir,
    root: String,
}

impl Phone {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let root = dir.path().join("processed").display().to_string();
        init_state(root.clone(), "a1b2".to_string()).expect("an install id");
        create_golfer(
            root.clone(),
            "Aaron".to_string(),
            Handedness::Right,
            NOW.to_string(),
        )
        .expect("a golfer");
        let mut state = get_state(root.clone());
        state.player_id = Some("aaron".to_string());
        state.club = Some("7i".to_string());
        set_state(root.clone(), state).expect("a state set_state takes");
        Phone { dir, root }
    }

    fn feed_text(&self, id: &str, boxes_json: String) -> RecordOutcome {
        let tmp = self.dir.path().join("tmp");
        fs::create_dir_all(&tmp).expect("tmp");
        let to = tmp.join(format!("{id}.jpg"));
        fs::copy(spike().join("captures").join(format!("{id}.jpg")), &to).expect("a capture");
        record_photo(PhotoIn {
            root: self.root.clone(),
            path: to.display().to_string(),
            boxes_json,
            captured_at: TAKEN.to_string(),
            now: NOW.to_string(),
            challenge_call: None,
        })
        .expect("a recorded photo")
    }

    /// A capture read as M33 read it.
    fn card(&self, id: &str) -> ShotCard {
        self.feed_text(id, raw_boxes(id)).card.expect("a card")
    }

    /// `IMG_2889` with one value box edited so a physics check flags it, as `tests/record.rs` does,
    /// since no capture trips one.
    fn flagged(&self) -> ShotCard {
        let text = raw_boxes("IMG_2889").replace("\"1.08\"", "\"2.08\"");
        let card = self.feed_text("IMG_2889", text).card.expect("a card");
        assert!(card.flagged);
        card
    }

    fn correct(&self, card: &ShotCard, edits: Vec<FieldEdit>) -> Result<ShotCard, String> {
        correct_shot(
            self.root.clone(),
            card.session_id.clone(),
            card.swing_id.clone(),
            edits,
            LATER.to_string(),
        )
    }

    fn stored(&self, card: &ShotCard) -> ShotData {
        let shots = ShotStore::new(Dirs::of(&self.root).shots);
        shots
            .all()
            .into_iter()
            .find(|shot| shot.shot_id == format!("{}-{}", card.session_id, card.swing_id))
            .expect("a stored shot")
    }

    fn analysis(&self, card: &ShotCard) -> ShotResult {
        let swing = Dirs::of(&self.root)
            .sessions
            .join(&card.session_id)
            .join(&card.swing_id);
        load_shot_analysis(&swing).expect("an analysis")
    }

    fn corrections(&self, card: &ShotCard) -> Vec<(String, String)> {
        let shot = self.stored(card);
        let provenance = shot.provenance.expect("a read's provenance");
        provenance.corrections.into_iter().collect()
    }

    /// Everything under the data root and its bytes.
    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).expect("a directory") {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    walk(&path, into);
                } else {
                    let bytes = fs::read(&path).expect("a file");
                    into.insert(path, bytes);
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(Path::new(&self.root), &mut files);
        files
    }
}

fn row<'a>(card: &'a ShotCard, field: &str) -> &'a CardRow {
    card.rows
        .iter()
        .find(|row| row.field == field)
        .unwrap_or_else(|| panic!("no row for {field}"))
}

fn typed(field: &str, printed: &str, letter: Option<&str>) -> FieldEdit {
    FieldEdit {
        field: field.to_string(),
        printed: Some(printed.to_string()),
        letter: letter.map(str::to_string),
    }
}

fn entry_of(field: &str, as_read: &str) -> (String, String) {
    (field.to_string(), as_read.to_string())
}

// ------------------------------------------------------------------------------------ the review

/// D9's whole path on a real read: the misread is corrected, the flag goes, the row says so, the map
/// holds what was read, the warning stays, and every later reader agrees. provisional: pasted copies.
#[test]
fn a_flagged_read_is_released_by_a_review_correcting_it() {
    let phone = Phone::new();
    let card = phone.flagged();
    let warned = card.warnings.clone();
    assert!(!warned.is_empty());
    assert!(
        read_corpus(&Dirs::of(&phone.root).sessions, "aaron", engine_versions()).swings[0]
            .shot_needs_review
    );

    let reviewed = phone
        .correct(&card, vec![typed("smash_factor", "1.08", None)])
        .expect("a review");

    assert!(!reviewed.flagged);
    assert_eq!(reviewed.warnings, warned);
    let corrected = row(&reviewed, "smash_factor");
    assert_eq!(corrected.state, RowState::Corrected);
    assert_eq!(corrected.value, Some(1.08));
    assert_eq!(corrected.display.as_deref(), Some("1.08"));
    assert_eq!(row(&reviewed, "carry_distance").state, RowState::Read);
    assert_eq!(
        phone.corrections(&reviewed),
        [entry_of("smash_factor", "2.08")]
    );

    let stored = phone.stored(&reviewed);
    assert_eq!(phone.analysis(&reviewed).shot, stored);
    let corpus = read_corpus(&Dirs::of(&phone.root).sessions, "aaron", engine_versions());
    assert!(!corpus.swings[0].shot_needs_review);
    assert_eq!(corpus.swings[0].shot.as_ref(), Some(&stored));
}

/// D9: a flagged shot with nothing wrong in it still has to be released. provisional: pasted copies.
#[test]
fn a_flagged_read_is_released_by_a_save_with_no_edits() {
    let phone = Phone::new();
    let card = phone.flagged();
    let reviewed = phone.correct(&card, Vec::new()).expect("a review");
    assert!(!reviewed.flagged);
    assert_eq!(reviewed.rows, card.rows, "nothing about a row moved");
    assert_eq!(phone.corrections(&reviewed), []);
}

/// M33's named case: the withheld sign no flag surfaces, typed in by a person off the photo. The sign
/// control's letters are the row's own. provisional: pasted copies.
#[test]
fn img_2893s_withheld_sidespin_can_be_typed_in_with_its_letter() {
    let phone = Phone::new();
    let card = phone.card("IMG_2893");
    let sidespin = row(&card, "sidespin");
    assert_eq!(sidespin.state, RowState::NotRead);
    assert_eq!(
        (sidespin.printed.as_deref(), sidespin.letter.as_deref()),
        (None, None)
    );
    let (draw, fade) = sidespin.sign_letters.clone().expect("D and F");

    let reviewed = phone
        .correct(&card, vec![typed("sidespin", "812", Some(&draw))])
        .expect("a value typed in");
    let sidespin = row(&reviewed, "sidespin");
    assert_eq!(sidespin.state, RowState::Corrected);
    assert_eq!(sidespin.value, Some(-812.0));
    assert_eq!(sidespin.display.as_deref(), Some("812 D"));
    assert_eq!(phone.corrections(&reviewed), [entry_of("sidespin", "")]);

    let reviewed = phone
        .correct(&card, vec![typed("sidespin", "812", Some(&fade))])
        .expect("the other letter");
    assert_eq!(row(&reviewed, "sidespin").value, Some(812.0));
    assert_eq!(phone.corrections(&reviewed), [entry_of("sidespin", "")]);
}

/// P6 finding 5, settled, on the card's own numbers: a person sees `Apex Ft` and types feet. The map
/// stays in `ShotData`'s yards, because C11 says it is the stored value. provisional: pasted copies.
#[test]
fn the_apex_is_typed_in_the_feet_the_card_shows() {
    let phone = Phone::new();
    let card = phone.card("IMG_2889");
    let apex = row(&card, "apex_height");
    assert_eq!(apex.label, "Apex Ft");
    let as_read = apex.value.expect("an apex");

    let reviewed = phone
        .correct(&card, vec![typed("apex_height", "90", None)])
        .expect("an edit");

    let apex = row(&reviewed, "apex_height");
    assert_eq!(apex.value, Some(30.0));
    assert_eq!(apex.printed.as_deref(), Some("90"));
    assert_eq!(
        phone.corrections(&reviewed),
        [entry_of("apex_height", &pyfmt::repr(as_read))]
    );
}

/// The pin on the whole conversion. A person who types what each row shows has changed nothing, on
/// every cell M33 read: same bits, no entry, no byte moved. Any drift in `printed` against the
/// parser's sign, unit or digits fails here. provisional: pasted copies.
#[test]
fn typing_back_what_every_row_shows_moves_no_byte() {
    let phone = Phone::new();
    let mut cells = 0;
    for id in CAPTURES {
        let card = phone.card(id);
        let edits: Vec<FieldEdit> = card
            .rows
            .iter()
            .filter_map(|row| {
                let printed = row.printed.as_deref()?;
                Some(typed(&row.field, printed, row.letter.as_deref()))
            })
            .collect();
        cells += edits.len();
        let before = phone.files();
        let reviewed = phone
            .correct(&card, edits)
            .unwrap_or_else(|refused| panic!("{id}: {refused}"));
        assert_eq!(reviewed, card, "{id}");
        let after = phone.files();
        let moved: Vec<&PathBuf> = after
            .iter()
            .filter(|(path, bytes)| before.get(*path) != Some(*bytes))
            .map(|(path, _)| path)
            .collect();
        assert_eq!(moved, Vec::<&PathBuf>::new(), "{id}");
    }
    // M33's summary, so this cannot pass by reading fewer cells than the run did.
    assert_eq!(cells, 71);
}

// --------------------------------------------------------------------------------- what a row says

/// `printed` and `letter` are what a review fills itself from, so they have to be `display` exactly.
/// provisional: pasted copies.
#[test]
fn printed_and_letter_are_display_in_two_pieces() {
    let phone = Phone::new();
    for id in CAPTURES {
        let card = phone.card(id);
        for row in &card.rows {
            let joined = match (&row.printed, &row.letter) {
                (Some(printed), Some(letter)) => Some(format!("{printed} {letter}")),
                (Some(printed), None) => Some(printed.clone()),
                (None, None) => None,
                (None, Some(letter)) => panic!("{id} {}: a letter on nothing: {letter}", row.field),
            };
            assert_eq!(row.display, joined, "{id} {}", row.field);
            if let Some(letter) = &row.letter {
                let (minus, plus) = row.sign_letters.clone().expect("the tile's letters");
                assert!(*letter == minus || *letter == plus, "{id} {}", row.field);
            }
        }
    }
}

/// The parser's rule, on the card: a lit tile showing zero lights nothing, so it shows nothing, where
/// a `0 R` would be a letter the screen never lit. The control still offers the two. provisional:
/// pasted copies.
#[test]
fn a_zero_on_a_lit_tile_shows_no_letter() {
    let phone = Phone::new();
    let card = phone.card("IMG_2889");
    let reviewed = phone
        .correct(&card, vec![typed("launch_direction", "0", None)])
        .expect("a zero");
    let direction = row(&reviewed, "launch_direction");
    assert_eq!(direction.value, Some(0.0));
    assert_eq!(direction.display.as_deref(), Some("0"));
    assert_eq!(direction.letter, None);
    assert!(direction.sign_letters.is_some());
}

// --------------------------------------------------------------------------------------- refusals

/// What P22 shows inline: the sentence, and a store unmoved. provisional: pasted copies.
#[test]
fn a_refusal_reaches_dart_in_its_own_words_and_writes_nothing() {
    let phone = Phone::new();
    let card = phone.flagged();
    let before = phone.files();
    for (edit, said) in [
        (
            typed("carry_distance", "8S.2", None),
            "Carry Yd: '8S.2' is not a number",
        ),
        (
            typed("carry_distance", "NaN", None),
            "Carry Yd: 'NaN' is not a number",
        ),
        (typed("impact_offset_h", "1", None), "'impact_offset_h'"),
        (typed("launch_direction", "2.2", None), "L or R"),
    ] {
        let refused = phone.correct(&card, vec![edit]).expect_err(said);
        assert!(refused.contains(said), "{refused}");
        assert_eq!(phone.files(), before, "{said} wrote something");
    }
}

/// C2: a time with no offset is refused, as every time crossing from Dart is, and this one is Dart's
/// obvious `DateTime.now().toIso8601String()`. provisional: pasted copies.
#[test]
fn a_time_with_no_offset_is_refused_and_writes_nothing() {
    let phone = Phone::new();
    let card = phone.flagged();
    let before = phone.files();
    let refused = correct_shot(
        phone.root.clone(),
        card.session_id.clone(),
        card.swing_id.clone(),
        Vec::new(),
        "2026-11-04T20:44:00.123456".to_string(),
    )
    .expect_err("no offset");
    assert!(refused.contains("offset"), "{refused}");
    assert_eq!(phone.files(), before);
}

/// Two calls reading the file, changing it and writing it would lose one's edit, and a stray second
/// tap on Save is how Dart could make them. Each of these changes a field of its own, so a lost one
/// shows. provisional: pasted copies.
#[test]
fn overlapping_corrections_each_land() {
    let phone = Phone::new();
    let card = phone.card("IMG_2889");
    let fields = [
        ("total_distance", "201"),
        ("carry_distance", "202"),
        ("club_head_speed", "203"),
        ("ball_speed", "204"),
        ("club_path", "205"),
        ("club_face_angle", "206"),
        ("launch_angle", "207"),
        ("smash_factor", "208"),
        ("backspin", "209"),
    ];
    std::thread::scope(|scope| {
        for (field, printed) in fields {
            let (phone, card) = (&phone, &card);
            scope.spawn(move || {
                phone
                    .correct(card, vec![typed(field, printed, None)])
                    .expect("an edit");
            });
        }
    });
    assert_eq!(phone.corrections(&card).len(), fields.len());
    let stored = serde_json::to_value(phone.stored(&card)).expect("a shot");
    for (field, printed) in fields {
        assert_eq!(
            stored[field].as_f64(),
            Some(printed.parse::<f64>().unwrap()),
            "{field}"
        );
    }
}
