"""Phone upload server — fills the `api/` seam with the upload-ingestion path only.

A phone browser posts a file tagged with a role (face_on / down_the_line /
shot_screen); the body is streamed to disk and handed to `SwingBundleStore`, which
groups it into the right swing. Nothing here triggers pose extraction, OCR, or
analysis — that wiring is a later phase.

`scripts/run_server.py` binds this to 127.0.0.1 only. Phones reach it through Tailscale,
which terminates TLS and proxies to that loopback port (ADR-016) — so the bind address
never widens, whether the phone is on the tailnet (`tailscale serve`) or off it
(`tailscale funnel`). Funnel makes these routes publicly reachable, so every `/api/`
route is gated on a shared token whenever `GOLF_UPLOAD_TOKEN` is set.
"""

from __future__ import annotations

import hashlib
import secrets
import uuid
from collections.abc import Callable
from pathlib import Path

from fastapi import Depends, FastAPI, Header, HTTPException, Query, Request
from fastapi.staticfiles import StaticFiles

from golf_coach.config import settings
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.manifest import EXPECTED_ROLES, Role

_STATIC_DIR = Path(__file__).parent / "static"
_CHUNK_SIZE = 1024 * 1024  # 1 MiB — bounds peak memory regardless of upload size


class _FromSettings:
    """Sentinel for `create_app(token=...)`.

    `None` has to mean "no authentication at all", so it can't double as "look it up".
    Defaulting to this sentinel keeps the lookup fail-closed: a caller that forgets the
    argument still gets whatever GOLF_UPLOAD_TOKEN says, rather than an open endpoint.
    """


_FROM_SETTINGS = _FromSettings()


def _status_message(swing_id: str, status: str, missing: list[Role]) -> str:
    if status == "complete":
        return f"Swing {swing_id}: complete"
    names = ", ".join(role.value.replace("_", "-") for role in missing)
    return f"Swing {swing_id}: waiting on {names}"


def _token_guard(expected: str | None) -> Callable:
    """Gate a route on the shared token, or wave everything through if none is set.

    No token configured means tailnet-only `tailscale serve`, where tailnet membership is
    the access control. Accepts the token in the `X-Upload-Token` header or a `?t=` query
    param: the query param is what makes the initial setup link openable on a phone, the
    header is what the page uses for every request after that.
    """

    async def guard(
        x_upload_token: str | None = Header(default=None),
        t: str | None = Query(default=None, include_in_schema=False),
    ) -> None:
        if expected is None:
            return
        supplied = x_upload_token or t
        # compare_digest needs a str on both sides; the `or ""` keeps a missing token on
        # the same constant-time path as a wrong one.
        if not secrets.compare_digest(supplied or "", expected):
            raise HTTPException(status_code=401, detail="missing or invalid upload token")

    return guard


def create_app(
    *,
    store: SwingBundleStore | None = None,
    token: str | None | _FromSettings = _FROM_SETTINGS,
) -> FastAPI:
    app = FastAPI(title="golf-coach upload")
    bundle_store = store or SwingBundleStore(settings.sessions_dir)
    incoming_dir = bundle_store.root / ".incoming"
    expected = settings.upload_token if isinstance(token, _FromSettings) else token
    # Declared as a route dependency rather than middleware so it resolves *before* the
    # handler touches `request.stream()` — an unauthenticated body never reaches disk.
    guard = [Depends(_token_guard(expected))]

    @app.post("/api/uploads", dependencies=guard)
    async def upload(
        request: Request,
        role: str,
        filename: str = "upload",
        swing_id: str | None = None,
    ) -> dict:
        try:
            parsed_role = Role(role)
        except ValueError:
            raise HTTPException(status_code=400, detail=f"unknown role {role!r}") from None

        incoming_dir.mkdir(parents=True, exist_ok=True)
        tmp_path = incoming_dir / f"{uuid.uuid4().hex}.part"
        digest = hashlib.sha256()
        size = 0
        try:
            with tmp_path.open("wb") as handle:
                async for chunk in request.stream():
                    size += len(chunk)
                    if size > settings.max_upload_bytes:
                        raise HTTPException(status_code=413, detail="upload too large")
                    digest.update(chunk)
                    handle.write(chunk)
        except Exception:
            tmp_path.unlink(missing_ok=True)
            raise

        session_id = bundle_store.current_session_id()
        result = bundle_store.assign_from_path(
            session_id=session_id,
            role=parsed_role,
            tmp_path=tmp_path,
            digest=digest.hexdigest(),
            original_filename=filename,
            content_type=request.headers.get("content-type", "application/octet-stream"),
            size_bytes=size,
            swing_id=swing_id,
        )
        return {
            "session_id": result.session_id,
            "swing_id": result.swing_id,
            "role": result.role.value,
            "status": result.status,
            "missing_roles": [role.value for role in result.missing_roles],
            "deduped": result.deduped,
            "message": _status_message(result.swing_id, result.status, result.missing_roles),
        }

    @app.get("/api/sessions/current", dependencies=guard)
    async def session_current() -> dict:
        return {"session_id": bundle_store.current_session_id()}

    @app.get("/api/sessions/{session_id}", dependencies=guard)
    async def session_detail(session_id: str) -> dict:
        manifests = bundle_store.get_session(session_id)
        return {
            "session_id": session_id,
            "swings": [
                {
                    "swing_id": manifest.swing_id,
                    "status": manifest.status(),
                    "created_at": manifest.created_at.isoformat(),
                    "updated_at": manifest.updated_at.isoformat(),
                    "roles": {
                        role.value: (
                            {
                                "original_filename": manifest.roles[role].original_filename,
                                "received_at": manifest.roles[role].received_at.isoformat(),
                            }
                            if role in manifest.roles
                            else None
                        )
                        for role in EXPECTED_ROLES
                    },
                }
                for manifest in manifests
            ],
        }

    # Deliberately ungated, and mounted last because it catches everything: the page is a
    # role picker and an empty upload form, holding no session data, and gating it would
    # break the `/?t=<token>` link a phone uses to learn the token in the first place.
    app.mount("/", StaticFiles(directory=_STATIC_DIR, html=True), name="static")
    return app


app = create_app()
