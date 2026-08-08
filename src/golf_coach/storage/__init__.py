"""Storage module — flat-file, content-addressed persistence for swing bundles.

One JSON manifest per swing, content-hash-keyed files, no read-modify-write on a
shared index — the same pattern `launch_monitor/screen/store.py` uses for parsed
shots. A bad or stale swing is fixed by editing or deleting one directory, not by
migrating a database. `config.py`'s `db_path` is reserved but unused; nothing here
touches SQLite.
"""

from __future__ import annotations

from golf_coach.storage.bundle_store import SwingBundleStore
from golf_coach.storage.keypoints_io import load_keypoints, save_keypoints
from golf_coach.storage.manifest import Role, SwingManifest

__all__ = [
    "Role",
    "SwingBundleStore",
    "SwingManifest",
    "load_keypoints",
    "save_keypoints",
]
