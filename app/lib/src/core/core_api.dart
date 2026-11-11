import 'package:flutter/foundation.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64;

import '../rust/third_party/app/api/about.dart' as rust_about;
import '../rust/third_party/app/api/about.dart' show AboutView;
import '../rust/third_party/app/api/export.dart' as rust_export;
import '../rust/third_party/app/api/export.dart'
    show ExportStatus, ExportSummary;
import '../rust/third_party/app/api/history.dart' as rust_history;
import '../rust/third_party/app/api/profile.dart' as rust_profile;
import '../rust/third_party/app/api/profile.dart' show ProfileView;
import '../rust/third_party/app/api/history.dart'
    show MishitVerdict, SessionView, ShotDetail;
import '../rust/third_party/app/api/record.dart' as rust_record;
import '../rust/third_party/app/api/record.dart'
    show AppEvent, PhotoIn, ReadLog, RecordOutcome, ShotCard;
import '../rust/third_party/app/api/review.dart' as rust_review;
import '../rust/third_party/app/api/review.dart' show FieldEdit;
import '../rust/third_party/app/api/setup.dart' as rust;
import '../rust/third_party/app/api/setup.dart'
    show
        AppState,
        Appearance,
        BagSlot,
        GolferView,
        Handedness,
        PracticeMode,
        TargetShape;

export '../rust/third_party/app/api/setup.dart'
    show
        AppState,
        Appearance,
        BagSlot,
        GolferView,
        Handedness,
        PracticeMode,
        TargetShape;
export '../rust/third_party/app/api/record.dart'
    show
        AppEvent,
        CardRow,
        EventKind,
        PhotoIn,
        ReadLog,
        ReadSource,
        RecordOutcome,
        RecordStatus,
        RowState,
        ShotCard;
export '../rust/third_party/app/api/review.dart' show FieldEdit;
export '../rust/third_party/app/api/history.dart'
    show
        MeasurementRow,
        MishitVerdict,
        SessionView,
        ShotDetail,
        ShotRow,
        UnscoredRow;
export '../rust/third_party/app/api/about.dart' show AboutView;
export '../rust/third_party/app/api/export.dart'
    show ExportStatus, ExportSummary;
export '../rust/third_party/app/api/profile.dart'
    show
        BlendView,
        ClubView,
        FindingView,
        NumberRow,
        PlayerView,
        ProfileView,
        ShapeView,
        ShareView,
        TopicView;

/// The facade as the app's Dart sees it, behind one interface so that a widget test runs with no Rust
/// (the plan's C22). [M38 P19]
///
/// **Each method is one facade function, with its name, its arguments and its answer**, and nothing
/// added. So [RustCore] is a line per call and cannot hold a rule of its own, and a fake that answers
/// what the facade documents is a fair stand-in. Later phases add the rest of C2's rows here as their
/// screens need them: P21 the two a read needs, P25 the last two of a shot's actions, P26 the graded
/// profile, and P27 the export's two and `about`.
///
/// A facade call that can refuse throws a [Refusal] carrying Rust's sentence, and the screens show
/// that sentence as it stands (rule 8). The ones that never refuse (`get_state`, `list_golfers`,
/// `all_clubs`, `bag_slots`) throw nothing.
abstract class GolfCore {
  Future<AppState> initState({required String root, required String installId});

  Future<AppState> getState({required String root});

  Future<AppState> setState({required String root, required AppState state});

  Future<List<GolferView>> listGolfers({required String root});

  Future<GolferView> createGolfer({
    required String root,
    required String name,
    required Handedness handedness,
    required String now,
  });

  Future<List<String>> allClubs();

  Future<List<BagSlot>> bagSlots({
    required String root,
    required String playerId,
  });

  Future<List<BagSlot>> setBagSlot({
    required String root,
    required String playerId,
    required String club,
    double? loftDeg,
    required String now,
  });

  Future<List<BagSlot>> removeBagSlot({
    required String root,
    required String playerId,
    required String club,
    required String now,
  });

  /// One photo and its boxes to a stored, analysed shot and its card. [M38 P21]
  ///
  /// `Failed` and `Duplicate` are answers and not refusals. A [Refusal] is a third thing, with no card
  /// and no status, and it says what to change: no golfer, no club, or a mode and shape that do not go
  /// together (the plan's P6 finding 13).
  Future<RecordOutcome> recordPhoto({required PhotoIn photo});

  /// One line of `.app/reads.jsonl`, the bay report's diagnostics (P14). [M38 P21]
  Future<void> logRead({required String root, required ReadLog read});

  /// One line of `.app/events.jsonl`: what the person did, or what went wrong (D16). A [Refusal] is a
  /// time with no offset or a file that would not open, and `AppModel.logEvent` swallows either, since
  /// a log that could not be written must not stop what it was logging. [M38 P30b]
  Future<void> logEvent({required String root, required AppEvent event});

  /// Save a review (C11): apply [edits], clear the flag, re-analyse, and answer the card as it now
  /// stands. Empty [edits] is "reviewed, nothing to change", which is how a flagged shot is released
  /// (D9). A [Refusal] is one sentence naming the tile, with nothing written. [M38 P22]
  Future<ShotCard> correctShot({
    required String root,
    required String sessionId,
    required String swingId,
    required List<FieldEdit> edits,
    required String now,
  });

  /// One golfer's sessions, newest first, each with its rows newest first, every entry listed whether
  /// or not it counts. A golfer with none answers an empty list. A [Refusal] is only a build whose
  /// device table has lost a tile. [M38 P24]
  Future<List<SessionView>> listSessions({
    required String root,
    required String playerId,
  });

  /// One shot's detail: here for the review's photo and the intent it was stamped with, and P25's
  /// whole screen. A [Refusal] is an entry that is not on this phone. [M38 P22]
  Future<ShotDetail> shotDetail({
    required String root,
    required String sessionId,
    required String swingId,
  });

  /// Which club hit one shot, flown again with that club's loft: the repair for a club left unchanged
  /// between swings. A [Refusal] is a sentence, with nothing written. [M38 P25]
  Future<ShotDetail> setShotClub({
    required String root,
    required String sessionId,
    required String swingId,
    required String club,
    required String now,
  });

  /// A person's verdict on one shot (D10), or with none the automatic rule's again. [M38 P25]
  Future<ShotDetail> setShotMishit({
    required String root,
    required String sessionId,
    required String swingId,
    MishitVerdict? verdict,
    required String now,
  });

  /// What one shot was practised as, re-analysed under it (C15). [M38 P22]
  Future<ShotDetail> setShotIntent({
    required String root,
    required String sessionId,
    required String swingId,
    required PracticeMode mode,
    TargetShape? targetShape,
    required String now,
  });

  /// Remove a stray photo's entry and its shot (D10). Nothing undoes it. [M38 P22]
  Future<void> deleteShot({
    required String root,
    required String sessionId,
    required String swingId,
  });

  /// One golfer's graded profile: each club with a tracked shot in bag order, then the player, every
  /// sentence `golf-core profile`'s and each club's mean `golf-core club-profile`'s. A golfer with no
  /// shots, or an id nobody holds, answers an empty profile and not a refusal. A [Refusal] is only a
  /// profile that fails its own check, which is a builder's bug. [M38 P26]
  Future<ProfileView> gradedProfile({
    required String root,
    required String playerId,
  });

  /// Everything C16 names, in one cumulative zip at [outPath], which it replaces. **It changes nothing
  /// on the phone**, `last_export_at` included: the summary's `exported_at` and `shots` are what to
  /// store once the share has gone (P13 finding 13). A [Refusal] is a sentence, the likely one being
  /// that there is nothing to export yet, with nothing left at [outPath]. [M38 P27]
  Future<ExportSummary> exportSessions({
    required String root,
    required String outPath,
    required String now,
  });

  /// The last export and the shots since, each count Rust's. Never refuses. [M38 P27]
  Future<ExportStatus> exportStatus({required String root});

  /// Which build of the core this is: the three version keys, the device and the crate's version.
  /// Never refuses. [M38 P27]
  Future<AboutView> about();
}

/// A facade refusal: Rust's `Err(String)`, in Rust's words.
///
/// The bridge throws that `String` itself (the plan's spike section), and a bare `String` is a thing no
/// `catch` can tell from a bug's. So [RustCore] turns it into this, once, and a screen catches only
/// this. A panic or a bridge failure is not one, and is left to surface as what it is.
class Refusal implements Exception {
  const Refusal(this.message);

  final String message;

  @override
  String toString() => message;
}

/// [GolfCore] over the generated bridge.
class RustCore implements GolfCore {
  const RustCore();

  @override
  Future<AppState> initState({
    required String root,
    required String installId,
  }) => _refusing(() => rust.initState(root: root, installId: installId));

  @override
  Future<AppState> getState({required String root}) =>
      rust.getState(root: root);

  @override
  Future<AppState> setState({required String root, required AppState state}) =>
      _refusing(() => rust.setState(root: root, state: state));

  @override
  Future<List<GolferView>> listGolfers({required String root}) =>
      rust.listGolfers(root: root);

  @override
  Future<GolferView> createGolfer({
    required String root,
    required String name,
    required Handedness handedness,
    required String now,
  }) => _refusing(
    () => rust.createGolfer(
      root: root,
      name: name,
      handedness: handedness,
      now: now,
    ),
  );

  @override
  Future<List<String>> allClubs() => rust.allClubs();

  @override
  Future<List<BagSlot>> bagSlots({
    required String root,
    required String playerId,
  }) => rust.bagSlots(root: root, playerId: playerId);

  @override
  Future<List<BagSlot>> setBagSlot({
    required String root,
    required String playerId,
    required String club,
    double? loftDeg,
    required String now,
  }) => _refusing(
    () => rust.setBagSlot(
      root: root,
      playerId: playerId,
      club: club,
      loftDeg: loftDeg,
      now: now,
    ),
  );

  @override
  Future<List<BagSlot>> removeBagSlot({
    required String root,
    required String playerId,
    required String club,
    required String now,
  }) => _refusing(
    () => rust.removeBagSlot(
      root: root,
      playerId: playerId,
      club: club,
      now: now,
    ),
  );

  @override
  Future<RecordOutcome> recordPhoto({required PhotoIn photo}) =>
      _refusing(() => rust_record.recordPhoto(photo: photo));

  @override
  Future<void> logRead({required String root, required ReadLog read}) =>
      _refusing(() => rust_record.logRead(root: root, read: read));

  @override
  Future<void> logEvent({required String root, required AppEvent event}) =>
      _refusing(() => rust_record.logEvent(root: root, event: event));

  @override
  Future<ShotCard> correctShot({
    required String root,
    required String sessionId,
    required String swingId,
    required List<FieldEdit> edits,
    required String now,
  }) => _refusing(
    () => rust_review.correctShot(
      root: root,
      sessionId: sessionId,
      swingId: swingId,
      edits: edits,
      now: now,
    ),
  );

  @override
  Future<List<SessionView>> listSessions({
    required String root,
    required String playerId,
  }) => _refusing(
    () => rust_history.listSessions(root: root, playerId: playerId),
  );

  @override
  Future<ShotDetail> shotDetail({
    required String root,
    required String sessionId,
    required String swingId,
  }) => _refusing(
    () => rust_history.shotDetail(
      root: root,
      sessionId: sessionId,
      swingId: swingId,
    ),
  );

  @override
  Future<ShotDetail> setShotClub({
    required String root,
    required String sessionId,
    required String swingId,
    required String club,
    required String now,
  }) => _refusing(
    () => rust_history.setShotClub(
      root: root,
      sessionId: sessionId,
      swingId: swingId,
      club: club,
      now: now,
    ),
  );

  @override
  Future<ShotDetail> setShotMishit({
    required String root,
    required String sessionId,
    required String swingId,
    MishitVerdict? verdict,
    required String now,
  }) => _refusing(
    () => rust_history.setShotMishit(
      root: root,
      sessionId: sessionId,
      swingId: swingId,
      verdict: verdict,
      now: now,
    ),
  );

  @override
  Future<ShotDetail> setShotIntent({
    required String root,
    required String sessionId,
    required String swingId,
    required PracticeMode mode,
    TargetShape? targetShape,
    required String now,
  }) => _refusing(
    () => rust_history.setShotIntent(
      root: root,
      sessionId: sessionId,
      swingId: swingId,
      mode: mode,
      targetShape: targetShape,
      now: now,
    ),
  );

  @override
  Future<void> deleteShot({
    required String root,
    required String sessionId,
    required String swingId,
  }) => _refusing(
    () => rust_history.deleteShot(
      root: root,
      sessionId: sessionId,
      swingId: swingId,
    ),
  );

  @override
  Future<ProfileView> gradedProfile({
    required String root,
    required String playerId,
  }) => _refusing(
    () => rust_profile.gradedProfile(root: root, playerId: playerId),
  );

  @override
  Future<ExportSummary> exportSessions({
    required String root,
    required String outPath,
    required String now,
  }) => _refusing(
    () => rust_export.exportSessions(root: root, outPath: outPath, now: now),
  );

  @override
  Future<ExportStatus> exportStatus({required String root}) =>
      rust_export.exportStatus(root: root);

  @override
  Future<AboutView> about() => rust_about.about();
}

Future<T> _refusing<T>(Future<T> Function() call) async {
  try {
    return await call();
  } on String catch (message) {
    throw Refusal(message);
  }
}

/// [AppState] with some fields changed, since the generated class has no `copyWith`.
///
/// A nullable field takes a getter, so that "set it to none" (`club: () => null`) and "leave it" (no
/// argument) are different calls. `install_id` is not here: `init_state` is its one writer, and
/// `set_state` keeps the stored one whatever it is given (the plan's P5 finding 4).
extension AppStateEdits on AppState {
  AppState copyWith({
    ValueGetter<String?>? playerId,
    ValueGetter<String?>? club,
    PracticeMode? mode,
    ValueGetter<TargetShape?>? targetShape,
    bool? challenge,
    List<TargetShape>? challengeShapes,
    Appearance? appearance,
    ValueGetter<String?>? lastExportAt,
    ValueGetter<PlatformInt64?>? lastExportShots,
  }) => AppState(
    installId: installId,
    playerId: playerId == null ? this.playerId : playerId(),
    club: club == null ? this.club : club(),
    mode: mode ?? this.mode,
    targetShape: targetShape == null ? this.targetShape : targetShape(),
    challenge: challenge ?? this.challenge,
    challengeShapes: challengeShapes ?? this.challengeShapes,
    appearance: appearance ?? this.appearance,
    lastExportAt: lastExportAt == null ? this.lastExportAt : lastExportAt(),
    lastExportShots: lastExportShots == null
        ? this.lastExportShots
        : lastExportShots(),
  );
}

/// [at] as RFC 3339 with its offset, which every `now` and time the facade takes must carry (C2).
///
/// **Not [DateTime.toIso8601String]**: on a local time that writes no offset at all, and `parse_now`
/// refuses it (the plan's P4 finding 7). A UTC time would be read, and is right for a `now`, but it is
/// wrong for a photo's time, which names the session by its own date (C8), so this writes the offset
/// for both rather than leave one spelling that is right for only one of them. Microseconds, which is
/// what a `DateTime` holds on iOS and the most `Timestamp::parse` reads.
String rfc3339(DateTime at) {
  String two(int n) => n.toString().padLeft(2, '0');
  final offset = at.timeZoneOffset.inMinutes;
  final sign = offset < 0 ? '-' : '+';
  final magnitude = offset.abs();
  final micros = at.millisecond * 1000 + at.microsecond;
  return '${at.year.toString().padLeft(4, '0')}-${two(at.month)}-${two(at.day)}'
      'T${two(at.hour)}:${two(at.minute)}:${two(at.second)}'
      '.${micros.toString().padLeft(6, '0')}'
      '$sign${two(magnitude ~/ 60)}:${two(magnitude % 60)}';
}

/// Now, in this device's offset, for a facade call's `now`.
String now() => rfc3339(DateTime.now());
