#!/bin/sh
# Holds the app's screen reader to M33's committed boxes, and its photo times to C9, on this Mac.
#
#   app/tool/reader_check.sh
#
# Compiles `app/ios/Runner/ScreenReader.swift` and `PhotoTime.swift`, the files the app ships, for
# macOS beside `reader_main.swift`, and runs it. It can, because neither imports UIKit or Flutter (the
# plan's C6). Run by every phase touching either file. `PhotoSource.swift` imports UIKit, so it is not
# here, and what it does on a phone is unproven until P30.
#
# Into `target/m38/`, which is gitignored with the rest of `target/`, as is the scratch the
# orientation check writes its JPEGs to. `-parse-as-library` because `reader_main.swift` has an
# `@main` and not top-level code, which swiftc allows only in a file named `main.swift`. Swift 5 as
# the Runner target is, and `-O` as the spike was built, so that a check passing here is the reader
# built the way it was measured. No stdin is read.
set -eu

tool_dir=$(cd "$(dirname "$0")" && pwd)
app_dir=$(dirname "$tool_dir")
repo_dir=$(dirname "$app_dir")
out_dir="$repo_dir/target/m38"
mkdir -p "$out_dir"

xcrun --sdk macosx swiftc -swift-version 5 -O -parse-as-library \
    "$app_dir/ios/Runner/ScreenReader.swift" \
    "$app_dir/ios/Runner/PhotoTime.swift" \
    "$tool_dir/reader_main.swift" \
    -o "$out_dir/reader-check" </dev/null

"$out_dir/reader-check" "$repo_dir" "$out_dir/reader-check-scratch" </dev/null
