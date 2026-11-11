// What the flow stored and exported is this test's output, read off the console and the file it writes,
// so it prints by design.
// ignore_for_file: avoid_print

import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/screens/bag_screen.dart';
import 'package:golf_app/src/screens/profile_screen.dart';
import 'package:golf_app/src/screens/record_screen.dart';
import 'package:golf_app/src/screens/review_screen.dart';
import 'package:golf_app/src/screens/settings_screen.dart';
import 'package:golf_app/src/screens/setup_screen.dart';
import 'package:golf_app/src/screens/shot_detail_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/ui/shell.dart';
import 'package:golf_app/src/widgets/change_sheet.dart';
import 'package:integration_test/integration_test.dart';

import 'export_test.dart' show zipEntries, sameBytes;
import 'record_test.dart'
    show TestPhone, capturedAt, libraryReady, pumpWhile, storedShots;
import 'setup_test.dart' show clubRow, pumpUntil, setLoft;
import 'shot_actions_test.dart' show tapShown;
import 'test_host.dart';

// Everything but a camera and a bay, end to end, on the Simulator. [M38 P28]
//
// Each earlier integration test starts from a root it built through the facade and drives one tab. This
// one starts from nothing and goes the way a golfer does, through the widgets alone: first-run setup, the
// club, the six captures as one library batch read by the Simulator's iOS Vision, a review of anything
// flagged, one value corrected from History's detail screen, History, Profile, and Settings' Export. So it
// is the first run to reach the detail screen and the Profile tab from Vision reads (P25 finding 7, P26
// finding 7), and the first to export a `.app/reads.jsonl` the bay report can count (P27 finding 2).
//
// The zip the share is handed is copied to `target/m38/whole-flow.zip`, and `app/tool/whole_flow.sh`
// then holds it on the host: `golf-bay-report truth` to M33's truth, `golf-lab import-export` into empty
// scratch directories, and `golf-bay-report compare`. Run it through that script, which runs this.
// Everything read off the captures is `provisional: pasted copies` (rule 12), and it is the Simulator's
// Vision, not a phone's (P18 finding 9).

// `repo` is `test_host.dart`'s, so this runs on a phone too, through `app/tool/device_test.sh` (P30).

/// The golfer setup types, whose `player_id` the script passes to `compare`.
const name = 'Aaron';
const playerId = 'aaron';

/// What is typed over the first shot's carry: a number no capture's carry is, so the edit cannot be a
/// no-op, and the one `truth` must not see (P15 finding 2: it holds what the reader gave).
const typed = '123.4';

/// Where the script finds the zip the share was handed.
const zipOut = 'target/m38/whole-flow.zip';

/// [TestPhone] with Documents one level down, so the scratch stands where the app's container does and
/// the `tmp/export/` beside Documents is the scratch's own (P27 finding 5), and a share that copies the
/// zip out and says it went. The library still hands over copies, under the scratch's `tmp/library/`.
class WholeFlowPhone extends TestPhone {
  WholeFlowPhone(super.scratch);

  /// Every path [share] was handed, in order.
  final List<String> shared = [];

  @override
  Future<String> documentsDir() async => '$scratch/Documents';

  @override
  Future<bool> share(String path) async {
    shared.add(path);
    File(path).copySync('$repo/$zipOut');
    return true;
  }
}

Finder byKey(String key) => find.byKey(ValueKey(key));

String textOf(WidgetTester tester, Finder finder) => tester
    .widget<Text>(
      find.descendant(of: finder, matching: find.byType(Text), matchRoot: true),
    )
    .data!;

/// Scroll the list under [listKey] until [target] is built and on screen, down or, with [up], back up.
/// Every list in the app builds lazily (P24 finding 7, P26 finding 7, P27 finding 5).
Future<void> scrollIn(
  WidgetTester tester,
  String listKey,
  Finder target, {
  bool up = false,
}) async {
  await tester.pumpAndSettle();
  await tester.scrollUntilVisible(
    target,
    up ? -100 : 100,
    scrollable: find.descendant(
      of: byKey(listKey),
      matching: find.byType(Scrollable),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets(
    'from a fresh root: setup, a club, the six captures, a correction, History, Profile and an export',
    (tester) async {
      expect(
        repo,
        isNotEmpty,
        reason: 'run this through app/tool/whole_flow.sh or device_test.sh',
      );
      final scratch = await Directory.systemTemp.createTemp('m38-whole-flow-');
      addTearDown(() => scratch.delete(recursive: true));
      final root = '${scratch.path}/Documents/processed';
      const core = RustCore();

      final manifest = jsonDecode(
        File('$repo/spikes/vision-ocr/photos.json').readAsStringSync(),
      ) as Map<String, Object?>;
      final captures = [
        for (final capture in (manifest['captures']! as List))
          '$repo/${(capture as Map<String, Object?>)['path']! as String}',
      ];
      expect(captures, hasLength(6));

      final phone = WholeFlowPhone(scratch.path);
      final model = AppModel(core: core, phone: phone);
      await tester.pumpWidget(GolfApp(model: model));

      // Setup, from nothing: the install id is minted, then a golfer and a bag through the widgets.
      await pumpUntil(tester, find.byType(SetupScreen));
      expect(Directory('$root/sessions').existsSync(), isFalse);
      final installId = model.state.installId!;
      expect(installId, matches(RegExp(r'^[0-9a-f]{4}$')));
      // Every capture is handed over with one photo time, so all six land in one session (C8).
      expect(capturedAt, startsWith('2026-11-05T'));
      final session = '2026-11-05-phone-$installId';
      Finder rowFor(String swing) => byKey('shot-$session-$swing');
      Finder cardOf(String swing) => byKey('card-$session-$swing');

      await tester.enterText(find.byType(TextField), name);
      await tester.tap(find.text('Continue'));
      await pumpUntil(tester, find.byType(BagScreen));
      expect(model.golfer!.playerId, playerId);
      await tester.scrollUntilVisible(clubRow('7i'), 200);
      await tester.tap(clubRow('7i'));
      await pumpUntil(
        tester,
        find.descendant(of: clubRow('7i'), matching: find.text('Add loft')),
      );
      // A loft, so no shot's flight is left unscored for the want of one (P25 finding 2).
      await setLoft(tester, '7i', '34');
      await pumpUntil(
        tester,
        find.descendant(of: clubRow('7i'), matching: find.text('34°')),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Done'));
      await pumpUntil(tester, find.byType(Shell));

      // The club, through the change sheet. Setup sets none (P19 finding 10).
      await pumpUntil(tester, find.byType(RecordScreen));
      await pumpUntil(tester, find.text('$name · No club · Fundamentals'));
      expect(byKey('shutter'), findsNothing);
      await tester.tap(byKey('change'));
      await pumpUntil(tester, find.byType(ChangeSheet));
      await tester.pumpAndSettle();
      await tester.tap(byKey('pick-club-7i'));
      await pumpUntil(
        tester,
        find.descendant(
          of: byKey('pick-club-7i'),
          matching: find.byIcon(Icons.check),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Done'));
      await pumpUntil(tester, find.text('$name · 7i · Fundamentals'));
      await tester.pumpAndSettle();

      // The six as one library batch, with no challenge on (P23 finding 6). Anything flagged is a forced
      // review once the batch's last photo is in (C14), and it is saved as it stands: what a golfer does
      // when the card agrees with the screen. None of the six read flagged on the Simulator (P21 finding
      // 2), so this branch is expected to do nothing, and the count is held to the log either way.
      phone.picks.add(captures);
      await tester.tap(byKey('library'));
      await pumpUntil(tester, byKey('batch'));
      final reviewed = <String>{};
      final deadline = DateTime.now().add(const Duration(seconds: 120));
      // A review covers the Record tab, whose library button is then offstage, so it is asked first.
      while (find.byType(ReviewScreen).evaluate().isNotEmpty ||
          !libraryReady(tester)) {
        if (DateTime.now().isAfter(deadline)) {
          fail('the batch had not finished after 120 s');
        }
        final review = find.byType(ReviewScreen);
        if (review.evaluate().isNotEmpty) {
          final screen = tester.widget<ReviewScreen>(review);
          expect(screen.forced, isTrue, reason: 'only a flag opens one here');
          if (reviewed.add(screen.card.swingId)) {
            // Past the route's slide, and not with `pumpAndSettle`, which the busy bar beneath keeps
            // from returning (P24 finding 7).
            await tester.pump(const Duration(milliseconds: 600));
            await tester.tap(byKey('review-save'));
          }
        }
        await tester.pump(const Duration(milliseconds: 50));
      }
      expect(byKey('batch'), findsNothing);
      expect(cardOf('${captures.length}'), findsOneWidget);

      final lines = [
        for (final line in File(
          '$root/.app/reads.jsonl',
        ).readAsLinesSync().where((line) => line.isNotEmpty))
          jsonDecode(line) as Map<String, Object?>,
      ];
      expect(lines, hasLength(captures.length), reason: 'one line a photo');
      expect(lines.map((line) => line['source']).toSet(), {'library'});
      final flagged = {
        for (final (at, line) in lines.indexed)
          if (line['status'] == 'flagged') '${at + 1}',
      };
      expect(
        lines.map((line) => line['status']).toSet().difference({
          'read',
          'flagged',
        }),
        isEmpty,
        reason: 'six captures, six shots: no failure and no duplicate',
      );
      expect(reviewed, flagged, reason: 'every flagged shot, and only those');
      final asRead = storedShots(root);
      expect(asRead, hasLength(captures.length));
      for (final MapEntry(key: id, value: shot) in asRead.entries) {
        final provenance = shot['provenance']! as Map<String, Object?>;
        expect(provenance['needs_review'], isFalse, reason: '$id reviewed');
      }
      final readCarry = asRead['$session-1']!['carry_distance'] as double?;
      expect(readCarry, isNotNull, reason: 'IMG_2889 reads its carry');

      // History, then one value corrected from shot 1's detail screen: run sheet step 8's path.
      await tapShown(tester, find.byIcon(Icons.history_outlined));
      await pumpUntil(tester, byKey('session-$session'));
      final listed = await core.listSessions(root: root, playerId: playerId);
      expect(listed.map((each) => each.sessionId), [session]);
      expect(listed.single.rows, hasLength(captures.length));
      await scrollIn(tester, 'history-list', rowFor('1'));
      await tapShown(tester, rowFor('1'));
      await pumpUntil(tester, find.byType(ShotDetailScreen));
      await pumpUntil(tester, cardOf('1'));
      expect(byKey('detail-heading'), findsOneWidget);
      expect(byKey('detail-problem'), findsNothing);

      await tapShown(tester, byKey('detail-edit'));
      await pumpUntil(tester, find.byType(ReviewScreen));
      await pumpUntil(tester, byKey('review-photo-shown'));
      expect(
        tester.widget<ReviewScreen>(find.byType(ReviewScreen)).forced,
        isFalse,
      );
      final carryField = byKey('field-carry_distance');
      final shownCarry = tester.widget<TextField>(carryField).controller!.text;
      expect(shownCarry, isNotEmpty);
      expect(shownCarry, isNot(typed));
      await tester.ensureVisible(carryField);
      await tester.pumpAndSettle();
      await tester.enterText(carryField, typed);
      await tester.pumpAndSettle();
      await tester.tap(byKey('review-save'));
      await pumpWhile(
        tester,
        () => find.byType(ReviewScreen).evaluate().isNotEmpty,
      );
      // The detail reads again once the review ends, and its card is then Rust's answer.
      final carryRow = find.descendant(
        of: cardOf('1'),
        matching: byKey('row-carry_distance'),
      );
      await pumpUntil(
        tester,
        find.descendant(of: carryRow, matching: find.text(typed)),
      );
      expect(
        find.descendant(of: carryRow, matching: find.text('corrected')),
        findsOneWidget,
      );
      await tapShown(tester, find.byType(BackButton));
      await pumpWhile(
        tester,
        () =>
            find.byType(ShotDetailScreen).evaluate().isNotEmpty ||
            find
                .descendant(of: rowFor('1'), matching: byKey('mark-corrected'))
                .evaluate()
                .isEmpty,
      );

      final corrected = storedShots(root)['$session-1']!;
      final provenance = corrected['provenance']! as Map<String, Object?>;
      final corrections = provenance['corrections']! as Map<String, Object?>;
      expect(corrected['carry_distance'], double.parse(typed));
      expect(corrections.keys, ['carry_distance']);
      expect(double.parse(corrections['carry_distance']! as String), readCarry);

      // History agrees with `list_sessions` after the edit: shot 1 alone is marked corrected, and its
      // carry on screen is the typed one.
      final after = await core.listSessions(root: root, playerId: playerId);
      final rows = {for (final row in after.single.rows) row.swingId: row};
      expect(
        {
          for (final row in rows.values)
            if (row.corrected) row.swingId,
        },
        {'1'},
      );
      expect(
        find.descendant(
          of: find.descendant(of: rowFor('1'), matching: byKey('carry')),
          matching: find.text(rows['1']!.carryDisplay!),
        ),
        findsOneWidget,
      );
      expect(rows['1']!.carryDisplay, contains('123'));

      // Profile: the 7i and the player, as real Rust grades the six Vision reads.
      final profile = await core.gradedProfile(root: root, playerId: playerId);
      expect(profile.clubs.map((club) => club.club), ['7i']);
      final club = profile.clubs.single;
      expect(club.nShots, captures.length);
      await tapShown(tester, find.byIcon(Icons.insights_outlined));
      await pumpUntil(tester, byKey('profile-heading'));
      await tester.pumpAndSettle();
      expect(textOf(tester, byKey('profile-heading')), '7i');
      expect(textOf(tester, byKey('profile-shots')), club.shots);
      expect(byKey('pick-player'), findsOneWidget);
      final carryNumber = club.numbers.singleWhere(
        (row) => row.name == 'carry_distance_yds',
      );
      await scrollIn(
        tester,
        'profile-list',
        byKey('number-carry_distance_yds'),
      );
      expect(
        textOf(
          tester,
          find.descendant(
            of: byKey('number-carry_distance_yds'),
            matching: byKey('center'),
          ),
        ),
        carryNumber.center,
        reason: 'a mean clears at 5, and there are 6, the corrected carry among them',
      );
      await scrollIn(tester, 'profile-list', byKey('pick-player'), up: true);
      await tester.tap(byKey('pick-player'));
      await tester.pumpAndSettle();
      expect(textOf(tester, byKey('profile-heading')), ProfileScreen.player);
      expect(textOf(tester, byKey('profile-shots')), profile.player!.shots);

      // Settings' Export, through the phone's share, which copies the zip out for the script.
      final zipFile = File('$repo/$zipOut');
      if (zipFile.existsSync()) {
        zipFile.deleteSync();
      }
      await tapShown(tester, find.byIcon(Icons.settings_outlined));
      await pumpUntil(tester, byKey('export-shots'));
      await scrollIn(tester, 'settings-list', byKey('export-button'));
      expect(
        textOf(tester, byKey('export-last')),
        SettingsScreen.neverExported,
      );
      expect(
        textOf(tester, byKey('export-shots')),
        '${captures.length} shots on this phone.',
      );
      await tester.tap(byKey('export-button'));
      await pumpUntil(tester, find.textContaining('Last export '));
      await tester.pumpAndSettle();
      expect(phone.shared, hasLength(1));
      expect(byKey('export-outcome'), findsNothing);
      expect(
        textOf(tester, byKey('export-shots')),
        '${captures.length} shots on this phone, 0 new since then.',
      );

      // The zip is the one the share was handed, cut after the correction, and it carries what the
      // host's three checks read: the shots, the golfer, the reads and the boxes.
      final bytes = zipFile.readAsBytesSync();
      expect(
        sameBytes(bytes, File(phone.shared.single).readAsBytesSync()),
        isTrue,
      );
      final entries = zipEntries(bytes);
      final names = [for (final entry in entries) entry.name];
      expect(
        names,
        containsAll([
          'golfers/$playerId.golfer.json',
          '.app/reads.jsonl',
          '.app/events.jsonl',
        ]),
      );
      expect(names.where((each) => each.startsWith('.app/state')), isEmpty);
      expect(
        names.where((each) => each.startsWith('.app/boxes/')),
        hasLength(captures.length),
      );
      final zipped = {
        for (final entry in entries)
          if (entry.name.startsWith('shots/') &&
              entry.name.endsWith('.shot.json'))
            for (final shot in [
              jsonDecode(
                utf8.decode(
                  entry.method == 8
                      ? ZLibDecoder(raw: true).convert(entry.data)
                      : entry.data,
                ),
              ) as Map<String, Object?>,
            ])
              shot['shot_id']! as String: shot,
      };
      expect(zipped.keys.toSet(), asRead.keys.toSet());
      expect(zipped['$session-1']!['carry_distance'], double.parse(typed));
      expect(
        (zipped['$session-1']!['provenance']!
            as Map<String, Object?>)['corrections'],
        corrections,
      );

      // What the person did, in the order done (D16) [M38 P30b]. The share's own line is written once
      // the sheet closes, after the zip was cut, so it is on the phone and not yet in the export.
      await model.eventsWritten;
      final events = [
        for (final line in File(
          '$root/.app/events.jsonl',
        ).readAsLinesSync().where((line) => line.isNotEmpty))
          jsonDecode(line) as Map<String, Object?>,
      ];
      expect(
        events.map((event) => event['kind']),
        containsAllInOrder([
          'app_start',
          'club_changed',
          'library_picked',
          'review_opened',
          'review_saved',
          'export_shared',
        ]),
      );
      expect(
        [
          for (final event in events)
            if (event['kind'] == 'error') event['detail'],
        ],
        isEmpty,
        reason: 'nothing went wrong in the flow, so nothing says it did',
      );
      expect(events.last['kind'], 'export_shared');
      final zippedEvents = entries.singleWhere(
        (entry) => entry.name == '.app/events.jsonl',
      );
      final eventsInZip = utf8
          .decode(
            zippedEvents.method == 8
                ? ZLibDecoder(raw: true).convert(zippedEvents.data)
                : zippedEvents.data,
          )
          .split('\n')
          .where((line) => line.isNotEmpty);
      expect(eventsInZip, hasLength(events.length - 1));

      final report = StringBuffer()
        ..writeln('# M38 P28: the whole flow on the $hostName')
        ..writeln()
        ..writeln(
          'provisional: pasted copies. Read by the $hostName\'s iOS Vision${onDevice ? '' : ', not a phone\'s'}. '
          'The share sheet was answered by the test, which '
          'copied the zip to $zipOut.',
        )
        ..writeln()
        ..writeln('- install: $installId, session $session')
        ..writeln(
          '- captures: ${captures.map((each) => each.split('/').last).join(', ')}',
        )
        ..writeln(
          '- reads: ${lines.map((line) => line['status']).join(', ')}; '
          'forced reviews ${reviewed.isEmpty ? 'none' : reviewed.join(', ')}',
        )
        ..writeln(
          '- shot 1 carry: read $readCarry, shown $shownCarry, typed $typed, '
          'corrections ${jsonEncode(corrections)}',
        )
        ..writeln(
          '- profile: 7i ${club.shots}; carry mean ${carryNumber.center}',
        )
        ..writeln(
          '- zip: ${phone.shared.single.split('/').last}, ${bytes.length} bytes, ${entries.length} entries',
        )
        ..writeln(
          '- events: ${[for (final event in events) '${event['kind']} (${event['detail']})'].join('; ')}',
        )
        ..writeln()
        ..writeln('status, boxes, ms_vision, ms_rust, ms_total');
      for (final line in lines) {
        report.writeln(
          '${line['status']}, ${line['boxes']}, ${line['ms_vision']}, '
          '${line['ms_rust']}, ${line['ms_total']}',
        );
      }
      File('$repo/target/m38/whole-flow.txt').writeAsStringSync('$report');
      printReport(report);
    },
  );
}
