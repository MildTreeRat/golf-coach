import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/src/rust/frb_generated.dart';
import 'package:golf_app/src/rust/third_party/app/api/about.dart';
import 'package:integration_test/integration_test.dart';

/// `app/tool/sim_test.sh` passes this, and a Simulator app can read a host path.
const repo = String.fromEnvironment('GOLF_REPO');

/// A key's value from the Rust that declares it, so this test holds no copy that a version bump
/// would have to find. They are three different numbers today, which is what lets a bridge that
/// crossed two fields fail here.
int declared(String file, String key) {
  final source = File('$repo/crates/contracts/src/$file').readAsStringSync();
  final match = RegExp('pub const $key: i64 = (\\d+);').firstMatch(source);
  expect(match, isNotNull, reason: '$key is not declared in $file');
  return int.parse(match!.group(1)!);
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  testWidgets('about answers the three version keys through the bridge', (
    tester,
  ) async {
    expect(repo, isNotEmpty, reason: 'run this through app/tool/sim_test.sh');

    final view = await about();

    expect(view.analysisVersion, declared('swing.rs', 'ANALYSIS_VERSION'));
    expect(
      view.screenParserVersion,
      declared('shot.rs', 'SCREEN_PARSER_VERSION'),
    );
    expect(view.careerVersion, declared('career.rs', 'CAREER_VERSION'));
    expect(view.device, 'full_swing');
  });
}
