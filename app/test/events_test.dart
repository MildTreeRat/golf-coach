import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/state/app_model.dart';

import 'fakes.dart';
import 'history_test.dart' show today;
import 'record_test.dart'
    show
        cardOf,
        library,
        outcome,
        photo,
        pumpRecord,
        recordCore,
        screenRead,
        shotCard,
        shutter;
import 'review_test.dart' show flaggedCard, save, session, shoot, tapIn, typeIn;
import 'settings_test.dart' show fakeZip, pumpSettings, summaryAt, tapExport;
import 'setup_test.dart' show rfc3339Shape;
import 'shot_detail_test.dart' show clubPick, detailCore, detailFor, openDetail;

// `.app/events.jsonl` from the person's side, over fakes (C22): each kind P30b names, written by the
// action it names, and an error nothing caught written too. [M38 P30b]
//
// What these pin is that the action writes its line, with the subject in its detail, and that a log
// which cannot be written stops nothing. The line's shape and the file are Rust's, held by
// `crates/app/tests/record.rs`, and the export carrying it by `tests/export_roundtrip.rs`. Every event
// is written without being awaited (`AppModel.logEvent`), so each test settles before it reads them.

/// Open the Record tab's change sheet.
Future<void> openSheet(WidgetTester tester) async {
  await tester.tap(find.byKey(const ValueKey('change')));
  await tester.pumpAndSettle();
}

/// Tap the change sheet's [key], brought on screen first, since the sheet scrolls.
Future<void> pick(WidgetTester tester, String key) =>
    tapIn(tester, find.byKey(ValueKey(key)));

void main() {
  group('app_start', () {
    testWidgets(
      'a launch after setup logs the screen it opens on, stamped now',
      (tester) async {
        final core = recordCore();
        await pumpRecord(tester, core, FakePhone());

        expect(core.eventsOf(EventKind.appStart), ['ready']);
        expect(core.events.first.kind, EventKind.appStart);
        expect(core.events.first.at, matches(rfc3339Shape));
      },
    );

    testWidgets('a first run logs that it opens on setup', (tester) async {
      final core = FakeCore();
      await tester.pumpWidget(
        GolfApp(
          model: AppModel(core: core, phone: FakePhone()),
        ),
      );
      await tester.pumpAndSettle();

      expect(core.eventsOf(EventKind.appStart), ['golfer']);
    });

    testWidgets(
      'a state that cannot be read logs the start and then why, as an error',
      (tester) async {
        final core = UnreadableCore();
        await tester.pumpWidget(
          GolfApp(
            model: AppModel(core: core, phone: FakePhone()),
          ),
        );
        await tester.pumpAndSettle();

        expect(core.eventsOf(EventKind.appStart), ['failed']);
        expect(core.eventsOf(EventKind.error), [
          'start: ${UnreadableCore.why}',
        ]);
      },
    );
  });

  testWidgets(
    'club_changed: a club picked, and a golfer picked whose bag clears it',
    (tester) async {
      final core = recordCore();
      await pumpRecord(tester, core, FakePhone());
      await openSheet(tester);

      await pick(tester, 'pick-club-pw');
      // Sam's bag has a pw, and no 7i, so this is the golfer alone, and the club stands.
      await pick(tester, 'pick-golfer-sam');

      expect(core.eventsOf(EventKind.clubChanged), ['pw', 'pw, golfer sam']);
      expect(core.eventsOf(EventKind.modeChanged), isEmpty);

      await pick(tester, 'pick-golfer-aaron');
      await pick(tester, 'pick-club-7i');
      await pick(tester, 'pick-golfer-sam');
      expect(core.eventsOf(EventKind.clubChanged).skip(2), [
        'pw, golfer aaron',
        '7i',
        'none, golfer sam',
      ]);
    },
  );

  testWidgets(
    'mode_changed: a mode, a shape, and a challenge turned on, each with what the next photo takes',
    (tester) async {
      final core = recordCore();
      await pumpRecord(tester, core, FakePhone());
      await openSheet(tester);

      await pick(tester, 'pick-mode-performance');
      await pick(tester, 'pick-mode-shotShaping');
      await pick(tester, 'pick-shape-draw');
      await pick(tester, 'challenge');

      expect(core.eventsOf(EventKind.modeChanged), [
        'performance',
        'shotShaping',
        'shotShaping draw',
        'shotShaping draw, challenge straight draw fade',
      ]);
      expect(core.eventsOf(EventKind.clubChanged), isEmpty);
    },
  );

  testWidgets(
    'a setting that is not a shot\'s, the appearance, logs no change',
    (tester) async {
      final core = setUpCore();
      final model = await pumpSettings(tester, core);
      await model.setAppearance(Appearance.light);
      await tester.pumpAndSettle();

      expect(core.eventsOf(EventKind.clubChanged), isEmpty);
      expect(core.eventsOf(EventKind.modeChanged), isEmpty);
    },
  );

  testWidgets(
    'camera_opened then camera_cancelled: a shutter tap and a cancel, naming the club',
    (tester) async {
      final core = recordCore();
      await pumpRecord(tester, core, FakePhone(taken: [null]));

      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(
        [
          for (final event in core.events.skip(1))
            '${event.kind.name} ${event.detail}',
        ],
        ['cameraOpened 7i', 'cameraCancelled 7i'],
      );
      expect(core.recorded, isEmpty);
    },
  );

  testWidgets(
    'camera_opened alone when a photo comes back, which reads.jsonl then records',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.add(outcome(RecordStatus.read, card: shotCard('1')));
      await pumpRecord(
        tester,
        core,
        FakePhone(taken: [photo('camera-1.jpg')], reads: [screenRead()]),
      );

      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(core.eventsOf(EventKind.cameraOpened), ['7i']);
      expect(core.eventsOf(EventKind.cameraCancelled), isEmpty);
      expect(core.logged.single.status, RecordStatus.read);
    },
  );

  testWidgets(
    'library_picked: how many came back and how many did not, and a cancel as none',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.addAll([
          outcome(RecordStatus.read, card: shotCard('1')),
          outcome(RecordStatus.read, card: shotCard('2')),
        ]);
      final skipped = 'IMG_9.HEIC could not be copied, in the fake\'s words.';
      final phone = FakePhone(
        picks: [
          PhotoPick(
            photos: [
              photo('lib-1.jpg', source: ReadSource.library_),
              photo('lib-2.jpg', source: ReadSource.library_),
            ],
            skipped: [skipped],
          ),
          PhotoPick.none,
        ],
        reads: [screenRead(), screenRead()],
      );
      await pumpRecord(tester, core, phone);

      await tester.tap(library());
      await tester.pumpAndSettle();
      await tester.tap(library());
      await tester.pumpAndSettle();

      expect(core.eventsOf(EventKind.libraryPicked), [
        '2 photos, 1 skipped, 7i',
        '0 photos, 7i',
      ]);
      // A photo the picker would not hand over is in no count of the bay's, so it is an error too.
      expect(core.eventsOf(EventKind.error), ['library: $skipped']);
    },
  );

  group('a review', () {
    testWidgets(
      'review_opened and review_saved: a flagged read\'s forced review, saved with nothing changed',
      (tester) async {
        final core = await shoot(
          tester,
          outcome(RecordStatus.flagged, card: flaggedCard('1')),
        );
        expect(core.eventsOf(EventKind.reviewOpened), ['$session/1, forced']);
        expect(core.eventsOf(EventKind.reviewSaved), isEmpty);

        core.correctAnswers.add(shotCard('1'));
        await tester.tap(save());
        await tester.pumpAndSettle();

        expect(core.eventsOf(EventKind.reviewSaved), ['$session/1, 0 edits']);
      },
    );

    testWidgets(
      'review_saved counts the edits sent, and intent_set names the intent a review stored',
      (tester) async {
        final core = await shoot(
          tester,
          outcome(RecordStatus.flagged, card: flaggedCard('1')),
        );
        core.correctAnswers.add(shotCard('1'));

        await typeIn(tester, 'carry_distance', '151');
        await tapIn(
          tester,
          find.byKey(const ValueKey('intent-shotShaping-draw')),
        );
        await tapIn(tester, save());

        expect(core.eventsOf(EventKind.intentSet), [
          '$session/1 shotShaping draw',
        ]);
        expect(core.eventsOf(EventKind.reviewSaved), ['$session/1, 1 edit']);
      },
    );

    testWidgets('a refused save logs the refusal as an error and no save', (
      tester,
    ) async {
      const refused = 'Carry is not a number, in the fake\'s words.';
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      core.correctAnswers.add(refused);

      await tester.tap(save());
      await tester.pumpAndSettle();

      expect(core.eventsOf(EventKind.reviewSaved), isEmpty);
      expect(core.eventsOf(EventKind.error), ['review: $refused']);
    });

    testWidgets('shot_deleted: a delete from the review', (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );

      await tapIn(tester, find.byKey(const ValueKey('review-delete')));
      await tester.tap(find.byKey(const ValueKey('delete-confirm')));
      await tester.pumpAndSettle();

      expect(core.eventsOf(EventKind.shotDeleted), ['$session/1']);
      expect(cardOf('1'), findsNothing);
    });
  });

  group('a shot\'s detail', () {
    testWidgets('club_set: the shot and the club it was given', (tester) async {
      final core = detailCore([detailFor('1')]);
      await openDetail(tester, core);

      await tapIn(tester, clubPick('pw'));

      expect(core.eventsOf(EventKind.clubSet), ['$today/1 pw']);
    });

    testWidgets('intent_set: the shot and the intent it was given', (
      tester,
    ) async {
      final core = detailCore([detailFor('1')]);
      await openDetail(tester, core);

      await tapIn(
        tester,
        find.byKey(const ValueKey('intent-shotShaping-draw')),
      );
      await tapIn(tester, find.byKey(const ValueKey('intent-drill')));

      expect(core.eventsOf(EventKind.intentSet), [
        '$today/1 shotShaping draw',
        '$today/1 drill',
      ]);
    });

    testWidgets('mishit_set: the shot and each verdict, automatic included', (
      tester,
    ) async {
      final core = detailCore([detailFor('1')]);
      await openDetail(tester, core);

      await tapIn(tester, find.byKey(const ValueKey('mishit-confirmed')));
      await tapIn(tester, find.byKey(const ValueKey('mishit-cleared')));
      await tapIn(tester, find.byKey(const ValueKey('mishit-automatic')));

      expect(core.eventsOf(EventKind.mishitSet), [
        '$today/1 confirmed',
        '$today/1 cleared',
        '$today/1 automatic',
      ]);
    });

    testWidgets('a refused action logs the refusal and no action', (
      tester,
    ) async {
      const refused = 'The pw is not in this bag, in the fake\'s words.';
      final core = detailCore([detailFor('1')])..refusedClub = refused;
      await openDetail(tester, core);

      await tapIn(tester, clubPick('pw'));

      expect(core.eventsOf(EventKind.clubSet), isEmpty);
      expect(core.eventsOf(EventKind.error), ['shot detail: $refused']);
    });

    testWidgets('shot_deleted: a delete from the detail screen', (
      tester,
    ) async {
      final core = detailCore([detailFor('1')]);
      await openDetail(tester, core);

      await tester.tap(find.byKey(const ValueKey('detail-delete')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('delete-confirm')));
      await tester.pumpAndSettle();

      expect(core.eventsOf(EventKind.shotDeleted), ['$today/1']);
    });
  });

  group('export_shared', () {
    testWidgets('a share finished, with the shots the zip holds', (
      tester,
    ) async {
      final core = setUpCore()..exportAnswers.add(summaryAt(fakeZip));
      await pumpSettings(tester, core);

      await tapExport(tester);

      expect(core.eventsOf(EventKind.exportShared), [
        'completed true, 6 shots',
      ]);
    });

    testWidgets('a share sheet closed with nothing done', (tester) async {
      final core = setUpCore()..exportAnswers.add(summaryAt(fakeZip, shots: 1));
      await pumpSettings(
        tester,
        core,
        phone: FakePhone(camera: false)..shareCompletes = false,
      );

      await tapExport(tester);

      expect(core.eventsOf(EventKind.exportShared), [
        'completed false, 1 shot',
      ]);
    });

    testWidgets('a share that failed is an error and no share', (tester) async {
      final core = setUpCore()..exportAnswers.add(summaryAt(fakeZip));
      final phone = FakePhone(camera: false)
        ..shareFails = PlatformException(
          code: 'share_failed',
          message: 'The share sheet would not open, in the fake\'s words.',
        );
      await pumpSettings(tester, core, phone: phone);

      await tapExport(tester);

      expect(core.eventsOf(EventKind.exportShared), isEmpty);
      expect(core.eventsOf(EventKind.error), [
        'share: The share sheet would not open, in the fake\'s words.',
      ]);
    });
  });

  group('error', () {
    testWidgets(
      'a refusal the Record tab shows is logged as an error naming where',
      (tester) async {
        const refused = 'Pick a club first, in the fake\'s words.';
        final core = recordCore()..recordAnswers.add(refused);
        await pumpRecord(
          tester,
          core,
          FakePhone(taken: [photo('camera-1.jpg')], reads: [screenRead()]),
        );

        await tester.tap(shutter());
        await tester.pumpAndSettle();

        expect(core.eventsOf(EventKind.error), ['record: $refused']);
      },
    );

    testWidgets(
      'a photo Swift could not open is logged, since it is in none of the bay\'s counts',
      (tester) async {
        final core = recordCore();
        await pumpRecord(
          tester,
          core,
          FakePhone(
            taken: [photo('camera-1.jpg')],
            reads: [
              PlatformException(code: 'unreadable', message: 'Not an image.'),
            ],
          ),
        );

        await tester.tap(shutter());
        await tester.pumpAndSettle();

        expect(core.eventsOf(EventKind.error), [
          'read: camera-1.jpg could not be opened, so it was not recorded: '
              'Not an image.',
        ]);
      },
    );

    testWidgets('an exception thrown in a widget and caught by nothing writes an error, and still reaches the '
        'handler before', (tester) async {
      final core = recordCore();
      final model = await pumpRecord(tester, core, FakePhone());
      final flutterBefore = FlutterError.onError;
      final dispatcherBefore = PlatformDispatcher.instance.onError;
      try {
        logUncaughtErrors(model);
        await tester.pumpWidget(
          MaterialApp(
            home: TextButton(
              key: const ValueKey('throws'),
              onPressed: () => throw StateError('thrown on purpose'),
              child: const Text('Throw'),
            ),
          ),
        );
        await tester.tap(find.byKey(const ValueKey('throws')));
        // The harness's own handler had it, which is the one before ours.
        expect(tester.takeException(), isA<StateError>());
        await tester.pump();
        await model.eventsWritten;
      } finally {
        FlutterError.onError = flutterBefore;
        PlatformDispatcher.instance.onError = dispatcherBefore;
      }

      final logged = core.eventsOf(EventKind.error).single.split('\n');
      expect(logged.first, 'Bad state: thrown on purpose');
      expect(logged[1], contains('gesture'), reason: 'what Flutter was doing');
      expect(logged.length, 2 + AppModel.stackLines);
    });

    testWidgets('an error from a future nothing awaited writes an error, and is answered as the handler before '
        'answered it', (tester) async {
      final core = recordCore();
      final model = await pumpRecord(tester, core, FakePhone());
      final flutterBefore = FlutterError.onError;
      final dispatcherBefore = PlatformDispatcher.instance.onError;
      final seen = <Object>[];
      bool handled;
      try {
        PlatformDispatcher.instance.onError = (error, stack) {
          seen.add(error);
          return true;
        };
        logUncaughtErrors(model);
        final error = StateError('unawaited on purpose');
        handled = PlatformDispatcher.instance.onError!(
          error,
          StackTrace.current,
        );
        expect(seen, [error]);
        PlatformDispatcher.instance.onError = null;
        logUncaughtErrors(model);
        expect(
          PlatformDispatcher.instance.onError!(error, StackTrace.empty),
          isFalse,
          reason: 'with no handler before, it stays unhandled',
        );
        await model.eventsWritten;
      } finally {
        FlutterError.onError = flutterBefore;
        PlatformDispatcher.instance.onError = dispatcherBefore;
      }

      expect(handled, isTrue);
      expect(core.eventsOf(EventKind.error), [
        startsWith('Bad state: unawaited on purpose\n'),
        'Bad state: unawaited on purpose',
      ]);
    });
  });

  testWidgets(
    'a log that cannot be written stops nothing: the photo is read, recorded and logged as a read',
    (tester) async {
      final core = recordCore()
        ..refusedEvent = 'events.jsonl would not open, in the fake\'s words.'
        ..recordAnswers.add(outcome(RecordStatus.read, card: shotCard('1')));
      final model = await pumpRecord(
        tester,
        core,
        FakePhone(taken: [photo('camera-1.jpg')], reads: [screenRead()]),
      );

      await tester.tap(shutter());
      await tester.pumpAndSettle();
      await model.eventsWritten;

      expect(cardOf('1'), findsOneWidget);
      expect(core.logged.single.status, RecordStatus.read);
      expect(find.byKey(const ValueKey('problem')), findsNothing);
      expect(core.events, isEmpty);
    },
  );
}

/// A facade whose state cannot be read, as a torn or unreadable data root would answer.
class UnreadableCore extends FakeCore {
  static const why = 'the state could not be read, in the fake\'s words';

  @override
  Future<AppState> getState({required String root}) async =>
      throw const Refusal(why);
}
