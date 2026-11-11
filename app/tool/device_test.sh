#!/bin/sh
# One integration test on the paired iPhone: sim_test.sh's twin for a device. [M38 P30]
#
#   app/tool/device_test.sh integration_test/<file>.dart
#
# **What a run costs: the app is uninstalled, and the phone's trust in the developer with it.**
# `flutter test -d <phone>` and `flutter drive` both uninstall the app when their run ends, which deletes
# its container and every shot in it (P30 findings 6 and 10). `drive` is used because it is the one of
# the two that `--keep-app-running` might change, and that is untried. So this script pulls the app's
# `Documents/` to `target/m38/device-backup-<time>/` before the run, and after it hands that pull to
# `device_restore.sh`, which reinstalls the release build, copies the data back and holds the phone to
# it byte for byte. Only the trust is left to a person: Settings → General → VPN & Device Management.
# So do not run this the day of a bay session, and never on a phone whose shots are not yet exported.
#
# Why it stages files: a Simulator app reads and writes host paths, and a phone cannot. The container's
# absolute path also changes on every install, so no path to it can be compiled in. So the test is built
# with GOLF_REPO=@tmp, and `integration_test/test_host.dart` finds the inputs in the container's
# `tmp/m38-repo/`. Only what the capture tests read goes there: M33's photos.json, its truth, the
# captures and C0's committed boxes, about 11 MB. What a test writes goes with the container, so the
# run's console, kept in `target/m38/device-<test>.log`, is its only record (`printReport`).

set -eu

if [ $# -ne 1 ]; then
    echo "usage: app/tool/device_test.sh integration_test/<file>.dart" >&2
    exit 2
fi
test_file=$1

tool_dir=$(cd "$(dirname "$0")" && pwd)
app_dir=$(dirname "$tool_dir")
repo_dir=$(dirname "$app_dir")
. "$tool_dir/env.sh"

bundle=dev.golfcoach.lab
scratch=$(mktemp -d "${TMPDIR:-/tmp}/m38-device-test.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
trap 'exit 130' INT TERM

# The first device CoreDevice lists. `devicectl` takes its identifier and `flutter` its UDID, and the JSON
# file is the only output devicectl calls stable.
xcrun devicectl list devices --quiet --json-output "$scratch/devices.json" </dev/null >/dev/null
device=$(plutil -extract result.devices.0.identifier raw "$scratch/devices.json" 2>/dev/null || true)
udid=$(plutil -extract result.devices.0.hardwareProperties.udid raw "$scratch/devices.json" 2>/dev/null || true)
if [ -z "$device" ] || [ -z "$udid" ]; then
    echo "device_test: no device is paired with this Mac" >&2
    exit 1
fi

copy() {
    xcrun devicectl device copy "$@" --device "$device" \
        --domain-type appDataContainer --domain-identifier "$bundle" --quiet </dev/null
}
installed() {
    xcrun devicectl device info apps --device "$device" --bundle-id "$bundle" --quiet \
        --json-output "$scratch/apps.json" </dev/null >/dev/null 2>&1 &&
        plutil -extract result.apps.0.bundleIdentifier raw "$scratch/apps.json" >/dev/null 2>&1
}

if ! installed; then
    echo "device_test: $bundle is not installed, so there is no container to stage into" >&2
    exit 1
fi
backup="$repo_dir/target/m38/device-backup-$(date '+%Y%m%d-%H%M%S')"
mkdir -p "$backup"
copy from --source Documents --destination "$backup"
echo "device_test: the app's Documents pulled to ${backup#"$repo_dir"/}"

stage="$scratch/m38-repo"
spike="$repo_dir/spikes/vision-ocr"
mkdir -p "$stage/spikes/vision-ocr/boxes/C0" "$stage/target/m38"
cp "$spike/photos.json" "$spike/truth_full_swing.json" "$stage/spikes/vision-ocr/"
cp -R "$spike/captures" "$stage/spikes/vision-ocr/"
cp -R "$spike/boxes/C0/raw" "$stage/spikes/vision-ocr/boxes/C0/"
# An empty directory does not travel, and the tests write into this one.
touch "$stage/target/m38/.staged"
copy to --source "$stage" --destination tmp/m38-repo

name=$(basename "$test_file" .dart)
log="$repo_dir/target/m38/device-$name.log"
cd "$app_dir"
status=0
flutter drive --driver=test_driver/integration_test.dart --target="$test_file" \
    -d "$udid" --dart-define=GOLF_REPO=@tmp </dev/null 2>&1 | tee "$log" || true
grep -q '^All tests passed' "$log" || status=1
echo "device_test: the run's console is ${log#"$repo_dir"/}"

if installed; then
    # Not seen yet: a run that left the app. Its outputs are still in the container, so take them.
    mkdir -p "$repo_dir/target/m38/device"
    copy from --source tmp/m38-repo/target/m38 --destination "$repo_dir/target/m38/device"
    rm -f "$repo_dir/target/m38/device/.staged"
    echo "device_test: the app outlived the run, and its outputs are in target/m38/device/"
fi
"$tool_dir/device_restore.sh" "$backup" || echo "device_test: the restore needs a person, above" >&2
exit "$status"
