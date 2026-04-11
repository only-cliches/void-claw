#!/usr/bin/env python3
"""
killme — void-claw container exit command.

Requests that the void-claw manager stop the current container session.

Environment variables:
  VOID_CLAW_URL      Base URL of the void-claw manager (default: http://127.0.0.1:7878)
  VOID_CLAW_TOKEN    Bearer token shown by the void-claw TUI           (required)
  VOID_CLAW_SESSION_TOKEN  Per-session token injected by void-claw     (required)
"""

import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

_TIMEOUT = 30


def _no_proxy_opener() -> urllib.request.OpenerDirector:
    return urllib.request.build_opener(urllib.request.ProxyHandler({}))


def _candidate_base_urls(base_url: str) -> list[str]:
    parsed = urllib.parse.urlparse(base_url)
    host = parsed.hostname or ""
    port = parsed.port or 80
    scheme = parsed.scheme or "http"

    out = [base_url]
    if host == "host.docker.internal":
        out.append(f"{scheme}://172.17.0.1:{port}")

    seen = set()
    uniq = []
    for u in out:
        if u not in seen:
            seen.add(u)
            uniq.append(u)
    return uniq


def main() -> None:
    base_url = os.environ.get("VOID_CLAW_URL", "http://127.0.0.1:7878").rstrip("/")

    token = os.environ.get("VOID_CLAW_TOKEN", "")
    if not token:
        print("killme: VOID_CLAW_TOKEN is not set", file=sys.stderr)
        sys.exit(1)

    session_token = os.environ.get("VOID_CLAW_SESSION_TOKEN", "")
    if not session_token:
        print("killme: VOID_CLAW_SESSION_TOKEN is not set", file=sys.stderr)
        sys.exit(1)

    body = json.dumps({}).encode()

    opener = _no_proxy_opener()
    last_err = None

    for candidate_base in _candidate_base_urls(base_url):
        req = urllib.request.Request(
            f"{candidate_base}/container/stop",
            data=body,
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
                "x-void-claw-session-token": session_token,
            },
            method="POST",
        )
        try:
            with opener.open(req, timeout=_TIMEOUT) as resp:
                data = json.loads(resp.read())
                if data.get("ok"):
                    sys.exit(0)
                print("killme: unexpected response from manager", file=sys.stderr)
                sys.exit(1)
        except urllib.error.HTTPError as exc:
            try:
                err = json.loads(exc.read())
                reason = err.get("reason", str(exc))
            except Exception:
                reason = str(exc)
            print(f"killme: denied — {reason}", file=sys.stderr)
            sys.exit(1)
        except urllib.error.URLError as exc:
            last_err = exc
            continue
        except TimeoutError:
            print("killme: request timed out", file=sys.stderr)
            sys.exit(1)

    reason = getattr(last_err, "reason", last_err)
    print(f"killme: request failed: {reason}", file=sys.stderr)
    print(f"  Is void-claw running? Is VOID_CLAW_URL correct? ({base_url})", file=sys.stderr)
    sys.exit(1)


if __name__ == "__main__":
    main()
