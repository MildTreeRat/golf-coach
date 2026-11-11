#!/bin/sh
# One integration test on the iOS Simulator, with the repo's path handed to it.
#
#   app/tool/sim_test.sh integration_test/smoke_test.dart 2>&1 | tail -20
#
# The path is relative to `app/`, where `flutter test` runs. A Simulator app reads and writes host
# paths, so a test takes GOLF_REPO from `--dart-define` and leaves its results under `target/m38/`
# (the plan's spike section), which this makes.
#
# Booted here and shut down on every way out, a failing test and an interrupt included, because a
# run nobody is watching should not leave a Simulator running for the next one to inherit. Every
# command is held off stdin: a phase runs this with no terminal, and a tool that stops to ask would
# hang until the call's timeout rather than fail.
set -eu

if [ $# -ne 1 ]; then
    echo "usage: app/tool/sim_test.sh integration_test/<file>.dart" >&2
    exit 2
fi
test_file=$1

tool_dir=$(cd "$(dirname "$0")" && pwd)
app_dir=$(dirname "$tool_dir")
repo_dir=$(dirname "$app_dir")
. "$tool_dir/env.sh"

# The name and an opening bracket, so "iPhone 17 Pro" is not taken for it.
device="iPhone 17"
udid=$(xcrun simctl list devices available </dev/null |
    sed -n "s/^ *$device (\([0-9A-F-]*\)) .*/\1/p" | head -n 1)
if [ -z "$udid" ]; then
    echo "sim_test: no available Simulator named \"$device\"" >&2
    exit 1
fi

shut_down() {
    xcrun simctl shutdown "$udid" </dev/null >/dev/null 2>&1 || true
}
trap shut_down EXIT
trap 'exit 130' INT TERM

# `boot` refuses a device that is already up, which is not a failure, and `bootstatus -b` is what
# says it is ready either way.
xcrun simctl boot "$udid" </dev/null >/dev/null 2>&1 || true
xcrun simctl bootstatus "$udid" -b </dev/null >/dev/null

mkdir -p "$repo_dir/target/m38"
cd "$app_dir"
flutter test "$test_file" -d "$udid" --dart-define=GOLF_REPO="$repo_dir" </dev/null
