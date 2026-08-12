#!/bin/bash
# Downloads the resume PDF from the latest release of the private resume repo.
set -euo pipefail

if [ -z "${GITHUB_TOKEN:-}" ]; then
    echo "Error: GITHUB_TOKEN is required for private repo access" >&2
    exit 1
fi

mkdir -p assets

ASSET_URL=$(curl -sSf -H "Authorization: token $GITHUB_TOKEN" \
  https://api.github.com/repos/maxwellpaulm/resume/releases/latest | \
  jq -r '.assets[] | select(.name == "paul_maxwell_resume.pdf") | .url')

if [ -z "$ASSET_URL" ] || [ "$ASSET_URL" = "null" ]; then
    echo "Error: could not find paul_maxwell_resume.pdf in the latest release" >&2
    exit 1
fi

curl -fL -H "Authorization: token $GITHUB_TOKEN" \
  -H "Accept: application/octet-stream" \
  "$ASSET_URL" -o assets/paul_maxwell_resume.pdf

echo "Resume downloaded to assets/paul_maxwell_resume.pdf"
