import 'dart:io';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/rust/third_party/app/api/profile.dart';
import 'package:golf_app/src/screens/record_screen.dart';
import 'package:golf_app/src/screens/review_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/state/challenge.dart';
import 'package:golf_app/src/state/record_flow.dart';
import 'package:golf_app/src/widgets/change_sheet.dart';

import 'fakes.dart';
import 'record_test.dart'
    show
        buttonAt,
        cardOf,
        header,
        library,
        outcome,
        photo,
        recordCore,
        screenRead,
        shotCard,
        shutter;

// Challenge mode (D11, C15), over fakes with every draw named, and once through real Rust. [M38 P23]
//
// The fakes say what Rust answered, so these pin what Dart does with the call: when it is drawn, what
// it is drawn over, which photo takes it, and what keeps it. The last test says what the fakes cannot:
// that a call passed to `record_photo` is stamped on the shot and lands in that shape's topic.

/// A [Random] whose every draw the test names, as an index into the ticks in [TargetShape]'s order, and
/// which keeps what each draw was over. A draw nobody named fails the test, since the test did not say
/// what came up.
class Draws implements Random {
  Draws([Iterable<TargetShape> named = const []]) : _named = [...named];

  final List<TargetShape> _named;

  /// Each draw's `max`: how many shapes it was over.
  final List<int> over = [];

  /// What the ticks are, in [TargetShape]'s order, for turning a named shape into the index a draw
  /// answers. All three unless a test says otherwise.
  List<TargetShape> ticks = TargetShape.values;

  /// One more draw, named.
  void name(TargetShape shape) => _named.add(shape);

  @override
  int nextInt(int max) {
    over.add(max);
    if (_named.isEmpty) {
      throw StateError('a draw over $max shapes with none named');
    }
    final at = ticks.indexOf(_named.removeAt(0));
    if (at < 0 || at >= max) {
      throw StateError('a named shape that is not one of the $max ticked');
    }
    return at;
  }

  @override
  bool nextBool() => throw UnsupportedError('a challenge draws only ints');

  @override
  double nextDouble() => throw UnsupportedError('a challenge draws only ints');
}

/// [recordCore]'s golfer and 7i, under [mode] and [shape], with a challenge on over [ticks], or off
/// when [ticks] is null.
FakeCore challengeCore({
  PracticeMode mode = PracticeMode.fundamentals,
  TargetShape? shape,
  List<TargetShape>? ticks = TargetShape.values,
}) {
  final core = recordCore();
  core.stored = core.stored.copyWith(
    mode: mode,
    targetShape: () => shape,
    challenge: ticks != null,
    challengeShapes: ticks ?? const [],
  );
  return core;
}

/// As `record_test.dart`'s `pumpRecord`, with the draws named.
Future<AppModel> pumpChallenge(
  WidgetTester tester,
  FakeCore core,
  FakePhone phone,
  Draws draws,
) async {
  tester.view
    ..physicalSize = const Size(1170, 2532)
    ..devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final model = AppModel(core: core, phone: phone, random: draws);
  await tester.pumpWidget(GolfApp(model: model));
  await tester.pumpAndSettle();
  expect(find.byType(RecordScreen), findsOneWidget);
  return model;
}

/// The call on the Record tab, or null with none showing.
String? shownCall(WidgetTester tester) {
  final shape = find.byKey(const ValueKey('challenge-shape'));
  return shape.evaluate().isEmpty ? null : tester.widget<Text>(shape).data;
}

Future<void> openSheet(WidgetTester tester) async {
  await tester.tap(find.byKey(const ValueKey('change')));
  await tester.pumpAndSettle();
}

Future<void> tapInSheet(WidgetTester tester, String key) async {
  final target = find.byKey(ValueKey(key));
  await tester.ensureVisible(target);
  await tester.pumpAndSettle();
  await tester.tap(target);
  await tester.pumpAndSettle();
}

Future<void> closeSheet(WidgetTester tester) async {
  final done = find.widgetWithText(FilledButton, 'Done');
  await tester.ensureVisible(done);
  await tester.pumpAndSettle();
  await tester.tap(done);
  await tester.pumpAndSettle();
}

/// The button inside one of the sheet's choices, which are keyed on the widget around it.
ButtonStyleButton choice(WidgetTester tester, String key) =>
    tester.widget<ButtonStyleButton>(
      find.descendant(
        of: find.byKey(ValueKey(key)),
        matching: find.bySubtype<ButtonStyleButton>(),
      ),
    );

/// A choice shown as picked, which is a check beside its label.
bool ticked(WidgetTester tester, String key) => find
    .descendant(
      of: find.byKey(ValueKey(key)),
      matching: find.byIcon(Icons.check),
    )
    .evaluate()
    .isNotEmpty;

/// The host's build of the bridge, which `flutter test`'s native-assets hook writes before any test runs
/// (C3's `aarch64-apple-darwin`). `RustLib.init` alone looks for the iOS framework.
String get hostBridge =>
    '${Directory.current.path}/build/native_assets/macos/libgolf_bridge.dylib';

/// The repo, from `app/`, where `flutter test` runs.
String get repo => Directory.current.parent.path;

/// A phone whose documents are a scratch directory, for the test through real Rust: the camera hands
/// over what the test queues and the reader answers M33's committed boxes for it.
class ScratchFakePhone extends FakePhone {
  ScratchFakePhone(this.scratch, {super.taken, super.reads});

  final String scratch;

  @override
  Future<String> documentsDir() async => scratch;
}

void main() {
  group('the change sheet', () {
    testWidgets(
      'offers the challenge under every mode but Drill, and turning it on ticks all three',
      (tester) async {
        final core = challengeCore(ticks: null);
        final draws = Draws([TargetShape.fade]);
        await pumpChallenge(tester, core, FakePhone(), draws);
        await openSheet(tester);

        for (final mode in [
          PracticeMode.shotShaping,
          PracticeMode.performance,
          PracticeMode.fundamentals,
        ]) {
          await tapInSheet(tester, 'pick-mode-${mode.name}');
          expect(find.byKey(const ValueKey('challenge')), findsOneWidget);
        }
        await tapInSheet(tester, 'pick-mode-drill');
        expect(find.byKey(const ValueKey('challenge')), findsNothing);
        await tapInSheet(tester, 'pick-mode-performance');

        expect(find.byKey(const ValueKey('tick-shape-draw')), findsNothing);
        await tapInSheet(tester, 'challenge');
        expect(core.stored.challenge, isTrue);
        expect(core.stored.challengeShapes, TargetShape.values);
        for (final shape in TargetShape.values) {
          expect(ticked(tester, 'tick-shape-${shape.name}'), isTrue);
        }
        // The first call, drawn over all three, the moment it was turned on.
        expect(draws.over, [3]);
        // Nothing about the challenge is written into the mode it goes back to.
        expect(core.stored.mode, PracticeMode.performance);
        expect(core.stored.targetShape, isNull);
      },
    );

    testWidgets(
      'under a challenge Drill cannot be picked, the last tick cannot come off, and the ticks outlast it',
      (tester) async {
        final core = challengeCore();
        final draws = Draws([TargetShape.fade]);
        await pumpChallenge(tester, core, FakePhone(), draws);
        await openSheet(tester);

        expect(choice(tester, 'pick-mode-drill').onPressed, isNull);
        expect(choice(tester, 'pick-mode-performance').onPressed, isNotNull);

        await tapInSheet(tester, 'tick-shape-straight');
        expect(core.stored.challengeShapes, [
          TargetShape.draw,
          TargetShape.fade,
        ]);
        await tapInSheet(tester, 'tick-shape-draw');
        expect(core.stored.challengeShapes, [TargetShape.fade]);
        expect(choice(tester, 'tick-shape-fade').onPressed, isNull);
        expect(choice(tester, 'tick-shape-draw').onPressed, isNotNull);
        expect(
          tester
              .widget<FilledButton>(find.widgetWithText(FilledButton, 'Done'))
              .onPressed,
          isNotNull,
        );

        await tapInSheet(tester, 'challenge');
        expect(core.stored.challenge, isFalse);
        expect(choice(tester, 'pick-mode-drill').onPressed, isNotNull);
        draws
          ..ticks = [TargetShape.fade]
          ..name(TargetShape.fade);
        await tapInSheet(tester, 'challenge');
        // The ticks last used, and not all three again, with a fresh call drawn over them.
        expect(core.stored.challengeShapes, [TargetShape.fade]);
        expect(draws.over, [3, 1]);
      },
    );

    test('will not close on a challenge with nothing ticked', () {
      final state = challengeCore(ticks: const []).stored;
      expect(sheetCanClose(state), isFalse);
      expect(sheetCanClose(state.copyWith(challenge: false)), isTrue);
      expect(
        sheetCanClose(state.copyWith(challengeShapes: [TargetShape.draw])),
        isTrue,
      );
      // And the pair it goes back to is still asked under a challenge.
      expect(
        sheetCanClose(
          state.copyWith(
            challengeShapes: [TargetShape.draw],
            mode: PracticeMode.shotShaping,
          ),
        ),
        isFalse,
      );
    });
  });

  group('the call', () {
    testWidgets(
      'shows large on Record, the header names the challenge, and a look at the sheet does not redraw it',
      (tester) async {
        final core = challengeCore(mode: PracticeMode.performance);
        final draws = Draws([TargetShape.draw]);
        await pumpChallenge(tester, core, FakePhone(), draws);

        expect(shownCall(tester), 'Draw');
        expect(header(tester), 'Aaron · 7i · Challenge');
        final size = tester
            .widget<Text>(find.byKey(const ValueKey('challenge-shape')))
            .style!
            .fontSize!;
        expect(size, greaterThanOrEqualTo(40));

        await openSheet(tester);
        await tapInSheet(tester, 'pick-club-pw');
        await closeSheet(tester);
        expect(shownCall(tester), 'Draw');
        expect(draws.over, [3]);
      },
    );

    testWidgets(
      'a recorded shot takes shot shaping and the call, and the next call is drawn with its card',
      (tester) async {
        final core = challengeCore()
          ..recordAnswers.addAll([
            outcome(RecordStatus.read, card: shotCard('1')),
            outcome(RecordStatus.read, card: shotCard('2')),
          ]);
        final phone = FakePhone(
          taken: [photo('camera-1.jpg'), photo('camera-2.jpg')],
          reads: [screenRead(), screenRead()],
        );
        final draws = Draws([
          TargetShape.draw,
          TargetShape.fade,
          TargetShape.fade,
        ]);
        await pumpChallenge(tester, core, phone, draws);

        await tester.tap(shutter());
        await tester.pumpAndSettle();
        expect(core.recorded.single.challengeCall, TargetShape.draw);
        expect(cardOf('1'), findsOneWidget);
        expect(shownCall(tester), 'Fade');

        // The same shape twice is a draw like any other.
        await tester.tap(shutter());
        await tester.pumpAndSettle();
        expect(core.recorded.last.challengeCall, TargetShape.fade);
        expect(shownCall(tester), 'Fade');
        expect(draws.over, [3, 3, 3]);
        expect(core.stored.mode, PracticeMode.fundamentals);
      },
    );

    testWidgets(
      'a failed read, a duplicate, a refusal and a photo that will not open each keep the call',
      (tester) async {
        final core = challengeCore()
          ..recordAnswers.addAll([
            outcome(RecordStatus.failed, message: 'retake, in Rust\'s words'),
            outcome(
              RecordStatus.duplicate,
              card: shotCard('1'),
              message: 'already held',
            ),
            'no club, in the fake\'s words',
          ]);
        final phone = FakePhone(
          taken: [
            for (final n in [1, 2, 3, 4]) photo('camera-$n.jpg'),
          ],
          reads: [
            screenRead(),
            screenRead(),
            screenRead(),
            PlatformException(code: 'unreadable', message: 'not an image'),
          ],
        );
        final draws = Draws([TargetShape.straight]);
        await pumpChallenge(tester, core, phone, draws);

        for (var n = 0; n < 4; n++) {
          await tester.tap(shutter());
          await tester.pumpAndSettle();
          expect(shownCall(tester), 'Straight');
        }
        expect(core.recorded.map((sent) => sent.challengeCall), [
          TargetShape.straight,
          TargetShape.straight,
          TargetShape.straight,
        ]);
        expect(draws.over, [3]);
      },
    );

    testWidgets(
      'off restores the prior mode: the header, the next photo and the stored state',
      (tester) async {
        final core = challengeCore(
          mode: PracticeMode.shotShaping,
          shape: TargetShape.fade,
        )..recordAnswers.add(outcome(RecordStatus.read, card: shotCard('1')));
        final phone = FakePhone(
          taken: [photo('camera-1.jpg')],
          reads: [screenRead()],
        );
        final draws = Draws([TargetShape.draw]);
        await pumpChallenge(tester, core, phone, draws);
        expect(header(tester), 'Aaron · 7i · Challenge');

        await openSheet(tester);
        expect(
          find.textContaining('Off goes back to Shot shaping: Fade'),
          findsOneWidget,
        );
        await tapInSheet(tester, 'challenge');
        await closeSheet(tester);

        expect(shownCall(tester), isNull);
        expect(find.byKey(const ValueKey('challenge-call')), findsNothing);
        expect(header(tester), 'Aaron · 7i · Shot shaping: Fade');
        await tester.tap(shutter());
        await tester.pumpAndSettle();
        expect(core.recorded.single.challengeCall, isNull);
        expect(core.stored.mode, PracticeMode.shotShaping);
        expect(core.stored.targetShape, TargetShape.fade);
        expect(draws.over, [3]);
      },
    );

    testWidgets(
      'ticking another shape keeps the call, and taking its own tick off draws again over the rest',
      (tester) async {
        final core = challengeCore(ticks: [TargetShape.draw]);
        final draws = Draws([TargetShape.draw])..ticks = [TargetShape.draw];
        await pumpChallenge(tester, core, FakePhone(), draws);
        expect(shownCall(tester), 'Draw');

        await openSheet(tester);
        await tapInSheet(tester, 'tick-shape-fade');
        expect(draws.over, [1]);
        draws
          ..ticks = [TargetShape.fade]
          ..name(TargetShape.fade);
        await tapInSheet(tester, 'tick-shape-draw');
        await closeSheet(tester);

        expect(shownCall(tester), 'Fade');
        expect(draws.over, [1, 1]);
      },
    );

    testWidgets(
      'a library batch of more than one records none under a challenge, and one photo takes the call',
      (tester) async {
        final core = challengeCore()
          ..recordAnswers.add(outcome(RecordStatus.read, card: shotCard('1')));
        final phone = FakePhone(
          picks: [
            PhotoPick(
              photos: [
                for (final name in ['IMG_1.HEIC', 'IMG_2.HEIC'])
                  photo(name, source: ReadSource.library_),
              ],
              skipped: const [],
            ),
            PhotoPick(
              photos: [photo('IMG_3.HEIC', source: ReadSource.library_)],
              skipped: const [],
            ),
          ],
          reads: [screenRead()],
        );
        final draws = Draws([TargetShape.fade, TargetShape.draw]);
        await pumpChallenge(tester, core, phone, draws);

        await tester.tap(library());
        await tester.pumpAndSettle();
        expect(
          find.text(
            'A challenge calls one shot at a time, so import one photo for '
            'each call. None of the 2 photos was recorded.',
          ),
          findsOneWidget,
        );
        expect(
          phone.calls.where((call) => call.startsWith('readScreen')),
          isEmpty,
        );
        expect(core.recorded, isEmpty);
        expect(shownCall(tester), 'Fade');

        await tester.tap(library());
        await tester.pumpAndSettle();
        expect(core.recorded.single.challengeCall, TargetShape.fade);
        expect(shownCall(tester), 'Draw');
      },
    );

    testWidgets(
      'a flagged shot is recorded under the call too, and deleting it in its review does not give the call back',
      (tester) async {
        final core = challengeCore()
          ..recordAnswers.add(
            outcome(RecordStatus.flagged, card: shotCard('1', flagged: true)),
          );
        final phone = FakePhone(
          taken: [photo('camera-1.jpg')],
          reads: [screenRead()],
        );
        final draws = Draws([TargetShape.draw, TargetShape.straight]);
        final model = AppModel(core: core, phone: phone, random: draws);
        await tester.pumpWidget(const SizedBox());
        await model.start();
        final reviewed = <ShotCard>[];
        final flow = RecordFlow(model: model)
          ..review = (card, {required forced, place}) async {
            reviewed.add(card);
            return const ReviewDeleted();
          };
        addTearDown(flow.dispose);
        expect(flow.challenge.call, TargetShape.draw);

        final done = flow.takePhoto();
        await tester.pumpAndSettle();
        await done;

        expect(core.recorded.single.challengeCall, TargetShape.draw);
        expect(reviewed.single.swingId, '1');
        expect(flow.notes, ['Shot 1 was deleted.']);
        expect(flow.challenge.call, TargetShape.straight);
        expect(draws.over, [3, 3]);
      },
    );
  });

  group('the draw', () {
    Future<AppModel> started(FakeCore core, Random random) async {
      final model = AppModel(core: core, phone: FakePhone(), random: random);
      await model.start();
      return model;
    }

    test('is uniform over the ticks, and never lands off them', () async {
      for (final ticks in [
        TargetShape.values,
        [TargetShape.draw, TargetShape.fade],
        [TargetShape.straight],
      ]) {
        final model = await started(challengeCore(ticks: ticks), Random(23));
        final challenge = Challenge(model: model);
        final counts = <TargetShape, int>{};
        const draws = 3000;
        for (var n = 0; n < draws; n++) {
          counts.update(challenge.call!, (k) => k + 1, ifAbsent: () => 1);
          challenge.recorded();
        }
        challenge.dispose();
        expect(counts.keys.toSet(), ticks.toSet());
        // A fair share is draws / n. Four standard deviations either side, on a seeded stream.
        final fair = draws / ticks.length;
        final sd = sqrt(draws * (1 / ticks.length) * (1 - 1 / ticks.length));
        for (final count in counts.values) {
          expect((count - fair).abs(), lessThanOrEqualTo(4 * sd + 1e-9));
        }
      }
    });

    test('is over the ticks in the order shapes are listed, whatever order they were stored in', () async {
      final draws = Draws([TargetShape.fade])
        ..ticks = [TargetShape.draw, TargetShape.fade];
      final model = await started(
        challengeCore(ticks: [TargetShape.fade, TargetShape.draw]),
        draws,
      );
      expect(challengeTicks(model.state), [TargetShape.draw, TargetShape.fade]);
      final challenge = Challenge(model: model);
      expect(challenge.call, TargetShape.fade);
      expect(draws.over, [2]);
      challenge.dispose();
    });

    test(
      'draws nothing with no challenge on, and a fresh call on each start',
      () async {
        final draws = Draws();
        final off = await started(challengeCore(ticks: null), draws);
        final none = Challenge(model: off);
        expect(none.call, isNull);
        expect(draws.over, isEmpty);
        none.dispose();

        // A relaunch under a challenge: the call is not stored, so a new one is drawn.
        final on = await started(
          challengeCore(),
          Draws([TargetShape.straight]),
        );
        final again = Challenge(model: on);
        expect(again.call, TargetShape.straight);
        again.dispose();
      },
    );
  });

  group('through real Rust', () {
    setUpAll(
      () async =>
          RustLib.init(externalLibrary: ExternalLibrary.open(hostBridge)),
    );

    testWidgets(
      'a challenge shot is stamped shot shaping and its call, and feeds that shape\'s topic in graded_profile',
      (tester) async {
        // Everything read off the captures is `provisional: pasted copies` (rule 12), here through M33's
        // committed boxes, since `flutter test` has no Vision.
        const capturedAt = '2026-11-05T10:00:00-05:00';
        const session = '2026-11-05-phone-a1b2';
        late final Directory scratch;
        late final List<Photo> copies;
        late final List<ScreenRead> reads;
        await tester.runAsync(() async {
          scratch = Directory.systemTemp.createTempSync('m38-p23-');
          copies = [
            for (final id in ['IMG_2889', 'IMG_2891'])
              Photo(
                path: File('$repo/spikes/vision-ocr/captures/$id.jpg')
                    .copySync('${scratch.path}/$id.jpg')
                    .path,
                capturedAt: capturedAt,
                source: ReadSource.camera,
              ),
          ];
          reads = [
            for (final id in ['IMG_2889', 'IMG_2891'])
              ScreenRead(
                json: File('$repo/spikes/vision-ocr/boxes/C0/raw/$id.json')
                    .readAsStringSync(),
                msVision: 0,
              ),
          ];
          final root = '${scratch.path}/processed';
          const core = RustCore();
          await core.initState(root: root, installId: 'a1b2');
          final golfer = await core.createGolfer(
            root: root,
            name: 'Aaron',
            handedness: Handedness.right,
            now: capturedAt,
          );
          await core.setBagSlot(
            root: root,
            playerId: golfer.playerId,
            club: '7i',
            loftDeg: 34,
            now: capturedAt,
          );
          final state = await core.getState(root: root);
          await core.setState(
            root: root,
            state: state.copyWith(
              playerId: () => golfer.playerId,
              club: () => '7i',
              mode: PracticeMode.performance,
            ),
          );
        });
        addTearDown(() => scratch.deleteSync(recursive: true));
        final root = '${scratch.path}/processed';

        tester.view
          ..physicalSize = const Size(1170, 2532)
          ..devicePixelRatio = 3;
        addTearDown(tester.view.reset);
        final phone = ScratchFakePhone(
          scratch.path,
          taken: [...copies],
          reads: [...reads],
        );
        final draws = Draws([TargetShape.draw, TargetShape.straight]);
        final model = AppModel(
          core: const RustCore(),
          phone: phone,
          random: draws,
        );
        await tester.pumpWidget(GolfApp(model: model));
        // Launched, and the camera's answer in, which is when the shutter is drawn.
        await pumpReal(tester, () => shutter().evaluate().isNotEmpty);

        Future<List<String>> shapeTopics() async {
          final view = (await tester.runAsync(
            () => gradedProfile(root: root, playerId: 'aaron'),
          ))!;
          return [
            for (final club in view.clubs)
              for (final topic in club.topics)
                if (topic.shape != null) '${club.club} ${topic.name}',
          ];
        }

        // One shot as the stored mode, before any challenge: no shape is declared, so no shape topic.
        await tester.tap(shutter());
        await pumpReal(tester, () => idleWith(tester, '$session-1'));
        expect(await shapeTopics(), isEmpty);

        await tester.runAsync(
          () => model.editState(
            (state) => state.copyWith(
              challenge: true,
              challengeShapes: TargetShape.values,
            ),
          ),
        );
        await pumpReal(tester, () => shownCall(tester) == 'Draw');
        await tester.tap(shutter());
        await pumpReal(tester, () => idleWith(tester, '$session-2'));
        expect(shownCall(tester), 'Straight');

        expect(await shapeTopics(), ['7i draw']);
        final rows = (await tester.runAsync(
          () async => [
            for (final swing in ['1', '2'])
              (await const RustCore().shotDetail(
                root: root,
                sessionId: session,
                swingId: swing,
              )).row,
          ],
        ))!;
        expect(rows.map((row) => (row.mode, row.targetShape)), [
          (PracticeMode.performance, null),
          (PracticeMode.shotShaping, TargetShape.draw),
        ]);
        // The mode the challenge goes back to was never touched.
        final stored = (await tester.runAsync(
          () => const RustCore().getState(root: root),
        ))!;
        expect(stored.mode, PracticeMode.performance);
        expect(draws.over, [3, 3]);
      },
    );
  });
}

/// The flow has finished a tap and its card for [entry] is up.
bool idleWith(WidgetTester tester, String entry) =>
    find.byKey(ValueKey('card-$entry')).evaluate().isNotEmpty &&
    buttonAt(tester, shutter()).onPressed != null;

/// Pumps frames while real time passes, until [done], for a test whose calls cross the real bridge:
/// its answers arrive on the real event loop, which a widget test's fake clock does not run.
Future<void> pumpReal(WidgetTester tester, bool Function() done) async {
  for (var at = 0; at < 600; at++) {
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 10)),
    );
    await tester.pump();
    if (done()) {
      return;
    }
  }
  fail('still waiting after 6 s of real time');
}
