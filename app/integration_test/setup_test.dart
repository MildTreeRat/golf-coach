import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/main.dart';
import 'package:golf_app/src/core/core_api.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/screens/bag_screen.dart';
import 'package:golf_app/src/screens/setup_screen.dart';
import 'package:golf_app/src/state/app_model.dart';
import 'package:golf_app/src/ui/shell.dart';
import 'package:integration_test/integration_test.dart';

// First-run setup through the widgets, real Rust and the real channel, on the Simulator. [M38 P19]
//
// What the widget tests cannot say: that `RustCore` turns a facade refusal into the `Refusal` a screen
// shows, that this app's `now` is one `parse_now` reads, and that what setup wrote is what the next
// launch reads. Each run starts from an empty scratch root and deletes it.

/// The real channel, with the data root moved to [scratch], so a run never meets the last run's
/// files or leaves its own in the app's container.
class ScratchPhone extends ChannelPhone {
  const ScratchPhone(this.scratch);

  final String scratch;

  @override
  Future<String> documentsDir() async => scratch;
}

/// Pump frames until [finder] finds something. A bridged call completes on the real clock, which
/// `pumpAndSettle` does not wait for, since no frame is scheduled while Rust works.
Future<void> pumpUntil(WidgetTester tester, Finder finder) async {
  final deadline = DateTime.now().add(const Duration(seconds: 20));
  while (finder.evaluate().isEmpty) {
    if (DateTime.now().isAfter(deadline)) {
      fail('nothing found by $finder in 20 s');
    }
    await tester.pump(const Duration(milliseconds: 50));
  }
}

Finder clubRow(String club) => find.byKey(ValueKey('club-$club'));

/// Type [text] as [club]'s loft and save it, leaving the answer to the caller to wait for.
///
/// **It settles first and after opening**, because the clock is real here: a dialog that is still
/// closing when the next opens makes two of everything (the first run found two `Save` buttons), and
/// a tap during either animation can land on the barrier.
Future<void> setLoft(WidgetTester tester, String club, String text) async {
  await tester.pumpAndSettle();
  await tester.tap(
    find.descendant(of: clubRow(club), matching: find.byType(TextButton)),
  );
  await pumpUntil(tester, find.byType(LoftDialog));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.descendant(
      of: find.byType(LoftDialog),
      matching: find.byType(TextField),
    ),
    text,
  );
  await tester.tap(find.text('Save'));
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets(
    'setup writes a golfer and a bag through real Rust, and the next launch finds them',
    (tester) async {
      final scratch = await Directory.systemTemp.createTemp('m38-setup-');
      addTearDown(() => scratch.delete(recursive: true));
      final phone = ScratchPhone(scratch.path);
      final model = AppModel(core: const RustCore(), phone: phone);

      await tester.pumpWidget(GolfApp(model: model));
      await pumpUntil(tester, find.byType(SetupScreen));
      expect(model.state.installId, matches(RegExp(r'^[0-9a-f]{4}$')));

      // The store's own sentence, through `RustCore`'s `Refusal`, under the field.
      await tester.enterText(find.byType(TextField), '!!!');
      await tester.tap(find.text('Continue'));
      await pumpUntil(
        tester,
        find.text("name '!!!' has no usable characters for an id"),
      );
      expect(find.byType(SetupScreen), findsOneWidget);

      await tester.enterText(find.byType(TextField), 'Aaron Sierra');
      await tester.tap(find.text('Left-handed'));
      await tester.tap(find.text('Continue'));
      await pumpUntil(tester, find.byType(BagScreen));
      expect(model.golfer!.playerId, 'aaron-sierra');

      await tester.tap(clubRow('driver'));
      await pumpUntil(
        tester,
        find.descendant(of: clubRow('driver'), matching: find.text('Add loft')),
      );
      await setLoft(tester, 'driver', '10.5');
      await pumpUntil(
        tester,
        find.descendant(of: clubRow('driver'), matching: find.text('10.5°')),
      );

      // Rust's finite guard, reached from a text field, as P5 finding 3 said it could be.
      await setLoft(tester, 'driver', 'Infinity');
      await pumpUntil(
        tester,
        find.text('a loft has to be a real number of degrees, and inf is not'),
      );
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      // Bag order is far down a long list, and the row has to be built before it can be tapped.
      await tester.scrollUntilVisible(clubRow('7i'), 200);
      await tester.tap(clubRow('7i'));
      await pumpUntil(
        tester,
        find.descendant(of: clubRow('7i'), matching: find.text('Add loft')),
      );
      await setLoft(tester, '7i', '34');
      await pumpUntil(
        tester,
        find.descendant(of: clubRow('7i'), matching: find.text('34°')),
      );

      await tester.pumpAndSettle();
      await tester.tap(find.text('Done'));
      await pumpUntil(tester, find.byType(Shell));

      // The next launch: a new model over the same root, which reads only the files.
      final again = AppModel(core: const RustCore(), phone: phone);
      await again.start();
      expect(again.stage, Stage.ready);
      expect(again.state.installId, model.state.installId);
      expect(again.golfer!.displayName, 'Aaron Sierra');
      expect(again.golfer!.handedness, Handedness.left);
      expect(
        again.bag.map((slot) => (slot.club, slot.loftDeg, slot.loftDisplay)),
        [('driver', 10.5, '10.5°'), ('7i', 34.0, '34°')],
      );

      // And they are the lab's files, where the lab looks: C7's layout under `processed/`.
      final root = '${scratch.path}/processed';
      final state = jsonDecode(
        File('$root/.app/state.json').readAsStringSync(),
      );
      expect(state['player_id'], 'aaron-sierra');
      final golfers = Directory(
        '$root/golfers',
      ).listSync().map((entry) => entry.uri.pathSegments.last).toList()..sort();
      expect(golfers, ['aaron-sierra.bag.json', 'aaron-sierra.golfer.json']);
    },
  );
}
