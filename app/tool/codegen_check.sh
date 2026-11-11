#!/bin/sh
# Regenerates the bridge, and fails if the generated files are not settled and compiling.
#
#   app/tool/codegen_check.sh
#
# Run by every phase that changes `crates/app/src/api/`, so the first generate here is that change
# landing, and differs as it should. What this holds is the second. It has to change nothing:
# generated code is committed, so a generator that moved its output between runs would put noise in
# every diff from here on.
#
# Three more things the generator's own exit code does not say, each measured when this was written:
#   - it can say "Done!" and exit 0 over a bridge that does not compile, sometimes printing the
#     compiler's errors and sometimes not, so a shape that binds and does not compile has to be found
#     by compiling;
#   - it binds whatever `rust_input` reaches, so a wire function outside the two `api` modules means
#     C1's line has been widened and a helper the crate keeps out of `api` has become callable from
#     Dart. This is asked before the compile, since a widened line usually stops compiling too, and
#     the compiler's 133 errors do not say why;
#   - it leaves a removed module's Dart behind, which is not checked here, since `flutter analyze`
#     fails on it, naming the function that is gone.
set -eu

tool_dir=$(cd "$(dirname "$0")" && pwd)
app_dir=$(dirname "$tool_dir")
. "$tool_dir/env.sh"
cd "$app_dir"

# Sorted by the C locale so the listing cannot move with the shell's language.
fingerprint() {
    find lib/src/rust rust/src/frb_generated.rs -type f | LC_ALL=C sort |
        while IFS= read -r file; do shasum -a 256 "$file"; done
}

generate() {
    if ! flutter_rust_bridge_codegen generate </dev/null >"$1" 2>&1; then
        tail -n 40 "$1" >&2
        echo "codegen_check: the generator failed" >&2
        exit 1
    fi
}

log=$(mktemp)
trap 'rm -f "$log"' EXIT

generate "$log"
first=$(fingerprint)
generate "$log"
second=$(fingerprint)

if [ "$first" != "$second" ]; then
    echo "codegen_check: a generated file differs between two runs:" >&2
    first_file=$(mktemp)
    printf '%s\n' "$first" >"$first_file"
    printf '%s\n' "$second" | diff "$first_file" - >&2 || true
    rm -f "$first_file"
    exit 1
fi

# `wire__<crate>__<module path>__<function>_impl` for each thing bound.
outside=$(grep -o 'fn wire__[A-Za-z0-9_]*_impl' rust/src/frb_generated.rs |
    grep -v -e 'fn wire__app__api__' -e 'fn wire__crate__api__' || true)
if [ -n "$outside" ]; then
    echo "codegen_check: bound from outside the two api modules, so rust_input reads wider than C1:" >&2
    printf '%s\n' "$outside" >&2
    exit 1
fi

if ! (cd rust && cargo check --quiet </dev/null) >"$log" 2>&1; then
    tail -n 40 "$log" >&2
    echo "codegen_check: the generated bridge does not compile" >&2
    exit 1
fi

bound=$(grep -c 'fn wire__[A-Za-z0-9_]*_impl' rust/src/frb_generated.rs)
echo "codegen_check: settled, compiling, $bound functions bound, all from the two api modules"
