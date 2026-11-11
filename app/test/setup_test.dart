import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/screens/bag_screen.dart';
import 'package:golf_app/src/screens/setup_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/ui/shell.dart';

import 'fakes.dart';

// First-run setup over fakes, with no Rust and no Swift (C22). [M38 P19]
//
// The integration test `integration_test/setup_test.dart` runs the same steps through real Rust. Here
// each refusal is one the test gave the fake, so a screen showing it is showing the facade's words.

/// The whole app over [core], settled on its first screen.
Future<AppModel> pumpApp(WidgetTester tester, FakeCore core) async {
  final model = AppModel(core: core, phone: FakePhone(), random: Random(1));
  await tester.pumpWidget(GolfApp(model: model));
  await tester.pumpAndSettle();
  return model;
}

/// A first run's setup, with the golfer made and the bag screen open.
Future<FakeCore> pumpBag(WidgetTester tester) async {
  final core = FakeCore();
  await pumpApp(tester, core);
  await tester.enterText(find.byType(TextField), 'Aaron');
  await tester.tap(find.text('Continue'));
  await tester.pumpAndSettle();
  expect(find.byType(BagScreen), findsOneWidget);
  return core;
}

Finder clubRow(String club) => find.byKey(ValueKey('club-$club'));

Future<void> typeLoft(WidgetTester tester, String club, String text) async {
  await tester.tap(
    find.descendant(of: clubRow(club), matching: find.byType(TextButton)),
  );
  await tester.pumpAndSettle();
  await tester.enterText(
    find.descendant(
      of: find.byType(LoftDialog),
      matching: find.byType(TextField),
    ),
    text,
  );
  await tester.tap(find.text('Save'));
  await tester.pumpAndSettle();
}

/// The shape `parse_now` reads: microseconds and a `±HH:MM` offset, never Dart's offsetless local time.
final rfc3339Shape = RegExp(
  r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{6}[+-]\d\d:\d\d$',
);

void main() {
  group('the first launch', () {
    test(
      'mints a four-hex install id and stores it through init_state',
      () async {
        final core = FakeCore();
        final model = AppModel(
          core: core,
          phone: FakePhone(),
          random: Random(1),
        );
        await model.start();

        expect(
          core.calls.where((call) => call.startsWith('initState')),
          hasLength(1),
        );
        expect(model.state.installId, matches(RegExp(r'^[0-9a-f]{4}$')));
        expect(model.root, '${FakePhone.documents}/processed');
      },
    );

    test('a later launch mints nothing, and keeps the id it finds', () async {
      final core = setUpCore();
      final model = AppModel(core: core, phone: FakePhone(), random: Random(1));
      await model.start();

      expect(core.calls.where((call) => call.startsWith('initState')), isEmpty);
      expect(model.state.installId, 'a1b2');
    });

    test('the id is four lowercase hex digits whatever the draw', () {
      for (final seed in [0, 1, 2, 3, 99]) {
        expect(mintInstallId(Random(seed)), matches(RegExp(r'^[0-9a-f]{4}$')));
      }
    });
  });

  group('the first screen', () {
    testWidgets('with no golfer is setup', (tester) async {
      await pumpApp(tester, FakeCore());
      expect(find.byType(SetupScreen), findsOneWidget);
      expect(find.byType(NavigationBar), findsNothing);
    });

    testWidgets(
      'with a golfer whose bag is empty is the bag, since the picker would list nothing',
      (tester) async {
        final core = setUpCore()..bags.clear();
        await pumpApp(tester, core);
        expect(find.byType(BagScreen), findsOneWidget);
      },
    );

    testWidgets('with a golfer and a bag is the tabs', (tester) async {
      await pumpApp(tester, setUpCore());
      expect(find.byType(Shell), findsOneWidget);
    });

    testWidgets('with a golfer the state names and no file holds is setup', (
      tester,
    ) async {
      final core = setUpCore()..golfers.clear();
      await pumpApp(tester, core);
      expect(find.byType(SetupScreen), findsOneWidget);
    });
  });

  group('setup', () {
    testWidgets(
      'a refused name shows Rust\'s sentence under the field and moves nothing',
      (tester) async {
        final core = FakeCore();
        const sentence = "name '!!!' has no usable characters for an id";
        core.refusedNames['!!!'] = sentence;
        await pumpApp(tester, core);

        await tester.enterText(find.byType(TextField), '!!!');
        await tester.tap(find.text('Continue'));
        await tester.pumpAndSettle();

        expect(find.text(sentence), findsOneWidget);
        expect(find.byType(SetupScreen), findsOneWidget);
        expect(core.stored.playerId, isNull);
        expect(core.calls, isNot(contains('setState')));
      },
    );

    testWidgets(
      'an empty name goes to Rust to refuse, since the slug rule is not Dart\'s',
      (tester) async {
        final core = FakeCore();
        core.refusedNames[''] = "name '' has no usable characters for an id";
        await pumpApp(tester, core);

        await tester.tap(find.text('Continue'));
        await tester.pumpAndSettle();

        expect(core.calls, contains('createGolfer  right'));
        expect(
          find.text("name '' has no usable characters for an id"),
          findsOneWidget,
        );
      },
    );

    testWidgets('a name and a hand make the golfer current and open the bag', (
      tester,
    ) async {
      final core = FakeCore();
      await pumpApp(tester, core);

      await tester.enterText(find.byType(TextField), 'Aaron');
      await tester.tap(find.text('Left-handed'));
      await tester.tap(find.text('Continue'));
      await tester.pumpAndSettle();

      expect(core.calls, contains('createGolfer Aaron left'));
      expect(core.stored.playerId, 'aaron');
      expect(find.byType(BagScreen), findsOneWidget);
      expect(find.text('left-handed'), findsOneWidget);
      expect(core.nows.single, matches(rfc3339Shape));
    });

    testWidgets(
      'a known golfer\'s stored hand is shown, and not the one ticked',
      (tester) async {
        final core = FakeCore(
          golfers: [
            const GolferView(
              playerId: 'aaron',
              displayName: 'Aaron',
              handedness: Handedness.left,
            ),
          ],
        );
        await pumpApp(tester, core);

        await tester.enterText(find.byType(TextField), 'Aaron');
        await tester.tap(find.text('Continue'));
        await tester.pumpAndSettle();

        expect(
          find.text('Already known as left-handed, and that stands.'),
          findsOneWidget,
        );
      },
    );
  });

  group('the bag', () {
    testWidgets(
      'lists every club the facade names, in its order and spelling',
      (tester) async {
        await pumpBag(tester);
        final titles = tester
            .widgetList<ListTile>(find.byType(ListTile))
            .map((tile) => tile.key)
            .whereType<ValueKey<String>>()
            .map((key) => key.value)
            .toList();
        expect(titles, ['club-driver', 'club-7i', 'club-pw']);
      },
    );

    testWidgets(
      'a tick puts a club in with no loft, and another takes it out',
      (tester) async {
        final core = await pumpBag(tester);

        await tester.tap(clubRow('7i'));
        await tester.pumpAndSettle();
        expect(core.calls.last, 'setBagSlot 7i null');
        expect(core.bags['aaron']!.map((slot) => slot.club), ['7i']);
        expect(
          find.descendant(of: clubRow('7i'), matching: find.text('Add loft')),
          findsOneWidget,
        );

        await tester.tap(
          find.descendant(of: clubRow('7i'), matching: find.byType(Checkbox)),
        );
        await tester.pumpAndSettle();
        expect(core.calls.last, 'removeBagSlot 7i');
        expect(core.bags['aaron'], isEmpty);
        expect(find.text('Add loft'), findsNothing);
      },
    );

    testWidgets(
      'a typed loft reaches set_bag_slot, and the row shows Rust\'s spelling of it',
      (tester) async {
        final core = await pumpBag(tester);
        await tester.tap(clubRow('7i'));
        await tester.pumpAndSettle();

        await typeLoft(tester, '7i', '34');

        expect(core.calls.last, 'setBagSlot 7i 34.0');
        expect(find.byType(LoftDialog), findsNothing);
        expect(
          find.descendant(of: clubRow('7i'), matching: find.text('34°')),
          findsOneWidget,
        );
        expect(
          find.text('34.0°'),
          findsNothing,
          reason: 'Dart formatted the number itself',
        );
      },
    );

    testWidgets(
      'a loft Rust refuses shows its sentence under the field, and the dialog stays',
      (tester) async {
        final core = await pumpBag(tester);
        await tester.tap(clubRow('7i'));
        await tester.pumpAndSettle();

        // `double.tryParse` reads `NaN`, so it is Rust's to refuse (the plan's P5 finding 3).
        await typeLoft(tester, '7i', 'NaN');

        expect(core.calls.last, startsWith('setBagSlot 7i NaN'));
        expect(find.text(core.refusedLofts), findsOneWidget);
        expect(find.byType(LoftDialog), findsOneWidget);
        expect(core.bags['aaron']!.single.loftDeg, isNull);
      },
    );

    testWidgets(
      'a loft is asked for no range, so an unlikely one is sent as typed',
      (tester) async {
        final core = await pumpBag(tester);
        await tester.tap(clubRow('7i'));
        await tester.pumpAndSettle();

        await typeLoft(tester, '7i', '340');

        expect(core.calls.last, 'setBagSlot 7i 340.0');
        expect(find.byType(LoftDialog), findsNothing);
      },
    );

    testWidgets('text that is not a number is not sent', (tester) async {
      final core = await pumpBag(tester);
      await tester.tap(clubRow('7i'));
      await tester.pumpAndSettle();
      final before = core.calls.length;

      await typeLoft(tester, '7i', 'thirty');

      expect(core.calls.length, before);
      expect(
        find.text('Type the loft in degrees, as a number.'),
        findsOneWidget,
      );
      expect(find.byType(LoftDialog), findsOneWidget);
    });

    testWidgets('an empty loft clears it', (tester) async {
      final core = await pumpBag(tester);
      await tester.tap(clubRow('7i'));
      await tester.pumpAndSettle();
      await typeLoft(tester, '7i', '34');

      await typeLoft(tester, '7i', '');

      expect(core.calls.last, 'setBagSlot 7i null');
      expect(
        find.descendant(of: clubRow('7i'), matching: find.text('Add loft')),
        findsOneWidget,
      );
    });

    testWidgets('Done waits for a club, and then opens the tabs', (
      tester,
    ) async {
      await pumpBag(tester);
      FilledButton done() =>
          tester.widget(find.widgetWithText(FilledButton, 'Done'));
      expect(done().onPressed, isNull);

      await tester.tap(clubRow('7i'));
      await tester.pumpAndSettle();
      expect(done().onPressed, isNotNull);

      await tester.tap(find.text('Done'));
      await tester.pumpAndSettle();
      expect(find.byType(Shell), findsOneWidget);
    });
  });

  group('now', () {
    test('carries an offset, and reads back as the same instant', () {
      final at = DateTime.now();
      final text = rfc3339(at);
      expect(text, matches(rfc3339Shape));
      expect(DateTime.parse(text).isAtSameMomentAs(at), isTrue, reason: text);
      expect(now(), matches(rfc3339Shape));
    });

    test('spells UTC as +00:00, with every field padded', () {
      expect(
        rfc3339(DateTime.utc(2026, 1, 2, 3, 4, 5, 6, 7)),
        '2026-01-02T03:04:05.006007+00:00',
      );
    });
  });
}
