// What the tab showed is this test's output, read off the console and the file it writes, so it prints
// by design.
// ignore_for_file: avoid_print

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/screens/profile_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/widgets/topic_tile.dart';
import 'package:integration_test/integration_test.dart';

import 'record_test.dart' show TestPhone, installId;
import 'setup_test.dart' show pumpUntil;

// The six captures' graded profile on the Profile tab, through the real bridge and real Rust. [M38 P26]
//
// The widget tests say what the tab does with each state of a `ProfileView`. This says that the view
// real Rust answers for real stores is what the tab draws: six shots of one club, recorded through the
// facade with M33's committed boxes, which P17 held to the app's own reader, so the Record tab's Vision
// is not what this tests. **With six shots of one club most grades are withheld, and that is the right
// answer on them** (P11 finding 11): every topic waits for its 10 judged shots, no blend exists, nothing
// is established, and every spread waits too, while each mean, which clears at 5, is through. Everything
// read off the captures is `provisional: pasted copies` (rule 12), and on the Simulator, not a phone.

const repo = String.fromEnvironment('GOLF_REPO');

/// The six captures with boxes, a minute apart in one session.
const captures = [
  'IMG_2889',
  'IMG_2891',
  'IMG_2892',
  'IMG_2893',
  'IMG_2894',
  '2026-10-07-1',
];

Finder byKey(String key) => find.byKey(ValueKey(key));

Finder inside(String key, Finder matching) =>
    find.descendant(of: byKey(key), matching: matching);

String textAt(WidgetTester tester, Finder finder) => tester
    .widget<Text>(
      find.descendant(of: finder, matching: find.byType(Text), matchRoot: true),
    )
    .data!;

/// Scrolls the profile until [target] is built and on screen, down or, with [up], back up.
Future<void> scrollTo(
  WidgetTester tester,
  Finder target, {
  bool up = false,
}) async {
  await tester.pumpAndSettle();
  await tester.scrollUntilVisible(
    target,
    up ? -100 : 100,
    scrollable: find.descendant(
      of: byKey('profile-list'),
      matching: find.byType(Scrollable),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets(
    'the six captures\' profile, as real Rust grades it, is what the tab draws',
    (tester) async {
      expect(repo, isNotEmpty, reason: 'run this through app/tool/sim_test.sh');
      final scratch = await Directory.systemTemp.createTemp('m38-profile-');
      addTearDown(() => scratch.delete(recursive: true));
      final root = '${scratch.path}/processed';
      const core = RustCore();

      await core.initState(root: root, installId: installId);
      final golfer = await core.createGolfer(
        root: root,
        name: 'Aaron',
        handedness: Handedness.right,
        now: now(),
      );
      // The pitching wedge is in the bag and never hit, so the selector has a club to say is missing.
      for (final (club, loft) in [('7i', 34.0), ('pw', 46.0)]) {
        await core.setBagSlot(
          root: root,
          playerId: golfer.playerId,
          club: club,
          loftDeg: loft,
          now: now(),
        );
      }
      final state = await core.getState(root: root);
      await core.setState(
        root: root,
        state: state.copyWith(
          playerId: () => golfer.playerId,
          club: () => '7i',
        ),
      );
      for (final (index, id) in captures.indexed) {
        final copy = File('$repo/spikes/vision-ocr/captures/$id.jpg')
            .copySync('${scratch.path}/$id.jpg');
        final recorded = await core.recordPhoto(
          photo: PhotoIn(
            root: root,
            path: copy.path,
            boxesJson: File('$repo/spikes/vision-ocr/boxes/C0/raw/$id.json')
                .readAsStringSync(),
            capturedAt: '2026-11-05T10:0$index:00-05:00',
            now: now(),
          ),
        );
        expect(recorded.status, isNot(RecordStatus.failed), reason: id);
      }
      final view = await core.gradedProfile(root: root, playerId: 'aaron');
      expect(view.clubs.map((club) => club.club), ['7i']);

      final model = AppModel(core: core, phone: TestPhone(scratch.path));
      await tester.pumpWidget(GolfApp(model: model));
      await pumpUntil(tester, find.text('Aaron · 7i · Fundamentals'));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.insights_outlined));
      await pumpUntil(tester, byKey('profile-heading'));
      await tester.pumpAndSettle();

      // The selector: the one club hit, then Player, and the wedge named as not in the profile.
      expect(byKey('pick-club-7i'), findsOneWidget);
      expect(byKey('pick-player'), findsOneWidget);
      expect(byKey('pick-club-pw'), findsNothing);
      expect(
        textAt(tester, byKey('profile-unhit')),
        ProfileScreen.unhit(['pw']),
      );
      expect(textAt(tester, byKey('profile-preamble')), view.preamble);

      // The 7 iron, as Rust answered it.
      final club = view.clubs.single;
      expect(textAt(tester, byKey('profile-heading')), '7i');
      expect(textAt(tester, byKey('profile-shots')), club.shots);
      expect(club.nShots, 6);
      expect(byKey('blend-card'), findsNothing);
      expect(club.blend, isNull);
      final shown = <String>[];
      for (final row in club.numbers) {
        final key = 'number-${row.name}';
        await scrollTo(tester, byKey(key));
        final center = row.center;
        expect(
          center,
          isNotNull,
          reason: '${row.name}: a mean clears at 5, and there are 6',
        );
        expect(textAt(tester, inside(key, byKey('center'))), center);
        expect(textAt(tester, inside(key, byKey('spread'))), row.spread);
        expect(
          row.withheld,
          isTrue,
          reason: '${row.name}: a spread waits for 10',
        );
        shown.add('${row.name}: $center; ${row.spread}');
      }
      expect(club.topics, isNotEmpty);
      for (final topic in club.topics) {
        final key = 'topic-${topic.name}';
        await scrollTo(tester, byKey(key));
        expect(topic.grade.percent, isNull, reason: topic.name);
        expect(inside(key, byKey('grade-percent')), findsNothing);
        expect(
          textAt(tester, inside(key, byKey('grade-text'))),
          topic.grade.text,
        );
        shown.add('${topic.name}: ${topic.grade.text}');
      }
      await scrollTo(tester, byKey('nothing-listed'));
      expect(club.nothingListed, isNotNull);
      expect(textAt(tester, byKey('nothing-listed')), club.nothingListed);

      // The player: the same six, and no numbers.
      final player = view.player!;
      await scrollTo(tester, byKey('pick-player'), up: true);
      await tester.tap(byKey('pick-player'));
      await tester.pumpAndSettle();
      expect(textAt(tester, byKey('profile-heading')), ProfileScreen.player);
      expect(textAt(tester, byKey('profile-shots')), player.shots);
      expect(textAt(tester, byKey('profile-basis')), player.basis);
      expect(find.byType(TopicTile), findsWidgets);
      expect(byKey('number-carry_distance_yds'), findsNothing);
      expect(byKey('blend-card'), findsNothing);
      expect(player.blend, isNull);

      final report = StringBuffer()
        ..writeln('# M38 P26: the six captures on the Profile tab')
        ..writeln()
        ..writeln(
          'provisional: pasted copies. Recorded through M33\'s committed C0 '
          'boxes on the iOS Simulator, not read by a phone.',
        )
        ..writeln()
        ..writeln('- captures: ${captures.join(', ')}')
        ..writeln('- clubs: ${view.clubs.map((each) => each.club).join(', ')}')
        ..writeln('- 7i: ${club.shots}; blend ${club.blend?.mean ?? 'none'}')
        ..writeln('- player: ${player.shots}')
        ..writeln('- not in the profile: pw')
        ..writeln('- nothing listed: ${club.nothingListed}');
      for (final line in shown) {
        report.writeln('- $line');
      }
      final out = Directory('$repo/target/m38')..createSync(recursive: true);
      File('${out.path}/profile.txt').writeAsStringSync('$report');
      print(report);
    },
  );
}
