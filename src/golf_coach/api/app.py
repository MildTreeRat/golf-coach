"""Phone upload server — ingestion, background analysis, and the results the two produce.

A phone browser posts a file tagged with a role (face_on / down_the_line / shot_screen); the body
is streamed to disk and handed to `SwingBundleStore`, which groups it into the right swing. When
the third role lands and the bundle is complete, `AnalysisWorker` runs the whole pipeline off the
event loop and the results page has something to show (M7 Phase 5).

`scripts/run_server.py` binds this to 127.0.0.1 only. Phones reach it through Tailscale, which
terminates TLS and proxies to that loopback port (ADR-016) — so the bind address never widens,
whether the phone is on the tailnet (`tailscale serve`) or off it (`tailscale funnel`). Funnel
makes these routes publicly reachable, so every `/api/` route is gated on a shared token whenever
`GOLF_UPLOAD_TOKEN` is set.
"""

from __future__ import annotations

import hashlib
import re
import secrets
import uuid
from collections.abc import AsyncIterator, Callable
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import Depends, FastAPI, Header, HTTPException, Query, Request
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.comparison import build_standing
from golf_coach.analysis.dispersion import build_dispersion
from golf_coach.api.state import load_analysis, load_state
from golf_coach.api.worker import AnalysisWorker, should_analyze
from golf_coach.config import settings
from golf_coach.contracts.career import CareerCorpus
from golf_coach.contracts.golfer import Golfer, Handedness, slugify
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.corpus import read_corpus
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import EXPECTED_ROLES, Role
from golf_coach.storage.session_meta import load_session_meta, set_current_player

_STATIC_DIR = Path(__file__).parent / "static"

# Path parameters are joined to filesystem paths, so they are validated rather than trusted.
# A literal `..` segment survives routing (it is one segment, so `{session_id}` matches it) and
# would otherwise walk out of `sessions_dir`.
_SAFE_SEGMENT = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")

# Which files a client may ask for by name. An allowlist rather than a sanitised filename: the
# swing directory also holds the raw uploads and the keypoint dumps, and none of that is served.
_ALIGNED = "aligned"
_VIDEO_ROLES = {role.value for role in (Role.FACE_ON, Role.DOWN_THE_LINE)}


class _FromSettings:
    """Sentinel for `create_app(token=...)`.

    `None` has to mean "no authentication at all", so it can't double as "look it up".
    Defaulting to this sentinel keeps the lookup fail-closed: a caller that forgets the
    argument still gets whatever GOLF_UPLOAD_TOKEN says, rather than an open endpoint.
    """


_FROM_SETTINGS = _FromSettings()


class GolferRequest(BaseModel):
    """Naming a golfer, from the upload page's form.

    `handedness` is optional because the page only asks for it when the typed name is new — a
    returning golfer's is already on file, and re-asking is an opportunity to contradict it.
    Required when the name *is* new: see `_resolve_golfer`.
    """

    name: str
    handedness: Handedness | None = None


def _resolve_golfer(golfers: GolferStore, payload: GolferRequest) -> Golfer:
    """A typed name to a stored golfer, creating one only when the slug is new.

    Refuses to invent a handedness for a golfer nobody stated one for. Defaulting it to
    right-handed would be wrong for one golfer in ten and *silently* wrong: nothing downstream
    can detect it, and the metric it corrupts (`head_hip_offset_impact_norm`, the only signed
    one) would simply read a normal impact position as a gross fault forever after.
    """
    player_id = slugify(payload.name)
    if not player_id:
        raise HTTPException(status_code=400, detail="a golfer needs a name with letters or digits")

    existing = golfers.get(player_id)
    if existing is not None:
        return existing
    if payload.handedness is None:
        raise HTTPException(
            status_code=400,
            detail=f"{payload.name!r} is new here — say whether they swing right- or left-handed",
        )
    return golfers.get_or_create(payload.name, payload.handedness)


def _golfer_payload(golfer: Golfer | None) -> dict | None:
    if golfer is None:
        return None
    return {
        "player_id": golfer.player_id,
        "display_name": golfer.display_name,
        "handedness": golfer.handedness.value,
    }


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
    header is what the page uses for every request after that — and it is also the only way a
    `<video>` element can authenticate, since a media element sends no custom headers.
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


def _safe(segment: str, kind: str) -> str:
    if not _SAFE_SEGMENT.match(segment):
        raise HTTPException(status_code=400, detail=f"invalid {kind}")
    return segment


def _media_type(path: Path) -> str:
    """iPhones upload `.mov`; only the aligned render is reliably `.mp4`."""
    return "video/quicktime" if path.suffix.lower() == ".mov" else "video/mp4"


def _corpus_summary(corpus: CareerCorpus) -> dict:
    """The evidence behind every `n` on the career page, without the swing list.

    A refusal is only actionable beside the reason the `n` is what it is, and for this corpus the
    reason is almost never "you have not swung enough" — it is three re-uploads collapsing into
    one swing, or a swing nobody attributed. So the page gets the counts and the itemised
    exclusions, which is what `CareerCorpus` carries them for.
    """
    return {
        "distinct_swings": corpus.distinct_swings,
        "distinct_shots": corpus.distinct_shots,
        "swing_dirs_seen": corpus.swing_dirs_seen,
        "sessions_scanned": corpus.sessions_scanned,
        "duplicates_collapsed": corpus.duplicates_collapsed,
        "shot_conflicts": corpus.shot_conflicts,
        "outdated_swings": corpus.outdated_swings,
        "unattributed_swings": corpus.unattributed_swings,
        "other_golfers": corpus.other_golfers,
        "analyzed_without_measurements": corpus.analyzed_without_measurements,
        "unknown_sources": corpus.unknown_sources,
        "metric_counts": corpus.metric_counts,
        "excluded": [
            {"ref": swing.ref, "reason": swing.reason.value, "detail": swing.detail}
            for swing in corpus.excluded
        ],
    }


def _analysis_summary(swing_dir: Path) -> dict:
    """The compact analysis block the status panel polls for, every five seconds, per swing."""
    state = load_state(swing_dir)
    if state is None:
        return {"status": "none", "score": None, "headline": None, "has_video": False}
    return {
        "status": state.status,
        "score": state.score,
        "headline": state.headline,
        "error": state.error,
        "partial": state.partial,
        "missing_roles": state.missing_roles,
        "has_video": bool(state.video),
        "video_codec": state.video_codec,
        "completed_at": state.completed_at.isoformat() if state.completed_at else None,
    }


def create_app(
    *,
    store: SwingBundleStore | None = None,
    golfers: GolferStore | None = None,
    token: str | None | _FromSettings = _FROM_SETTINGS,
    worker: AnalysisWorker | None | _FromSettings = _FROM_SETTINGS,
) -> FastAPI:
    bundle_store = store or SwingBundleStore(settings.sessions_dir)
    golfer_store = golfers or GolferStore(settings.golfers_dir)
    incoming_dir = bundle_store.root / ".incoming"
    # Unwrapped here and nowhere deeper: `_token_guard` needs a plain `str` for
    # `compare_digest`, and keeping it ignorant of `SecretStr` is what lets a test pass a
    # literal token. One of the three sanctioned `get_secret_value` sites.
    if isinstance(token, _FromSettings):
        configured = settings.upload_token
        expected = configured.get_secret_value() if configured else None
    else:
        expected = token
    if isinstance(worker, _FromSettings):
        analysis = AnalysisWorker(bundle_store) if settings.analysis_enabled else None
    else:
        analysis = worker

    @asynccontextmanager
    async def lifespan(_: FastAPI) -> AsyncIterator[None]:
        if analysis is not None:
            await analysis.start()
        try:
            yield
        finally:
            if analysis is not None:
                await analysis.stop()

    app = FastAPI(title="golf-coach upload", lifespan=lifespan)
    # Declared as a route dependency rather than middleware so it resolves *before* the
    # handler touches `request.stream()` — an unauthenticated body never reaches disk.
    guard = [Depends(_token_guard(expected))]

    def swing_dir_of(session_id: str, swing_id: str) -> Path:
        return bundle_store.root / session_id / swing_id

    def session_dir_of(session_id: str) -> Path:
        return bundle_store.root / session_id

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
        # Read from the session cursor, never from the request: both phones post into the same
        # swing, and only one of them is being held by someone who knows whose swing it is.
        current_player = load_session_meta(session_dir_of(session_id)).player_id
        result = bundle_store.assign_from_path(
            session_id=session_id,
            role=parsed_role,
            tmp_path=tmp_path,
            digest=digest.hexdigest(),
            original_filename=filename,
            content_type=request.headers.get("content-type", "application/octet-stream"),
            size_bytes=size,
            swing_id=swing_id,
            player_id=current_player,
        )

        # The whole point of Phase 5: the third file landing is what starts the analysis. A
        # partial bundle waits for someone to ask for it explicitly (`/analyze`), because no
        # timeout guesses right about whether a second phone is still walking back.
        queued = False
        if result.status == "complete":
            manifest = bundle_store.get_swing(result.session_id, result.swing_id)
            if analysis is not None and manifest is not None and should_analyze(manifest):
                queued = analysis.submit(result.session_id, result.swing_id)

        return {
            "session_id": result.session_id,
            "swing_id": result.swing_id,
            "role": result.role.value,
            "status": result.status,
            "missing_roles": [role.value for role in result.missing_roles],
            "deduped": result.deduped,
            "queued": queued,
            "player_id": result.player_id,
            "message": _status_message(result.swing_id, result.status, result.missing_roles),
        }

    @app.get("/api/sessions/current", dependencies=guard)
    async def session_current() -> dict:
        return {"session_id": bundle_store.current_session_id()}

    @app.get("/api/golfers", dependencies=guard)
    async def list_golfers() -> dict:
        """Every known golfer — what lets the page tell a returning name from a new one."""
        return {"golfers": [_golfer_payload(g) for g in golfer_store.list_all()]}

    @app.get("/api/sessions/current/golfer", dependencies=guard)
    async def get_current_golfer() -> dict:
        session_id = bundle_store.current_session_id()
        player_id = load_session_meta(session_dir_of(session_id)).player_id
        return {
            "session_id": session_id,
            "player_id": player_id,
            "golfer": _golfer_payload(golfer_store.get(player_id) if player_id else None),
        }

    @app.post("/api/sessions/current/golfer", dependencies=guard)
    async def set_current_golfer(payload: GolferRequest) -> dict:
        """Point the session at a golfer, and adopt whatever arrived before anyone said so.

        The backfill is why uploads are never blocked on this. Files land at the pace of a bay
        session; selecting a golfer reaches back over the ones that are still unlabeled, so
        forgetting until the third swing costs nothing. Already-attributed swings are left alone
        — that is what makes handing the club to someone else a safe thing to do.
        """
        golfer = _resolve_golfer(golfer_store, payload)
        session_id = bundle_store.current_session_id()
        set_current_player(session_dir_of(session_id), golfer.player_id)
        attributed = bundle_store.attribute_unlabeled(session_id, golfer.player_id)
        return {
            "session_id": session_id,
            "player_id": golfer.player_id,
            "golfer": _golfer_payload(golfer),
            "attributed": attributed,
        }

    @app.get("/api/golfers/{player_id}/career", dependencies=guard)
    async def golfer_career(player_id: str) -> dict:
        """One golfer judged against their own history. [Career mode, step 6]

        Serves the three analysis contracts as they are, rather than a flattened view. The MCP
        server flattens the identical data (`mcp/career.py`) because a model reads a flat payload
        better; a page does not need that, and inventing a second shape here would give the two
        surfaces a way to disagree about what career mode says. What is shared is the layer under
        both — `build_baseline`, `build_dispersion`, `build_standing`, all over one `read_corpus`
        so the three provably describe the same swings.

        Everything on this route refuses today. That is the feature, and it is why the page is
        worth building before the bay session rather than after it.
        """
        _safe(player_id, "player id")
        golfer = golfer_store.get(player_id)
        if golfer is None:
            raise HTTPException(status_code=404, detail="no such golfer")

        corpus = read_corpus(bundle_store.root, player_id)
        return {
            "player_id": golfer.player_id,
            "display_name": golfer.display_name,
            "handedness": golfer.handedness.value,
            "corpus": _corpus_summary(corpus),
            "baseline": build_baseline(corpus).model_dump(mode="json"),
            "dispersion": build_dispersion(corpus).model_dump(mode="json"),
            "standing": build_standing(corpus).model_dump(mode="json"),
        }

    # Declared before `/api/sessions/{session_id}` would be ambiguous only if the paths had the
    # same shape; they don't, but `current` must stay above it regardless — FastAPI matches in
    # declaration order and `{session_id}` would happily swallow the literal.
    @app.get("/api/sessions/{session_id}", dependencies=guard)
    async def session_detail(session_id: str) -> dict:
        _safe(session_id, "session id")
        manifests = bundle_store.get_session(session_id)
        return {
            "session_id": session_id,
            "swings": [
                {
                    "swing_id": manifest.swing_id,
                    "status": manifest.status(),
                    "created_at": manifest.created_at.isoformat(),
                    "updated_at": manifest.updated_at.isoformat(),
                    "player_id": manifest.player_id,
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
                    "analysis": _analysis_summary(
                        swing_dir_of(session_id, manifest.swing_id)
                    ),
                }
                for manifest in manifests
            ],
        }

    @app.get("/api/sessions/{session_id}/swings/{swing_id}", dependencies=guard)
    async def swing_detail(session_id: str, swing_id: str) -> dict:
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        manifest = bundle_store.get_swing(session_id, swing_id)
        if manifest is None:
            raise HTTPException(status_code=404, detail="no such swing")
        swing_dir = swing_dir_of(session_id, swing_id)
        return {
            "session_id": session_id,
            "swing_id": swing_id,
            "status": manifest.status(),
            "missing_roles": [role.value for role in manifest.missing_roles()],
            "player_id": manifest.player_id,
            "analysis": _analysis_summary(swing_dir),
            # Null until the worker has finished; the page renders a waiting state for that.
            "result": load_analysis(swing_dir),
        }

    @app.post("/api/sessions/{session_id}/swings/{swing_id}/golfer", dependencies=guard)
    async def set_swing_golfer(session_id: str, swing_id: str, payload: GolferRequest) -> dict:
        """Re-attribute one swing. The repair path for a misfiled golfer.

        Unlike the session cursor, this **overwrites** — it is the explicit human-driven override,
        and the only way to correct a swing that got stamped with the wrong name. It exists
        because uploads are deliberately never blocked on identity, and any rule that attributes
        automatically needs a way to be told it was wrong.
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        golfer = _resolve_golfer(golfer_store, payload)
        manifest = bundle_store.set_player(session_id, swing_id, golfer.player_id)
        if manifest is None:
            raise HTTPException(status_code=404, detail="no such swing")
        return {
            "session_id": session_id,
            "swing_id": swing_id,
            "player_id": golfer.player_id,
            "golfer": _golfer_payload(golfer),
        }

    @app.post("/api/sessions/{session_id}/swings/{swing_id}/analyze", dependencies=guard)
    async def analyze_swing(session_id: str, swing_id: str) -> dict:
        """The "Analyze anyway" override, for a bundle that will never be complete."""
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        manifest = bundle_store.get_swing(session_id, swing_id)
        if manifest is None:
            raise HTTPException(status_code=404, detail="no such swing")
        if analysis is None:
            raise HTTPException(status_code=503, detail="analysis worker is disabled")
        if Role.FACE_ON not in manifest.roles:
            # Every checkpoint is measured from the face-on view, so there is nothing to score
            # without it — better a clear 400 than a result with no checkpoints in it.
            raise HTTPException(
                status_code=400,
                detail="a face-on clip is required — every checkpoint is measured from it",
            )
        analysis.submit(session_id, swing_id, force=True)
        return {
            "session_id": session_id,
            "swing_id": swing_id,
            "queued": True,
            "missing_roles": [role.value for role in manifest.missing_roles()],
        }

    @app.get("/api/sessions/{session_id}/swings/{swing_id}/video/{name}", dependencies=guard)
    async def swing_video(session_id: str, swing_id: str, name: str) -> FileResponse:
        """The aligned render, or one raw view when there was no second angle to align to.

        Range requests matter here rather than being a nicety: iOS Safari will not play a
        `<video>` from a source that cannot serve them. Starlette's FileResponse handles it.
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        swing_dir = swing_dir_of(session_id, swing_id)

        if name == _ALIGNED:
            path = swing_dir / "aligned.mp4"
        elif name in _VIDEO_ROLES:
            manifest = bundle_store.get_swing(session_id, swing_id)
            role_file = manifest.roles.get(Role(name)) if manifest else None
            if role_file is None:
                raise HTTPException(status_code=404, detail="no such video")
            path = swing_dir / role_file.filename
        else:
            raise HTTPException(status_code=404, detail="no such video")

        if not path.is_file():
            raise HTTPException(status_code=404, detail="no such video")
        # No `filename=`: that sets Content-Disposition: attachment, which turns the results
        # page's <video> into a download prompt. The page's download link uses the anchor's
        # own `download` attribute instead, so one route serves both.
        return FileResponse(path, media_type=_media_type(path))

    # Deliberately ungated, and mounted last because it catches everything: the page is a
    # role picker and an empty upload form, holding no session data, and gating it would
    # break the `/?t=<token>` link a phone uses to learn the token in the first place.
    app.mount("/", StaticFiles(directory=_STATIC_DIR, html=True), name="static")
    return app


app = create_app()
