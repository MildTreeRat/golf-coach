import 'dart:async';
import 'dart:io';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/screens/history_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/widgets/shot_card.dart';

import 'challenge_test.dart' show ScratchFakePhone, hostBridge, pumpReal, repo;
import 'fakes.dart';
import 'record_test.dart'
    show outcome, photo, recordCore, screenRead, shotCard, shutter;

// The History tab over fakes, with no Rust and no Swift (C22), and once through real Rust. [M38 P24]
//
// Every row, sentence and refusal here is one the test gave the fake, so a screen showing it is showing
// the facade's words. What these pin is what Dart does with them: the order it was handed, each mark on
// its own row, a value with no display as not read and never 0, and when the stores are read again.

const today = '2026-11-05-phone-a1b2';
const yesterday = '2026-11-04-phone-a1b2';

/// A row as `list_sessions` answers it. Everything a row shows is the test's to set.
ShotRow shotRow(
  String swing, {
  String session = today,
  String time = '10:00',
  String? carry = '150.2',
  String? ballSpeed = '120.5',
  PracticeMode mode = PracticeMode.fundamentals,
  TargetShape? shape,
  bool flagged = false,
  bool corrected = false,
  bool mishit = false,
  bool untracked = false,
  List<String> leftOut = const [],
  String? photoPath,
}) => ShotRow(
  sessionId: session,
  swingId: swing,
  time: '${session.substring(0, 10)}T$time:00-05:00',
  timeDisplay: time,
  club: '7i',
  mode: mode,
  targetShape: shape,
  carryLabel: 'Carry Yd',
  carryDisplay: carry,
  ballSpeedLabel: 'Ball Speed Mph',
  ballSpeedDisplay: ballSpeed,
  flagged: flagged,
  corrected: corrected,
  mishit: mishit,
  untracked: untracked,
  photoPath: photoPath,
  leftOut: leftOut,
);

SessionView sessionOf(String id, List<ShotRow> rows) => SessionView(
  sessionId: id,
  date: id.substring(0, 10),
  count: rows.length,
  rows: rows,
);

/// What `list_sessions` says of a stale entry, as `crate::history` words it.
const stale =
    'Its stored analysis names another photo, so no club or player number '
    'counts it. Saving a review of it analyses it again.';

const drillSentence =
    'Hit as a drill, so no club or player number counts it. Change what it '
    'was practised as if it should count.';

/// The app on an iPhone-sized screen, settled on the Record tab, which is where it opens.
Future<AppModel> pumpApp(
  WidgetTester tester,
  FakeCore core, [
  FakePhone? phone,
]) async {
  tester.view
    ..physicalSize = const Size(1170, 2532)
    ..devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final model = AppModel(
    core: core,
    phone: phone ?? FakePhone(camera: false),
    random: Random(1),
  );
  await tester.pumpWidget(GolfApp(model: model));
  await tester.pumpAndSettle();
  return model;
}

Future<void> enterHistory(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.history_outlined));
  await tester.pumpAndSettle();
}

Future<void> enterRecord(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.photo_camera_outlined));
  await tester.pumpAndSettle();
}

Finder rowFor(String swing, {String session = today}) =>
    find.byKey(ValueKey('shot-$session-$swing'));

Finder inShot(String swing, Finder matching, {String session = today}) =>
    find.descendant(
      of: rowFor(swing, session: session),
      matching: matching,
    );

/// The row keys on screen, top to bottom.
List<String> shownRows(WidgetTester tester) {
  final rows = find.byType(HistoryRow).evaluate().toList()
    ..sort(
      (a, b) => tester
          .getTopLeft(find.byWidget(a.widget))
          .dy
          .compareTo(tester.getTopLeft(find.byWidget(b.widget)).dy),
    );
  return [for (final row in rows) (row.widget.key! as ValueKey<String>).value];
}

/// Scrolls History's list until [target] is built and on screen.
Future<void> scrollTo(WidgetTester tester, Finder target) =>
    tester.scrollUntilVisible(
      target,
      100,
      scrollable: find.descendant(
        of: find.byKey(const ValueKey('history-list')),
        matching: find.byType(Scrollable),
      ),
    );

int reads(FakeCore core) =>
    core.calls.where((call) => call.startsWith('listSessions')).length;

const marks = [
  'mark-flagged',
  'mark-corrected',
  'mark-mishit',
  'mark-drill',
  'mark-left-out',
];

/// The marks [swing]'s row shows.
List<String> marksOn(String swing) => [
  for (final mark in marks)
    if (inShot(swing, find.byKey(ValueKey(mark))).evaluate().isNotEmpty) mark,
];

/// The phone's reader, held until the test lets it answer, so a tap's record lands while the test is
/// on another tab.
class GatedPhone extends FakePhone {
  GatedPhone({super.taken, super.reads}) : super(camera: true);

  final gate = Completer<void>();

  @override
  Future<ScreenRead> readScreen(String path) async {
    await gate.future;
    return super.readScreen(path);
  }
}

/// The facade with each `list_sessions` held until the test answers it, so two reads can be out at once
/// and answer in either order.
class HeldCore extends FakeCore {
  HeldCore(FakeCore base)
    : super(state: base.stored, golfers: base.golfers, bags: base.bags);

  final List<Completer<List<SessionView>>> held = [];

  @override
  Future<List<SessionView>> listSessions({
    required String root,
    required String playerId,
  }) {
    calls.add('listSessions $playerId');
    final answer = Completer<List<SessionView>>();
    held.add(answer);
    return answer.future;
  }
}

void main() {
  testWidgets(
    'sessions and their rows show in the order Rust answers, the newest session open',
    (tester) async {
      final core = recordCore()
        ..sessions['aaron'] = [
          // Rust's order: neither a sort by swing number nor one by session id would keep it.
          sessionOf(today, [
            shotRow('3', time: '10:20'),
            shotRow('1', time: '10:10'),
            shotRow('2', time: '10:00'),
          ]),
          sessionOf(yesterday, [
            shotRow('2', session: yesterday, time: '18:30'),
            shotRow('1', session: yesterday, time: '18:00'),
          ]),
        ];
      await pumpApp(tester, core);
      await enterHistory(tester);

      expect(find.text("Aaron's shots"), findsOneWidget);
      expect(shownRows(tester), [
        'shot-$today-3',
        'shot-$today-1',
        'shot-$today-2',
      ]);
      // Each session headed by its date and its count, the newest first.
      final headers = [
        for (final id in [today, yesterday])
          tester.getTopLeft(find.byKey(ValueKey('session-$id'))).dy,
      ];
      expect(headers[0], lessThan(headers[1]));
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('session-$today')),
          matching: find.text('2026-11-05'),
        ),
        findsOneWidget,
      );
      expect(find.text('3 shots'), findsOneWidget);
      expect(find.text('2 shots'), findsOneWidget);

      // The older session opens on a tap, below the newer one's rows, and closes again.
      await tester.tap(find.byKey(const ValueKey('session-$yesterday')));
      await tester.pumpAndSettle();
      expect(shownRows(tester), [
        'shot-$today-3',
        'shot-$today-1',
        'shot-$today-2',
        'shot-$yesterday-2',
        'shot-$yesterday-1',
      ]);
      await tester.tap(find.byKey(const ValueKey('session-$today')));
      await tester.pumpAndSettle();
      expect(shownRows(tester), ['shot-$yesterday-2', 'shot-$yesterday-1']);
    },
  );

  testWidgets(
    'each mark is on its own row and no other, and a row that counts has none',
    (tester) async {
      final core = recordCore()
        ..sessions['aaron'] = [
          sessionOf(today, [
            shotRow('1'),
            shotRow('2', flagged: true),
            shotRow('3', corrected: true),
            shotRow('4', mishit: true),
            shotRow(
              '5',
              mode: PracticeMode.drill,
              untracked: true,
              leftOut: [drillSentence],
            ),
            shotRow('6', leftOut: [stale]),
          ]),
        ];
      await pumpApp(tester, core);
      await enterHistory(tester);

      expect(marksOn('1'), isEmpty);
      expect(marksOn('2'), ['mark-flagged']);
      expect(marksOn('3'), ['mark-corrected']);
      expect(marksOn('4'), ['mark-mishit']);
      expect(marksOn('5'), ['mark-drill']);
      expect(inShot('2', find.text(HistoryRow.flagged)), findsOneWidget);
      expect(inShot('3', find.text(HistoryRow.corrected)), findsOneWidget);
      expect(inShot('4', find.text(HistoryRow.mishit)), findsOneWidget);
      expect(inShot('5', find.text(HistoryRow.drill)), findsOneWidget);
      // Built as it scrolls, so the last row is reached first.
      await scrollTo(tester, rowFor('6'));
      expect(marksOn('6'), ['mark-left-out']);
      expect(inShot('6', find.text(HistoryRow.leftOut)), findsOneWidget);

      // A shot left out with no mark of its own says why, in Rust's words. A drill's mark says it, so
      // its sentence is not repeated on the row.
      expect(inShot('6', find.text(stale)), findsOneWidget);
      expect(find.text(drillSentence), findsNothing);
      expect(find.byKey(const ValueKey('left-out')), findsOneWidget);

      // Marks stand together: a flagged shot a person has also corrected shows both.
      core.sessions['aaron'] = [
        sessionOf(today, [
          shotRow('1', flagged: true, corrected: true, mishit: true),
        ]),
      ];
      await enterRecord(tester);
      await enterHistory(tester);
      expect(marksOn('1'), ['mark-flagged', 'mark-corrected', 'mark-mishit']);
    },
  );

  testWidgets(
    'a carry not read shows as not read and never as 0, and so does a ball speed',
    (tester) async {
      final core = recordCore()
        ..sessions['aaron'] = [
          sessionOf(today, [
            shotRow('1', carry: null),
            shotRow('2', ballSpeed: null),
          ]),
        ];
      await pumpApp(tester, core);
      await enterHistory(tester);

      final carry = inShot('1', find.byKey(const ValueKey('carry')));
      expect(
        find.descendant(of: carry, matching: find.text('Carry Yd')),
        findsOneWidget,
      );
      expect(
        find.descendant(of: carry, matching: find.text(CardRowView.notRead)),
        findsOneWidget,
      );
      for (final zero in ['0', '0.0']) {
        expect(
          find.descendant(of: carry, matching: find.text(zero)),
          findsNothing,
        );
      }
      expect(
        inShot('1', find.text('120.5')),
        findsOneWidget,
        reason: 'the ball speed beside it was read',
      );

      final speed = inShot('2', find.byKey(const ValueKey('ball-speed')));
      expect(
        find.descendant(of: speed, matching: find.text(CardRowView.notRead)),
        findsOneWidget,
      );
      expect(inShot('2', find.text('150.2')), findsOneWidget);
    },
  );

  testWidgets('a row shows its shot, clock, club, intent and photo', (
    tester,
  ) async {
    final core = recordCore()
      ..sessions['aaron'] = [
        sessionOf(today, [
          shotRow(
            '1',
            time: '20:42',
            mode: PracticeMode.shotShaping,
            shape: TargetShape.draw,
            photoPath: '/fake/processed/sessions/$today/1/IMG_2889.HEIC',
          ),
          shotRow('2'),
        ]),
      ];
    await pumpApp(tester, core);
    await enterHistory(tester);

    // Rust's clock as it reads where the photo was taken, not a Dart parse of the RFC 3339 time.
    expect(inShot('1', find.text('Shot 1 · 20:42 · 7i')), findsOneWidget);
    // A challenge's shot is stored as shot shaping and its call, and shows as any other (P23 finding 5).
    expect(inShot('1', find.text('Shot shaping: Draw')), findsOneWidget);
    expect(
      inShot('1', find.byKey(const ValueKey('thumbnail-photo'))),
      findsOneWidget,
    );
    expect(
      inShot('2', find.byKey(const ValueKey('thumbnail-none'))),
      findsOneWidget,
    );
    // Decoded to a bound, not whole: twice the square's side in pixels at the iPhone's 3x.
    final image = tester.widget<Image>(
      inShot('1', find.byKey(const ValueKey('thumbnail-photo'))),
    );
    final resized = image.image as ResizeImage;
    expect((resized.width, resized.height), (336, 336));
    expect(resized.policy, ResizeImagePolicy.fit);
  });

  testWidgets('a golfer with no shots is told where they come from', (
    tester,
  ) async {
    final core = recordCore();
    await pumpApp(tester, core);
    await enterHistory(tester);

    expect(find.byKey(const ValueKey('history-empty')), findsOneWidget);
    expect(
      find.text(HistoryScreen.noShots(core.golfers.first)),
      findsOneWidget,
    );
    expect(find.textContaining('No shots for Aaron yet.'), findsOneWidget);
    expect(find.byType(HistoryRow), findsNothing);
  });

  testWidgets(
    'the stores are read on entering the tab, each time, and not before',
    (tester) async {
      final core = recordCore();
      final model = await pumpApp(tester, core);
      expect(reads(core), 0, reason: 'the app opens on Record');

      await enterHistory(tester);
      expect(reads(core), 1);
      expect(find.byKey(const ValueKey('history-empty')), findsOneWidget);

      core.sessions['aaron'] = [
        sessionOf(today, [shotRow('1')]),
      ];
      await enterRecord(tester);
      expect(reads(core), 1);
      await enterHistory(tester);
      expect(reads(core), 2);
      expect(rowFor('1'), findsOneWidget);
      expect(find.byKey(const ValueKey('history-empty')), findsNothing);

      // A setting changing while History is shown reads nothing: only a shot written or another golfer.
      await model.setAppearance(Appearance.light);
      await tester.pumpAndSettle();
      expect(reads(core), 2);
    },
  );

  testWidgets(
    'a shot the Record tab records while History is shown is listed as it lands',
    (tester) async {
      final core = recordCore()
        ..recordAnswers.add(outcome(RecordStatus.read, card: shotCard('1')));
      final phone = GatedPhone(taken: [photo('a.jpg')], reads: [screenRead()]);
      await pumpApp(tester, core, phone);

      await tester.tap(shutter());
      await tester.pump();
      // The read is held, so the golfer is on History before the shot is recorded. No settling while
      // it is held: the Record tab's progress bar runs until the flow is done.
      await tester.tap(find.byIcon(Icons.history_outlined));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      expect(find.byType(HistoryScreen), findsOneWidget);
      expect(reads(core), 1);
      expect(find.byKey(const ValueKey('history-empty')), findsOneWidget);

      core.sessions['aaron'] = [
        sessionOf(today, [shotRow('1')]),
      ];
      phone.gate.complete();
      await tester.pumpAndSettle();

      expect(core.recorded, hasLength(1));
      expect(reads(core), 2);
      expect(rowFor('1'), findsOneWidget);
    },
  );

  testWidgets(
    'another golfer is read at once while History is shown, and on entry when not',
    (tester) async {
      final core = recordCore()
        ..sessions['aaron'] = [
          sessionOf(today, [shotRow('1')]),
        ]
        ..sessions['sam'] = [
          sessionOf(today, [shotRow('7')]),
        ];
      final model = await pumpApp(tester, core);
      await enterHistory(tester);
      expect(rowFor('1'), findsOneWidget);

      await model.chooseGolfer('sam');
      await tester.pumpAndSettle();
      expect(core.calls.last, 'listSessions sam');
      expect(find.text("Sam's shots"), findsOneWidget);
      expect(rowFor('1'), findsNothing);
      expect(rowFor('7'), findsOneWidget);

      // Chosen while History is not shown: its list is not read until entry, and until then the last
      // golfer's rows are not left standing under the new golfer's name.
      await enterRecord(tester);
      await model.chooseGolfer('aaron');
      await tester.pumpAndSettle();
      expect(core.calls.last, isNot('listSessions aaron'));
      expect(find.text("Aaron's shots", skipOffstage: false), findsOneWidget);
      expect(
        find.byKey(const ValueKey('shot-$today-7'), skipOffstage: false),
        findsNothing,
      );
      expect(
        find.text(HistoryScreen.reading, skipOffstage: false),
        findsOneWidget,
      );
      await enterHistory(tester);
      expect(core.calls.last, 'listSessions aaron');
      expect(rowFor('1'), findsOneWidget);
    },
  );

  testWidgets(
    'of two reads out at once, the later one asked is the one shown, whichever answers last',
    (tester) async {
      final core = HeldCore(recordCore());
      final model = await pumpApp(tester, core);
      await enterHistory(tester);
      model.shotsChanged();
      await tester.pump();
      expect(core.held, hasLength(2));

      core.held[1].complete([
        sessionOf(today, [shotRow('2'), shotRow('1')]),
      ]);
      await tester.pumpAndSettle();
      expect(rowFor('2'), findsOneWidget);

      // The first read's answer was read before the second shot landed, and arrives after.
      core.held[0].complete([
        sessionOf(today, [shotRow('1')]),
      ]);
      await tester.pumpAndSettle();
      expect(rowFor('2'), findsOneWidget);
      expect(rowFor('1'), findsOneWidget);
    },
  );

  testWidgets('a refusal shows Rust\'s sentence', (tester) async {
    const refused = 'refused by the fake: the device table has lost a tile';
    final core = recordCore()..sessions['aaron'] = refused;
    await pumpApp(tester, core);
    await enterHistory(tester);

    expect(
      find.descendant(
        of: find.byKey(const ValueKey('history-problem')),
        matching: find.text(refused),
      ),
      findsOneWidget,
    );
  });

  testWidgets(
    'a tap on a row opens that shot, and the list is read again on the way back',
    (tester) async {
      final core = recordCore()
        ..sessions['aaron'] = [
          sessionOf(today, [shotRow('2'), shotRow('1')]),
        ];
      final model = AppModel(
        core: core,
        phone: FakePhone(camera: false),
        random: Random(1),
      );
      await model.start();
      final opened = <String>[];
      final back = Completer<void>();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HistoryScreen(
              model: model,
              active: true,
              openShot: (context, row) {
                opened.add('${row.sessionId}/${row.swingId}');
                return back.future;
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(reads(core), 1);

      await tester.tap(rowFor('1'));
      await tester.pumpAndSettle();
      expect(opened, ['$today/1']);
      expect(reads(core), 1, reason: 'not while the detail is up');

      core.sessions['aaron'] = [
        sessionOf(today, [shotRow('2')]),
      ];
      back.complete();
      await tester.pumpAndSettle();
      expect(reads(core), 2);
      expect(rowFor('1'), findsNothing);
    },
  );

  group('through real Rust', () {
    setUpAll(
      () async =>
          RustLib.init(externalLibrary: ExternalLibrary.open(hostBridge)),
    );

    testWidgets(
      'History lists what record_photo stored, in Rust\'s order, with its values and a drill marked',
      (tester) async {
        // Read off two captures through M33's committed boxes, since `flutter test` has no Vision, so
        // `provisional: pasted copies` (rule 12).
        late final Directory scratch;
        late final List<ShotCard> cards;
        late final List<SessionView> answered;
        await tester.runAsync(() async {
          scratch = Directory.systemTemp.createTempSync('m38-p24-');
          final root = '${scratch.path}/processed';
          const core = RustCore();
          await core.initState(root: root, installId: 'a1b2');
          final golfer = await core.createGolfer(
            root: root,
            name: 'Aaron',
            handedness: Handedness.right,
            now: '2026-11-05T09:00:00-05:00',
          );
          await core.setBagSlot(
            root: root,
            playerId: golfer.playerId,
            club: '7i',
            loftDeg: 34,
            now: '2026-11-05T09:00:00-05:00',
          );
          final state = await core.getState(root: root);
          await core.setState(
            root: root,
            state: state.copyWith(
              playerId: () => golfer.playerId,
              club: () => '7i',
            ),
          );
          // Recorded out of the order they were taken, so arrival and the photo's time disagree.
          cards = [
            for (final (id, time) in [
              ('IMG_2891', '10:05'),
              ('IMG_2889', '10:00'),
            ])
              (await core.recordPhoto(
                photo: PhotoIn(
                  root: root,
                  path: File('$repo/spikes/vision-ocr/captures/$id.jpg')
                      .copySync('${scratch.path}/$id.jpg')
                      .path,
                  boxesJson: File(
                    '$repo/spikes/vision-ocr/boxes/C0/raw/$id.json',
                  ).readAsStringSync(),
                  capturedAt: '2026-11-05T$time:00-05:00',
                  now: '2026-11-05T11:00:00-05:00',
                ),
              )).card!,
          ];
          await core.setShotIntent(
            root: root,
            sessionId: today,
            swingId: '2',
            mode: PracticeMode.drill,
            now: '2026-11-05T11:01:00-05:00',
          );
          answered = await core.listSessions(root: root, playerId: 'aaron');
        });
        addTearDown(() => scratch.deleteSync(recursive: true));

        tester.view
          ..physicalSize = const Size(1170, 2532)
          ..devicePixelRatio = 3;
        addTearDown(tester.view.reset);
        final model = AppModel(
          core: const RustCore(),
          phone: ScratchFakePhone(scratch.path)..camera = false,
          random: Random(1),
        );
        await tester.pumpWidget(GolfApp(model: model));
        await pumpReal(
          tester,
          () => find.byIcon(Icons.history_outlined).evaluate().isNotEmpty,
        );
        await tester.tap(find.byIcon(Icons.history_outlined));
        await pumpReal(tester, () => rowFor('2').evaluate().isNotEmpty);

        expect(answered.single.rows.map((row) => row.swingId), ['1', '2']);
        expect(shownRows(tester), ['shot-$today-1', 'shot-$today-2']);
        expect(marksOn('1'), isEmpty);
        expect(marksOn('2'), ['mark-drill']);
        for (final (at, card) in cards.indexed) {
          final carry = card.rows.firstWhere(
            (row) => row.field == 'carry_distance',
          );
          final swing = '${at + 1}';
          expect(card.swingId, swing);
          expect(
            find.descendant(
              of: inShot(swing, find.byKey(const ValueKey('carry'))),
              matching: find.text(carry.display ?? CardRowView.notRead),
            ),
            findsOneWidget,
            reason: 'shot $swing',
          );
        }
        expect(inShot('2', find.text('Drill')), findsOneWidget);
      },
    );
  });
}
