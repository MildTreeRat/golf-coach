# M38's toolchain, for a shell that has none of it: source this, do not run it.
#
#   . app/tool/env.sh
#
# Why a script at all: `cargo` is not on PATH in the shell a phase gets, and each Bash call starts
# fresh, so "how this Mac builds the app" has to be one line that can open any command.
#
# Why these paths and not /opt/homebrew or a system install: /opt/homebrew belongs to another
# account on this Mac, so nothing here may depend on it, and the pins (the plan's C3) are exact, so
# they live beside M33's ffmpeg under a directory that names the version and cannot drift.
#
# Prepended rather than appended so that these win over anything else on PATH. No `set -e` and no
# `exit`: this runs inside the caller's shell, and either would take that shell down with it.

GOLF_COACH_TOOLS="$HOME/.local/share/golf-coach"

export PATH="$GOLF_COACH_TOOLS/flutter-3.47.9/bin:$GOLF_COACH_TOOLS/frb/bin:$HOME/.cargo/bin:$PATH"
