#!/bin/bash
# Copies the resume PDF committed at the newest release tag of the private
# resume repo. Auth is whatever SSH identity git finds: a read-only deploy
# key in CI, your own key locally.
set -euo pipefail

REPO=git@github.com:maxwellpaulm/resume.git
PDF=paul_maxwell_resume.pdf

TAG=$(git ls-remote --tags --refs --sort=-v:refname "$REPO" 'v*' |
  awk 'NR == 1 { sub(".*refs/tags/", ""); print }')

if [ -z "$TAG" ]; then
    echo "Error: no release tag found in $REPO" >&2
    exit 1
fi

CHECKOUT=$(mktemp -d)
trap 'rm -rf "$CHECKOUT"' EXIT

git -c advice.detachedHead=false clone --quiet --depth 1 --branch "$TAG" "$REPO" "$CHECKOUT"

if [ ! -f "$CHECKOUT/$PDF" ]; then
    echo "Error: $PDF is not committed at $TAG" >&2
    exit 1
fi

mkdir -p assets
cp "$CHECKOUT/$PDF" "assets/$PDF"

echo "Resume $TAG copied to assets/$PDF"
