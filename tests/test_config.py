"""The secret-handling guarantees of `config.py`, pinned as tests rather than comments (ADR-019).

`SecretStr` is only a real defence while two things stay true: the secrets keep that type, and
the set of places allowed to unwrap them stays small enough that a reader can check it. Neither
is enforced by anything else — mypy accepts `str` and `SecretStr` equally, and
`.get_secret_value()` is a public method any module may call.

The masking matters because this repo is public and its secrets sit in plaintext in a gitignored
`.env`; OS keychains were considered for the storage side and declined for platform lock-in
(ADR-019). Masking is what stops a stray `print(settings)`, a `model_dump()` folded into a log
line, or an exception traceback from turning that local-only plaintext into something that
travels.
"""

from __future__ import annotations

import ast
import json
from pathlib import Path

from pydantic import SecretStr

from golf_coach.config import Settings

REPO_ROOT = Path(__file__).resolve().parents[1]

#: The only files allowed to turn a `SecretStr` back into a `str`. Each is a boundary where an
#: outside caller needs a plain string and cannot be handed a wrapper: the Anthropic SDK, the
#: constant-time token comparison, and the operator-facing setup link.
SANCTIONED_UNWRAP_SITES = {
    "scripts/run_server.py",
    "src/golf_coach/api/app.py",
    "src/golf_coach/api/pipeline.py",
}


def _files_calling_get_secret_value() -> set[str]:
    """Real attribute accesses only, via `ast` rather than a text search.

    `config.py` names the method in its module docstring, and explaining the mechanism is that
    docstring's whole job — a grep-based pin would force the prose to avoid the word it exists
    to describe. Parsing also means a mention in a comment or a test fixture cannot trip the pin.
    """
    found: set[str] = set()
    for directory in ("src", "scripts"):
        for path in sorted((REPO_ROOT / directory).rglob("*.py")):
            tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
            if any(
                isinstance(node, ast.Attribute) and node.attr == "get_secret_value"
                for node in ast.walk(tree)
            ):
                found.add(path.relative_to(REPO_ROOT).as_posix())
    return found


def test_the_secrets_are_secretstr_not_str() -> None:
    """The annotation is the whole mechanism: as `str`, every other test here still passes."""
    settings = Settings(upload_token="token", anthropic_api_key="key")

    assert isinstance(settings.upload_token, SecretStr)
    assert isinstance(settings.anthropic_api_key, SecretStr)


def test_no_rendering_of_the_settings_leaks_a_secret() -> None:
    """The four ways a secret reaches a log by accident, plus the two field-level ones."""
    sentinel = "SENTINEL-b7f3a1e9"
    settings = Settings(upload_token=sentinel, anthropic_api_key=sentinel)

    renderings = {
        "repr(settings)": repr(settings),
        "str(settings)": str(settings),
        "f'{settings}'": f"{settings}",
        "model_dump()": json.dumps(settings.model_dump(mode="json"), default=str),
        "repr(field)": repr(settings.anthropic_api_key),
        "f'{field}'": f"{settings.upload_token}",
    }

    for how, rendered in renderings.items():
        assert sentinel not in rendered, (
            f"{how} rendered a secret in clear text. A `print(settings)` or a logged traceback "
            "would put it in scrollback, and the secrets here live in plaintext on disk already "
            "— masking in output is the defence that is left"
        )


def test_the_secret_survives_the_masking() -> None:
    """Masking everything is easy; the value still has to arrive intact at the one caller."""
    settings = Settings(anthropic_api_key="sk-ant-not-a-real-key")

    assert settings.anthropic_api_key is not None
    assert settings.anthropic_api_key.get_secret_value() == "sk-ant-not-a-real-key"


def test_only_the_sanctioned_sites_unwrap_a_secret() -> None:
    """A fourth `get_secret_value` should be a decision, not a convenience.

    This is the half that keeps the masking meaningful. Masking a value nobody unwraps is easy;
    the live risk is `.get_secret_value()` becoming the reflexive way to make a type error go
    away, one call site at a time, until the wrapper protects nothing.
    """
    found = _files_calling_get_secret_value()

    assert found == SANCTIONED_UNWRAP_SITES, (
        "the set of files unwrapping a secret changed.\n"
        f"  added:   {sorted(found - SANCTIONED_UNWRAP_SITES)}\n"
        f"  removed: {sorted(SANCTIONED_UNWRAP_SITES - found)}\n"
        "If the new site is right, add it to SANCTIONED_UNWRAP_SITES and record why in ADR-019 "
        "— the point of this pin is that widening the surface is deliberate."
    )


def test_an_unset_secret_is_none_rather_than_an_empty_secretstr() -> None:
    """`scripts/run_server.py` refuses a non-loopback bind on `not settings.upload_token`.

    That is a security refusal, so the falsiness it depends on is pinned here rather than left to
    read as an accident. `SecretStr` implements `__len__`, so an empty one is falsy too — both
    states have to fail the check.
    """
    assert Settings(upload_token=None).upload_token is None
    assert not Settings(upload_token=None).upload_token
    assert not Settings(upload_token="").upload_token
    assert Settings(upload_token="set").upload_token
