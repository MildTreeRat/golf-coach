// `flutter drive`'s host side, for running an integration test on a phone. [M38 P30]
//
// Why `drive` and not `flutter test -d <phone>`: `flutter test` uninstalls the app when its run ends,
// and that deletes the app's container, a golfer's shots with it (P30 finding 6). `drive` stops the app
// and leaves it installed. This is the package's own driver, unchanged. The Simulator keeps
// `app/tool/sim_test.sh`, which has no container to lose.
import 'package:integration_test/integration_test_driver.dart';

Future<void> main() => integrationDriver();
