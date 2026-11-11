//! The setup facade, driven as Dart drives it: strings in, views out, against a scratch data root.
//! [M38 P5]

use std::fs;
use std::path::PathBuf;

use app::api::setup::{
    all_clubs, bag_slots, create_golfer, get_state, init_state, list_golfers, remove_bag_slot,
    set_bag_slot, set_state, AppState, Appearance, BagSlot, GolferView, Handedness, PracticeMode,
    TargetShape,
};
use app::Dirs;
use contracts::club::ClubId;
use contracts::club_spec::ClubSpec;
use storage::bag_store::BagStore;
use tempfile::TempDir;

const NOW: &str = "2026-11-04T20:42:00-05:00";
const LATER: &str = "2026-11-04T20:43:00-05:00";

/// `<scratch>/processed`, which does not exist yet: on a first run nothing has made it.
fn scratch() -> (TempDir, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let root = dir.path().join("processed").display().to_string();
    (dir, root)
}

fn state_file(root: &str) -> PathBuf {
    PathBuf::from(root).join(".app").join("state.json")
}

fn stored_bag(root: &str, player_id: &str) -> contracts::bag::Bag {
    BagStore::new(Dirs::of(root).golfers)
        .get(player_id)
        .expect("a stored bag")
}

fn set(root: &str, club: &str, loft: Option<f64>, now: &str) -> Result<Vec<BagSlot>, String> {
    set_bag_slot(
        root.to_string(),
        "aaron".to_string(),
        club.to_string(),
        loft,
        now.to_string(),
    )
}

fn remove(root: &str, club: &str, now: &str) -> Result<Vec<BagSlot>, String> {
    remove_bag_slot(
        root.to_string(),
        "aaron".to_string(),
        club.to_string(),
        now.to_string(),
    )
}

fn clubs(slots: &[BagSlot]) -> Vec<&str> {
    slots.iter().map(|slot| slot.club.as_str()).collect()
}

// --- Golfers ---

#[test]
fn a_new_golfer_is_keyed_by_the_slug_of_the_name_and_shown_as_typed() {
    let (_dir, root) = scratch();
    let golfer = create_golfer(
        root.clone(),
        "  Aaron Sierra ".to_string(),
        Handedness::Left,
        NOW.to_string(),
    )
    .unwrap();
    let expected = GolferView {
        player_id: "aaron-sierra".to_string(),
        display_name: "Aaron Sierra".to_string(),
        handedness: Handedness::Left,
    };
    assert_eq!(golfer, expected);
    assert_eq!(list_golfers(root), vec![expected]);
}

/// The store's load-bearing rule through the facade: a form submitted with the other side ticked
/// must not flip a stored handedness, and a retyped name must not split one history in two.
#[test]
fn a_known_golfers_handedness_wins_over_the_one_given_and_nothing_is_rewritten() {
    let (_dir, root) = scratch();
    let first = |name: &str, handedness, now: &str| {
        create_golfer(root.clone(), name.to_string(), handedness, now.to_string()).unwrap()
    };
    first("Aaron", Handedness::Left, NOW);
    let file = Dirs::of(&root).golfers.join("aaron.golfer.json");
    let written = fs::read_to_string(&file).unwrap();

    let again = first(" AARON ", Handedness::Right, LATER);

    assert_eq!(again.player_id, "aaron");
    assert_eq!(again.display_name, "Aaron");
    assert_eq!(again.handedness, Handedness::Left);
    assert_eq!(fs::read_to_string(&file).unwrap(), written);
    assert_eq!(list_golfers(root.clone()).len(), 1);
}

#[test]
fn golfers_list_by_display_name_and_none_is_an_empty_list() {
    let (_dir, root) = scratch();
    assert!(list_golfers(root.clone()).is_empty());
    for name in ["Zed", "aaron", "Mia"] {
        create_golfer(
            root.clone(),
            name.to_string(),
            Handedness::Right,
            NOW.to_string(),
        )
        .unwrap();
    }
    let names: Vec<String> = list_golfers(root)
        .into_iter()
        .map(|golfer| golfer.display_name)
        .collect();
    assert_eq!(names, ["aaron", "Mia", "Zed"]);
}

/// P19 shows this text under the name field, so it has to be the store's sentence.
#[test]
fn a_name_with_nothing_to_key_on_is_refused_in_the_stores_words_and_writes_nothing() {
    let (_dir, root) = scratch();
    let refused = create_golfer(
        root.clone(),
        "!!!".to_string(),
        Handedness::Right,
        NOW.to_string(),
    )
    .unwrap_err();
    assert_eq!(refused, "name '!!!' has no usable characters for an id");
    assert!(!Dirs::of(&root).golfers.exists());
}

#[test]
fn a_time_with_no_offset_is_refused_before_a_golfer_or_a_bag_is_written() {
    let (_dir, root) = scratch();
    let naive = "2026-11-04T20:42:00";
    let refused = create_golfer(
        root.clone(),
        "Aaron".to_string(),
        Handedness::Right,
        naive.to_string(),
    )
    .unwrap_err();
    assert!(refused.contains("offset"), "{refused}");
    let refused = set(&root, "7i", None, naive).unwrap_err();
    assert!(refused.contains("offset"), "{refused}");
    assert!(!Dirs::of(&root).golfers.exists());
}

// --- Clubs and bags ---

/// Sorting by the wire name is the bug bag order exists to prevent: `3w` would land after `2h`.
#[test]
fn all_clubs_is_every_club_in_bag_order_and_its_own_spelling() {
    let clubs = all_clubs();
    let expected: Vec<&str> = ClubId::ALL.iter().map(|club| club.as_str()).collect();
    assert_eq!(clubs, expected);
    assert_eq!(&clubs[..3], ["driver", "3w", "5w"]);
    assert_eq!(clubs.last().map(String::as_str), Some("putter"));
}

#[test]
fn a_slot_is_declared_with_a_loft_or_without_one_and_without_one_is_not_zero() {
    let (_dir, root) = scratch();
    set(&root, "pw", None, NOW).unwrap();
    let answered = set(&root, "7i", Some(34.5), NOW).unwrap();

    let seven = BagSlot {
        club: "7i".to_string(),
        loft_deg: Some(34.5),
        loft_display: Some("34.5°".to_string()),
    };
    let wedge = BagSlot {
        club: "pw".to_string(),
        loft_deg: None,
        loft_display: None,
    };
    assert_eq!(answered, [seven.clone(), wedge.clone()], "bag order");
    assert_eq!(bag_slots(root, "aaron".to_string()), [seven, wedge]);
}

#[test]
fn a_whole_number_loft_displays_as_a_person_says_it() {
    let (_dir, root) = scratch();
    let answered = set(&root, "7i", Some(34.0), NOW).unwrap();
    assert_eq!(answered[0].loft_display.as_deref(), Some("34°"));
}

/// D7, on disk: the facade adds nothing a person did not say, so a blank stays a blank that the lab
/// reads as nobody having said.
#[test]
fn a_slot_on_disk_holds_the_club_and_the_loft_and_nothing_else_declared() {
    let (_dir, root) = scratch();
    set(&root, "7i", Some(34.0), NOW).unwrap();
    let entry = &stored_bag(&root, "aaron").entries[&ClubId::SevenIron];
    let expected = ClubSpec {
        loft_deg: Some(34.0),
        ..ClubSpec::blank(ClubId::SevenIron)
    };
    assert_eq!(entry.spec, expected);
    assert_eq!(entry.recorded_at, NOW.parse().unwrap());
    assert_eq!(entry.retired_at, None);
    assert_eq!(entry.provenance, None);
}

#[test]
fn declaring_the_same_loft_again_writes_nothing() {
    let (_dir, root) = scratch();
    set(&root, "7i", Some(34.0), NOW).unwrap();
    let file = BagStore::new(Dirs::of(&root).golfers).path_for("aaron");
    let written = fs::read_to_string(&file).unwrap();
    let answered = set(&root, "7i", Some(34.0), LATER).unwrap();
    assert_eq!(answered[0].loft_deg, Some(34.0));
    assert_eq!(fs::read_to_string(&file).unwrap(), written);
}

#[test]
fn changing_a_loft_shelves_the_old_one() {
    let (_dir, root) = scratch();
    set(&root, "7i", Some(34.0), NOW).unwrap();
    let answered = set(&root, "7i", Some(30.5), LATER).unwrap();
    assert_eq!(answered[0].loft_deg, Some(30.5));
    let bag = stored_bag(&root, "aaron");
    let shelved = bag.retired_for(ClubId::SevenIron);
    assert_eq!(shelved.len(), 1);
    assert_eq!(shelved[0].spec.loft_deg, Some(34.0));
    assert_eq!(shelved[0].retired_at, Some(LATER.parse().unwrap()));
}

#[test]
fn a_removed_slot_leaves_the_picker_and_is_retired_and_not_lost() {
    let (_dir, root) = scratch();
    set(&root, "7i", Some(34.0), NOW).unwrap();
    set(&root, "pw", None, NOW).unwrap();

    let answered = remove(&root, "7i", LATER).unwrap();

    assert_eq!(clubs(&answered), ["pw"]);
    assert_eq!(clubs(&bag_slots(root.clone(), "aaron".to_string())), ["pw"]);
    let bag = stored_bag(&root, "aaron");
    assert!(!bag.entries.contains_key(&ClubId::SevenIron));
    let shelved = bag.retired_for(ClubId::SevenIron);
    assert_eq!(shelved.len(), 1);
    assert_eq!(shelved[0].spec.loft_deg, Some(34.0));
    assert_eq!(shelved[0].retired_at, Some(LATER.parse().unwrap()));
}

#[test]
fn removing_a_slot_that_is_not_in_the_bag_refuses_nothing_and_writes_nothing() {
    let (_dir, root) = scratch();
    assert!(remove(&root, "7i", NOW).unwrap().is_empty(), "no bag");
    assert!(!Dirs::of(&root).golfers.exists());

    set(&root, "pw", None, NOW).unwrap();
    let file = BagStore::new(Dirs::of(&root).golfers).path_for("aaron");
    let written = fs::read_to_string(&file).unwrap();
    assert_eq!(clubs(&remove(&root, "7i", LATER).unwrap()), ["pw"]);
    assert_eq!(fs::read_to_string(&file).unwrap(), written);
}

#[test]
fn no_bag_and_an_unreadable_bag_both_list_no_slots() {
    let (_dir, root) = scratch();
    assert!(bag_slots(root.clone(), "aaron".to_string()).is_empty());
    set(&root, "7i", None, NOW).unwrap();
    let file = BagStore::new(Dirs::of(&root).golfers).path_for("aaron");
    fs::write(file, "{not json").unwrap();
    assert!(bag_slots(root, "aaron".to_string()).is_empty());
}

/// JSON cannot spell these. Without the guard the answer says `NaN°`, the file says `null`, and the
/// same `NaN` again shelves a phantom, since the file reread holds no loft and the new entry holds
/// one. So the second half compares the whole file, which holds both the live loft and the shelf.
#[test]
fn a_loft_that_is_not_finite_is_refused_in_the_facades_words_and_changes_no_bag() {
    let (_dir, root) = scratch();
    let not_finite = [
        (f64::NAN, "NaN"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
    ];
    for (loft, spelled) in not_finite {
        assert_eq!(
            set(&root, "7i", Some(loft), NOW).unwrap_err(),
            format!("a loft has to be a real number of degrees, and {spelled} is not")
        );
    }
    assert!(!Dirs::of(&root).golfers.exists(), "no bag was started");

    set(&root, "7i", Some(34.0), NOW).unwrap();
    let file = BagStore::new(Dirs::of(&root).golfers).path_for("aaron");
    let written = fs::read_to_string(&file).unwrap();
    for (loft, _) in not_finite {
        set(&root, "7i", Some(loft), LATER).unwrap_err();
    }
    assert_eq!(fs::read_to_string(&file).unwrap(), written);
}

/// The other half of the same answer: finite is the whole rule, so no range hides in the facade.
/// `3.4` is the typo a range could not catch anyway, and `340` flies as any iron does (P5's finding).
#[test]
fn any_real_loft_stands_as_given_however_unlikely() {
    let (_dir, root) = scratch();
    for (loft, shown) in [
        (340.0, "340°"),
        (-5.0, "-5°"),
        (3.4, "3.4°"),
        (0.0, "0°"),
        (1e-3, "0.001°"),
    ] {
        let answered = set(&root, "7i", Some(loft), NOW).unwrap();
        assert_eq!(answered[0].loft_deg, Some(loft));
        assert_eq!(answered[0].loft_display.as_deref(), Some(shown));
        let read_back = bag_slots(root.clone(), "aaron".to_string());
        assert_eq!(read_back[0].loft_deg, Some(loft), "off the file");
    }
}

/// The one `Validate` refusal a slot can reach today: `Bag`'s slug rule, whose message Dart must see
/// whole. The store's own refusal (a bag it will not overwrite) comes through the same way.
#[test]
fn a_refusal_from_the_contract_or_the_store_reaches_dart_in_its_own_words() {
    let (_dir, root) = scratch();
    let refused = set_bag_slot(
        root.clone(),
        "../aaron".to_string(),
        "7i".to_string(),
        Some(34.0),
        NOW.to_string(),
    )
    .unwrap_err();
    assert!(
        refused.contains("player_id '../aaron' is not a slug"),
        "{refused}"
    );

    set(&root, "7i", None, NOW).unwrap();
    let file = BagStore::new(Dirs::of(&root).golfers).path_for("aaron");
    fs::write(&file, "{not json").unwrap();
    let refused = set(&root, "pw", None, LATER).unwrap_err();
    assert!(refused.ends_with("refusing to overwrite it"), "{refused}");
    let refused = remove(&root, "7i", LATER).unwrap_err();
    assert!(refused.ends_with("refusing to overwrite it"), "{refused}");
    assert_eq!(fs::read_to_string(&file).unwrap(), "{not json");
}

#[test]
fn a_club_not_in_its_own_spelling_is_refused_by_both_slot_calls() {
    let (_dir, root) = scratch();
    for call in [
        set(&root, "7 iron", None, NOW),
        remove(&root, "7 iron", NOW),
    ] {
        let refused = call.unwrap_err();
        assert!(refused.contains("'7 iron' is not a club"), "{refused}");
    }
    assert!(!Dirs::of(&root).golfers.exists());
}

// --- The app's own state ---

fn every_field_set(install_id: Option<&str>) -> AppState {
    AppState {
        install_id: install_id.map(str::to_string),
        player_id: Some("aaron".to_string()),
        club: Some("7i".to_string()),
        mode: PracticeMode::ShotShaping,
        target_shape: Some(TargetShape::Draw),
        challenge: true,
        challenge_shapes: vec![TargetShape::Draw, TargetShape::Fade],
        appearance: Appearance::Light,
        last_export_at: Some(NOW.to_string()),
        last_export_shots: Some(31),
    }
}

/// Each field is named so that a field added to `AppState` has to be given a default here.
#[test]
fn state_on_nothing_answers_the_defaults_and_writes_nothing() {
    let (_dir, root) = scratch();
    assert_eq!(
        get_state(root.clone()),
        AppState {
            install_id: None,
            player_id: None,
            club: None,
            mode: PracticeMode::Fundamentals,
            target_shape: None,
            challenge: false,
            challenge_shapes: Vec::new(),
            appearance: Appearance::Dark,
            last_export_at: None,
            last_export_shots: None,
        }
    );
    assert!(!PathBuf::from(root).exists());
}

#[test]
fn init_state_writes_the_id_it_is_given_when_none_exists() {
    let (_dir, root) = scratch();
    let answered = init_state(root.clone(), "a1b2".to_string()).unwrap();
    assert_eq!(answered.install_id.as_deref(), Some("a1b2"));
    assert_eq!(get_state(root.clone()), answered);
    assert!(state_file(&root).is_file(), "C7's path");
}

/// Dart mints an id on every launch and this is what keeps the first, so the file is compared whole:
/// a relaunch has no reason to touch it.
#[test]
fn init_state_keeps_an_id_that_exists_and_writes_nothing() {
    let (_dir, root) = scratch();
    init_state(root.clone(), "a1b2".to_string()).unwrap();
    set_state(root.clone(), every_field_set(None)).unwrap();
    let written = fs::read_to_string(state_file(&root)).unwrap();

    let answered = init_state(root.clone(), "ffff".to_string()).unwrap();

    assert_eq!(answered, every_field_set(Some("a1b2")));
    assert_eq!(fs::read_to_string(state_file(&root)).unwrap(), written);
}

/// A file that exists and holds no install id yet keeps everything else it holds.
#[test]
fn init_state_on_settings_with_no_id_adds_the_id_and_keeps_the_settings() {
    let (_dir, root) = scratch();
    set_state(root.clone(), every_field_set(None)).unwrap();
    let answered = init_state(root.clone(), "a1b2".to_string()).unwrap();
    assert_eq!(answered, every_field_set(Some("a1b2")));
}

/// C8: the id goes into every session's name, and each of these would be a session name the store
/// refuses or a collision the four digits exist to avoid.
#[test]
fn init_state_refuses_an_id_that_is_not_four_lowercase_hex_digits() {
    let (_dir, root) = scratch();
    for id in ["", "a1b", "a1b2c", "A1B2", "g1b2", "a/b2", "é1b"] {
        let refused = init_state(root.clone(), id.to_string()).expect_err(id);
        assert!(refused.contains("not an install id"), "{refused}");
    }
    assert!(!state_file(&root).exists());
}

#[test]
fn state_survives_a_reload_field_for_field_and_leaves_no_temporary_file() {
    let (_dir, root) = scratch();
    init_state(root.clone(), "a1b2".to_string()).unwrap();
    let wanted = every_field_set(Some("a1b2"));

    assert_eq!(set_state(root.clone(), wanted.clone()).unwrap(), wanted);

    assert_eq!(get_state(root.clone()), wanted);
    assert!(!state_file(&root).with_extension("tmp").exists());
    let names: Vec<_> = fs::read_dir(Dirs::of(&root).app)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["state.json"]);
}

/// `init_state` is the one writer of the id, so a stale or default copy in Dart's model cannot rename
/// this phone's later sessions, whichever way it differs.
#[test]
fn set_state_cannot_move_or_clear_or_invent_the_install_id() {
    let (_dir, root) = scratch();
    let answered = set_state(root.clone(), every_field_set(Some("ffff"))).unwrap();
    assert_eq!(answered.install_id, None, "invented, before init_state");
    assert_eq!(get_state(root.clone()).install_id, None);

    init_state(root.clone(), "a1b2".to_string()).unwrap();
    for given in [Some("ffff"), None] {
        let answered = set_state(root.clone(), every_field_set(given)).unwrap();
        assert_eq!(answered.install_id.as_deref(), Some("a1b2"));
        assert_eq!(get_state(root.clone()).install_id.as_deref(), Some("a1b2"));
    }
}

#[test]
fn set_state_refuses_a_club_or_a_time_it_could_not_use_and_keeps_what_was_stored() {
    let (_dir, root) = scratch();
    let stored = set_state(root.clone(), every_field_set(None)).unwrap();

    let refused = set_state(
        root.clone(),
        AppState {
            club: Some("7 iron".to_string()),
            ..AppState::default()
        },
    )
    .unwrap_err();
    assert!(refused.contains("'7 iron' is not a club"), "{refused}");

    let refused = set_state(
        root.clone(),
        AppState {
            last_export_at: Some("2026-11-04T20:42:00".to_string()),
            ..AppState::default()
        },
    )
    .unwrap_err();
    assert!(refused.contains("offset"), "{refused}");

    assert_eq!(get_state(root), stored);
}

/// C15 is asked of a shot and not here: a change sheet passes through this state on its way to a
/// shape, and refusing it would stop a person getting there.
#[test]
fn set_state_stores_shot_shaping_with_no_shape_yet() {
    let (_dir, root) = scratch();
    let halfway = AppState {
        mode: PracticeMode::ShotShaping,
        target_shape: None,
        ..AppState::default()
    };
    assert_eq!(set_state(root, halfway.clone()).unwrap(), halfway);
}

#[test]
fn an_unreadable_state_file_reads_as_the_defaults_and_the_next_write_replaces_it() {
    let (_dir, root) = scratch();
    fs::create_dir_all(Dirs::of(&root).app).unwrap();
    fs::write(state_file(&root), "{not json").unwrap();
    assert_eq!(get_state(root.clone()), AppState::default());

    let answered = init_state(root.clone(), "a1b2".to_string()).unwrap();
    assert_eq!(answered.install_id.as_deref(), Some("a1b2"));
    assert_eq!(get_state(root).install_id.as_deref(), Some("a1b2"));
}

/// A later build's settings file, read by this one or the reverse: a key that is absent takes its
/// default, and one this build does not know costs nothing.
#[test]
fn a_state_file_missing_keys_or_holding_unknown_ones_keeps_what_it_can_read() {
    let (_dir, root) = scratch();
    fs::create_dir_all(Dirs::of(&root).app).unwrap();
    fs::write(
        state_file(&root),
        r#"{"install_id": "a1b2", "appearance": "light", "from_a_later_build": 1}"#,
    )
    .unwrap();
    assert_eq!(
        get_state(root),
        AppState {
            install_id: Some("a1b2".to_string()),
            appearance: Appearance::Light,
            ..AppState::default()
        }
    );
}

/// The file names the enums as `contracts` does, which is also what a reader of the file sees.
#[test]
fn the_state_file_spells_its_enums_as_the_manifest_does() {
    let (_dir, root) = scratch();
    set_state(root.clone(), every_field_set(None)).unwrap();
    let file: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(state_file(&root)).unwrap()).unwrap();
    assert_eq!(file["mode"], "shot_shaping");
    assert_eq!(file["target_shape"], "draw");
    assert_eq!(
        file["challenge_shapes"],
        serde_json::json!(["draw", "fade"])
    );
    assert_eq!(file["appearance"], "light");
}

/// Nothing serializes bridge calls, so a sheet saving twice can overlap. Without the lock in
/// `crate::state`, writers sharing a `.tmp` fail on the rename or tear the file, and a torn file reads
/// as the defaults and loses the id.
#[test]
fn overlapping_saves_each_succeed_and_the_install_id_survives_them() {
    let (_dir, root) = scratch();
    init_state(root.clone(), "a1b2".to_string()).unwrap();
    std::thread::scope(|scope| {
        for shots in 0..8 {
            let root = root.clone();
            scope.spawn(move || {
                for _ in 0..25 {
                    let state = AppState {
                        last_export_shots: Some(shots),
                        ..AppState::default()
                    };
                    set_state(root.clone(), state).expect("a save");
                }
            });
        }
    });
    assert_eq!(get_state(root).install_id.as_deref(), Some("a1b2"));
}

// --- The enums' spelling ---

/// Each variant `contracts` declares, through the conversion and back, written the same way.
fn holds_to_contracts<Ours, Theirs>(variants: &[Theirs])
where
    Ours: serde::Serialize + From<Theirs> + Copy + std::fmt::Debug,
    Theirs: serde::Serialize + From<Ours> + Copy + PartialEq + std::fmt::Debug,
{
    for theirs in variants {
        let ours = Ours::from(*theirs);
        assert_eq!(
            serde_json::to_value(ours).unwrap(),
            serde_json::to_value(theirs).unwrap(),
            "{ours:?}"
        );
        assert_eq!(Theirs::from(ours), *theirs);
    }
}

/// The `match`es in `crate::state` make a variant added on either side a build failure, so this only
/// has to hold the spelling of the ones that exist. `Appearance` has no twin, so its spelling is here.
#[test]
fn each_plain_enum_is_spelled_as_contracts_spells_it() {
    use contracts::golfer::Handedness as H;
    use contracts::intent::{PracticeMode as M, TargetShape as S};

    holds_to_contracts::<Handedness, H>(&[H::Right, H::Left]);
    holds_to_contracts::<PracticeMode, M>(&[
        M::Fundamentals,
        M::ShotShaping,
        M::Performance,
        M::Drill,
    ]);
    holds_to_contracts::<TargetShape, S>(&[S::Straight, S::Draw, S::Fade]);

    for (appearance, spelled) in [
        (Appearance::Dark, "dark"),
        (Appearance::System, "system"),
        (Appearance::Light, "light"),
    ] {
        assert_eq!(serde_json::to_value(appearance).unwrap(), spelled);
    }
}
