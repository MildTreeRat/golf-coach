// The app's look (D13): Material 3, one seed colour, dark first. [M38 P19]
//
// It is used at the bay, between swings, often with a glove on, so **every tap target is at least
// `touchTarget`** rather than Material's 48, and numbers are set in tabular figures, so that a column
// of carries lines up digit under digit and a value changing does not shift its neighbours.

import 'package:flutter/material.dart';

import '../core/core_api.dart';

/// The one colour the schemes are built from: a fairway green, readable on a dark bay screen.
const seed = Color(0xFF2E7D32);

/// The smallest a tap target may be, on either side.
const touchTarget = 56.0;

/// The theme for [brightness]. The app passes both, and [themeModeOf] picks between them.
ThemeData golfTheme(Brightness brightness) {
  const target = Size(touchTarget, touchTarget);
  return ThemeData(
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(seedColor: seed, brightness: brightness),
    materialTapTargetSize: MaterialTapTargetSize.padded,
    textTheme: _tabularFigures,
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(minimumSize: target),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(minimumSize: target),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(minimumSize: target),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(minimumSize: target),
    ),
    // A segmented button takes no `minimumSize`: Flutter 3.47 drops it from the style it hands each
    // segment and sizes the row from a 40 px floor plus the density. Its largest density, 4, adds
    // the 16 that makes 56, and `test/app_test.dart` measures the result.
    segmentedButtonTheme: SegmentedButtonThemeData(
      style: SegmentedButton.styleFrom(
        visualDensity: const VisualDensity(
          vertical: VisualDensity.maximumDensity,
        ),
      ),
    ),
    listTileTheme: const ListTileThemeData(minTileHeight: touchTarget),
  );
}

/// Which of the two themes [appearance] asks for. `Dark` is the default `AppState` holds.
ThemeMode themeModeOf(Appearance appearance) => switch (appearance) {
  Appearance.dark => ThemeMode.dark,
  Appearance.system => ThemeMode.system,
  Appearance.light => ThemeMode.light,
};

const _tabular = TextStyle(fontFeatures: [FontFeature.tabularFigures()]);

/// Merged over the typography's own styles by [ThemeData], so each keeps its size and weight and gains
/// only the figures.
const _tabularFigures = TextTheme(
  displayLarge: _tabular,
  displayMedium: _tabular,
  displaySmall: _tabular,
  headlineLarge: _tabular,
  headlineMedium: _tabular,
  headlineSmall: _tabular,
  titleLarge: _tabular,
  titleMedium: _tabular,
  titleSmall: _tabular,
  bodyLarge: _tabular,
  bodyMedium: _tabular,
  bodySmall: _tabular,
  labelLarge: _tabular,
  labelMedium: _tabular,
  labelSmall: _tabular,
);
