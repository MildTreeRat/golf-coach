// Tests print their reports by design; this is the one place that does it for a phone.
// ignore_for_file: avoid_print

import 'dart:io';

// Where an integration test finds the repo's inputs and leaves its outputs, and what it calls the host it
// ran on. [M38 P30]
//
// On the Simulator, `app/tool/sim_test.sh` passes the host repo as `GOLF_REPO`, because a Simulator app
// reads and writes host paths. A phone cannot see the Mac's disk, and its container's absolute path
// changes on every install, so no path to it can be compiled in. So `app/tool/device_test.sh` passes
// `@tmp` and copies the inputs into the container's `tmp/m38-repo/` before the run. The app's own data
// root, `Documents/processed/`, is never in reach of a test, which keeps a golfer's shots out of one.
//
// **On a phone the files a test writes are lost**: `flutter drive` uninstalls the app when its run ends,
// container and all (P30 finding 10). So there the console is the only record, and [printReport] is how a
// report reaches it whole.

const _given = String.fromEnvironment('GOLF_REPO');

/// Whether this run is on a phone, through `device_test.sh`.
const onDevice = _given == '@tmp';

/// The repo's root, or on a phone the copy of its inputs inside the app's own `tmp/`. `systemTemp` is the
/// container's `tmp/` there, as it is the scratch every test already makes.
final String repo = onDevice ? '${Directory.systemTemp.path}/m38-repo' : _given;

/// What an output file is named for and a heading calls the host.
const hostTag = onDevice ? 'device' : 'sim';
const hostName = onDevice ? 'iPhone' : 'Simulator';

/// Prints [report] one line per call. A phone's `print` goes through the system log, which cut P30's first
/// device report off after its eleventh line, a little under 1 KB; one line is far below that.
void printReport(Object report) {
  for (final line in '$report'.split('\n')) {
    print(line);
  }
}
