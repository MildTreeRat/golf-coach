// The table and the box comparison are this test's output, read off the console and the file it writes,
// so it prints by design.
// ignore_for_file: avoid_print

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/rust/third_party/app/api/bay.dart';
import 'package:golf_app/src/rust/third_party/app/api/record.dart';
import 'package:golf_app/src/rust/third_party/app/api/setup.dart';
import 'package:integration_test/integration_test.dart';

import 'test_host.dart';

// The six captures through the Simulator's iOS Vision and the real bridge, held to M33's truth.
// [M38 P18]
//
// The first read by iOS's framework, and it is the Simulator's and not a device's. Each capture goes
// the way a library photo will: `readScreen` on the channel, then `record_photo` with the boxes as
// Swift wrote them. The same six with M33's committed boxes go into a second root, P15's control, so
// that the file this writes puts the Simulator's T1 to T4 beside macOS's from one code path, and no
// number of M33's is copied here.
//
// **Rule 11's hard stop is `report.wrong`**, a wrong number on a photo nothing flagged. T2 to T4 are
// recorded and stop nothing (D1: the bay is the test). Everything read off the captures is
// `provisional: pasted copies` (rule 12), which `report.text` says on its first line.

/// `repo` is `test_host.dart`'s: the host repo on the Simulator, and on a phone a copy of its inputs in
/// the app's own `tmp/`, staged by `app/tool/device_test.sh` (P30).
final spike = '$repo/spikes/vision-ocr';

/// One time for every stamp, as `crates/app/tests/bay_truth.rs` has it: the report reads no time, and a
/// fixed one keeps both roots' session ids alike.
const at = '2026-11-05T10:00:00-05:00';

/// How far a box may sit from M33's and still be called the same box, in pixels of the upright image.
const positionLimit = 1.0;

/// A data root as first-run setup leaves it: a golfer, a club, and nothing else.
Future<String> phoneRoot(Directory scratch, String name) async {
  final root = '${scratch.path}/$name/processed';
  await initState(root: root, installId: 'a1b2');
  final golfer = await createGolfer(
    root: root,
    name: 'Aaron',
    handedness: Handedness.right,
    now: at,
  );
  final state = await getState(root: root);
  await setState(
    root: root,
    state: AppState(
      installId: state.installId,
      playerId: golfer.playerId,
      club: '7i',
      mode: state.mode,
      targetShape: state.targetShape,
      challenge: state.challenge,
      challengeShapes: state.challengeShapes,
      appearance: state.appearance,
      lastExportAt: state.lastExportAt,
      lastExportShots: state.lastExportShots,
    ),
  );
  return root;
}

/// [boxesJson] recorded under [root] from a copy of the capture, since a `Failed` read moves the file
/// it is given and `spikes/` is read-only (P6 finding 11).
Future<RecordOutcome> record(
  String root,
  Directory scratch,
  String capture,
  String boxesJson,
) async {
  final tmp = Directory('${scratch.path}/tmp')..createSync(recursive: true);
  final copy = File(capture).copySync('${tmp.path}/${capture.split('/').last}');
  return recordPhoto(
    photo: PhotoIn(
      root: root,
      path: copy.path,
      boxesJson: boxesJson,
      capturedAt: at,
      now: at,
    ),
  );
}

/// The boxes of one answer, in the order the reader gave them.
List<Map<String, Object?>> boxesOf(String json) =>
    ((jsonDecode(json) as Map<String, Object?>)['boxes']! as List)
        .cast<Map<String, Object?>>();

/// The furthest any of a box's four numbers sits from another's, in pixels.
double moved(Map<String, Object?> a, Map<String, Object?> b) {
  var furthest = 0.0;
  for (final key in ['x', 'y', 'width', 'height']) {
    final delta = ((a[key]! as num) - (b[key]! as num)).abs().toDouble();
    if (delta > furthest) furthest = delta;
  }
  return furthest;
}

/// How the Simulator's boxes stand against M33's committed ones: one line, and the texts only one side
/// holds. The plan's comparison is count, text and position within [positionLimit].
///
/// **Measured, and held to nothing.** The first run found the Simulator's Vision is not macOS's to the
/// box (P18's findings): the boxes come in another order, a few texts and counts differ, and positions
/// move by pixels. Rule 11 stops on a wrong cell, which the report counts, and not on a box. So a box is
/// paired by its text, each of M33's with the nearest unpaired Simulator box reading the same, because
/// pairing by index compares unrelated boxes from the first one missing or out of order. The colour and
/// the confidence are said over the pairs, since only a pair is one glyph run read twice.
(String, List<String>) compareBoxes(String id, String ours, String theirs) {
  final sim = boxesOf(ours);
  final m33 = boxesOf(theirs);
  final unpaired = List.of(sim);
  final m33Only = <Object?>[];
  var paired = 0;
  var withinLimit = 0;
  var withinFive = 0;
  var sameConfidence = 0;
  var furthest = 0.0;
  var colour = 0.0;
  for (final theirBox in m33) {
    Map<String, Object?>? best;
    var bestMoved = double.infinity;
    for (final ourBox in unpaired) {
      if (ourBox['text'] != theirBox['text']) continue;
      final distance = moved(ourBox, theirBox);
      if (distance < bestMoved) {
        best = ourBox;
        bestMoved = distance;
      }
    }
    if (best == null) {
      m33Only.add(theirBox['text']);
      continue;
    }
    unpaired.remove(best);
    paired++;
    if (bestMoved <= positionLimit) withinLimit++;
    if (bestMoved <= 5) withinFive++;
    if (bestMoved > furthest) furthest = bestMoved;
    final ourRgb = (best['rgb']! as List).cast<num>();
    final theirRgb = (theirBox['rgb']! as List).cast<num>();
    for (var c = 0; c < 3; c++) {
      final delta = (ourRgb[c] - theirRgb[c]).abs().toDouble();
      if (delta > colour) colour = delta;
    }
    if (best['confidence'] == theirBox['confidence']) sameConfidence++;
  }
  final simOnly = [for (final box in unpaired) box['text']];
  final sameOrder =
      jsonEncode([for (final box in sim) box['text']]) ==
      jsonEncode([for (final box in m33) box['text']]);
  final line =
      '$id: ${sim.length} boxes against M33\'s ${m33.length}, $paired paired by text, '
      '${sameOrder ? 'in the same order' : 'in another order'}; '
      'of the pairs $withinLimit within $positionLimit px and $withinFive within 5 px, '
      'the furthest ${furthest.toStringAsFixed(3)} px, '
      'rgb within ${colour.toStringAsFixed(3)}, '
      'the same confidence on $sameConfidence';
  return (
    line,
    [
      if (m33Only.isNotEmpty) '$id: only M33 read ${jsonEncode(m33Only)}',
      if (simOnly.isNotEmpty)
        '$id: only the $hostName read ${jsonEncode(simOnly)}',
    ],
  );
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets(
    'the six captures read by iOS Vision on the $hostName leave no wrong cell on an unflagged photo',
    (tester) async {
      expect(
        repo,
        isNotEmpty,
        reason: 'run this through app/tool/sim_test.sh or device_test.sh',
      );
      const phone = ChannelPhone();

      final documents = await phone.documentsDir();
      expect(
        Directory(documents).existsSync(),
        isTrue,
        reason: 'documentsDir answered $documents',
      );

      final photosPath = '$spike/photos.json';
      final truthPath = '$spike/truth_full_swing.json';
      final manifest = jsonDecode(
        File(photosPath).readAsStringSync(),
      ) as Map<String, Object?>;
      final captures = (manifest['captures']! as List)
          .cast<Map<String, Object?>>();
      expect(captures, hasLength(6));

      final scratch = await Directory.systemTemp.createTemp('vision-gate-');
      final simRoot = await phoneRoot(scratch, 'sim');
      final m33Root = await phoneRoot(scratch, 'm33');
      final boxesOut = Directory('$repo/target/m38/$hostTag-c0')
        ..createSync(recursive: true);

      final reads = <String>[];
      final comparisons = <String>[];
      final differences = <String>[];
      String? os;
      String? platform;
      for (final capture in captures) {
        final id = capture['id']! as String;
        final path = '$repo/${capture['path']! as String}';
        final committed = File('$spike/boxes/C0/raw/$id.json')
            .readAsStringSync();

        final read = await phone.readScreen(path);
        File('${boxesOut.path}/$id.json').writeAsStringSync(read.json);
        final answer = jsonDecode(read.json) as Map<String, Object?>;
        os ??= answer['os'] as String?;
        platform ??= answer['platform'] as String?;

        final outcome = await record(simRoot, scratch, path, read.json);
        reads.add(
          '$id: ${outcome.status.name}, ${outcome.boxes} boxes, '
          'Swift ${read.msVision} ms, Rust ${outcome.msRust} ms',
        );
        await record(m33Root, scratch, path, committed);

        final (line, differs) = compareBoxes(id, read.json, committed);
        comparisons.add(line);
        differences.addAll(differs);
      }

      final sim = await truthReport(
        root: simRoot,
        photosPath: photosPath,
        truthPath: truthPath,
      );
      final m33 = await truthReport(
        root: m33Root,
        photosPath: photosPath,
        truthPath: truthPath,
      );

      final out = StringBuffer()
        ..writeln('# M38 P18: the six captures through iOS Vision and Rust')
        ..writeln(
          '# ${onDevice ? 'A device' : 'Simulator, not a device'}. Read by: platform $platform, os $os.',
        )
        ..writeln()
        ..writeln('## $hostName, iOS Vision through the channel and the bridge')
        ..writeln()
        ..write(sim.text)
        ..writeln()
        ..writeln(
          '## M33 on macOS, its committed C0 boxes through the same code',
        )
        ..writeln()
        ..write(m33.text)
        ..writeln()
        ..writeln('## Each read on the $hostName')
        ..writeln()
        ..writeAll(reads, '\n')
        ..writeln()
        ..writeln()
        ..writeln(
          '## Boxes against M33\'s committed ones (count, text, position within '
          '$positionLimit px)',
        )
        ..writeln()
        ..writeAll(comparisons, '\n')
        ..writeln()
        ..writeln()
        ..writeAll(differences, '\n')
        ..writeln();
      File('$repo/target/m38/$hostTag-c0.txt')
          .writeAsStringSync(out.toString());
      printReport(out);
      await scratch.delete(recursive: true);

      expect(
        sim.wrong,
        0,
        reason: 'rule 11: a wrong cell on a photo nothing flagged',
      );
      expect(sim.held, isTrue, reason: 'every capture found and no wrong cell');
      expect(m33.held, isTrue, reason: 'the control the $hostName sits beside');
    },
  );
}
