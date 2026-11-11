#!/bin/sh
# The whole flow on the Simulator, then its export held on the host. [M38 P28]
#
#   app/tool/whole_flow.sh 2>&1 | tail -40
#
# Step 1 is `integration_test/whole_flow_test.dart` through `sim_test.sh`: from a fresh root, setup, a
# club, the six captures read by the Simulator's iOS Vision, one correction, History, Profile and Export,
# with the zip the share was handed copied to `target/m38/whole-flow.zip`, which must hold the events'
# log since P30b. Then three checks on this Mac,
# each holding the phone's export to an answer it did not compute:
#   - `golf-bay-report truth`: every cell as the reader gave it, against M33's typed truth. A wrong cell
#     on an unflagged photo, or a capture not found, exits 3;
#   - `golf-lab import-export` into empty scratch directories, never `data/` (rule 6), the lab's way in;
#   - `golf-bay-report compare`: the corpus `read_corpus` reads off the export against the one it reads
#     off what the lab wrote. Any difference exits 3.
# `set -e` carries each, so exit 0 means: the flow ran, wrong 0, and the corpora equal. `bars` runs last
# for its numbers alone. Six library photos are no bay session, so it says INVALID by construction, and
# a verdict exits 0 whatever it says (P14 finding 5).
#
# Everything read off the captures is `provisional: pasted copies` (rule 12), and the Simulator's
# Vision, not a phone's.
set -eu

tool_dir=$(cd "$(dirname "$0")" && pwd)
app_dir=$(dirname "$tool_dir")
repo_dir=$(dirname "$app_dir")
. "$tool_dir/env.sh"

zip="$repo_dir/target/m38/whole-flow.zip"
# The golfer the test sets up, whose `player_id` `compare` takes (P15 finding 13).
player=aaron

# A zip left by an earlier run must not pass for this one's, so it goes before the test runs.
rm -f "$zip"
"$tool_dir/sim_test.sh" integration_test/whole_flow_test.dart
if [ ! -f "$zip" ]; then
    echo "whole_flow: the test passed and left no $zip" >&2
    exit 1
fi

scratch=$(mktemp -d "${TMPDIR:-/tmp}/m38-whole-flow.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
trap 'exit 130' INT TERM

cd "$repo_dir"
# Built once, before the first check, so the checks' output is theirs and not the compiler's.
cargo build -q -p app --bin golf-bay-report -p golf-lab --bin golf-lab
bay() { cargo run -q -p app --bin golf-bay-report -- "$@" </dev/null; }

# The events' log travels beside the reads (D16) [M38 P30b], and the lab ignores it as a dotted name.
echo "--- the events' log is in the export"
if ! unzip -l "$zip" | grep -q ' \.app/events\.jsonl$'; then
    echo "whole_flow: $zip holds no .app/events.jsonl" >&2
    exit 1
fi

echo "--- golf-bay-report truth"
bay truth "$zip" \
    --photos spikes/vision-ocr/photos.json \
    --truth spikes/vision-ocr/truth_full_swing.json

echo "--- golf-lab import-export, into empty scratch directories"
cargo run -q -p golf-lab --bin golf-lab -- import-export "$zip" \
    --sessions-dir "$scratch/sessions" \
    --shots-dir "$scratch/shots" \
    --golfers-dir "$scratch/golfers" </dev/null

echo "--- golf-bay-report compare"
bay compare "$zip" --sessions-dir "$scratch/sessions" --player "$player"

echo "--- golf-bay-report bars, for its numbers: six library photos are no session"
bay bars "$zip" --source all

echo "whole_flow: held. The flow ran, truth held, and the corpora are equal."
