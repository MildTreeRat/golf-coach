import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/screens/record_screen.dart';
import 'package:golf_app/src/screens/review_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/widgets/change_sheet.dart';
import 'package:golf_app/src/widgets/shot_card.dart';

import 'fakes.dart';
import 'setup_test.dart' show rfc3339Shape;

// The Record tab over fakes, with no Rust and no Swift (C22). [M38 P21]
//
// Every card, sentence and refusal here is one the test gave the fake, so a screen showing it is
// showing the facade's words and not its own. `integration_test/record_test.dart` runs the captures
// through real Vision and real Rust.

const at = '2026-11-05T10:00:00-05:00';

Photo photo(String name, {ReadSource source = ReadSource.camera}) =>
    Photo(path: '/fake/tmp/$name', capturedAt: at, source: source);

/// Boxes as Swift would hand them over: text the app passes on and never reads.
ScreenRead screenRead({int msVision = 120}) =>
    ScreenRead(json: '{"boxes": [], "os": "fake"}', msVision: msVision);

const carry = CardRow(
  field: 'carry_distance',
  label: 'Carry Yd',
  value: 150.2,
  display: '150.2',
  printed: '150.2',
  state: RowState.read,
);

const direction = CardRow(
  field: 'launch_direction',
  label: 'Launch Dir',
  value: -2.2,
  display: '2.2 L',
  printed: '2.2',
  letter: 'L',
  state: RowState.read,
  signLetters: ('L', 'R'),
);

const sidespinNotRead = CardRow(
  field: 'sidespin',
  label: 'Side Spin',
  state: RowState.notRead,
  signLetters: ('L', 'R'),
);

ShotCard shotCard(
  String swing, {
  bool flagged = false,
  List<String> warnings = const [],
  List<CardRow> rows = const [carry, direction, sidespinNotRead],
}) => ShotCard(
  sessionId: '2026-11-05-phone-a1b2',
  swingId: swing,
  flagged: flagged,
  confidence: 0.9,
  warnings: warnings,
  rows: rows,
);

RecordOutcome outcome(
  RecordStatus status, {
  ShotCard? card,
  String? message,
  String sha = 'ab12',
}) => RecordOutcome(
  status: status,
  card: card,
  message: message,
  photoSha256: sha,
  boxes: 42,
  msRust: 20,
);

/// A golfer with two clubs and the 7i chosen, and a second golfer whose bag has no 7i.
FakeCore recordCore({String? club = '7i'}) {
  final core = setUpCore();
  core.stored = core.stored.copyWith(club: () => club);
  core.bags['aaron'] = const [
    BagSlot(club: '7i', loftDeg: 34, loftDisplay: '34°'),
    BagSlot(club: 'pw'),
  ];
  core.golfers.add(
    const GolferView(
      playerId: 'sam',
      displayName: 'Sam',
      handedness: Handedness.left,
    ),
  );
  core.bags['sam'] = const [BagSlot(club: 'driver'), BagSlot(club: 'pw')];
  return core;
}

/// The app over [core] and [phone] on an iPhone-sized screen, settled on the Record tab.
Future<AppModel> pumpRecord(
  WidgetTester tester,
  FakeCore core,
  FakePhone phone,
) async {
  // 390 by 844 points, the iPhone 17's portrait, so the sheet and the card are laid out as there.
  tester.view
    ..physicalSize = const Size(1170, 2532)
    ..devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final model = AppModel(core: core, phone: phone, random: Random(1));
  await tester.pumpWidget(GolfApp(model: model));
  await tester.pumpAndSettle();
  expect(find.byType(RecordScreen), findsOneWidget);
  return model;
}

String header(WidgetTester tester) => tester
    .widget<Text>(find.byKey(const ValueKey('record-header')))
    .textSpan!
    .toPlainText();

Finder cardOf(String swing) =>
    find.byKey(ValueKey('card-2026-11-05-phone-a1b2-$swing'));

Finder rowOf(String field) => find.byKey(ValueKey('row-$field'));

Finder inRow(String field, String text) =>
    find.descendant(of: rowOf(field), matching: find.text(text));

Finder shutter() => find.byKey(const ValueKey('shutter'));

Finder library() => find.byKey(const ValueKey('library'));

ButtonStyleButton buttonAt(WidgetTester tester, Finder finder) =>
    tester.widget<ButtonStyleButton>(finder);

void main() {
  testWidgets(
    'a read shows its card as Rust spelled each row, and is logged after the card is drawn',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.add(
          outcome(RecordStatus.read, card: shotCard('1'), sha: 'c0ffee'),
        );
      final phone = FakePhone(
        taken: [photo('camera-1.jpg')],
        reads: [screenRead()],
      );
      await pumpRecord(tester, core, phone);

      await tester.tap(shutter());
      // Everything up to the card, and no frame yet: the log waits on the card's first frame, since
      // that is where `ms_total` stops.
      await tester.idle();
      expect(core.calls, contains('recordPhoto /fake/tmp/camera-1.jpg'));
      expect(core.logged, isEmpty);
      await tester.pumpAndSettle();

      expect(cardOf('1'), findsOneWidget);
      expect(inRow('carry_distance', 'Carry Yd'), findsOneWidget);
      expect(inRow('carry_distance', '150.2'), findsOneWidget);
      expect(inRow('launch_direction', '2.2 L'), findsOneWidget);
      expect(phone.calls.where((call) => call.startsWith('readScreen')), [
        'readScreen /fake/tmp/camera-1.jpg',
      ]);

      final sent = core.recorded.single;
      expect(sent.root, '${FakePhone.documents}/processed');
      expect(sent.boxesJson, '{"boxes": [], "os": "fake"}');
      expect(sent.capturedAt, at);
      expect(sent.now, matches(rfc3339Shape));
      expect(sent.challengeCall, isNull);

      final line = core.logged.single;
      expect(line.status, RecordStatus.read);
      expect(line.photoSha256, 'c0ffee');
      expect(line.source, ReadSource.camera);
      expect(line.boxes, 42);
      expect(line.msVision, 120);
      expect(line.msRust, 20);
      expect(line.msTotal, greaterThanOrEqualTo(0));
      expect(line.at, matches(rfc3339Shape));
    },
  );

  testWidgets(
    'a row nothing was read for is highlighted and says not read, never 0',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.add(outcome(RecordStatus.read, card: shotCard('1')));
      final phone = FakePhone(
        taken: [photo('camera-1.jpg')],
        reads: [screenRead()],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(inRow('sidespin', CardRowView.notRead), findsOneWidget);
      expect(
        find.descendant(of: rowOf('sidespin'), matching: find.text('0')),
        findsNothing,
      );
      final colors = Theme.of(tester.element(rowOf('sidespin'))).colorScheme;
      Color? background(String field) => tester
          .widget<Container>(
            find
                .descendant(of: rowOf(field), matching: find.byType(Container))
                .first,
          )
          .color;
      expect(background('sidespin'), colors.errorContainer);
      expect(background('carry_distance'), isNull);
    },
  );

  testWidgets(
    'a flagged read marks its card with Rust\'s warnings, is logged as flagged, and then is reviewed',
    (tester) async {
      final card = shotCard(
        '1',
        flagged: true,
        warnings: ['Carry is longer than total, so check both.'],
      );
      final core = recordCore()
        ..recordAnswers.add(outcome(RecordStatus.flagged, card: card))
        ..details['${card.sessionId}/1'] = detailOf(card);
      final phone = FakePhone(
        taken: [photo('camera-1.jpg')],
        reads: [screenRead()],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(
        find.descendant(
          of: find.byKey(
            const ValueKey('card-2026-11-05-phone-a1b2-1'),
            skipOffstage: false,
          ),
          matching: find.byKey(const ValueKey('flagged'), skipOffstage: false),
          skipOffstage: false,
        ),
        findsOneWidget,
      );
      expect(core.logged.single.status, RecordStatus.flagged);
      // Since P22 a flag is a review pushed over the tab, after the log line. `test/review_test.dart`
      // holds what it does.
      expect(find.byType(ReviewScreen), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(ReviewScreen),
          matching: find.text('Carry is longer than total, so check both.'),
        ),
        findsOneWidget,
      );
      expect(
        core.calls.indexOf('logRead flagged'),
        lessThan(core.calls.indexOf('shotDetail ${card.sessionId}/1')),
      );
    },
  );

  testWidgets(
    'a duplicate shows the held shot under Rust\'s sentence, and is logged as one',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.add(
          outcome(
            RecordStatus.duplicate,
            card: shotCard('4'),
            message: 'held, in the fake\'s words',
          ),
        );
      final phone = FakePhone(
        taken: [photo('camera-1.jpg')],
        reads: [screenRead()],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(cardOf('4'), findsOneWidget);
      expect(
        find.descendant(
          of: cardOf('4'),
          matching: find.text('held, in the fake\'s words'),
        ),
        findsOneWidget,
      );
      expect(core.logged.single.status, RecordStatus.duplicate);
    },
  );

  testWidgets(
    'a failed read shows Rust\'s sentence to retake and no card, and is logged as failed',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.add(
          outcome(RecordStatus.failed, message: 'retake, in the fake\'s words'),
        );
      final phone = FakePhone(
        taken: [photo('camera-1.jpg')],
        reads: [screenRead()],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(
        find.descendant(
          of: find.byKey(const ValueKey('failed')),
          matching: find.text('retake, in the fake\'s words'),
        ),
        findsOneWidget,
      );
      expect(find.byType(ShotCardView), findsNothing);
      // The bay report reads the log alone for the session's size, so a failure is a line too.
      expect(core.logged.single.status, RecordStatus.failed);
    },
  );

  testWidgets(
    'a refusal shows Rust\'s sentence, writes no log line, and takes the last card away',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.addAll([
          outcome(RecordStatus.read, card: shotCard('1')),
          'no club, in the fake\'s words',
        ]);
      final phone = FakePhone(
        taken: [photo('camera-1.jpg'), photo('camera-2.jpg')],
        reads: [screenRead(), screenRead()],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(shutter());
      await tester.pumpAndSettle();
      expect(cardOf('1'), findsOneWidget);

      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(
        find.descendant(
          of: find.byKey(const ValueKey('problem')),
          matching: find.text('no club, in the fake\'s words'),
        ),
        findsOneWidget,
      );
      // A card left up could be taken for the photo just refused.
      expect(cardOf('1'), findsNothing);
      expect(core.logged, hasLength(1));
    },
  );

  testWidgets(
    'a photo Swift cannot open is said, never reaches record_photo, and writes no log line',
    (tester) async {
      final core = recordCore();
      final phone = FakePhone(
        picks: [
          PhotoPick(
            photos: [photo('IMG_7.HEIC', source: ReadSource.library_)],
            skipped: const [],
          ),
        ],
        reads: [PlatformException(code: 'unreadable', message: 'not an image')],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(library());
      await tester.pumpAndSettle();

      expect(
        find.text(
          'IMG_7.HEIC could not be opened, so it was not recorded: not an image',
        ),
        findsOneWidget,
      );
      expect(core.recorded, isEmpty);
      expect(core.logged, isEmpty);
    },
  );

  testWidgets(
    'with no camera the shutter is hidden and the library is the one way in',
    (tester) async {
      await pumpRecord(tester, recordCore(), FakePhone(camera: false));
      expect(shutter(), findsNothing);
      expect(library(), findsOneWidget);
    },
  );

  testWidgets('with a camera the shutter is there, and at least 88 across', (
    tester,
  ) async {
    await pumpRecord(tester, recordCore(), FakePhone());
    expect(shutter(), findsOneWidget);
    expect(tester.getSize(shutter()).width, greaterThanOrEqualTo(88));
    expect(library(), findsOneWidget);
  });

  testWidgets(
    'the header follows the stored state: golfer, club, mode and shape, and the device',
    (tester) async {
      final model = await pumpRecord(tester, recordCore(), FakePhone());
      expect(header(tester), 'Aaron · 7i · Fundamentals');
      expect(find.text(deviceName), findsOneWidget);
      expect(deviceName, 'Full Swing');

      await model.editState(
        (state) => state.copyWith(
          club: () => 'pw',
          mode: PracticeMode.shotShaping,
          targetShape: () => TargetShape.draw,
        ),
      );
      await tester.pumpAndSettle();
      expect(header(tester), 'Aaron · pw · Shot shaping: Draw');

      await model.editState((state) => state.copyWith(club: () => null));
      await tester.pumpAndSettle();
      expect(header(tester), 'Aaron · No club · Shot shaping: Draw');
    },
  );

  testWidgets('the change sheet saves each pick to the state as it is made', (
    tester,
  ) async {
    final core = recordCore();
    await pumpRecord(tester, core, FakePhone());
    await tester.tap(find.byKey(const ValueKey('change')));
    await tester.pumpAndSettle();
    expect(find.byType(ChangeSheet), findsOneWidget);
    // The bag's slots alone (D7), in bag order.
    expect(find.byKey(const ValueKey('pick-club-7i')), findsOneWidget);
    expect(find.byKey(const ValueKey('pick-club-pw')), findsOneWidget);
    expect(find.byKey(const ValueKey('pick-club-driver')), findsNothing);

    await tester.tap(find.byKey(const ValueKey('pick-club-pw')));
    await tester.pumpAndSettle();
    expect(core.stored.club, 'pw');

    await tester.tap(find.byKey(const ValueKey('pick-mode-performance')));
    await tester.pumpAndSettle();
    expect(core.stored.mode, PracticeMode.performance);

    await tester.tap(find.text('Done'));
    await tester.pumpAndSettle();
    expect(find.byType(ChangeSheet), findsNothing);
    expect(header(tester), 'Aaron · pw · Performance');
  });

  testWidgets(
    'the sheet will not close on shot shaping with no shape, a shape releases it, and leaving the mode clears it',
    (tester) async {
      final core = recordCore();
      await pumpRecord(tester, core, FakePhone());
      await tester.tap(find.byKey(const ValueKey('change')));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('pick-mode-shotShaping')));
      await tester.pumpAndSettle();
      expect(core.stored.mode, PracticeMode.shotShaping);
      expect(core.stored.targetShape, isNull);
      expect(
        find.text('Pick the shape you are trying to hit.'),
        findsOneWidget,
      );
      final done = find.widgetWithText(FilledButton, 'Done');
      expect(tester.widget<FilledButton>(done).onPressed, isNull);

      // The barrier, above the sheet, asks the sheet's PopScope, which says no.
      await tester.tapAt(const Offset(195, 40));
      await tester.pumpAndSettle();
      expect(find.byType(ChangeSheet), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey('pick-shape-fade')));
      await tester.pumpAndSettle();
      expect(core.stored.targetShape, TargetShape.fade);
      expect(tester.widget<FilledButton>(done).onPressed, isNotNull);

      await tester.tap(find.byKey(const ValueKey('pick-mode-drill')));
      await tester.pumpAndSettle();
      expect(core.stored.mode, PracticeMode.drill);
      expect(core.stored.targetShape, isNull);
      expect(find.byKey(const ValueKey('pick-shape-fade')), findsNothing);

      await tester.tapAt(const Offset(195, 40));
      await tester.pumpAndSettle();
      expect(find.byType(ChangeSheet), findsNothing);
      expect(header(tester), 'Aaron · 7i · Drill');
    },
  );

  testWidgets(
    'choosing another golfer lists their bag, and drops a club their bag does not hold',
    (tester) async {
      final core = recordCore();
      final model = await pumpRecord(tester, core, FakePhone());
      await tester.tap(find.byKey(const ValueKey('change')));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('pick-golfer-sam')));
      await tester.pumpAndSettle();
      expect(core.stored.playerId, 'sam');
      expect(core.stored.club, isNull);
      expect(model.golfer!.displayName, 'Sam');
      expect(find.byKey(const ValueKey('pick-club-driver')), findsOneWidget);
      expect(find.byKey(const ValueKey('pick-club-7i')), findsNothing);

      await tester.tap(find.byKey(const ValueKey('pick-club-pw')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('pick-golfer-aaron')));
      await tester.pumpAndSettle();
      // Aaron's bag holds a pw too, so it stays.
      expect(core.stored.club, 'pw');

      await tester.tap(find.text('Done'));
      await tester.pumpAndSettle();
      expect(header(tester), 'Aaron · pw · Fundamentals');
    },
  );

  testWidgets('the club and mode are kept from one shot to the next', (
    tester,
  ) async {
    final core = recordCore(club: 'pw');
    core
      ..stored = core.stored.copyWith(mode: PracticeMode.performance)
      ..recordAnswers.addAll([
        outcome(RecordStatus.read, card: shotCard('1')),
        outcome(RecordStatus.read, card: shotCard('2')),
      ]);
    final phone = FakePhone(
      taken: [photo('camera-1.jpg'), photo('camera-2.jpg')],
      reads: [screenRead(), screenRead()],
    );
    await pumpRecord(tester, core, phone);
    final before = core.calls.length;

    await tester.tap(shutter());
    await tester.pumpAndSettle();
    await tester.tap(shutter());
    await tester.pumpAndSettle();

    expect(cardOf('2'), findsOneWidget);
    expect(core.calls.skip(before), [
      'recordPhoto /fake/tmp/camera-1.jpg',
      'logRead read',
      'recordPhoto /fake/tmp/camera-2.jpg',
      'logRead read',
    ]);
    expect(core.stored.club, 'pw');
    expect(core.stored.mode, PracticeMode.performance);
    expect(header(tester), 'Aaron · pw · Performance');
  });

  testWidgets(
    'a library batch reads one photo at a time with progress, holds every control, and shows what was skipped',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.addAll([
          for (final swing in ['1', '2', '3'])
            outcome(RecordStatus.read, card: shotCard(swing), sha: swing),
        ]);
      final phone = FakePhone(
        picks: [
          PhotoPick(
            photos: [
              for (final name in ['IMG_1.HEIC', 'IMG_2.HEIC', 'IMG_3.HEIC'])
                photo(name, source: ReadSource.library_),
            ],
            skipped: const ['IMG_4.HEIC, in the fake\'s words'],
          ),
        ],
        reads: [screenRead(), screenRead(), screenRead()],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(library());

      for (final swing in ['1', '2', '3']) {
        await tester.pump();
        expect(find.text('Reading $swing of 3'), findsOneWidget);
        expect(cardOf(swing), findsOneWidget);
        expect(buttonAt(tester, shutter()).onPressed, isNull);
        expect(buttonAt(tester, library()).onPressed, isNull);
        expect(
          buttonAt(tester, find.byKey(const ValueKey('change'))).onPressed,
          isNull,
        );
      }
      await tester.pumpAndSettle();

      expect(find.byKey(const ValueKey('batch')), findsNothing);
      expect(cardOf('3'), findsOneWidget);
      expect(find.text('IMG_4.HEIC, in the fake\'s words'), findsOneWidget);
      expect(buttonAt(tester, library()).onPressed, isNotNull);
      // Each photo read, recorded and logged before the next is read.
      expect(
        [...phone.calls.where((call) => call.startsWith('readScreen'))]
            .map((call) => call.split('/').last),
        ['IMG_1.HEIC', 'IMG_2.HEIC', 'IMG_3.HEIC'],
      );
      expect(
        core.calls.where(
          (call) => call.startsWith('recordPhoto') || call.startsWith('log'),
        ),
        [
          'recordPhoto /fake/tmp/IMG_1.HEIC',
          'logRead read',
          'recordPhoto /fake/tmp/IMG_2.HEIC',
          'logRead read',
          'recordPhoto /fake/tmp/IMG_3.HEIC',
          'logRead read',
        ],
      );
      expect(core.logged.map((line) => (line.source, line.photoSha256)), [
        (ReadSource.library_, '1'),
        (ReadSource.library_, '2'),
        (ReadSource.library_, '3'),
      ]);
    },
  );

  testWidgets(
    'a refusal part way through a batch ends it, and no photo after it is read',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.addAll([
          outcome(RecordStatus.read, card: shotCard('1')),
          'no club, in the fake\'s words',
        ]);
      final phone = FakePhone(
        picks: [
          PhotoPick(
            photos: [
              for (final name in ['IMG_1.HEIC', 'IMG_2.HEIC', 'IMG_3.HEIC'])
                photo(name, source: ReadSource.library_),
            ],
            skipped: const [],
          ),
        ],
        reads: [screenRead(), screenRead(), screenRead()],
      );
      await pumpRecord(tester, core, phone);
      await tester.tap(library());
      await tester.pumpAndSettle();

      expect(find.text('no club, in the fake\'s words'), findsOneWidget);
      expect(
        phone.calls.where((call) => call.startsWith('readScreen')),
        hasLength(2),
      );
      expect(core.logged, hasLength(1));
      expect(find.byKey(const ValueKey('batch')), findsNothing);
    },
  );

  testWidgets('a cancelled camera or pick changes nothing', (tester) async {
    final core = recordCore();
    final phone = FakePhone(taken: [null], picks: [PhotoPick.none]);
    await pumpRecord(tester, core, phone);
    await tester.tap(shutter());
    await tester.pumpAndSettle();
    await tester.tap(library());
    await tester.pumpAndSettle();

    expect(core.recorded, isEmpty);
    expect(find.byKey(const ValueKey('problem')), findsNothing);
    expect(buttonAt(tester, shutter()).onPressed, isNotNull);
  });
}
