import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';

/// The facade with no Rust behind it, answering what each function's documentation says it answers
/// (C22). [M38 P19]
///
/// It is not a second implementation of the stores, and holds none of their rules: a refusal is one a
/// test names, in [refusedNames] or [refusedLofts], so a test that asks for Rust's sentence on screen
/// is asking for the sentence it gave the fake, word for word. [calls] logs every call in order.
class FakeCore implements GolfCore {
  FakeCore({
    AppState? state,
    List<GolferView>? golfers,
    Map<String, List<BagSlot>>? bags,
    this.clubs = const ['driver', '7i', 'pw'],
  }) : stored = state ?? defaultState(),
       golfers = golfers ?? [],
       bags = bags ?? {};

  /// `AppState`'s Rust default, before `init_state`.
  static AppState defaultState() => const AppState(
    mode: PracticeMode.fundamentals,
    challenge: false,
    challengeShapes: [],
    appearance: Appearance.dark,
  );

  AppState stored;
  final List<GolferView> golfers;
  final Map<String, List<BagSlot>> bags;
  final List<String> clubs;

  /// A name to the refusal `create_golfer` answers for it.
  final Map<String, String> refusedNames = {};

  /// The refusal `set_bag_slot` answers for a loft that is not finite, as Rust's guard does. Any
  /// other loft stands, as Rust's does.
  String refusedLofts = 'refused by the fake: not a finite loft';

  final List<String> calls = [];

  /// Every `now` the app passed, to check its spelling.
  final List<String> nows = [];

  /// What each `record_photo` answers, in order: a [RecordOutcome], or a `String` it refuses with. A
  /// call with nothing queued fails the test, since the test did not say what Rust answered. [M38 P21]
  final List<Object> recordAnswers = [];

  /// Every photo `record_photo` was given, in order.
  final List<PhotoIn> recorded = [];

  /// Every line `log_read` was given, in order.
  final List<ReadLog> logged = [];

  /// What `log_read` refuses with, if a test names one.
  String? refusedLog;

  /// Every line `log_event` was given, in order. **Not in [calls]**, whose order the tests before P30b
  /// pin, since an event is written beside an action and never instead of one. [M38 P30b]
  final List<AppEvent> events = [];

  /// What `log_event` refuses with, if a test names one: a log that cannot be written.
  String? refusedEvent;

  /// The details of the events of [kind], in the order written.
  List<String> eventsOf(EventKind kind) => [
    for (final event in events)
      if (event.kind == kind) event.detail,
  ];

  /// Each golfer's sessions by player id, as `list_sessions` answers them: a list of [SessionView], or a
  /// `String` it refuses with. A golfer with nothing here has no sessions, as Rust answers one with
  /// none. The fake sorts nothing, since Rust's answer arrives in order (P9 finding 10). [M38 P24]
  final Map<String, Object> sessions = {};

  /// Each shot's detail by `session/swing`: a [ShotDetail], or a `String` it refuses with. A shot
  /// with nothing here fails the test, since the test did not say what is stored. [M38 P22]
  final Map<String, Object> details = {};

  /// What each `correct_shot` answers, in order: a [ShotCard], or a `String` it refuses with. A call
  /// with nothing queued fails the test.
  final List<Object> correctAnswers = [];

  /// Every review saved, in order, with the edits it sent.
  final List<({String session, String swing, List<FieldEdit> edits})>
  corrections = [];

  /// What `set_shot_intent` refuses with, if a test names one.
  String? refusedIntent;

  /// What `set_shot_club` refuses with, if a test names one. [M38 P25]
  String? refusedClub;

  /// What `set_shot_mishit` refuses with, if a test names one. [M38 P25]
  String? refusedMishit;

  /// What `delete_shot` refuses with, if a test names one.
  String? refusedDelete;

  /// Each golfer's graded profile by player id: a [ProfileView], or a `String` it refuses with. A
  /// golfer with nothing here fails the test, since even an empty profile carries Rust's sentences,
  /// which the test has to say. [M38 P26]
  final Map<String, Object> profiles = {};

  /// What each `export_sessions` answers, in order: an [ExportSummary], or a `String` it refuses with. A
  /// call with nothing queued fails the test. [M38 P27]
  final List<Object> exportAnswers = [];

  /// What `export_status` answers. The fake does not move it after an export, since the counts are
  /// Rust's: a test that shows a later status sets it here.
  ExportStatus status = const ExportStatus(shots: 0, shotsSince: 0);

  /// What `about` answers.
  AboutView aboutView = const AboutView(
    analysisVersion: 17,
    screenParserVersion: 5,
    careerVersion: 3,
    device: 'full_swing',
    crateVersion: '0.1.0',
  );

  @override
  Future<AppState> initState({
    required String root,
    required String installId,
  }) async {
    calls.add('initState $installId');
    if (stored.installId == null) {
      stored = AppState(
        installId: installId,
        playerId: stored.playerId,
        club: stored.club,
        mode: stored.mode,
        targetShape: stored.targetShape,
        challenge: stored.challenge,
        challengeShapes: stored.challengeShapes,
        appearance: stored.appearance,
        lastExportAt: stored.lastExportAt,
        lastExportShots: stored.lastExportShots,
      );
    }
    return stored;
  }

  @override
  Future<AppState> getState({required String root}) async {
    calls.add('getState');
    return stored;
  }

  @override
  Future<AppState> setState({
    required String root,
    required AppState state,
  }) async {
    calls.add('setState');
    // The stored id stands whatever `state` says, as Rust's does.
    final id = stored.installId;
    stored = AppState(
      installId: id,
      playerId: state.playerId,
      club: state.club,
      mode: state.mode,
      targetShape: state.targetShape,
      challenge: state.challenge,
      challengeShapes: state.challengeShapes,
      appearance: state.appearance,
      lastExportAt: state.lastExportAt,
      lastExportShots: state.lastExportShots,
    );
    return stored;
  }

  @override
  Future<List<GolferView>> listGolfers({required String root}) async {
    calls.add('listGolfers');
    return List.of(golfers);
  }

  @override
  Future<GolferView> createGolfer({
    required String root,
    required String name,
    required Handedness handedness,
    required String now,
  }) async {
    calls.add('createGolfer $name ${handedness.name}');
    nows.add(now);
    final refused = refusedNames[name];
    if (refused != null) {
      throw Refusal(refused);
    }
    final id = name.toLowerCase();
    final known = golfers.where((golfer) => golfer.playerId == id).firstOrNull;
    if (known != null) {
      return known;
    }
    final golfer = GolferView(
      playerId: id,
      displayName: name,
      handedness: handedness,
    );
    golfers.add(golfer);
    return golfer;
  }

  @override
  Future<List<String>> allClubs() async => clubs;

  @override
  Future<List<BagSlot>> bagSlots({
    required String root,
    required String playerId,
  }) async {
    calls.add('bagSlots $playerId');
    return List.of(bags[playerId] ?? const []);
  }

  @override
  Future<List<BagSlot>> setBagSlot({
    required String root,
    required String playerId,
    required String club,
    double? loftDeg,
    required String now,
  }) async {
    calls.add('setBagSlot $club $loftDeg');
    nows.add(now);
    if (loftDeg != null && !loftDeg.isFinite) {
      throw Refusal(refusedLofts);
    }
    final bag = {
      for (final slot in bags[playerId] ?? const <BagSlot>[]) slot.club: slot,
    };
    bag[club] = BagSlot(
      club: club,
      loftDeg: loftDeg,
      loftDisplay: rustDisplay(loftDeg),
    );
    bags[playerId] = [
      for (final known in clubs)
        if (bag[known] != null) bag[known]!,
    ];
    return List.of(bags[playerId]!);
  }

  @override
  Future<List<BagSlot>> removeBagSlot({
    required String root,
    required String playerId,
    required String club,
    required String now,
  }) async {
    calls.add('removeBagSlot $club');
    nows.add(now);
    bags[playerId] = [
      for (final slot in bags[playerId] ?? const <BagSlot>[])
        if (slot.club != club) slot,
    ];
    return List.of(bags[playerId]!);
  }

  @override
  Future<RecordOutcome> recordPhoto({required PhotoIn photo}) async {
    calls.add('recordPhoto ${photo.path}');
    nows.add(photo.now);
    recorded.add(photo);
    if (recordAnswers.isEmpty) {
      throw StateError('recordPhoto with no answer queued');
    }
    final answer = recordAnswers.removeAt(0);
    if (answer is String) {
      throw Refusal(answer);
    }
    return answer as RecordOutcome;
  }

  @override
  Future<void> logRead({required String root, required ReadLog read}) async {
    calls.add('logRead ${read.status.name}');
    nows.add(read.at);
    final refused = refusedLog;
    if (refused != null) {
      throw Refusal(refused);
    }
    logged.add(read);
  }

  @override
  Future<void> logEvent({required String root, required AppEvent event}) async {
    final refused = refusedEvent;
    if (refused != null) {
      throw Refusal(refused);
    }
    events.add(event);
  }

  @override
  Future<ShotCard> correctShot({
    required String root,
    required String sessionId,
    required String swingId,
    required List<FieldEdit> edits,
    required String now,
  }) async {
    calls.add('correctShot $sessionId/$swingId');
    nows.add(now);
    corrections.add((session: sessionId, swing: swingId, edits: edits));
    if (correctAnswers.isEmpty) {
      throw StateError('correctShot with no answer queued');
    }
    final answer = correctAnswers.removeAt(0);
    if (answer is String) {
      throw Refusal(answer);
    }
    return answer as ShotCard;
  }

  @override
  Future<List<SessionView>> listSessions({
    required String root,
    required String playerId,
  }) async {
    calls.add('listSessions $playerId');
    final answer = sessions[playerId];
    if (answer is String) {
      throw Refusal(answer);
    }
    return List.of(answer as List<SessionView>? ?? const []);
  }

  @override
  Future<ShotDetail> shotDetail({
    required String root,
    required String sessionId,
    required String swingId,
  }) async {
    calls.add('shotDetail $sessionId/$swingId');
    return _detail(sessionId, swingId);
  }

  ShotDetail _detail(String sessionId, String swingId) {
    final answer = details['$sessionId/$swingId'];
    if (answer == null) {
      throw StateError('no detail for $sessionId/$swingId');
    }
    if (answer is String) {
      throw Refusal(answer);
    }
    return answer as ShotDetail;
  }

  /// Answers the held detail with the club it was given, and holds that: the one field the call names,
  /// and no flight or loft, which is Rust's to work out. [M38 P25]
  @override
  Future<ShotDetail> setShotClub({
    required String root,
    required String sessionId,
    required String swingId,
    required String club,
    required String now,
  }) async {
    calls.add('setShotClub $sessionId/$swingId $club');
    nows.add(now);
    final refused = refusedClub;
    if (refused != null) {
      throw Refusal(refused);
    }
    return _hold(sessionId, swingId, (row) => rowWith(row, club: () => club));
  }

  /// Answers the held detail with the verdict it was given, and holds that. **Only the verdict**: what
  /// is in force (`mishit`) is the corpus's, through a rule the fake does not hold, so a test that shows
  /// it sets it on the detail it holds. [M38 P25]
  @override
  Future<ShotDetail> setShotMishit({
    required String root,
    required String sessionId,
    required String swingId,
    MishitVerdict? verdict,
    required String now,
  }) async {
    calls.add('setShotMishit $sessionId/$swingId ${verdict?.name}');
    nows.add(now);
    final refused = refusedMishit;
    if (refused != null) {
      throw Refusal(refused);
    }
    return _hold(
      sessionId,
      swingId,
      (row) => rowWith(row, mishitVerdict: () => verdict),
    );
  }

  /// The held detail with its row changed by [change], held in its place and answered.
  ShotDetail _hold(
    String sessionId,
    String swingId,
    ShotRow Function(ShotRow row) change,
  ) {
    final held = _detail(sessionId, swingId);
    final detail = ShotDetail(
      row: change(held.row),
      card: held.card,
      noCard: held.noCard,
      measurements: held.measurements,
      unscored: held.unscored,
      notes: held.notes,
    );
    details['$sessionId/$swingId'] = detail;
    return detail;
  }

  /// Answers the held detail with the intent it was given, and holds that: the one field the call
  /// names, and no rule about which pairs go together, which a test names in [refusedIntent].
  @override
  Future<ShotDetail> setShotIntent({
    required String root,
    required String sessionId,
    required String swingId,
    required PracticeMode mode,
    TargetShape? targetShape,
    required String now,
  }) async {
    calls.add(
      'setShotIntent $sessionId/$swingId ${mode.name} ${targetShape?.name}',
    );
    nows.add(now);
    final refused = refusedIntent;
    if (refused != null) {
      throw Refusal(refused);
    }
    return _hold(
      sessionId,
      swingId,
      (row) => withIntent(row, mode, targetShape),
    );
  }

  @override
  Future<void> deleteShot({
    required String root,
    required String sessionId,
    required String swingId,
  }) async {
    calls.add('deleteShot $sessionId/$swingId');
    final refused = refusedDelete;
    if (refused != null) {
      throw Refusal(refused);
    }
    details.remove('$sessionId/$swingId');
  }

  @override
  Future<ProfileView> gradedProfile({
    required String root,
    required String playerId,
  }) async {
    calls.add('gradedProfile $playerId');
    final answer = profiles[playerId];
    if (answer == null) {
      throw StateError('no profile for $playerId');
    }
    if (answer is String) {
      throw Refusal(answer);
    }
    return answer as ProfileView;
  }

  @override
  Future<ExportSummary> exportSessions({
    required String root,
    required String outPath,
    required String now,
  }) async {
    calls.add('exportSessions $outPath');
    nows.add(now);
    if (exportAnswers.isEmpty) {
      throw StateError('exportSessions with no answer queued');
    }
    final answer = exportAnswers.removeAt(0);
    if (answer is String) {
      throw Refusal(answer);
    }
    return answer as ExportSummary;
  }

  @override
  Future<ExportStatus> exportStatus({required String root}) async {
    calls.add('exportStatus');
    return status;
  }

  @override
  Future<AboutView> about() async {
    calls.add('about');
    return aboutView;
  }
}

/// [row] with another intent, everything else as it was. [M38 P22]
ShotRow withIntent(ShotRow row, PracticeMode mode, TargetShape? shape) =>
    rowWith(row, mode: mode, targetShape: () => shape);

/// [row] with the fields named changed, everything else as it was, since the generated class has no
/// `copyWith`. A nullable field takes a getter, as [AppStateEdits.copyWith]'s do. [M38 P25]
ShotRow rowWith(
  ShotRow row, {
  ValueGetter<String?>? club,
  PracticeMode? mode,
  ValueGetter<TargetShape?>? targetShape,
  bool? flagged,
  bool? mishit,
  ValueGetter<MishitVerdict?>? mishitVerdict,
  bool? untracked,
  ValueGetter<String?>? photoPath,
  List<String>? leftOut,
}) => ShotRow(
  sessionId: row.sessionId,
  swingId: row.swingId,
  time: row.time,
  timeDisplay: row.timeDisplay,
  club: club == null ? row.club : club(),
  mode: mode ?? row.mode,
  targetShape: targetShape == null ? row.targetShape : targetShape(),
  carryLabel: row.carryLabel,
  carryDisplay: row.carryDisplay,
  ballSpeedLabel: row.ballSpeedLabel,
  ballSpeedDisplay: row.ballSpeedDisplay,
  flagged: flagged ?? row.flagged,
  corrected: row.corrected,
  mishit: mishit ?? row.mishit,
  mishitVerdict: mishitVerdict == null ? row.mishitVerdict : mishitVerdict(),
  untracked: untracked ?? row.untracked,
  photoPath: photoPath == null ? row.photoPath : photoPath(),
  leftOut: leftOut ?? row.leftOut,
);

/// A loft as Rust's `format!("{loft}°")` spells the ones a test uses: `34°`, where Dart would write
/// `34.0`. So a screen showing this string is showing Rust's text, and one formatting the number
/// itself would show something else.
String? rustDisplay(double? loft) {
  if (loft == null) {
    return null;
  }
  final whole = loft == loft.truncateToDouble();
  return '${whole ? loft.toInt() : loft}°';
}

/// The phone with no Swift behind it.
///
/// The camera, the library and the reader answer what a test queues, in order: each [takePhoto] the
/// next of [taken] (`null` is a cancel), each [pickPhotos] the next of [picks], and each [readScreen]
/// the next of [reads]. A call with nothing queued
/// fails the test, since the test did not say what the golfer did. [calls] logs every call in order.
class FakePhone implements Phone {
  FakePhone({
    this.camera = true,
    List<Photo?>? taken,
    List<PhotoPick>? picks,
    List<Object>? reads,
  }) : taken = taken ?? [],
       picks = picks ?? [],
       reads = reads ?? [];

  static const documents = '/fake/Documents';

  bool camera;
  final List<Photo?> taken;
  final List<PhotoPick> picks;

  /// What each [readScreen] answers, in order: a [ScreenRead], or a [PlatformException] it throws, as
  /// Swift's `unreadable` reaches Dart. [M38 P21]
  final List<Object> reads;

  /// What [share] answers: whether the golfer finished an action with the file.
  bool shareCompletes = true;

  /// What [share] throws instead, as Swift's `busy` or `share_failed` reach Dart, if a test names one.
  /// [M38 P27]
  PlatformException? shareFails;

  final List<String> calls = [];

  @override
  Future<String> documentsDir() async => documents;

  @override
  Future<ScreenRead> readScreen(String path) async {
    calls.add('readScreen $path');
    if (reads.isEmpty) {
      throw StateError('readScreen with no read queued');
    }
    final answer = reads.removeAt(0);
    if (answer is PlatformException) {
      throw answer;
    }
    return answer as ScreenRead;
  }

  @override
  Future<bool> cameraAvailable() async {
    calls.add('cameraAvailable');
    return camera;
  }

  @override
  Future<Photo?> takePhoto() async {
    calls.add('takePhoto');
    if (taken.isEmpty) {
      throw StateError('takePhoto with no photo queued');
    }
    return taken.removeAt(0);
  }

  @override
  Future<PhotoPick> pickPhotos() async {
    calls.add('pickPhotos');
    if (picks.isEmpty) {
      throw StateError('pickPhotos with no pick queued');
    }
    return picks.removeAt(0);
  }

  @override
  Future<bool> share(String path) async {
    calls.add('share $path');
    final fails = shareFails;
    if (fails != null) {
      throw fails;
    }
    return shareCompletes;
  }
}

/// A golfer and a bag, as a launch after setup finds them.
FakeCore setUpCore({Appearance appearance = Appearance.dark}) {
  const golfer = GolferView(
    playerId: 'aaron',
    displayName: 'Aaron',
    handedness: Handedness.right,
  );
  return FakeCore(
    state: AppState(
      installId: 'a1b2',
      playerId: golfer.playerId,
      mode: PracticeMode.fundamentals,
      challenge: false,
      challengeShapes: const [],
      appearance: appearance,
    ),
    golfers: [golfer],
    bags: {
      golfer.playerId: const [
        BagSlot(club: '7i', loftDeg: 34, loftDisplay: '34°'),
      ],
    },
  );
}

/// [card]'s detail as `shot_detail` answers it, stamped with the intent a test names. Only what a
/// review reads is the test's to set: the photo and the intent. [M38 P22]
ShotDetail detailOf(
  ShotCard card, {
  PracticeMode mode = PracticeMode.fundamentals,
  TargetShape? shape,
  String? photoPath,
}) => ShotDetail(
  row: ShotRow(
    sessionId: card.sessionId,
    swingId: card.swingId,
    time: '2026-11-05T10:00:00-05:00',
    timeDisplay: '10:00',
    club: '7i',
    mode: mode,
    targetShape: shape,
    carryLabel: 'Carry Yd',
    ballSpeedLabel: 'Ball Speed Mph',
    flagged: card.flagged,
    corrected: false,
    mishit: false,
    untracked: false,
    photoPath: photoPath,
    leftOut: const [],
  ),
  card: card,
  measurements: const [],
  unscored: const [],
  notes: const [],
);
