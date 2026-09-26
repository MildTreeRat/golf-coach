"""The seam between Python and the Rust detector. [M20 P6]

**What is and is not tested here.** The *detector* is not: it lives in `crates/trigger` and is
pinned by `spec/vectors/audio/` — 30 vectors, one per stored clip, recorded by the Python
reference this file's predecessor tested and verified against the `{role}.audio.json` the pipeline
had already written. `cargo test` runs those. Re-asserting the algorithm from Python would be a
second, weaker copy of that suite.

What *is* tested is the adapter: finding the binary, handing it a waveform, parsing what comes
back, and failing in the one way `api/pipeline.py` knows how to degrade from.

This file replaces `tests/audio/test_impact.py`, whose fixtures came from a **seeded numpy RNG** —
which is precisely why `docs/CONFORMANCE.md` §5 had to name `audio/impact.py` end to end as
uncovered: a seeded generator does not reproduce in another language, so there was no oracle a
port could be held to. Everything generated here is deterministic and dependency-free.
"""

from __future__ import annotations

import math
import re
import subprocess
from array import array
from pathlib import Path

import pytest

from golf_coach.analysis import phases
from golf_coach.audio.trigger import TriggerUnavailable, binary, detect_strikes

_RATE = 48_000


def _built() -> bool:
    try:
        binary()
    except TriggerUnavailable:
        return False
    return True


needs_binary = pytest.mark.skipif(
    not _built(), reason="golf-trigger is not built — run `cargo build --release`"
)


def _bay(seconds: float, strikes_at: tuple[float, ...]) -> array:
    """Room tone with broadband cracks dropped into it, from arithmetic and nothing else."""
    n = int(seconds * _RATE)
    samples = array("h", bytes(2 * n))
    for i in range(n):
        t = i / _RATE
        hum = int(180 * math.sin(t * 120 * math.tau))
        samples[i] = hum + (i * 7919) % 211 - 105
    for at in strikes_at:
        start = int(at * _RATE)
        for k in range(_RATE // 100):
            if start + k >= n:
                break
            decay = math.exp(-k / 300)
            click = (((start + k) * 31) % 2003 - 1001) * 12 * decay
            samples[start + k] = max(-32_768, min(32_767, int(samples[start + k] + click)))
    return samples


@needs_binary
def test_a_crack_in_room_tone_is_found_where_it_happened() -> None:
    strikes = detect_strikes(_bay(4.0, (2.0,)), _RATE)

    assert strikes, "a broadband transient should be detected"
    loudest = strikes[0]
    assert abs(loudest.sample / _RATE - 2.0) < 0.05
    assert 0.0 <= loudest.confidence <= 1.0
    assert loudest.prominence > 0.0


@needs_binary
def test_a_frame_is_never_invented() -> None:
    """The detector has never seen the video, so it cannot know the fps (`contracts/audio.py`).

    ADR-010 §2 at the narrowest scope it applies: a strike whose frame is unknown is not frame 0,
    and `api/pipeline.py::_frames_derived` is what fills it in once an fps is known.
    """
    assert all(strike.frame is None for strike in detect_strikes(_bay(4.0, (2.0,)), _RATE))


@needs_binary
def test_digital_silence_is_a_result_and_not_a_guess() -> None:
    """No floor to measure against, so nothing is reported — ADR-013, and an empty list is a fact.

    The same answer a rehearsal swing gets, and the signal M11 P5 reads: the file existing with no
    strikes in it means the detector ran and heard nothing, where the file's absence means nobody
    ever listened.
    """
    assert detect_strikes(array("h", bytes(2 * _RATE * 4)), _RATE) == []


@needs_binary
def test_a_plain_sequence_is_accepted_like_an_array() -> None:
    """`api/pipeline.py` hands over an `array` and the corpus tools hand over lists."""
    samples = _bay(3.0, (1.5,))
    assert detect_strikes(list(samples), _RATE) == detect_strikes(samples, _RATE)


def test_a_rate_that_cannot_be_a_rate_is_refused() -> None:
    with pytest.raises(ValueError, match="rate must be positive"):
        detect_strikes(array("h", [0, 0, 0]), 0)


def test_a_missing_binary_is_reported_as_unavailable_and_not_as_a_crash(monkeypatch) -> None:
    """The one failure `api/pipeline.py` knows how to degrade from.

    It keeps an older stored detection when there is one and writes a note when there is not; what
    it must never do is fall back to the pose impact silently, which ran 5-7 frames early on four
    of this corpus's bundles.
    """
    monkeypatch.setattr("golf_coach.audio.trigger.shutil.which", lambda _: None)
    monkeypatch.setattr("golf_coach.audio.trigger._BUILT", Path("nowhere") / "golf-trigger")

    with pytest.raises(TriggerUnavailable, match="cargo build --release"):
        binary()


def test_a_detector_that_fails_carries_its_stderr_out(monkeypatch) -> None:
    """A non-zero exit is not a silent empty list.

    An empty strike list is a *measurement* — the detector ran and the clip was quiet — so a
    crashed detector must not be able to produce one. The two would be indistinguishable on disk.
    """

    def blow_up(*_args, **_kwargs):
        raise subprocess.CalledProcessError(1, "golf-trigger", stderr=b"unsupported sample rate")

    monkeypatch.setattr("golf_coach.audio.trigger.subprocess.run", blow_up)
    monkeypatch.setattr("golf_coach.audio.trigger.shutil.which", lambda _: "golf-trigger")

    with pytest.raises(TriggerUnavailable, match="unsupported sample rate"):
        detect_strikes(array("h", [0, 1, 2]), _RATE)


# The cutting rules the Rust side implements are not its own numbers: `crates/trigger/src/clip.rs`
# derives them from `analysis/phases.py::window_around`, which decides how much footage a swing
# needs before a checkpoint can be scored. Two languages cannot share a constant, so the seam is
# pinned from this side instead — these tests are what fails if `phases.py` moves and the cutter
# does not.
_CLIP_RS = Path(__file__).resolve().parents[2] / "crates/trigger/src/clip.rs"


def _rust_const(name: str) -> float:
    """The value of one `const NAME: f64 = <literal>;` in `clip.rs`."""
    pattern = rf"^(?:pub )?const {name}: f64 = ([0-9.]+);"
    match = re.search(pattern, _CLIP_RS.read_text("utf-8"), re.M)
    assert match, f"{name} is no longer a plain literal in clip.rs"
    return float(match.group(1))


def test_the_cutter_reads_the_window_constants_this_repo_actually_uses() -> None:
    """The three numbers `clip.rs` copies across the language boundary."""
    assert _rust_const("LONGEST_DOWNSWING_S") == phases.POSSIBLE_DOWNSWING_S[1]
    assert _rust_const("WINDOW_LEAD_DOWNSWINGS") == phases._WINDOW_LEAD
    assert _rust_const("WINDOW_TRAIL_DOWNSWINGS") == phases._WINDOW_TRAIL


def test_the_pre_roll_still_covers_the_longest_window_the_analysis_will_ask_for() -> None:
    """The margin that is only 0.15 s wide, checked rather than remembered.

    `window_around` keeps `[top - max(5*dn, 1.5 s), impact + 3*dn)` and the worst case is the
    slowest downswing `select_swing` admits at all. Widen that band, raise `_WINDOW_LEAD`, or
    raise the address floor past `5 * dn`, and the pre-roll stops covering it — at which point
    every live clip is cut too short at the front and the address-dependent checkpoints go with
    it.
    """
    downswing = phases.POSSIBLE_DOWNSWING_S[1]
    lead = downswing + max(phases._WINDOW_LEAD * downswing, phases._MIN_ADDRESS_LEAD_S)
    trail = phases._WINDOW_TRAIL * downswing

    assert lead <= _rust_const("PRE_ROLL_S"), "the live pre-roll no longer covers the window"
    assert trail <= _rust_const("POST_ROLL_S"), "the live post-roll no longer covers the window"
