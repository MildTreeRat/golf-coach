import 'dart:math';

import 'package:flutter/foundation.dart';

import '../core/core_api.dart';
import '../phone/phone.dart';

/// Where the app is in its first run, which decides the screen it opens on.
enum Stage {
  /// Reading the state, before anything can be shown.
  loading,

  /// The data root could not be opened, and [AppModel.failure] says why.
  failed,

  /// No golfer is chosen: first-run setup asks for one.
  golfer,

  /// A golfer and an empty bag, so the club picker would list nothing (D7): setup asks for clubs.
  bag,

  /// The four tabs.
  ready,
}

/// The app's state, as one [ChangeNotifier] the screens listen to (C22). [M38 P19]
///
/// **It holds what Rust answered, and replaces it with each answer**: every setter in the facade
/// answers the stored result (the plan's P5 finding 8), so the model never edits a copy of its own and
/// cannot drift from the files. It holds no golf (rule 8).
///
/// The stage is read from the files at [start], and moved only by setup's own steps after that. So a
/// relaunch part way through setup lands where it left off, and taking every club out of the bag later
/// does not throw the app back into setup.
class AppModel extends ChangeNotifier {
  AppModel({
    required this.core,
    required this.phone,
    Random? random,
    DateTime Function()? clock,
  }) : _random = random ?? Random.secure(),
       _clock = clock ?? DateTime.now;

  final GolfCore core;
  final Phone phone;
  final Random _random;
  final DateTime Function() _clock;

  Future<void>? _started;
  Stage _stage = Stage.loading;
  String? _failure;
  String? _root;
  AppState? _state;
  List<String> _clubs = const [];
  List<GolferView> _golfers = const [];
  GolferView? _golfer;
  List<BagSlot> _bag = const [];
  bool _keptHandedness = false;

  Stage get stage => _stage;

  /// Why [Stage.failed], as the error said it.
  String? get failure => _failure;

  /// The data root, `<Documents>/processed` (C7). Set once [start] has read it.
  String get root => _root!;

  AppState get state => _state!;

  /// Every club a bag can hold, in bag order and `ClubId`'s spelling.
  List<String> get clubs => _clubs;

  List<GolferView> get golfers => _golfers;

  /// The golfer `state.player_id` names, or none.
  GolferView? get golfer => _golfer;

  /// The current golfer's bag, in bag order.
  List<BagSlot> get bag => _bag;

  /// The model's one source of chance: the install id's mint, and since P23 a challenge's calls, each
  /// `nextInt(n)` over its ticks. Injected so a test can name every draw (C15). [M38 P23]
  Random get random => _random;

  /// Whether setup's name folded to a golfer already known with the other hand, whose stored
  /// handedness stands (P5 finding 8). Setup says so rather than show the tick it was given.
  bool get keptHandedness => _keptHandedness;

  /// Read everything a first screen needs. Called by the app once, and safe to call again: a second
  /// call answers the first's future.
  Future<void> start() => _started ??= _load();

  Future<void> _load() async {
    Object? failed;
    try {
      final root = '${await phone.documentsDir()}/processed';
      // Known before the state is read, so that a state that cannot be read is still logged (D16).
      _root = root;
      var state = await core.getState(root: root);
      if (state.installId == null) {
        // The first run, and the only time an id is minted. `init_state` is its one writer, and it
        // keeps one already stored, so even a mint on a later launch could not rename a session.
        state = await core.initState(
          root: root,
          installId: mintInstallId(_random),
        );
      }
      _state = state;
      _clubs = await core.allClubs();
      await _readGolfers();
      _stage = _golfer == null
          ? Stage.golfer
          : _bag.isEmpty
          ? Stage.bag
          : Stage.ready;
    } catch (error) {
      failed = error;
      _failure = '$error';
      _stage = Stage.failed;
    }
    // A second one in a session's log is a relaunch: the app was closed, or it crashed.
    logEvent(EventKind.appStart, _stage.name);
    if (failed != null) {
      logProblem('start', '$failed');
    }
    notifyListeners();
  }

  Future<void> _readGolfers() async {
    _golfers = await core.listGolfers(root: root);
    final id = state.playerId;
    _golfer = _golfers.where((golfer) => golfer.playerId == id).firstOrNull;
    final golfer = _golfer;
    _bag = golfer == null
        ? const []
        : await core.bagSlots(root: root, playerId: golfer.playerId);
  }

  /// The golfer [name] folds to, created if new, made current, and setup moved on to the bag. A known
  /// golfer keeps their bag and their handedness. Throws [Refusal] with Rust's sentence, having
  /// changed nothing.
  Future<void> createGolfer(String name, Handedness handedness) async {
    final golfer = await core.createGolfer(
      root: root,
      name: name,
      handedness: handedness,
      now: _now(),
    );
    _state = await core.setState(
      root: root,
      state: state.copyWith(playerId: () => golfer.playerId),
    );
    await _readGolfers();
    _keptHandedness = golfer.handedness != handedness;
    _stage = Stage.bag;
    notifyListeners();
  }

  /// Put [club] in the current golfer's bag with [loftDeg] if known, or change its loft. Throws
  /// [Refusal] with Rust's sentence.
  Future<void> setSlot(String club, double? loftDeg) async {
    _bag = await core.setBagSlot(
      root: root,
      playerId: _golfer!.playerId,
      club: club,
      loftDeg: loftDeg,
      now: _now(),
    );
    notifyListeners();
  }

  /// Take [club] out of the current golfer's bag. Throws [Refusal] with Rust's sentence.
  Future<void> removeSlot(String club) async {
    _bag = await core.removeBagSlot(
      root: root,
      playerId: _golfer!.playerId,
      club: club,
      now: _now(),
    );
    notifyListeners();
  }

  /// Setup's last step. Only with a club in the bag, since [Stage.bag] is what an empty one opens on.
  void finishSetup() {
    if (_stage != Stage.bag || _bag.isEmpty) {
      return;
    }
    _stage = Stage.ready;
    notifyListeners();
  }

  /// Change the settings and store them, holding what was stored. Throws [Refusal] with Rust's
  /// sentence, having changed nothing.
  Future<void> editState(AppState Function(AppState state) edit) async {
    final before = state;
    _state = await core.setState(root: root, state: edit(state));
    _logChanges(before);
    notifyListeners();
  }

  Future<void> setAppearance(Appearance appearance) =>
      editState((state) => state.copyWith(appearance: appearance));

  /// Make [playerId] the current golfer, from the Record tab's change sheet, and read their bag. Throws
  /// [Refusal] with Rust's sentence, having changed nothing. [M38 P21]
  ///
  /// **The club is kept only if the new golfer's bag holds it**, and is cleared otherwise, since the
  /// club picker lists the bag's slots alone (D7) and a club from another golfer's bag would be stamped
  /// on the next shot with nothing on screen to pick it again. The mode and shape stand, since they are
  /// the session's and not the golfer's.
  Future<void> chooseGolfer(String playerId) async {
    final bag = await core.bagSlots(root: root, playerId: playerId);
    final keepsClub = bag.any((slot) => slot.club == state.club);
    final before = state;
    _state = await core.setState(
      root: root,
      state: state.copyWith(
        playerId: () => playerId,
        club: keepsClub ? null : () => null,
      ),
    );
    _logChanges(before);
    await _readGolfers();
    // The bag screen's note is about the name just typed, and this golfer was picked, not typed.
    _keptHandedness = false;
    notifyListeners();
  }

  /// Settings' Add a golfer: the golfer [name] folds to, created if new, and made current by
  /// [chooseGolfer]'s rule, so a club the new bag does not hold is cleared. Throws [Refusal] with
  /// Rust's sentence. [M38 P27]
  ///
  /// **Not [createGolfer]**, which is setup's step and moves the stage to [Stage.bag]: the app's root
  /// would then swap the tabs for setup's bag screen, and the Record tab's state, a library batch's
  /// included, would go with them. So the stage stays [Stage.ready], and Settings opens the bag screen
  /// over the tabs. As in setup, a known golfer keeps their bag and their handedness, and
  /// [keptHandedness] says so.
  Future<void> addGolfer(String name, Handedness handedness) async {
    final golfer = await core.createGolfer(
      root: root,
      name: name,
      handedness: handedness,
      now: _now(),
    );
    await chooseGolfer(golfer.playerId);
    _keptHandedness = golfer.handedness != handedness;
    notifyListeners();
  }

  /// How many times the phone's shots have been written since launch: a recorded photo, a review's
  /// end, and P25's actions. Nothing reads its value but to see that it moved. [M38 P24]
  int get shotsVersion => _shotsVersion;
  int _shotsVersion = 0;

  /// Say that a shot was written, so a screen showing the stores reads them again.
  ///
  /// **Here because History cannot see the Record tab's flow**: each tab owns its own state, and a
  /// library batch runs on while the golfer looks at another tab, so History, if it is the one shown,
  /// has to hear of each shot as it lands. Entering History reads the stores anyway, so this matters
  /// only while History is up. It holds no golf: the model says *that* something changed, and History
  /// asks Rust *what*.
  void shotsChanged() {
    _shotsVersion++;
    notifyListeners();
  }

  /// Now by the model's clock, as every facade `now` is spelled, for a flow calling the facade beside
  /// the model (P21's `RecordFlow`), so that a test's clock is the one clock.
  String now() => _now();

  String _now() => rfc3339(_clock());

  /// Every event logged so far, each written after the one before.
  Future<void> _events = Future<void>.value();

  /// Completes once every event logged so far has been written, or has failed to be. Nothing in the app
  /// waits on it: it is for a test.
  Future<void> get eventsWritten => _events;

  /// Append [kind] with [detail] to `.app/events.jsonl`, stamped now by the model's clock (D16), so P32
  /// can say what the person did at the bay and not only what was read. [M38 P30b]
  ///
  /// **It never throws and never waits**, since a log that could not be written must not stop what it
  /// was logging: the write is chained after the last one and not awaited, and a failure goes to the
  /// console alone, because logging it here would be the write that just failed. Chained, so the file
  /// holds the events in the order they happened, and a slow write holds back only the ones after it.
  /// Before [start] knows the data root there is nowhere to write, so nothing is.
  ///
  /// [detail] is short text for a person, naming the subject. It holds no golf (rule 8): a name, a
  /// count or a sentence someone else wrote, and never a number worked out here.
  void logEvent(EventKind kind, String detail) {
    final root = _root;
    if (root == null) {
      return;
    }
    final event = AppEvent(at: _now(), kind: kind, detail: detail);
    _events = _events.then((_) => _write(root, event));
  }

  Future<void> _write(String root, AppEvent event) async {
    try {
      await core.logEvent(root: root, event: event);
    } catch (failed) {
      debugPrint('events.jsonl: ${event.kind.name} not written: $failed');
    }
  }

  /// A refusal or a failure the person was shown, as an `error` naming [where] it was (D16): the
  /// sentence on screen is gone once the next tap clears it, and at the bay nobody else saw it.
  void logProblem(String where, String sentence) =>
      logEvent(EventKind.error, '$where: $sentence');

  /// An error nothing caught, as an `error`: [error]'s message, what Flutter was doing if it says, and
  /// [stack]'s first [stackLines] lines. **It never throws**, since it is called from the app's last
  /// handlers, and an error thrown there has nowhere left to go.
  void logError(Object error, StackTrace? stack, {String? context}) {
    try {
      logEvent(EventKind.error, errorDetail(error, stack, context: context));
    } catch (failed) {
      debugPrint('events.jsonl: an error could not be logged: $failed');
    }
  }

  /// How many lines of a stack an `error`'s detail keeps: P30b's five, enough to find the call, and
  /// short enough that a loop of errors does not fill the phone.
  static const stackLines = 5;

  /// A change to what the next photo is stamped with, as `club_changed` and `mode_changed` (D16). Here
  /// because [editState] and [chooseGolfer] are the state's two writers between swings, so the change
  /// sheet, Settings and anything later are logged without each having to remember to. Settings' own
  /// rows, the appearance and the last export, are not changes to a shot, and log nothing.
  void _logChanges(AppState before) {
    final after = state;
    if (after.club != before.club || after.playerId != before.playerId) {
      logEvent(
        EventKind.clubChanged,
        [
          after.club ?? 'none',
          if (after.playerId != before.playerId) 'golfer ${after.playerId}',
        ].join(', '),
      );
    }
    if (after.mode != before.mode ||
        after.targetShape != before.targetShape ||
        after.challenge != before.challenge ||
        !listEquals(after.challengeShapes, before.challengeShapes)) {
      logEvent(EventKind.modeChanged, intentDetail(after));
    }
  }
}

/// What the next photo will be stamped as, for an event's detail: `fundamentals`, `shotShaping draw`,
/// or with a challenge on, its ticks after (`fundamentals, challenge draw fade`). The enums' own names,
/// since a log is read by a person and by P30c, and neither needs the app's words. [M38 P30b]
String intentDetail(AppState state) => [
  [state.mode.name, ?state.targetShape?.name].join(' '),
  if (state.challenge)
    [
      'challenge',
      for (final shape in state.challengeShapes) shape.name,
    ].join(' '),
].join(', ');

/// `1 photo`, `3 photos`: a count for an event's detail. [M38 P30b]
String counted(int count, String noun) =>
    '$count $noun${count == 1 ? '' : 's'}';

/// An `error`'s detail: the message, then [context] if Flutter gave one, then the first
/// [AppModel.stackLines] lines of [stack] that hold anything. [M38 P30b]
String errorDetail(Object error, StackTrace? stack, {String? context}) => [
  '$error',
  ?context,
  ...?stack
      ?.toString()
      .split('\n')
      .where((line) => line.trim().isNotEmpty)
      .take(AppModel.stackLines),
].join('\n');

/// C8's install id: 4 lowercase hex digits, which `init_state` refuses otherwise. [random] is
/// `Random.secure` in the app, so two phones are unlikely to share one, and a test's own elsewhere.
String mintInstallId(Random random) =>
    random.nextInt(0x10000).toRadixString(16).padLeft(4, '0');
