// What the review stored is this test's output, read off the console and the file it writes, so it
// prints by design.
// ignore_for_file: avoid_print

import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/rust/third_party/app/api/history.dart' as history;
import 'package:golf_app/src/rust/third_party/app/api/setup.dart' as setup;
import 'package:golf_app/src/screens/record_screen.dart';
import 'package:golf_app/src/screens/review_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:integration_test/integration_test.dart';

import 'record_test.dart'
    show
        TestPhone,
        cardOf,
        installId,
        libraryReady,
        pumpWhile,
        session,
        storedShots;
import 'setup_test.dart' show pumpUntil;

// A person's edit through the review, the real bridge and real Rust, down to the file. [M38 P22]
//
// The widget tests say what Save sends. This says what it stores: one capture recorded through the
// Record tab, its card opened, its carry typed over and its intent changed, and then the shot on disk
// holds the typed carry, `corrections` holds the carry as read (C11), the flag is off, and the
// manifest holds the intent. The photo shown is the stored copy, decoded by the Simulator. Everything
// read off the capture is `provisional: pasted copies` (rule 12), and the Simulator's Vision.

const repo = String.fromEnvironment('GOLF_REPO');

/// What is typed over the carry: a number no capture's carry is, so the edit cannot be a no-op.
const typed = '123.4';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets(
    'a carry typed in the review is stored, with the carry as read in corrections, and the intent with it',
    (tester) async {
      expect(repo, isNotEmpty, reason: 'run this through app/tool/sim_test.sh');
      final scratch = await Directory.systemTemp.createTemp('m38-review-');
      addTearDown(() => scratch.delete(recursive: true));
      final root = '${scratch.path}/processed';

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
        state: state.copyWith(
          playerId: () => golfer.playerId,
          club: () => '7i',
        ),
      );

      final manifest = jsonDecode(
        File('$repo/spikes/vision-ocr/photos.json').readAsStringSync(),
      ) as Map<String, Object?>;
      final capture =
          '$repo/${((manifest['captures']! as List).first as Map<String, Object?>)['path']! as String}';

      final phone = TestPhone(scratch.path)..picks.add([capture]);
      final model = AppModel(core: const RustCore(), phone: phone);
      await tester.pumpWidget(GolfApp(model: model));
      await pumpUntil(tester, find.text('Aaron · 7i · Fundamentals'));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('library')));
      await pumpUntil(tester, cardOf('1'));
      await pumpWhile(tester, () => !libraryReady(tester));
      final shotId = '$session-1';
      final asRead = storedShots(root)[shotId]!;
      final readCarry = asRead['carry_distance'] as double?;
      expect(
        readCarry,
        isNotNull,
        reason: 'the first capture reads its carry (P21 finding 2)',
      );
      expect(
        (asRead['provenance']! as Map<String, Object?>)['needs_review'],
        isFalse,
        reason: 'so its review is opened by a tap and is not forced',
      );
      expect(find.byType(ReviewScreen), findsNothing);

      // The unflagged card opens the review, with the stored photo drawn by the Simulator's decoder.
      await tester.pumpAndSettle();
      await tester.tap(cardOf('1'));
      await pumpUntil(tester, find.byType(ReviewScreen));
      await pumpUntil(tester, find.byKey(const ValueKey('review-photo-shown')));
      final carryField = find.byKey(const ValueKey('field-carry_distance'));
      final shownCarry = tester.widget<TextField>(carryField).controller!.text;
      expect(shownCarry, isNotEmpty);
      expect(shownCarry, isNot(typed));

      await tester.ensureVisible(carryField);
      await tester.pumpAndSettle();
      await tester.enterText(carryField, typed);
      await tester.pumpAndSettle();
      final performance = find.byKey(const ValueKey('intent-performance'));
      await tester.ensureVisible(performance);
      await tester.pumpAndSettle();
      await tester.tap(performance);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('review-save')));
      await pumpWhile(
        tester,
        () => find.byType(ReviewScreen).evaluate().isNotEmpty,
      );
      await tester.pumpAndSettle();

      // Back on the Record tab, the card is Rust's answer: the typed carry, marked corrected.
      expect(find.byType(RecordScreen), findsOneWidget);
      final carryRow = find.descendant(
        of: cardOf('1'),
        matching: find.byKey(const ValueKey('row-carry_distance')),
      );
      expect(
        find.descendant(of: carryRow, matching: find.text(typed)),
        findsOneWidget,
      );
      expect(
        find.descendant(of: carryRow, matching: find.text('corrected')),
        findsOneWidget,
      );

      // On disk: the typed value in the field, the as-read value in the audit, and the flag off.
      final corrected = storedShots(root)[shotId]!;
      final provenance = corrected['provenance']! as Map<String, Object?>;
      final corrections = provenance['corrections']! as Map<String, Object?>;
      expect(corrected['carry_distance'], 123.4);
      expect(corrections.keys, ['carry_distance']);
      expect(
        double.parse(corrections['carry_distance']! as String),
        readCarry,
        reason: 'corrections holds the carry as read, by value',
      );
      expect(provenance['needs_review'], isFalse);
      // Every other reading is the reader's, untouched.
      for (final key in asRead.keys) {
        if (key != 'carry_distance' && key != 'provenance') {
          expect(corrected[key], asRead[key], reason: key);
        }
      }

      final detail = await history.shotDetail(
        root: root,
        sessionId: session,
        swingId: '1',
      );
      expect(detail.row.mode, PracticeMode.performance);
      expect(detail.row.targetShape, isNull);
      expect(detail.row.corrected, isTrue);

      final report = StringBuffer()
        ..writeln('# M38 P22: one edit through the review, on the Simulator')
        ..writeln()
        ..writeln(
          'provisional: pasted copies. Read by the Simulator\'s iOS Vision, '
          'not a phone\'s.',
        )
        ..writeln()
        ..writeln('- capture: ${capture.split('/').last}')
        ..writeln('- carry shown in the review: $shownCarry')
        ..writeln('- carry typed: $typed')
        ..writeln('- carry_distance stored: ${corrected['carry_distance']}')
        ..writeln('- corrections: ${jsonEncode(corrections)}')
        ..writeln('- needs_review: ${provenance['needs_review']}')
        ..writeln(
          '- intent: ${detail.row.mode.name} ${detail.row.targetShape?.name}',
        );
      final out = Directory('$repo/target/m38')..createSync(recursive: true);
      File('${out.path}/review-edit.txt').writeAsStringSync('$report');
      print(report);
    },
  );
}
