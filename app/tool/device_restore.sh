#!/bin/sh
# Puts the release app back on the paired iPhone and, given a pull of its `Documents/`, the data with it.
# [M38 P30]
#
#   app/tool/device_restore.sh [target/m38/device-backup-<time>]
#
# Why it exists: `flutter test` and `flutter drive` both uninstall the app from a phone when their run
# ends, which deletes its container and every shot in it (P30 findings 6 and 10), and a debug build left
# behind will not open from the home screen (C20). So this builds the release app and installs it over
# whatever is there, which keeps a container that still exists (P30 finding 8). With a backup, it copies
# that backup into `Documents/` and holds the phone to it: a second pull must be byte-identical.
#
# What it cannot do: when the uninstall removed the team's last app, iOS dropped the developer trust
# with it, and only a person can give it back, under Settings → General → VPN & Device Management.
# `devicectl device process launch` says which: a launch refused for an untrusted profile.

set -eu

if [ $# -gt 1 ]; then
    echo "usage: app/tool/device_restore.sh [target/m38/device-backup-<time>]" >&2
    exit 2
fi
backup=${1:-}

tool_dir=$(cd "$(dirname "$0")" && pwd)
app_dir=$(dirname "$tool_dir")
repo_dir=$(dirname "$app_dir")
. "$tool_dir/env.sh"

bundle=dev.golfcoach.lab
scratch=$(mktemp -d "${TMPDIR:-/tmp}/m38-device-restore.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
trap 'exit 130' INT TERM

if [ -n "$backup" ] && [ ! -d "$backup/processed" ]; then
    echo "device_restore: $backup holds no processed/, so it is not a pull of the app's Documents" >&2
    exit 1
fi

xcrun devicectl list devices --quiet --json-output "$scratch/devices.json" </dev/null >/dev/null
device=$(plutil -extract result.devices.0.identifier raw "$scratch/devices.json" 2>/dev/null || true)
if [ -z "$device" ]; then
    echo "device_restore: no device is paired with this Mac" >&2
    exit 1
fi

(cd "$app_dir" && flutter build ios --release </dev/null | tail -1)
xcrun devicectl device install app --device "$device" "$app_dir/build/ios/iphoneos/Runner.app" \
    --quiet </dev/null
echo "device_restore: the release build is installed, $(date '+%Y-%m-%d %H:%M:%S %z')"

if [ -n "$backup" ]; then
    copy() {
        xcrun devicectl device copy "$@" --device "$device" \
            --domain-type appDataContainer --domain-identifier "$bundle" --quiet </dev/null
    }
    copy to --source "$backup" --destination Documents
    mkdir -p "$scratch/after"
    copy from --source Documents --destination "$scratch/after"
    diff -r "$backup/processed" "$scratch/after/processed"
    echo "device_restore: Documents/processed/ on the phone is byte-identical to ${backup#"$repo_dir"/}"
fi

if xcrun devicectl device process launch --device "$device" "$bundle" --quiet </dev/null >/dev/null 2>&1; then
    echo "device_restore: the app launched"
else
    echo "device_restore: the app is installed and the phone refused to launch it: trust the developer" \
        "profile under Settings → General → VPN & Device Management" >&2
    exit 3
fi
