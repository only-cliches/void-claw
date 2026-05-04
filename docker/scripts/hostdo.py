#!/usr/bin/env python3
"""
hostdo — harness-hat container-side command bridge (Python implementation).

Routes commands through the harness-hat host execution server for policy
enforcement and developer approval.  Requires only the Python 3 standard
library — no third-party packages.

Environment variables:
  HARNESS_HAT_URL      Base URL of the harness-hat manager (default: http://127.0.0.1:7878)
  HARNESS_HAT_TOKEN    Bearer token shown by the harness-hat TUI           (required)
  HARNESS_HAT_SESSION_TOKEN  Per-session token injected by harness-hat     (required)

Exit code mirrors the executed command; exits 1 on infrastructure errors.

Requires Python 3 (stdlib only — no third-party packages).
"""

import json
import os
import sys
import urllib.request
import urllib.error
import urllib.parse

# 6-minute timeout: 5-minute approval window + headroom for slow commands.
_TIMEOUT = 360


def _no_proxy_opener() -> urllib.request.OpenerDirector:
    """
    Return a URL opener that bypasses HTTP_PROXY / HTTPS_PROXY env vars.

    The harness-hat control channel must never be routed through the MITM proxy
    that harness-hat itself is managing — doing so would create a dependency loop
    and cause the approval request to be intercepted before it reaches the
    manager.
    """
    return urllib.request.build_opener(urllib.request.ProxyHandler({}))


def _default_gateway_ip() -> str:
    """
    Best-effort IPv4 default gateway lookup from /proc/net/route.
    """
    try:
        with open("/proc/net/route", "r", encoding="utf-8") as f:
            next(f, None)  # header
            for line in f:
                cols = line.strip().split()
                if len(cols) < 4:
                    continue
                destination_hex = cols[1]
                gateway_hex = cols[2]
                flags_hex = cols[3]
                if destination_hex != "00000000":
                    continue
                flags = int(flags_hex, 16)
                if (flags & 0x2) == 0:  # RTF_GATEWAY
                    continue
                g = int(gateway_hex, 16)
                octets = [
                    str(g & 0xFF),
                    str((g >> 8) & 0xFF),
                    str((g >> 16) & 0xFF),
                    str((g >> 24) & 0xFF),
                ]
                return ".".join(octets)
    except Exception:
        pass
    return ""


def _candidate_base_urls(base_url: str) -> list[str]:
    """
    Build candidate manager URLs.
    If host.docker.internal is unreachable in this runtime, fallback to the
    container's default gateway IP (and common bridge gateway as last resort).
    """
    parsed = urllib.parse.urlparse(base_url)
    host = parsed.hostname or ""
    port = parsed.port or 80
    scheme = parsed.scheme or "http"

    out = [base_url]
    if host == "host.docker.internal":
        gw = _default_gateway_ip()
        if gw:
            out.append(f"{scheme}://{gw}:{port}")
        # Common Linux default bridge fallback.
        out.append(f"{scheme}://172.17.0.1:{port}")

    # Stable dedupe.
    seen = set()
    uniq = []
    for u in out:
        if u not in seen:
            seen.add(u)
            uniq.append(u)
    return uniq


def main() -> None:
    argv = sys.argv[1:]
    if not argv:
        print("hostdo: no command specified", file=sys.stderr)
        print("usage: hostdo <command> [args...]", file=sys.stderr)
        sys.exit(1)

    base_url = os.environ.get("HARNESS_HAT_URL", "http://127.0.0.1:7878").rstrip("/")

    token = os.environ.get("HARNESS_HAT_TOKEN", "")
    if not token:
        print("hostdo: HARNESS_HAT_TOKEN is not set", file=sys.stderr)
        print("  Set it to the token shown in the harness-hat TUI.", file=sys.stderr)
        sys.exit(1)

    session_token = os.environ.get("HARNESS_HAT_SESSION_TOKEN", "")
    if not session_token:
        print("hostdo: HARNESS_HAT_SESSION_TOKEN is not set", file=sys.stderr)
        print(
            "  This container was likely started with an older harness-hat image.",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        cwd = os.getcwd()
    except OSError as exc:
        print(f"hostdo: cannot determine working directory: {exc}", file=sys.stderr)
        sys.exit(1)

    body = json.dumps({
        "argv": argv,
        "cwd": cwd,
    }).encode()

    opener = _no_proxy_opener()

    data = None
    last_err = None
    attempted = []
    for candidate_base in _candidate_base_urls(base_url):
        attempted.append(candidate_base)
        req = urllib.request.Request(
            f"{candidate_base}/exec",
            data=body,
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
                "X-Hostdo-Pid": str(os.getpid()),
                "x-harness-hat-session-token": session_token,
            },
            method="POST",
        )
        try:
            with opener.open(req, timeout=_TIMEOUT) as resp:
                data = json.loads(resp.read())
                break
        except urllib.error.HTTPError as exc:
            try:
                err = json.loads(exc.read())
                reason = err.get("reason", str(exc))
            except Exception:
                reason = str(exc)
            print(f"hostdo: denied — {reason}", file=sys.stderr)
            sys.exit(1)
        except urllib.error.URLError as exc:
            last_err = exc
            continue
        except TimeoutError:
            print("hostdo: request timed out (6 minutes)", file=sys.stderr)
            sys.exit(1)

    if data is None:
        reason = getattr(last_err, "reason", last_err)
        print(f"hostdo: request failed: {reason}", file=sys.stderr)
        print(
            "  Is harness-hat running? Is HARNESS_HAT_URL correct? "
            f"({base_url})",
            file=sys.stderr,
        )
        if len(attempted) > 1:
            print("  Tried endpoints:", file=sys.stderr)
            for u in attempted:
                print(f"    - {u}", file=sys.stderr)
        sys.exit(1)

    stdout: str = data.get("stdout", "")
    stderr: str = data.get("stderr", "")
    exit_code: int = int(data.get("exit_code", 1))

    if stdout:
        sys.stdout.write(stdout)
        sys.stdout.flush()
    if stderr:
        sys.stderr.write(stderr)
        sys.stderr.flush()

    sys.exit(exit_code)


if __name__ == "__main__":
    main()
