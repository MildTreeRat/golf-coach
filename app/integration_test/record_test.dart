// Each card is this test's output, read off the console and the file it writes, so it prints by design.
// ignore_for_file: avoid_print

import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/rust/third_party/app/api/history.dart';
import 'package:golf_app/src/rust/third_party/app/api/setup.dart' as setup;
import 'package:golf_app/src/screens/record_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/widgets/change_sheet.dart';
import 'package:golf_app/src/widgets/shot_card.dart';
import 'package:integration_test/integration_test.dart';

import 'setup_test.dart' show ScratchPhone, pumpUntil;

// The Record tab over the six captures, through the Simulator's iOS Vision and real Rust. [M38 P21]
//
// What the widget tests cannot say: that each card on screen is the shot as stored, that a read of
// nothing and a photo already held come back as Rust says, and that every read leaves its line in
// `.app/reads.jsonl` for the bay report. The library is the way in, since a native picker cannot be
// driven from here and the Simulator's camera waits on a prompt (P20 finding 2), so `TestPhone` hands
// back copies, as the picker does. Everything read off the captures is `provisional: pasted copies`
// (rule 12), and it is the Simulator's Vision, not a phone's (P18 finding 9).

/// `app/tool/sim_test.sh` passes this, and a Simulator app can read and write a host path.
const repo = String.fromEnvironment('GOLF_REPO');

/// One photo time for every copy, so all six land in one session, `2026-11-05-phone-a1b2` (C8).
const capturedAt = '2026-11-05T10:00:00-05:00';
const installId = 'a1b2';
const session = '2026-11-05-phone-$installId';

/// The real channel with no camera, and a library that hands over copies of what the test names.
///
/// **Copies, never the captures**: a `Failed` read moves the file it is given and `spikes/` is
/// read-only (P6 finding 11), and `Photo.discard` deletes what it is handed. Each copy is in a
/// directory of its own under the scratch's `tmp/`, as `PhotoSource.swift` lays them out.
class TestPhone extends ScratchPhone {
  TestPhone(super.scratch);

  /// The files each [pickPhotos] copies, in order.
  final List<List<String>> picks = [];
  var _copies = 0;

  @override
  Future<bool> cameraAvailable() async => false;

  @override
  Future<PhotoPick> pickPhotos() async => PhotoPick(
    photos: [for (final path in picks.removeAt(0)) _copy(path)],
    skipped: const [],
  );

  Photo _copy(String path) {
    final dir = Directory('$scratch/tmp/library/${_copies++}')
      ..createSync(recursive: true);
    final copy = File(path).copySync('${dir.path}/${path.split('/').last}');
    return Photo(
      path: copy.path,
      capturedAt: capturedAt,
      source: ReadSource.library_,
    );
  }
}

/// A grey PNG with nothing on it, which Vision reads no text from: `record_photo`'s `Failed`.
Future<String> blankPhoto(Directory scratch) async {
  final recorder = ui.PictureRecorder();
  Canvas(recorder).drawRect(
    const Rect.fromLTWH(0, 0, 640, 480),
    Paint()..color = const Color(0xFF808080),
  );
  final image = await recorder.endRecording().toImage(640, 480);
  final png = await image.toByteData(format: ui.ImageByteFormat.png);
  final file = File('${scratch.path}/blank.png')
    ..writeAsBytesSync(png!.buffer.asUint8List());
  return file.path;
}

/// What one card showed, row by row, read off the widgets while it was on screen.
typedef Shown = List<({String field, RowState state, List<String> texts})>;

Shown shownOn(WidgetTester tester, Finder card) => [
  for (final view in tester.widgetList<CardRowView>(
    find.descendant(of: card, matching: find.byType(CardRowView)),
  ))
    (
      field: view.row.field,
      state: view.row.state,
      texts: [
        for (final text in tester.widgetList<Text>(
          find.descendant(of: find.byWidget(view), matching: find.byType(Text)),
        ))
          text.data!,
      ],
    ),
];

/// Pump frames while [waiting] holds, for up to 20 s, as `pumpUntil` does for a finder.
Future<void> pumpWhile(WidgetTester tester, bool Function() waiting) async {
  final deadline = DateTime.now().add(const Duration(seconds: 20));
  while (waiting()) {
    if (DateTime.now().isAfter(deadline)) {
      fail('still waiting after 20 s');
    }
    await tester.pump(const Duration(milliseconds: 50));
  }
}

/// The library button is enabled, so the flow has finished with the last tap, its log line included.
bool libraryReady(WidgetTester tester) =>
    tester
        .widget<ButtonStyleButton>(find.byKey(const ValueKey('library')))
        .onPressed !=
    null;

Finder cardOf(String swing) => find.byKey(ValueKey('card-$session-$swing'));

/// Every stored shot under [root], by its id.
Map<String, Map<String, Object?>> storedShots(String root) => {
  for (final file in Directory('$root/shots').listSync().whereType<File>())
    if (file.path.endsWith('.shot.json'))
      for (final shot in [
        jsonDecode(file.readAsStringSync()) as Map<String, Object?>,
      ])
        shot['shot_id']! as String: shot,
};

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets(
    'the six captures through the Record tab: each card is the shot as stored, and every read is logged',
    (tester) async {
      expect(repo, isNotEmpty, reason: 'run this through app/tool/sim_test.sh');
      final scratch = await Directory.systemTemp.createTemp('m38-record-');
      addTearDown(() => scratch.delete(recursive: true));
      final root = '${scratch.path}/processed';

      // A data root as setup leaves it: a golfer and a bag, and no club yet (P19 finding 10).
      await setup.initState(root: root, installId: installId);
      final golfer = await setup.createGolfer(
        root: root,
        name: 'Aaron',
        handedness: Handedness.right,
        now: now(),
      );
      await setup.setBagSlot(
        root: root,
        playerId: golfer.playerId,
        club: '7i',
        loftDeg: 34,
        now: now(),
      );
      final state = await setup.getState(root: root);
      await setup.setState(
        root: root,
        state: state.copyWith(playerId: () => golfer.playerId),
      );

      final manifest = jsonDecode(
        File('$repo/spikes/vision-ocr/photos.json').readAsStringSync(),
      ) as Map<String, Object?>;
      final captures = [
        for (final capture in (manifest['captures']! as List))
          '$repo/${(capture as Map<String, Object?>)['path']! as String}',
      ];
      expect(captures, hasLength(6));

      final phone = TestPhone(scratch.path);
      final model = AppModel(core: const RustCore(), phone: phone);
      await tester.pumpWidget(GolfApp(model: model));
      await pumpUntil(tester, find.byType(RecordScreen));
      await pumpUntil(tester, find.text('Aaron · No club · Fundamentals'));
      expect(find.byKey(const ValueKey('shutter')), findsNothing);

      // The club, picked through the sheet and stored by real Rust, which `record_photo` then reads.
      await tester.tap(find.byKey(const ValueKey('change')));
      await pumpUntil(tester, find.byType(ChangeSheet));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('pick-club-7i')));
      await pumpUntil(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('pick-club-7i')),
          matching: find.byIcon(Icons.check),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Done'));
      await pumpUntil(tester, find.text('Aaron · 7i · Fundamentals'));
      await tester.pumpAndSettle();

      // The six as one library batch, each card read off the screen while it is up.
      phone.picks.add(captures);
      await tester.tap(find.byKey(const ValueKey('library')));
      final report = StringBuffer()
        ..writeln('# M38 P21: the six captures through the Record tab')
        ..writeln()
        ..writeln(
          'provisional: pasted copies. Read by the Simulator\'s iOS Vision, '
          'not a phone\'s.',
        );
      final notRead = <String>[];
      for (var at = 0; at < captures.length; at++) {
        final swing = '${at + 1}';
        await pumpUntil(tester, cardOf(swing));
        final shown = shownOn(tester, cardOf(swing));
        final batch = find.byKey(const ValueKey('batch'));
        expect(batch, findsOneWidget, reason: 'a batch shows its progress');

        // The card on screen against the shot as stored: Rust's card of it from disk, and the file.
        final detail = await shotDetail(
          root: root,
          sessionId: session,
          swingId: swing,
        );
        final stored = storedShots(root)['$session-$swing']!;
        final rows = detail.card!.rows;
        expect(
          shown.map((row) => row.field),
          rows.map((row) => row.field),
          reason: '$swing: the card shows every row, in the screen\'s order',
        );
        final name = captures[at].split('/').last;
        report
          ..writeln()
          ..writeln('## $name, shot $swing');
        for (var r = 0; r < rows.length; r++) {
          final row = rows[r];
          expect(stored[row.field], row.value, reason: '$name ${row.field}');
          expect(shown[r].state, row.state, reason: '$name ${row.field}');
          expect(shown[r].texts, contains(row.label));
          expect(
            shown[r].texts,
            contains(row.display ?? CardRowView.notRead),
            reason: '$name ${row.field}: what is on screen is what is stored',
          );
          if (row.state == RowState.notRead) {
            expect(stored[row.field], isNull);
            notRead.add('$name ${row.field}');
          }
          report.writeln(
            '- ${row.label}: ${row.display ?? CardRowView.notRead}',
          );
        }
      }
      await pumpWhile(tester, () => !libraryReady(tester));
      expect(find.byKey(const ValueKey('batch')), findsNothing);

      // A read of nothing: Rust's sentence, no card, and the photo kept only as a diagnostic (C13).
      phone.picks.add([await blankPhoto(scratch)]);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('library')));
      await pumpUntil(
        tester,
        find.descendant(
          of: find.byKey(const ValueKey('failed')),
          matching: find.text(
            'This photo could not be read. Retake it, filling the frame with the tiles.',
          ),
        ),
      );
      expect(find.byType(ShotCardView), findsNothing);
      await pumpWhile(tester, () => !libraryReady(tester));
      final failed = Directory('$root/.app/failed').listSync();
      expect(failed, hasLength(1));

      // The first capture again: the held shot, under Rust's sentence.
      phone.picks.add([captures.first]);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('library')));
      await pumpUntil(
        tester,
        find.descendant(
          of: cardOf('1'),
          matching: find.text(
            'This photo is already recorded, so this is the shot it holds.',
          ),
        ),
      );
      await pumpWhile(tester, () => !libraryReady(tester));

      // Every read has its line, in order, a failure's included (P14 finding 10).
      final lines = [
        for (final line in File(
          '$root/.app/reads.jsonl',
        ).readAsLinesSync().where((line) => line.isNotEmpty))
          jsonDecode(line) as Map<String, Object?>,
      ];
      expect(lines.map((line) => line['status']), [
        for (var at = 0; at < captures.length; at++) 'read',
        'failed',
        'duplicate',
      ]);
      expect(lines.map((line) => line['source']).toSet(), {'library'});
      report
        ..writeln()
        ..writeln('## reads.jsonl')
        ..writeln()
        ..writeln('status, boxes, ms_vision, ms_rust, ms_total');
      for (final line in lines) {
        final total = line['ms_total']! as int;
        expect(total, greaterThanOrEqualTo(line['ms_vision']! as int));
        expect(total, greaterThanOrEqualTo(line['ms_rust']! as int));
        report.writeln(
          '${line['status']}, ${line['boxes']}, ${line['ms_vision']}, '
          '${line['ms_rust']}, $total',
        );
      }
      report
        ..writeln()
        ..writeln(
          'Rows not read: ${notRead.isEmpty ? 'none' : notRead.join(', ')}',
        );

      // Every copy handed over is gone, moved by the failed read or deleted once Rust answered.
      await pumpWhile(
        tester,
        () =>
            Directory('${scratch.path}/tmp')
                .listSync(recursive: true)
                .whereType<File>()
                .isNotEmpty,
      );
      // The captures themselves are untouched (rule 5).
      for (final capture in captures) {
        expect(File(capture).existsSync(), isTrue, reason: capture);
      }

      final out = Directory('$repo/target/m38')..createSync(recursive: true);
      File('${out.path}/record-cards.txt').writeAsStringSync('$report');
      print(report);
    },
  );
}
