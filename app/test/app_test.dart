import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/ui/shell.dart';
import 'package:golf_app/src/ui/theme.dart';

import 'fakes.dart';

// The shell and the theme over fakes, with no Rust and no Swift (C22): what D13 approved. [M38 P19]

Future<AppModel> pumpReady(
  WidgetTester tester, {
  Appearance appearance = Appearance.dark,
}) async {
  final model = AppModel(
    core: setUpCore(appearance: appearance),
    phone: FakePhone(),
    random: Random(1),
  );
  await tester.pumpWidget(GolfApp(model: model));
  await tester.pumpAndSettle();
  return model;
}

ThemeData themeOf(WidgetTester tester) =>
    Theme.of(tester.element(find.byType(Shell)));

void main() {
  testWidgets(
    'the tabs are Record, History, Profile and Settings, and Record opens first',
    (tester) async {
      await pumpReady(tester);

      final bar = tester.widget<NavigationBar>(find.byType(NavigationBar));
      expect(bar.selectedIndex, 0);
      final labels = find.descendant(
        of: find.byType(NavigationBar),
        matching: find.byType(Text),
      );
      expect(tester.widgetList<Text>(labels).map((text) => text.data), [
        'Record',
        'History',
        'Profile',
        'Settings',
      ]);
    },
  );

  testWidgets('a tab opens on a tap, and the others are kept', (tester) async {
    await pumpReady(tester);

    await tester.tap(find.byIcon(Icons.settings_outlined));
    await tester.pumpAndSettle();

    expect(
      tester.widget<NavigationBar>(find.byType(NavigationBar)).selectedIndex,
      3,
    );
    final stack = tester.widget<IndexedStack>(find.byType(IndexedStack));
    expect(stack.index, ShellTab.settings.index);
    expect(stack.children, hasLength(4));
  });

  testWidgets('the app is dark by default, as AppState is', (tester) async {
    await pumpReady(tester);
    expect(themeOf(tester).brightness, Brightness.dark);
    expect(themeOf(tester).useMaterial3, isTrue);
  });

  testWidgets(
    'the app follows the appearance the settings hold, and a change to it',
    (tester) async {
      final model = await pumpReady(tester, appearance: Appearance.light);
      expect(themeOf(tester).brightness, Brightness.light);

      await model.setAppearance(Appearance.dark);
      await tester.pumpAndSettle();
      expect(themeOf(tester).brightness, Brightness.dark);
      expect(model.state.appearance, Appearance.dark);
    },
  );

  test('every theme sets numbers in tabular figures', () {
    for (final brightness in Brightness.values) {
      final text = golfTheme(brightness).textTheme;
      for (final style in [
        text.bodyLarge,
        text.titleMedium,
        text.headlineMedium,
        text.labelLarge,
      ]) {
        expect(
          style!.fontFeatures,
          contains(const FontFeature.tabularFigures()),
        );
      }
    }
  });

  testWidgets('a tap target is at least 56 on either side', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: golfTheme(Brightness.dark),
        home: Scaffold(
          body: Column(
            children: [
              FilledButton(onPressed: () {}, child: const Text('f')),
              TextButton(onPressed: () {}, child: const Text('t')),
              OutlinedButton(onPressed: () {}, child: const Text('o')),
              IconButton(onPressed: () {}, icon: const Icon(Icons.add)),
              SegmentedButton<int>(
                segments: const [ButtonSegment(value: 1, label: Text('s'))],
                selected: const {1},
                onSelectionChanged: (_) {},
              ),
              const ListTile(title: Text('l')),
            ],
          ),
        ),
      ),
    );
    for (final type in [
      FilledButton,
      TextButton,
      OutlinedButton,
      IconButton,
      SegmentedButton<int>,
    ]) {
      // `.first`: a segmented button is built of text buttons, and the one put here comes first.
      final size = tester.getSize(find.byType(type).first);
      expect(size.width, greaterThanOrEqualTo(touchTarget), reason: '$type');
      expect(size.height, greaterThanOrEqualTo(touchTarget), reason: '$type');
    }
    expect(
      tester.getSize(find.byType(ListTile)).height,
      greaterThanOrEqualTo(touchTarget),
    );
  });
}
