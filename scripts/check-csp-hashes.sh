#!/bin/bash
# Drift gate for the Content-Security-Policy the owner pastes into
# Cloudflare (see security/cloudflare-headers.md). Recomputes the sha256
# of every inline <script> body actually shipped in dist/**/*.html and
# compares that set, exactly, against the non-comment lines of
# security/csp-hashes.txt — the file the Cloudflare rule mirrors.
#
# A hash present in dist but missing from the file means a script
# changed (or a new one was added) and the CSP would now block it.
# A hash present in the file but absent from dist is just as much
# drift: a stale allowance nobody would ever notice go unused, until
# the day it silently permits something it shouldn't.
#
# Run after `cargo run -p site -- --strict` has produced dist/.
set -euo pipefail

cd "$(dirname "$0")/.."

DIST=dist
HASHES_FILE=security/csp-hashes.txt

if [ ! -d "$DIST" ]; then
    echo "Error: $DIST not found — build the site first (cargo run -p site -- --strict)" >&2
    exit 1
fi

if [ ! -f "$HASHES_FILE" ]; then
    echo "Error: $HASHES_FILE not found" >&2
    exit 1
fi

python3 - "$DIST" "$HASHES_FILE" <<'PYEOF'
import base64
import hashlib
import html
import re
import sys
from pathlib import Path

dist_dir, hashes_file = sys.argv[1], sys.argv[2]

script_re = re.compile(r"<script(?:\s[^>]*)?>(.*?)</script>", re.DOTALL | re.IGNORECASE)

# hash -> first snippet seen with that hash, for readable failure output.
dist_hashes = {}
for path in sorted(Path(dist_dir).rglob("*.html")):
    content = path.read_text(encoding="utf-8")
    for match in script_re.finditer(content):
        body = match.group(1)
        if body.strip() == "":
            # External/module scripts (<script src="...">) have no body
            # to hash and aren't covered by this policy's script-src
            # hash allowlist.
            continue
        digest = base64.b64encode(hashlib.sha256(body.encode("utf-8")).digest()).decode()
        sha = f"sha256-{digest}"
        dist_hashes.setdefault(sha, (body[:60].replace("\n", "\\n"), str(path)))

allowed_hashes = set()
for line in Path(hashes_file).read_text(encoding="utf-8").splitlines():
    stripped = line.strip()
    if not stripped or stripped.startswith("#"):
        continue
    allowed_hashes.add(stripped)

dist_set = set(dist_hashes.keys())

extra_in_dist = dist_set - allowed_hashes
stale_in_file = allowed_hashes - dist_set

ok = True

if extra_in_dist:
    ok = False
    print("CSP DRIFT: inline <script> hash(es) in dist/ not listed in " + hashes_file + ":")
    for sha in sorted(extra_in_dist):
        snippet, path = dist_hashes[sha]
        print(f"  {sha}")
        print(f"    first 60 chars: {html.unescape(snippet)!r}")
        print(f"    found in: {path}")

if stale_in_file:
    ok = False
    print("CSP DRIFT: hash(es) listed in " + hashes_file + " but not found in any dist/**/*.html script:")
    for sha in sorted(stale_in_file):
        print(f"  {sha}")

if not ok:
    print()
    print("The Cloudflare CSP and " + hashes_file + " must list exactly the hashes")
    print("of the inline scripts the site currently ships. Update both together.")
    sys.exit(1)

print(f"OK: {len(dist_set)} inline <script> hash(es) in dist/ match {hashes_file} exactly.")
for sha in sorted(dist_set):
    snippet, _ = dist_hashes[sha]
    print(f"  {sha}  ({html.unescape(snippet)!r}...)")
PYEOF
