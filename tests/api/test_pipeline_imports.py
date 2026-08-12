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


def test_api_package_init_stays_import_light() -> None:
    code = "import golf_coach.api, sys; print('fastapi' in sys.modules)"

    out = subprocess.run(
        [sys.executable, "-c", code], capture_output=True, text=True, check=True
    )

    assert out.stdout.strip() == "False"
