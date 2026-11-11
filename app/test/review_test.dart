import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/screens/review_screen.dart';
import 'package:golf_app/src/widgets/shot_card.dart';

import 'fakes.dart';
import 'record_test.dart'
    show
        buttonAt,
        carry,
        direction,
        library,
        outcome,
        photo,
        pumpRecord,
        recordCore,
        screenRead,
        shotCard,
        shutter,
        sidespinNotRead;

// The review screen over fakes, with no Rust and no Swift (C22). [M38 P22]
//
// What it pins is D9 from the golfer's side: a flagged read is a review nothing leaves but Save or
// Delete, an unflagged one is a review on a tap that can be left, and what Save sends is exactly the
// rows a person changed, as typed. Every refusal on screen is one the test gave the fake. The real
// bridge and a real edit on disk are `integration_test/review_test.dart`'s.

const session = '2026-11-05-phone-a1b2';

const ballSpeed = CardRow(
  field: 'ball_speed',
  label: 'Ball Speed Mph',
  value: 120.5,
  display: '120.5',
  printed: '120.5',
  state: RowState.read,
);

const apex = CardRow(
  field: 'apex_height',
  label: 'Apex Ft',
  value: 28.4,
  display: '85.2',
  printed: '85.2',
  state: RowState.read,
);

const rows = [carry, direction, sidespinNotRead, ballSpeed, apex];

const warning = 'Carry is longer than total, so check both.';

ShotCard flaggedCard(String swing) =>
    shotCard(swing, flagged: true, warnings: [warning], rows: rows);

/// The core with [answers] queued for `record_photo` and a detail held for each card among them.
FakeCore coreFor(List<RecordOutcome> answers, {String? photoPath}) {
  final core = recordCore()..recordAnswers.addAll(answers);
  for (final answer in answers) {
    final card = answer.card;
    if (card != null) {
      core.details['$session/${card.swingId}'] = detailOf(
        card,
        photoPath: photoPath,
      );
    }
  }
  return core;
}

/// One camera photo, read as [answer], and everything it leads to settled.
Future<FakeCore> shoot(
  WidgetTester tester,
  RecordOutcome answer, {
  String? photoPath,
}) async {
  final core = coreFor([answer], photoPath: photoPath);
  await pumpRecord(
    tester,
    core,
    FakePhone(taken: [photo('camera-1.jpg')], reads: [screenRead()]),
  );
  await tester.tap(shutter());
  await tester.pumpAndSettle();
  return core;
}

Finder review() => find.byType(ReviewScreen);

Finder field(String name) => find.byKey(ValueKey('field-$name'));

Finder letterOf(String name, String letter) => find.descendant(
  of: find.byKey(ValueKey('sign-$name')),
  matching: find.text(letter),
);

Finder save() => find.byKey(const ValueKey('review-save'));

Finder cardShown(String swing) =>
    find.byKey(ValueKey('card-$session-$swing'), skipOffstage: false);

/// Bring [finder] on screen in the review's scroll, then tap it.
Future<void> tapIn(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
  await tester.pumpAndSettle();
}

Future<void> typeIn(WidgetTester tester, String name, String text) async {
  await tester.ensureVisible(field(name));
  await tester.pumpAndSettle();
  await tester.enterText(field(name), text);
  await tester.pumpAndSettle();
}

/// A control on the tab beneath the review, found offstage, since the review covers it.
ButtonStyleButton offstage(WidgetTester tester, String key) => tester
    .widget<ButtonStyleButton>(find.byKey(ValueKey(key), skipOffstage: false));

void main() {
  testWidgets(
    'a flagged read pushes its review, which neither a close button nor the system\'s back leaves',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );

      expect(review(), findsOneWidget);
      expect(find.text('Shot 1'), findsOneWidget);
      expect(
        find.descendant(of: review(), matching: find.text(warning)),
        findsOneWidget,
      );
      expect(find.byType(CloseButton), findsNothing);
      expect(find.byType(BackButton), findsNothing);
      // The review comes after the read's log line, which is the bay's record of it.
      expect(core.logged.single.status, RecordStatus.flagged);
      expect(
        core.calls.indexOf('logRead flagged'),
        lessThan(core.calls.indexOf('shotDetail $session/1')),
      );

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(review(), findsOneWidget);
      // And the tab beneath is held, so no photo can be taken past it.
      expect(offstage(tester, 'shutter').onPressed, isNull);
      expect(offstage(tester, 'library').onPressed, isNull);
      expect(core.corrections, isEmpty);
    },
  );

  testWidgets(
    'Save with nothing changed sends no edits, releases the shot, and the card is the one Rust answered',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      core.correctAnswers.add(shotCard('1', rows: rows));

      await tester.tap(save());
      await tester.pumpAndSettle();

      expect(review(), findsNothing);
      expect(core.corrections.single.session, session);
      expect(core.corrections.single.swing, '1');
      expect(core.corrections.single.edits, isEmpty);
      expect(core.nows.last, isNotEmpty);
      expect(
        core.calls.where((call) => call.startsWith('setShotIntent')),
        isEmpty,
      );
      // The card held is replaced by Rust's answer, which no longer carries the flag.
      expect(cardShown('1'), findsOneWidget);
      expect(find.byKey(const ValueKey('flagged')), findsNothing);
      expect(buttonAt(tester, shutter()).onPressed, isNotNull);
    },
  );

  testWidgets(
    'a typed number, a letter and a clear each reach correct_shot as typed, and an untouched row is not sent',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      core.correctAnswers.add(shotCard('1', rows: rows));

      await typeIn(tester, 'carry_distance', '151');
      await tapIn(tester, letterOf('launch_direction', 'R'));
      await typeIn(tester, 'sidespin', '640');
      await tapIn(tester, letterOf('sidespin', 'R'));
      await tapIn(tester, find.byKey(const ValueKey('clear-ball_speed')));
      expect(
        tester.widget<TextField>(field('ball_speed')).controller!.text,
        isEmpty,
      );
      await tester.tap(save());
      await tester.pumpAndSettle();

      expect(core.corrections.single.edits, const [
        FieldEdit(field: 'carry_distance', printed: '151'),
        FieldEdit(field: 'launch_direction', printed: '2.2', letter: 'R'),
        FieldEdit(field: 'sidespin', printed: '640', letter: 'R'),
        FieldEdit(field: 'ball_speed'),
      ]);
      expect(review(), findsNothing);
    },
  );

  testWidgets(
    'an emptied field is a clear, a field put back as it was is not sent, and the number goes as typed',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      core.correctAnswers.add(shotCard('1', rows: rows));

      await typeIn(tester, 'carry_distance', '');
      await typeIn(tester, 'launch_direction', '3');
      await typeIn(tester, 'launch_direction', '2.2');
      // Apex is typed in feet, as the tile prints it, and Rust is what turns it into yards.
      await typeIn(tester, 'apex_height', ' 90 ');
      // A not-read row left empty, with a letter picked, is still not a change.
      await tapIn(tester, letterOf('sidespin', 'L'));
      await tester.tap(save());
      await tester.pumpAndSettle();

      expect(core.corrections.single.edits, const [
        FieldEdit(field: 'carry_distance'),
        FieldEdit(field: 'apex_height', printed: ' 90 '),
      ]);
    },
  );

  testWidgets(
    'a refusal shows Rust\'s sentence on the review, which stays up with the flag, and a second Save releases it',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      const refused = 'Carry Yd: \'15x\' is not a number, in the fake\'s words';
      core.correctAnswers.addAll([refused, shotCard('1', rows: rows)]);

      await typeIn(tester, 'carry_distance', '15x');
      await tester.tap(save());
      await tester.pumpAndSettle();

      expect(review(), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('review-problem')),
          matching: find.text(refused),
        ),
        findsOneWidget,
      );
      // The draft stands, so the person corrects the typo rather than starting again.
      expect(
        tester.widget<TextField>(field('carry_distance')).controller!.text,
        '15x',
      );
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(review(), findsOneWidget);

      await typeIn(tester, 'carry_distance', '151');
      await tester.tap(save());
      await tester.pumpAndSettle();
      expect(review(), findsNothing);
      expect(core.corrections.map((saved) => saved.edits), [
        [const FieldEdit(field: 'carry_distance', printed: '15x')],
        [const FieldEdit(field: 'carry_distance', printed: '151')],
      ]);
    },
  );

  testWidgets(
    'a row nothing was read for is highlighted on the review, empty, says not read, and has no letter chosen',
    (tester) async {
      await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );

      Color? background(String name) => tester
          .widget<Container>(
            find
                .descendant(
                  of: find.byKey(ValueKey('edit-$name')),
                  matching: find.byType(Container),
                )
                .first,
          )
          .color;
      final colors = Theme.of(
        tester.element(find.byKey(const ValueKey('edit-sidespin'))),
      ).colorScheme;
      expect(background('sidespin'), colors.errorContainer);
      expect(background('carry_distance'), isNull);
      final sidespin = tester.widget<TextField>(field('sidespin'));
      expect(sidespin.controller!.text, isEmpty);
      expect(sidespin.decoration!.hintText, CardRowView.notRead);
      expect(
        tester
            .widget<SegmentedButton<String>>(
              find.byKey(const ValueKey('sign-sidespin')),
            )
            .selected,
        isEmpty,
      );
      // A read row starts from what the screen printed, and its letter from the one lit.
      expect(
        tester.widget<TextField>(field('launch_direction')).controller!.text,
        '2.2',
      );
      expect(
        tester
            .widget<SegmentedButton<String>>(
              find.byKey(const ValueKey('sign-launch_direction')),
            )
            .selected,
        {'L'},
      );
      expect(
        tester.widget<TextField>(field('apex_height')).controller!.text,
        '85.2',
      );
      // A tile with no letters has no control for one.
      expect(find.byKey(const ValueKey('sign-carry_distance')), findsNothing);
    },
  );

  testWidgets(
    'a batch reviews its flagged shots after its last photo, in the order read, one at a time',
    (tester) async {
      final core = coreFor([
        outcome(RecordStatus.flagged, card: flaggedCard('1'), sha: '1'),
        outcome(RecordStatus.read, card: shotCard('2'), sha: '2'),
        outcome(RecordStatus.flagged, card: flaggedCard('3'), sha: '3'),
      ])..correctAnswers.addAll([shotCard('1'), shotCard('3')]);
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

      // Every photo read and logged before the first review opens (C14).
      expect(
        core.calls.where(
          (call) =>
              call.startsWith('recordPhoto') ||
              call.startsWith('logRead') ||
              call.startsWith('shotDetail'),
        ),
        [
          'recordPhoto /fake/tmp/IMG_1.HEIC',
          'logRead flagged',
          'recordPhoto /fake/tmp/IMG_2.HEIC',
          'logRead read',
          'recordPhoto /fake/tmp/IMG_3.HEIC',
          'logRead flagged',
          'shotDetail $session/1',
        ],
      );
      expect(find.text('Shot 1'), findsOneWidget);
      expect(find.text('Review 1 of 2'), findsOneWidget);
      expect(offstage(tester, 'library').onPressed, isNull);

      await tester.tap(save());
      await tester.pumpAndSettle();
      expect(find.text('Shot 3'), findsOneWidget);
      expect(find.text('Review 2 of 2'), findsOneWidget);
      expect(offstage(tester, 'library').onPressed, isNull);

      await tester.tap(save());
      await tester.pumpAndSettle();
      expect(review(), findsNothing);
      expect(core.corrections.map((saved) => saved.swing), ['1', '3']);
      expect(buttonAt(tester, library()).onPressed, isNotNull);
      // The last photo read is the one shown, and it is Rust's answer to its review.
      expect(cardShown('3'), findsOneWidget);
      expect(find.byKey(const ValueKey('flagged')), findsNothing);
    },
  );

  testWidgets(
    'an unflagged read is not forced, and its card opens the review on a tap, which can be left with nothing written',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.read, card: shotCard('1', rows: rows)),
      );
      expect(review(), findsNothing);
      expect(
        core.calls.where((call) => call.startsWith('shotDetail')),
        isEmpty,
      );

      await tester.tap(cardShown('1'));
      await tester.pumpAndSettle();
      expect(review(), findsOneWidget);
      expect(find.byType(CloseButton), findsOneWidget);

      await typeIn(tester, 'carry_distance', '151');
      await tester.tap(find.byType(CloseButton));
      await tester.pumpAndSettle();
      expect(review(), findsNothing);
      expect(core.corrections, isEmpty);
      expect(buttonAt(tester, shutter()).onPressed, isNotNull);

      // The system's back leaves it too.
      await tester.tap(cardShown('1'));
      await tester.pumpAndSettle();
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(review(), findsNothing);
      expect(core.corrections, isEmpty);
    },
  );

  testWidgets(
    'Delete asks first, then removes the shot, leaves even a forced review, and takes its card away',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );

      await tester.tap(find.byKey(const ValueKey('review-delete')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('delete-keep')));
      await tester.pumpAndSettle();
      expect(review(), findsOneWidget);
      expect(
        core.calls.where((call) => call.startsWith('deleteShot')),
        isEmpty,
      );

      await tester.tap(find.byKey(const ValueKey('review-delete')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('delete-confirm')));
      await tester.pumpAndSettle();

      expect(review(), findsNothing);
      expect(core.calls.where((call) => call.startsWith('deleteShot')), [
        'deleteShot $session/1',
      ]);
      expect(core.corrections, isEmpty);
      expect(find.byType(ShotCardView), findsNothing);
      expect(find.text('Shot 1 was deleted.'), findsOneWidget);
      expect(buttonAt(tester, shutter()).onPressed, isNotNull);
    },
  );

  testWidgets(
    'a refused delete shows Rust\'s sentence and keeps the forced review',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      core.refusedDelete = 'could not delete, in the fake\'s words';

      await tester.tap(find.byKey(const ValueKey('review-delete')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('delete-confirm')));
      await tester.pumpAndSettle();

      expect(review(), findsOneWidget);
      expect(
        find.text('could not delete, in the fake\'s words'),
        findsOneWidget,
      );
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(review(), findsOneWidget);
    },
  );

  testWidgets(
    'the intent is a pick that Save stores before the values, and only when it differs from the shot\'s',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      core.correctAnswers.add(shotCard('1', rows: rows));

      // Every pair Rust takes, and no half of one: shot shaping comes with each shape.
      for (final key in [
        'intent-fundamentals',
        'intent-shotShaping-straight',
        'intent-shotShaping-draw',
        'intent-shotShaping-fade',
        'intent-performance',
        'intent-drill',
      ]) {
        expect(find.byKey(ValueKey(key)), findsOneWidget, reason: key);
      }
      expect(find.byKey(const ValueKey('intent-shotShaping')), findsNothing);
      Finder chosen(String key) => find.descendant(
        of: find.byKey(ValueKey(key)),
        matching: find.byIcon(Icons.check),
      );
      expect(chosen('intent-fundamentals'), findsOneWidget);

      await tapIn(
        tester,
        find.byKey(const ValueKey('intent-shotShaping-draw')),
      );
      expect(chosen('intent-shotShaping-draw'), findsOneWidget);
      expect(chosen('intent-fundamentals'), findsNothing);
      // Nothing is written by a pick.
      expect(
        core.calls.where((call) => call.startsWith('setShotIntent')),
        isEmpty,
      );

      await tester.tap(save());
      await tester.pumpAndSettle();
      expect(
        core.calls.where(
          (call) =>
              call.startsWith('setShotIntent') ||
              call.startsWith('correctShot'),
        ),
        ['setShotIntent $session/1 shotShaping draw', 'correctShot $session/1'],
      );
      expect(review(), findsNothing);
    },
  );

  testWidgets(
    'an intent picked back to the shot\'s own is not sent, and a refused one keeps the review up',
    (tester) async {
      final core = await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
      );
      core
        ..correctAnswers.add(shotCard('1', rows: rows))
        ..refusedIntent = 'not an intent, in the fake\'s words';

      await tapIn(tester, find.byKey(const ValueKey('intent-drill')));
      await tester.tap(save());
      await tester.pumpAndSettle();
      expect(review(), findsOneWidget);
      expect(find.text('not an intent, in the fake\'s words'), findsOneWidget);
      // Refused before the values were sent, so nothing was.
      expect(core.corrections, isEmpty);

      await tapIn(tester, find.byKey(const ValueKey('intent-fundamentals')));
      await tester.tap(save());
      await tester.pumpAndSettle();
      expect(review(), findsNothing);
      expect(
        core.calls.where((call) => call.startsWith('setShotIntent')),
        hasLength(1),
      );
      expect(core.corrections, hasLength(1));
    },
  );

  testWidgets(
    'the photo is the stored one, zoomable, and decoded to fit twice the screen and not whole',
    (tester) async {
      const stored = '/fake/Documents/processed/sessions/$session/1/IMG_1.jpg';
      await shoot(
        tester,
        outcome(RecordStatus.flagged, card: flaggedCard('1')),
        photoPath: stored,
      );

      final viewer = find.byKey(const ValueKey('review-photo'));
      expect(viewer, findsOneWidget);
      expect(tester.widget<InteractiveViewer>(viewer).maxScale, greaterThan(1));
      final image = tester.widget<Image>(
        find.descendant(of: viewer, matching: find.byType(Image)),
      );
      final resized = image.image as ResizeImage;
      // 390 points at 3 pixels each, twice over.
      expect(resized.width, 2340);
      expect(resized.height, 2340);
      expect(resized.policy, ResizeImagePolicy.fit);
      expect((resized.imageProvider as FileImage).file.path, stored);
    },
  );

  testWidgets(
    'a review whose entry is not on this phone says so, and lets go even when forced',
    (tester) async {
      final card = flaggedCard('1');
      final core = coreFor([outcome(RecordStatus.flagged, card: card)]);
      core.details['$session/1'] = 'not on this phone, in the fake\'s words';
      await pumpRecord(
        tester,
        core,
        FakePhone(taken: [photo('camera-1.jpg')], reads: [screenRead()]),
      );
      await tester.tap(shutter());
      await tester.pumpAndSettle();

      expect(review(), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('review-gone')),
          matching: find.text('not on this phone, in the fake\'s words'),
        ),
        findsOneWidget,
      );
      expect(save(), findsNothing);
      await tester.tap(find.byKey(const ValueKey('review-close')));
      await tester.pumpAndSettle();
      expect(review(), findsNothing);
      expect(find.byType(ShotCardView), findsNothing);
      expect(buttonAt(tester, shutter()).onPressed, isNotNull);
    },
  );
}
