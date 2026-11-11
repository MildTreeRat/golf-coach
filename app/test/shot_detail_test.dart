import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/screens/history_screen.dart';
import 'package:golf_app/src/screens/review_screen.dart';
import 'package:golf_app/src/screens/shot_detail_screen.dart';
import 'package:golf_app/src/state/app_model.dart';

import 'fakes.dart';
import 'history_test.dart'
    show enterHistory, pumpApp, reads, rowFor, sessionOf, shotRow, today;
import 'record_test.dart' show recordCore, shotCard;
import 'review_test.dart' show tapIn;
import 'setup_test.dart' show rfc3339Shape;

// A shot's detail screen over fakes, with no Rust and no Swift (C22). [M38 P25]
//
// Every row, sentence and refusal here is one the test gave the fake, so a screen showing it is showing
// the facade's words. What these pin is what Dart does with them: each action calls the one facade
// function it names, with this shot and a `now`, the answer replaces what is shown, a refusal is shown
// in Rust's words with nothing moved, and the screen goes back to History when the shot is gone. The
// real bridge, on disk and in the corpus, is `integration_test/shot_actions_test.dart`'s.

const measured = MeasurementRow(
  name: 'start_line_offline_yds',
  display: '-2.2619',
  unit: 'yards',
  detail: 'Where it started, in the fake\'s words, and not where it finished.',
);

const notMeasured = UnscoredRow(
  name: 'flight_carry_yds',
  label: 'flight_carry_yds',
  reason: 'The flight could not reach this carry, in the fake\'s words.',
);

const note = 'A note, in the fake\'s words.';

const leftOut = 'Not counted, in the fake\'s words.';

/// [swing]'s detail as `shot_detail` answers it, with a card unless [noCard] says why not.
ShotDetail detailFor(
  String swing, {
  String? club = '7i',
  bool flagged = false,
  bool mishit = false,
  MishitVerdict? verdict,
  List<String> leftOut = const [],
  String? photoPath,
  String? noCard,
}) => ShotDetail(
  row: rowWith(
    shotRow(swing, flagged: flagged, leftOut: leftOut, photoPath: photoPath),
    club: () => club,
    mishit: mishit,
    mishitVerdict: () => verdict,
  ),
  card: noCard == null ? shotCard(swing, flagged: flagged) : null,
  noCard: noCard,
  measurements: noCard == null ? const [measured] : const [],
  unscored: noCard == null ? const [notMeasured] : const [],
  notes: noCard == null ? const [note] : const [],
);

/// The golfer's bag of 7i and pw, one session holding [details], and History listing them.
FakeCore detailCore(List<ShotDetail> details) {
  final core = recordCore();
  core.sessions['aaron'] = [
    sessionOf(today, [for (final detail in details) detail.row]),
  ];
  for (final detail in details) {
    core.details['$today/${detail.row.swingId}'] = detail;
  }
  return core;
}

/// The app, on History, with [swing]'s detail opened from its row.
Future<AppModel> openDetail(
  WidgetTester tester,
  FakeCore core, [
  String swing = '1',
]) async {
  final model = await pumpApp(tester, core);
  await enterHistory(tester);
  await tester.tap(rowFor(swing));
  await tester.pumpAndSettle();
  expect(find.byType(ShotDetailScreen), findsOneWidget);
  return model;
}

Finder clubPick(String club) => find.byKey(ValueKey('detail-club-$club'));

/// Whether [pick] is drawn chosen: a [Pick] is a filled button with a check then.
bool chosen(Finder pick) => find
    .descendant(of: pick, matching: find.byIcon(Icons.check))
    .evaluate()
    .isNotEmpty;

String heading(WidgetTester tester) =>
    tester.widget<Text>(find.byKey(const ValueKey('detail-heading'))).data!;

/// The mishit control's position, by its name: `automatic`, `confirmed` or `cleared`.
String verdictShown(WidgetTester tester) =>
    ((tester.widget(
              find.byKey(const ValueKey('mishit')),
            ) as SegmentedButton).selected.single
            as Enum)
        .name;

String? problem(WidgetTester tester) {
  final found = find.descendant(
    of: find.byKey(const ValueKey('detail-problem')),
    matching: find.byType(Text),
  );
  return found.evaluate().isEmpty ? null : tester.widget<Text>(found).data;
}

List<String> callsOf(FakeCore core, String name) => [
  for (final call in core.calls)
    if (call.startsWith(name)) call,
];

/// Whether each control on the detail screen can be tapped: Delete, Edit, two clubs, an intent and the
/// mishit control's three segments.
List<bool> controlsEnabled(WidgetTester tester) {
  final buttons = [
    for (final key in ['detail-delete', 'detail-edit'])
      tester.widget<ButtonStyleButton>(find.byKey(ValueKey(key))),
    for (final key in ['detail-club-7i', 'detail-club-pw', 'intent-drill'])
      tester.widget<ButtonStyleButton>(
        find.descendant(
          of: find.byKey(ValueKey(key)),
          matching: find.bySubtype<ButtonStyleButton>(),
        ),
      ),
  ];
  // A segmented button is built of text buttons (P19 finding 6), each disabled with the control.
  final segments = tester.widgetList<TextButton>(
    find.descendant(
      of: find.byKey(const ValueKey('mishit')),
      matching: find.byType(TextButton),
    ),
  );
  expect(segments, hasLength(3));
  return [
    for (final button in [...buttons, ...segments]) button.onPressed != null,
  ];
}

/// The facade with `set_shot_club` held until the test lets it answer.
class HeldClubCore extends FakeCore {
  HeldClubCore(FakeCore base)
    : super(state: base.stored, golfers: base.golfers, bags: base.bags) {
    sessions.addAll(base.sessions);
    details.addAll(base.details);
  }

  final gate = Completer<void>();

  @override
  Future<ShotDetail> setShotClub({
    required String root,
    required String sessionId,
    required String swingId,
    required String club,
    required String now,
  }) async {
    await gate.future;
    return super.setShotClub(
      root: root,
      sessionId: sessionId,
      swingId: swingId,
      club: club,
      now: now,
    );
  }
}

void main() {
  testWidgets(
    'a History row opens its detail: the photo, the card, each measurement with how it was taken, '
    'what was not measured, the notes and why it is not counted, all in Rust\'s words',
    (tester) async {
      const stored = '/fake/Documents/processed/sessions/$today/1/IMG_1.jpg';
      final core = detailCore([
        detailFor('1', leftOut: [leftOut], photoPath: stored),
      ]);
      await openDetail(tester, core);

      expect(callsOf(core, 'shotDetail'), ['shotDetail $today/1']);
      expect(heading(tester), '10:00 · 7i');
      final viewer = find.byKey(const ValueKey('detail-photo'));
      expect(viewer, findsOneWidget);
      final image = tester.widget<Image>(
        find.descendant(of: viewer, matching: find.byType(Image)),
      );
      expect(
        ((image.image as ResizeImage).imageProvider as FileImage).file.path,
        stored,
      );
      expect(find.byKey(ValueKey('card-$today-1')), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('detail-left-out')),
          matching: find.text(leftOut),
        ),
        findsOneWidget,
      );

      final measurement = find.byKey(
        const ValueKey('measure-start_line_offline_yds'),
      );
      await tester.ensureVisible(measurement);
      for (final text in [
        measured.name,
        measured.display,
        measured.unit,
        measured.detail,
      ]) {
        expect(
          find.descendant(of: measurement, matching: find.text(text)),
          findsOneWidget,
          reason: text,
        );
      }
      final unscored = find.byKey(const ValueKey('unscored-flight_carry_yds'));
      await tester.ensureVisible(unscored);
      expect(
        find.descendant(of: unscored, matching: find.text(notMeasured.reason)),
        findsOneWidget,
      );
      await tester.ensureVisible(find.byKey(const ValueKey('note')));
      expect(find.text(note), findsOneWidget);
      // Nothing is flagged, so nothing says to review it.
      expect(find.byKey(const ValueKey('detail-flagged')), findsNothing);

      // Back on History, which reads the stores again.
      final before = reads(core);
      await tester.tap(find.byType(BackButton));
      await tester.pumpAndSettle();
      expect(find.byType(ShotDetailScreen), findsNothing);
      expect(find.byType(HistoryScreen), findsOneWidget);
      expect(reads(core), before + 1);
    },
  );

  testWidgets('a club tap calls set_shot_club with that club and shows the answer, and a refusal shows Rust\'s '
      'sentence and moves nothing', (tester) async {
    final core = detailCore([detailFor('1')]);
    final model = await openDetail(tester, core);
    expect(chosen(clubPick('7i')), isTrue);
    expect(chosen(clubPick('pw')), isFalse);

    final version = model.shotsVersion;
    await tapIn(tester, clubPick('pw'));
    expect(callsOf(core, 'setShotClub'), ['setShotClub $today/1 pw']);
    expect(core.nows.last, matches(rfc3339Shape));
    expect(chosen(clubPick('pw')), isTrue);
    expect(chosen(clubPick('7i')), isFalse);
    expect(heading(tester), '10:00 · pw');
    expect(problem(tester), isNull);
    expect(model.shotsVersion, greaterThan(version));

    core.refusedClub = 'No club can be set here, in the fake\'s words.';
    final refusedAt = model.shotsVersion;
    await tapIn(tester, clubPick('7i'));
    expect(callsOf(core, 'setShotClub'), hasLength(2));
    expect(problem(tester), core.refusedClub);
    expect(chosen(clubPick('pw')), isTrue);
    expect(heading(tester), '10:00 · pw');
    expect(model.shotsVersion, refusedAt, reason: 'a refusal wrote nothing');

    // The next action clears the last one's refusal.
    core.refusedClub = null;
    await tapIn(tester, clubPick('7i'));
    expect(problem(tester), isNull);
  });

  testWidgets(
    'the club picker lists the bag alone, and says so when the shot\'s club is not in it',
    (tester) async {
      final core = detailCore([detailFor('1', club: 'driver')]);
      await openDetail(tester, core);

      expect(clubPick('7i'), findsOneWidget);
      expect(clubPick('pw'), findsOneWidget);
      expect(clubPick('driver'), findsNothing);
      expect(chosen(clubPick('7i')) || chosen(clubPick('pw')), isFalse);
      expect(find.text(ShotDetailScreen.notInBag('driver')), findsOneWidget);
    },
  );

  testWidgets('an intent tap calls set_shot_intent with its pair, and a refusal is shown with the old one still '
      'chosen', (tester) async {
    final core = detailCore([detailFor('1')]);
    await openDetail(tester, core);
    final fundamentals = find.byKey(const ValueKey('intent-fundamentals'));
    final draw = find.byKey(const ValueKey('intent-shotShaping-draw'));
    expect(chosen(fundamentals), isTrue);

    await tapIn(tester, draw);
    expect(callsOf(core, 'setShotIntent'), [
      'setShotIntent $today/1 shotShaping draw',
    ]);
    expect(core.nows.last, matches(rfc3339Shape));
    expect(chosen(draw), isTrue);
    expect(chosen(fundamentals), isFalse);

    core.refusedIntent = 'Not that intent, in the fake\'s words.';
    await tapIn(tester, find.byKey(const ValueKey('intent-drill')));
    expect(callsOf(core, 'setShotIntent'), hasLength(2));
    expect(problem(tester), core.refusedIntent);
    expect(chosen(draw), isTrue);
  });

  testWidgets('the mishit control starts on the person\'s verdict, none being automatic, and each position calls '
      'set_shot_mishit with its verdict', (tester) async {
    final core = detailCore([
      detailFor('1', mishit: true),
      detailFor('2', verdict: MishitVerdict.confirmed, mishit: true),
    ]);
    await openDetail(tester, core);
    // Held out by the automatic rule, with no verdict: the control is on automatic, and the line says
    // whose call it is.
    expect(verdictShown(tester), 'automatic');
    expect(
      find.text(
        ShotDetailScreen.mishitInForce(
          (core.details['$today/1']! as ShotDetail).row,
        ),
      ),
      findsOneWidget,
    );

    await tapIn(tester, find.byKey(const ValueKey('mishit-confirmed')));
    expect(callsOf(core, 'setShotMishit'), [
      'setShotMishit $today/1 confirmed',
    ]);
    expect(core.nows.last, matches(rfc3339Shape));
    expect(verdictShown(tester), 'confirmed');
    await tapIn(tester, find.byKey(const ValueKey('mishit-cleared')));
    expect(
      callsOf(core, 'setShotMishit').last,
      'setShotMishit $today/1 cleared',
    );
    expect(verdictShown(tester), 'cleared');
    await tapIn(tester, find.byKey(const ValueKey('mishit-automatic')));
    expect(
      callsOf(core, 'setShotMishit').last,
      'setShotMishit $today/1 null',
      reason: 'automatic is no verdict, which hands the shot back to the rule',
    );
    expect(verdictShown(tester), 'automatic');

    core.refusedMishit = 'No verdict here, in the fake\'s words.';
    await tapIn(tester, find.byKey(const ValueKey('mishit-confirmed')));
    expect(problem(tester), core.refusedMishit);
    expect(verdictShown(tester), 'automatic');

    // A shot with a verdict opens on it.
    await tester.tap(find.byType(BackButton));
    await tester.pumpAndSettle();
    await tester.tap(rowFor('2'));
    await tester.pumpAndSettle();
    expect(verdictShown(tester), 'confirmed');
  });

  test('the mishit line reads both what is in force and whose call it is', () {
    ShotRow row({required bool mishit, MishitVerdict? verdict}) =>
        rowWith(shotRow('1'), mishit: mishit, mishitVerdict: () => verdict);
    final lines = {
      for (final (mishit, verdict) in [
        (true, null),
        (false, null),
        (true, MishitVerdict.confirmed),
        (false, MishitVerdict.cleared),
      ])
        ShotDetailScreen.mishitInForce(row(mishit: mishit, verdict: verdict)),
    };
    expect(lines, hasLength(4), reason: 'four states, four lines');
    expect(
      ShotDetailScreen.mishitInForce(row(mishit: true)),
      contains('automatic rule'),
    );
    expect(
      ShotDetailScreen.mishitInForce(row(mishit: false)),
      contains('automatic rule'),
    );
    expect(
      ShotDetailScreen.mishitInForce(
        row(mishit: true, verdict: MishitVerdict.confirmed),
      ),
      contains('held out'),
    );
    expect(
      ShotDetailScreen.mishitInForce(
        row(mishit: false, verdict: MishitVerdict.cleared),
      ),
      startsWith('Your call: a real shot'),
    );
  });

  testWidgets('delete asks first, a keep calls nothing, and a delete calls delete_shot and goes back to a History '
      'that reads again', (tester) async {
    final core = detailCore([detailFor('1'), detailFor('2')]);
    final model = await openDetail(tester, core);

    await tester.tap(find.byKey(const ValueKey('detail-delete')));
    await tester.pumpAndSettle();
    expect(
      find.text(
        'Its photo and its read go from this phone, and nothing brings them back.',
      ),
      findsOneWidget,
      reason: 'the review\'s dialog, saying it cannot be undone',
    );
    await tester.tap(find.byKey(const ValueKey('delete-keep')));
    await tester.pumpAndSettle();
    expect(callsOf(core, 'deleteShot'), isEmpty);
    expect(find.byType(ShotDetailScreen), findsOneWidget);

    final before = reads(core);
    final version = model.shotsVersion;
    core.sessions['aaron'] = [
      sessionOf(
        today,
        [core.details['$today/2']! as ShotDetail].map((d) => d.row).toList(),
      ),
    ];
    await tester.tap(find.byKey(const ValueKey('detail-delete')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('delete-confirm')));
    await tester.pumpAndSettle();
    expect(callsOf(core, 'deleteShot'), ['deleteShot $today/1']);
    expect(find.byType(ShotDetailScreen), findsNothing);
    expect(find.byType(HistoryScreen), findsOneWidget);
    expect(reads(core), greaterThan(before));
    expect(model.shotsVersion, greaterThan(version));
    expect(rowFor('1'), findsNothing);
    expect(rowFor('2'), findsOneWidget);
  });

  testWidgets('a refused delete shows Rust\'s sentence and looks again, and an entry then found gone says so with '
      'nothing left to tap', (tester) async {
    final core = detailCore([detailFor('1')]);
    await openDetail(tester, core);

    core.refusedDelete = 'Could not remove it, in the fake\'s words.';
    await tester.tap(find.byKey(const ValueKey('detail-delete')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('delete-confirm')));
    await tester.pumpAndSettle();
    expect(problem(tester), core.refusedDelete);
    expect(
      callsOf(core, 'shotDetail'),
      hasLength(2),
      reason: 'it looked again',
    );
    expect(find.byType(ShotDetailScreen), findsOneWidget);
    expect(find.byKey(const ValueKey('detail-delete')), findsOneWidget);

    // Refused after the entry went: looking again finds it gone.
    core.details['$today/1'] = 'Not on this phone, in the fake\'s words.';
    await tester.tap(find.byKey(const ValueKey('detail-delete')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('delete-confirm')));
    await tester.pumpAndSettle();
    expect(problem(tester), core.refusedDelete);
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('detail-gone')),
        matching: find.text('Not on this phone, in the fake\'s words.'),
      ),
      findsOneWidget,
    );
    for (final key in ['detail-delete', 'detail-edit', 'mishit']) {
      expect(find.byKey(ValueKey(key)), findsNothing, reason: key);
    }
    await tester.tap(find.byType(BackButton));
    await tester.pumpAndSettle();
    expect(find.byType(HistoryScreen), findsOneWidget);
  });

  testWidgets(
    'an entry not on this phone when the detail opens says so, with nothing to act on',
    (tester) async {
      final core = detailCore([detailFor('1')]);
      core.details['$today/1'] = 'Not on this phone, in the fake\'s words.';
      await openDetail(tester, core);

      expect(
        find.text('Not on this phone, in the fake\'s words.'),
        findsOneWidget,
      );
      for (final key in [
        'detail-delete',
        'detail-edit',
        'mishit',
        'detail-club-7i',
        'intent-drill',
      ]) {
        expect(find.byKey(ValueKey(key)), findsNothing, reason: key);
      }
    },
  );

  testWidgets(
    'Edit opens the review not forced, and the detail is read again however it ends',
    (tester) async {
      final core = detailCore([detailFor('1')]);
      final model = await openDetail(tester, core);

      await tester.tap(find.byKey(const ValueKey('detail-edit')));
      await tester.pumpAndSettle();
      expect(find.byType(ReviewScreen), findsOneWidget);
      expect(find.byType(CloseButton), findsOneWidget, reason: 'not forced');
      final version = model.shotsVersion;
      await tester.tap(find.byType(CloseButton));
      await tester.pumpAndSettle();
      expect(find.byType(ReviewScreen), findsNothing);
      expect(find.byType(ShotDetailScreen), findsOneWidget);
      // The review's own load, then this screen's, on top of the first.
      expect(callsOf(core, 'shotDetail'), hasLength(3));
      expect(model.shotsVersion, greaterThan(version));

      // A save goes through `correct_shot`, as the review's does, and the card is a tap away too.
      core.correctAnswers.add(shotCard('1'));
      await tapIn(tester, find.byKey(ValueKey('card-$today-1')));
      expect(find.byType(ReviewScreen), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('review-save')));
      await tester.pumpAndSettle();
      expect(callsOf(core, 'correctShot'), ['correctShot $today/1']);
      expect(find.byType(ShotDetailScreen), findsOneWidget);
      expect(callsOf(core, 'shotDetail'), hasLength(5));
    },
  );

  testWidgets(
    'a review that deletes the shot ends the detail too, back on History',
    (tester) async {
      final core = detailCore([detailFor('1')]);
      await openDetail(tester, core);

      await tester.tap(find.byKey(const ValueKey('detail-edit')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('review-delete')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('delete-confirm')));
      await tester.pumpAndSettle();
      expect(callsOf(core, 'deleteShot'), ['deleteShot $today/1']);
      expect(find.byType(ReviewScreen), findsNothing);
      expect(find.byType(ShotDetailScreen), findsNothing);
      expect(find.byType(HistoryScreen), findsOneWidget);
    },
  );

  testWidgets(
    'a flagged shot found here says what releases it, and its review is not forced',
    (tester) async {
      final core = detailCore([detailFor('1', flagged: true)]);
      await openDetail(tester, core);

      expect(find.byKey(const ValueKey('detail-flagged')), findsOneWidget);
      expect(find.text(ShotDetailScreen.flagged), findsOneWidget);
      expect(
        find.byType(ReviewScreen),
        findsNothing,
        reason: 'nothing opens by itself',
      );
      await tester.tap(find.byKey(const ValueKey('detail-edit')));
      await tester.pumpAndSettle();
      expect(find.byType(ReviewScreen), findsOneWidget);
      expect(find.byType(CloseButton), findsOneWidget);
    },
  );

  testWidgets(
    'an entry with no card says why, and offers only a verdict and a delete',
    (tester) async {
      final core = detailCore([
        detailFor('1', noCard: 'No shot is held for it, in the fake\'s words.'),
      ]);
      await openDetail(tester, core);

      expect(
        find.descendant(
          of: find.byKey(const ValueKey('detail-no-card')),
          matching: find.text('No shot is held for it, in the fake\'s words.'),
        ),
        findsOneWidget,
      );
      for (final key in ['detail-edit', 'detail-club-7i', 'intent-drill']) {
        expect(find.byKey(ValueKey(key)), findsNothing, reason: key);
      }
      expect(find.byKey(const ValueKey('mishit')), findsOneWidget);
      expect(find.byKey(const ValueKey('detail-delete')), findsOneWidget);
    },
  );

  testWidgets(
    'while an action is out every control is disabled, and they come back with its answer',
    (tester) async {
      final core = HeldClubCore(detailCore([detailFor('1')]));
      await openDetail(tester, core);
      expect(controlsEnabled(tester), everyElement(isTrue));

      await tester.ensureVisible(clubPick('pw'));
      await tester.pumpAndSettle();
      await tester.tap(clubPick('pw'));
      await tester.pump();
      expect(controlsEnabled(tester), [for (var i = 0; i < 8; i++) false]);

      core.gate.complete();
      await tester.pumpAndSettle();
      expect(controlsEnabled(tester), [for (var i = 0; i < 8; i++) true]);
      expect(chosen(clubPick('pw')), isTrue);
    },
  );
}
