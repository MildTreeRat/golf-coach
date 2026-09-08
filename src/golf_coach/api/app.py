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
from datetime import UTC, datetime
from pathlib import Path

from fastapi import Depends, FastAPI, Header, HTTPException, Query, Request
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel

from golf_coach.analysis.baseline import build_baseline
from golf_coach.analysis.club_profile import build_bag_profile
from golf_coach.analysis.comparison import build_standing
from golf_coach.analysis.dispersion import build_dispersion
from golf_coach.analysis.tempo_trainer import build_career_tempo
from golf_coach.api.flight_view import DEFAULT_PATH_POINTS, flight_view
from golf_coach.api.pipeline import loft_remedy
from golf_coach.api.state import (
    judged_metrics,
    load_analysis,
    load_state,
    resolve_placements,
    resolve_tempo_plan,
)
from golf_coach.api.worker import AnalysisWorker, should_analyze
from golf_coach.clubs import catalogue
from golf_coach.clubs.lookup import ClubLookupOutcome, look_up_club, look_up_set
from golf_coach.config import settings
from golf_coach.contracts.bag import BagEntry
from golf_coach.contracts.career import CareerCorpus
from golf_coach.contracts.club import ClubId, parse_club
from golf_coach.contracts.club_profile import BagProfile
from golf_coach.contracts.club_spec import (
    ClubSpec,
    SpecProvenance,
    parse_shaft_flex,
    parse_shaft_material,
)
from golf_coach.contracts.conversation import Transcript
from golf_coach.contracts.golfer import Golfer, Handedness, slugify
from golf_coach.contracts.mishit import MishitVerdict
from golf_coach.launch_monitor.screen.store import ShotStore
from golf_coach.storage.bag_store import BagStore
from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.corpus import read_corpus
from golf_coach.storage.flight_inputs import loft_for_club
from golf_coach.storage.golfer_store import GolferStore
from golf_coach.storage.manifest import EXPECTED_ROLES, Role, SwingManifest
from golf_coach.storage.session_meta import (
    load_session_meta,
    set_current_club,
    set_current_player,
)
from golf_coach.storage.transcript_store import (
    latest_for_swing,
    load_transcript,
    new_transcript,
    save_transcript,
    visible_turns,
)

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


class ClubRequest(BaseModel):
    """Naming a club — from the session cursor picker or the per-swing repair control.

    `club` is a plain `str` and not a `ClubId` deliberately. Typing it as the enum would have
    pydantic reject "7 iron" with a 422 before `parse_club` ever ran, and those tolerant spellings
    are the entire reason that parser exists — a bay is not the place to discover that the field
    wanted "7i". Parsing happens in `_resolve_club` instead, so `contracts/club.py` keeps its claim
    to be the only place free text becomes a `ClubId`, and the boundary keeps its own 400.
    """

    club: str


class MishitRequest(BaseModel):
    """The golfer's verdict on a shot, from the per-swing repair control (ADR-028).

    `verdict` is `null` (or absent) to **clear** a verdict back to "no verdict", so the automatic
    rule decides again — `"confirmed"` and `"cleared"` are the only other values. Typed as the enum
    directly, unlike `ClubRequest`: this is a closed three-value vocabulary with no tolerant
    spellings to preserve, so a 422 on anything else is the right boundary.
    """

    verdict: MishitVerdict | None = None


class BagEntryRequest(BaseModel):
    """One club being declared or edited, from the bag page's row form. [M9 P19]

    Every field is optional and defaults exactly as `BagEntry` defaults it, `loft_deg` most
    importantly: a golfer who has never put their irons on a loft machine still has a bag, and a
    form that refused to save without one would mean recording nothing. An omitted field is
    *undeclared* rather than zero, which is why none of these carries a numeric default.

    The club is not in the body. It is the path segment, so the route that edits a 7 iron cannot be
    handed a payload claiming to be a wedge — `Bag._keys_match_entries` catches that mismatch on the
    way to disk, and not putting the club in two places is what stops it ever being asked to.

    `recorded_at` is absent for the reason `bag_store.py` states in its module docstring: the store
    owns the clock and discards whatever a caller passes, so a field for it here would be a value
    the API accepts and silently ignores.

    **`shaft` split into six fields in M12 P2** and this list moved with it, because a body field
    that no longer exists on `BagEntry` is worse than a missing one: pydantic would accept the old
    `shaft` string, the route would drop it on the way to the contract, and the page would show a
    200 for a value that went nowhere. **M12 P5 finished the job**: the head, assembly and
    performance groups are here now, because `POST /api/clubs/lookup` fills them and a spec that
    arrives on the page and cannot be posted back is a form that silently loses what it was shown.

    **The list is hand-written and pinned rather than derived**, and the pin is what makes that
    safe: `test_the_request_carries_every_spec_field` compares this field set against
    `ClubSpec.model_fields` on every run. Deriving it with `create_model` was the alternative and
    it buys less than it looks — three fields differ from the contract on purpose (`club` is the
    path segment, the two vocabularies are `str`, `provenance` is added), so the derivation would
    need three exceptions and would still not be readable at the boundary a 422 comes from.

    **`shaft_material` and `shaft_flex` are `str` here and enums on `ClubSpec`**, for the reason
    `ClubRequest` states two classes up: typing them as the enum makes pydantic reject `"S"` and
    `"Stiff Flex"` with a 422 before `parse_shaft_flex` ever runs, and those tolerant spellings are
    the entire reason that parser exists — a shaft band says `"S"` and nothing else. They are parsed
    in `_resolve_shaft_vocabularies`, which keeps `contracts/club_spec.py` the only place text
    becomes a `ShaftFlex`.

    **`provenance` is typed as the contract model and not as loose text** [M12 P5]. It is echoed
    back verbatim from a lookup response rather than composed by the page, so there are no tolerant
    spellings to accept and a malformed block is a 422 rather than a value quietly dropped on the
    way to disk — which for this field would mean an LLM's answer landing in the bag looking like
    something a person typed (ADR-026 §7). Absent means typed: `set_bag_entry` stamps `"typed"`,
    so the M9 hand-filled form keeps working and still says where its numbers came from.
    """

    # --- Identity. `club` is deliberately absent: it is the path segment (see above).
    make: str = ""
    model: str = ""
    model_year: int | None = None
    head_type: str = ""
    set_composition: str = ""

    # --- Head.
    loft_deg: float | None = None
    lie_deg: float | None = None
    bounce_deg: float | None = None
    grind: str = ""
    offset_mm: float | None = None
    face_angle_deg: float | None = None
    head_weight_g: float | None = None
    adjustable_hosel: bool | None = None
    loft_range_deg: tuple[float, float] | None = None

    # --- Shaft. The two vocabularies are `str` here and parsed at the boundary.
    shaft_model: str = ""
    shaft_material: str = ""
    shaft_flex: str = ""
    shaft_weight_g: float | None = None
    shaft_torque_deg: float | None = None
    shaft_kick_point: str = ""

    # --- Assembly.
    length_in: float | None = None
    swing_weight: str = ""
    total_weight_g: float | None = None
    grip: str = ""

    # --- Head performance.
    cor: float | None = None
    moi_g_cm2: float | None = None
    usga_conforming: bool | None = None

    # --- Where the numbers came from. Not a spec field; see the docstring.
    provenance: SpecProvenance | None = None


class ClubLookupRequest(BaseModel):
    """A make, a model and one or more slots, on their way to being looked up. [M12 P5]

    `club` and `slots` are the same question asked for one club or for a set, and a body may carry
    either. `slots` wins when both are present rather than being a 400, because the two cannot
    contradict each other in a way worth refusing — a set containing the single club is the same
    request, and a bay is not the place to argue about which field the page filled in.

    Every club is a plain `str` for `ClubRequest`'s reason: `_resolve_club` parses it, so "7 iron"
    works here as it does at the bay and `contracts/club.py` stays the only place free text becomes
    a `ClubId`.

    `make` and `model` carry no default that would let a blank one through — they are the catalogue
    key, and the route refuses a lookup without both rather than buying an answer that can never be
    remembered (see `look_up_clubs`).
    """

    make: str = ""
    model: str = ""
    model_year: int | None = None
    club: str = ""
    slots: list[str] = []


class AskRequest(BaseModel):
    """One follow-up question, from the results page's chat panel. [ADR-020]

    `conversation_id` is absent on the first question and echoed back on every one after it, which
    is what makes the panel a conversation rather than a series of unrelated asks. The server
    mints the id; a client-supplied one that names no stored transcript is a 404 rather than a
    silent new conversation, so a typo cannot quietly lose the thread it was meant to continue.
    """

    question: str
    conversation_id: str | None = None


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


def _resolve_shaft_vocabularies(payload: BagEntryRequest) -> dict[str, object]:
    """The saved entry's fields, with the two closed vocabularies parsed — or a 400. [M12 P2]

    `_resolve_club`'s shape applied to the other two vocabularies this repo closed, and refusing for
    the same asymmetry: a rejected flex costs one retype in a form the golfer is looking at, while a
    nudged one records a graphite shaft's weight against a steel label where nothing downstream ever
    flags it (`club_spec.py`'s `parse_shaft_material`).

    An empty string stays absent rather than becoming a refusal. Omitting a field is *undeclared*,
    which is `BagEntryRequest`'s rule for every other field and has to be this one's too — the M9
    row form does not ask for a material at all, so a 400 on blank would make it unsavable.
    """
    fields = payload.model_dump()
    for name, parse in (("shaft_material", parse_shaft_material), ("shaft_flex", parse_shaft_flex)):
        typed = fields[name]
        if not typed:
            fields[name] = None
            continue
        parsed = parse(typed)
        if parsed is None:
            raise HTTPException(
                status_code=400,
                detail=f"{typed!r} is not a {name.replace('_', ' ')} this recognises",
            )
        fields[name] = parsed
    return fields


def _resolve_club(payload: ClubRequest) -> ClubId:
    """Typed text to a club, or a 400 naming what was rejected. The first caller of `parse_club`.

    Never nudged toward a nearest match, for the reason `parse_club`'s own docstring gives: the
    cost is asymmetric. A refused tag costs one retype at the bay; a wrong one pools a wedge's
    carries into a 7 iron's average, where nothing downstream can ever detect it (ADR-024 §5).
    """
    club = parse_club(payload.club)
    if club is None:
        raise HTTPException(
            status_code=400,
            detail=f"{payload.club!r} is not a club in the bag — try '7i', '7 iron' or 'pw'",
        )
    return club


def _resolve_slots(payload: ClubLookupRequest) -> tuple[ClubId, ...]:
    """The slots a lookup was asked for, parsed and deduplicated in the order they arrived. [P5]

    Every string goes through `_resolve_club`, `slots` included, so a set request cannot smuggle in
    a spelling the per-club save route would later refuse — the page would show a filled-in row for
    a club that cannot be saved, which is a worse failure than a 400 on the way in.

    Duplicates collapse here rather than in `look_up_set`, because the catalogue pass below runs
    first and a slot asked for twice would otherwise be looked up once and rendered twice.
    """
    typed = payload.slots or ([payload.club] if payload.club else [])
    if not typed:
        raise HTTPException(
            status_code=400,
            detail="name a club to look up — 'club' for one, or 'slots' for a set",
        )
    return tuple(dict.fromkeys(_resolve_club(ClubRequest(club=text)) for text in typed))


def _look_up_missing(
    make: str, model: str, model_year: int | None, slots: tuple[ClubId, ...]
) -> ClubLookupOutcome:
    """The clubs the catalogue could not answer, in **one** model call. [P5]

    No slots is an empty outcome and no call at all, which is the whole point of the catalogue: a
    bag looked up a second time costs nothing and reaches no network. It is a value rather than a
    `None` so the caller has one shape to read either way.

    One slot goes through `look_up_club` and several through `look_up_set` — the same request
    underneath, kept as two calls because a set is asked for together so its loft, lie and length
    progression stays internally consistent (ADR-026 §6), and a single club has no progression to
    keep. The key is unwrapped here and passed on as a plain `str`: `api/app.py` is one of
    `tests/test_config.py`'s sanctioned sites and ADR-019's surface does not widen for this.
    """
    if not slots:
        return ClubLookupOutcome()
    api_key = (
        settings.anthropic_api_key.get_secret_value() if settings.anthropic_api_key else None
    )
    if len(slots) == 1:
        return look_up_club(
            make, model, model_year, slots[0], llm_model=settings.coaching_model, api_key=api_key
        )
    return look_up_set(
        make, model, model_year, slots, llm_model=settings.coaching_model, api_key=api_key
    )


def _spec_of(entry: BagEntry) -> ClubSpec:
    """The published specification inside a bag entry, with the three declaration fields dropped.

    `BagEntry` **is** a `ClubSpec`, so `catalogue.remember` would accept one as it stands — and
    pydantic would keep the subclass, writing `recorded_at`, `retired_at` and `provenance` into
    every catalogue row. Those are facts about one golfer's declaration and the catalogue is a
    statement about a manufactured object; a row carrying them would say a T150 7 iron was declared
    on a Tuesday, which is true of nobody but the person who declared it.

    Built by walking `ClubSpec.model_fields` rather than listing the two dozen names, for the
    reason `same_club_as` compares by exclusion: a field added to the contract has to travel here
    from the day it is added, and a hand-written list's failure is a spec silently missing it.
    """
    return ClubSpec(**{name: getattr(entry, name) for name in ClubSpec.model_fields})


def _golfer_payload(golfer: Golfer | None) -> dict | None:
    if golfer is None:
        return None
    return {
        "player_id": golfer.player_id,
        "display_name": golfer.display_name,
        "handedness": golfer.handedness.value,
    }


def _club_value(club: ClubId | None) -> str | None:
    """A club as it goes over the wire. `None` stays `None` and means *no club recorded*.

    `ClubId` is a `StrEnum` so the encoder would serialise it unaided; writing `.value` out is the
    same explicitness every `role.value` in this module already has, and it is what makes the
    `None` branch — which is a real state for every swing predating M9 — visible at each site.
    """
    return club.value if club is not None else None


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

    That open-when-unset default is only safe while the token cannot go missing by accident,
    which is a property of `config.py` and not of this file: `env_file` there is absolute, so
    `.env` is found from any working directory. It was relative once, and a server started from
    elsewhere silently read no `.env` at all — which lands here as `expected is None` and opens
    every route. `run_server.py`'s refusal does not cover it, because that only fires on a
    non-loopback bind and a Tailscale-fronted server binds loopback. Hence the pin in
    `tests/test_config.py::test_the_env_file_is_read_from_the_repo_regardless_of_cwd`.
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


def _bag_summary(profile: BagProfile) -> dict:
    """The bag page's payload — a `BagProfile` plus the three things its JSON does not carry.

    The career route above serves its contracts exactly as they are and says why. This one cannot,
    and the difference is `contracts/club_profile.py`'s own doing: `ClubProfile.category`,
    `clubs_used` and `clubs_declared` are plain properties rather than `computed_field`s, which P14
    chose deliberately and named this surface as the projector for.

    `category` is the half that matters. A page deriving it in JavaScript would hold a second copy
    of `CLUB_CATEGORY` in a static file nothing tests — the exact failure `GET /api/clubs` was added
    in M9 P7 to prevent, and quiet in the same way: a club added to `ClubId` would keep working at
    every route here while rendering under the wrong heading at the bay.

    The two lists come over as **counts**. The page has no use for a second copy of each profile and
    every use for "10 clubs, 8 of them hit", so the membership rule stays on the contract where the
    two categories cannot drift apart from the data.
    """
    return {
        "player_id": profile.player_id,
        "clubs": [
            {**club.model_dump(mode="json"), "category": club.category.value}
            for club in profile.clubs
        ],
        "clubs_used": len(profile.clubs_used),
        "clubs_declared": len(profile.clubs_declared),
        "untagged_swings": profile.untagged_swings,
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


def _swing_row(manifest: SwingManifest, swing_dir: Path) -> dict:
    """One swing as every listing renders it.

    Lifted out of `session_detail` when `GET /api/sessions` arrived, rather than copied into it.
    The upload page's status panel and the library page show the same five things about a swing —
    who, which club, which roles landed, what it scored — and a second literal here is a second
    thing that goes stale when a field is added. Same reason `/api/clubs` exists (M9 P7).
    """
    return {
        "swing_id": manifest.swing_id,
        "status": manifest.status(),
        "created_at": manifest.created_at.isoformat(),
        "updated_at": manifest.updated_at.isoformat(),
        "player_id": manifest.player_id,
        "club": _club_value(manifest.club),
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
        "analysis": _analysis_summary(swing_dir),
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
    # Derived from the golfer store rather than taken as a fourth `create_app` argument, because a
    # bag is not stored anywhere a golfer is not: `bag_store.py` writes `<player_id>.bag.json` into
    # the same directory `GolferStore` writes `<player_id>.golfer.json` into, and the two suffixes
    # are what keep them apart. Deriving the root means the pair cannot be pointed at different
    # directories by a caller, and no test fixture has to learn about a store it does not exercise.
    bag_store = BagStore(golfer_store.root)
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

        # Both cursors, read once and before a byte is streamed. Never from the request: both
        # phones post into the same swing, and only one of them is being held by someone who
        # knows whose swing it is and what they hit. The club is *required* where the golfer is
        # not — an untagged golfer is repairable later by `attribute_unlabeled`, whereas nothing
        # but memory can say which club hit swing 3, and a mistagged one silently pools a wedge
        # into a 7 iron's carry average (ADR-024 §5). So this refuses rather than accepting a
        # shot it can never file.
        #
        # `session_id` is computed here and threaded down rather than read again after the
        # stream: a 2 GiB upload spanning midnight would otherwise check one session's cursor
        # and write the swing into the next day's, which is a mistag nothing downstream shows.
        session_id = bundle_store.current_session_id()
        meta = load_session_meta(session_dir_of(session_id))
        if meta.club is None:
            raise HTTPException(
                status_code=409,
                detail=(
                    "no club selected for this session — POST /api/sessions/current/club "
                    "(e.g. {\"club\": \"7i\"}) before uploading"
                ),
            )

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

        result = bundle_store.assign_from_path(
            session_id=session_id,
            role=parsed_role,
            tmp_path=tmp_path,
            digest=digest.hexdigest(),
            original_filename=filename,
            content_type=request.headers.get("content-type", "application/octet-stream"),
            size_bytes=size,
            swing_id=swing_id,
            player_id=meta.player_id,
            club=meta.club,
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
            # The club the swing *says*, which on the deduped path is not the club the retry
            # asked for — a phone re-sending after the cursor moved on must not be told its
            # stale club won. `AssignmentResult.club` carries that distinction (M9 P5).
            "club": _club_value(result.club),
            "message": _status_message(result.swing_id, result.status, result.missing_roles),
        }

    @app.get("/api/sessions/current", dependencies=guard)
    async def session_current() -> dict:
        return {"session_id": bundle_store.current_session_id()}

    @app.get("/api/golfers", dependencies=guard)
    async def list_golfers() -> dict:
        """Every known golfer — what lets the page tell a returning name from a new one."""
        return {"golfers": [_golfer_payload(g) for g in golfer_store.list_all()]}

    @app.get("/api/clubs", dependencies=guard)
    async def list_clubs() -> dict:
        """Everything the upload page's club picker needs to render, in one round trip. [M9 P7]

        Sibling of `/api/golfers` above, and the answer to the question P6 deferred: the picker
        **derives** its list from here rather than inlining it. A hand-written list in
        `static/index.html` would make `contracts/club.py` the second copy of a vocabulary, and the
        failure mode is quiet — a club added to `ClubId` would parse at every route in this module
        while being unpickable at the bay, with nothing testing the static file to catch it.

        **Two things on one route** because a phone on cellular pays for round trips, and neither
        half is useful alone: the taxonomy without the bag cannot put the golfer's own clubs first,
        and the bag without the taxonomy cannot offer the club they have just borrowed.

        Order is `ClubId`'s declaration order at both sites — directly here, and through
        `Bag.club_ids` for the bag, which walks the enum for exactly this reason. Sorting either
        list is the bug the ordering exists to prevent; `3w` lands between `2h` and `5h`
        alphabetically, which is not a bag anyone recognises.

        No labels. The ids go over the wire as they are and the page styles them, because a
        server-side "7 iron" would be a second spelling table beside `_build_aliases` — free to
        disagree with the one that does the parsing.
        """
        session_id = bundle_store.current_session_id()
        player_id = load_session_meta(session_dir_of(session_id)).player_id
        # `BagStore.get` reads an unreadable or older-schema bag as absent, which is the behaviour
        # wanted here: a corrupt bag costs the "in the bag" shortcut, never the picker itself.
        bag = bag_store.get(player_id) if player_id else None
        return {
            "clubs": [club.value for club in ClubId],
            "bag": [club.value for club in bag.club_ids] if bag is not None else [],
        }

    @app.post("/api/clubs/lookup", dependencies=guard)
    def look_up_clubs(payload: ClubLookupRequest) -> dict:
        """What a named club *is* — from the catalogue if it is known, from a model if it is not.

        **Writes nothing** (ADR-026 §5). What comes back is a proposal; the golfer confirming it on
        the bag route below is what turns it into a declaration, and that separation is the only
        thing keeping a hallucinated lie angle distinguishable from a typed one.

        **Deliberately `def`, not `async def`**, exactly as `ask_about_swing` is and for the same
        reason: the model call is blocking and a seven-slot request at `EFFORT = "high"` takes long
        enough to matter. Starlette runs a sync handler in a threadpool, so a slow lookup costs one
        worker and never the event loop, the upload stream or the analysis worker. P4 left the
        timing question open and this is the answer to it — the budget is a threadpool slot, not
        the server.

        **The catalogue is consulted per slot, not per request**, so a set half of which has been
        confirmed before asks the model only for the other half. Serving one hit and looking up the
        rest is the common shape once a bag is being filled in a club at a time.

        A make or model that slugifies to nothing is a 400 rather than a lookup. The model would
        answer something for "" and `catalogue.remember` could never key it, so the request would
        be a purchase that has to be made again every time — the catalogue calling the API forever
        while looking exactly like it was working (`catalogue.catalogue_key`).

        Every other failure is a 200 carrying a `note`: no key, no `llm` extra, a rate limit, a
        refusal. A golfer who cannot reach a model still has a form to type into, which is the M9
        state this milestone improves on and not a regression from it.
        """
        make, model = payload.make.strip(), payload.model.strip()
        if not slugify(make) or not slugify(model):
            raise HTTPException(
                status_code=400,
                detail="a lookup needs a make and a model — e.g. 'Titleist' and 'T150'",
            )
        requested = _resolve_slots(payload)

        remembered = {
            slot: row
            for slot in requested
            if (row := catalogue.lookup(make, model, payload.model_year, slot)) is not None
        }
        outcome = _look_up_missing(
            make, model, payload.model_year, tuple(c for c in requested if c not in remembered)
        )
        # Keyed on the slot the spec carries, which `clubs/lookup.py` sets from the *request* and
        # never from the model's echo — so a row can never land against a club nobody asked for.
        looked_up = {spec.club: spec for spec in outcome.specs}

        candidates: list[dict[str, object]] = []
        for slot in requested:
            row = remembered.get(slot)
            if row is not None:
                # The row's own provenance travels, not the word "catalogue": the catalogue is
                # where an answer was kept and not where it came from, and flattening the two
                # would lose exactly the distinction ADR-026 §7 asks it to preserve.
                candidates.append(
                    {
                        "served_from": "catalogue",
                        "spec": row.spec.model_dump(mode="json"),
                        "provenance": row.provenance.model_dump(mode="json"),
                    }
                )
            elif (spec := looked_up.get(slot)) is not None:
                candidates.append(
                    {
                        "served_from": "lookup",
                        "spec": spec.model_dump(mode="json"),
                        # One provenance for the whole call, echoed onto each spec it produced. The
                        # page posts it back on save, which is what carries the model id and its
                        # own confidence notes into the bag entry.
                        "provenance": (
                            outcome.provenance.model_dump(mode="json")
                            if outcome.provenance is not None
                            else None
                        ),
                    }
                )

        return {
            "make": make,
            "model": model,
            "model_year": payload.model_year,
            # Identity is echoed rather than read back off a candidate, because the page needs it
            # even when there are no candidates at all — a noted empty answer still has to leave
            # the golfer looking at the club they asked about.
            "candidates": candidates,
            "note": outcome.note,
        }

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

    @app.get("/api/sessions/current/club", dependencies=guard)
    async def get_current_club() -> dict:
        session_id = bundle_store.current_session_id()
        return {
            "session_id": session_id,
            "club": _club_value(load_session_meta(session_dir_of(session_id)).club),
        }

    @app.post("/api/sessions/current/club", dependencies=guard)
    async def set_club_cursor(payload: ClubRequest) -> dict:
        """Point the session at a club. Every swing uploaded from now on is tagged with it.

        **No backfill, and that is the whole asymmetry with the golfer route above.** Picking a
        golfer reaches back over the session's unlabeled swings because a session usually has one
        golfer, so the reach is almost always right. A session has many clubs — reaching back from
        the wedge you just pulled would confidently retag the seven 7-iron shots before it, and
        nothing downstream could tell (ADR-024 §5). Reaching for the next club must therefore
        change what happens *next* and nothing that already happened; a swing tagged wrong is
        repaired one swing at a time, through the route below.
        """
        club = _resolve_club(payload)
        session_id = bundle_store.current_session_id()
        set_current_club(session_dir_of(session_id), club)
        return {"session_id": session_id, "club": club.value}

    def _registered(player_id: str) -> Golfer:
        """The four golfer-scoped routes' shared front door: validate the id, then know the golfer.

        `player_id` reaches a filesystem path in every one of them — `read_corpus` walks the
        sessions for it and `BagStore` names a file after it — so it is validated rather than
        trusted, and a literal `..` survives routing as a single segment. One home for that pair
        because a route that skipped either would look exactly like the three that do not.
        """
        _safe(player_id, "player id")
        golfer = golfer_store.get(player_id)
        if golfer is None:
            raise HTTPException(status_code=404, detail="no such golfer")
        return golfer

    def _bag_for(player_id: str) -> dict:
        """One golfer's whole bag, rebuilt from disk. One place, three routes. [M9 P19]

        The two writers below return **this**, not the entry they just saved. That gives the page a
        single render path, and it means a save shows exactly what a reload would — including P16's
        bag-changed caveat, which declaring an entry is precisely what creates. A response carrying
        only the saved row would leave the page to guess at that, and guessing wrong reads as the
        caveat appearing from nowhere on the next visit.

        The cost is one `read_corpus` per write, which is what the career route already pays per
        read.
        """
        corpus = read_corpus(bundle_store.root, player_id)
        # `BagStore.get` is the tolerant reader: an absent or unreadable bag is None, and None is
        # not a degraded mode here — `build_bag_profile` takes it and the golfer loses the loft and
        # nothing else.
        return _bag_summary(build_bag_profile(corpus, bag_store.get(player_id)))

    @app.get("/api/golfers/{player_id}/career", dependencies=guard)
    async def golfer_career(player_id: str) -> dict:
        """One golfer judged against their own history. [Career mode, step 6]

        Serves the analysis contracts as they are, rather than a flattened view. The MCP
        server flattens the identical data (`mcp/career.py`) because a model reads a flat payload
        better; a page does not need that, and inventing a second shape here would give the two
        surfaces a way to disagree about what career mode says. What is shared is the layer under
        both — `build_baseline`, `build_dispersion`, `build_standing`, `build_career_tempo`, all
        over one `read_corpus` so they provably describe the same swings.

        Everything on this route refuses today. That is the feature, and it is why the page is
        worth building before the bay session rather than after it.
        """
        golfer = _registered(player_id)

        corpus = read_corpus(bundle_store.root, player_id)
        return {
            "player_id": golfer.player_id,
            "display_name": golfer.display_name,
            "handedness": golfer.handedness.value,
            "corpus": _corpus_summary(corpus),
            "baseline": build_baseline(corpus).model_dump(mode="json"),
            "dispersion": build_dispersion(corpus).model_dump(mode="json"),
            "standing": build_standing(corpus).model_dump(mode="json"),
            # A fourth contract over the same corpus, and the only one carrying a *target* rather
            # than a description (ADR-023 addendum). It rides on this route rather than getting its
            # own for the reason the three above share one: the tempo the page prints and the
            # tempo_ratio card under it are the same golfer's, and two routes is how they acquire
            # a way to disagree about which swings that golfer has.
            "tempo": build_career_tempo(corpus).model_dump(mode="json"),
        }

    @app.get("/api/golfers/{player_id}/bag", dependencies=guard)
    async def golfer_bag(player_id: str) -> dict:
        """Every club this golfer has hit or declared, and what each one's history says. [M9 P19]

        The career route above answers "what does this golfer usually do"; this one answers it per
        club, which is the only cut on which a distance means anything — a mean carry pooled over a
        driver and a wedge describes nobody's shot (ADR-024).

        Nothing new is computed here. `analysis/club_profile.py` narrows the corpus per club and
        hands each slice to career mode's own guard, so a per-club mean refuses at the same floors a
        whole-bag one does — applied to a fraction of the same history, which is why almost
        everything on this route refuses for far longer. `scripts/club_profile.py` and the MCP
        `get_bag_profile` read the identical builder; three surfaces over one builder is what stops
        them disagreeing about how far someone hits a 7 iron.

        **The expected answer today is an empty club list**, because every swing on disk predates
        the club tag. That is a statement about the tags rather than about the golfer, and the page
        says so in different words than a refusal would.
        """
        _registered(player_id)
        return _bag_for(player_id)

    @app.post("/api/golfers/{player_id}/bag/{club}", dependencies=guard)
    async def set_bag_entry(player_id: str, club: str, payload: BagEntryRequest) -> dict:
        """Declare the physical club in one slot, or edit the one already there. [M9 P19]

        The first writer of a bag in this repo. Loft is why it exists and why it could not wait for
        the fitting models: it is the anchor those models need and it is unrecoverable after the
        fact, so the input lands now (ADR-024, Deferred).

        **An unchanged save is deliberately not special-cased here.** `BagStore.set_entry` already
        returns without writing when `BagEntry.same_club_as` matches, and its docstring names this
        route as the reason that branch exists: someone opening the page and pressing save on an
        untouched row would otherwise move `recorded_at` and hand P16 a bag-changed caveat over
        shots that were all hit with the same club.

        The club is taken off the path through `_resolve_club`, so "7 iron" works here as it does at
        the bay and `contracts/club.py` keeps its claim to be the only place free text becomes a
        `ClubId`. No `_safe` on that segment: it never becomes a filesystem path — the bag file is
        named after the golfer — and `parse_club` is the stricter guard anyway.

        **This is the confirm half of ADR-026 §5**, and the second write it performs is
        `catalogue.remember`. What a model proposed is a proposal; what a person accepted is worth
        serving to the next lookup of the same club, so the catalogue learns here and never at the
        lookup route. It cannot fail this request: every refusal in `remember` is silent, because
        the bag is the record and the catalogue is a cache — a golfer whose club is now correctly
        in their bag must not see a 500 because a JSON file was busy.

        **What the catalogue learns is this golfer's club**, which is the cost on the record: a
        shaft cut half an inch short is remembered as the model's length and served to the next
        lookup of that model. Accepted for the same reason ADR-026 §1 accepts a bent loft — the
        fields are editable and every row carries provenance — and the thing that would fix it, a
        per-field "as built" flag, is the deferred measured-loft field wearing a different hat.
        """
        golfer = _registered(player_id)
        club_id = _resolve_club(ClubRequest(club=club))
        # One clock reading for both stamps below. `recorded_at` is set here only because the
        # contract requires one; `set_entry` discards it and stamps its own. The store owns the
        # clock (`bag_store.py`), the same way `GolferStore.get_or_create` owns `created_at` — one
        # stamping site, in the layer that knows when the write actually happened.
        now = datetime.now(tz=UTC)
        # Absent means typed, and typed is a real provenance — a golfer reading the numbers off the
        # manufacturer's own page is a better source than the model is. The alternative, leaving it
        # None, would make every hand-filled entry indistinguishable from the pre-M12 ones that
        # genuinely predate anything recording a source (`BagEntry.provenance`).
        provenance = payload.provenance or SpecProvenance(source="typed", retrieved_at=now)
        entry = BagEntry(
            club=club_id,
            recorded_at=now,
            **{**_resolve_shaft_vocabularies(payload), "provenance": provenance},
        )
        bag_store.set_entry(golfer.player_id, entry)
        catalogue.remember(_spec_of(entry), provenance)
        return _bag_for(golfer.player_id)

    @app.delete("/api/golfers/{player_id}/bag/{club}", dependencies=guard)
    async def remove_bag_entry(player_id: str, club: str) -> dict:
        """Take a club out of the bag — and keep every shot it ever hit. [M9 P19]

        `BagStore.remove_entry` shelves the outgoing entry on `Bag.retired` rather than deleting it,
        so this destroys no measured loft. What the golfer loses is the club's place in the
        *current* bag, and its history is untouched: `contracts/club_profile.py` keeps `in_bag` and
        `n_swings > 0` apart for exactly this, and a club sold last year still answers "how far did
        I hit it".

        404 when the slot was already empty, which is `remove_entry` returning None. A 200 there
        would tell a page its request changed something when nothing did.
        """
        golfer = _registered(player_id)
        club_id = _resolve_club(ClubRequest(club=club))
        if bag_store.remove_entry(golfer.player_id, club_id) is None:
            raise HTTPException(status_code=404, detail=f"{club_id.value} is not in the bag")
        return _bag_for(golfer.player_id)

    @app.get("/api/sessions", dependencies=guard)
    async def list_sessions() -> dict:
        """Every session that holds a swing, newest first. The library page's one round trip.

        Until this existed there was no way to *enumerate*: the API could answer "today" and it
        could answer a session you already knew the id of, so a swing from a previous session was
        reachable only by typing its `results.html?session=…&swing=…` URL by hand. The upload page
        only ever renders the current session, which is right for the bay and wrong for looking
        back at anything.

        **Whole rows, not just ids.** Fifteen swings across six sessions is the corpus this serves,
        and a page that listed session ids and then fetched each one would pay six round trips to
        render one screen. `_swing_row` is the same projection `session_detail` sends, so the
        library and the status panel cannot disagree about what a swing looks like.

        Sessions with no swing bundle in them are skipped. `session.json` alone is a session that
        was *opened* — a golfer and club cursor were set at the bay — and four of those exist with
        nothing filmed under them; listing them as browsable history would be four dead rows.
        """
        sessions = []
        for session_id in reversed(bundle_store.list_session_ids()):
            manifests = bundle_store.get_session(session_id)
            if not manifests:
                continue
            sessions.append(
                {
                    "session_id": session_id,
                    "swings": [
                        _swing_row(manifest, swing_dir_of(session_id, manifest.swing_id))
                        for manifest in manifests
                    ],
                }
            )
        return {"sessions": sessions}

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
                _swing_row(manifest, swing_dir_of(session_id, manifest.swing_id))
                for manifest in manifests
            ],
        }

    @app.delete("/api/sessions/{session_id}/swings/{swing_id}", dependencies=guard)
    async def delete_swing(session_id: str, swing_id: str) -> dict:
        """Remove one swing entirely. The undo for a phantom the assignment rule created.

        Uploading a *corrected* file for a role a swing already has does not replace it — the
        store opens a new swing, because "newest swing lacking that role" cannot tell a repair
        from the next shot (`bundle_store.assign_from_path`). The repair is `?swing_id=`, which
        the upload page now sends; this route is what clears up the phantoms made before it did,
        and it is the reason a stray one-file swing is no longer permanent.

        Refused mid-analysis rather than racing it. The worker writes `analysis.state.json` when
        its run ends and `save_state` re-creates the directory to do it, so deleting underneath a
        running job leaves a state file for a swing that no longer exists — a swing the status
        page then renders forever with no files in it. Waiting is the golfer's call, so this says
        so instead of blocking.
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        state = load_state(swing_dir_of(session_id, swing_id))
        if state is not None and state.status in ("queued", "running"):
            raise HTTPException(
                status_code=409,
                detail=(
                    f"swing {swing_id} is being analyzed ({state.status}) — "
                    "wait for it to finish, then delete it"
                ),
            )
        if not bundle_store.delete_swing(session_id, swing_id):
            raise HTTPException(status_code=404, detail="no such swing")
        return {"session_id": session_id, "swing_id": swing_id, "deleted": True}

    @app.get("/api/sessions/{session_id}/swings/{swing_id}", dependencies=guard)
    async def swing_detail(session_id: str, swing_id: str) -> dict:
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        manifest = bundle_store.get_swing(session_id, swing_id)
        if manifest is None:
            raise HTTPException(status_code=404, detail="no such swing")
        swing_dir = swing_dir_of(session_id, swing_id)
        result = load_analysis(swing_dir)
        return {
            "session_id": session_id,
            "swing_id": swing_id,
            "status": manifest.status(),
            "missing_roles": [role.value for role in manifest.missing_roles()],
            "player_id": manifest.player_id,
            "club": _club_value(manifest.club),
            "analysis": _analysis_summary(swing_dir),
            # Null until the worker has finished; the page renders a waiting state for that.
            "result": result,
            # Two derived views over `result`, sent beside it rather than folded into it —
            # `load_analysis` returns what the pipeline wrote, and a route must not edit that.
            #
            # Both exist so the page can name a set without typing one. `measurements` mixes three
            # kinds of number: the six that back the checkpoint table, three that genuinely have
            # no band yet, and five placements whose meaning lives in a `detail` string. Rendered
            # as one list they were all captioned "measured, not yet judged", which was false for
            # eleven of the fourteen and dangerous for the placements — a distance from a tour
            # population read as a bare float looks like a score. The registries decide which is
            # which (`contracts/placements.py`, `contracts/checkpoints.py`); the page renders.
            "population": resolve_placements(result),
            "judged_metrics": judged_metrics(result),
            # And a third: a metronome built from the tour's own durations, for the one checkpoint
            # that ships a verdict a golfer cannot act on. Sent whenever it can be built rather
            # than only when tempo failed — *whether to show it* is the page's call and it already
            # holds the verdict, while *what the beats are* is this side's and must not be
            # recomputed there (ADR-023).
            "tempo_plan": resolve_tempo_plan(result),
        }

    # `def`, not `async def`, and it is the only route here that is. FastAPI runs a sync handler
    # in a threadpool and an async one on the event loop, so the choice only matters for a handler
    # that does real arithmetic — and this is the only one that does. Measured on this corpus:
    # every other route answers in 3-24 ms, this one in 15 ms where the screen printed a spin and
    # up to ~960 ms where it did not, because `spin_solve.carry_window` measures the whole
    # carry-against-spin curve in about forty flights. Blocking the loop for a second would stall
    # the upload page's 5 s status poll in another tab, which is a real second user on a
    # single-user server.
    @app.get("/api/sessions/{session_id}/swings/{swing_id}/flight", dependencies=guard)
    def swing_flight(
        session_id: str,
        swing_id: str,
        points: int = Query(DEFAULT_PATH_POINTS, ge=2, le=1000),
    ) -> dict:
        """The simulated ball flight for this swing's shot — the path, and what it is not. [M15 P14]

        **Flown here rather than read off `analysis.json`.** The stored artifact carries the six
        `flight_*` measurements but no path, and a path is what a viewer draws; re-integrating it
        costs about ten milliseconds. `api/flight_view.py` holds the derivation and the reasons.

        **A refusal is a 200 and a 404 is something else entirely**, and the line between them is
        whether there is a shot to fly. Ten of the thirteen shots on disk cannot be flown — a
        carry above the model's own peak, a 3 wood whose loft nobody has declared — and those are
        findings about the shot that come back with `flew: false` and a reason a page can print.
        A swing with no shot-screen photo, or one whose photo has never been read, has no flight
        resource at all and no sentence about physics to offer; that is the 404, and it is the
        same rule `swing_video` applies to a render that was never made.

        **No OCR here.** The store is keyed on the photo's sha256 and `api/pipeline.py::_shot_for`
        files a parse under exactly that digest, so an imported screen attaches without opening
        the image — and a request handler is the wrong place to start PaddleOCR on a phone's
        upload. An unread photo says so and names the two things that read it.
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        manifest = bundle_store.get_swing(session_id, swing_id)
        if manifest is None:
            raise HTTPException(status_code=404, detail="no such swing")

        role_file = manifest.roles.get(Role.SHOT_SCREEN)
        if role_file is None:
            raise HTTPException(
                status_code=404,
                detail=(
                    "this swing has no shot-screen photo, so there are no launch conditions to "
                    "fly — ball flight is simulated from what the launch monitor printed"
                ),
            )
        shot = ShotStore(settings.shots_dir).get(role_file.content_sha256)
        if shot is None:
            raise HTTPException(
                status_code=404,
                detail=(
                    "this swing's shot screen has not been read yet — analyze the swing, or "
                    "import the screen with scripts/import_shot_screens.py"
                ),
            )

        # The two artifacts a shot has to borrow from the swing it was hit with (M15 P10): the
        # club's loft, which picks a branch of the spin solve, and the golfer's handedness, which
        # flips the spin axis into the integrator's sign. Both are resolved off the stores this
        # app is already holding rather than through `flight_inputs.read_flight_inputs`, whose
        # photo-sha256 join exists for a caller that has a shot and no manifest. This one has the
        # manifest.
        golfer = golfer_store.get(manifest.player_id) if manifest.player_id else None
        loft_deg, gap = loft_for_club(
            manifest.player_id, manifest.club, golfers_dir=golfer_store.root
        )
        return {
            "session_id": session_id,
            "swing_id": swing_id,
            **flight_view(
                shot,
                club=manifest.club,
                loft_deg=loft_deg,
                loft_remedy=loft_remedy(gap, manifest.club),
                handedness=golfer.handedness if golfer is not None else None,
                points=points,
            ),
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

    @app.post("/api/sessions/{session_id}/swings/{swing_id}/club", dependencies=guard)
    async def set_swing_club(session_id: str, swing_id: str, payload: ClubRequest) -> dict:
        """Retag one swing. The club's **only** repair path, and deliberately the only one.

        Same shape as the golfer's repair route, and it overwrites for the same reason: it is the
        explicit human-driven override. What it has no counterpart to is `attribute_unlabeled` —
        there is no bulk fix for the club and there is not meant to be one, because only the person
        who hit swing 3 knows what they hit it with (ADR-024 §5).
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        club = _resolve_club(payload)
        manifest = bundle_store.set_club(session_id, swing_id, club)
        if manifest is None:
            raise HTTPException(status_code=404, detail="no such swing")
        return {"session_id": session_id, "swing_id": swing_id, "club": club.value}

    @app.post("/api/sessions/{session_id}/swings/{swing_id}/mishit", dependencies=guard)
    async def set_swing_mishit(session_id: str, swing_id: str, payload: MishitRequest) -> dict:
        """Confirm, clear, or reset the mishit verdict on one swing's shot (ADR-028).

        The explicit human override, `set_swing_club`'s shape exactly — and, like it, with no bulk
        counterpart, because only the golfer who hit the swing knows whether they topped it. A
        `null` verdict resets to automatic. 409 rather than 404 when the swing has no shot screen:
        the manifest exists, but a mishit verdict on a swing with no ball flight has nothing to act
        on.
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        existing = bundle_store.get_swing(session_id, swing_id)
        if existing is None:
            raise HTTPException(status_code=404, detail="no such swing")
        if Role.SHOT_SCREEN not in existing.roles:
            raise HTTPException(
                status_code=409,
                detail="this swing has no shot screen, so there is no shot to flag as a mishit",
            )
        bundle_store.set_mishit(session_id, swing_id, payload.verdict)
        return {
            "session_id": session_id,
            "swing_id": swing_id,
            "mishit": payload.verdict.value if payload.verdict else None,
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

    @app.post("/api/sessions/{session_id}/swings/{swing_id}/ask", dependencies=guard)
    def ask_about_swing(session_id: str, swing_id: str, payload: AskRequest) -> dict:
        """Ask a follow-up question about a swing, continuing a conversation if given one.

        **Deliberately `def`, not `async def`.** The tool-runner loop is blocking and takes
        seconds — several model round trips with tool calls between them. Starlette runs a sync
        handler in a threadpool, so it cannot stall the event loop, the upload stream or the
        analysis worker. If turns ever get long enough to risk an HTTP timeout the answer is a
        background job polled like `AnalysisWorker`; that is a bigger change and deliberately not
        where this starts (ADR-020).

        Every expected failure is a 200 carrying a `note` — no key, no `llm` extra, a rate limit,
        a refusal. Coaching is the least important thing that happens to a swing and must never be
        able to break the page a score is rendered on.
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        question = payload.question.strip()
        if not question:
            raise HTTPException(status_code=400, detail="a question is required")

        swing_dir = swing_dir_of(session_id, swing_id)
        if bundle_store.get_swing(session_id, swing_id) is None:
            raise HTTPException(status_code=404, detail="no such swing")

        # Imported here rather than at module scope so the server still starts without the `llm`
        # extra — `tests/api/test_pipeline_imports.py` keeps `app.py` importable without it, and
        # a missing SDK has to read as "coaching is off", not as a server that will not boot.
        try:
            from golf_coach.feedback.conversation import ask
            from golf_coach.mcp.runner_tools import build_tools
            from golf_coach.mcp.server import instructions
        except ImportError:
            return {
                "conversation_id": payload.conversation_id,
                "text": None,
                "tools_called": [],
                "note": (
                    "no answer: the `llm` extra is not installed on the server "
                    "(pip install -e '.[llm]'). The scores and tips above are unaffected."
                ),
            }

        resumed = _resume_or_seed(payload.conversation_id, session_id, swing_id, swing_dir)
        if resumed is None:
            raise HTTPException(status_code=404, detail="no such conversation")
        transcript, brief = resumed

        golfers_dir = golfer_store.root if golfer_store.root.exists() else None
        outcome = ask(
            question,
            transcript=transcript,
            tools=build_tools(bundle_store.root, _shot_source(), golfers_dir=golfers_dir),
            briefing=instructions(career_tools=golfers_dir is not None),
            model=settings.coaching_model,
            brief=brief,
            api_key=(
                settings.anthropic_api_key.get_secret_value()
                if settings.anthropic_api_key
                else None
            ),
        )
        if outcome.transcript is not None:
            save_transcript(outcome.transcript, settings.conversations_dir)

        return {
            "conversation_id": transcript.conversation_id,
            "text": outcome.text,
            "tools_called": outcome.tools_called,
            "note": outcome.note,
        }

    def _resume_or_seed(
        conversation_id: str | None, session_id: str, swing_id: str, swing_dir: Path
    ) -> tuple[Transcript, str | None] | None:
        """An existing conversation, or a new one seeded from the swing's stored analysis.

        The brief is `feedback.coach.build_brief` over the same `analysis.json` the coaching
        paragraph on this page was written from — one rendering, so the conversation and the
        paragraph above it cannot describe the swing differently. A swing with no analysis yet
        seeds nothing and the model looks everything up through the tools instead.
        """
        from golf_coach.contracts.swing import SwingBundleResult
        from golf_coach.feedback.coach import build_brief

        if conversation_id:
            existing = load_transcript(settings.conversations_dir, conversation_id)
            return (existing, None) if existing is not None else None

        brief: str | None = None
        stored = load_analysis(swing_dir)
        if stored is not None:
            try:
                brief = build_brief(SwingBundleResult.model_validate(stored))
            except ValueError:
                # An older-schema artifact still answers questions through the tools; it just
                # does not seed. Same tolerance `load_analysis` itself applies one line up.
                brief = None
        return (
            new_transcript(
                model=settings.coaching_model, session_id=session_id, swing_id=swing_id
            ),
            brief,
        )

    def _shot_source():
        """The shot reader the conversation's tools use. Built per request, like the CLI's."""
        from golf_coach.launch_monitor.composite import CompositeShotDataSource
        from golf_coach.launch_monitor.screen.source import ScreenShotDataSource

        return CompositeShotDataSource([ScreenShotDataSource(settings.shots_dir)])

    @app.get("/api/sessions/{session_id}/swings/{swing_id}/conversation", dependencies=guard)
    async def swing_conversation(session_id: str, swing_id: str) -> dict:
        """The swing's most recent conversation, rendered for display, or an empty one.

        The results page holds no conversation id after a reload (ADR-020), so this is how it
        picks the thread back up: it resolves the swing to its newest transcript server-side, so
        a second phone opening the same swing URL resumes the same thread rather than starting a
        parallel one. Empty (not 404) when none exists — 'no conversation yet' is the ordinary
        first-visit state and the panel renders the same either way. `visible_turns` drops
        thinking and tool blocks; the stored blocks are untouched (ADR-020).
        """
        _safe(session_id, "session id")
        _safe(swing_id, "swing id")
        transcript = latest_for_swing(settings.conversations_dir, session_id, swing_id)
        if transcript is None:
            return {"conversation_id": None, "turns": []}
        return {
            "conversation_id": transcript.conversation_id,
            "turns": visible_turns(transcript),
        }

    @app.get("/api/conversations/{conversation_id}", dependencies=guard)
    async def conversation_detail(conversation_id: str) -> dict:
        """One conversation, rendered for display.

        `visible_turns` drops the thinking and tool blocks — a `tool_result` is a JSON payload a
        golfer has no use for, and the sentence that quoted what mattered from it is the next text
        block along. The stored blocks are untouched; this is a rendering (ADR-020).
        """
        transcript = load_transcript(settings.conversations_dir, conversation_id)
        if transcript is None:
            raise HTTPException(status_code=404, detail="no such conversation")
        return {
            "conversation_id": transcript.conversation_id,
            "session_id": transcript.session_id,
            "swing_id": transcript.swing_id,
            "model": transcript.model,
            "updated_at": transcript.updated_at.isoformat(),
            "turns": visible_turns(transcript),
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
