// What the export wrote is this test's output, read off the console and the file it writes, so it
// prints by design.
// ignore_for_file: avoid_print

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/screens/settings_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:integration_test/integration_test.dart';

import 'record_test.dart' show TestPhone, installId, session;
import 'setup_test.dart' show pumpUntil;

// The Settings tab's Export through the real bridge and real Rust, on the Simulator. [M38 P27]
//
// Six captures recorded through the facade with M33's committed C0 boxes, as P25's and P26's tests do,
// so the Record tab's Vision is not what this tests. Then the tab's Export: a real zip cut by
// `export_sessions` under `tmp/export/`, handed to the phone's `share`, and recorded as the last export.
// **The share sheet is not reached**: a native sheet cannot be driven from here, so [ExportPhone]
// answers it by copying the zip to `target/m38/export.zip`, which is the zip itself as the sheet was
// given it. The sheet stays unproven until P30. Everything read off the captures is `provisional:
// pasted copies` (rule 12), and on the Simulator, not a phone.

const repo = String.fromEnvironment('GOLF_REPO');

const captures = [
  'IMG_2889',
  'IMG_2891',
  'IMG_2892',
  'IMG_2893',
  'IMG_2894',
  '2026-10-07-1',
];

/// [TestPhone] with Documents one level down, so the scratch directory stands where the app's container
/// does and `tmp/` beside it is the scratch's own, and a share that copies the zip out and says it went.
class ExportPhone extends TestPhone {
  ExportPhone(super.scratch);

  /// Every path [share] was handed, in order.
  final List<String> shared = [];

  @override
  Future<String> documentsDir() async => '$scratch/Documents';

  @override
  Future<bool> share(String path) async {
    shared.add(path);
    File(path).copySync('$repo/target/m38/export.zip');
    return true;
  }
}

/// One entry of a zip's central directory.
typedef ZipEntry = ({String name, int method, Uint8List data});

/// A zip's entries in the order its central directory lists them, each with its method and, for a
/// stored one, its bytes. Read by hand, since the app takes no zip package (rule 10): the end record,
/// then each central header and the local header it points at.
List<ZipEntry> zipEntries(Uint8List bytes) {
  final data = ByteData.sublistView(bytes);
  int u16(int at) => data.getUint16(at, Endian.little);
  int u32(int at) => data.getUint32(at, Endian.little);
  var end = bytes.length - 22;
  while (u32(end) != 0x06054b50) {
    end--;
  }
  final entries = <ZipEntry>[];
  var at = u32(end + 16);
  for (var index = 0; index < u16(end + 10); index++) {
    expect(u32(at), 0x02014b50, reason: 'central header $index');
    final method = u16(at + 10);
    final size = u32(at + 20);
    final nameLength = u16(at + 28);
    final name = utf8.decode(bytes.sublist(at + 46, at + 46 + nameLength));
    final local = u32(at + 42);
    final start = local + 30 + u16(local + 26) + u16(local + 28);
    entries.add((
      name: name,
      method: method,
      data: Uint8List.sublistView(bytes, start, start + size),
    ));
    at += 46 + nameLength + u16(at + 30) + u16(at + 32);
  }
  return entries;
}

bool sameBytes(Uint8List a, Uint8List b) {
  if (a.length != b.length) {
    return false;
  }
  for (var index = 0; index < a.length; index++) {
    if (a[index] != b[index]) {
      return false;
    }
  }
  return true;
}

Finder byKey(String key) => find.byKey(ValueKey(key));

String textAt(WidgetTester tester, String key) =>
    tester.widget<Text>(byKey(key)).data!;

Future<void> scrollTo(WidgetTester tester, Finder target) async {
  await tester.pumpAndSettle();
  await tester.scrollUntilVisible(
    target,
    100,
    scrollable: find.descendant(
      of: byKey('settings-list'),
      matching: find.byType(Scrollable),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets(
    'Export cuts a real zip of the six shots, hands it to the share, and records it as the last export',
    (tester) async {
      expect(repo, isNotEmpty, reason: 'run this through app/tool/sim_test.sh');
      final scratch = await Directory.systemTemp.createTemp('m38-export-');
      addTearDown(() => scratch.delete(recursive: true));
      final root = '${scratch.path}/Documents/processed';
      const core = RustCore();

      await core.initState(root: root, installId: installId);
      final golfer = await core.createGolfer(
        root: root,
        name: 'Aaron',
        handedness: Handedness.right,
        now: now(),
      );
      await core.setBagSlot(
        root: root,
        playerId: golfer.playerId,
        club: '7i',
        loftDeg: 34,
        now: now(),
      );
      final state = await core.getState(root: root);
      await core.setState(
        root: root,
        state: state.copyWith(
          playerId: () => golfer.playerId,
          club: () => '7i',
        ),
      );
      final photos = <String, Uint8List>{};
      for (final (index, id) in captures.indexed) {
        final copy = File('$repo/spikes/vision-ocr/captures/$id.jpg')
            .copySync('${scratch.path}/$id.jpg');
        photos[id] = copy.readAsBytesSync();
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
      final before = await core.exportStatus(root: root);
      expect(before.shots, 6);
      expect(before.lastExportDisplay, isNull);

      final phone = ExportPhone(scratch.path);
      final model = AppModel(core: core, phone: phone);
      await tester.pumpWidget(GolfApp(model: model));
      await pumpUntil(tester, find.text('Aaron · 7i · Fundamentals'));
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.settings_outlined));
      await pumpUntil(tester, byKey('export-shots'));
      await scrollTo(tester, byKey('export-button'));
      expect(textAt(tester, 'export-last'), SettingsScreen.neverExported);
      expect(textAt(tester, 'export-shots'), '6 shots on this phone.');

      await tester.tap(byKey('export-button'));
      await pumpUntil(tester, find.textContaining('Last export '));
      await tester.pumpAndSettle();

      // The one zip, cut beside Documents and handed to the share as it was named.
      expect(phone.shared, hasLength(1));
      final zip = phone.shared.single;
      final exportDir = Directory('${scratch.path}/tmp/export');
      expect(zip, startsWith('${exportDir.path}/golf-coach-$installId-'));
      expect(zip, endsWith('.zip'));
      expect(exportDir.listSync().map((each) => each.path), [zip]);

      // Recorded as the last export, with Rust's counts on screen.
      final after = await core.exportStatus(root: root);
      final stored = await core.getState(root: root);
      expect(stored.lastExportShots, 6);
      expect(stored.lastExportAt, after.lastExportAt);
      expect(after.shots, 6);
      expect(after.shotsSince, 0);
      expect(
        textAt(tester, 'export-last'),
        SettingsScreen.lastExport(after.lastExportDisplay!, 6),
      );
      expect(
        textAt(tester, 'export-shots'),
        '6 shots on this phone, 0 new since then.',
      );
      expect(byKey('export-outcome'), findsNothing);

      // The zip holds the six shots: each entry's manifest, analysis and photo, its shot, and the
      // golfer, and every photo stored as the capture's own bytes.
      final copied = File('$repo/target/m38/export.zip').readAsBytesSync();
      expect(sameBytes(copied, File(zip).readAsBytesSync()), isTrue);
      final entries = zipEntries(copied);
      final names = [for (final entry in entries) entry.name];
      expect(names, [...names]..sort(), reason: 'sorted by name');
      expect(
        names,
        containsAll(['golfers/aaron.golfer.json', 'golfers/aaron.bag.json']),
      );
      expect(names.where((name) => name.startsWith('.app/state')), isEmpty);
      for (var swing = 1; swing <= captures.length; swing++) {
        final entry = 'sessions/$session/$swing';
        expect(names, contains('$entry/manifest.json'));
        expect(names, contains('$entry/shot_analysis.json'));
      }
      // The shot store keys a shot by its photo's digest, as `.app/boxes/` keys the boxes it was read
      // from, so the two name the same six photos.
      Set<String> digests(String prefix, String suffix) => {
        for (final name in names)
          if (name.startsWith(prefix) && name.endsWith(suffix))
            name.substring(prefix.length, name.length - suffix.length),
      };
      final shots = digests('shots/', '.shot.json');
      expect(shots, hasLength(captures.length));
      expect(digests('.app/boxes/', '.json'), shots);
      final stillPhotos = [
        for (final entry in entries)
          if (entry.name.startsWith('sessions/') &&
              !entry.name.endsWith('.json'))
            entry,
      ];
      expect(stillPhotos, hasLength(captures.length));
      for (final photo in stillPhotos) {
        expect(photo.method, 0, reason: '${photo.name} is stored');
      }
      for (final MapEntry(key: id, value: bytes) in photos.entries) {
        expect(
          stillPhotos.where((photo) => sameBytes(photo.data, bytes)),
          hasLength(1),
          reason: '$id is in the zip as its own bytes',
        );
      }
      for (final entry in entries) {
        if (entry.name.endsWith('.json')) {
          expect(entry.method, 8, reason: '${entry.name} is deflated');
        }
      }

      // A second export replaces the first: one zip in the directory, the second one shared.
      await tester.tap(byKey('export-button'));
      final deadline = DateTime.now().add(const Duration(seconds: 20));
      while (phone.shared.length < 2 ||
          find.byType(LinearProgressIndicator).evaluate().isNotEmpty) {
        expect(DateTime.now().isBefore(deadline), isTrue);
        await tester.pump(const Duration(milliseconds: 50));
      }
      expect(exportDir.listSync().map((each) => each.path), [
        phone.shared.last,
      ]);

      final report = StringBuffer()
        ..writeln('# M38 P27: the six captures exported from the Settings tab')
        ..writeln()
        ..writeln(
          'provisional: pasted copies. Recorded through M33\'s committed C0 '
          'boxes on the iOS Simulator, not read by a phone. The share sheet '
          'was answered by the test, which copied the zip here.',
        )
        ..writeln()
        ..writeln('- captures: ${captures.join(', ')}')
        ..writeln('- zip: ${zip.split('/').last}, ${copied.length} bytes')
        ..writeln('- entries: ${entries.length}')
        ..writeln(
          '- stored photos: ${stillPhotos.length}, each a capture\'s bytes',
        )
        ..writeln('- shown: ${textAt(tester, 'export-last')}')
        ..writeln('- shown: ${textAt(tester, 'export-shots')}')
        ..writeln(
          '- state: last_export_at ${stored.lastExportAt}, '
          'last_export_shots ${stored.lastExportShots}',
        )
        ..writeln('- second export: one zip left in tmp/export');
      for (final name in names) {
        report.writeln('  - $name');
      }
      File('$repo/target/m38/export.txt').writeAsStringSync('$report');
      print(report);
    },
  );
}
