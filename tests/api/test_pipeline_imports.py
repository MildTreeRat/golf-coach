"""The extras boundary ADR-008 draws, pinned as a test rather than a comment.

`scripts/analyze_bundle.py` imports `golf_coach.api.pipeline` and runs on a `vision`-only
install. That works only as long as importing the pipeline does not drag in the web framework —
so `api/__init__.py` stays docstring-only and `pipeline.py` imports no fastapi, directly or
transitively. Both are easy to break by adding one convenient import.

The same holds for the `llm` extra since M6: `pipeline.py` imports `feedback.coach`, which is
allowed to *use* `anthropic` but not to import it at module scope. A top-level import there would
make the whole CLI unrunnable without a dependency it needs only when a key is configured. And for
the `audio` extra since M11, where `audio_for` reaches for a decoder and numpy inside the call.

The last test here is a different **kind** of pin and is worth reading as one. Every test above
protects an install that might not have a library; M15 P6's protects a *rule* about a library that
is present — `analysis/` is stdlib and `contracts` only, and a physics integrator is where that
rule gets broken by someone being helpful. ADR-027 §Decision 1 asked for it by name before the
module it guards was written.
"""

from __future__ import annotations

import subprocess
import sys


def test_importing_the_pipeline_does_not_import_fastapi() -> None:
    # A subprocess, because pytest has already imported fastapi for the other API tests and an
    # in-process `sys.modules` check would pass no matter what this module does.
    code = (
        "import golf_coach.api.pipeline, sys;"
        "print('fastapi' in sys.modules or 'starlette' in sys.modules)"
    )

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False", (
        "importing golf_coach.api.pipeline pulled in fastapi/starlette — that breaks "
        "scripts/analyze_bundle.py on an install without the `api` extra"
    )


def test_importing_the_pipeline_does_not_import_anthropic() -> None:
    """M6: the coaching call is lazy, so a `vision`-only install still runs the CLI.

    Meaningful only when `anthropic` is actually installed — with the extra absent the assertion
    passes for the wrong reason, which is the state this test is here to survive.
    """
    code = "import golf_coach.api.pipeline, sys; print('anthropic' in sys.modules)"

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False", (
        "importing golf_coach.api.pipeline pulled in anthropic — that breaks "
        "scripts/analyze_bundle.py on an install without the `llm` extra"
    )


def test_importing_the_pipeline_does_not_import_the_audio_extra() -> None:
    """M11: `audio_for` decodes through ffmpeg and detects with numpy, both behind lazy imports.

    The same boundary as the two above, and the one this milestone is most likely to break: the
    obvious way to write `audio_for` is to import `FfmpegAudioSource` beside `FileVideoSource` at
    the top of the module, which would make the analysis core need a decoder to be *imported* —
    not to be used. `numpy` is checked alongside `imageio_ffmpeg` because `audio/impact.py` is the
    other half of the same lazy block, and it is the half that would go unnoticed: the analysis
    core's stdlib-only rule (ADR-008) is about `analysis/`, so nothing else would complain.
    """
    code = (
        "import golf_coach.api.pipeline, sys;"
        "print(bool({'imageio_ffmpeg', 'numpy'} & sys.modules.keys()))"
    )

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False", (
        "importing golf_coach.api.pipeline pulled in imageio_ffmpeg or numpy — that breaks "
        "scripts/analyze_bundle.py on an install without the `audio` extra"
    )


def test_importing_the_coach_does_not_import_anthropic() -> None:
    code = "import golf_coach.feedback.coach, sys; print('anthropic' in sys.modules)"

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False", (
        "golf_coach.feedback.coach imports anthropic at module scope — it must stay inside "
        "`_sdk()` so the analysis core installs without the `llm` extra"
    )


def test_importing_the_conversation_module_does_not_import_anthropic() -> None:
    """ADR-020's loop, held to the same rule as the coaching call it sits beside.

    `api/app.py` reaches this module for the follow-up route, so a module-scope `import anthropic`
    here would make the whole upload server — ingestion, analysis, results — require the `llm`
    extra to start. It uses `coach._sdk()` instead, which is why the seam is worth a pin of its
    own rather than resting on `coach.py`'s.
    """
    code = "import golf_coach.feedback.conversation, sys; print('anthropic' in sys.modules)"

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False", (
        "golf_coach.feedback.conversation imports anthropic at module scope — it must stay "
        "inside `coach._sdk()` so the upload server starts without the `llm` extra"
    )


def test_the_query_layer_imports_without_either_sdk() -> None:
    """ADR-020 put a second adapter beside `mcp/server.py`, and it imports `anthropic`.

    `query.py` and `career.py` are the half both adapters read through, and they are documented as
    base-install: `tests/mcp/test_query.py` and the career tests run with neither SDK present. One
    convenient `from golf_coach.mcp.runner_tools import ...` at the top of either would make the
    whole reading layer — and every test over it — need the `llm` extra.

    Checked in both directions on purpose. `mcp` is the MCP SDK for the stdio server; `anthropic`
    is the tool runner. Neither belongs here, and importing `golf_coach.mcp.query` must load
    neither, but `golf_coach.mcp.server` legitimately loads the first and `runner_tools` the
    second — so a check that only looked at one of them would pass while the other leaked in.
    """
    for module in ("golf_coach.mcp.query", "golf_coach.mcp.career"):
        code = f"import {module}, sys; print(bool({{'anthropic', 'mcp'}} & sys.modules.keys()))"

        out = subprocess.run(
            [sys.executable, "-c", code], capture_output=True, text=True, check=True
        )

        assert out.stdout.strip() == "False", (
            f"importing {module} pulled in anthropic or the MCP SDK — that module is the half "
            "both adapters read through, and it is documented as running on a base install"
        )


def test_the_pose_modules_import_without_the_vision_stack() -> None:
    """`pose/overlay.py` and `pose/side_by_side.py` both advertise import-cheapness in their own
    docstrings ("imported lazily so importing this module stays cheap"). Nothing held them to it.

    Until this pin they reached `capture.source` at runtime for `Frame` — an annotation-only use,
    but a real import — so the property was underwritten by a `TYPE_CHECKING` block in *another*
    module. Hoisting `import numpy as np` in `capture/source.py` is a one-line edit that
    `docs/CODE_STANDARDS.md` R2 explicitly permits (`capture` may use numpy), and it would have
    silently made all three `pose` modules require the ML stack at import time.

    **`pose/worker.py` is the fourth, and it is here for a different reason** [M23 P3]. That module
    is ADR-033's sidecar, and it is the one module in this repo that is *supposed* to reach
    MediaPipe — the inverse of every pin above. It still has to import cheaply, because `_startup`
    reports an absent `vision` extra as an `unavailable` handshake line carrying the fix, which the
    pool prints. A module-scope `import cv2` would turn that message into an ImportError traceback
    on stderr and an empty protocol channel, which the pool can only report as a worker that died.
    """
    modules = (
        "golf_coach.pose.estimator",
        "golf_coach.pose.overlay",
        "golf_coach.pose.side_by_side",
        "golf_coach.pose.worker",
    )

    for module in modules:
        code = f"import {module}, sys; print(bool({{'numpy', 'cv2'}} & sys.modules.keys()))"

        out = subprocess.run(
            [sys.executable, "-c", code], capture_output=True, text=True, check=True
        )

        assert out.stdout.strip() == "False", (
            f"importing {module} pulled in numpy/cv2 — that module states it stays cheap to "
            "import, and scripts on a base install rely on it"
        )


def test_api_package_init_stays_import_light() -> None:
    code = "import golf_coach.api, sys; print('fastapi' in sys.modules)"

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False"


def test_the_club_lookup_imports_without_anthropic() -> None:
    """M12 P4: `clubs/lookup.py` mirrors `coach.py`'s lazy seam and needs its own pin.

    The mirror is structural, not shared — ADR-008 forbids `clubs/` importing `feedback/` — so this
    module's `_sdk()` is a second copy of the same three lines and nothing about `coach.py` passing
    says anything about this one. `api/app.py` reaches it for the lookup route, so a module-scope
    `import anthropic` here would make the whole upload server require the `llm` extra to start,
    exactly as it would from `conversation.py` above.
    """
    code = "import golf_coach.clubs.lookup, sys; print('anthropic' in sys.modules)"

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False", (
        "golf_coach.clubs.lookup imports anthropic at module scope — it must stay inside "
        "`_sdk()` so the upload server starts without the `llm` extra"
    )


def test_the_flight_integrator_imports_without_numpy_or_scipy() -> None:
    """M15 P6, and ADR-027 §Decision 1 asked for this pin by name before the module existed.

    `analysis/flight.py` is the most numpy-shaped code in the repo: a fixed-step RK4 over a
    six-component state, six-element tuples added componentwise, and a cross product spelled out
    by hand. Every one of those is a line somebody could shorten with an array, and the analysis
    core's stdlib-only rule (ADR-008, `docs/CODE_STANDARDS.md` R2) is the only thing saying they
    should not be. Unlike the pins above this is not about an *extra* being absent — it is about
    a rule that has no other enforcement, because `numpy` is installed here and an import of it
    would break nothing a test would otherwise notice.

    `scipy` is checked beside it because the shortcut is not really `numpy` — it is
    `scipy.integrate.solve_ivp`, which would replace this module's integrator, its solved landing
    and its convergence measurements in one import, and take the per-point spin ratio and clamp
    flag (M15 P16 draws them) with it.

    The benchmark loader is checked too. `flight_model.py` is where the coefficient table lives,
    so it is the second place an array would look natural, and it is upstream of this module —
    an import there would fail this test rather than go unnoticed.

    **M15 P8 added the third and it is the sharpest of them.** `spin_solve.py` is a root find and
    a golden-section search written out by hand, and `scipy.optimize.brentq` and
    `minimize_scalar` are a one-line substitution for each — with the bracket check that
    distinguishes a one-solution band from a two-solution one, and the peak search's own handling
    of a shot whose carry never falls, disappearing into a library that has no opinion about
    either.

    **M15 P9 added the fourth, and it is the one with the weakest pull toward a library.**
    `flight_infer.py` holds no search of its own — it is comparisons over what `spin_solve`
    returned. It is here anyway because it sits *between* two modules that are pinned and imports
    both: a `numpy` arriving through it would be an import the two sharper pins could not see.

    **M15 P11 added the fifth for a reason none of the others has.** `flight_measure.py` is the
    module `analysis/engine.py` imports, so it is the one that carries whatever it pulls in into
    *every* `analyze_swing` call — including the ones on a base install with no extras. The four
    above are reachable only from the CLI until this one exists.
    """
    for module in (
        "golf_coach.analysis.flight",
        "golf_coach.analysis.spin_solve",
        "golf_coach.analysis.flight_infer",
        "golf_coach.analysis.flight_measure",
        "golf_coach.analysis.benchmarks.flight_model",
    ):
        code = f"import {module}, sys; print(bool({{'numpy', 'scipy'}} & sys.modules.keys()))"

        out = subprocess.run(
            [sys.executable, "-c", code], capture_output=True, text=True, check=True
        )

        assert out.stdout.strip() == "False", (
            f"importing {module} pulled in numpy or scipy — the analysis core is stdlib and "
            "contracts only (ADR-008), and a physics integrator is where that rule gets broken"
        )
