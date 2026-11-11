import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';

import '../core/core_api.dart';
import '../phone/phone.dart';
import '../screens/review_screen.dart'
    show ReviewDeleted, ReviewEnd, ReviewPlace, ReviewSaved;
import 'app_model.dart';
import 'challenge.dart';

/// Where a library batch has got to: photo [index] of [total], counted from 1.
typedef BatchProgress = ({int index, int total});

/// How the flow puts a review in front of the golfer and waits for it to end, since it has no
/// `BuildContext` of its own: the screen sets one, as `showReview` over its context. [M38 P22]
typedef PresentReview = Future<ReviewEnd?> Function(
  ShotCard card, {
  required bool forced,
  ReviewPlace? place,
});

/// The Record tab's work between swings: a photo in, read by Swift, recorded by Rust, shown, and
/// logged. [M38 P21]
///
/// **One photo at a time, each awaited before the next** (C14). `record_photo` holds a lock of its own
/// since P6, so this is not what keeps a store whole, but it keeps a batch in order and the lock
/// uncontended. And while anything runs [busy] is set, which the screen reads to hold the shutter, the
/// library and the change sheet, so a batch runs under the club and intent it started with.
///
/// What a photo can end as, each shown and none guessed:
///
/// - **An answer from `record_photo`**, [last]: `Read`, `Flagged`, `Duplicate` or `Failed`, each with
///   Rust's card or sentence. Every one is logged, `failed` included, since the bay report reads the
///   log alone for the session's size, T4 and T5 (the plan's P14 finding 10).
/// - **A refusal**, [problem]: Rust's sentence saying what to change (no golfer, no club, or a mode and
///   shape that do not go together). It has no status, so it writes no log line (P6 finding 8), and it
///   ends a batch, because every photo after it would be refused for the same reason.
/// - **A photo Swift could not open**, a sentence in [notes]: no boxes, so `record_photo` is never
///   called and there is no card, no status and no log line. **So it is in none of the bay's
///   counts**, as a refusal is not, and the screen says so rather than leave it silent (P18 finding 8).
///   A batch carries on past it, since the next photo may open. A camera photo always opens, so only
///   the library reaches this.
///
/// [notes] also holds what the library picker would not hand over (P20 finding 3), shown with the
/// batch. Both lists start empty at each tap.
///
/// **A flagged shot is reviewed before the next photo** (D9) [M38 P22]: every card `record_photo`
/// answers with its stored `needs_review` set is queued, and the queue is put in front of the golfer
/// through [review] once the tap's work is done, one forced review at a time, before [busy] clears. So
/// a camera photo's review follows its card and its log line, and a batch's come after its last photo,
/// in the order read (C14), and a batch a refusal ended still has its flagged shots reviewed. A duplicate whose held shot is still
/// flagged is queued too, since the card's flag is the stored one and is what forces a review: a review
/// can be cut off, by the app being closed, and nothing else would ask again.
///
/// **Under a challenge** (D11) [M38 P23], each photo is recorded with [challenge]'s call as it stood when
/// the photo reached the flow, and a recorded one (`Read` or `Flagged`) draws the next. `Failed`,
/// `Duplicate`, a refusal and a photo Swift cannot open keep the call, since nothing was recorded under
/// it. **A library batch of more than one is refused under a challenge**, recording none: the call is
/// shown before the shot, so only one photo can have been hit to it, and stamping the rest with calls
/// drawn after they were taken would grade them in shape topics the golfer never aimed at.
///
/// **What the golfer did is logged beside the reads** (D16) [M38 P30b]: the camera opened and
/// cancelled, the library's pick, and each refusal and note above as an `error`, since the sentence on
/// screen goes at the next tap. Through [AppModel.logEvent], which neither throws nor waits, so a log
/// that cannot be written costs nothing a photo needs.
class RecordFlow extends ChangeNotifier {
  RecordFlow({required this.model}) : challenge = Challenge(model: model);

  final AppModel model;

  /// The challenge's call, which the Record tab shows large and each photo is recorded under.
  final Challenge challenge;

  bool? _camera;
  bool _disposed = false;
  bool _busy = false;
  BatchProgress? _batch;
  RecordOutcome? _last;
  String? _problem;
  List<String> _notes = const [];
  final List<ShotCard> _toReview = [];

  /// How a review is shown, set by the screen. With none, a flagged shot only marks its card, which is
  /// what a flow with no screen can do.
  PresentReview? review;

  /// Whether the phone has a camera, as UIKit answers it, or `null` until [start] has asked. With none
  /// the shutter is hidden and the library is the one way in.
  bool? get camera => _camera;

  /// A photo is being taken, picked, read, recorded or logged.
  bool get busy => _busy;

  /// The batch running, or none.
  BatchProgress? get batch => _batch;

  /// The last answer `record_photo` gave, which the card shows. Cleared by a refusal, so a card from
  /// before it cannot be taken for the photo just refused.
  RecordOutcome? get last => _last;

  /// The sentence the last tap ended on with no answer to show: a refusal, the camera's or the
  /// picker's own refusal, or a log line that could not be written.
  String? get problem => _problem;

  /// A sentence for each photo of the last tap that was not recorded and is not [problem]: one the
  /// picker would not hand over, or one Swift could not open.
  List<String> get notes => _notes;

  @override
  void dispose() {
    _disposed = true;
    challenge.dispose();
    super.dispose();
  }

  /// Every change, unless the screen has gone: a bridged call can answer after its tab was disposed.
  @override
  void notifyListeners() {
    if (!_disposed) {
      super.notifyListeners();
    }
  }

  /// Ask once whether there is a camera. Before the answer neither way is assumed, and the shutter
  /// stays hidden.
  Future<void> start() async {
    _camera = await model.phone.cameraAvailable();
    notifyListeners();
  }

  /// The shutter: one photo, read at once (D5). A cancel is nothing, but for its event.
  Future<void> takePhoto() => _run(() async {
    final aim = _aim();
    model.logEvent(EventKind.cameraOpened, aim);
    final Photo? photo;
    try {
      photo = await model.phone.takePhoto();
    } on PlatformException catch (refused) {
      _refused('camera', _sentenceOf(refused));
      return;
    }
    if (photo == null) {
      model.logEvent(EventKind.cameraCancelled, aim);
      return;
    }
    await _recordOne(photo);
  });

  /// "From library": any number of photos, one at a time, with progress (C14). A cancel is nothing.
  Future<void> importPhotos() => _run(() async {
    final PhotoPick pick;
    try {
      pick = await model.phone.pickPhotos();
    } on PlatformException catch (refused) {
      _refused('library', _sentenceOf(refused));
      return;
    }
    _notes = [...pick.skipped];
    final photos = pick.photos;
    model.logEvent(
      EventKind.libraryPicked,
      [
        counted(photos.length, 'photo'),
        if (pick.skipped.isNotEmpty) '${pick.skipped.length} skipped',
        _aim(),
      ].join(', '),
    );
    for (final skipped in pick.skipped) {
      model.logProblem('library', skipped);
    }
    if (challenge.call != null && photos.length > 1) {
      for (final copy in photos) {
        unawaited(copy.discard());
      }
      _refused(
        'library',
        'A challenge calls one shot at a time, so import one photo for each '
            'call. None of the ${photos.length} photos was recorded.',
      );
      return;
    }
    for (var at = 0; at < photos.length; at++) {
      _batch = (index: at + 1, total: photos.length);
      notifyListeners();
      if (!await _recordOne(photos[at])) {
        // A refusal stands for every photo after it, so none is read. Each is a copy in `tmp/`, and
        // the library still holds the original.
        for (final rest in photos.skip(at + 1)) {
          unawaited(rest.discard());
        }
        return;
      }
    }
  });

  /// Runs [work] with [busy] set, starting from no problem and no notes, and leaves [busy] clear
  /// whatever it ends on.
  Future<void> _run(Future<void> Function() work) async {
    if (_busy) {
      return;
    }
    _busy = true;
    _problem = null;
    _notes = const [];
    notifyListeners();
    try {
      await work();
      await _reviewQueued();
    } finally {
      _toReview.clear();
      _busy = false;
      _batch = null;
      notifyListeners();
    }
  }

  /// The tap's flagged shots, each reviewed in turn, with [busy] still set so nothing else can start.
  Future<void> _reviewQueued() async {
    final review = this.review;
    if (review == null || _toReview.isEmpty) {
      return;
    }
    _batch = null;
    notifyListeners();
    final queued = List.of(_toReview);
    for (var at = 0; at < queued.length; at++) {
      final card = queued[at];
      _apply(
        card,
        await review(
          card,
          forced: true,
          place: (index: at + 1, total: queued.length),
        ),
      );
    }
  }

  /// A card's tap: the same review, which the golfer may leave with nothing changed when the shot is
  /// not flagged (D9). Not while anything runs, so a review cannot open under a batch.
  Future<void> open(ShotCard card) async {
    final review = this.review;
    if (review == null || _busy) {
      return;
    }
    _busy = true;
    notifyListeners();
    try {
      _apply(card, await review(card, forced: card.flagged));
    } finally {
      _busy = false;
      notifyListeners();
    }
  }

  /// What a review's end does to what is shown. A saved card replaces the card held if it is the same
  /// shot (P7 finding 12), and a deleted shot takes it away, with a note, so a card is never left up for
  /// an entry that has gone. A back-out changes nothing.
  void _apply(ShotCard card, ReviewEnd? end) {
    // Every end, a back-out too: a review that stored an intent and then had a value refused can be
    // left with the intent stored (P22 finding 3), and a read of the stores costs one list [M38 P24].
    model.shotsChanged();
    final last = _last;
    final showing = last?.card;
    final same =
        showing != null &&
        showing.sessionId == card.sessionId &&
        showing.swingId == card.swingId;
    switch (end) {
      case ReviewSaved(card: final saved):
        if (same) {
          _last = RecordOutcome(
            status: last!.status,
            card: saved,
            message: last.message,
            photoSha256: last.photoSha256,
            boxes: last.boxes,
            msRust: last.msRust,
          );
        }
      case ReviewDeleted():
        if (same) {
          _last = null;
        }
        _notes = [..._notes, 'Shot ${card.swingId} was deleted.'];
      case null:
        break;
    }
    notifyListeners();
  }

  /// [sentence] as the tap's [problem], and logged as an `error` naming [where] (D16) [M38 P30b].
  void _refused(String where, String sentence) {
    _problem = sentence;
    model.logProblem(where, sentence);
  }

  /// What a photo taken now would be stamped with, for a camera or library event's detail: the club,
  /// and a challenge's call if one is up. [M38 P30b]
  String _aim() => [
    model.state.club ?? 'no club',
    if (challenge.call case final call?) 'call ${call.name}',
  ].join(', ');

  /// One photo through `readScreen`, `record_photo` and `log_read`. False if a refusal ended it, which
  /// ends a batch too.
  Future<bool> _recordOne(Photo photo) async {
    // From the photo reaching Dart: a camera photo's tap answered, or a batch's turn reached.
    final watch = Stopwatch()..start();
    // The call on screen when the photo arrived, which is the one it was hit to. Nothing can change it
    // before `record_photo`, since the sheet is held while the flow is busy.
    final call = challenge.call;
    try {
      final ScreenRead read;
      try {
        read = await model.phone.readScreen(photo.path);
      } on PlatformException catch (unreadable) {
        final note =
            '${photo.path.split('/').last} could not be opened, so it was not '
            'recorded: ${_sentenceOf(unreadable)}';
        _notes = [..._notes, note];
        // In none of the bay's counts, so the event is the one trace of it (P18 finding 8).
        model.logProblem('read', note);
        notifyListeners();
        return true;
      }
      final RecordOutcome outcome;
      try {
        outcome = await model.core.recordPhoto(
          photo: PhotoIn(
            root: model.root,
            path: photo.path,
            boxesJson: read.json,
            capturedAt: photo.capturedAt,
            now: model.now(),
            // Stamped `{shot_shaping, call}` by Rust, and the state's mode stands (P6 finding 13).
            challengeCall: call,
          ),
        );
      } on Refusal catch (refusal) {
        _last = null;
        _refused('record', refusal.message);
        return false;
      }
      _last = outcome;
      // `Read` and `Flagged` opened an entry. A failed read and a duplicate opened none.
      final opened =
          outcome.status == RecordStatus.read ||
          outcome.status == RecordStatus.flagged;
      // A shot recorded under the call draws the next one, shown with this card. A failed read and a
      // duplicate recorded nothing under it, so the golfer is asked for the same shape again.
      if (call != null && opened) {
        challenge.recorded();
      }
      // History, if it is the tab shown while a batch runs, lists each shot as it lands [M38 P24].
      if (opened) {
        model.shotsChanged();
      }
      // Queued here, and reviewed once the tap's work is done: after this photo's log line, and after a
      // batch's last photo (C14).
      if (outcome.card case final card? when card.flagged) {
        _toReview.add(card);
      }
      notifyListeners();
      // The card's first frame, which is where the golfer sees the read: `ms_total`'s end.
      await _nextFrame();
      watch.stop();
      try {
        await model.core.logRead(
          root: model.root,
          read: ReadLog(
            at: model.now(),
            photoSha256: outcome.photoSha256,
            source: photo.source,
            status: outcome.status,
            boxes: outcome.boxes,
            msVision: read.msVision,
            msRust: outcome.msRust,
            msTotal: watch.elapsedMilliseconds,
          ),
        );
      } on Refusal catch (refusal) {
        // The shot is stored and its card stands, but the bay would not count this read, so say so.
        _refused(
          'reads.jsonl',
          'The read is recorded, but its diagnostics line was not: '
              '${refusal.message}',
        );
        return false;
      }
      return true;
    } finally {
      // Once `record_photo` has answered, whatever it said (P20 finding 7), and on a photo that never
      // reached it. Not awaited: it never throws, and the next photo need not wait on a delete.
      unawaited(photo.discard());
    }
  }
}

/// Swift's sentence, which every refusal on the channel carries as its message, or its code if a
/// refusal ever came without one.
String _sentenceOf(PlatformException refused) =>
    refused.message ?? refused.code;

/// Completes once the next frame has been drawn: the one the card's change asked for, which is where
/// `ms_total` stops. A frame is asked for here too, so that this cannot wait on an idle screen if
/// nothing listening drew the change.
Future<void> _nextFrame() {
  final drawn = Completer<void>();
  SchedulerBinding.instance
    ..addPostFrameCallback((_) => drawn.complete())
    ..ensureVisualUpdate();
  return drawn.future;
}
