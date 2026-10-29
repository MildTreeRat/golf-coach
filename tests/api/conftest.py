"""What `tests/api/` shares since M29 closed the upload route: a swing put on disk. [M29 P32]

These tests built their disk state by posting to `POST /api/uploads`, with the session's golfer and
club picked first through the two cursor routes. All three now answer 410 (the M29 plan's D11), so
a swing is filed the way the upload route filed one: the bytes written to `.incoming/` and hashed,
then handed to `SwingBundleStore.assign_from_path` with the golfer and the club the cursors would
have supplied. Through the frozen store rather than a hand-written manifest, so the read routes are
still tested over exactly what the store writes, and the dedupe, the "newest swing lacking that
role" rule and the `swing_id` override behave as they did behind the route.
"""

from __future__ import annotations

import hashlib
import uuid
from collections.abc import Callable

import pytest

from golf_coach.contracts.club import ClubId
from golf_coach.storage.bundle_store import AssignmentResult, SwingBundleStore
from golf_coach.storage.manifest import Role

#: `file_swing`'s signature: the store, the role and the bytes, then the cursors' two values and the
#: upload's optional target, as keywords.
FileSwing = Callable[..., AssignmentResult]


@pytest.fixture
def file_swing() -> FileSwing:
    """File one role's bytes into today's session, as `POST /api/uploads` did before M29.

    `club` defaults to a 7 iron because the route refused an upload with no club cursor, and every
    test that is not about the club picked one; pass `club=None` only to build what the route could
    never have written. `player_id` defaults to no golfer, as an upload before anyone picked one.
    """

    def file(
        store: SwingBundleStore,
        role: str,
        content: bytes,
        *,
        player_id: str | None = None,
        club: ClubId | None = ClubId.SEVEN_IRON,
        swing_id: str | None = None,
        filename: str | None = None,
    ) -> AssignmentResult:
        incoming = store.root / ".incoming"
        incoming.mkdir(parents=True, exist_ok=True)
        tmp_path = incoming / f"{uuid.uuid4().hex}.part"
        tmp_path.write_bytes(content)
        return store.assign_from_path(
            session_id=store.current_session_id(),
            role=Role(role),
            tmp_path=tmp_path,
            digest=hashlib.sha256(content).hexdigest(),
            original_filename=filename or f"{role}.mov",
            content_type="application/octet-stream",
            size_bytes=len(content),
            swing_id=swing_id,
            player_id=player_id,
            club=club,
        )

    return file
