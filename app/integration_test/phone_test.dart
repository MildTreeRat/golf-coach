import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:golf_app/src/phone/phone.dart';
import 'package:integration_test/integration_test.dart';

// The `golf/phone` channel's new calls through the real Swift, as far as a Simulator can take them.
// [M38 P20]
//
// What runs here: `documentsDir` names a directory that is there, `cameraAvailable` answers, and
// `share` refuses a missing file and a call with no path before presenting anything, with its code
// and sentence. **What does not**: the camera, the library picker and the share sheet themselves, since
// each waits for a finger. They are unproven until P30. `PhotoTime`, the photo's time, is held on macOS
// by `app/tool/reader_check.sh`, since it imports no UIKit (C6).
//
// **`takePhoto` is not called, because this Simulator has a camera.** The plan expected
// `cameraAvailable` to be false here, and on the iOS 26.2 Simulator it is true: `takePhoto` raises the
// system's camera prompt and waits for a finger on it (the plan's P20 findings). The prompt cannot be
// answered from outside: a `simctl privacy grant` before the run did not survive the test's install. So
// `no_camera`, the one refusal `takePhoto` gives before asking, is reached by no host here.

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  const phone = ChannelPhone();

  testWidgets(
    'documentsDir names the container\'s Documents, and it is there',
    (tester) async {
      final path = await phone.documentsDir();

      expect(path, endsWith('/Documents'));
      expect(await Directory(path).exists(), isTrue);
    },
  );

  testWidgets('cameraAvailable answers UIKit\'s bool through the channel', (
    tester,
  ) async {
    // Measured and not asserted: the plan expected false and this runtime says true, and which a
    // later runtime says is UIKit's business. That the channel answered a bool at all is the wiring.
    final available = await phone.cameraAvailable();
    debugPrint('cameraAvailable on this host: $available');
  });

  testWidgets('share refuses a file that is not there, and names it', (
    tester,
  ) async {
    final scratch = await Directory.systemTemp.createTemp('m38-phone-');
    addTearDown(() => scratch.delete(recursive: true));
    final missing = '${scratch.path}/export.zip';

    await expectLater(
      phone.share(missing),
      throwsA(
        isA<PlatformException>()
            .having((error) => error.code, 'code', 'no_file')
            .having((error) => error.message, 'message', contains(missing)),
      ),
    );
  });

  testWidgets('share with no path is refused as bad arguments', (tester) async {
    await expectLater(
      ChannelPhone.channel.invokeMethod<bool>('share'),
      throwsA(
        isA<PlatformException>().having(
          (error) => error.code,
          'code',
          'bad_arguments',
        ),
      ),
    );
  });
}
