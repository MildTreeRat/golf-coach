import 'dart:async';
import 'dart:io';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/screens/settings_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/ui/shell.dart';

import 'fakes.dart';

// The Settings tab over fakes, with no Rust and no Swift (C22): each row, and the export's three calls
// in their order (P13 finding 13). [M38 P27]

/// The model's clock in every test here, so the zip's name and the `now` it is cut under are known.
final clock = DateTime(2026, 11, 5, 11);

/// The summary's own time, a few seconds after the clock: what `last_export_at` must hold, and not the
/// time the share sheet closed.
const exportedAt = '2026-11-05T11:00:07.000000-05:00';

ExportSummary summaryAt(String path, {int shots = 6}) => ExportSummary(
  path: path,
  exportedAt: exportedAt,
  shots: shots,
  files: 21,
  bytes: 4096,
);

/// The phone's share logged into the core's calls, so one list holds the export's whole order, and
/// [onShare] run as the sheet answers.
class OrderedPhone extends FakePhone {
  OrderedPhone(this.core, {this.onShare}) : super(camera: false);

  final FakeCore core;
  final void Function()? onShare;

  @override
  Future<bool> share(String path) async {
    core.calls.add('share $path');
    onShare?.call();
    return super.share(path);
  }
}

/// A phone whose Documents is a real directory, so the export's own files can be looked at.
class DiskPhone extends OrderedPhone {
  DiskPhone(super.core, this.container);

  final String container;

  @override
  Future<String> documentsDir() async => '$container/Documents';
}

/// A core whose export waits until the test lets it answer, once what [exportAnswers] queued is spent.
class HeldCore extends FakeCore {
  HeldCore()
    : super(
        state: setUpCore().stored,
        golfers: setUpCore().golfers,
        bags: setUpCore().bags,
      );

  final held = Completer<ExportSummary>();

  @override
  Future<ExportSummary> exportSessions({
    required String root,
    required String outPath,
    required String now,
  }) {
    if (exportAnswers.isNotEmpty) {
      return super.exportSessions(root: root, outPath: outPath, now: now);
    }
    calls.add('exportSessions $outPath');
    return held.future;
  }
}

Future<AppModel> pumpSettings(
  WidgetTester tester,
  FakeCore core, {
  FakePhone? phone,
}) async {
  tester.view
    ..physicalSize = const Size(1170, 2532)
    ..devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final model = AppModel(
    core: core,
    phone: phone ?? FakePhone(camera: false),
    random: Random(1),
    clock: () => clock,
  );
  await tester.pumpWidget(GolfApp(model: model));
  await tester.pumpAndSettle();
  await enterSettings(tester);
  return model;
}

Future<void> enterSettings(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.settings_outlined));
  await tester.pumpAndSettle();
}

Future<void> enterRecord(WidgetTester tester) async {
  await tester.tap(find.byIcon(Icons.photo_camera_outlined));
  await tester.pumpAndSettle();
}

Finder byKey(String key) => find.byKey(ValueKey(key));

Finder inside(String key, Finder matching) =>
    find.descendant(of: byKey(key), matching: matching);

/// The text [finder] shows: its own, or the one [Text] inside it.
String textAt(WidgetTester tester, Finder finder) => tester
    .widget<Text>(
      find.descendant(of: finder, matching: find.byType(Text), matchRoot: true),
    )
    .data!;

/// Scrolls the tab until [target] is built and on screen, down or, with [up], back up.
Future<void> scrollTo(
  WidgetTester tester,
  Finder target, {
  bool up = false,
}) async {
  await tester.scrollUntilVisible(
    target,
    up ? -100 : 100,
    scrollable: find.descendant(
      of: byKey('settings-list'),
      matching: find.byType(Scrollable),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> tapExport(WidgetTester tester) async {
  await scrollTo(tester, byKey('export-button'));
  await tester.tap(byKey('export-button'));
  await tester.pumpAndSettle();
}

/// Where the zip goes for [FakePhone.documents] at the test's clock.
final fakeZip = exportPath(
  '${FakePhone.documents}/processed',
  'a1b2',
  rfc3339(clock),
);

int statusReads(FakeCore core) =>
    core.calls.where((call) => call == 'exportStatus').length;

/// The calls from the export's first on, each cut to its name.
List<String> exportCalls(FakeCore core) {
  final names = [for (final call in core.calls) call.split(' ').first];
  return names.sublist(names.indexOf('exportSessions'));
}

/// Two golfers: Aaron with the 7i, current and holding it, and Beth with a wedge alone.
FakeCore twoGolfers() {
  final core = setUpCore();
  core.stored = core.stored.copyWith(club: () => '7i');
  core.golfers.add(
    const GolferView(
      playerId: 'beth',
      displayName: 'Beth',
      handedness: Handedness.left,
    ),
  );
  core.bags['beth'] = const [BagSlot(club: 'pw')];
  return core;
}

void main() {
  group('the rows', () {
    testWidgets(
      'golfers with the current one ticked, the bag, the appearance, the export, the warning and the build',
      (tester) async {
        await pumpSettings(tester, twoGolfers());

        expect(textAt(tester, byKey('settings-title')), SettingsScreen.title);
        expect(inside('golfer-aaron', find.byIcon(Icons.check)), findsOne);
        expect(inside('golfer-beth', find.byIcon(Icons.check)), findsNothing);
        expect(inside('golfer-beth', find.text('left-handed')), findsOne);
        expect(byKey('golfer-add'), findsOneWidget);
        expect(inside('bag-row', find.text('7i')), findsOneWidget);

        final appearance = tester.widget<SegmentedButton<Appearance>>(
          byKey('appearance'),
        );
        expect(appearance.selected, {Appearance.dark});
        expect(
          [for (final each in appearance.segments) each.value],
          [Appearance.dark, Appearance.system, Appearance.light],
        );

        await scrollTo(tester, byKey('export-button'));
        expect(
          textAt(tester, byKey('export-last')),
          SettingsScreen.neverExported,
        );
        expect(textAt(tester, byKey('export-shots')), '0 shots on this phone.');
        expect(byKey('export-outcome'), findsNothing);

        await scrollTo(tester, byKey('delete-warning'));
        expect(
          inside('delete-warning', find.text(SettingsScreen.deleteWarning)),
          findsOne,
        );

        // Each as `about` answered it, and nothing copied from a constant on this side.
        for (final (key, value) in [
          ('about-analysis', '17'),
          ('about-screen', '5'),
          ('about-career', '3'),
          ('about-device', 'full_swing'),
          ('about-build', '0.1.0'),
        ]) {
          await scrollTo(tester, byKey(key));
          expect(textAt(tester, inside(key, byKey('value'))), value);
        }
      },
    );

    testWidgets('the bag reads what the golfer carries, in bag order', (
      tester,
    ) async {
      final core = setUpCore();
      core.bags['aaron'] = const [
        BagSlot(club: 'driver'),
        BagSlot(club: '7i', loftDeg: 34, loftDisplay: '34°'),
        BagSlot(club: 'pw'),
      ];
      await pumpSettings(tester, core);
      expect(inside('bag-row', find.text('driver, 7i, pw')), findsOneWidget);
    });

    testWidgets(
      'a status is Rust\'s counts and time, with nothing worked out here',
      (tester) async {
        final core = setUpCore()
          ..status = const ExportStatus(
            lastExportAt: '2026-11-05T11:00:00-05:00',
            lastExportDisplay: '2026-11-05 11:00',
            lastExportShots: 6,
            shots: 9,
            shotsSince: 4,
          );
        await pumpSettings(tester, core);
        await scrollTo(tester, byKey('export-button'));

        expect(
          textAt(tester, byKey('export-last')),
          'Last export 2026-11-05 11:00, holding 6 shots.',
        );
        // 4 since, from Rust, where 9 less 6 would be 3: a stray deleted and a photo retaken.
        expect(
          textAt(tester, byKey('export-shots')),
          '9 shots on this phone, 4 new since then.',
        );
      },
    );

    testWidgets('one shot is a shot, in both lines', (tester) async {
      final core = setUpCore()
        ..status = const ExportStatus(
          lastExportAt: '2026-11-05T11:00:00-05:00',
          lastExportDisplay: '2026-11-05 11:00',
          lastExportShots: 1,
          shots: 1,
          shotsSince: 0,
        );
      await pumpSettings(tester, core);
      await scrollTo(tester, byKey('export-button'));

      expect(
        textAt(tester, byKey('export-last')),
        'Last export 2026-11-05 11:00, holding 1 shot.',
      );
      expect(
        textAt(tester, byKey('export-shots')),
        '1 shot on this phone, 0 new since then.',
      );
    });
  });

  group('golfers', () {
    testWidgets(
      'a tap makes another golfer current, and a club their bag does not hold is cleared',
      (tester) async {
        final core = twoGolfers();
        final model = await pumpSettings(tester, core);

        await tester.tap(byKey('golfer-beth'));
        await tester.pumpAndSettle();

        expect(model.golfer?.playerId, 'beth');
        expect(core.stored.playerId, 'beth');
        expect(core.stored.club, isNull);
        expect(inside('golfer-beth', find.byIcon(Icons.check)), findsOne);
        expect(inside('golfer-aaron', find.byIcon(Icons.check)), findsNothing);
        expect(inside('bag-row', find.text('pw')), findsOneWidget);
      },
    );

    testWidgets('a tap on the current golfer writes nothing', (tester) async {
      final core = twoGolfers();
      await pumpSettings(tester, core);
      core.calls.clear();

      await tester.tap(byKey('golfer-aaron'));
      await tester.pumpAndSettle();

      expect(core.calls, isEmpty);
    });

    testWidgets(
      'Add a golfer asks on setup\'s screen, then opens their bag over the tabs, and setup is not re-entered',
      (tester) async {
        final core = setUpCore();
        final model = await pumpSettings(tester, core);

        await tester.tap(byKey('golfer-add'));
        await tester.pumpAndSettle();
        expect(find.text('Who is swinging?'), findsOneWidget);
        await tester.enterText(find.byType(TextField), 'Beth');
        await tester.tap(find.text('Continue'));
        await tester.pumpAndSettle();

        expect(core.calls, contains('createGolfer Beth right'));
        expect(model.stage, Stage.ready);
        expect(model.golfer?.playerId, 'beth');
        // The bag screen, for the new golfer, with no setup Done: it closes back to the tabs.
        expect(find.text('Your bag'), findsOneWidget);
        expect(find.text('Beth'), findsOneWidget);
        expect(find.text('right-handed'), findsOneWidget);
        expect(find.text('Done'), findsNothing);
        await tester.tap(find.byKey(const ValueKey('club-pw')));
        await tester.pumpAndSettle();
        expect(core.bags['beth']!.map((slot) => slot.club), ['pw']);

        await tester.pageBack();
        await tester.pumpAndSettle();
        expect(find.byType(Shell), findsOneWidget);
        expect(find.byType(SettingsScreen), findsOneWidget);
        expect(inside('golfer-beth', find.byIcon(Icons.check)), findsOne);
        expect(inside('bag-row', find.text('pw')), findsOneWidget);
      },
    );

    testWidgets(
      'a known name is that golfer, whose hand stands, and the bag screen says so',
      (tester) async {
        final core = twoGolfers();
        await pumpSettings(tester, core);

        await tester.tap(byKey('golfer-add'));
        await tester.pumpAndSettle();
        await tester.enterText(find.byType(TextField), 'Beth');
        await tester.tap(find.text('Continue'));
        await tester.pumpAndSettle();

        expect(core.golfers, hasLength(2));
        expect(
          find.text('Already known as left-handed, and that stands.'),
          findsOneWidget,
        );

        // The note is about the name just typed: a golfer picked afterwards does not carry it.
        await tester.pageBack();
        await tester.pumpAndSettle();
        await tester.tap(byKey('golfer-aaron'));
        await tester.pumpAndSettle();
        await tester.tap(byKey('bag-row'));
        await tester.pumpAndSettle();
        expect(find.text('right-handed'), findsOneWidget);
        expect(find.textContaining('Already known'), findsNothing);
      },
    );

    testWidgets(
      'a name Rust refuses keeps the name screen up, with Rust\'s sentence under the field',
      (tester) async {
        final core = setUpCore();
        core.refusedNames['?'] = 'refused by the fake: a name needs a letter';
        final model = await pumpSettings(tester, core);

        await tester.tap(byKey('golfer-add'));
        await tester.pumpAndSettle();
        await tester.enterText(find.byType(TextField), '?');
        await tester.tap(find.text('Continue'));
        await tester.pumpAndSettle();

        expect(
          find.text('refused by the fake: a name needs a letter'),
          findsOneWidget,
        );
        expect(find.text('Who is swinging?'), findsOneWidget);
        expect(model.golfer?.playerId, 'aaron');
      },
    );
  });

  testWidgets(
    'the bag row opens the bag screen with no Done, and a change shows back on the row',
    (tester) async {
      final core = setUpCore();
      await pumpSettings(tester, core);

      await tester.tap(byKey('bag-row'));
      await tester.pumpAndSettle();
      expect(find.text('Your bag'), findsOneWidget);
      expect(find.text('Done'), findsNothing);
      await tester.tap(find.byKey(const ValueKey('club-pw')));
      await tester.pumpAndSettle();
      expect(core.calls, contains('setBagSlot pw null'));

      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(inside('bag-row', find.text('7i, pw')), findsOneWidget);
    },
  );

  testWidgets('a tap on an appearance stores it, and the app follows', (
    tester,
  ) async {
    final core = setUpCore();
    await pumpSettings(tester, core);
    expect(
      Theme.of(tester.element(find.byType(Shell))).brightness,
      Brightness.dark,
    );

    await tester.tap(find.text('Light'));
    await tester.pumpAndSettle();

    expect(core.stored.appearance, Appearance.light);
    expect(
      Theme.of(tester.element(find.byType(Shell))).brightness,
      Brightness.light,
    );
    expect(
      tester.widget<SegmentedButton<Appearance>>(byKey('appearance')).selected,
      {Appearance.light},
    );
  });

  group('export', () {
    test('the zip goes in tmp beside Documents, outside the root, named for the install and the time', () {
      const root =
          '/var/mobile/Containers/Data/Application/C0FFEE/Documents/processed';
      final path = exportPath(root, 'a1b2', '2026-11-05T11:00:07.123456-05:00');
      expect(
        path,
        '/var/mobile/Containers/Data/Application/C0FFEE/tmp/export/'
        'golf-coach-a1b2-2026-11-05-110007.zip',
      );
      expect(path.startsWith('$root/'), isFalse);
    });

    testWidgets(
      'cuts the zip, shares it, and only then records the summary\'s own time and shots',
      (tester) async {
        final core = setUpCore();
        final phone = OrderedPhone(
          core,
          // What Rust's status would answer once the export is recorded.
          onShare: () => core.status = const ExportStatus(
            lastExportAt: exportedAt,
            lastExportDisplay: '2026-11-05 11:00',
            lastExportShots: 6,
            shots: 6,
            shotsSince: 0,
          ),
        );
        core.exportAnswers.add(summaryAt(fakeZip));
        await pumpSettings(tester, core, phone: phone);

        await tapExport(tester);

        expect(exportCalls(core), [
          'exportSessions',
          'share',
          'setState',
          'exportStatus',
        ]);
        expect(core.calls, contains('exportSessions $fakeZip'));
        expect(core.calls, contains('share $fakeZip'));
        // The zip was cut under the model's clock, and the summary's time is what is stored.
        expect(core.nows.last, rfc3339(clock));
        expect(core.stored.lastExportAt, exportedAt);
        expect(core.stored.lastExportShots, 6);
        expect(
          textAt(tester, byKey('export-last')),
          'Last export 2026-11-05 11:00, holding 6 shots.',
        );
        expect(
          textAt(tester, byKey('export-shots')),
          '6 shots on this phone, 0 new since then.',
        );
        expect(byKey('export-outcome'), findsNothing);
      },
    );

    testWidgets(
      'the path shared is the one the summary names, and a second export records the second summary',
      (tester) async {
        final core = setUpCore();
        core.exportAnswers
          ..add(summaryAt('/elsewhere/first.zip', shots: 6))
          ..add(summaryAt('/elsewhere/second.zip', shots: 7));
        await pumpSettings(tester, core, phone: OrderedPhone(core));

        await tapExport(tester);
        await tapExport(tester);

        expect(core.calls.where((call) => call.startsWith('share')), [
          'share /elsewhere/first.zip',
          'share /elsewhere/second.zip',
        ]);
        expect(core.stored.lastExportShots, 7);
      },
    );

    testWidgets(
      'a sheet closed with nothing sent records no export, and says so',
      (tester) async {
        final core = setUpCore();
        final phone = OrderedPhone(core)..shareCompletes = false;
        core.exportAnswers.add(summaryAt(fakeZip));
        await pumpSettings(tester, core, phone: phone);

        await tapExport(tester);

        expect(exportCalls(core), ['exportSessions', 'share', 'exportStatus']);
        expect(core.stored.lastExportAt, isNull);
        expect(core.stored.lastExportShots, isNull);
        expect(
          textAt(tester, byKey('export-outcome')),
          SettingsScreen.notShared,
        );
        expect(
          textAt(tester, byKey('export-last')),
          SettingsScreen.neverExported,
        );
      },
    );

    testWidgets(
      'Rust\'s refusal shows as it stands, and nothing is shared or recorded',
      (tester) async {
        final core = setUpCore();
        final phone = OrderedPhone(core);
        core.exportAnswers.add('There is nothing on this phone to export yet.');
        await pumpSettings(tester, core, phone: phone);

        await tapExport(tester);

        expect(exportCalls(core), ['exportSessions', 'exportStatus']);
        expect(phone.calls.where((call) => call.startsWith('share')), isEmpty);
        expect(core.stored.lastExportAt, isNull);
        expect(
          textAt(tester, byKey('export-outcome')),
          'There is nothing on this phone to export yet.',
        );
      },
    );

    testWidgets(
      'a share sheet that fails shows Swift\'s sentence, and records nothing',
      (tester) async {
        final core = setUpCore();
        final phone = OrderedPhone(core)
          ..shareFails = PlatformException(
            code: 'busy',
            message:
                'The camera, the library or the share sheet is already open.',
          );
        core.exportAnswers.add(summaryAt(fakeZip));
        await pumpSettings(tester, core, phone: phone);

        await tapExport(tester);

        expect(core.stored.lastExportAt, isNull);
        expect(
          textAt(tester, byKey('export-outcome')),
          'The camera, the library or the share sheet is already open.',
        );
      },
    );

    testWidgets(
      'a failed export\'s sentence goes once the next one is recorded',
      (tester) async {
        final core = setUpCore();
        core.exportAnswers
          ..add('There is nothing on this phone to export yet.')
          ..add(summaryAt(fakeZip));
        await pumpSettings(tester, core, phone: OrderedPhone(core));

        await tapExport(tester);
        expect(byKey('export-outcome'), findsOneWidget);
        await tapExport(tester);
        expect(byKey('export-outcome'), findsNothing);
        expect(core.stored.lastExportAt, exportedAt);
      },
    );

    testWidgets(
      'the button is held, with a bar, while an export is out, and the last one\'s sentence is gone',
      (tester) async {
        final core = HeldCore();
        core.exportAnswers.add('There is nothing on this phone to export yet.');
        await pumpSettings(tester, core, phone: OrderedPhone(core));
        await tapExport(tester);
        expect(byKey('export-outcome'), findsOneWidget);

        await tester.tap(byKey('export-button'));
        await tester.pump();
        expect(byKey('export-busy'), findsOneWidget);
        expect(byKey('export-outcome'), findsNothing);
        await tester.tap(byKey('export-button'));
        await tester.pump();
        expect(
          core.calls.where((call) => call.startsWith('exportSessions')),
          hasLength(2),
        );

        core.held.complete(summaryAt(fakeZip));
        await tester.pumpAndSettle();
        expect(byKey('export-busy'), findsNothing);
        expect(core.stored.lastExportAt, exportedAt);
      },
    );

    testWidgets(
      'only the newest zip is kept: the last export\'s directory goes before the next is cut',
      (tester) async {
        final container = Directory.systemTemp.createTempSync('m38-settings-');
        addTearDown(() => container.deleteSync(recursive: true));
        final old = File('${container.path}/tmp/export/golf-coach-a1b2-old.zip')
          ..createSync(recursive: true);
        // A photo PhotoSource copied is in tmp too, and stays.
        final photo = File('${container.path}/tmp/library/0/IMG_2889.jpg')
          ..createSync(recursive: true);
        final core = setUpCore();
        final zip = exportPath(
          '${container.path}/Documents/processed',
          'a1b2',
          rfc3339(clock),
        );
        core.exportAnswers.add(summaryAt(zip));
        await pumpSettings(
          tester,
          core,
          phone: DiskPhone(core, container.path),
        );

        await tapExport(tester);

        expect(core.calls, contains('exportSessions $zip'));
        expect(old.existsSync(), isFalse);
        expect(photo.existsSync(), isTrue);
      },
    );
  });

  group('when the status is read', () {
    testWidgets(
      'on entering the tab, and after a shot written while it is shown, and not on a setting',
      (tester) async {
        final core = setUpCore();
        tester.view
          ..physicalSize = const Size(1170, 2532)
          ..devicePixelRatio = 3;
        addTearDown(tester.view.reset);
        final model = AppModel(
          core: core,
          phone: FakePhone(camera: false),
          random: Random(1),
          clock: () => clock,
        );
        await tester.pumpWidget(GolfApp(model: model));
        await tester.pumpAndSettle();
        expect(statusReads(core), 0);

        await enterSettings(tester);
        expect(statusReads(core), 1);

        model.shotsChanged();
        await tester.pumpAndSettle();
        expect(statusReads(core), 2);

        await model.setAppearance(Appearance.light);
        await tester.pumpAndSettle();
        expect(statusReads(core), 2);

        await enterRecord(tester);
        model.shotsChanged();
        await tester.pumpAndSettle();
        expect(statusReads(core), 2);

        await enterSettings(tester);
        expect(statusReads(core), 3);
      },
    );

    testWidgets('the build is read once, on the first entry', (tester) async {
      final core = setUpCore();
      await pumpSettings(tester, core);
      await enterRecord(tester);
      await enterSettings(tester);

      expect(core.calls.where((call) => call == 'about'), hasLength(1));
    });
  });
}
