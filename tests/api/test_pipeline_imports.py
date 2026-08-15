"""The extras boundary ADR-008 draws, pinned as a test rather than a comment.

`scripts/analyze_bundle.py` imports `golf_coach.api.pipeline` and runs on a `vision`-only
install. That works only as long as importing the pipeline does not drag in the web framework —
so `api/__init__.py` stays docstring-only and `pipeline.py` imports no fastapi, directly or
transitively. Both are easy to break by adding one convenient import.

The same holds for the `llm` extra since M6: `pipeline.py` imports `feedback.coach`, which is
allowed to *use* `anthropic` but not to import it at module scope. A top-level import there would
make the whole CLI unrunnable without a dependency it needs only when a key is configured.
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
    """
    modules = (
        "golf_coach.pose.estimator",
        "golf_coach.pose.overlay",
        "golf_coach.pose.side_by_side",
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
