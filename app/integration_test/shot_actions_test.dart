// What the actions stored is this test's output, read off the console and the file it writes, so it
// prints by design.
// ignore_for_file: avoid_print

import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/rust/third_party/app/api/profile.dart' as profile;
import 'package:golf_app/src/screens/history_screen.dart';
import 'package:golf_app/src/screens/shot_detail_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:integration_test/integration_test.dart';

import 'record_test.dart'
    show TestPhone, installId, pumpWhile, session, storedShots;
import 'setup_test.dart' show pumpUntil;

// A shot's three repairs through the detail screen, the real bridge and real Rust. [M38 P25]
//
// The widget tests say what each action calls. This says what it stores, and that the two readers of
// the stores then agree: three captures recorded, then from History one shot's club changed, one's
// mishit confirmed and one deleted, each through its detail screen. Afterwards the History tab on screen,
// `list_sessions`' answer, the manifests on disk and the corpus as `graded_profile` counts it all say
// the same. The captures are recorded through the facade with M33's committed boxes, which P17 held to
// the app's own reader, so the Record tab's Vision is not what this tests, and everything read off them
// is `provisional: pasted copies` (rule 12).

const repo = String.fromEnvironment('GOLF_REPO');

/// Each capture and when it is said to have been taken, a few minutes apart in one session, so
/// History lists them newest first as 3, 2, 1.
const captures = [
  ('IMG_2889', '10:00'),
  ('IMG_2891', '10:05'),
  ('IMG_2892', '10:10'),
];

Finder rowFor(String swing) => find.byKey(ValueKey('shot-$session-$swing'));

Finder inRow(String swing, Finder matching) =>
    find.descendant(of: rowFor(swing), matching: matching);

/// Settle, bring [finder] on screen, settle, and tap it. The clock is real here, so a tap during an
/// animation can land on the barrier (P19 finding 5).
Future<void> tapShown(WidgetTester tester, Finder finder) async {
  await tester.pumpAndSettle();
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
}

/// Open [swing]'s detail from its History row, and wait for its card.
Future<void> openShot(WidgetTester tester, String swing) async {
  await tapShown(tester, rowFor(swing));
  await pumpUntil(tester, find.byKey(ValueKey('card-$session-$swing')));
}

/// Back from the detail to History, and wait for History to have read again.
Future<void> backToHistory(WidgetTester tester, bool Function() read) async {
  await tapShown(tester, find.byType(BackButton));
  await pumpWhile(
    tester,
    () => find.byType(ShotDetailScreen).evaluate().isNotEmpty || !read(),
  );
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets('a club changed, a mishit confirmed and a shot deleted from their detail screens leave History, the '
      'manifests and the corpus agreeing', (tester) async {
    expect(repo, isNotEmpty, reason: 'run this through app/tool/sim_test.sh');
    final scratch = await Directory.systemTemp.createTemp('m38-actions-');
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
      state: state.copyWith(playerId: () => golfer.playerId, club: () => '7i'),
    );
    for (final (id, time) in captures) {
      final copy = File('$repo/spikes/vision-ocr/captures/$id.jpg')
          .copySync('${scratch.path}/$id.jpg');
      final recorded = await core.recordPhoto(
        photo: PhotoIn(
          root: root,
          path: copy.path,
          boxesJson: File('$repo/spikes/vision-ocr/boxes/C0/raw/$id.json')
              .readAsStringSync(),
          capturedAt: '2026-11-05T$time:00-05:00',
          now: now(),
        ),
      );
      expect(recorded.status, RecordStatus.read, reason: id);
    }
    final before = await profile.gradedProfile(root: root, playerId: 'aaron');
    expect(
      {for (final club in before.clubs) club.club: club.nShots},
      {'7i': 3},
      reason: 'all three counted under the 7i before any action',
    );

    final phone = TestPhone(scratch.path);
    final model = AppModel(core: core, phone: phone);
    await tester.pumpWidget(GolfApp(model: model));
    await pumpUntil(tester, find.text('Aaron · 7i · Fundamentals'));
    await tapShown(tester, find.byIcon(Icons.history_outlined));
    await pumpUntil(tester, rowFor('1'));
    expect(find.byType(HistoryRow), findsNWidgets(3));

    // Shot 1: hit with the pitching wedge, not the 7i.
    await openShot(tester, '1');
    await tapShown(tester, find.byKey(const ValueKey('detail-club-pw')));
    await pumpUntil(
      tester,
      find.descendant(
        of: find.byKey(const ValueKey('detail-club-pw')),
        matching: find.byIcon(Icons.check),
      ),
    );
    expect(find.byKey(const ValueKey('detail-problem')), findsNothing);
    await backToHistory(
      tester,
      () => inRow('1', find.text('Shot 1 · 10:00 · pw')).evaluate().isNotEmpty,
    );

    // Shot 2: a mishit, whatever the automatic rule says.
    await openShot(tester, '2');
    await tapShown(tester, find.byKey(const ValueKey('mishit-confirmed')));
    await pumpUntil(
      tester,
      find.text('Your call: a mishit, held out of the distance averages.'),
    );
    expect(find.byKey(const ValueKey('detail-problem')), findsNothing);
    await backToHistory(
      tester,
      () => inRow(
        '2',
        find.byKey(const ValueKey('mark-mishit')),
      ).evaluate().isNotEmpty,
    );

    // Shot 3: a stray, deleted. Delete goes back to History by itself.
    await openShot(tester, '3');
    await tapShown(tester, find.byKey(const ValueKey('detail-delete')));
    await tapShown(tester, find.byKey(const ValueKey('delete-confirm')));
    await pumpWhile(
      tester,
      () =>
          find.byType(ShotDetailScreen).evaluate().isNotEmpty ||
          rowFor('3').evaluate().isNotEmpty,
    );
    await tester.pumpAndSettle();

    // History, on screen.
    expect(find.byType(HistoryRow), findsNWidgets(2));
    expect(inRow('1', find.text('Shot 1 · 10:00 · pw')), findsOneWidget);
    expect(inRow('2', find.text('Shot 2 · 10:05 · 7i')), findsOneWidget);
    expect(inRow('1', find.byKey(const ValueKey('mark-mishit'))), findsNothing);
    expect(
      inRow('2', find.byKey(const ValueKey('mark-mishit'))),
      findsOneWidget,
    );

    // `list_sessions`, whose mishit is the corpus's own answer (`CorpusSwing::is_mishit`).
    final sessions = await core.listSessions(root: root, playerId: 'aaron');
    final rows = sessions.single.rows;
    expect(rows.map((row) => row.swingId), ['2', '1']);
    final byId = {for (final row in rows) row.swingId: row};
    expect(byId['1']!.club, 'pw');
    expect(byId['1']!.mishit, isFalse);
    expect(byId['1']!.mishitVerdict, isNull);
    expect(byId['2']!.club, '7i');
    expect(byId['2']!.mishit, isTrue);
    expect(byId['2']!.mishitVerdict, MishitVerdict.confirmed);

    // The manifests, and what the delete took.
    Map<String, Object?> manifest(String swing) => jsonDecode(
      File('$root/sessions/$session/$swing/manifest.json').readAsStringSync(),
    ) as Map<String, Object?>;
    expect(manifest('1')['club'], 'pw');
    expect(manifest('2')['mishit'], 'confirmed');
    expect(Directory('$root/sessions/$session/3').existsSync(), isFalse);
    final shots = storedShots(root);
    expect(shots.keys.toSet(), {'$session-1', '$session-2'});

    // The corpus, as the graded profile counts it: the wedge holds shot 1, the 7i holds shot 2 alone,
    // since a mishit is still a tracked shot, and the deleted one is in nobody's count.
    final after = await profile.gradedProfile(root: root, playerId: 'aaron');
    final counts = {for (final club in after.clubs) club.club: club.nShots};
    expect(counts, {'7i': 1, 'pw': 1});

    final report = StringBuffer()
      ..writeln('# M38 P25: three shot actions through the detail screen')
      ..writeln()
      ..writeln(
        'provisional: pasted copies. Recorded through M33\'s committed C0 '
        'boxes on the iOS Simulator, not read by a phone.',
      )
      ..writeln()
      ..writeln('- captures: ${captures.map((each) => each.$1).join(', ')}')
      ..writeln('- shot 1: club 7i -> ${byId['1']!.club}')
      ..writeln(
        '- shot 2: mishit ${byId['2']!.mishit}, '
        'verdict ${byId['2']!.mishitVerdict?.name}',
      )
      ..writeln('- shot 3: deleted, entry gone, ${shots.length} shots stored')
      ..writeln('- History rows: ${rows.map((row) => row.swingId).join(', ')}')
      ..writeln(
        '- graded profile shots by club: before '
        '${before.clubs.map((club) => '${club.club} ${club.nShots}').join(', ')}, '
        'after ${after.clubs.map((club) => '${club.club} ${club.nShots}').join(', ')}',
      );
    final out = Directory('$repo/target/m38')..createSync(recursive: true);
    File('${out.path}/shot-actions.txt').writeAsStringSync('$report');
    print(report);
  });
}
