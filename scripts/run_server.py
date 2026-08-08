"""Dev CLI: run the phone-upload server. [M7 Phases 3+5, exposed in Phase 6]

Usage:
    python scripts/run_server.py
    python scripts/run_server.py --port 9000

Serves the upload page and /api/uploads at http://127.0.0.1:<port>/, loopback only.

Reaching it from a phone does NOT mean widening this bind. Tailscale terminates TLS and
proxies to the loopback port (ADR-016):

    tailscale serve --bg <port>      # your own devices, over the tailnet
    tailscale funnel --bg <port>     # anyone with the link — needs GOLF_UPLOAD_TOKEN

`--host` exists for unusual setups, but binding beyond loopback without GOLF_UPLOAD_TOKEN
set is refused: the upload endpoint would otherwise let anyone who can route to this
machine write files to disk.

Requires the `api` extra: pip install -e '.[api]'
"""

from __future__ import annotations

import argparse
import ipaddress
import sys

from golf_coach.config import settings

_DEFAULT_HOST = "127.0.0.1"


def _is_loopback(host: str) -> bool:
    """True only for addresses that cannot be reached from another machine.

    Unparseable hostnames count as non-loopback: `localhost` resolves to loopback, but so
    do plenty of names that don't, and guessing wrong here is the failure this guards.
    """
    if host == "localhost":
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="run_server", description=__doc__)
    parser.add_argument(
        "--host", default=_DEFAULT_HOST, help="bind address (default: loopback only)"
    )
    parser.add_argument("--port", type=int, default=settings.api_port)
    args = parser.parse_args(argv)

    if not _is_loopback(args.host) and not settings.upload_token:
        print(
            f"error: refusing to bind {args.host} with no upload token.\n"
            "       /api/uploads has no other authentication, so a non-loopback bind\n"
            "       would expose an unauthenticated file-upload endpoint.\n"
            "       Either set GOLF_UPLOAD_TOKEN in .env, or leave the bind on\n"
            "       127.0.0.1 and put `tailscale serve` in front of it (ADR-016).",
            file=sys.stderr,
        )
        return 2

    try:
        import uvicorn
    except ImportError:
        print(
            "error: uvicorn not installed - the server needs the `api` extra.\n"
            "       pip install -e '.[api]'",
            file=sys.stderr,
        )
        return 1

    from golf_coach.api.app import create_app

    print(f"upload server:  http://{args.host}:{args.port}/")
    if settings.upload_token:
        print(f"phone setup link ends with:  /?t={settings.upload_token}")
    else:
        print("no GOLF_UPLOAD_TOKEN set - /api/ is open to anything that reaches this port.")
    print(f"to reach it from a phone:  tailscale serve --bg {args.port}")

    uvicorn.run(create_app(), host=args.host, port=args.port)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
