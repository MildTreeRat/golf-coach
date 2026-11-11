import 'dart:ui' show PlatformDispatcher;

import 'package:flutter/material.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/screens/bag_screen.dart';
import 'package:golf_app/src/screens/setup_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/ui/shell.dart';
import 'package:golf_app/src/ui/theme.dart';

Future<void> main() async {
  await RustLib.init();
  final model = AppModel(core: const RustCore(), phone: const ChannelPhone());
  logUncaughtErrors(model);
  runApp(GolfApp(model: model));
}

/// Send every error nothing caught to `.app/events.jsonl` as an `error` (D16), and then on to whatever
/// had it before, so the console, and a test's harness, still see it as they did. [M38 P30b]
///
/// **Two handlers, because an error goes uncaught two ways.** One thrown while building, laying out or
/// painting, or in a gesture's callback, reaches [FlutterError.onError]. One from a future nothing
/// awaited reaches [PlatformDispatcher.onError]. Before this there was neither (the plan's P30 finding
/// 13), so an error at the bay left no trace once the screen moved on.
///
/// **Neither throws**, since [AppModel.logError] does not. The second answers what the handler before
/// it would have, and with none `false`, so an error that went unhandled before is still reported as
/// unhandled: logging it is not handling it. An error before [AppModel.start] has found the data root
/// has nowhere to go, and goes only where it went before.
void logUncaughtErrors(AppModel model) {
  final presented = FlutterError.onError;
  FlutterError.onError = (details) {
    model.logError(
      details.exceptionAsString(),
      details.stack,
      context: details.context?.toString(),
    );
    presented?.call(details);
  };
  final dispatcher = PlatformDispatcher.instance;
  final reported = dispatcher.onError;
  dispatcher.onError = (error, stack) {
    model.logError(error, stack);
    return reported?.call(error, stack) ?? false;
  };
}

/// The app: the theme the settings ask for, and the screen [AppModel.stage] says to open on. [M38 P19]
///
/// The model is handed in rather than made here, so that a widget test runs the whole app over fakes
/// with no Rust and no Swift (C22), and an integration test over the real ones with a scratch root.
class GolfApp extends StatefulWidget {
  const GolfApp({super.key, required this.model});

  final AppModel model;

  @override
  State<GolfApp> createState() => _GolfAppState();
}

class _GolfAppState extends State<GolfApp> {
  @override
  void initState() {
    super.initState();
    widget.model.start();
  }

  @override
  Widget build(BuildContext context) {
    final model = widget.model;
    return ListenableBuilder(
      listenable: model,
      builder: (context, _) => MaterialApp(
        title: 'Golf Coach',
        theme: golfTheme(Brightness.light),
        darkTheme: golfTheme(Brightness.dark),
        // Dark until the state is read, since that is the default it holds (D13).
        themeMode: model.stage == Stage.loading || model.stage == Stage.failed
            ? ThemeMode.dark
            : themeModeOf(model.state.appearance),
        home: _Home(model: model),
      ),
    );
  }
}

/// The root route's screen, which follows the stage. A [ListenableBuilder] of its own, so that it
/// rebuilds whatever the navigator has cached of the route.
class _Home extends StatelessWidget {
  const _Home({required this.model});

  final AppModel model;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: model,
      builder: (context, _) => switch (model.stage) {
        Stage.loading => const Scaffold(
          body: Center(child: CircularProgressIndicator()),
        ),
        Stage.failed => _Failed(failure: model.failure ?? ''),
        Stage.golfer => SetupScreen(model: model),
        Stage.bag => BagScreen(model: model, onDone: model.finishSetup),
        Stage.ready => Shell(model: model),
      },
    );
  }
}

/// The data root could not be opened. Nothing the app does works without it, so it says why and stops.
class _Failed extends StatelessWidget {
  const _Failed({required this.failure});

  final String failure;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Golf Coach could not open its data.',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 16),
              SelectableText(failure),
            ],
          ),
        ),
      ),
    );
  }
}
