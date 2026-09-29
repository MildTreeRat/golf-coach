"""A worker that speaks ADR-033 and never poses anything. [M23 P4]

`crates/pose/tests/worker.rs` drives this instead of `golf_coach.pose.worker`, through
`Config::worker_args`. It exists because the failures `Worker` is built for — a crash mid-job, a
hang, a garbage line, a reply about the wrong job — cannot be provoked in the real worker without
breaking it, and because `cargo test` must not need MediaPipe, a 4K clip or 40 seconds.

**Stdlib only, and no `golf_coach` import.** Any Python on the box can run it, which is what lets
the resolution test exercise clause 9's real candidate list rather than a contrived one.

One argument: the mode. Each is one way a child process can behave, and `worker.rs` names the mode
it drives in the test that drives it. A second, optional argument is a marker *file path*, which is
how `crash_once` behaves differently in the retry's fresh process than it did in the one that died —
a mode cannot remember anything across a respawn any other way, and clause 5's retry is by
definition a different process (M23 P5).
"""

import json
import os
import sys
import time

PROTOCOL = 1

# A `KeypointsFile` small enough to read in a test and valid enough to pass `Lines::read`'s
# validation: two landmarks rather than 33, because `FrameKeypoints.landmarks` is described as 33
# in `contracts/` and deliberately not bounded there — corpus clips carry frames MediaPipe found
# nobody in, and the Rust `Validate` mirrors that.
KEYPOINTS = {
    "clip": {"fps": 60.0, "width": 1080, "height": 1920, "frame_count": 2},
    "frames": [
        {
            "frame_index": i,
            "timestamp_ms": i * 16.0,
            "landmarks": [
                {"x": 0.5, "y": 0.5, "z": 0.0, "visibility": 0.9},
                {"x": 0.25, "y": 0.75, "z": -0.1, "visibility": 0.8},
            ],
            "camera_id": "face_on",
        }
        for i in range(2)
    ],
    "pose_estimator": "mediapipe:heavy",
}


def send(message):
    sys.stdout.write(json.dumps(message, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def ready():
    send(
        {
            "pose": "ready",
            "protocol": PROTOCOL,
            "estimator": "mediapipe:heavy",
            "variant": "heavy",
            "model": "/models/pose_landmarker_heavy.task",
        }
    )


def keypoints_for(job):
    """The fixed landmarks, carrying back the `camera_id` the job asked for. [M23 P5]

    The echo is what makes attribution checkable *through the payload* rather than only through the
    envelope: `pool.rs`' tests submit N jobs whose `camera_id` is their own id, so a reply delivered
    to the wrong job is caught inside the `KeypointsFile` and not just beside it. `FrameKeypoints.
    camera_id` is free-form by design, which is why it is the field this borrows.
    """
    camera_id = job.get("camera_id") or "face_on"
    frames = [dict(frame, camera_id=camera_id) for frame in KEYPOINTS["frames"]]
    return dict(KEYPOINTS, frames=frames)


def accepted(job_id, frames=2):
    send(
        {
            "job_id": job_id,
            "pose": "accepted",
            "frames": frames,
            "fps": 60.0,
            "width": 1080,
            "height": 1920,
        }
    )


def forever():
    while True:
        time.sleep(3600)


def main(mode, marker=None):
    # The two modes that answer before the handshake, or instead of it.
    if mode == "no_handshake":
        # What a Python that cannot `import golf_coach` looks like from the outside: stderr, then
        # a non-zero exit, and not one byte on stdout.
        print("ModuleNotFoundError: No module named 'golf_coach'", file=sys.stderr)
        return 1
    if mode == "unavailable":
        send({"pose": "unavailable", "reason": "model_absent", "detail": "no .task on disk"})
        return 1
    if mode == "bad_protocol":
        send(
            {
                "pose": "ready",
                "protocol": PROTOCOL + 98,
                "estimator": "mediapipe:heavy",
                "variant": "heavy",
                "model": "/models/pose_landmarker_heavy.task",
            }
        )
        forever()
    if mode == "chatty":
        # A dependency that printed to stdout, which clause 1 forbids and `Lines::read` refuses.
        sys.stdout.write("INFO: loading the graph\n")
        sys.stdout.flush()
        forever()
    if mode == "reply_first":
        send({"job_id": "j-0", "pose": "failed", "reason": "bad_job", "detail": "out of order"})
        forever()
    if mode == "hang_handshake":
        forever()
    if mode == "venv_only":
        # The dev-box shape, and the only way one script can behave differently per interpreter: a
        # system Python dies here exactly as it would on the real worker's `import golf_coach`, and
        # the checkout's `.venv` answers. This is what clause 9's fallback is for.
        if ".venv" not in sys.executable.replace("\\", "/"):
            print(f"not the venv: {sys.executable}", file=sys.stderr)
            return 1

    ready()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        job = json.loads(line)
        job_id = job["job_id"]

        if mode == "crash_before_accept":
            return 3
        if mode == "fail_before_accept":
            # Clause 5's `clip_unreadable`: decided instead of opening a container, so it is a legal
            # first reply with no acceptance before it.
            send(
                {
                    "job_id": job_id,
                    "pose": "failed",
                    "reason": "clip_unreadable",
                    "detail": "no such file",
                }
            )
            continue
        if mode == "bad_job_unattributed":
            # What the real worker sends for a line that would not parse: there was nothing to echo.
            send(
                {
                    "job_id": "",
                    "pose": "failed",
                    "reason": "bad_job",
                    "detail": "line is not JSON",
                }
            )
            continue
        if mode == "wrong_job":
            accepted("somebody-elses-job")
            forever()
        if mode == "done_without_accept":
            send({"job_id": job_id, "pose": "done", "keypoints": KEYPOINTS})
            continue
        if mode == "hang_accept":
            forever()

        accepted(job_id)

        if mode == "crash_once":
            # Clause 5's one retryable case, made observable: the first process to see a job dies
            # after acknowledging it, and the fresh worker the pool spawns finds the marker and
            # answers. Without the file every process would crash and the retry would look like a
            # policy that does not work rather than one that does.
            if marker is not None and not os.path.exists(marker):
                with open(marker, "w", encoding="utf-8"):
                    pass
                print("Traceback (most recent call last):", file=sys.stderr)
                print("RuntimeError: the graph closed", file=sys.stderr)
                return 3
        if mode == "slow":
            # Long enough to be in flight while the test does something else, short enough that a
            # suite of them is still seconds. Nothing here poses anything, so this is the only way a
            # test can see N workers draining at once.
            time.sleep(0.4)

        if mode == "crash_after_accept":
            print("Traceback (most recent call last):", file=sys.stderr)
            print("RuntimeError: the graph closed", file=sys.stderr)
            return 3
        if mode == "hang_work":
            forever()
        if mode == "garbage":
            sys.stdout.write("this is not a protocol message\n")
            sys.stdout.flush()
            forever()
        if mode == "second_accept":
            accepted(job_id)
            forever()
        if mode == "fail_after_accept":
            send(
                {
                    "job_id": job_id,
                    "pose": "failed",
                    "reason": "pose_failed",
                    "detail": "RuntimeError: the graph closed",
                }
            )
            continue

        send({"job_id": job_id, "pose": "done", "keypoints": keypoints_for(job)})

    return 0


if __name__ == "__main__":
    sys.exit(
        main(
            sys.argv[1] if len(sys.argv) > 1 else "ready",
            sys.argv[2] if len(sys.argv) > 2 else None,
        )
    )
